import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { NativeClaudeCodeReviewerAdapter } from "../../../src/services/reviewer-runtime/native-claude-code-adapter.mjs";
import { ReviewerProfileRegistry } from "../../../src/services/reviewer-runtime/profile-registry.mjs";
import { digest } from "../../../src/services/reviewer-runtime/contract.mjs";

const subject = {commit: "candidate", tree: "tree", patchIdentity: "patch"};
const result = {schemaVersion: 1, subject, verdict: "acceptable_as_is", findings: [],
  decisiveEvidence: [{path: "app-server/src/index.mjs", startLine: 1, endLine: 1, sha256: "a".repeat(64)}], limitations: []};
const reviewBoundary = {schemaVersion: 1, baselineCommit: "b".repeat(40), candidateCommit: subject.commit,
  candidateTree: subject.tree, taskPatchDigest: subject.patchIdentity, gateReceiptDigest: "d".repeat(64),
  paths: [{path: "app-server/src/index.mjs", action: "modify"}], evidenceCatalog: result.decisiveEvidence,
  baselineEvidenceCatalog: [], changeDiff: "diff --git a/app-server/src/index.mjs b/app-server/src/index.mjs\n"};
function profile(overrides = {}) {
  const value = {schemaVersion: 1, profileId: "anthropic.claude-code.sonnet-review-v1", enabled: true,
    requestedModel: "sonnet", provider: "anthropic", reasoning: "medium",
    capabilities: ["structured_output", "repository_read"], outputSchema: "work-engine.implementation-review.v1",
    effectiveInstructions: "Review exact subject.", isolatedHome: true,
    limitations: ["Direct Anthropic only."], acceptingAuthority: "accepted-plan", ...overrides};
  value.configurationDigest = digest(value); return value;
}
const catalog = {schemaVersion: 1, catalogId: "native", observedAt: "2026-09-01T00:00:00Z",
  expiresAt: "2099-01-01T00:00:00Z", source: "fixture", sourceSha256: "b".repeat(64),
  models: [{slug: "sonnet", provider: "anthropic", capabilities: ["structured_output", "repository_read"], routingConstraints: ["direct"]}]};
const catalogSource = {source: catalog.source, sourceSha256: catalog.sourceSha256};
const policy = {classification: "confidential", access: "episode actors", retention: "projection",
  exactRetentionAuthorized: false, redaction: "raw bodies omitted", tamperEvidence: "sha256"};

async function credentials(root) {
  const source = path.join(root, "fixture-credentials.json");
  await writeFile(source, '{"fixture":"subscription"}\n', {mode: 0o600});
  return source;
}

async function promptFromRequest(request) {
  const promptFileIndex = request.args.indexOf("--stdin-file");
  assert.notEqual(promptFileIndex, -1);
  return readFile(request.args[promptFileIndex + 1], "utf8");
}

async function transportResult(request, envelope = null, {exitCode = 0, stderr = "",
  observedModels = null, receiptOverrides = {}} = {}) {
  const receiptIndex = request.args.indexOf("--receipt");
  const commandIndex = request.args.indexOf("--");
  const command = request.args.slice(commandIndex + 1);
  const modelIndex = command.indexOf("--model");
  const sessionFlag = command.includes("--session-id") ? "--session-id" : "--resume";
  const sessionIndex = command.indexOf(sessionFlag);
  const stdinShaIndex = request.args.indexOf("--stdin-sha256");
  const stdinFileIndex = request.args.indexOf("--stdin-file");
  const stdinBytes = await readFile(request.args[stdinFileIndex + 1]);
  const requestedModel = command[modelIndex + 1];
  const observed = envelope?.model ?? Object.keys(envelope?.modelUsage ?? {})[0] ?? null;
  const base = {schema_version: 1, request: {transport: "anthropic", continuity: "retained",
    command_sha256: digest(command), stdin_sha256: request.args[stdinShaIndex + 1],
    stdin_size_bytes: stdinBytes.length,
    session_mode: sessionFlag === "--session-id" ? "new" : "resume",
    paid_failover_explicitly_allowed: false, batch_route_explicitly_allowed: false,
    session_id: command[sessionIndex + 1]}, attempts: [{transport: "anthropic", harness: "claude-code",
    gateway: "anthropic", requested_model: requestedModel, requested_upstream_provider: null,
    observed_models: observedModels ?? [observed ?? requestedModel], returncode: exitCode, duration_ms: 1,
    stdout_sha256: "a".repeat(64), stderr_sha256: "b".repeat(64), quota_signature: null}],
  selected_transport: exitCode === 0 ? "anthropic" : null,
  failover: {attempted: false, allowed: false, reason: null, continuity_claim: null},
  upstream_provider_observed: false,
  result: exitCode === 0 ? "success" : "failed"};
  const receipt = {...base, ...receiptOverrides,
    request: {...base.request, ...(receiptOverrides.request ?? {})},
    failover: {...base.failover, ...(receiptOverrides.failover ?? {})},
    attempts: receiptOverrides.attempts ?? base.attempts};
  await writeFile(request.args[receiptIndex + 1], `${JSON.stringify(receipt)}\n`);
  return {exitCode, stderr, stdout: envelope === null ? "" : JSON.stringify(envelope)};
}

