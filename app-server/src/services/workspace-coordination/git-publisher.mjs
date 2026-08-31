import { createHash } from "node:crypto";
import { execFileSync, spawnSync } from "node:child_process";
import path from "node:path";
import { digest, freeze, normalizeLease, requireOperationId, requireRecord, requireText } from "./contract.mjs";

function textGit(repository, args) {
  return execFileSync("git", ["-C", repository, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function runGit(repository, args, options = {}) {
  return spawnSync("git", ["-C", repository, ...args], {
    encoding: options.encoding === undefined ? "utf8" : options.encoding,
    input: options.input, env: options.env ?? process.env,
  });
}

function oid(value, label) {
  requireText(value, label);
  if (!/^[0-9a-f]{40,64}$/.test(value)) throw new TypeError(`${label} must be a Git object id`);
  return value;
}

function sha256(value, label) {
  requireText(value, label);
  if (!/^[0-9a-f]{64}$/.test(value)) throw new TypeError(`${label} must be a SHA-256 digest`);
  return value;
}

function manifestActions(entries) {
  if (!Array.isArray(entries) || entries.length === 0) throw new TypeError("publication manifest must be non-empty");
  const result = new Map();
  for (const entry of entries) {
    requireRecord(entry, "publication manifest entry"); requireText(entry.path, "publication manifest path");
    if (path.isAbsolute(entry.path) || entry.path.split("/").some((part) => part === ".." || part === "")) throw new TypeError("publication manifest path is unsafe");
    if (!['include', 'delete'].includes(entry.action) || result.has(entry.path)) throw new TypeError("publication manifest is invalid");
    result.set(entry.path, entry.action);
  }
  return result;
}

function diffActions(repository, from, to) {
  const result = runGit(repository, ["diff", "--name-status", "--no-renames", "-z", from, to, "--"]);
  if (result.status !== 0) throw new Error(result.stderr.trim() || "cannot inspect publication delta");
  const parts = result.stdout.split("\0");
  if (parts.at(-1) !== "" || parts.length % 2 !== 1) throw new Error("cannot interpret publication delta");
  return new Map(Array.from({ length: (parts.length - 1) / 2 }, (_, index) => {
    const status = parts[index * 2]; const file = parts[index * 2 + 1];
    return [file, status === "D" ? "delete" : "include"];
  }));
}

function equalActions(expected, actual, { allowMissing = false } = {}) {
  if (!allowMissing && expected.size !== actual.size) return false;
  return [...actual].every(([file, action]) => expected.get(file) === action);
}

function acceptedCheckpointMetadata(repository, git, checkpoint, commit, tree) {
  const ref = requireText(checkpoint.ref, "accepted checkpoint ref");
  if (!/^refs\/work-engine\/checkpoints\/[A-Za-z0-9][A-Za-z0-9._-]{0,127}\/slice-[1-9][0-9]*\/accepted$/.test(ref)) {
    throw new Error("accepted checkpoint ref is outside the lifecycle namespace");
  }
  if (git.text(repository, ["rev-parse", "--verify", ref]) !== commit) throw new Error("accepted checkpoint ref does not match its commit");
  let metadata;
  try { metadata = JSON.parse(git.text(repository, ["show", "-s", "--format=%B", commit])); }
  catch { throw new Error("accepted checkpoint commit metadata is invalid"); }
  const bindings = {
    work_engine_checkpoint: 1, kind: "accepted", tree,
    task_patch_digest: checkpoint.taskPatchDigest,
    manifest_digest: sha256(checkpoint.manifestDigest, "accepted checkpoint manifest digest"),
    gate_receipt_digest: sha256(checkpoint.gateReceiptDigest, "accepted checkpoint gate receipt digest"),
    plan_version: requireText(checkpoint.planVersion, "accepted checkpoint plan version"),
    scope_revision: requireText(checkpoint.scopeRevision, "accepted checkpoint scope revision"),
  };
  if (Object.entries(bindings).some(([key, value]) => metadata?.[key] !== value)) {
    throw new Error("accepted checkpoint lifecycle metadata mismatch");
  }
  return { ref, metadata };
}

function branchIsCheckedOut(repository, branchRef, read = textGit) {
  return read(repository, ["worktree", "list", "--porcelain"]).split("\n").some((line) => line === `branch ${branchRef}`);
}

export function createCanonicalGitPublisher({ coordination, worktrees, git = { text: textGit, run: runGit } } = {}) {
  if (!coordination || typeof coordination.admitMutation !== "function") throw new TypeError("canonical publisher requires workspace coordination");
  if (!worktrees || typeof worktrees.allocate !== "function") throw new TypeError("canonical publisher requires worktree lifecycle");

  return Object.freeze({
    async publish({
      repository, targetBranch, expectedParent, checkpoint, manifest, authorization,
      lease: leaseValue, operationId, holder, validation, message,
    }) {
      const repositoryRoot = path.resolve(requireText(repository, "publication repository"));
      requireOperationId(operationId); requireText(holder, "publication holder");
      if (!/^[A-Za-z0-9][A-Za-z0-9._/-]*$/.test(targetBranch) || targetBranch.includes("..")) throw new TypeError("publication target branch is unsafe");
      const branchRef = `refs/heads/${targetBranch}`;
      oid(expectedParent, "publication expected parent"); requireRecord(checkpoint, "accepted checkpoint");
      const checkpointCommit = oid(checkpoint.commitOid, "accepted checkpoint commit");
      const checkpointTree = oid(checkpoint.treeOid, "accepted checkpoint tree");
      const baselineCommit = oid(checkpoint.baselineCommitOid, "accepted checkpoint baseline");
      sha256(checkpoint.taskPatchDigest, "accepted checkpoint patch digest");
      const expectedActions = manifestActions(manifest);
      const accepted = acceptedCheckpointMetadata(repositoryRoot, git, checkpoint, checkpointCommit, checkpointTree);
      requireRecord(authorization, "publication authorization");
      if (authorization.decision !== "create" || !Array.isArray(authorization.paths)
          || new Set(authorization.paths).size !== authorization.paths.length
          || authorization.paths.length !== expectedActions.size
          || authorization.paths.some((file) => !expectedActions.has(file))
          || authorization.checkpointCommitOid !== checkpointCommit
          || authorization.checkpointTreeOid !== checkpointTree
          || authorization.targetBranch !== targetBranch) {
        throw new Error("publication authorization does not match the attributed manifest");
      }
      requireText(authorization.reference, "publication authorization reference");
      if (typeof validation !== "function") throw new TypeError("canonical publication requires integrated-tree validation");
      requireRecord(message, "publication message"); requireText(message.subject, "publication message subject");
      if (message.subject.includes("\n") || typeof message.body !== "string") throw new TypeError("publication message is invalid");
      const lease = normalizeLease(leaseValue);
      if (lease.resource.type !== "git-ref" || lease.resource.id !== `${repositoryRoot}#${branchRef}` || lease.holder !== holder) {
        throw new Error("publication lease does not match the target Git ref and holder");
      }
      if (git.text(repositoryRoot, ["rev-parse", "--verify", branchRef]) !== expectedParent) return freeze({ status: "parent_changed" });
      if (branchIsCheckedOut(repositoryRoot, branchRef, git.text)) return freeze({ status: "target_checked_out" });
      if (git.text(repositoryRoot, ["rev-parse", `${checkpointCommit}^{tree}`]) !== checkpointTree) throw new Error("accepted checkpoint tree mismatch");
      const patch = git.run(repositoryRoot, ["diff-tree", "--binary", "--no-renames", "--no-ext-diff",
        `${baselineCommit}^{tree}`, checkpointTree], { encoding: null });
      if (patch.status !== 0 || createHash("sha256").update(patch.stdout).digest("hex") !== checkpoint.taskPatchDigest) {
        throw new Error("accepted checkpoint patch identity mismatch");
      }
      if (!equalActions(expectedActions, diffActions(repositoryRoot, baselineCommit, checkpointCommit))) {
        throw new Error("accepted checkpoint delta does not match the attributed manifest");
      }

      const merge = git.run(repositoryRoot, [
        "merge-tree", "--write-tree", "--messages", `--merge-base=${baselineCommit}`,
        expectedParent, checkpointCommit,
      ]);
      if (merge.status !== 0) return freeze({ status: "semantic_conflict", details: merge.stdout.trim() || merge.stderr.trim() });
      const integratedTree = merge.stdout.split("\n", 1)[0].trim(); oid(integratedTree, "integrated tree");
      const parentTree = git.text(repositoryRoot, ["rev-parse", `${expectedParent}^{tree}`]);
      const integratedActions = diffActions(repositoryRoot, parentTree, integratedTree);
      if (!equalActions(expectedActions, integratedActions, { allowMissing: true })) throw new Error("integration produced unmanifested content");
      if (integratedTree === parentTree) return freeze({ status: "already_integrated", tree: integratedTree });

      const allocation = worktrees.allocate({
        repository: repositoryRoot, operationId: `${operationId}-integration`, holder,
        intentId: `publish:${operationId}`, baselineCommit: expectedParent,
      });
      if (allocation.status !== "allocated") return freeze({ status: "integration_workspace_blocked" });
      let commitOid = null;
      try {
        git.text(allocation.path, ["read-tree", "--reset", "-u", integratedTree]);
        if (git.text(allocation.path, ["write-tree"]) !== integratedTree) throw new Error("integration worktree index does not match merged tree");
        const validationResult = await validation({ worktree: allocation.path, tree: integratedTree, parent: expectedParent });
        requireRecord(validationResult, "integrated-tree validation result");
        if (validationResult.status !== "passed" || validationResult.tree !== integratedTree) {
          git.text(allocation.path, ["reset", "--hard", expectedParent]);
          return freeze({ status: "validation_failed", validation: structuredClone(validationResult) });
        }
        sha256(validationResult.receiptDigest, "integrated-tree validation receipt digest");
        if (git.text(allocation.path, ["write-tree"]) !== integratedTree
            || git.text(allocation.path, ["diff", "--name-only"]) !== ""
            || git.text(allocation.path, ["ls-files", "--others", "--exclude-standard"]) !== "") {
          return freeze({ status: "validation_mutated_integration", path: allocation.path });
        }
        const commitMessage = `${message.subject}${message.body ? `\n\n${message.body}` : ""}\n`;
        const created = git.run(repositoryRoot, ["commit-tree", integratedTree, "-p", expectedParent], { input: commitMessage });
        if (created.status !== 0) throw new Error(created.stderr.trim() || "cannot create publication commit");
        commitOid = created.stdout.trim(); oid(commitOid, "publication commit");
        let admission;
        try {
          admission = coordination.admitMutation({ lease, operationId, mutate: () => {
            if (git.text(repositoryRoot, ["rev-parse", "--verify", branchRef]) !== expectedParent) throw new Error("publication parent changed at mutation admission");
            if (branchIsCheckedOut(repositoryRoot, branchRef, git.text)) throw new Error("publication target became checked out");
            const update = git.run(repositoryRoot, ["update-ref", branchRef, commitOid, expectedParent]);
            if (update.status !== 0) throw new Error("atomic branch publication was rejected");
            return { branchRef, parent: expectedParent, commit: commitOid, tree: integratedTree };
          }});
        } catch (error) {
          if (git.text(repositoryRoot, ["rev-parse", "--verify", branchRef]) === commitOid) {
            git.text(allocation.path, ["reset", "--hard", commitOid]);
            return freeze({ status: "published_unconfirmed", commit: commitOid, tree: integratedTree, reason: error.message });
          }
          throw error;
        }
        git.text(allocation.path, ["reset", "--hard", commitOid]);
        const receipt = {
          schemaVersion: 1, status: "published", commit: commitOid, tree: integratedTree, parent: expectedParent,
          branchRef, fencingToken: lease.fencingToken, validation: structuredClone(validationResult),
          authorizationReference: authorization.reference,
          acceptedCheckpoint: { ...structuredClone(checkpoint), verifiedRef: accepted.ref }, startingTip: expectedParent,
          observedBranchGeneration: { resourceGeneration: lease.fencingToken, tip: commitOid }, admission,
        };
        return freeze({ ...receipt, receiptDigest: digest(receipt) });
      } finally {
        if (commitOid && git.text(allocation.path, ["rev-parse", "HEAD"]) !== commitOid) {
          try { git.text(allocation.path, ["reset", "--hard", commitOid]); } catch {}
        }
        worktrees.cleanup(allocation);
      }
    },
  });
}
