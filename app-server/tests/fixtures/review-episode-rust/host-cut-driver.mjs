import {spawnSync} from "node:child_process";
import {createHash} from "node:crypto";
import {mkdirSync, readFileSync, writeFileSync} from "node:fs";
import path from "node:path";

import {createImplementationReviewService} from "../../../src/services/implementation-review/service.mjs";
import {digest as episodeDigest} from "../../../src/services/review-episode/contract.mjs";
import {makeProductionPathClaimRevision} from "../../../src/services/claim-evidence/production-path-contract.mjs";
import {createNativeReviewHostOwners} from "../../../src/services/slice-campaign/native-review-host.mjs";
import {createSupervisorCampaignCapabilityHostRuntime} from "../../../src/services/slice-campaign/capability-host-runtime.mjs";
import {SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL} from "../../../src/services/slice-campaign/host-effect-runtime.mjs";
import {createSliceCampaignService} from "../../../src/services/slice-campaign/service.mjs";
import {openSqliteSliceCampaignStore} from "../../../src/services/slice-campaign/sqlite-store.mjs";

const [mode, stateRoot, binary] = process.argv.slice(2);
if (!["first", "recover"].includes(mode) || !path.isAbsolute(stateRoot)
    || !path.isAbsolute(binary)) throw new Error("host cut fixture arguments differ");
const root = path.resolve(new URL("../../../../", import.meta.url).pathname);
const reviewRoot = path.join(stateRoot, "native-episode");
const result = JSON.parse(readFileSync(path.join(root,
  "app-server/tests/fixtures/implementation-review/acceptable-as-is.json"), "utf8"));
const identity = {runId: "host-os-cut", sliceNumber: 1, attemptId: "attempt-1", planVersion: "plan-v1"};
const obligationId = "generic";
const episodeId = episodeDigest({identity, obligationId}).slice(0, 32);
const campaignKey = `${identity.runId}:${identity.sliceNumber}:${identity.attemptId}:${identity.planVersion}`;
const countFile = path.join(stateRoot, "provider-entry-count.json");
const barrier = path.join(stateRoot, "after-durable-result.ready");
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
const binarySha = sha(readFileSync(binary));
const legacyFactory = async () => ({identity: {backend: "fixture"}, preflight() {},
  finalize(value) {return value;}, validateReceipt(value) {return value;}, checkpoint() {},
  offer() {}, resumeTerminal() {}});
const effect = (expectedRevision) => ({generationId: "generation-host-os-cut", effect: {
  protocol: SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL, capability: "capability.native_review",
  operation: "execute", input: {identity, expected_revision: expectedRevision,
    obligation_id: obligationId, operation_id: "host-os-cut:initial"}}});

if (mode === "first") {
  mkdirSync(stateRoot, {recursive: true});
  const init = spawnSync(binary, ["init-native", "--root", reviewRoot]);
  if (init.status !== 0) throw new Error(`native root init failed: ${init.stderr?.toString()}`);
  const requiredClaims = ["builder_projection", "campaign_terminalization"].map((boundary) =>
    makeProductionPathClaimRevision({
      proposition: "The selected native result was independently observed under the required production path.",
      subject: {candidate: result.subject, reviewEpisodeId: episodeId},
      coveredState: "admitted_native_review_result", consumptionBoundary: boundary,
      consumer: `${boundary === "builder_projection" ? "slice-builder" : "slice-campaign"}:${campaignKey}`,
      acceptance: {owner: "operator", source: "host-os-cut", unestablishedRoute: "operator"},
      profile: {id: "production-path-v1", revision: "production-path-profile-v1",
        allowedMechanisms: ["native-review-host-receipt-v1"],
        admissibleObservers: ["app-server.reviewer-host"], integrityRequired: true,
        requiredRealization: "claude-sonnet-5", requiredCapabilities: ["repository_read"],
        continuity: "fresh_initial"},
    }));
  const store = await openSqliteSliceCampaignStore({filePath: path.join(stateRoot, "slice-campaign.sqlite3")});
  const service = createSliceCampaignService({store,
    implementationReview: createImplementationReviewService(),
    reviewSubject: {async createCandidate(value) {return value;},
      async createPhysicalProfile({subject}) {return {subject};}},
    receiptFinalizer: {async finalize({receipt}) {return receipt;}}});
  let campaign = service.admit({identity, workspace: "/private/host-os-cut",
    acceptedBoundary: {reference: "plan", sha256: "1".repeat(64)},
    baseline: {acceptedCommit: "base", acceptedTree: "base-tree", interSliceCommit: "inter"}});
  for (const phase of ["implementing", "gate_ready"]) campaign = service.advance({identity,
    expectedRevision: campaign.revision, phase, consequence: {}});
  campaign = await service.bindCandidate({identity, expectedRevision: campaign.revision,
    request: {commit: result.subject.commit, tree: result.subject.tree,
      manifestSha256: result.subject.patchIdentity}});
  campaign = service.advance({identity, expectedRevision: campaign.revision,
    phase: "review_ready", consequence: {}});
  campaign = service.bindReviewSelection({identity, expectedRevision: campaign.revision,
    selection: {schemaVersion: 2, owner: "slice-supervisor", selectionId: "selection:host-os-cut",
      subject: result.subject, specialists: [{obligationId, skill: "implementation-review",
        selection: "selected", requiredClaims}]}});
  store.close();
  writeFileSync(countFile, "0\n");
  writeFileSync(path.join(stateRoot, "fixture-credentials.json"), `${JSON.stringify({claudeAiOauth: {
    accessToken: "fixture-login-access-token", refreshToken: "fixture-refresh-token",
    expiresAt: Date.parse("2099-01-01T00:00:00Z")}})}\n`, {mode: 0o600});
}

