import { SCHEMA_VERSION } from "../claim-evidence/contract.mjs";
import { canonicalJson, validateTransportSafeJson } from "../claim-evidence/identity.mjs";
import { readClaimEvidence } from "../claim-evidence/read-service.mjs";
import { exactFields, nonempty, requireCondition } from "../claim-evidence/validation.mjs";

export const DEVELOPMENT_CLAIM_CONTEXT_NAME = "work-engine.claim-evidence";
export const DEVELOPMENT_CLAIM_CONTEXT_KIND =
  "application/vnd.work-engine.claim-evidence+json;version=1";

function deepFreeze(value) {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    Object.values(value).forEach(deepFreeze);
    Object.freeze(value);
  }
  return value;
}

export function projectDevelopmentClaimContext(store, request) {
  if (!store || typeof store.exportStore !== "function") {
    throw new TypeError("development claim context requires a claim-evidence store");
  }
  validateTransportSafeJson(request, "development claim context request");
  exactFields(
    request,
    ["schema_version", "request_id", "consumer", "selections"],
    "development claim context request",
  );
  requireCondition(
    request.schema_version === SCHEMA_VERSION,
    "development claim context request version is unsupported",
  );
  nonempty(request.request_id, "development claim context request_id");
  exactFields(
    request.consumer,
    ["identity", "revision", "decision_scope"],
    "development claim context consumer",
  );
  nonempty(request.consumer.identity, "development claim context consumer identity");
  nonempty(request.consumer.revision, "development claim context consumer revision");
  nonempty(request.consumer.decision_scope, "development claim context decision scope");

  const receipt = readClaimEvidence(store, {
    schema_version: SCHEMA_VERSION,
    request_id: request.request_id,
    operation: "project_relevant_revisions",
    parameters: { selections: request.selections },
  });
  requireCondition(
    receipt.outcome === "succeeded",
    `development claim context projection failed: ${receipt.refusal?.message ?? "unknown refusal"}`,
  );
  requireCondition(
    receipt.result.relevant_exact_revisions.every(({ revision }) =>
      revision.decision_scope === request.consumer.decision_scope
    ),
    "development claim context decision scope does not match every selected revision",
  );

  const context = deepFreeze(structuredClone({
    schema_version: SCHEMA_VERSION,
    context_kind: "relevant_exact_claim_revisions",
    consumer: request.consumer,
    projection: receipt.projection,
    relevant_exact_revisions: receipt.result.relevant_exact_revisions,
  }));
  return deepFreeze({
    context,
    request_context: {
      [DEVELOPMENT_CLAIM_CONTEXT_NAME]: {
        kind: DEVELOPMENT_CLAIM_CONTEXT_KIND,
        value: canonicalJson(context),
      },
    },
  });
}
