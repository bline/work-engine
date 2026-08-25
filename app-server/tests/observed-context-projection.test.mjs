import assert from "node:assert/strict";
import { createHash, generateKeyPairSync } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  ContextLifecycleEvidenceCollector,
  FileRoleBindingRegistry,
  loadRuntimeManifest,
  normalizeCodexLifecycleNotification,
  projectObservedContext,
  verifyObservedContextProjection,
} from "../src/index.mjs";

const sha = (digit) => digit.repeat(64);
const fileSha256 = async (filePath) => createHash("sha256")
  .update(await readFile(filePath))
  .digest("hex");

function fixture() {
  const collector = new ContextLifecycleEvidenceCollector();
  collector.record(normalizeCodexLifecycleNotification({
    method: "thread/tokenUsage/updated",
    params: {
      threadId: "thread-1",
      turnId: "turn-1",
      tokenUsage: {
        last: { inputTokens: 1, cachedInputTokens: 0, outputTokens: 1, reasoningOutputTokens: 0, totalTokens: 2 },
        total: { inputTokens: 2, cachedInputTokens: 0, outputTokens: 1, reasoningOutputTokens: 0, totalTokens: 3 },
        modelContextWindow: 100_000,
      },
    },
  }));
  return {
    logicalRoleInstanceId: "strategic-planner:main",
    runtimeBinding: {
      threadId: "thread-1",
      bindingRevision: 4,
      environmentRevision: "runtime-environment-1",
    },
    lastCompletedTurnId: "turn-1",
    visibleItems: [{
      identity: "request-context:objective",
      origin: "application",
      trustClass: "trusted_application_data",
      instructionApplicability: "none",
      contentRef: { kind: "request-context", reference: "objective", sha256: sha("a") },
    }, {
      identity: "thread-item:web-result",
      origin: "retrieved_content",
      trustClass: "untrusted_data",
      instructionApplicability: "none",
      contentRef: { kind: "thread-item", reference: "web-result", sha256: sha("b") },
      producer: "web-search",
    }],
    governingSources: [{
      identity: "role-contract",
      owner: "skills/strategic-planner/SKILL.md",
      contentRef: { kind: "skill", reference: "strategic-planner", sha256: sha("c") },
    }],
    activatedSkills: [{ name: "strategic-planner", path: "skills/strategic-planner/SKILL.md", sha256: sha("c") }],
    lifecycleSnapshot: collector.snapshot("thread-1"),
    expectedNextWork: { reference: "roadmap-review", sha256: sha("d") },
    sourceInventoryCompleteness: "partial",
    omissions: [{ scope: "provider-effective-prompt", reason: "provider does not expose exact prompt" }],
  };
}

test("host signs a deterministic attributed observed-context projection", () => {
  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  const options = {
    componentId: "work-engine.app-server.observed-context-projector",
    buildRevision: "git:foundation",
    keyId: "host-key-1",
    privateKey,
  };
  const first = projectObservedContext(fixture(), options);
  const second = projectObservedContext(fixture(), options);

  assert.deepEqual(first, second);
  assert.equal(first.sourceRevision.startsWith("sha256:"), true);
  assert.deepEqual(first.observedContext.unknowns, [
    "exact_effective_model_input",
    "hidden_reasoning_state",
    "provider_internal_instructions",
  ]);
  assert.equal(first.observedContext.visibleItems[1].trustClass, "untrusted_data");
  assert.equal(Object.isFrozen(first.observedContext), true);
  assert.equal(verifyObservedContextProjection(first, {
    resolvePublicKey: (keyId) => keyId === "host-key-1" ? publicKey : null,
  }), true);
});

