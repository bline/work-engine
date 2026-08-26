import { createHash } from "node:crypto";

function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

function lifecycleIdentity(episodeId, phase) {
  return `context-lifecycle:${createHash("sha256")
    .update(`${episodeId}\0${phase}`)
    .digest("hex")}`;
}

export class LiveContextLifecycleCoordinator {
  constructor({
    transitionRuntime,
    inferenceRuntime,
    checkpointPublisher,
    projectionForPreparation,
    transitionAt = ["replacement_candidate", "critical"],
  }) {
    if (!transitionRuntime || typeof transitionRuntime.beginPreparation !== "function"
        || typeof transitionRuntime.attestContextWindow !== "function"
        || typeof transitionRuntime.promotePreparation !== "function"
        || typeof transitionRuntime.retireAndReconcile !== "function") {
      throw new TypeError("live lifecycle coordinator requires a complete transition runtime");
    }
    if (!inferenceRuntime || typeof inferenceRuntime.inspect !== "function") {
      throw new TypeError("live lifecycle coordinator requires semantic inference");
    }
    if (!checkpointPublisher || typeof checkpointPublisher.publish !== "function") {
      throw new TypeError("live lifecycle coordinator requires checkpoint publication");
    }
    if (typeof projectionForPreparation !== "function") {
      throw new TypeError("live lifecycle coordinator requires a projection resolver");
    }
    if (!Array.isArray(transitionAt) || transitionAt.length === 0
        || transitionAt.some((value) => ![
          "comfortable", "approaching", "replacement_candidate", "critical",
        ].includes(value))) {
      throw new TypeError("live lifecycle transition dispositions are invalid");
    }
    this.transitionRuntime = transitionRuntime;
    this.inferenceRuntime = inferenceRuntime;
    this.checkpointPublisher = checkpointPublisher;
    this.projectionForPreparation = projectionForPreparation;
    this.transitionAt = new Set(transitionAt);
    this.inFlight = new Map();
  }

  run(input) {
    record(input, "live lifecycle request");
    const episodeId = text(input.episodeId, "live lifecycle episode id");
    const existing = this.inFlight.get(episodeId);
    if (existing) return existing;
    const operation = this.#run({ ...input, episodeId }).finally(() => {
      if (this.inFlight.get(episodeId) === operation) this.inFlight.delete(episodeId);
    });
    this.inFlight.set(episodeId, operation);
    return operation;
  }

  async #run({
    episodeId,
    subject,
    pressureDisposition,
    role,
    skills = [],
    projectionContext = null,
    signal,
  }) {
    record(subject, "live lifecycle subject");
    record(role, "live lifecycle role");
    text(subject.logicalRoleInstanceId, "live lifecycle role identity");
    text(subject.threadId, "live lifecycle thread");
    if (!Number.isSafeInteger(subject.bindingRevision) || subject.bindingRevision < 1) {
      throw new TypeError("live lifecycle binding revision must be positive");
    }
    if (role.logicalRoleInstanceId !== subject.logicalRoleInstanceId) {
      throw new TypeError("live lifecycle role does not match its subject");
    }
    if (!Array.isArray(skills)) throw new TypeError("live lifecycle skills must be an array");
    if (projectionContext !== null) record(projectionContext, "live lifecycle projection context");
    if (!this.transitionAt.has(pressureDisposition)) {
      return freeze({
        status: "not_scheduled",
        episodeId,
        pressureDisposition,
      });
    }
    signal?.throwIfAborted();
    const prepared = await this.transitionRuntime.beginPreparation(subject);
    const attestation = await this.transitionRuntime.attestContextWindow({
      role,
      preparation: prepared.preparation,
      clientUserMessageId: lifecycleIdentity(episodeId, "identity"),
      signal,
    });
    if (attestation.validation.status !== "accepted") {
      return freeze({
        status: "stopped",
        phase: "identity_attestation",
        episodeId,
        preparation: prepared.preparation,
        attestation,
      });
    }
    const projected = await this.projectionForPreparation({
      episodeId,
      subject: freeze({ ...subject }),
      role,
      skills,
      preparation: prepared.preparation,
      attestation,
      projectionContext,
      signal,
    });
    record(projected, "live lifecycle projection result");
    record(projected.projection, "live lifecycle projection");
    if (!Array.isArray(projected.sourceMaterials)) {
      throw new TypeError("live lifecycle projection source materials must be an array");
    }
    const inspection = await this.inferenceRuntime.inspect({
      projection: projected.projection,
      sourceMaterials: projected.sourceMaterials,
      signal,
    });
    if (inspection.verification?.disposition !== "accepted") {
      return freeze({
        status: "stopped",
        phase: "semantic_verification",
        episodeId,
        preparation: prepared.preparation,
        attestation,
        inspection,
      });
    }
    const publication = await this.checkpointPublisher.publish({
      projection: projected.projection,
      candidate: inspection.candidate,
      verification: inspection.verification,
      expectedPublicationRevision: projected.expectedPublicationRevision ?? null,
      previousLedgerEntry: projected.previousLedgerEntry ?? null,
    });
    if (publication.status !== "published") {
      return freeze({
        status: "stopped",
        phase: "checkpoint_publication",
        episodeId,
        preparation: prepared.preparation,
        attestation,
        inspection,
        publication,
      });
    }
    const promoted = await this.transitionRuntime.promotePreparation({
      preparation: prepared.preparation,
      publication: publication.publication,
      ledgerEntry: publication.ledgerEntry,
      previousLedgerEntry: projected.previousLedgerEntry ?? null,
      expectedFence: {
        ...publication.currentFence,
        predecessorContextWindowId:
          attestation.validation.receipt.current_context_window_id,
      },
    });
    const transition = await this.transitionRuntime.retireAndReconcile({
      role,
      lease: promoted.lease,
      retirementClientUserMessageId: lifecycleIdentity(episodeId, "retirement"),
      rehydrationClientUserMessageId: lifecycleIdentity(episodeId, "rehydration"),
      receiptNonce: lifecycleIdentity(episodeId, "receipt"),
      skills,
      signal,
    });
    return freeze({
      status: transition.reconciliation.status === "reconciled"
        ? "reconciled"
        : "unreconciled",
      episodeId,
      preparation: prepared.preparation,
      attestation,
      inspection,
      publication,
      promoted,
      transition,
    });
  }
}
