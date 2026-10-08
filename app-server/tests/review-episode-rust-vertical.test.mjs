import assert from "node:assert/strict";
import { fork } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";

const binary = process.env.WORK_ENGINE_REVIEW_EPISODE_TEST_BINARY;
const compiler = process.env.WORK_ENGINE_COMPILER_TEST_BINARY;
const selectedTest = binary && compiler ? test : test.skip;
const root = path.resolve(new URL("../..", import.meta.url).pathname);
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");

for (const variant of ["legacy_legacy", "c3_only", "r3_only", "both"]) selectedTest(
  `real OS host ${variant} composes the capability route, claims, maximum and negative paths`, async () => {
  const child = fork(path.join(root, "app-server/tests/fixtures/review-episode-rust/native-host-driver.mjs"), [], {
    execArgv: [], env: {...process.env, WORK_ENGINE_HOST_VARIANT: variant,
      WORK_ENGINE_HOST_COMPILER_BINARY: compiler,
      WORK_ENGINE_HOST_COMPILER_SHA256: sha(readFileSync(compiler)),
      WORK_ENGINE_HOST_EPISODE_BINARY: binary,
      WORK_ENGINE_HOST_EPISODE_SHA256: sha(readFileSync(binary))},
    stdio: ["ignore", "ignore", "pipe", "ipc"],
  });
  let stderr = "";
  child.stderr.on("data", (chunk) => { stderr += chunk.toString(); });
  const waitFor = (predicate) => new Promise((resolve, reject) => {
    const timer = setTimeout(() => {child.off("message", onMessage); reject(new Error("host response timeout"));}, 30_000);
    const onMessage = (message) => {
      if (predicate(message)) {clearTimeout(timer); child.off("message", onMessage); resolve(message);}
    };
    child.on("message", onMessage);
    child.once("exit", (code) => {clearTimeout(timer); reject(new Error(`host exited ${code}: ${stderr}`));});
  });
  try {
    const ready = await waitFor((message) => message?.type === "ready");
    assert.equal(ready.variant, variant);
    assert.equal(ready.corpus.hostProviderCalls, 1);
    assert.equal(ready.corpus.maximumHistoryBytes, 31_207_216);
    assert.equal(ready.corpus.maximumHistoryRows, 4);
    const send = async (id, kind) => {
      const waiting = waitFor((message) => message?.type === "work" && message.id === id);
      child.send({type: "work", id, kind});
      return waiting;
    };
    const normal = await send("normal", "normal");
    assert.equal(normal.outcome, "ok", JSON.stringify(normal));
    assert.deepEqual(normal.components.map(({name}) => name),
      ["capability_recover_claims_and_episode_exact_read",
        "compiler_projection_and_two_satisfactions", "episode_exact_read"]);
    const maximum = await send("maximum", "maximum");
    assert.equal(maximum.components[0].outcome, "ok", JSON.stringify(maximum));
    assert.deepEqual(maximum.components.map(({name}) => name),
      ["capability_recover_claims_and_episode_exact_read",
        "compiler_max_projection_and_six_satisfactions", "episode_near_ceiling_history"]);
    if (maximum.outcome === "error") {
      assert.equal(maximum.components[2].outcome, "error");
      assert.ok(maximum.components[2].error.transport?.timedOut === true
        || /refus|ceiling/i.test(maximum.components[2].error.message), JSON.stringify(maximum));
    }
    for (let i = 0; i < 4; i++) {
      const negative = await send(`negative-${i}`, "error_contention");
      assert.equal(negative.outcome, "error");
      assert.equal(negative.caseName, ["compiler_wrong_hash", "episode_wrong_scope",
        "sqlite_busy_write", "compiler_100ms_timeout"][i]);
      assert.notEqual(negative.error?.kind, "unexpected_success", JSON.stringify(negative));
      assert.equal(negative.components[0].outcome, "ok", JSON.stringify(negative));
    }
  } finally {
    if (child.connected) child.send({type: "close"});
    await new Promise((resolve) => { if (child.exitCode !== null) resolve(); else child.once("exit", resolve); });
  }
});
