import assert from "node:assert/strict";
import test from "node:test";

import { applyOperation, blankStore } from "../../../src/services/claim-evidence/service.mjs";
import { buildProjection, queryReliance, resolveRecord, traverse } from "../../../src/services/claim-evidence/projections.mjs";
import { validateStore } from "../../../src/services/claim-evidence/validation.mjs";

const exactReference = (reference = "source") => ({
  owner: "repository", reference, revision: "blob-1", integrity_sha256: "a".repeat(64),
  freshness: "current", status: "verified",
});
const authority = {
  schema_version: 1, grant_id: "grant:producer", actor: "producer",
  profile: "proposal-research-v1",
  permissions: ["create_claim", "publish_revision", "publish_lineage", "record_reliance", "retire_reliance"],
  decision_scope: "proposal-formation", authority_reference: exactReference("authority"),
};
const revision = (overrides = {}) => ({
  proposition: "A bounded proposition", support_qualification: "supported",
  assumptions: [], limitations: [], confidence: { estimate: 1 },
  evidence_references: [exactReference()], sensitivity_references: [],
  evidence_mode: "direct_source", judgment_kind: "semantic",
  decision_scope: "proposal-formation",
  profile_payload: { materiality: "material", support_qualification: "supported" },
  reopening_conditions: [], tombstone: false, ...overrides,
});
const operation = (operationId, action, payload, expectedState = null) => ({
  schema_version: 1, operation_id: operationId, action,
  profile: "proposal-research-v1", expected_state: expectedState, payload,
});

test("vertical claim lifecycle is exact, queryable, idempotent, and failure-atomic", () => {
  const initial = blankStore([authority]);
  const created = applyOperation(initial, operation("create", "create_claim", {
    subject: {
      namespace: "research", subject_kind: "proposal", stable_subject_id: "placement",
      evidence_baseline: exactReference("baseline"), content_set: ["proposal.md"],
    },
    statement_identity: "Claims are service-owned",
    initial_revision: revision(),
  }), authority);
  assert.equal(initial.claims.length, 0);
  const first = created.result_identity;

  const replay = applyOperation(created.store, operation("create", "create_claim", {
    subject: {
      namespace: "research", subject_kind: "proposal", stable_subject_id: "placement",
      evidence_baseline: exactReference("baseline"), content_set: ["proposal.md"],
    },
    statement_identity: "Claims are service-owned",
    initial_revision: revision(),
  }), authority);
  assert.equal(replay.idempotent, true);
  assert.deepEqual(replay.store, created.store);

  const revised = applyOperation(created.store, operation("revise", "publish_revision", {
    claim_id: created.store.claims[0].id,
    revision: revision({ proposition: "Claims are owned by the provider-neutral service" }),
  }, first), authority);
  const second = revised.result_identity;
  const linked = applyOperation(revised.store, operation("supersede", "publish_lineage", {
    relationship: "supersession", sources: [first], target: second,
  }), authority);
  const relied = applyOperation(linked.store, operation("rely", "record_reliance", {
    consumer: "proposal:one", consumer_revision: "tree-1", decision_scope: "proposal-formation",
    claim_revision_id: first, state: "active", predecessor_reliance: null,
  }), authority);
  const activeReliance = relied.result_identity;
  const retired = applyOperation(relied.store, operation("retire", "retire_reliance", {
    consumer: "proposal:one", consumer_revision: "tree-1", decision_scope: "proposal-formation",
    claim_revision_id: first, state: "retired", predecessor_reliance: activeReliance,
  }, activeReliance), authority);

  const projection = buildProjection(retired.store);
  assert.equal(resolveRecord(projection, first).authority.grant_id, authority.grant_id);
  assert.deepEqual(new Set(traverse(projection, second, "predecessors").revision_ids), new Set([first, second]));
  assert.equal(queryReliance(projection, null, "proposal:one").reliances.length, 2);
  assert.equal(projection.completeness, "available");

  const mistypedPredecessor = structuredClone(retired.store);
  mistypedPredecessor.reliances.at(-1).predecessor_reliance = mistypedPredecessor.claims[0].id;
  assert.throws(() => validateStore(mistypedPredecessor), /reliance predecessor is dangling/);

  const beforeFailure = structuredClone(retired.store);
  assert.throws(() => applyOperation(retired.store, operation("bad-edge", "publish_lineage", {
    relationship: "derivation", sources: [second], target: second,
  }), authority), /cyclic lineage/);
  assert.deepEqual(retired.store, beforeFailure);
});