const reviewerExecuteProcess = async (request) => {
  const count = Number(readFileSync(countFile, "utf8")) + 1;
  writeFileSync(countFile, `${count}\n`);
  const sessionIndex = request.args.indexOf("--session-id");
  const response = {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
    session_id: request.args[sessionIndex + 1], model: "claude-sonnet-5", structured_output: result})};
  const receiptIndex = request.args.indexOf("--receipt");
  const command = request.args.slice(request.args.indexOf("--") + 1);
  const model = command[command.indexOf("--model") + 1];
  const stdinBytes = readFileSync(request.args[request.args.indexOf("--stdin-file") + 1]);
  const receiptPath = request.args[receiptIndex + 1];
  let existing = {};
  try {existing = JSON.parse(readFileSync(receiptPath, "utf8"));} catch {}
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
  return response;
};
let ownersForRead = null;
const ownersFactory = async (options) => {
  ownersForRead = await createNativeReviewHostOwners({...options,
  reviewerCredentialSourcePath: path.join(stateRoot, "fixture-credentials.json"),
  reviewerExecuteProcess, reviewEpisodeRust: {binaryPath: binary, expectedSha256: binarySha,
    root: reviewRoot, timeoutMs: 2_000}, reviewerSubjectWorkspaceFactory: async () => root,
  reviewBoundaryFactory: ({subject}) => ({schemaVersion: 1, baselineCommit: "b".repeat(40),
    candidateCommit: subject.commit, candidateTree: subject.tree,
    taskPatchDigest: subject.patchIdentity, gateReceiptDigest: "d".repeat(64),
    paths: [{path: "src/task.mjs", action: "modify"}],
    evidenceCatalog: result.decisiveEvidence, baselineEvidenceCatalog: [],
    changeDiff: "diff --git a/src/task.mjs b/src/task.mjs\n"}),
  afterDurableResult: mode === "first" ? async ({episode}) => {
    writeFileSync(barrier, `${JSON.stringify({episodeRevision: episode.revision,
      providerCalls: Number(readFileSync(countFile, "utf8"))})}\n`);
    await new Promise(() => { setInterval(() => {}, 60_000); });
  } : null,
  });
  return ownersForRead;
};
const host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: root, stateRoot,
  canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory,
  nativeReviewOwnersFactory: ownersFactory});
const campaignStore = await openSqliteSliceCampaignStore({filePath: path.join(stateRoot, "slice-campaign.sqlite3")});
const prepared = campaignStore.get(`${identity.runId}:${identity.sliceNumber}:${identity.attemptId}:${identity.planVersion}`);
campaignStore.close();
const response = await host.dispatch(effect(prepared.revision));
const obligation = response.result.campaign.nativeReview.obligations[obligationId];
const episodeHistory = ownersForRead.reviewEpisode.history({identity: {...identity,
  reviewObligationId: obligationId, reviewEpisodeId: episodeId}});
process.stdout.write(`${JSON.stringify({status: obligation.status,
  episodeRevision: obligation.episodeRef?.revision ?? null,
  episodeHistoryLength: episodeHistory.length,
  claims: obligation.claimEvidence?.map(({status}) => status) ?? [],
  providerCalls: Number(readFileSync(countFile, "utf8"))})}\n`);
host.close();
