import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import { compileRunExtensionBundle, createRunExtensionRegistry } from "../src/index.mjs";

const ROOT = path.resolve(new URL("../..", import.meta.url).pathname);
const FIXTURES = path.join(ROOT, "app-server/tests/fixtures/run-extension-bundle");
const policy = {
  workspaceRoot: ROOT,
  repositoryRevision: "2af529971c1b5660de4caad7092cd270dcd162eb",
  allowedCapabilities: ["product-development.intake.read-source"],
  allowedAdapters: ["product-development.intake.read-source"],
  allowedProviders: ["local"],
};

async function fixture(name) {
  const value = JSON.parse(await readFile(path.join(FIXTURES, name), "utf8"));
  if (!value.extends) return value;
  const base = await fixture(value.extends);
  return { ...base, ...value, run: { ...base.run, ...value.run } };
}

test("sealed extension compiles exact skill closure and creates only its run-local registry", async () => {
  const attachment = await compileRunExtensionBundle(await fixture("sealed-bundle.json"), policy);
  assert.match(attachment.sha256, /^[0-9a-f]{64}$/);
  assert.equal(attachment.compiled_skills[0].skill_id, "repo-search");
  assert.equal(attachment.run.network, "sealed");
  assert.equal(attachment.run.credentials, "none");
  const registry = createRunExtensionRegistry(attachment, new Map([
    ["product-development.intake.read-source", async (args) => ({ success: true, contentItems: [{ type: "inputText", text: args.value }] })],
  ]));
  assert.equal(registry.specs[0].name, "s12e-fixture");
  assert.equal((await registry.bridge.dispatch({ namespace: "s12e-fixture", tool: "read_fixture", arguments: { value: "sealed" } })).success, true);
  assert.equal((await registry.bridge.dispatch({ namespace: "s12e-fixture", tool: "publish", arguments: {} })).success, false);
});

test("forbidden and unmediated bundles fail before activation", async () => {
  await assert.rejects(compileRunExtensionBundle(await fixture("rejected-bundle.json"), policy),
    /prohibited authority|production authority/);
  const stale = await fixture("sealed-bundle.json");
  stale.skills[0].structure_sha256 = "0".repeat(64);
  await assert.rejects(compileRunExtensionBundle(stale, policy), /digest mismatch/);
});

test("linguistic dry-run has no publication or production authority", async () => {
  const attachment = await compileRunExtensionBundle(await fixture("linguistic-dry-run.json"), policy);
  assert.equal(attachment.run.retention, "clean");
  assert.deepEqual(attachment.effects, []);
  assert.equal(attachment.registry.some((entry) => /publish|production|claim|review/.test(entry.name)), false);
});
