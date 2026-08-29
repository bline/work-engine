import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { promisify } from "node:util";

import { createReviewSubjectService } from "../../../src/services/review-subject/service.mjs";

const execFileAsync = promisify(execFile);
const workspaceRoot = path.resolve(new URL("../../../..", import.meta.url).pathname);

async function git(repository, ...args) {
  return (await execFileAsync("git", ["-C", repository, ...args], { encoding: "utf8" })).stdout.trim();
}

async function fixture(t) {
  const repository = await mkdtemp(path.join(os.tmpdir(), "review-subject-service."));
  t.after(() => rm(repository, { recursive: true, force: true }));
  await git(repository, "init", "--quiet", "--initial-branch=main");
  await git(repository, "config", "user.name", "Review Subject Test");
  await git(repository, "config", "user.email", "review-subject@example.invalid");
  await writeFile(path.join(repository, ".gitignore"), "ignored.txt\n.env\n");
  await writeFile(path.join(repository, "task.py"), "def before():\n    return 1\n");
  await writeFile(path.join(repository, "overlap.txt"), "baseline\n");
  await git(repository, "add", ".gitignore", "task.py", "overlap.txt");
  await git(repository, "commit", "--quiet", "-m", "baseline");
  const baseline = await git(repository, "rev-parse", "HEAD");
  return { repository, baseline };
}

async function snapshot(repository) {
  const indexPath = await git(repository, "rev-parse", "--git-path", "index");
  const index = path.isAbsolute(indexPath) ? indexPath : path.join(repository, indexPath);
  return {
    branch: await git(repository, "branch", "--show-current"),
    head: await git(repository, "rev-parse", "HEAD"),
    index: createHash("sha256").update(await readFile(index)).digest("hex"),
    status: await git(repository, "status", "--porcelain=v2", "--untracked-files=all"),
  };
}

function request({ repository, baseline, attempt = 1 }) {
  return {
    schema_version: 1,
    repository,
    run_id: "review-subject-test",
    slice_number: 7,
    candidate_attempt: attempt,
    baseline_commit_oid: baseline,
    parent_checkpoint_commit_oid: null,
    plan_version: "s7-test-plan",
    scope_revision: "s7-test-scope",
    gate_receipt_digest: "a".repeat(64),
    created_at: "2026-08-29T12:00:00+00:00",
    paths: [
      { path: "task.py", action: "include", attribution: "task_owned" },
      { path: "overlap.txt", action: "include", attribution: "pre_existing_overlap" },
    ],
  };
}

test("host service produces immutable attributed subject and deterministic physical profile", async (t) => {
  const value = await fixture(t);
  await writeFile(path.join(value.repository, "user-staged.txt"), "user staged\n");
  await git(value.repository, "add", "user-staged.txt");
  await writeFile(path.join(value.repository, "task.py"), "def after():\n    return 2\n");
  await writeFile(path.join(value.repository, "overlap.txt"), "declared overlap\n");
  await writeFile(path.join(value.repository, "unrelated.txt"), "unrelated\n");
  await writeFile(path.join(value.repository, "ignored.txt"), "ignored\n");
  await writeFile(path.join(value.repository, ".env"), "SECRET=value\n");
  const before = await snapshot(value.repository);
  const service = createReviewSubjectService({ workspaceRoot });

  const candidate = await service.createCandidate(request(value));
  assert.deepEqual(await snapshot(value.repository), before);
  assert.equal(candidate.checkpoint_kind, "candidate");
  assert.deepEqual(candidate.paths.map(({ path: item, attribution }) => [item, attribution]), [
    ["task.py", "task_owned"], ["overlap.txt", "pre_existing_overlap"],
  ]);
  const accepted = await service.transitionCandidate({ candidate, kind: "accepted" });
  assert.deepEqual(await snapshot(value.repository), before);
  assert.deepEqual(
    await service.validateCheckpoint({ receipt: accepted, kind: "accepted", requirePaths: true }),
    accepted,
  );
  const stoppedCandidate = await service.createCandidate(request({ ...value, attempt: 2 }));
  const stopped = await service.transitionCandidate({ candidate: stoppedCandidate, kind: "stopped" });
  assert.deepEqual(await snapshot(value.repository), before);
  assert.deepEqual(
    await service.validateCheckpoint({ receipt: stopped, kind: "stopped", requirePaths: true }),
    stopped,
  );
  const subject = {
    schema_version: 1,
    construction_method: "full_slice_checkpoint_lifecycle_receipt",
    evidence_cutoff: "2026-08-29T12:01:00+00:00",
    checkpoint: accepted,
  };
  const first = await service.createPhysicalProfile({ subject });
  const second = await service.createPhysicalProfile({ subject });
  assert.deepEqual(second, first);
  assert.equal(first.subject.checkpoint.checkpoint_commit_oid, accepted.checkpoint_commit_oid);
  assert.equal(first.subject.checkpoint.checkpoint_tree_oid, accepted.checkpoint_tree_oid);
  assert.equal(first.subject.checkpoint.manifest_digest, accepted.manifest_digest);
  assert.equal(first.subject.checkpoint.task_patch_digest, accepted.task_patch_digest);
  assert.deepEqual(Object.keys(first.observations).sort(), [
    "changed_symbols", "configuration_file_count", "documentation_file_count", "file_categories",
    "file_count", "files", "hunk_count", "line_totals", "module_distribution", "test_file_count",
  ]);
  for (const forbidden of ["reviewer_selection", "semantic_risk", "quality", "acceptance", "findings"])
    assert.equal(forbidden in first, false);
  await assert.rejects(
    service.invoke({ operation: "select_reviewer", input: {} }),
    /unsupported review-subject operation/,
  );
  const refs = (await git(value.repository, "for-each-ref", "--format=%(refname)")).split("\n").filter(Boolean);
  assert.equal(refs.every((ref) => ref === "refs/heads/main" || ref.startsWith("refs/work-engine/checkpoints/")), true);
});

test("service refuses stale refs, tampered subjects, and non-private authority", async (t) => {
  const value = await fixture(t);
  await writeFile(path.join(value.repository, "task.py"), "def changed():\n    return 3\n");
  const service = createReviewSubjectService({ workspaceRoot });
  const candidate = await service.createCandidate(request(value));
  const accepted = await service.transitionCandidate({ candidate, kind: "accepted" });
  const competing = await service.createCandidate(request({ ...value, attempt: 2 }));
  await assert.rejects(
    service.transitionCandidate({
      candidate: competing, kind: "accepted", expectedAccepted: candidate.checkpoint_commit_oid,
    }),
    /ref conflict/,
  );
  const subject = {
    schema_version: 1,
    construction_method: "full_slice_checkpoint_lifecycle_receipt",
    evidence_cutoff: "2026-08-29T12:01:00+00:00",
    checkpoint: structuredClone(accepted),
  };
  subject.checkpoint.paths[0].attribution = "validation_dependency";
  await assert.rejects(service.createPhysicalProfile({ subject }), /path attribution manifest/);
});
