import { createHash } from "node:crypto";

export const TOKEN_USAGE_PRESSURE_PROFILE_SCHEMA_VERSION = 1;

const PROFILE_FIELDS = ["schemaVersion", "usageField", "windowField", "rounding", "saturation"];

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function rejectUnknown(value, allowed, label) {
  const unknown = Object.keys(value).filter((field) => !allowed.includes(field)).sort();
  if (unknown.length > 0) throw new TypeError(`${label} contains unsupported fields: ${unknown.join(", ")}`);
}

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

function revision(value) {
  return `sha256:${createHash("sha256").update(canonical(value)).digest("hex")}`;
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

export function validateTokenUsagePressureProfile(value) {
  record(value, "token usage pressure profile");
  rejectUnknown(value, PROFILE_FIELDS, "token usage pressure profile");
  const normalized = {
    schemaVersion: value.schemaVersion,
    usageField: value.usageField,
    windowField: value.windowField,
    rounding: value.rounding,
    saturation: value.saturation,
  };
  if (normalized.schemaVersion !== TOKEN_USAGE_PRESSURE_PROFILE_SCHEMA_VERSION) {
    throw new TypeError("unsupported token usage pressure profile schema version");
  }
  if (normalized.usageField !== "last.totalTokens"
      || normalized.windowField !== "modelContextWindow"
      || normalized.rounding !== "floor"
      || normalized.saturation !== "clamp_10000") {
    throw new TypeError("unsupported token usage pressure measurement profile");
  }
  return freeze({ ...normalized, profileRevision: revision(normalized) });
}

export class TokenUsagePressureProjector {
  constructor({ profile, now = () => new Date().toISOString() }) {
    this.profile = validateTokenUsagePressureProfile(profile);
    if (typeof now !== "function") throw new TypeError("token usage pressure clock must be a function");
    this.now = now;
  }

  project(lifecycleSnapshot) {
    record(lifecycleSnapshot, "lifecycle evidence snapshot");
    const usage = lifecycleSnapshot.latestTokenUsage;
    if (usage === null) return freeze({ status: "unavailable", reason: "token_usage_missing" });
    record(usage, "latest token usage observation");
    const measuredTokens = usage.details?.last?.totalTokens;
    const contextWindow = usage.details?.modelContextWindow;
    if (!Number.isSafeInteger(measuredTokens) || measuredTokens < 0) {
      throw new TypeError("latest token usage last.totalTokens must be a non-negative safe integer");
    }
    const reportedInputTokens = usage.details?.last?.inputTokens;
    if (!Number.isSafeInteger(reportedInputTokens) || reportedInputTokens < 0) {
      throw new TypeError("latest token usage last.inputTokens must be a non-negative safe integer");
    }
    if (!Number.isSafeInteger(contextWindow) || contextWindow < 1) {
      return freeze({ status: "unavailable", reason: "context_window_missing" });
    }
    if (!Number.isSafeInteger(usage.sequence) || usage.sequence < 1) {
      throw new TypeError("latest token usage sequence must be a positive safe integer");
    }
    const observedAt = this.now();
    if (typeof observedAt !== "string" || Number.isNaN(Date.parse(observedAt))) {
      throw new TypeError("token usage pressure clock must return an ISO timestamp");
    }
    const sourceEvidence = {
      profileRevision: this.profile.profileRevision,
      lifecycleSchemaVersion: lifecycleSnapshot.schemaVersion,
      threadId: lifecycleSnapshot.threadId,
      observation: usage,
    };
    const sourceRevision = revision(sourceEvidence);
    return freeze({
      status: "projected",
      measuredTokens,
      reportedInputTokens,
      contextWindow,
      profileRevision: this.profile.profileRevision,
      observation: {
        schemaVersion: 1,
        observationId: `token-usage:${lifecycleSnapshot.threadId}:${usage.sequence}`,
        sequence: usage.sequence,
        observedAt,
        pressureBasisPoints: Math.min(10_000, Math.floor((measuredTokens / contextWindow) * 10_000)),
        source: "codex_app_server:last.totalTokens/modelContextWindow",
        sourceRevision,
      },
    });
  }
}
