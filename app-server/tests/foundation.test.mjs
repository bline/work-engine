import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  BindingConflictError,
  CapabilityError,
  CodexAppServerAdapter,
  DynamicToolBridge,
  ExactSkillResolver,
  FileRoleBindingRegistry,
  REQUEST_CONTEXT_INPUT_PREFIX,
  negotiateCapabilities,
} from "../src/index.mjs";

class FakeTransport {
  constructor() {
    this.requests = [];
    this.notifications = [];
    this.requestHandler = null;
    this.threadStarts = 0;
    this.turns = 0;
    this.notificationHandlers = new Set();
    this.closedHandlers = new Set();
  }

  onServerRequest(handler) {
    this.requestHandler = handler;
  }

  onNotification(handler) {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }

  onClosed(handler) {
    this.closedHandlers.add(handler);
    return () => this.closedHandlers.delete(handler);
  }

  notify(method, params) {
    this.notifications.push({ method, params });
  }

  async request(method, params) {
    this.requests.push({ method, params });
    if (method === "initialize") {
      return {
        userAgent: "work-engine/0.149.1 (Linux; x86_64)",
        codexHome: "/tmp/codex",
        platformFamily: "unix",
        platformOs: "linux",
      };
    }
    if (method === "thread/start") {
      this.threadStarts += 1;
      return { thread: { id: `thread-${this.threadStarts}` } };
    }
    if (method === "thread/resume") return { thread: { id: params.threadId } };
    if (method === "turn/start") {
      this.turns += 1;
      return { turn: { id: `turn-${this.turns}` } };
    }
    throw new Error(`unexpected fake request: ${method}`);
  }

  invokeServerRequest(request) {
    return this.requestHandler(request);
  }

  emitNotification(notification) {
    for (const handler of this.notificationHandlers) handler(notification);
  }

  emitClosed(error) {
    for (const handler of this.closedHandlers) handler(error);
  }
}

function completedTurn({
  threadId = "thread-1",
  turnId = "turn-1",
  status = "completed",
  text = "result",
  error = null,
} = {}) {
  return {
    method: "turn/completed",
    params: {
      threadId,
      turn: {
        id: turnId,
        items: text == null ? [] : [{
          type: "agentMessage",
          id: "message-1",
          text,
          phase: "final_answer",
          memoryCitation: null,
          delivery: null,
        }],
        itemsView: "full",
        status,
        error,
        startedAt: 1,
        completedAt: 2,
        durationMs: 1000,
      },
    },
  };
}

async function fixture(t) {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-app-server."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const skillRoot = path.join(directory, "skills");
  const skillDirectory = path.join(skillRoot, "planner");
  await mkdir(skillDirectory, { recursive: true });
  const skillPath = path.join(skillDirectory, "SKILL.md");
  await writeFile(skillPath, "# Planner\n", "utf8");
  return { directory, skillRoot, skillPath };
}

test("capability negotiation fails closed for unknown capabilities", () => {
  assert.throws(
    () => negotiateCapabilities(["thread_start", "future_method"]),
    CapabilityError,
  );
  const negotiated = negotiateCapabilities(["thread_start", "exact_skill_input"]);
  assert.equal(negotiated.initializeCapabilities.experimentalApi, false);
});

test("runtime capability pin matches the generated-binding lock", async () => {
  const lock = JSON.parse(await readFile(
    new URL("../protocol-bindings.lock.json", import.meta.url),
    "utf8",
  ));
  const negotiated = negotiateCapabilities(["thread_start"]);
  assert.equal(negotiated.protocolVersion, lock.codexCliVersion);
  assert.equal(negotiated.initializeCapabilities.experimentalApi, lock.experimental);
});

test("role bindings persist by logical identity and require revision-bound replacement", async (t) => {
  const { directory } = await fixture(t);
  const filePath = path.join(directory, "state", "role-bindings.json");
  const registry = new FileRoleBindingRegistry(filePath, {
    now: () => "2026-08-24T00:00:00.000Z",
  });
  const first = await registry.bind({
    logicalRoleInstanceId: "strategic-planner:main",
    threadId: "thread-a",
    protocolVersion: "0.149.1",
    environmentFingerprint: "fingerprint-a",
  });
  assert.equal(first.bindingRevision, 1);
  assert.equal((await new FileRoleBindingRegistry(filePath).get(
    "strategic-planner:main",
  )).threadId, "thread-a");
  await assert.rejects(
    registry.bind({
      logicalRoleInstanceId: "strategic-planner:main",
      threadId: "thread-b",
      protocolVersion: "0.149.1",
      environmentFingerprint: "fingerprint-b",
      expectedBindingRevision: null,
    }),
    BindingConflictError,
  );
  const replaced = await registry.bind({
    logicalRoleInstanceId: "strategic-planner:main",
    threadId: "thread-b",
    protocolVersion: "0.149.1",
    environmentFingerprint: "fingerprint-b",
    expectedBindingRevision: 1,
  });
  assert.equal(replaced.bindingRevision, 2);
  assert.equal(JSON.parse(await readFile(filePath, "utf8")).revision, 2);
});

