import { PERMISSIONS, PROFILES, SCHEMA_VERSION } from "./contract.mjs";
import { digest, revisionId, stableClaimId } from "./identity.mjs";
import {
  exactFields, nonempty, operationPayloadDigest, requireCondition, validateAuthority, validateStore,
} from "./validation.mjs";

export function blankStore(authorities = []) {
  const store = {
    schema_version: SCHEMA_VERSION,
    projection_boundary: {
      actual_content_set: "all records in canonical/store.json",
      source_watermark: null,
      excluded_inputs: [],
      failed_inputs: [],
      freshness: "current_after_verified_rebuild",
      completeness: "available",
    },
    authorities: structuredClone(authorities), claims: [], revisions: [], lineage: [], reliances: [], operations: [],
  };
  validateStore(store);
  return store;
}

export function revisionHeads(store, claimId) {
  const revisions = new Set(store.revisions.filter((item) => item.claim_id === claimId).map((item) => item.id));
  const predecessors = new Set(store.revisions.filter((item) => item.claim_id === claimId && item.predecessor_revision).map((item) => item.predecessor_revision));
  return new Set([...revisions].filter((item) => !predecessors.has(item)));
}

function authorize(store, authority, action, profile) {
  validateAuthority(authority);
  requireCondition(authority.profile === profile, "authority profile mismatch");
  requireCondition(authority.permissions.includes(action), "unauthorized publication");
  const existing = store.authorities.find((item) => item.grant_id === authority.grant_id);
  requireCondition(existing, "authority grant was not admitted by the trusted launcher");
  requireCondition(digest(existing) === digest(authority), "authority grant conflict");
}

function makeRevision(claimId, predecessor, payload, authority) {
  exactFields(payload, ["proposition", "support_qualification", "assumptions", "limitations", "confidence", "evidence_references", "sensitivity_references", "evidence_mode", "judgment_kind", "decision_scope", "profile_payload", "reopening_conditions", "tombstone"], "revision payload");
  const revision = {
    schema_version: SCHEMA_VERSION, claim_id: claimId, predecessor_revision: predecessor,
    ...payload, producer: authority.actor, authority_ref: authority.grant_id,
  };
  revision.id = revisionId(revision);
  return revision;
}

