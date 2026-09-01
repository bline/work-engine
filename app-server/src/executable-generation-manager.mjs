import { randomUUID } from "node:crypto";

const TERMINAL_FAILURES = new Set([
  "snapshot_invalid",
  "build_failed",
  "validation_failed",
  "environment_migration_required",
  "bootstrap_restart_required",
  "activation_failed",
  "shutdown_interrupted",
]);

function text(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function generationRecord(generation) {
  return {
    generationId: text(generation?.generationId, "generation id"),
    sourceDigest: text(generation?.sourceDigest, "generation source digest"),
    environmentFingerprint: text(
      generation?.environmentFingerprint,
      "generation environment fingerprint",
    ),
    bootstrapFingerprint: text(
      generation?.bootstrapFingerprint,
      "generation bootstrap fingerprint",
    ),
    ...(generation?.extensionAttachment ? {
      extensionAttachment: structuredClone(generation.extensionAttachment),
      baseSourceDigest: text(generation.baseSourceDigest, "generation base source digest"),
    } : {}),
  };
}

function runtimeGeneration(generation, record, activatedByReloadId) {
  const bind = (name) => typeof generation?.[name] === "function"
    ? generation[name].bind(generation)
    : undefined;
  return {
    ...record,
    activatedByReloadId,
    validate: bind("validate"),
    activate: bind("activate"),
    dispatch: bind("dispatch"),
    dispose: bind("dispose"),
  };
}

function freeze(value) {
  return Object.freeze(value);
}

async function disposeCandidate(candidate) {
  try {
    await candidate?.dispose?.();
    return null;
  } catch (error) {
    return error;
  }
}

export class ExecutableGenerationAdmissionError extends Error {
  constructor(code, message, details = {}) {
    super(message);
    this.name = "ExecutableGenerationAdmissionError";
    this.code = code;
    this.details = freeze({ ...details });
  }
}

export class InMemoryReplaceableSubstrateArbiter {
  constructor() {
    this.holder = null;
  }

  acquire({ kind, subject }) {
    text(kind, "replaceable substrate kind");
    text(subject, "replaceable substrate subject");
    if (this.holder) {
      throw new ExecutableGenerationAdmissionError(
        "lifecycle_transition_in_flight",
        `replaceable substrate transition is already held by ${this.holder.kind}`,
        { holder: { ...this.holder } },
      );
    }
    const token = freeze({ tokenId: randomUUID(), kind, subject });
    this.holder = token;
    return token;
  }

  release(token) {
    if (!this.holder || token?.tokenId !== this.holder.tokenId) {
      throw new ExecutableGenerationAdmissionError(
        "invalid_lifecycle_token",
        "replaceable substrate transition release requires the active token",
      );
    }
    this.holder = null;
  }

  snapshot() {
    return this.holder ? freeze({ ...this.holder }) : null;
  }
}

export class ExecutableGenerationManager {
  static async create({
    activeGeneration,
    store,
    snapshotter,
    candidateBuilder,
    substrateArbiter,
    reloadIdFactory = () => `reload-${randomUUID()}`,
  }) {
    const record = generationRecord(activeGeneration);
    if (!store || typeof store.initializeActive !== "function"
        || typeof store.beginReload !== "function") {
      throw new TypeError("executable generation manager requires a durable store");
    }
    if (typeof snapshotter !== "function" || typeof candidateBuilder !== "function") {
      throw new TypeError("executable generation manager requires snapshot and build functions");
    }
    if (!substrateArbiter || typeof substrateArbiter.acquire !== "function"
        || typeof substrateArbiter.release !== "function") {
      throw new TypeError("executable generation manager requires a substrate arbiter");
    }
    await store.initializeActive(record);
    return new ExecutableGenerationManager({
      activeGeneration: runtimeGeneration(activeGeneration, record, null),
      store,
      snapshotter,
      candidateBuilder,
      substrateArbiter,
      reloadIdFactory,
    });
  }

  constructor({
    activeGeneration,
    store,
    snapshotter,
    candidateBuilder,
    substrateArbiter,
    reloadIdFactory,
  }) {
    this.activeGeneration = activeGeneration;
    this.store = store;
    this.snapshotter = snapshotter;
    this.candidateBuilder = candidateBuilder;
    this.substrateArbiter = substrateArbiter;
    this.reloadIdFactory = reloadIdFactory;
    this.admissions = new Map();
    this.reload = null;
  }

  openAdmission({ kind, id, subjectId = id }) {
    text(kind, "admission kind");
    text(id, "admission id");
    text(subjectId, "admission subject id");
    if (this.reload) {
      throw new ExecutableGenerationAdmissionError(
        "reload_fence_active",
        "new generation-bound work is unavailable while executable reload is fenced",
        { reloadId: this.reload.reloadId },
      );
    }
    if (this.admissions.has(id)) {
      throw new ExecutableGenerationAdmissionError(
        "duplicate_admission",
        `generation-bound admission already exists: ${id}`,
      );
    }
    const generation = this.activeGeneration;
    const token = freeze({
      tokenId: randomUUID(),
      kind,
      id,
      subjectId,
      generationId: generation.generationId,
    });
    this.admissions.set(id, token);
    return token;
  }

  closeAdmission(token) {
    const admission = this.admissions.get(token?.id);
    if (!admission || admission.tokenId !== token?.tokenId) {
      throw new ExecutableGenerationAdmissionError(
        "invalid_admission_token",
        "generation-bound admission close requires the active token",
      );
    }
    this.admissions.delete(admission.id);
    const reloadCompletion = this.reload?.completion ?? null;
    this.#advanceIfDrained();
    return reloadCompletion;
  }

  async markTurnAdmissionExercised(token) {
    const admission = this.admissions.get(token?.id);
    if (!admission || admission.tokenId !== token?.tokenId || admission.kind !== "turn") {
      throw new ExecutableGenerationAdmissionError(
        "invalid_turn_admission",
        "successor exercise requires an active turn admission",
      );
    }
    if (!this.activeGeneration.activatedByReloadId) return null;
    return this.store.markExercised({
      generationId: admission.generationId,
      turnId: admission.subjectId,
    });
  }

  async runInAdmission({ kind, subjectId }, operation) {
    text(kind, "admission kind");
    text(subjectId, "admission subject id");
    if (typeof operation !== "function") throw new TypeError("admission requires an operation");
    const admission = [...this.admissions.values()].find((candidate) =>
      candidate.kind === kind && candidate.subjectId === subjectId
    );
    if (!admission || admission.generationId !== this.activeGeneration.generationId) {
      throw new ExecutableGenerationAdmissionError(
        "admission_not_active",
        "generation-bound operation requires its active admitted subject",
        { kind, subjectId },
      );
    }
    return operation(this.activeGeneration, admission);
  }

  async runDuringActiveAdmissions(operation) {
    if (typeof operation !== "function") throw new TypeError("callback admission requires an operation");
    if (this.admissions.size === 0) {
      throw new ExecutableGenerationAdmissionError(
        "callback_without_active_work",
        "generation callback requires active generation-bound work",
      );
    }
    return operation(this.activeGeneration);
  }

  async runAdmission({ kind, id, subjectId = id }, operation) {
    if (typeof operation !== "function") throw new TypeError("admission requires an operation");
    const admission = this.openAdmission({ kind, id, subjectId });
    const generation = this.activeGeneration;
    try {
      if (kind === "turn" && generation.activatedByReloadId) {
        await this.markTurnAdmissionExercised(admission);
      }
      return await operation(generation);
    } finally {
      this.closeAdmission(admission);
    }
  }

  async requestReload({ requestedByTurnId, source }) {
    text(requestedByTurnId, "requesting turn id");
    const admission = [...this.admissions.values()].find((candidate) =>
      candidate.kind === "turn" && candidate.subjectId === requestedByTurnId
    );
    if (!admission || admission.kind !== "turn"
        || admission.generationId !== this.activeGeneration.generationId) {
      throw new ExecutableGenerationAdmissionError(
        "reload_request_outside_turn",
        "executable reload must be requested by an admitted active-generation turn",
      );
    }
    if (this.reload) {
      throw new ExecutableGenerationAdmissionError(
        "reload_already_requested",
        "an executable reload request is already active",
        { reloadId: this.reload.reloadId },
      );
    }
    const reloadId = text(this.reloadIdFactory(), "reload id");
    const lifecycleToken = this.substrateArbiter.acquire({
      kind: "executable_generation",
      subject: reloadId,
    });
    let resolveCompletion;
    const completion = new Promise((resolve) => { resolveCompletion = resolve; });
    const context = {
      reloadId,
      requestedByTurnId,
      source: structuredClone(source),
      predecessor: this.activeGeneration,
      lifecycleToken,
      completion,
      resolveCompletion,
      advancing: false,
      phase: "admitting",
    };
    // The in-memory fence must exist before the first durable write yields.
    this.reload = context;
    try {
      await this.store.beginReload({
        reloadId,
        requestedByTurnId,
        predecessorGenerationId: this.activeGeneration.generationId,
      });
      await this.store.transition({
        reloadId,
        expectedStatus: "requested",
        status: "draining",
        details: { activeAdmissions: this.admissions.size },
      });
      context.phase = "draining";
      this.#advanceIfDrained();
      return freeze({ status: "staged", reloadId, activation: "after_global_quiescence", completion });
    } catch (error) {
      if (this.reload?.reloadId === reloadId) this.reload = null;
      this.substrateArbiter.release(lifecycleToken);
      throw error;
    }
  }

  #advanceIfDrained() {
    if (!this.reload || this.reload.advancing || this.admissions.size !== 0) return;
    this.reload.advancing = true;
    this.#advance(this.reload).catch((error) => {
      // A receipt-store failure makes safe recovery unknowable. Keep the
      // admission fence and substrate token held, but resolve the observer so
      // the host can surface the fail-closed condition instead of hanging.
      this.reload.phase = "manager_failed";
      this.reload.failureType = error instanceof Error ? error.name : typeof error;
      this.reload.resolveCompletion(freeze({
        status: "manager_failed",
        outcome: "candidate_invalid",
        reloadId: this.reload.reloadId,
        failureType: this.reload.failureType,
        admission: "closed",
      }));
    });
  }

  async #fail(context, expectedStatus, status, error, outcome = "candidate_invalid") {
    await this.store.transition({
      reloadId: context.reloadId,
      expectedStatus,
      status,
      outcome,
      details: { failureType: error instanceof Error ? error.name : typeof error },
    });
    return this.#finish(context, freeze({
      status,
      outcome,
      reloadId: context.reloadId,
      failureType: error instanceof Error ? error.name : typeof error,
    }));
  }

  #reopen(context) {
    if (this.reload?.reloadId === context.reloadId) this.reload = null;
    this.substrateArbiter.release(context.lifecycleToken);
  }

  #finish(context, result) {
    this.#reopen(context);
    context.resolveCompletion(result);
    return result;
  }

  async #advance(context) {
    let snapshot;
    await this.store.transition({
      reloadId: context.reloadId,
      expectedStatus: "draining",
      status: "snapshotting",
    });
    try {
      snapshot = await this.snapshotter(context.source);
    } catch (error) {
      return this.#fail(context, "snapshotting", "snapshot_invalid", error);
    }
    if (snapshot.extensionAttachment) {
      await this.store.bindExtension({
        reloadId: context.reloadId,
        attachment: snapshot.extensionAttachment,
        baseSourceDigest: snapshot.baseSourceDigest,
      });
    }
    await this.store.transition({
      reloadId: context.reloadId,
      expectedStatus: "snapshotting",
      status: "building",
      details: { snapshotId: snapshot.snapshotId, sourceDigest: snapshot.sourceDigest },
    });
    let candidate;
    try {
      candidate = await this.candidateBuilder(snapshot);
      generationRecord(candidate);
    } catch (error) {
      return this.#fail(context, "building", "build_failed", error);
    }
    await this.store.transition({
      reloadId: context.reloadId,
      expectedStatus: "building",
      status: "validating",
      details: { candidateGenerationId: candidate.generationId },
    });
    let validation;
    try {
      validation = await candidate.validate();
      if (validation?.valid !== true) throw new Error("candidate validation did not accept");
    } catch (error) {
      await disposeCandidate(candidate);
      return this.#fail(context, "validating", "validation_failed", error);
    }

    const predecessor = generationRecord(context.predecessor);
    const successor = generationRecord(candidate);
    if (successor.bootstrapFingerprint !== predecessor.bootstrapFingerprint) {
      const disposalFailure = await disposeCandidate(candidate);
      await this.store.transition({
        reloadId: context.reloadId,
        expectedStatus: "validating",
        status: "bootstrap_restart_required",
        outcome: "bootstrap_restart_required",
        details: {
          candidateGenerationId: successor.generationId,
          disposalFailureType: disposalFailure instanceof Error ? disposalFailure.name : null,
        },
      });
      return this.#finish(context, freeze({
        status: "bootstrap_restart_required",
        outcome: "bootstrap_restart_required",
        reloadId: context.reloadId,
      }));
    }
    if (successor.environmentFingerprint !== predecessor.environmentFingerprint) {
      const disposalFailure = await disposeCandidate(candidate);
      await this.store.transition({
        reloadId: context.reloadId,
        expectedStatus: "validating",
        status: "environment_migration_required",
        outcome: "environment_migration_required",
        details: {
          candidateGenerationId: successor.generationId,
          disposalFailureType: disposalFailure instanceof Error ? disposalFailure.name : null,
        },
      });
      return this.#finish(context, freeze({
        status: "environment_migration_required",
        outcome: "environment_migration_required",
        reloadId: context.reloadId,
      }));
    }
    await this.store.transition({
      reloadId: context.reloadId,
      expectedStatus: "validating",
      status: "ready",
      details: { candidateGenerationId: successor.generationId },
    });
    try {
      await candidate.activate();
      await this.store.activate({
        reloadId: context.reloadId,
        expectedActiveGenerationId: predecessor.generationId,
        successor: {
          ...successor, sourceDigest: snapshot.sourceDigest,
          ...(snapshot.extensionAttachment ? {
            extensionAttachment: snapshot.extensionAttachment,
            baseSourceDigest: snapshot.baseSourceDigest,
          } : {}),
        },
        details: { sourceDigest: snapshot.sourceDigest },
      });
    } catch (error) {
      await disposeCandidate(candidate);
      return this.#fail(context, "ready", "activation_failed", error);
    }
    this.activeGeneration = runtimeGeneration(candidate, {
      ...successor,
      sourceDigest: snapshot.sourceDigest,
      ...(snapshot.extensionAttachment ? {
        extensionAttachment: snapshot.extensionAttachment,
        baseSourceDigest: snapshot.baseSourceDigest,
      } : {}),
    }, context.reloadId);
    const result = {
      status: "active_unexercised",
      outcome: "implementation_compatible",
      reloadId: context.reloadId,
      generationId: successor.generationId,
      sourceDigest: snapshot.sourceDigest,
    };
    this.#reopen(context);
    let predecessorRetirement;
    try {
      await context.predecessor.dispose?.();
      await this.store.recordPredecessorRetirement({
        reloadId: context.reloadId,
        status: "retired",
      });
      predecessorRetirement = "retired";
    } catch (error) {
      try {
        await this.store.recordPredecessorRetirement({
          reloadId: context.reloadId,
          status: "retirement_failed",
          failureType: error instanceof Error ? error.name : typeof error,
        });
        predecessorRetirement = "retirement_failed";
      } catch {
        predecessorRetirement = "receipt_failed";
      }
    }
    const completed = freeze({ ...result, predecessorRetirement });
    context.resolveCompletion(completed);
    return completed;
  }

  async recordEffect({ generationId, effectId }) {
    return this.store.recordEffect({ generationId, effectId });
  }

  async recordExtensionTransition({ admissionId, state, details = {} }) {
    const admission = this.admissions.get(admissionId);
    if (!admission || admission.generationId !== this.activeGeneration.generationId) {
      throw new ExecutableGenerationAdmissionError(
        "extension_transition_outside_admission",
        "extension transition requires active generation-bound admission",
      );
    }
    if (!this.activeGeneration.extensionAttachment) {
      throw new ExecutableGenerationAdmissionError(
        "extension_not_active", "active generation has no extension attachment",
      );
    }
    return this.store.transitionExtension({
      generationId: admission.generationId, state, details,
    });
  }

  async close({ abandonActiveWork = false } = {}) {
    let shutdownReceiptError = null;
    if (!abandonActiveWork && (this.reload || this.admissions.size > 0)) {
      throw new ExecutableGenerationAdmissionError(
        "generation_manager_busy",
        "executable generation manager cannot close with active work",
      );
    }
    if (abandonActiveWork && this.reload?.advancing) {
      await this.reload.completion;
    } else if (abandonActiveWork && this.reload) {
      const context = this.reload;
      if (context.phase === "draining") {
        try {
          await this.store.transition({
            reloadId: context.reloadId,
            expectedStatus: "draining",
            status: "shutdown_interrupted",
            outcome: "candidate_invalid",
            details: { reason: "stable_bootstrap_shutdown" },
          });
        } catch (error) {
          shutdownReceiptError = error;
        }
      }
      this.reload = null;
      this.admissions.clear();
      this.substrateArbiter.release(context.lifecycleToken);
      context.resolveCompletion(freeze({
        status: "shutdown_interrupted",
        outcome: "candidate_invalid",
        reloadId: context.reloadId,
        admission: "closed",
      }));
    } else if (abandonActiveWork) {
      this.admissions.clear();
    }
    await this.activeGeneration.dispose?.();
    if (shutdownReceiptError) throw shutdownReceiptError;
  }

  snapshot() {
    return freeze({
      activeGeneration: generationRecord(this.activeGeneration),
      reload: this.reload ? freeze({
        reloadId: this.reload.reloadId,
        requestedByTurnId: this.reload.requestedByTurnId,
        phase: this.reload.phase === "manager_failed"
          ? "manager_failed"
          : this.reload.advancing ? "advancing" : this.reload.phase,
        ...(this.reload.failureType ? { failureType: this.reload.failureType } : {}),
      }) : null,
      admissions: [...this.admissions.values()].map((value) => freeze({
        kind: value.kind,
        id: value.id,
        subjectId: value.subjectId,
        generationId: value.generationId,
      })),
    });
  }
}

export function isExecutableGenerationTerminalFailure(status) {
  return TERMINAL_FAILURES.has(status);
}
