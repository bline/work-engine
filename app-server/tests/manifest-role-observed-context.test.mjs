import assert from "node:assert/strict";
import { generateKeyPairSync } from "node:crypto";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  projectManifestRoleObservedContext,
  projectRuntimeManifest,
  verifyObservedContextProjection,
} from "../src/index.mjs";

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
        developer_instructions: "Run the fixture.",
        skills: [{ name: "probe", path: "skills/probe/SKILL.md" }],
      },
    },
  }, { baseDirectory: directory }).projectRole("probe", "dev");
  const keys = generateKeyPairSync("ed25519");
  const result = await projectManifestRoleObservedContext({
    delivery: {
      logicalRoleInstanceId: "probe:dev",
      threadId: "thread-1",
      turnId: "turn-1",
      binding: { bindingRevision: 2 },
      roleProjection,
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
  assert.equal(result.sourceMaterials.length, 3);
  assert.equal(result.sourceMaterials[0].contentRef.sha256.length, 64);
  assert.equal(result.sourceMaterials[1].content, "# Probe fixture\n");
  await assert.rejects(
    projectManifestRoleObservedContext({
      delivery: {
        logicalRoleInstanceId: "probe:dev",
        threadId: "thread-1",
        turnId: "turn-1",
        binding: { bindingRevision: 2 },
        roleProjection,
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
});
