import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
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
  satisfyRuntimeRequirements,
} from "../src/index.mjs";
import { compileSkill } from "../src/skill-compiler.mjs";

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

test("compiled slice-builder requirements admit one generic manifest role and reject excess before delivery", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const compiled = await compileSkill({
    structureSource: await readFile(path.join(root, "app-server/migrations/skills/slice-builder/structure.yaml"), "utf8"),
    interfaceSource: await readFile(path.join(root, "app-server/migrations/skills/slice-builder/interface.yaml"), "utf8"),
    workspaceRoot: root,
  });
  const manifestPath = path.join(root, "app-server/runtime-manifest.yaml");
  const manifest = await loadRuntimeManifest(manifestPath);
  const receipt = satisfyRuntimeRequirements({
    manifest,
    roleId: "slice-builder",
    requirements: compiled.ir.runtime_requirements,
  });
  assert.equal(receipt.requirements_sha256, compiled.ir.runtime_requirements.sha256);
  assert.equal(receipt.compiled_skill_sha256, compiled.ir.output_sha256);
  assert.match(receipt.sha256, /^[0-9a-f]{64}$/);

  let deliveries = 0;
  const runtime = new ManifestRoleRuntime({
    adapter: { deliverTurn: async () => { deliveries += 1; return { turnId: "turn-1" }; } },
    manifest,
    runtimeRequirements: { "slice-builder": compiled.ir.runtime_requirements },
  });
  const delivered = await runtime.deliverTurn({ roleId: "slice-builder", instanceId: "vertical", text: "prove" });
  assert.equal(deliveries, 1);
  assert.equal(delivered.runtimeSatisfaction.sha256, receipt.sha256);
  assert.equal(delivered.skillRuntimeSatisfactions["repo-search"].skill_id, "repo-search");
  assert.equal(delivered.roleProjection.role.continuity, "retained");

  const loaded = await loadRuntimeManifestDocument(manifestPath);
  const excessive = structuredClone(loaded.document);
  excessive.roles["slice-builder"].effects.push("state.user_history");
  const excessiveManifest = projectRuntimeManifest(excessive, {
    baseDirectory: path.dirname(loaded.sourcePath),
    sourcePath: loaded.sourcePath,
    runtimeRequirementsByRole: { "slice-builder": compiled.ir.runtime_requirements },
  });
  const refused = new ManifestRoleRuntime({
    adapter: { deliverTurn: async () => { deliveries += 1; return {}; } },
    manifest: excessiveManifest,
    runtimeRequirements: { "slice-builder": compiled.ir.runtime_requirements },
  });
  await assert.rejects(
    refused.deliverTurn({ roleId: "slice-builder", instanceId: "excess", text: "must refuse" }),
    /grants prohibited effect state\.user_history/,
  );
  assert.equal(deliveries, 1);

  const refusalCases = [
    ["missing-capability", (role) => role.capabilities.splice(role.capabilities.indexOf("capability.repository_evidence"), 1), /missing required capability capability\.repository_evidence/],
    ["continuity", (role) => { role.continuity = "ephemeral"; }, /incompatible continuity/],
    ["fingerprint", (role) => { role.compiled_skill_sha256 = "0".repeat(64); }, /compiled skill fingerprint differs/],
  ];
  for (const [name, mutate, pattern] of refusalCases) {
    const invalid = structuredClone(loaded.document);
    mutate(invalid.roles["slice-builder"]);
    const invalidManifest = projectRuntimeManifest(invalid, {
      baseDirectory: path.dirname(loaded.sourcePath),
      runtimeRequirementsByRole: { "slice-builder": compiled.ir.runtime_requirements },
    });
    const invalidRuntime = new ManifestRoleRuntime({
      adapter: { deliverTurn: async () => { deliveries += 1; return {}; } },
      manifest: invalidManifest,
      runtimeRequirements: { "slice-builder": compiled.ir.runtime_requirements },
    });
    await assert.rejects(
      invalidRuntime.deliverTurn({ roleId: "slice-builder", instanceId: name, text: "must refuse" }),
      pattern,
    );
  }
  assert.equal(deliveries, 1);

  const excessCapability = structuredClone(loaded.document);
  excessCapability.roles["slice-builder"].capabilities.push("capability.unapproved");
  const excessCapabilityManifest = projectRuntimeManifest(excessCapability, {
    baseDirectory: path.dirname(loaded.sourcePath),
    runtimeRequirementsByRole: { "slice-builder": compiled.ir.runtime_requirements },
  });
  await assert.rejects(
    new ManifestRoleRuntime({ adapter: { deliverTurn: async () => { deliveries += 1; } }, manifest: excessCapabilityManifest })
      .deliverTurn({ roleId: "slice-builder", instanceId: "extra-cap", text: "must refuse" }),
    /exceeds capability ceiling with capability\.unapproved/,
  );
  assert.equal(deliveries, 1);
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

test("repo-search is admitted as an exact role-free secondary skill", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const manifest = await loadRuntimeManifest(path.join(root, "app-server/runtime-manifest.yaml"));
  const projection = manifest.projectRole("slice-builder", "repo-search-vertical");
  const skill = projection.skills.find(({ name }) => name === "repo-search");

  assert.ok(skill);
  assert.equal(manifest.roleIds.includes("repo-search"), false);
  assert.equal(skill.path, path.join(root, "skills/repo-search/SKILL.md"));
  assert.equal(skill.compiledSkillSha256, "cf72505d1850e757f6cc6ef3331d0f66ee4c5bf51b52a527c27fe8b78c17a949");
  assert.deepEqual(skill.capabilities, [
    "capability.direct_source_observation",
    "capability.repository_evidence",
  ]);
  assert.deepEqual(skill.effects, []);
  assert.equal("role_id" in skill.runtimeRequirements, false);
  assert.equal(skill.runtimeRequirements.contract.kind, "skill");

  const receipt = satisfyRuntimeRequirements({
    manifest,
    roleId: "slice-builder",
    skillName: "repo-search",
    requirements: skill.runtimeRequirements,
  });
  assert.equal(receipt.skill_id, "repo-search");
  assert.equal(receipt.compiled_skill_sha256, skill.compiledSkillSha256);
});

