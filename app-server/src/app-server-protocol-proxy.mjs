import { EventEmitter } from "node:events";
import { chmod, lstat, unlink } from "node:fs/promises";
import http from "node:http";

import { WebSocket, WebSocketServer } from "ws";

const RPC_PATH = "/rpc";

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
  constructor({ transport, socketPath, rpcPath = RPC_PATH }) {
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
    this.transport = transport;
    this.socketPath = socketPath;
    this.rpcPath = rpcPath;
    this.peer = null;
    this.pendingServerRequests = new Map();
    this.socketIdentity = null;
    this.closing = false;
    this.httpServer = http.createServer((request, response) => {
      response.writeHead(426, { Connection: "close", "Content-Length": "0" });
      response.end();
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
