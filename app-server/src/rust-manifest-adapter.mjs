import { createHash, randomUUID } from "node:crypto";
import { spawnSync } from "node:child_process";
import { constants, closeSync, fstatSync, mkdtempSync, openSync, readSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { types as utilTypes } from "node:util";
import { parseDocument } from "yaml";

const MAX = 4 * 1024 * 1024;
const MAX_BINARY = 128 * 1024 * 1024;
const MAX_STDERR = 64 * 1024;
const DEADLINE_MS = 100;
const PRODUCER = "work-engine.manifest-compiler.rust-v1";
const PROFILE = "manifest-sync-v1";
const DIGEST = /^[0-9a-f]{64}$/;
const isDigest = (value) => typeof value === "string" && DIGEST.test(value);
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");

function issue(code, message, ErrorClass = Error, errorPath = null) {
  const error = new ErrorClass(message);
  error.code = code;
  error.path = errorPath;
  return error;
}
function quotedSize(value) {
  if (value.length > MAX) throw issue("resource_limit", "manifest JSON string exceeds limit", TypeError);
  let bytes = 2;
  for (let i = 0; i < value.length; i++) {
    const unit = value.charCodeAt(i);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++i);
      if (!(next >= 0xdc00 && next <= 0xdfff)) throw issue("invalid_request", "unsupported Unicode scalar", TypeError);
      bytes += 4;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw issue("invalid_request", "unsupported Unicode scalar", TypeError);
    } else if (unit === 34 || unit === 92 || unit === 8 || unit === 9 || unit === 10 || unit === 12 || unit === 13) {
      bytes += 2;
    } else if (unit < 32) {
      bytes += 6;
    } else {
      bytes += unit < 0x80 ? 1 : unit < 0x800 ? 2 : 3;
    }
    if (bytes > MAX) throw issue("resource_limit", "manifest JSON string exceeds limit", TypeError);
  }
  return bytes;
}
function shape(value, depth, visiting, memo) {
  if (depth > 64) throw issue("resource_limit", "manifest JSON nesting exceeds limit", TypeError);
  if (value === null) return { size: 4, height: 0 };
  if (typeof value === "boolean") return { size: value ? 4 : 5, height: 0 };
  if (typeof value === "string") return { size: quotedSize(value), height: 0 };
  if (typeof value === "number" && Number.isFinite(value)) return { size: Buffer.byteLength(JSON.stringify(value)), height: 0 };
  if (typeof value !== "object") throw issue("invalid_request", "manifest input must contain JSON data", TypeError);
  if (utilTypes.isProxy(value)) throw issue("invalid_request", "proxy input is unsupported", TypeError);
  if (visiting.has(value)) throw issue("invalid_request", "cyclic manifest input", TypeError);
  const prior = memo.get(value);
  if (prior) {
    if (depth + prior.height > 64) throw issue("resource_limit", "manifest JSON nesting exceeds limit", TypeError);
    return prior;
  }
  const array = Array.isArray(value);
  if (Object.getPrototypeOf(value) !== (array ? Array.prototype : Object.prototype) && (!array && Object.getPrototypeOf(value) !== null)) throw issue("invalid_request", "manifest input must use data records", TypeError);
  visiting.add(value);
  try {
    let size = 2;
    let height = 0;
    if (array) {
      if (value.length > Math.floor((MAX - 1) / 2)) throw issue("resource_limit", "manifest array expansion exceeds limit", TypeError);
      if (Reflect.ownKeys(value).some((k) => k !== "length" && (typeof k !== "string" || !/^(0|[1-9][0-9]*)$/.test(k) || Number(k) >= value.length))) throw issue("invalid_request", "array has unsupported fields", TypeError);
      for (let index = 0; index < value.length; index++) {
        const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
        if (!descriptor || !Object.hasOwn(descriptor, "value")) throw issue("invalid_request", "array must be dense data", TypeError);
        const child = shape(descriptor.value, depth + 1, visiting, memo);
        size += child.size + (index ? 1 : 0);
        height = Math.max(height, 1 + child.height);
        if (size > MAX) throw issue("resource_limit", "manifest request expansion exceeds limit", TypeError);
      }
    } else {
      const keys = Reflect.ownKeys(value);
      for (let index = 0; index < keys.length; index++) {
        const key = keys[index];
        if (typeof key !== "string") throw issue("invalid_request", "symbol key is unsupported", TypeError);
        const descriptor = Object.getOwnPropertyDescriptor(value, key);
        if (!descriptor || !Object.hasOwn(descriptor, "value")) throw issue("invalid_request", "accessor is unsupported", TypeError);
        size += quotedSize(key) + 1 + (index ? 1 : 0);
        if (size > MAX) throw issue("resource_limit", "manifest request expansion exceeds limit", TypeError);
        const child = shape(descriptor.value, depth + 1, visiting, memo);
        size += child.size;
        height = Math.max(height, 1 + child.height);
        if (size > MAX) throw issue("resource_limit", "manifest request expansion exceeds limit", TypeError);
      }
    }
    const measured = { size, height };
    memo.set(value, measured);
    return measured;
  } finally { visiting.delete(value); }
}
function copyData(value) {
  if (value === null || typeof value !== "object") return value;
  if (Array.isArray(value)) {
    const out = [];
    for (let index = 0; index < value.length; index++) out.push(copyData(Object.getOwnPropertyDescriptor(value, String(index)).value));
    return out;
  }
  const out = Object.create(null);
  for (const key of Reflect.ownKeys(value)) out[key] = copyData(Object.getOwnPropertyDescriptor(value, key).value);
  return out;
}
function ownData(value) {
  shape(value, 0, new Set(), new WeakMap());
  return copyData(value);
}

