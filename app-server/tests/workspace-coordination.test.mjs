import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  createCanonicalGitPublisher, createGitWorktreeLifecycle,
  createWorkspaceCoordinationService, InMemoryWorkspaceCoordinationStore,
  openSqliteWorkspaceCoordinationStore, openWorkspaceDevelopmentRuntime,
} from "../src/index.mjs";

function git(repository, ...args) {
  return execFileSync("git", ["-C", repository, ...args], { encoding: "utf8" }).trim();
}

async function repositoryFixture(t) {
  const root = await mkdtemp(path.join(os.tmpdir(), "workspace-coordination-repo."));
  const runtime = await mkdtemp(path.join(os.tmpdir(), "workspace-coordination-runtime."));
  t.after(async () => { await rm(root, { recursive: true, force: true }); await rm(runtime, { recursive: true, force: true }); });
  git(root, "init", "--quiet", "--initial-branch=human");
  git(root, "config", "user.name", "Workspace Test"); git(root, "config", "user.email", "workspace@example.invalid");
  await writeFile(path.join(root, "base.txt"), "base\n"); git(root, "add", "base.txt"); git(root, "commit", "--quiet", "-m", "base");
  const base = git(root, "rev-parse", "HEAD"); git(root, "branch", "canonical", base);
  git(root, "remote", "add", "origin", "https://invalid.invalid/no-network.git");
  return { root, runtime, base };
}

function patchDigest(repository, baseline, checkpointTree) {
  const result = spawnSync("git", ["-C", repository, "diff-tree", "--binary", "--no-renames", "--no-ext-diff",
    `${baseline}^{tree}`, checkpointTree]);
  assert.equal(result.status, 0);
  return createHash("sha256").update(result.stdout).digest("hex");
}

let acceptedCheckpointOrdinal = 0;
function makeAcceptedCheckpoint(repository, baselineCommit, candidateCommit, treeOid) {
  const ordinal = ++acceptedCheckpointOrdinal;
  const checkpoint = {
    treeOid, baselineCommitOid: baselineCommit,
    taskPatchDigest: patchDigest(repository, baselineCommit, treeOid),
    manifestDigest: createHash("sha256").update(`manifest-${ordinal}`).digest("hex"),
    gateReceiptDigest: createHash("sha256").update(`gate-${ordinal}`).digest("hex"),
    planVersion: `plan-${ordinal}`, scopeRevision: `scope-${ordinal}`,
    ref: `refs/work-engine/checkpoints/test-${ordinal}/slice-1/accepted`,
  };
  const metadata = {
    work_engine_checkpoint: 1, kind: "accepted", tree: treeOid,
    task_patch_digest: checkpoint.taskPatchDigest, manifest_digest: checkpoint.manifestDigest,
    gate_receipt_digest: checkpoint.gateReceiptDigest,
    plan_version: checkpoint.planVersion, scope_revision: checkpoint.scopeRevision,
  };
  const created = spawnSync("git", ["-C", repository, "commit-tree", treeOid, "-p", candidateCommit], {
    encoding: "utf8", input: `${JSON.stringify(metadata)}\n`,
  });
  assert.equal(created.status, 0, created.stderr);
  checkpoint.commitOid = created.stdout.trim();
  git(repository, "update-ref", checkpoint.ref, checkpoint.commitOid);
  return checkpoint;
}

function publicationAuthorization(checkpoint, targetBranch, paths) {
  return {
    decision: "create", reference: "user:test", paths,
    checkpointCommitOid: checkpoint.commitOid, checkpointTreeOid: checkpoint.treeOid, targetBranch,
  };
}

