import { createHash, randomUUID } from "node:crypto";
import { spawn } from "node:child_process";
import { mkdtemp, readFile, writeFile, chmod, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { parseDocument } from "yaml";

const MAX_RESULT = 16 * 1024 * 1024;
const MAX_STDERR = 2 * 1024 * 1024;
const DEADLINE_MS = 60_000;
const CLEANUP_MS = 5_000;
const PRODUCER = "work-engine.skill-compiler.rust-v1";
const DEFAULT_SCRIPT_DIGEST = "24583ce14385728da3459acd848368fbb7768258bdb80dfefa3f00cef7545450";
const DIGEST = /^[0-9a-f]{64}$/;

function hash(bytes) { return createHash("sha256").update(bytes).digest("hex"); }
function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}
function exactObject(value, fields, label) {
  if (!value || typeof value !== "object" || Array.isArray(value) || Object.keys(value).length !== fields.length || fields.some((key) => !Object.hasOwn(value, key))) {
    throw new Error(`${label} has invalid fields`);
  }
}
function sourceBytes(value, label) {
  if (Buffer.isBuffer(value)) {
    const decoded = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(value);
    if (!Buffer.from(decoded).equals(value)) throw new TypeError(`${label} is not UTF-8`);
    return value;
  }
  if (typeof value !== "string") throw new TypeError(`${label} must be text or Buffer`);
  const bytes = Buffer.from(value);
  if (bytes.toString("utf8") !== value) throw new TypeError(`${label} contains unsupported Unicode`);
  return bytes;
}
function encodedOutput(text) {
  if (typeof text !== "string" || text.length > MAX_RESULT * 2 || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(text)) throw new Error("compiler output has invalid base64");
  const bytes = Buffer.from(text, "base64");
  if (bytes.toString("base64") !== text) throw new Error("compiler output has noncanonical base64");
  return bytes;
}
function killGroup(child, signal) {
  if (child.pid) { try { process.kill(-child.pid, signal); } catch (error) { if (error.code !== "ESRCH") throw error; } }
}
async function invoke(binary, request, environment, signal, timeoutMs) {
  if (signal?.aborted) throw signal.reason ?? new Error("compiler cancelled");
  const child = spawn(binary, [], { detached: true, stdio: ["pipe", "pipe", "pipe"], env: environment });
  const stdout = []; const stderr = [];
  let outSize = 0; let errSize = 0; let exit = null; let closed = false; let failure = null;
  let cleanupTimer;
  let deadlineTimer;
  const stop = (reason, emergency = false) => {
    failure ??= reason;
    killGroup(child, emergency ? "SIGKILL" : "SIGTERM");
    child.stdin.destroy();
    if (!cleanupTimer) cleanupTimer = setTimeout(() => { killGroup(child, "SIGKILL"); child.stdout.destroy(); child.stderr.destroy(); }, CLEANUP_MS);
  };
  const aborted = () => stop(signal.reason ?? new Error("compiler cancelled"));
  signal?.addEventListener("abort", aborted, { once: true });
  deadlineTimer = setTimeout(() => stop(new Error("compiler timeout"), true), timeoutMs);
  const completion = new Promise((resolve, reject) => {
    child.on("error", (error) => stop(error, true));
    child.on("exit", (code, childSignal) => { exit = { code, signal: childSignal }; });
    child.on("close", () => { closed = true; resolve(); });
    child.stdout.on("data", (chunk) => {
      outSize += chunk.length;
      if (outSize > MAX_RESULT) stop(new Error("compiler response exceeds limit"), true);
      else stdout.push(chunk);
    });
    child.stderr.on("data", (chunk) => {
      errSize += chunk.length;
      if (errSize > MAX_STDERR) stop(new Error("compiler stderr exceeds limit"), true);
      else stderr.push(chunk);
    });
    child.stdin.on("error", (error) => { if (!failure) stop(error); });
  });
  try {
    child.stdin.end(request);
    await completion;
    if (!closed || !exit) throw new Error("compiler cleanup unconfirmed: direct exit not observed");
    if (failure) throw failure;
    return { ...exit, stdout: Buffer.concat(stdout), stderr: Buffer.concat(stderr) };
  } finally {
    clearTimeout(deadlineTimer); clearTimeout(cleanupTimer);
    signal?.removeEventListener("abort", aborted);
    if (!closed) { killGroup(child, "SIGKILL"); child.stdin.destroy(); child.stdout.destroy(); child.stderr.destroy(); }
  }
}

