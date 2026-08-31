import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, readFile, readdir, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { reviewerRuntimeDigest as digest, OpenRouterCodexReviewerAdapter, ReviewerProfileRegistry } from "../../../src/index.mjs";

const fixtureDirectory = fileURLToPath(new URL("../../fixtures/reviewer-runtime/", import.meta.url));
const fixtureProcess = path.join(fixtureDirectory, "fake-codex-openrouter.mjs");
const subject = { commit: "c", tree: "t", patchIdentity: "p" };
function profile() { const value = { schemaVersion: 1, profileId: "p", enabled: true, requestedModel: "openai/gpt-5.2-codex", provider: "openrouter", reasoning: "high", capabilities: ["structured_output"], outputSchema: "work-engine.implementation-review.v1", effectiveInstructions: "Review.", isolatedHome: true, limitations: ["Catalog admission is not inference proof."], acceptingAuthority: "plan" }; value.configurationDigest = digest(value); return value; }
const catalog = JSON.parse(await readFile(path.join(fixtureDirectory, "catalog-projection.json"), "utf8"));
const policy = { classification: "confidential", access: "episode actors", retention: "attempt lifetime", exactRetentionAuthorized: false, redaction: "omit bodies", tamperEvidence: "sha256" };
const roleInstructions = "Canonical role: preserve configured versus observed identity and do not claim acceptance authority.";
const completed = (result = { schemaVersion: 1, subject, verdict: "acceptable_as_is", findings: [], decisiveEvidence: [{ path: "x", startLine: 1, endLine: 1, sha256: "b".repeat(64) }], limitations: [] }) => ({ exitCode: 0, stderr: "", stdout: `${JSON.stringify({ type: "review.completed", observed: { model: "openai/gpt-5.2-codex", provider: "openrouter" }, result })}\n` });
function executeFixture({ env, args, input }) {
  assert.deepEqual(args.slice(0, 4), ["exec", "--json", "--model", "openai/gpt-5.2-codex"]);
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [fixtureProcess], { env, stdio: ["pipe", "pipe", "pipe"] });
    let stdout = "", stderr = "";
    child.stdout.setEncoding("utf8"); child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => stdout += chunk); child.stderr.on("data", (chunk) => stderr += chunk);
    child.on("error", reject); child.on("close", (exitCode) => resolve({ exitCode, stdout, stderr })); child.stdin.end(input);
  });
}

test("adapter retains one isolated home for exact-session continuation and retires it", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "review-adapter.")); t.after(() => rm(root, { recursive: true, force: true }));
  const homes = [];
  const adapter = new OpenRouterCodexReviewerAdapter({ registry: new ReviewerProfileRegistry({ profiles: [profile()] }), isolatedRoot: root, now: () => Date.parse("2026-08-30T12:00:00Z"), executeProcess: async (request) => { homes.push(request.env.CODEX_HOME); assert.match(await readFile(path.join(request.env.CODEX_HOME, "config.toml"), "utf8"), /openai\/gpt-5.2-codex/); const delivered=JSON.parse(request.input).instructions; assert.match(delivered,/Canonical role/); assert.match(delivered,/subordinate to the canonical role instructions/); assert.match(delivered,/Review\./); assert.match(delivered,/Catalog admission is not inference proof/); return executeFixture(request); } });
  const initial = await adapter.execute({ instanceId: "episode", profileId: "p", subject, catalogProjection: catalog, rawEventPolicy: policy, roleInstructions });
  const continued = await adapter.execute({ instanceId: "episode", profileId: "p", subject, catalogProjection: catalog, rawEventPolicy: policy, continuationSessionId: "session-1", roleInstructions });
  assert.equal(initial.isolation.freshEntry, true); assert.equal(continued.isolation.continuation, true);
  assert.equal(homes[0], homes[1]); assert.deepEqual(await readdir(root), [path.basename(homes[0])]);
  assert.equal(await adapter.retire("episode"), true); assert.deepEqual(await readdir(root), []);
});

test("adapter rejects missing retained state and duplicate fresh entry", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "review-adapter.")); t.after(() => rm(root, { recursive: true, force: true }));
  const adapter = new OpenRouterCodexReviewerAdapter({ registry: new ReviewerProfileRegistry({ profiles: [profile()] }), isolatedRoot: root, now: () => Date.parse("2026-08-30T12:00:00Z"), executeProcess: async () => completed() });
  await assert.rejects(adapter.execute({ instanceId: "missing", profileId: "p", subject, catalogProjection: catalog, rawEventPolicy: policy, continuationSessionId: "session-1", roleInstructions }), /requires retained reviewer state/);
  await assert.rejects(adapter.execute({ instanceId: "missing-role", profileId: "p", subject, catalogProjection: catalog, rawEventPolicy: policy }), /canonical role instructions are required/);
  await adapter.execute({ instanceId: "episode", profileId: "p", subject, catalogProjection: catalog, rawEventPolicy: policy, roleInstructions });
  await assert.rejects(adapter.execute({ instanceId: "episode", profileId: "p", subject, catalogProjection: catalog, rawEventPolicy: policy, roleInstructions }), /already has retained state/);
  await adapter.retire("episode");
});

test("adapter fails closed for catalog, capability, routing, and subject drift", async () => {
  const registry = new ReviewerProfileRegistry({ profiles: [profile()] });
  const request = (projection, instanceId) => new OpenRouterCodexReviewerAdapter({ registry, now: () => Date.parse("2026-08-30T12:00:00Z"), executeProcess: async () => completed() }).execute({ instanceId, profileId: "p", subject, catalogProjection: projection, rawEventPolicy: policy, roleInstructions });
  await assert.rejects(request({ ...catalog, expiresAt: "2026-08-30T01:00:00Z" }, "stale"), /not fresh/);
  await assert.rejects(request({ ...catalog, models: catalog.models.map((model) => ({ ...model, slug: "other" })) }, "model"), /model\/provider absent/);
  await assert.rejects(request({ ...catalog, models: catalog.models.map((model) => ({ ...model, capabilities: [] })) }, "capability"), /missing capability/);
  await assert.rejects(request({ ...catalog, models: catalog.models.map((model) => ({ ...model, routingConstraints: ["unresolved"] })) }, "routing"), /routing constraints unresolved/);
  const adapter = new OpenRouterCodexReviewerAdapter({ registry, now: () => Date.parse("2026-08-30T12:00:00Z"), executeProcess: async () => completed({ schemaVersion: 1, subject: { ...subject, tree: "other" } }) });
  const receipt = await adapter.execute({ instanceId: "subject", profileId: "p", subject, catalogProjection: catalog, rawEventPolicy: policy, roleInstructions });
  assert.equal(receipt.failure.kind, "subject_drift"); assert.equal(receipt.result, null); await adapter.retire("subject");
});
