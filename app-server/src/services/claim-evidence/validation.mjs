import {
  ClaimEvidenceError, COMPLETENESS_STATES, LINEAGE_RELATIONSHIPS, PERMISSIONS,
  PROFILES, REFERENCE_STATUSES, RELIANCE_STATES, SCHEMA_VERSION,
} from "./contract.mjs";
import { digest, revisionId, stableClaimId, validateTransportSafeJson } from "./identity.mjs";

export function requireCondition(condition, message) {
  if (!condition) throw new ClaimEvidenceError(message);
}

export function exactFields(value, fields, label) {
  requireCondition(value && typeof value === "object" && !Array.isArray(value), `${label} must be an object`);
  const actual = Object.keys(value).sort();
  const expected = [...fields].sort();
  requireCondition(JSON.stringify(actual) === JSON.stringify(expected), `${label} has missing or unknown fields`);
}

export function nonempty(value, label) {
  requireCondition(typeof value === "string" && value.length > 0, `${label} must be a nonempty string`);
  return value;
}

export function stringList(value, label, { required = false } = {}) {
  requireCondition(Array.isArray(value) && value.every((item) => typeof item === "string" && item.length > 0), `${label} must be a string array`);
  requireCondition(!required || value.length > 0, `${label} must not be empty`);
  return value;
}

export function validateReference(reference, label = "reference") {
  exactFields(reference, ["owner", "reference", "revision", "integrity_sha256", "freshness", "status"], label);
  requireCondition(REFERENCE_STATUSES.has(reference.status), `${label} has unknown status`);
  for (const key of ["owner", "reference", "revision", "freshness"]) nonempty(reference[key], `${label}.${key}`);
  requireCondition(typeof reference.integrity_sha256 === "string" && /^[0-9a-f]{64}$/.test(reference.integrity_sha256), `${label} integrity is invalid`);
}

export function validateAuthority(authority) {
  exactFields(authority, ["schema_version", "grant_id", "actor", "profile", "permissions", "decision_scope", "authority_reference"], "authority");
  requireCondition(authority.schema_version === SCHEMA_VERSION, "unsupported authority version");
  for (const key of ["grant_id", "actor", "decision_scope"]) nonempty(authority[key], `authority.${key}`);
  requireCondition(PROFILES.has(authority.profile), "unknown authority profile");
  requireCondition(Array.isArray(authority.permissions) && authority.permissions.every((item) => PERMISSIONS.has(item)), "unknown authority permission");
  requireCondition(authority.permissions.length === new Set(authority.permissions).size, "authority permissions must be unique");
  validateReference(authority.authority_reference, "authority reference");
  requireCondition(authority.authority_reference.status === "verified", "authority reference is not verified");
}

export function validateProfilePayload(profile, payload) {
  if (profile === "proposal-research-v1") {
    exactFields(payload, ["materiality", "support_qualification"], "proposal research payload");
    nonempty(payload.materiality, "proposal research materiality");
    nonempty(payload.support_qualification, "proposal research support qualification");
  } else if (profile === "revision-bound-review-finding-v1") {
    exactFields(payload, ["finding_id", "severity", "episode", "outcome"], "review finding payload");
    for (const key of ["finding_id", "severity", "episode", "outcome"]) nonempty(payload[key], `review finding ${key}`);
  } else {
    throw new ClaimEvidenceError("unknown profile");
  }
}

const STORE_FIELDS = ["schema_version", "projection_boundary", "authorities", "claims", "revisions", "lineage", "reliances", "operations"];
const REVISION_FIELDS = ["id", "schema_version", "claim_id", "predecessor_revision", "proposition", "support_qualification", "assumptions", "limitations", "confidence", "evidence_references", "sensitivity_references", "producer", "evidence_mode", "judgment_kind", "decision_scope", "profile_payload", "authority_ref", "reopening_conditions", "tombstone"];