test("native Claude adapter constructs only direct-Anthropic retained commands and verifies UUID", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  const calls = [];
  const credentialSourcePath = await credentials(root);
  const adapter = new NativeClaudeCodeReviewerAdapter({registry: new ReviewerProfileRegistry({profiles: [profile()]}),
    workspaceRoot: root, stateRoot: path.join(root, "state"),
    credentialSourcePath,
    catalogSource,
    transportScript: path.join(root, "transport.py"),
    executeProcess: async (request) => {
      calls.push(request);
      const sessionFlag = request.args.indexOf("--session-id");
      const resumeFlag = request.args.indexOf("--resume");
      const session = request.args[sessionFlag >= 0 ? sessionFlag + 1 : resumeFlag + 1];
      return transportResult(request, {type: "result", subtype: "success",
        session_id: session, model: "claude-sonnet-5", structured_output: result});
    }});
  const initial = await adapter.execute({instanceId: "episode", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Read-only review."});
  assert.equal(initial.failure, null);
  assert.equal(initial.receipt.harness, "claude-code");
  assert.equal(initial.receipt.gateway, "anthropic");
  assert.equal(initial.receipt.continuity, "fresh_initial");
  const isolatedCredentials = path.join(root, "state", "native-claude", digest("episode"), "config", ".credentials.json");
  assert.equal(await readFile(isolatedCredentials, "utf8"), '{"fixture":"subscription"}\n');
  assert.equal((await stat(isolatedCredentials)).mode & 0o777, 0o600);
  assert.equal(calls[0].args.includes("openrouter"), false);
  assert.deepEqual(calls[0].args.slice(1, 7), ["--transport", "anthropic", "--continuity", "retained", "--receipt", calls[0].args[6]]);
  assert.equal(calls[0].args.includes("--session-id"), true);
  assert.equal(calls[0].args.includes("--strict-mcp-config"), true);
  assert.equal(calls[0].args.some((value) => /Write|Edit|Bash/.test(value)), false);
  const latestAttempt = JSON.parse(await readFile(path.join(root, "state", "native-claude",
    digest("episode"), "latest-attempt.json"), "utf8"));
  assert.match(latestAttempt.sessionBindingDigest, /^[0-9a-f]{64}$/);
  assert.match(latestAttempt.commandDigest, /^[0-9a-f]{64}$/);
  assert.match(latestAttempt.promptDigest, /^[0-9a-f]{64}$/);
  assert.equal(latestAttempt.catalogDigest, digest(catalog));
  assert.equal(latestAttempt.subjectDigest, digest(subject));
  const prompt = await promptFromRequest(calls[0]);
  assert.equal(calls[0].args.includes(prompt), false);
  assert.equal(latestAttempt.promptSizeBytes, Buffer.byteLength(prompt));
  assert.match(prompt, /Execution-profile constraints are subordinate to the selected review obligation/);
  assert.match(prompt, /Review exact subject\./);
  const continued = await adapter.execute({instanceId: "episode", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Read-only review.",
    continuationSessionId: initial.runtimeSessionId});
  assert.equal(continued.receipt.continuity, "same_session_resume");
  assert.equal(calls[1].args.includes("--resume"), true);
  await assert.rejects(adapter.execute({instanceId: "episode", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Read-only review.",
    continuationSessionId: "00000000-0000-4000-8000-000000000000"}), /pre-registered session/);
});

test("native Claude adapter uses the exact profile/catalog-authorized Anthropic model", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-opus-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  const opus = profile({profileId: "anthropic.claude-code.opus-review-v1",
    requestedModel: "opus", reasoning: "high"});
  const opusCatalog = {...catalog, models: [{...catalog.models[0], slug: "opus",
    routingConstraints: ["direct-anthropic-only"]}]};
  const calls = [];
  const adapter = new NativeClaudeCodeReviewerAdapter({
    registry: new ReviewerProfileRegistry({profiles: [opus]}), workspaceRoot: root,
    stateRoot: path.join(root, "state"), credentialSourcePath: await credentials(root),
    catalogSource, transportScript: path.join(root, "transport.py"),
    now: () => Date.parse("2026-09-06T12:00:00Z"), executeProcess: async (request) => {
      calls.push(request);
      const sessionIndex = request.args.indexOf("--session-id");
      return transportResult(request, {type: "result", subtype: "success",
        session_id: request.args[sessionIndex + 1], model: "claude-opus", structured_output: result});
    },
  });
  const completed = await adapter.execute({instanceId: "opus", profileId: opus.profileId, subject,
    catalogProjection: opusCatalog, rawEventPolicy: policy, reviewBoundary,
    roleInstructions: "Read-only review."});
  assert.equal(completed.receipt.requestedModel, "opus");
  assert.equal(completed.receipt.observedModel, "claude-opus");
  assert.equal(completed.transportReceipt.attempts[0].requested_model, "opus");
  assert.deepEqual(completed.transportReceipt.attempts[0].observed_models, ["claude-opus"]);
  assert.deepEqual(completed.transportReceipt.failover, {attempted: false, allowed: false,
    reason: null, continuity_claim: null});
  assert.deepEqual(calls[0].args.slice(calls[0].args.indexOf("--model"),
    calls[0].args.indexOf("--model") + 2), ["--model", "opus"]);
});

