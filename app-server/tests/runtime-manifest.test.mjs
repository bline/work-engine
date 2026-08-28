import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  CodexAppServerAdapter,
  DynamicToolBridge,
  ExactSkillResolver,
  FileRoleBindingRegistry,
  ManifestRoleRuntime,
  loadRuntimeManifest,
  loadRuntimeManifestDocument,
  projectRuntimeManifest,
} from "../src/index.mjs";

class ManifestTransport {
  constructor() {
    this.requests = [];
    this.thread = 0;
    this.turn = 0;
  }

  onServerRequest() {}
  onNotification() { return () => {}; }
  notify() {}

  async request(method, params) {
    this.requests.push({ method, params });
    if (method === "initialize") {
      return {
        userAgent: "work-engine/0.149.1 (Linux; x86_64)",
        codexHome: "/tmp/codex",
        platformFamily: "unix",
        platformOs: "linux",
      };
    }
    if (method === "thread/start") {
      this.thread += 1;
      return { thread: { id: `manifest-thread-${this.thread}` } };
    }
    if (method === "turn/start") {
      this.turn += 1;
      return { turn: { id: `manifest-turn-${this.turn}` } };
    }
    throw new Error(`unexpected request ${method}`);
  }
}

async function fixture(t) {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-manifest."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const skillRoot = path.join(directory, "skills");
  for (const name of ["alpha", "shared"]) {
    await mkdir(path.join(skillRoot, name), { recursive: true });
    await writeFile(path.join(skillRoot, name, "SKILL.md"), `# ${name}\n`, "utf8");
  }
  const manifestPath = path.join(directory, "runtime.yaml");
  await writeFile(manifestPath, `
schema_version: 1
manifest_id: test.runtime
roles:
  alpha:
    contract: skills/alpha/SKILL.md
    developer_instructions: Keep alpha bounded.
    thread_options:
      cwd: .
      approval_policy: never
      sandbox: read-only
    skills:
      - {name: alpha, path: skills/alpha/SKILL.md}
      - {name: shared, path: skills/shared/SKILL.md}
  beta:
    contract: skills/shared/SKILL.md
    developer_instructions: Keep beta isolated.
    thread_options:
      cwd: .
      approval_policy: on-request
      sandbox: workspace-write
    capabilities:
      - fixture.lookup
    skills:
      - {name: shared, path: skills/shared/SKILL.md}
`, "utf8");
  return { directory, skillRoot, manifestPath };
}

test("runtime manifest projects arbitrary role instances and exact skills", async (t) => {
  const { directory, skillRoot, manifestPath } = await fixture(t);
  const manifest = await loadRuntimeManifest(manifestPath);
  assert.deepEqual(manifest.roleIds, ["alpha", "beta"]);
  const first = manifest.projectRole("alpha", "one");
  const second = manifest.projectRole("alpha", "two");
  assert.equal(first.role.logicalRoleInstanceId, "alpha:one");
  assert.deepEqual(first.role.capabilities, []);
  assert.deepEqual(manifest.projectRole("beta", "one").role.capabilities, ["fixture.lookup"]);
  assert.equal(second.role.logicalRoleInstanceId, "alpha:two");
  assert.equal(first.role.threadOptions.cwd, directory);
  assert.match(first.role.runtimeEnvironmentRevision, /^[a-f0-9]{64}$/);
  assert.deepEqual(first.skills.map(({ name }) => name), ["alpha", "shared"]);
  assert.equal(first.roleContract.path, path.join(skillRoot, "alpha/SKILL.md"));
  assert.match(first.manifest.sha256, /^[a-f0-9]{64}$/);
  assert.equal(first.manifest.path, manifestPath);
  assert.equal(Object.isFrozen(first), true);
});

test("snapshot delivery paths do not change canonical role environment identity", async (t) => {
  const { directory, manifestPath } = await fixture(t);
  const loaded = await loadRuntimeManifestDocument(manifestPath);
  const first = projectRuntimeManifest(loaded.document, {
    baseDirectory: path.join(directory, "generation-one"),
    identityBaseDirectory: directory,
    sourceSha256: loaded.sourceSha256,
  }).projectRole("alpha", "main");
  const second = projectRuntimeManifest(loaded.document, {
    baseDirectory: path.join(directory, "generation-two"),
    identityBaseDirectory: directory,
    sourceSha256: loaded.sourceSha256,
  }).projectRole("alpha", "main");

  assert.equal(first.role.runtimeEnvironmentRevision, second.role.runtimeEnvironmentRevision);
  assert.notEqual(first.skills[0].path, second.skills[0].path);
});

