import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
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
const contractRejectedResult = {...result,
  limitations: ["Runtime behavior was inferred; deterministic checks were not executed by the reviewer."]};
const legacyFactory = async () => ({identity: {backend: "fixture"}, preflight() {}, finalize(value) { return value; },
  validateReceipt(value) { return value; }, checkpoint() {}, offer() {}, resumeTerminal() {}});
const effect = (operation, input) => ({generationId: "generation-native", effect: {
  protocol: SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL, capability: "capability.native_review", operation, input,
}});

async function credentialSource(stateRoot) {
  const source = path.join(stateRoot, "fixture-credentials.json");
  await writeFile(source, '{"fixture":"subscription"}\n', {mode: 0o600});
  return source;
}

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
  const reviewerCredentialSourcePath = await credentialSource(stateRoot);
  const calls = [];
  const ownersFactory = async (options) => {
    const owners = await createNativeReviewHostOwners({...options, reviewerCredentialSourcePath,
      reviewerExecuteProcess: async (request) => {
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
  const reviewerCredentialSourcePath = await credentialSource(stateRoot);
  let observedClosure = null;
  const ownersFactory = (options) => createNativeReviewHostOwners({...options, reviewerCredentialSourcePath,
    reviewerExecuteProcess: async (request) => {
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

test("agent-instruction specialist contract rejection preserves its outer result for same-session correction", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "native-review-specialist-correction-"));
  t.after(() => rm(stateRoot, {recursive: true, force: true}));
  const commit = "6566cdedef217582d03caf79c96c84006ec685d4";
  const tree = "076f1c438479f6ea8ae35c4c0ea54818c0396456";
  const specialistSubject = {commit, tree, patchIdentity: "d".repeat(64)};
  let campaign = await seed(stateRoot, {candidate: {commit, tree,
    manifestSha256: specialistSubject.patchIdentity,
    paths: [{path: "app-server/migrations/skills/slice-supervisor/structure.yaml"}]},
  specialists: [{obligationId: "instructions", skill: "agent-instruction-review", selection: "selected"}]});
  const reviewerCredentialSourcePath = await credentialSource(stateRoot);
  const calls = [];
  const ownersFactory = (options) => createNativeReviewHostOwners({...options, reviewerCredentialSourcePath,
    reviewerExecuteProcess: async (request) => {
      const prompt = request.args.at(-1);
      const marker = "WORK_ENGINE_AGENT_INSTRUCTION_REVIEW_V1\n";
      const start = prompt.indexOf(marker);
      const end = prompt.indexOf("\n\nExecution-profile constraints are subordinate", start);
      const closure = JSON.parse(prompt.slice(start + marker.length, end)).effective_instruction_closure;
      const sessionIndex = Math.max(request.args.indexOf("--session-id"), request.args.indexOf("--resume"));
      const session = request.args[sessionIndex + 1];
      const receiptPath = request.args[request.args.indexOf("--receipt") + 1];
      await writeFile(receiptPath, JSON.stringify({request: {session_id: session}, result: "success"}));
      calls.push({session, prompt});
      const inner = {...contractRejectedResult, subject: specialistSubject,
        ...(calls.length === 2 ? {verdict: "incomplete"} : {})};
      const specialistResult = {schemaVersion: 1, perspective: "agent-instruction-review",
        subject: specialistSubject, closureRevision: closure.closureRevision, applicability: "applicable",
        applicabilityReason: "The immutable candidate contains governed role instructions.",
        result: inner, findingDetails: [], limitations: ["Controlled deterministic fixture."]};
      return {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
        session_id: session, model: "claude-sonnet-5", structured_output: specialistResult})};
    }});
  const host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  t.after(() => host.close());
  let dispatched = await host.dispatch(effect("execute", {identity, expected_revision: campaign.revision,
    obligation_id: "instructions", operation_id: "specialist-correction:initial"}));
  campaign = dispatched.result.campaign;
  assert.equal(dispatched.result.failure.failureSignature, "result_contract_rejected");
  assert.equal(campaign.nativeReview.obligations.instructions.status, "correction_required");
  assert.equal(campaign.nativeReview.obligations.instructions.failure.recovery.rejectedResult.perspective,
    "agent-instruction-review");
  dispatched = await host.dispatch(effect("correct_result", {identity, expected_revision: campaign.revision,
    obligation_id: "instructions", operation_id: "specialist-correction:correct"}));
  assert.equal(dispatched.result.failure, null);
  assert.equal(dispatched.result.campaign.nativeReview.obligations.instructions.status, "awaiting_builder");
  assert.equal(calls.length, 2);
  assert.equal(calls[1].session, calls[0].session);
  assert.match(calls[1].prompt, /PREVIOUS STRUCTURED RESULT/);
  assert.match(calls[1].prompt, /agent-instruction-review/);
});

test("admitted provider failure is durable and exact redelivery cannot replay inference", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "native-review-ambiguous-"));
  t.after(() => rm(stateRoot, {recursive: true, force: true}));
  let campaign = await seed(stateRoot); let calls = 0;
  const reviewerCredentialSourcePath = await credentialSource(stateRoot);
  const ownersFactory = (options) => createNativeReviewHostOwners({...options, reviewerCredentialSourcePath,
    reviewerExecuteProcess: async () => {
    calls += 1; return {exitCode: 1, stdout: "", stderr: "ambiguous transport failure"};
  }});
  const host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  t.after(() => host.close());
  const input = {identity, expected_revision: campaign.revision, obligation_id: "generic", operation_id: "ambiguous:initial"};
  const failed = await host.dispatch(effect("execute", input));
  assert.equal(failed.result.failure.providerEntry, "unknown");
  assert.equal(calls, 1);
  campaign = (await host.dispatch({generationId: "generation-native", effect: {
    protocol: SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL, capability: "capability.lifecycle_control", operation: "recover", input: {identity},
  }})).result;
  assert.equal(campaign.nativeReview.obligations.generic.status, "executing");
  await assert.rejects(host.dispatch(effect("execute", {...input, expected_revision: campaign.revision})), /provider replay is refused/);
  assert.equal(calls, 1);
});