export async function compileRustSkill({ structureSource, interfaceSource, workspaceRoot, verifySources = true, signal, binaryPath = process.env.WORK_ENGINE_COMPILER_RUST_BINARY, expectedBinarySha256 = process.env.WORK_ENGINE_COMPILER_RUST_BINARY_SHA256, timeoutMs = DEADLINE_MS }) {
  if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > DEADLINE_MS) throw new TypeError("compiler timeout must be within the 60-second profile");
  if (!path.isAbsolute(binaryPath ?? "") || !DIGEST.test(expectedBinarySha256 ?? "")) throw new Error("Rust compiler selection requires an absolute binary and expected SHA-256");
  const binary = await readFile(binaryPath);
  if (hash(binary) !== expectedBinarySha256) throw new Error("Rust compiler binary digest mismatch");
  const temporary = await mkdtemp(path.join(os.tmpdir(), "work-engine-compiler-"));
  const copy = path.join(temporary, "compiler");
  try {
    await writeFile(copy, binary, { flag: "wx", mode: 0o700 });
    await chmod(copy, 0o700);
    if (hash(await readFile(copy)) !== expectedBinarySha256) throw new Error("Rust compiler captured copy digest mismatch");
    const root = path.resolve(workspaceRoot ?? process.cwd());
    const requestId = randomUUID();
    const structureBytes = sourceBytes(structureSource, "structureSource");
    const interfaceBytes = sourceBytes(interfaceSource, "interfaceSource");
    const request = JSON.stringify({
      schema_version: 2, request_id: requestId,
      operation: verifySources ? "compile_skill_verified" : "compile_skill_unverified",
      structure_source_base64: structureBytes.toString("base64"),
      interface_source_base64: interfaceBytes.toString("base64"),
    });
    if (Buffer.byteLength(request) > MAX_RESULT) throw new Error("compiler request exceeds limit");
    const script = path.resolve(root, process.env.WORK_ENGINE_COMPILER_AEG_SCRIPT ?? "skills/agent-environment-graph/scripts/agent_environment_graph.py");
    const environment = {
      ...process.env,
      WORK_ENGINE_COMPILER_WORKSPACE_ROOT: root,
      WORK_ENGINE_COMPILER_PYTHON: process.env.WORK_ENGINE_COMPILER_PYTHON ?? "/usr/bin/python3",
      WORK_ENGINE_COMPILER_AEG_SCRIPT: script,
      WORK_ENGINE_COMPILER_AEG_SCRIPT_SHA256: process.env.WORK_ENGINE_COMPILER_AEG_SCRIPT_SHA256 ?? DEFAULT_SCRIPT_DIGEST,
      WORK_ENGINE_COMPILER_AEG_INVARIANTS: path.resolve(root, process.env.WORK_ENGINE_COMPILER_AEG_INVARIANTS ?? "docs/workflow-invariants.md"),
      WORK_ENGINE_COMPILER_AEG_ENVIRONMENTS: path.resolve(root, process.env.WORK_ENGINE_COMPILER_AEG_ENVIRONMENTS ?? "docs/agent-environments.yaml"),
    };
    const result = await invoke(copy, request, environment, signal, timeoutMs);
    if (result.stdout.length > MAX_RESULT || result.stderr.length > MAX_STDERR) throw new Error("compiler response exceeds limit");
    let response;
    try {
      const responseText = new TextDecoder("utf-8", { fatal: true }).decode(result.stdout);
      const unique = parseDocument(responseText, { uniqueKeys: true, maxAliasCount: 0 });
      if (unique.errors.length) throw unique.errors[0];
      response = JSON.parse(responseText);
    }
    catch { throw new Error("compiler returned malformed JSON"); }
    if (response?.status === "error") {
      exactObject(response, ["schema_version", "request_id", "status", "error"], "compiler error response");
      if (response.schema_version !== 2 || response.request_id !== requestId || result.code === 0) throw new Error("compiler error response correlation or exit mismatch");
      throw new Error(`Rust compiler ${response.error?.code ?? "error"}: ${response.error?.message ?? "unknown"}`);
    }
    exactObject(response, ["schema_version", "request_id", "status", "ir", "output_base64", "verification"], "compiler response");
    if (result.code !== 0 || result.signal || result.stderr.length || response.schema_version !== 2 || response.request_id !== requestId || response.status !== "ok") throw new Error("compiler success response or exit mismatch");
    if (response.ir?.compiler !== PRODUCER || response.ir?.runtime_requirements?.verified_sources !== verifySources || response.verification?.mode !== (verifySources ? "verified" : "unverified")) throw new Error("compiler producer or operation mismatch");
    if (response.ir.input_sha256?.structure !== hash(structureBytes) || response.ir.input_sha256?.interface !== hash(interfaceBytes)) throw new Error("compiler input digest mismatch");
    const requirements = response.ir.runtime_requirements;
    const { sha256: requirementsSha256, ...unsignedRequirements } = requirements;
    if (requirementsSha256 !== hash(Buffer.from(canonicalJson(unsignedRequirements)))) throw new Error("compiler requirements digest mismatch");
    if (!Array.isArray(response.verification.sources) || response.verification.sources.some((item) => typeof item?.path !== "string" || !DIGEST.test(item.sha256))) throw new Error("compiler source evidence is malformed");
    if (verifySources && response.ir.role_profile) {
      const python = response.verification.python;
      if (python?.direct_child_reaped !== true || python.exit_code !== 0 || python.script_sha256 !== environment.WORK_ENGINE_COMPILER_AEG_SCRIPT_SHA256 || response.ir.role_projection?.backend_sha256 !== python.script_sha256) throw new Error("compiler Python observation is incomplete");
    } else if (Object.hasOwn(response.verification, "python")) throw new Error("compiler returned an unexpected Python observation");
    const output = encodedOutput(response.output_base64);
    if (hash(output) !== response.ir.output_sha256 || response.ir.runtime_requirements.compiled_skill_sha256 !== response.ir.output_sha256) throw new Error("compiler output digest mismatch");
    return { ir: response.ir, output, verification: { ...response.verification, rust_binary_sha256: expectedBinarySha256, rust_binary_source: binaryPath } };
  } finally { await rm(temporary, { recursive: true, force: true }); }
}

export const rustCompilerAdapterInternals = Object.freeze({ sourceBytes, encodedOutput });
