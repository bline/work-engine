import assert from "node:assert/strict";
import test from "node:test";
import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { chmod, mkdir, mkdtemp, readFile, rm, stat, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { promisify } from "node:util";

import { createLegacyReviewCompatibilityAdapter, createReviewSubjectService, createSliceCampaignService, openSqliteSliceCampaignStore } from "../src/index.mjs";
import { openPrivateSqliteDatabase } from "../src/services/slice-campaign/sqlite-store.mjs";

const sha = (value) => createHash("sha256").update(value).digest("hex");
const identity = { runId: "skills-migration", sliceNumber: 9, attemptId: "s8-attempt-2", planVersion: "s8-planning-v2" };
const execFileAsync = promisify(execFile);
const workspaceRoot = path.resolve(new URL("../..", import.meta.url).pathname);
const git = async (repository, ...args) => (await execFileAsync("git", ["-C", repository, ...args], { encoding: "utf8" })).stdout.trim();

async function actualBackendFixture(t) {
  const repository = await mkdtemp(path.join(os.tmpdir(), "slice-campaign-vertical."));
  t.after(() => rm(repository, { recursive: true, force: true }));
  await git(repository, "init", "--quiet", "--initial-branch=main");
  await git(repository, "config", "user.name", "Campaign Vertical");
  await git(repository, "config", "user.email", "campaign@example.invalid");
  await writeFile(path.join(repository, "task.py"), "def before():\n    return 1\n");
  await git(repository, "add", "task.py");
  await git(repository, "commit", "--quiet", "-m", "baseline");
  const baseline = await git(repository, "rev-parse", "HEAD");
  await writeFile(path.join(repository, "task.py"), "def after():\n    return 2\n");
  const checkpointDirectory = path.join(repository, "skills/slice-checkpoint/scripts");
  await mkdir(checkpointDirectory, { recursive: true });
  await writeFile(path.join(checkpointDirectory, "checkpoint.py"), await readFile(path.join(workspaceRoot, "skills/slice-checkpoint/scripts/checkpoint.py")));
  return { repository, baseline };
}

test("actual S7 backend atomically binds a canonical candidate to its v2 physical profile", async (t) => {
  const value = await actualBackendFixture(t);
  const reviewSubject = createReviewSubjectService({ workspaceRoot });
  const service = createSliceCampaignService({
    reviewSubject,
    legacyReview: { async review() { return { status: "passed" }; } },
    receiptFinalizer: { async finalize(receipt) { return receipt; } },
  });
  let state = service.admit({ identity, workspace: value.repository,
    acceptedBoundary: { reference: "plan:s8-v10", sha256: sha("plan-v10") },
    baseline: { acceptedCommit: "7da2404", acceptedTree: "b0772d54", interSliceCommit: "2fcdd6c" } });
  state = service.advance({ identity, expectedRevision: state.revision, phase: "implementing", consequence: { artifact: "implementation" } });
  state = service.advance({ identity, expectedRevision: state.revision, phase: "gate_ready", consequence: { gate: "passed" } });
  const beforeBinding = state;
  const request = { schema_version: 1, repository: value.repository, run_id: "skills-migration",
    slice_number: 9, candidate_attempt: 1, baseline_commit_oid: value.baseline,
    parent_checkpoint_commit_oid: null, plan_version: "s8-planning-v10", scope_revision: "candidate-profile-v2",
    gate_receipt_digest: sha("gate"), created_at: "2026-08-29T15:00:00+00:00",
    paths: [{ path: "task.py", action: "include", attribution: "task_owned" }] };
  state = await service.bindCandidate({ identity, expectedRevision: state.revision, request });
  assert.equal(state.candidate.checkpoint_kind, "candidate");
  assert.equal(state.physicalProfile.schema_version, 2);
  assert.equal(state.physicalProfile.analyzer.version, "2");
  assert.equal(state.physicalProfile.subject.construction_method, "slice_checkpoint_candidate_receipt");
  assert.equal(state.physicalProfile.subject.evidence_cutoff, state.candidate.created_at);
  for (const field of ["checkpoint_commit_oid", "checkpoint_tree_oid", "manifest_digest", "task_patch_digest", "ref"])
    assert.equal(state.physicalProfile.subject.checkpoint[field], state.candidate[field]);
  assert.deepEqual(await reviewSubject.createPhysicalProfile({ subject: {
    schema_version: 2, construction_method: "slice_checkpoint_candidate_receipt",
    evidence_cutoff: state.candidate.created_at, checkpoint: state.candidate,
  } }), state.physicalProfile);
  for (const forbidden of ["reviewer_selection", "semantic_risk", "quality", "acceptance", "findings"])
    assert.equal(forbidden in state.physicalProfile, false);

  let refuseProfile = true;
  const failing = createSliceCampaignService({
    reviewSubject: { async createCandidate() { return state.candidate; }, async createPhysicalProfile({ subject }) {
      if (refuseProfile) throw new Error("profile refused");
      return { schema_version: 2, subject };
    } },
    legacyReview: { async review() { return {}; } }, receiptFinalizer: { async finalize(value) { return value; } },
  });
  let failedState = failing.admit({ identity: { ...identity, attemptId: "atomic-failure" }, workspace: `${value.repository}-failure`,
    acceptedBoundary: { reference: "plan:s8-v10", sha256: sha("plan-v10") }, baseline: beforeBinding.baseline });
  failedState = failing.advance({ identity: failedState.identity, expectedRevision: failedState.revision, phase: "implementing", consequence: {} });
  failedState = failing.advance({ identity: failedState.identity, expectedRevision: failedState.revision, phase: "gate_ready", consequence: {} });
  const failedRevision = failedState.revision;
  await assert.rejects(failing.bindCandidate({ identity: failedState.identity, expectedRevision: failedRevision, request }), /profile refused/);
  const reconciled = failing.recover(failedState.identity);
  assert.notEqual(reconciled.revision, failedRevision);
  assert.deepEqual(reconciled.candidate, state.candidate);
  assert.equal(reconciled.physicalProfile, null);
  await assert.rejects(failing.bindCandidate({ identity: failedState.identity, expectedRevision: reconciled.revision,
    request: { ...request, gate_receipt_digest: sha("changed") } }), /conflicts with bound candidate/);
  refuseProfile = false;
  const resumed = await failing.bindCandidate({ identity: failedState.identity, expectedRevision: reconciled.revision, request });
  assert.deepEqual(resumed.candidate, reconciled.candidate);
  assert.equal(resumed.physicalProfile.subject.checkpoint, resumed.candidate);
});

test("SQLite campaign owner survives reconstruction with CAS and exclusive admission", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "slice-campaign-sqlite."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const database = path.join(directory, "campaign.sqlite");
  const owners = {
    reviewSubject: { async createCandidate(request) { return { ...request, created_at: "2026-08-29T15:00:00Z" }; }, async createPhysicalProfile({ subject }) { return { subject }; } },
    legacyReview: { async review() { return { status: "passed" }; } }, receiptFinalizer: { async finalize(value) { return value; } },
  };
  let store = await openSqliteSliceCampaignStore({ filePath: database });
  let service = createSliceCampaignService({ store, ...owners });
  const durableIdentity = { ...identity, attemptId: "durable" };
  let state = service.admit({ identity: durableIdentity, workspace: "/durable-workspace",
    acceptedBoundary: { reference: "plan:durable", sha256: sha("durable") },
    baseline: { acceptedCommit: "accepted", acceptedTree: "tree", interSliceCommit: "inter" } });
  state = service.advance({ identity: durableIdentity, expectedRevision: state.revision, phase: "implementing", consequence: {} });
  store.close();
  store = await openSqliteSliceCampaignStore({ filePath: database });
  t.after(() => store.close());
  service = createSliceCampaignService({ store, ...owners });
  assert.equal(service.recover(durableIdentity).revision, state.revision);
  assert.throws(() => service.admit({ identity: { ...identity, attemptId: "conflict" }, workspace: "/durable-workspace",
    acceptedBoundary: { reference: "plan:conflict", sha256: sha("conflict") }, baseline: state.baseline }), /already has an admitted/);
  assert.throws(() => service.advance({ identity: durableIdentity, expectedRevision: sha("stale"), phase: "gate_ready", consequence: {} }), /revision conflict/);
});

