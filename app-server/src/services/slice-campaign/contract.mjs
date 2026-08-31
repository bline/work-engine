import { createHash } from "node:crypto";

export const SLICE_CAMPAIGN_SCHEMA_VERSION = 2;
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

export function validateReviewSelection(value, expectedSubject) {
  requireRecord(value, "review selection");
  const fields = ["schemaVersion", "owner", "selectionId", "subject", "specialists"];
  if (Object.keys(value).some((field) => !fields.includes(field)) || fields.some((field) => !(field in value))) {
    throw new TypeError("review selection fields are invalid");
  }
  if (value.schemaVersion !== 1 || value.owner !== "slice-supervisor") throw new TypeError("review selection owner is invalid");
  requireText(value.selectionId, "review selection selectionId");
  requireRecord(value.subject, "review selection subject");
  if (digest(value.subject) !== digest(expectedSubject)) throw new TypeError("review selection subject does not match the immutable candidate");
  if (!Array.isArray(value.specialists) || value.specialists.length === 0) throw new TypeError("review selection requires specialist dispositions");
  const ids = [];
  for (const [index, specialist] of value.specialists.entries()) {
    requireRecord(specialist, `review selection specialists[${index}]`);
    const specialistFields = ["obligationId", "skill", "selection"];
    if (Object.keys(specialist).some((field) => !specialistFields.includes(field))
        || specialistFields.some((field) => !(field in specialist))) throw new TypeError("review specialist disposition fields are invalid");
    requireText(specialist.obligationId, "review specialist obligationId");
    requireText(specialist.skill, "review specialist skill");
    if (!["selected", "omitted"].includes(specialist.selection)) throw new TypeError("review specialist disposition is invalid");
    ids.push(specialist.obligationId);
  }
  if (new Set(ids).size !== ids.length) throw new TypeError("review specialist obligation IDs must be unique");
  return value;
}
