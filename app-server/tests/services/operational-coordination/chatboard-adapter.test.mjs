import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";

import { createChatboardAdapter } from "../../../src/services/operational-coordination/chatboard-adapter.mjs";

const session = "11111111-1111-4111-8111-111111111111";
const claimId = "22222222-2222-4222-8222-222222222222";

test("chatboard adapter binds repository and invokes a fixed exec-file argument boundary", async () => {
  const calls = [];
  const adapter = createChatboardAdapter({
    workspaceRoot: "/tmp/bound-repository",
    runFile: async (...args) => {
      calls.push(args);
      return {stdout: JSON.stringify({revision: "revision-1", claim: {
        resource: "campaign:test", claim_id: claimId, session_id: session,
        authority: "advisory_coordination_only",
      }})};
    },
  });
  const result = await adapter.execute("claim", {
    resource: "campaign:test", author: "slice-supervisor", session_id: session,
    claim_id: claimId, ttl_seconds: 60, note: "bounded test",
  });
  assert.equal(result.claim.authority, "advisory_coordination_only");
  assert.deepEqual(calls[0][1].slice(0, 3), [
    path.resolve("/tmp/bound-repository/skills/durable-state/scripts/codex_chatboard.py"),
    "--repository", path.resolve("/tmp/bound-repository"),
  ]);
  assert.equal(calls[0][1].includes("--claim-id"), true);
});

test("chatboard adapter fails closed on transport, malformed output, and identity drift", async () => {
  const input = {resource: "campaign:test", session_id: session, claim_id: claimId};
  for (const [runFile, pattern] of [
    [async () => { throw new Error("secret transport detail"); }, /operation failed/],
    [async () => ({stdout: "not-json"}), /malformed output/],
    [async () => ({stdout: JSON.stringify({revision: "r", resource: "other", released: true})}),
      /identity is invalid/],
  ]) {
    const adapter = createChatboardAdapter({workspaceRoot: "/tmp/repository", runFile});
    await assert.rejects(adapter.execute("release", input), pattern);
  }
});

test("chatboard adapter validates operation-specific result shapes", async () => {
  const outputs = {
    read: {revision: null, messages: [], claims: {}},
    post: {revision: "r", message: {message_id: claimId, session_id: session}},
    release: {revision: "r", resource: "campaign:test", released: false, reason: "absent"},
  };
  const adapter = createChatboardAdapter({workspaceRoot: "/tmp/repository",
    runFile: async (_command, args) => ({stdout: JSON.stringify(outputs[args[3]])})});
  assert.deepEqual(await adapter.execute("read", {since: 0, limit: 100}), outputs.read);
  assert.deepEqual(await adapter.execute("post", {author: "supervisor", session_id: session,
    topic: "test", body: "test", references: [], message_id: claimId}), outputs.post);
  assert.deepEqual(await adapter.execute("release", {resource: "campaign:test",
    session_id: session, claim_id: claimId}), outputs.release);
});

test("post replay recovery paginates past the oldest 500 messages", async () => {
  const calls = [];
  const messages = Array.from({length: 500}, (_, index) => ({
    sequence: index + 1, message_id: `old-${index + 1}`,
  }));
  const target = {sequence: 501, message_id: claimId, author: "supervisor",
    session_id: session, topic: "test", body: "durable", references: ["candidate:1"]};
  const adapter = createChatboardAdapter({workspaceRoot: "/tmp/repository",
    runFile: async (_command, args) => {
      calls.push(args);
      if (calls.length === 1) throw new Error("response lost after durable post");
      const since = Number(args[args.indexOf("--since") + 1]);
      return {stdout: JSON.stringify({revision: "r", claims: {},
        messages: since === 0 ? messages : [target]})};
    }});
  const result = await adapter.execute("post", {author: "supervisor", session_id: session,
    topic: "test", body: "durable", references: ["candidate:1"], message_id: claimId});
  assert.equal(result.message.message_id, claimId);
  assert.equal(calls.length, 3);
  assert.equal(calls[2][calls[2].indexOf("--since") + 1], "500");
});

test("post replay recovery fails closed on malformed or non-progressing pagination", async () => {
  for (const messages of [
    [{sequence: "1"}],
    Array.from({length: 500}, () => ({sequence: 1})),
  ]) {
    let calls = 0;
    const adapter = createChatboardAdapter({workspaceRoot: "/tmp/repository",
      runFile: async () => {
        calls += 1;
        if (calls === 1) throw new Error("response lost");
        return {stdout: JSON.stringify({revision: "r", claims: {}, messages})};
      }});
    await assert.rejects(adapter.execute("post", {author: "supervisor", session_id: session,
      topic: "test", body: "durable", references: [], message_id: claimId}), /operation failed/);
    assert.equal(calls, 2);
  }
});