test("provider-entered contract rejection is durable and only the exact retained session can correct its result", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "native-review-result-correction-"));
  t.after(() => rm(stateRoot, {recursive: true, force: true}));
  let campaign = await seed(stateRoot);
  const reviewerCredentialSourcePath = await credentialSource(stateRoot);
  const calls = [];
  const ownersFactory = (options) => createNativeReviewHostOwners({...options, reviewerCredentialSourcePath,
    reviewerExecuteProcess: async (request) => {
      const sessionIndex = Math.max(request.args.indexOf("--session-id"), request.args.indexOf("--resume"));
      const session = request.args[sessionIndex + 1];
      const receiptPath = request.args[request.args.indexOf("--receipt") + 1];
      await writeFile(receiptPath, JSON.stringify({schema_version: 1, result: "success",
        request: {session_id: session}}));
      calls.push({session, resume: request.args.includes("--resume"), prompt: request.args.at(-1)});
      const structuredOutput = calls.length === 1
        ? contractRejectedResult
        : {...contractRejectedResult, verdict: "incomplete"};
      return {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
        session_id: session, model: "claude-sonnet-5", structured_output: structuredOutput})};
    }});
  const host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  t.after(() => host.close());
  let dispatched = await host.dispatch(effect("execute", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: "result-correction:initial"}));
  campaign = dispatched.result.campaign;
  assert.equal(dispatched.result.failure.failureSignature, "result_contract_rejected");
  assert.equal(dispatched.result.failure.providerEntry, "entered");
  assert.equal(campaign.nativeReview.obligations.generic.status, "correction_required");
  assert.equal(calls.length, 1);
  await assert.rejects(host.dispatch(effect("retry", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: "result-correction:retry-refused"})),
  /no recoverable definite pre-provider failure/);
  dispatched = await host.dispatch(effect("correct_result", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: "result-correction:correct"}));
  assert.equal(dispatched.result.failure, null);
  assert.equal(dispatched.result.campaign.nativeReview.obligations.generic.status, "awaiting_builder");
  assert.equal(calls.length, 2);
  assert.equal(calls[1].session, calls[0].session);
  assert.equal(calls[1].resume, true);
  assert.match(calls[1].prompt, /same-session correction/);
  assert.match(calls[1].prompt, /acceptable_as_is requires decisive evidence and no limitation/);
  assert.doesNotMatch(calls[0].prompt, /same-session correction/);
});

