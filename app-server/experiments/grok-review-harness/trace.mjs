import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const MAX_TRACE_OUTPUT = 64 * 1024 * 1024;
const ALLOWED_TOOLS = new Set(["read_file", "grep", "list_dir"]);

export class GrokReviewTraceError extends Error {}

function parseJsonLines(text, field) {
  return text.split("\n").filter(Boolean).map((line, index) => {
    try {
      return JSON.parse(line);
    } catch (error) {
      throw new GrokReviewTraceError(`${field} line ${index + 1} is invalid JSON: ${error.message}`);
    }
  });
}

function confinedPath(packetRoot, requested, field) {
  if (typeof requested !== "string" || requested.length === 0) {
    throw new GrokReviewTraceError(`${field} must name a packet path`);
  }
  const resolved = path.resolve(packetRoot, requested);
  const prefix = `${path.resolve(packetRoot)}${path.sep}`;
  if (resolved !== path.resolve(packetRoot) && !resolved.startsWith(prefix)) {
    throw new GrokReviewTraceError(`${field} escapes the immutable packet: ${requested}`);
  }
  return resolved;
}

export function validateGrokTraceEvents({ packet, receipt, updatesText }) {
  const updates = parseJsonLines(updatesText, "updates.jsonl");
  const calls = new Map();
  const completed = new Set();

  for (const entry of updates) {
    const update = entry?.params?.update;
    if (update?.sessionUpdate === "tool_call") {
      const name = update.title;
      if (!ALLOWED_TOOLS.has(name)) {
        throw new GrokReviewTraceError(`provider invoked undeclared tool ${name}`);
      }
      calls.set(update.toolCallId, { name, rawInput: update.rawInput ?? {} });
    }
    if (update?.sessionUpdate === "tool_call_update" && update.status === "completed") {
      completed.add(update.toolCallId);
    }
  }

  const readPaths = new Set();
  const toolCounts = { read_file: 0, grep: 0, list_dir: 0 };
  for (const [id, call] of calls) {
    if (!completed.has(id)) continue;
    toolCounts[call.name] += 1;
    if (call.name === "read_file") {
      readPaths.add(confinedPath(packet.root, call.rawInput.target_file, "read_file target"));
    } else if (call.name === "list_dir") {
      confinedPath(packet.root, call.rawInput.target_directory, "list_dir target");
    } else if (call.rawInput.path != null) {
      confinedPath(packet.root, call.rawInput.path, "grep path");
    }
  }
  if ([...completed].filter((id) => calls.has(id)).length === 0) {
    throw new GrokReviewTraceError("provider completed no declared evidence tool call");
  }

  const citations = [
    ...receipt.findings.flatMap((finding) => finding.evidence),
    ...receipt.decisive_evidence,
  ];
  for (const citation of citations) {
    const expected = path.resolve(packet.root, "evidence", ...citation.path.split("/"));
    if (!readPaths.has(expected)) {
      throw new GrokReviewTraceError(`provider trace does not show a direct read of cited file ${citation.path}`);
    }
  }

  return { tool_counts: toolCounts, directly_read_files: [...readPaths].length };
}

export async function inspectGrokSessionTrace({
  command = "grok",
  commandArgsPrefix = [],
  packet,
  receipt,
  sessionId,
  timeoutMs = 30_000,
} = {}) {
  const traceRoot = await mkdtemp(path.join(tmpdir(), "work-engine-grok-trace-"));
  const archivePath = path.join(traceRoot, `${sessionId}.tar.gz`);
  try {
    await execFileAsync(command, [
      ...commandArgsPrefix,
      "trace", sessionId, "--local", "--output", archivePath, "--json",
    ], {
      cwd: packet.root,
      encoding: "utf8",
      timeout: timeoutMs,
      maxBuffer: MAX_TRACE_OUTPUT,
      windowsHide: true,
    });
    const { stdout: listing } = await execFileAsync("tar", ["-tzf", archivePath], {
      encoding: "utf8",
      timeout: timeoutMs,
      maxBuffer: MAX_TRACE_OUTPUT,
    });
    const updatesEntry = listing.split("\n").find((entry) => entry.endsWith("/updates.jsonl"));
    if (!updatesEntry) throw new GrokReviewTraceError("provider trace has no updates.jsonl");
    const { stdout: updatesText } = await execFileAsync("tar", ["-xOzf", archivePath, updatesEntry], {
      encoding: "utf8",
      timeout: timeoutMs,
      maxBuffer: MAX_TRACE_OUTPUT,
    });
    const evidence = validateGrokTraceEvents({ packet, receipt, updatesText });
    const archive = await readFile(archivePath);
    return {
      status: "verified",
      session_id: sessionId,
      archive_path: archivePath,
      archive_sha256: createHash("sha256").update(archive).digest("hex"),
      ...evidence,
    };
  } catch (error) {
    await rm(traceRoot, { recursive: true, force: true });
    if (error instanceof GrokReviewTraceError) throw error;
    throw new GrokReviewTraceError(`provider trace inspection failed: ${error.message}`);
  }
}

