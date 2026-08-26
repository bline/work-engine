import assert from "node:assert/strict";
import { createHash, generateKeyPairSync } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  CodexAppServerAdapter,
  ContextCheckpointPublisher,
  ExactSkillResolver,
  FileRoleBindingRegistry,
  RetainedRoleShadowLifecycleRuntime,
  SemanticContextInferenceRuntime,
  TokenUsagePressureProjector,
  createRetainedRoleShadowHost,
  openSqliteAppServerStateStore,
  projectManifestRoleObservedContext,
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
  constructor({
    threadId = "durable-thread-1",
    turnId = "durable-turn-1",
    inputTokens = 10_000,
    totalTokens = 12_000,
  } = {}) {
    this.notificationHandlers = new Set();
    this.threadId = threadId;
    this.turnId = turnId;
    this.inputTokens = inputTokens;
    this.totalTokens = totalTokens;
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
    if (method === "thread/start") return { thread: { id: this.threadId } };
    if (method === "turn/start") {
      queueMicrotask(() => {
        this.emit({
          method: "thread/tokenUsage/updated",
          params: {
            threadId: params.threadId,
            turnId: this.turnId,
            tokenUsage: {
              last: { inputTokens: this.inputTokens, cachedInputTokens: 1_000, cacheWriteInputTokens: 0, outputTokens: this.totalTokens - this.inputTokens, reasoningOutputTokens: 0, totalTokens: this.totalTokens },
              total: { inputTokens: this.inputTokens, cachedInputTokens: 1_000, cacheWriteInputTokens: 0, outputTokens: this.totalTokens - this.inputTokens, reasoningOutputTokens: 0, totalTokens: this.totalTokens },
              modelContextWindow: 100_000,
            },
          },
        });
        this.emit({
          method: "turn/completed",
          params: {
            threadId: params.threadId,
            turn: {
              id: this.turnId,
              status: "completed",
              items: [{ type: "agentMessage", phase: "final_answer", text: "complete" }],
            },
          },
        });
      });
      return { turn: { id: this.turnId } };
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

test("replacement-candidate turn signs, verifies, and durably publishes in shadow mode", async (t) => {
  const directory = await mkdtemp(path.join(tmpdir(), "replacement-candidate-shadow-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const skillDirectory = path.join(directory, "skills", "strategic-planner");
  await mkdir(skillDirectory, { recursive: true });
  const skillPath = path.join(skillDirectory, "SKILL.md");
  const content = {
    human: "The user authorized the next bounded App Server slice.",
    evidence: "The observed-context projection and continuation schema tests pass.",
    skill: "The strategic planner is advisory and preserves canonical authority.",
    next: "Build the hidden semantic compiler and independent verifier against fixtures.",
  };
  await writeFile(skillPath, content.skill, "utf8");
  const sha = Object.fromEntries(Object.entries(content).map(([key, value]) => [
    key,
    createHash("sha256").update(value, "utf8").digest("hex"),
  ]));
  const renderFixture = async (name, replacements = {}) => {
    let value = await readFile(new URL(`./fixtures/semantic-context/${name}`, import.meta.url), "utf8");
    for (const [token, replacement] of Object.entries({
      HUMAN_SHA: sha.human,
      EVIDENCE_SHA: sha.evidence,
      SKILL_SHA: sha.skill,
      NEXT_SHA: sha.next,
      ...replacements,
    })) value = value.replaceAll(`__${token}__`, replacement);
    return value.replaceAll("skills/strategic-planner/SKILL.md", skillPath);
  };
  const manifest = projectRuntimeManifest({
    schema_version: 1,
    manifest_id: "replacement.shadow",
    roles: {
      "strategic-planner": {
        contract: "skills/strategic-planner/SKILL.md",
        developer_instructions: "Run the bounded fixture.",
        thread_options: { cwd: ".", approval_policy: "never", sandbox: "read-only" },
        skills: [{ name: "strategic-planner", path: "skills/strategic-planner/SKILL.md" }],
      },
    },
  }, { baseDirectory: directory });
  const transport = new LifecycleTransport({ inputTokens: 70_000, totalTokens: 75_000 });
  const adapter = new CodexAppServerAdapter({
    transport,
    registry: new FileRoleBindingRegistry(path.join(directory, "bindings.json")),
    skillResolver: await ExactSkillResolver.create([path.join(directory, "skills")]),
  });
  await adapter.initialize();
  const store = await openSqliteAppServerStateStore({
    filePath: path.join(directory, "state", "app-server.sqlite3"),
  });
  t.after(() => store.close());
  const keys = generateKeyPairSync("ed25519");
  const inferenceTimes = ["2026-08-25T13:00:00Z", "2026-08-25T13:00:01Z"];
  const compiler = {
    infer: async () => ({
      outputText: await renderFixture("compiler-valid.yaml"),
      provenance: { producer: "recorded-compiler", model: "fixture", version: "1", inferenceId: "compiler-1" },
    }),
  };
  const verifier = {
    infer: async (request) => ({
      outputText: await renderFixture("verifier-accepted.yaml", {
        SOURCE_SHA: request.input.sourceRevision.slice(7),
        CANDIDATE_SHA: request.input.candidate.candidateRevision.slice(7),
      }),
      provenance: { producer: "recorded-verifier", model: "fixture", version: "1", inferenceId: "verifier-1" },
    }),
  };
  const inferenceRuntime = new SemanticContextInferenceRuntime({
    compiler,
    verifier,
    resolvePublicKey: (keyId) => keyId === "shadow-key-1" ? keys.publicKey : null,
    now: () => inferenceTimes.shift(),
  });
  const authorityRevision = `sha256:${"e".repeat(64)}`;
  const authorityEvidence = `sha256:${"f".repeat(64)}`;
  const checkpointPublisher = new ContextCheckpointPublisher({
    store,
    resolvePublicKey: (keyId) => keyId === "shadow-key-1" ? keys.publicKey : null,
    revalidateAuthority: async ({ references }) => ({
      status: "current",
      authorityRevision,
      checkedReferences: references,
      evidenceRefs: [authorityEvidence],
    }),
    now: () => "2026-08-25T13:00:02Z",
  });
  const host = createRetainedRoleShadowHost({
    adapter,
    manifest,
    episodeStore: store,
    pressureProfile: profile(),
    pressurePolicyForRole: async () => ({
      schemaVersion: 1,
      unit: "basis_points",
      approaching: { enter: 5_000, exit: 4_500 },
      replacementCandidate: { enter: 7_000, exit: 6_500 },
      critical: { enter: 9_000, exit: 8_500 },
    }),
    scheduleForRole: async () => ({
      schemaVersion: 1,
      inspectAt: ["replacement_candidate", "critical"],
      publishAcceptedCheckpoint: true,
    }),
    inferenceRuntimeForRole: async () => inferenceRuntime,
    checkpointPublisherForRole: async () => checkpointPublisher,
    projectionForTurn: async ({ delivery, lifecycleSnapshot }) => {
      const assembled = await projectManifestRoleObservedContext({
        delivery,
        lifecycleSnapshot,
        visibleMaterials: [{
          identity: "user-message:proceed",
          origin: "human",
          trustClass: "human_authority_input",
          instructionApplicability: "contract_defined",
          contentRef: { kind: "thread-item", reference: "user-message:proceed" },
          content: content.human,
        }, {
          identity: "evidence:observed-context",
          origin: "tool",
          trustClass: "attributed_evidence",
          instructionApplicability: "none",
          producer: "app-server-test-gate",
          contentRef: { kind: "evidence", reference: "evidence:observed-context" },
          content: content.evidence,
        }],
        expectedNextWork: { reference: "expected-next-work:semantic-inference", content: content.next },
        sourceInventoryCompleteness: "partial",
        omissions: [{ scope: "provider-effective-prompt", reason: "provider does not expose exact effective input" }],
        signing: {
          componentId: "replacement-shadow-test",
          buildRevision: "test-1",
          keyId: "shadow-key-1",
          privateKey: keys.privateKey,
        },
      });
      store.initializeCheckpointFence({
        logicalRoleInstanceId: delivery.logicalRoleInstanceId,
        threadId: delivery.threadId,
        bindingRevision: delivery.binding.bindingRevision,
        sourceRevision: assembled.projection.sourceRevision,
        authorityRevision,
        publicationRevision: null,
        ledgerRevision: null,
      });
      return assembled;
    },
    now: () => "2026-08-25T13:00:03Z",
  });
  t.after(host.close);

  const outcome = await host.runtime.deliverTurn({
    roleId: "strategic-planner",
    instanceId: "main",
    clientUserMessageId: "candidate-message-1",
    text: content.human,
  });
  assert.equal(outcome.shadow.status, "recorded");
  assert.equal(outcome.shadow.episode.pressure.disposition, "replacement_candidate");
  assert.equal(outcome.shadow.episode.inference.status, "accepted");
  assert.equal(outcome.shadow.episode.checkpoint.status, "published");
  assert.deepEqual(outcome.shadow.episode.transition, { status: "not_requested", retirementAttempted: false });
  const durable = store.snapshot("strategic-planner:main");
  assert.equal(durable.publication.checkpointRevision, outcome.shadow.episode.checkpoint.checkpointRevision);
  assert.equal(durable.ledgerEntry.entryRevision, outcome.shadow.episode.checkpoint.ledgerRevision);
});
