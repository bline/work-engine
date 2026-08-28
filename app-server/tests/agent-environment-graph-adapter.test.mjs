import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import { parse } from "yaml";

import {
  agentEnvironmentGraphAdapterInternals,
  createAgentEnvironmentGraphAdapter,
} from "../src/agent-environment-graph-adapter.mjs";

const root = path.resolve(new URL("../..", import.meta.url).pathname);
const script = path.join(root, "skills/agent-environment-graph/scripts/agent_environment_graph.py");

async function canonicalBuilderRole() {
  const environment = parse(await readFile(path.join(root, "docs/agent-environments.yaml"), "utf8"));
  return environment.roles["role.builder"];
}

test("adapter obtains the complete canonical role projection from the Python owner", async () => {
  const adapter = createAgentEnvironmentGraphAdapter({ workspaceRoot: root });
  const result = await adapter.projectRole({ roleId: "role.builder", role: await canonicalBuilderRole() });
  assert.equal(result.status, "closed_projection");
  assert.equal(result.canonical_role_match, true);
  assert.equal(result.projection.role_id, "role.builder");
  assert.deepEqual(result.projection.role, await canonicalBuilderRole());
  assert.ok(result.projection.invariants["INV-001"]);
  assert.ok(result.projection.mechanisms);
  assert.match(result.backend_sha256, /^[0-9a-f]{64}$/);
});

test("adapter preserves fail-closed candidate parity", async () => {
  const adapter = createAgentEnvironmentGraphAdapter({ workspaceRoot: root });
  const role = await canonicalBuilderRole();
  role.objective = "Unauthorized local replacement.";
  await assert.rejects(
    adapter.projectRole({ roleId: "role.builder", role }),
    /candidate role differs from canonical role role\.builder/,
  );
});

test("adapter rejects malformed or provenance-mismatched envelopes", async () => {
  const scriptSha256 = createHash("sha256").update(await readFile(script)).digest("hex");
  const base = {
    schema_version: 1,
    status: "closed_projection",
    backend: "work-engine.agent-environment-graph.v1",
    backend_sha256: scriptSha256,
    canonical_role_match: true,
    projection: { schema_version: 1, status: "generated_projection" },
  };
  const adapter = createAgentEnvironmentGraphAdapter({
    workspaceRoot: root,
    invokeProcess: async () => ({
      code: 0, signal: null, stderr: "",
      stdout: JSON.stringify({ ...base, backend_sha256: "0".repeat(64) }),
    }),
  });
  await assert.rejects(
    adapter.projectRole({ roleId: "role.builder", role: await canonicalBuilderRole() }),
    /backend digest mismatch/,
  );

  const malformed = createAgentEnvironmentGraphAdapter({
    workspaceRoot: root,
    invokeProcess: async () => ({ code: 0, signal: null, stderr: "", stdout: "not-json" }),
  });
  await assert.rejects(
    malformed.projectRole({ roleId: "role.builder", role: await canonicalBuilderRole() }),
    /returned invalid JSON/,
  );
});

test("adapter rejects stderr and nonzero process outcomes", async () => {
  const stderrAdapter = createAgentEnvironmentGraphAdapter({
    workspaceRoot: root,
    invokeProcess: async () => ({ code: 0, signal: null, stdout: "{}", stderr: "warning" }),
  });
  await assert.rejects(
    stderrAdapter.projectRole({ roleId: "role.builder", role: await canonicalBuilderRole() }),
    /wrote stderr on success/,
  );
  const failedAdapter = createAgentEnvironmentGraphAdapter({
    workspaceRoot: root,
    invokeProcess: async () => ({ code: 2, signal: null, stdout: "", stderr: "refused" }),
  });
  await assert.rejects(
    failedAdapter.projectRole({ roleId: "role.builder", role: await canonicalBuilderRole() }),
    /projection failed: refused/,
  );
});

test("real process invocation contains spawn and early-stdin failures", async () => {
  const { invoke } = agentEnvironmentGraphAdapterInternals;
  await assert.rejects(
    invoke(path.join(root, "definitely-missing-aeg-executable"), [], {
      input: "{}", timeout: 250, maxBuffer: 1024,
    }),
    /ENOENT/,
  );
  await assert.rejects(
    invoke(process.execPath, ["-e", "process.stdin.destroy(); process.exit(0)"], {
      input: "x".repeat(8 * 1024 * 1024), timeout: 500, maxBuffer: 1024,
    }),
    /EPIPE|stream destroyed|write after end/i,
  );
});

test("real process invocation kills a timed-out child", async () => {
  const { invoke } = agentEnvironmentGraphAdapterInternals;
  const started = Date.now();
  await assert.rejects(
    invoke(process.execPath, ["-e", "setInterval(() => {}, 1000)"], {
      input: "", timeout: 50, maxBuffer: 1024,
    }),
    /timed out after 50ms/,
  );
  assert.ok(Date.now() - started < 1000);
});

test("real process invocation bounds stdout and stderr independently", async () => {
  const { invoke } = agentEnvironmentGraphAdapterInternals;
  await assert.rejects(
    invoke(process.execPath, ["-e", "process.stdout.write('x'.repeat(1024))"], {
      input: "", timeout: 500, maxBuffer: 32,
    }),
    /stdout exceeded 32 bytes/,
  );
  await assert.rejects(
    invoke(process.execPath, ["-e", "process.stderr.write('x'.repeat(1024))"], {
      input: "", timeout: 500, maxBuffer: 32,
    }),
    /stderr exceeded 32 bytes/,
  );
});