test("manifest role runtime delivers projected roles through the shared adapter", async (t) => {
  const { directory, skillRoot, manifestPath } = await fixture(t);
  const transport = new ManifestTransport();
  const adapter = new CodexAppServerAdapter({
    transport,
    registry: new FileRoleBindingRegistry(path.join(directory, "bindings.json")),
    skillResolver: await ExactSkillResolver.create([skillRoot]),
    roleToolBridgeResolver: (capabilities) => capabilities.includes("fixture.lookup")
      ? new DynamicToolBridge([{
        namespace: "fixture",
        name: "lookup",
        description: "Return one bounded fixture value.",
        inputSchema: { type: "object", properties: {}, additionalProperties: false },
        handler: () => "fixture-value",
      }])
      : null,
  });
  await adapter.initialize();
  const runtime = new ManifestRoleRuntime({
    adapter,
    manifest: await loadRuntimeManifest(manifestPath),
  });

  const delivery = await runtime.deliverTurn({
    roleId: "beta",
    instanceId: "probe",
    clientUserMessageId: "probe-1",
    text: "Exercise this role.",
    requestContext: {
      "work-engine.probe": { kind: "application", value: "bound-value" },
    },
  });
  assert.equal(delivery.logicalRoleInstanceId, "beta:probe");
  assert.equal(delivery.roleProjection.manifest.manifestId, "test.runtime");
  const start = transport.requests.find(({ method }) => method === "thread/start");
  assert.equal(start.params.developerInstructions, "Keep beta isolated.");
  assert.equal(start.params.approvalPolicy, "on-request");
  assert.deepEqual(start.params.dynamicTools.map(({ name }) => name), ["fixture"]);
  const turn = transport.requests.find(({ method }) => method === "turn/start");
  assert.equal(turn.params.input[0].type, "skill");
  assert.equal(turn.params.input[0].name, "shared");
  assert.equal(turn.params.input[1].text.startsWith("WORK_ENGINE_REQUEST_CONTEXT_V1\n"), true);
  assert.equal(turn.params.input[2].text, "Exercise this role.");

  const changedManifestPath = path.join(directory, "changed-runtime.yaml");
  await writeFile(changedManifestPath, `
schema_version: 1
manifest_id: test.runtime.changed
roles:
  beta:
    contract: skills/shared/SKILL.md
    developer_instructions: Beta now has a different runtime contract projection.
    thread_options:
      cwd: .
      approval_policy: on-request
      sandbox: workspace-write
    skills:
      - {name: shared, path: skills/shared/SKILL.md}
`, "utf8");
  const changedRuntime = new ManifestRoleRuntime({
    adapter,
    manifest: await loadRuntimeManifest(changedManifestPath),
  });
  await assert.rejects(
    changedRuntime.deliverTurn({
      roleId: "beta",
      instanceId: "probe",
      clientUserMessageId: "probe-2",
      text: "Do not reuse an incompatible role thread.",
    }),
    /runtime environment changed; replace the runtime binding/,
  );
});

test("runtime manifest rejects ambiguous or unsupported composition", async (t) => {
  const { directory } = await fixture(t);
  const invalidPath = path.join(directory, "invalid.yaml");
  await writeFile(invalidPath, `
schema_version: 1
manifest_id: invalid.runtime
roles:
  alpha:
    contract: missing/SKILL.md
    developer_instructions: Alpha.
    thread_options: {cwd: ., ephemeral: true}
    skills:
      - {name: missing, path: missing/SKILL.md}
`, "utf8");
  await assert.rejects(loadRuntimeManifest(invalidPath), /unsupported fields: ephemeral/);

  const duplicatePath = path.join(directory, "duplicate.yaml");
  await writeFile(duplicatePath, `
schema_version: 1
manifest_id: duplicate.runtime
roles:
  alpha:
    contract: first/SKILL.md
    developer_instructions: Alpha.
    skills:
      - {name: repeated, path: first/SKILL.md}
      - {name: repeated, path: second/SKILL.md}
`, "utf8");
  await assert.rejects(loadRuntimeManifest(duplicatePath), /duplicate skill name repeated/);

  const unsupportedTurnOptionPath = path.join(directory, "unsupported-turn-option.yaml");
  await writeFile(unsupportedTurnOptionPath, `
schema_version: 1
manifest_id: unsupported.turn.option
roles:
  alpha:
    contract: skills/alpha/SKILL.md
    developer_instructions: Alpha.
    thread_options: {effort: high}
    skills:
      - {name: alpha, path: skills/alpha/SKILL.md}
`, "utf8");
  await assert.rejects(
    loadRuntimeManifest(unsupportedTurnOptionPath),
    /unsupported fields: effort/,
  );
});
