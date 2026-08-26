import assert from "node:assert/strict";
import { mkdtemp, rm, stat } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { WebSocket } from "ws";

import { AppServerProtocolProxy } from "../src/index.mjs";

class DelegateTransport {
  constructor() {
    this.notificationHandlers = new Set();
    this.closedHandlers = new Set();
    this.serverRequestHandler = null;
    this.requests = [];
    this.notifications = [];
  }

  onServerRequest(handler) {
    this.serverRequestHandler = handler;
  }

  onNotification(handler) {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }

  onClosed(handler) {
    this.closedHandlers.add(handler);
    return () => this.closedHandlers.delete(handler);
  }

  async request(method, params) {
    this.requests.push({ method, params });
    if (method === "fail") throw new Error("bounded failure");
    return { method, accepted: true };
  }

  notify(method, params) {
    this.notifications.push({ method, params });
  }

  emitNotification(message) {
    for (const handler of this.notificationHandlers) handler(message);
  }

  invokeServerRequest(message) {
    return this.serverRequestHandler(message);
  }

  close() {}
}

function inbox(peer) {
  const messages = [];
  const waiters = [];
  peer.on("message", (data) => {
    const message = JSON.parse(data.toString());
    const waiter = waiters.shift();
    if (waiter) waiter(message);
    else messages.push(message);
  });
  return () => {
    if (messages.length > 0) return Promise.resolve(messages.shift());
    return new Promise((resolve) => waiters.push(resolve));
  };
}

function deferred() {
  let resolve;
  const promise = new Promise((settle) => { resolve = settle; });
  return { promise, resolve };
}

async function fixture(t) {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-proxy."));
  const socketPath = path.join(directory, "app-server.sock");
  const transport = new DelegateTransport();
  const proxy = new AppServerProtocolProxy({ transport, socketPath });
  await proxy.listen();
  t.after(async () => {
    await proxy.close();
    await rm(directory, { recursive: true, force: true });
  });
  const peer = new WebSocket(`ws+unix://${socketPath}:/rpc`);
  await new Promise((resolve, reject) => {
    peer.once("open", resolve);
    peer.once("error", reject);
  });
  t.after(() => peer.close());
  return { proxy, transport, peer, nextMessage: inbox(peer), socketPath };
}

test("Unix WebSocket endpoint forwards client JSON-RPC requests and notifications", async (t) => {
  const { transport, peer, nextMessage, socketPath } = await fixture(t);

  assert.equal((await stat(socketPath)).mode & 0o777, 0o600);
  peer.send(JSON.stringify({ id: "initialize", method: "initialize", params: { clientInfo: {} } }));
  assert.deepEqual(await nextMessage(), {
    id: "initialize",
    result: { method: "initialize", accepted: true },
  });
  peer.send(JSON.stringify({ method: "initialized" }));
  peer.send(JSON.stringify({ id: 2, method: "barrier", params: {} }));
  assert.deepEqual(await nextMessage(), {
    id: 2,
    result: { method: "barrier", accepted: true },
  });

  assert.deepEqual(transport.requests, [
    { method: "initialize", params: { clientInfo: {} } },
    { method: "barrier", params: {} },
  ]);
  assert.deepEqual(transport.notifications, [{ method: "initialized", params: undefined }]);
});

test("backend notifications and server requests remain bidirectionally transparent", async (t) => {
  const { transport, peer, nextMessage } = await fixture(t);

  transport.emitNotification({ method: "thread/started", params: { thread: { id: "thread-1" } } });
  assert.deepEqual(await nextMessage(), {
    method: "thread/started",
    params: { thread: { id: "thread-1" } },
  });

  const resultPromise = transport.invokeServerRequest({
    id: 41,
    method: "item/tool/call",
    params: { threadId: "thread-1" },
  });
  assert.deepEqual(await nextMessage(), {
    id: 41,
    method: "item/tool/call",
    params: { threadId: "thread-1" },
  });
  peer.send(JSON.stringify({ id: 41, result: { success: true } }));
  assert.deepEqual(await resultPromise, { success: true });
});

test("invalid input and backend failures become bounded JSON-RPC errors", async (t) => {
  const { peer, nextMessage } = await fixture(t);

  peer.send("not JSON");
  assert.deepEqual(await nextMessage(), {
    id: null,
    error: { code: -32700, message: "Parse error" },
  });
  peer.send(JSON.stringify({ id: 9, method: "fail", params: {} }));
  assert.deepEqual(await nextMessage(), {
    id: 9,
    error: { code: -32000, message: "bounded failure" },
  });
});

test("a late backend response is discarded after its remote client disconnects", async (t) => {
  const { proxy, transport, peer } = await fixture(t);
  const entered = deferred();
  const release = deferred();
  const requestErrors = [];
  const protocolErrors = [];
  proxy.on("requestError", (event) => requestErrors.push(event));
  proxy.on("protocolError", (error) => protocolErrors.push(error));
  transport.request = async (method) => {
    entered.resolve();
    await release.promise;
    return { method, accepted: true };
  };

  peer.send(JSON.stringify({ id: 12, method: "slow", params: {} }));
  await entered.promise;
  const closed = new Promise((resolve) => peer.once("close", resolve));
  peer.close();
  await closed;
  release.resolve();
  await new Promise((resolve) => setImmediate(resolve));

  assert.deepEqual(requestErrors, []);
  assert.deepEqual(protocolErrors, []);
});
