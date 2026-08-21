import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import test from "node:test";
import { createFakeChrome } from "../../chrome-vision/tests/fixtures/fake-chrome.mjs";

const repository = path.resolve(import.meta.dirname, "../../..");
const preflightCli = path.join(repository, "skills/slice-supervisor/scripts/campaign-preflight.mjs");
const chromeCli = path.join(repository, "skills/chrome-vision/scripts/chrome-vision.mjs");

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
    /Comparative repository analysis definitions are not slice-supervisor campaigns\. Run: python3 skills\/comparative-repository-analysis\/scripts\/comparison_artifacts\.py prepare-run --config comparative\.yaml --output comparison-contract\.json/,
  );
  await fs.writeFile(path.join(root, "engineering.yaml"), "version: 2\nobjective: test\n");
  const engineering = JSON.parse(await run(process.execPath, [preflightCli, "engineering.yaml"], { cwd: root }));
  assert.equal(engineering.engineConfig.version, 2);
  assert.equal(engineering.engineConfig.objective, "test");
  assert.equal(engineering.engineConfig.source, "engineering.yaml");
  assert.equal(engineering.engineConfig.builder.skill, "slice-builder");
  assert.deepEqual(engineering.engineConfig.slice_completion_commit, { prompt: "enabled" });
  assert.deepEqual(engineering.engineConfig.explicit_fields, ["version", "objective"]);
  assert.ok(engineering.engineConfig.defaulted_fields.includes("builder"));
  assert.deepEqual(engineering.engineConfig.amendments, []);
  await fs.writeFile(path.join(root, "disabled.yaml"), "version: 2\nobjective: test\nslice_completion_commit:\n  prompt: disabled\n");
  const disabled = JSON.parse(await run(process.execPath, [preflightCli, "disabled.yaml"], { cwd: root }));
  assert.equal(disabled.engineConfig.slice_completion_commit.prompt, "disabled");
  await fs.writeFile(path.join(root, "bad-commit-policy.yaml"), "version: 2\nobjective: test\nslice_completion_commit:\n  prompt: automatic\n");
  await assert.rejects(run(process.execPath, [preflightCli, "bad-commit-policy.yaml"], { cwd: root }), /prompt: enabled or disabled/);
  const authored = await fs.readFile(path.join(root, "engineering.yaml"));
  assert.deepEqual(engineering.campaignSource, {
    source: "file",
    authoredReference: "engineering.yaml",
    resolvedPath: path.join(root, "engineering.yaml"),
    pathBase: root,
    sha256: crypto.createHash("sha256").update(authored).digest("hex"),
    schemaVersion: 1,
  });
});

test("campaign preflight preserves an approved same-model review amendment", async t => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "same-model-review-campaign-"));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  await fs.writeFile(path.join(root, "campaign.yaml"), `version: 2
objective: test
builder:
  context:
    adversarial_review:
      provider: codex
      skill: codex-adversarial-review
      model: gpt-5.6-sol
      reasoning_effort: low
      evidence_class: accepted_same_model_review
      isolation: fresh_process
amendments:
  - timestamp: 2026-08-21T00:00:00-06:00
    changed_fields: [builder.context.independent_review, builder.context.adversarial_review]
    prior_values: {review: claude}
    new_values: {review: codex}
    reason: Claude quota unavailable; accepted same-model review approved.
    human_approval: I approve Sol low as accepted same-model review.
`);
  const result = JSON.parse(await run(process.execPath, [preflightCli, "campaign.yaml"], { cwd: root }));
  assert.equal(result.engineConfig.builder.context.adversarial_review.evidence_class, "accepted_same_model_review");
  assert.equal("independent_review" in result.engineConfig.builder.context, false);
  assert.equal(result.engineConfig.amendments.length, 1);
  assert.match(result.engineConfig.amendments[0].human_approval, /I approve/);
});
