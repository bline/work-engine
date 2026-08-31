import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { createReviewFindingBridge } from "../../../src/services/claim-evidence/review-finding-bridge.mjs";
import { openSqliteClaimEvidenceStore } from "../../../src/services/claim-evidence/sqlite-store.mjs";

const source = {owner: "supervisor", reference: "authority.json", revision: "v1", integrity_sha256: "a".repeat(64), freshness: "current", status: "verified"};
const authority = {schema_version: 1, grant_id: "grant", actor: "review-bridge", profile: "revision-bound-review-finding-v1",
  permissions: ["create_claim", "publish_revision", "record_reliance"], decision_scope: "review-scope", authority_reference: source};
const episode = {identity: {runId: "run", sliceNumber: 1, attemptId: "a", planVersion: "p", reviewObligationId: "o", reviewEpisodeId: "e"}, revision: "b".repeat(64)};
const finding = (status = "open") => ({id: "F-1", severity: "high", title: "Finding", consequence: "Consequence",
  confidence: "high", status});
const result = (status = "open") => ({findings: [finding(status)], limitations: []});

async function fixture(t) {
  const directory = await mkdtemp(path.join(os.tmpdir(), "review-finding-bridge."));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const store = await openSqliteClaimEvidenceStore({filePath: path.join(directory, "claims.sqlite"), bootstrapAuthorities: [authority]});
  t.after(() => store.close());
  return {store, bridge: createReviewFindingBridge({store})};
}

test("finding publication and reliance are idempotent exact-revision operations", async (t) => {
  const {store, bridge} = await fixture(t);
  const initial = bridge.publishFindings({authority, operationPrefix: "initial", episode, result: result()});
  const replay = bridge.publishFindings({authority, operationPrefix: "initial", episode, result: result()});
  assert.deepEqual(replay, initial);
  const relied = bridge.recordReliance({authority, operationId: "rely:F-1", finding: initial[0],
    consumer: "builder", consumerRevision: "candidate-1", decisionScope: "review-scope"});
  const reliedReplay = bridge.recordReliance({authority, operationId: "rely:F-1", finding: initial[0],
    consumer: "builder", consumerRevision: "candidate-1", decisionScope: "review-scope"});
  assert.deepEqual(reliedReplay, relied);
  const updatedEpisode = {...episode, revision: "c".repeat(64)};
  const updated = bridge.publishFindings({authority, operationPrefix: "resolved", episode: updatedEpisode,
    result: result("verified_resolved"), previousFindings: [relied]});
  assert.equal(updated[0].initialRevisionRef.revision, initial[0].revisionRef.revision);
  assert.notEqual(updated[0].revisionRef.revision, initial[0].revisionRef.revision);
  assert.equal(updated[0].relianceRef, null);
  const reliedOnResolved = bridge.recordReliance({authority, operationId: "rely:F-1:resolved", finding: updated[0],
    consumer: "builder", consumerRevision: "candidate-2", decisionScope: "review-scope"});
  assert.notEqual(reliedOnResolved.relianceRef.revision, relied.relianceRef.revision);
  assert.deepEqual(store.exportStore().operations.map(({action}) => action),
    ["create_claim", "record_reliance", "publish_revision", "record_reliance"]);
});

test("finding bridge refuses operation drift and unadmitted authority without changing canonical state", async (t) => {
  const {store, bridge} = await fixture(t);
  bridge.publishFindings({authority, operationPrefix: "initial", episode, result: result()});
  const before = store.exportStore();
  assert.throws(() => bridge.publishFindings({authority, operationPrefix: "initial", episode,
    result: {findings: [{...finding(), consequence: "different"}], limitations: []}}), /operation identity conflict/);
  assert.throws(() => bridge.publishFindings({authority: {...authority, grant_id: "missing"}, operationPrefix: "other", episode, result: result()}), /not admitted/);
  assert.deepEqual(store.exportStore(), before);
});
