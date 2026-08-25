import { spawn } from "node:child_process";
import { EventEmitter } from "node:events";
import readline from "node:readline";

export class AppServerTransportError extends Error {}

export class StdioJsonRpcTransport extends EventEmitter {
  static spawn({
    command = "codex",
    args = ["app-server", "--stdio"],
    cwd = process.cwd(),
    env = process.env,
  } = {}) {
    const child = spawn(command, args, { cwd, env, stdio: ["pipe", "pipe", "pipe"] });
    return new StdioJsonRpcTransport(child);
  }

  constructor(child) {
    super();
    this.child = child;
    this.nextId = 1;
    this.pending = new Map();
    this.serverRequestHandler = null;
    this.closed = false;

    readline.createInterface({ input: child.stdout }).on("line", (line) => {
      this.#receive(line).catch((error) => this.emit("protocolError", error));
    });
    child.stderr?.on("data", (chunk) => this.emit("stderr", chunk.toString()));
    child.on("error", (error) => this.#fail(error));
    child.on("exit", (code, signal) => {
      this.#fail(new AppServerTransportError(`App Server exited (${code ?? signal})`));
    });
  }

  onServerRequest(handler) {
    this.serverRequestHandler = handler;
  }

  onNotification(handler) {
    this.on("notification", handler);
    return () => this.off("notification", handler);
  }

  onClosed(handler) {
    this.on("closed", handler);
    return () => this.off("closed", handler);
  }

  #send(message) {
    if (this.closed || !this.child.stdin?.writable) {
      throw new AppServerTransportError("App Server transport is closed");
    }
    this.child.stdin.write(`${JSON.stringify(message)}\n`);
  }

  request(method, params) {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      try {
        this.#send({ method, id, params });
      } catch (error) {
        this.pending.delete(id);
        reject(error);
      }
    });
  }

  notify(method, params) {
    this.#send(params === undefined ? { method } : { method, params });
  }

  async #receive(line) {
    let message;
    try {
      message = JSON.parse(line);
    } catch (error) {
      throw new AppServerTransportError(`invalid App Server JSON: ${error.message}`);
    }

    if (message.id != null && ("result" in message || "error" in message)) {
      const pending = this.pending.get(message.id);
      if (!pending) throw new AppServerTransportError(`unexpected response id ${message.id}`);
      this.pending.delete(message.id);
      if (message.error) pending.reject(new AppServerTransportError(message.error.message));
      else pending.resolve(message.result);
      return;
    }

    if (message.id != null && message.method) {
      if (!this.serverRequestHandler) {
        this.#send({ id: message.id, error: { code: -32601, message: "method not found" } });
        return;
      }
      try {
        const result = await this.serverRequestHandler(message);
        this.#send({ id: message.id, result });
      } catch (error) {
        this.#send({
          id: message.id,
          error: { code: -32000, message: error instanceof Error ? error.message : "request failed" },
        });
      }
      return;
    }

    if (message.method) {
      this.emit("notification", message);
      return;
    }
    throw new AppServerTransportError("unrecognized App Server message");
  }

  #fail(error) {
    if (this.closed) return;
    this.closed = true;
    for (const pending of this.pending.values()) pending.reject(error);
    this.pending.clear();
    this.emit("closed", error);
  }

  close() {
    if (this.closed) return;
    const error = new AppServerTransportError("App Server transport closed");
    this.closed = true;
    this.child.stdin?.end();
    for (const pending of this.pending.values()) {
      pending.reject(error);
    }
    this.pending.clear();
    this.emit("closed", error);
  }
}
