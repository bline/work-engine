import { fork } from "node:child_process";
import { EventEmitter } from "node:events";
import path from "node:path";

const PROTOCOL = "work-engine.executable-generation-ipc";
const PROTOCOL_VERSION = 1;

function text(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function generationRecord(value) {
  return Object.freeze({
    generationId: text(value?.generationId, "generation id"),
    sourceDigest: text(value?.sourceDigest, "generation source digest"),
    environmentFingerprint: text(
      value?.environmentFingerprint,
      "generation environment fingerprint",
    ),
    bootstrapFingerprint: text(
      value?.bootstrapFingerprint,
      "generation bootstrap fingerprint",
    ),
  });
}

function failureType(error) {
  return error instanceof Error ? error.name : typeof error;
}

function diagnostic(error, fallbackCode, fallbackMessage) {
  const candidateCode = error instanceof Error ? error.code : null;
  const code = typeof candidateCode === "string" && /^[a-z][a-z0-9_]{0,63}$/.test(candidateCode)
    ? candidateCode : fallbackCode;
  const candidateMessage = error instanceof Error ? error.message : null;
  const normalizedMessage = typeof candidateMessage === "string"
    ? candidateMessage.replace(/[\u0000-\u001f\u007f]+/g, " ").trim().slice(0, 240)
    : "";
  return { code, message: normalizedMessage || fallbackMessage };
}

function diagnosticEnvelope(value, fallbackCode, fallbackMessage) {
  const code = typeof value?.code === "string" && /^[a-z][a-z0-9_]{0,63}$/.test(value.code)
    ? value.code : fallbackCode;
  const normalizedMessage = typeof value?.message === "string"
    ? value.message.replace(/[\u0000-\u001f\u007f]+/g, " ").trim().slice(0, 240)
    : "";
  return { code, message: normalizedMessage || fallbackMessage };
}

function protocolMessage(message) {
  return message && message.protocol === PROTOCOL && message.version === PROTOCOL_VERSION
    && Number.isSafeInteger(message.id);
}

export class ExecutableGenerationWorkerError extends Error {
  constructor(code, message, details = {}) {
    super(message);
    this.name = "ExecutableGenerationWorkerError";
    this.code = code;
    this.details = Object.freeze({ ...details });
  }
}

export class ForkedExecutableGenerationWorker extends EventEmitter {
  static async spawn({
    entryPath,
    args = [],
    cwd = process.cwd(),
    env = process.env,
    requestTimeoutMs = 10_000,
    dispatchTimeoutMs = 30 * 60_000,
  } = {}) {
    text(entryPath, "generation worker entry path");
    if (!Array.isArray(args) || !args.every((value) => typeof value === "string")) {
      throw new TypeError("generation worker arguments must be strings");
    }
    const child = fork(path.resolve(entryPath), args, {
      cwd,
      env,
      serialization: "json",
      stdio: ["ignore", "pipe", "pipe", "ipc"],
    });
    const worker = new ForkedExecutableGenerationWorker(child, {
      requestTimeoutMs,
      dispatchTimeoutMs,
    });
    try {
      const description = await worker.#request("generation.describe", {});
      Object.assign(worker, generationRecord(description));
      return worker;
    } catch (error) {
      worker.#terminate();
      throw error;
    }
  }

  constructor(child, {
    requestTimeoutMs = 10_000,
    dispatchTimeoutMs = 30 * 60_000,
  } = {}) {
    super();
    if (!child || typeof child.send !== "function") {
      throw new TypeError("generation worker requires a forked child process");
    }
    if (!Number.isSafeInteger(requestTimeoutMs) || requestTimeoutMs < 1) {
      throw new TypeError("generation worker timeout must be a positive integer");
    }
    if (!Number.isSafeInteger(dispatchTimeoutMs) || dispatchTimeoutMs < 1) {
      throw new TypeError("generation worker dispatch timeout must be a positive integer");
    }
    this.child = child;
    this.requestTimeoutMs = requestTimeoutMs;
    this.dispatchTimeoutMs = dispatchTimeoutMs;
    this.nextRequestId = 1;
    this.pending = new Map();
    this.closed = false;
    this.disposed = false;
    child.on("message", (message) => this.#receive(message));
    child.stdout?.on("data", (chunk) => this.emit("stdout", chunk.toString()));
    child.stderr?.on("data", (chunk) => this.emit("stderr", chunk.toString()));
    child.on("error", (error) => this.#close(error));
    child.on("exit", (code, signal) => this.#close(new ExecutableGenerationWorkerError(
      "worker_exited",
      `generation worker exited (${code ?? signal})`,
      { code, signal },
    )));
  }

  #receive(message) {
    if (!protocolMessage(message)) {
      this.#close(new ExecutableGenerationWorkerError(
        "invalid_worker_message",
        "generation worker returned an invalid protocol message",
      ));
      return;
    }
    const pending = this.pending.get(message.id);
    if (!pending) {
      this.#close(new ExecutableGenerationWorkerError(
        "unexpected_worker_response",
        `generation worker returned unexpected response ${message.id}`,
      ));
      return;
    }
    if (message.kind === "effect_request") {
      this.#handleEffectRequest(message, pending);
      return;
    }
    if (message.kind !== undefined) {
      this.#close(new ExecutableGenerationWorkerError(
        "invalid_worker_message",
        "generation worker returned an unsupported protocol message kind",
      ));
      return;
    }
    this.pending.delete(message.id);
    clearTimeout(pending.timeout);
    if (message.error) {
      const bounded = diagnosticEnvelope(
        message.error,
        "worker_request_failed",
        `generation worker ${pending.method} failed`,
      );
      pending.reject(new ExecutableGenerationWorkerError(
        "worker_request_failed",
        bounded.message,
        { diagnosticCode: bounded.code },
      ));
    } else {
      pending.resolve(message.result);
    }
  }

  async #handleEffectRequest(message, pending) {
    if (!Number.isSafeInteger(message.effectId) || typeof pending.effect !== "function") {
      this.#close(new ExecutableGenerationWorkerError(
        "invalid_worker_effect",
        "generation worker requested an effect outside its dispatch boundary",
      ));
      return;
    }
    const response = {
      protocol: PROTOCOL,
      version: PROTOCOL_VERSION,
      kind: "effect_response",
      id: message.id,
      effectId: message.effectId,
    };
    try {
      response.result = await pending.effect(structuredClone(message.payload));
    } catch (error) {
      response.error = diagnostic(error, "effect_failed", "stable generation effect failed");
    }
    if (this.closed || !this.child.connected) return;
    this.child.send(response, (error) => {
      if (error) this.#close(new ExecutableGenerationWorkerError(
        "worker_effect_response_failed",
        "generation worker effect response could not be delivered",
        { failureType: failureType(error) },
      ));
    });
  }

  #request(method, params, { effect = null, timeoutMs = this.requestTimeoutMs } = {}) {
    if (this.closed || !this.child.connected) {
      return Promise.reject(new ExecutableGenerationWorkerError(
        "worker_unavailable",
        "generation worker is unavailable",
      ));
    }
    const id = this.nextRequestId++;
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pending.delete(id);
        reject(new ExecutableGenerationWorkerError(
          "worker_timeout",
          `generation worker ${method} timed out`,
        ));
      }, timeoutMs);
      this.pending.set(id, { method, resolve, reject, timeout, effect });
      this.child.send({ protocol: PROTOCOL, version: PROTOCOL_VERSION, id, method, params }, (error) => {
        if (!error) return;
        const pending = this.pending.get(id);
        if (!pending) return;
        this.pending.delete(id);
        clearTimeout(pending.timeout);
        reject(new ExecutableGenerationWorkerError(
          "worker_send_failed",
          `generation worker ${method} could not be delivered`,
          { failureType: failureType(error) },
        ));
      });
    });
  }

  #close(error) {
    if (this.closed) return;
    this.closed = true;
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timeout);
      pending.reject(error);
    }
    this.pending.clear();
    this.emit("closed", error);
  }

  #terminate() {
    if (this.child.connected) this.child.disconnect();
    if (this.child.exitCode === null && this.child.signalCode === null) this.child.kill();
  }

  async validate() {
    const result = await this.#request("generation.validate", {});
    return Object.freeze({ valid: result?.valid === true });
  }

  async activate() {
    const result = await this.#request("generation.activate", {});
    if (result?.active !== true) {
      throw new ExecutableGenerationWorkerError(
        "activation_refused",
        "generation worker did not acknowledge activation",
      );
    }
  }

  dispatch(operation, payload, effect = null) {
    if (effect !== null && typeof effect !== "function") {
      throw new TypeError("generation dispatch effect must be a function or null");
    }
    return this.#request("generation.dispatch", {
      operation: text(operation, "generation operation"),
      payload: structuredClone(payload),
    }, { effect, timeoutMs: this.dispatchTimeoutMs });
  }

  async dispose() {
    if (this.disposed) return;
    this.disposed = true;
    try {
      if (!this.closed) await this.#request("generation.dispose", {});
    } finally {
      this.#terminate();
    }
  }
}

export const EXECUTABLE_GENERATION_IPC_PROTOCOL = Object.freeze({
  name: PROTOCOL,
  version: PROTOCOL_VERSION,
});