test("SQLite coordination persists generations and rejects stale mutation admission", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "workspace-coordination-sqlite."));
  t.after(async () => rm(directory, { recursive: true, force: true }));
  const filePath = path.join(directory, "coordination.sqlite");
  let instant = Date.parse("2026-08-30T00:00:00Z");
  let store = await openSqliteWorkspaceCoordinationStore({ filePath });
  let service = createWorkspaceCoordinationService({ store, now: () => new Date(instant).toISOString(), newId: (() => { let id = 0; return () => `lease-${++id}`; })() });
  const resource = { type: "git-ref", id: "/repo#refs/heads/canonical" };
  const first = service.acquire({ resource, holder: "agent-a", intentId: "publish-a", ttlMs: 1000 });
  assert.equal(first.status, "acquired"); assert.equal(first.lease.fencingToken, 1);
  assert.equal(service.acquire({ resource, holder: "agent-b", intentId: "publish-b", ttlMs: 1000 }).status, "blocked");
  let asynchronousMutationEntered = false;
  assert.throws(() => service.admitMutation({
    lease: first.lease, operationId: "async-rejected",
    mutate: async () => { asynchronousMutationEntered = true; return { changed: true }; },
  }), /must be synchronous/);
  assert.equal(asynchronousMutationEntered, false);
  assert.throws(() => service.admitMutation({
    lease: first.lease, operationId: "thenable-rejected", mutate: () => Promise.resolve({ changed: true }),
  }), /returned an asynchronous result/);
  assert.equal(service.admitMutation({
    lease: first.lease, operationId: "thenable-rejected", mutate: () => ({ changed: false }),
  }).status, "admitted");
  instant += 1001;
  const second = service.acquire({ resource, holder: "agent-b", intentId: "publish-b", ttlMs: 1000 });
  assert.equal(second.lease.fencingToken, 2);
  assert.throws(() => service.admitMutation({ lease: first.lease, operationId: "stale", mutate: () => ({ changed: true }) }), /superseded/);
  const admitted = service.admitMutation({ lease: second.lease, operationId: "current", mutate: () => ({ changed: true }) });
  assert.equal(admitted.fencingToken, 2);
  instant += 1001;
  assert.throws(() => service.admitMutation({ lease: second.lease, operationId: "expired", mutate: () => ({}) }), /expired/);
  store.close();
  store = await openSqliteWorkspaceCoordinationStore({ filePath }); t.after(() => store.close());
  service = createWorkspaceCoordinationService({ store, now: () => new Date(instant).toISOString() });
  assert.equal(service.inspect(resource).generation, 2);
  assert.throws(() => service.admitMutation({ lease: second.lease, operationId: "current", mutate: () => ({}) }), /already admitted/);
});

test("worktree cleanup recovers after coordination restart and retains dirty commits", async (t) => {
  const fixture = await repositoryFixture(t);
  const databasePath = path.join(fixture.runtime, "coordination.sqlite");
  const worktreeRoot = path.join(fixture.runtime, "worktrees");
  let store = await openSqliteWorkspaceCoordinationStore({ filePath: databasePath });
  let coordination = createWorkspaceCoordinationService({ store });
  let worktrees = createGitWorktreeLifecycle({ coordination, runtimeRoot: worktreeRoot });
  const allocation = worktrees.allocate({ repository: fixture.root, operationId: "restart-cleanup", holder: "builder", intentId: "restart", baselineCommit: fixture.base });
  await writeFile(path.join(allocation.path, "checkpoint.txt"), "checkpoint\n");
  git(allocation.path, "add", "checkpoint.txt"); git(allocation.path, "commit", "--quiet", "-m", "checkpoint");
  const checkpoint = git(allocation.path, "rev-parse", "HEAD");
  await writeFile(path.join(allocation.path, "uncommitted.txt"), "retain me\n");
  const retained = worktrees.cleanup(allocation);
  assert.equal(retained.status, "retained_dirty"); assert.equal(retained.retainedCommit, checkpoint);
  assert.equal(git(fixture.root, "rev-parse", allocation.privateRef), checkpoint);

  await rm(path.join(allocation.path, "uncommitted.txt"));
  store.close();
  store = await openSqliteWorkspaceCoordinationStore({ filePath: databasePath }); t.after(() => store.close());
  coordination = createWorkspaceCoordinationService({ store });
  worktrees = createGitWorktreeLifecycle({ coordination, runtimeRoot: worktreeRoot });
  assert.equal(worktrees.cleanup(allocation).status, "removed");
  assert.equal(coordination.inspect(allocation.lease.resource).lease, null);
  assert.equal(git(fixture.root, "rev-parse", allocation.privateRef), checkpoint);
});

