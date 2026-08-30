import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { createImplementationReviewService } from "../../../src/services/implementation-review/service.mjs";
import { digest } from "../../../src/services/review-episode/contract.mjs";
import { createReviewEpisodeService } from "../../../src/services/review-episode/service.mjs";

const sha = (value) => createHash("sha256").update(value).digest("hex");
const fixture = async (name) => JSON.parse(await readFile(new URL(`../../fixtures/implementation-review/${name}.json`, import.meta.url)));
const reference = (owner, reference, revision, value = reference) => ({ owner, reference, revision, sha256: sha(String(value)), freshness: "exact_revision" });
const identity = { runId: "s9", sliceNumber: 1, attemptId: "attempt-1", planVersion: "plan-1", reviewObligationId: "review-1", reviewEpisodeId: "episode-1" };
function authority(subject, generation = 1, predecessorRevision = null, provider = "claude") {
  return { schemaVersion: 1, grantId: `grant-${generation}`, identity,
    source: reference("human", "accepted-review-plan", "plan-1"),
    writer: { actorId: "reviewer", provider, generation, runtimeSession: reference("runtime", `session-${generation}`, `generation-${generation}`) },
    readers: ["reviewer", "builder", "supervisor"],
    initialSubject: { ...reference("checkpoint", "candidate", subject.commit), sha256: digest(subject) }, predecessorRevision };
}

test("review episode preserves retained remediation truth, recovery, replacement, and fences", async () => {
  const implementationReview = createImplementationReviewService();
  const service = createReviewEpisodeService({ implementationReview });
  const initialResult = await fixture("remediation-required");
  const firstAuthority = authority(initialResult.subject);
  let state = service.begin({ authority: firstAuthority, transitionId: "begin" });
  state = service.transition({ authority: firstAuthority, expectedRevision: state.revision,
    transitionId: "initial-result", action: "record_result", payload: { result: initialResult, unresolvedQuestions: [] } });
  assert.equal(state.phase, "remediation");
  const originalRevision = state.revision;
  const remediatedSubject = { commit: "candidate-2", tree: "tree-2", patchIdentity: "patch-2" };
  const remediatedReference = { ...reference("checkpoint", "candidate-2", "candidate-2"), sha256: digest(remediatedSubject) };
  state = service.transition({ authority: firstAuthority, expectedRevision: state.revision,
    transitionId: "remediation", action: "record_remediation_subject", payload: { subject: remediatedReference } });
  const accepted = await fixture("acceptable-as-is");
  accepted.subject = remediatedSubject;
  const resolved = structuredClone(initialResult.findings[0]);
  resolved.status = "verified_resolved";
  resolved.remediationEvidence = accepted.decisiveEvidence;
  accepted.findings = [resolved];
  state = service.transition({ authority: firstAuthority, expectedRevision: state.revision,
    transitionId: "re-evaluation", action: "record_result", payload: { result: accepted, unresolvedQuestions: [] } });
  assert.equal(state.phase, "reported");
  assert.equal(state.currentResult.findings[0].status, "verified_resolved");
  assert.equal(service.read({ identity, revision: originalRevision }).currentResult.verdict, "remediation_required");
  assert.equal(service.history({ identity }).length, 4);

  state = service.transition({ authority: firstAuthority, expectedRevision: state.revision,
    transitionId: "uncertain", action: "mark_uncertain", payload: { reason: "session lost", reconciliationAction: "replace reviewer" } });
  const successor = authority(remediatedSubject, 2, state.revision, "codex");
  state = service.transition({ authority: successor, expectedRevision: state.revision,
    transitionId: "replace", action: "replace_writer", payload: { reason: "session unavailable", pendingAction: "reconcile exact state" } });
  assert.equal(state.continuity, "reconstructed_continuation");
  assert.equal(state.writer.generation, 2);
  assert.throws(() => service.transition({ authority: firstAuthority, expectedRevision: state.revision,
    transitionId: "stale", action: "mark_uncertain", payload: { reason: "stale", reconciliationAction: "stop" } }), /current writer generation/);
  const retired = service.transition({ authority: successor, expectedRevision: state.revision,
    transitionId: "retire", action: "retire", payload: { outcome: "superseded", reason: "closed", protectedReferences: [remediatedReference] } });
  assert.throws(() => service.transition({ authority: successor, expectedRevision: retired.revision,
    transitionId: "reactivate", action: "mark_uncertain", payload: { reason: "x", reconciliationAction: "x" } }), /retired/);
});

test("review episode retries are idempotent and conflicting transitions fail", async () => {
  const result = await fixture("acceptable-as-is");
  const service = createReviewEpisodeService({ implementationReview: createImplementationReviewService() });
  const grant = authority(result.subject);
  const first = service.begin({ authority: grant, transitionId: "begin" });
  const replay = service.begin({ authority: grant, transitionId: "begin" });
  assert.equal(first.revision, replay.revision);
  const reported = service.transition({ authority: grant, expectedRevision: first.revision,
    transitionId: "result", action: "record_result", payload: { result, unresolvedQuestions: [] } });
  const retried = service.transition({ authority: grant, expectedRevision: first.revision,
    transitionId: "result", action: "record_result", payload: { result, unresolvedQuestions: [] } });
  assert.equal(reported.revision, retried.revision);
  assert.throws(() => service.transition({ authority: grant, expectedRevision: reported.revision,
    transitionId: "result", action: "mark_uncertain", payload: { reason: "x", reconciliationAction: "y" } }), /conflicts/);
});
