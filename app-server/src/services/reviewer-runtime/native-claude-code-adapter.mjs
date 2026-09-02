import { spawn } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { constants as fsConstants } from "node:fs";
import { access, chmod, copyFile, mkdir, readFile, readdir, rm, stat, writeFile } from "node:fs/promises";
import path from "node:path";

import {
  digest, ReviewerRuntimeError, validateCatalogProjection, validateRawEventPolicy,
} from "./contract.mjs";

const TOOLS = [
  "Read", "Glob", "Grep",
  "mcp__codebase-memory-mcp__list_projects",
  "mcp__codebase-memory-mcp__index_status",
  "mcp__codebase-memory-mcp__search_graph",
  "mcp__codebase-memory-mcp__trace_path",
  "mcp__codebase-memory-mcp__get_code_snippet",
  "mcp__codebase-memory-mcp__check_index_coverage",
  "mcp__codebase-memory-mcp__query_graph",
  "mcp__codebase-memory-mcp__get_architecture",
].join(",");

const SPECIALIST_MARKER = "WORK_ENGINE_AGENT_INSTRUCTION_REVIEW_V1";
const CLOUD_ROUTE_ENVIRONMENT = Object.freeze([
  "CLAUDE_CODE_USE_BEDROCK",
  "CLAUDE_CODE_USE_VERTEX",
  "CLAUDE_CODE_USE_FOUNDRY",
  "ANTHROPIC_BEDROCK_BASE_URL",
  "ANTHROPIC_VERTEX_BASE_URL",
  "ANTHROPIC_FOUNDRY_BASE_URL",
]);

function directAnthropicEnvironment(environment) {
  const configured = CLOUD_ROUTE_ENVIRONMENT.filter((name) => Object.hasOwn(environment, name));
  if (configured.length) {
    throw new ReviewerRuntimeError("configuration",
      `native Claude direct-Anthropic route refuses cloud routing environment: ${configured.join(", ")}`);
  }
  const env = {...environment};
  delete env.OPENROUTER_API_KEY; delete env.OPENROUTER_MANAGEMENT_KEY;
  delete env.ANTHROPIC_BASE_URL; delete env.ANTHROPIC_AUTH_TOKEN;
  return env;
}

function obligationInstructions(roleInstructions) {
  const specialistIndex = roleInstructions.indexOf(SPECIALIST_MARKER);
  if (specialistIndex === -1) return roleInstructions.trim();
  return `You perform one advisory, read-only agent-instruction specialist review of the exact immutable subject supplied by the host. The generic implementation-review protocol and schema do not apply to this obligation. Follow the loaded specialist and finding contracts, preserve their authority limits, and return only the bound specialist schema.\n\n${roleInstructions.slice(specialistIndex).trim()}`;
}

function execute({command, args, env, cwd}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {env, cwd, stdio: ["ignore", "pipe", "pipe"]});
    let stdout = "", stderr = "";
    child.stdout.setEncoding("utf8"); child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => stdout += chunk);
    child.stderr.on("data", (chunk) => stderr += chunk);
    child.on("error", reject);
    child.on("close", (exitCode) => resolve({exitCode, stdout, stderr}));
  });
}

function sessionUuid(instanceId) {
  const value = createHash("sha256").update(`work-engine-native-claude:${instanceId}`).digest("hex").slice(0, 32).split("");
  value[12] = "4"; value[16] = ["8", "9", "a", "b"][parseInt(value[16], 16) % 4];
  const compact = value.join("");
  return `${compact.slice(0, 8)}-${compact.slice(8, 12)}-${compact.slice(12, 16)}-${compact.slice(16, 20)}-${compact.slice(20)}`;
}

async function filesBelow(root) {
  const found = [];
  const visit = async (directory) => {
    for (const entry of await readdir(directory, {withFileTypes: true})) {
      const current = path.join(directory, entry.name);
      if (entry.isDirectory()) await visit(current);
      else if (entry.isFile()) found.push(current);
    }
  };
  try { await visit(root); } catch (error) { if (error?.code !== "ENOENT") throw error; }
  return found;
}

