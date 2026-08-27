import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { promisify } from "node:util";

import { observeGitCheckpoint } from "../../../src/services/claim-evidence/git-checkpoint-observation.mjs";

const execFileAsync = promisify(execFile);

async function checkpoint(t) {
  const repositoryPath = await mkdtemp(path.join(tmpdir(), "git-checkpoint-observation-test-"));
  t.after(async () => rm(repositoryPath, { recursive: true, force: true }));
  await execFileAsync("git", ["init", "--quiet", repositoryPath]);
  await execFileAsync("git", ["-C", repositoryPath, "config", "user.name", "Checkpoint Test"]);
  await execFileAsync("git", ["-C", repositoryPath, "config", "user.email", "checkpoint@example.invalid"]);
  await writeFile(path.join(repositoryPath, "evidence.txt"), "evidence\n");
  await execFileAsync("git", ["-C", repositoryPath, "add", "evidence.txt"]);
  await execFileAsync("git", ["-C", repositoryPath, "commit", "--quiet", "-m", "evidence"]);
  const { stdout: commitOut } = await execFileAsync("git", ["-C", repositoryPath, "rev-parse", "HEAD"]);
  const commit = commitOut.trim();
  const { stdout: treeOut } = await execFileAsync("git", ["-C", repositoryPath, "rev-parse", `${commit}^{tree}`]);
  return { repositoryPath, commit, tree: treeOut.trim() };
}

const reference = {
  owner: "repository", reference: "baseline", revision: "tree:base",
  integrity_sha256: "a".repeat(64), freshness: "current", status: "verified",
};

const request = (checkpointValue, overrides = {}) => ({
  repositoryPath: checkpointValue.repositoryPath,
  expectedCommit: checkpointValue.commit,
  expectedTree: checkpointValue.tree,
  producerIdentity: "app-server:git-checkpoint",
  observedAt: "2026-08-26T12:00:00.000Z",
  origin: { kind: "git_repository", reference: "repository:test", trust_classification: "host_verified_artifact" },
  subject: { namespace: "repository", subject_kind: "checkpoint", stable_subject_id: "test", content_set: ["evidence.txt"] },
  evidenceBaseline: reference,
  executableGeneration: "generation-one",
  adapterVersion: "git-checkpoint-observation-v1",
  ...overrides,
});

test("Git adapter verifies exact opaque commit and tree and is deterministic", async (t) => {
  const value = await checkpoint(t);
  const first = await observeGitCheckpoint(request(value));
  const replay = await observeGitCheckpoint(request(value));
  assert.deepEqual(first, replay);
  assert.equal(first.artifact.checkpoint.commit, value.commit);
  assert.equal(first.artifact.checkpoint.tree, value.tree);
  assert.equal(first.artifact.reference, `git+${first.artifact.checkpoint.object_format}:${value.commit}`);
  assert.match(first.artifact.digest.value, /^[0-9a-f]{64}$/);
  assert.equal("authority_ref" in first, false);
  assert.equal("profile" in first, false);
});

test("Git adapter refuses missing, mistyped, or mismatched checkpoint identity", async (t) => {
  const value = await checkpoint(t);
  await assert.rejects(
    observeGitCheckpoint(request(value, { expectedCommit: "f".repeat(value.commit.length) })),
    /verification failed/,
  );
  await assert.rejects(
    observeGitCheckpoint(request(value, { expectedTree: "e".repeat(value.tree.length) })),
    /tree identity mismatch/,
  );
  await assert.rejects(
    observeGitCheckpoint(request(value, { expectedCommit: `-${value.commit.slice(1)}` })),
    /opaque lowercase hexadecimal/,
  );
});