export function applyOperation(inputStore, request, authority) {
  validateStore(inputStore);
  exactFields(request, ["schema_version", "operation_id", "action", "profile", "expected_state", "payload"], "operation");
  requireCondition(request.schema_version === SCHEMA_VERSION && PROFILES.has(request.profile), "unsupported operation");
  requireCondition(PERMISSIONS.has(request.action), "unknown operation action");
  nonempty(request.operation_id, "operation.operation_id");
  authorize(inputStore, authority, request.action, request.profile);
  const payloadSha = operationPayloadDigest(request, authority);
  const prior = inputStore.operations.find((item) => item.operation_id === request.operation_id);
  if (prior) {
    requireCondition(prior.payload_sha256 === payloadSha, "operation identity conflict");
    return { idempotent: true, result_identity: prior.result_identity, store: structuredClone(inputStore) };
  }

  const store = structuredClone(inputStore);
  let payload = structuredClone(request.payload);
  let resultIdentity;
  if (request.action === "create_claim") {
    exactFields(payload, ["subject", "statement_identity", "initial_revision"], "create claim payload");
    requireCondition(request.expected_state === null, "create claim expected state must be null");
    const claimId = stableClaimId(payload.subject);
    requireCondition(payload.initial_revision.decision_scope === authority.decision_scope, "authority decision scope mismatch");
    requireCondition(!store.claims.some((item) => item.id === claimId), "claim identity already exists");
    store.claims.push({
      id: claimId, schema_version: SCHEMA_VERSION, profile: request.profile, subject: payload.subject,
      statement_identity: payload.statement_identity, created_by: authority.actor, authority_ref: authority.grant_id,
    });
    const revision = makeRevision(claimId, null, payload.initial_revision, authority);
    store.revisions.push(revision); resultIdentity = revision.id;
  } else if (request.action === "publish_revision") {
    exactFields(payload, ["claim_id", "revision"], "publish revision payload");
    const claim = store.claims.find((item) => item.id === payload.claim_id);
    requireCondition(claim && claim.profile === request.profile, "claim is missing or profile mismatched");
    requireCondition(payload.revision.decision_scope === authority.decision_scope, "authority decision scope mismatch");
    requireCondition(revisionHeads(store, payload.claim_id).has(request.expected_state), "conflicting predecessor");
    const revision = makeRevision(payload.claim_id, request.expected_state, payload.revision, authority);
    requireCondition(!store.revisions.some((item) => item.id === revision.id), "revision already exists outside idempotent operation");
    store.revisions.push(revision); resultIdentity = revision.id;
  } else if (["publish_lineage", "retract_revision"].includes(request.action)) {
    if (request.action === "retract_revision") payload = { relationship: "retraction", ...payload };
    exactFields(payload, ["relationship", "sources", "target"], "lineage payload");
    requireCondition(Array.isArray(payload.sources) && payload.sources.length > 0 && payload.sources.every((item) => typeof item === "string" && item), "lineage sources must be a nonempty string array");
    nonempty(payload.target, "lineage target");
    const resourceIds = new Set([...payload.sources, payload.target]);
    const selected = store.revisions.filter((item) => resourceIds.has(item.id));
    requireCondition(new Set(selected.map((item) => item.decision_scope)).size === 1 && selected[0]?.decision_scope === authority.decision_scope, "authority decision scope mismatch");
    const claimProfiles = new Map(store.claims.map((item) => [item.id, item.profile]));
    requireCondition(new Set(selected.map((item) => claimProfiles.get(item.claim_id))).size === 1 && claimProfiles.get(selected[0]?.claim_id) === request.profile, "lineage resource profile mismatch");
    const edge = {
      schema_version: SCHEMA_VERSION, relationship: payload.relationship, sources: payload.sources,
      target: payload.target, authority_ref: authority.grant_id, operation_id: request.operation_id,
    };
    edge.id = `lineage@${digest(edge)}`;
    store.lineage.push(edge); resultIdentity = edge.id;
  } else {
    exactFields(payload, ["consumer", "consumer_revision", "decision_scope", "claim_revision_id", "state", "predecessor_reliance"], "reliance payload");
    requireCondition(request.action !== "record_reliance" || payload.state === "active", "new reliance must be active");
    requireCondition(request.action !== "retire_reliance" || ["retired", "superseded"].includes(payload.state), "retirement state is invalid");
    requireCondition(payload.decision_scope === authority.decision_scope, "authority decision scope mismatch");
    if (request.action === "record_reliance") {
      requireCondition(request.expected_state === null && payload.predecessor_reliance === null, "new reliance cannot claim a predecessor");
    } else {
      const predecessor = store.reliances.find((item) => item.id === payload.predecessor_reliance);
      requireCondition(predecessor && request.expected_state === predecessor.id, "conflicting reliance predecessor");
      requireCondition(predecessor.state === "active", "only active reliance can be retired");
      requireCondition(["consumer", "consumer_revision", "decision_scope", "claim_revision_id"].every((key) => payload[key] === predecessor[key]), "reliance retirement changed its exact scope");
    }
    const reliance = { schema_version: SCHEMA_VERSION, ...payload, authority_ref: authority.grant_id, operation_id: request.operation_id };
    reliance.id = `reliance@${digest(reliance)}`;
    store.reliances.push(reliance); resultIdentity = reliance.id;
  }
  store.operations.push({
    operation_id: request.operation_id, action: request.action, payload_sha256: payloadSha,
    result_identity: resultIdentity, authority_ref: authority.grant_id,
  });
  validateStore(store);
  return { idempotent: false, result_identity: resultIdentity, store };
}
