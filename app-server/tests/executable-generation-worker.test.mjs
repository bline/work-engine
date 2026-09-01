import assert from "node:assert/strict";
import { ChildProcess } from "node:child_process";
import {
  appendFile, copyFile, mkdir, mkdtemp, readFile, rm, unlink, writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  ExecutableGenerationDispatchHost,
  ExecutableGenerationManager,
  ExecutableGenerationStartupError,
  ExecutableGenerationWorkerError,
  FileExecutableGenerationStore,
  ForkedExecutableGenerationWorker,
  GenerationBoundAppServerTransport,
  InMemoryReplaceableSubstrateArbiter,
  createExecutableGenerationBootstrap,
  openSqliteAppServerStateStore,
} from "../src/index.mjs";
import {
  DEFAULT_EXECUTABLE_GENERATION_FILES,
  DEFAULT_ROLE_EXECUTABLE_GENERATION_FILES,
} from "../src/executable-generation-bootstrap.mjs";
import { createExecutableGenerationRoleEnvironment } from "../src/executable-generation-role-environment.mjs";
import {
  createSupervisorCampaignHostEffectRuntime,
  SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL,
} from "../src/services/slice-campaign/host-effect-runtime.mjs";
import { createSupervisorCampaignCapabilityHostRuntime } from
  "../src/services/slice-campaign/capability-host-runtime.mjs";

const ENTRY = path.resolve(
  "app-server/tests/fixtures/executable-generation-worker-fixture.mjs",
);

test("role generations pin product-development services and deterministic validators", () => {
  for (const expected of [
    "app-server/src/services/product-development/artifact-root.mjs",
    "app-server/src/services/product-development/intake-delivery.mjs",
    "app-server/src/services/product-development/proposal-delivery.mjs",
    "app-server/roles/strategic-planner.mjs",
    "app-server/roles/strategic-planning-handoff.mjs",
    "skills/idea-intake/scripts/idea_intake.py",
    "skills/idea-intake/schemas/intake-record-v1.schema.json",
    "skills/proposal-packets/scripts/proposal_packets.py",
    "skills/proposal-packets/schemas/proposal-packet-v1.schema.json",
  ]) assert.equal(DEFAULT_ROLE_EXECUTABLE_GENERATION_FILES.includes(expected), true, expected);
  for (const expected of [
    "app-server/src/services/slice-campaign/capability-contract.mjs",
    "app-server/src/services/slice-campaign/host-effect-runtime.mjs",
    "app-server/src/services/slice-campaign/strategic-reconciliation.mjs",
  ]) assert.equal(DEFAULT_EXECUTABLE_GENERATION_FILES.includes(expected), true, expected);
});

function record(generationId) {
  return {
    generationId,
    sourceDigest: `source-${generationId}`,
    environmentFingerprint: "environment-1",
    bootstrapFingerprint: "bootstrap-1",
  };
}

function deferred() {
  let resolve;
  const promise = new Promise((settle) => { resolve = settle; });
  return { promise, resolve };
}

function spawnWorker(generationId, overrides = {}) {
  return ForkedExecutableGenerationWorker.spawn({
    entryPath: ENTRY,
    requestTimeoutMs: 2_000,
    env: {
      ...process.env,
      WORK_ENGINE_TEST_GENERATION: JSON.stringify(record(generationId)),
      ...overrides,
    },
  });
}

async function copyDefaultExecutableGenerationFiles(workspaceRoot) {
  for (const relative of DEFAULT_EXECUTABLE_GENERATION_FILES) {
    const target = path.join(workspaceRoot, relative);
    await mkdir(path.dirname(target), { recursive: true });
    await copyFile(path.resolve(relative), target);
  }
}

test("forked generation worker enforces validation and activation before dispatch", async (t) => {
  const worker = await spawnWorker("g1");
  t.after(() => worker.dispose());
  assert.deepEqual(record("g1"), {
    generationId: worker.generationId,
    sourceDigest: worker.sourceDigest,
    environmentFingerprint: worker.environmentFingerprint,
    bootstrapFingerprint: worker.bootstrapFingerprint,
  });

  await assert.rejects(
    worker.dispatch("echo", { value: 1 }),
    (error) => error instanceof ExecutableGenerationWorkerError
      && error.code === "worker_request_failed",
  );
  assert.deepEqual(await worker.validate(), { valid: true });
  await worker.activate();
  assert.deepEqual(await worker.dispatch("echo", { value: 2 }), {
    disposition: "respond",
    result: { generationId: "g1", payload: { value: 2 } },
  });
});

test("worker exit rejects an in-flight dispatch with bounded failure evidence", async (t) => {
  const worker = await spawnWorker("g1");
  t.after(() => worker.dispose());
  await worker.validate();
  await worker.activate();

  await assert.rejects(
    worker.dispatch("crash", {}),
    (error) => error instanceof ExecutableGenerationWorkerError
      && error.code === "worker_exited"
      && error.details.code === 17,
  );
});

test("worker effects exist only inside their admitted dispatch", async (t) => {
  const worker = await spawnWorker("g1");
  t.after(() => worker.dispose());
  await worker.validate();
  await worker.activate();
  const effects = [];

  assert.throws(
    () => worker.dispatch("effect", { method: "thread/start" }, {}),
    /generation dispatch effect must be a function or null/,
  );

  assert.deepEqual(await worker.dispatch("effect", { method: "thread/start" }, async (payload) => {
    effects.push(payload);
    return { thread: { id: "thread-role" } };
  }), {
    disposition: "respond",
    result: { thread: { id: "thread-role" } },
  });
  assert.deepEqual(effects, [{ method: "thread/start" }]);
  await assert.rejects(
    worker.dispatch("effect", { method: "thread/start" }, async () => {
      throw new Error("bounded fixture effect failure");
    }),
    (error) => error instanceof ExecutableGenerationWorkerError
      && error.code === "worker_request_failed"
      && error.details.diagnosticCode === "effect_failed",
  );
  assert.deepEqual(await worker.dispatch("effect", { method: "thread/start" }, async () => ({
    thread: { id: "thread-after-failure" },
  })), {
    disposition: "respond",
    result: { thread: { id: "thread-after-failure" } },
  });
  await assert.rejects(
    worker.dispatch("effect", { method: "thread/start" }),
    (error) => error instanceof ExecutableGenerationWorkerError
      && error.code === "invalid_worker_effect",
  );
});

