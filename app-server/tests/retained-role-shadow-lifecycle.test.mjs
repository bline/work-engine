import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  CodexAppServerAdapter,
  ExactSkillResolver,
  FileRoleBindingRegistry,
  RetainedRoleShadowLifecycleRuntime,
  TokenUsagePressureProjector,
  createRetainedRoleShadowHost,
  openSqliteAppServerStateStore,
  projectRuntimeManifest,
  validateTokenUsagePressureProfile,
} from "../src/index.mjs";

const profile = () => ({
  schemaVersion: 1,
  usageField: "last.totalTokens",
  windowField: "modelContextWindow",
  rounding: "floor",
  saturation: "clamp_10000",
});

function snapshot({ turnId = "turn-1", activeTokens = 25_000, window = 100_000 } = {}) {
  return {
    schemaVersion: 1,
    threadId: "thread-1",
    latestTokenUsage: {
      schemaVersion: 1,
      observationType: "token_usage",
      sequence: 7,
      source: { provider: "codex", transport: "app-server", protocolVersion: "0.149.1", method: "thread/tokenUsage/updated" },
      threadId: "thread-1",
      turnId,
      details: {
        last: { inputTokens: activeTokens, cachedInputTokens: 0, cacheWriteInputTokens: 0, outputTokens: 0, reasoningOutputTokens: 0, totalTokens: activeTokens },
        total: { inputTokens: activeTokens, cachedInputTokens: 0, cacheWriteInputTokens: 0, outputTokens: 0, reasoningOutputTokens: 0, totalTokens: activeTokens },
        modelContextWindow: window,
      },
    },
  };
}

test("token usage pressure profile is explicit and projects scheduling basis points", () => {
  const validated = validateTokenUsagePressureProfile(profile());
  assert.match(validated.profileRevision, /^sha256:[a-f0-9]{64}$/);
  const projector = new TokenUsagePressureProjector({ profile: profile(), now: () => "2026-08-25T12:00:00Z" });
  const result = projector.project(snapshot());
  assert.equal(result.status, "projected");
  assert.equal(result.observation.pressureBasisPoints, 2_500);
  assert.equal(result.measuredTokens, 25_000);
  assert.match(result.observation.sourceRevision, /^sha256:[a-f0-9]{64}$/);
  assert.throws(() => validateTokenUsagePressureProfile({ ...profile(), usageField: "total.totalTokens" }), /unsupported/);
  assert.equal(projector.project({ ...snapshot(), latestTokenUsage: null }).reason, "token_usage_missing");
  assert.equal(projector.project(snapshot({ window: null })).reason, "context_window_missing");
});

test("retained manifest role records one shadow observation after turn completion", async () => {
  const calls = [];
  const delivery = {
    logicalRoleInstanceId: "strategic-planner:main",
    threadId: "thread-1",
    turnId: "turn-1",
    replayedDelivery: false,
    binding: { bindingRevision: 3 },
    roleProjection: { roleId: "strategic-planner" },
  };
  const roleRuntime = {
    deliverTurn: async () => delivery,
    adapter: { waitForTurnCompletion: async () => ({ status: "completed", outputText: "done" }) },
  };
  const coordinator = { observe: async (input) => { calls.push(input); return { status: "recorded", episode: { episodeId: input.episodeId } }; } };
  const runtime = new RetainedRoleShadowLifecycleRuntime({
    roleRuntime,
    lifecycleEvidence: { snapshot: () => snapshot() },
    pressureProjector: new TokenUsagePressureProjector({ profile: profile(), now: () => "2026-08-25T12:00:00Z" }),
    coordinatorForRole: async (identity) => {
      assert.equal(identity, "strategic-planner:main");
      return coordinator;
    },
  });
  const result = await runtime.deliverTurn({ roleId: "strategic-planner", instanceId: "main", text: "Plan", clientUserMessageId: "message-1" });
  assert.equal(result.shadow.status, "recorded");
  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0].subject, { logicalRoleInstanceId: "strategic-planner:main", threadId: "thread-1", bindingRevision: 3 });
  assert.equal(calls[0].projection, null);
  assert.equal(calls[0].contextTelemetry.retainedInputTokens, 25_000);
  assert.equal(calls[0].contextTelemetry.reportedContextWindowTokens, 100_000);
});

test("retained role never applies stale token usage to a completed turn", async () => {
  let observed = false;
  const runtime = new RetainedRoleShadowLifecycleRuntime({
    roleRuntime: {
      deliverTurn: async () => ({ logicalRoleInstanceId: "role:one", threadId: "thread-1", turnId: "turn-2", replayedDelivery: false, binding: { bindingRevision: 1 } }),
      adapter: { waitForTurnCompletion: async () => ({ status: "completed" }) },
    },
    lifecycleEvidence: { snapshot: () => snapshot({ turnId: "turn-1" }) },
    pressureProjector: new TokenUsagePressureProjector({ profile: profile() }),
    coordinatorForRole: async () => ({ observe: async () => { observed = true; } }),
  });
  const result = await runtime.deliverTurn({});
  assert.equal(result.shadow.reason, "completed_turn_token_usage_unavailable");
  assert.equal(observed, false);
});

class LifecycleTransport {
  constructor() {
    this.notificationHandlers = new Set();
  }

  onServerRequest() {}
  onClosed() {}
  notify() {}
  onNotification(handler) {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }

  emit(notification) {
    for (const handler of this.notificationHandlers) handler(notification);
  }

