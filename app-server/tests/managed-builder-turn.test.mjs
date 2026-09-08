import assert from "node:assert/strict";
import test from "node:test";

import { createManagedBuilderTurnHandler } from "../src/managed-builder-turn.mjs";

test("managed builder capability hard-binds one slice-builder role turn", async () => {
  const turns = [];
  const handler = createManagedBuilderTurnHandler(async (turn) => {
    turns.push(turn);
    return {delivery: {logicalRoleInstanceId: `${turn.roleId}:${turn.instanceId}`},
      completion: {outputText: "complete"}};
  });
  const result = await handler({
    instance_id: "s13-attempt-10",
    client_user_message_id: "supervisor:s13:attempt-10:implementation",
    text: "Perform the exact accepted implementation turn.",
  });
  assert.deepEqual(turns, [{
    roleId: "slice-builder",
    instanceId: "s13-attempt-10",
    clientUserMessageId: "supervisor:s13:attempt-10:implementation",
    text: "Perform the exact accepted implementation turn.",
  }]);
  assert.equal(result.delivery.logicalRoleInstanceId, "slice-builder:s13-attempt-10");
});

test("managed builder capability rejects identity widening before delivery", async () => {
  let deliveries = 0;
  const handler = createManagedBuilderTurnHandler(async () => { deliveries += 1; });
  await assert.rejects(handler({
    role_id: "slice-supervisor",
    instance_id: "migration",
    client_user_message_id: "attempt",
    text: "escape",
  }), /unsupported fields: role_id/);
  await assert.rejects(handler({
    instance_id: "invalid:instance",
    client_user_message_id: "attempt",
    text: "escape",
  }), /unsupported characters/);
  assert.equal(deliveries, 0);
});