test("a fixture tool call reaches only the typed stable supervisor campaign host runtime", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-supervisor-host-effect-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const worker = await spawnWorker("generation-host-bound");
  await worker.validate();
  await worker.activate();
  const manager = await ExecutableGenerationManager.create({
    activeGeneration: worker,
    store: new FileExecutableGenerationStore(path.join(root, "generations.json")),
    substrateArbiter: new InMemoryReplaceableSubstrateArbiter(),
    snapshotter: async () => {}, candidateBuilder: async () => {},
  });
  const calls = [];
  const runtime = createSupervisorCampaignHostEffectRuntime({ registrations: [{
    capability: "capability.preflight", operation: "validate",
    validateInput(value) {
      assert.deepEqual(Object.keys(value).sort(), ["campaign"]);
      assert.deepEqual(Object.keys(value.campaign).sort(), ["identity"]);
      return value;
    },
    async handler(value) {
      calls.push(value);
      return { schema_version: 1, status: "valid", identity: value.input.campaign.identity };
    },
    validateOutput(value) {
      assert.deepEqual(Object.keys(value).sort(), ["identity", "schema_version", "status"]);
      return value;
    },
  }] });
  let serverRequest;
  const delegate = {
    onServerRequest(handler) { serverRequest = handler; }, onNotification() {}, onClosed() {},
    async request() { throw new Error("supervisor effect must not reach App Server transport"); },
    async notify() {},
  };
  new GenerationBoundAppServerTransport({
    transport: delegate,
    dispatchHost: new ExecutableGenerationDispatchHost(manager),
    supervisorCampaignHostEffectRuntime: runtime,
  });
  const envelope = {
    protocol: SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL,
    capability: "capability.preflight", operation: "validate",
    input: { campaign: { identity: "campaign-worker-vertical" } },
  };
  assert.deepEqual(await serverRequest({
    method: "item/tool/call",
    params: { tool: "fixture_supervisor_campaign_effect", arguments: envelope },
  }), { schema_version: 1, status: "valid", identity: "campaign-worker-vertical" });
  assert.deepEqual(calls, [{
    generationId: "generation-host-bound",
    input: { campaign: { identity: "campaign-worker-vertical" } },
  }]);

  await assert.rejects(serverRequest({ method: "item/tool/call", params: {
    tool: "fixture_supervisor_campaign_effect",
    arguments: { ...envelope, operation: "unknown" },
  }}), (error) => error instanceof ExecutableGenerationWorkerError
      && error.details.diagnosticCode === "effect_failed");
  assert.equal(calls.length, 1);
  await manager.close();
  runtime.close();
  runtime.close();
});

test("dispatch and effect failures preserve only bounded diagnostics", async (t) => {
  const worker = await spawnWorker("g1");
  t.after(() => worker.dispose());
  await worker.validate();
  await worker.activate();

  await assert.rejects(
    worker.dispatch("effect", { method: "thread/start" }, async () => {
      const error = new Error("bounded effect failed\nwithout a stack");
      error.code = "capability_unavailable";
      error.details = { secret: "must-not-cross-ipc" };
      throw error;
    }),
    (error) => error instanceof ExecutableGenerationWorkerError
      && error.code === "worker_request_failed"
      && error.message === "bounded effect failed without a stack"
      && error.details.diagnosticCode === "capability_unavailable"
      && !JSON.stringify(error).includes("must-not-cross-ipc")
      && !JSON.stringify(error).includes("stack"),
  );

  await assert.rejects(
    worker.dispatch("unsupported", {}),
    (error) => error instanceof ExecutableGenerationWorkerError
      && error.message === "unsupported fixture operation"
      && error.details.diagnosticCode === "dispatch_failed",
  );
});

test("malformed parent IPC fails an in-flight worker effect promptly and safely", async (t) => {
  const worker = await spawnWorker("g1");
  t.after(() => worker.dispose());
  await worker.validate();
  await worker.activate();

  const startedAt = Date.now();
  await assert.rejects(
    worker.dispatch("effect", { method: "thread/start" }, async () => new Promise((_, reject) => {
      worker.child.send({
        protocol: "not-the-generation-protocol",
        version: 1,
        id: 4,
        secret: "must-not-cross-ipc-diagnostic",
      }, (error) => {
        if (error) reject(error);
      });
    })),
    (error) => error instanceof ExecutableGenerationWorkerError
      && error.code === "worker_request_failed"
      && error.message === "generation bootstrap sent an invalid protocol message"
      && error.details.diagnosticCode === "invalid_parent_message"
      && !JSON.stringify(error).includes("must-not-cross-ipc-diagnostic"),
  );
  assert.ok(Date.now() - startedAt < 1_000);
});

test("stable transport keeps in-flight work on its predecessor and routes later work to successor", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-worker-swap-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const dispatches = [];
  const requestStarted = deferred();
  const requestRelease = deferred();
  const generation = (generationId) => ({
    ...record(generationId),
    async validate() { return { valid: true }; },
    async activate() {},
    async dispose() {},
    async dispatch(operation) {
      dispatches.push({ generationId, operation });
      return { disposition: "forward" };
    },
  });
  const store = new FileExecutableGenerationStore(path.join(root, "generations.json"));
  const manager = await ExecutableGenerationManager.create({
    activeGeneration: generation("g1"),
    store,
    substrateArbiter: new InMemoryReplaceableSubstrateArbiter(),
    snapshotter: async () => ({ snapshotId: "snapshot-g2", sourceDigest: "source-g2" }),
    candidateBuilder: async () => generation("g2"),
    reloadIdFactory: () => "reload-1",
  });
  const delegate = {
    onServerRequest() {},
    onNotification() {},
    onClosed() {},
    notify() {},
    async request(method) {
      if (method === "held") {
        requestStarted.resolve();
        await requestRelease.promise;
      }
      return { method };
    },
  };
  const transport = new GenerationBoundAppServerTransport({
    transport: delegate,
    dispatchHost: new ExecutableGenerationDispatchHost(manager),
  });

  const heldRequest = transport.request("held", {});
  await requestStarted.promise;
  let staged;
  await manager.runAdmission({ kind: "turn", id: "turn-reload" }, async () => {
    staged = await manager.requestReload({ requestedByTurnId: "turn-reload", source: {} });
  });
  assert.equal(manager.snapshot().activeGeneration.generationId, "g1");
  requestRelease.resolve();
  assert.deepEqual(await heldRequest, { method: "held" });
  assert.equal((await staged.completion).generationId, "g2");

  assert.deepEqual(await transport.request("later", {}), { method: "later" });
  assert.deepEqual(dispatches, [
    { generationId: "g1", operation: "app_server.request" },
    { generationId: "g2", operation: "app_server.request" },
  ]);
});

