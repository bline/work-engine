import { spawnSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";

import { projectRuntimeManifest, satisfyRuntimeRequirements } from "../../../src/runtime-manifest.mjs";
import { createImplementationReviewService } from "../../../src/services/implementation-review/service.mjs";
import { digest as episodeDigest } from "../../../src/services/review-episode/contract.mjs";
import { createReviewEpisodeService } from "../../../src/services/review-episode/service.mjs";
import { openSqliteReviewEpisodeStore } from "../../../src/services/review-episode/sqlite-store.mjs";
import { createRustReviewEpisodeService } from "../../../src/services/review-episode/rust-adapter.mjs";
import { digest as claimDigest } from "../../../src/services/claim-evidence/identity.mjs";
import { openSqliteClaimEvidenceStore } from "../../../src/services/claim-evidence/sqlite-store.mjs";
import { createProductionPathEvidenceService } from "../../../src/services/claim-evidence/production-path-service.mjs";
import { makeProductionPathClaimRevision, normalizeProductionPathObservation,
  productionPathReference } from "../../../src/services/claim-evidence/production-path-contract.mjs";
import {createNativeReviewHostOwners} from "../../../src/services/slice-campaign/native-review-host.mjs";
import {createSupervisorCampaignCapabilityHostRuntime} from "../../../src/services/slice-campaign/capability-host-runtime.mjs";
import {SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL} from "../../../src/services/slice-campaign/host-effect-runtime.mjs";
import {createSliceCampaignService} from "../../../src/services/slice-campaign/service.mjs";
import {openSqliteSliceCampaignStore} from "../../../src/services/slice-campaign/sqlite-store.mjs";

const root = path.resolve(new URL("../../../../", import.meta.url).pathname);
const profilePath = path.join(root, "app-server/tests/fixtures/review-episode-rust/host-workload-v1.json");
const profileBytes = readFileSync(profilePath);
const profile = JSON.parse(profileBytes);
const compilerBytes = readFileSync(path.join(root, profile.compilerFixture.path));
const compiler = JSON.parse(compilerBytes);
const reviewBytes = readFileSync(path.join(root, profile.episodeFixture.path));
const originalResult = JSON.parse(reviewBytes);
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
if (sha(compilerBytes) !== profile.compilerFixture.sha256
    || sha(reviewBytes) !== profile.episodeFixture.sha256) throw new Error("HOST fixture hash differs");
const variant = process.env.WORK_ENGINE_HOST_VARIANT;
if (!profile.measurement.variants.includes(variant)) throw new Error("HOST variant is invalid");
const rustCompiler = variant === "c3_only" || variant === "both";
const rustEpisode = variant === "r3_only" || variant === "both";
const coldOnly = process.env.WORK_ENGINE_HOST_COLD === "1";
const compilerSelection = rustCompiler ? {
  binaryPath: process.env.WORK_ENGINE_HOST_COMPILER_BINARY,
  expectedSha256: process.env.WORK_ENGINE_HOST_COMPILER_SHA256,
  protocolVersion: 3, launchProfileId: "manifest-sync-v1",
} : null;
const reviewSelection = rustEpisode ? {
  binaryPath: process.env.WORK_ENGINE_HOST_EPISODE_BINARY,
  expectedSha256: process.env.WORK_ENGINE_HOST_EPISODE_SHA256,
  root: "", timeoutMs: 250,
} : null;
const stateRoot = mkdtempSync(path.join(os.tmpdir(), `work-engine-host-${variant}-`));
if (reviewSelection) {
  reviewSelection.root = path.join(stateRoot, "native-episode");
  const initialized = spawnSync(reviewSelection.binaryPath,
    ["init-native", "--root", reviewSelection.root], {timeout: 10_000});
  if (initialized.status !== 0) throw new Error(`native root init failed: ${initialized.stderr?.toString()}`);
}

const reference = (owner, reference, revision, value) => ({owner, reference, revision,
  sha256: episodeDigest(value), freshness: "exact immutable revision"});
const campaignIdentity = (attemptId) => ({runId: "host-profile-v1", sliceNumber: 1,
  attemptId, planVersion: "v1"});
const episodeIdentity = (campaign, obligationId) => ({...campaign, reviewObligationId: obligationId,
  reviewEpisodeId: episodeDigest({identity: campaign, obligationId}).slice(0, 32)});
const campaignKey = (campaign) => `${campaign.runId}:${campaign.sliceNumber}:${campaign.attemptId}:${campaign.planVersion}`;
const campaigns = new Map();
const scopeKey = (identity) => `${identity.runId}:${identity.sliceNumber}:${identity.attemptId}:${identity.planVersion}`;
function createCampaign(attemptId, subject, withClaims) {
  const identity = campaignIdentity(attemptId);
  const obligationId = "implementation-review";
  const episode = episodeIdentity(identity, obligationId);
  const requiredClaims = withClaims ? ["builder_projection", "campaign_terminalization"].map((boundary) =>
    makeProductionPathClaimRevision({
      proposition: "The selected native result was independently observed under the required production path.",
      subject: {candidate: subject, reviewEpisodeId: episode.reviewEpisodeId},
      coveredState: "admitted_native_review_result", consumptionBoundary: boundary,
      consumer: `${boundary === "builder_projection" ? "slice-builder" : "slice-campaign"}:${campaignKey(identity)}`,
      acceptance: {owner: "operator", source: "host-profile-v1", unestablishedRoute: "operator"},
      profile: {id: "production-path-v1", revision: "production-path-profile-v1",
        allowedMechanisms: ["native-review-host-receipt-v1"],
        admissibleObservers: ["app-server.reviewer-host"], integrityRequired: true,
        requiredRealization: "claude-sonnet-5", requiredCapabilities: ["repository_read"],
        continuity: "fresh_initial"},
    })) : [];
  const selection = {schemaVersion: withClaims ? 2 : 1, owner: "slice-supervisor",
    selectionId: `selection:${attemptId}`, subject,
    specialists: [{obligationId, skill: "implementation-review", selection: "selected",
      ...(withClaims ? {requiredClaims} : {})}]};
  const campaign = {identity, reviewSelection: selection};
  campaigns.set(scopeKey(identity), campaign);
  const selectionRevision = episodeDigest(selection);
  const authority = {schemaVersion: 1,
    grantId: `grant:native-review:${episodeDigest({identity, obligationId})}`,
    identity: episode,
    source: {owner: "slice-supervisor", reference: selection.selectionId,
      revision: selectionRevision, sha256: selectionRevision, freshness: "exact immutable revision"},
    writer: {actorId: `implementation-reviewer:${obligationId}`, provider: "claude", generation: 1,
      runtimeSession: reference("reviewer-runtime", `session:${attemptId}`, "generation-1", `session:${attemptId}`)},
    readers: ["reviewer", "builder", "supervisor"],
    initialSubject: reference("checkpoint", subject.commit, subject.tree, subject),
    predecessorRevision: null};
  return {campaign, authority, requiredClaims, selectionRevision};
}

const implementationReview = createImplementationReviewService();
const legacyStore = rustEpisode ? null
  : await openSqliteReviewEpisodeStore({filePath: path.join(stateRoot, "legacy-episode.sqlite3")});
const claimStore = await openSqliteClaimEvidenceStore({filePath: path.join(stateRoot, "claims.sqlite3"),
  bootstrapAuthorities: []});
const claims = createProductionPathEvidenceService({store: claimStore});
const episode = rustEpisode
  ? createRustReviewEpisodeService({...reviewSelection, implementationReview})
  : createReviewEpisodeService({store: legacyStore, implementationReview});
const setupEpisode = rustEpisode
  ? createRustReviewEpisodeService({...reviewSelection, implementationReview, timeoutMs: 10_000})
  : episode;
if (rustEpisode) {
  const resolve = (identity) => campaigns.get(scopeKey(identity));
  episode.bindCampaignOwner(resolve);
  setupEpisode.bindCampaignOwner(resolve);
}

const normalResult = structuredClone(originalResult);
const normal = createCampaign("normal", normalResult.subject, true);
const normalBegin = setupEpisode.begin({authority: normal.authority, transitionId: "normal:begin"});
const observation = normalizeProductionPathObservation({
  event_identity: "host-profile-observation-v1", selection: {id: normal.campaign.reviewSelection.selectionId,
    revision: normal.selectionRevision}, obligationId: normal.authority.identity.reviewObligationId,
  subject: {candidate: normalResult.subject, reviewEpisodeId: normal.authority.identity.reviewEpisodeId},
  coveredState: "admitted_native_review_result",
  execution: {attemptId: "controlled-provider-v1", resultDigest: claimDigest(normalResult)},
  realization: {requested: "claude-sonnet-5", observed: "claude-sonnet-5"},
  capabilityEnvelope: {capabilities: ["repository_read"], mutationAuthorized: false},
  continuity: {mode: "fresh_initial", sessionId: "controlled-session-v1"},
  transport: {mechanism: "native-review-host-receipt-v1", digest: "a".repeat(64)},
  observer: {identity: "app-server.reviewer-host", kind: "app_server_host"},
  observedAt: "2026-10-07T00:00:00.000Z", adapterVersion: "host-profile-v1",
  artifacts: [{owner: "reviewer-runtime", reference: "controlled-transport", digest: "a".repeat(64),
    status: "verified"}],
});
const evidenceAdmissions = normal.requiredClaims.map((claim) => {
  const establishment = claims.admit({operationId: `host-profile:${claim.revision}`, claim, observation});
  if (establishment.status !== "established"
      || claimDigest(claims.read(establishment.id)) !== claimDigest(establishment)) {
    throw new Error("HOST claim establishment is not exact");
  }
  const consumption = {schemaVersion: 1, claimRevision: claim.revision,
    establishment: establishment.id, boundary: claim.consumptionBoundary, consumer: claim.consumer};
  return {claimRevisionRef: productionPathReference("claim-evidence", claim.claimId, claim.revision, claim),
    establishmentRef: productionPathReference("claim-evidence", establishment.id, establishment.id, establishment),
    observationRef: productionPathReference("claim-evidence", observation.id, observation.id, observation),
    consumptionRef: productionPathReference("slice-campaign", `claim-consumption:${claim.claimId}`,
      claimDigest(consumption), consumption), status: establishment.status,
    boundary: claim.consumptionBoundary, consumer: claim.consumer};
});
const normalReported = setupEpisode.transition({authority: normal.authority,
  expectedRevision: normalBegin.revision, transitionId: "normal:result", action: "record_result",
  payload: {result: normalResult, unresolvedQuestions: [], evidenceAdmissions}});
if (normalReported.phase !== "reported" || normalReported.evidenceAdmissions.length !== 2) {
  throw new Error("HOST normal corpus did not establish exact two-claim result");
}

// The capability runtime is the measured outer host. This fixture's controlled
// reviewer executes once during cold setup; hot work only recovers durable state.
const hostIdentity = campaignIdentity("host-route");
const hostObligation = "generic";
const hostEpisodeId = episodeDigest({identity: hostIdentity, obligationId: hostObligation}).slice(0, 32);
const hostCampaignKey = campaignKey(hostIdentity);
const hostRequiredClaims = ["builder_projection", "campaign_terminalization"].map((boundary) =>
  makeProductionPathClaimRevision({
    proposition: "The selected native result was independently observed under the required production path.",
    subject: {candidate: originalResult.subject, reviewEpisodeId: hostEpisodeId},
    coveredState: "admitted_native_review_result", consumptionBoundary: boundary,
    consumer: `${boundary === "builder_projection" ? "slice-builder" : "slice-campaign"}:${hostCampaignKey}`,
    acceptance: {owner: "operator", source: "host-profile-v1", unestablishedRoute: "operator"},
    profile: {id: "production-path-v1", revision: "production-path-profile-v1",
      allowedMechanisms: ["native-review-host-receipt-v1"],
      admissibleObservers: ["app-server.reviewer-host"], integrityRequired: true,
      requiredRealization: "claude-sonnet-5", requiredCapabilities: ["repository_read"],
      continuity: "fresh_initial"},
  }));
const campaignStore = await openSqliteSliceCampaignStore({filePath: path.join(stateRoot, "slice-campaign.sqlite3")});
const campaignSeedService = createSliceCampaignService({store: campaignStore,
  implementationReview, reviewSubject: {
    async createCandidate(value) { return value; },
    async createPhysicalProfile({subject}) { return {subject}; },
  }, receiptFinalizer: {async finalize({receipt}) { return receipt; }}});
let hostCampaign = campaignSeedService.admit({identity: hostIdentity,
  workspace: "/private/host-profile-workspace",
  acceptedBoundary: {reference: "plan", sha256: "1".repeat(64)},
  baseline: {acceptedCommit: "base", acceptedTree: "base-tree", interSliceCommit: "inter"}});
for (const phase of ["implementing", "gate_ready"]) hostCampaign = campaignSeedService.advance({
  identity: hostIdentity, expectedRevision: hostCampaign.revision, phase, consequence: {}});
hostCampaign = await campaignSeedService.bindCandidate({identity: hostIdentity,
  expectedRevision: hostCampaign.revision,
  request: {commit: originalResult.subject.commit, tree: originalResult.subject.tree,
    manifestSha256: originalResult.subject.patchIdentity}});
hostCampaign = campaignSeedService.advance({identity: hostIdentity,
  expectedRevision: hostCampaign.revision, phase: "review_ready", consequence: {}});
hostCampaign = campaignSeedService.bindReviewSelection({identity: hostIdentity,
  expectedRevision: hostCampaign.revision, selection: {schemaVersion: 2,
    owner: "slice-supervisor", selectionId: "selection:host-route:v1", subject: originalResult.subject,
    specialists: [{obligationId: hostObligation, skill: "implementation-review",
      selection: "selected", requiredClaims: hostRequiredClaims}]}});
campaignStore.close();
const credentialsPath = path.join(stateRoot, "fixture-credentials.json");
writeFileSync(credentialsPath, `${JSON.stringify({claudeAiOauth: {
  accessToken: "fixture-login-access-token", refreshToken: "fixture-refresh-token",
  expiresAt: Date.parse("2099-01-01T00:00:00Z")}})}\n`, {mode: 0o600});
let providerCalls = 0;
let hostOwners = null;
const hostOwnersFactory = async (options) => {
  hostOwners = await createNativeReviewHostOwners({...options,
    compilerSelection,
    ...(reviewSelection ? {reviewEpisodeRust: {...reviewSelection}} : {}),
    reviewerCredentialSourcePath: credentialsPath,
    reviewerSubjectWorkspaceFactory: async () => root,
    reviewBoundaryFactory: ({subject}) => ({schemaVersion: 1, baselineCommit: "b".repeat(40),
      candidateCommit: subject.commit, candidateTree: subject.tree,
      taskPatchDigest: subject.patchIdentity, gateReceiptDigest: "d".repeat(64),
      paths: [{path: "src/task.mjs", action: "modify"}],
      evidenceCatalog: originalResult.decisiveEvidence, baselineEvidenceCatalog: [],
      changeDiff: "diff --git a/src/task.mjs b/src/task.mjs\n"}),
    reviewerExecuteProcess: async (request) => {
      providerCalls++;
      const toolsIndex = request.args.indexOf("--tools");
      const grantedTools = request.args[toolsIndex + 1]?.split(",") ?? [];
      if (grantedTools.slice(0, 3).join(",") !== "Read,Glob,Grep"
          || grantedTools.some((tool) => /^(?:Bash|Write|Edit|mcp__review_episode)/.test(tool))) {
        throw new Error("HOST controlled peer saw an unexpected tool grant");
      }
      const mcp = JSON.parse(readFileSync(request.args[request.args.indexOf("--mcp-config") + 1], "utf8"));
      if (Object.keys(mcp.mcpServers).join(",") !== "codebase-memory-mcp") {
        throw new Error("HOST controlled peer saw an unexpected MCP grant");
      }
      const peer = spawnSync(process.execPath, ["-e", `const fs=require('fs');
        const fds=fs.readdirSync('/proc/self/fd').map((name)=>{try{return fs.readlinkSync('/proc/self/fd/'+name)}catch{return ''}});
        process.stdout.write(JSON.stringify({cwd:process.cwd(),fds,envKeys:Object.keys(process.env)}));`],
      {env: request.env, cwd: request.cwd, stdio: ["ignore", "pipe", "pipe"]});
      if (peer.status !== 0) throw new Error("HOST controlled peer could not launch");
      const peerObserved = JSON.parse(peer.stdout.toString());
      if (peerObserved.cwd !== root
          || peerObserved.fds.some((fd) => fd.includes("review-episode-admission-"))
          || peerObserved.envKeys.some((key) => /REVIEW_EPISODE.*(?:DESCRIPTOR|GRANT)/.test(key))) {
        throw new Error("HOST controlled peer crossed the private admission boundary");
      }
      const sessionIndex = request.args.indexOf("--session-id");
      const response = {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
        session_id: request.args[sessionIndex + 1], model: "claude-sonnet-5",
        structured_output: originalResult})};
      const receiptIndex = request.args.indexOf("--receipt");
      const commandIndex = request.args.indexOf("--");
      if (receiptIndex >= 0 && commandIndex >= 0) {
        const command = request.args.slice(commandIndex + 1);
        const model = command[command.indexOf("--model") + 1];
        const stdinBytes = readFileSync(request.args[request.args.indexOf("--stdin-file") + 1]);
        const receiptPath = request.args[receiptIndex + 1];
        let existing = {};
        try { existing = JSON.parse(readFileSync(receiptPath, "utf8")); } catch {}
        writeFileSync(receiptPath, `${JSON.stringify({...existing, schema_version: 1,
          request: {...existing.request, transport: "anthropic", continuity: "retained",
            command_sha256: sha(Buffer.from(JSON.stringify(command))),
            stdin_sha256: request.args[request.args.indexOf("--stdin-sha256") + 1],
            stdin_size_bytes: stdinBytes.length,
            claude_code_oauth_token_present: Boolean(request.env.CLAUDE_CODE_OAUTH_TOKEN),
            session_mode: "new", session_id: command[command.indexOf("--session-id") + 1],
            paid_failover_explicitly_allowed: false, batch_route_explicitly_allowed: false},
          attempts: [{transport: "anthropic", gateway: "anthropic", requested_model: model,
            requested_upstream_provider: null, observed_models: ["claude-sonnet-5"],
            returncode: 0, duration_ms: 1}], selected_transport: "anthropic",
          failover: {attempted: false, allowed: false, reason: null, continuity_claim: null},
          upstream_provider_observed: false, result: "success"})}\n`);
      }
      return response;
    }});
  return hostOwners;
};
const hostEffect = (operation, input) => ({generationId: "generation-host-profile",
  effect: {protocol: SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL,
    capability: "capability.native_review", operation, input}});
