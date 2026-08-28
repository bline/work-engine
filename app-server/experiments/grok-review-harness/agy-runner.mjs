import { spawn } from "node:child_process";
import { chmod, copyFile, mkdir } from "node:fs/promises";
import path from "node:path";

import { buildReviewPrompt } from "./prompt.mjs";
import { admitReview } from "./receipt.mjs";

const MAX_OUTPUT = 8 * 1024 * 1024;
const ALLOWED_TOOLS = new Set([
  "finish",
  "find_by_name",
  "grep_search",
  "list_dir",
  "read_file",
  "sed_file",
  "view_file",
]);
const DIRECT_READ_TOOLS = new Set(["read_file", "sed_file", "view_file"]);
const AGY_REVIEWER_AGENT = "work-engine-evidence-reviewer";
const AGY_REVIEWER_TOOLS = new Set(["finish", "grep_search", "view_file"]);

export const AGY_REVIEW_SCHEMA = {
  type: "object",
  additionalProperties: false,
  required: ["subject", "verdict", "findings", "decisive_evidence", "limitations", "metrics"],
  properties: {
    subject: {
      type: "object",
      additionalProperties: false,
      required: ["commit", "tree", "patch_identity"],
      properties: {
        commit: { type: "string" },
        tree: { type: "string" },
        patch_identity: { type: "string" },
      },
    },
    verdict: { enum: ["acceptable_as_is", "remediation_required", "incomplete"] },
    findings: {
      type: "array",
      items: {
        type: "object",
        additionalProperties: false,
        required: [
          "id", "severity", "title", "evidence", "observed", "violated_expectation",
          "consequence", "reproduced_or_inferred", "confidence", "recommended_remediation",
        ],
        properties: {
          id: { type: "string" },
          severity: { enum: ["blocker", "high", "medium", "low", "info"] },
          title: { type: "string" },
          evidence: { type: "array", minItems: 1, items: { $ref: "#/$defs/citation" } },
          observed: { type: "string" },
          violated_expectation: { type: "string" },
          consequence: { type: "string" },
          reproduced_or_inferred: { enum: ["reproduced", "inferred"] },
          confidence: { enum: ["high", "medium", "low"] },
          recommended_remediation: { type: "string" },
        },
      },
    },
    decisive_evidence: {
      type: "array",
      items: {
        type: "object",
        additionalProperties: false,
        required: ["path", "start_line", "end_line", "reason"],
        properties: {
          path: { type: "string" },
          start_line: { type: "integer", minimum: 1 },
          end_line: { type: "integer", minimum: 1 },
          reason: { type: "string" },
        },
      },
    },
    limitations: { type: "array", items: { type: "string" } },
    metrics: {
      type: "object",
      additionalProperties: false,
      required: ["files_considered", "findings_by_severity"],
      properties: {
        files_considered: { type: "integer", minimum: 0 },
        findings_by_severity: {
          type: "object",
          additionalProperties: false,
          required: ["blocker", "high", "medium", "low", "info"],
          properties: Object.fromEntries(
            ["blocker", "high", "medium", "low", "info"].map((key) => [key, { type: "integer", minimum: 0 }]),
          ),
        },
      },
    },
  },
  $defs: {
    citation: {
      type: "object",
      additionalProperties: false,
      required: ["path", "start_line", "end_line"],
      properties: {
        path: { type: "string" },
        start_line: { type: "integer", minimum: 1 },
        end_line: { type: "integer", minimum: 1 },
      },
    },
  },
};

function classifyFailure(text) {
  if (/usage limit|rate.?limit|quota|credits? exhausted/i.test(text)) return "provider_quota";
  if (/permission/i.test(text)) return "provider_permission";
  return "provider_error";
}