test("native Claude adapter fails closed on profile/catalog authorization mismatches", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-catalog-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  const request = async ({candidate = profile(), projection = catalog, now = "2026-09-06T12:00:00Z"}, id) => {
    const adapter = new NativeClaudeCodeReviewerAdapter({
      registry: new ReviewerProfileRegistry({profiles: [candidate]}), workspaceRoot: root,
      stateRoot: path.join(root, `state-${id}`), credentialSourcePath: await credentials(root),
      catalogSource, transportScript: path.join(root, "transport.py"),
      now: () => Date.parse(now), executeProcess: async () => { throw new Error("provider entry is forbidden"); },
    });
    return adapter.execute({instanceId: id, profileId: candidate.profileId, subject,
      catalogProjection: projection, rawEventPolicy: policy, reviewBoundary,
      roleInstructions: "Read-only review."});
  };
  await assert.rejects(request({candidate: profile({provider: "openrouter"})}, "provider"),
    /requires an Anthropic reviewer profile/);
  await assert.rejects(request({projection: {...catalog, models: [{...catalog.models[0], slug: "opus"}]}}, "model"),
    /absent from the bound catalog/);
  await assert.rejects(request({projection: {...catalog, models: [{...catalog.models[0], capabilities: ["structured_output"]}]}}, "capability"),
    /missing capability repository_read/);
  await assert.rejects(request({projection: {...catalog, models: [{...catalog.models[0], routingConstraints: ["openrouter"]}]}}, "routing"),
    /does not authorize direct Anthropic routing/);
  await assert.rejects(request({projection: {...catalog, models: [{...catalog.models[0], routingConstraints: []}]}}, "routing-empty"),
    /does not authorize direct Anthropic routing/);
  await assert.rejects(request({projection: {...catalog, expiresAt: "2026-09-05T00:00:00Z"}}, "stale"),
    /catalog is not fresh/);
});

