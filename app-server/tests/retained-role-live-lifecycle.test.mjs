import assert from "node:assert/strict";
import test from "node:test";

import {
  ContextLifecycleEvidenceCollector,
  ContextPressureController,
  RetainedRoleLiveLifecycleRuntime,
  TokenUsagePressureProjector,
} from "../src/index.mjs";

const pressureProfile = {
  schemaVersion: 1,
  usageField: "last.totalTokens",
  windowField: "modelContextWindow",
  rounding: "floor",
  saturation: "clamp_10000",
};

const pressurePolicy = {
  schemaVersion: 1,
  unit: "basis_points",
  approaching: { enter: 6_500, exit: 5_500 },
  replacementCandidate: { enter: 7_800, exit: 6_800 },
  critical: { enter: 9_000, exit: 8_000 },
};

function tokenUsageObservation() {
  const breakdown = {
    inputTokens: 80_000,
    cachedInputTokens: 0,
    cacheWriteInputTokens: 0,
    outputTokens: 0,
    reasoningOutputTokens: 0,
    totalTokens: 80_000,
  };
  return {
    schemaVersion: 1,
    observationType: "token_usage",
    source: {
      provider: "codex",
      transport: "app-server",
      protocolVersion: "0.149.1",
      method: "thread/tokenUsage/updated",
    },
    threadId: "thread-1",
    turnId: "turn-1",
    details: { last: breakdown, total: breakdown, modelContextWindow: 100_000 },
  };
}

function fixture({ tokenUsage = true, pressureStatus = "projected", lifecycleError = null } = {}) {
  const calls = [];
  const delivery = Object.freeze({
    logicalRoleInstanceId: "strategic-planner:main",
    threadId: "thread-1",
    turnId: "turn-1",
    replayedDelivery: false,
    binding: Object.freeze({ bindingRevision: 2 }),
    roleProjection: Object.freeze({
      role: Object.freeze({ logicalRoleInstanceId: "strategic-planner:main" }),
      skills: Object.freeze([{ name: "strategic-planner" }]),
    }),
  });
  const completion = Object.freeze({ status: "completed", outputText: "Done." });
  const lifecycleSnapshot = Object.freeze({
    latestTokenUsage: tokenUsage ? Object.freeze({ turnId: delivery.turnId }) : null,
  });
  const roleRuntime = {
    adapter: {
      async waitForTurnCompletion(input) {
        calls.push(["waitForTurnCompletion", input]);
        return completion;
      },
    },
    async deliverTurn(turn) {
      calls.push(["deliverTurn", turn]);
      return delivery;
    },
  };
  const lifecycleEvidence = {
    snapshot(threadId) {
      calls.push(["snapshot", threadId]);
      return lifecycleSnapshot;
    },
  };
  const pressureProjector = {
    project(snapshot) {
      calls.push(["project", snapshot]);
      return pressureStatus === "projected" ? {
        status: "projected",
        observation: { sequence: 8 },
      } : { status: pressureStatus, reason: "unavailable" };
    },
  };
  const pressureControllerForRole = async (roleId) => {
    calls.push(["pressureControllerForRole", roleId]);
    return {
      observe(observation) {
        calls.push(["observe", observation]);
        return { disposition: "replacement_candidate" };
      },
    };
  };
  const coordinatorForRole = async (roleId) => {
    calls.push(["coordinatorForRole", roleId]);
    return {
      async run(input) {
        calls.push(["run", input]);
        if (lifecycleError) throw lifecycleError;
        return { status: "reconciled" };
      },
    };
  };
  return {
    calls,
    runtime: new RetainedRoleLiveLifecycleRuntime({
      roleRuntime,
      lifecycleEvidence,
      pressureProjector,
      pressureControllerForRole,
      coordinatorForRole,
    }),
  };
}