const legacyFactory = async () => ({identity: {backend: "fixture"}, preflight() {},
  finalize(value) {return value;}, validateReceipt(value) {return value;}, checkpoint() {},
  offer() {}, resumeTerminal() {}});
let host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: root, stateRoot,
  canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory,
  nativeReviewOwnersFactory: hostOwnersFactory});
const executedHost = await host.dispatch(hostEffect("execute", {identity: hostIdentity,
  expected_revision: hostCampaign.revision, obligation_id: hostObligation,
  operation_id: "host-profile:initial"}));
hostCampaign = executedHost.result.campaign;
if (hostCampaign.nativeReview.obligations[hostObligation].status !== "reported"
    || hostCampaign.nativeReview.obligations[hostObligation].claimEvidence.length !== 2
    || providerCalls !== 1) throw new Error("HOST controlled initial execution did not close");
host.close();
host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: root, stateRoot,
  canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory,
  nativeReviewOwnersFactory: hostOwnersFactory});
const hostClaimReadStore = await openSqliteClaimEvidenceStore({
  filePath: path.join(stateRoot, "review-findings.sqlite3")});
const hostClaimReader = createProductionPathEvidenceService({store: hostClaimReadStore});
async function recoverHostRoute() {
  const response = await host.dispatch(hostEffect("recover", {identity: hostIdentity,
    obligation_id: hostObligation}));
  const obligation = response.result.obligation;
  if (response.result.campaign_revision !== hostCampaign.revision
      || obligation?.status !== "reported" || obligation.claimEvidence.length !== 2
      || providerCalls !== 1) throw new Error("HOST recovery changed the reported obligation");
  for (const item of obligation.claimEvidence) {
    const establishment = hostClaimReader.read(item.establishmentRef.reference);
    if (establishment.status !== "established"
        || claimDigest(establishment) !== item.establishmentRef.sha256) {
      throw new Error("HOST claim readback differs");
    }
  }
  const identity = {...hostIdentity, reviewObligationId: hostObligation,
    reviewEpisodeId: hostEpisodeId};
  const persisted = hostOwners.reviewEpisode.read({identity,
    revision: obligation.episodeRef.revision});
  if (persisted.phase !== "reported") throw new Error("HOST exact Episode read differs");
  return {campaignRevision: response.result.campaign_revision,
    episodeRevision: persisted.revision};
}

