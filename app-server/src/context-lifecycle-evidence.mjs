export const CONTEXT_LIFECYCLE_EVIDENCE_SCHEMA_VERSION = 1;

const OBSERVATION_TYPES = new Set([
  "context_transition_signal",
  "token_usage",
]);

function requireRecord(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function requireNonNegativeInteger(value, label, { nullable = false } = {}) {
  if (nullable && value === null) return null;
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new TypeError(`${label} must be a non-negative safe integer${nullable ? " or null" : ""}`);
  }
  return value;
}

function deepFreeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) deepFreeze(child);
  return Object.freeze(value);
}

function normalizeSource(source) {
  requireRecord(source, "lifecycle evidence source");
  return {
    provider: requireText(source.provider, "lifecycle evidence provider"),
    transport: requireText(source.transport, "lifecycle evidence transport"),
    protocolVersion: requireText(
      source.protocolVersion,
      "lifecycle evidence protocol version",
    ),
    method: requireText(source.method, "lifecycle evidence source method"),
  };
}

function normalizeTokenBreakdown(value, label) {
  requireRecord(value, label);
  return {
    inputTokens: requireNonNegativeInteger(value.inputTokens, `${label} inputTokens`),
    cachedInputTokens: requireNonNegativeInteger(
      value.cachedInputTokens,
      `${label} cachedInputTokens`,
    ),
    cacheWriteInputTokens: requireNonNegativeInteger(
      Object.hasOwn(value, "cacheWriteInputTokens")
        ? value.cacheWriteInputTokens
        : 0,
      `${label} cacheWriteInputTokens`,
    ),
    outputTokens: requireNonNegativeInteger(value.outputTokens, `${label} outputTokens`),
    reasoningOutputTokens: requireNonNegativeInteger(
      value.reasoningOutputTokens,
      `${label} reasoningOutputTokens`,
    ),
    totalTokens: requireNonNegativeInteger(value.totalTokens, `${label} totalTokens`),
  };
}

function normalizeDetails(observationType, details) {
  requireRecord(details, "lifecycle evidence details");
  if (observationType === "token_usage") {
    return {
      last: normalizeTokenBreakdown(details.last, "last token usage"),
      total: normalizeTokenBreakdown(details.total, "total token usage"),
      modelContextWindow: requireNonNegativeInteger(
        details.modelContextWindow ?? null,
        "modelContextWindow",
        { nullable: true },
      ),
    };
  }
  if (details.signal !== "context_compaction") {
    throw new TypeError("context transition signal must be context_compaction");
  }
  if (!["started", "completed", "reported"].includes(details.phase)) {
    throw new TypeError("context transition signal phase is invalid");
  }
  if (details.classification !== "unclassified") {
    throw new TypeError("provider transition signals must remain unclassified");
  }
  const providerItemId = details.providerItemId ?? null;
  if (details.phase === "reported" && providerItemId !== null) {
    throw new TypeError(
      "reported context transition signals cannot claim a provider item id",
    );
  }
  if (details.phase !== "reported" && providerItemId === null) {
    throw new TypeError(
      "started or completed context transition signals require a provider item id",
    );
  }
  if (providerItemId !== null) {
    requireText(providerItemId, "context transition provider item id");
  }
  return {
    signal: details.signal,
    phase: details.phase,
    classification: details.classification,
    providerItemId,
  };
}

export function normalizeLifecycleObservation(observation) {
  requireRecord(observation, "lifecycle observation");
  if (observation.schemaVersion !== CONTEXT_LIFECYCLE_EVIDENCE_SCHEMA_VERSION) {
    throw new TypeError("unsupported lifecycle observation schema version");
  }
  if (!OBSERVATION_TYPES.has(observation.observationType)) {
    throw new TypeError("unsupported lifecycle observation type");
  }
  const normalized = {
    schemaVersion: CONTEXT_LIFECYCLE_EVIDENCE_SCHEMA_VERSION,
    observationType: observation.observationType,
    source: normalizeSource(observation.source),
    threadId: requireText(observation.threadId, "lifecycle observation thread id"),
    turnId: requireText(observation.turnId, "lifecycle observation turn id"),
    details: normalizeDetails(observation.observationType, observation.details),
  };
  return deepFreeze(normalized);
}

export class ContextLifecycleEvidenceCollector {
  constructor({ retentionLimit = 256, initialSequence = 0 } = {}) {
    if (!Number.isSafeInteger(retentionLimit) || retentionLimit < 1) {
      throw new TypeError(
        "lifecycle evidence retention limit must be a positive safe integer",
      );
    }
    requireNonNegativeInteger(initialSequence, "lifecycle evidence initial sequence");
    this.retentionLimit = retentionLimit;
    this.nextSequence = initialSequence + 1;
    this.droppedThroughSequence = initialSequence;
    this.retained = [];
  }

  record(observation) {
    const normalized = normalizeLifecycleObservation(observation);
    const recorded = deepFreeze({
      ...normalized,
      sequence: this.nextSequence,
    });
    this.nextSequence += 1;
    this.retained.push(recorded);
    while (this.retained.length > this.retentionLimit) {
      this.droppedThroughSequence = this.retained.shift().sequence;
    }
    return recorded;
  }

  observations({ threadId = null, afterSequence = 0 } = {}) {
    if (threadId !== null) requireText(threadId, "lifecycle evidence thread id");
    requireNonNegativeInteger(afterSequence, "lifecycle evidence sequence cursor");
    return Object.freeze(this.retained.filter((observation) =>
      observation.sequence > afterSequence
      && (threadId === null || observation.threadId === threadId)
    ));
  }

  snapshot(threadId) {
    requireText(threadId, "lifecycle evidence thread id");
    const observations = this.observations({ threadId });
    const tokenUsage = observations.filter((observation) =>
      observation.observationType === "token_usage"
    ).at(-1) ?? null;
    const transitionSignals = observations.filter((observation) =>
      observation.observationType === "context_transition_signal"
    );
    return deepFreeze({
      schemaVersion: CONTEXT_LIFECYCLE_EVIDENCE_SCHEMA_VERSION,
      threadId,
      retainedObservationCount: observations.length,
      firstRetainedSequence: observations[0]?.sequence ?? null,
      lastRetainedSequence: observations.at(-1)?.sequence ?? null,
      retention: {
        limit: this.retentionLimit,
        droppedThroughSequence: this.droppedThroughSequence,
        earliestGloballyRetainedSequence: this.retained[0]?.sequence ?? null,
      },
      latestTokenUsage: tokenUsage,
      transitionSignals,
    });
  }
}