test("a generation can respond locally without invoking the stable App Server effect", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-worker-response-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  let forwarded = false;
  const active = {
    ...record("g1"),
    async dispatch() { return { disposition: "respond", result: { local: true } }; },
  };
  const manager = await ExecutableGenerationManager.create({
    activeGeneration: active,
    store: new FileExecutableGenerationStore(path.join(root, "generations.json")),
    substrateArbiter: new InMemoryReplaceableSubstrateArbiter(),
    snapshotter: async () => {},
    candidateBuilder: async () => {},
  });
  const host = new ExecutableGenerationDispatchHost(manager);

  assert.deepEqual(await host.run({
    kind: "test",
    id: "operation-1",
    operation: "local",
    payload: {},
  }, async () => { forwarded = true; }), { local: true });
  assert.equal(forwarded, false);
});

test("asynchronous generation decisions preserve client send order without serializing responses", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-worker-order-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const notificationDecision = deferred();
  const requestResponse = deferred();
  const events = [];
  const active = {
    ...record("g1"),
    async dispatch(operation) {
      if (operation === "app_server.notification") await notificationDecision.promise;
      return { disposition: "forward" };
    },
  };
  const manager = await ExecutableGenerationManager.create({
    activeGeneration: active,
    store: new FileExecutableGenerationStore(path.join(root, "generations.json")),
    substrateArbiter: new InMemoryReplaceableSubstrateArbiter(),
    snapshotter: async () => {},
    candidateBuilder: async () => {},
  });
  const transport = new GenerationBoundAppServerTransport({
    dispatchHost: new ExecutableGenerationDispatchHost(manager),
    transport: {
      onServerRequest() {},
      onNotification() {},
      notify(method) { events.push(`notify:${method}`); },
      request(method) {
        events.push(`request:${method}`);
        return requestResponse.promise;
      },
    },
  });

  const notification = transport.notify("initialized");
  const request = transport.request("thread/list", {});
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(events, []);
  notificationDecision.resolve();
  await notification;
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(events, ["notify:initialized", "request:thread/list"]);
  requestResponse.resolve({ accepted: true });
  assert.deepEqual(await request, { accepted: true });
});

test("bootstrap reconciles compatible workspace edits and refuses stale recovery", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-worker-bootstrap-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const workspaceRoot = path.join(root, "workspace");
  const stateRoot = path.join(root, "state");
  const targetRoot = path.join(workspaceRoot, "app-server", "src");
  await copyDefaultExecutableGenerationFiles(workspaceRoot);
  const delegate = {
    onServerRequest() {},
    onNotification() {},
    onClosed() {},
    notify() {},
    async request(method) { return { method }; },
  };

  const first = await createExecutableGenerationBootstrap({
    workspaceRoot,
    stateRoot,
    transport: delegate,
    workerRequestTimeoutMs: 2_000,
  });
  const activeGenerationId = first.manager.snapshot().activeGeneration.generationId;
  assert.deepEqual(await first.transport.request("first", {}), { method: "first" });
  await first.close();

  await rm(path.join(stateRoot, "generations", activeGenerationId), {
    recursive: true,
    force: true,
  });

  const resumed = await createExecutableGenerationBootstrap({
    workspaceRoot,
    stateRoot,
    transport: delegate,
    workerRequestTimeoutMs: 2_000,
  });
  assert.equal(resumed.startupSelection.outcome, "durable_active_current");
  assert.equal(resumed.startupSelection.selectedGenerationId, activeGenerationId);
  assert.equal(resumed.startupSelection.reconciliationId, null);
  assert.equal(resumed.currentSnapshot.directory, path.join(
    stateRoot,
    "generations",
    activeGenerationId,
  ));
  await resumed.close();

  await appendFile(path.join(targetRoot, "default-executable-generation-worker.mjs"), "\n");
  const second = await createExecutableGenerationBootstrap({
    workspaceRoot,
    stateRoot,
    transport: delegate,
    workerRequestTimeoutMs: 2_000,
  });

  assert.notEqual(second.currentSnapshot.snapshotId, activeGenerationId);
  assert.equal(
    second.manager.snapshot().activeGeneration.generationId,
    second.currentSnapshot.snapshotId,
  );
  assert.equal(second.startupSelection.outcome, "implementation_compatible");
  assert.equal(second.startupSelection.durableGenerationId, activeGenerationId);
  assert.equal(second.startupSelection.selectedGenerationId, second.currentSnapshot.snapshotId);
  assert.match(second.startupSelection.reconciliationId, /^startup-/);
  const reconciledState = await new FileExecutableGenerationStore(
    path.join(stateRoot, "generation-state.json"),
  ).read();
  assert.equal(reconciledState.schemaVersion, 3);
  assert.equal(reconciledState.startupReconciliations.length, 1);
  assert.equal(
    reconciledState.startupReconciliations[0].successorGenerationId,
    second.currentSnapshot.snapshotId,
  );
  assert.deepEqual(await second.transport.request("second", {}), { method: "second" });
  const reconciledGenerationId = second.currentSnapshot.snapshotId;
  await second.close();

  const restarted = await createExecutableGenerationBootstrap({
    workspaceRoot,
    stateRoot,
    transport: delegate,
    bootstrapFingerprint: "work-engine.app-server-bootstrap-ipc-v2",
    workerRequestTimeoutMs: 2_000,
  });
  assert.equal(restarted.startupSelection.outcome, "bootstrap_restart_completed");
  assert.equal(restarted.startupSelection.selectedGenerationId, reconciledGenerationId);
  assert.match(restarted.startupSelection.reconciliationId, /^startup-/);
  await restarted.close();

  await appendFile(
    path.join(targetRoot, "default-executable-generation-worker.mjs"),
    "\nthis is not valid JavaScript !!!\n",
  );
  await assert.rejects(
    createExecutableGenerationBootstrap({
      workspaceRoot,
      stateRoot,
      transport: delegate,
      bootstrapFingerprint: "work-engine.app-server-bootstrap-ipc-v2",
      workerRequestTimeoutMs: 2_000,
    }),
    (error) => error instanceof ExecutableGenerationStartupError
      && error.code === "candidate_invalid"
      && error.details.durableGenerationId === reconciledGenerationId,
  );
  const stateAfterInvalidCandidate = await new FileExecutableGenerationStore(
    path.join(stateRoot, "generation-state.json"),
  ).read();
  assert.equal(stateAfterInvalidCandidate.activeGeneration.generationId, reconciledGenerationId);

  await unlink(path.join(targetRoot, "default-executable-generation-worker.mjs"));
  await assert.rejects(
    createExecutableGenerationBootstrap({
      workspaceRoot,
      stateRoot,
      transport: delegate,
      bootstrapFingerprint: "work-engine.app-server-bootstrap-ipc-v2",
      workerRequestTimeoutMs: 2_000,
    }),
    (error) => error instanceof ExecutableGenerationStartupError
      && error.code === "workspace_snapshot_invalid"
      && error.details.durableGenerationId === reconciledGenerationId,
  );
  const preservedState = await new FileExecutableGenerationStore(
    path.join(stateRoot, "generation-state.json"),
  ).read();
  assert.equal(preservedState.activeGeneration.generationId, reconciledGenerationId);
});

