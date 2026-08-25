import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  CodexAppServerAdapter,
  ExactSkillResolver,
  FileRoleBindingRegistry,
  ManifestRoleRuntime,
  PINNED_PROTOCOL,
  StdioJsonRpcTransport,
  StrategicPlannerRuntime,
  assertCompatibleServer,
  loadRuntimeManifest,
} from "../src/index.mjs";

const enabled = process.env.WORK_ENGINE_APP_SERVER_INTEGRATION === "1";
const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "../..");

async function sha256(filePath) {
  return createHash("sha256").update(await readFile(filePath)).digest("hex");
}

test("pinned Codex App Server completes the initialize handshake", { skip: !enabled }, async (t) => {
  const transport = StdioJsonRpcTransport.spawn();
  t.after(() => transport.close());
  const response = await transport.request("initialize", {
    clientInfo: {
      name: "work-engine-integration-test",
      title: "Work Engine Integration Test",
      version: "0.1.0",
    },
    capabilities: { experimentalApi: false, requestAttestation: false },
  });
  assert.equal(assertCompatibleServer(response), PINNED_PROTOCOL.codexCliVersion);
  transport.notify("initialized");
  assert.equal(response.platformFamily.length > 0, true);
});

test("live manifest role can read request-bound context", {
  skip: !enabled,
  timeout: 120_000,
}, async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-live-manifest-role."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const skillRoot = path.join(directory, "skills");
  const skillDirectory = path.join(skillRoot, "context-echo");
  await mkdir(skillDirectory, { recursive: true });
  await writeFile(path.join(skillDirectory, "SKILL.md"), `---
name: context-echo
description: Prove that request-bound context is visible in a live model turn.
---

# Context Echo

Read the request-context entry named \`work-engine.probe\`. Return exactly its
value and no other text.
`, "utf8");
  const manifestPath = path.join(directory, "runtime-manifest.yaml");
  await writeFile(manifestPath, `
schema_version: 1
manifest_id: work-engine.live-context-probe
roles:
  context-echo:
    contract: skills/context-echo/SKILL.md
    developer_instructions: |-
      This is a bounded visibility probe. Read the request-context entry named
      work-engine.probe and return exactly its value with no other text.
    thread_options:
      cwd: ${JSON.stringify(ROOT)}
      approval_policy: never
      sandbox: read-only
    skills:
      - {name: context-echo, path: skills/context-echo/SKILL.md}
`, "utf8");

  const transport = StdioJsonRpcTransport.spawn({ cwd: ROOT });
  t.after(() => transport.close());
  const adapter = new CodexAppServerAdapter({
    transport,
    registry: new FileRoleBindingRegistry(path.join(directory, "bindings.json")),
    skillResolver: await ExactSkillResolver.create([skillRoot]),
  });
  await adapter.initialize();
  const runtime = new ManifestRoleRuntime({
    adapter,
    manifest: await loadRuntimeManifest(manifestPath),
  });
  const probeValue = `visible-${randomUUID()}`;
  const delivery = await runtime.deliverTurn({
    roleId: "context-echo",
    instanceId: "live-probe",
    clientUserMessageId: `context-probe-${randomUUID()}`,
    text: "Return the request-bound probe value according to your exact role contract.",
    requestContext: {
      "work-engine.probe": { kind: "application", value: probeValue },
    },
  });
  const completion = await adapter.waitForTurnCompletion({
    ...delivery,
    signal: AbortSignal.timeout(90_000),
  });

  assert.equal(delivery.roleProjection.roleId, "context-echo");
  assert.equal(completion.status, "completed");
  assert.equal(completion.outputText?.trim(), probeValue);
});

test("live strategic planner returns an exact request-bound handoff", {
  skip: !enabled,
  timeout: 300_000,
}, async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-live-planner."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const transport = StdioJsonRpcTransport.spawn({ cwd: ROOT });
  t.after(() => transport.close());
  const adapter = new CodexAppServerAdapter({
    transport,
    registry: new FileRoleBindingRegistry(path.join(directory, "bindings.json")),
    skillResolver: await ExactSkillResolver.create([path.join(ROOT, "skills")]),
  });
  await adapter.initialize();
  const planner = new StrategicPlannerRuntime({
    adapter,
    manifest: await loadRuntimeManifest(path.join(ROOT, "app-server/runtime-manifest.yaml")),
  });
  const objective = "Prove one live request-bound strategic-planner handoff.";
  const roadmapDigest = await sha256(path.join(ROOT, "roadmap.md"));
  const lifecyclePath = path.join(
    ROOT,
    "app-server/docs/semantic-context-lifecycle-manager.md",
  );
  const lifecycleDigest = await sha256(lifecyclePath);
  const repositoryRevision = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: ROOT,
    encoding: "utf8",
  }).trim();
  const result = await planner.requestReview({
    instanceId: "live-integration",
    clientUserMessageId: `live-planner-${randomUUID()}`,
    strategicObjective: objective,
    evidenceCutoff: {
      roadmapRevision: `sha256:${roadmapDigest}`,
      repositoryRevision: `git:${repositoryRevision}`,
    },
    canonicalReferences: [{
      owner: "app-server-foundation-plan",
      reference: "app-server/docs/semantic-context-lifecycle-manager.md",
      revision: `sha256:${lifecycleDigest}`,
      integritySha256: lifecycleDigest,
      freshnessRule: "exact_content_sha256",
    }],
    continuity: "initialized",
    signal: AbortSignal.timeout(270_000),
  });

  assert.equal(result.logicalRoleInstanceId, "strategic-planner:live-integration");
  assert.equal(result.completion.status, "completed");
  assert.equal(result.handoff.schema_version, 1);
  assert.equal(result.handoff.strategic_objective, objective);
  assert.equal(result.handoff.continuity, "initialized");
  assert.equal(
    result.handoff.evidence_cutoff.repository_revision,
    `git:${repositoryRevision}`,
  );
});
