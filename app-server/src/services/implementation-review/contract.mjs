import { createHash } from "node:crypto";

export const IMPLEMENTATION_REVIEW_SCHEMA_VERSION = 1;
export const IMPLEMENTATION_REVIEW_VERDICTS = Object.freeze([
  "acceptable_as_is", "remediation_required", "incomplete",
]);
export const IMPLEMENTATION_REVIEW_FINDING_STATUSES = Object.freeze([
  "open", "remediation_presented", "verified_resolved", "withdrawn", "unresolved",
]);

const VERDICTS = new Set(IMPLEMENTATION_REVIEW_VERDICTS);
const STATUSES = new Set(IMPLEMENTATION_REVIEW_FINDING_STATUSES);
const SEVERITIES = new Set(["blocker", "high", "medium", "low", "info"]);
const BASES = new Set(["reproduced", "inferred"]);
const CONFIDENCE = new Set(["high", "medium", "low"]);
const SHA256 = /^[0-9a-f]{64}$/;

export class ImplementationReviewError extends Error {}

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

export function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new ImplementationReviewError(`${label} must be an object`);
  return value;
}

export function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new ImplementationReviewError(`${label} must be non-empty text`);
  return value;
}

export function exact(value, fields, label) {
  record(value, label);
  const expected = new Set(fields);
  const unknown = Object.keys(value).filter((field) => !expected.has(field));
  const missing = fields.filter((field) => !(field in value));
  if (unknown.length || missing.length) throw new ImplementationReviewError(`${label} fields are invalid`);
}

export function sha256(value, label) {
  if (typeof value !== "string" || !SHA256.test(value)) throw new ImplementationReviewError(`${label} must be lowercase SHA-256`);
  return value;
}

export function validateSubject(value, label = "review subject") {
  exact(value, ["commit", "tree", "patchIdentity"], label);
  for (const field of ["commit", "tree", "patchIdentity"]) text(value[field], `${label}.${field}`);
  return value;
}

export function validateEvidence(value, label = "review evidence") {
  exact(value, ["path", "startLine", "endLine", "sha256"], label);
  text(value.path, `${label}.path`);
  if (!Number.isSafeInteger(value.startLine) || value.startLine < 1
      || !Number.isSafeInteger(value.endLine) || value.endLine < value.startLine) {
    throw new ImplementationReviewError(`${label} line bounds are invalid`);
  }
  sha256(value.sha256, `${label}.sha256`);
  return value;
}

function evidenceArray(value, label, { required = false } = {}) {
  if (!Array.isArray(value) || (required && value.length === 0)) throw new ImplementationReviewError(`${label} must be ${required ? "a non-empty" : "an"} array`);
  value.forEach((item, index) => validateEvidence(item, `${label}[${index}]`));
  return value;
}

export function validateFinding(value, label = "review finding") {
  exact(value, [
    "id", "severity", "title", "evidence", "observed", "violatedExpectation", "consequence",
    "basis", "confidence", "recommendedRemediation", "status", "remediationEvidence",
  ], label);
  for (const field of ["id", "title", "observed", "violatedExpectation", "consequence", "recommendedRemediation"]) text(value[field], `${label}.${field}`);
  if (!SEVERITIES.has(value.severity)) throw new ImplementationReviewError(`${label}.severity is invalid`);
  if (!BASES.has(value.basis)) throw new ImplementationReviewError(`${label}.basis is invalid`);
  if (!CONFIDENCE.has(value.confidence)) throw new ImplementationReviewError(`${label}.confidence is invalid`);
  if (!STATUSES.has(value.status)) throw new ImplementationReviewError(`${label}.status is invalid`);
  evidenceArray(value.evidence, `${label}.evidence`, { required: true });
  evidenceArray(value.remediationEvidence, `${label}.remediationEvidence`);
  if (["remediation_presented", "verified_resolved"].includes(value.status) && value.remediationEvidence.length === 0) {
    throw new ImplementationReviewError(`${label} remediation status requires remediation evidence`);
  }
  return value;
}

export function validateImplementationReviewResult(value) {
  exact(value, ["schemaVersion", "subject", "verdict", "findings", "decisiveEvidence", "limitations"], "implementation review result");
  if (value.schemaVersion !== IMPLEMENTATION_REVIEW_SCHEMA_VERSION) throw new ImplementationReviewError("implementation review schema version is invalid");
  validateSubject(value.subject);
  if (!VERDICTS.has(value.verdict)) throw new ImplementationReviewError("implementation review verdict is invalid");
  if (!Array.isArray(value.findings)) throw new ImplementationReviewError("implementation review findings must be an array");
  value.findings.forEach((finding, index) => validateFinding(finding, `implementation review findings[${index}]`));
  const ids = value.findings.map(({ id }) => id);
  if (new Set(ids).size !== ids.length) throw new ImplementationReviewError("implementation review finding IDs must be unique");
  evidenceArray(value.decisiveEvidence, "implementation review decisiveEvidence");
  if (!Array.isArray(value.limitations) || value.limitations.some((item) => typeof item !== "string" || item.trim() === "")) {
    throw new ImplementationReviewError("implementation review limitations must be non-empty text values");
  }
  const pending = value.findings.filter(({ status }) => ["open", "remediation_presented", "unresolved"].includes(status));
  if (value.verdict === "acceptable_as_is" && (value.decisiveEvidence.length === 0 || value.limitations.length || pending.length)) {
    throw new ImplementationReviewError("acceptable_as_is requires decisive evidence and no limitation or unresolved finding");
  }
  if (value.verdict === "remediation_required" && pending.length === 0) {
    throw new ImplementationReviewError("remediation_required requires an unresolved evidence-bearing finding");
  }
  if (value.verdict === "incomplete" && value.limitations.length === 0) {
    throw new ImplementationReviewError("incomplete requires explicit limitations");
  }
  return value;
}
