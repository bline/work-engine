import { mkdir, readFile, rename, rmdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";

const SCHEMA_VERSION = 1;

export class BindingConflictError extends Error {}

function blankState() {
  return { schemaVersion: SCHEMA_VERSION, revision: 0, bindings: {}, deliveries: {} };
}

function validateState(value) {
  if (
    !value ||
    value.schemaVersion !== SCHEMA_VERSION ||
    !Number.isInteger(value.revision) ||
    value.revision < 0 ||
    !value.bindings ||
    Array.isArray(value.bindings) ||
    typeof value.bindings !== "object"
  ) {
    throw new Error("invalid role binding registry state");
  }
  if (value.deliveries == null) value.deliveries = {};
  if (Array.isArray(value.deliveries) || typeof value.deliveries !== "object") {
    throw new Error("invalid role binding delivery state");
  }
  return value;
}

function requireIdentity(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
}

export class FileRoleBindingRegistry {
  #writeTail = Promise.resolve();

  constructor(filePath, { now = () => new Date().toISOString(), transitionGate = null } = {}) {
    this.filePath = path.resolve(filePath);
    this.now = now;
    this.setTransitionGate(transitionGate);
  }

  setTransitionGate(transitionGate) {
    if (transitionGate && typeof transitionGate.runBindingAdmission !== "function") {
      throw new TypeError("binding transition gate must provide binding admission");
    }
    if (this.transitionGate && transitionGate !== this.transitionGate) {
      throw new TypeError("role binding registry transition gate cannot be replaced");
    }
    this.transitionGate = transitionGate;
  }

  async #readState() {
    try {
      return validateState(JSON.parse(await readFile(this.filePath, "utf8")));
    } catch (error) {
      if (error?.code === "ENOENT") return blankState();
      throw error;
    }
  }

  async #writeState(state) {
    await mkdir(path.dirname(this.filePath), { recursive: true });
    const temporary = `${this.filePath}.${process.pid}.${randomUUID()}.tmp`;
    await writeFile(temporary, `${JSON.stringify(state, null, 2)}\n`, {
      encoding: "utf8",
      mode: 0o600,
    });
    await rename(temporary, this.filePath);
  }

  async #withWriteLock(operation) {
    await mkdir(path.dirname(this.filePath), { recursive: true });
    const lockPath = `${this.filePath}.lock`;
    try {
      await mkdir(lockPath);
    } catch (error) {
      if (error?.code === "EEXIST") {
        throw new BindingConflictError("role binding registry has another active writer");
      }
      throw error;
    }
    try {
      return await operation();
    } finally {
      await rmdir(lockPath);
    }
  }

  async get(logicalRoleInstanceId) {
    requireIdentity(logicalRoleInstanceId, "logicalRoleInstanceId");
    const state = await this.#readState();
    return state.bindings[logicalRoleInstanceId] ?? null;
  }

  bind(input, { transitionAdmissionPermit = null } = {}) {
    if (!this.transitionGate) return this.#bind(input);
    return this.transitionGate.runBindingAdmission({
      logicalRoleInstanceId: input?.logicalRoleInstanceId,
      admissionPermit: transitionAdmissionPermit,
    }, () => this.#bind(input));
  }

  async #bind({
    logicalRoleInstanceId,
    threadId,
    protocolVersion,
    environmentFingerprint,
    expectedBindingRevision = null,
  }) {
    requireIdentity(logicalRoleInstanceId, "logicalRoleInstanceId");
    requireIdentity(threadId, "threadId");
    requireIdentity(protocolVersion, "protocolVersion");
    requireIdentity(environmentFingerprint, "environmentFingerprint");

    const operation = this.#writeTail.then(() => this.#withWriteLock(async () => {
      const state = await this.#readState();
      const existing = state.bindings[logicalRoleInstanceId] ?? null;
      if ((existing?.bindingRevision ?? null) !== expectedBindingRevision) {
        throw new BindingConflictError(
          `role binding revision changed for ${logicalRoleInstanceId}`,
        );
      }
      const binding = {
        logicalRoleInstanceId,
        provider: "codex-app-server",
        threadId,
        protocolVersion,
        environmentFingerprint,
        bindingRevision: (existing?.bindingRevision ?? 0) + 1,
        boundAt: this.now(),
      };
      const next = {
        schemaVersion: SCHEMA_VERSION,
        revision: state.revision + 1,
        bindings: { ...state.bindings, [logicalRoleInstanceId]: binding },
        deliveries: state.deliveries,
      };
      await this.#writeState(next);
      return binding;
    }));
    this.#writeTail = operation.catch(() => {});
    return operation;
  }

  #deliveryKey(logicalRoleInstanceId, clientUserMessageId) {
    return JSON.stringify([logicalRoleInstanceId, clientUserMessageId]);
  }

  async getDelivery(logicalRoleInstanceId, clientUserMessageId) {
    requireIdentity(logicalRoleInstanceId, "logicalRoleInstanceId");
    requireIdentity(clientUserMessageId, "clientUserMessageId");
    const state = await this.#readState();
    return state.deliveries[this.#deliveryKey(logicalRoleInstanceId, clientUserMessageId)] ?? null;
  }

  async beginDelivery({
    logicalRoleInstanceId,
    clientUserMessageId,
    threadId,
    requestFingerprint,
  }) {
    requireIdentity(logicalRoleInstanceId, "logicalRoleInstanceId");
    requireIdentity(clientUserMessageId, "clientUserMessageId");
    requireIdentity(threadId, "threadId");
    requireIdentity(requestFingerprint, "requestFingerprint");
    const operation = this.#writeTail.then(() => this.#withWriteLock(async () => {
      const state = await this.#readState();
      const deliveryKey = this.#deliveryKey(logicalRoleInstanceId, clientUserMessageId);
      const existing = state.deliveries[deliveryKey] ?? null;
      if (existing) return { created: false, delivery: existing };
      const delivery = {
        logicalRoleInstanceId,
        clientUserMessageId,
        threadId,
        requestFingerprint,
        status: "pending",
        turnId: null,
        startedAt: this.now(),
        completedAt: null,
      };
      await this.#writeState({
        schemaVersion: SCHEMA_VERSION,
        revision: state.revision + 1,
        bindings: state.bindings,
        deliveries: { ...state.deliveries, [deliveryKey]: delivery },
      });
      return { created: true, delivery };
    }));
    this.#writeTail = operation.catch(() => {});
    return operation;
  }

  async completeDelivery({ logicalRoleInstanceId, clientUserMessageId, threadId, turnId }) {
    requireIdentity(logicalRoleInstanceId, "logicalRoleInstanceId");
    requireIdentity(clientUserMessageId, "clientUserMessageId");
    requireIdentity(threadId, "threadId");
    requireIdentity(turnId, "turnId");
    const operation = this.#writeTail.then(() => this.#withWriteLock(async () => {
      const state = await this.#readState();
      const deliveryKey = this.#deliveryKey(logicalRoleInstanceId, clientUserMessageId);
      const existing = state.deliveries[deliveryKey];
      if (!existing || existing.status !== "pending" || existing.threadId !== threadId) {
        throw new BindingConflictError("turn delivery is not the expected pending delivery");
      }
      const delivery = {
        ...existing,
        status: "completed",
        turnId,
        completedAt: this.now(),
      };
      await this.#writeState({
        schemaVersion: SCHEMA_VERSION,
        revision: state.revision + 1,
        bindings: state.bindings,
        deliveries: { ...state.deliveries, [deliveryKey]: delivery },
      });
      return delivery;
    }));
    this.#writeTail = operation.catch(() => {});
    return operation;
  }
}
