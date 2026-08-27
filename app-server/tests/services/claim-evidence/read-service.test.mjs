import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { digest } from "../../../src/services/claim-evidence/identity.mjs";
import { readClaimEvidence } from "../../../src/services/claim-evidence/read-service.mjs";
import { openSqliteClaimEvidenceStore } from "../../../src/services/claim-evidence/sqlite-store.mjs";

const exactReference = (reference = "source") => ({
  owner: "repository", reference, revision: "blob-1", integrity_sha256: "a".repeat(64),
  freshness: "current", status: "verified",
});
const authority = {
  schema_version: 1, grant_id: "grant:reader-test", actor: "producer",
  profile: "proposal-research-v1",
  permissions: ["create_claim", "publish_revision", "publish_lineage", "record_reliance"],
  decision_scope: "proposal-formation", authority_reference: exactReference("authority"),
};
const revision = (proposition) => ({
  proposition, support_qualification: "supported", assumptions: [], limitations: [],
  confidence: { estimate: 1 }, evidence_references: [exactReference()], sensitivity_references: [],
  evidence_mode: "direct_source", judgment_kind: "semantic", decision_scope: "proposal-formation",
  profile_payload: { materiality: "material", support_qualification: "supported" },
  reopening_conditions: [], tombstone: false,
});
const operation = (operationId, action, payload, expectedState = null) => ({
  schema_version: 1, operation_id: operationId, action, profile: "proposal-research-v1",
  expected_state: expectedState, payload,
});
const read = (requestId, operationName, parameters) => ({
  schema_version: 1, request_id: requestId, operation: operationName, parameters,
});

async function temporaryDatabase(t) {
  const directory = await mkdtemp(path.join(tmpdir(), "claim-read-service-test-"));
  t.after(async () => rm(directory, { recursive: true, force: true }));
  return path.join(directory, "claims.sqlite3");
}

function createClaim(store, suffix) {
  return store.publish(operation(`create-${suffix}`, "create_claim", {
    subject: {
      namespace: "research", subject_kind: "proposal", stable_subject_id: suffix,
      evidence_baseline: exactReference(`baseline-${suffix}`), content_set: [`${suffix}.md`],
    },
    statement_identity: `statement-${suffix}`,
    initial_revision: revision(`proposition-${suffix}`),
  }), authority).result_identity;
}

test("persisted read service is exact, bounded, provenance-bearing, and mutation-free", async (t) => {
  const filePath = await temporaryDatabase(t);
  let store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  const first = createClaim(store, "alpha");
  createClaim(store, "beta");
  createClaim(store, "gamma");
  const claimId = store.exportStore().revisions.find((item) => item.id === first).claim_id;
  const second = store.publish(operation("revise-alpha", "publish_revision", {
    claim_id: claimId, revision: revision("newer proposition"),
  }, first), authority).result_identity;
  store.publish(operation("link-alpha", "publish_lineage", {
    relationship: "supersession", sources: [first], target: second,
  }), authority);
  store.publish(operation("rely-alpha", "record_reliance", {
    consumer: "proposal:consumer", consumer_revision: "tree-1", decision_scope: "proposal-formation",
    claim_revision_id: first, state: "active", predecessor_reliance: null,
  }), authority);
  store.close();

  store = await openSqliteClaimEvidenceStore({ filePath });
  t.after(() => store.close());
  const before = store.exportStore();
  const beforeDigest = digest(before);

  const resolved = readClaimEvidence(store, read("resolve-old", "resolve", { identity: first }));
  assert.equal(resolved.outcome, "succeeded");
  assert.equal(resolved.result.kind, "revision");
  assert.equal(resolved.result.revision.id, first);
  assert.notEqual(resolved.result.revision.id, second);

  const criteria = { namespace: "research" };
  const pageOne = readClaimEvidence(store, read("discover-1", "discover", { criteria, limit: 2, cursor: null }));
  assert.equal(pageOne.result.applicability, "not_assessed");
  assert.equal(pageOne.result.candidates.length, 2);
  assert.equal(pageOne.result.page.total_count, 3);
  assert.equal(pageOne.result.page.truncated, true);
  const pageTwo = readClaimEvidence(store, read("discover-2", "discover", {
    criteria, limit: 2, cursor: pageOne.result.page.next_cursor,
  }));
  assert.equal(pageTwo.result.candidates.length, 1);
  assert.equal(pageTwo.result.page.truncated, false);
  assert.deepEqual(
    [...pageOne.result.candidates, ...pageTwo.result.candidates].map((item) => item.claim.id),
    before.claims.map((item) => item.id).sort(),
  );

  const lineage = readClaimEvidence(store, read("lineage", "traverse_lineage", {
    revision_id: second, direction: "predecessors", max_revision_ids: 1, max_lineage_edges: 1,
  }));
  assert.equal(lineage.result.direction, "predecessors");
  assert.equal(lineage.result.bounds.revision_ids.total_count, 2);
  assert.equal(lineage.result.bounds.revision_ids.truncated, true);
  assert.equal(lineage.result.bounds.lineage_edges.truncated, true);

  const direct = readClaimEvidence(store, read("direct", "query_direct_reliance", {
    revision_id: first, limit: 10, cursor: null,
  }));
  const reverse = readClaimEvidence(store, read("reverse", "query_reverse_reliance", {
    consumer: "proposal:consumer", limit: 10, cursor: null,
  }));
  assert.equal(direct.result.query.revision_id, first);
  assert.equal(direct.result.query.consumer, null);
  assert.equal(reverse.result.query.revision_id, null);
  assert.equal(reverse.result.query.consumer, "proposal:consumer");
  assert.deepEqual(direct.result.reliances, reverse.result.reliances);

  for (const response of [resolved, pageOne, pageTwo, lineage, direct, reverse]) {
    assert.equal(response.projection.projection_schema_version, 1);
    assert.equal(typeof response.projection.build_version, "string");
    assert.equal(response.projection.projection_identity.sha256, beforeDigest);
    assert.equal(response.projection.projection_identity.source_watermark, null);
    assert.equal(response.projection.actual_content_set, "all records in canonical/store.json");
    assert.equal(response.projection.freshness, "current_after_verified_rebuild");
    assert.equal(response.projection.completeness, "available");
    assert.deepEqual(response.projection.excluded_inputs, []);
    assert.deepEqual(response.projection.failed_inputs, []);
  }
  assert.equal(digest(store.exportStore()), beforeDigest);
  assert.equal(store.exportStore().operations.length, before.operations.length);
});

