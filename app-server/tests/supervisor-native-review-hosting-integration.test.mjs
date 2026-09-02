import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { createImplementationReviewService } from "../src/services/implementation-review/service.mjs";
import { createNativeReviewHostOwners } from "../src/services/slice-campaign/native-review-host.mjs";
import { createSupervisorCampaignCapabilityHostRuntime } from "../src/services/slice-campaign/capability-host-runtime.mjs";
import { SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL } from "../src/services/slice-campaign/host-effect-runtime.mjs";
import { createSliceCampaignService } from "../src/services/slice-campaign/service.mjs";
import { openSqliteSliceCampaignStore } from "../src/services/slice-campaign/sqlite-store.mjs";

const repository = path.resolve(new URL("../..", import.meta.url).pathname);
const subject = {commit: "immutable-candidate", tree: "immutable-tree", patchIdentity: "c".repeat(64)};
const identity = {runId: "native-host", sliceNumber: 1, attemptId: "attempt-1", planVersion: "plan-v1"};
const result = {schemaVersion: 1, subject, verdict: "acceptable_as_is", findings: [],
  decisiveEvidence: [{path: "app-server/src/index.mjs", startLine: 1, endLine: 1, sha256: "a".repeat(64)}], limitations: []};
const legacyFactory = async () => ({identity: {backend: "fixture"}, preflight() {}, finalize(value) { return value; },
  validateReceipt(value) { return value; }, checkpoint() {}, offer() {}, resumeTerminal() {}});
const effect = (operation, input) => ({generationId: "generation-native", effect: {
  protocol: SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL, capability: "capability.native_review", operation, input,
}});

async function seed(stateRoot, {candidate = {commit: subject.commit, tree: subject.tree, manifestSha256: subject.patchIdentity},
  specialists = [{obligationId: "generic", skill: "implementation-review", selection: "selected"}]} = {}) {
  const selectionSubject = {commit: candidate.commit, tree: candidate.tree, patchIdentity: candidate.manifestSha256};
  const store = await openSqliteSliceCampaignStore({filePath: path.join(stateRoot, "slice-campaign.sqlite3")});
  const service = createSliceCampaignService({store, implementationReview: createImplementationReviewService(),
    reviewSubject: {async createCandidate(request) { return request; }, async createPhysicalProfile({subject: value}) { return {subject: value}; }},
    receiptFinalizer: {async finalize({receipt}) { return receipt; }}});
  let campaign = service.admit({identity, workspace: "/private/native-review-workspace",
    acceptedBoundary: {reference: "plan", sha256: "1".repeat(64)},
    baseline: {acceptedCommit: "base", acceptedTree: "base-tree", interSliceCommit: "inter"}});
  campaign = service.advance({identity, expectedRevision: campaign.revision, phase: "implementing", consequence: {}});
  campaign = service.advance({identity, expectedRevision: campaign.revision, phase: "gate_ready", consequence: {}});
  campaign = await service.bindCandidate({identity, expectedRevision: campaign.revision,
    request: candidate});
  campaign = service.advance({identity, expectedRevision: campaign.revision, phase: "review_ready", consequence: {}});
  campaign = service.bindReviewSelection({identity, expectedRevision: campaign.revision, selection: {
    schemaVersion: 1, owner: "slice-supervisor", selectionId: "selection:native-host:v1", subject: selectionSubject,
    specialists,
  }});
  store.close(); return campaign;
}

test("read-only supervisor executes one selected native review and recovers durable closure after restart", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "native-review-hosting-"));
  t.after(() => rm(stateRoot, {recursive: true, force: true}));
  let campaign = await seed(stateRoot);
  const calls = [];
  const ownersFactory = async (options) => {
    const owners = await createNativeReviewHostOwners({...options, reviewerExecuteProcess: async (request) => {
    calls.push(request);
    assert.deepEqual(request.args.slice(1, 5), ["--transport", "anthropic", "--continuity", "retained"]);
    assert.equal(request.args.includes("openrouter"), false);
    assert.equal(request.args.some((value) => /--model-override|--mcp-config=.*caller/.test(value)), false);
    const sessionIndex = request.args.indexOf("--session-id");
    return {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
      session_id: request.args[sessionIndex + 1], model: "claude-sonnet-5", structured_output: result})};
    }});
    const source = await readFile(path.join(repository, "app-server/reviewer-profiles.yaml"), "utf8");
    assert.deepEqual(owners.catalogSource, {source: "app-server/reviewer-profiles.yaml",
      sourceSha256: createHash("sha256").update(source).digest("hex")});
    assert.notEqual(owners.catalogSource.sourceSha256, "8".repeat(64));
    return owners;
  };
  let host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  const executed = await host.dispatch(effect("execute", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: "native-host:generic:initial"}));
  assert.equal(calls.length, 1);
  assert.equal(executed.result.campaign.nativeReview.obligations.generic.status, "reported");
  const durableRevision = executed.result.campaign.revision;
  host.close();

  host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  t.after(() => host.close());
  const recovered = await host.dispatch(effect("recover", {identity, obligation_id: "generic"}));
  assert.equal(recovered.result.campaign_revision, durableRevision);
  assert.equal(recovered.result.obligation.status, "reported");
  assert.match(recovered.result.obligation.runtimeSessionRef.reference,
    /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  assert.equal(calls.length, 1);
  await assert.rejects(host.dispatch(effect("execute", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: "native-host:generic:initial", command: "caller"})), /unsupported field command/);
  await assert.rejects(host.dispatch(effect("execute", {identity, expected_revision: campaign.revision,
    obligation_id: "unselected", operation_id: "native-host:unselected"})), /revision conflict|not selected/);
  assert.equal(calls.length, 1);
});

