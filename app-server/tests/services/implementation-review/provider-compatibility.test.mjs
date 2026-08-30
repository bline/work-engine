import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { createImplementationReviewService } from "../../../src/services/implementation-review/service.mjs";

test("Claude, Codex, and experimental transports admit identical provider-neutral meaning", async () => {
  const result = JSON.parse(await readFile(new URL("../../fixtures/implementation-review/remediation-required.json", import.meta.url)));
  const service = createImplementationReviewService();
  const transportEnvelopes = [
    { provider: "claude", session: "claude-session", output: result },
    { provider: "codex", target: "/root/reviewer", output: result },
    { provider: "experimental-grok", trace: "trace-1", output: result },
  ];
  const admitted = transportEnvelopes.map(({ output }) => service.admit({ result: output, expectedSubject: result.subject }));
  assert.equal(new Set(admitted.map(({ resultRevision }) => resultRevision)).size, 1);
  assert.deepEqual(admitted[0].result, admitted[1].result);
  assert.equal("provider" in admitted[0].result, false);
  assert.equal("session" in admitted[0].result, false);
});
