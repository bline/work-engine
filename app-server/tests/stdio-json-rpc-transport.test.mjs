import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import test from "node:test";

import {
  PINNED_PROTOCOL,
  StdioJsonRpcTransport,
  assertCompatibleCodexCliOutput,
} from "../src/index.mjs";

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

test("the pinned Codex CLI version is parsed strictly", () => {
  assert.equal(
    assertCompatibleCodexCliOutput(`codex-cli ${PINNED_PROTOCOL.codexCliVersion}\n`),
    PINNED_PROTOCOL.codexCliVersion,
  );
  assert.throws(
    () => assertCompatibleCodexCliOutput("codex-cli 9.9.9\n"),
    /does not match pinned 0\.149\.1/,
  );
  assert.throws(() => assertCompatibleCodexCliOutput("unexpected output"), /unknown/);
});

test("child stderr is observable without entering protocol stdout", async () => {
  const child = childProcessFixture();
  const transport = new StdioJsonRpcTransport(child);
  const stderr = [];
  const notifications = [];
  transport.on("stderr", (chunk) => stderr.push(chunk));
  transport.onNotification((message) => notifications.push(message));

  child.stderr.write("bounded child diagnostic\n");
  child.stdout.write('{"method":"ready"}\n');
  await new Promise((resolve) => setImmediate(resolve));

  assert.deepEqual(stderr, ["bounded child diagnostic\n"]);
  assert.deepEqual(notifications, [{ method: "ready" }]);
  transport.close();
});