function runProcess(command, args, { cwd, env, timeoutMs, onProgress }) {
  return new Promise((resolve) => {
    const child = spawn(command, args, { cwd, env, stdio: ["ignore", "pipe", "pipe"], windowsHide: true });
    const stdout = [];
    const stderr = [];
    let stdoutBytes = 0;
    let stderrBytes = 0;
    let timedOut = false;
    let overflow = false;
    const retain = (chunks, chunk, count) => {
      if (count + chunk.length > MAX_OUTPUT) {
        overflow = true;
        child.kill("SIGTERM");
        return count;
      }
      chunks.push(chunk);
      return count + chunk.length;
    };
    child.stdout.on("data", (chunk) => { stdoutBytes = retain(stdout, chunk, stdoutBytes); });
    child.stderr.on("data", (chunk) => { stderrBytes = retain(stderr, chunk, stderrBytes); });
    const startedAt = Date.now();
    const timer = setTimeout(() => {
      timedOut = true;
      child.kill("SIGTERM");
      setTimeout(() => child.kill("SIGKILL"), 1_000).unref();
    }, timeoutMs);
    timer.unref();
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

function parseEvents(stdout) {
  return stdout.split(/\r?\n/).filter((line) => line.trim() !== "").map((line, index) => {
    try {
      const raw = JSON.parse(line);
      if (raw.event === "init") return { type: "init", ...(raw.init ?? raw) };
      if (raw.event === "step_update") return { type: "step_update", ...(raw.step_update ?? raw) };
      if (raw.event === "result") return { type: "result", ...(raw.result ?? raw) };
      return raw;
    } catch (error) {
      throw new Error(`Agy stream line ${index + 1} is not JSON: ${error.message}`);
    }
  });
}

function toolName(event) {
  if (event.type !== "step_update") return null;
  if (event.step_type !== "tool" && event.step_type !== "finish" && !event.tool_name && !event.tool_info) return null;
  return event.tool_name ?? event.tool_info?.name ?? event.name ?? (event.step_type === "finish" ? "finish" : null);
}

function toolParameters(event) {
  return event.tool_info?.parameters ?? event.parameters ?? event.input ?? {};
}

function toolCompleted(event) {
  const status = event.tool_info?.status ?? event.status;
  if (status && ["completed", "success", "succeeded"].includes(String(status).toLowerCase())) return true;
  return Object.hasOwn(event.tool_info ?? {}, "output") || Object.hasOwn(event, "output");
}

function candidatePaths(parameters) {
  const values = [];
  for (const key of [
    "path", "file_path", "target_file", "AbsolutePath", "directory", "DirectoryPath",
    "root", "search_path", "SearchDirectory",
  ]) {
    const value = parameters?.[key];
    if (typeof value === "string" && value.trim()) values.push(value);
  }
  if (Array.isArray(parameters?.paths)) values.push(...parameters.paths.filter((value) => typeof value === "string"));
  return values;
}

function unquoteProviderString(value) {
  const trimmed = value.trim();
  if (trimmed.startsWith('"') && trimmed.endsWith('"')) {
    try {
      const parsed = JSON.parse(trimmed);
      if (typeof parsed === "string") return parsed;
    } catch {
      // Preserve the original so confinement still fails closed.
    }
  }
  return value;
}

function confinedPath(packetRoot, candidate) {
  const absolute = path.resolve(packetRoot, unquoteProviderString(candidate));
  const relative = path.relative(packetRoot, absolute);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new Error(`Agy tool path escapes the immutable packet: ${candidate}`);
  }
  return absolute;
}

function citationFiles(receipt) {
  return new Set([
    ...receipt.findings.flatMap((finding) => finding.evidence.map((item) => item.path)),
    ...receipt.decisive_evidence.map((item) => item.path),
  ]);
}

export function inspectAgyStream({ packet, receipt, stdout, expectedModel }) {
  const events = parseEvents(stdout);
  const init = events.find((event) => event.type === "system" && event.subtype === "init")
    ?? events.find((event) => event.type === "init");
  if (!init) throw new Error("Agy stream has no initialization event");
  if (expectedModel && init.model !== expectedModel) {
    throw new Error(`Agy initialized model ${init.model ?? "unknown"}, expected ${expectedModel}`);
  }
  if (init.agent !== AGY_REVIEWER_AGENT) {
    throw new Error(`Agy initialized agent ${init.agent ?? "unknown"}, expected ${AGY_REVIEWER_AGENT}`);
  }
  if (!Array.isArray(init.tools)) throw new Error("Agy initialization did not report its available tools");
  const initializedTools = new Set(init.tools);
  if (initializedTools.size !== AGY_REVIEWER_TOOLS.size
    || [...AGY_REVIEWER_TOOLS].some((tool) => !initializedTools.has(tool))) {
    throw new Error(`Agy initialized an unexpected tool set: ${[...initializedTools].sort().join(", ")}`);
  }
  const final = [...events].reverse().find((event) => event.type === "result");
  if (!final || final.status !== "SUCCESS") throw new Error("Agy stream has no successful result event");

  const directReads = new Set();
  const toolCounts = {};
  for (const event of events) {
    const name = toolName(event);
    if (!name) continue;
    if (!ALLOWED_TOOLS.has(name)) throw new Error(`Agy used undeclared tool: ${name}`);
    toolCounts[name] = (toolCounts[name] ?? 0) + 1;
    const paths = candidatePaths(toolParameters(event));
    for (const candidate of paths) {
      const absolute = confinedPath(packet.root, candidate);
      if (DIRECT_READ_TOOLS.has(name) && toolCompleted(event)) directReads.add(absolute);
    }
  }

  for (const cited of citationFiles(receipt)) {
    const expected = path.resolve(packet.root, "evidence", cited);
    if (!directReads.has(expected)) throw new Error(`Agy trace does not show a completed direct read of cited file: ${cited}`);
  }
  return {
    status: "verified",
    conversation_id: final.conversation_id ?? init.conversation_id,
    model: init.model,
    agent: init.agent,
    available_tools: [...initializedTools].sort(),
    tool_counts: toolCounts,
    directly_read_files: directReads.size,
  };
}

export async function runAgyReview({
  packet,
  command = "agy",
  commandArgsPrefix = [],
  env = process.env,
  model = "gemini-3.7-flash-high",
  effort = "high",
  timeoutMs = 180_000,
  agentPath = path.resolve("app-server/experiments/grok-review-harness/agy-reviewer.agent.md"),
  conversationId,
  allowPro = false,
  onProgress,
} = {}) {
  const isFlash = /^gemini-[\w.-]*flash[\w.-]*$/i.test(model);
  const isPro = /^gemini-[\w.-]*pro[\w.-]*$/i.test(model);
  if (!isFlash && !(allowPro === true && isPro)) {
    throw new Error(`Agy review model must be Gemini Flash unless Pro is explicitly allowed, got ${model}`);
  }
  const packetAgentPath = path.join(packet.root, ".agents", "agents", AGY_REVIEWER_AGENT, "agent.md");
  await mkdir(path.dirname(packetAgentPath), { recursive: true });
  await copyFile(agentPath, packetAgentPath);
  await chmod(packetAgentPath, 0o444);
  const args = [
    ...commandArgsPrefix,
    "-p", buildReviewPrompt(packet.manifest),
    "--agent", AGY_REVIEWER_AGENT,
    "--new-project",
    "--model", model,
    "--effort", effort,
    "--sandbox",
    "--disable-slash-commands",
    "--output-format", "stream-json",
    "--json-schema", JSON.stringify(AGY_REVIEW_SCHEMA),
    "--print-timeout", `${Math.ceil(timeoutMs / 1_000)}s`,
  ];
  if (conversationId) args.push("--conversation", conversationId);
  const processResult = await runProcess(command, args, { cwd: packet.root, env, timeoutMs, onProgress });
  if (processResult.timedOut) return { status: "timeout", process: processResult };
  if (processResult.overflow) return { status: "output_limit", process: processResult };
  if (processResult.code !== 0) {
    return {
      status: classifyFailure(`${processResult.stdout}\n${processResult.stderr}`),
      process: processResult,
    };
  }

  try {
    const events = parseEvents(processResult.stdout);
    const final = [...events].reverse().find((event) => event.type === "result");
    if (!final || final.status !== "SUCCESS") throw new Error("Agy did not return a successful result event");
    const sessionId = final.conversation_id;
    const receipt = await admitReview({
      packet,
      response: final.structured_output,
      provider: "agy-gemini",
      sessionId,
      transportUsage: final.usage ?? null,
    });
    const trace = inspectAgyStream({ packet, receipt, stdout: processResult.stdout, expectedModel: model });
    return { status: "completed", receipt: { ...receipt, trace }, process: processResult };
  } catch (error) {
    let diagnostics = [];
    try {
      diagnostics = parseEvents(processResult.stdout).map((event) => ({
        type: event.type ?? null,
        subtype: event.subtype ?? null,
        step_type: event.step_type ?? null,
        status: event.status ?? null,
        tool_name: toolName(event),
        has_structured_output: Object.hasOwn(event, "structured_output"),
      }));
    } catch {
      // The primary rejection reason already reports malformed stream data.
    }
    return { status: "rejected_output", reason: error.message, diagnostics, process: processResult };
  }
}
