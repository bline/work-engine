import assert from "node:assert/strict";
import test from "node:test";

import {
  loadSemanticContextRuntimeProfile,
  projectSemanticContextRuntimeProfile,
} from "../src/index.mjs";

test("local semantic shadow profile is closed, revision-bound, and non-publishing", async () => {
  const profile = await loadSemanticContextRuntimeProfile(
    new URL("../semantic-context-profile.yaml", import.meta.url).pathname,
  );

  assert.equal(profile.profileId, "work-engine.local-shadow");
  assert.match(profile.source.sha256, /^[a-f0-9]{64}$/);
  assert.equal(profile.pressurePolicy.approaching.enter, 6_500);
  assert.equal(profile.pressurePolicy.replacementCandidate.enter, 7_500);
  assert.equal(profile.pressurePolicy.critical.enter, 9_000);
  assert.deepEqual(profile.shadowSchedule.inspectAt, ["replacement_candidate", "critical"]);
  assert.equal(profile.shadowSchedule.publishAcceptedCheckpoint, false);
  assert.equal(Object.isFrozen(profile), true);
});

test("local semantic shadow profile rejects publication and unknown fields", () => {
  const base = {
    schema_version: 1,
    profile_id: "test.shadow",
    pressure_profile: {
      usage_field: "last.totalTokens",
      window_field: "modelContextWindow",
      rounding: "floor",
      saturation: "clamp_10000",
    },
    pressure_policy: {
      unit: "basis_points",
      approaching: { enter: 100, exit: 50 },
      replacement_candidate: { enter: 200, exit: 150 },
      critical: { enter: 300, exit: 250 },
    },
    shadow_schedule: {
      inspect_at: ["replacement_candidate"],
      publish_accepted_checkpoint: true,
    },
  };
  assert.throws(
    () => projectSemanticContextRuntimeProfile(base, { sha256: "a".repeat(64) }),
    /cannot publish checkpoints/,
  );
  assert.throws(
    () => projectSemanticContextRuntimeProfile(
      { ...base, unsupported: true },
      { sha256: "a".repeat(64) },
    ),
    /unsupported fields: unsupported/,
  );
});