export function validateStore(store) {
  exactFields(store, STORE_FIELDS, "store");
  requireCondition(store.schema_version === SCHEMA_VERSION, "unsupported store version");
  const boundary = store.projection_boundary;
  exactFields(boundary, ["actual_content_set", "source_watermark", "excluded_inputs", "failed_inputs", "freshness", "completeness"], "projection boundary");
  requireCondition(COMPLETENESS_STATES.has(boundary.completeness), "unknown completeness");
  nonempty(boundary.actual_content_set, "projection boundary actual_content_set");
  requireCondition(boundary.source_watermark === null || typeof boundary.source_watermark === "string", "projection source watermark is invalid");
  stringList(boundary.excluded_inputs, "projection excluded inputs");
  stringList(boundary.failed_inputs, "projection failed inputs");
  nonempty(boundary.freshness, "projection freshness");
  for (const key of ["authorities", "claims", "revisions", "lineage", "reliances", "operations"]) requireCondition(Array.isArray(store[key]), "record collections must be arrays");
  store.authorities.forEach(validateAuthority);

  const index = new Map();
  for (const collection of ["authorities", "claims", "revisions", "lineage", "reliances", "operations"]) {
    for (const record of store[collection]) {
      const key = collection === "authorities" ? record.grant_id : (record.id ?? record.operation_id);
      nonempty(key, `${collection} record identity`);
      requireCondition(!index.has(key), `duplicate record identity: ${key}`);
      index.set(key, record);
    }
  }
  const authorities = new Map(store.authorities.map((item) => [item.grant_id, item]));
  const claims = new Map(store.claims.map((item) => [item.id, item]));
  const revisions = new Map(store.revisions.map((item) => [item.id, item]));

  for (const claim of store.claims) {
    exactFields(claim, ["id", "schema_version", "profile", "subject", "statement_identity", "created_by", "authority_ref"], "claim");
    requireCondition(claim.schema_version === SCHEMA_VERSION, "unsupported claim version");
    requireCondition(claim.id === stableClaimId(claim.subject), "claim identity does not match bounded subject");
    const authority = authorities.get(claim.authority_ref);
    requireCondition(PROFILES.has(claim.profile) && authority, "claim profile or authority is invalid");
    requireCondition(authority.profile === claim.profile, "claim authority profile is invalid");
    requireCondition(claim.created_by === authority.actor, "claim creator does not match authority actor");
    requireCondition(authority.permissions.includes("create_claim"), "claim authority lacks create_claim permission");
    for (const key of ["statement_identity", "created_by", "authority_ref"]) nonempty(claim[key], `claim.${key}`);
    validateReference(claim.subject.evidence_baseline, "subject evidence baseline");
    stringList(claim.subject.content_set, "subject content set", { required: true });
  }
  for (const revision of store.revisions) {
    exactFields(revision, REVISION_FIELDS, "revision");
    const claim = claims.get(revision.claim_id);
    const authority = authorities.get(revision.authority_ref);
    requireCondition(claim && authority, "revision has dangling claim or authority");
    requireCondition(revision.schema_version === SCHEMA_VERSION, "unsupported revision version");
    requireCondition(revision.predecessor_revision === null || revisions.has(revision.predecessor_revision), "revision predecessor is dangling");
    const withoutId = { ...revision }; delete withoutId.id;
    requireCondition(revision.id === revisionId(withoutId), "revision identity mismatch");
    validateTransportSafeJson(revision.confidence, "revision confidence");
    validateProfilePayload(claim.profile, revision.profile_payload);
    requireCondition(authority.profile === claim.profile, "revision authority profile is invalid");
    requireCondition(authority.decision_scope === revision.decision_scope, "revision authority scope is invalid");
    requireCondition(revision.producer === authority.actor, "revision producer does not match authority actor");
    for (const key of ["proposition", "support_qualification", "producer", "evidence_mode", "judgment_kind", "decision_scope", "authority_ref"]) nonempty(revision[key], `revision.${key}`);
    stringList(revision.assumptions, "revision assumptions");
    stringList(revision.limitations, "revision limitations");
    stringList(revision.reopening_conditions, "revision reopening conditions");
    requireCondition(Array.isArray(revision.evidence_references) && Array.isArray(revision.sensitivity_references), "revision references must be arrays");
    requireCondition(typeof revision.tombstone === "boolean", "revision tombstone must be boolean");
    [...revision.evidence_references, ...revision.sensitivity_references].forEach((item) => validateReference(item));
  }

  const adjacency = new Map();
  for (const edge of store.lineage) {
    exactFields(edge, ["id", "schema_version", "relationship", "sources", "target", "authority_ref", "operation_id"], "lineage");
    requireCondition(LINEAGE_RELATIONSHIPS.has(edge.relationship) && revisions.has(edge.target), "lineage type or target is invalid");
    requireCondition(edge.schema_version === SCHEMA_VERSION, "unsupported lineage version");
    requireCondition(Array.isArray(edge.sources) && edge.sources.length > 0 && edge.sources.every((item) => revisions.has(item)), "lineage source is dangling");
    for (const key of ["id", "authority_ref", "operation_id"]) nonempty(edge[key], `lineage.${key}`);
    const authority = authorities.get(edge.authority_ref);
    requireCondition(authority, "lineage authority is dangling");
    const resources = [...edge.sources, edge.target].map((item) => revisions.get(item));
    requireCondition(new Set(resources.map((item) => claims.get(item.claim_id).profile)).size === 1 && claims.get(resources[0].claim_id).profile === authority.profile, "lineage authority profile is invalid");
    requireCondition(new Set(resources.map((item) => item.decision_scope)).size === 1 && resources[0].decision_scope === authority.decision_scope, "lineage authority scope is invalid");
    for (const source of edge.sources) {
      if (!adjacency.has(source)) adjacency.set(source, new Set());
      adjacency.get(source).add(edge.target);
    }
  }
  const done = new Set();
  function visit(node, active = new Set()) {
    requireCondition(!active.has(node), "cyclic lineage");
    if (done.has(node)) return;
    const next = new Set(active); next.add(node);
    for (const target of adjacency.get(node) ?? []) visit(target, next);
    done.add(node);
  }
  for (const node of adjacency.keys()) visit(node);

  for (const reliance of store.reliances) {
    exactFields(reliance, ["id", "schema_version", "consumer", "consumer_revision", "decision_scope", "claim_revision_id", "state", "predecessor_reliance", "authority_ref", "operation_id"], "reliance");
    requireCondition(revisions.has(reliance.claim_revision_id), "reliance target is dangling");
    requireCondition(reliance.schema_version === SCHEMA_VERSION, "unsupported reliance version");
    requireCondition(RELIANCE_STATES.has(reliance.state), "unknown reliance state");
    requireCondition(
      reliance.predecessor_reliance === null
        || store.reliances.some((item) => item.id === reliance.predecessor_reliance),
      "reliance predecessor is dangling",
    );
    const authority = authorities.get(reliance.authority_ref);
    requireCondition(authority, "reliance authority is dangling");
    const profile = claims.get(revisions.get(reliance.claim_revision_id).claim_id).profile;
    requireCondition(authority.profile === profile, "reliance authority profile is invalid");
    requireCondition(authority.decision_scope === reliance.decision_scope, "reliance authority scope is invalid");
    for (const key of ["id", "consumer", "consumer_revision", "decision_scope", "claim_revision_id", "authority_ref", "operation_id"]) nonempty(reliance[key], `reliance.${key}`);
  }
  const lineageIds = new Set(store.lineage.map((item) => item.id));
  const relianceIds = new Set(store.reliances.map((item) => item.id));
  for (const operation of store.operations) {
    exactFields(operation, ["operation_id", "action", "payload_sha256", "result_identity", "authority_ref"], "operation receipt");
    for (const key of ["operation_id", "result_identity", "authority_ref"]) nonempty(operation[key], `operation receipt.${key}`);
    requireCondition(PERMISSIONS.has(operation.action), "operation receipt action is invalid");
    requireCondition(typeof operation.payload_sha256 === "string" && /^[0-9a-f]{64}$/.test(operation.payload_sha256), "operation receipt digest is invalid");
    const authority = authorities.get(operation.authority_ref);
    requireCondition(authority, "operation receipt authority is dangling");
    requireCondition(authority.permissions.includes(operation.action), "operation receipt authority lacks action permission");
    const result = index.get(operation.result_identity);
    requireCondition(result && result.authority_ref === operation.authority_ref, "operation receipt result authority is invalid");
    if (["create_claim", "publish_revision"].includes(operation.action)) requireCondition(revisions.has(operation.result_identity), "operation receipt result type is invalid");
    else if (["publish_lineage", "retract_revision"].includes(operation.action)) requireCondition(lineageIds.has(operation.result_identity), "operation receipt result type is invalid");
    else requireCondition(relianceIds.has(operation.result_identity), "operation receipt result type is invalid");
  }
  return { index, authorities, claims, revisions };
}

export function operationPayloadDigest(operation, authority) {
  return digest({ operation, admitted_grant: authority });
}