let maximum = null;
let maxHistory = null;
let maxHistoryBytes = null;
let maxDocument = null;
let maxCompilerRequestBytes = null;
if (!coldOnly) {
const maxQuestion = "x".repeat(7_800_000);
maximum = createCampaign("maximum", normalResult.subject, false);
let maxState = setupEpisode.begin({authority: maximum.authority,
  transitionId: "maximum:begin", unresolvedQuestions: [maxQuestion]});
maxState = setupEpisode.transition({authority: maximum.authority, expectedRevision: maxState.revision,
  transitionId: "maximum:uncertain-1", action: "mark_uncertain",
  payload: {reason: "controlled max history", reconciliationAction: "replace writer"}});
const successor = {...maximum.authority,
  writer: {...maximum.authority.writer, generation: 2,
    runtimeSession: reference("reviewer-runtime", "session:maximum:2", "generation-2", "session:maximum:2")},
  predecessorRevision: maxState.revision};
maxState = setupEpisode.transition({authority: successor, expectedRevision: maxState.revision,
  transitionId: "maximum:replace", action: "replace_writer",
  payload: {reason: "controlled replacement", pendingAction: "reconcile exact state"}});
maxState = setupEpisode.transition({authority: successor, expectedRevision: maxState.revision,
  transitionId: "maximum:uncertain-2", action: "mark_uncertain",
  payload: {reason: "controlled max history", reconciliationAction: "read exact history"}});
maxHistory = setupEpisode.history({identity: maximum.authority.identity});
if (maxHistory.length !== 4) throw new Error("HOST maximum history does not have four rows");
maxHistoryBytes = Buffer.byteLength(JSON.stringify(maxHistory));
if (maxHistoryBytes >= 32 * 1024 * 1024) throw new Error("HOST maximum history exceeds inherited response ceiling");

maxDocument = structuredClone(compiler.document);
maxDocument.roles["slice-supervisor"].developer_instructions = "x".repeat(4_000_000);
const compilerPayload = {schema_version: 3, request_id: "0".repeat(36),
  operation: "project_runtime_manifest", document: maxDocument,
  options: {base_directory: compiler.options.baseDirectory,
    identity_base_directory: compiler.options.identityBaseDirectory,
    requirements_base_directory: compiler.options.requirementsBaseDirectory,
    source_path: compiler.options.sourcePath, source_sha256: compiler.options.sourceSha256,
    runtime_requirements_by_role: compiler.options.runtimeRequirementsByRole}};
maxCompilerRequestBytes = Buffer.byteLength(JSON.stringify(compilerPayload));
if (maxCompilerRequestBytes > 4 * 1024 * 1024) throw new Error("HOST max compiler request exceeds ceiling");
}