test("skill resolution produces exact App Server input items and rejects escapes", async (t) => {
  const { directory, skillRoot, skillPath } = await fixture(t);
  const resolver = await ExactSkillResolver.create([skillRoot]);
  assert.deepEqual(await resolver.resolve([{ name: "planner", path: skillPath }]), [{
    type: "skill",
    name: "planner",
    path: skillPath,
  }]);
  const outside = path.join(directory, "outside", "SKILL.md");
  await mkdir(path.dirname(outside), { recursive: true });
  await writeFile(outside, "# Outside\n", "utf8");
  await assert.rejects(
    resolver.resolve([{ name: "outside", path: outside }]),
    /outside configured roots/,
  );
});

test("dynamic tools are grouped by namespace and dispatched per bridge", async () => {
  const bridge = new DynamicToolBridge([
    {
      namespace: "evidence",
      name: "read_claim",
      description: "Read one exact claim",
      inputSchema: { type: "object", properties: { id: { type: "string" } } },
      handler: ({ id }) => ({ id, revision: "r1" }),
    },
  ]);
  assert.deepEqual(bridge.specs()[0], {
    type: "namespace",
    name: "evidence",
    description: "Work Engine dynamic tools for evidence",
    tools: [{
      type: "function",
      name: "read_claim",
      description: "Read one exact claim",
      inputSchema: { type: "object", properties: { id: { type: "string" } } },
    }],
  });
  const result = await bridge.dispatch({
    namespace: "evidence",
    tool: "read_claim",
    arguments: { id: "claim-1" },
  });
  assert.equal(result.success, true);
  assert.deepEqual(JSON.parse(result.contentItems[0].text), {
    id: "claim-1",
    revision: "r1",
  });
});

test("adapter binds once, resumes by logical role, injects skills, and dispatches tools", async (t) => {
  const { directory, skillRoot, skillPath } = await fixture(t);
  const transport = new FakeTransport();
  const registry = new FileRoleBindingRegistry(path.join(directory, "bindings.json"));
  const skillResolver = await ExactSkillResolver.create([skillRoot]);
  const adapter = new CodexAppServerAdapter({ transport, registry, skillResolver });
  await adapter.initialize();

  const bridge = new DynamicToolBridge([{
    name: "lookup",
    description: "Look up a bounded value",
    inputSchema: { type: "object" },
    handler: () => "found",
  }]);
  const role = {
    logicalRoleInstanceId: "strategic-planner:roadmap",
    developerInstructions: "Plan without mutating the repository.",
    threadOptions: { cwd: directory, permissions: "read-only" },
  };
  const first = await adapter.deliverTurn({
    role,
    text: "Reconcile the roadmap.",
    clientUserMessageId: "message-1",
    skills: [{ name: "planner", path: skillPath }],
    toolBridge: bridge,
    requestContext: {
      roadmap: { kind: "application", value: "roadmap.md@revision-1" },
    },
  });
  const second = await adapter.deliverTurn({
    role,
    text: "Continue from durable state.",
    clientUserMessageId: "message-2",
    skills: [{ name: "planner", path: skillPath }],
    toolBridge: bridge,
  });
  const replayed = await adapter.deliverTurn({
    role,
    text: "Continue from durable state.",
    clientUserMessageId: "message-2",
    skills: [{ name: "planner", path: skillPath }],
    toolBridge: bridge,
  });

  assert.equal(first.createdThread, true);
  assert.equal(second.createdThread, false);
  assert.equal(first.threadId, second.threadId);
  assert.equal(replayed.replayedDelivery, true);
  assert.equal(replayed.turnId, second.turnId);
  await assert.rejects(
    adapter.deliverTurn({
      role,
      text: "The same message identity cannot name different content.",
      clientUserMessageId: "message-2",
      skills: [{ name: "planner", path: skillPath }],
      toolBridge: bridge,
    }),
    /reused for different turn content/,
  );
  assert.deepEqual(
    transport.requests.map(({ method }) => method),
    ["initialize", "thread/start", "turn/start", "thread/resume", "turn/start"],
  );
  const threadStart = transport.requests.find(({ method }) => method === "thread/start");
  assert.equal(threadStart.params.ephemeral, false);
  assert.equal(threadStart.params.dynamicTools[0].name, "lookup");
  const turnStart = transport.requests.find(({ method }) => method === "turn/start");
  assert.equal(turnStart.params.clientUserMessageId, "message-1");
  assert.equal(turnStart.params.input[0].type, "skill");
  assert.equal(turnStart.params.input[1].type, "text");
  assert.equal(turnStart.params.input[1].text.startsWith(REQUEST_CONTEXT_INPUT_PREFIX), true);
  assert.equal(turnStart.params.input[2].type, "text");
  assert.equal("additionalContext" in turnStart.params, false);
  const requestContext = JSON.parse(
    turnStart.params.input[1].text.slice(REQUEST_CONTEXT_INPUT_PREFIX.length),
  );
  assert.deepEqual(requestContext, {
    entries: {
      roadmap: { kind: "application", value: "roadmap.md@revision-1" },
    },
    schema_version: 1,
    type: "work-engine.request-context",
  });
  const toolResult = await transport.invokeServerRequest({
    method: "item/tool/call",
    params: {
      threadId: first.threadId,
      turnId: first.turnId,
      callId: "call-1",
      namespace: null,
      tool: "lookup",
      arguments: {},
    },
  });
  assert.deepEqual(toolResult, {
    success: true,
    contentItems: [{ type: "inputText", text: "found" }],
  });

  const changedBridge = new DynamicToolBridge([{
    name: "different_lookup",
    description: "Expose a different thread tool",
    inputSchema: { type: "object" },
    handler: () => "different",
  }]);
  await assert.rejects(
    adapter.deliverTurn({
      role,
      text: "Do not silently change the tool surface.",
      clientUserMessageId: "message-3",
      skills: [{ name: "planner", path: skillPath }],
      toolBridge: changedBridge,
    }),
    /replace the runtime binding/,
  );
});