test("retained role live runtime pauses completion through lifecycle reconciliation", async () => {
  const { runtime, calls } = fixture();
  const started = await runtime.startTurn({
    roleId: "strategic-planner",
    instanceId: "main",
    text: "Continue.",
  });
  assert.equal(started.delivery.turnId, "turn-1");
  const outcome = await started.completion;
  assert.equal(outcome.completion.outputText, "Done.");
  assert.equal(outcome.lifecycle.status, "reconciled");
  assert.deepEqual(calls.map(([name]) => name), [
    "deliverTurn",
    "waitForTurnCompletion",
    "snapshot",
    "project",
    "pressureControllerForRole",
    "observe",
    "coordinatorForRole",
    "run",
  ]);
  const lifecycleInput = calls.find(([name]) => name === "run")[1];
  assert.equal(lifecycleInput.pressureDisposition, "replacement_candidate");
  assert.equal(lifecycleInput.projectionContext.completion.outputText, "Done.");
});

test("retained role live runtime does not transition without completed-turn usage", async () => {
  const { runtime, calls } = fixture({ tokenUsage: false });
  const result = await runtime.deliverTurn({ text: "Continue." });
  assert.deepEqual(result.lifecycle, {
    status: "not_observed",
    reason: "completed_turn_token_usage_unavailable",
  });
  assert.equal(calls.some(([name]) => name === "coordinatorForRole"), false);
});

test("completed domain output survives a post-turn lifecycle failure", async () => {
  const error = Object.assign(new Error("identity control was not sterile"), {
    code: "non_sterile_identity_attestation",
  });
  const { runtime } = fixture({ lifecycleError: error });
  const result = await runtime.deliverTurn({ text: "Continue." });
  assert.equal(result.completion.outputText, "Done.");
  assert.deepEqual(result.lifecycle, {
    status: "failed",
    reason: "post_turn_lifecycle_failed",
    error: {
      name: "Error",
      message: "identity control was not sterile",
      code: "non_sterile_identity_attestation",
    },
  });
});

test("active-turn pressure observation is replayed byte-identically at completion", async () => {
  let observedAt = "2026-09-10T20:00:00.000Z";
  let finishTurn;
  const completion = new Promise((resolve) => { finishTurn = resolve; });
  const lifecycleEvidence = new ContextLifecycleEvidenceCollector({ now: () => observedAt });
  const recorded = lifecycleEvidence.record(tokenUsageObservation());
  const controller = new ContextPressureController({ policy: pressurePolicy });
  const pressureObservations = [];
  const runtime = new RetainedRoleLiveLifecycleRuntime({
    roleRuntime: {
      async deliverTurn() {
        return {
          logicalRoleInstanceId: "strategic-planner:main",
          threadId: "thread-1",
          turnId: "turn-1",
          replayedDelivery: false,
          binding: { bindingRevision: 1 },
          roleProjection: { role: {}, skills: [] },
        };
      },
      adapter: { waitForTurnCompletion: async () => completion },
    },
    lifecycleEvidence,
    pressureProjector: new TokenUsagePressureProjector({ profile: pressureProfile }),
    pressureControllerForRole: async () => ({
      observe(value) {
        pressureObservations.push(value);
        return controller.observe(value);
      },
    }),
    coordinatorForRole: async () => ({ run: async () => ({ status: "reconciled" }) }),
    activeTurnScheduler: { observe: async () => ({ status: "scheduled" }) },
  });

  const started = await runtime.startTurn({ text: "Continue." });
  await runtime.observeLifecycleObservation(recorded);
  observedAt = "2026-09-10T20:05:00.000Z";
  finishTurn({ status: "completed", outputText: "Done." });
  await started.completion;

  assert.equal(controller.snapshot().lastObservation.observedAt, "2026-09-10T20:00:00.000Z");
  assert.equal(pressureObservations.length, 2);
  assert.deepEqual(pressureObservations[1], pressureObservations[0]);
  assert.equal(controller.observe(structuredClone(pressureObservations[0])).status, "replayed");
});