function exact(value, fields, label) {
  if (!value || typeof value !== "object" || Array.isArray(value) || Object.keys(value).length !== fields.length || fields.some((key) => !Object.hasOwn(value, key))) throw issue("protocol_error", `${label} has invalid fields`);
}
export function captureRustManifestSelection(selection) {
  const captured = ownData(selection);
  if (!captured || typeof captured !== "object" || Array.isArray(captured)) throw issue("binary_unavailable", "sealed compiler selection must be data", TypeError);
  return Object.freeze({ ...captured });
}
function selectedPath(selection = null) {
  const binaryPath = selection ? selection.binaryPath : process.env.WORK_ENGINE_COMPILER_RUST_BINARY;
  const expected = selection ? selection.expectedSha256 : process.env.WORK_ENGINE_COMPILER_RUST_BINARY_SHA256;
  if (selection && (selection.protocolVersion !== 3 || selection.launchProfileId !== PROFILE || Object.keys(selection).sort().join(",") !== "binaryPath,expectedSha256,launchProfileId,protocolVersion")) throw issue("binary_unavailable", "sealed Rust manifest selection has invalid profile");
  if (!path.isAbsolute(binaryPath ?? "") || !isDigest(expected)) throw issue("binary_unavailable", "Rust manifest compiler selection requires an absolute binary and expected SHA-256");
  return { binaryPath, expected };
}
function capturedBinary(selection = null) {
  const { binaryPath, expected } = selectedPath(selection);
  let fd;
  try { fd = openSync(binaryPath, constants.O_RDONLY | constants.O_NONBLOCK | constants.O_NOFOLLOW); }
  catch (error) { throw issue("binary_unavailable", `Rust manifest compiler cannot be opened: ${error.code ?? error.message}`); }
  try {
    const stat = fstatSync(fd);
    if (!stat.isFile() || !(stat.mode & 0o111) || stat.size < 1 || stat.size > MAX_BINARY) throw issue("binary_unavailable", "Rust manifest compiler must be a bounded executable regular file");
    const bytes = Buffer.allocUnsafe(stat.size);
    let offset = 0;
    while (offset < bytes.length) {
      const n = readSync(fd, bytes, offset, bytes.length - offset, null);
      if (n === 0) throw issue("binary_unavailable", "Rust manifest compiler ended during capture");
      offset += n;
    }
    const observedSha256 = hash(bytes);
    if (observedSha256 !== expected) throw issue("binary_mismatch", "Rust manifest compiler digest mismatch");
    return { bytes, observation: Object.freeze({ binaryPath, observedSha256, protocolVersion: 3, launchProfileId: PROFILE }) };
  } finally { closeSync(fd); }
}
export function observeRustManifestSelection(selection = null) { return capturedBinary(selection).observation; }
function strictResponse(bytes) {
  let source;
  try { source = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes); }
  catch { throw issue("protocol_error", "Rust manifest compiler response is not UTF-8"); }
  if (Buffer.from(source).length !== bytes.length) throw issue("protocol_error", "Rust manifest compiler response has BOM");
  let inString = false;
  let escaped = false;
  let depth = 0;
  for (const ch of source) {
    if (inString) {
      if (escaped) escaped = false;
      else if (ch === "\\") escaped = true;
      else if (ch === '"') inString = false;
    } else if (ch === '"') inString = true;
    else if (ch === "{" || ch === "[") {
      if (++depth > 64) throw issue("protocol_error", "Rust manifest response nesting exceeds limit");
    } else if (ch === "}" || ch === "]") depth--;
  }
  let value;
  try {
    const parsed = parseDocument(source, { uniqueKeys: true, maxAliasCount: 0 });
    if (parsed.errors.length) throw parsed.errors[0];
    value = JSON.parse(source);
  } catch { throw issue("protocol_error", "Rust manifest compiler returned malformed JSON"); }
  validateParsedScalars(value);
  return value;
}
function validateParsedScalars(value) {
  const pending = [value];
  while (pending.length) {
    const item = pending.pop();
    if (item === null || typeof item === "boolean") continue;
    if (typeof item === "number") {
      if (!Number.isFinite(item)) throw issue("protocol_error", "Rust manifest response contains nonfinite number");
      continue;
    }
    if (typeof item === "string") {
      for (let i = 0; i < item.length; i++) {
        const unit = item.charCodeAt(i);
        if (unit >= 0xd800 && unit <= 0xdbff) {
          const next = item.charCodeAt(++i);
          if (!(next >= 0xdc00 && next <= 0xdfff)) throw issue("protocol_error", "Rust manifest response contains unsupported Unicode scalar");
        } else if (unit >= 0xdc00 && unit <= 0xdfff) throw issue("protocol_error", "Rust manifest response contains unsupported Unicode scalar");
      }
      continue;
    }
    if (Array.isArray(item)) { for (const child of item) pending.push(child); continue; }
    if (item && typeof item === "object") {
      for (const [key, child] of Object.entries(item)) { pending.push(key, child); }
      continue;
    }
    throw issue("protocol_error", "Rust manifest response contains unsupported JSON value");
  }
}
function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}
const IDENTIFIER = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;
function isRecord(value) { return value !== null && typeof value === "object" && !Array.isArray(value); }
function absolute(value) { return typeof value === "string" && value.startsWith("/"); }
function grantList(value) {
  return Array.isArray(value) && value.every((entry) => typeof entry === "string" && IDENTIFIER.test(entry)) &&
    value.every((entry, index) => index === 0 || value[index - 1] < entry);
}
function validateResult(operation, value, payload) {
  if (operation === "project_runtime_manifest") {
    exact(value, ["manifestId", "source", "roles", "requirementsBaseDirectory"], "projected manifest");
    exact(value.source, ["manifestId", "schemaVersion", "path", "sha256"], "manifest source");
    if (typeof value.manifestId !== "string" || value.manifestId !== payload.document?.manifest_id || !IDENTIFIER.test(value.manifestId) || value.source.manifestId !== value.manifestId || value.source.schemaVersion !== 1 || !isDigest(value.source.sha256) || !(value.source.path === null || absolute(value.source.path)) || !absolute(value.requirementsBaseDirectory) || !isRecord(value.roles)) throw issue("protocol_error", "projected manifest identity is invalid");
    if (Object.keys(value.roles).sort().join("\0") !== Object.keys(payload.document.roles ?? {}).sort().join("\0")) throw issue("protocol_error", "projected manifest role set differs");
    for (const [id, role] of Object.entries(value.roles)) {
      if (!IDENTIFIER.test(id)) throw issue("protocol_error", "projected role ID is invalid");
      exact(role, ["roleContract", "developerInstructions", "threadOptions", "skills", "capabilities", "effects", "continuity", "compiledSkillSha256", "runtimeRequirements", "runtimeEnvironmentRevision"], `role ${id}`);
      exact(role.roleContract, ["path", "activatedPath"], `role ${id} contract`);
      if (!absolute(role.roleContract.path) || !absolute(role.roleContract.activatedPath) || !Array.isArray(role.skills) || !grantList(role.capabilities) || !grantList(role.effects) || !isDigest(role.runtimeEnvironmentRevision) || !["retained", "ephemeral"].includes(role.continuity) || typeof role.developerInstructions !== "string" || !(role.compiledSkillSha256 === null || isDigest(role.compiledSkillSha256))) throw issue("protocol_error", `role ${id} projection is invalid`);
      const options = role.threadOptions;
      if (!isRecord(options) || Object.keys(options).some((key) => !["cwd", "approvalPolicy", "sandbox", "model", "effort", "serviceTier", "personality"].includes(key)) || Object.entries(options).some(([key, field]) => typeof field !== "string" || field.trim() === "" || (key === "cwd" && !absolute(field))) || (options.approvalPolicy && !["untrusted", "on-request", "never"].includes(options.approvalPolicy)) || (options.sandbox && !["read-only", "workspace-write", "danger-full-access"].includes(options.sandbox)) || (options.personality && !["none", "friendly", "pragmatic"].includes(options.personality))) throw issue("protocol_error", `role ${id} thread options are invalid`);
      const skillNames = new Set();
      for (const skill of role.skills) {
        if (!isRecord(skill) || typeof skill.name !== "string" || !skill.name.trim() || skillNames.has(skill.name) || !absolute(skill.path) || !absolute(skill.identityPath)) throw issue("protocol_error", `role ${id} skill projection is invalid`);
        skillNames.add(skill.name);
        const compiled = Object.hasOwn(skill, "compiledEnvironment");
        const fields = compiled ? ["name", "path", "identityPath", "compiledEnvironment", "compiledSkillSha256", "capabilities", "effects", ...(Object.hasOwn(skill, "runtimeRequirements") ? ["runtimeRequirements"] : [])] : ["name", "path", "identityPath", ...(Object.hasOwn(skill, "runtimeRequirements") ? ["runtimeRequirements"] : [])];
        exact(skill, fields, `role ${id} skill ${skill.name}`);
        if (compiled) {
          exact(skill.compiledEnvironment, ["structure", "interface"], `role ${id} skill ${skill.name} compiled environment`);
          if (!absolute(skill.compiledEnvironment.structure) || !absolute(skill.compiledEnvironment.interface) || !isDigest(skill.compiledSkillSha256) || !grantList(skill.capabilities) || !grantList(skill.effects)) throw issue("protocol_error", `role ${id} skill ${skill.name} compiled metadata is invalid`);
        }
      }
      if (!role.skills.some((skill) => skill.path === role.roleContract.activatedPath)) throw issue("protocol_error", `role ${id} contract input is absent`);
    }
  } else if (operation === "satisfy_runtime_requirements") {
    const expected = ["schema_version", "role_id", "requirements_sha256", "compiled_skill_sha256", "manifest_sha256", "runtime_environment_revision", "sha256"];
    if (payload.skill_name !== null) expected.push("skill_id");
    exact(value, expected, "satisfaction receipt");
    if (value.schema_version !== 1 || value.role_id !== payload.role_id || value.requirements_sha256 !== payload.requirements?.sha256 || value.compiled_skill_sha256 !== payload.requirements?.compiled_skill_sha256 || value.manifest_sha256 !== payload.manifest?.source?.sha256 || value.runtime_environment_revision !== payload.manifest?.roles?.[payload.role_id]?.runtimeEnvironmentRevision || (payload.skill_name !== null && value.skill_id !== payload.skill_name) || !isDigest(value.sha256)) throw issue("protocol_error", "satisfaction receipt identity differs");
    const { sha256, ...unsigned } = value;
    if (hash(Buffer.from(canonicalJson(unsigned))) !== sha256) throw issue("protocol_error", "satisfaction receipt digest differs");
  } else throw issue("invalid_request", "unsupported manifest operation", TypeError);
}

