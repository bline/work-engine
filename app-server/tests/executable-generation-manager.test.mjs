import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  ExecutableGenerationAdmissionError,
  ExecutableGenerationConflictError,
  ExecutableGenerationManager,
  FileExecutableGenerationStore,
  InMemoryReplaceableSubstrateArbiter,
} from "../src/index.mjs";

function deferred() {
  let resolve;
  const promise = new Promise((settle) => { resolve = settle; });
  return { promise, resolve };
}

function generation(generationId, overrides = {}) {
  return {
    generationId,
    sourceDigest: `source-${generationId}`,
    environmentFingerprint: "environment-1",
    bootstrapFingerprint: "bootstrap-1",
    async validate() { return { valid: true }; },
    async activate() {},
    async dispose() {},
    ...overrides,
  };
}

async function managerFixture(t, { candidate = generation("g2"), arbiter = null } = {}) {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-generation-manager-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const store = new FileExecutableGenerationStore(path.join(root, "generations.json"));
  const substrateArbiter = arbiter ?? new InMemoryReplaceableSubstrateArbiter();
  const manager = await ExecutableGenerationManager.create({
    activeGeneration: generation("g1"),
    store,
    substrateArbiter,
    snapshotter: async () => ({ snapshotId: "snapshot-2", sourceDigest: "source-g2" }),
    candidateBuilder: async () => candidate,
    reloadIdFactory: () => "reload-1",
  });
  return { manager, store, substrateArbiter };
}

test("extension attachment shares generation lifecycle, store, admission, and terminal receipts", async (t) => {
  const attachment = {
    schema_version: 1, bundle_id: "fixture", sha256: "a".repeat(64),
    run: { retention: "retain" }, registry: [],
  };
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-extension-manager-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const store = new FileExecutableGenerationStore(path.join(root, "generations.json"));
  const manager = await ExecutableGenerationManager.create({
    activeGeneration: generation("g1"), store,
    substrateArbiter: new InMemoryReplaceableSubstrateArbiter(),
    snapshotter: async () => ({ snapshotId: "g2", sourceDigest: "source-g2",
      extensionAttachment: attachment, baseSourceDigest: "source-g1" }),
    candidateBuilder: async () => generation("g2"), reloadIdFactory: () => "reload-extension",
  });
  let staged;
  await manager.runAdmission({ kind: "turn", id: "trigger" }, async () => {
    staged = await manager.requestReload({ requestedByTurnId: "trigger", source: { extensionBundle: {} } });
  });
  assert.equal((await staged.completion).status, "active_unexercised");
  await manager.runAdmission({ kind: "turn", id: "extension-run" }, async () => {
    for (const state of ["executed", "artifact_sealed", "detached", "retained"]) {
      const receipt = await manager.recordExtensionTransition({ admissionId: "extension-run", state,
        details: state === "executed" ? { workload: "succeeded", transport: "succeeded" }
          : state === "artifact_sealed" ? { artifact: "sealed" } : {} });
      assert.equal(receipt.production_admission, false);
    }
  });
  const durable = await store.read();
  assert.equal(durable.activeGeneration.extensionAttachment.sha256, attachment.sha256);
  assert.equal(durable.reloads["reload-extension"].extension.state, "retained");
  await assert.rejects(manager.recordExtensionTransition({ admissionId: "missing", state: "cleaned" }),
    /active generation-bound admission/);
});

