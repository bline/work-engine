import { randomUUID } from "node:crypto";
import { mkdir, readFile, rename, rmdir, writeFile } from "node:fs/promises";
import path from "node:path";

const SCHEMA_VERSION = 2;

const TRANSITIONS = new Map([
  ["requested", new Set(["draining"])],
  ["draining", new Set(["snapshotting", "shutdown_interrupted"])],
  ["snapshotting", new Set(["building", "snapshot_invalid"])],
  ["building", new Set(["validating", "build_failed"])],
  ["validating", new Set([
    "ready", "validation_failed", "environment_migration_required",
    "bootstrap_restart_required",
  ])],
  ["ready", new Set(["active_unexercised", "activation_failed"])],
  ["active_unexercised", new Set(["active_exercised"])],
]);

function text(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function clone(value) {
  return structuredClone(value);
}

function blankState() {
  return {
    schemaVersion: SCHEMA_VERSION,
    revision: 0,
    activeGeneration: null,
    reloads: {},
    startupReconciliations: [],
  };
}

function migrateState(value) {
  if (value?.schemaVersion === 1) {
    return { ...value, schemaVersion: SCHEMA_VERSION, startupReconciliations: [] };
  }
  return value;
}

function validateState(input) {
  const value = migrateState(input);
  if (!value || value.schemaVersion !== SCHEMA_VERSION
      || !Number.isSafeInteger(value.revision) || value.revision < 0
      || typeof value.reloads !== "object" || value.reloads === null
      || !Array.isArray(value.startupReconciliations)) {
    throw new TypeError("invalid executable generation store state");
  }
  return value;
}

function sameGenerationIdentity(left, right) {
  return left.generationId === right.generationId
    && left.sourceDigest === right.sourceDigest
    && left.environmentFingerprint === right.environmentFingerprint
    && left.bootstrapFingerprint === right.bootstrapFingerprint;
}

export class ExecutableGenerationConflictError extends Error {
  constructor(message) {
    super(message);
    this.name = "ExecutableGenerationConflictError";
  }
}

export class FileExecutableGenerationStore {
  #writeTail = Promise.resolve();

  constructor(filePath, { now = () => new Date().toISOString() } = {}) {
    this.filePath = path.resolve(filePath);
    this.now = now;
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
        throw new ExecutableGenerationConflictError(
          "executable generation store has another active writer",
        );
      }
      throw error;
    }
    try {
      return await operation();
    } finally {
      await rmdir(lockPath);
    }
  }

  #write(operation) {
    const result = this.#writeTail.then(() => this.#withWriteLock(operation));
    this.#writeTail = result.catch(() => {});
    return result;
  }

  read() {
    return this.#readState().then(clone);
  }

  initializeActive(generation) {
    text(generation?.generationId, "active generation id");
    return this.#write(async () => {
      const state = await this.#readState();
      if (state.activeGeneration) {
        if (!sameGenerationIdentity(state.activeGeneration, generation)) {
          throw new ExecutableGenerationConflictError(
            "durable active generation differs from the runtime generation",
          );
        }
        return clone(state.activeGeneration);
      }
      const activeGeneration = { ...clone(generation), activatedByReloadId: null };
      await this.#writeState({
        ...state,
        revision: state.revision + 1,
        activeGeneration,
      });
      return clone(activeGeneration);
    });
  }

  activateStartupCandidate({ expectedActiveGenerationId, successor }) {
    text(expectedActiveGenerationId, "startup predecessor generation id");
    text(successor?.generationId, "startup successor generation id");
    return this.#write(async () => {
      const state = await this.#readState();
      const predecessor = state.activeGeneration;
      if (!predecessor || predecessor.generationId !== expectedActiveGenerationId) {
        throw new ExecutableGenerationConflictError(
          "active generation changed during startup reconciliation",
        );
      }
      if (predecessor.environmentFingerprint !== successor.environmentFingerprint) {
        throw new ExecutableGenerationConflictError(
          "startup candidate requires an environment migration",
        );
      }
      const recordedAt = this.now();
      const reconciliationId = `startup-${randomUUID()}`;
      const outcome = predecessor.bootstrapFingerprint === successor.bootstrapFingerprint
        ? "implementation_compatible"
        : "bootstrap_restart_completed";
      const activeGeneration = {
        ...clone(successor),
        activatedByReloadId: null,
        activatedByStartupReconciliationId: reconciliationId,
        activatedAt: recordedAt,
      };
      const reconciliation = {
        reconciliationId,
        predecessorGenerationId: predecessor.generationId,
        predecessorSourceDigest: predecessor.sourceDigest,
        predecessorBootstrapFingerprint: predecessor.bootstrapFingerprint,
        successorGenerationId: successor.generationId,
        successorSourceDigest: successor.sourceDigest,
        environmentFingerprint: successor.environmentFingerprint,
        successorBootstrapFingerprint: successor.bootstrapFingerprint,
        outcome,
        recordedAt,
      };
      await this.#writeState({
        ...state,
        revision: state.revision + 1,
        activeGeneration,
        startupReconciliations: [...state.startupReconciliations, reconciliation],
      });
      return clone(reconciliation);
    });
  }

  beginReload({ reloadId, requestedByTurnId, predecessorGenerationId }) {
    text(reloadId, "reload id");
    text(requestedByTurnId, "requesting turn id");
    text(predecessorGenerationId, "predecessor generation id");
    return this.#write(async () => {
      const state = await this.#readState();
      if (state.reloads[reloadId]) {
        throw new ExecutableGenerationConflictError(`reload already exists: ${reloadId}`);
      }
      if (state.activeGeneration?.generationId !== predecessorGenerationId) {
        throw new ExecutableGenerationConflictError("reload predecessor is not active");
      }
      const recordedAt = this.now();
      const reload = {
        reloadId,
        requestedByTurnId,
        predecessorGenerationId,
        successorGenerationId: null,
        status: "requested",
        outcome: null,
        requestedAt: recordedAt,
        updatedAt: recordedAt,
        acceptedTurnId: null,
        producedEffects: [],
        predecessorRetirement: null,
        history: [{ status: "requested", recordedAt, details: {} }],
      };
      await this.#writeState({
        ...state,
        revision: state.revision + 1,
        reloads: { ...state.reloads, [reloadId]: reload },
      });
      return clone(reload);
    });
  }

  transition({ reloadId, expectedStatus, status, details = {}, outcome = null }) {
    text(reloadId, "reload id");
    if (!TRANSITIONS.get(expectedStatus)?.has(status)) {
      throw new TypeError(`invalid executable generation transition: ${expectedStatus} -> ${status}`);
    }
    return this.#write(async () => {
      const state = await this.#readState();
      const reload = state.reloads[reloadId];
      if (!reload || reload.status !== expectedStatus) {
        throw new ExecutableGenerationConflictError(
          `reload ${reloadId} is not in expected state ${expectedStatus}`,
        );
      }
      const recordedAt = this.now();
      const nextReload = {
        ...reload,
        status,
        outcome,
        updatedAt: recordedAt,
        history: [...reload.history, { status, recordedAt, details: clone(details) }],
      };
      await this.#writeState({
        ...state,
        revision: state.revision + 1,
        reloads: { ...state.reloads, [reloadId]: nextReload },
      });
      return clone(nextReload);
    });
  }

  activate({ reloadId, expectedActiveGenerationId, successor, details = {} }) {
    text(successor?.generationId, "successor generation id");
    return this.#write(async () => {
      const state = await this.#readState();
      const reload = state.reloads[reloadId];
      if (!reload || reload.status !== "ready") {
        throw new ExecutableGenerationConflictError("reload is not ready for activation");
      }
      if (state.activeGeneration?.generationId !== expectedActiveGenerationId) {
        throw new ExecutableGenerationConflictError("active generation changed before activation");
      }
      const recordedAt = this.now();
      const activeGeneration = {
        ...clone(successor),
        activatedByReloadId: reloadId,
        activatedAt: recordedAt,
      };
      const nextReload = {
        ...reload,
        successorGenerationId: successor.generationId,
        status: "active_unexercised",
        outcome: "implementation_compatible",
        updatedAt: recordedAt,
        history: [...reload.history, {
          status: "active_unexercised",
          recordedAt,
          details: clone(details),
        }],
      };
      await this.#writeState({
        ...state,
        revision: state.revision + 1,
        activeGeneration,
        reloads: { ...state.reloads, [reloadId]: nextReload },
      });
      return clone(nextReload);
    });
  }

  markExercised({ generationId, turnId }) {
    return this.#write(async () => {
      const state = await this.#readState();
      if (state.activeGeneration?.generationId !== generationId) {
        throw new ExecutableGenerationConflictError("only the active generation can be exercised");
      }
      const reloadId = state.activeGeneration.activatedByReloadId;
      if (!reloadId) return null;
      const reload = state.reloads[reloadId];
      if (!["active_unexercised", "active_exercised"].includes(reload.status)) {
        throw new ExecutableGenerationConflictError("active reload cannot accept a turn");
      }
      if (reload.acceptedTurnId) return clone(reload);
      const recordedAt = this.now();
      const nextReload = {
        ...reload,
        status: "active_exercised",
        acceptedTurnId: text(turnId, "accepted turn id"),
        updatedAt: recordedAt,
        history: [...reload.history, {
          status: "active_exercised",
          recordedAt,
          details: { turnId },
        }],
      };
      await this.#writeState({
        ...state,
        revision: state.revision + 1,
        reloads: { ...state.reloads, [reloadId]: nextReload },
      });
      return clone(nextReload);
    });
  }

  recordEffect({ generationId, effectId }) {
    return this.#write(async () => {
      const state = await this.#readState();
      if (state.activeGeneration?.generationId !== generationId) {
        throw new ExecutableGenerationConflictError("effect generation is not active");
      }
      const reloadId = state.activeGeneration.activatedByReloadId;
      if (!reloadId) return null;
      const reload = state.reloads[reloadId];
      if (reload.status !== "active_exercised") {
        throw new ExecutableGenerationConflictError("generation has not accepted a turn");
      }
      const normalizedEffectId = text(effectId, "effect id");
      if (reload.producedEffects.includes(normalizedEffectId)) return clone(reload);
      const recordedAt = this.now();
      const nextReload = {
        ...reload,
        producedEffects: [...reload.producedEffects, normalizedEffectId],
        updatedAt: recordedAt,
        history: [...reload.history, {
          status: "effect_recorded",
          recordedAt,
          details: { effectId: normalizedEffectId },
        }],
      };
      await this.#writeState({
        ...state,
        revision: state.revision + 1,
        reloads: { ...state.reloads, [reloadId]: nextReload },
      });
      return clone(nextReload);
    });
  }

  recordPredecessorRetirement({ reloadId, status, failureType = null }) {
    if (!["retired", "retirement_failed"].includes(status)) {
      throw new TypeError("invalid predecessor retirement status");
    }
    return this.#write(async () => {
      const state = await this.#readState();
      const reload = state.reloads[reloadId];
      if (!reload || !["active_unexercised", "active_exercised"].includes(reload.status)) {
        throw new ExecutableGenerationConflictError("reload successor is not active");
      }
      const recordedAt = this.now();
      const nextReload = {
        ...reload,
        predecessorRetirement: { status, failureType, recordedAt },
        updatedAt: recordedAt,
        history: [...reload.history, {
          status: `predecessor_${status}`,
          recordedAt,
          details: { failureType },
        }],
      };
      await this.#writeState({
        ...state,
        revision: state.revision + 1,
        reloads: { ...state.reloads, [reloadId]: nextReload },
      });
      return clone(nextReload);
    });
  }
}