test("verification rejects tampering, another key, and a mismatched lifecycle thread", () => {
  const signer = generateKeyPairSync("ed25519");
  const other = generateKeyPairSync("ed25519");
  const projection = projectObservedContext(fixture(), {
    componentId: "projector",
    buildRevision: "build-1",
    keyId: "key-1",
    privateKey: signer.privateKey,
  });
  const tampered = structuredClone(projection);
  tampered.observedContext.visibleItems[0].trustClass = "governing_instruction";

  assert.equal(verifyObservedContextProjection(tampered, {
    resolvePublicKey: () => signer.publicKey,
  }), false);
  assert.equal(verifyObservedContextProjection(projection, {
    resolvePublicKey: () => other.publicKey,
  }), false);

  const mismatch = fixture();
  mismatch.runtimeBinding.threadId = "thread-other";
  assert.throws(() => projectObservedContext(mismatch, {
    componentId: "projector",
    buildRevision: "build-1",
    keyId: "key-1",
    privateKey: signer.privateKey,
  }), /does not match runtime binding/);
});

test("projection rejects unclassified trust and undeclared source fields", () => {
  const { privateKey } = generateKeyPairSync("ed25519");
  const source = fixture();
  source.visibleItems[0].trustClass = "self_declared_authority";
  assert.throws(() => projectObservedContext(source, {
    componentId: "projector",
    buildRevision: "build-1",
    keyId: "key-1",
    privateKey,
  }), /trustClass is unsupported/);

  const extra = fixture();
  extra.visibleItems[0].authority = "invented";
  assert.throws(() => projectObservedContext(extra, {
    componentId: "projector",
    buildRevision: "build-1",
    keyId: "key-1",
    privateKey,
  }), /unsupported fields: authority/);

  const injected = fixture();
  injected.visibleItems[1].instructionApplicability = "governing";
  assert.throws(() => projectObservedContext(injected, {
    componentId: "projector",
    buildRevision: "build-1",
    keyId: "key-1",
    privateKey,
  }), /cannot govern instructions/);

  const incomplete = fixture();
  incomplete.omissions = [];
  assert.throws(() => projectObservedContext(incomplete, {
    componentId: "projector",
    buildRevision: "build-1",
    keyId: "key-1",
    privateKey,
  }), /requires an explicit omission/);
});

test("projection binds a real manifest role, binding revision, and lifecycle snapshot", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-observed-context."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const manifest = await loadRuntimeManifest(fileURLToPath(
    new URL("../runtime-manifest.yaml", import.meta.url),
  ));
  const role = manifest.projectRole("strategic-planner", "projection-proof");
  const registry = new FileRoleBindingRegistry(path.join(directory, "bindings.json"));
  const binding = await registry.bind({
    logicalRoleInstanceId: role.role.logicalRoleInstanceId,
    threadId: "thread-projection",
    protocolVersion: "0.149.1",
    environmentFingerprint: role.role.runtimeEnvironmentRevision,
  });
  const collector = new ContextLifecycleEvidenceCollector();
  const source = fixture();
  source.logicalRoleInstanceId = role.role.logicalRoleInstanceId;
  source.runtimeBinding = {
    threadId: binding.threadId,
    bindingRevision: binding.bindingRevision,
    environmentRevision: role.role.runtimeEnvironmentRevision,
  };
  source.lifecycleSnapshot = collector.snapshot(binding.threadId);
  source.activatedSkills = await Promise.all(role.skills.map(async (skill) => ({
    name: skill.name,
    path: skill.path,
    sha256: await fileSha256(skill.path),
  })));
  source.governingSources = [{
    identity: "role-contract",
    owner: role.roleContract.path,
    contentRef: {
      kind: "skill",
      reference: role.roleContract.path,
      sha256: await fileSha256(role.roleContract.path),
    },
  }];
  const keys = generateKeyPairSync("ed25519");
  const projection = projectObservedContext(source, {
    componentId: "work-engine.app-server.observed-context-projector",
    buildRevision: "test-build",
    keyId: "test-key",
    privateKey: keys.privateKey,
  });

  assert.equal(projection.observedContext.logicalRoleInstanceId, "strategic-planner:projection-proof");
  assert.equal(projection.observedContext.runtimeBinding.bindingRevision, 1);
  assert.equal(projection.observedContext.activatedSkills[0].sha256.length, 64);
  assert.equal(verifyObservedContextProjection(projection, {
    resolvePublicKey: () => keys.publicKey,
  }), true);
});
