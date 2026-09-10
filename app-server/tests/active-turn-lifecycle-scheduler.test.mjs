import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { ActiveTurnLifecycleScheduler } from "../src/active-turn-lifecycle-scheduler.mjs";
import { openSqliteAppServerStateStore } from "../src/sqlite-app-server-state.mjs";
import { InMemoryContextTransitionLeaseGate } from "../src/context-transition-lease.mjs";

const observation = {logicalRoleInstanceId:"slice-builder:one", bindingRevision:1,
  threadId:"thread-1", turnId:"turn-1", triggeringObservationId:"usage-1",
  disposition:"critical", retryLimit:1, observedAt:"2026-09-10T00:00:00.000Z"};

test("aborted preparation retries before domain completion and survives restart", async () => {
  const root=await mkdtemp(path.join(os.tmpdir(),"active-lifecycle-"));
  const file=path.join(root,"state.sqlite");
  try {
    let calls=0;
    let store=await openSqliteAppServerStateStore({filePath:file});
    const gate=new InMemoryContextTransitionLeaseGate();
    const scheduler=new ActiveTurnLifecycleScheduler({store, transitionGate:gate, reserve:async (permit)=>{
      assert.equal(permit.type,"work-engine.lifecycle-reserve-permit");
      calls+=1; if(calls===1) throw new Error("aborted"); return {preparationRevision:"prep-2"};
    }});
    const result=await scheduler.observe(observation);
    assert.equal(result.status,"reconciled");
    assert.equal(calls,2);
    store.close();
    store=await openSqliteAppServerStateStore({filePath:file});
    const replay=new ActiveTurnLifecycleScheduler({store,transitionGate:new InMemoryContextTransitionLeaseGate(),reserve:async()=>{throw new Error("must not replay");}});
    assert.deepEqual(await replay.recover(),[]);
    store.close();
  } finally {await rm(root,{recursive:true,force:true});}
});

test("restart recovers the exact scheduled active-turn obligation", async () => {
  const root=await mkdtemp(path.join(os.tmpdir(),"active-restart-"));
  const file=path.join(root,"state.sqlite");
  try {
    let store=await openSqliteAppServerStateStore({filePath:file});
    store.scheduleActiveTurnLifecycle(observation);
    store.close();
    store=await openSqliteAppServerStateStore({filePath:file});
    let permit;
    const gate=new InMemoryContextTransitionLeaseGate();
    const scheduler=new ActiveTurnLifecycleScheduler({store,transitionGate:gate,reserve:async(value)=>{
      permit=value; return {status:"prepared-after-restart"};
    }});
    const [result]=await scheduler.recover();
    assert.equal(result.status,"reconciled");
    assert.equal(permit.threadId,observation.threadId);
    assert.equal(permit.turnId,observation.turnId);
    assert.equal(permit.triggeringObservationId,observation.triggeringObservationId);
    for (const field of ["logicalRoleInstanceId","bindingRevision","threadId","turnId",
      "triggeringObservationId","lifecycleRevision"]) {
      const invalid={...permit,[field]:field==="bindingRevision"?2:`wrong-${field}`};
      assert.throws(() => gate.beginPreparation({logicalRoleInstanceId:observation.logicalRoleInstanceId,
        threadId:observation.threadId,bindingRevision:1,lifecycleReservePermit:invalid}),
        /exact lifecycle reserve/);
    }
    assert.deepEqual(Object.keys(permit).sort(),["bindingRevision","disposition","lifecycleRevision",
      "logicalRoleInstanceId","permitRevision","schemaVersion","threadId","triggeringObservationId","turnId","type"].sort());
    store.close();
  } finally {await rm(root,{recursive:true,force:true});}
});

test("restart CAS-reclaims only an expired preparing claim and fences its stale worker", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "active-claim-crash-"));
  const file = path.join(root, "state.sqlite");
  try {
    let store = await openSqliteAppServerStateStore({filePath: file});
    const scheduled = store.scheduleActiveTurnLifecycle(observation);
    const stranded = store.claimActiveTurnLifecycle({identity: scheduled.identity,
      expectedRevision: scheduled.revision, claimedAt: "2026-09-10T00:00:00.000Z",
      claimExpiresAt: "2026-09-10T00:00:01.000Z"});
    store.close();

    store = await openSqliteAppServerStateStore({filePath: file});
    let reserveCalls = 0;
    const scheduler = new ActiveTurnLifecycleScheduler({store,
      transitionGate: new InMemoryContextTransitionLeaseGate(),
      now: () => "2026-09-10T00:00:02.000Z", claimTimeoutMs: 60_000,
      reserve: async () => { reserveCalls += 1; return {preparationRevision: "reclaimed-prep"}; }});
    const [reconciled] = await scheduler.recover();
    assert.equal(reconciled.status, "reconciled");
    assert.equal(reconciled.attempt, 2);
    assert.equal(reconciled.error.name, "LifecycleClaimExpired");
    assert.equal(reserveCalls, 1);
    assert.throws(() => store.completeActiveTurnLifecycle({identity: stranded.identity,
      expectedRevision: stranded.revision, result: {fabricated: true},
      completedAt: "2026-09-10T00:00:03.000Z"}), /revision conflict/);
    assert.deepEqual(await scheduler.recover(), []);
    store.close();
  } finally { await rm(root, {recursive: true, force: true}); }
});

test("concurrent restart scans cannot take over the same preparing obligation", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "active-claim-race-"));
  const file = path.join(root, "state.sqlite");
  try {
    let seed = await openSqliteAppServerStateStore({filePath: file});
    const scheduled = seed.scheduleActiveTurnLifecycle(observation);
    seed.claimActiveTurnLifecycle({identity: scheduled.identity, expectedRevision: scheduled.revision,
      claimedAt: "2026-09-10T00:00:00.000Z", claimExpiresAt: "2026-09-10T00:00:01.000Z"});
    seed.close();
    const firstStore = await openSqliteAppServerStateStore({filePath: file});
    const secondStore = await openSqliteAppServerStateStore({filePath: file});
    let release;
    const blocked = new Promise((resolve) => { release = resolve; });
    let reserveCalls = 0;
    const options = {transitionGate: new InMemoryContextTransitionLeaseGate(),
      now: () => "2026-09-10T00:00:02.000Z", claimTimeoutMs: 60_000};
    const first = new ActiveTurnLifecycleScheduler({store: firstStore, ...options,
      reserve: async () => { reserveCalls += 1; await blocked; return {owner: "first"}; }});
    const second = new ActiveTurnLifecycleScheduler({store: secondStore, ...options,
      reserve: async () => { reserveCalls += 1; return {owner: "second"}; }});
    const firstRecovery = first.recover();
    await new Promise((resolve) => setImmediate(resolve));
    assert.deepEqual(await second.recover(), []);
    release();
    assert.equal((await firstRecovery)[0].status, "reconciled");
    assert.equal(reserveCalls, 1);
    firstStore.close(); secondStore.close();
  } finally { await rm(root, {recursive: true, force: true}); }
});