test("fences new admissions synchronously and swaps generations only after global quiescence", async (t) => {
  const events = [];
  const candidate = generation("g2", {
    async activate() { events.push("candidate_activated"); },
  });
  const { manager, store } = await managerFixture(t, { candidate });
  const turnRelease = deferred();
  const stagedReady = deferred();

  const predecessorTurn = manager.runAdmission({ kind: "turn", id: "turn-1" }, async (active) => {
    assert.equal(active.generationId, "g1");
    const reloadRequest = manager.requestReload({
      requestedByTurnId: "turn-1",
      source: { files: ["src/worker.mjs"] },
    });
    await assert.rejects(
      manager.runAdmission({ kind: "turn", id: "turn-racing" }, async () => {}),
      (error) => error instanceof ExecutableGenerationAdmissionError
        && error.code === "reload_fence_active",
    );
    const staged = await reloadRequest;
    stagedReady.resolve(staged);
    await turnRelease.promise;
  });

  const staged = await stagedReady.promise;
  assert.equal(manager.snapshot().reload.phase, "draining");
  turnRelease.resolve();
  await predecessorTurn;
  const result = await staged.completion;

  assert.deepEqual(events, ["candidate_activated"]);
  assert.equal(result.outcome, "implementation_compatible");
  assert.equal(manager.snapshot().activeGeneration.generationId, "g2");
  const state = await store.read();
  assert.equal(state.activeGeneration.generationId, "g2");
  assert.equal(state.reloads["reload-1"].status, "active_unexercised");
  assert.equal(state.reloads["reload-1"].predecessorRetirement.status, "retired");
});

test("records first successor exercise and consequential effects", async (t) => {
  const { manager, store } = await managerFixture(t);
  let staged;
  await manager.runAdmission({ kind: "turn", id: "turn-1" }, async () => {
    staged = await manager.requestReload({ requestedByTurnId: "turn-1", source: {} });
  });
  await staged.completion;

  await manager.runAdmission({ kind: "turn", id: "turn-2" }, async (active) => {
    assert.equal(active.generationId, "g2");
    await manager.recordEffect({ generationId: active.generationId, effectId: "effect-1" });
  });

  const reload = (await store.read()).reloads["reload-1"];
  assert.equal(reload.status, "active_exercised");
  assert.equal(reload.acceptedTurnId, "turn-2");
  assert.deepEqual(reload.producedEffects, ["effect-1"]);
});

test("refuses incompatible environments without replacing the active generation", async (t) => {
  const candidate = generation("g2", { environmentFingerprint: "environment-2" });
  const { manager, store } = await managerFixture(t, { candidate });
  let staged;
  await manager.runAdmission({ kind: "turn", id: "turn-1" }, async () => {
    staged = await manager.requestReload({ requestedByTurnId: "turn-1", source: {} });
  });

  const result = await staged.completion;
  assert.equal(result.outcome, "environment_migration_required");
  assert.equal(manager.snapshot().activeGeneration.generationId, "g1");
  assert.equal((await store.read()).activeGeneration.generationId, "g1");
  await manager.runAdmission({ kind: "turn", id: "turn-2" }, async (active) => {
    assert.equal(active.generationId, "g1");
  });
});

test("separates bootstrap restart requirements from invalid candidates", async (t) => {
  await t.test("bootstrap fingerprint change", async (nested) => {
    const { manager, store } = await managerFixture(nested, {
      candidate: generation("g2", { bootstrapFingerprint: "bootstrap-2" }),
    });
    let staged;
    await manager.runAdmission({ kind: "turn", id: "turn-1" }, async () => {
      staged = await manager.requestReload({ requestedByTurnId: "turn-1", source: {} });
    });
    const result = await staged.completion;
    assert.equal(result.outcome, "bootstrap_restart_required");
    assert.equal((await store.read()).activeGeneration.generationId, "g1");
  });

  await t.test("candidate validation failure", async (nested) => {
    const { manager, store } = await managerFixture(nested, {
      candidate: generation("g2", { async validate() { return { valid: false }; } }),
    });
    let staged;
    await manager.runAdmission({ kind: "turn", id: "turn-1" }, async () => {
      staged = await manager.requestReload({ requestedByTurnId: "turn-1", source: {} });
    });
    const result = await staged.completion;
    assert.equal(result.outcome, "candidate_invalid");
    assert.equal(result.status, "validation_failed");
    assert.equal((await store.read()).activeGeneration.generationId, "g1");
  });
});

