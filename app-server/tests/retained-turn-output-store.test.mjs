import assert from "node:assert/strict";
import { mkdtemp, rm, unlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { RetainedTurnOutputStore } from "../src/retained-turn-output-store.mjs";

test("retained output is content addressed and integrity checked", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "retained-output-"));
  try {
    const store = new RetainedTurnOutputStore(root);
    const metadata = await store.put(Buffer.from("exact output"));
    assert.deepEqual(await store.put(Buffer.from("exact output")), metadata);
    const loaded = await store.read(metadata);
    assert.equal(loaded.status, "available");
    assert.equal(loaded.bytes.toString(), "exact output");
    await unlink(path.join(root, metadata.digest));
    assert.deepEqual(await store.read(metadata), {status: "unavailable", reason: "blob_missing"});
    await writeFile(path.join(root, metadata.digest), "corrupt");
    assert.deepEqual(await store.read(metadata), {status: "integrity_failed", reason: "blob_digest_mismatch"});
  } finally { await rm(root, {recursive: true, force: true}); }
});
