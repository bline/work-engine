import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  ContextInputCustodyController,
  FileRoleBindingRegistry,
  OperatorSwitchboard,
  openSqliteAppServerStateStore,
  projectRuntimeManifest,
} from "../src/index.mjs";

function deferred() {
  let resolve;
  const promise = new Promise((settle) => { resolve = settle; });
  return { promise, resolve };
}

function manifest() {
  return projectRuntimeManifest({
    schema_version: 1,
    manifest_id: "switchboard.test",
    roles: {
      alpha: {
        contract: "skills/alpha/SKILL.md",
        developer_instructions: "Alpha test role.",
        skills: [{ name: "alpha", path: "skills/alpha/SKILL.md" }],
      },
      beta: {
        contract: "skills/beta/SKILL.md",
        developer_instructions: "Beta test role.",
        skills: [{ name: "beta", path: "skills/beta/SKILL.md" }],
      },
    },
  }, { baseDirectory: "/tmp/switchboard-test" });
}

async function fixture(t, runtimeResult = null, inputCustody = null) {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-switchboard."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const registryPath = path.join(directory, "bindings.json");
  const registry = new FileRoleBindingRegistry(registryPath, {
    now: () => "2026-08-26T03:00:00.000Z",
  });
  const calls = [];
  const runtime = {
    async deliverTurn(turn) {
      calls.push(turn);
      return runtimeResult ?? {
        logicalRoleInstanceId: `${turn.roleId}:${turn.instanceId}`,
        threadId: "thread-alpha",
        turnId: "turn-alpha",
        createdThread: true,
        replayedDelivery: false,
        binding: { bindingRevision: 1 },
      };
    },
  };
  let message = 0;
  const completions = [];
  const switchboard = new OperatorSwitchboard({
    manifest: manifest(),
    runtime,
    registry,
    completionWaiter: async (delivery) => {
      completions.push(delivery);
      return { outputText: "bounded model response" };
    },
    inputCustody,
    messageIdFactory: () => `operator-message-${++message}`,
  });
  return { switchboard, registry, registryPath, calls, completions };
}

test("input arriving under a closed context admission is durably queued without reaching the role", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-custody-switchboard."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const store = await openSqliteAppServerStateStore({
    filePath: path.join(directory, "state.sqlite3"),
  });
  t.after(() => store.close());
  const custody = new ContextInputCustodyController({ store });
  const completion = deferred();
  const { switchboard, registry, calls } = await fixture(t, {
    logicalRoleInstanceId: "alpha:main",
    threadId: "thread-alpha",
    turnId: "turn-alpha",
    createdThread: false,
    replayedDelivery: false,
    binding: { bindingRevision: 1 },
  }, custody);
  switchboard.completionWaiter = () => completion.promise;
  await registry.bind({
    logicalRoleInstanceId: "alpha:main",
    threadId: "thread-alpha",
    protocolVersion: "0.149.1",
    environmentFingerprint: "switchboard-test",
  });
  await switchboard.handleLine(":we attach alpha:main");
  const first = await switchboard.startLine("current predecessor work");
  await custody.closeAdmission({
    logicalRoleInstanceId: "alpha:main",
    threadId: "thread-alpha",
    bindingRevision: 1,
    transitionRevision: `sha256:${"a".repeat(64)}`,
  });

  const queued = await switchboard.startLine("input after the transition fence");
  assert.equal(queued.result.kind, "queued_message");
  assert.equal(queued.result.logicalRoleInstanceId, "alpha:main");
  assert.equal(calls.length, 1);
  assert.deepEqual(store.pendingContextInputs({
    logicalRoleInstanceId: "alpha:main",
  }).map((item) => item.input.text), ["input after the transition fence"]);

  completion.resolve({ outputText: "predecessor completed" });
  await first.completion;
});

test("administrative commands never reach the role runtime or create bindings", async (t) => {
  const { switchboard, registryPath, calls } = await fixture(t);

  assert.deepEqual(await switchboard.handleLine(":we agents"), {
    kind: "command",
    command: "agents",
    roles: ["alpha", "beta"],
  });
  const attached = await switchboard.handleLine(":we attach alpha:main");
  assert.deepEqual(attached.attachment, { roleId: "alpha", instanceId: "main" });
  assert.equal(attached.binding, null);
  assert.equal((await switchboard.handleLine(":we status")).binding, null);
  assert.deepEqual((await switchboard.handleLine(":we threads")).threads, []);
  assert.deepEqual((await switchboard.handleLine(":we detach")).previousAttachment, {
    roleId: "alpha",
    instanceId: "main",
  });
  await assert.rejects(switchboard.handleLine(":we handoff beta"), /unknown switchboard command/);
  await assert.rejects(switchboard.handleLine(":we attach missing:main"), /does not define role/);

  assert.deepEqual(calls, []);
  await assert.rejects(readFile(registryPath, "utf8"), { code: "ENOENT" });
});

