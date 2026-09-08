import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { prepareGenerationRollover } from "../scripts/prepare-generation-rollover.mjs";

test("generation rollover copies only integrity-verified semantic context into a private fresh root", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "generation-rollover-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  const from = path.join(root, "generation-v1");
  const to = path.join(root, "generation-v2");
  await mkdir(from);
  await writeFile(path.join(from, "semantic-context.sqlite3"), "semantic-state", {mode: 0o600});
  await writeFile(path.join(from, "generation-state.json"), "do-not-copy");
  await writeFile(path.join(from, "role-bindings.json"), "do-not-copy");
  const receipt = await prepareGenerationRollover({from, to, socket: path.join(root, "stopped.sock")});
  assert.equal(await readFile(path.join(to, "semantic-context.sqlite3"), "utf8"), "semantic-state");
  assert.equal(receipt.copiedGenerationIdentity, false);
  assert.equal(receipt.copiedRoleBindings, false);
  assert.equal((await stat(to)).mode & 0o777, 0o700);
  assert.equal((await stat(path.join(to, "semantic-context.sqlite3"))).mode & 0o777, 0o600);
  await assert.rejects(prepareGenerationRollover({from, to, socket: path.join(root, "stopped.sock")}),
    /target generation root already exists/);
});

test("generation rollover refuses a live App Server socket without creating the target", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "generation-rollover-live-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  const from = path.join(root, "generation-v1");
  const to = path.join(root, "generation-v2");
  const socket = path.join(root, "app-server.sock");
  await mkdir(from);
  await writeFile(path.join(from, "semantic-context.sqlite3"), "semantic-state");
  const server = net.createServer();
  await new Promise((resolve, reject) => server.listen(socket, resolve).once("error", reject));
  t.after(() => new Promise((resolve) => server.close(resolve)));
  await assert.rejects(prepareGenerationRollover({from, to, socket}), /socket is active/);
  await assert.rejects(stat(to), {code: "ENOENT"});
});