test("bootstrap owns one stable supervisor campaign host runtime across reload and shutdown", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-host-runtime-bootstrap-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const workspaceRoot = path.join(root, "workspace");
  const targetRoot = path.join(workspaceRoot, "app-server", "src");
  await copyDefaultExecutableGenerationFiles(workspaceRoot);
  const delegate = {
    onServerRequest() {}, onNotification() {}, onClosed() {}, notify() {},
    async request(method) { return { method }; },
  };
  const runtime = { async dispatch() {}, async close() {} };

  await assert.rejects(createExecutableGenerationBootstrap({
    workspaceRoot,
    stateRoot: path.join(root, "mutually-exclusive-state"),
    transport: delegate,
    supervisorCampaignHostEffectRuntime: runtime,
    supervisorCampaignHostEffectRuntimeFactory: async () => runtime,
  }), /runtime and factory are mutually exclusive/);

  let factoryWorker;
  const childrenBeforeFailure = new Set(
    process._getActiveHandles().filter((handle) => handle instanceof ChildProcess),
  );
  await assert.rejects(createExecutableGenerationBootstrap({
    workspaceRoot,
    stateRoot: path.join(root, "factory-failure-state"),
    transport: delegate,
    workerRequestTimeoutMs: 2_000,
    supervisorCampaignHostEffectRuntimeFactory: async () => {
      factoryWorker = process._getActiveHandles().find((handle) =>
        handle instanceof ChildProcess && !childrenBeforeFailure.has(handle));
      throw new Error("fixture host runtime factory failure");
    },
  }), (error) => error instanceof ExecutableGenerationStartupError
      && error.code === "candidate_invalid"
      && error.details.failureType === "Error");
  assert.equal(factoryWorker instanceof ChildProcess, true);
  assert.equal(factoryWorker.connected, false);

  const events = [];
  let factoryCalls = 0;
  let bootstrap;
  bootstrap = await createExecutableGenerationBootstrap({
    workspaceRoot,
    stateRoot: path.join(root, "stable-runtime-state"),
    transport: delegate,
    workerRequestTimeoutMs: 2_000,
    supervisorCampaignHostEffectRuntimeFactory: async ({ workspaceRoot: ownerWorkspaceRoot,
      stateRoot: ownerStateRoot }) => {
      factoryCalls += 1;
      assert.equal(ownerWorkspaceRoot, path.resolve(workspaceRoot));
      assert.equal(ownerStateRoot, path.resolve(root, "stable-runtime-state"));
      return {
        async dispatch() {},
        async close() {
          await assert.rejects(
            bootstrap.manager.activeGeneration.dispatch("echo", {}),
            (error) => error instanceof ExecutableGenerationWorkerError
              && error.code === "worker_unavailable",
          );
          events.push("runtime:worker-unavailable");
        },
      };
    },
  });
  assert.equal(factoryCalls, 1);

  await appendFile(path.join(targetRoot, "default-executable-generation-worker.mjs"), "\n");
  let staged;
  await bootstrap.manager.runAdmission({ kind: "turn", id: "reload-stable-runtime" }, async () => {
    staged = await bootstrap.manager.requestReload({
      requestedByTurnId: "reload-stable-runtime",
      source: {},
    });
  });
  await staged.completion;
  assert.equal(factoryCalls, 1);

  await bootstrap.close();
  assert.deepEqual(events, ["runtime:worker-unavailable"]);
});

test("bootstrap binds one sealed extension to generation identity and recovers it on restart", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-extension-bootstrap-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const workspaceRoot = path.join(root, "workspace");
  const stateRoot = path.join(root, "state");
  await copyDefaultExecutableGenerationFiles(workspaceRoot);
  for (const relative of [
    "app-server/migrations/skills/repo-search/structure.yaml",
    "app-server/migrations/skills/repo-search/interface.yaml",
  ]) {
    await mkdir(path.dirname(path.join(workspaceRoot, relative)), { recursive: true });
    await copyFile(path.resolve(relative), path.join(workspaceRoot, relative));
  }
  const bundle = JSON.parse(await readFile(
    "app-server/tests/fixtures/run-extension-bundle/sealed-bundle.json", "utf8",
  ));
  const delegate = { onServerRequest() {}, onNotification() {}, onClosed() {}, notify() {},
    async request(method) { return { method }; } };
  const first = await createExecutableGenerationBootstrap({
    workspaceRoot, stateRoot, transport: delegate, workerRequestTimeoutMs: 2_000,
    extensionHostPolicy: {
      repositoryRevision: "2af529971c1b5660de4caad7092cd270dcd162eb",
      allowedCapabilities: ["product-development.intake.read-source"],
      allowedAdapters: ["product-development.intake.read-source"], allowedProviders: ["local"],
    },
  });
  let staged;
  await first.manager.runAdmission({ kind: "turn", id: "attach" }, async () => {
    staged = await first.manager.requestReload({ requestedByTurnId: "attach", source: { extensionBundle: bundle } });
  });
  const activated = await staged.completion;
  assert.match(activated.generationId, /^generation-[0-9a-f]{64}$/);
  assert.equal(first.manager.activeGeneration.extensionAttachment.bundle_id, "fixture-research");
  const active = (await new FileExecutableGenerationStore(first.statePath).read()).activeGeneration;
  assert.match(active.extensionAttachment.sha256, /^[0-9a-f]{64}$/);
  await first.close();

  const restarted = await createExecutableGenerationBootstrap({
    workspaceRoot, stateRoot, transport: delegate, workerRequestTimeoutMs: 2_000,
  });
  assert.equal(restarted.startupSelection.outcome, "durable_active_current");
  assert.equal(restarted.manager.snapshot().activeGeneration.generationId, active.generationId);
  await restarted.close();

  const rebound = await createExecutableGenerationBootstrap({
    workspaceRoot, stateRoot, transport: delegate, workerRequestTimeoutMs: 2_000,
    bootstrapFingerprint: "work-engine.app-server-bootstrap-ipc-extension-reconcile-v2",
  });
  assert.equal(rebound.startupSelection.outcome, "bootstrap_restart_completed");
  const reboundActive = (await new FileExecutableGenerationStore(rebound.statePath).read())
    .activeGeneration;
  assert.equal(reboundActive.extensionAttachment.sha256, active.extensionAttachment.sha256);
  assert.equal(rebound.manager.activeGeneration.extensionAttachment.sha256,
    active.extensionAttachment.sha256);
  await rebound.close();
});