function compile(document, count) {
  const manifest = projectRuntimeManifest(document, {...compiler.options, compilerSelection});
  const cases = count === 2
    ? compiler.satisfactionCases.filter(({roleId, skillName}) =>
      roleId === "slice-supervisor" && skillName === null
      || roleId === "implementation-reviewer" && skillName === "repo-search")
    : compiler.satisfactionCases;
  for (const item of cases) satisfyRuntimeRequirements({manifest, roleId: item.roleId,
    skillName: item.skillName, requirements: item.requirements});
  return manifest;
}

const slow = path.join(stateRoot, "slow-compiler");
writeFileSync(slow, "#!/bin/sh\nsleep 1\n", {mode: 0o700});
const slowSelection = {binaryPath: slow, expectedSha256: sha(readFileSync(slow)),
  protocolVersion: 3, launchProfileId: "manifest-sync-v1"};
let errorIndex = 0;
async function operation(kind) {
  const start = process.hrtime.bigint();
  const components = [];
  let caseName = null;
  const attempt = async (name, fn) => {
    const began = process.hrtime.bigint();
    try {
      await fn();
      components.push({name, outcome: "ok", elapsedMs: Number(process.hrtime.bigint() - began) / 1e6});
    } catch (caught) {
      components.push({name, outcome: "error",
        error: {name: caught.name, kind: caught.kind ?? caught.code ?? null, message: caught.message,
          transport: caught.transport ?? null},
        elapsedMs: Number(process.hrtime.bigint() - began) / 1e6});
    }
  };
    if (kind === "normal") {
      await attempt("capability_recover_claims_and_episode_exact_read", recoverHostRoute);
      await attempt("compiler_projection_and_two_satisfactions", () => compile(compiler.document, 2));
      await attempt("episode_exact_read", () => {
        const state = episode.read({identity: normal.authority.identity, revision: normalReported.revision});
        if (state.revision !== normalReported.revision || state.evidenceAdmissions.length !== 2) {
          throw new Error("normal exact read differs");
        }
      });
    } else if (kind === "maximum") {
      await attempt("capability_recover_claims_and_episode_exact_read", recoverHostRoute);
      await attempt("compiler_max_projection_and_six_satisfactions", () => compile(maxDocument, 6));
      await attempt("episode_near_ceiling_history", () => {
        const history = episode.history({identity: maximum.authority.identity});
        if (history.length !== 4) throw new Error("maximum history differs");
      });
    } else if (kind === "error_contention") {
      caseName = profile.workloads.error_contention.cases[errorIndex++ % 4];
      await attempt("capability_recover_claims_and_episode_exact_read", recoverHostRoute);
      await attempt(caseName, () => {
      if (caseName === "compiler_wrong_hash") {
        projectRuntimeManifest(compiler.document, {...compiler.options,
          compilerSelection: {binaryPath: compilerSelection?.binaryPath ?? process.env.WORK_ENGINE_HOST_COMPILER_BINARY,
            expectedSha256: "f".repeat(64), protocolVersion: 3, launchProfileId: "manifest-sync-v1"}});
      } else if (caseName === "episode_wrong_scope") {
        const otherScope = episode.read({identity: {...normal.authority.identity,
          reviewObligationId: "other"}});
        if (otherScope !== null) throw new Error("wrong scope returned an Episode");
        throw Object.assign(new Error("wrong scope refused"), {kind: "ScopeRefusal"});
      } else if (caseName === "sqlite_busy_write") {
        const file = rustEpisode ? path.join(reviewSelection.root, "review-episodes.sqlite")
          : path.join(stateRoot, "legacy-episode.sqlite3");
        const db = new DatabaseSync(file);
        db.exec("BEGIN IMMEDIATE");
        try {
          const collision = createCampaign(`busy-${randomUUID()}`, normalResult.subject, false);
          episode.begin({authority: collision.authority, transitionId: "busy:begin"});
        } finally { db.exec("ROLLBACK"); db.close(); }
      } else {
        projectRuntimeManifest(compiler.document, {...compiler.options, compilerSelection: slowSelection});
      }
      const unexpected = new Error(`${caseName} unexpectedly succeeded`);
      unexpected.kind = "unexpected_success";
      throw unexpected;
      });
    } else throw new Error(`unknown workload ${kind}`);
  const failed = components.find((component) => component.outcome === "error");
  return {outcome: failed ? "error" : "ok", caseName, error: failed?.error ?? null,
    components, elapsedMs: Number(process.hrtime.bigint() - start) / 1e6};
}