test("native Claude model choice cannot drift within a retained session or reuse a fresh binding", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-binding-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  const sonnet = profile();
  const opus = profile({profileId: "anthropic.claude-code.opus-review-v1",
    requestedModel: "opus", reasoning: "high"});
  let calls = 0;
  const adapter = new NativeClaudeCodeReviewerAdapter({
    registry: new ReviewerProfileRegistry({profiles: [sonnet, opus]}), workspaceRoot: root,
    stateRoot: path.join(root, "state"), credentialSourcePath: await credentials(root),
    catalogSource, transportScript: path.join(root, "transport.py"), executeProcess: async (request) => {
      calls += 1;
      const command = request.args.slice(request.args.indexOf("--") + 1);
      const flag = command.includes("--session-id") ? "--session-id" : "--resume";
      return transportResult(request, {type: "result", subtype: "success",
        session_id: command[command.indexOf(flag) + 1], model: "claude-sonnet-5",
        structured_output: result});
    },
  });
  const initial = await adapter.execute({instanceId: "bound", profileId: sonnet.profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review."});
  await assert.rejects(adapter.execute({instanceId: "bound", profileId: opus.profileId, subject,
    catalogProjection: {...catalog, models: [{...catalog.models[0], slug: "opus"}]},
    rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review.",
    continuationSessionId: initial.runtimeSessionId}), /immutable profile and model binding/);
  await assert.rejects(adapter.execute({instanceId: "bound", profileId: sonnet.profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review."}),
  /fresh session already has a binding/);
  assert.equal(calls, 1);
});

test("native Claude success requires exact transport model provenance and no fallback", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-provenance-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  const create = async (id, executeProcess) => new NativeClaudeCodeReviewerAdapter({
    registry: new ReviewerProfileRegistry({profiles: [profile()]}), workspaceRoot: root,
    stateRoot: path.join(root, `state-${id}`), credentialSourcePath: await credentials(root),
    catalogSource, transportScript: path.join(root, "transport.py"), executeProcess,
  });
  const execute = (adapter, id) => adapter.execute({instanceId: id, profileId: profile().profileId,
    subject, catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary,
    roleInstructions: "Review."});
  const envelope = (request) => {
    const command = request.args.slice(request.args.indexOf("--") + 1);
    return {type: "result", subtype: "success",
      session_id: command[command.indexOf("--session-id") + 1], model: "claude-sonnet-5",
      structured_output: result};
  };
  await assert.rejects(execute(await create("missing", async (request) => ({exitCode: 0, stderr: "",
    stdout: JSON.stringify(envelope(request))})), "missing"), /transport receipt is unavailable/);
  await assert.rejects(execute(await create("fallback", async (request) => transportResult(request,
    envelope(request), {receiptOverrides: {failover: {attempted: true, allowed: true}}})), "fallback"),
  /exact direct-Anthropic attempt binding/);
  await assert.rejects(execute(await create("unobserved", async (request) => transportResult(request,
    envelope(request), {observedModels: []})), "unobserved"),
  /exact direct-Anthropic attempt binding/);
  await assert.rejects(execute(await create("model", async (request) => transportResult(request,
    envelope(request), {observedModels: ["claude-opus"]})), "model"),
  /result model differs from the exact transport provenance/);
});

test("native Claude adapter validates evidence by fields rather than provider property order", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-evidence-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  const credentialSourcePath = await credentials(root);
  const executeWithEvidence = (evidence) => new NativeClaudeCodeReviewerAdapter({
    registry: new ReviewerProfileRegistry({profiles: [profile()]}), workspaceRoot: root,
    stateRoot: path.join(root, `state-${evidence.sha256.slice(0, 8)}`), credentialSourcePath,
    catalogSource, transportScript: path.join(root, "transport.py"),
    executeProcess: async (request) => {
      const sessionIndex = request.args.indexOf("--session-id");
      const resumeIndex = request.args.indexOf("--resume");
      return transportResult(request, {type: "result", subtype: "success",
        session_id: request.args[(sessionIndex >= 0 ? sessionIndex : resumeIndex) + 1], structured_output: {...result,
          decisiveEvidence: [evidence]}});
    },
  });
  const reordered = {sha256: "a".repeat(64), endLine: 1,
    path: "app-server/src/index.mjs", startLine: 1};
  const accepted = await executeWithEvidence(reordered).execute({instanceId: "reordered-evidence",
    profileId: profile().profileId, subject, catalogProjection: catalog,
    rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review."});
  assert.equal(accepted.failure, null);

  const forged = {...reordered, sha256: "f".repeat(64)};
  await assert.rejects(executeWithEvidence(forged).execute({instanceId: "forged-evidence",
    profileId: profile().profileId, subject, catalogProjection: catalog,
    rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review."}),
  /outside the host-computed review catalog/);
  const remediationAdapter = executeWithEvidence(forged);
  const remediationSession = remediationAdapter.runtimeSessionId("forged-remediation-evidence");
  await assert.rejects(remediationAdapter.execute({instanceId: "forged-remediation-evidence",
    profileId: profile().profileId, subject, catalogProjection: catalog,
    rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review.",
    continuationSessionId: remediationSession}), /outside the host-computed review catalog/);
});

test("native Claude correction prompt states immutable finding and result closure contracts", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-correction-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  const credentialSourcePath = await credentials(root);
  let prompt;
  const priorFinding = {id: "prior", severity: "medium", title: "Prior title",
    evidence: result.decisiveEvidence, observed: "Prior observation.",
    violatedExpectation: "Prior expectation.", consequence: "Prior consequence.",
    basis: "reproduced", confidence: "high", recommendedRemediation: "Prior remediation.",
    status: "open", remediationEvidence: []};
  const corrected = {...result, findings: [{...priorFinding, status: "verified_resolved",
    remediationEvidence: result.decisiveEvidence}]};
  const adapter = new NativeClaudeCodeReviewerAdapter({
    registry: new ReviewerProfileRegistry({profiles: [profile()]}), workspaceRoot: root,
    stateRoot: path.join(root, "state"), credentialSourcePath, catalogSource,
    transportScript: path.join(root, "transport.py"), executeProcess: async (request) => {
      prompt = await promptFromRequest(request);
      const resumeIndex = request.args.indexOf("--resume");
      return transportResult(request, {type: "result", subtype: "success",
        session_id: request.args[resumeIndex + 1], structured_output: corrected});
    },
  });
  const session = adapter.runtimeSessionId("correction");
  await adapter.execute({instanceId: "correction", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Read-only review.",
    continuationSessionId: session, resultCorrection: {message: "contract rejected",
      rejectedResult: {...result, limitations: ["Cannot accept."]},
      requiredPriorResult: {...result, verdict: "remediation_required", findings: [priorFinding]}}});
  assert.match(prompt, /copy these immutable fields exactly, without paraphrase or correction/);
  assert.match(prompt, /You may change only status and remediationEvidence/);
  assert.match(prompt, /Do not repair an old citation, replace old evidence with current-subject evidence/);
  assert.match(prompt, /top-level result for a generic review and the nested result field for a specialist review/);
  assert.match(prompt, /acceptable_as_is requires non-empty decisiveEvidence, an empty limitations array/);
  assert.match(prompt, /Never combine acceptable_as_is with a limitation or unresolved finding/);
});

test("native Claude adapter refreshes isolated credentials only after exact retained authentication failure", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-auth-refresh-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  const stateRoot = path.join(root, "state");
  const credentialSourcePath = await credentials(root);
  await writeFile(credentialSourcePath, '{"fixture":"refreshed-subscription"}\n', {mode: 0o600});
  const adapter = new NativeClaudeCodeReviewerAdapter({registry: new ReviewerProfileRegistry({profiles: [profile()]}),
    workspaceRoot: root, stateRoot, credentialSourcePath, catalogSource,
    transportScript: path.join(root, "transport.py"), executeProcess: async (request) => {
      const resumeFlag = request.args.indexOf("--resume");
      return transportResult(request, {type: "result", subtype: "success",
        session_id: request.args[resumeFlag + 1], model: "claude-sonnet-5", structured_output: result});
    }});

  const prepare = async (instanceId, assistantMessages, {newerDecoy = false} = {}) => {
    const session = adapter.runtimeSessionId(instanceId);
    const instanceRoot = path.join(stateRoot, "native-claude", digest(instanceId));
    const configRoot = path.join(instanceRoot, "config");
    const sessionRoot = path.join(configRoot, "projects", "fixture");
    await mkdir(sessionRoot, {recursive: true});
    await writeFile(path.join(configRoot, ".credentials.json"), '{"fixture":"stale-subscription"}\n', {mode: 0o600});
    const observedAt = new Date();
    const receiptName = "failed.transport.json";
    await writeFile(path.join(instanceRoot, receiptName), `${JSON.stringify({
      request: {session_id: session}, result: "failed", attempts: [{duration_ms: 1_000}],
    })}\n`);
    await writeFile(path.join(instanceRoot, "latest-attempt.json"), `${JSON.stringify({
      schemaVersion: 1, attemptId: "fixture-attempt", sessionId: session, transportReceipt: receiptName,
    })}\n`);
    const records = assistantMessages.map(({text, timestamp = observedAt.toISOString()}) => JSON.stringify({
      type: "assistant", timestamp, message: {role: "assistant", content: [{type: "text", text}]},
    }));
    await writeFile(path.join(sessionRoot, `${session}.jsonl`), `${records.join("\n")}\n`);
    if (newerDecoy) {
      await writeFile(path.join(instanceRoot, "newer-decoy.transport.json"), `${JSON.stringify({
        request: {session_id: session}, result: "success", attempts: [{duration_ms: 1}],
      })}\n`);
    }
    return {session, configRoot};
  };

  const exact = await prepare("exact-auth-failure", [
    {text: "Preparing authentication state."},
    {text: "Not logged in · Please run /login"},
    {text: "Authentication is required."},
  ], {newerDecoy: true});
  const refreshed = await adapter.execute({instanceId: "exact-auth-failure", profileId: profile().profileId,
    subject, catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review.",
    continuationSessionId: exact.session, refreshCredentials: true});
  assert.equal(refreshed.failure, null);
  assert.equal(await readFile(path.join(exact.configRoot, ".credentials.json"), "utf8"),
    '{"fixture":"refreshed-subscription"}\n');

  const legacy = await prepare("legacy-auth-failure", [
    {text: "Failed to authenticate: OAuth session expired and could not be refreshed"},
  ]);
  const recordedRecovery = await adapter.recoverFailure("legacy-auth-failure");
  assert.equal(recordedRecovery.failureSignature, "authentication_required");
  const legacyInstanceRoot = path.dirname(legacy.configRoot);
  await rm(path.join(legacyInstanceRoot, "latest-attempt.json"));
  assert.equal(await adapter.recoverFailure("legacy-auth-failure"), null);
  assert.deepEqual(await adapter.recoverFailure("legacy-auth-failure", {recordedRecovery}), recordedRecovery);
  const reconstructed = JSON.parse(await readFile(path.join(legacyInstanceRoot, "latest-attempt.json"), "utf8"));
  assert.equal(reconstructed.sessionId, legacy.session);
  assert.equal(reconstructed.transportReceipt, "failed.transport.json");
  assert.deepEqual(reconstructed.legacyEvidence, {
    transportReceiptDigest: recordedRecovery.transportReceiptDigest,
    sessionArtifactDigest: recordedRecovery.sessionArtifactDigest,
  });

  await rm(path.join(legacyInstanceRoot, "latest-attempt.json"));
  const concurrentRecoveries = await Promise.all([
    adapter.recoverFailure("legacy-auth-failure", {recordedRecovery}),
    adapter.recoverFailure("legacy-auth-failure", {recordedRecovery}),
  ]);
  assert.deepEqual(concurrentRecoveries, [recordedRecovery, recordedRecovery]);
  assert.equal(JSON.parse(await readFile(path.join(legacyInstanceRoot, "latest-attempt.json"), "utf8"))
    .transportReceipt, "failed.transport.json");

  await rm(path.join(legacyInstanceRoot, "latest-attempt.json"));
  assert.equal(await adapter.recoverFailure("legacy-auth-failure", {recordedRecovery: {
    ...recordedRecovery, transportReceiptDigest: "0".repeat(64),
  }}), null);

  const other = await prepare("other-failure", [
    {text: "Not logged in · Please run /login", timestamp: "2026-01-01T00:00:00.000Z"},
    {text: "Quota unavailable"},
  ]);
  assert.equal(await adapter.recoverFailure("other-failure"), null);
  const preserved = await adapter.execute({instanceId: "other-failure", profileId: profile().profileId,
    subject, catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review.",
    continuationSessionId: other.session});
  assert.equal(preserved.failure, null);
  assert.equal(await readFile(path.join(other.configRoot, ".credentials.json"), "utf8"),
    '{"fixture":"stale-subscription"}\n');
});

test("native Claude adapter preserves transport failure and rejects subject or UUID drift", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-refusal-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  const registry = new ReviewerProfileRegistry({profiles: [profile()]});
  const credentialSourcePath = await credentials(root);
  let mode = "transport";
  const adapter = new NativeClaudeCodeReviewerAdapter({registry, workspaceRoot: root, stateRoot: path.join(root, "state"),
    credentialSourcePath,
    catalogSource,
    transportScript: path.join(root, "transport.py"), executeProcess: async (request) => {
      if (mode === "transport") return transportResult(request, null, {exitCode: 1, stderr: "quota"});
      const index = request.args.indexOf("--session-id"); const session = request.args[index + 1];
      return transportResult(request, {type: "result", subtype: "success",
        session_id: mode === "uuid" ? "00000000-0000-4000-8000-000000000000" : session,
        structured_output: {...result, subject: {...subject, tree: "drift"},
          ...(mode === "subject" ? {decisiveEvidence: [{...result.decisiveEvidence[0],
            sha256: "f".repeat(64)}]} : {})}});
    }});
  const failed = await adapter.execute({instanceId: "failure", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review."});
  assert.equal(failed.failure.kind, "transport");
  mode = "uuid";
  await assert.rejects(adapter.execute({instanceId: "uuid", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review."}), /different session UUID/);
  mode = "subject";
  const drift = await adapter.execute({instanceId: "subject", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review."});
  assert.equal(drift.failure.kind, "subject_drift");
});

test("native Claude adapter fails closed on catalog provenance drift and inherited cloud routing", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-route-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  let calls = 0;
  const credentialSourcePath = await credentials(root);
  const create = (baseEnvironment = {}) => new NativeClaudeCodeReviewerAdapter({
    registry: new ReviewerProfileRegistry({profiles: [profile()]}), workspaceRoot: root,
    stateRoot: path.join(root, "state"), transportScript: path.join(root, "transport.py"),
    catalogSource, baseEnvironment, credentialSourcePath,
    executeProcess: async (request) => { calls += 1; return transportResult(request, null, {exitCode: 1}); },
  });
  const request = {instanceId: "route", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review."};
  await assert.rejects(create().execute({...request,
    catalogProjection: {...catalog, sourceSha256: "c".repeat(64)}}), /catalog provenance differs/);
  for (const name of ["CLAUDE_CODE_USE_BEDROCK", "CLAUDE_CODE_USE_VERTEX", "CLAUDE_CODE_USE_FOUNDRY",
    "ANTHROPIC_BEDROCK_BASE_URL", "ANTHROPIC_VERTEX_BASE_URL", "ANTHROPIC_FOUNDRY_BASE_URL"]) {
    await assert.rejects(create({[name]: "1"}).execute(request), new RegExp(name));
  }
  assert.equal(calls, 0);
});

test("native Claude adapter replaces generic preamble for the agent-instruction obligation", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-specialist-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  let prompt;
  const credentialSourcePath = await credentials(root);
  const specialistSubject = subject;
  const specialistResult = {schemaVersion: 1, perspective: "agent-instruction-review", subject: specialistSubject,
    closureRevision: "d".repeat(64), applicability: "applicable", applicabilityReason: "Normative text is present.",
    result: {...result, subject: specialistSubject}, findingDetails: [], limitations: ["Fixture."]};
  const adapter = new NativeClaudeCodeReviewerAdapter({registry: new ReviewerProfileRegistry({profiles: [profile()]}),
    workspaceRoot: root, stateRoot: path.join(root, "state"), catalogSource,
    credentialSourcePath,
    transportScript: path.join(root, "transport.py"), baseEnvironment: {}, executeProcess: async (request) => {
      prompt = await promptFromRequest(request); const sessionIndex = request.args.indexOf("--session-id");
      return transportResult(request, {type: "result", subtype: "success",
        session_id: request.args[sessionIndex + 1], structured_output: specialistResult});
    }});
  await adapter.execute({instanceId: "specialist", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary,
    roleInstructions: "You perform one advisory generic implementation review. Return the generic implementation-review schema.\n\nWORK_ENGINE_AGENT_INSTRUCTION_REVIEW_V1\n{}"});
  assert.doesNotMatch(prompt, /You perform one advisory generic implementation review/);
  assert.doesNotMatch(prompt, /Return the generic implementation-review schema/);
  assert.match(prompt, /one advisory, read-only agent-instruction specialist review/);
  assert.match(prompt, /generic implementation-review protocol and schema do not apply/);
  assert.match(prompt, /WORK_ENGINE_AGENT_INSTRUCTION_REVIEW_V1/);
});

test("native Claude adapter fails before process entry when isolated credentials are unavailable", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-no-auth-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  let calls = 0;
  const adapter = new NativeClaudeCodeReviewerAdapter({
    registry: new ReviewerProfileRegistry({profiles: [profile()]}), workspaceRoot: root,
    stateRoot: path.join(root, "state"), catalogSource,
    credentialSourcePath: path.join(root, "missing-credentials.json"),
    transportScript: path.join(root, "transport.py"),
    executeProcess: async () => { calls += 1; throw new Error("must not run"); },
  });
  const execution = await adapter.execute({instanceId: "no-auth", profileId: profile().profileId,
    subject, catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary, roleInstructions: "Review."});
  assert.equal(calls, 0);
  assert.equal(execution.failure.failureSignature, "authentication_unavailable");
  assert.equal(execution.failure.providerEntry, "not_entered");
  assert.equal(execution.failure.sessionAvailable, false);
  assert.deepEqual(execution.failure.recovery, {schemaVersion: 1,
    failureSignature: "authentication_unavailable", providerEntry: "not_entered",
    sessionAvailable: false, sessionId: execution.runtimeSessionId});
});

test("native Claude adapter records and retries an exact pre-spawn process failure", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-pre-spawn-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  let calls = 0;
  const adapter = new NativeClaudeCodeReviewerAdapter({
    registry: new ReviewerProfileRegistry({profiles: [profile()]}), workspaceRoot: root,
    stateRoot: path.join(root, "state"), catalogSource, credentialSourcePath: await credentials(root),
    transportScript: path.join(root, "transport.py"), executeProcess: async (request) => {
      calls += 1;
      if (calls === 1) throw Object.assign(new Error("argument list too long"), {code: "E2BIG"});
      const sessionIndex = request.args.indexOf("--session-id");
      return transportResult(request, {type: "result", subtype: "success",
        session_id: request.args[sessionIndex + 1], model: "claude-sonnet-5", structured_output: result});
    },
  });
  const request = {instanceId: "pre-spawn", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy,
    reviewBoundary: {...reviewBoundary, changeDiff: "x".repeat(150_000)}, roleInstructions: "Review."};
  const failed = await adapter.execute(request);
  assert.equal(failed.failure.failureSignature, "process_start_failed");
  assert.equal(failed.failure.providerEntry, "not_entered");
  assert.equal(failed.failure.recovery.errorCode, "E2BIG");
  assert.equal(failed.transportReceipt, null);
  assert.deepEqual(await adapter.recoverFailure("pre-spawn"), failed.failure.recovery);
  await writeFile(adapter.credentialSourcePath, '{"fixture":"refreshed-subscription"}\n', {mode: 0o600});
  const retried = await adapter.execute({...request, preSpawnRetry: true, refreshCredentials: true});
  assert.equal(retried.failure, null);
  assert.equal(calls, 2);
  assert.equal(await readFile(path.join(root, "state", "native-claude", digest("pre-spawn"),
    "config", ".credentials.json"), "utf8"), '{"fixture":"refreshed-subscription"}\n');
});

test("native Claude adapter does not relabel an unclassified process exception as pre-provider", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "native-claude-adapter-uncertain-spawn-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  const adapter = new NativeClaudeCodeReviewerAdapter({
    registry: new ReviewerProfileRegistry({profiles: [profile()]}), workspaceRoot: root,
    stateRoot: path.join(root, "state"), catalogSource, credentialSourcePath: await credentials(root),
    transportScript: path.join(root, "transport.py"),
    executeProcess: async () => { throw new Error("unclassified transport exception"); },
  });
  await assert.rejects(adapter.execute({instanceId: "uncertain-spawn", profileId: profile().profileId,
    subject, catalogProjection: catalog, rawEventPolicy: policy, reviewBoundary,
    roleInstructions: "Review."}), /process outcome is uncertain/);
  assert.equal(await adapter.recoverFailure("uncertain-spawn"), null);
});