  async request(method, params) {
    if (method === "initialize") {
      return {
        userAgent: "work-engine/0.149.1 (Linux; x86_64)",
        codexHome: "/tmp/codex",
        platformFamily: "unix",
        platformOs: "linux",
      };
    }
    if (method === "thread/start") return { thread: { id: "durable-thread-1" } };
    if (method === "turn/start") {
      queueMicrotask(() => {
        this.emit({
          method: "thread/tokenUsage/updated",
          params: {
            threadId: params.threadId,
            turnId: "durable-turn-1",
            tokenUsage: {
              last: { inputTokens: 10_000, cachedInputTokens: 1_000, cacheWriteInputTokens: 0, outputTokens: 1_000, reasoningOutputTokens: 0, totalTokens: 12_000 },
              total: { inputTokens: 10_000, cachedInputTokens: 1_000, cacheWriteInputTokens: 0, outputTokens: 1_000, reasoningOutputTokens: 0, totalTokens: 12_000 },
              modelContextWindow: 100_000,
            },
          },
        });
        this.emit({
          method: "turn/completed",
          params: {
            threadId: params.threadId,
            turn: {
              id: "durable-turn-1",
              status: "completed",
              items: [{ type: "agentMessage", phase: "final_answer", text: "complete" }],
            },
          },
        });
      });
      return { turn: { id: "durable-turn-1" } };
    }
    throw new Error(`unexpected request ${method}`);
  }
}

test("manifest delivery persists a real comfortable shadow episode across SQLite restart", async (t) => {
  const directory = await mkdtemp(path.join(tmpdir(), "retained-role-shadow-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const skillDirectory = path.join(directory, "skills", "probe");
  await mkdir(skillDirectory, { recursive: true });
  const skillPath = path.join(skillDirectory, "SKILL.md");
  await writeFile(skillPath, "# Probe fixture\n", "utf8");
  const manifest = projectRuntimeManifest({
    schema_version: 1,
    manifest_id: "shadow.integration",
    roles: {
      probe: {
        contract: "skills/probe/SKILL.md",
        developer_instructions: "Run the bounded fixture.",
        thread_options: { cwd: ".", approval_policy: "never", sandbox: "read-only" },
        skills: [{ name: "probe", path: "skills/probe/SKILL.md" }],
      },
    },
  }, { baseDirectory: directory });
  const transport = new LifecycleTransport();
  const adapter = new CodexAppServerAdapter({
    transport,
    registry: new FileRoleBindingRegistry(path.join(directory, "bindings.json")),
    skillResolver: await ExactSkillResolver.create([path.join(directory, "skills")]),
  });
  await adapter.initialize();
  const databasePath = path.join(directory, "state", "app-server.sqlite3");
  let store = await openSqliteAppServerStateStore({ filePath: databasePath });
  const pressurePolicy = () => ({
    schemaVersion: 1,
    unit: "basis_points",
    approaching: { enter: 5_000, exit: 4_500 },
    replacementCandidate: { enter: 7_000, exit: 6_500 },
    critical: { enter: 9_000, exit: 8_500 },
  });
  const shadowSchedule = () => ({
    schemaVersion: 1,
    inspectAt: ["replacement_candidate", "critical"],
    publishAcceptedCheckpoint: false,
  });
  const inferenceRuntime = async () => ({
    inspect: async () => { throw new Error("comfortable pressure must not inspect"); },
  });
  const host = createRetainedRoleShadowHost({
    adapter,
    manifest,
    episodeStore: store,
    pressureProfile: profile(),
    pressurePolicyForRole: async () => pressurePolicy(),
    scheduleForRole: async () => shadowSchedule(),
    inferenceRuntimeForRole: inferenceRuntime,
    now: () => "2026-08-25T12:00:00Z",
  });
  t.after(host.close);

  const outcome = await host.runtime.deliverTurn({
    roleId: "probe",
    instanceId: "dev",
    clientUserMessageId: "durable-message-1",
    text: "Exercise the retained role.",
  });
  assert.equal(outcome.completion.outputText, "complete");
  assert.equal(outcome.shadow.status, "recorded");
  assert.equal(outcome.shadow.episode.pressure.pressureBasisPoints, 1_200);
  assert.equal(outcome.shadow.episode.pressure.disposition, "comfortable");
  assert.equal(host.sequenceFloor, 0);
  assert.equal(host.recoveryForRole("probe:dev").status, "empty");
  assert.deepEqual(outcome.shadow.episode.transition, {
    status: "not_requested",
    retirementAttempted: false,
  });
  const episodeId = outcome.shadow.episode.episodeId;
  store.close();

  store = await openSqliteAppServerStateStore({ filePath: databasePath });
  t.after(() => store.close());
  const recovered = store.get(episodeId);
  assert.equal(recovered.episodeRevision, outcome.shadow.episode.episodeRevision);
  assert.equal(store.integrityCheck()[0], "ok");

  const recoveryAdapter = {
    deliverTurn: async () => { throw new Error("restart recovery must not deliver a turn"); },
    waitForTurnCompletion: async () => { throw new Error("restart recovery must not wait"); },
    onNotification: () => () => {},
  };
  const restartedHost = createRetainedRoleShadowHost({
    adapter: recoveryAdapter,
    manifest,
    episodeStore: store,
    pressureProfile: profile(),
    pressurePolicyForRole: async () => pressurePolicy(),
    scheduleForRole: async () => shadowSchedule(),
    inferenceRuntimeForRole: inferenceRuntime,
    now: () => "2026-08-25T12:01:00Z",
  });
  t.after(restartedHost.close);
  await restartedHost.coordinatorForRole("probe:dev");
  assert.equal(restartedHost.sequenceFloor, 1);
  assert.equal(restartedHost.recoveryForRole("probe:dev").status, "restored");
  assert.equal(restartedHost.recoveryForRole("probe:dev").minimumSequence, 1);
});
