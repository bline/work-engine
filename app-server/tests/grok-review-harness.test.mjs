import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import process from "node:process";
import test from "node:test";
import { promisify } from "node:util";

import { buildGrokReviewPacket } from "../experiments/grok-review-harness/packet.mjs";
import { inspectAgyStream, runAgyReview } from "../experiments/grok-review-harness/agy-runner.mjs";
import { runGrokReview } from "../experiments/grok-review-harness/runner.mjs";
import { validateGrokTraceEvents } from "../experiments/grok-review-harness/trace.mjs";

const execFileAsync = promisify(execFile);

async function git(root, ...args) {
  return (await execFileAsync("git", args, { cwd: root, encoding: "utf8" })).stdout.trim();
}

async function repositoryFixture() {
  const root = await mkdtemp(path.join(tmpdir(), "grok-review-repo-"));
  await git(root, "init", "--quiet");
  await git(root, "config", "user.email", "test@example.invalid");
  await git(root, "config", "user.name", "Review Harness Test");
  await mkdir(path.join(root, "src"));
  await mkdir(path.join(root, "test"));
  await writeFile(path.join(root, "src", "service.mjs"), "export const value = 1;\n");
  await writeFile(path.join(root, "test", "service.test.mjs"), "// baseline\n");
  await writeFile(path.join(root, "DESIGN.md"), "# Contract\nPreserve truth.\n");
  await git(root, "add", ".");
  await git(root, "commit", "--quiet", "-m", "base");
  const baseCommit = await git(root, "rev-parse", "HEAD");

  await writeFile(path.join(root, "src", "service.mjs"), "export const value = 2;\nexport const unsafe = true;\n");
  await writeFile(path.join(root, "test", "service.test.mjs"), "// candidate\n");
  await git(root, "add", ".");
  await git(root, "commit", "--quiet", "-m", "candidate");
  const commit = await git(root, "rev-parse", "HEAD");
  const tree = await git(root, "show", "-s", "--format=%T", commit);
  const patch = (await execFileAsync("git", [
    "diff", "--no-ext-diff", "--binary", baseCommit, commit, "--",
    "src/service.mjs", "test/service.test.mjs",
  ], { cwd: root, encoding: "buffer" })).stdout;
  const patchIdentity = createHash("sha256").update(patch).digest("hex");

  return {
    root,
    reviewCase: {
      subject: { commit, tree, patch_identity: patchIdentity },
      base_commit: baseCommit,
      changed_files: ["src/service.mjs", "test/service.test.mjs"],
      dependency_files: [],
      contract_files: ["DESIGN.md"],
      focus: ["Preserve truth"],
    },
    async dispose() { await rm(root, { recursive: true, force: true }); },
  };
}

function responseFor(packet, evidence = { path: "src/service.mjs", start_line: 2, end_line: 2 }) {
  return JSON.stringify({
    subject: { ...packet.manifest.subject },
    verdict: "remediation_required",
    findings: [{
      id: "GR-001",
      severity: "medium",
      title: "Unsafe state is exposed",
      evidence: [evidence],
      observed: "The candidate exports an unsafe marker.",
      violated_expectation: "The contract requires preserved truth.",
      consequence: "Consumers can observe an invalid state.",
      reproduced_or_inferred: "inferred",
      confidence: "high",
      recommended_remediation: "Remove or fence the unsafe marker.",
    }],
    decisive_evidence: [{ ...evidence, reason: "The changed export creates the reviewed risk." }],
    limitations: ["Tests were not executed by the reviewer."],
    metrics: {
      files_considered: 3,
      findings_by_severity: { blocker: 0, high: 0, medium: 1, low: 0, info: 0 },
    },
  });
}

async function admittedTrace() {
  return {
    status: "verified",
    session_id: "fake",
    archive_path: "/tmp/fake-trace.tar.gz",
    archive_sha256: "0".repeat(64),
    tool_counts: { read_file: 1, grep: 0, list_dir: 0 },
    directly_read_files: 1,
  };
}

test("packet construction binds exact Git objects and excludes repository instructions", async () => {
  const fixture = await repositoryFixture();
  const packet = await buildGrokReviewPacket({ repositoryRoot: fixture.root, reviewCase: fixture.reviewCase });
  try {
    assert.equal(packet.manifest.subject.commit, fixture.reviewCase.subject.commit);
    assert.equal(packet.manifest.subject.tree, fixture.reviewCase.subject.tree);
    assert.equal(packet.manifest.files.length, 3);
    assert.match(await readFile(path.join(packet.root, "change.patch"), "utf8"), /unsafe = true/);
    await assert.rejects(readFile(path.join(packet.root, "AGENTS.md"), "utf8"));
  } finally {
    await packet.dispose();
    await fixture.dispose();
  }
});

