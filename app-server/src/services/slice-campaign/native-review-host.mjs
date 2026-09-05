import { execFile, execFileSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { access, mkdir, readFile, rename, rm } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";
import { parse as parseYaml } from "yaml";

import { projectRuntimeManifest, loadRuntimeManifestDocument } from "../../runtime-manifest.mjs";
import { ImplementationReviewerRuntime } from "../../../roles/implementation-reviewer.mjs";
import { createAgentInstructionReviewService } from "../agent-instruction-review/service.mjs";
import { AgentInstructionReviewError, digest as instructionDigest } from "../agent-instruction-review/contract.mjs";
import { createReviewFindingBridge } from "../claim-evidence/review-finding-bridge.mjs";
import { openSqliteClaimEvidenceStore } from "../claim-evidence/sqlite-store.mjs";
import { createImplementationReviewService } from "../implementation-review/service.mjs";
import { ImplementationReviewError } from "../implementation-review/contract.mjs";
import { createReviewEpisodeService, ReviewEpisodeResultError } from "../review-episode/service.mjs";
import { digest as episodeDigest } from "../review-episode/contract.mjs";
import { openSqliteReviewEpisodeStore } from "../review-episode/sqlite-store.mjs";
import { NativeClaudeCodeReviewerAdapter } from "../reviewer-runtime/native-claude-code-adapter.mjs";
import { ReviewerRuntimeError } from "../reviewer-runtime/contract.mjs";
import { ReviewerProfileRegistry } from "../reviewer-runtime/profile-registry.mjs";
import { createNativeReviewClosureService } from "./native-review-closure.mjs";

const PROFILE_ID = "anthropic.claude-code.sonnet-review-v1";
const run = promisify(execFile);
const POLICY = Object.freeze({classification: "confidential", access: "episode actors",
  retention: "bounded projection retained", exactRetentionAuthorized: false,
  redaction: "raw bodies omitted", tamperEvidence: "sha256 digest"});

function ref(owner, reference, revision, sha256) {
  return Object.freeze({owner, reference, revision, sha256, freshness: "exact immutable revision"});
}
function exactRef(owner, reference, revision, integrity_sha256) {
  return Object.freeze({owner, reference, revision, integrity_sha256,
    freshness: "exact immutable revision", status: "verified"});
}
function subjectOf(campaign) {
  const candidate = campaign.candidate;
  if (!candidate) throw new Error("native review host requires an immutable campaign candidate");
  return Object.freeze({commit: candidate.checkpoint_commit_oid ?? candidate.commit,
    tree: candidate.checkpoint_tree_oid ?? candidate.tree,
    patchIdentity: candidate.task_patch_digest ?? candidate.manifestSha256});
}
function identityKey(identity) {
  return `${identity.runId}:${identity.sliceNumber}:${identity.attemptId}:${identity.planVersion}`;
}
function instanceId(identity, obligationId) {
  return `review-${episodeDigest({identity, obligationId}).slice(0, 32)}`;
}
function exactPreSpawnRecovery(recovery, expectedSession) {
  return recovery?.schemaVersion === 1
    && recovery.failureSignature === "authentication_unavailable"
    && recovery.providerEntry === "not_entered"
    && recovery.sessionAvailable === false
    && recovery.sessionId === expectedSession ? recovery : null;
}
function lineCount(content) { return Math.max(1, content.split("\n").length - (content.endsWith("\n") ? 1 : 0)); }

async function inspectSubjectWorkspace(directory, subject) {
  const identity = await run("git", ["-C", directory, "rev-parse", "HEAD", "HEAD^{tree}"], {
    encoding: "utf8",
  });
  const [commit, tree] = identity.stdout.trim().split(/\s+/);
  if (commit !== subject.commit || tree !== subject.tree) {
    throw new Error("native review subject workspace differs from the immutable subject");
  }
  const status = await run("git", ["-C", directory, "status", "--porcelain=v1"], {encoding: "utf8"});
  if (status.stdout.trim()) throw new Error("native review subject workspace is not clean");
  return path.resolve(directory);
}

export async function materializeImmutableSubjectWorkspace({workspaceRoot, stateRoot, subject}) {
  const source = path.resolve(workspaceRoot);
  const root = path.join(path.resolve(stateRoot), "review-subject-workspaces");
  const target = path.join(root, subject.tree);
  const sourceIdentity = await run("git", ["-C", source, "rev-parse",
    `${subject.commit}^{commit}`, `${subject.commit}^{tree}`], {encoding: "utf8"});
  const [commit, tree] = sourceIdentity.stdout.trim().split(/\s+/);
  if (commit !== subject.commit || tree !== subject.tree) {
    throw new Error("native review subject identity is unavailable from the host repository");
  }
  try {
    await access(target);
    return await inspectSubjectWorkspace(target, subject);
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
  await mkdir(root, {recursive: true, mode: 0o700});
  const temporary = path.join(root, `.${subject.tree}.${randomUUID()}.tmp`);
  try {
    await run("git", ["clone", "--shared", "--no-checkout", "--quiet", "--", source, temporary]);
    await run("git", ["-C", temporary, "checkout", "--detach", "--quiet", subject.commit]);
    await inspectSubjectWorkspace(temporary, subject);
    try { await rename(temporary, target); }
    catch (error) {
      if (!["EEXIST", "ENOTEMPTY"].includes(error?.code)) throw error;
      await inspectSubjectWorkspace(target, subject);
    }
    return await inspectSubjectWorkspace(target, subject);
  } finally {
    await rm(temporary, {recursive: true, force: true});
  }
}

function gitBlob(workspaceRoot, commit, filePath) {
  return execFileSync("git", ["-C", workspaceRoot, "show", `${commit}:${filePath}`], {encoding: "utf8"});
}

export function createReviewBoundary({workspaceRoot, campaign, subject}) {
  const candidate = campaign.candidate;
  const baselineCommit = execFileSync("git", ["-C", workspaceRoot, "rev-parse",
    `${candidate.baseline_commit_oid}^{commit}`], {encoding: "utf8"}).trim();
  const candidateCommit = execFileSync("git", ["-C", workspaceRoot, "rev-parse",
    `${subject.commit}^{commit}`], {encoding: "utf8"}).trim();
  const candidateTree = execFileSync("git", ["-C", workspaceRoot, "rev-parse",
    `${candidateCommit}^{tree}`], {encoding: "utf8"}).trim();
  if (candidateCommit !== subject.commit || candidateTree !== subject.tree) {
    throw new ReviewerRuntimeError("configuration",
      "native review candidate identity differs from the immutable subject");
  }
  const patch = execFileSync("git", ["-C", workspaceRoot, "diff-tree", "--binary",
    "--no-renames", "--no-ext-diff", `${baselineCommit}^{tree}`, candidateTree], {
    encoding: null, maxBuffer: 16 * 1024 * 1024,
  });
  const taskPatchDigest = createHash("sha256").update(patch).digest("hex");
  if (taskPatchDigest !== subject.patchIdentity) {
    throw new ReviewerRuntimeError("configuration",
      "native review patch identity differs from the immutable subject");
  }
  const declared = new Set((candidate.paths ?? []).map(({path: filePath}) => filePath));
  const changed = execFileSync("git", ["-C", workspaceRoot, "diff", "--no-ext-diff",
    "--no-renames", "--name-status", "-z", baselineCommit, candidateCommit], {
    encoding: "utf8", maxBuffer: 16 * 1024 * 1024,
  }).split("\0").filter(Boolean);
  const paths = [];
  for (let index = 0; index < changed.length; index += 2) {
    const status = changed[index];
    const filePath = changed[index + 1];
    if (!filePath || !declared.has(filePath) || !["A", "M", "D"].includes(status)) {
      throw new ReviewerRuntimeError("configuration",
        "native review boundary differs from the declared candidate path set");
    }
    paths.push(Object.freeze({path: filePath,
      action: Object.freeze({A: "add", M: "modify", D: "delete"})[status]}));
  }
  const entry = (commit, filePath) => {
    const content = gitBlob(workspaceRoot, commit, filePath);
    return Object.freeze({
      path: filePath,
      startLine: 1,
      endLine: lineCount(content),
      sha256: createHash("sha256").update(content).digest("hex"),
    });
  };
  const evidenceCatalog = paths.filter(({action}) => action !== "delete")
    .map(({path: filePath}) => entry(candidateCommit, filePath));
  const baselineEvidenceCatalog = paths.filter(({action}) => action !== "add")
    .map(({path: filePath}) => entry(baselineCommit, filePath));
  const changeDiff = execFileSync("git", ["-C", workspaceRoot, "diff", "--no-ext-diff",
    "--no-renames", "--unified=80", baselineCommit, candidateCommit, "--",
    ...paths.map(({path: filePath}) => filePath)], {
    encoding: "utf8", maxBuffer: 16 * 1024 * 1024,
  });
  return Object.freeze({
    schemaVersion: 1,
    baselineCommit,
    candidateCommit,
    candidateTree,
    taskPatchDigest,
    gateReceiptDigest: candidate.gate_receipt_digest,
    paths: Object.freeze(paths),
    evidenceCatalog: Object.freeze(evidenceCatalog),
    baselineEvidenceCatalog: Object.freeze(baselineEvidenceCatalog),
    changeDiff,
  });
}

function instructionClosure({workspaceRoot, campaign, subject, obligationId}) {
  const candidate = campaign.candidate;
  const paths = (candidate.paths ?? []).map(({path: value}) => value)
    .filter((value) => /(?:structure|interface)\.ya?ml$|SKILL\.md$|instructions?/i.test(value));
  if (!paths.length) throw new Error("selected agent-instruction review has no exact normative subject material");
  const commit = candidate.checkpoint_commit_oid ?? candidate.commit;
  const fragments = paths.map((filePath, index) => {
    const content = gitBlob(workspaceRoot, commit, filePath);
    return Object.freeze({id: `subject.${index}`, kind: filePath.endsWith("SKILL.md") ? "skill_contract" : "governed_instruction",
      path: filePath, startLine: 1, endLine: lineCount(content), sha256: instructionDigest(Buffer.from(content)),
      precedence: "subject", inclusion: "loaded", condition: null, content});
  });
  const body = {schemaVersion: 1, subject,
    manifest: {manifestId: `candidate:${commit}`, path: null,
      sha256: candidate.manifest_digest ?? candidate.manifestSha256 ?? instructionDigest(candidate.paths ?? [])},
    role: {roleId: `immutable-subject:${obligationId}`,
      runtimeEnvironmentRevision: instructionDigest({subject, fragments: fragments.map(({sha256}) => sha256)})},
    sourceInventoryComplete: true, fragments, omissions: [], limitations: []};
  return Object.freeze({...body, closureRevision: instructionDigest(body)});
}

function findingAuthority() {
  const authorityReference = exactRef("app-server", "native-review-finding-authority",
    "supervisor-native-review-hosting-v1", "9".repeat(64));
  return Object.freeze({schema_version: 1, grant_id: "grant:app-server-native-review-findings-v1",
    actor: "producer:app-server-native-review-bridge", profile: "revision-bound-review-finding-v1",
    permissions: ["create_claim", "publish_revision", "record_reliance"],
    decision_scope: "slice-campaign-native-review", authority_reference: authorityReference});
}

async function openClaims(filePath, authority) {
  try { await access(filePath); return openSqliteClaimEvidenceStore({filePath}); }
  catch (error) {
    if (error?.code !== "ENOENT") throw error;
    return openSqliteClaimEvidenceStore({filePath, bootstrapAuthorities: [authority]});
  }
}

export async function createNativeReviewHostOwners({workspaceRoot, stateRoot,
  reviewerExecuteProcess, claudeExecutable = "claude", pythonExecutable = "python3",
  reviewerCredentialSourcePath = null,
  reviewerSubjectWorkspaceFactory = materializeImmutableSubjectWorkspace,
  reviewBoundaryFactory = createReviewBoundary,
  runtimeSourceRoot = new URL("../../../../", import.meta.url).pathname} = {}) {
  const manifestPath = path.join(runtimeSourceRoot, "app-server/runtime-manifest.yaml");
  const loadedManifest = await loadRuntimeManifestDocument(manifestPath);
  const manifest = projectRuntimeManifest(loadedManifest.document, {
    baseDirectory: path.dirname(loadedManifest.sourcePath), sourcePath: loadedManifest.sourcePath,
    sourceSha256: loadedManifest.sourceSha256,
  });
  const registrySource = "app-server/reviewer-profiles.yaml";
  const registryContent = await readFile(path.join(runtimeSourceRoot, registrySource), "utf8");
  const registryDocument = parseYaml(registryContent);
  const catalogSource = Object.freeze({source: registrySource,
    sourceSha256: createHash("sha256").update(registryContent).digest("hex")});
  const registry = new ReviewerProfileRegistry({profiles: registryDocument.profiles});
  const adapter = new NativeClaudeCodeReviewerAdapter({registry, workspaceRoot,
    stateRoot: path.join(stateRoot, "reviewer-runtime"), executeProcess: reviewerExecuteProcess,
    claudeExecutable, pythonExecutable,
    credentialSourcePath: reviewerCredentialSourcePath,
    subjectWorkspaceFactory: reviewerSubjectWorkspaceFactory,
    transportScript: path.join(runtimeSourceRoot, "skills/claude-recon-implementation/scripts/claude_transport.py"),
    catalogSource});
  const agentInstructionReview = createAgentInstructionReviewService();
  const reviewer = new ImplementationReviewerRuntime({manifest, adapter, agentInstructionReview});
  const implementationReview = createImplementationReviewService();
  const episodeStore = await openSqliteReviewEpisodeStore({filePath: path.join(stateRoot, "review-episodes.sqlite3")});
  const authority = findingAuthority();
  const claimStore = await openClaims(path.join(stateRoot, "review-findings.sqlite3"), authority);
  const reviewEpisode = createReviewEpisodeService({store: episodeStore, implementationReview});
  const nativeReview = createNativeReviewClosureService({reviewEpisode, reviewer,
    findingBridge: createReviewFindingBridge({store: claimStore})});
  return Object.freeze({implementationReview, agentInstructionReview, nativeReview, adapter, authority, catalogSource,
    reviewBoundaryFactory,
    close() { episodeStore.close(); claimStore.close(); }});
}

export function createNativeReviewHost({workspaceRoot, campaignService, owners} = {}) {
  if (!campaignService?.recover || !campaignService?.runNativeReview
      || !campaignService?.retryNativeReview || !campaignService?.correctNativeReviewResult) {
    throw new TypeError("native review host requires Slice Campaign");
  }
  const request = (campaign, obligationId, operationId,
    {remediationSubject = null, continuationSessionId = null, resultCorrection = null} = {}) => {
    const disposition = campaign.reviewSelection?.specialists.find(({obligationId: value}) => value === obligationId);
    if (!disposition || disposition.selection !== "selected") throw new Error("native review obligation is not selected by the supervisor");
    const subject = remediationSubject ?? subjectOf(campaign);
    const instance = instanceId(campaign.identity, obligationId);
    const derivedSessionId = owners.adapter.runtimeSessionId(instance);
    if (continuationSessionId !== null && continuationSessionId !== derivedSessionId) {
      throw new Error("native review recovered session differs from the host-bound reviewer session");
    }
    const sessionId = continuationSessionId ?? derivedSessionId;
    const selectionRevision = episodeDigest(campaign.reviewSelection);
    const source = ref("slice-supervisor", campaign.reviewSelection.selectionId,
      selectionRevision, selectionRevision);
    const episodeIdentity = {...campaign.identity, reviewObligationId: obligationId,
      reviewEpisodeId: episodeDigest({identity: campaign.identity, obligationId}).slice(0, 32)};
    const subjectReference = ref("checkpoint", subject.commit, subject.tree, episodeDigest(subject));
    const existingBinding = campaign.nativeReview?.obligations?.[obligationId] ?? null;
    const initialSubject = existingBinding?.initialEpisodeRef
      ? owners.nativeReview.recoverInitialSubject({binding: existingBinding, identity: episodeIdentity})
      : subjectReference;
    const authority = {schemaVersion: 1,
      grantId: `grant:native-review:${episodeDigest({identity: campaign.identity, obligationId})}`,
      identity: episodeIdentity,
      source, writer: {actorId: `implementation-reviewer:${obligationId}`, provider: "claude",
        generation: 1, runtimeSession: ref("reviewer-runtime", sessionId, "generation-1", episodeDigest(sessionId))},
      readers: ["reviewer", "builder", "supervisor"],
      initialSubject, predecessorRevision: null};
    const catalogProjection = {schemaVersion: 1, catalogId: "work-engine.native-claude.direct-anthropic-v1",
      observedAt: "2026-09-01T00:00:00Z", expiresAt: "2099-01-01T00:00:00Z",
      source: owners.catalogSource.source, sourceSha256: owners.catalogSource.sourceSha256,
      models: [{slug: "sonnet", provider: "anthropic",
        capabilities: ["structured_output", "repository_read"], routingConstraints: ["direct-anthropic-only"]}]};
    const reviewerRequest = {instanceId: instance, profileId: PROFILE_ID, subject,
      reviewBoundary: owners.reviewBoundaryFactory({workspaceRoot, campaign, subject}),
      catalogProjection, rawEventPolicy: POLICY,
      ...(remediationSubject || continuationSessionId ? {continuationSessionId: sessionId} : {}),
      ...(resultCorrection ? {resultCorrection} : {}),
      ...(disposition.skill === "agent-instruction-review" ? {
        closure: instructionClosure({workspaceRoot, campaign, subject, obligationId}),
      } : {})};
    const base = {obligationId, authority, reviewerRequest, findingAuthority: owners.authority,
      operationPrefix: `native-review:${episodeDigest({identity: campaign.identity, obligationId, operationId})}`,
      contextRequest: {requestId: `native-review:${operationId}:context`, consumer: {
        identity: `slice-builder:${identityKey(campaign.identity)}`, revision: subject.tree,
        decision_scope: "slice-campaign-native-review"},
        limitations: ["Claims are evidence records, not review selection, evaluation, or acceptance."]}};
    return {base, subject, sessionId};
  };
  const recoverProviderResult = async (campaign, obligationId) => {
    const recovered = await owners.adapter.recoverResult(instanceId(campaign.identity, obligationId), subjectOf(campaign));
    if (!recovered) return null;
    const obligation = campaign.nativeReview?.obligations?.[obligationId] ?? null;
    const disposition = campaign.reviewSelection?.specialists
      .find(({obligationId: value}) => value === obligationId);
    let implementationResult = recovered.result;
    try {
      if (disposition?.skill === "agent-instruction-review") {
        const admitted = owners.agentInstructionReview.admit({result: recovered.result,
          closure: instructionClosure({workspaceRoot, campaign, subject: subjectOf(campaign), obligationId})});
        implementationResult = admitted.implementationReviewResult;
      }
      owners.implementationReview.admit({result: implementationResult,
        expectedSubject: subjectOf(campaign)});
      if (obligation?.status === "remediation_executing") {
        const {base} = request(campaign, obligationId, "recover-provider-result");
        owners.nativeReview.validateRecoveredResult({binding: obligation,
          authority: base.authority, result: implementationResult});
      }
      return Object.freeze({schemaVersion: 1, failureSignature: "result_contract_corrected",
        providerEntry: "entered", sessionAvailable: true, sessionId: recovered.sessionId,
        correctedResult: recovered.result, correctedResultDigest: recovered.resultDigest,
        correctedSubjectDigest: recovered.subjectDigest,
        transportReceiptDigest: recovered.transportReceiptDigest,
        sessionArtifactDigest: recovered.sessionArtifactDigest});
    } catch (error) {
      if (!(error instanceof ImplementationReviewError)
          && !(error instanceof AgentInstructionReviewError)
          && !(error instanceof ReviewEpisodeResultError)) throw error;
      return Object.freeze({schemaVersion: 1, failureSignature: "result_contract_rejected",
        providerEntry: "entered", sessionAvailable: true, sessionId: recovered.sessionId,
        message: error.message, rejectedResult: recovered.result,
        rejectedResultDigest: recovered.resultDigest,
        rejectedSubjectDigest: recovered.subjectDigest,
        transportReceiptDigest: recovered.transportReceiptDigest,
        sessionArtifactDigest: recovered.sessionArtifactDigest});
    }
  };
  return Object.freeze({
    async execute({identity, expected_revision, obligation_id, operation_id}) {
      const campaign = campaignService.recover(identity);
      const {base} = request(campaign, obligation_id, operation_id);
      const outcome = await campaignService.runNativeReview({identity, expectedRevision: expected_revision,
        request: {...base, beginTransitionId: `${operation_id}:begin`, resultTransitionId: `${operation_id}:result`}});
      return Object.freeze({campaign: outcome.campaign, builder_context: outcome.builderContext,
        failure: outcome.failure ?? null});
    },
    async recover({identity, obligation_id}) {
      const campaign = campaignService.recover(identity);
      const obligation = campaign.nativeReview?.obligations?.[obligation_id] ?? null;
      const reviewInstance = instanceId(campaign.identity, obligation_id);
      const recordedRecovery = obligation?.failure?.recovery ?? null;
      const recovery = obligation?.status === "correction_required"
        ? recordedRecovery
        : obligation?.status === "retryable_failure"
        ? exactPreSpawnRecovery(recordedRecovery, owners.adapter.runtimeSessionId(reviewInstance))
          ?? await owners.adapter.recoverFailure(reviewInstance, {recordedRecovery})
        : obligation?.status === "executing"
          ? await owners.adapter.recoverFailure(reviewInstance)
          : obligation?.status === "retry_executing"
            ? await recoverProviderResult(campaign, obligation_id)
            : obligation?.status === "correction_executing"
              ? await recoverProviderResult(campaign, obligation_id)
              : obligation?.status === "remediation_executing"
                ? await recoverProviderResult(campaign, obligation_id) : null;
      return Object.freeze({campaign_revision: campaign.revision,
        obligation, recovery});
    },
    async retry({identity, expected_revision, obligation_id, operation_id}) {
      const campaign = campaignService.recover(identity);
      const instance = instanceId(campaign.identity, obligation_id);
      const obligation = campaign.nativeReview?.obligations?.[obligation_id] ?? null;
      const recordedRecovery = obligation?.failure?.recovery ?? null;
      const recovery = obligation?.status === "retryable_failure"
        ? exactPreSpawnRecovery(recordedRecovery, owners.adapter.runtimeSessionId(instance))
          ?? await owners.adapter.recoverFailure(instance, {recordedRecovery})
        : await owners.adapter.recoverFailure(instance);
      if (!recovery) throw new Error("native review retry has no recoverable definite pre-provider failure");
      const {base, sessionId} = request(campaign, obligation_id, operation_id,
        recovery.sessionAvailable ? {continuationSessionId: recovery.sessionId} : {});
      const outcome = await campaignService.retryNativeReview({identity,
        expectedRevision: expected_revision, recovery,
        request: {...base, beginTransitionId: `${operation_id}:begin`,
          resultTransitionId: `${operation_id}:result`, retrySessionId: sessionId}});
      return Object.freeze({campaign: outcome.campaign, builder_context: outcome.builderContext,
        failure: outcome.failure ?? null});
    },
    async correctResult({identity, expected_revision, obligation_id, operation_id}) {
      const campaign = campaignService.recover(identity);
      const obligation = campaign.nativeReview?.obligations?.[obligation_id] ?? null;
      const recovery = obligation?.status === "correction_required"
        ? obligation.failure?.recovery ?? null
        : ["retry_executing", "correction_executing", "remediation_executing"].includes(obligation?.status)
          ? await recoverProviderResult(campaign, obligation_id) : null;
      if (!recovery) throw new Error("native review result correction has no exact provider result evidence");
      const {base} = request(campaign, obligation_id, operation_id, {
        continuationSessionId: recovery.sessionId,
        ...(recovery.failureSignature === "result_contract_rejected" ? {
          resultCorrection: Object.freeze({message: recovery.message,
            rejectedResult: recovery.rejectedResult}),
        } : {}),
      });
      const outcome = await campaignService.correctNativeReviewResult({identity,
        expectedRevision: expected_revision, recovery,
        request: {...base, resultTransitionId: `${operation_id}:corrected-result`}});
      return Object.freeze({campaign: outcome.campaign, builder_context: outcome.builderContext,
        failure: outcome.failure ?? null});
    },
    recordFindingEvaluation({identity, expected_revision, obligation_id, operation_id,
      finding_id, consumer_revision}) {
      const campaign = campaignService.recover(identity);
      if (consumer_revision !== subjectOf(campaign).tree) throw new Error("native finding evaluation consumer revision differs from immutable candidate");
      return campaignService.recordNativeFindingEvaluation({identity, expectedRevision: expected_revision,
        request: {obligationId: obligation_id, authority: owners.authority, operationId: operation_id,
          findingId: finding_id, consumer: `slice-builder:${identityKey(campaign.identity)}`,
          consumerRevision: consumer_revision, decisionScope: "slice-campaign-native-review"}}).campaign;
    },
    async executeRemediation({identity, expected_revision, obligation_id, operation_id,
      remediation_subject}) {
      const campaign = campaignService.recover(identity);
      const {base} = request(campaign, obligation_id, operation_id, {remediationSubject: remediation_subject});
      const remediationSubjectReference = ref("checkpoint", remediation_subject.commit,
        remediation_subject.tree, episodeDigest(remediation_subject));
      const executionRequest = {...base, subjectTransitionId: `${operation_id}:subject`,
        resultTransitionId: `${operation_id}:result`, remediationSubject: remediation_subject};
      const legacyAdmittedRequest = episodeDigest(base.authority.initialSubject)
          === episodeDigest(remediationSubjectReference) ? null : {
          ...executionRequest, authority: {...base.authority, initialSubject: remediationSubjectReference},
        };
      const outcome = await campaignService.runNativeRemediation({identity, expectedRevision: expected_revision,
        request: executionRequest, remediationSubjectReference, legacyAdmittedRequest});
      return Object.freeze({campaign: outcome.campaign, builder_context: outcome.builderContext,
        failure: outcome.failure ?? null});
    },
  });
}
