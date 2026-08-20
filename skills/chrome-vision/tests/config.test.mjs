import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { loadChromeVisionConfig, resolveChromeVisionConfig } from "../src/config.mjs";

test("inline and file-authored configs normalize paths from their authoring source", async t => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "chrome-config-"));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  const value = { version: 1, endpoint: "http://127.0.0.1:9222", artifactDirectory: "artifacts" };
  await fs.writeFile(path.join(root, "chrome.yaml"), "version: 1\nendpoint: http://127.0.0.1:9222\nartifactDirectory: artifacts\n");
  const file = await loadChromeVisionConfig("chrome.yaml", { cwd: root });
  const inline = resolveChromeVisionConfig(value, { baseDirectory: root, source: "campaign.yaml" });
  assert.deepEqual(file.config, inline.config);
  assert.equal(file.config.artifactDirectory, path.join(root, "artifacts"));
});

test("parsed YAML is rejected by config-v1 JSON Schema before broker startup", async t => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "chrome-config-invalid-"));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  await fs.writeFile(path.join(root, "chrome.yaml"), "version: 1\nendpoint: http://127.0.0.1:9222\nlimits:\n  events: 0\nunexpected: true\n");
  await assert.rejects(loadChromeVisionConfig("chrome.yaml", { cwd: root }), /unexpected is not allowed.*events must be at least 1/);
});
