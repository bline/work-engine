import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { copyFile, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { existsSync, readdirSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { createSupervisorCampaignCapabilityDefinitions } from
  "../src/services/slice-campaign/capability-contract.mjs";
import { createSupervisorCampaignCapabilityHostRuntime } from
  "../src/services/slice-campaign/capability-host-runtime.mjs";
import { PINNED_PROTOCOL, assertCompatibleCodexCliOutput } from "../src/capabilities.mjs";

const root = path.resolve(new URL("../..", import.meta.url).pathname);
const sessionId = "11111111-1111-4111-8111-111111111111";
const claimId = "22222222-2222-4222-8222-222222222222";
const messageId = "33333333-3333-4333-8333-333333333333";

function git(repository, ...args) {
  return execFileSync("git", ["-C", repository, ...args], {encoding: "utf8"}).trim();
}

async function fixture(t) {
  const repository = await mkdtemp(path.join(os.tmpdir(), "work-engine-coordination-repo-"));
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "work-engine-coordination-state-"));
  t.after(async () => { await rm(repository, {recursive: true, force: true});
    await rm(stateRoot, {recursive: true, force: true}); });
  const scripts = path.join(repository, "skills/durable-state/scripts");
  await mkdir(scripts, {recursive: true});
  for (const file of ["codex_chatboard.py", "durable_state.py"]) {
    await copyFile(path.join(root, "skills/durable-state/scripts", file), path.join(scripts, file));
  }
  await writeFile(path.join(repository, "README.md"), "temporary coordination fixture\n");
  git(repository, "init", "--quiet", "--initial-branch=main");
  git(repository, "config", "user.name", "Coordination Test");
  git(repository, "config", "user.email", "coordination@example.invalid");
  git(repository, "add", ".");
  git(repository, "commit", "--quiet", "-m", "fixture");
  return {repository, stateRoot};
}

function legacy() {
  return {identity: {backend: "fixture"}, preflight() {}, finalize() {},
    validateReceipt(value) { return value; }, checkpoint() {}, offer() {}, resumeTerminal() {}};
}

function pinnedCodexExecutable() {
  const candidates = [process.env.WORK_ENGINE_CODEX, "codex"].filter(Boolean);
  const cache = path.join(os.homedir(), ".npm/_npx");
  if (existsSync(cache)) {
    for (const relative of readdirSync(cache, {recursive: true})) {
      if (relative.endsWith("/vendor/x86_64-unknown-linux-musl/bin/codex")) {
        candidates.push(path.join(cache, relative));
      }
    }
  }
  for (const candidate of candidates) {
    try {
      assertCompatibleCodexCliOutput(execFileSync(candidate, ["--version"], {encoding: "utf8"}));
      return candidate;
    } catch { /* Continue to another installed executable. */ }
  }
  throw new Error(`no installed codex-cli ${PINNED_PROTOCOL.codexCliVersion} executable is available`);
}

test("read-only supervisor coordination remains durable across stable-host reconstruction", async (t) => {
  const {repository, stateRoot} = await fixture(t);
  let runtime = await createSupervisorCampaignCapabilityHostRuntime({
    workspaceRoot: repository, stateRoot, canonicalBranches: ["main"],
    legacyAdapterFactory: async () => legacy(),
  });
  t.after(() => runtime.close());
  const definitions = () => createSupervisorCampaignCapabilityDefinitions(
    (effect) => runtime.dispatch({generationId: "generation-coordination", effect}));
  let capability = definitions().get("capability.operational_coordination");
  assert.deepEqual((await capability.handler({operation: "read", input: {since: 0, limit: 100}}))
    .result.messages, []);
  const claimed = await capability.handler({operation: "claim", input: {
    resource: "campaign:temporary", author: "slice-supervisor", session_id: sessionId,
    claim_id: claimId, ttl_seconds: 3600, note: "temporary vertical proof",
  }});
  assert.equal(claimed.result.claim.authority, "advisory_coordination_only");
  const replay = await capability.handler({operation: "claim", input: {
    resource: "campaign:temporary", author: "slice-supervisor", session_id: sessionId,
    claim_id: claimId, ttl_seconds: 3600, note: "temporary vertical proof",
  }});
  assert.equal(replay.result.claim.claim_id, claimId);
  await capability.handler({operation: "post", input: {
    author: "slice-supervisor", session_id: sessionId, topic: "temporary-proof",
    body: "Stable host coordination proof.", references: ["fixture:temporary"],
    message_id: messageId,
  }});
  runtime.close();

  runtime = await createSupervisorCampaignCapabilityHostRuntime({
    workspaceRoot: repository, stateRoot, canonicalBranches: ["main"],
    legacyAdapterFactory: async () => legacy(),
  });
  capability = definitions().get("capability.operational_coordination");
  const recovered = (await capability.handler({operation: "read", input: {since: 0, limit: 100}})).result;
  assert.equal(recovered.claims["campaign:temporary"].claim_id, claimId);
  assert.equal(recovered.messages[0].message_id, messageId);
  await capability.handler({operation: "release", input: {
    resource: "campaign:temporary", session_id: sessionId, claim_id: claimId,
  }});

  const canonical = JSON.parse(execFileSync("python3", [
    path.join(repository, "skills/durable-state/scripts/codex_chatboard.py"),
    "--repository", repository, "read",
  ], {encoding: "utf8"}));
  assert.equal(canonical.claims["campaign:temporary"], undefined);
  assert.equal(canonical.messages[0].message_id, messageId);
  assert.equal(runtime.identity.operational_coordination.repository, repository);
});

test("integration mode verifies the pinned Codex CLI before inhabitation proof", {
  skip: process.env.WORK_ENGINE_APP_SERVER_INTEGRATION !== "1",
}, () => {
  const executable = pinnedCodexExecutable();
  assert.equal(assertCompatibleCodexCliOutput(
    execFileSync(executable, ["--version"], {encoding: "utf8"})),
  PINNED_PROTOCOL.codexCliVersion);
});
