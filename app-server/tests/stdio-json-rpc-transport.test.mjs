import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import test from "node:test";

import { StdioJsonRpcTransport } from "../src/index.mjs";

function childProcessFixture() {
  const child = new EventEmitter();
  child.stdin = new PassThrough();
  child.stdout = new PassThrough();
  child.stderr = new PassThrough();
  return child;
}

test("spawn readiness resolves only after the App Server child exists", async () => {
  const child = childProcessFixture();
  const transport = new StdioJsonRpcTransport(child);
  let ready = false;
  transport.ready().then(() => { ready = true; });

  await Promise.resolve();
  assert.equal(ready, false);
  child.emit("spawn");
  await transport.ready();
  assert.equal(ready, true);

  transport.close();
});
