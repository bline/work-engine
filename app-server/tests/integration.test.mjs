import assert from "node:assert/strict";
import test from "node:test";

import {
  PINNED_PROTOCOL,
  StdioJsonRpcTransport,
  assertCompatibleServer,
} from "../src/index.mjs";

const enabled = process.env.WORK_ENGINE_APP_SERVER_INTEGRATION === "1";

test("pinned Codex App Server completes the initialize handshake", { skip: !enabled }, async (t) => {
  const transport = StdioJsonRpcTransport.spawn();
  t.after(() => transport.close());
  const response = await transport.request("initialize", {
    clientInfo: {
      name: "work-engine-integration-test",
      title: "Work Engine Integration Test",
      version: "0.1.0",
    },
    capabilities: { experimentalApi: false, requestAttestation: false },
  });
  assert.equal(assertCompatibleServer(response), PINNED_PROTOCOL.codexCliVersion);
  transport.notify("initialized");
  assert.equal(response.platformFamily.length > 0, true);
});
