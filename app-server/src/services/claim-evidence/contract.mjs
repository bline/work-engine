export const SCHEMA_VERSION = 1;
export const BUILD_VERSION = "claim-evidence-javascript-v1";

export const PROFILES = new Set([
  "proposal-research-v1",
  "revision-bound-review-finding-v1",
]);
export const LINEAGE_RELATIONSHIPS = new Set([
  "refresh", "correction", "supersession", "composition", "derivation",
  "identity_fork", "retraction",
]);
export const REFERENCE_STATUSES = new Set([
  "verified", "unavailable", "moved_resolvable", "excluded", "integrity_mismatch",
]);
export const PERMISSIONS = new Set([
  "create_claim", "publish_revision", "publish_lineage", "record_reliance",
  "retire_reliance", "retract_revision",
]);
export const RELIANCE_STATES = new Set(["active", "retired", "superseded"]);
export const COMPLETENESS_STATES = new Set(["available", "partial", "unavailable"]);

export class ClaimEvidenceError extends TypeError {}

