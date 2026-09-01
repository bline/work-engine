import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { readFile, realpath, stat } from "node:fs/promises";
import path from "node:path";

import { preflightCampaign } from "../../../../skills/slice-supervisor/scripts/campaign-preflight.mjs";

const TIMEOUT_MS = 60_000;
const MAX_BUFFER = 16 * 1024 * 1024;
const BACKEND_PATHS = Object.freeze({
  checkpoint_lifecycle: "skills/slice-supervisor/scripts/checkpoint_lifecycle.py",
  completion_offer_lifecycle: "skills/slice-supervisor/scripts/completion_offer_lifecycle.py",
  finalize_receipt: "skills/slice-supervisor/scripts/finalize_receipt.py",
  resume_campaign: "skills/slice-supervisor/scripts/resume_campaign.py",
  append_metrics: "skills/slice-supervisor/scripts/append_metrics.py",
  assemble_receipt: "skills/slice-supervisor/scripts/assemble_receipt.py",
  slice_checkpoint: "skills/slice-checkpoint/scripts/checkpoint.py",
  completion_commit: "skills/slice-completion-commit/scripts/completion_commit.py",
});
const PREFLIGHT_PATH = "skills/slice-supervisor/scripts/campaign-preflight.mjs";
const BRIDGE_PATH = "app-server/scripts/supervisor-campaign-backend.py";

function sha256(bytes) { return createHash("sha256").update(bytes).digest("hex"); }

function requireRecord(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function invoke(command, args, { cwd, input, timeout = TIMEOUT_MS, maxBuffer = MAX_BUFFER }) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
    let stdout = Buffer.alloc(0);
    let stderr = Buffer.alloc(0);
    let settled = false;
    let timer;
    const finish = (error, value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      if (error) reject(error); else resolve(value);
    };
    const append = (current, chunk) => {
      const next = Buffer.concat([current, chunk]);
      if (next.length > maxBuffer) {
        child.kill("SIGKILL");
        finish(new Error("fixed supervisor backend exceeded its output limit"));
      }
      return next;
    };
    child.stdout.on("data", (chunk) => { stdout = append(stdout, chunk); });
    child.stderr.on("data", (chunk) => { stderr = append(stderr, chunk); });
    child.on("error", (error) => finish(error));
    child.on("close", (code, signal) => finish(null, { code, signal, stdout, stderr }));
    child.stdin.on("error", (error) => finish(error));
    timer = setTimeout(() => {
      child.kill("SIGKILL");
      finish(new Error("fixed supervisor backend timed out"));
    }, timeout);
    child.stdin.end(input);
  });
}

function inside(root, candidate) {
  const relative = path.relative(root, candidate);
  return relative === "" || (relative !== ".." && !relative.startsWith(`..${path.sep}`)
    && !path.isAbsolute(relative));
}

async function confinedExisting(root, value, label, expectedKind) {
  requireText(value, label);
  if (path.isAbsolute(value) || value.split(/[\\/]/).some((part) => part === ".." || part === "")) {
    throw new TypeError(`${label} must be a safe repository-relative path`);
  }
  const resolved = await realpath(path.resolve(root, value));
  if (!inside(root, resolved)) throw new TypeError(`${label} escapes the repository`);
  const observed = await stat(resolved);
  if (expectedKind === "file" ? !observed.isFile() : !observed.isDirectory()) {
    throw new TypeError(`${label} has the wrong filesystem kind`);
  }
  return resolved;
}

async function confinedRepository(root, value) {
  requireText(value, "completion repository");
  const resolved = await realpath(path.resolve(value));
  if (!inside(root, resolved)) throw new TypeError("completion repository escapes the live repository");
  if (!(await stat(resolved)).isDirectory()) throw new TypeError("completion repository must be a directory");
  return resolved;
}

async function confinedOutput(root, value, label) {
  requireText(value, label);
  if (path.isAbsolute(value) || value.split(/[\\/]/).some((part) => part === ".." || part === "")) {
    throw new TypeError(`${label} must be repository-relative`);
  }
  const destination = path.resolve(root, value);
  if (!inside(root, destination)) throw new TypeError(`${label} escapes the repository`);
  let existing = destination;
  while (true) {
    try {
      const resolved = await realpath(existing);
      if (!inside(root, resolved)) throw new TypeError(`${label} escapes the repository`);
      return destination;
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
      const parent = path.dirname(existing);
      if (parent === existing) throw error;
      existing = parent;
    }
  }
}

function metricsRelative(preflight) {
  requireRecord(preflight, "campaign preflight");
  const value = preflight.engineConfig?.metrics?.path;
  requireText(value, "campaign metrics path");
  if (path.isAbsolute(value) || value.split(/[\\/]/).some((part) => part === ".." || part === "")) {
    throw new TypeError("campaign metrics path must be repository-relative");
  }
  return value;
}

