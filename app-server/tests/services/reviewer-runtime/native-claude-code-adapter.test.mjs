import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { NativeClaudeCodeReviewerAdapter } from "../../../src/services/reviewer-runtime/native-claude-code-adapter.mjs";
import { ReviewerProfileRegistry } from "../../../src/services/reviewer-runtime/profile-registry.mjs";
import { digest } from "../../../src/services/reviewer-runtime/contract.mjs";

const subject = {commit: "candidate", tree: "tree", patchIdentity: "patch"};
const result = {schemaVersion: 1, subject, verdict: "acceptable_as_is", findings: [],
  decisiveEvidence: [{path: "app-server/src/index.mjs", startLine: 1, endLine: 1, sha256: "a".repeat(64)}], limitations: []};
function profile() {
  const value = {schemaVersion: 1, profileId: "anthropic.claude-code.sonnet-review-v1", enabled: true,
    requestedModel: "sonnet", provider: "anthropic", reasoning: "medium",
    capabilities: ["structured_output", "repository_read"], outputSchema: "work-engine.implementation-review.v1",
    effectiveInstructions: "Review exact subject.", isolatedHome: true,
    limitations: ["Direct Anthropic only."], acceptingAuthority: "accepted-plan"};
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
      return {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
        session_id: session, model: "claude-sonnet-5", structured_output: result})};
    }});
  const initial = await adapter.execute({instanceId: "episode", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, roleInstructions: "Read-only review."});
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
  const prompt = calls[0].args.at(-1);
  assert.match(prompt, /Execution-profile constraints are subordinate to the selected review obligation/);
  assert.match(prompt, /Review exact subject\./);
  const continued = await adapter.execute({instanceId: "episode", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, roleInstructions: "Read-only review.",
    continuationSessionId: initial.runtimeSessionId});
  assert.equal(continued.receipt.continuity, "same_session_resume");
  assert.equal(calls[1].args.includes("--resume"), true);
  await assert.rejects(adapter.execute({instanceId: "episode", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, roleInstructions: "Read-only review.",
    continuationSessionId: "00000000-0000-4000-8000-000000000000"}), /pre-registered session/);
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
      if (mode === "transport") return {exitCode: 1, stdout: "", stderr: "quota"};
      const index = request.args.indexOf("--session-id"); const session = request.args[index + 1];
      return {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
        session_id: mode === "uuid" ? "00000000-0000-4000-8000-000000000000" : session,
        structured_output: {...result, subject: {...subject, tree: "drift"}}})};
    }});
  const failed = await adapter.execute({instanceId: "failure", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, roleInstructions: "Review."});
  assert.equal(failed.failure.kind, "transport");
  mode = "uuid";
  await assert.rejects(adapter.execute({instanceId: "uuid", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, roleInstructions: "Review."}), /different session UUID/);
  mode = "subject";
  const drift = await adapter.execute({instanceId: "subject", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, roleInstructions: "Review."});
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
    executeProcess: async () => { calls += 1; return {exitCode: 1, stdout: "", stderr: ""}; },
  });
  const request = {instanceId: "route", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy, roleInstructions: "Review."};
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
      prompt = request.args.at(-1); const sessionIndex = request.args.indexOf("--session-id");
      return {exitCode: 0, stderr: "", stdout: JSON.stringify({type: "result", subtype: "success",
        session_id: request.args[sessionIndex + 1], structured_output: specialistResult})};
    }});
  await adapter.execute({instanceId: "specialist", profileId: profile().profileId, subject,
    catalogProjection: catalog, rawEventPolicy: policy,
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
    subject, catalogProjection: catalog, rawEventPolicy: policy, roleInstructions: "Review."});
  assert.equal(calls, 0);
  assert.equal(execution.failure.failureSignature, "authentication_unavailable");
  assert.equal(execution.failure.providerEntry, "not_entered");
  assert.equal(execution.failure.sessionAvailable, false);
  assert.deepEqual(execution.failure.recovery, {schemaVersion: 1,
    failureSignature: "authentication_unavailable", providerEntry: "not_entered",
    sessionAvailable: false, sessionId: execution.runtimeSessionId});
});
