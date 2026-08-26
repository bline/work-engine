import assert from "node:assert/strict";
import test from "node:test";

import { RetainedRoleLiveLifecycleRuntime } from "../src/index.mjs";

function fixture({ tokenUsage = true, pressureStatus = "projected" } = {}) {
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
