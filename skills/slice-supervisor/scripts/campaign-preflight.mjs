#!/usr/bin/env node
import fs from "node:fs/promises";
import crypto from "node:crypto";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { parse } from "yaml";
import { loadChromeVisionConfig, resolveChromeVisionConfig } from "../../chrome-vision/src/config.mjs";

const allowed = new Set(["version","objective","work_source","builder","validation","metrics","limits","approval","notifications","stop_on","capabilities","slice_completion_commit","amendments"]);
const currentBuilderModels = new Set(["gpt-6.1-terra", "gpt-6.1-sol", "gpt-6-sol", "gpt-6-astra"]);
const currentReasoningEfforts = new Set(["low", "medium", "high", "xhigh", "max", "ultra"]);
const providerResolver = path.resolve(import.meta.dirname, "../../slice-builder/scripts/resolve_provider.py");
const defaults = {
  work_source: null,
  builder: {
    skill: "slice-builder",
    model: "gpt-6.1-terra", // docs/model-selection.md
    reasoning_effort: "medium",
    context: {
      repository_evidence: { skill: "repo-search", provider: "codex-codebase-memory" },
      independent_review: { skill: "claude-recon-implementation", provider: "claude" },
    },
  },
  validation: {
    profile: "engineering-proportional",
    requirements: ["semantic_proof", "risk_proportional_checks", "workspace_integrity"],
  },
  metrics: { path: "docs/ai-workflow-metrics.jsonl" },
  limits: {},
  approval: { plan: "procedural_when_safe", uninterrupted_after_plan: false },
  notifications: { intervention: true, completion: false },
  slice_completion_commit: { prompt: "enabled" },
  amendments: [],
  stop_on: ["objective_complete", "human_judgment", "unresolved_architecture", "quota_exhaustion", "validation_nonconvergence"],
};

function mergeDefaults(base, authored) {
  if (!base || typeof base !== "object" || Array.isArray(base) || !authored || typeof authored !== "object" || Array.isArray(authored)) return structuredClone(authored);
  const merged = structuredClone(base);
  for (const [key, value] of Object.entries(authored)) {
    merged[key] = key in merged ? mergeDefaults(merged[key], value) : structuredClone(value);
  }
  return merged;
}