test("request context is closed, deterministic, and part of delivery identity", async (t) => {
  const { directory, skillRoot } = await fixture(t);
  const transport = new FakeTransport();
  const adapter = new CodexAppServerAdapter({
    transport,
    registry: new FileRoleBindingRegistry(path.join(directory, "bindings.json")),
    skillResolver: await ExactSkillResolver.create([skillRoot]),
  });
  await adapter.initialize();
  const role = { logicalRoleInstanceId: "role:context" };

  await adapter.deliverTurn({
    role,
    text: "Use the bound context.",
    clientUserMessageId: "context-message",
    requestContext: {
      zeta: { value: "last", kind: "application" },
      alpha: { kind: "application", value: "first" },
    },
  });
  const turn = transport.requests.find(({ method }) => method === "turn/start");
  assert.equal(
    turn.params.input[0].text,
    `${REQUEST_CONTEXT_INPUT_PREFIX}{"entries":{"alpha":{"kind":"application","value":"first"},"zeta":{"kind":"application","value":"last"}},"schema_version":1,"type":"work-engine.request-context"}`,
  );

  await assert.rejects(
    adapter.deliverTurn({
      role,
      text: "Use the bound context.",
      clientUserMessageId: "context-message",
      requestContext: {
        alpha: { kind: "application", value: "changed" },
      },
    }),
    /reused for different turn content/,
  );
  await assert.rejects(
    adapter.deliverTurn({
      role,
      text: "Reject ambiguous context.",
      clientUserMessageId: "invalid-context",
      requestContext: {
        alpha: { kind: "application", value: "first", authority: "invented" },
      },
    }),
    /unsupported fields: authority/,
  );
  await assert.rejects(
    adapter.deliverTurn({
      role,
      text: "Reject unsafe context names.",
      clientUserMessageId: "invalid-context-name",
      requestContext: {
        "__proto__": { kind: "application", value: "unsafe" },
      },
    }),
    /must contain at least one named entry|entry name is invalid/,
  );
});

test("adapter consumes completed turns without racing fast notifications", async (t) => {
  const { directory, skillRoot } = await fixture(t);
  const transport = new FakeTransport();
  const adapter = new CodexAppServerAdapter({
    transport,
    registry: new FileRoleBindingRegistry(path.join(directory, "bindings.json")),
    skillResolver: await ExactSkillResolver.create([skillRoot]),
  });
  await adapter.initialize();

  transport.emitNotification(completedTurn({ text: "final handoff" }));
  const completion = await adapter.waitForTurnCompletion({
    threadId: "thread-1",
    turnId: "turn-1",
  });
  assert.equal(completion.outputText, "final handoff");
  assert.equal(completion.status, "completed");

  const originalWait = adapter.waitForTurnCompletion({
    threadId: "thread-1",
    turnId: "turn-running",
  });
  const joinedReplayWait = adapter.waitForTurnCompletion({
    threadId: "thread-1",
    turnId: "turn-running",
    replayedDelivery: true,
  });
  transport.emitNotification(completedTurn({
    turnId: "turn-running",
    text: "joined result",
  }));
  assert.equal((await originalWait).outputText, "joined result");
  assert.equal((await joinedReplayWait).outputText, "joined result");

  transport.emitNotification(completedTurn({
    turnId: "turn-failed",
    status: "failed",
    text: null,
    error: { message: "provider failed", codexErrorInfo: null, additionalDetails: null },
  }));
  await assert.rejects(
    adapter.waitForTurnCompletion({ threadId: "thread-1", turnId: "turn-failed" }),
    /provider failed/,
  );
  await assert.rejects(
    adapter.waitForTurnCompletion({
      threadId: "thread-1",
      turnId: "turn-not-retained",
      replayedDelivery: true,
    }),
    /reconcile it from App Server thread state/,
  );

  const interruptedWait = adapter.waitForTurnCompletion({
    threadId: "thread-1",
    turnId: "turn-still-running",
  });
  transport.emitClosed(new Error("connection lost"));
  await assert.rejects(interruptedWait, /connection lost/);
});
