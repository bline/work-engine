import { execFileSync } from "node:child_process";
import path from "node:path";

import { canonicalJson, digest, freeze } from "../workspace-coordination/contract.mjs";

const OFFER_FIELDS = new Set([
  "schema_version", "offer_id", "state", "request", "result", "reason", "prior_oid", "decision",
]);
const ACCEPTED_CHECKPOINT_FIELDS = new Set([
  "schema_version", "checkpoint_id", "checkpoint_kind", "repository", "run_id",
  "slice_number", "candidate_attempt", "baseline_commit_oid", "baseline_tree_oid",
  "checkpoint_commit_oid", "checkpoint_tree_oid", "parent_checkpoint_commit_oid",
  "plan_version", "scope_revision", "gate_receipt_digest", "task_patch_digest",
  "paths", "ref", "manifest_digest", "created_at", "limitations", "candidate_checkpoint_id",
]);
const CHECKPOINT_ATTRIBUTIONS = new Set([
  "task_owned", "user_owned_baseline", "pre_existing_overlap",
  "generated_dependency", "validation_dependency",
]);

function requireRecord(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new TypeError(`${label} must be an object`);
  return value;
}
function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new TypeError(`${label} must be non-empty`);
  return value;
}
function exactKeys(value, keys, label) {
  requireRecord(value, label);
  if (JSON.stringify(Object.keys(value).sort()) !== JSON.stringify([...keys].sort())) throw new TypeError(`${label} fields are invalid`);
  return value;
}
function oid(value, label) {
  requireText(value, label);
  if (!/^[0-9a-f]{40,64}$/.test(value)) throw new TypeError(`${label} must be a lowercase Git object id`);
  return value;
}
function sha256(value, label) {
  requireText(value, label);
  if (!/^[0-9a-f]{64}$/.test(value)) throw new TypeError(`${label} must be SHA-256`);
  return value;
}
function safeIdentity(value, label) {
  requireText(value, label);
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(value)) throw new TypeError(`${label} is unsafe`);
  return value;
}
function safePath(value, label) {
  requireText(value, label);
  if (path.isAbsolute(value) || value.split("/").some((part) => !part || part === "..")) throw new TypeError(`${label} is unsafe`);
  return value;
}
function gitText(repository, args) {
  return execFileSync("git", ["-C", repository, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}
function offerBlob(repository, objectId, label) {
  let parsed;
  const bytes = gitText(repository, ["cat-file", "blob", oid(objectId, `${label} oid`)]);
  try { parsed = JSON.parse(bytes); } catch { throw new Error(`${label} is not JSON`); }
  if (bytes !== canonicalJson(parsed)) throw new Error(`${label} is not canonical JSON`);
  return parsed;
}
function normalizeManifest(value) {
  if (!Array.isArray(value) || value.length === 0) throw new TypeError("completion offer accepted_paths must be non-empty");
  const paths = new Set();
  return value.map((entry) => {
    exactKeys(entry, new Set(["action", "path"]), "completion offer accepted path");
    safePath(entry.path, "completion offer accepted path");
    if (!["include", "delete"].includes(entry.action) || paths.has(entry.path)) throw new TypeError("completion offer accepted path is invalid");
    paths.add(entry.path);
    return structuredClone(entry);
  });
}
function normalizeProposal(value, manifest) {
  exactKeys(value, new Set([
    "schema_version", "subject", "body", "paths", "checkpoint_commit_oid",
    "checkpoint_tree_oid", "task_patch_digest", "provenance",
  ]), "completion offer proposal");
  if (value.schema_version !== 2) throw new Error("completion publication requires proposal schema version 2");
  requireText(value.subject, "completion proposal subject");
  if (value.subject.includes("\n") || typeof value.body !== "string") throw new TypeError("completion proposal message is invalid");
  oid(value.checkpoint_commit_oid, "completion proposal checkpoint commit");
  oid(value.checkpoint_tree_oid, "completion proposal checkpoint tree");
  sha256(value.task_patch_digest, "completion proposal task patch");
  exactKeys(value.provenance, new Set(["schema_version", "producer", "evidence"]), "completion proposal provenance");
  if (value.provenance.schema_version !== 1) throw new Error("completion proposal provenance version is invalid");
  requireText(value.provenance.producer, "completion proposal provenance producer");
  if (!Array.isArray(value.provenance.evidence) || value.provenance.evidence.length === 0) throw new TypeError("completion proposal provenance evidence must be non-empty");
  for (const evidence of value.provenance.evidence) {
    exactKeys(evidence, new Set(["kind", "digest"]), "completion proposal provenance evidence");
    requireText(evidence.kind, "completion proposal provenance evidence kind");
    sha256(evidence.digest, "completion proposal provenance evidence digest");
  }
  const expected = manifest.map(({ path: manifestPath }) => manifestPath);
  if (!Array.isArray(value.paths) || new Set(value.paths).size !== value.paths.length
      || JSON.stringify([...value.paths].sort()) !== JSON.stringify([...expected].sort())) {
    throw new Error("completion proposal paths do not match the accepted manifest");
  }
  value.paths.forEach((proposalPath) => safePath(proposalPath, "completion proposal path"));
  return structuredClone(value);
}
function normalizeOffer(repository, value) {
  exactKeys(value, new Set([...OFFER_FIELDS, "artifact_oid", "ref"]), "loaded completion offer");
  if (value.schema_version !== 2 || value.state !== "create_authorized" || value.result !== null
      || value.reason !== null || !value.prior_oid) throw new Error("completion publication requires a schema-v2 create_authorized offer");
  const requestFields = new Set([
    "repository", "run_id", "slice_number", "expected_branch", "expected_head_oid",
    "accepted_paths", "proposal",
  ]);
  requireRecord(value.request, "completion offer request");
  const requestKeys = new Set(Object.keys(value.request));
  const withOperational = new Set([...requestFields, "operational_paths"]);
  if (JSON.stringify([...requestKeys].sort()) !== JSON.stringify([...requestFields].sort())
      && JSON.stringify([...requestKeys].sort()) !== JSON.stringify([...withOperational].sort())) {
    throw new TypeError("completion offer request fields are invalid");
  }
  if (path.resolve(requireText(value.request.repository, "completion repository")) !== repository
      || value.request.repository !== repository) throw new Error("completion offer repository does not match the host repository");
  const runId = safeIdentity(value.request.run_id, "completion offer run id");
  if (!Number.isSafeInteger(value.request.slice_number) || value.request.slice_number < 1) throw new TypeError("completion offer slice number must be positive");
  requireText(value.request.expected_branch, "completion offer expected branch");
  oid(value.request.expected_head_oid, "completion offer expected parent");
  const manifest = normalizeManifest(value.request.accepted_paths);
  const proposal = normalizeProposal(value.request.proposal, manifest);
  if (value.request.operational_paths !== undefined) {
    if (!Array.isArray(value.request.operational_paths) || new Set(value.request.operational_paths).size !== value.request.operational_paths.length) throw new TypeError("completion operational paths are invalid");
    value.request.operational_paths.forEach((entry) => safePath(entry, "completion operational path"));
    if (value.request.operational_paths.some((entry) => manifest.some(({ path: manifestPath }) => manifestPath === entry))) throw new Error("completion operational paths overlap the accepted manifest");
  }
  exactKeys(value.decision, new Set(["decision", "authority"]), "completion offer decision");
  if (value.decision.decision !== "create") throw new Error("completion offer does not authorize creation");
  exactKeys(value.decision.authority, new Set(["kind", "reference", "observed_at"]), "completion offer authority");
  if (value.decision.authority.kind !== "human") throw new Error("completion offer authority is not human");
  requireText(value.decision.authority.reference, "completion offer authority reference");
  if (Number.isNaN(Date.parse(requireText(value.decision.authority.observed_at, "completion offer authority observation")))) throw new TypeError("completion offer authority observation is invalid");
  if (value.offer_id !== digest(value.request)) throw new Error("completion offer identity does not match its request");
  const expectedRef = `refs/work-engine/completion-offers/${runId}/slice-${value.request.slice_number}`;
  if (value.ref !== expectedRef || gitText(repository, ["rev-parse", "--verify", value.ref]) !== oid(value.artifact_oid, "completion offer artifact")) throw new Error("completion offer ref does not select its artifact");
  const projected = structuredClone(value); delete projected.artifact_oid; delete projected.ref;
  if (canonicalJson(offerBlob(repository, value.artifact_oid, "completion offer artifact")) !== canonicalJson(projected)) throw new Error("completion offer artifact bytes do not match the supplied offer");
  const prior = offerBlob(repository, value.prior_oid, "completion offer predecessor");
  exactKeys(prior, OFFER_FIELDS, "completion offer predecessor");
  if (prior.schema_version !== 2 || prior.state !== "open" || prior.offer_id !== value.offer_id
      || canonicalJson(prior.request) !== canonicalJson(value.request) || prior.result !== null
      || prior.reason !== null || prior.prior_oid !== null || prior.decision !== null) {
    throw new Error("completion offer predecessor is not the exact open authority source");
  }
  return freeze({ offer: structuredClone(value), manifest, proposal });
}
function normalizeCheckpoint(repository, value, manifest) {
  exactKeys(value, ACCEPTED_CHECKPOINT_FIELDS, "accepted checkpoint lifecycle receipt");
  if (value.schema_version !== 1 || value.checkpoint_kind !== "accepted"
      || value.repository !== repository) throw new Error("completion publication requires the host repository's accepted checkpoint receipt");
  safeIdentity(value.run_id, "accepted checkpoint run id");
  for (const field of ["slice_number", "candidate_attempt"]) {
    if (!Number.isSafeInteger(value[field]) || value[field] < 1) throw new TypeError(`accepted checkpoint ${field} must be positive`);
  }
  for (const field of ["baseline_commit_oid", "baseline_tree_oid", "checkpoint_commit_oid", "checkpoint_tree_oid", "parent_checkpoint_commit_oid"]) oid(value[field], `accepted checkpoint ${field}`);
  for (const field of ["checkpoint_id", "candidate_checkpoint_id", "task_patch_digest", "manifest_digest", "gate_receipt_digest"]) sha256(value[field], `accepted checkpoint ${field}`);
  requireText(value.plan_version, "accepted checkpoint plan version");
  requireText(value.scope_revision, "accepted checkpoint scope revision");
  requireText(value.created_at, "accepted checkpoint creation time");
  if (!Array.isArray(value.limitations)) throw new TypeError("accepted checkpoint limitations must be an array");
  if (!Array.isArray(value.paths) || value.paths.length === 0) throw new TypeError("accepted checkpoint paths must be non-empty");
  const projectedManifest = value.paths.map((entry) => {
    exactKeys(entry, new Set(["action", "attribution", "content_digest", "path"]), "accepted checkpoint path");
    safePath(entry.path, "accepted checkpoint path");
    if (!["include", "delete"].includes(entry.action) || !CHECKPOINT_ATTRIBUTIONS.has(entry.attribution)) throw new TypeError("accepted checkpoint path attribution is invalid");
    sha256(entry.content_digest, "accepted checkpoint path content digest");
    return { action: entry.action, path: entry.path };
  });
  if (new Set(projectedManifest.map(({ path: manifestPath }) => manifestPath)).size !== projectedManifest.length
      || canonicalJson(projectedManifest) !== canonicalJson(manifest)
      || value.manifest_digest !== digest(value.paths)) throw new Error("accepted checkpoint manifest does not match the completion offer");
  const expectedRef = `refs/work-engine/checkpoints/${value.run_id}/slice-${value.slice_number}/accepted`;
  if (value.ref !== expectedRef || gitText(repository, ["rev-parse", "--verify", value.ref]) !== value.checkpoint_commit_oid) throw new Error("accepted checkpoint ref does not select its lifecycle commit");
  if (gitText(repository, ["rev-parse", `${value.baseline_commit_oid}^{tree}`]) !== value.baseline_tree_oid
      || gitText(repository, ["rev-parse", `${value.checkpoint_commit_oid}^{tree}`]) !== value.checkpoint_tree_oid
      || gitText(repository, ["rev-parse", `${value.checkpoint_commit_oid}^`]) !== value.parent_checkpoint_commit_oid) throw new Error("accepted checkpoint Git lineage is invalid");
  let metadata;
  try { metadata = JSON.parse(gitText(repository, ["show", "-s", "--format=%B", value.checkpoint_commit_oid])); }
  catch { throw new Error("accepted checkpoint metadata is invalid"); }
  const expectedMetadata = {
    work_engine_checkpoint: 1, kind: "accepted", candidate_checkpoint_id: value.candidate_checkpoint_id,
    run_id: value.run_id, slice_number: value.slice_number, candidate_attempt: value.candidate_attempt,
    tree: value.checkpoint_tree_oid, task_patch_digest: value.task_patch_digest,
    manifest_digest: value.manifest_digest, gate_receipt_digest: value.gate_receipt_digest,
    plan_version: value.plan_version, scope_revision: value.scope_revision,
  };
  if (canonicalJson(metadata) !== canonicalJson(expectedMetadata) || value.checkpoint_id !== digest(expectedMetadata)) throw new Error("accepted checkpoint lifecycle metadata is invalid");
  const full = freeze(structuredClone(value));
  const publisher = freeze({
    commitOid: value.checkpoint_commit_oid, treeOid: value.checkpoint_tree_oid,
    baselineCommitOid: value.baseline_commit_oid, taskPatchDigest: value.task_patch_digest,
    manifestDigest: value.manifest_digest, gateReceiptDigest: value.gate_receipt_digest,
    planVersion: value.plan_version, scopeRevision: value.scope_revision, ref: value.ref,
  });
  return freeze({ full, publisher, checkpointDigest: digest(full) });
}
function derive(repository, offerValue, checkpointValue) {
  const { offer, manifest, proposal } = normalizeOffer(repository, offerValue);
  const checkpoint = normalizeCheckpoint(repository, checkpointValue, manifest);
  if (checkpoint.publisher.commitOid !== proposal.checkpoint_commit_oid
      || checkpoint.publisher.treeOid !== proposal.checkpoint_tree_oid
      || checkpoint.publisher.taskPatchDigest !== proposal.task_patch_digest) {
    throw new Error("accepted checkpoint does not match the authorized completion proposal");
  }
  const offerBinding = freeze({
    offerId: offer.offer_id, artifactOid: offer.artifact_oid, ref: offer.ref,
    predecessorOid: offer.prior_oid, requestDigest: digest(offer.request),
    decision: structuredClone(offer.decision),
  });
  const publication = freeze({
    targetBranch: offer.request.expected_branch,
    expectedParent: offer.request.expected_head_oid,
    manifest: structuredClone(manifest),
    message: { subject: proposal.subject, body: proposal.body },
  });
  const basis = freeze({ schemaVersion: 1, offerBinding,
    acceptedCheckpointDigest: checkpoint.checkpointDigest, publication });
  const preparationRevision = digest(basis);
  return freeze({
    basis, preparationRevision, operationId: `completion-${offer.offer_id}`,
    request: {
      targetBranch: publication.targetBranch, expectedParent: publication.expectedParent,
      checkpoint: checkpoint.publisher, manifest: publication.manifest,
      authorization: {
        decision: "create", reference: `completion-publication:${preparationRevision}:${checkpoint.checkpointDigest}`,
        paths: [...proposal.paths], checkpointCommitOid: checkpoint.publisher.commitOid,
        checkpointTreeOid: checkpoint.publisher.treeOid, targetBranch: publication.targetBranch,
      },
      message: publication.message,
    },
  });
}
function deriveFromRecord(repository, offerValue, record) {
  const { offer, manifest, proposal } = normalizeOffer(repository, offerValue);
  const source = record.status === "published" ? record.sealed : record;
  const match = /^completion-publication:([0-9a-f]{64}):([0-9a-f]{64})$/.exec(source.authorization?.reference ?? "");
  if (!match) throw new Error("completion publication record lacks its authority binding");
  const offerBinding = freeze({
    offerId: offer.offer_id, artifactOid: offer.artifact_oid, ref: offer.ref,
    predecessorOid: offer.prior_oid, requestDigest: digest(offer.request),
    decision: structuredClone(offer.decision),
  });
  const publication = freeze({ targetBranch: offer.request.expected_branch,
    expectedParent: offer.request.expected_head_oid, manifest: structuredClone(manifest),
    message: { subject: proposal.subject, body: proposal.body } });
  const basis = freeze({ schemaVersion: 1, offerBinding,
    acceptedCheckpointDigest: match[2], publication });
  if (digest(basis) !== match[1]) throw new Error("completion publication durable authority digest is invalid");
  const checkpoint = structuredClone(source.acceptedCheckpoint); delete checkpoint.verifiedRef;
  if (checkpoint.commitOid !== proposal.checkpoint_commit_oid
      || checkpoint.treeOid !== proposal.checkpoint_tree_oid
      || checkpoint.taskPatchDigest !== proposal.task_patch_digest) throw new Error("completion publication checkpoint conflicts with the authorized offer");
  return freeze({ basis, preparationRevision: match[1], operationId: `completion-${offer.offer_id}`,
    request: { targetBranch: publication.targetBranch, expectedParent: publication.expectedParent,
      checkpoint, manifest: publication.manifest, authorization: structuredClone(source.authorization),
      message: publication.message } });
}
function verifyWorkspaceRecord(derived, current) {
  if (!current?.record) throw new Error("completion publication preparation is absent");
  const source = current.record.status === "published" ? current.record.sealed : current.record;
  if (!["prepared", "sealed"].includes(source.status) || source.operationId !== derived.operationId
      || source.targetBranch !== derived.request.targetBranch
      || source.expectedParent !== derived.request.expectedParent
      || canonicalJson(source.manifest) !== canonicalJson([...derived.request.manifest].sort((a, b) => a.path.localeCompare(b.path)))
      || canonicalJson(source.authorization) !== canonicalJson(derived.request.authorization)
      || canonicalJson(source.message) !== canonicalJson(derived.request.message)) {
    throw new Error("completion publication preparation conflicts with its authority binding");
  }
  return source;
}
function preparationRecord(derived, current) {
  const source = verifyWorkspaceRecord(derived, current);
  const value = {
    schemaVersion: 1, status: "prepared", vocabulary: "private_prepared_publication",
    operationId: derived.operationId, preparationRevision: derived.preparationRevision,
    offerBinding: structuredClone(derived.basis.offerBinding),
    acceptedCheckpointDigest: derived.basis.acceptedCheckpointDigest,
    workspacePublicationRevision: current.revision, workspaceStatus: source.status,
    integratedTree: source.tree, privateRef: source.privateRef ?? source.allocation?.privateRef ?? null,
  };
  return freeze({ ...value, recordDigest: digest(value) });
}
function completionRecord(derived, current) {
  verifyWorkspaceRecord(derived, current);
  if (current.record.status !== "published") throw new Error("completion publication is not durably published");
  const publication = current.record.publication;
  const confirmed = publication.status !== "published_unconfirmed";
  const value = {
    schemaVersion: 1, status: confirmed ? "published" : "published_unconfirmed",
    vocabulary: "human_visible_ref_observed", operationId: derived.operationId,
    preparationRevision: derived.preparationRevision,
    offerBinding: structuredClone(derived.basis.offerBinding),
    workspacePublicationRevision: current.revision,
    publication: structuredClone(publication), fencingProvenance: confirmed ? "confirmed" : "unconfirmed",
  };
  return freeze({ ...value, recordDigest: digest(value) });
}
function boundedResult(derived, result) {
  return freeze({ ...structuredClone(result), operationId: derived.operationId,
    preparationRevision: derived.preparationRevision });
}

export function createCompletionPublicationService({ workspace } = {}) {
  if (!workspace || typeof workspace.preparePublication !== "function"
      || typeof workspace.inspectPublication !== "function"
      || typeof workspace.sealPublication !== "function"
      || typeof workspace.promotePublication !== "function"
      || typeof workspace.reconcilePublication !== "function") {
    throw new TypeError("completion publication requires the workspace-development owner");
  }
  const repository = workspace.repository;
  return Object.freeze({
    prepare({ offer, acceptedCheckpoint } = {}) {
      const derived = derive(repository, offer, acceptedCheckpoint);
      const result = workspace.preparePublication({ operationId: derived.operationId, ...derived.request });
      if (!result?.record || !["prepared", "sealed", "published"].includes(result.record.status)) {
        return boundedResult(derived, result?.record ?? result);
      }
      return preparationRecord(derived, result);
    },
    complete({ offer, preparationRevision, validation } = {}) {
      const normalizedOffer = normalizeOffer(repository, offer).offer;
      const operationId = `completion-${normalizedOffer.offer_id}`;
      let current = workspace.inspectPublication(operationId);
      if (!current) throw new Error("completion publication preparation is absent");
      const derived = deriveFromRecord(repository, offer, current.record);
      if (derived.preparationRevision !== preparationRevision) throw new Error("completion publication revision does not match its authority binding");
      let source = verifyWorkspaceRecord(derived, current);
      if (source.status === "prepared") {
        const sealed = workspace.sealPublication({
          operationId, preparationRevision: current.revision, validation,
        });
        if (sealed.record?.status !== "sealed") return boundedResult(derived, sealed.record ?? sealed);
        current = sealed; source = sealed.record;
      } else if (canonicalJson(source.validation) !== canonicalJson(validation)) {
        throw new Error("completion publication validation conflicts with the sealed tree binding");
      }
      if (current.record.status === "published") return completionRecord(derived, current);
      const result = workspace.promotePublication({ operationId, preparedRevision: current.revision });
      if (result?.record?.status === "published") return completionRecord(derived, result);
      return boundedResult(derived, result);
    },
    reconcile({ offer, preparationRevision } = {}) {
      const normalizedOffer = normalizeOffer(repository, offer).offer;
      const operationId = `completion-${normalizedOffer.offer_id}`;
      const current = workspace.inspectPublication(operationId);
      if (!current) throw new Error("completion publication preparation is absent");
      const derived = deriveFromRecord(repository, offer, current.record);
      if (derived.preparationRevision !== preparationRevision) throw new Error("completion publication revision does not match its authority binding");
      const source = verifyWorkspaceRecord(derived, current);
      if (current.record.status === "published") return completionRecord(derived, current);
      if (source.status === "prepared") return boundedResult(derived, { status: "validation_required" });
      const result = workspace.reconcilePublication({ operationId, preparedRevision: current.revision });
      if (result?.record?.status === "published") return completionRecord(derived, result);
      return boundedResult(derived, result);
    },
  });
}