test("serializes executable replacement against another replaceable substrate", async (t) => {
  const arbiter = new InMemoryReplaceableSubstrateArbiter();
  const contextToken = arbiter.acquire({ kind: "model_context", subject: "role:main" });
  const { manager } = await managerFixture(t, { arbiter });

  await manager.runAdmission({ kind: "turn", id: "turn-1" }, async () => {
    await assert.rejects(
      manager.requestReload({ requestedByTurnId: "turn-1", source: {} }),
      (error) => error instanceof ExecutableGenerationAdmissionError
        && error.code === "lifecycle_transition_in_flight",
    );
  });
  assert.equal(manager.snapshot().reload, null);
  arbiter.release(contextToken);
});

test("rejects a runtime generation that conflicts with durable active identity", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-generation-store-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const store = new FileExecutableGenerationStore(path.join(root, "generations.json"));
  const active = {
    generationId: "g1",
    sourceDigest: "source-g1",
    environmentFingerprint: "environment-1",
    bootstrapFingerprint: "bootstrap-1",
  };
  await store.initializeActive(active);

  await assert.rejects(
    store.initializeActive({ ...active, environmentFingerprint: "changed" }),
    (error) => error instanceof ExecutableGenerationConflictError,
  );
});

test("migrates version-1 state and atomically records startup reconciliation", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-generation-startup-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const storePath = path.join(root, "generations.json");
  const active = {
    generationId: "g1",
    sourceDigest: "source-g1",
    environmentFingerprint: "environment-1",
    bootstrapFingerprint: "bootstrap-1",
    activatedByReloadId: null,
  };
  await writeFile(storePath, `${JSON.stringify({
    schemaVersion: 1,
    revision: 1,
    activeGeneration: active,
    reloads: {},
  })}\n`);
  const store = new FileExecutableGenerationStore(storePath, {
    now: () => "2026-08-26T12:00:00.000Z",
  });

  const migrated = await store.read();
  assert.equal(migrated.schemaVersion, 3);
  assert.deepEqual(migrated.startupReconciliations, []);
  const receipt = await store.activateStartupCandidate({
    expectedActiveGenerationId: "g1",
    successor: { ...active, generationId: "g2", sourceDigest: "source-g2" },
  });
  assert.equal(receipt.outcome, "implementation_compatible");
  assert.equal(receipt.predecessorGenerationId, "g1");
  assert.equal(receipt.successorGenerationId, "g2");

  const state = await store.read();
  assert.equal(state.schemaVersion, 3);
  assert.equal(state.activeGeneration.generationId, "g2");
  assert.equal(state.startupReconciliations.length, 1);
  const restarted = await store.activateStartupCandidate({
    expectedActiveGenerationId: "g2",
    successor: {
      ...active,
      generationId: "g3",
      sourceDigest: "source-g3",
      bootstrapFingerprint: "bootstrap-2",
    },
  });
  assert.equal(restarted.outcome, "bootstrap_restart_completed");
  assert.equal(restarted.predecessorBootstrapFingerprint, "bootstrap-1");
  assert.equal(restarted.successorBootstrapFingerprint, "bootstrap-2");
  await assert.rejects(
    store.activateStartupCandidate({
      expectedActiveGenerationId: "g1",
      successor: { ...active, generationId: "g4", sourceDigest: "source-g4" },
    }),
    (error) => error instanceof ExecutableGenerationConflictError,
  );
  assert.equal((await store.read()).activeGeneration.generationId, "g3");
});

test("stable shutdown records and abandons a staged reload without activating it", async (t) => {
  let disposed = false;
  const { manager, store } = await managerFixture(t);
  manager.activeGeneration.dispose = async () => { disposed = true; };
  const admission = manager.openAdmission({ kind: "turn", id: "provider-turn", subjectId: "turn-1" });
  const staged = await manager.requestReload({ requestedByTurnId: "turn-1", source: {} });

  await manager.close({ abandonActiveWork: true });
  assert.equal((await staged.completion).status, "shutdown_interrupted");
  assert.equal((await store.read()).reloads["reload-1"].status, "shutdown_interrupted");
  assert.equal(manager.snapshot().activeGeneration.generationId, "g1");
  assert.equal(disposed, true);
  assert.throws(() => manager.closeAdmission(admission),
    (error) => error instanceof ExecutableGenerationAdmissionError
      && error.code === "invalid_admission_token");
});