process.on("message", (message) => {
  if (message?.type === "probe") {
    process.send?.({type: "probe", id: message.id, at: process.hrtime.bigint().toString()});
  } else if (message?.type === "work") {
    operation(message.kind).then((outcome) => process.send?.({type: "work", id: message.id,
      kind: message.kind, ...outcome}), (error) => process.send?.({type: "work", id: message.id,
      kind: message.kind, outcome: "error", error: {name: error.name, message: error.message}}));
  } else if (message?.type === "close") {
    host.close(); hostClaimReadStore.close(); legacyStore?.close(); claimStore.close();
    rmSync(stateRoot, {recursive: true, force: true});
    process.send?.({type: "closed"});
    process.exit(0);
  }
});
process.send?.({type: "ready", variant, stateRoot,
  corpus: {profileSha256: sha(profileBytes), compilerFixtureSha256: sha(compilerBytes),
    episodeFixtureSha256: sha(reviewBytes), normalSelectionDigest: normal.selectionRevision,
    normalResultRevision: normalReported.revision,
    hostCampaignRevision: hostCampaign.revision, hostProviderCalls: providerCalls,
    maximumSelectionDigest: maximum ? episodeDigest(maximum.campaign.reviewSelection) : null,
    maximumHistoryRows: maxHistory?.length ?? null, maximumHistoryBytes: maxHistoryBytes,
    maximumHistorySha256: maxHistory ? sha(Buffer.from(JSON.stringify(maxHistory))) : null,
    maximumQuestionCharacters: coldOnly ? null : 7_800_000,
    maximumCompilerRequestBytes: maxCompilerRequestBytes,
    maximumCompilerDocumentSha256: maxDocument ? sha(Buffer.from(JSON.stringify(maxDocument))) : null}});