test("cursors are bound to operation, query, and canonical projection", async (t) => {
  const filePath = await temporaryDatabase(t);
  const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  t.after(() => store.close());
  createClaim(store, "alpha");
  createClaim(store, "beta");
  const criteria = { namespace: "research" };
  const firstPage = readClaimEvidence(store, read("first", "discover", { criteria, limit: 1, cursor: null }));
  const cursor = firstPage.result.page.next_cursor;
  assert.equal(readClaimEvidence(store, read("wrong-query", "discover", {
    criteria: { stable_subject_id: "alpha" }, limit: 1, cursor,
  })).refusal.message, "cursor query mismatch");
  assert.match(readClaimEvidence(store, read("wrong-operation", "query_reverse_reliance", {
    consumer: "nobody", limit: 1, cursor,
  })).refusal.message, /cursor operation mismatch/);
  createClaim(store, "gamma");
  assert.equal(readClaimEvidence(store, read("stale", "discover", {
    criteria, limit: 1, cursor,
  })).refusal.message, "cursor projection mismatch");
});

test("closed requests and unavailable projections refuse without apparent empty results", () => {
  const snapshot = {
    schema_version: 1,
    projection_boundary: {
      actual_content_set: "bounded fixture", source_watermark: "watermark-1", excluded_inputs: ["excluded"],
      failed_inputs: ["failed"], freshness: "stale", completeness: "partial",
    },
    authorities: [authority], claims: [], revisions: [], lineage: [], reliances: [], operations: [],
  };
  let exports = 0;
  const partialStore = { exportStore() { exports += 1; return structuredClone(snapshot); } };
  const partial = readClaimEvidence(partialStore, read("partial", "discover", {
    criteria: { namespace: "research" }, limit: 1, cursor: null,
  }));
  assert.equal(partial.outcome, "succeeded");
  assert.equal(partial.projection.completeness, "partial");
  assert.equal(partial.projection.freshness, "stale");
  assert.deepEqual(partial.projection.excluded_inputs, ["excluded"]);
  assert.deepEqual(partial.projection.failed_inputs, ["failed"]);
  assert.equal(exports, 1);

  snapshot.projection_boundary.completeness = "unavailable";
  const unavailable = readClaimEvidence(partialStore, read("unavailable", "discover", {
    criteria: { namespace: "research" }, limit: 1, cursor: null,
  }));
  assert.equal(unavailable.outcome, "refused");
  assert.equal(unavailable.result, null);
  assert.equal(unavailable.projection.completeness, "unavailable");
  assert.match(unavailable.refusal.message, /completeness is unavailable/);

  const malformed = readClaimEvidence(partialStore, { schema_version: 1, request_id: "bad", operation: "resolve", parameters: {}, extra: true });
  assert.equal(malformed.outcome, "refused");
  assert.equal(malformed.projection, null);
  assert.match(malformed.refusal.message, /missing or unknown fields/);
  assert.equal(readClaimEvidence(partialStore, read("unknown", "unknown", {})).outcome, "refused");
  assert.equal(readClaimEvidence(partialStore, read("bad-limit", "discover", {
    criteria: { namespace: "research" }, limit: 0, cursor: null,
  })).outcome, "refused");

  const corruptStore = { exportStore() { throw new TypeError("stored claim-evidence state failed its canonical integrity check"); } };
  const corrupt = readClaimEvidence(corruptStore, read("corrupt", "resolve", { identity: "revision" }));
  assert.equal(corrupt.outcome, "refused");
  assert.equal(corrupt.projection, null);
  assert.equal(corrupt.refusal.code, "projection_unavailable");
  assert.match(corrupt.refusal.message, /integrity check/);
});