test("bootstrap refuses a workspace edit that changes the environment fingerprint", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-worker-migration-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const workspaceRoot = path.join(root, "workspace");
  const stateRoot = path.join(root, "state");
  const targetRoot = path.join(workspaceRoot, "app-server", "src");
  await copyDefaultExecutableGenerationFiles(workspaceRoot);
  const delegate = {
    onServerRequest() {},
    onNotification() {},
    onClosed() {},
    notify() {},
    async request(method) { return { method }; },
  };
  const first = await createExecutableGenerationBootstrap({
    workspaceRoot,
    stateRoot,
    transport: delegate,
    workerRequestTimeoutMs: 2_000,
  });
  const activeGenerationId = first.manager.snapshot().activeGeneration.generationId;
  await first.close();

  const statePath = path.join(stateRoot, "generation-state.json");
  const originalState = JSON.parse(await readFile(statePath, "utf8"));
  await writeFile(statePath, `${JSON.stringify({
    ...originalState,
    activeGeneration: {
      ...originalState.activeGeneration,
      environmentFingerprint: "environment-mismatch-with-unchanged-source",
    },
  }, null, 2)}\n`);
  await assert.rejects(
    createExecutableGenerationBootstrap({
      workspaceRoot,
      stateRoot,
      transport: delegate,
      workerRequestTimeoutMs: 2_000,
    }),
    (error) => error instanceof ExecutableGenerationStartupError
      && error.code === "environment_migration_required",
  );
  await writeFile(statePath, `${JSON.stringify(originalState, null, 2)}\n`);

  const workerPath = path.join(targetRoot, "default-executable-generation-worker.mjs");
  const workerSource = await readFile(workerPath, "utf8");
  const changedSource = workerSource.replace(
    /function transparentEnvironmentFingerprint\(\) \{[\s\S]*?\n\}/,
    'function transparentEnvironmentFingerprint() {\n  return "environment-changed";\n}',
  );
  assert.notEqual(changedSource, workerSource);
  await writeFile(workerPath, changedSource);

  await assert.rejects(
    createExecutableGenerationBootstrap({
      workspaceRoot,
      stateRoot,
      transport: delegate,
      workerRequestTimeoutMs: 2_000,
    }),
    (error) => error instanceof ExecutableGenerationStartupError
      && error.code === "environment_migration_required"
      && error.details.durableGenerationId === activeGenerationId,
  );
  const preservedState = await new FileExecutableGenerationStore(
    path.join(stateRoot, "generation-state.json"),
  ).read();
  assert.equal(preservedState.activeGeneration.generationId, activeGenerationId);
  assert.deepEqual(preservedState.startupReconciliations, []);
});

test("environment tools bind a reload to the provider turn and activate after turn completion", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-worker-tools-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const workspaceRoot = path.join(root, "workspace");
  const targetRoot = path.join(workspaceRoot, "app-server", "src");
  await copyDefaultExecutableGenerationFiles(workspaceRoot);
  const requests = [];
  const delegate = {
    serverRequestHandler: null,
    notificationHandler: null,
    onServerRequest(handler) { this.serverRequestHandler = handler; },
    onNotification(handler) { this.notificationHandler = handler; },
    onClosed() {},
    notify() {},
    async request(method, params) {
      requests.push({ method, params });
      if (method === "thread/start") return { thread: { id: "thread-1" } };
      if (method === "turn/start") return { turn: { id: "turn-1" } };
      return { accepted: true };
    },
    emitNotification(notification) { this.notificationHandler(notification); },
    invokeServerRequest(request) { return this.serverRequestHandler(request); },
  };
  const bootstrap = await createExecutableGenerationBootstrap({
    workspaceRoot,
    stateRoot: path.join(root, "state"),
    transport: delegate,
    workerRequestTimeoutMs: 2_000,
  });
  t.after(() => bootstrap.close());

  await bootstrap.transport.request("initialize", {
    capabilities: { experimentalApi: false },
  });
  assert.equal(requests[0].params.capabilities.experimentalApi, true);
  await bootstrap.transport.request("thread/start", { cwd: workspaceRoot });
  const environment = requests[1].params.dynamicTools.find((tool) =>
    tool.type === "namespace" && tool.name === "environment"
  );
  assert.deepEqual(environment.tools.map((tool) => tool.name), ["status", "reload"]);
  await bootstrap.transport.request("turn/start", {
    threadId: "thread-1",
    input: [{ type: "text", text: "test" }],
  });
  const admitted = bootstrap.manager.snapshot();
  assert.equal(admitted.admissions.length, 1, JSON.stringify(admitted));
  assert.equal(admitted.admissions[0].subjectId, "turn-1");

  const status = await delegate.invokeServerRequest({
    id: 1,
    method: "item/tool/call",
    params: {
      threadId: "thread-1",
      turnId: "turn-1",
      callId: "call-status",
      namespace: "environment",
      tool: "status",
      arguments: {},
    },
  });
  assert.equal(status.success, true);
  assert.equal(JSON.parse(status.contentItems[0].text).activeGeneration.generationId,
    bootstrap.manager.snapshot().activeGeneration.generationId);

  await appendFile(path.join(targetRoot, "default-executable-generation-worker.mjs"), "\n");
  const reload = await delegate.invokeServerRequest({
    id: 2,
    method: "item/tool/call",
    params: {
      threadId: "thread-1",
      turnId: "turn-1",
      callId: "call-reload",
      namespace: "environment",
      tool: "reload",
      arguments: {},
    },
  });
  const staged = JSON.parse(reload.contentItems[0].text);
  assert.equal(staged.status, "staged");
  const predecessorId = bootstrap.manager.snapshot().activeGeneration.generationId;
  await assert.rejects(bootstrap.transport.request("thread/list", {}),
    (error) => error.code === "reload_fence_active");

  const completed = deferred();
  bootstrap.transport.onNotification((notification) => completed.resolve(notification));
  delegate.emitNotification({
    method: "turn/completed",
    params: { threadId: "thread-1", turn: { id: "turn-1", status: "completed", items: [] } },
  });
  assert.equal((await completed.promise).method, "turn/completed");
  assert.notEqual(bootstrap.manager.snapshot().activeGeneration.generationId, predecessorId);
  assert.equal(bootstrap.manager.snapshot().reload, null);
});

