import assert from "node:assert/strict";
import test from "node:test";

import {
  createSupervisorCampaignHostEffectRuntime,
  SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL,
} from "../src/services/slice-campaign/host-effect-runtime.mjs";

function exactRecord(value, keys, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)
      || Object.keys(value).length !== keys.length
      || keys.some((key) => !Object.hasOwn(value, key))) {
    throw new TypeError(`${label} schema is invalid`);
  }
  return value;
}

function fixtureRuntime(calls) {
  return createSupervisorCampaignHostEffectRuntime({ registrations: [{
    capability: "capability.preflight",
    operation: "validate",
    validateInput(value) {
      exactRecord(value, ["campaign"], "preflight input");
      exactRecord(value.campaign, ["identity"], "campaign");
      if (typeof value.campaign.identity !== "string") throw new TypeError("campaign identity is invalid");
      return value;
    },
    async handler(value) {
      calls.push(value);
      return { schema_version: 1, status: "valid", identity: value.input.campaign.identity };
    },
    validateOutput(value) {
      exactRecord(value, ["schema_version", "status", "identity"], "preflight output");
      if (value.schema_version !== 1 || value.status !== "valid"
          || typeof value.identity !== "string") throw new TypeError("preflight output is invalid");
      return value;
    },
  }] });
}

const effect = (overrides = {}) => ({
  protocol: SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL,
  capability: "capability.preflight",
  operation: "validate",
  input: { campaign: { identity: "campaign-one" } },
  ...overrides,
});

test("host-effect runtime dispatches only exact typed registrations with host generation identity", async () => {
  const calls = [];
  const runtime = fixtureRuntime(calls);
  const result = await runtime.dispatch({ generationId: "generation-host-observed", effect: effect() });
  assert.deepEqual(result, { schema_version: 1, status: "valid", identity: "campaign-one" });
  assert.deepEqual(calls, [{
    generationId: "generation-host-observed",
    input: { campaign: { identity: "campaign-one" } },
  }]);
  result.identity = "caller-mutation";
  assert.equal(calls[0].input.campaign.identity, "campaign-one");
});

test("host-effect runtime refuses protocol, capability, operation, input, and output drift", async () => {
  const calls = [];
  const runtime = fixtureRuntime(calls);
  for (const invalid of [
    effect({ protocol: "work-engine.supervisor-capability-effect.v2" }),
    effect({ capability: "capability.workspace_coordination" }),
    effect({ operation: "execute" }),
    { ...effect(), extra: true },
    effect({ input: { campaign: { identity: "campaign-one" }, extra: true } }),
  ]) await assert.rejects(runtime.dispatch({ generationId: "generation-one", effect: invalid }));
  assert.equal(calls.length, 0);

  const invalidOutput = createSupervisorCampaignHostEffectRuntime({ registrations: [{
    capability: "capability.preflight", operation: "validate",
    validateInput: (value) => value,
    handler: async () => ({ status: "fabricated", extra: true }),
    validateOutput(value) { return exactRecord(value, ["status"], "output"); },
  }] });
  await assert.rejects(invalidOutput.dispatch({ generationId: "generation-one", effect: effect() }), /schema/);
});

test("host-effect runtime validates registrations and closes idempotently", async () => {
  assert.throws(() => createSupervisorCampaignHostEffectRuntime({ registrations: [{}] }), /capability/);
  const runtime = fixtureRuntime([]);
  runtime.close();
  runtime.close();
  await assert.rejects(runtime.dispatch({ generationId: "generation-one", effect: effect() }), /closed/);
});
