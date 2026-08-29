import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";

import {
  REVIEW_SUBJECT_BACKEND,
  REVIEW_SUBJECT_SCHEMA_VERSION,
  ReviewSubjectError,
  requireOperation,
  requireRecord,
  validateBackendEnvelope,
} from "./contract.mjs";

const DEFAULT_TIMEOUT_MS = 30_000;
const DEFAULT_MAX_BUFFER = 8 * 1024 * 1024;

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function invoke(command, args, { input, timeout, maxBuffer }) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
    let stdout = Buffer.alloc(0);
    let stderr = Buffer.alloc(0);
    let settled = false;
    let timer;
    const finish = (error, result) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      if (error) reject(error);
      else resolve(result);
    };
    const append = (current, chunk) => {
      const next = Buffer.concat([current, chunk]);
      if (next.length > maxBuffer) {
        child.kill("SIGKILL");
        finish(new ReviewSubjectError(`review-subject backend exceeded ${maxBuffer} bytes`));
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
      finish(new ReviewSubjectError(`review-subject backend timed out after ${timeout}ms`));
    }, timeout);
    child.stdin.end(input);
  });
}

export function createLegacyReviewSubjectBackend({
  workspaceRoot,
  python = "python3",
  bridgePath = "app-server/scripts/review-subject-backend.py",
  checkpointPath = "skills/slice-checkpoint/scripts/checkpoint.py",
  physicalProfilePath = "skills/code-change-profile/scripts/code_change_profile.py",
  invokeProcess = invoke,
  timeout = DEFAULT_TIMEOUT_MS,
  maxBuffer = DEFAULT_MAX_BUFFER,
} = {}) {
  const root = path.resolve(workspaceRoot ?? process.cwd());
  const bridge = path.resolve(root, bridgePath);
  const checkpoint = path.resolve(root, checkpointPath);
  const physicalProfile = path.resolve(root, physicalProfilePath);

  return Object.freeze({
    async invoke({ operation, input }) {
      requireOperation(operation);
      requireRecord(input, "review-subject operation input");
      const backendDigests = {
        checkpoint: sha256(await readFile(checkpoint)),
        physical_profile: sha256(await readFile(physicalProfile)),
      };
      const request = {
        schema_version: REVIEW_SUBJECT_SCHEMA_VERSION,
        backend: REVIEW_SUBJECT_BACKEND,
        operation,
        expected_backend_sha256: backendDigests,
        input,
      };
      const execution = await invokeProcess(python, [bridge, "--workspace-root", root], {
        input: Buffer.from(`${JSON.stringify(request)}\n`), timeout, maxBuffer,
      });
      if (!execution || execution.code !== 0 || execution.signal) {
        const detail = execution?.stderr?.toString("utf8").trim() || `exit ${execution?.code ?? "unknown"}`;
        throw new ReviewSubjectError(`review-subject backend failed: ${detail}`);
      }
      if (execution.stderr.length !== 0) {
        throw new ReviewSubjectError("review-subject backend wrote stderr on success");
      }
      let envelope;
      try {
        envelope = JSON.parse(execution.stdout.toString("utf8"));
      } catch (error) {
        throw new ReviewSubjectError(`review-subject backend returned invalid JSON: ${error.message}`);
      }
      return validateBackendEnvelope(envelope, { operation, backendDigests });
    },
  });
}

export const legacyReviewSubjectBackendInternals = Object.freeze({ invoke, sha256 });