test("SQLite campaign database is private before SQLite opens it and rejects unsafe paths", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "slice-campaign-private-sqlite."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const database = path.join(directory, "campaign.sqlite");
  let observedBeforeOpen = false;
  const previousUmask = process.umask(0);
  try {
    const observed = openPrivateSqliteDatabase(database, class ObservingDatabase {
      close() {}
    }, {
      beforeDatabaseOpen({ stat }) {
        observedBeforeOpen = true;
        assert.equal(stat.isFile(), true);
        assert.equal(stat.mode & 0o777, 0o600);
      },
    });
    observed.close();
  } finally {
    process.umask(previousUmask);
  }
  assert.equal(observedBeforeOpen, true);

  await chmod(database, 0o644);
  let store = await openSqliteSliceCampaignStore({ filePath: database });
  store.close();
  assert.equal((await stat(database)).mode & 0o777, 0o600);

  const link = path.join(directory, "campaign-link.sqlite");
  await symlink(database, link);
  await assert.rejects(openSqliteSliceCampaignStore({ filePath: link }), /regular file/);
});

test("campaign vertical preserves owners, exclusive admission, expected/actual identity, recovery, and terminal release", async () => {
  const calls = [];
  const service = createSliceCampaignService({
    reviewSubject: {
      async createCandidate(request) { calls.push("candidate"); return { commit: request.commit, tree: request.tree, manifestSha256: request.manifestSha256 }; },
      async createPhysicalProfile({ subject }) { calls.push("profile"); return { profileId: sha(subject.checkpoint.commit), subject }; },
    },
    legacyReview: createLegacyReviewCompatibilityAdapter({ invoke: async ({ subject }) => {
      calls.push("legacy-review"); return { status: "passed", subjectCommit: subject.commit };
    } }),
    receiptFinalizer: { async finalize(value) { calls.push("receipt"); return { digest: sha(JSON.stringify(value)) }; } },
    completionOffer: { async offer() { calls.push("offer"); return { status: "open", publicationAuthorized: false }; } },
  });
  let state = service.admit({ identity, workspace: "/workspace", acceptedBoundary: { reference: "plan:s8", sha256: sha("plan") },
    expectedImpact: { reference: "impact:prospective", sha256: sha("expected") },
    baseline: { acceptedCommit: "7da2404", acceptedTree: "b0772d54", interSliceCommit: "2fcdd6c" } });
  assert.equal(state.phase, "accepted");
  assert.throws(() => service.admit({ identity: { ...identity, attemptId: "other" }, workspace: "/workspace",
    acceptedBoundary: { reference: "plan:other", sha256: sha("other") }, baseline: state.baseline }), /already has an admitted/);
  state = service.advance({ identity, expectedRevision: state.revision, phase: "implementing", consequence: { artifact: "implementation" } });
  state = service.advance({ identity, expectedRevision: state.revision, phase: "gate_ready", consequence: { gate: "passed" } });
  state = await service.bindCandidate({ identity, expectedRevision: state.revision,
    request: { commit: "candidate-commit", tree: "candidate-tree", manifestSha256: sha("actual-manifest") } });
  assert.notEqual(state.expectedImpact.sha256, state.candidate.manifestSha256);
  state = service.advance({ identity, expectedRevision: state.revision, phase: "review_ready", consequence: { reviewBoundary: "legacy" } });
  state = await service.runLegacyReview({ identity, expectedRevision: state.revision, selectionPlan: { owner: "slice-supervisor", selections: [] } });
  await assert.rejects(service.runLegacyReview({ identity, expectedRevision: state.revision, selectionPlan: {} }), /without a prior review/);
  state = await service.terminalize({ identity, expectedRevision: state.revision, outcome: "accepted", receipt: { status: "accepted" } });
  await assert.rejects(service.runLegacyReview({ identity, expectedRevision: state.revision, selectionPlan: {} }), /review-ready/);
  assert.equal(service.recover(identity).revision, state.revision);
  assert.equal(state.terminal.completionOffer.publicationAuthorized, false);
  assert.deepEqual(calls, ["candidate", "profile", "legacy-review", "receipt", "offer"]);
});