test("repo-search admission is independent of the ambient working directory", async (t) => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const manifest = await loadRuntimeManifest(path.join(root, "app-server/runtime-manifest.yaml"));
  const requirements = manifest.roles["slice-builder"].skills
    .find(({ name }) => name === "repo-search").runtimeRequirements;
  const temporaryCwd = await mkdtemp(path.join(os.tmpdir(), "work-engine-manifest-cwd."));
  t.after(() => rm(temporaryCwd, { recursive: true, force: true }));
  const originalCwd = process.cwd();

  try {
    process.chdir(temporaryCwd);
    const receipt = satisfyRuntimeRequirements({
      manifest,
      roleId: "slice-builder",
      skillName: "repo-search",
      requirements,
    });
    assert.equal(receipt.skill_id, "repo-search");
    assert.equal(receipt.compiled_skill_sha256, requirements.compiled_skill_sha256);
  } finally {
    process.chdir(originalCwd);
  }
});

test("repo-search secondary admission refuses missing, excess, effect, path, and fingerprint grants", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const manifestPath = path.join(root, "app-server/runtime-manifest.yaml");
  const manifest = await loadRuntimeManifest(manifestPath);
  const requirements = manifest.roles["slice-builder"].skills
    .find(({ name }) => name === "repo-search").runtimeRequirements;
  const loaded = await loadRuntimeManifestDocument(manifestPath);

  const cases = [
    ["missing", (skill) => skill.capabilities.splice(skill.capabilities.indexOf("capability.repository_evidence"), 1), /missing required capability capability\.repository_evidence/],
    ["excess", (skill) => skill.capabilities.push("capability.independent_review"), /exceeds capability ceiling with capability\.independent_review/],
    ["effect", (skill) => skill.effects.push("state.agent_worktree"), /exceeds effect ceiling with state\.agent_worktree/],
    ["path", (skill) => { skill.path = "../skills/strategic-planner/SKILL.md"; }, /path differs from requirements/],
    ["fingerprint", (skill) => { skill.compiled_skill_sha256 = "0".repeat(64); }, /compiled skill fingerprint differs/],
  ];
  for (const [name, mutate, expected] of cases) {
    const document = structuredClone(loaded.document);
    mutate(document.roles["slice-builder"].skills.find(({ name: skillName }) => skillName === "repo-search"));
    const projected = projectRuntimeManifest(document, {
      baseDirectory: path.dirname(loaded.sourcePath),
      identityBaseDirectory: root,
      runtimeRequirementsByRole: { "slice-builder:repo-search": requirements },
    });
    assert.throws(
      () => satisfyRuntimeRequirements({ manifest: projected, roleId: "slice-builder", skillName: "repo-search", requirements }),
      expected,
      name,
    );
  }
});