test("failed worktree allocation removes partial checkout and releases its directory lease", async (t) => {
  const fixture = await repositoryFixture(t);
  const coordination = createWorkspaceCoordinationService();
  const repositoryId = createHash("sha256").update(fixture.root).digest("hex").slice(0, 16);
  const expectedPath = path.join(fixture.runtime, repositoryId, "partial");
  const worktrees = createGitWorktreeLifecycle({
    coordination, runtimeRoot: fixture.runtime,
    git: (repository, args) => {
      const result = git(repository, ...args);
      if (repository === expectedPath && args[0] === "rev-parse" && args[1] === "HEAD") throw new Error("injected allocation failure");
      return result;
    },
  });
  assert.throws(() => worktrees.allocate({ repository: fixture.root, operationId: "partial", holder: "builder", intentId: "partial", baselineCommit: fixture.base }), /injected/);
  assert.doesNotMatch(git(fixture.root, "worktree", "list", "--porcelain"), new RegExp(expectedPath.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
  assert.equal(coordination.inspect({ type: "directory", id: expectedPath }).lease, null);
});

test("two local worktrees stage and commit independently without network or human-worktree interference", async (t) => {
  const fixture = await repositoryFixture(t);
  const coordination = createWorkspaceCoordinationService();
  const worktrees = createGitWorktreeLifecycle({ coordination, runtimeRoot: fixture.runtime });
  const first = worktrees.allocate({ repository: fixture.root, operationId: "agent-a", holder: "a", intentId: "slice-a", baselineCommit: fixture.base });
  const second = worktrees.allocate({ repository: fixture.root, operationId: "agent-b", holder: "b", intentId: "slice-b", baselineCommit: fixture.base });
  await writeFile(path.join(first.path, "a.txt"), "a\n"); git(first.path, "add", "a.txt"); git(first.path, "commit", "--quiet", "-m", "a");
  await writeFile(path.join(second.path, "b.txt"), "b\n"); git(second.path, "add", "b.txt"); git(second.path, "commit", "--quiet", "-m", "b");
  assert.notEqual(git(first.path, "rev-parse", "HEAD"), git(second.path, "rev-parse", "HEAD"));
  assert.equal(git(fixture.root, "rev-parse", "HEAD"), fixture.base); assert.equal(git(fixture.root, "status", "--porcelain"), "");
  const firstCommit = git(first.path, "rev-parse", "HEAD");
  assert.equal(worktrees.cleanup(first).status, "removed"); assert.equal(git(fixture.root, "rev-parse", first.privateRef), firstCommit);
  assert.equal(worktrees.cleanup(second).status, "removed");
});

test("App Server runtime assigns worktrees and confines publication to configured canonical branches", async (t) => {
  const fixture = await repositoryFixture(t);
  const runtime = await openWorkspaceDevelopmentRuntime({
    repository: fixture.root, runtimeRoot: fixture.runtime, canonicalBranches: ["canonical"],
  });
  t.after(() => runtime.close());
  const task = runtime.allocateAgentWorktree({ operationId: "runtime-task", agentId: "builder", intentId: "slice", baselineCommit: fixture.base });
  await writeFile(path.join(task.path, "runtime.txt"), "runtime\n"); git(task.path, "add", "runtime.txt"); git(task.path, "commit", "--quiet", "-m", "checkpoint");
  const candidateCommit = git(task.path, "rev-parse", "HEAD"); const checkpointTree = git(task.path, "rev-parse", "HEAD^{tree}");
  runtime.cleanupAgentWorktree(task);
  const checkpoint = makeAcceptedCheckpoint(fixture.root, fixture.base, candidateCommit, checkpointTree);
  const request = {
    operationId: "runtime-publish", expectedParent: fixture.base,
    checkpoint,
    manifest: [{ path: "runtime.txt", action: "include" }],
    authorization: publicationAuthorization(checkpoint, "canonical", ["runtime.txt"]),
    validation: async ({ tree }) => ({ status: "passed", tree, receiptDigest: "e".repeat(64) }),
    message: { subject: "runtime publication", body: "" },
  };
  await assert.rejects(() => runtime.publishAcceptedCheckpoint({ ...request, targetBranch: "human" }), /not a configured canonical branch/);
  const result = await runtime.publishAcceptedCheckpoint({ ...request, targetBranch: "canonical" });
  assert.equal(result.status, "published"); assert.equal(result.leaseReleaseStatus, "released");
  assert.match(result.receiptDigest, /^[0-9a-f]{64}$/);
  assert.equal(result.acceptedCheckpoint.commitOid, checkpoint.commitOid);
  assert.deepEqual(result.observedBranchGeneration, { resourceGeneration: 1, tip: result.commit });
  assert.equal(git(fixture.root, "rev-parse", "canonical"), result.commit);
});

test("canonical publisher integrates, validates, fences, and preserves the checked-out human branch", async (t) => {
  const fixture = await repositoryFixture(t);
  const coordination = createWorkspaceCoordinationService();
  const worktrees = createGitWorktreeLifecycle({ coordination, runtimeRoot: fixture.runtime });
  const task = worktrees.allocate({ repository: fixture.root, operationId: "task", holder: "builder", intentId: "task", baselineCommit: fixture.base });
  await writeFile(path.join(task.path, "task.txt"), "accepted\n"); git(task.path, "add", "task.txt"); git(task.path, "commit", "--quiet", "-m", "checkpoint");
  const candidateCommit = git(task.path, "rev-parse", "HEAD"); const checkpointTree = git(task.path, "rev-parse", "HEAD^{tree}");
  worktrees.cleanup(task);
  const checkpoint = makeAcceptedCheckpoint(fixture.root, fixture.base, candidateCommit, checkpointTree);
  const lease = coordination.acquire({
    resource: { type: "git-ref", id: `${fixture.root}#refs/heads/canonical` },
    holder: "publisher", intentId: "publish-task", ttlMs: 60_000,
  }).lease;
  const publisher = createCanonicalGitPublisher({ coordination, worktrees });
  const validationTrees = [];
  const result = await publisher.publish({
    repository: fixture.root, targetBranch: "canonical", expectedParent: fixture.base,
    checkpoint,
    manifest: [{ path: "task.txt", action: "include" }],
    authorization: publicationAuthorization(checkpoint, "canonical", ["task.txt"]),
    lease, operationId: "publish-task", holder: "publisher",
    validation: async ({ worktree, tree }) => {
      validationTrees.push(tree); assert.equal(await readFile(path.join(worktree, "task.txt"), "utf8"), "accepted\n");
      return { status: "passed", tree, receiptDigest: "a".repeat(64) };
    },
    message: { subject: "publish task", body: "validated integrated tree" },
  });
  assert.equal(result.status, "published"); assert.deepEqual(validationTrees, [result.tree]);
  assert.equal(git(fixture.root, "rev-parse", "canonical"), result.commit);
  assert.equal(git(fixture.root, "rev-parse", "human"), fixture.base);
  assert.equal(git(fixture.root, "status", "--porcelain"), "");
  assert.equal(git(fixture.root, "show", "canonical:task.txt"), "accepted");
});

test("publication reconciles unrelated branch advancement without touching a dirty human index", async (t) => {
  const fixture = await repositoryFixture(t);
  const coordination = createWorkspaceCoordinationService();
  const worktrees = createGitWorktreeLifecycle({ coordination, runtimeRoot: fixture.runtime });
  const task = worktrees.allocate({ repository: fixture.root, operationId: "reconcile-task", holder: "builder", intentId: "task", baselineCommit: fixture.base });
  await writeFile(path.join(task.path, "task.txt"), "task\n"); git(task.path, "add", "task.txt"); git(task.path, "commit", "--quiet", "-m", "checkpoint");
  const candidateCommit = git(task.path, "rev-parse", "HEAD"); const checkpointTree = git(task.path, "rev-parse", "HEAD^{tree}"); worktrees.cleanup(task);
  const checkpoint = makeAcceptedCheckpoint(fixture.root, fixture.base, candidateCommit, checkpointTree);

  const advance = worktrees.allocate({ repository: fixture.root, operationId: "unrelated-advance", holder: "other", intentId: "other", baselineCommit: fixture.base });
  await writeFile(path.join(advance.path, "unrelated.txt"), "other\n"); git(advance.path, "add", "unrelated.txt"); git(advance.path, "commit", "--quiet", "-m", "unrelated");
  const advanced = git(advance.path, "rev-parse", "HEAD"); worktrees.cleanup(advance);
  git(fixture.root, "update-ref", "refs/heads/canonical", advanced, fixture.base);

  await writeFile(path.join(fixture.root, "human.txt"), "human staged work\n"); git(fixture.root, "add", "human.txt");
  const beforeIndex = git(fixture.root, "write-tree"); const beforeHead = git(fixture.root, "rev-parse", "HEAD");
  const lease = coordination.acquire({ resource: { type: "git-ref", id: `${fixture.root}#refs/heads/canonical` }, holder: "publisher", intentId: "reconcile", ttlMs: 60_000 }).lease;
  const publisher = createCanonicalGitPublisher({ coordination, worktrees });
  const result = await publisher.publish({
    repository: fixture.root, targetBranch: "canonical", expectedParent: advanced,
    checkpoint,
    manifest: [{ path: "task.txt", action: "include" }], authorization: publicationAuthorization(checkpoint, "canonical", ["task.txt"]),
    lease, operationId: "reconcile", holder: "publisher",
    validation: async ({ worktree, tree }) => {
      assert.equal(await readFile(path.join(worktree, "task.txt"), "utf8"), "task\n");
      assert.equal(await readFile(path.join(worktree, "unrelated.txt"), "utf8"), "other\n");
      return { status: "passed", tree, receiptDigest: "c".repeat(64) };
    },
    message: { subject: "reconcile task", body: "" },
  });
  assert.equal(result.status, "published"); assert.equal(result.parent, advanced);
  assert.equal(git(fixture.root, "rev-parse", "HEAD"), beforeHead); assert.equal(git(fixture.root, "write-tree"), beforeIndex);
  assert.equal(git(fixture.root, "diff", "--cached", "--name-only"), "human.txt");
  assert.equal(git(fixture.root, "show", "canonical:task.txt"), "task");
  assert.equal(git(fixture.root, "show", "canonical:unrelated.txt"), "other");
});

test("publication uses the declared checkpoint baseline across equivalent non-ancestral histories", async (t) => {
  const fixture = await repositoryFixture(t);
  const coordination = createWorkspaceCoordinationService();
  const worktrees = createGitWorktreeLifecycle({ coordination, runtimeRoot: fixture.runtime });

  const task = worktrees.allocate({
    repository: fixture.root, operationId: "private-lineage-task", holder: "builder",
    intentId: "task", baselineCommit: fixture.base,
  });
  await writeFile(path.join(task.path, "task.txt"), "task\n");
  git(task.path, "add", "task.txt"); git(task.path, "commit", "--quiet", "-m", "checkpoint");
  const candidateCommit = git(task.path, "rev-parse", "HEAD");
  const checkpointTree = git(task.path, "rev-parse", "HEAD^{tree}");
  worktrees.cleanup(task);
  const checkpoint = makeAcceptedCheckpoint(fixture.root, fixture.base, candidateCommit, checkpointTree);

  const publicBaselineResult = spawnSync(
    "git", ["-C", fixture.root, "commit-tree", `${fixture.base}^{tree}`],
    { encoding: "utf8", input: "equivalent public baseline\n" },
  );
  assert.equal(publicBaselineResult.status, 0, publicBaselineResult.stderr);
  const publicBaseline = publicBaselineResult.stdout.trim();
  const advance = worktrees.allocate({
    repository: fixture.root, operationId: "public-lineage-advance", holder: "other",
    intentId: "other", baselineCommit: publicBaseline,
  });
  await writeFile(path.join(advance.path, "unrelated.txt"), "public lineage\n");
  git(advance.path, "add", "unrelated.txt"); git(advance.path, "commit", "--quiet", "-m", "public advance");
  const advanced = git(advance.path, "rev-parse", "HEAD");
  worktrees.cleanup(advance);
  git(fixture.root, "update-ref", "refs/heads/canonical", advanced, fixture.base);

  const lease = coordination.acquire({
    resource: { type: "git-ref", id: `${fixture.root}#refs/heads/canonical` },
    holder: "publisher", intentId: "non-ancestral", ttlMs: 60_000,
  }).lease;
  const result = await createCanonicalGitPublisher({ coordination, worktrees }).publish({
    repository: fixture.root, targetBranch: "canonical", expectedParent: advanced,
    checkpoint,
    manifest: [{ path: "task.txt", action: "include" }],
    authorization: publicationAuthorization(checkpoint, "canonical", ["task.txt"]),
    lease, operationId: "non-ancestral-publish", holder: "publisher",
    validation: async ({ worktree, tree }) => {
      assert.equal(await readFile(path.join(worktree, "task.txt"), "utf8"), "task\n");
      assert.equal(await readFile(path.join(worktree, "unrelated.txt"), "utf8"), "public lineage\n");
      return { status: "passed", tree, receiptDigest: "f".repeat(64) };
    },
    message: { subject: "publish across equivalent histories", body: "" },
  });
  assert.equal(result.status, "published");
  assert.equal(result.parent, advanced);
  assert.equal(git(fixture.root, "show", "canonical:task.txt"), "task");
  assert.equal(git(fixture.root, "show", "canonical:unrelated.txt"), "public lineage");
});

test("publisher refuses checked-out targets, wrong-resource leases, and semantic conflicts", async (t) => {
  const fixture = await repositoryFixture(t);
  const coordination = createWorkspaceCoordinationService();
  const worktrees = createGitWorktreeLifecycle({ coordination, runtimeRoot: fixture.runtime });
  const task = worktrees.allocate({ repository: fixture.root, operationId: "conflict-task", holder: "builder", intentId: "task", baselineCommit: fixture.base });
  await writeFile(path.join(task.path, "base.txt"), "task version\n"); git(task.path, "add", "base.txt"); git(task.path, "commit", "--quiet", "-m", "checkpoint");
  const candidateCommit = git(task.path, "rev-parse", "HEAD"); const checkpointTree = git(task.path, "rev-parse", "HEAD^{tree}"); worktrees.cleanup(task);
  const checkpoint = makeAcceptedCheckpoint(fixture.root, fixture.base, candidateCommit, checkpointTree);
  const publisher = createCanonicalGitPublisher({ coordination, worktrees });
  const wrong = coordination.acquire({ resource: { type: "git-ref", id: `${fixture.root}#refs/heads/other` }, holder: "publisher", intentId: "wrong", ttlMs: 60_000 }).lease;
  const common = {
    repository: fixture.root, expectedParent: fixture.base,
    checkpoint,
    manifest: [{ path: "base.txt", action: "include" }], authorization: publicationAuthorization(checkpoint, "canonical", ["base.txt"]),
    operationId: "conflict-publish", holder: "publisher", validation: async ({ tree }) => ({ status: "passed", tree, receiptDigest: "b".repeat(64) }),
    message: { subject: "publish conflict", body: "" },
  };
  await assert.rejects(() => publisher.publish({ ...common, targetBranch: "canonical", lease: wrong }), /does not match/);
  const humanLease = coordination.acquire({ resource: { type: "git-ref", id: `${fixture.root}#refs/heads/human` }, holder: "publisher", intentId: "human", ttlMs: 60_000 }).lease;
  assert.equal((await publisher.publish({
    ...common, targetBranch: "human", lease: humanLease,
    authorization: publicationAuthorization(checkpoint, "human", ["base.txt"]),
  })).status, "target_checked_out");

  const canonicalLease = coordination.acquire({ resource: { type: "git-ref", id: `${fixture.root}#refs/heads/canonical` }, holder: "publisher", intentId: "canonical", ttlMs: 60_000 }).lease;
  const candidateOnly = { ...checkpoint, commitOid: candidateCommit, ref: "refs/work-engine/checkpoints/test-candidate/slice-1/candidate-1" };
  await assert.rejects(() => publisher.publish({
    ...common, checkpoint: candidateOnly, targetBranch: "canonical", lease: canonicalLease,
    authorization: publicationAuthorization(candidateOnly, "canonical", ["base.txt"]),
  }), /outside the lifecycle namespace/);
  assert.equal((await publisher.publish({ ...common, expectedParent: checkpoint.commitOid, targetBranch: "canonical", lease: canonicalLease })).status, "parent_changed");
  await assert.rejects(() => publisher.publish({
    ...common, targetBranch: "canonical", lease: canonicalLease,
    manifest: [{ path: "not-the-checkpoint.txt", action: "include" }],
    authorization: publicationAuthorization(checkpoint, "canonical", ["not-the-checkpoint.txt"]),
  }), /checkpoint delta does not match/);

  await writeFile(path.join(fixture.root, "base.txt"), "branch version\n"); git(fixture.root, "add", "base.txt"); git(fixture.root, "commit", "--quiet", "-m", "branch change");
  const advanced = git(fixture.root, "rev-parse", "HEAD"); git(fixture.root, "update-ref", "refs/heads/canonical", advanced, fixture.base);
  git(fixture.root, "reset", "--hard", fixture.base);
  assert.equal((await publisher.publish({ ...common, expectedParent: advanced, targetBranch: "canonical", lease: canonicalLease })).status, "semantic_conflict");
  assert.equal(git(fixture.root, "rev-parse", "canonical"), advanced);
});

test("publisher refuses validation mutations and does not move the canonical ref", async (t) => {
  const fixture = await repositoryFixture(t);
  const coordination = createWorkspaceCoordinationService();
  const worktrees = createGitWorktreeLifecycle({ coordination, runtimeRoot: fixture.runtime });
  const task = worktrees.allocate({ repository: fixture.root, operationId: "mutation-task", holder: "builder", intentId: "task", baselineCommit: fixture.base });
  await writeFile(path.join(task.path, "task.txt"), "task\n"); git(task.path, "add", "task.txt"); git(task.path, "commit", "--quiet", "-m", "checkpoint");
  const candidateCommit = git(task.path, "rev-parse", "HEAD"); const checkpointTree = git(task.path, "rev-parse", "HEAD^{tree}"); worktrees.cleanup(task);
  const checkpoint = makeAcceptedCheckpoint(fixture.root, fixture.base, candidateCommit, checkpointTree);
  const lease = coordination.acquire({ resource: { type: "git-ref", id: `${fixture.root}#refs/heads/canonical` }, holder: "publisher", intentId: "mutation", ttlMs: 60_000 }).lease;
  const result = await createCanonicalGitPublisher({ coordination, worktrees }).publish({
    repository: fixture.root, targetBranch: "canonical", expectedParent: fixture.base,
    checkpoint,
    manifest: [{ path: "task.txt", action: "include" }], authorization: publicationAuthorization(checkpoint, "canonical", ["task.txt"]),
    lease, operationId: "mutation-publish", holder: "publisher",
    validation: async ({ worktree, tree }) => {
      await writeFile(path.join(worktree, "validation-output.txt"), "unexpected\n");
      return { status: "passed", tree, receiptDigest: "d".repeat(64) };
    },
    message: { subject: "refuse mutation", body: "" },
  });
  assert.equal(result.status, "validation_mutated_integration");
  assert.equal(git(fixture.root, "rev-parse", "canonical"), fixture.base);
  git(fixture.root, "worktree", "remove", "--force", result.path);
});