test("restart reconstructs a contract-rejected result after provider success preceded host admission", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "native-review-orphaned-result-"));
  t.after(() => rm(stateRoot, {recursive: true, force: true}));
  let campaign = await seed(stateRoot);
  const reviewerCredentialSourcePath = await credentialSource(stateRoot);
  const sessions = [];
  const ownersFactory = (options) => createNativeReviewHostOwners({...options, reviewerCredentialSourcePath,
    reviewerExecuteProcess: async (request) => {
      const freshIndex = request.args.indexOf("--session-id");
      const resumeIndex = request.args.indexOf("--resume");
      const session = request.args[freshIndex >= 0 ? freshIndex + 1 : resumeIndex + 1];
      const receiptPath = request.args[request.args.indexOf("--receipt") + 1];
      const sessionDirectory = path.join(request.env.CLAUDE_CONFIG_DIR, "projects", "fixture");
      await mkdir(sessionDirectory, {recursive: true});
      sessions.push({session, resume: resumeIndex >= 0, prompt: request.args.at(-1)});
      if (sessions.length === 1) {
        const timestamp = new Date().toISOString();
        await writeFile(receiptPath, JSON.stringify({request: {session_id: session}, result: "failed",
          attempts: [{duration_ms: 10_000}]}));
        await writeFile(path.join(sessionDirectory, `${session}.jsonl`), `${JSON.stringify({type: "assistant",
          timestamp,
          message: {role: "assistant", content: [{type: "text", text: "Not logged in · Please run /login"}]}})}\n`);
        return {exitCode: 1, stdout: "", stderr: "authentication required"};
      }
      if (sessions.length === 2) {
        await writeFile(path.join(sessionDirectory, `${session}.jsonl`), `${JSON.stringify({type: "assistant",
          timestamp: new Date().toISOString(), sessionId: session,
          message: {role: "assistant", content: [{type: "tool_use",
            name: "StructuredOutput", input: contractRejectedResult}]}})}\n`, {flag: "a"});
        await writeFile(receiptPath, JSON.stringify({request: {session_id: session}, result: "success"}));
        return {exitCode: 0, stdout: "host-lost-the-success-envelope", stderr: ""};
      }
      await writeFile(receiptPath, JSON.stringify({request: {session_id: session}, result: "success"}));
      return {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
        session_id: session, model: "claude-sonnet-5",
        structured_output: {...contractRejectedResult, verdict: "incomplete"}})};
    }});
  let host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  const operationId = "orphaned-result:initial";
  let dispatched = await host.dispatch(effect("execute", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: operationId}));
  campaign = dispatched.result.campaign;
  assert.equal(campaign.nativeReview.obligations.generic.status, "retryable_failure");
  await assert.rejects(host.dispatch(effect("retry", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: operationId})), /malformed JSON/);
  host.close();
  host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  t.after(() => host.close());
  const recovered = await host.dispatch(effect("recover", {identity, obligation_id: "generic"}));
  assert.equal(recovered.result.obligation.status, "retry_executing");
  assert.equal(recovered.result.recovery.failureSignature, "result_contract_rejected");
  assert.equal(recovered.result.recovery.providerEntry, "entered");
  assert.equal(recovered.result.recovery.sessionId, sessions[0].session);
  dispatched = await host.dispatch(effect("correct_result", {identity,
    expected_revision: recovered.result.campaign_revision, obligation_id: "generic",
    operation_id: "orphaned-result:correct"}));
  assert.equal(dispatched.result.failure, null);
  assert.equal(dispatched.result.campaign.nativeReview.obligations.generic.status, "awaiting_builder");
  assert.equal(sessions.length, 3);
  assert.equal(sessions[2].session, sessions[0].session);
  assert.equal(sessions[2].resume, true);
  assert.match(sessions[2].prompt, /same-session correction/);
});

