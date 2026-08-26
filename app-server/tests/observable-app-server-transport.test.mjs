import assert from "node:assert/strict";
import test from "node:test";

import {
  ObservableAppServerTransport,
  formatAppServerProtocolEvent,
} from "../src/index.mjs";

class DelegateTransport {
  constructor() {
    this.notificationHandlers = new Set();
    this.closedHandlers = new Set();
    this.serverRequestHandler = null;
    this.requests = [];
    this.notifications = [];
  }

  onServerRequest(handler) {
    this.serverRequestHandler = handler;
  }

  onNotification(handler) {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }

  onClosed(handler) {
    this.closedHandlers.add(handler);
    return () => this.closedHandlers.delete(handler);
  }

  async request(method, params) {
    this.requests.push({ method, params });
    if (method === "thread/start") return { thread: { id: "thread-1" } };
    if (method === "turn/start") return { turn: { id: "turn-1" } };
    if (method === "fail") throw new Error("secret provider failure detail");
    return {};
  }

  notify(method, params) {
    if (method === "fail-notification") {
      throw new Error("secret notification failure detail");
    }
    this.notifications.push({ method, params });
  }

  emit(notification) {
    for (const handler of this.notificationHandlers) handler(notification);
  }

  closeWith(error) {
    for (const handler of this.closedHandlers) handler(error);
  }

  invokeServerRequest(request) {
    return this.serverRequestHandler(request);
  }
}

function clock() {
  let tick = 0;
  return {
    now: () => `2026-08-26T02:00:0${tick++}.000Z`,
    monotonicNow: () => tick * 10,
  };
}

test("observable transport forwards protocol operations and emits metadata-safe events", async () => {
  const delegate = new DelegateTransport();
  const times = clock();
  const observed = [];
  const transport = new ObservableAppServerTransport({
    transport: delegate,
    onEvent: (event) => observed.push(event),
    ...times,
  });

  const thread = await transport.request("thread/start", {
    developerInstructions: "secret developer instructions",
    ephemeral: false,
  });
  const turn = await transport.request("turn/start", {
    threadId: thread.thread.id,
    input: [{ type: "text", text: "secret user text" }],
  });
  transport.notify("initialized", { secret: "notification detail" });
  delegate.emit({
    method: "thread/tokenUsage/updated",
    params: {
      threadId: thread.thread.id,
      turnId: turn.turn.id,
      tokenUsage: {
        last: {
          inputTokens: 100,
          cachedInputTokens: 20,
          outputTokens: 25,
          totalTokens: 125,
        },
        modelContextWindow: 1_000,
      },
    },
  });

  assert.deepEqual(delegate.requests.map(({ method }) => method), ["thread/start", "turn/start"]);
  assert.equal(observed[0].kind, "request_started");
  assert.equal(observed[1].kind, "request_completed");
  assert.equal(observed[0].operationId, observed[1].operationId);
  assert.equal(observed[1].subject.threadId, "thread-1");
  assert.equal(observed[3].subject.threadId, "thread-1");
  assert.equal(observed[3].subject.turnId, "turn-1");
  const tokenEvent = observed.at(-1);
  assert.deepEqual(tokenEvent.metrics, {
    inputTokens: 100,
    cachedInputTokens: 20,
    outputTokens: 25,
    totalTokens: 125,
    modelContextWindow: 1_000,
  });
  assert.match(formatAppServerProtocolEvent(tokenEvent), /tokens=125\/1000/);
  const serialized = JSON.stringify(transport.snapshot());
  assert.doesNotMatch(serialized, /secret/);
  assert.doesNotMatch(serialized, /developerInstructions|secret user text/);
});

test("observable transport records failures without retaining their messages", async () => {
  const delegate = new DelegateTransport();
  const times = clock();
  const transport = new ObservableAppServerTransport({ transport: delegate, ...times });

  await assert.rejects(
    transport.request("fail", { prompt: "secret request" }),
    /secret provider failure detail/,
  );
  const event = transport.snapshot().events.at(-1);
  assert.equal(event.kind, "request_failed");
  assert.equal(event.failureType, "Error");
  assert.doesNotMatch(JSON.stringify(event), /secret/);

  assert.throws(
    () => transport.notify("fail-notification", { prompt: "secret notification" }),
    /secret notification failure detail/,
  );
  const notificationEvent = transport.snapshot().events.at(-1);
  assert.equal(notificationEvent.kind, "notification_failed");
  assert.equal(notificationEvent.failureType, "Error");
  assert.doesNotMatch(JSON.stringify(notificationEvent), /secret/);
});

test("server requests remain transparent and observer failures cannot affect delivery", async () => {
  const delegate = new DelegateTransport();
  const times = clock();
  const transport = new ObservableAppServerTransport({
    transport: delegate,
    retentionLimit: 3,
    onEvent: () => { throw new Error("observer failure"); },
    ...times,
  });
  transport.onServerRequest(async (request) => ({
    success: true,
    contentItems: [{ type: "inputText", text: request.method }],
  }));

  const response = await delegate.invokeServerRequest({
    id: 7,
    method: "item/tool/call",
    params: { threadId: "thread-1", arguments: { secret: "tool input" } },
  });
  assert.equal(response.success, true);
  assert.equal(transport.snapshot().events.length, 2);
  assert.equal(transport.snapshot().observationErrors.length, 2);
  assert.equal(transport.snapshot().events[0].operationId, "server-request:7");

  await transport.request("thread/start", {});
  const snapshot = transport.snapshot();
  assert.equal(snapshot.events.length, 3);
  assert.equal(snapshot.latestSequence, 4);
  assert.doesNotMatch(JSON.stringify(snapshot), /tool input|observer failure/);
});
