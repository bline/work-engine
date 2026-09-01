import { createHash } from "node:crypto";
import { execFileSync, spawnSync } from "node:child_process";
import path from "node:path";
import { digest, freeze, normalizeLease, requireOperationId, requireRecord, requireText } from "./contract.mjs";

function textGit(repository, args) { return execFileSync("git", ["-C", repository, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim(); }
function runGit(repository, args, options = {}) { return spawnSync("git", ["-C", repository, ...args], { encoding: options.encoding === undefined ? "utf8" : options.encoding, input: options.input, env: options.env ?? process.env }); }
function oid(value, label) { requireText(value, label); if (!/^[0-9a-f]{40,64}$/.test(value)) throw new TypeError(`${label} must be a Git object id`); return value; }
function sha256(value, label) { requireText(value, label); if (!/^[0-9a-f]{64}$/.test(value)) throw new TypeError(`${label} must be a SHA-256 digest`); return value; }
function exactKeys(value, keys, label) { requireRecord(value, label); if (JSON.stringify(Object.keys(value).sort()) !== JSON.stringify([...keys].sort())) throw new TypeError(`${label} fields are invalid`); return value; }

function manifestActions(entries) {
  if (!Array.isArray(entries) || entries.length === 0) throw new TypeError("publication manifest must be non-empty");
  const result = new Map();
  for (const entry of entries) {
    exactKeys(entry, ["action", "path"], "publication manifest entry"); requireText(entry.path, "publication manifest path");
    if (path.isAbsolute(entry.path) || entry.path.split("/").some((part) => part === ".." || part === "")) throw new TypeError("publication manifest path is unsafe");
    if (!["include", "delete"].includes(entry.action) || result.has(entry.path)) throw new TypeError("publication manifest is invalid");
    result.set(entry.path, entry.action);
  }
  return result;
}
function diffActions(repository, from, to, run = runGit) {
  const result = run(repository, ["diff", "--name-status", "--no-renames", "-z", from, to, "--"]);
  if (result.status !== 0) throw new Error(result.stderr.trim() || "cannot inspect publication delta");
  const parts = result.stdout.split("\0");
  if (parts.at(-1) !== "" || parts.length % 2 !== 1) throw new Error("cannot interpret publication delta");
  return new Map(Array.from({ length: (parts.length - 1) / 2 }, (_, index) => [parts[index * 2 + 1], parts[index * 2] === "D" ? "delete" : "include"]));
}
function equalActions(expected, actual, { allowMissing = false } = {}) { return (allowMissing || expected.size === actual.size) && [...actual].every(([file, action]) => expected.get(file) === action); }
function acceptedCheckpointMetadata(repository, git, checkpoint, commit, tree) {
  const ref = requireText(checkpoint.ref, "accepted checkpoint ref");
  if (!/^refs\/work-engine\/checkpoints\/[A-Za-z0-9][A-Za-z0-9._-]{0,127}\/slice-[1-9][0-9]*\/accepted$/.test(ref)) throw new Error("accepted checkpoint ref is outside the lifecycle namespace");
  if (git.text(repository, ["rev-parse", "--verify", ref]) !== commit) throw new Error("accepted checkpoint ref does not match its commit");
  let metadata; try { metadata = JSON.parse(git.text(repository, ["show", "-s", "--format=%B", commit])); } catch { throw new Error("accepted checkpoint commit metadata is invalid"); }
  const bindings = { work_engine_checkpoint: 1, kind: "accepted", tree, task_patch_digest: checkpoint.taskPatchDigest, manifest_digest: sha256(checkpoint.manifestDigest, "accepted checkpoint manifest digest"), gate_receipt_digest: sha256(checkpoint.gateReceiptDigest, "accepted checkpoint gate receipt digest"), plan_version: requireText(checkpoint.planVersion, "accepted checkpoint plan version"), scope_revision: requireText(checkpoint.scopeRevision, "accepted checkpoint scope revision") };
  if (Object.entries(bindings).some(([key, value]) => metadata?.[key] !== value)) throw new Error("accepted checkpoint lifecycle metadata mismatch");
  return { ref };
}
function checkoutPaths(repository, branchRef, read = textGit) {
  const result = []; let worktree = null;
  for (const line of read(repository, ["worktree", "list", "--porcelain"]).split("\n")) {
    if (line.startsWith("worktree ")) worktree = line.slice(9);
    else if (line === `branch ${branchRef}` && worktree !== null) result.push(worktree);
    else if (line === "") worktree = null;
  }
  return result.sort();
}
function withDigest(value, field) { const record = structuredClone(value); record[field] = digest(record); return freeze(record); }
function validateDigestRecord(value, field, label) { requireRecord(value, label); sha256(value[field], `${label} digest`); const projected = structuredClone(value); delete projected[field]; if (digest(projected) !== value[field]) throw new Error(`${label} digest mismatch`); return value; }
function normalizeValidation(value, tree) {
  exactKeys(value, ["gateResult", "profile", "receiptDigest", "requirements", "schemaVersion", "status", "tree"], "integrated-tree validation receipt");
  if (value.schemaVersion !== 1 || value.status !== "passed" || value.tree !== tree) throw new Error("integrated-tree validation did not pass for the prepared tree");
  requireText(value.profile, "integrated-tree validation profile"); requireRecord(value.gateResult, "integrated-tree validation gate result");
  if (!Array.isArray(value.requirements) || value.requirements.length === 0 || value.requirements.some((item) => typeof item !== "string" || !item.trim()) || new Set(value.requirements).size !== value.requirements.length) throw new TypeError("integrated-tree validation requirements must be unique non-empty strings");
  const projected = structuredClone(value); delete projected.receiptDigest;
  if (sha256(value.receiptDigest, "integrated-tree validation receipt digest") !== digest(projected)) throw new Error("integrated-tree validation receipt digest mismatch");
  return freeze(structuredClone(value));
}
function publicationReceipt(sealed, admission, status = "published") {
  if (!admission || admission.operationId !== sealed.operationId || admission.result?.commit !== sealed.commit) {
    throw new Error("publication admission does not prove the observed branch mutation");
  }
  return withDigest({ schemaVersion: 2, status, operationId: sealed.operationId, commit: sealed.commit, tree: sealed.tree, parent: sealed.expectedParent, branchRef: sealed.branchRef, fencingToken: admission.fencingToken, validation: structuredClone(sealed.validation), authorizationReference: sealed.authorization.reference, acceptedCheckpoint: structuredClone(sealed.acceptedCheckpoint), manifest: structuredClone(sealed.manifest), startingTip: sealed.expectedParent, preparedRevision: sealed.sealedDigest, observedBranchGeneration: { resourceGeneration: admission.fencingToken, tip: sealed.commit }, admission: structuredClone(admission) }, "receiptDigest");
}
function unconfirmedPublication(sealed, reason) {
  return freeze({ schemaVersion: 1, status: "published_unconfirmed", operationId: sealed.operationId, branchRef: sealed.branchRef, commit: sealed.commit, tree: sealed.tree, preparedRevision: sealed.sealedDigest, reason });
}

export function createCanonicalGitPublisher({ coordination, worktrees, git = { text: textGit, run: runGit } } = {}) {
  if (!coordination || typeof coordination.admitMutation !== "function" || typeof coordination.inspectAdmission !== "function") throw new TypeError("canonical publisher requires workspace coordination");
  if (!worktrees || typeof worktrees.allocate !== "function" || typeof worktrees.cleanup !== "function") throw new TypeError("canonical publisher requires worktree lifecycle");
  const publisher = {
    prepare({ repository, targetBranch, expectedParent, checkpoint, manifest, authorization, operationId, holder, message }) {
      const repositoryRoot = path.resolve(requireText(repository, "publication repository")); requireOperationId(operationId); requireText(holder, "publication holder");
      if (!/^[A-Za-z0-9][A-Za-z0-9._/-]*$/.test(targetBranch) || targetBranch.includes("..")) throw new TypeError("publication target branch is unsafe");
      const branchRef = `refs/heads/${targetBranch}`; oid(expectedParent, "publication expected parent"); requireRecord(checkpoint, "accepted checkpoint");
      const checkpointCommit = oid(checkpoint.commitOid, "accepted checkpoint commit"); const checkpointTree = oid(checkpoint.treeOid, "accepted checkpoint tree"); const baselineCommit = oid(checkpoint.baselineCommitOid, "accepted checkpoint baseline"); sha256(checkpoint.taskPatchDigest, "accepted checkpoint patch digest");
      const expectedActions = manifestActions(manifest); const accepted = acceptedCheckpointMetadata(repositoryRoot, git, checkpoint, checkpointCommit, checkpointTree);
      exactKeys(authorization, ["checkpointCommitOid", "checkpointTreeOid", "decision", "paths", "reference", "targetBranch"], "publication authorization");
      if (authorization.decision !== "create" || !Array.isArray(authorization.paths) || new Set(authorization.paths).size !== authorization.paths.length || authorization.paths.length !== expectedActions.size || authorization.paths.some((file) => !expectedActions.has(file)) || authorization.checkpointCommitOid !== checkpointCommit || authorization.checkpointTreeOid !== checkpointTree || authorization.targetBranch !== targetBranch) throw new Error("publication authorization does not match the attributed manifest");
      requireText(authorization.reference, "publication authorization reference"); exactKeys(message, ["body", "subject"], "publication message"); requireText(message.subject, "publication message subject"); if (message.subject.includes("\n") || typeof message.body !== "string") throw new TypeError("publication message is invalid");
      if (git.text(repositoryRoot, ["rev-parse", "--verify", branchRef]) !== expectedParent) return freeze({ status: "parent_changed" });
      if (git.text(repositoryRoot, ["rev-parse", `${checkpointCommit}^{tree}`]) !== checkpointTree) throw new Error("accepted checkpoint tree mismatch");
      const patch = git.run(repositoryRoot, ["diff-tree", "--binary", "--no-renames", "--no-ext-diff", `${baselineCommit}^{tree}`, checkpointTree], { encoding: null });
      if (patch.status !== 0 || createHash("sha256").update(patch.stdout).digest("hex") !== checkpoint.taskPatchDigest) throw new Error("accepted checkpoint patch identity mismatch");
      if (!equalActions(expectedActions, diffActions(repositoryRoot, baselineCommit, checkpointCommit, git.run))) throw new Error("accepted checkpoint delta does not match the attributed manifest");
      const merge = git.run(repositoryRoot, ["merge-tree", "--write-tree", "--messages", `--merge-base=${baselineCommit}`, expectedParent, checkpointCommit]);
      if (merge.status !== 0) return freeze({ status: "semantic_conflict", details: merge.stdout.trim() || merge.stderr.trim() });
      const tree = merge.stdout.split("\n", 1)[0].trim(); oid(tree, "integrated tree"); const parentTree = git.text(repositoryRoot, ["rev-parse", `${expectedParent}^{tree}`]);
      if (!equalActions(expectedActions, diffActions(repositoryRoot, parentTree, tree, git.run), { allowMissing: true })) throw new Error("integration produced unmanifested content");
      if (tree === parentTree) return freeze({ status: "already_integrated", tree });
      const allocation = worktrees.allocate({ repository: repositoryRoot, operationId: `${operationId}-integration`, holder, intentId: `publish:${operationId}`, baselineCommit: expectedParent });
      if (allocation.status !== "allocated") return freeze({ status: "integration_workspace_blocked" });
      try {
        git.text(allocation.path, ["read-tree", "--reset", "-u", tree]); if (git.text(allocation.path, ["write-tree"]) !== tree) throw new Error("integration worktree index does not match merged tree");
        const requestDigest = digest({ repository: repositoryRoot, targetBranch, operationId, holder, expectedParent, checkpoint: structuredClone(checkpoint), manifest: structuredClone(manifest), authorization: structuredClone(authorization), message: structuredClone(message) });
        return withDigest({ schemaVersion: 1, status: "prepared", operationId, repository: repositoryRoot, targetBranch, branchRef, expectedParent, tree, requestDigest, acceptedCheckpoint: { ...structuredClone(checkpoint), verifiedRef: accepted.ref }, manifest: structuredClone(manifest).sort((a, b) => a.path.localeCompare(b.path)), authorization: structuredClone(authorization), message: structuredClone(message), holder, allocation: structuredClone(allocation) }, "preparationDigest");
      } catch (error) { try { worktrees.cleanup(allocation); } catch {} throw error; }
    },
    sealValidation({ preparation, validation }) {
      validateDigestRecord(preparation, "preparationDigest", "publication preparation"); if (preparation.status !== "prepared") throw new Error("publication preparation is not sealable"); const normalized = normalizeValidation(validation, preparation.tree); const allocation = preparation.allocation;
      if (git.text(allocation.path, ["write-tree"]) !== preparation.tree || git.text(allocation.path, ["diff", "--name-only"]) !== "" || git.text(allocation.path, ["ls-files", "--others", "--exclude-standard"]) !== "") return freeze({ status: "validation_mutated_integration", path: allocation.path, retained: worktrees.cleanup(allocation) });
      const created = git.run(preparation.repository, ["commit-tree", preparation.tree, "-p", preparation.expectedParent], { input: `${preparation.message.subject}${preparation.message.body ? `\n\n${preparation.message.body}` : ""}\n` });
      if (created.status !== 0) throw new Error(created.stderr.trim() || "cannot create publication commit"); const commit = created.stdout.trim(); oid(commit, "publication commit"); git.text(allocation.path, ["reset", "--hard", commit]); const cleanup = worktrees.cleanup(allocation);
      if (cleanup.status !== "removed" || git.text(preparation.repository, ["rev-parse", "--verify", allocation.privateRef]) !== commit) throw new Error("sealed publication private ref was not retained");
      const projected = structuredClone(preparation); delete projected.preparationDigest; delete projected.allocation;
      return withDigest({ ...projected, schemaVersion: 2, status: "sealed", commit, privateRef: allocation.privateRef, preparationDigest: preparation.preparationDigest, validation: structuredClone(normalized), cleanup: structuredClone(cleanup) }, "sealedDigest");
    },
    promote({ sealed, lease: leaseValue, operationId, holder }) {
      validateDigestRecord(sealed, "sealedDigest", "sealed publication"); requireOperationId(operationId); requireText(holder, "publication holder"); if (sealed.operationId !== operationId || sealed.holder !== holder) throw new Error("sealed publication operation or holder mismatch");
      if (git.text(sealed.repository, ["rev-parse", "--verify", sealed.privateRef]) !== sealed.commit) throw new Error("sealed publication private ref mismatch");
      const lease = normalizeLease(leaseValue); if (lease.resource.type !== "git-ref" || lease.resource.id !== `${sealed.repository}#${sealed.branchRef}` || lease.holder !== holder) throw new Error("publication lease does not match the target Git ref and holder");
      const observed = git.text(sealed.repository, ["rev-parse", "--verify", sealed.branchRef]);
      if (observed === sealed.commit) {
        const existingAdmission = coordination.inspectAdmission(operationId);
        return existingAdmission ? publicationReceipt(sealed, existingAdmission, "published_observed") : unconfirmedPublication(sealed, "mutation admission is unavailable");
      }
      if (observed !== sealed.expectedParent) return freeze({ status: "parent_changed", observed });
      const checkedOut = checkoutPaths(sealed.repository, sealed.branchRef, git.text); if (checkedOut.length) return freeze({ status: "checkout_transition_required", branchRef: sealed.branchRef, expectedParent: sealed.expectedParent, preparedCommit: sealed.commit, preparedRevision: sealed.sealedDigest, checkedOut });
      let admission;
      try { admission = coordination.admitMutation({ lease, operationId, mutate: () => { if (git.text(sealed.repository, ["rev-parse", "--verify", sealed.branchRef]) !== sealed.expectedParent) throw new Error("publication parent changed at mutation admission"); if (checkoutPaths(sealed.repository, sealed.branchRef, git.text).length) throw new Error("publication target became checked out"); const update = git.run(sealed.repository, ["update-ref", sealed.branchRef, sealed.commit, sealed.expectedParent]); if (update.status !== 0) throw new Error("atomic branch publication was rejected"); return { branchRef: sealed.branchRef, parent: sealed.expectedParent, commit: sealed.commit, tree: sealed.tree }; } }); }
      catch (error) {
        if (git.text(sealed.repository, ["rev-parse", "--verify", sealed.branchRef]) === sealed.commit) {
          const recoveredAdmission = coordination.inspectAdmission(operationId);
          return recoveredAdmission ? publicationReceipt(sealed, recoveredAdmission, "published_observed") : unconfirmedPublication(sealed, error.message);
        }
        throw error;
      }
      return publicationReceipt(sealed, admission);
    },
    reconcile({ sealed, operationId }) {
      validateDigestRecord(sealed, "sealedDigest", "sealed publication"); requireOperationId(operationId); if (sealed.operationId !== operationId) throw new Error("sealed publication operation mismatch"); const observed = git.text(sealed.repository, ["rev-parse", "--verify", sealed.branchRef]);
      if (observed === sealed.commit) {
        const admission = coordination.inspectAdmission(operationId);
        return admission ? publicationReceipt(sealed, admission, "published_observed") : unconfirmedPublication(sealed, "mutation admission is unavailable");
      }
      if (observed !== sealed.expectedParent) return freeze({ status: "ambiguous", observed });
      const checkedOut = checkoutPaths(sealed.repository, sealed.branchRef, git.text); return checkedOut.length ? freeze({ status: "checkout_transition_required", branchRef: sealed.branchRef, expectedParent: sealed.expectedParent, preparedCommit: sealed.commit, preparedRevision: sealed.sealedDigest, checkedOut }) : freeze({ status: "eligible", expectedParent: sealed.expectedParent, preparedCommit: sealed.commit });
    },
    async publish({ repository, targetBranch, expectedParent, checkpoint, manifest, authorization, lease, operationId, holder, validation, message }) {
      if (typeof validation !== "function") throw new TypeError("canonical publication requires integrated-tree validation"); const preparation = publisher.prepare({ repository, targetBranch, expectedParent, checkpoint, manifest, authorization, operationId, holder, message }); if (preparation.status !== "prepared") return preparation;
      const legacy = await validation({ worktree: preparation.allocation.path, tree: preparation.tree, parent: preparation.expectedParent }); requireRecord(legacy, "integrated-tree validation result");
      if (legacy.status !== "passed" || legacy.tree !== preparation.tree) { git.text(preparation.allocation.path, ["reset", "--hard", preparation.expectedParent]); worktrees.cleanup(preparation.allocation); return freeze({ status: "validation_failed", validation: structuredClone(legacy) }); }
      const projected = { schemaVersion: 1, status: "passed", tree: preparation.tree, profile: "legacy-callback", requirements: ["integrated_tree_validation"], gateResult: structuredClone(legacy) };
      const sealed = publisher.sealValidation({ preparation, validation: { ...projected, receiptDigest: digest(projected) } }); return sealed.status === "sealed" ? publisher.promote({ sealed, lease, operationId, holder }) : sealed;
    },
  };
  return Object.freeze(publisher);
}

export const canonicalGitPublisherInternals = Object.freeze({ checkoutPaths, normalizeValidation });
