import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { lstatSync, mkdirSync, rmSync } from "node:fs";
import path from "node:path";
import { digest, freeze, normalizeLease, requireOperationId, requireRecord, requireText } from "./contract.mjs";

function defaultGit(repository, args) {
  return execFileSync("git", ["-C", repository, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function oid(value, label) {
  requireText(value, label);
  if (!/^[0-9a-f]{40,64}$/.test(value)) throw new TypeError(`${label} must be a Git object id`);
  return value;
}

function repositoryIdentity(repository) {
  return createHash("sha256").update(repository).digest("hex").slice(0, 16);
}

export function createGitWorktreeLifecycle({ coordination, runtimeRoot, git = defaultGit } = {}) {
  if (!coordination || typeof coordination.acquire !== "function" || typeof coordination.release !== "function"
      || typeof coordination.inspect !== "function") {
    throw new TypeError("Git worktree lifecycle requires workspace coordination");
  }
  const root = path.resolve(requireText(runtimeRoot, "worktree runtime root"));
  mkdirSync(root, { recursive: true, mode: 0o700 });
  if (!lstatSync(root).isDirectory() || lstatSync(root).isSymbolicLink()) throw new Error("worktree runtime root must be a real directory");

  return Object.freeze({
    allocate({ repository, operationId, holder, intentId, baselineCommit, ttlMs = 3_600_000 }) {
      const repositoryRoot = path.resolve(requireText(repository, "worktree repository"));
      if (path.resolve(git(repositoryRoot, ["rev-parse", "--show-toplevel"])) !== repositoryRoot) {
        throw new Error("worktree repository must be the Git worktree root");
      }
      requireOperationId(operationId); oid(baselineCommit, "worktree baseline commit");
      const resolvedBaseline = git(repositoryRoot, ["rev-parse", "--verify", `${baselineCommit}^{commit}`]);
      if (resolvedBaseline !== baselineCommit) throw new Error("worktree baseline did not resolve exactly");
      const repositoryId = repositoryIdentity(repositoryRoot);
      const worktreePath = path.join(root, repositoryId, operationId);
      mkdirSync(path.dirname(worktreePath), { recursive: true, mode: 0o700 });
      const admission = coordination.acquire({
        resource: { type: "directory", id: worktreePath }, holder, intentId, ttlMs,
      });
      if (admission.status !== "acquired") return freeze({ status: "blocked", current: admission.current });
      try {
        git(repositoryRoot, ["worktree", "add", "--detach", worktreePath, baselineCommit]);
        const head = git(worktreePath, ["rev-parse", "HEAD"]);
        if (head !== baselineCommit) throw new Error("allocated worktree HEAD does not match its baseline");
        const privateRef = `refs/work-engine/workspaces/${repositoryId}/${operationId}`;
        git(repositoryRoot, ["update-ref", privateRef, head]);
        return freeze({
          status: "allocated", schemaVersion: 1, repository: repositoryRoot, repositoryId,
          operationId, path: worktreePath, baselineCommit, baselineTree: git(worktreePath, ["rev-parse", "HEAD^{tree}"]),
          privateRef, lease: admission.lease,
        });
      } catch (error) {
        try { git(repositoryRoot, ["worktree", "remove", "--force", worktreePath]); } catch {}
        rmSync(worktreePath, { recursive: true, force: true });
        coordination.release(admission.lease);
        throw error;
      }
    },
    cleanup(value) {
      requireRecord(value, "worktree allocation");
      if (value.status !== "allocated") throw new TypeError("cleanup requires an allocated worktree receipt");
      const worktreePath = path.resolve(value.path);
      if (worktreePath !== path.join(root, value.repositoryId, value.operationId)) throw new Error("worktree allocation path is outside its namespace");
      const status = git(worktreePath, ["status", "--porcelain=v2", "--untracked-files=all"]);
      const head = git(worktreePath, ["rev-parse", "HEAD"]);
      git(value.repository, ["update-ref", value.privateRef, head]);
      if (status) return freeze({ status: "retained_dirty", path: worktreePath, retainedCommit: head, privateRef: value.privateRef });
      git(value.repository, ["worktree", "remove", "--force", worktreePath]);
      const released = coordination.release(value.lease);
      return freeze({ status: "removed", path: worktreePath, retainedCommit: head, privateRef: value.privateRef, released });
    },
    rebindLease(value, leaseValue) {
      requireRecord(value, "worktree allocation");
      const keys = ["baselineCommit", "baselineTree", "lease", "operationId", "path", "privateRef", "repository", "repositoryId", "schemaVersion", "status"];
      if (JSON.stringify(Object.keys(value).sort()) !== JSON.stringify(keys.sort())
          || value.status !== "allocated" || value.schemaVersion !== 1) {
        throw new TypeError("lease rebinding requires an exact allocated worktree receipt");
      }
      const repositoryRoot = path.resolve(requireText(value.repository, "worktree allocation repository"));
      if (path.resolve(git(repositoryRoot, ["rev-parse", "--show-toplevel"])) !== repositoryRoot) {
        throw new Error("worktree allocation repository must be the Git worktree root");
      }
      requireOperationId(value.operationId, "worktree allocation operation");
      const repositoryId = repositoryIdentity(repositoryRoot);
      const worktreePath = path.resolve(requireText(value.path, "worktree allocation path"));
      if (value.repositoryId !== repositoryId
          || worktreePath !== path.join(root, repositoryId, value.operationId)
          || value.privateRef !== `refs/work-engine/workspaces/${repositoryId}/${value.operationId}`) {
        throw new Error("worktree allocation is outside its lifecycle namespace");
      }
      oid(value.baselineCommit, "worktree allocation baseline commit");
      oid(value.baselineTree, "worktree allocation baseline tree");
      normalizeLease(value.lease);
      const lease = normalizeLease(leaseValue);
      if (lease.resource.type !== "directory" || path.resolve(lease.resource.id) !== worktreePath) {
        throw new Error("replacement lease does not fence the allocated worktree");
      }
      const current = coordination.inspect(lease.resource);
      if (!current?.lease || current.generation !== lease.fencingToken
          || digest(current.lease) !== digest(lease)) {
        throw new Error("worktree lease is absent or superseded");
      }
      if (Date.parse(lease.expiresAt) <= Date.now()) throw new Error("worktree lease is expired");
      return freeze({...structuredClone(value), path: worktreePath, lease: structuredClone(lease)});
    },
  });
}
