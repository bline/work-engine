import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { createReviewFindingBridge } from "../../../src/services/claim-evidence/review-finding-bridge.mjs";
import { projectReviewerClaimContext, renderReviewerClaimContext } from "../../../src/services/claim-evidence/reviewer-projection.mjs";
import { openSqliteClaimEvidenceStore } from "../../../src/services/claim-evidence/sqlite-store.mjs";

const ref = {owner: "supervisor", reference: "authority.json", revision: "v1", integrity_sha256: "a".repeat(64), freshness: "current", status: "verified"};
const authority = {schema_version: 1, grant_id: "grant", actor: "producer", profile: "revision-bound-review-finding-v1",
  permissions: ["create_claim"], decision_scope: "scope", authority_reference: ref};
const episode = {identity: {runId: "r", sliceNumber: 1, attemptId: "a", planVersion: "p", reviewObligationId: "o", reviewEpisodeId: "e"}, revision: "b".repeat(64)};

test("reviewer projection is bounded, authority-free, and decision-scope exact", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "reviewer-projection."));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const store = await openSqliteClaimEvidenceStore({filePath: path.join(directory, "claims.sqlite"), bootstrapAuthorities: [authority]});
  t.after(() => store.close());
  const [published] = createReviewFindingBridge({store}).publishFindings({authority, operationPrefix: "finding", episode,
    result: {findings: [{id: "F-1", severity: "high", title: "Finding", consequence: "Consequence", confidence: "high", status: "open"}], limitations: []}});
  const request = {schema_version: 1, request_id: "projection", consumer: {identity: "reviewer", revision: "candidate", decision_scope: "scope"},
    selections: [{revision_id: published.revisionRef.revision, selection_reason: "Exact prior finding"}],
    limitations: ["This context does not select, evaluate, or accept a finding."]};
  const context = projectReviewerClaimContext(store, request);
  assert.equal(context.relevant_exact_revisions.length, 1);
  assert.equal(context.authority.reviewAcceptanceAuthorized, false);
  assert.equal("permissions" in context.relevant_exact_revisions[0], false);
  assert.equal("operations" in context, false);
  assert.equal("store" in context, false);
  assert.match(renderReviewerClaimContext(context), /^WORK_ENGINE_REVIEWER_CLAIM_EVIDENCE_V1\n/);
  assert.throws(() => projectReviewerClaimContext(store, {...request,
    consumer: {...request.consumer, decision_scope: "other"}}), /decision scope mismatch/);
  assert.throws(() => projectReviewerClaimContext(store, {...request, limitations: []}), /requires limitations/);
});
