import { createHash } from "node:crypto";
import { validateImplementationReviewResult } from "../implementation-review/contract.mjs";

export const REVIEW_EPISODE_SCHEMA_VERSION = 1;
const SHA256 = /^[0-9a-f]{64}$/;

export class ReviewEpisodeError extends Error {}

export function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}

export const digest = (value) => createHash("sha256").update(canonicalJson(value)).digest("hex");

export function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

export function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new ReviewEpisodeError(`${label} must be an object`);
  return value;
}

export function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new ReviewEpisodeError(`${label} must be non-empty text`);
  return value;
}

export function sha256(value, label) {
  if (typeof value !== "string" || !SHA256.test(value)) throw new ReviewEpisodeError(`${label} must be lowercase SHA-256`);
  return value;
}

export function exact(value, fields, label) {
  record(value, label);
  const expected = new Set(fields);
  if (Object.keys(value).some((field) => !expected.has(field)) || fields.some((field) => !(field in value))) {
    throw new ReviewEpisodeError(`${label} fields are invalid`);
  }
}

export function validateReference(value, label = "review episode reference") {
  exact(value, ["owner", "reference", "revision", "sha256", "freshness"], label);
  for (const field of ["owner", "reference", "revision", "freshness"]) text(value[field], `${label}.${field}`);
  sha256(value.sha256, `${label}.sha256`);
  return value;
}

export function validateIdentity(value) {
  exact(value, ["runId", "sliceNumber", "attemptId", "planVersion", "reviewObligationId", "reviewEpisodeId"], "review episode identity");
  for (const field of ["runId", "attemptId", "planVersion", "reviewObligationId", "reviewEpisodeId"]) text(value[field], `review episode identity.${field}`);
  if (!Number.isSafeInteger(value.sliceNumber) || value.sliceNumber < 1) throw new ReviewEpisodeError("review episode identity.sliceNumber must be positive");
  return value;
}

export function validateWriter(value) {
  exact(value, ["actorId", "provider", "generation", "runtimeSession"], "review episode writer");
  text(value.actorId, "review episode writer.actorId");
  text(value.provider, "review episode writer.provider");
  if (!Number.isSafeInteger(value.generation) || value.generation < 1) throw new ReviewEpisodeError("review episode writer.generation must be positive");
  validateReference(value.runtimeSession, "review episode writer.runtimeSession");
  return value;
}

export function validateAuthority(value) {
  exact(value, ["schemaVersion", "grantId", "identity", "source", "writer", "readers", "initialSubject", "predecessorRevision"], "review episode authority");
  if (value.schemaVersion !== 1) throw new ReviewEpisodeError("review episode authority schema version is invalid");
  text(value.grantId, "review episode authority.grantId");
  validateIdentity(value.identity);
  validateReference(value.source, "review episode authority.source");
  validateWriter(value.writer);
  if (!Array.isArray(value.readers) || value.readers.length === 0 || value.readers.some((reader) => typeof reader !== "string" || reader.trim() === "") || new Set(value.readers).size !== value.readers.length) {
    throw new ReviewEpisodeError("review episode authority.readers are invalid");
  }
  validateReference(value.initialSubject, "review episode authority.initialSubject");
  if (value.writer.generation === 1 && value.predecessorRevision !== null) throw new ReviewEpisodeError("initial review episode authority cannot name a predecessor");
  if (value.writer.generation > 1) sha256(value.predecessorRevision, "review episode authority.predecessorRevision");
  return value;
}

export function validateState(value) {
  exact(value, [
    "schemaVersion", "identity", "authority", "writer", "status", "phase", "subject", "currentResult",
    "unresolvedQuestions", "pendingAction", "handledTransitions", "continuity", "uncertainty", "retirement",
  ], "review episode state");
  if (value.schemaVersion !== REVIEW_EPISODE_SCHEMA_VERSION) throw new ReviewEpisodeError("review episode state schema version is invalid");
  validateIdentity(value.identity);
  exact(value.authority, ["grantId", "source", "manifestRevision", "readers"], "review episode authority binding");
  text(value.authority.grantId, "review episode authority binding.grantId");
  validateReference(value.authority.source, "review episode authority binding.source");
  sha256(value.authority.manifestRevision, "review episode authority binding.manifestRevision");
  if (!Array.isArray(value.authority.readers) || value.authority.readers.some((item) => typeof item !== "string" || item.trim() === "")) throw new ReviewEpisodeError("review episode authority binding.readers are invalid");
  validateWriter(value.writer);
  if (!["active", "uncertain", "retired"].includes(value.status)) throw new ReviewEpisodeError("review episode status is invalid");
  if (!["initial_review", "remediation", "re_evaluation", "reported"].includes(value.phase)) throw new ReviewEpisodeError("review episode phase is invalid");
  validateReference(value.subject, "review episode subject");
  if (value.currentResult !== null) validateImplementationReviewResult(value.currentResult);
  if (!Array.isArray(value.unresolvedQuestions) || value.unresolvedQuestions.some((item) => typeof item !== "string" || item.trim() === "")) throw new ReviewEpisodeError("review episode unresolvedQuestions are invalid");
  text(value.pendingAction, "review episode pendingAction");
  record(value.handledTransitions, "review episode handledTransitions");
  for (const [id, revision] of Object.entries(value.handledTransitions)) { text(id, "review episode transition id"); sha256(revision, "review episode transition digest"); }
  if (!["fresh_initial", "same_session", "reconstructed_continuation"].includes(value.continuity)) throw new ReviewEpisodeError("review episode continuity is invalid");
  if (value.status === "uncertain") {
    exact(value.uncertainty, ["reason", "reconciliationAction"], "review episode uncertainty");
    text(value.uncertainty.reason, "review episode uncertainty.reason");
    text(value.uncertainty.reconciliationAction, "review episode uncertainty.reconciliationAction");
  } else if (value.uncertainty !== null) throw new ReviewEpisodeError("only uncertain review episode state may contain uncertainty");
  if (value.status === "retired") {
    exact(value.retirement, ["outcome", "reason", "protectedReferences"], "review episode retirement");
    text(value.retirement.outcome, "review episode retirement.outcome");
    text(value.retirement.reason, "review episode retirement.reason");
    if (!Array.isArray(value.retirement.protectedReferences) || value.retirement.protectedReferences.length === 0) throw new ReviewEpisodeError("review episode retirement requires protected references");
    value.retirement.protectedReferences.forEach((item, index) => validateReference(item, `review episode retirement.protectedReferences[${index}]`));
  } else if (value.retirement !== null) throw new ReviewEpisodeError("only retired review episode state may contain retirement");
  return value;
}

export const identityKey = (identity) => digest(validateIdentity(identity));
