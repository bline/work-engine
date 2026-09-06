import { EventEmitter } from "node:events";
import { chmod, lstat, unlink } from "node:fs/promises";
import http from "node:http";

import { WebSocket, WebSocketServer } from "ws";

const RPC_PATH = "/rpc";
const CONTROL_PATH = "/work-engine/control";
const CONTROL_BODY_LIMIT = 16 * 1024;

function protocolId(value) {
  if (typeof value === "string" || Number.isSafeInteger(value)) return value;
  return null;
}

function responseError(error) {
  return {
    code: -32000,
    message: error instanceof Error ? error.message : "App Server request failed",
  };
}

function sendHttpJson(response, statusCode, value) {
  const body = Buffer.from(`${JSON.stringify(value)}\n`, "utf8");
  response.writeHead(statusCode, {
    Connection: "close",
    "Content-Length": String(body.length),
    "Content-Type": "application/json; charset=utf-8",
  });
  response.end(body);
}

async function readHttpJson(request) {
  const chunks = [];
  let length = 0;
  for await (const chunk of request) {
    length += chunk.length;
    if (length > CONTROL_BODY_LIMIT) {
      const error = new Error("operator control request body exceeds 16 KiB");
      error.code = "control_body_too_large";
      throw error;
    }
    chunks.push(chunk);
  }
  try {
    return JSON.parse(Buffer.concat(chunks).toString("utf8"));
  } catch {
    const error = new Error("operator control request body must be valid JSON");
    error.code = "invalid_control_json";
    throw error;
  }
}

function sendJson(peer, message) {
  if (!peer || peer.readyState !== WebSocket.OPEN) {
    throw new Error("Codex remote client is not connected");
  }
  peer.send(JSON.stringify(message));
}

function rejectUpgrade(socket, status, reason) {
  socket.write(
    `HTTP/1.1 ${status} ${reason}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n`,
  );
  socket.destroy();
}

async function sameSocket(pathname, identity) {
  try {
    const current = await lstat(pathname);
    return current.isSocket() && current.dev === identity.dev && current.ino === identity.ino;
  } catch (error) {
    if (error?.code === "ENOENT") return false;
    throw error;
  }
}

