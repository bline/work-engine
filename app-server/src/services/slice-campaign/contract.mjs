import { createHash } from "node:crypto";

export const SLICE_CAMPAIGN_SCHEMA_VERSION = 1;
export const SLICE_PHASES = Object.freeze(["accepted", "implementing", "gate_ready", "review_ready", "terminal"]);

export function requireRecord(value, label) {
  if (!value || Array.isArray(value) || typeof value !== "object") throw new TypeError(`${label} must be an object`);
  return value;
}

export function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new TypeError(`${label} must be a non-empty string`);
  return value;
}

export function requireSha256(value, label) {
  requireText(value, label);
  if (!/^[0-9a-f]{64}$/.test(value)) throw new TypeError(`${label} must be a SHA-256 digest`);
  return value;
}

export function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}

export function digest(value) {
  return createHash("sha256").update(canonicalJson(value)).digest("hex");
}

export function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

export function normalizeIdentity(value) {
  requireRecord(value, "slice identity");
  const identity = {
    runId: requireText(value.runId, "slice identity runId"),
    sliceNumber: value.sliceNumber,
    attemptId: requireText(value.attemptId, "slice identity attemptId"),
    planVersion: requireText(value.planVersion, "slice identity planVersion"),
  };
  if (!Number.isInteger(identity.sliceNumber) || identity.sliceNumber < 1) throw new TypeError("slice identity sliceNumber must be a positive integer");
  return freeze(identity);
}

export function identityKey(identity) {
  return `${identity.runId}:${identity.sliceNumber}:${identity.attemptId}:${identity.planVersion}`;
}