test("read-only supervisor executes the selected agent-instruction specialist with an immutable closure", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "native-review-specialist-"));
  t.after(() => rm(stateRoot, {recursive: true, force: true}));
  const commit = "6566cdedef217582d03caf79c96c84006ec685d4";
  const tree = "076f1c438479f6ea8ae35c4c0ea54818c0396456";
  const specialistSubject = {commit, tree, patchIdentity: "d".repeat(64)};
  let campaign = await seed(stateRoot, {candidate: {commit, tree, manifestSha256: specialistSubject.patchIdentity,
    paths: [{path: "app-server/migrations/skills/slice-supervisor/structure.yaml"}]},
  specialists: [{obligationId: "instructions", skill: "agent-instruction-review", selection: "selected"}]});
  let observedClosure = null;
  const ownersFactory = (options) => createNativeReviewHostOwners({...options, reviewerExecuteProcess: async (request) => {
    const prompt = request.args.at(-1);
    assert.doesNotMatch(prompt, /You perform one advisory, read-only implementation review/);
    assert.doesNotMatch(prompt, /Return the generic implementation-review schema/);
    assert.match(prompt, /one advisory, read-only agent-instruction specialist review/);
    assert.match(prompt, /generic implementation-review protocol and schema do not apply/);
    assert.match(prompt, /Execution-profile constraints are subordinate to the selected review obligation/);
    const marker = "WORK_ENGINE_AGENT_INSTRUCTION_REVIEW_V1\n";
    const start = prompt.indexOf(marker);
    const end = prompt.indexOf("\n\nExecution-profile constraints are subordinate", start);
    assert.notEqual(start, -1); assert.notEqual(end, -1);
    const payload = JSON.parse(prompt.slice(start + marker.length, end));
    observedClosure = payload.effective_instruction_closure;
    const sessionIndex = request.args.indexOf("--session-id");
    const specialistResult = {schemaVersion: 1, perspective: "agent-instruction-review", subject: specialistSubject,
      closureRevision: observedClosure.closureRevision, applicability: "applicable",
      applicabilityReason: "The immutable candidate contains governed role instructions.",
      result: {...result, subject: specialistSubject}, findingDetails: [], limitations: ["Controlled deterministic fixture."]};
    return {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
      session_id: request.args[sessionIndex + 1], model: "claude-sonnet-5", structured_output: specialistResult})};
  }});
  let host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  const executed = await host.dispatch(effect("execute", {identity, expected_revision: campaign.revision,
    obligation_id: "instructions", operation_id: "native-host:instructions:initial"}));
  assert.equal(executed.result.campaign.nativeReview.obligations.instructions.status, "reported");
  assert.equal(observedClosure.subject.commit, commit);
  assert.equal(observedClosure.fragments[0].path, "app-server/migrations/skills/slice-supervisor/structure.yaml");
  const durableRevision = executed.result.campaign.revision;
  host.close();
  host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  t.after(() => host.close());
  const recovered = await host.dispatch(effect("recover", {identity, obligation_id: "instructions"}));
  assert.equal(recovered.result.campaign_revision, durableRevision);
  assert.equal(recovered.result.obligation.status, "reported");
});

test("admitted provider failure is durable and exact redelivery cannot replay inference", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "native-review-ambiguous-"));
  t.after(() => rm(stateRoot, {recursive: true, force: true}));
  let campaign = await seed(stateRoot); let calls = 0;
  const ownersFactory = (options) => createNativeReviewHostOwners({...options, reviewerExecuteProcess: async () => {
    calls += 1; return {exitCode: 1, stdout: "", stderr: "ambiguous transport failure"};
  }});
  const host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  t.after(() => host.close());
  const input = {identity, expected_revision: campaign.revision, obligation_id: "generic", operation_id: "ambiguous:initial"};
  await assert.rejects(host.dispatch(effect("execute", input)), /did not produce an admitted result/);
  assert.equal(calls, 1);
  campaign = (await host.dispatch({generationId: "generation-native", effect: {
    protocol: SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL, capability: "capability.lifecycle_control", operation: "recover", input: {identity},
  }})).result;
  assert.equal(campaign.nativeReview.obligations.generic.status, "executing");
  await assert.rejects(host.dispatch(effect("execute", {...input, expected_revision: campaign.revision})), /provider replay is refused/);
  assert.equal(calls, 1);
});
