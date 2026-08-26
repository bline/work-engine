import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  ContextInputCustodyController,
  normalizeContextTransitionInput,
  openSqliteAppServerStateStore,
} from "../src/index.mjs";

const transitionRevision = `sha256:${"a".repeat(64)}`;
const reconciliationRevision = `sha256:${"b".repeat(64)}`;

async function harness(t) {
  const directory = await mkdtemp(path.join(tmpdir(), "context-input-custody-"));
  t.after(async () => rm(directory, { recursive: true, force: true }));
  const filePath = path.join(directory, "state.sqlite3");
  const store = await openSqliteAppServerStateStore({ filePath });
  t.after(() => {
    if (!store.closed) store.close();
  });
  return { filePath, store, controller: new ContextInputCustodyController({ store }) };
}

function fence() {
  return {
    logicalRoleInstanceId: "strategic-planner:main",
    threadId: "thread-planner-1",
    bindingRevision: 1,
    transitionRevision,
    closedAt: "2026-08-26T12:00:00.000Z",
  };
}

function input(number) {
  return {
    logicalRoleInstanceId: "strategic-planner:main",
    roleId: "strategic-planner",
    instanceId: "main",
    threadId: "thread-planner-1",
    bindingRevision: 1,
    clientUserMessageId: `operator:queued-${number}`,
    sourceKind: "human",
    text: `queued input ${number}`,
  };
}

test("closed context admission durably queues exact attributed input in arrival order", async (t) => {
  const { filePath, store, controller } = await harness(t);
  assert.equal((await controller.closeAdmission(fence())).status, "closed");
  const first = await controller.queueIfClosed(input(1));
  const second = await controller.queueIfClosed(input(2));
  assert.equal(first.status, "queued");
  assert.equal(second.status, "queued");
  assert.ok(first.item.sequence < second.item.sequence);
  assert.equal((await controller.queueIfClosed(structuredClone(input(1)))).status, "replayed");
  await assert.rejects(
    controller.queueIfClosed({ ...input(1), text: "different input" }),
    /reused for different input/,
  );

  store.close();
  const recovered = await openSqliteAppServerStateStore({ filePath });
  t.after(() => recovered.close());
  const queued = recovered.pendingContextInputs({
    logicalRoleInstanceId: "strategic-planner:main",
    transitionRevision,
  });
  assert.deepEqual(queued.map((item) => item.input.text), ["queued input 1", "queued input 2"]);
  assert.equal(queued[0].input.inputRevision, normalizeContextTransitionInput(input(1)).inputRevision);
  assert.deepEqual(recovered.integrityCheck(), ["ok"]);
});

test("accepted reconciliation releases queued input in order and reopens only after receipts", async (t) => {
  const { store, controller } = await harness(t);
  await controller.closeAdmission(fence());
  await controller.queueIfClosed(input(1));
  await controller.queueIfClosed(input(2));
  const deliveries = [];
  const outcome = await controller.releaseAfterReconciliation({
    logicalRoleInstanceId: "strategic-planner:main",
    transitionRevision,
    reconciliationRevision,
  }, async (queued) => {
    deliveries.push(queued.clientUserMessageId);
    return {
      threadId: queued.threadId,
      turnId: `turn-${deliveries.length}`,
      clientUserMessageId: queued.clientUserMessageId,
      replayedDelivery: false,
    };
  });
  assert.equal(outcome.status, "released");
  assert.deepEqual(deliveries, ["operator:queued-1", "operator:queued-2"]);
  assert.equal(outcome.released.length, 2);
  assert.equal(store.pendingContextInputs({
    logicalRoleInstanceId: "strategic-planner:main",
  }).length, 0);
  assert.equal(store.contextInputAdmission("strategic-planner:main").status, "open");

  const replay = await controller.releaseAfterReconciliation({
    logicalRoleInstanceId: "strategic-planner:main",
    transitionRevision,
    reconciliationRevision,
  }, async () => { throw new Error("replay must not redeliver"); });
  assert.equal(replay.status, "replayed");
});

test("failed delivery keeps admission closed and restart resumes from the first unreceipted input", async (t) => {
  const { filePath, store, controller } = await harness(t);
  await controller.closeAdmission(fence());
  await controller.queueIfClosed(input(1));
  await controller.queueIfClosed(input(2));
  const attempted = [];
  await assert.rejects(
    controller.releaseAfterReconciliation({
      logicalRoleInstanceId: "strategic-planner:main",
      transitionRevision,
      reconciliationRevision,
    }, async (queued) => {
      attempted.push(queued.clientUserMessageId);
      if (queued.clientUserMessageId.endsWith("2")) throw new Error("delivery unavailable");
      return {
        threadId: queued.threadId,
        turnId: "turn-first",
        clientUserMessageId: queued.clientUserMessageId,
        replayedDelivery: false,
      };
    }),
    /delivery unavailable/,
  );
  assert.equal(store.contextInputAdmission("strategic-planner:main").status, "releasing");
  assert.deepEqual(attempted, ["operator:queued-1", "operator:queued-2"]);
  store.close();

  const recovered = await openSqliteAppServerStateStore({ filePath });
  t.after(() => recovered.close());
  const resumed = new ContextInputCustodyController({ store: recovered });
  const retried = [];
  const outcome = await resumed.releaseAfterReconciliation({
    logicalRoleInstanceId: "strategic-planner:main",
    transitionRevision,
    reconciliationRevision,
  }, async (queued) => {
    retried.push(queued.clientUserMessageId);
    return {
      threadId: queued.threadId,
      turnId: "turn-second",
      clientUserMessageId: queued.clientUserMessageId,
      replayedDelivery: true,
    };
  });
  assert.equal(outcome.status, "released");
  assert.deepEqual(retried, ["operator:queued-2"]);
  assert.equal(recovered.contextInputAdmission("strategic-planner:main").status, "open");
});

test("input remains outside custody while admission is open", async (t) => {
  const { controller } = await harness(t);
  const result = await controller.queueIfClosed(input(1));
  assert.equal(result.status, "admission_open");
  assert.equal(result.input.text, "queued input 1");
});
