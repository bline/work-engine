import assert from "node:assert/strict";
import test from "node:test";

import {
  CodexAppServerAdapter,
  CodexAppServerInferenceCapability,
} from "../src/index.mjs";

class InferenceTransport {
  constructor() {
    this.requests = [];
    this.notificationHandlers = new Set();
    this.threadCount = 0;
    this.turnCount = 0;
  }

  onServerRequest() {}

  onNotification(handler) {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }

  onClosed() {
    return () => {};
  }

  notify() {}

  emit(notification) {
    for (const handler of this.notificationHandlers) handler(notification);
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
      this.threadCount += 1;
      return { thread: { id: `inference-thread-${this.threadCount}` } };
    }
    if (method === "turn/start") {
      this.turnCount += 1;
      const turnId = `inference-turn-${this.turnCount}`;
      queueMicrotask(() => {
        this.emit({
          method: "thread/tokenUsage/updated",
          params: {
            threadId: params.threadId,
            turnId,
            tokenUsage: {
              last: {
                inputTokens: 123,
                cachedInputTokens: 23,
                cacheWriteInputTokens: 0,
                outputTokens: 45,
                reasoningOutputTokens: 6,
                totalTokens: 168,
              },
              total: {
                inputTokens: 123,
                cachedInputTokens: 23,
                cacheWriteInputTokens: 0,
                outputTokens: 45,
                reasoningOutputTokens: 6,
                totalTokens: 168,
              },
              modelContextWindow: 100_000,
            },
          },
        });
        this.emit({
          method: "turn/completed",
          params: {
            threadId: params.threadId,
            turn: {
              id: turnId,
              status: "completed",
              items: [{ type: "agentMessage", phase: "final_answer", text: "result: accepted" }],
            },
          },
        });
      });
      return { turn: { id: turnId } };
    }
    throw new Error(`unexpected request ${method}`);
  }
}

test("Codex inference uses an ephemeral bounded turn and returns provenance and usage", async () => {
  const transport = new InferenceTransport();
  const adapter = new CodexAppServerAdapter({ transport, registry: {}, skillResolver: {} });
  await adapter.initialize();
  const capability = new CodexAppServerInferenceCapability({
    adapter,
    producer: "work-engine.semantic-context-compiler",
    version: "1",
    model: "test-model",
    threadOptions: { cwd: "/tmp", approval_policy: "never", sandbox: "read-only" },
    nextClientMessageId: () => "compiler-message-1",
  });

  const result = await capability.infer({
    instructions: "Compile only the supplied bounded material.",
    input: { zeta: "untrusted", alpha: 1 },
    outputContract: { format: "yaml", fields: ["result"] },
  });

  const threadStart = transport.requests.find((request) => request.method === "thread/start");
  assert.deepEqual(threadStart.params, {
    cwd: "/tmp",
    approval_policy: "never",
    sandbox: "read-only",
    model: "test-model",
    developerInstructions: "Compile only the supplied bounded material.",
    ephemeral: true,
  });
  const turnStart = transport.requests.find((request) => request.method === "turn/start");
  assert.equal(turnStart.params.clientUserMessageId, "compiler-message-1");
  assert.match(turnStart.params.input[0].text, /Do not invoke tools or seek material outside/);
  assert.match(turnStart.params.input[0].text, /"input":\{"alpha":1,"zeta":"untrusted"\}/);
  assert.equal(result.outputText, "result: accepted");
  assert.deepEqual(result.provenance, {
    producer: "work-engine.semantic-context-compiler",
    model: "test-model",
    version: "1",
    inferenceId: "codex-app-server:inference-turn-1",
  });
  assert.deepEqual(result.usage, {
    inputTokens: 123,
    cachedInputTokens: 23,
    outputTokens: 45,
    costMicrounits: null,
  });
  assert.equal(transport.notificationHandlers.size, 1);
});

test("separate inference capabilities create separate ephemeral threads", async () => {
  const transport = new InferenceTransport();
  const adapter = new CodexAppServerAdapter({ transport, registry: {}, skillResolver: {} });
  await adapter.initialize();
  const create = (producer, clientUserMessageId) => new CodexAppServerInferenceCapability({
    adapter,
    producer,
    version: "1",
    nextClientMessageId: () => clientUserMessageId,
  });
  const compiler = create("compiler", "compiler-1");
  const verifier = create("verifier", "verifier-1");

  const first = await compiler.infer({
    instructions: "Compile.", input: { value: 1 }, outputContract: { format: "yaml" },
  });
  const second = await verifier.infer({
    instructions: "Verify.", input: { value: 1 }, outputContract: { format: "yaml" },
  });

  assert.equal(transport.threadCount, 2);
  assert.notEqual(first.provenance.inferenceId, second.provenance.inferenceId);
  assert.deepEqual(
    transport.requests.filter((request) => request.method === "thread/start")
      .map((request) => request.params.ephemeral),
    [true, true],
  );
});

test("Codex inference rejects undeclared request fields before delivery", async () => {
  const transport = new InferenceTransport();
  const adapter = new CodexAppServerAdapter({ transport, registry: {}, skillResolver: {} });
  await adapter.initialize();
  const capability = new CodexAppServerInferenceCapability({
    adapter,
    producer: "compiler",
    version: "1",
  });

  await assert.rejects(
    capability.infer({
      instructions: "Compile.",
      input: {},
      outputContract: {},
      tools: ["shell"],
    }),
    /unsupported fields: tools/,
  );
  assert.equal(transport.requests.filter((request) => request.method !== "initialize").length, 0);
});