function assistantText(record) {
  if (record?.type !== "assistant" || record?.message?.role !== "assistant") return [];
  const content = record.message.content;
  if (typeof content === "string") return [content];
  if (!Array.isArray(content)) return [];
  return content.filter((item) => item?.type === "text" && typeof item.text === "string")
    .map((item) => item.text);
}

function structuredOutputs(record) {
  if (record?.type !== "assistant" || record?.message?.role !== "assistant"
      || !Array.isArray(record.message.content)) return [];
  return record.message.content
    .filter((item) => item?.type === "tool_use" && item.name === "StructuredOutput"
      && item.input && typeof item.input === "object" && !Array.isArray(item.input))
    .map((item) => item.input);
}

const string = {type: "string", minLength: 1};
const subjectSchema = {type: "object", required: ["commit", "tree", "patchIdentity"], properties: {
  commit: string, tree: string, patchIdentity: string,
}, additionalProperties: false};
const evidenceSchema = {type: "object", required: ["path", "startLine", "endLine", "sha256"], properties: {
  path: string, startLine: {type: "integer", minimum: 1}, endLine: {type: "integer", minimum: 1},
  sha256: {type: "string", pattern: "^[0-9a-f]{64}$"},
}, additionalProperties: false};
const findingSchema = {type: "object", required: ["id", "severity", "title", "evidence", "observed", "violatedExpectation", "consequence", "basis", "confidence", "recommendedRemediation", "status", "remediationEvidence"], properties: {
  id: string, severity: {type: "string", enum: ["critical", "high", "medium", "low", "info"]}, title: string,
  evidence: {type: "array", minItems: 1, items: evidenceSchema}, observed: string, violatedExpectation: string,
  consequence: string, basis: {type: "string", enum: ["reproduced", "inferred"]}, confidence: {type: "string", enum: ["high", "medium", "low"]},
  recommendedRemediation: string, status: {type: "string", enum: ["open", "verified_resolved"]},
  remediationEvidence: {type: "array", items: evidenceSchema},
}, additionalProperties: false};
const implementationSchema = {type: "object", required: ["schemaVersion", "subject", "verdict", "findings", "decisiveEvidence", "limitations"], properties: {
  schemaVersion: {const: 1}, subject: subjectSchema,
  verdict: {type: "string", enum: ["acceptable_as_is", "remediation_required", "incomplete"]},
  findings: {type: "array", items: findingSchema}, decisiveEvidence: {type: "array", minItems: 1, items: evidenceSchema},
  limitations: {type: "array", items: {type: "string"}},
}, additionalProperties: false};

const instructionFindingSchema = {type: "object", required: [
  "id", "severity", "instruction", "placement", "protectedDistinction", "causalExposure",
  "exactRouteNecessary", "authoritySource", "consequence", "confidence", "limitations", "advisoryOutcome",
], properties: {
  id: string, severity: {type: "string", enum: ["blocker", "high", "medium", "low", "info"]},
  instruction: {type: "object", required: ["fragmentId", "path", "startLine", "endLine"], properties: {
    fragmentId: string, path: string, startLine: {type: "integer", minimum: 1}, endLine: {type: "integer", minimum: 1},
  }, additionalProperties: false},
  placement: {type: "object", required: ["semanticOwner", "consumer", "audience", "scope", "precedence", "loadingReach"], properties: {
    semanticOwner: string, consumer: string, audience: string, scope: string,
    precedence: {type: "string", enum: ["system", "developer", "role", "skill", "subject"]}, loadingReach: string,
  }, additionalProperties: false},
  protectedDistinction: string,
  causalExposure: {type: "object", required: ["reasonLoaded", "failureModeLoaded", "sourceTraceable", "reviewerReconstructed"], properties: {
    reasonLoaded: {type: "boolean"}, failureModeLoaded: {type: "boolean"}, sourceTraceable: {type: "boolean"}, reviewerReconstructed: {type: "boolean"},
  }, additionalProperties: false},
  exactRouteNecessary: {type: "boolean"}, authoritySource: string, consequence: string,
  confidence: {type: "string", enum: ["high", "medium", "low"]}, limitations: {type: "array", items: {type: "string"}},
  advisoryOutcome: {type: "string", enum: ["retain", "restate", "split", "move", "demote", "remove"]},
}, additionalProperties: false};

