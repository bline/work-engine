import assert from "node:assert/strict";
import test from "node:test";

import {
  ContextLifecycleEvidenceCollector,
  InMemoryContextLifecycleEpisodeStore,
  createContextLifecycleEpisode,
  lifecyclePressureSequenceFloor,
  restoreContextPressureController,
  validateContextPressurePolicy,
} from "../src/index.mjs";

const policy = () => ({
  schemaVersion: 1,
  unit: "basis_points",
  approaching: { enter: 5_000, exit: 4_500 },
  replacementCandidate: { enter: 7_000, exit: 6_500 },
  critical: { enter: 9_000, exit: 8_500 },
});

function episode({ sequence = 7, disposition = "replacement_candidate", policyRevision = validateContextPressurePolicy(policy()).policyRevision } = {}) {
  return createContextLifecycleEpisode({
    schemaVersion: 1,
    type: "context-lifecycle-shadow-episode",
    episodeId: `episode-${sequence}`,
    requestRevision: `sha256:${"a".repeat(64)}`,
    scheduleRevision: `sha256:${"b".repeat(64)}`,
    mode: "shadow",
    subject: { logicalRoleInstanceId: "probe:dev", threadId: "thread-1", bindingRevision: 1 },
    startedAt: "2026-08-25T12:00:00Z",
    completedAt: "2026-08-25T12:00:01Z",
    pressure: {
      policyRevision,
      observationId: `observation-${sequence}`,
      observationSequence: sequence,
      pressureBasisPoints: 7_500,
      previousDisposition: "comfortable",
      disposition,
      transitionReason: "threshold_crossed",
      measurementSource: "fixture",
      measurementSourceRevision: `sha256:${"c".repeat(64)}`,
    },
    sourceRevision: null,
    inference: { status: "not_scheduled", candidateRevision: null, verificationRevision: null, compiler: null, verifier: null, blockerCount: 0, uncertaintyCount: 0 },
    checkpoint: { status: "not_attempted", checkpointRevision: null, ledgerRevision: null, reason: "inspection_not_scheduled" },
    measurements: {
      context: { retainedInputTokens: 75_000, reportedContextWindowTokens: 100_000, growthSincePreviousTokens: null, estimatedAvoidedInputTokens: null, estimateMethod: null, estimateRevision: null },
      compiler: { durationMs: null, inputTokens: null, cachedInputTokens: null, outputTokens: null, costMicrounits: null },
      verifier: { durationMs: null, inputTokens: null, cachedInputTokens: null, outputTokens: null, costMicrounits: null },
      inspectionDurationMs: null,
      publicationDurationMs: null,
      checkpointBytes: null,
    },
    transition: { status: "not_requested", retirementAttempted: false },
    failure: null,
  });
}

test("pressure recovery preserves hysteresis and rejects sequences below the durable floor", () => {
  const store = new InMemoryContextLifecycleEpisodeStore();
  store.append(episode({}));
  const recovery = restoreContextPressureController({
    policy: policy(),
    episodeStore: store,
    logicalRoleInstanceId: "probe:dev",
  });
  assert.equal(recovery.status, "restored");
  assert.equal(recovery.minimumSequence, 7);
  assert.equal(recovery.initialDisposition, "replacement_candidate");
  assert.throws(() => recovery.controller.observe({
    schemaVersion: 1,
    observationId: "stale",
    sequence: 7,
    observedAt: "2026-08-25T12:01:00Z",
    pressureBasisPoints: 6_800,
    source: "fixture",
    sourceRevision: `sha256:${"d".repeat(64)}`,
  }), /recovered floor/);
  const held = recovery.controller.observe({
    schemaVersion: 1,
    observationId: "next",
    sequence: 8,
    observedAt: "2026-08-25T12:01:00Z",
    pressureBasisPoints: 6_800,
    source: "fixture",
    sourceRevision: `sha256:${"d".repeat(64)}`,
  });
  assert.equal(held.disposition, "replacement_candidate");
  assert.equal(held.reason, "hysteresis_held");
});

test("collector resumes above the global durable pressure sequence", () => {
  const store = new InMemoryContextLifecycleEpisodeStore();
  store.append(episode({ sequence: 11 }));
  const floor = lifecyclePressureSequenceFloor(store);
  const collector = new ContextLifecycleEvidenceCollector({ initialSequence: floor });
  const recorded = collector.record({
    schemaVersion: 1,
    observationType: "token_usage",
    source: { provider: "codex", transport: "app-server", protocolVersion: "0.149.1", method: "thread/tokenUsage/updated" },
    threadId: "thread-1",
    turnId: "turn-2",
    details: {
      last: { inputTokens: 1, cachedInputTokens: 0, cacheWriteInputTokens: 0, outputTokens: 0, reasoningOutputTokens: 0, totalTokens: 1 },
      total: { inputTokens: 1, cachedInputTokens: 0, cacheWriteInputTokens: 0, outputTokens: 0, reasoningOutputTokens: 0, totalTokens: 1 },
      modelContextWindow: 100_000,
    },
  });
  assert.equal(recorded.sequence, 12);
  assert.equal(collector.snapshot("thread-1").retention.droppedThroughSequence, 11);
});

test("a changed pressure policy resets disposition but preserves the sequence floor", () => {
  const store = new InMemoryContextLifecycleEpisodeStore();
  store.append(episode({}));
  const changed = policy();
  changed.critical = { enter: 9_500, exit: 9_000 };
  const recovery = restoreContextPressureController({
    policy: changed,
    episodeStore: store,
    logicalRoleInstanceId: "probe:dev",
  });
  assert.equal(recovery.status, "policy_changed");
  assert.equal(recovery.initialDisposition, "comfortable");
  assert.equal(recovery.minimumSequence, 7);
});
