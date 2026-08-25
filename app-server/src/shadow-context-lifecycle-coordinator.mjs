import { createHash } from "node:crypto";
import {
  CONTEXT_PRESSURE_DISPOSITIONS,
} from "./context-pressure-controller.mjs";
import {
  createContextLifecycleEpisode,
} from "./context-lifecycle-episode.mjs";

export const SHADOW_CONTEXT_LIFECYCLE_SCHEDULE_SCHEMA_VERSION = 1;

const SHA_REVISION = /^sha256:[a-f0-9]{64}$/;
const DISPOSITIONS = new Set(CONTEXT_PRESSURE_DISPOSITIONS);

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function rejectUnknown(value, allowed, label) {
  const unknown = Object.keys(value).filter((field) => !allowed.includes(field)).sort();
  if (unknown.length > 0) {
    throw new TypeError(`${label} contains unsupported fields: ${unknown.join(", ")}`);
  }
}

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonical(value[key])}`
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}

function digest(value) {
  return `sha256:${createHash("sha256").update(canonical(value)).digest("hex")}`;
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

function timestamp(value, label) {
  text(value, label);
  if (Number.isNaN(Date.parse(value))) throw new TypeError(`${label} must be an ISO timestamp`);
  return value;
}

function normalizeSubject(value) {
  record(value, "shadow lifecycle subject");
  rejectUnknown(
    value,
    ["logicalRoleInstanceId", "threadId", "bindingRevision"],
    "shadow lifecycle subject",
  );
  if (!Number.isSafeInteger(value.bindingRevision) || value.bindingRevision < 1) {
    throw new TypeError("shadow lifecycle binding revision must be a positive safe integer");
  }
  return freeze({
    logicalRoleInstanceId: text(value.logicalRoleInstanceId, "shadow lifecycle logical role"),
    threadId: text(value.threadId, "shadow lifecycle thread id"),
    bindingRevision: value.bindingRevision,
  });
}

function nullableInteger(value, label) {
  if (value === null) return null;
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new TypeError(`${label} must be a non-negative safe integer or null`);
  }
  return value;
}

function normalizeContextTelemetry(value = {}) {
  record(value, "shadow lifecycle context telemetry");
  rejectUnknown(value, [
    "retainedInputTokens", "reportedContextWindowTokens", "growthSincePreviousTokens",
    "estimatedAvoidedInputTokens", "estimateMethod", "estimateRevision",
  ], "shadow lifecycle context telemetry");
  const estimate = value.estimatedAvoidedInputTokens ?? null;
  const method = value.estimateMethod ?? null;
  const estimateRevision = value.estimateRevision ?? null;
  if ((estimate === null) !== (method === null) || (estimate === null) !== (estimateRevision === null)) {
    throw new TypeError("avoided-token estimates require value, method, and revision together");
  }
  if (method !== null) text(method, "shadow lifecycle estimate method");
  if (estimateRevision !== null && !SHA_REVISION.test(estimateRevision)) {
    throw new TypeError("shadow lifecycle estimate revision must be SHA-256 bound");
  }
  return freeze({
    retainedInputTokens: nullableInteger(
      value.retainedInputTokens ?? null,
      "shadow lifecycle retained input tokens",
    ),
    reportedContextWindowTokens: nullableInteger(
      value.reportedContextWindowTokens ?? null,
      "shadow lifecycle reported context window tokens",
    ),
    growthSincePreviousTokens: nullableInteger(
      value.growthSincePreviousTokens ?? null,
      "shadow lifecycle context growth",
    ),
    estimatedAvoidedInputTokens: nullableInteger(estimate, "shadow lifecycle avoided input estimate"),
    estimateMethod: method,
    estimateRevision,
  });
}

export function validateShadowContextLifecycleSchedule(value) {
  record(value, "shadow lifecycle schedule");
  rejectUnknown(
    value,
    ["schemaVersion", "inspectAt", "publishAcceptedCheckpoint"],
    "shadow lifecycle schedule",
  );
  if (value.schemaVersion !== SHADOW_CONTEXT_LIFECYCLE_SCHEDULE_SCHEMA_VERSION) {
    throw new TypeError("unsupported shadow lifecycle schedule schema version");
  }
  if (!Array.isArray(value.inspectAt) || value.inspectAt.length === 0) {
    throw new TypeError("shadow lifecycle schedule inspectAt must be a non-empty array");
  }
  if (value.inspectAt.some((item) => !DISPOSITIONS.has(item))) {
    throw new TypeError("shadow lifecycle schedule contains an unsupported pressure disposition");
  }
  if (new Set(value.inspectAt).size !== value.inspectAt.length) {
    throw new TypeError("shadow lifecycle schedule cannot repeat a pressure disposition");
  }
  if (typeof value.publishAcceptedCheckpoint !== "boolean") {
    throw new TypeError("shadow lifecycle publishAcceptedCheckpoint must be boolean");
  }
  const schedule = {
    schemaVersion: SHADOW_CONTEXT_LIFECYCLE_SCHEDULE_SCHEMA_VERSION,
    inspectAt: [...value.inspectAt],
    publishAcceptedCheckpoint: value.publishAcceptedCheckpoint,
  };
  return freeze({ ...schedule, scheduleRevision: digest(schedule) });
}

function blankInferenceMeasurement() {
  return {
    durationMs: null,
    inputTokens: null,
    cachedInputTokens: null,
    outputTokens: null,
    costMicrounits: null,
  };
}

function elapsed(start, monotonicNow) {
  return Math.max(0, Math.round(monotonicNow() - start));
}

function sourceRevision(projection) {
  const value = projection?.sourceRevision ?? null;
  if (value !== null && !SHA_REVISION.test(value)) {
    throw new TypeError("shadow lifecycle projection source revision must be SHA-256 bound");
  }
  return value;
}

function nullableRevision(value, label) {
  if (value === null) return null;
  text(value, label);
  if (!SHA_REVISION.test(value)) throw new TypeError(`${label} must be SHA-256 bound`);
  return value;
}

function assertProjectionSubject(projection, subject) {
  record(projection, "shadow lifecycle projection");
  const observed = record(projection.observedContext, "shadow lifecycle observed context");
  const binding = record(observed.runtimeBinding, "shadow lifecycle runtime binding");
  if (observed.logicalRoleInstanceId !== subject.logicalRoleInstanceId
      || binding.threadId !== subject.threadId
      || binding.bindingRevision !== subject.bindingRevision) {
    throw new TypeError("shadow lifecycle projection does not match the requested role binding");
  }
}

function inferenceReceipt(result) {
  return {
    status: result.verification.disposition,
    candidateRevision: result.candidate.candidateRevision,
    verificationRevision: result.verification.verificationRevision,
    compiler: result.candidate.compiler,
    verifier: result.verification.verifier,
    blockerCount: result.verification.blockers.length,
    uncertaintyCount: result.verification.uncertainty.length,
  };
}

function checkpointReceipt(outcome) {
  if (outcome.status === "published") {
    return {
      status: "published",
      checkpointRevision: outcome.publication.checkpointRevision,
      ledgerRevision: outcome.ledgerEntry.entryRevision,
      reason: null,
    };
  }
  return {
    status: "rejected",
    checkpointRevision: null,
    ledgerRevision: null,
    reason: text(outcome.reason, "checkpoint publication rejection reason"),
  };
}

export class ShadowContextLifecycleCoordinator {
  constructor({
    logicalRoleInstanceId,
    pressureController,
    inferenceRuntime,
    checkpointPublisher,
    episodeStore,
    schedule,
    now = () => new Date().toISOString(),
    monotonicNow = () => performance.now(),
  }) {
    this.logicalRoleInstanceId = text(logicalRoleInstanceId, "shadow lifecycle logical role");
    if (!pressureController || typeof pressureController.observe !== "function") {
      throw new TypeError("shadow lifecycle coordinator requires a pressure controller");
    }
    if (!inferenceRuntime || typeof inferenceRuntime.inspect !== "function") {
      throw new TypeError("shadow lifecycle coordinator requires a semantic inference runtime");
    }
    if (!episodeStore || typeof episodeStore.get !== "function" || typeof episodeStore.append !== "function") {
      throw new TypeError("shadow lifecycle coordinator requires an episode store");
    }
    this.schedule = validateShadowContextLifecycleSchedule(schedule);
    if (this.schedule.publishAcceptedCheckpoint
        && (!checkpointPublisher || typeof checkpointPublisher.publish !== "function")) {
      throw new TypeError("checkpoint publication schedule requires a checkpoint publisher");
    }
    if (typeof now !== "function" || typeof monotonicNow !== "function") {
      throw new TypeError("shadow lifecycle coordinator clocks must be functions");
    }
    this.pressureController = pressureController;
    this.inferenceRuntime = inferenceRuntime;
    this.checkpointPublisher = checkpointPublisher;
    this.episodeStore = episodeStore;
    this.now = now;
    this.monotonicNow = monotonicNow;
    this.inFlight = new Map();
  }

  async observe(input = {}) {
    record(input, "shadow lifecycle observation request");
    rejectUnknown(input, [
      "episodeId", "subject", "pressureObservation", "projection", "sourceMaterials",
      "expectedPublicationRevision", "previousLedgerEntry", "contextTelemetry", "signal",
    ], "shadow lifecycle observation request");
    const episodeId = text(input.episodeId, "shadow lifecycle episode id");
    const concurrencyRevision = digest({ ...input, signal: null });
    const existing = this.inFlight.get(episodeId);
    if (existing) {
      if (existing.concurrencyRevision !== concurrencyRevision) {
        throw new TypeError("shadow lifecycle episode id is already running with different evidence");
      }
      return existing.promise;
    }
    const promise = this.#observeOnce(input).finally(() => {
      if (this.inFlight.get(episodeId)?.promise === promise) this.inFlight.delete(episodeId);
    });
    this.inFlight.set(episodeId, { concurrencyRevision, promise });
    return promise;
  }

  async #observeOnce({
    episodeId,
    subject,
    pressureObservation,
    projection = null,
    sourceMaterials = [],
    expectedPublicationRevision = null,
    previousLedgerEntry = null,
    contextTelemetry = {},
    signal,
  }) {
    const normalizedEpisodeId = text(episodeId, "shadow lifecycle episode id");
    const normalizedSubject = normalizeSubject(subject);
    if (normalizedSubject.logicalRoleInstanceId !== this.logicalRoleInstanceId) {
      throw new TypeError("shadow lifecycle subject does not match the coordinator role");
    }
    const telemetry = normalizeContextTelemetry(contextTelemetry);
    const normalizedExpectedPublicationRevision = nullableRevision(
      expectedPublicationRevision,
      "shadow lifecycle expected publication revision",
    );
    const previousLedgerRevision = nullableRevision(
      previousLedgerEntry?.entryRevision ?? null,
      "shadow lifecycle previous ledger revision",
    );
    const requestRevision = digest({
      episodeId: normalizedEpisodeId,
      subject: normalizedSubject,
      pressureObservation,
      sourceRevision: sourceRevision(projection),
      sourceMaterials,
      expectedPublicationRevision: normalizedExpectedPublicationRevision,
      previousLedgerRevision,
      contextTelemetry: telemetry,
      scheduleRevision: this.schedule.scheduleRevision,
    });
    const existing = await this.episodeStore.get(normalizedEpisodeId);
    if (existing) {
      if (existing.requestRevision !== requestRevision) {
        throw new TypeError("shadow lifecycle episode id was reused for a different request");
      }
      return freeze({ status: "replayed", episode: existing });
    }

    const startedAt = timestamp(this.now(), "shadow lifecycle episode start");
    const pressure = this.pressureController.observe(pressureObservation);
    let stage = "scheduling";
    let inference = {
      status: "not_scheduled",
      candidateRevision: null,
      verificationRevision: null,
      compiler: null,
      verifier: null,
      blockerCount: 0,
      uncertaintyCount: 0,
    };
    let checkpoint = {
      status: "not_attempted",
      checkpointRevision: null,
      ledgerRevision: null,
      reason: "inspection_not_scheduled",
    };
    let compilerMeasurement = blankInferenceMeasurement();
    let verifierMeasurement = blankInferenceMeasurement();
    let inspectionDurationMs = null;
    let publicationDurationMs = null;
    let checkpointBytes = null;
    let failure = null;

    try {
      if (projection !== null) {
        stage = "projection_validation";
        assertProjectionSubject(projection, normalizedSubject);
      }
      if (this.schedule.inspectAt.includes(pressure.disposition)) {
        if (projection === null) {
          stage = "projection_validation";
          assertProjectionSubject(projection, normalizedSubject);
        }
        stage = "inference";
        const inspectionStarted = this.monotonicNow();
        const result = await this.inferenceRuntime.inspect({
          projection,
          sourceMaterials,
          signal,
        });
        inspectionDurationMs = elapsed(inspectionStarted, this.monotonicNow);
        inference = inferenceReceipt(result);
        compilerMeasurement = result.measurements?.compiler ?? compilerMeasurement;
        verifierMeasurement = result.measurements?.verifier ?? verifierMeasurement;
        if (inference.status === "accepted" && this.schedule.publishAcceptedCheckpoint) {
          stage = "checkpoint_publication";
          const publicationStarted = this.monotonicNow();
          const outcome = await this.checkpointPublisher.publish({
            projection,
            candidate: result.candidate,
            verification: result.verification,
            expectedPublicationRevision: normalizedExpectedPublicationRevision,
            previousLedgerEntry,
          });
          publicationDurationMs = elapsed(publicationStarted, this.monotonicNow);
          checkpoint = checkpointReceipt(outcome);
          if (outcome.status === "published") {
            checkpointBytes = Buffer.byteLength(JSON.stringify(outcome.publication), "utf8");
          }
        } else {
          checkpoint = {
            status: "not_attempted",
            checkpointRevision: null,
            ledgerRevision: null,
            reason: inference.status !== "accepted"
              ? "verification_not_accepted"
              : "shadow_publication_disabled",
          };
        }
      }
    } catch (error) {
      failure = {
        stage,
        name: error?.name ?? "Error",
        message: error?.message ?? String(error),
      };
      if (stage === "checkpoint_publication") {
        checkpoint = {
          status: "failed",
          checkpointRevision: null,
          ledgerRevision: null,
          reason: "checkpoint_publication_failed",
        };
      } else {
        inference = {
          status: "failed",
          candidateRevision: null,
          verificationRevision: null,
          compiler: null,
          verifier: null,
          blockerCount: 0,
          uncertaintyCount: 0,
        };
        checkpoint = {
          status: "not_attempted",
          checkpointRevision: null,
          ledgerRevision: null,
          reason: "inference_failed",
        };
      }
    }

    const episode = createContextLifecycleEpisode({
      schemaVersion: 1,
      type: "context-lifecycle-shadow-episode",
      episodeId: normalizedEpisodeId,
      requestRevision,
      scheduleRevision: this.schedule.scheduleRevision,
      mode: "shadow",
      subject: normalizedSubject,
      startedAt,
      completedAt: timestamp(this.now(), "shadow lifecycle episode completion"),
      pressure: {
        policyRevision: pressure.policyRevision,
        observationId: pressure.observation.observationId,
        observationSequence: pressure.observation.sequence,
        pressureBasisPoints: pressure.observation.pressureBasisPoints,
        previousDisposition: pressure.previousDisposition,
        disposition: pressure.disposition,
        transitionReason: pressure.reason,
        measurementSource: pressure.observation.source,
        measurementSourceRevision: pressure.observation.sourceRevision,
      },
      sourceRevision: sourceRevision(projection),
      inference,
      checkpoint,
      measurements: {
        context: telemetry,
        compiler: compilerMeasurement,
        verifier: verifierMeasurement,
        inspectionDurationMs,
        publicationDurationMs,
        checkpointBytes,
      },
      transition: { status: "not_requested", retirementAttempted: false },
      failure,
    });
    const stored = await this.episodeStore.append(episode);
    return freeze({
      status: failure ? "failed" : stored.status === "replayed" ? "replayed" : "recorded",
      episode: stored.episode,
    });
  }
}