function outputSchema(specialist) {
  if (!specialist) return implementationSchema;
  return {type: "object", required: ["schemaVersion", "perspective", "subject", "closureRevision", "applicability", "applicabilityReason", "result", "findingDetails", "limitations"], properties: {
    schemaVersion: {const: 1}, perspective: {const: "agent-instruction-review"}, subject: subjectSchema,
    closureRevision: {type: "string", pattern: "^[0-9a-f]{64}$"}, applicability: {enum: ["applicable", "omitted"]},
    applicabilityReason: string, result: implementationSchema,
    findingDetails: {type: "array", items: instructionFindingSchema}, limitations: {type: "array", minItems: 1, items: string},
  }, additionalProperties: false};
}

function parseResult(stdout, expectedSession) {
  let envelope;
  try { envelope = JSON.parse(stdout); }
  catch { throw new ReviewerRuntimeError("output", "native Claude returned malformed JSON"); }
  if (envelope?.session_id !== expectedSession) {
    throw new ReviewerRuntimeError("continuity", "native Claude returned a different session UUID");
  }
  if (envelope?.type !== "result" || envelope?.subtype !== "success" || !envelope.structured_output) {
    throw new ReviewerRuntimeError("output", "native Claude did not return a successful structured result");
  }
  return {envelope, result: envelope.structured_output};
}

export class NativeClaudeCodeReviewerAdapter {
  constructor({registry, workspaceRoot, stateRoot, executeProcess = execute,
    pythonExecutable = "python3", claudeExecutable = "claude",
    transportScript = "skills/claude-recon-implementation/scripts/claude_transport.py",
    catalogSource = null, baseEnvironment = process.env, credentialSourcePath = null} = {}) {
    if (!registry?.admit) throw new TypeError("native Claude adapter requires a reviewer registry");
    if (!workspaceRoot || !stateRoot) throw new TypeError("native Claude adapter requires host-owned workspace and state roots");
    Object.assign(this, {registry, workspaceRoot: path.resolve(workspaceRoot), stateRoot: path.resolve(stateRoot),
      executeProcess, pythonExecutable, claudeExecutable,
      transportScript: path.resolve(workspaceRoot, transportScript),
      catalogSource: catalogSource === null ? null : Object.freeze(structuredClone(catalogSource)),
      baseEnvironment: Object.freeze({...baseEnvironment}),
      credentialSourcePath: path.resolve(credentialSourcePath
        ?? path.join(baseEnvironment.CLAUDE_CONFIG_DIR
          ?? path.join(baseEnvironment.HOME ?? "", ".claude"), ".credentials.json"))});
  }

  runtimeSessionId(instanceId) { return sessionUuid(instanceId); }

