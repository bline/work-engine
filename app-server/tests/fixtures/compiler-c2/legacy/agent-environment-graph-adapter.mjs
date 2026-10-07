import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";

const BACKEND = "work-engine.agent-environment-graph.v1";
const DIGEST = /^[0-9a-f]{64}$/;
const ENVELOPE_FIELDS = new Set([
  "schema_version", "status", "backend", "backend_sha256",
  "canonical_role_match", "projection",
]);

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function exactFields(value, allowed, label) {
  record(value, label);
  const unknown = Object.keys(value).filter((field) => !allowed.has(field));
  if (unknown.length > 0) throw new TypeError(`${label} contains unknown field ${unknown[0]}`);
}

function invoke(command, args, { input, timeout = 30_000, maxBuffer = 2 * 1024 * 1024 } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: ["pipe", "pipe", "pipe"] });
    let stdout = Buffer.alloc(0);
    let stderr = Buffer.alloc(0);
    let settled = false;
    let timer = null;
    const finish = (error, result) => {
      if (settled) return;
      settled = true;
      if (timer !== null) clearTimeout(timer);
      if (error) reject(error);
      else resolve(result);
    };
    const append = (current, chunk, stream) => {
      if (settled) return current;
      const next = Buffer.concat([current, chunk]);
      if (next.length > maxBuffer) {
        child.kill("SIGKILL");
        finish(new Error(`Agent Environment Graph ${stream} exceeded ${maxBuffer} bytes`));
      }
      return next;
    };
    child.stdout.on("data", (chunk) => { stdout = append(stdout, chunk, "stdout"); });
    child.stderr.on("data", (chunk) => { stderr = append(stderr, chunk, "stderr"); });
    child.on("error", (error) => finish(error));
    child.stdin.on("error", (error) => finish(error));
    child.on("close", (code, signal) => finish(null, {
      code, signal, stdout: stdout.toString("utf8"), stderr: stderr.toString("utf8"),
    }));
    timer = setTimeout(() => {
      child.kill("SIGKILL");
      finish(new Error(`Agent Environment Graph projection timed out after ${timeout}ms`));
    }, timeout);
    try {
      child.stdin.end(input);
    } catch (error) {
      finish(error);
    }
  });
}

function validateEnvelope(value, expectedBackendSha256) {
  exactFields(value, ENVELOPE_FIELDS, "Agent Environment Graph envelope");
  if (value.schema_version !== 1) throw new TypeError("Agent Environment Graph envelope schema_version must be 1");
  if (value.status !== "closed_projection") throw new TypeError("Agent Environment Graph did not return a closed projection");
  if (value.backend !== BACKEND) throw new TypeError("Agent Environment Graph backend identity mismatch");
  if (!DIGEST.test(value.backend_sha256) || value.backend_sha256 !== expectedBackendSha256) {
    throw new TypeError("Agent Environment Graph backend digest mismatch");
  }
  if (value.canonical_role_match !== true) throw new TypeError("Agent Environment Graph did not establish canonical role equality");
  record(value.projection, "Agent Environment Graph projection");
  if (value.projection.schema_version !== 1 || value.projection.status !== "generated_projection") {
    throw new TypeError("Agent Environment Graph projection contract is invalid");
  }
  return value;
}

export function createAgentEnvironmentGraphAdapter({
  workspaceRoot,
  scriptPath = "skills/agent-environment-graph/scripts/agent_environment_graph.py",
  invariantsPath = "docs/workflow-invariants.md",
  environmentsPath = "docs/agent-environments.yaml",
  invokeProcess = invoke,
} = {}) {
  const root = path.resolve(workspaceRoot ?? process.cwd());
  const script = path.resolve(root, scriptPath);
  const invariants = path.resolve(root, invariantsPath);
  const environments = path.resolve(root, environmentsPath);

  return Object.freeze({
    async projectRole({ roleId, role }) {
      if (typeof roleId !== "string" || roleId.length === 0) throw new TypeError("roleId is required");
      record(role, "candidate role");
      const request = JSON.stringify({ schema_version: 1, role_id: roleId, role });
      const expectedBackendSha256 = sha256(await readFile(script));
      const result = await invokeProcess("python3", [
        script, "project-role-machine", "--invariants", invariants,
        "--environments", environments,
      ], { input: request, timeout: 30_000, maxBuffer: 2 * 1024 * 1024 });
      if (!result || result.code !== 0 || result.signal) {
        const detail = result?.stderr?.trim() || `exit ${result?.code ?? "unknown"}`;
        throw new Error(`Agent Environment Graph projection failed: ${detail}`);
      }
      if (result.stderr !== "") throw new Error("Agent Environment Graph wrote stderr on success");
      let envelope;
      try {
        envelope = JSON.parse(result.stdout);
      } catch (error) {
        throw new TypeError(`Agent Environment Graph returned invalid JSON: ${error.message}`);
      }
      return validateEnvelope(envelope, expectedBackendSha256);
    },
  });
}

export const agentEnvironmentGraphAdapterInternals = Object.freeze({ validateEnvelope, invoke, sha256 });
