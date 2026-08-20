import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import test from "node:test";
import { createFakeChrome } from "../../chrome-vision/tests/fixtures/fake-chrome.mjs";

const repository = path.resolve(import.meta.dirname, "../../../..");
const preflightCli = path.join(repository, "work-engine/skills/slice-supervisor/scripts/campaign-preflight.mjs");
const chromeCli = path.join(repository, "work-engine/skills/chrome-vision/scripts/chrome-vision.mjs");

function run(command, args, { cwd, input = "" }) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, stdio: ["pipe", "pipe", "pipe"] });
    let stdout = ""; let stderr = "";
    child.stdout.on("data", chunk => { stdout += chunk; });
    child.stderr.on("data", chunk => { stderr += chunk; });
    child.on("error", reject);
    child.on("close", code => code === 0 ? resolve(stdout) : reject(new Error(`${stderr}\n${stdout}`)));
    child.stdin.end(input);
  });
}

test("campaign preflight resolves external Chrome YAML into a cwd-independent NDJSON broker startup", async t => {
  const chrome = await createFakeChrome();
  t.after(() => chrome.close());
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "site2json-campaign-"));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  const campaignDirectory = path.join(root, "campaign");
  const configDirectory = path.join(campaignDirectory, "config");
  const otherDirectory = path.join(root, "elsewhere");
  await fs.mkdir(configDirectory, { recursive: true });
  await fs.mkdir(otherDirectory);
  await fs.writeFile(path.join(configDirectory, "chrome.yaml"), `version: 1\nendpoint: ${chrome.endpoint}\nartifactDirectory: artifacts\ntargetAliases:\n  page:\n    type: page\n`);
  await fs.writeFile(path.join(campaignDirectory, "campaign.yaml"), "version: 2\nobjective: test\ncapabilities:\n  chrome_vision:\n    config: config/chrome.yaml\n");

  const preflight = JSON.parse(await run(process.execPath, [preflightCli, "campaign/campaign.yaml"], { cwd: root }));
  assert.equal(preflight.engineConfig.capabilities.chrome_vision.source, "file");
  assert.equal(preflight.engineConfig.capabilities.chrome_vision.authoredReference, "config/chrome.yaml");
  assert.match(preflight.engineConfig.capabilities.chrome_vision.sha256, /^[a-f0-9]{64}$/);
  assert.equal(preflight.resolvedCapabilities.chrome_vision.artifactDirectory, path.join(configDirectory, "artifacts"));
  assert.equal("config" in preflight.engineConfig.capabilities.chrome_vision, false, "durable provenance excludes expanded runtime config");

  const output = await run(process.execPath, [chromeCli, path.join(configDirectory, "chrome.yaml")], {
    cwd: otherDirectory,
    input: `${JSON.stringify({ operation: "chrome.targets.list" })}\n`,
  });
  const packet = JSON.parse(output.trim());
  assert.equal(packet.operation, "chrome.targets.list", JSON.stringify(packet));
  assert.ok(Array.isArray(packet.evidence));
});

test("campaign preflight supports inline config and rejects unknown capability fields", async t => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "site2json-inline-campaign-"));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  await fs.writeFile(path.join(root, "inline.yaml"), "version: 2\nobjective: test\ncapabilities:\n  chrome_vision:\n    config:\n      version: 1\n      endpoint: http://127.0.0.1:9222\n      artifactDirectory: artifacts\n");
  const result = JSON.parse(await run(process.execPath, [preflightCli, "inline.yaml"], { cwd: root }));
  assert.equal(result.engineConfig.capabilities.chrome_vision.source, "inline");
  assert.equal(result.resolvedCapabilities.chrome_vision.artifactDirectory, path.join(root, "artifacts"));
  await fs.writeFile(path.join(root, "invalid.yaml"), "version: 2\nobjective: test\ncapabilities:\n  chrome_vision:\n    config: chrome.yaml\n    required: true\n");
  await assert.rejects(run(process.execPath, [preflightCli, "invalid.yaml"], { cwd: root }), /must contain only config/);
  await fs.writeFile(path.join(root, "unknown.yaml"), "version: 2\nobjective: test\ncapabilities:\n  imaginary_browser: {}\n");
  await assert.rejects(run(process.execPath, [preflightCli, "unknown.yaml"], { cwd: root }), /Unknown capabilities: imaginary_browser/);
});

test("campaign preflight routes comparative definitions without changing kindless engineering campaigns", async t => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "site2json-campaign-kind-"));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  await fs.writeFile(path.join(root, "comparative.yaml"), "kind: comparative_repository_analysis\nconfiguration_version: 1\n");
  await assert.rejects(
    run(process.execPath, [preflightCli, "comparative.yaml"], { cwd: root }),
    /Comparative repository analysis definitions are not slice-supervisor campaigns\. Run: python3 work-engine\/skills\/comparative-repository-analysis\/scripts\/comparison_artifacts\.py prepare-run --config comparative\.yaml --output comparison-contract\.json/,
  );
  await fs.writeFile(path.join(root, "engineering.yaml"), "version: 2\nobjective: test\n");
  const engineering = JSON.parse(await run(process.execPath, [preflightCli, "engineering.yaml"], { cwd: root }));
  assert.equal(engineering.engineConfig.version, 2);
  assert.equal(engineering.engineConfig.objective, "test");
});
