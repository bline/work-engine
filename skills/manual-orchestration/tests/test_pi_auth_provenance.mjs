// Run with node --experimental-strip-types --test test_pi_auth_provenance.mjs.
import assert from "node:assert/strict";
import { test } from "node:test";
import extension from "../scripts/pi_auth_provenance.ts";

function harness(auth) {
  const handlers = new Map();
  const entries = [];
  extension({ on: (name, handler) => handlers.set(name, handler), appendEntry: (...entry) => entries.push(entry) });
  const ctx = {
    model: { provider: "openai", id: "selected-model" },
    modelRegistry: {
      getProviderAuth: async (provider) => {
        assert.equal(provider, "anthropic");
        return auth();
      },
    },
  };
  return { entries, emit: (name, data) => handlers.get(name)(data, ctx) };
}

const assistant = { message: { role: "assistant", provider: "anthropic", responseId: "msg_123",
  model: "requested-model", responseModel: "served-model" } };

test("records only allowlisted provenance and the response's identity", async () => {
  for (const [apiKey, authorization, source, expected, safeSource] of [
    ["sk-ant-oat-synthetic", undefined, "OAuth", "oauth", "OAuth"],
    ["synthetic-api", undefined, "ANTHROPIC_API_KEY", "api_key", "ANTHROPIC_API_KEY"],
    [undefined, "Bearer synthetic-auth", "ANTHROPIC_AUTH_TOKEN", "bearer", "ANTHROPIC_AUTH_TOKEN"],
    [undefined, undefined, "secret-unrecognized-source", "unknown", null],
  ]) {
    const { emit, entries } = harness(() => ({ auth: { apiKey }, source }));
    emit("turn_start");
    await emit("before_provider_headers", { headers: authorization ? { Authorization: authorization } : {} });
    emit("message_end", { message: { role: "user" } });
    emit("message_end", assistant);
    assert.equal(entries.length, 1);
    const [[type, { captured_at, ...data }]] = entries;
    assert.ok(Number.isFinite(Date.parse(captured_at)));
    assert.equal(type, "auth-provenance");
    assert.deepEqual(data, { responseId: "msg_123", provider: "anthropic", model: "served-model",
      confidence: "configuration_snapshot", billing_class: expected, source: safeSource });
  }
});

test("does not guess when requests overlap or reuse an earlier turn's snapshot", async () => {
  const { emit, entries } = harness(() => ({ auth: { apiKey: "synthetic" }, source: "stored credential" }));
  const request = () => emit("before_provider_headers", { headers: {} });
  await request(); // e.g., out-of-turn compaction
  emit("turn_start");
  emit("message_end", assistant);
  emit("turn_start");
  await Promise.all([request(), request()]);
  emit("message_end", assistant);
  emit("turn_start");
  await request();
  emit("message_end", { message: { ...assistant.message, provider: "openai" } });
  emit("message_end", assistant);
  emit("turn_start");
  await request();
  emit("message_end", { message: { ...assistant.message, responseId: undefined } });
  assert.deepEqual(entries, []);
});

test("returns unknown on resolver failure or a conflicting auth header", async () => {
  for (const [resolve, headers] of [
    [() => { throw new Error("secret"); }, {}],
    [() => ({ auth: { apiKey: "synthetic" }, source: "stored credential" }), { "X-API-Key": "different" }],
  ]) {
    const { emit, entries } = harness(resolve);
    emit("turn_start");
    await emit("before_provider_headers", { headers });
    emit("message_end", assistant);
    assert.equal(entries[0][1].billing_class, "unknown");
    assert.ok(!JSON.stringify(entries).includes("secret"));
  }
});
