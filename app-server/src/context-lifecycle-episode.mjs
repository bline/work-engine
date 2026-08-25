import { createHash } from "node:crypto";

export const CONTEXT_LIFECYCLE_EPISODE_SCHEMA_VERSION = 1;
export const CONTEXT_LIFECYCLE_EPISODE_TYPE = "context-lifecycle-shadow-episode";

const SHA_REVISION = /^sha256:[a-f0-9]{64}$/;
const DISPOSITIONS = new Set([
  "comfortable", "approaching", "replacement_candidate", "critical",
]);
const INFERENCE_STATUSES = new Set([
  "not_scheduled", "accepted", "rejected", "unresolved", "failed",
]);
const CHECKPOINT_STATUSES = new Set(["not_attempted", "published", "rejected", "failed"]);

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

function nullableText(value, label) {
  return value === null ? null : text(value, label);
}

function revision(value, label) {
  text(value, label);
  if (!SHA_REVISION.test(value)) throw new TypeError(`${label} must be SHA-256 bound`);
  return value;
}

function timestamp(value, label) {
  text(value, label);
  if (Number.isNaN(Date.parse(value))) throw new TypeError(`${label} must be an ISO timestamp`);
  return value;
}

function integer(value, label, { nullable = false } = {}) {
  if (nullable && value === null) return null;
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new TypeError(`${label} must be a non-negative safe integer`);
  }
  return value;
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

function normalizeSubject(value) {
  record(value, "lifecycle episode subject");
  rejectUnknown(
    value,
    ["logicalRoleInstanceId", "threadId", "bindingRevision"],
    "lifecycle episode subject",
  );
  const bindingRevision = integer(value.bindingRevision, "lifecycle episode binding revision");
  if (bindingRevision < 1) throw new TypeError("lifecycle episode binding revision must be positive");
  return {
    logicalRoleInstanceId: text(value.logicalRoleInstanceId, "lifecycle episode logical role"),
    threadId: text(value.threadId, "lifecycle episode thread id"),
    bindingRevision,
  };
}

function normalizePressure(value) {
  record(value, "lifecycle episode pressure");
  rejectUnknown(value, [
    "policyRevision", "observationId", "observationSequence", "pressureBasisPoints",
    "previousDisposition", "disposition", "transitionReason", "measurementSource",
    "measurementSourceRevision",
  ], "lifecycle episode pressure");
  if (!DISPOSITIONS.has(value.previousDisposition) || !DISPOSITIONS.has(value.disposition)) {
    throw new TypeError("lifecycle episode pressure disposition is unsupported");
  }
  const pressureBasisPoints = integer(
    value.pressureBasisPoints,
    "lifecycle episode pressure basis points",
  );
  if (pressureBasisPoints > 10_000) {
    throw new TypeError("lifecycle episode pressure basis points cannot exceed 10000");
  }
  const observationSequence = integer(
    value.observationSequence,
    "lifecycle episode observation sequence",
  );
  if (observationSequence < 1) {
    throw new TypeError("lifecycle episode observation sequence must be positive");
  }
  return {
    policyRevision: revision(value.policyRevision, "lifecycle episode pressure policy revision"),
    observationId: text(value.observationId, "lifecycle episode pressure observation id"),
    observationSequence,
    pressureBasisPoints,
    previousDisposition: value.previousDisposition,
    disposition: value.disposition,
    transitionReason: text(value.transitionReason, "lifecycle episode pressure transition reason"),
    measurementSource: text(value.measurementSource, "lifecycle episode measurement source"),
    measurementSourceRevision: revision(
      value.measurementSourceRevision,
      "lifecycle episode measurement source revision",
    ),
  };
}

