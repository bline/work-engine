import assert from "node:assert/strict";
import test from "node:test";

import {
  ContextPressureController,
  validateContextPressurePolicy,
} from "../src/index.mjs";

function policy(overrides = {}) {
  return {
    schemaVersion: 1,
    unit: "basis_points",
    approaching: { enter: 6_500, exit: 5_500 },
    replacementCandidate: { enter: 7_800, exit: 6_800 },
    critical: { enter: 9_000, exit: 8_000 },
    ...overrides,
  };
}

function observation(index, pressureBasisPoints) {
  return {
    schemaVersion: 1,
    observationId: `observation-${index}`,
    sequence: index,
    observedAt: `2026-08-25T22:${String(index).padStart(2, "0")}:00.000Z`,
    pressureBasisPoints,
    source: "test-host-projection",
    sourceRevision: `sha256:${"a".repeat(64)}`,
  };
}

test("pressure policy is explicit, revision-bound, and strictly validated", () => {
  const first = validateContextPressurePolicy(policy());
  const second = validateContextPressurePolicy(policy());
  assert.equal(first.policyRevision, second.policyRevision);
  assert.equal(Object.isFrozen(first), true);
  assert.throws(
    () => validateContextPressurePolicy(policy({ sampleEveryTurns: 1 })),
    /unsupported fields/,
  );
  assert.throws(
    () => validateContextPressurePolicy(policy({
      approaching: { enter: 6_500, exit: 6_500 },
    })),
    /exit must be lower/,
  );
  assert.throws(
    () => validateContextPressurePolicy(policy({
      replacementCandidate: { enter: 7_800, exit: 6_000 },
    })),
    /thresholds must be ordered/,
  );
});

test("controller advances through configured pressure dispositions", () => {
  const controller = new ContextPressureController({ policy: policy() });
  const comfortable = controller.observe(observation(1, 6_000));
  assert.equal(comfortable.disposition, "comfortable");
  assert.equal(comfortable.changed, false);

  const approaching = controller.observe(observation(2, 6_500));
  assert.equal(approaching.disposition, "approaching");
  assert.equal(approaching.changed, true);

  const candidate = controller.observe(observation(3, 7_800));
  assert.equal(candidate.disposition, "replacement_candidate");

  const critical = controller.observe(observation(4, 9_000));
  assert.equal(critical.disposition, "critical");
  assert.equal(controller.snapshot().disposition, "critical");
});

test("hysteresis holds each disposition until its configured exit band", () => {
  const controller = new ContextPressureController({ policy: policy() });
  assert.equal(controller.observe(observation(1, 9_500)).disposition, "critical");

  const criticalHeld = controller.observe(observation(2, 8_500));
  assert.equal(criticalHeld.disposition, "critical");
  assert.equal(criticalHeld.reason, "hysteresis_held");

  assert.equal(
    controller.observe(observation(3, 7_500)).disposition,
    "replacement_candidate",
  );
  const candidateHeld = controller.observe(observation(4, 7_000));
  assert.equal(candidateHeld.disposition, "replacement_candidate");
  assert.equal(candidateHeld.reason, "hysteresis_held");

  assert.equal(controller.observe(observation(5, 6_500)).disposition, "approaching");
  const approachingHeld = controller.observe(observation(6, 6_000));
  assert.equal(approachingHeld.disposition, "approaching");
  assert.equal(approachingHeld.reason, "hysteresis_held");
  assert.equal(controller.observe(observation(7, 5_000)).disposition, "comfortable");
});

test("high pressure can skip stages without granting lifecycle authority", () => {
  const controller = new ContextPressureController({ policy: policy() });
  const result = controller.observe(observation(1, 9_400));
  assert.equal(result.previousDisposition, "comfortable");
  assert.equal(result.disposition, "critical");
  assert.deepEqual(Object.keys(controller.snapshot()).sort(), [
    "disposition",
    "lastObservation",
    "lastTransition",
    "minimumSequence",
    "policyRevision",
    "schemaVersion",
  ]);
});

test("exact observation replay is idempotent and conflicting reuse fails", () => {
  const controller = new ContextPressureController({ policy: policy() });
  const source = observation(1, 7_900);
  const first = controller.observe(source);
  const replay = controller.observe(structuredClone(source));
  assert.equal(replay.status, "replayed");
  assert.equal(replay.disposition, first.disposition);
  assert.throws(
    () => controller.observe({ ...source, pressureBasisPoints: 9_500 }),
    /reused for different evidence/,
  );
  assert.throws(
    () => controller.observe({
      ...observation(2, 8_000),
      observationId: "stale-observation",
      sequence: 1,
    }),
    /increase monotonically/,
  );
});