export async function createLegacySupervisorControlAdapter({
  workspaceRoot, python = "python3", invokeProcess = invoke,
} = {}) {
  const root = await realpath(path.resolve(requireText(workspaceRoot, "workspace root")));
  const bridge = path.join(root, BRIDGE_PATH);
  const files = Object.fromEntries(Object.entries(BACKEND_PATHS).map(([name, relative]) => [
    name, path.join(root, relative),
  ]));
  const captured = Object.fromEntries(await Promise.all(Object.entries(files).map(
    async ([name, file]) => [name, sha256(await readFile(file))],
  )));
  const bridgeDigest = sha256(await readFile(bridge));
  const preflightFile = path.join(root, PREFLIGHT_PATH);
  const preflightDigest = sha256(await readFile(preflightFile));

  const verifyFixedBytes = async () => {
    if (sha256(await readFile(bridge)) !== bridgeDigest
        || sha256(await readFile(preflightFile)) !== preflightDigest) {
      throw new Error("fixed supervisor backend changed after host startup");
    }
    for (const [name, file] of Object.entries(files)) {
      if (sha256(await readFile(file)) !== captured[name]) {
        throw new Error("fixed supervisor backend changed after host startup");
      }
    }
  };

  const call = async (operation, input) => {
    await verifyFixedBytes();
    const execution = await invokeProcess(python, [bridge], {
      cwd: root,
      input: Buffer.from(`${JSON.stringify({
        schema_version: 1, operation, expected_backend_sha256: captured, input,
      })}\n`),
    });
    if (!execution || execution.code !== 0 || execution.signal) {
      const detail = execution?.stderr?.toString("utf8").trim() || "fixed backend failed";
      throw new Error(`fixed supervisor backend failed: ${detail}`);
    }
    if (execution.stderr.length !== 0) throw new Error("fixed supervisor backend wrote stderr on success");
    let envelope;
    try { envelope = JSON.parse(execution.stdout.toString("utf8")); }
    catch { throw new Error("fixed supervisor backend returned invalid JSON"); }
    requireRecord(envelope, "fixed supervisor backend result");
    if (envelope.schema_version !== 1 || envelope.operation !== operation
        || JSON.stringify(envelope.backend_sha256) !== JSON.stringify(captured)) {
      throw new Error("fixed supervisor backend result binding is invalid");
    }
    return envelope.result;
  };

  const deriveMetricsPath = async (preflight) => {
    const campaignPath = await realpath(requireText(
      preflight?.campaignSource?.resolvedPath,
      "campaign source resolved path",
    ));
    if (!inside(root, campaignPath)) throw new TypeError("campaign source escapes the repository");
    return confinedOutput(root, metricsRelative(preflight), "campaign metrics path");
  };

  return Object.freeze({
    async preflight({ campaign_path: campaignPath }) {
      const resolved = await confinedExisting(root, campaignPath, "campaign path", "file");
      await verifyFixedBytes();
      return preflightCampaign(path.relative(root, resolved), { cwd: root });
    },
    checkpoint(operation, input) { return call(`checkpoint.${operation}`, input); },
    async finalize(input) {
      return call("receipt.finalize", {
        path: await deriveMetricsPath(input.campaign_preflight),
        semantic_receipt: input.semantic_receipt,
        telemetry_ingress: input.telemetry_ingress,
        campaign_preflight: input.campaign_preflight,
        handoff_receipt: input.handoff_receipt,
        checkpoint_receipt: input.checkpoint_receipt ?? null,
        completion_commit_receipt: input.completion_commit_receipt ?? null,
      });
    },
    validateReceipt(receipt) { return call("receipt.validate", { receipt }); },
    async offer(operation, input) {
      const projected = structuredClone(input);
      if (operation === "open") {
        projected.request.repository = await confinedRepository(root, projected.request.repository);
      } else if (operation === "load") {
        projected.repository = await confinedRepository(root, projected.repository);
      } else {
        projected.offer.request.repository = await confinedRepository(
          root, projected.offer.request.repository,
        );
      }
      return call(`offer.${operation}`, projected);
    },
    async resumeTerminal(input) {
      return call("resume.terminal", {
        path: await deriveMetricsPath(input.campaign_preflight),
        campaign_preflight: input.campaign_preflight,
        run_id: input.run_id,
      });
    },
    identity: Object.freeze({
      bridge_sha256: bridgeDigest,
      preflight_sha256: preflightDigest,
      backend_sha256: Object.freeze({ ...captured }),
    }),
  });
}

export const legacySupervisorControlAdapterInternals = Object.freeze({ invoke, inside });
