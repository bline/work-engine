import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { digest, ReviewerRuntimeError, validateCatalogProjection, validateExecutionReceipt, validateRawEventPolicy } from "./contract.mjs";

function run({ command, args, env, cwd, input }) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, env, stdio: ["pipe", "pipe", "pipe"] });
    let stdout = "", stderr = "";
    child.stdout.setEncoding("utf8"); child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => stdout += chunk); child.stderr.on("data", (chunk) => stderr += chunk);
    child.on("error", reject); child.on("close", (exitCode) => resolve({ exitCode, stdout, stderr }));
    child.stdin.end(input);
  });
}
function catalogModel(profile, catalog, now) {
  validateCatalogProjection(catalog);
  if (Date.parse(catalog.observedAt) > now || Date.parse(catalog.expiresAt) <= now) throw new ReviewerRuntimeError("catalog", "catalog is not fresh");
  const model = catalog.models.find((candidate) => candidate.slug === profile.requestedModel && candidate.provider === profile.provider);
  if (!model) throw new ReviewerRuntimeError("catalog", "requested model/provider absent");
  for (const capability of profile.capabilities) if (!model.capabilities.includes(capability)) throw new ReviewerRuntimeError("catalog", `missing capability ${capability}`);
  if (model.routingConstraints.length) throw new ReviewerRuntimeError("catalog", "routing constraints unresolved");
  return model;
}
function evidence(events, policy) {
  const eventDigest = digest(events);
  return policy.exactRetentionAuthorized
    ? { mode: "exact", events, eventDigest, omitted: [] }
    : { mode: "authenticated_projection", events: null, eventDigest, eventCount: events.length, eventTypes: [...new Set(events.map((event) => event.type))].sort(), omitted: ["raw event bodies"] };
}

export class OpenRouterCodexReviewerAdapter {
  constructor({ registry, executeProcess = run, executable = "codex", isolatedRoot = os.tmpdir(), now = () => Date.now() }) {
    if (!registry?.admit) throw new TypeError("adapter requires registry");
    Object.assign(this, { registry, executeProcess, executable, isolatedRoot, now });
    this.retainedHomes = new Map();
  }
  async execute({ instanceId, profileId, subject, catalogProjection, rawEventPolicy, continuationSessionId = null, roleInstructions }) {
    if (typeof instanceId !== "string" || !instanceId.trim()) throw new ReviewerRuntimeError("configuration", "reviewer instanceId is required");
    if (typeof roleInstructions !== "string" || !roleInstructions.trim()) throw new ReviewerRuntimeError("configuration", "canonical role instructions are required");
    const attemptId = randomUUID(), subjectDigest = digest(subject);
    const { profile, registryRevision } = this.registry.admit(profileId);
    const model = catalogModel(profile, catalogProjection, this.now());
    validateRawEventPolicy(rawEventPolicy);
    await mkdir(this.isolatedRoot, { recursive: true });
    let home = this.retainedHomes.get(instanceId);
    const freshEntry = continuationSessionId === null;
    if (freshEntry) {
      if (home) throw new ReviewerRuntimeError("continuity", "reviewer instance already has retained state");
      home = await mkdtemp(path.join(this.isolatedRoot, "work-engine-reviewer-"));
      this.retainedHomes.set(instanceId, home);
    } else if (!home) throw new ReviewerRuntimeError("continuity", "continuation requires retained reviewer state");
    try {
      await writeFile(path.join(home, "config.toml"), `model = ${JSON.stringify(profile.requestedModel)}\nmodel_provider = ${JSON.stringify(profile.provider)}\nreasoning_effort = ${JSON.stringify(profile.reasoning)}\n`, { mode: 0o600 });
      let transport;
      try {
        const instructions = `${roleInstructions.trim()}\n\nExecution-profile constraints are subordinate to the canonical role instructions:\n${profile.effectiveInstructions}\n\nKnown execution limitations:\n${profile.limitations.length ? profile.limitations.map((limitation) => `- ${limitation}`).join("\n") : "- None declared."}`;
        transport = await this.executeProcess({ command: this.executable, args: ["exec", "--json", "--model", profile.requestedModel, ...(continuationSessionId ? ["resume", continuationSessionId] : [])], env: { CODEX_HOME: home, PATH: process.env.PATH ?? "" }, cwd: home, input: JSON.stringify({ subject, outputSchema: profile.outputSchema, instructions }) });
      } catch (error) { throw new ReviewerRuntimeError("spawn", `process start failed: ${error.message}`); }
      let events;
      try { events = transport.stdout.trim() ? transport.stdout.trim().split(/\r?\n/).map(JSON.parse) : []; }
      catch { throw new ReviewerRuntimeError("output", "malformed JSON events"); }
      const completed = events.findLast((event) => event.type === "review.completed");
      let failure = null, result = null;
      if (transport.exitCode !== 0) failure = { kind: "transport", message: `process exited ${transport.exitCode}` };
      else if (!completed?.result) failure = { kind: "output", message: "completed result missing" };
      else if (digest(completed.result.subject) !== subjectDigest) failure = { kind: "subject_drift", message: "result subject differs" };
      else result = completed.result;
      const observed = completed?.observed ?? {};
      const receipt = { schemaVersion: 1, attemptId, profileId, profileConfigurationDigest: profile.configurationDigest, subjectDigest, catalog: { catalogId: catalogProjection.catalogId, sourceSha256: catalogProjection.sourceSha256, observedAt: catalogProjection.observedAt, expiresAt: catalogProjection.expiresAt, model: model.slug }, configured: { requestedModel: profile.requestedModel, provider: profile.provider, reasoning: profile.reasoning, capabilities: profile.capabilities, outputSchema: profile.outputSchema, registryRevision }, observed: { model: observed.model ?? "unknown", provider: observed.provider ?? "unknown", servingVariant: observed.servingVariant ?? "unknown" }, isolation: { codexHomeIsolated: true, freshEntry, continuation: !freshEntry, builderContextInherited: false, mutationAuthorized: false }, transport: { executable: this.executable, exitCode: transport.exitCode, stderrDigest: digest(transport.stderr ?? "") }, rawEvidence: { policy: rawEventPolicy, ...evidence(events, rawEventPolicy) }, failure, result };
      validateExecutionReceipt(receipt);
      return Object.freeze(structuredClone(receipt));
    } catch (error) {
      if (freshEntry) await this.retire(instanceId);
      throw error;
    }
  }
  async retire(instanceId) {
    const home = this.retainedHomes.get(instanceId);
    if (!home) return false;
    this.retainedHomes.delete(instanceId);
    await rm(home, { recursive: true, force: true });
    return true;
  }
}