test("runner admits one exact cited review and binds citation bytes", async () => {
  const fixture = await repositoryFixture();
  const packet = await buildGrokReviewPacket({ repositoryRoot: fixture.root, reviewCase: fixture.reviewCase });
  try {
    const result = await runGrokReview({
      packet,
      command: process.execPath,
      commandArgsPrefix: [path.resolve("app-server/tests/fixtures/fake-grok-cli.mjs")],
      env: { ...process.env, FAKE_GROK_RESPONSE: responseFor(packet) },
      timeoutMs: 5_000,
      traceInspector: admittedTrace,
    });
    assert.equal(result.status, "completed");
    assert.equal(result.receipt.verdict, "remediation_required");
    assert.match(result.receipt.findings[0].evidence[0].evidence_sha256, /^[0-9a-f]{64}$/);
  } finally {
    await packet.dispose();
    await fixture.dispose();
  }
});

test("runner rejects fabricated citations outside the packet", async () => {
  const fixture = await repositoryFixture();
  const packet = await buildGrokReviewPacket({ repositoryRoot: fixture.root, reviewCase: fixture.reviewCase });
  try {
    const result = await runGrokReview({
      packet,
      command: process.execPath,
      commandArgsPrefix: [path.resolve("app-server/tests/fixtures/fake-grok-cli.mjs")],
      env: {
        ...process.env,
        FAKE_GROK_RESPONSE: responseFor(packet, { path: "src/invented.mjs", start_line: 1, end_line: 1 }),
      },
      timeoutMs: 5_000,
      traceInspector: admittedTrace,
    });
    assert.equal(result.status, "rejected_output");
    assert.match(result.reason, /not in the packet manifest/);
  } finally {
    await packet.dispose();
    await fixture.dispose();
  }
});

test("runner classifies provider quota without manufacturing a review", async () => {
  const fixture = await repositoryFixture();
  const packet = await buildGrokReviewPacket({ repositoryRoot: fixture.root, reviewCase: fixture.reviewCase });
  try {
    const result = await runGrokReview({
      packet,
      command: process.execPath,
      commandArgsPrefix: [path.resolve("app-server/tests/fixtures/fake-grok-cli.mjs")],
      env: { ...process.env, FAKE_GROK_ERROR: "quota" },
      timeoutMs: 5_000,
      traceInspector: admittedTrace,
    });
    assert.equal(result.status, "provider_quota");
    assert.equal("receipt" in result, false);
  } finally {
    await packet.dispose();
    await fixture.dispose();
  }
});

test("trace admission requires a completed direct read for every cited file", async () => {
  const fixture = await repositoryFixture();
  const packet = await buildGrokReviewPacket({ repositoryRoot: fixture.root, reviewCase: fixture.reviewCase });
  const citedPath = path.join(packet.root, "evidence", "src", "service.mjs");
  const receipt = {
    findings: [{ evidence: [{ path: "src/service.mjs", start_line: 2, end_line: 2 }] }],
    decisive_evidence: [],
  };
  try {
    const updatesText = [
      JSON.stringify({ params: { update: {
        sessionUpdate: "tool_call",
        toolCallId: "call-1",
        title: "read_file",
        rawInput: { target_file: citedPath },
      } } }),
      JSON.stringify({ params: { update: {
        sessionUpdate: "tool_call_update",
        toolCallId: "call-1",
        status: "completed",
      } } }),
    ].join("\n");
    const trace = validateGrokTraceEvents({ packet, receipt, updatesText });
    assert.equal(trace.tool_counts.read_file, 1);

    assert.throws(() => validateGrokTraceEvents({
      packet,
      receipt,
      updatesText: updatesText.replace(citedPath, path.join(packet.root, "manifest.json")),
    }), /does not show a direct read/);
  } finally {
    await packet.dispose();
    await fixture.dispose();
  }
});

test("trace admission rejects undeclared tools and paths outside the packet", async () => {
  const fixture = await repositoryFixture();
  const packet = await buildGrokReviewPacket({ repositoryRoot: fixture.root, reviewCase: fixture.reviewCase });
  const receipt = { findings: [], decisive_evidence: [] };
  try {
    const undeclared = JSON.stringify({ params: { update: {
      sessionUpdate: "tool_call",
      toolCallId: "call-1",
      title: "use_tool",
      rawInput: {},
    } } });
    assert.throws(() => validateGrokTraceEvents({ packet, receipt, updatesText: undeclared }), /undeclared tool/);

    const escaped = [
      JSON.stringify({ params: { update: {
        sessionUpdate: "tool_call",
        toolCallId: "call-2",
        title: "read_file",
        rawInput: { target_file: "/etc/passwd" },
      } } }),
      JSON.stringify({ params: { update: {
        sessionUpdate: "tool_call_update",
        toolCallId: "call-2",
        status: "completed",
      } } }),
    ].join("\n");
    assert.throws(() => validateGrokTraceEvents({ packet, receipt, updatesText: escaped }), /escapes the immutable packet/);
  } finally {
    await packet.dispose();
    await fixture.dispose();
  }
});

