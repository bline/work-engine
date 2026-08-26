import assert from "node:assert/strict";
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
} from "../src/index.mjs";

const ENTRY = path.resolve(
  "app-server/tests/fixtures/executable-generation-worker-fixture.mjs",
);

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

  assert.deepEqual(await worker.dispatch("effect", { method: "thread/start" }, async (payload) => {
    effects.push(payload);
    return { thread: { id: "thread-role" } };
  }), {
    disposition: "respond",
    result: { thread: { id: "thread-role" } },
  });
  assert.deepEqual(effects, [{ method: "thread/start" }]);
  await assert.rejects(
    worker.dispatch("effect", { method: "thread/start" }),
    (error) => error instanceof ExecutableGenerationWorkerError
      && error.code === "invalid_worker_effect",
  );
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
  const sourceRoot = path.resolve("app-server/src");
  const targetRoot = path.join(workspaceRoot, "app-server", "src");
  await mkdir(targetRoot, { recursive: true });
  for (const name of [
    "default-executable-generation-worker.mjs",
    "executable-generation-worker-runtime.mjs",
  ]) {
    await copyFile(path.join(sourceRoot, name), path.join(targetRoot, name));
  }
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
  assert.equal(reconciledState.schemaVersion, 2);
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

test("bootstrap refuses a workspace edit that changes the environment fingerprint", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-worker-migration-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const workspaceRoot = path.join(root, "workspace");
  const stateRoot = path.join(root, "state");
  const sourceRoot = path.resolve("app-server/src");
  const targetRoot = path.join(workspaceRoot, "app-server", "src");
  await mkdir(targetRoot, { recursive: true });
  for (const name of [
    "default-executable-generation-worker.mjs",
    "executable-generation-worker-runtime.mjs",
  ]) {
    await copyFile(path.join(sourceRoot, name), path.join(targetRoot, name));
  }
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
  await mkdir(targetRoot, { recursive: true });
  for (const name of [
    "default-executable-generation-worker.mjs",
    "executable-generation-worker-runtime.mjs",
  ]) {
    await copyFile(path.resolve("app-server/src", name), path.join(targetRoot, name));
  }
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
  const roleStatus = deferred();
  const roleTurnRelease = deferred();
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
        const role = typeof params.developerInstructions === "string";
        const threadId = role ? "thread-role" : "thread-shell";
        if (role) this.notificationHandler({
          method: "thread/started",
          params: { thread: { id: threadId } },
        });
        return { thread: { id: threadId } };
      }
      if (method === "turn/start") {
        assert.equal(params.threadId, "thread-role");
        setImmediate(async () => {
          await roleTurnRelease.promise;
          try {
            roleStatus.resolve(await this.serverRequestHandler({
              id: 91,
              method: "item/tool/call",
              params: {
                threadId: "thread-role",
                turnId: "turn-role",
                callId: "call-role-status",
                namespace: "environment",
                tool: "status",
                arguments: {},
              },
            }));
          } catch (error) {
            roleStatus.resolve({ error });
          }
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
    roleBindingsPath: path.join(root, "bindings.json"),
    workerRequestTimeoutMs: 2_000,
    workerDispatchTimeoutMs: 2_000,
  });
  t.after(() => bootstrap.close({ abandonActiveWork: true }));
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
    input: [{ type: "text", text: ":we attach strategic-planner:main", text_elements: [] }],
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
  const deliveredCompletion = await waitForSyntheticCompletion();
  assert.equal(Number.isInteger(deliveredCompletion.params.turn.startedAt), true);
  assert.equal(Number.isInteger(deliveredCompletion.params.turn.completedAt), true);
  assert.deepEqual(forwardedNotifications.map((item) => item.method), [
    "turn/started",
    "turn/completed",
  ]);
  assert.equal(forwardedNotifications.at(-1).params.turn.items[0].text,
    "role-owned response");
  const status = await roleStatus.promise;
  assert.equal(status.success, true, status.error?.stack);
  assert.equal(JSON.parse(status.contentItems[0].text).activeGeneration.generationId,
    bootstrap.manager.snapshot().activeGeneration.generationId);
  assert.equal(requests.filter((request) => request.method === "turn/start").length, 1);
  const roleThreadStart = requests.filter((request) => request.method === "thread/start")[1];
  assert.deepEqual(roleThreadStart.params.dynamicTools.map((tool) => tool.name), [
    "environment",
  ]);
  assert.equal(requests.find((request) => request.method === "turn/start").params.threadId,
    "thread-role");
  assert.deepEqual(bootstrap.manager.snapshot().admissions, []);
});