  async #seedCredentials(configRoot, {refresh = false} = {}) {
    const destination = path.join(configRoot, ".credentials.json");
    if (!refresh) {
      try { await access(destination); await chmod(destination, 0o600); return false; }
      catch (error) { if (error?.code !== "ENOENT") throw error; }
    }
    try {
      await copyFile(this.credentialSourcePath, destination,
        refresh ? 0 : fsConstants.COPYFILE_EXCL);
      await chmod(destination, 0o600);
      return true;
    } catch (error) {
      if (!refresh && error?.code === "EEXIST") { await chmod(destination, 0o600); return false; }
      throw new ReviewerRuntimeError("authentication",
        "native Claude subscription credentials are unavailable to the isolated reviewer");
    }
  }

  async recoverFailure(instanceId) {
    const instanceRoot = path.join(this.stateRoot, "native-claude", digest(instanceId));
    const expectedSession = this.runtimeSessionId(instanceId);
    const files = await filesBelow(instanceRoot);
    let attempt;
    try { attempt = JSON.parse(await readFile(path.join(instanceRoot, "latest-attempt.json"), "utf8")); }
    catch { return null; }
    if (attempt?.schemaVersion !== 1 || attempt.sessionId !== expectedSession
        || typeof attempt.transportReceipt !== "string"
        || path.basename(attempt.transportReceipt) !== attempt.transportReceipt
        || !attempt.transportReceipt.endsWith(".transport.json")) return null;
    const receiptFile = files.find((value) => path.basename(value) === attempt.transportReceipt);
    if (!receiptFile) return null;
    let receipt;
    try { receipt = {file: receiptFile, value: JSON.parse(await readFile(receiptFile, "utf8")),
      mtime: (await stat(receiptFile)).mtimeMs}; }
    catch { return null; }
    if (!receipt || receipt.value?.request?.session_id !== expectedSession
        || receipt.value?.result !== "failed") return null;
    const sessionFile = files.find((value) => path.basename(value) === `${expectedSession}.jsonl`);
    if (!sessionFile) return null;
    const sessionBytes = await readFile(sessionFile);
    const records = sessionBytes.toString("utf8").split("\n").filter(Boolean).flatMap((line) => {
      try { return [JSON.parse(line)]; } catch { return []; }
    });
    const attemptDuration = receipt.value?.attempts?.at(-1)?.duration_ms;
    if (!Number.isFinite(attemptDuration) || attemptDuration < 0) return null;
    const attemptStartedAt = receipt.mtime - attemptDuration - 1_000;
    const attemptCompletedAt = receipt.mtime + 1_000;
    const authenticationRequired = records.some((record) => {
      const observedAt = Date.parse(record?.timestamp);
      return Number.isFinite(observedAt) && observedAt >= attemptStartedAt
        && observedAt <= attemptCompletedAt
        && assistantText(record).some((text) => text.trim() === "Not logged in · Please run /login");
    });
    if (!authenticationRequired) return null;
    return Object.freeze({schemaVersion: 1, failureSignature: "authentication_required",
      providerEntry: "not_entered", sessionAvailable: true, sessionId: expectedSession,
      transportReceiptDigest: digest(receipt.value),
      sessionArtifactDigest: createHash("sha256").update(sessionBytes).digest("hex")});
  }

  async recoverResult(instanceId, subject) {
    const instanceRoot = path.join(this.stateRoot, "native-claude", digest(instanceId));
    const expectedSession = this.runtimeSessionId(instanceId);
    const files = await filesBelow(instanceRoot);
    const receipts = [];
    for (const file of files.filter((value) => value.endsWith(".transport.json"))) {
      try { receipts.push({file, value: JSON.parse(await readFile(file, "utf8")), mtime: (await stat(file)).mtimeMs}); }
      catch {}
    }
    const receipt = receipts.sort((left, right) => right.mtime - left.mtime)
      .find(({value}) => value?.request?.session_id === expectedSession && value?.result === "success") ?? null;
    if (!receipt) return null;
    const sessionFile = files.find((value) => path.basename(value) === `${expectedSession}.jsonl`);
    if (!sessionFile) return null;
    const sessionBytes = await readFile(sessionFile);
    const records = sessionBytes.toString("utf8").split("\n").filter(Boolean).flatMap((line) => {
      try { return [JSON.parse(line)]; } catch { return []; }
    });
    const outputs = records.flatMap((record) => structuredOutputs(record).map((result) => ({
      result, observedAt: Date.parse(record.timestamp),
    }))).filter(({observedAt}) => Number.isFinite(observedAt) && observedAt <= receipt.mtime + 1_000);
    const result = outputs.at(-1)?.result ?? null;
    const implementationResult = result?.result ?? result;
    if (!implementationResult?.subject || digest(implementationResult.subject) !== digest(subject)) return null;
    return Object.freeze({schemaVersion: 1, providerEntry: "entered",
      sessionAvailable: true, sessionId: expectedSession, result: Object.freeze(structuredClone(result)),
      resultDigest: digest(result), subjectDigest: digest(implementationResult.subject),
      transportReceiptDigest: digest(receipt.value),
      sessionArtifactDigest: createHash("sha256").update(sessionBytes).digest("hex")});
  }

  async execute({instanceId, profileId, subject, catalogProjection, rawEventPolicy,
    continuationSessionId = null, roleInstructions, resultCorrection = null,
    refreshCredentials = false}) {
    if (typeof instanceId !== "string" || !instanceId.trim()) throw new ReviewerRuntimeError("configuration", "native Claude instanceId is required");
    if (typeof roleInstructions !== "string" || !roleInstructions.trim()) throw new ReviewerRuntimeError("configuration", "canonical role instructions are required");
    const {profile, registryRevision} = this.registry.admit(profileId);
    if (profile.provider !== "anthropic" || profile.requestedModel !== "sonnet") {
      throw new ReviewerRuntimeError("configuration", "native Claude adapter admits only the direct Anthropic sonnet profile");
    }
    validateCatalogProjection(catalogProjection); validateRawEventPolicy(rawEventPolicy);
    if (this.catalogSource !== null
        && (catalogProjection.source !== this.catalogSource.source
          || catalogProjection.sourceSha256 !== this.catalogSource.sourceSha256)) {
      throw new ReviewerRuntimeError("catalog", "native Claude catalog provenance differs from the admitted reviewer profile source");
    }
    const catalogModel = catalogProjection.models.find(({slug, provider}) =>
      slug === profile.requestedModel && provider === profile.provider);
    if (!catalogModel) throw new ReviewerRuntimeError("catalog", "native Claude profile is absent from the bound catalog");
    const expectedSession = this.runtimeSessionId(instanceId);
    if (continuationSessionId !== null && continuationSessionId !== expectedSession) {
      throw new ReviewerRuntimeError("continuity", "native Claude continuation differs from the pre-registered session");
    }
    if (typeof refreshCredentials !== "boolean" || (refreshCredentials && continuationSessionId !== expectedSession)) {
      throw new ReviewerRuntimeError("authentication",
        "native Claude credential refresh requires the exact retained session");
    }
    const instanceRoot = path.join(this.stateRoot, "native-claude", digest(instanceId));
    const configRoot = path.join(instanceRoot, "config");
    await mkdir(configRoot, {recursive: true, mode: 0o700});
    const attemptId = randomUUID();
    try { await this.#seedCredentials(configRoot, {refresh: refreshCredentials}); }
    catch (error) {
      if (!(error instanceof ReviewerRuntimeError)) throw error;
      const recovery = Object.freeze({schemaVersion: 1,
        failureSignature: "authentication_unavailable", providerEntry: "not_entered",
        sessionAvailable: false, sessionId: expectedSession});
      return Object.freeze({attemptId, failure: {kind: "authentication", message: error.message,
        providerEntry: "not_entered", failureSignature: "authentication_unavailable",
        sessionAvailable: false, recovery}, result: null, runtimeSessionId: expectedSession,
        transportReceipt: null});
    }
    const mcpPath = path.join(instanceRoot, "mcp.json");
    const receiptPath = path.join(instanceRoot, `${randomUUID()}.transport.json`);
    await writeFile(mcpPath, `${JSON.stringify({mcpServers: {"codebase-memory-mcp": {
      type: "stdio", command: "codebase-memory-mcp", args: [],
    }}}, null, 2)}\n`, {mode: 0o600});
    const specialist = roleInstructions.includes(SPECIALIST_MARKER);
    const selectedInstructions = obligationInstructions(roleInstructions);
    const task = resultCorrection === null
      ? `Review only the immutable subject below. Return only the required structured result. Do not mutate files, run gates, select reviewers, accept work, or use network tools.\n\nSUBJECT\n${JSON.stringify(subject)}`
      : `This is a same-session correction of your previously returned structured result, not a new review. Do not repeat repository reconnaissance or invoke tools. Preserve the exact subject, findings, and evidence unless the stated contract rejection itself requires a semantic correction. Reconcile your own verdict, findings, decisive evidence, and limitations, then return only one corrected structured result. The host will apply the unchanged canonical validator; the host is not choosing or rewriting your judgment.\n\nCONTRACT REJECTION\n${resultCorrection.message}\n\nPREVIOUS STRUCTURED RESULT\n${JSON.stringify(resultCorrection.rejectedResult)}\n\nSUBJECT\n${JSON.stringify(subject)}`;
    const prompt = `${selectedInstructions}\n\nExecution-profile constraints are subordinate to the selected review obligation and its canonical instructions:\n${profile.effectiveInstructions}\n\nKnown execution limitations:\n${profile.limitations.length ? profile.limitations.map((limitation) => `- ${limitation}`).join("\n") : "- None declared."}\n\n${task}`;
    const claudeArgs = ["-p", "--effort", profile.reasoning, "--model", profile.requestedModel,
      ...(continuationSessionId === null ? ["--session-id", expectedSession] : ["--resume", expectedSession]),
      "--strict-mcp-config", "--mcp-config", mcpPath, "--tools", TOOLS,
      "--output-format", "json", "--json-schema", JSON.stringify(outputSchema(specialist)),
      "--dangerously-skip-permissions", prompt];
    const args = [this.transportScript, "--transport", "anthropic", "--continuity", "retained",
      "--receipt", receiptPath, "--", this.claudeExecutable, ...claudeArgs];
    const env = {...directAnthropicEnvironment(this.baseEnvironment), CLAUDE_CONFIG_DIR: configRoot};
    await writeFile(path.join(instanceRoot, "latest-attempt.json"), `${JSON.stringify({
      schemaVersion: 1, attemptId, sessionId: expectedSession,
      transportReceipt: path.basename(receiptPath),
    })}\n`, {mode: 0o600});
    let transport;
    try { transport = await this.executeProcess({command: this.pythonExecutable, args, env, cwd: this.workspaceRoot}); }
    catch (error) { throw new ReviewerRuntimeError("spawn", `native Claude process start failed: ${error.message}`); }
    let transportReceipt = null;
    try { transportReceipt = JSON.parse(await readFile(receiptPath, "utf8")); } catch {}
    if (transport.exitCode !== 0) {
      const recovery = await this.recoverFailure(instanceId);
      return Object.freeze({attemptId, failure: {kind: "transport", message: `native Claude exited ${transport.exitCode}`,
        providerEntry: recovery?.providerEntry ?? "unknown",
        failureSignature: recovery?.failureSignature ?? null,
        sessionAvailable: recovery?.sessionAvailable ?? false,
        ...(recovery ? {recovery} : {})}, result: null,
        runtimeSessionId: expectedSession, transportReceipt});
    }
    const {envelope, result} = parseResult(transport.stdout, expectedSession);
    if (digest(result.subject) !== digest(subject)) {
      return Object.freeze({attemptId, failure: {kind: "subject_drift", message: "native Claude result subject differs"}, result: null,
        runtimeSessionId: expectedSession, transportReceipt});
    }
    return Object.freeze({attemptId, failure: null, result, runtimeSessionId: expectedSession,
      receipt: Object.freeze({schemaVersion: 1, attemptId, profileId,
        profileConfigurationDigest: profile.configurationDigest, registryRevision,
        harness: "claude-code", gateway: "anthropic", requestedModel: profile.requestedModel,
        observedModel: envelope.model ?? Object.keys(envelope.modelUsage ?? {})[0] ?? "unknown",
        claudeVersion: transportReceipt?.claude_version ?? "unknown", sessionId: expectedSession,
        continuity: continuationSessionId === null ? "fresh_initial" : "same_session_resume",
        mutationAuthorized: false, transportReceiptDigest: transportReceipt ? digest(transportReceipt) : null}),
      transportReceipt});
  }

  async retire(instanceId) {
    await rm(path.join(this.stateRoot, "native-claude", digest(instanceId)), {recursive: true, force: true});
    return true;
  }
}