test("ordinary input routes to exactly the attached manifest role and returns final output", async (t) => {
  const { switchboard, calls, completions } = await fixture(t);
  await assert.rejects(switchboard.handleLine("message before attachment"), /no role instance/);
  await switchboard.handleLine(":we attach beta:probe");

  const result = await switchboard.handleLine("Preserve this operator message exactly.");

  assert.deepEqual(calls, [{
    roleId: "beta",
    instanceId: "probe",
    clientUserMessageId: "operator-message-1",
    text: "Preserve this operator message exactly.",
    signal: undefined,
  }]);
  assert.equal(result.delivery.logicalRoleInstanceId, "beta:probe");
  assert.equal(result.outputText, "bounded model response");
  assert.deepEqual(completions, [{
    threadId: "thread-alpha",
    turnId: "turn-alpha",
    replayedDelivery: false,
    signal: undefined,
  }]);
});

test("turn start returns delivery before the attached role completes", async (t) => {
  const completion = deferred();
  const { switchboard } = await fixture(t, {
    logicalRoleInstanceId: "alpha:main",
    threadId: "thread-alpha",
    turnId: "turn-alpha",
    createdThread: false,
    replayedDelivery: false,
    binding: { bindingRevision: 2 },
  });
  switchboard.completionWaiter = () => completion.promise;
  await switchboard.handleLine(":we attach alpha:main");

  const started = await switchboard.startLine("Begin visible work.");
  assert.equal(started.delivery.turnId, "turn-alpha");
  let completed = false;
  started.completion.then(() => { completed = true; });
  await Promise.resolve();
  assert.equal(completed, false);

  completion.resolve({ outputText: "finished later" });
  assert.equal((await started.completion).outputText, "finished later");
});

test("threads are a sorted read-only view and status exposes only observer metadata", async (t) => {
  const { switchboard, registry, calls } = await fixture(t);
  await registry.bind({
    logicalRoleInstanceId: "beta:two",
    threadId: "thread-2",
    protocolVersion: "0.149.1",
    environmentFingerprint: "fingerprint-2",
  });
  await registry.bind({
    logicalRoleInstanceId: "alpha:one",
    threadId: "thread-1",
    protocolVersion: "0.149.1",
    environmentFingerprint: "fingerprint-1",
  });

  const threads = await switchboard.handleLine(":we threads");
  assert.deepEqual(threads.threads.map(({ logicalRoleInstanceId }) =>
    logicalRoleInstanceId), ["alpha:one", "beta:two"]);
  assert.equal(Object.isFrozen(threads.threads[0]), true);
  assert.deepEqual(calls, []);
});

test("a lifecycle-aware runtime completion is not awaited a second time", async (t) => {
  const runtimeResult = {
    delivery: {
      logicalRoleInstanceId: "alpha:main",
      threadId: "thread-1",
      turnId: "turn-1",
      createdThread: false,
      replayedDelivery: false,
      binding: { bindingRevision: 3 },
    },
    completion: { outputText: "already completed" },
    shadow: { status: "recorded" },
  };
  const { switchboard, completions } = await fixture(t, runtimeResult);
  await switchboard.handleLine(":we attach alpha:main");

  const result = await switchboard.handleLine("Run through the lifecycle host.");

  assert.equal(result.outputText, "already completed");
  assert.deepEqual(result.shadow, { status: "recorded" });
  assert.deepEqual(completions, []);
  assert.deepEqual((await switchboard.handleLine(":we status")).lifecycle, {
    status: "recorded",
  });
});

test("a live lifecycle receipt is exposed through the compatible observer view", async (t) => {
  const runtimeResult = {
    delivery: {
      logicalRoleInstanceId: "alpha:main",
      threadId: "thread-1",
      turnId: "turn-1",
      createdThread: false,
      replayedDelivery: false,
      binding: { bindingRevision: 3 },
    },
    completion: { outputText: "continued after replacement" },
    lifecycle: { status: "reconciled" },
  };
  const { switchboard } = await fixture(t, runtimeResult);
  await switchboard.handleLine(":we attach alpha:main");
  const result = await switchboard.handleLine("Run through the live lifecycle host.");
  assert.deepEqual(result.shadow, { status: "reconciled" });
  assert.deepEqual((await switchboard.handleLine(":we status")).lifecycle, {
    status: "reconciled",
  });
});

test("attachment persistence precedes visible switchboard state changes", async (t) => {
  const { registry } = await fixture(t);
  const writes = [];
  const switchboard = new OperatorSwitchboard({
    manifest: manifest(),
    runtime: { async deliverTurn() { throw new Error("not used"); } },
    registry,
    initialAttachment: { roleId: "alpha", instanceId: "restored" },
    onAttachmentChange: async (attachment) => { writes.push(attachment); },
  });

  assert.deepEqual((await switchboard.handleLine(":we status")).attachment, {
    roleId: "alpha",
    instanceId: "restored",
  });
  await switchboard.handleLine(":we attach beta:next");
  await switchboard.handleLine(":we detach");
  assert.deepEqual(writes, [
    { roleId: "beta", instanceId: "next" },
    null,
  ]);
});
