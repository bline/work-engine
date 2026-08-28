import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";

import { buildGrokReviewPrompt, grokReviewerSystemPrompt } from "./prompt.mjs";
import { admitGrokReview, parseGrokCliEnvelope } from "./receipt.mjs";
import { inspectGrokSessionTrace } from "./trace.mjs";

const MAX_OUTPUT = 8 * 1024 * 1024;

function classifyFailure(text) {
  if (/usage limit|rate.?limit|quota/i.test(text)) return "provider_quota";
  if (/permission/i.test(text)) return "provider_permission";
  return "provider_error";
}

function runProcess(command, args, { cwd, env, timeoutMs, onProgress }) {
  return new Promise((resolve) => {
    const child = spawn(command, args, {
      cwd,
      env,
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
    const stdout = [];
    const stderr = [];
    let stdoutBytes = 0;
    let stderrBytes = 0;
    let timedOut = false;
    let overflow = false;

    const retain = (chunks, chunk, counter) => {
      const next = counter + chunk.length;
      if (next > MAX_OUTPUT) {
        overflow = true;
        child.kill("SIGTERM");
        return counter;
      }
      chunks.push(chunk);
      return next;
    };
    child.stdout.on("data", (chunk) => { stdoutBytes = retain(stdout, chunk, stdoutBytes); });
    child.stderr.on("data", (chunk) => { stderrBytes = retain(stderr, chunk, stderrBytes); });

    const timer = setTimeout(() => {
      timedOut = true;
      child.kill("SIGTERM");
      setTimeout(() => child.kill("SIGKILL"), 1_000).unref();
    }, timeoutMs);
    timer.unref();
    const startedAt = Date.now();
    const progressTimer = onProgress ? setInterval(() => {
      onProgress({ phase: "provider_running", elapsed_ms: Date.now() - startedAt });
    }, 10_000) : null;
    progressTimer?.unref();

    child.on("error", (error) => {
      clearTimeout(timer);
      if (progressTimer) clearInterval(progressTimer);
      resolve({ code: null, signal: null, stdout: "", stderr: error.message, timedOut, overflow });
    });
    child.on("close", (code, signal) => {
      clearTimeout(timer);
      if (progressTimer) clearInterval(progressTimer);
      resolve({
        code,
        signal,
        stdout: Buffer.concat(stdout).toString("utf8"),
        stderr: Buffer.concat(stderr).toString("utf8"),
        timedOut,
        overflow,
      });
    });
  });
}

export async function runGrokReview({
  packet,
  command = "grok",
  commandArgsPrefix = [],
  env = process.env,
  model = "grok-4.6",
  reasoningEffort = "medium",
  maxTurns = 10,
  timeoutMs = 180_000,
  sessionId = randomUUID(),
  traceInspector = inspectGrokSessionTrace,
  onProgress,
} = {}) {
  const prompt = buildGrokReviewPrompt(packet.manifest);
  const args = [
    ...commandArgsPrefix,
    "-p", prompt,
    "--cwd", packet.root,
    "--model", model,
    "--reasoning-effort", reasoningEffort,
    "--permission-mode", "plan",
    "--disable-web-search",
    "--no-subagents",
    "--tools", "Read,Grep,Glob",
    "--disallowed-tools", "search_tool,use_tool",
    "--max-turns", String(maxTurns),
    "--session-id", sessionId,
    "--system-prompt-override", grokReviewerSystemPrompt(),
    "--output-format", "json",
  ];
  const processResult = await runProcess(command, args, {
    cwd: packet.root,
    env,
    timeoutMs,
    onProgress,
  });

  if (processResult.timedOut) {
    return { status: "timeout", session_id: sessionId, process: processResult };
  }
  if (processResult.overflow) {
    return { status: "output_limit", session_id: sessionId, process: processResult };
  }
  if (processResult.code !== 0) {
    const combined = `${processResult.stdout}\n${processResult.stderr}`;
    return {
      status: classifyFailure(combined),
      session_id: sessionId,
      process: processResult,
    };
  }

  try {
    const envelope = parseGrokCliEnvelope(processResult.stdout);
    if (envelope.sessionId !== sessionId) {
      throw new Error(`Grok returned session ${envelope.sessionId}, expected ${sessionId}`);
    }
    const receipt = await admitGrokReview({ packet, envelope });
    const trace = await traceInspector({
      command,
      commandArgsPrefix,
      packet,
      receipt,
      sessionId,
    });
    return { status: "completed", receipt: { ...receipt, trace }, process: processResult };
  } catch (error) {
    return {
      status: "rejected_output",
      session_id: sessionId,
      reason: error.message,
      process: processResult,
    };
  }
}