function normalizeContextMeasurements(value) {
  record(value, "lifecycle episode context measurements");
  rejectUnknown(value, [
    "retainedInputTokens", "reportedContextWindowTokens", "growthSincePreviousTokens",
    "estimatedAvoidedInputTokens", "estimateMethod", "estimateRevision",
  ], "lifecycle episode context measurements");
  const estimatedAvoidedInputTokens = integer(
    value.estimatedAvoidedInputTokens,
    "estimated avoided input tokens",
    { nullable: true },
  );
  const estimateMethod = nullableText(value.estimateMethod, "context estimate method");
  const estimateRevision = value.estimateRevision === null
    ? null
    : revision(value.estimateRevision, "context estimate revision");
  if ((estimatedAvoidedInputTokens === null) !== (estimateMethod === null)
      || (estimatedAvoidedInputTokens === null) !== (estimateRevision === null)) {
    throw new TypeError("avoided-token estimates require value, method, and revision together");
  }
  return {
    retainedInputTokens: integer(
      value.retainedInputTokens,
      "retained context input tokens",
      { nullable: true },
    ),
    reportedContextWindowTokens: integer(
      value.reportedContextWindowTokens,
      "reported context window tokens",
      { nullable: true },
    ),
    growthSincePreviousTokens: integer(
      value.growthSincePreviousTokens,
      "context growth since previous observation",
      { nullable: true },
    ),
    estimatedAvoidedInputTokens,
    estimateMethod,
    estimateRevision,
  };
}

function normalizeInferenceMeasurement(value, label) {
  record(value, label);
  rejectUnknown(
    value,
    ["durationMs", "inputTokens", "cachedInputTokens", "outputTokens", "costMicrounits"],
    label,
  );
  return {
    durationMs: integer(value.durationMs, `${label} durationMs`, { nullable: true }),
    inputTokens: integer(value.inputTokens, `${label} inputTokens`, { nullable: true }),
    cachedInputTokens: integer(value.cachedInputTokens, `${label} cachedInputTokens`, { nullable: true }),
    outputTokens: integer(value.outputTokens, `${label} outputTokens`, { nullable: true }),
    costMicrounits: integer(value.costMicrounits, `${label} costMicrounits`, { nullable: true }),
  };
}

function normalizeMeasurements(value) {
  record(value, "lifecycle episode measurements");
  rejectUnknown(
    value,
    ["context", "compiler", "verifier", "inspectionDurationMs", "publicationDurationMs", "checkpointBytes"],
    "lifecycle episode measurements",
  );
  return {
    context: normalizeContextMeasurements(value.context),
    compiler: normalizeInferenceMeasurement(value.compiler, "compiler measurement"),
    verifier: normalizeInferenceMeasurement(value.verifier, "verifier measurement"),
    inspectionDurationMs: integer(
      value.inspectionDurationMs,
      "lifecycle episode inspection duration",
      { nullable: true },
    ),
    publicationDurationMs: integer(
      value.publicationDurationMs,
      "lifecycle episode publication duration",
      { nullable: true },
    ),
    checkpointBytes: integer(
      value.checkpointBytes,
      "lifecycle episode checkpoint bytes",
      { nullable: true },
    ),
  };
}

function normalizeProvenance(value, label) {
  record(value, label);
  rejectUnknown(value, ["producer", "model", "version", "inferenceId"], label);
  return {
    producer: text(value.producer, `${label} producer`),
    model: value.model === null ? null : text(value.model, `${label} model`),
    version: text(value.version, `${label} version`),
    inferenceId: text(value.inferenceId, `${label} inference id`),
  };
}

function normalizeInference(value) {
  record(value, "lifecycle episode inference");
  rejectUnknown(value, [
    "status", "candidateRevision", "verificationRevision", "compiler", "verifier",
    "blockerCount", "uncertaintyCount",
  ], "lifecycle episode inference");
  if (!INFERENCE_STATUSES.has(value.status)) {
    throw new TypeError("lifecycle episode inference status is unsupported");
  }
  return {
    status: value.status,
    candidateRevision: value.candidateRevision === null
      ? null
      : revision(value.candidateRevision, "lifecycle episode candidate revision"),
    verificationRevision: value.verificationRevision === null
      ? null
      : revision(value.verificationRevision, "lifecycle episode verification revision"),
    compiler: value.compiler === null ? null : normalizeProvenance(value.compiler, "compiler provenance"),
    verifier: value.verifier === null ? null : normalizeProvenance(value.verifier, "verifier provenance"),
    blockerCount: integer(value.blockerCount, "lifecycle episode blocker count"),
    uncertaintyCount: integer(value.uncertaintyCount, "lifecycle episode uncertainty count"),
  };
}

