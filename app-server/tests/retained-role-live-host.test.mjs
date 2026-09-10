import assert from "node:assert/strict";
import test from "node:test";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { InMemoryContextTransitionLeaseGate } from "../src/context-transition-lease.mjs";
import { openSqliteAppServerStateStore } from "../src/sqlite-app-server-state.mjs";

import {
  createRetainedRoleLiveHost,
  projectRuntimeManifest,
} from "../src/index.mjs";

function fixture() {
  const transitionGate = {
    beginPreparation() {},
    abortPreparation() {},
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
      abortPreparation() {},
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
      abortPreparation() {},
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

test("production notification schedules and retries preparation before active turn completes", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "live-host-active-"));
  let store;
  try {
    store = await openSqliteAppServerStateStore({filePath:path.join(root,"state.sqlite")});
    const gate = new InMemoryContextTransitionLeaseGate();
    let notify; let closes = 0;
    const adapter = { transitionGate:gate,
      onNotification(handler){notify=handler;return()=>{};},
      deliverTurn:async ({role})=>({logicalRoleInstanceId:role.logicalRoleInstanceId,
        threadId:"thread-active",turnId:"turn-active",binding:{bindingRevision:1}}),
      waitForTurnCompletion:()=>new Promise(()=>{}),
    };
    const manifest = projectRuntimeManifest({schema_version:1,manifest_id:"active.vertical",roles:{probe:{
      contract:"skills/probe/SKILL.md",developer_instructions:"probe",thread_options:{cwd:".",approval_policy:"never",sandbox:"read-only"},skills:[{name:"probe",path:"skills/probe/SKILL.md"}]}}},
      {baseDirectory:"/tmp/live-host-active"});
    const host=createRetainedRoleLiveHost({adapter,manifest,episodeStore:store,transitionGate:gate,
      inputCustody:{closeAdmission(){closes+=1;if(closes===1)throw new Error("abort first preparation");},abortPreparation(){},admission(){},releaseAfterReconciliation(){}},
      pressureProfile:{schemaVersion:1,usageField:"last.totalTokens",windowField:"modelContextWindow",rounding:"floor",saturation:"clamp_10000"},
      pressurePolicyForRole:async()=>({schemaVersion:1,unit:"basis_points",approaching:{enter:5000,exit:4500},replacementCandidate:{enter:7000,exit:6500},critical:{enter:9000,exit:8500}}),
      inferenceRuntimeForRole:async()=>({}),checkpointPublisherForRole:async()=>({}),projectionForPreparation:async()=>({})});
    host.runtime.startTurn({roleId:"probe",instanceId:"main",clientUserMessageId:"message-1",text:"domain still active"});
    await new Promise(setImmediate);
    const usage={inputTokens:950,cachedInputTokens:0,cacheWriteInputTokens:0,outputTokens:0,reasoningOutputTokens:0,totalTokens:950};
    notify({method:"thread/tokenUsage/updated",params:{threadId:"thread-active",turnId:"turn-active",tokenUsage:{last:usage,total:usage,modelContextWindow:1000}}});
    for(let i=0;i<10&&closes<2;i++) await new Promise(setImmediate);
    assert.equal(closes,2);
    assert.equal(gate.snapshot("probe:main").phase,"preparing");
    host.close();
  } finally {store?.close();await rm(root,{recursive:true,force:true});}
});
