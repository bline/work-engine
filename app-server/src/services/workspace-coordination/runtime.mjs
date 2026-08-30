import { execFileSync } from "node:child_process";
import path from "node:path";
import { freeze, requireOperationId, requireRecord, requireText } from "./contract.mjs";
import { createCanonicalGitPublisher } from "./git-publisher.mjs";
import { createGitWorktreeLifecycle } from "./git-worktree.mjs";
import { createWorkspaceCoordinationService } from "./service.mjs";
import { openSqliteWorkspaceCoordinationStore } from "./sqlite-store.mjs";

function git(repository, args) {
  return execFileSync("git", ["-C", repository, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function branchName(value) {
  requireText(value, "canonical branch");
  if (!/^[A-Za-z0-9][A-Za-z0-9._/-]*$/.test(value) || value.includes("..")) throw new TypeError("canonical branch is unsafe");
  return value;
}

export async function openWorkspaceDevelopmentRuntime({
  repository, runtimeRoot, canonicalBranches, publisherId = "app-server-canonical-publisher",
  storePath = null, worktreeRoot = null,
} = {}) {
  const repositoryRoot = path.resolve(requireText(repository, "workspace repository"));
  if (path.resolve(git(repositoryRoot, ["rev-parse", "--show-toplevel"])) !== repositoryRoot) {
    throw new Error("workspace repository must be the Git worktree root");
  }
  const root = path.resolve(requireText(runtimeRoot, "workspace runtime root"));
  if (!Array.isArray(canonicalBranches) || canonicalBranches.length === 0) throw new TypeError("canonicalBranches must be non-empty");
  const allowedBranches = new Set(canonicalBranches.map(branchName));
  if (allowedBranches.size !== canonicalBranches.length) throw new TypeError("canonicalBranches must be unique");
  requireText(publisherId, "canonical publisher id");

  const store = await openSqliteWorkspaceCoordinationStore({ filePath: storePath ?? path.join(root, "coordination.sqlite") });
  const coordination = createWorkspaceCoordinationService({ store });
  const worktrees = createGitWorktreeLifecycle({ coordination, runtimeRoot: worktreeRoot ?? path.join(root, "worktrees") });
  const publisher = createCanonicalGitPublisher({ coordination, worktrees });
  let closed = false;
  function requireOpen() { if (closed) throw new Error("workspace development runtime is closed"); }

  return Object.freeze({
    repository: repositoryRoot,
    canonicalBranches: freeze([...allowedBranches].sort()),
    allocateAgentWorktree({ operationId, agentId, intentId, baselineCommit, ttlMs } = {}) {
      requireOpen();
      return worktrees.allocate({ repository: repositoryRoot, operationId, holder: requireText(agentId, "agent id"), intentId, baselineCommit, ttlMs });
    },
    cleanupAgentWorktree(allocation) { requireOpen(); return worktrees.cleanup(allocation); },
    async publishAcceptedCheckpoint({ targetBranch, operationId, publicationTtlMs = 3_600_000, ...request } = {}) {
      requireOpen(); requireOperationId(operationId); branchName(targetBranch); requireRecord(request, "publication request");
      if (!allowedBranches.has(targetBranch)) throw new Error("publication target is not a configured canonical branch");
      const admission = coordination.acquire({
        resource: { type: "git-ref", id: `${repositoryRoot}#refs/heads/${targetBranch}` },
        holder: publisherId, intentId: `canonical-publication:${operationId}`, ttlMs: publicationTtlMs,
      });
      if (admission.status !== "acquired") return freeze({ status: "publication_blocked", current: admission.current });
      try {
        const result = await publisher.publish({
          ...request, repository: repositoryRoot, targetBranch, operationId,
          holder: publisherId, lease: admission.lease,
        });
        let releaseStatus = "released";
        try { if (!coordination.release(admission.lease)) releaseStatus = "not_released"; }
        catch { releaseStatus = "release_failed"; }
        return freeze({ ...result, leaseReleaseStatus: releaseStatus });
      } catch (error) {
        try { coordination.release(admission.lease); } catch {}
        throw error;
      }
    },
    inspectResource(resource) { requireOpen(); return coordination.inspect(resource); },
    close() { if (!closed) { closed = true; store.close(); } },
  });
}