export function invokeRustManifest(operation, payload, selection = null) {
  const requestId = randomUUID();
  const body = ownData({ schema_version: 3, request_id: requestId, operation, ...payload });
  const request = Buffer.from(JSON.stringify(body));
  if (request.length > MAX) throw issue("resource_limit", "Rust manifest request exceeds limit", TypeError);
  const { bytes, observation } = capturedBinary(selection);
  const directory = mkdtempSync(path.join(os.tmpdir(), "work-engine-manifest-"));
  const executable = path.join(directory, "compiler");
  try {
    writeFileSync(executable, bytes, { flag: "wx", mode: 0o700 });
    if (hash(readFileSync(executable)) !== observation.observedSha256) throw issue("binary_mismatch", "captured manifest compiler digest mismatch");
    const result = spawnSync(executable, [], { input: request, encoding: "buffer", timeout: DEADLINE_MS, killSignal: "SIGKILL", maxBuffer: MAX + 1, env: { PATH: "/usr/bin:/bin" } });
    if (result.error) {
      if (result.error.code === "ETIMEDOUT") throw issue("timeout", "Rust manifest compiler timed out");
      if (result.error.code === "ENOBUFS") throw issue("resource_limit", "Rust manifest compiler response exceeds limit");
      throw issue("process_failed", `Rust manifest compiler failed: ${result.error.code ?? result.error.message}`);
    }
    if (result.stdout?.length > MAX || result.stderr?.length > MAX_STDERR) throw issue("resource_limit", "Rust manifest compiler output exceeds limit");
    if (result.signal) throw issue("process_failed", "Rust manifest compiler exited by signal");
    const response = strictResponse(result.stdout ?? Buffer.alloc(0));
    if (response?.status === "error") {
      exact(response, ["schema_version", "request_id", "operation", "status", "producer", "error"], "manifest error envelope");
      exact(response.error, ["code", "path", "message"], "manifest error");
      const domainCodes = new Set(["invalid_manifest", "invalid_requirements", "requirements_unsatisfied", "invalid_request", "unsupported_version", "resource_limit"]);
      const hostCodes = new Set(["binary_unavailable", "binary_mismatch", "protocol_error", "timeout", "process_failed", "cleanup_unconfirmed"]);
      const expectedExit = domainCodes.has(response.error.code) ? 2 : hostCodes.has(response.error.code) ? 1 : null;
      if (response.schema_version !== 3 || response.request_id !== requestId || response.operation !== operation || response.producer !== PRODUCER || expectedExit === null || result.status !== expectedExit || typeof response.error.message !== "string" || !response.error.message.trim() || Buffer.byteLength(response.error.message) > 1024 || !(response.error.path === null || typeof response.error.path === "string" && response.error.path.startsWith("/") && Buffer.byteLength(response.error.path) <= 1024)) throw issue("protocol_error", "Rust manifest error correlation or exit mismatch");
      const ErrorClass = ["invalid_manifest", "invalid_requirements", "invalid_request", "resource_limit"].includes(response.error.code) ? TypeError : Error;
      throw issue(response.error.code, response.error.message, ErrorClass, response.error.path);
    }
    exact(response, ["schema_version", "request_id", "operation", "status", "producer", "result"], "manifest success envelope");
    if (response.schema_version !== 3 || response.request_id !== requestId || response.operation !== operation || response.producer !== PRODUCER || response.status !== "ok" || result.status !== 0 || result.stderr?.length) throw issue("protocol_error", "Rust manifest response correlation or exit mismatch");
    if (!response.result || typeof response.result !== "object" || Array.isArray(response.result)) throw issue("protocol_error", "Rust manifest result has invalid shape");
    validateResult(operation, response.result, payload);
    return Object.freeze({ result: response.result, observation });
  } finally { rmSync(directory, { recursive: true, force: true }); }
}