test("restart admits an already-returned corrected result without replaying the correction provider call", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "native-review-orphaned-correction-"));
  t.after(() => rm(stateRoot, {recursive: true, force: true}));
  let campaign = await seed(stateRoot);
  const reviewerCredentialSourcePath = await credentialSource(stateRoot);
  const calls = [];
  const correctedResult = {...contractRejectedResult, verdict: "incomplete"};
  const ownersFactory = (options) => createNativeReviewHostOwners({...options, reviewerCredentialSourcePath,
    reviewerExecuteProcess: async (request) => {
      const freshIndex = request.args.indexOf("--session-id");
      const resumeIndex = request.args.indexOf("--resume");
      const session = request.args[freshIndex >= 0 ? freshIndex + 1 : resumeIndex + 1];
      const receiptPath = request.args[request.args.indexOf("--receipt") + 1];
      calls.push({session, resume: resumeIndex >= 0, prompt: request.args.at(-1)});
      if (calls.length === 1) {
        await writeFile(receiptPath, JSON.stringify({request: {session_id: session}, result: "success"}));
        return {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
          session_id: session, model: "claude-sonnet-5", structured_output: contractRejectedResult})};
      }
      const sessionDirectory = path.join(request.env.CLAUDE_CONFIG_DIR, "projects", "fixture");
      await mkdir(sessionDirectory, {recursive: true});
      await writeFile(path.join(sessionDirectory, `${session}.jsonl`), `${JSON.stringify({type: "assistant",
        timestamp: new Date().toISOString(), sessionId: session,
        message: {role: "assistant", content: [{type: "tool_use",
          name: "StructuredOutput", input: correctedResult}]}})}\n`, {flag: "a"});
      await writeFile(receiptPath, JSON.stringify({request: {session_id: session}, result: "success"}));
      return {exitCode: 0, stdout: "host-lost-the-corrected-envelope", stderr: ""};
    }});
  let host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  let dispatched = await host.dispatch(effect("execute", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: "orphaned-correction:initial"}));
  campaign = dispatched.result.campaign;
  assert.equal(campaign.nativeReview.obligations.generic.status, "correction_required");
  await assert.rejects(host.dispatch(effect("correct_result", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: "orphaned-correction:correct"})), /malformed JSON/);
  host.close();
  host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  t.after(() => host.close());
  const recovered = await host.dispatch(effect("recover", {identity, obligation_id: "generic"}));
  assert.equal(recovered.result.obligation.status, "correction_executing");
  assert.equal(recovered.result.recovery.failureSignature, "result_contract_corrected");
  assert.equal(recovered.result.recovery.sessionId, calls[0].session);
  dispatched = await host.dispatch(effect("correct_result", {identity,
    expected_revision: recovered.result.campaign_revision, obligation_id: "generic",
    operation_id: "orphaned-correction:recover"}));
  assert.equal(dispatched.result.failure, null);
  assert.equal(dispatched.result.campaign.nativeReview.obligations.generic.status, "awaiting_builder");
  assert.equal(calls.length, 2);
  assert.equal(calls[1].session, calls[0].session);
  assert.equal(calls[1].resume, true);
});

