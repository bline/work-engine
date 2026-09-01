import { execFileSync } from "node:child_process";
import path from "node:path";
import { digest, freeze, requireOperationId, requireRecord, requireText } from "./contract.mjs";
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

function publicationRequestDigest({ repository, targetBranch, operationId, holder, request }) {
  return digest({ repository, targetBranch, operationId, holder, ...structuredClone(request) });
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
    acquireResource({ resource, holder, intentId, ttlMs } = {}) {
      requireOpen(); return coordination.acquire({ resource, holder, intentId, ttlMs });
    },
    releaseResource(lease) { requireOpen(); return coordination.release(lease); },
    allocateAgentWorktree({ operationId, agentId, intentId, baselineCommit, ttlMs } = {}) {
      requireOpen();
      return worktrees.allocate({ repository: repositoryRoot, operationId, holder: requireText(agentId, "agent id"), intentId, baselineCommit, ttlMs });
    },
    cleanupAgentWorktree(allocation) { requireOpen(); return worktrees.cleanup(allocation); },
    preparePublication({ targetBranch, operationId, ...request } = {}) {
      requireOpen(); requireOperationId(operationId); branchName(targetBranch); requireRecord(request, "publication request");
      if (!allowedBranches.has(targetBranch)) throw new Error("publication target is not a configured canonical branch");
      const current = coordination.loadPublication(operationId);
      if (current) {
        const source = current.record.status === "published" ? current.record.sealed : current.record;
        const requested = publicationRequestDigest({
          repository: repositoryRoot, targetBranch, operationId, holder: publisherId, request,
        });
        if (source.requestDigest !== requested) throw new Error("publication operation conflicts with its durable request binding");
        return current;
      }
      const prepared = publisher.prepare({
        ...request, repository: repositoryRoot, targetBranch, operationId, holder: publisherId,
      });
      if (prepared.status !== "prepared") return freeze({ revision: null, record: prepared });
      return coordination.savePublication({ operationId, record: prepared });
    },
    sealPublication({ operationId, preparationRevision, validation } = {}) {
      requireOpen(); requireOperationId(operationId); requireText(preparationRevision, "preparation revision");
      const current = coordination.loadPublication(operationId);
      if (!current || current.revision !== preparationRevision || current.record.status !== "prepared") {
        throw new Error("prepared publication revision is absent, stale, or not sealable");
      }
      const sealed = publisher.sealValidation({ preparation: current.record, validation });
      if (sealed.status !== "sealed") return freeze({ revision: current.revision, record: sealed });
      return coordination.savePublication({ operationId, expectedRevision: current.revision, record: sealed });
    },
    promotePublication({ operationId, preparedRevision, publicationTtlMs = 3_600_000 } = {}) {
      requireOpen(); requireOperationId(operationId); requireText(preparedRevision, "prepared publication revision");
      const current = coordination.loadPublication(operationId);
      if (!current || current.revision !== preparedRevision) throw new Error("sealed publication revision is absent or stale");
      if (current.record.status === "published") return current;
      if (current.record.status !== "sealed") throw new Error("prepared publication is not sealed");
      const sealed = current.record;
      const admission = coordination.acquire({
        resource: { type: "git-ref", id: `${repositoryRoot}#${sealed.branchRef}` },
        holder: publisherId, intentId: `canonical-publication:${operationId}`, ttlMs: publicationTtlMs,
      });
      if (admission.status !== "acquired") return freeze({ status: "publication_blocked", current: admission.current });
      try {
        const result = publisher.promote({ sealed, lease: admission.lease, operationId, holder: publisherId });
        if (["published", "published_observed", "published_unconfirmed"].includes(result.status)) {
          return coordination.savePublication({
            operationId, expectedRevision: current.revision,
            record: { schemaVersion: 1, status: "published", operationId, sealed, publication: result },
          });
        }
        return result;
      } finally { try { coordination.release(admission.lease); } catch {} }
    },
    reconcilePublication({ operationId, preparedRevision } = {}) {
      requireOpen(); requireOperationId(operationId); requireText(preparedRevision, "prepared publication revision");
      const current = coordination.loadPublication(operationId);
      if (!current || current.revision !== preparedRevision) throw new Error("publication revision is absent or stale");
      if (current.record.status === "published") return current;
      if (current.record.status !== "sealed") throw new Error("publication is not sealed");
      const result = publisher.reconcile({ sealed: current.record, operationId });
      if (["published_observed", "published_unconfirmed"].includes(result.status)) {
        return coordination.savePublication({
          operationId, expectedRevision: current.revision,
          record: { schemaVersion: 1, status: "published", operationId,
            sealed: current.record, publication: result },
        });
      }
      return result;
    },
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
    inspectPublication(operationId) { requireOpen(); return coordination.loadPublication(operationId); },
    close() { if (!closed) { closed = true; store.close(); } },
  });
}
