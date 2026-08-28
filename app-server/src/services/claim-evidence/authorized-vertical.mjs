import { SCHEMA_VERSION } from "./contract.mjs";
import { digest, validateTransportSafeJson } from "./identity.mjs";
import { readClaimEvidence } from "./read-service.mjs";
import { exactFields, nonempty, requireCondition, validateReference } from "./validation.mjs";

const VERTICAL_PROFILES = new Set([
  "proposal-research-v1",
  "revision-bound-review-finding-v1",
]);

function validatePublication(publication) {
  exactFields(publication, ["source_binding", "authority", "operation"], "authorized vertical publication");
  validateReference(publication.source_binding, "authorized vertical source binding");
  requireCondition(publication.source_binding.status === "verified", "authorized vertical source binding is not verified");
  const { authority, operation, source_binding: sourceBinding } = publication;
  requireCondition(operation?.schema_version === SCHEMA_VERSION, "authorized vertical operation version is unsupported");
  requireCondition(operation.action === "create_claim" && operation.expected_state === null, "authorized vertical must publish an initial claim revision");
  requireCondition(VERTICAL_PROFILES.has(operation.profile) && authority?.profile === operation.profile, "authorized vertical authority profile mismatch");
  requireCondition(operation.payload?.initial_revision?.decision_scope === authority.decision_scope, "authorized vertical decision scope mismatch");
  requireCondition(digest(operation.payload?.subject?.evidence_baseline) === digest(sourceBinding), "authorized vertical subject source binding mismatch");
  requireCondition(
    Array.isArray(operation.payload?.initial_revision?.evidence_references)
      && operation.payload.initial_revision.evidence_references.some((reference) => digest(reference) === digest(sourceBinding)),
    "authorized vertical revision source binding mismatch",
  );
  requireCondition(operation.payload.subject.content_set.includes(sourceBinding.reference), "authorized vertical source binding is outside the subject content set");
}

export function runAuthorizedClaimsVertical(store, request) {
  if (!store || typeof store.publish !== "function" || typeof store.exportStore !== "function") {
    throw new TypeError("authorized vertical claim-evidence store is required");
  }
  validateTransportSafeJson(request, "authorized vertical request");
  exactFields(request, ["schema_version", "vertical_id", "publications", "relevant_revisions"], "authorized vertical request");
  requireCondition(request.schema_version === SCHEMA_VERSION, "authorized vertical request version is unsupported");
  nonempty(request.vertical_id, "authorized vertical vertical_id");
  requireCondition(Array.isArray(request.publications) && request.publications.length === VERTICAL_PROFILES.size, "authorized vertical requires exactly two publications");
  request.publications.forEach(validatePublication);
  requireCondition(
    new Set(request.publications.map((item) => item.operation.profile)).size === VERTICAL_PROFILES.size
      && request.publications.every((item) => VERTICAL_PROFILES.has(item.operation.profile)),
    "authorized vertical requires one publication for each profile",
  );
  const operationIds = new Set();
  for (const publication of request.publications) {
    nonempty(publication.operation.operation_id, "authorized vertical publication operation_id");
    requireCondition(!operationIds.has(publication.operation.operation_id), "authorized vertical publication operation ids must be unique");
    operationIds.add(publication.operation.operation_id);
  }
  requireCondition(
    Array.isArray(request.relevant_revisions)
      && request.relevant_revisions.length >= 1
      && request.relevant_revisions.length <= VERTICAL_PROFILES.size,
    "authorized vertical relevant revisions must contain from 1 through 2 items",
  );
  const selectedOperations = new Set();
  for (const selection of request.relevant_revisions) {
    exactFields(selection, ["publication_operation_id", "selection_reason"], "authorized vertical relevant revision");
    nonempty(selection.publication_operation_id, "authorized vertical relevant revision publication_operation_id");
    nonempty(selection.selection_reason, "authorized vertical relevant revision selection_reason");
    requireCondition(operationIds.has(selection.publication_operation_id), "authorized vertical relevant revision does not name a publication");
    requireCondition(!selectedOperations.has(selection.publication_operation_id), "authorized vertical relevant revisions must be unique");
    selectedOperations.add(selection.publication_operation_id);
  }

  const results = request.publications.map((publication) => {
    const published = store.publish(publication.operation, publication.authority);
    return {
      operation_id: publication.operation.operation_id,
      profile: publication.operation.profile,
      revision_id: published.result_identity,
      idempotent: published.idempotent,
    };
  });
  const revisionByOperation = new Map(results.map((item) => [item.operation_id, item.revision_id]));
  const projectionReceipt = readClaimEvidence(store, {
    schema_version: SCHEMA_VERSION,
    request_id: `${request.vertical_id}:relevant-revisions`,
    operation: "project_relevant_revisions",
    parameters: {
      selections: request.relevant_revisions.map((selection) => ({
        revision_id: revisionByOperation.get(selection.publication_operation_id),
        selection_reason: selection.selection_reason,
      })),
    },
  });
  requireCondition(projectionReceipt.outcome === "succeeded", `authorized vertical builder projection failed: ${projectionReceipt.refusal?.message ?? "unknown refusal"}`);
  return {
    schema_version: SCHEMA_VERSION,
    vertical_id: request.vertical_id,
    publications: results,
    builder_input: {
      schema_version: SCHEMA_VERSION,
      context_kind: "relevant_exact_revisions",
      projection: projectionReceipt.projection,
      relevant_exact_revisions: projectionReceipt.result.relevant_exact_revisions,
    },
  };
}