export class AppServerProtocolProxy extends EventEmitter {
  constructor({
    transport,
    socketPath,
    rpcPath = RPC_PATH,
    controlPath = CONTROL_PATH,
    operatorControl = null,
  }) {
    super();
    if (!transport || typeof transport.request !== "function"
        || typeof transport.notify !== "function"
        || typeof transport.onServerRequest !== "function"
        || typeof transport.onNotification !== "function") {
      throw new TypeError("App Server protocol proxy requires a compatible transport");
    }
    if (typeof socketPath !== "string" || socketPath.length === 0) {
      throw new TypeError("App Server protocol proxy requires a Unix socket path");
    }
    if (typeof rpcPath !== "string" || !rpcPath.startsWith("/")) {
      throw new TypeError("App Server protocol proxy RPC path must begin with /");
    }
    if (typeof controlPath !== "string" || !controlPath.startsWith("/")
        || controlPath === rpcPath) {
      throw new TypeError("App Server protocol proxy control path must be distinct and begin with /");
    }
    if (operatorControl !== null
        && (typeof operatorControl.status !== "function"
          || typeof operatorControl.interrupt !== "function")) {
      throw new TypeError("App Server protocol proxy operator control is incompatible");
    }
    this.transport = transport;
    this.operatorControl = operatorControl;
    this.socketPath = socketPath;
    this.rpcPath = rpcPath;
    this.controlPath = controlPath;
    this.peer = null;
    this.pendingServerRequests = new Map();
    this.socketIdentity = null;
    this.closing = false;
    this.httpServer = http.createServer((request, response) => {
      this.#receiveHttp(request, response).catch((error) => {
        if (response.headersSent) {
          response.destroy();
          return;
        }
        const clientError = error instanceof TypeError
          || ["control_body_too_large", "invalid_control_json"].includes(error?.code);
        const conflict = ["ambiguous_active_turn", "no_active_turn"]
          .includes(error?.code);
        sendHttpJson(response, clientError ? 400 : conflict ? 409 : 500, {
          ok: false,
          error: {
            code: typeof error?.code === "string" ? error.code : "operator_control_failed",
            message: error instanceof Error ? error.message : "operator control failed",
          },
        });
        this.emit("controlError", error);
      });
    });
    this.webSocketServer = new WebSocketServer({ noServer: true });

    this.httpServer.on("upgrade", (request, socket, head) => {
      if (request.url !== this.rpcPath) {
        rejectUpgrade(socket, 404, "Not Found");
        return;
      }
      if (this.peer && this.peer.readyState !== WebSocket.CLOSED) {
        rejectUpgrade(socket, 409, "Conflict");
        return;
      }
      this.webSocketServer.handleUpgrade(request, socket, head, (peer) => {
        this.webSocketServer.emit("connection", peer, request);
      });
    });
    this.webSocketServer.on("connection", (peer) => this.#attach(peer));
    transport.onServerRequest((request) => this.#requestClient(request));
    transport.onNotification((notification) => {
      if (!this.peer || this.peer.readyState !== WebSocket.OPEN) return;
      try {
        sendJson(this.peer, notification);
      } catch (error) {
        this.emit("protocolError", error);
      }
    });
    transport.onClosed?.((error) => {
      this.peer?.close(1011, "App Server transport closed");
      this.emit("transportClosed", error);
    });
    transport.onLifecycleError?.((error) => this.emit("protocolError", error));
  }

  async listen() {
    if (this.httpServer.listening) return this.socketPath;
    await new Promise((resolve, reject) => {
      const onError = (error) => {
        this.httpServer.off("listening", onListening);
        reject(error);
      };
      const onListening = () => {
        this.httpServer.off("error", onError);
        resolve();
      };
      this.httpServer.once("error", onError);
      this.httpServer.once("listening", onListening);
      this.httpServer.listen(this.socketPath);
    });
    await chmod(this.socketPath, 0o600);
    const identity = await lstat(this.socketPath);
    this.socketIdentity = { dev: identity.dev, ino: identity.ino };
    return this.socketPath;
  }

  #attach(peer) {
    this.peer = peer;
    peer.on("message", (data, isBinary) => {
      if (isBinary) {
        peer.close(1003, "App Server messages must be text");
        return;
      }
      this.#receive(peer, data.toString()).catch((error) => this.emit("protocolError", error));
    });
    peer.on("close", () => {
      if (this.peer === peer) this.peer = null;
      const error = new Error("Codex remote client disconnected");
      for (const pending of this.pendingServerRequests.values()) pending.reject(error);
      this.pendingServerRequests.clear();
      this.emit("clientDisconnected");
    });
    peer.on("error", (error) => this.emit("protocolError", error));
    this.emit("clientConnected");
  }

  async #receiveHttp(request, response) {
    if (request.url !== this.controlPath) {
      response.writeHead(426, { Connection: "close", "Content-Length": "0" });
      response.end();
      return;
    }
    if (request.method !== "POST") {
      response.writeHead(405, {
        Allow: "POST",
        Connection: "close",
        "Content-Length": "0",
      });
      response.end();
      return;
    }
    const input = await readHttpJson(request);
    if (!input || typeof input !== "object" || Array.isArray(input)
        || typeof input.command !== "string"
        || Object.keys(input).some((key) => !["command", "target"].includes(key))) {
      throw new TypeError("operator control request must contain only command and optional target");
    }
    this.emit("controlRequest", { command: input.command });
    let result;
    if (input.command === "status") {
      if (input.target !== undefined) {
        throw new TypeError("operator status does not accept a target");
      }
      result = {
        ...this.operatorControl?.status(),
        clientConnected: Boolean(this.peer && this.peer.readyState === WebSocket.OPEN),
      };
    } else if (input.command === "interrupt") {
      if (!this.operatorControl) {
        const error = new Error("operator interruption is unavailable");
        error.code = "operator_control_unavailable";
        throw error;
      }
      result = await this.operatorControl.interrupt(input.target ?? {});
    } else if (input.command === "disconnect") {
      if (input.target !== undefined) {
        throw new TypeError("operator disconnect does not accept a target");
      }
      const connected = Boolean(this.peer && this.peer.readyState === WebSocket.OPEN);
      if (connected) this.peer.close(1000, "Detached by Work Engine operator control");
      result = { schemaVersion: 1, status: connected ? "disconnect_requested" : "disconnected" };
    } else {
      throw new TypeError(`unknown operator control command ${input.command}`);
    }
    sendHttpJson(response, 200, { ok: true, result });
    this.emit("controlResponse", { command: input.command, status: result?.status ?? "ok" });
  }

  async #receive(peer, text) {
    let message;
    try {
      message = JSON.parse(text);
    } catch {
      sendJson(peer, { id: null, error: { code: -32700, message: "Parse error" } });
      return;
    }
    if (!message || typeof message !== "object" || Array.isArray(message)) {
      sendJson(peer, { id: null, error: { code: -32600, message: "Invalid Request" } });
      return;
    }

    const id = protocolId(message.id);
    if (id !== null && ("result" in message || "error" in message)) {
      const pending = this.pendingServerRequests.get(id);
      if (!pending) {
        throw new Error(`unexpected Codex remote response id ${id}`);
      }
      this.pendingServerRequests.delete(id);
      if (message.error) pending.reject(new Error(message.error.message ?? "client request failed"));
      else pending.resolve(message.result);
      return;
    }

    if (typeof message.method !== "string" || message.method.length === 0) {
      sendJson(peer, { id, error: { code: -32600, message: "Invalid Request" } });
      return;
    }
    if (id === null) {
      await this.transport.notify(message.method, message.params);
      return;
    }
    this.emit("clientRequest", { id, method: message.method });
    let result;
    try {
      result = await this.transport.request(message.method, message.params);
      this.emit("clientResponse", {
        id,
        method: message.method,
        threadId: result?.thread?.id ?? null,
        turnId: result?.turn?.id ?? null,
        turnStatus: result?.turn?.status ?? null,
      });
    } catch (error) {
      if (!this.closing && this.peer === peer) {
        this.emit("requestError", {
          method: message.method,
          error: error instanceof Error ? error : new Error("App Server request failed"),
        });
      }
      if (this.peer === peer && peer.readyState === WebSocket.OPEN) {
        sendJson(peer, { id: message.id, error: responseError(error) });
      }
      return;
    }
    if (this.peer === peer && peer.readyState === WebSocket.OPEN) {
      sendJson(peer, { id: message.id, result });
    }
  }

  #requestClient(request) {
    const id = protocolId(request?.id);
    if (id === null) throw new Error("App Server request requires a protocol id");
    if (this.pendingServerRequests.has(id)) {
      throw new Error(`duplicate App Server request id ${id}`);
    }
    return new Promise((resolve, reject) => {
      this.pendingServerRequests.set(id, { resolve, reject });
      try {
        sendJson(this.peer, request);
      } catch (error) {
        this.pendingServerRequests.delete(id);
        reject(error);
      }
    });
  }

  async close() {
    if (this.closing) return;
    this.closing = true;
    const error = new Error("App Server protocol proxy closed");
    for (const pending of this.pendingServerRequests.values()) pending.reject(error);
    this.pendingServerRequests.clear();
    if (this.peer) {
      this.peer.close(1001, "App Server protocol proxy closed");
      this.peer = null;
    }
    await new Promise((resolve) => this.webSocketServer.close(resolve));
    if (this.httpServer.listening) {
      await new Promise((resolve, reject) => this.httpServer.close((closeError) => {
        if (closeError) reject(closeError);
        else resolve();
      }));
    }
    if (this.socketIdentity && await sameSocket(this.socketPath, this.socketIdentity)) {
      await unlink(this.socketPath);
    }
    this.transport.close?.();
  }
}
