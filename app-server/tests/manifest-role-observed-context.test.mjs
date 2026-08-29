import assert from "node:assert/strict";
import { createHash, generateKeyPairSync } from "node:crypto";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  projectManifestRoleObservedContext,
  projectRuntimeManifest,
  verifyObservedContextProjection,
} from "../src/index.mjs";

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonical(value[key])}`
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}

test("manifest-role projection binds classified materials, skill bytes, and runtime identity", async (t) => {
  const directory = await mkdtemp(path.join(tmpdir(), "manifest-observed-context-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const skillDirectory = path.join(directory, "skills", "probe");
  await mkdir(skillDirectory, { recursive: true });
  const skillPath = path.join(skillDirectory, "SKILL.md");
  await writeFile(skillPath, "# Probe fixture\n", "utf8");
  const roleProjection = projectRuntimeManifest({
    schema_version: 1,
    manifest_id: "observed.fixture",
    roles: {
      probe: {
        contract: "skills/probe/SKILL.md",
        developer_instructions: "Run the fixture because substituted inputs invalidate continuation.",
        compiled_skill_sha256: "a".repeat(64),
        skills: [{ name: "probe", path: "skills/probe/SKILL.md" }],
      },
    },
  }, { baseDirectory: directory }).projectRole("probe", "dev");
  const satisfactionBody = {
    schema_version: 1,
    role_id: "probe",
    requirements_sha256: "b".repeat(64),
    compiled_skill_sha256: "a".repeat(64),
    manifest_sha256: roleProjection.manifest.sha256,
    runtime_environment_revision: roleProjection.role.runtimeEnvironmentRevision,
  };
  const runtimeSatisfaction = {
    ...satisfactionBody,
    sha256: createHash("sha256").update(canonical(satisfactionBody)).digest("hex"),
  };
  const keys = generateKeyPairSync("ed25519");
  const result = await projectManifestRoleObservedContext({
    delivery: {
      logicalRoleInstanceId: "probe:dev",
      threadId: "thread-1",
      turnId: "turn-1",
      binding: { bindingRevision: 2 },
      roleProjection,
      runtimeSatisfaction,
    },
    lifecycleSnapshot: { schemaVersion: 1, threadId: "thread-1", retainedObservationCount: 1 },
    visibleMaterials: [{
      identity: "user-message:one",
      origin: "human",
      trustClass: "human_authority_input",
      instructionApplicability: "contract_defined",
      contentRef: { kind: "thread-item", reference: "user-message:one" },
      content: "Proceed with the bounded fixture.",
    }],
    expectedNextWork: { reference: "next:one", content: "Continue the fixture." },
    sourceInventoryCompleteness: "partial",
    omissions: [{ scope: "provider-effective-prompt", reason: "not exposed" }],
    signing: {
      componentId: "manifest-observed-context-test",
      buildRevision: "test-1",
      keyId: "key-1",
      privateKey: keys.privateKey,
    },
  });
  assert.equal(verifyObservedContextProjection(result.projection, {
    resolvePublicKey: (keyId) => keyId === "key-1" ? keys.publicKey : null,
  }), true);
  assert.equal(result.projection.observedContext.runtimeBinding.bindingRevision, 2);
  assert.equal(result.projection.observedContext.governingSources[0].owner, skillPath);
  assert.match(
    result.projection.observedContext.governingSources[0].contentRef.reference,
    /^work-engine\.runtime-role-environment\/v1\//,
  );
  assert.equal(result.sourceMaterials.length, 4);
  assert.equal(result.sourceMaterials[0].contentRef.sha256.length, 64);
  assert.equal(result.sourceMaterials[1].content, "# Probe fixture\n");
  assert.match(result.sourceMaterials[2].content, /substituted inputs invalidate continuation/);
  assert.match(result.sourceMaterials[2].content, new RegExp(runtimeSatisfaction.sha256));
  await assert.rejects(
    projectManifestRoleObservedContext({
      delivery: {
        logicalRoleInstanceId: "probe:substituted",
        threadId: "thread-1",
        turnId: "turn-1",
        binding: { bindingRevision: 2 },
        roleProjection,
        runtimeSatisfaction,
      },
      lifecycleSnapshot: { schemaVersion: 1, threadId: "thread-1", retainedObservationCount: 1 },
      visibleMaterials: [],
      expectedNextWork: { reference: "next:one", content: "Continue the fixture." },
      sourceInventoryCompleteness: "complete",
      signing: {
        componentId: "manifest-observed-context-test",
        buildRevision: "test-1",
        keyId: "key-1",
        privateKey: keys.privateKey,
      },
    }),
    /delivery does not match projected logical identity/,
  );
  await assert.rejects(
    projectManifestRoleObservedContext({
      delivery: {
        logicalRoleInstanceId: "probe:dev",
        threadId: "thread-1",
        turnId: "turn-1",
        binding: { bindingRevision: 2 },
        roleProjection,
        runtimeSatisfaction,
      },
      lifecycleSnapshot: { schemaVersion: 1, threadId: "thread-1", retainedObservationCount: 1 },
      visibleMaterials: [{
        identity: "user-message:one",
        origin: "human",
        trustClass: "human_authority_input",
        instructionApplicability: "contract_defined",
        contentRef: { kind: "thread-item", reference: "user-message:one", sha256: "caller-owned" },
        content: "Proceed with the bounded fixture.",
      }],
      expectedNextWork: { reference: "next:one", content: "Continue the fixture." },
      sourceInventoryCompleteness: "partial",
      omissions: [{ scope: "provider-effective-prompt", reason: "not exposed" }],
      signing: {
        componentId: "manifest-observed-context-test",
        buildRevision: "test-1",
        keyId: "key-1",
        privateKey: keys.privateKey,
      },
    }),
    /unsupported fields: sha256/,
  );
  await assert.rejects(
    projectManifestRoleObservedContext({
      delivery: {
        logicalRoleInstanceId: "probe:dev",
        threadId: "thread-1",
        turnId: "turn-1",
        binding: { bindingRevision: 2 },
        roleProjection,
        runtimeSatisfaction: { ...runtimeSatisfaction, requirements_sha256: "c".repeat(64) },
      },
      lifecycleSnapshot: { schemaVersion: 1, threadId: "thread-1", retainedObservationCount: 1 },
      visibleMaterials: [],
      expectedNextWork: { reference: "next:one", content: "Continue the fixture." },
      sourceInventoryCompleteness: "complete",
      signing: {
        componentId: "manifest-observed-context-test",
        buildRevision: "test-1",
        keyId: "key-1",
        privateKey: keys.privateKey,
      },
    }),
    /runtime satisfaction does not match/,
  );
});

test("uncompiled manifest-role projection preserves legacy governing material and rejects satisfaction", async (t) => {
  const directory = await mkdtemp(path.join(tmpdir(), "manifest-observed-context-legacy-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const skillDirectory = path.join(directory, "skills", "probe");
  await mkdir(skillDirectory, { recursive: true });
  const skillPath = path.join(skillDirectory, "SKILL.md");
  await writeFile(skillPath, "# Legacy probe fixture\n", "utf8");
  const roleProjection = projectRuntimeManifest({
    schema_version: 1,
    manifest_id: "observed.legacy.fixture",
    roles: {
      probe: {
        contract: "skills/probe/SKILL.md",
        developer_instructions: "Run the legacy fixture.",
        skills: [{ name: "probe", path: "skills/probe/SKILL.md" }],
      },
    },
  }, { baseDirectory: directory }).projectRole("probe", "dev");
  assert.equal(roleProjection.role.compiledSkillSha256, null);
  const keys = generateKeyPairSync("ed25519");
  const input = {
    delivery: {
      logicalRoleInstanceId: "probe:dev",
      threadId: "thread-legacy",
      turnId: "turn-legacy",
      binding: { bindingRevision: 1 },
      roleProjection,
    },
    lifecycleSnapshot: {
      schemaVersion: 1,
      threadId: "thread-legacy",
      retainedObservationCount: 1,
    },
    visibleMaterials: [],
    expectedNextWork: { reference: "next:legacy", content: "Continue the legacy fixture." },
    sourceInventoryCompleteness: "complete",
    signing: {
      componentId: "manifest-observed-context-test",
      buildRevision: "test-legacy",
      keyId: "key-legacy",
      privateKey: keys.privateKey,
    },
  };
  const result = await projectManifestRoleObservedContext(input);
  assert.equal(
    result.projection.observedContext.governingSources[0].contentRef.reference,
    skillPath,
  );
  assert.equal(result.sourceMaterials.length, 2);
  assert.equal(result.sourceMaterials[0].content, "# Legacy probe fixture\n");

  await assert.rejects(projectManifestRoleObservedContext({
    ...input,
    delivery: {
      ...input.delivery,
      runtimeSatisfaction: { schema_version: 1, sha256: "a".repeat(64) },
    },
  }), /uncompiled manifest role cannot claim runtime satisfaction/);
});