export async function preflightCampaign(campaignPath, { cwd = process.cwd() } = {}) {
  const resolvedCampaignPath = path.resolve(cwd, campaignPath);
  let campaign;
  let campaignBytes;
  try { campaignBytes = await fs.readFile(resolvedCampaignPath); campaign = parse(campaignBytes.toString("utf8")); }
  catch (error) { throw new Error(`Cannot read campaign configuration ${resolvedCampaignPath}: ${error.message}`); }
  if (!campaign || typeof campaign !== "object" || Array.isArray(campaign)) throw new Error("Campaign configuration must be a mapping");
  if (campaign.kind === "comparative_repository_analysis") {
    throw new Error(`Comparative repository analysis definitions are not slice-supervisor campaigns. Run: python3 skills/comparative-repository-analysis/scripts/comparison_artifacts.py prepare-run --config ${campaignPath} --output comparison-contract.json`);
  }
  const unknown = Object.keys(campaign).filter(key => !allowed.has(key));
  if (unknown.length) throw new Error(`Unknown campaign fields: ${unknown.sort().join(", ")}`);
  if (campaign.version !== 2) throw new Error("Campaign configuration version 2 is required for capabilities");
  if (typeof campaign.objective !== "string" || !campaign.objective.trim()) throw new Error("Campaign objective must be a nonempty string");
  if (campaign.capabilities !== undefined && (!campaign.capabilities || typeof campaign.capabilities !== "object" || Array.isArray(campaign.capabilities))) throw new Error("capabilities must be a mapping");
  const unknownCapabilities = Object.keys(campaign.capabilities || {}).filter(key => key !== "chrome_vision");
  if (unknownCapabilities.length) throw new Error(`Unknown capabilities: ${unknownCapabilities.sort().join(", ")}`);
  const declaration = campaign.capabilities?.chrome_vision;
  const authoredFields = Object.keys(campaign).filter(key => allowed.has(key));
  const engineConfig = mergeDefaults(defaults, campaign);
  const authoredContext = campaign.builder?.context;
  if (!engineConfig.builder || typeof engineConfig.builder !== "object" || Array.isArray(engineConfig.builder) || !engineConfig.builder.context || typeof engineConfig.builder.context !== "object" || Array.isArray(engineConfig.builder.context)) {
    throw new Error("builder and builder.context must be mappings");
  }
  if (authoredContext && Object.hasOwn(authoredContext, "independent_review") && Object.hasOwn(authoredContext, "adversarial_review")) {
    throw new Error("builder.context must configure exactly one review role, not both independent_review and adversarial_review");
  }
  if (authoredContext && Object.hasOwn(authoredContext, "adversarial_review")) {
    delete engineConfig.builder.context.independent_review;
  }
  if (!currentBuilderModels.has(engineConfig.builder.model)) {
    throw new Error("builder.model must be a supported current model");
  }
  if (!currentReasoningEfforts.has(engineConfig.builder.reasoning_effort)) {
    throw new Error("builder.reasoning_effort must be supported by the current builder model");
  }
  try {
    execFileSync("python3", [providerResolver, "--config-version", "2", "--context-json", JSON.stringify(engineConfig.builder.context), "--builder-model", engineConfig.builder.model, "--builder-reasoning-effort", engineConfig.builder.reasoning_effort], { encoding: "utf8" });
  } catch (error) {
    throw new Error(`builder.context is invalid: ${error.stderr?.toString().trim() || error.message}`);
  }
  if (!engineConfig.slice_completion_commit || !["enabled", "disabled"].includes(engineConfig.slice_completion_commit.prompt) || Object.keys(engineConfig.slice_completion_commit).some(key => key !== "prompt")) {
    throw new Error("slice_completion_commit must contain only prompt: enabled or disabled");
  }
  engineConfig.source = campaignPath;
  engineConfig.explicit_fields = authoredFields;
  engineConfig.defaulted_fields = Object.keys(defaults).filter(key => !(key in campaign));
  const amendmentFields = new Set(["timestamp", "changed_fields", "prior_values", "new_values", "reason", "human_approval"]);
  if (!Array.isArray(engineConfig.amendments)) throw new Error("amendments must be an array");
  for (const [index, amendment] of engineConfig.amendments.entries()) {
    if (!amendment || typeof amendment !== "object" || Array.isArray(amendment)) throw new Error(`amendments[${index}] must be a mapping`);
    const unknownAmendmentFields = Object.keys(amendment).filter(key => !amendmentFields.has(key));
    const missingAmendmentFields = [...amendmentFields].filter(key => !(key in amendment));
    if (unknownAmendmentFields.length || missingAmendmentFields.length) throw new Error(`amendments[${index}] must contain exactly timestamp, changed_fields, prior_values, new_values, reason, and human_approval`);
    if (typeof amendment.timestamp !== "string" || !amendment.timestamp.trim()) throw new Error(`amendments[${index}].timestamp must be nonempty`);
    if (!Array.isArray(amendment.changed_fields) || amendment.changed_fields.length === 0 || amendment.changed_fields.some(value => typeof value !== "string" || !value.trim())) throw new Error(`amendments[${index}].changed_fields must contain nonempty strings`);
    for (const key of ["prior_values", "new_values"]) if (!amendment[key] || typeof amendment[key] !== "object" || Array.isArray(amendment[key])) throw new Error(`amendments[${index}].${key} must be a mapping`);
    for (const key of ["reason", "human_approval"]) if (typeof amendment[key] !== "string" || !amendment[key].trim()) throw new Error(`amendments[${index}].${key} must be nonempty`);
  }
  const campaignSource = {
    source: "file",
    authoredReference: campaignPath,
    resolvedPath: resolvedCampaignPath,
    pathBase: path.dirname(resolvedCampaignPath),
    sha256: crypto.createHash("sha256").update(campaignBytes).digest("hex"),
    schemaVersion: 1,
  };
  if (!declaration) return { campaignPath: resolvedCampaignPath, engineConfig, campaignSource, resolvedCapabilities: {} };
  if (!declaration || typeof declaration !== "object" || Array.isArray(declaration) || Object.keys(declaration).some(key => key !== "config") || !("config" in declaration)) throw new Error("capabilities.chrome_vision must contain only config");
  const baseDirectory = path.dirname(resolvedCampaignPath);
  let resolved;
  let durable;
  if (typeof declaration.config === "string") {
    resolved = await loadChromeVisionConfig(declaration.config, { cwd: baseDirectory });
    durable = { source: "file", authoredReference: declaration.config, resolvedPath: resolved.identity.resolvedPath, pathBase: resolved.identity.pathBase, sha256: resolved.identity.sha256, schemaVersion: resolved.identity.schemaVersion };
  } else if (declaration.config && typeof declaration.config === "object" && !Array.isArray(declaration.config)) {
    resolved = resolveChromeVisionConfig(declaration.config, { baseDirectory, source: resolvedCampaignPath });
    durable = { source: "inline", campaignPath: resolvedCampaignPath, pathBase: resolved.identity.pathBase, sha256: resolved.identity.sha256, schemaVersion: resolved.identity.schemaVersion };
  } else throw new Error("capabilities.chrome_vision.config must be a path string or inline mapping");
  engineConfig.capabilities.chrome_vision = durable;
  return { campaignPath: resolvedCampaignPath, engineConfig, campaignSource, resolvedCapabilities: { chrome_vision: resolved.config } };
}

if (process.argv[1] && path.resolve(process.argv[1]) === new URL(import.meta.url).pathname) {
  const campaignPath = process.argv[2];
  if (!campaignPath) throw new Error("Usage: campaign-preflight <campaign.yaml>");
  process.stdout.write(`${JSON.stringify(await preflightCampaign(campaignPath))}\n`);
}
