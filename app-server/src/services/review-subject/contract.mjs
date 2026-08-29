export const REVIEW_SUBJECT_BACKEND = "work-engine.review-subject.legacy-v1";
export const REVIEW_SUBJECT_SCHEMA_VERSION = 1;

export const REVIEW_SUBJECT_OPERATIONS = Object.freeze([
  "create_candidate",
  "transition_candidate",
  "validate_checkpoint",
  "create_physical_profile",
  "validate_physical_profile",
]);

const OPERATION_SET = new Set(REVIEW_SUBJECT_OPERATIONS);
const DIGEST = /^[0-9a-f]{64}$/;

export class ReviewSubjectError extends Error {}

export function requireRecord(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new ReviewSubjectError(`${label} must be an object`);
  }
  return value;
}

export function requireExactFields(value, fields, label) {
  requireRecord(value, label);
  const expected = new Set(fields);
  const unknown = Object.keys(value).filter((field) => !expected.has(field));
  const missing = fields.filter((field) => !(field in value));
  if (unknown.length > 0 || missing.length > 0) {
    throw new ReviewSubjectError(`${label} fields do not match schema version 1`);
  }
}

export function requireDigest(value, label) {
  if (typeof value !== "string" || !DIGEST.test(value)) {
    throw new ReviewSubjectError(`${label} must be lowercase SHA-256`);
  }
  return value;
}

export function requireOperation(value) {
  if (typeof value !== "string" || !OPERATION_SET.has(value)) {
    throw new ReviewSubjectError(`unsupported review-subject operation: ${String(value)}`);
  }
  return value;
}

export function validateBackendEnvelope(value, { operation, backendDigests }) {
  requireExactFields(
    value,
    ["schema_version", "backend", "backend_sha256", "operation", "result"],
    "review-subject backend envelope",
  );
  if (value.schema_version !== REVIEW_SUBJECT_SCHEMA_VERSION) {
    throw new ReviewSubjectError("review-subject backend schema version mismatch");
  }
  if (value.backend !== REVIEW_SUBJECT_BACKEND) {
    throw new ReviewSubjectError("review-subject backend identity mismatch");
  }
  if (value.operation !== operation) {
    throw new ReviewSubjectError("review-subject backend operation mismatch");
  }
  requireExactFields(value.backend_sha256, ["checkpoint", "physical_profile"], "review-subject backend identity");
  for (const name of ["checkpoint", "physical_profile"]) {
    requireDigest(value.backend_sha256[name], `review-subject ${name} backend identity`);
    if (value.backend_sha256[name] !== backendDigests[name]) {
      throw new ReviewSubjectError(`review-subject ${name} backend identity mismatch`);
    }
  }
  requireRecord(value.result, "review-subject backend result");
  return value.result;
}
