import assert from "node:assert/strict";
import test from "node:test";

import {isChildTimeout, summarizeChildTimeouts} from "./fixtures/review-episode-rust/timeout-cause.mjs";

test("HOST raw sample accounting retains both C3 and R3 child timeouts", () => {
  const measured = [
    {components: [{name: "compiler_100ms_timeout", error: {kind: "timeout", transport: null}}]},
    {components: [{name: "episode_near_ceiling_history", error: {kind: "Transport",
      transport: {code: "ETIMEDOUT", timedOut: true}}}]},
    {error: {kind: "Transport", transport: {code: "ETIMEDOUT", timedOut: true}}, components: []},
    {components: [{name: "compiler_wrong_hash", error: {kind: "Selection", transport: null}}]},
  ];
  assert.equal(isChildTimeout(measured[0].components[0].error), true);
  assert.equal(isChildTimeout(measured[1].components[0].error), true);
  assert.equal(isChildTimeout(measured[3].components[0].error), false);
  assert.deepEqual(summarizeChildTimeouts(measured), {timedOutOperations: 3,
    timedOutComponents: {compiler_100ms_timeout: 1, episode_near_ceiling_history: 1}});
});