function normalizeCheckpoint(value) {
  record(value, "lifecycle episode checkpoint");
  rejectUnknown(
    value,
    ["status", "checkpointRevision", "ledgerRevision", "reason"],
    "lifecycle episode checkpoint",
  );
  if (!CHECKPOINT_STATUSES.has(value.status)) {
    throw new TypeError("lifecycle episode checkpoint status is unsupported");
  }
  return {
    status: value.status,
    checkpointRevision: value.checkpointRevision === null
      ? null
      : revision(value.checkpointRevision, "lifecycle episode checkpoint revision"),
    ledgerRevision: value.ledgerRevision === null
      ? null
      : revision(value.ledgerRevision, "lifecycle episode ledger revision"),
    reason: nullableText(value.reason, "lifecycle episode checkpoint reason"),
  };
}

function normalizeFailure(value) {
  if (value === null) return null;
  record(value, "lifecycle episode failure");
  rejectUnknown(value, ["stage", "name", "message"], "lifecycle episode failure");
  return {
    stage: text(value.stage, "lifecycle episode failure stage"),
    name: text(value.name, "lifecycle episode failure name"),
    message: text(value.message, "lifecycle episode failure message"),
  };
}

function normalizeBody(value) {
  record(value, "lifecycle episode");
  rejectUnknown(value, [
    "schemaVersion", "type", "episodeId", "requestRevision", "scheduleRevision",
    "mode", "subject", "startedAt", "completedAt", "pressure", "sourceRevision",
    "inference", "checkpoint", "measurements", "transition", "failure",
  ], "lifecycle episode");
  if (value.schemaVersion !== CONTEXT_LIFECYCLE_EPISODE_SCHEMA_VERSION
      || value.type !== CONTEXT_LIFECYCLE_EPISODE_TYPE
      || value.mode !== "shadow") {
    throw new TypeError("unsupported lifecycle episode schema, type, or mode");
  }
  record(value.transition, "lifecycle episode transition");
  rejectUnknown(
    value.transition,
    ["status", "retirementAttempted"],
    "lifecycle episode transition",
  );
  if (value.transition.status !== "not_requested" || value.transition.retirementAttempted !== false) {
    throw new TypeError("shadow lifecycle episodes cannot request retirement");
  }
  const startedAt = timestamp(value.startedAt, "lifecycle episode startedAt");
  const completedAt = timestamp(value.completedAt, "lifecycle episode completedAt");
  if (Date.parse(completedAt) < Date.parse(startedAt)) {
    throw new TypeError("lifecycle episode cannot complete before it starts");
  }
  const inference = normalizeInference(value.inference);
  const checkpoint = normalizeCheckpoint(value.checkpoint);
  const failure = normalizeFailure(value.failure);
  const semanticOutcome = ["accepted", "rejected", "unresolved"].includes(inference.status);
  if (semanticOutcome && (
    inference.candidateRevision === null
    || inference.verificationRevision === null
    || inference.compiler === null
    || inference.verifier === null
  )) {
    throw new TypeError("semantic inference outcomes require candidate, verification, and provenance bindings");
  }
  if (!semanticOutcome && (
    inference.candidateRevision !== null
    || inference.verificationRevision !== null
    || inference.compiler !== null
    || inference.verifier !== null
  )) {
    throw new TypeError("unrun or failed inference cannot retain semantic success bindings");
  }
  if (inference.status === "accepted"
      && (inference.blockerCount !== 0 || inference.uncertaintyCount !== 0)) {
    throw new TypeError("accepted inference cannot report blockers or uncertainty");
  }
  if (inference.status === "unresolved" && inference.uncertaintyCount < 1) {
    throw new TypeError("unresolved inference must report uncertainty");
  }
  if (checkpoint.status === "published") {
    if (inference.status !== "accepted"
        || checkpoint.checkpointRevision === null
        || checkpoint.ledgerRevision === null
        || checkpoint.reason !== null) {
      throw new TypeError("published checkpoints require accepted inference and exact revisions");
    }
  } else if (checkpoint.checkpointRevision !== null
      || checkpoint.ledgerRevision !== null
      || checkpoint.reason === null) {
    throw new TypeError("unpublished checkpoints require no revisions and an explicit reason");
  }
  const failedStage = inference.status === "failed" || checkpoint.status === "failed";
  if (failedStage !== (failure !== null)) {
    throw new TypeError("lifecycle episode failure evidence must match a failed stage");
  }
  return {
    schemaVersion: CONTEXT_LIFECYCLE_EPISODE_SCHEMA_VERSION,
    type: CONTEXT_LIFECYCLE_EPISODE_TYPE,
    episodeId: text(value.episodeId, "lifecycle episode id"),
    requestRevision: revision(value.requestRevision, "lifecycle episode request revision"),
    scheduleRevision: revision(value.scheduleRevision, "lifecycle episode schedule revision"),
    mode: "shadow",
    subject: normalizeSubject(value.subject),
    startedAt,
    completedAt,
    pressure: normalizePressure(value.pressure),
    sourceRevision: value.sourceRevision === null
      ? null
      : revision(value.sourceRevision, "lifecycle episode source revision"),
    inference,
    checkpoint,
    measurements: normalizeMeasurements(value.measurements),
    transition: { status: "not_requested", retirementAttempted: false },
    failure,
  };
}

