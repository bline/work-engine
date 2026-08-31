import { SCHEMA_VERSION } from "./contract.mjs";
import { canonicalJson, validateTransportSafeJson } from "./identity.mjs";
import { readClaimEvidence } from "./read-service.mjs";
import { exactFields, nonempty, requireCondition } from "./validation.mjs";

export const REVIEWER_CLAIM_CONTEXT_KIND = "reviewer_relevant_exact_claim_revisions";
export const REVIEWER_CLAIM_CONTEXT_MARKER = "WORK_ENGINE_REVIEWER_CLAIM_EVIDENCE_V1";

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  Object.values(value).forEach(freeze);
  return Object.freeze(value);
}

export function validateReviewerClaimContext(context) {
  validateTransportSafeJson(context, "reviewer claim context");
  exactFields(context, [
    "schema_version", "context_kind", "consumer", "projection",
    "relevant_exact_revisions", "limitations", "authority",
  ], "reviewer claim context");
  requireCondition(context.schema_version === SCHEMA_VERSION, "reviewer claim context version is unsupported");
  requireCondition(context.context_kind === REVIEWER_CLAIM_CONTEXT_KIND, "reviewer claim context kind is unsupported");
  exactFields(context.consumer, ["identity", "revision", "decision_scope"], "reviewer claim context consumer");
  for (const field of ["identity", "revision", "decision_scope"]) nonempty(context.consumer[field], `reviewer claim context consumer.${field}`);
  exactFields(context.projection, ["projection_schema_version", "build_version", "projection_identity", "freshness", "completeness", "actual_content_set", "excluded_inputs", "failed_inputs", "unresolved_references"], "reviewer claim projection identity");
  requireCondition(Array.isArray(context.relevant_exact_revisions) && context.relevant_exact_revisions.length > 0, "reviewer claim context requires exact revisions");
  requireCondition(Array.isArray(context.limitations) && context.limitations.every((item) => typeof item === "string" && item.trim()), "reviewer claim context limitations are invalid");
  exactFields(context.authority, [
    "claimPublicationAuthorized", "claimSelectionAuthorized", "findingEvaluationAuthorized",
    "reviewAcceptanceAuthorized", "campaignAcceptanceAuthorized", "mutationAuthorized",
  ], "reviewer claim context authority");
  requireCondition(Object.values(context.authority).every((value) => value === false), "reviewer claim context cannot confer authority");
  for (const selected of context.relevant_exact_revisions) {
    requireCondition(selected.revision?.decision_scope === context.consumer.decision_scope, "reviewer claim context decision scope mismatch");
    requireCondition(!("permissions" in selected) && !("operation" in selected) && !("store" in selected), "reviewer claim context exposes registry mechanics");
  }
  requireCondition(!("store" in context) && !("operations" in context) && !("permissions" in context), "reviewer claim context exposes canonical state");
  return context;
}

export function projectReviewerClaimContext(store, request) {
  if (!store || typeof store.exportStore !== "function") throw new TypeError("reviewer claim projection requires a claim-evidence store");
  validateTransportSafeJson(request, "reviewer claim projection request");
  exactFields(request, ["schema_version", "request_id", "consumer", "selections", "limitations"], "reviewer claim projection request");
  requireCondition(request.schema_version === SCHEMA_VERSION, "reviewer claim projection request version is unsupported");
  nonempty(request.request_id, "reviewer claim projection request_id");
  exactFields(request.consumer, ["identity", "revision", "decision_scope"], "reviewer claim projection consumer");
  for (const field of ["identity", "revision", "decision_scope"]) nonempty(request.consumer[field], `reviewer claim projection consumer.${field}`);
  requireCondition(Array.isArray(request.limitations) && request.limitations.length > 0
    && request.limitations.every((item) => typeof item === "string" && item.trim()), "reviewer claim projection requires limitations");
  const receipt = readClaimEvidence(store, {
    schema_version: SCHEMA_VERSION, request_id: request.request_id,
    operation: "project_relevant_revisions", parameters: {selections: request.selections},
  });
  requireCondition(receipt.outcome === "succeeded", `reviewer claim projection failed: ${receipt.refusal?.message ?? "unknown refusal"}`);
  const context = {
    schema_version: SCHEMA_VERSION,
    context_kind: REVIEWER_CLAIM_CONTEXT_KIND,
    consumer: structuredClone(request.consumer),
    projection: receipt.projection,
    relevant_exact_revisions: receipt.result.relevant_exact_revisions,
    limitations: [...request.limitations],
    authority: {
      claimPublicationAuthorized: false, claimSelectionAuthorized: false,
      findingEvaluationAuthorized: false, reviewAcceptanceAuthorized: false,
      campaignAcceptanceAuthorized: false, mutationAuthorized: false,
    },
  };
  validateReviewerClaimContext(context);
  return freeze(context);
}

export function renderReviewerClaimContext(context) {
  validateReviewerClaimContext(context);
  return `${REVIEWER_CLAIM_CONTEXT_MARKER}\n${canonicalJson(context)}`;
}