test("turn-completion lifecycle failures are surfaced without an unhandled rejection", async (t) => {
  const lifecycleFailure = deferred();
  const delegate = {
    notificationHandler: null,
    onServerRequest() {},
    onNotification(handler) { this.notificationHandler = handler; },
    notify() {},
    async request() { return { accepted: true }; },
  };
  const manager = {
    runAdmission: async (_admission, operation) => operation({
      async dispatch() { return { disposition: "forward" }; },
    }),
    snapshot: () => ({}),
    closeAdmission() { throw new Error("durable close failed"); },
  };
  const transport = new GenerationBoundAppServerTransport({
    transport: delegate,
    dispatchHost: {
      manager,
      run: manager.runAdmission,
      runInAdmission: async (_admission, operation) => operation({
        async dispatch() { return { disposition: "forward" }; },
      }),
    },
  });
  transport.onLifecycleError((error) => lifecycleFailure.resolve(error));
  transport.turnAdmissions.set("turn-1", { id: "provider-turn:turn-1" });

  delegate.notificationHandler({
    method: "turn/completed",
    params: { turn: { id: "turn-1", status: "completed" } },
  });

  assert.equal((await lifecycleFailure.promise).message, "durable close failed");
});

test("manifest generation intercepts commands and routes ordinary shell turns to one role", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-worker-switchboard-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const requests = [];
  const forwardedNotifications = [];
  const syntheticCompletions = [];
  const syntheticCompletionWaiters = [];
  const syntheticStarts = [];
  const syntheticStartWaiters = [];
  const strategicToolResult = deferred();
  const nestedPlannerObserved = deferred();
  const roleTurnRelease = deferred();
  let roleThreadCount = 0;
  let outerStrategicToolPending = false;
  const strategicInput = {
    instance_id: "main",
    client_user_message_id: "strategic-nested-1",
    strategic_objective: "Keep the exact production worker probe aligned",
    evidence_cutoff: {
      roadmap_revision: "roadmap-1",
      repository_revision: "repository-1",
      campaign_terminals: [],
    },
    canonical_references: [{
      owner: "slice-supervisor",
      reference: "accepted-plan.json",
      revision: "revision-1",
      freshness_rule: "immutable test fixture",
    }],
    continuity: "initialized",
  };
  const delegate = {
    notificationHandler: null,
    serverRequestHandler: null,
    onServerRequest(handler) { this.serverRequestHandler = handler; },
    onNotification(handler) { this.notificationHandler = handler; },
    onClosed() {},
    notify(method, params) { requests.push({ method, params, notification: true }); },
    async request(method, params) {
      requests.push({ method, params });
      if (method === "initialize") return { userAgent: "codex-cli/0.149.1" };
      if (method === "thread/start") {
        if (params.ephemeral === true) return { thread: { id: "thread-inference" } };
        const role = typeof params.developerInstructions === "string";
        const threadId = role
          ? (++roleThreadCount === 1 ? "thread-role" : "thread-strategic")
          : "thread-shell";
        if (role) this.notificationHandler({
          method: "thread/started",
          params: { thread: { id: threadId } },
        });
        return { thread: { id: threadId } };
      }
      if (method === "turn/start") {
        if (params.threadId === "thread-inference") {
          setImmediate(() => this.notificationHandler({
            method: "turn/completed",
            params: {
              threadId: "thread-inference",
              turn: {
                id: "turn-inference",
                status: "completed",
                items: [{
                  type: "agentMessage",
                  id: "agent-inference",
                  text: "invalid bounded compiler output",
                  phase: "final_answer",
                  memoryCitation: null,
                  delivery: null,
                }],
              },
            },
          }));
          return { turn: { id: "turn-inference" } };
        }
        if (params.threadId === "thread-strategic") {
          assert.equal(outerStrategicToolPending, true);
          nestedPlannerObserved.resolve();
          setImmediate(() => this.notificationHandler({
            method: "turn/completed",
            params: {
              threadId: "thread-strategic",
              turn: {
                id: "turn-strategic",
                status: "completed",
                items: [{
                  type: "agentMessage",
                  id: "agent-strategic",
                  text: JSON.stringify({
                    schema_version: 1,
                    strategic_objective: strategicInput.strategic_objective,
                    evidence_cutoff: strategicInput.evidence_cutoff,
                    continuity: strategicInput.continuity,
                    verdict: "continue",
                    current_rationale: "The no-op probe changes no durable strategy.",
                    assumptions: { confirmed: [], changed: [], invalidated: [] },
                    route_changes: {
                      priorities: [], dependencies: [], newly_important: [], deferred: [],
                    },
                    recommended_campaign: {
                      disposition: "continue_current",
                      objective: null,
                      work_source: null,
                      reason: "No campaign change is supported.",
                    },
                    open_uncertainties: [], authority_required: [], revisit_when: [],
                  }),
                  phase: "final_answer",
                  memoryCitation: null,
                  delivery: null,
                }],
                itemsView: "full",
                error: null,
                startedAt: 3,
                completedAt: 4,
                durationMs: 1,
              },
            },
          }));
          return { turn: { id: "turn-strategic", status: "inProgress" } };
        }
        assert.equal(params.threadId, "thread-role");
        setImmediate(async () => {
          await roleTurnRelease.promise;
          outerStrategicToolPending = true;
          try {
            strategicToolResult.resolve(await this.serverRequestHandler({
              id: 91,
              method: "item/tool/call",
              params: {
                threadId: "thread-role",
                turnId: "turn-role",
                callId: "call-strategic-reconciliation",
                namespace: "campaign",
                tool: "strategic_reconciliation",
                arguments: { operation: "reconcile", input: strategicInput },
              },
            }));
          } catch (error) {
            strategicToolResult.resolve({ error });
          } finally {
            outerStrategicToolPending = false;
          }
          this.notificationHandler({
            method: "thread/tokenUsage/updated",
            params: {
              threadId: "thread-role",
              turnId: "turn-role",
              tokenUsage: {
                last: {
                  inputTokens: 10_000,
                  cachedInputTokens: 1_000,
                  cacheWriteInputTokens: 0,
                  outputTokens: 2_000,
                  reasoningOutputTokens: 0,
                  totalTokens: 12_000,
                },
                total: {
                  inputTokens: 10_000,
                  cachedInputTokens: 1_000,
                  cacheWriteInputTokens: 0,
                  outputTokens: 2_000,
                  reasoningOutputTokens: 0,
                  totalTokens: 12_000,
                },
                modelContextWindow: 100_000,
              },
            },
          });
          this.notificationHandler({
            method: "turn/completed",
            params: {
              threadId: "thread-role",
              turn: {
                id: "turn-role",
                status: "completed",
                items: [{
                  type: "agentMessage",
                  id: "agent-role",
                  text: "role-owned response",
                  phase: "final_answer",
                  memoryCitation: null,
                  delivery: null,
                }],
                itemsView: "full",
                error: null,
                startedAt: 1,
                completedAt: 2,
                durationMs: 1_000,
              },
            },
          });
        });
        return {
          turn: {
            id: "turn-role",
            status: "inProgress",
            items: [],
            itemsView: "full",
            error: null,
            startedAt: 1,
            completedAt: null,
            durationMs: null,
          },
        };
      }
      throw new Error(`unexpected App Server request ${method}`);
    },
  };
  const bootstrap = await createExecutableGenerationBootstrap({
    workspaceRoot: path.resolve("."),
    stateRoot: path.join(root, "state"),
    transport: delegate,
    runtimeManifestPath: path.resolve("app-server/runtime-manifest.yaml"),
    semanticContextProfilePath: path.resolve(
      "app-server/tests/fixtures/semantic-context-host-inspection-profile.yaml",
    ),
    semanticContextStatePath: path.join(root, "semantic-context.sqlite3"),
    roleBindingsPath: path.join(root, "bindings.json"),
    workerRequestTimeoutMs: 2_000,
    workerDispatchTimeoutMs: 2_000,
    supervisorCampaignHostEffectRuntimeFactory: ({ workspaceRoot, stateRoot }) =>
      createSupervisorCampaignCapabilityHostRuntime({
        workspaceRoot, stateRoot, canonicalBranches: ["main"],
      }),
  });
  t.after(() => bootstrap.close({ abandonActiveWork: true }));
  const generationConfig = JSON.parse(await readFile(path.join(
    root,
    "state",
    "generations",
    bootstrap.manager.snapshot().activeGeneration.generationId,
    "app-server/generated/executable-role-environment.json",
  ), "utf8"));
  const transportedBuilderRequirements = generationConfig.manifest.runtimeRequirementsByRole[
    "slice-builder"
  ];
  assert.equal(generationConfig.manifest.requirementsBaseDirectory, path.resolve("."));
  assert.equal(transportedBuilderRequirements.schema_version, 1);
  assert.equal(transportedBuilderRequirements.verified_sources, true);
  assert.equal(
    transportedBuilderRequirements.compiled_skill_sha256,
    generationConfig.manifest.document.roles["slice-builder"].compiled_skill_sha256,
  );
  assert.match(transportedBuilderRequirements.sha256, /^[0-9a-f]{64}$/);
  bootstrap.transport.onNotification((notification) => {
    forwardedNotifications.push(notification);
    if (notification.method === "turn/started") {
      const waiter = syntheticStartWaiters.shift();
      if (waiter) waiter(notification);
      else syntheticStarts.push(notification);
    }
    if (notification.method === "turn/completed") {
      const waiter = syntheticCompletionWaiters.shift();
      if (waiter) waiter(notification);
      else syntheticCompletions.push(notification);
    }
  });
  const waitForSyntheticCompletion = () => {
    if (syntheticCompletions.length > 0) return Promise.resolve(syntheticCompletions.shift());
    return new Promise((resolve) => syntheticCompletionWaiters.push(resolve));
  };
  const waitForSyntheticStart = () => {
    if (syntheticStarts.length > 0) return Promise.resolve(syntheticStarts.shift());
    return new Promise((resolve) => syntheticStartWaiters.push(resolve));
  };
  const stage = async (name, operation) => {
    try {
      return await operation;
    } catch (error) {
      error.message = `${name}: ${error.message}; requests=${JSON.stringify(
        requests.map((request) => request.method),
      )}`;
      throw error;
    }
  };

  await stage("initialize", bootstrap.transport.request(
    "initialize",
    { clientInfo: { name: "test" } },
  ));
  await bootstrap.transport.notify("initialized");
  await stage("shell thread", bootstrap.transport.request(
    "thread/start",
    { cwd: path.resolve(".") },
  ));

  const attached = await stage("attach", bootstrap.transport.request("turn/start", {
    threadId: "thread-shell",
    clientUserMessageId: "shell-command-1",
    input: [{ type: "text", text: ":we attach slice-supervisor:main", text_elements: [] }],
  }));
  assert.equal(attached.turn.status, "inProgress");
  assert.deepEqual(attached.turn.items, []);
  assert.equal(attached.turn.itemsView, "notLoaded");
  assert.equal(attached.turn.startedAt, null);
  const attachedCompletion = await waitForSyntheticCompletion();
  await waitForSyntheticStart();
  assert.equal(Number.isInteger(attachedCompletion.params.turn.startedAt), true);
  assert.equal(Number.isInteger(attachedCompletion.params.turn.completedAt), true);
  assert.deepEqual(forwardedNotifications.map((item) => item.method), [
    "turn/started",
    "turn/completed",
  ]);
  assert.equal(JSON.parse(
    forwardedNotifications.at(-1).params.turn.items[0].text,
  ).command, "attach");
  forwardedNotifications.length = 0;
  assert.equal(requests.filter((request) => request.method === "turn/start").length, 0);

  const delivered = await stage("role delivery", bootstrap.transport.request("turn/start", {
    threadId: "thread-shell",
    clientUserMessageId: "shell-message-1",
    input: [{ type: "text", text: "Reconcile this exact objective.", text_elements: [] }],
  }));
  assert.equal(delivered.turn.status, "inProgress");
  assert.deepEqual(delivered.turn.items, []);
  assert.equal(delivered.turn.itemsView, "notLoaded");
  assert.equal(delivered.turn.startedAt, null);
  await waitForSyntheticStart();
  assert.deepEqual(forwardedNotifications.map((item) => item.method), ["turn/started"]);
  roleTurnRelease.resolve();
  const nestedOutcome = await Promise.race([
    nestedPlannerObserved.promise.then(() => "started"),
    strategicToolResult.promise.then((value) => ({ toolResult: value })),
    new Promise((_, reject) => setTimeout(() => reject(new Error(
      `nested strategic planner turn did not start; requests=${JSON.stringify(
        requests.map(({ method, params }) => [method, params?.threadId]),
      )}`,
    )), 1_000)),
  ]);
  if (nestedOutcome !== "started") {
    throw new Error(`strategic tool returned before nested planner: ${JSON.stringify(nestedOutcome)}`);
  }
  const deliveredCompletion = await waitForSyntheticCompletion();
  assert.equal(Number.isInteger(deliveredCompletion.params.turn.startedAt), true);
  assert.equal(Number.isInteger(deliveredCompletion.params.turn.completedAt), true);
  assert.equal(
    deliveredCompletion.params.turn.status,
    "completed",
    JSON.stringify(deliveredCompletion.params.turn.error),
  );
  assert.deepEqual(forwardedNotifications.map((item) => item.method), [
    "turn/started",
    "turn/completed",
  ]);
  const deliveredText = forwardedNotifications.at(-1).params.turn.items[0].text;
  const toolResult = await strategicToolResult.promise;
  assert.equal(toolResult.success, true, toolResult.error?.stack);
  const strategicResult = JSON.parse(toolResult.contentItems[0].text);
  assert.equal(strategicResult.generation_id,
    bootstrap.manager.snapshot().activeGeneration.generationId);
  assert.equal(strategicResult.result.status, "completed");
  assert.equal(strategicResult.result.handoff.strategic_objective,
    strategicInput.strategic_objective);
  assert.equal(strategicResult.result.handoff.verdict, "continue");
  const roleTurnRequests = requests.filter((request) => request.method === "turn/start");
  assert.equal(roleTurnRequests.length, 3);
  assert.equal(requests.some((request) =>
    request.method === "thread/start" && request.params.ephemeral === true
  ), true);
  const roleThreadStart = requests.find((request) => request.method === "thread/start"
    && request.params.dynamicTools?.some((tool) => tool.name === "campaign"));
  assert.deepEqual(roleThreadStart.params.dynamicTools.map((tool) => tool.name), [
    "campaign",
    "environment",
  ]);
  assert.deepEqual(roleTurnRequests.map((request) => request.params.threadId), [
    "thread-role", "thread-strategic", "thread-inference",
  ]);
  assert.deepEqual(bootstrap.manager.snapshot().admissions, []);
  const lifecycleStore = await openSqliteAppServerStateStore({
    filePath: path.join(root, "semantic-context.sqlite3"),
  });
  t.after(() => lifecycleStore.close());
  const receipts = lifecycleStore.receipts({ logicalRoleInstanceId: "slice-supervisor:main" });
  assert.equal(receipts.length, 1);
  assert.equal(receipts[0].pressure.disposition, "replacement_candidate");
  assert.equal(receipts[0].inference.status, "failed");
  assert.match(deliveredText, /^role-owned response\n\[lifecycle\]/);
  assert.deepEqual(receipts[0].transition, {
    status: "not_requested",
    retirementAttempted: false,
  });
  for (const [method, params] of [
    ["turn/start", {
      threadId: "thread-role",
      clientUserMessageId: "bypass-turn",
      input: [{ type: "text", text: "bypass", text_elements: [] }],
    }],
    ["turn/steer", {
      threadId: "thread-role",
      expectedTurnId: "turn-role",
      input: [{ type: "text", text: "bypass", text_elements: [] }],
    }],
    ["thread/inject_items", { threadId: "thread-role", items: [] }],
    ["thread/compact/start", { threadId: "thread-role" }],
    ["thread/realtime/appendText", { threadId: "thread-role", text: "bypass" }],
  ]) {
    const forwardedBefore = requests.filter((request) => request.method === method).length;
    await assert.rejects(
      bootstrap.transport.request(method, params),
      /role-owned threads accept domain input only through the Work Engine switchboard/,
    );
    assert.equal(
      requests.filter((request) => request.method === method).length,
      forwardedBefore,
    );
  }
  assert.equal(requests.some((request) => [
    "turn/steer", "thread/inject_items", "thread/compact/start",
    "thread/realtime/appendText",
  ].includes(request.method)), false);
});