export function createContextLifecycleEpisode(value) {
  const body = normalizeBody(value);
  return freeze({ ...body, episodeRevision: digest(body) });
}

export function verifyContextLifecycleEpisode(value) {
  try {
    record(value, "lifecycle episode");
    rejectUnknown(value, [
      "schemaVersion", "type", "episodeId", "requestRevision", "scheduleRevision",
      "mode", "subject", "startedAt", "completedAt", "pressure", "sourceRevision",
      "inference", "checkpoint", "measurements", "transition", "failure", "episodeRevision",
    ], "lifecycle episode");
    const { episodeRevision, ...candidate } = value;
    return digest(normalizeBody(candidate)) === episodeRevision;
  } catch {
    return false;
  }
}

export class InMemoryContextLifecycleEpisodeStore {
  constructor() {
    this.byId = new Map();
  }

  get(episodeId) {
    text(episodeId, "lifecycle episode id");
    return this.byId.get(episodeId) ?? null;
  }

  append(episode) {
    if (!verifyContextLifecycleEpisode(episode)) {
      throw new TypeError("episode store requires an integrity-valid lifecycle episode");
    }
    const existing = this.byId.get(episode.episodeId);
    if (existing) {
      if (existing.episodeRevision !== episode.episodeRevision) {
        throw new TypeError("lifecycle episode id was reused for different evidence");
      }
      return freeze({ status: "replayed", episode: existing });
    }
    this.byId.set(episode.episodeId, episode);
    return freeze({ status: "appended", episode });
  }

  receipts({ logicalRoleInstanceId = null } = {}) {
    if (logicalRoleInstanceId !== null) text(logicalRoleInstanceId, "logical role instance id");
    return Object.freeze([...this.byId.values()].filter((episode) =>
      logicalRoleInstanceId === null
      || episode.subject.logicalRoleInstanceId === logicalRoleInstanceId
    ));
  }
}

