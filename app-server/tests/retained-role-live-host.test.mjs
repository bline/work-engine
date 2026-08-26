import assert from "node:assert/strict";
import test from "node:test";

import {
  createRetainedRoleLiveHost,
  projectRuntimeManifest,
} from "../src/index.mjs";

function fixture() {
  const transitionGate = {
    beginPreparation() {},
    acquire() {},
    runTurnAdmission() {},
    admitToolEffect() {},
  };
  let detached = false;
  const adapter = {
    transitionGate,
    deliverTurn() {},
    waitForTurnCompletion() {},
    onNotification() { return () => { detached = true; }; },
  };
  const manifest = projectRuntimeManifest({
    schema_version: 1,
    manifest_id: "live-host.test",
    roles: {
      probe: {
        contract: "skills/probe/SKILL.md",
        developer_instructions: "Do not perform domain work; this role only exercises host construction.",
        thread_options: { cwd: ".", approval_policy: "never", sandbox: "read-only" },
        skills: [{ name: "probe", path: "skills/probe/SKILL.md" }],
      },
    },
  }, { baseDirectory: "/tmp/live-host-test" });
  const episodeStore = { receipts: () => [] };
  const inferenceRuntime = { inspect() {} };
  const checkpointPublisher = { publish() {} };
  const host = createRetainedRoleLiveHost({
    adapter,
    manifest,
    episodeStore,
    transitionGate,
    inputCustody: {
      closeAdmission() {},
      admission() {},
      releaseAfterReconciliation() {},
    },
    pressureProfile: {
      schemaVersion: 1,
      usageField: "last.totalTokens",
      windowField: "modelContextWindow",
      rounding: "floor",
      saturation: "clamp_10000",
    },
    pressurePolicyForRole: async () => ({
      schemaVersion: 1,
      unit: "basis_points",
      approaching: { enter: 5_000, exit: 4_500 },
      replacementCandidate: { enter: 7_000, exit: 6_500 },
      critical: { enter: 9_000, exit: 8_500 },
    }),
    inferenceRuntimeForRole: async () => inferenceRuntime,
    checkpointPublisherForRole: async () => checkpointPublisher,
    projectionForPreparation: async () => ({ projection: {}, sourceMaterials: [] }),
  });
  return { host, adapter, transitionGate, detached: () => detached };
}

test("retained role live host shares one gate and caches role lifecycle components", async () => {
  const { host, detached } = fixture();
  assert.equal(host.sequenceFloor, 0);
  assert.equal(
    await host.pressureControllerForRole("probe:main"),
    await host.pressureControllerForRole("probe:main"),
  );
  assert.equal(
    await host.coordinatorForRole("probe:main"),
    await host.coordinatorForRole("probe:main"),
  );
  host.close();
  assert.equal(detached(), true);
});

test("retained role live host refuses a gate not installed on its adapter", () => {
  const { host, adapter } = fixture();
  host.close();
  assert.throws(() => createRetainedRoleLiveHost({
    adapter,
    manifest: host.runtime.roleRuntime.manifest,
    episodeStore: { receipts: () => [] },
    transitionGate: {
      beginPreparation() {},
      acquire() {},
      runTurnAdmission() {},
    },
    inputCustody: { closeAdmission() {} },
    pressureProfile: {},
    pressurePolicyForRole() {},
    inferenceRuntimeForRole() {},
    checkpointPublisherForRole() {},
    projectionForPreparation() {},
  }), /share one transition gate/);
});
