import assert from "node:assert/strict";
import test from "node:test";

import {
  ContextLifecycleEvidenceCollector,
  attachCodexLifecycleEvidence,
  normalizeCodexLifecycleNotification,
} from "../src/index.mjs";

function breakdown(totalTokens) {
  return {
    inputTokens: totalTokens - 4,
    cachedInputTokens: 1,
    outputTokens: 2,
    reasoningOutputTokens: 1,
    totalTokens,
  };
}

function tokenUsageNotification({ threadId = "thread-1", turnId = "turn-1" } = {}) {
  return {
    method: "thread/tokenUsage/updated",
    params: {
      threadId,
      turnId,
      tokenUsage: {
        last: breakdown(10),
        total: breakdown(20),
        modelContextWindow: 100_000,
      },
    },
  };
}

test("Codex lifecycle notifications normalize into provider-neutral observations", () => {
  assert.deepEqual(normalizeCodexLifecycleNotification(tokenUsageNotification()), {
    schemaVersion: 1,
    observationType: "token_usage",
    source: {
      provider: "codex",
      transport: "app-server",
      protocolVersion: "0.149.1",
      method: "thread/tokenUsage/updated",
    },
    threadId: "thread-1",
    turnId: "turn-1",
    details: {
      last: { ...breakdown(10), cacheWriteInputTokens: 0 },
      total: { ...breakdown(20), cacheWriteInputTokens: 0 },
      modelContextWindow: 100_000,
    },
  });

  const compaction = normalizeCodexLifecycleNotification({
    method: "item/completed",
    params: {
      threadId: "thread-1",
      turnId: "turn-2",
      item: { type: "contextCompaction", id: "item-1" },
    },
  });
  assert.equal(compaction.observationType, "context_transition_signal");
  assert.deepEqual(compaction.details, {
    signal: "context_compaction",
    phase: "completed",
    classification: "unclassified",
    providerItemId: "item-1",
  });
  assert.deepEqual(normalizeCodexLifecycleNotification({
    method: "thread/compacted",
    params: { threadId: "thread-1", turnId: "turn-2" },
  }).details, {
    signal: "context_compaction",
    phase: "reported",
    classification: "unclassified",
    providerItemId: null,
  });
  assert.equal(
    normalizeCodexLifecycleNotification({
      method: "item/completed",
      params: { item: { type: "agentMessage" } },
    }),
    null,
  );
  assert.equal(
    normalizeCodexLifecycleNotification({ method: "turn/started", params: {} }),
    null,
  );
});

test("recognized malformed lifecycle notifications fail instead of becoming evidence", () => {
  assert.throws(
    () => normalizeCodexLifecycleNotification({
      method: "thread/tokenUsage/updated",
      params: {
        threadId: "thread-1",
        turnId: "turn-1",
        tokenUsage: { last: breakdown(-1), total: breakdown(20) },
      },
    }),
    /non-negative safe integer/,
  );
  assert.throws(
    () => normalizeCodexLifecycleNotification({
      method: "item/started",
      params: {
        threadId: "thread-1",
        turnId: "turn-1",
        item: { type: "contextCompaction" },
      },
    }),
    /provider item id/,
  );
});

test("collector retains immutable bounded evidence and projects one thread", () => {
  const collector = new ContextLifecycleEvidenceCollector({ retentionLimit: 2 });
  const first = collector.record(normalizeCodexLifecycleNotification(
    tokenUsageNotification({ threadId: "thread-1", turnId: "turn-1" }),
  ));
  collector.record(normalizeCodexLifecycleNotification({
    method: "item/started",
    params: {
      threadId: "thread-1",
      turnId: "turn-2",
      item: { type: "contextCompaction", id: "item-1" },
    },
  }));
  const third = collector.record(normalizeCodexLifecycleNotification(
    tokenUsageNotification({ threadId: "thread-2", turnId: "turn-3" }),
  ));

  assert.equal(first.sequence, 1);
  assert.equal(third.sequence, 3);
  assert.deepEqual(collector.observations().map(({ sequence }) => sequence), [2, 3]);
  assert.deepEqual(collector.observations({ afterSequence: 2 }), [third]);
  assert.throws(() => {
    third.details.total.totalTokens = 0;
  }, /read only|Cannot assign/);

  const threadOne = collector.snapshot("thread-1");
  assert.equal(threadOne.latestTokenUsage, null);
  assert.equal(threadOne.transitionSignals.length, 1);
  assert.equal(threadOne.transitionSignals[0].details.classification, "unclassified");
  assert.equal(threadOne.firstRetainedSequence, 2);
  assert.equal(threadOne.lastRetainedSequence, 2);
  assert.deepEqual(threadOne.retention, {
    limit: 2,
    droppedThroughSequence: 1,
    earliestGloballyRetainedSequence: 2,
  });
  assert.equal(Object.isFrozen(threadOne), true);
});

test("subscription records supported observations and surfaces normalization errors", () => {
  const handlers = new Set();
  const adapter = {
    onNotification(handler) {
      handlers.add(handler);
      return () => handlers.delete(handler);
    },
  };
  const collector = new ContextLifecycleEvidenceCollector();
  const errors = [];
  const unsubscribe = attachCodexLifecycleEvidence({
    adapter,
    collector,
    onError: (error, notification) => errors.push({ error, notification }),
  });

  for (const handler of handlers) handler(tokenUsageNotification());
  for (const handler of handlers) handler({
    method: "thread/tokenUsage/updated",
    params: { threadId: "thread-1", turnId: "turn-2", tokenUsage: {} },
  });
  for (const handler of handlers) handler({ method: "turn/started", params: {} });

  assert.equal(collector.observations().length, 1);
  assert.equal(errors.length, 1);
  assert.match(errors[0].error.message, /must be an object/);
  unsubscribe();
  assert.equal(handlers.size, 0);
});
