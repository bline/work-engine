import assert from "node:assert/strict";
import test from "node:test";

import { LiveContextLifecycleCoordinator } from "../src/index.mjs";

const SUBJECT = Object.freeze({
  logicalRoleInstanceId: "strategic-planner:main",
  threadId: "thread-planner",
  bindingRevision: 3,
});
const ROLE = Object.freeze({ logicalRoleInstanceId: SUBJECT.logicalRoleInstanceId });

function fixture({ verificationDisposition = "accepted", publicationStatus = "published" } = {}) {
  const calls = [];
  const preparation = Object.freeze({ preparationRevision: "preparation-1" });
  const attestation = Object.freeze({
    validation: Object.freeze({
      status: "accepted",
      receipt: Object.freeze({ current_context_window_id: "window-7" }),
    }),
  });
  const projection = Object.freeze({ observedContext: Object.freeze({}) });
  const sourceMaterials = Object.freeze([]);
  const candidate = Object.freeze({ candidateRevision: "candidate-1" });
  const verification = Object.freeze({
    disposition: verificationDisposition,
    verificationRevision: "verification-1",
  });
  const publication = Object.freeze({ checkpointRevision: "checkpoint-1" });
  const ledgerEntry = Object.freeze({ entryRevision: "ledger-1" });
  const currentFence = Object.freeze({
    logicalRoleInstanceId: SUBJECT.logicalRoleInstanceId,
    threadId: SUBJECT.threadId,
    bindingRevision: SUBJECT.bindingRevision,
    sourceRevision: "source-1",
    authorityRevision: "authority-1",
    publicationRevision: publication.checkpointRevision,
    ledgerRevision: ledgerEntry.entryRevision,
  });
  const lease = Object.freeze({ leaseRevision: "lease-1" });
  const transitionRuntime = {
    async beginPreparation(subject) {
      calls.push(["beginPreparation", subject]);
      return { preparation };
    },
    async attestContextWindow(input) {
      calls.push(["attestContextWindow", input]);
      return attestation;
    },
    async promotePreparation(input) {
      calls.push(["promotePreparation", input]);
      return { lease };
    },
    async retireAndReconcile(input) {
      calls.push(["retireAndReconcile", input]);
      return { reconciliation: { status: "reconciled" } };
    },
  };
  const inferenceRuntime = {
    async inspect(input) {
      calls.push(["inspect", input]);
      return { candidate, verification };
    },
  };
  const checkpointPublisher = {
    async publish(input) {
      calls.push(["publish", input]);
      if (publicationStatus !== "published") return { status: publicationStatus };
      return { status: "published", publication, ledgerEntry, currentFence };
    },
  };
  const projectionForPreparation = async (input) => {
    calls.push(["projectionForPreparation", input]);
    return { projection, sourceMaterials };
  };
  return {
    calls,
    coordinator: new LiveContextLifecycleCoordinator({
      transitionRuntime,
      inferenceRuntime,
      checkpointPublisher,
      projectionForPreparation,
    }),
  };
}

test("live lifecycle does not close admission below a configured transition band", async () => {
  const { coordinator, calls } = fixture();
  const result = await coordinator.run({
    episodeId: "episode-comfortable",
    subject: SUBJECT,
    role: ROLE,
    pressureDisposition: "comfortable",
  });
  assert.deepEqual(result, {
    status: "not_scheduled",
    episodeId: "episode-comfortable",
    pressureDisposition: "comfortable",
  });
  assert.deepEqual(calls, []);
});

test("live lifecycle orders preservation, publication, promotion, and actuation", async () => {
  const { coordinator, calls } = fixture();
  const result = await coordinator.run({
    episodeId: "episode-live",
    subject: SUBJECT,
    role: ROLE,
    skills: [{ name: "strategic-planner" }],
    pressureDisposition: "replacement_candidate",
  });
  assert.equal(result.status, "reconciled");
  assert.deepEqual(calls.map(([name]) => name), [
    "beginPreparation",
    "attestContextWindow",
    "projectionForPreparation",
    "inspect",
    "publish",
    "promotePreparation",
    "retireAndReconcile",
  ]);
  const promote = calls.find(([name]) => name === "promotePreparation")[1];
  assert.equal(promote.expectedFence.predecessorContextWindowId, "window-7");
  const retire = calls.find(([name]) => name === "retireAndReconcile")[1];
  assert.match(retire.retirementClientUserMessageId, /^context-lifecycle:[a-f0-9]{64}$/);
  assert.notEqual(retire.retirementClientUserMessageId, retire.rehydrationClientUserMessageId);
});

test("live lifecycle fails closed before publication when semantic verification is unresolved", async () => {
  const { coordinator, calls } = fixture({ verificationDisposition: "unresolved" });
  const result = await coordinator.run({
    episodeId: "episode-unresolved",
    subject: SUBJECT,
    role: ROLE,
    pressureDisposition: "critical",
  });
  assert.equal(result.status, "stopped");
  assert.equal(result.phase, "semantic_verification");
  assert.deepEqual(calls.map(([name]) => name), [
    "beginPreparation",
    "attestContextWindow",
    "projectionForPreparation",
    "inspect",
  ]);
});

test("live lifecycle deduplicates concurrent execution of one episode", async () => {
  let release;
  const pending = new Promise((resolve) => { release = resolve; });
  const { coordinator, calls } = fixture();
  const original = coordinator.transitionRuntime.beginPreparation;
  coordinator.transitionRuntime.beginPreparation = async (subject) => {
    await pending;
    return original(subject);
  };
  const input = {
    episodeId: "episode-deduplicated",
    subject: SUBJECT,
    role: ROLE,
    pressureDisposition: "replacement_candidate",
  };
  const first = coordinator.run(input);
  const second = coordinator.run(input);
  assert.equal(first, second);
  release();
  await Promise.all([first, second]);
  assert.equal(calls.filter(([name]) => name === "beginPreparation").length, 1);
});