const SUMMARY_METRICS = Object.freeze([
  ["retainedInputTokens", (episode) => episode.measurements.context.retainedInputTokens],
  ["reportedContextWindowTokens", (episode) => episode.measurements.context.reportedContextWindowTokens],
  ["growthSincePreviousTokens", (episode) => episode.measurements.context.growthSincePreviousTokens],
  ["estimatedAvoidedInputTokens", (episode) => episode.measurements.context.estimatedAvoidedInputTokens],
  ["compilerInputTokens", (episode) => episode.measurements.compiler.inputTokens],
  ["compilerCachedInputTokens", (episode) => episode.measurements.compiler.cachedInputTokens],
  ["compilerOutputTokens", (episode) => episode.measurements.compiler.outputTokens],
  ["compilerCostMicrounits", (episode) => episode.measurements.compiler.costMicrounits],
  ["verifierInputTokens", (episode) => episode.measurements.verifier.inputTokens],
  ["verifierCachedInputTokens", (episode) => episode.measurements.verifier.cachedInputTokens],
  ["verifierOutputTokens", (episode) => episode.measurements.verifier.outputTokens],
  ["verifierCostMicrounits", (episode) => episode.measurements.verifier.costMicrounits],
  ["inspectionDurationMs", (episode) => episode.measurements.inspectionDurationMs],
  ["publicationDurationMs", (episode) => episode.measurements.publicationDurationMs],
  ["checkpointBytes", (episode) => episode.measurements.checkpointBytes],
]);

export function summarizeContextLifecycleEpisodes(episodes) {
  if (!Array.isArray(episodes) || episodes.length === 0) {
    throw new TypeError("lifecycle episode summary requires at least one episode");
  }
  if (episodes.some((episode) => !verifyContextLifecycleEpisode(episode))) {
    throw new TypeError("lifecycle episode summary requires integrity-valid episodes");
  }
  const policyRevision = episodes[0].pressure.policyRevision;
  const scheduleRevision = episodes[0].scheduleRevision;
  if (episodes.some((episode) =>
    episode.pressure.policyRevision !== policyRevision
    || episode.scheduleRevision !== scheduleRevision
  )) {
    throw new TypeError("lifecycle episode summaries cannot mix policy or schedule revisions");
  }
  const counts = {
    total: episodes.length,
    inspected: episodes.filter((episode) => episode.inference.status !== "not_scheduled").length,
    accepted: episodes.filter((episode) => episode.inference.status === "accepted").length,
    rejected: episodes.filter((episode) => episode.inference.status === "rejected").length,
    unresolved: episodes.filter((episode) => episode.inference.status === "unresolved").length,
    failed: episodes.filter((episode) => episode.failure !== null).length,
    published: episodes.filter((episode) => episode.checkpoint.status === "published").length,
  };
  const byDisposition = Object.fromEntries([...DISPOSITIONS].map((disposition) => [
    disposition,
    episodes.filter((episode) => episode.pressure.disposition === disposition).length,
  ]));
  const measurements = Object.fromEntries(SUMMARY_METRICS.map(([name, select]) => {
    const values = episodes.map(select).filter((value) => value !== null);
    return [name, { observed: values.length, missing: episodes.length - values.length, total: values.reduce((a, b) => a + b, 0) }];
  }));
  const unique = (values) => [...new Set(values.filter((value) => value !== null))].sort();
  const profile = (value) => value === null
    ? null
    : canonical({ producer: value.producer, model: value.model, version: value.version });
  return freeze({
    schemaVersion: 1,
    policyRevision,
    scheduleRevision,
    counts,
    byDisposition,
    dimensions: {
      logicalRoleInstanceIds: unique(episodes.map((episode) => episode.subject.logicalRoleInstanceId)),
      measurementSources: unique(episodes.map((episode) => episode.pressure.measurementSource)),
      compilerProfiles: unique(episodes.map((episode) => profile(episode.inference.compiler))),
      verifierProfiles: unique(episodes.map((episode) => profile(episode.inference.verifier))),
      estimateMethods: unique(episodes.map((episode) => episode.measurements.context.estimateMethod)),
    },
    measurements,
  });
}