test("executable role snapshots refuse missing, corrupt, or excessive transported requirements before adapter delivery", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-worker-requirements-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const bootstrap = await createExecutableGenerationBootstrap({
    workspaceRoot: path.resolve("."),
    stateRoot: path.join(root, "state"),
    transport: {
      onServerRequest() {}, onNotification() {}, onClosed() {}, notify() {},
      async request() { return {}; },
    },
    runtimeManifestPath: path.resolve("app-server/runtime-manifest.yaml"),
    semanticContextProfilePath: path.resolve(
      "app-server/tests/fixtures/semantic-context-host-inspection-profile.yaml",
    ),
    workerRequestTimeoutMs: 2_000,
    workerDispatchTimeoutMs: 2_000,
  });
  await bootstrap.close({ abandonActiveWork: true });
  const snapshotRoot = path.join(
    root,
    "state",
    "generations",
    bootstrap.manager.snapshot().activeGeneration.generationId,
  );
  const configPath = path.join(
    snapshotRoot,
    "app-server/generated/executable-role-environment.json",
  );
  const original = JSON.parse(await readFile(configPath, "utf8"));

  for (const [label, mutate, expected] of [
    ["missing-base", (config) => { delete config.manifest.requirementsBaseDirectory; }, /manifest requirements base directory must be a non-empty string/],
    ["wrong-base", (config) => { config.manifest.requirementsBaseDirectory = path.resolve("app-server"); }, /contract differs from compiled requirements/],
    ["missing", (config) => { delete config.manifest.runtimeRequirementsByRole["slice-builder"]; }, /no verified compiled requirements/],
    ["corrupt", (config) => { config.manifest.runtimeRequirementsByRole["slice-builder"].sha256 = "0".repeat(64); }, /runtime requirements digest mismatch/],
    ["excessive", (config) => { config.manifest.document.roles["slice-builder"].capabilities.push("capability.uncompiled"); }, /exceeds capability ceiling/],
  ]) {
    const config = structuredClone(original);
    mutate(config);
    await writeFile(configPath, `${JSON.stringify(config, null, 2)}\n`);
    const attachmentPath = path.join(root, `${label}-attachment.json`);
    await writeFile(attachmentPath, `${JSON.stringify({ roleId: "slice-builder", instanceId: "main" })}\n`);
    let environment;
    try {
      environment = await createExecutableGenerationRoleEnvironment({
        snapshotRoot,
        bindingsPath: path.join(root, `${label}-bindings.json`),
        attachmentPath,
        semanticContextStatePath: path.join(root, `${label}-semantic-context.sqlite3`),
      });
    } catch (error) {
      assert.match(error.message, expected, label);
      continue;
    }
    t.after(() => environment.close());
    let adapterDeliveries = 0;
    await environment.handleRequest({ method: "thread/start", params: {} }, async () => ({
      thread: { id: `ui-${label}` },
    }));
    await assert.rejects(
      environment.handleRequest({
        method: "turn/start",
        params: {
          threadId: `ui-${label}`,
          input: [{ type: "text", text: "Build the accepted slice." }],
        },
      }, async () => {
        adapterDeliveries += 1;
        return {};
      }),
      expected,
    );
    assert.equal(adapterDeliveries, 0, `${label} requirements reached adapter delivery`);
  }
});