test("Agy runner admits a Gemini Flash review with directly read citations", async () => {
  const fixture = await repositoryFixture();
  const packet = await buildGrokReviewPacket({ repositoryRoot: fixture.root, reviewCase: fixture.reviewCase });
  try {
    const result = await runAgyReview({
      packet,
      command: process.execPath,
      commandArgsPrefix: [path.resolve("app-server/tests/fixtures/fake-agy-cli.mjs")],
      env: {
        ...process.env,
        FAKE_AGY_RESPONSE: responseFor(packet),
        FAKE_AGY_READ_PATH: path.join(packet.root, "evidence", "src", "service.mjs"),
      },
      timeoutMs: 5_000,
    });
    assert.equal(result.status, "completed");
    assert.equal(result.receipt.provider, "agy-gemini");
    assert.equal(result.receipt.trace.model, "gemini-3.7-flash-high");
  } finally {
    await packet.dispose();
    await fixture.dispose();
  }
});

test("Agy runner rejects Pro models without explicit opt-in", async () => {
  const fixture = await repositoryFixture();
  const packet = await buildGrokReviewPacket({ repositoryRoot: fixture.root, reviewCase: fixture.reviewCase });
  try {
    await assert.rejects(
      runAgyReview({ packet, model: "gemini-3.1-pro-high" }),
      /must be Gemini Flash unless Pro is explicitly allowed/,
    );
  } finally {
    await packet.dispose();
    await fixture.dispose();
  }
});

test("Agy runner admits an explicitly authorized Pro execution profile", async () => {
  const fixture = await repositoryFixture();
  const packet = await buildGrokReviewPacket({ repositoryRoot: fixture.root, reviewCase: fixture.reviewCase });
  try {
    const result = await runAgyReview({
      packet,
      command: process.execPath,
      commandArgsPrefix: [path.resolve("app-server/tests/fixtures/fake-agy-cli.mjs")],
      env: {
        ...process.env,
        FAKE_AGY_RESPONSE: responseFor(packet),
        FAKE_AGY_READ_PATH: path.join(packet.root, "evidence", "src", "service.mjs"),
      },
      model: "gemini-3.1-pro-high",
      allowPro: true,
      timeoutMs: 5_000,
    });
    assert.equal(result.status, "completed");
    assert.equal(result.receipt.trace.model, "gemini-3.1-pro-high");
  } finally {
    await packet.dispose();
    await fixture.dispose();
  }
});

test("Agy trace admission rejects undeclared tools", async () => {
  const packet = { root: "/tmp/agy-packet" };
  const receipt = { findings: [], decisive_evidence: [] };
  const stdout = [
    JSON.stringify({
      type: "system", subtype: "init", model: "gemini-3.7-flash-high",
      agent: "work-engine-evidence-reviewer", tools: ["view_file", "grep_search", "finish"],
    }),
    JSON.stringify({ type: "step_update", step_type: "tool", tool_name: "run_command", tool_info: { parameters: {} } }),
    JSON.stringify({ type: "result", status: "SUCCESS", conversation_id: "x" }),
  ].join("\n");
  assert.throws(() => inspectAgyStream({
    packet,
    receipt,
    stdout,
    expectedModel: "gemini-3.7-flash-high",
  }), /undeclared tool/);
});

test("Agy trace admission rejects searches outside the immutable project", () => {
  const packet = { root: "/tmp/agy-packet" };
  const receipt = { findings: [], decisive_evidence: [] };
  const stdout = [
    JSON.stringify({ event: "init", init: {
      model: "gemini-3.7-flash-high",
      agent: "work-engine-evidence-reviewer",
      tools: ["view_file", "grep_search", "finish"],
    } }),
    JSON.stringify({ event: "step_update", step_update: {
      step_type: "tool",
      tool_name: "find_by_name",
      tool_info: { parameters: { SearchDirectory: "/" }, output: "" },
    } }),
    JSON.stringify({ event: "result", result: { status: "SUCCESS", conversation_id: "x" } }),
  ].join("\n");
  assert.throws(() => inspectAgyStream({
    packet,
    receipt,
    stdout,
    expectedModel: "gemini-3.7-flash-high",
  }), /escapes the immutable packet/);
});

test("Agy trace normalizes provider-quoted direct-read paths", async () => {
  const fixture = await repositoryFixture();
  const packet = await buildGrokReviewPacket({ repositoryRoot: fixture.root, reviewCase: fixture.reviewCase });
  const response = JSON.parse(responseFor(packet));
  const receipt = {
    findings: response.findings,
    decisive_evidence: response.decisive_evidence,
  };
  const cited = path.join(packet.root, "evidence", "src", "service.mjs");
  const stdout = [
    JSON.stringify({
      type: "system", subtype: "init", model: "gemini-3.7-flash-high",
      agent: "work-engine-evidence-reviewer", tools: ["view_file", "grep_search", "finish"],
    }),
    JSON.stringify({
      type: "step_update",
      step_type: "tool",
      tool_name: "view_file",
      tool_info: { parameters: { AbsolutePath: JSON.stringify(cited) }, output: "bytes" },
    }),
    JSON.stringify({ type: "result", status: "SUCCESS", conversation_id: "quoted" }),
  ].join("\n");
  try {
    const trace = inspectAgyStream({ packet, receipt, stdout, expectedModel: "gemini-3.7-flash-high" });
    assert.equal(trace.directly_read_files, 1);
  } finally {
    await packet.dispose();
    await fixture.dispose();
  }
});