test("definite pre-provider authentication failure resumes the exact retained session only through retry", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "native-review-auth-retry-"));
  t.after(() => rm(stateRoot, {recursive: true, force: true}));
  let campaign = await seed(stateRoot);
  const reviewerCredentialSourcePath = await credentialSource(stateRoot);
  const sessions = [];
  let isolatedConfigRoot = null;
  const ownersFactory = (options) => createNativeReviewHostOwners({...options, reviewerCredentialSourcePath,
    reviewerExecuteProcess: async (request) => {
      const freshIndex = request.args.indexOf("--session-id");
      const resumeIndex = request.args.indexOf("--resume");
      const session = request.args[freshIndex >= 0 ? freshIndex + 1 : resumeIndex + 1];
      isolatedConfigRoot = request.env.CLAUDE_CONFIG_DIR;
      sessions.push({session, fresh: freshIndex >= 0, resume: resumeIndex >= 0});
      if (sessions.length === 1) {
        const receiptPath = request.args[request.args.indexOf("--receipt") + 1];
        const sessionDirectory = path.join(request.env.CLAUDE_CONFIG_DIR, "projects", "fixture");
        await mkdir(sessionDirectory, {recursive: true});
        const timestamp = new Date().toISOString();
        await writeFile(receiptPath, JSON.stringify({request: {session_id: session}, result: "failed",
          attempts: [{duration_ms: 10_000}]}));
        await writeFile(path.join(sessionDirectory, `${session}.jsonl`), `${JSON.stringify({type: "assistant",
          timestamp,
          message: {role: "assistant", content: [{type: "text", text: "Not logged in · Please run /login"}]}})}\n`);
        return {exitCode: 1, stdout: "", stderr: "authentication required"};
      }
      return {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
        session_id: session, model: "claude-sonnet-5", structured_output: result})};
    }});
  const host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  t.after(() => host.close());
  const operationId = "auth-retry:initial";
  let dispatched = await host.dispatch(effect("execute", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: operationId}));
  campaign = dispatched.result.campaign;
  assert.equal(dispatched.result.failure.failureSignature, "authentication_required");
  assert.equal(campaign.nativeReview.obligations.generic.status, "retryable_failure");
  const recovered = await host.dispatch(effect("recover", {identity, obligation_id: "generic"}));
  assert.equal(recovered.result.recovery.failureSignature, "authentication_required");
  assert.equal(recovered.result.recovery.sessionId, sessions[0].session);
  await writeFile(reviewerCredentialSourcePath, '{"fixture":"refreshed-subscription"}\n', {mode: 0o600});
  dispatched = await host.dispatch(effect("retry", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: operationId}));
  assert.equal(dispatched.result.failure, null);
  assert.equal(dispatched.result.campaign.nativeReview.obligations.generic.status, "reported");
  assert.deepEqual(sessions, [
    {session: sessions[0].session, fresh: true, resume: false},
    {session: sessions[0].session, fresh: false, resume: true},
  ]);
  assert.equal(await readFile(path.join(isolatedConfigRoot, ".credentials.json"), "utf8"),
    '{"fixture":"refreshed-subscription"}\n');
});

test("credential repair retries the exact deterministic UUID when no process or session existed", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "native-review-auth-pre-spawn-"));
  t.after(() => rm(stateRoot, {recursive: true, force: true}));
  let campaign = await seed(stateRoot);
  const reviewerCredentialSourcePath = path.join(stateRoot, "initially-missing-credentials.json");
  const sessions = [];
  const ownersFactory = (options) => createNativeReviewHostOwners({...options, reviewerCredentialSourcePath,
    reviewerExecuteProcess: async (request) => {
      const freshIndex = request.args.indexOf("--session-id");
      const resumeIndex = request.args.indexOf("--resume");
      sessions.push({session: request.args[freshIndex + 1], fresh: freshIndex >= 0, resume: resumeIndex >= 0});
      return {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
        session_id: request.args[freshIndex + 1], model: "claude-sonnet-5", structured_output: result})};
    }});
  const host = await createSupervisorCampaignCapabilityHostRuntime({workspaceRoot: repository, stateRoot,
    canonicalBranches: ["main"], legacyAdapterFactory: legacyFactory, nativeReviewOwnersFactory: ownersFactory});
  t.after(() => host.close());
  const operationId = "pre-spawn-auth-retry:initial";
  let dispatched = await host.dispatch(effect("execute", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: operationId}));
  campaign = dispatched.result.campaign;
  assert.equal(sessions.length, 0);
  assert.equal(dispatched.result.failure.failureSignature, "authentication_unavailable");
  assert.equal(campaign.nativeReview.obligations.generic.status, "retryable_failure");
  await writeFile(reviewerCredentialSourcePath, '{"fixture":"subscription"}\n', {mode: 0o600});
  dispatched = await host.dispatch(effect("retry", {identity, expected_revision: campaign.revision,
    obligation_id: "generic", operation_id: operationId}));
  assert.equal(dispatched.result.failure, null);
  assert.equal(dispatched.result.campaign.nativeReview.obligations.generic.status, "reported");
  assert.equal(sessions.length, 1);
  assert.deepEqual(sessions[0], {session: dispatched.result.campaign.nativeReview.obligations.generic.runtimeSessionRef.reference,
    fresh: true, resume: false});
});
