#!/usr/bin/env node
import fs from "node:fs/promises";
import path from "node:path";
import { parse } from "yaml";
import { loadChromeVisionConfig, resolveChromeVisionConfig } from "../../chrome-vision/src/config.mjs";

const allowed = new Set(["version","objective","work_source","builder","validation","metrics","limits","approval","notifications","stop_on","capabilities"]);

export async function preflightCampaign(campaignPath, { cwd = process.cwd() } = {}) {
  const resolvedCampaignPath = path.resolve(cwd, campaignPath);
  let campaign;
  try { campaign = parse(await fs.readFile(resolvedCampaignPath, "utf8")); }
  catch (error) { throw new Error(`Cannot read campaign configuration ${resolvedCampaignPath}: ${error.message}`); }
  if (!campaign || typeof campaign !== "object" || Array.isArray(campaign)) throw new Error("Campaign configuration must be a mapping");
  if (campaign.kind === "comparative_repository_analysis") {
    throw new Error(`Comparative repository analysis definitions are not slice-supervisor campaigns. Run: python3 work-engine/skills/comparative-repository-analysis/scripts/comparison_artifacts.py prepare-run --config ${campaignPath} --output comparison-contract.json`);
  }
  const unknown = Object.keys(campaign).filter(key => !allowed.has(key));
  if (unknown.length) throw new Error(`Unknown campaign fields: ${unknown.sort().join(", ")}`);
  if (campaign.version !== 2) throw new Error("Campaign configuration version 2 is required for capabilities");
  if (campaign.capabilities !== undefined && (!campaign.capabilities || typeof campaign.capabilities !== "object" || Array.isArray(campaign.capabilities))) throw new Error("capabilities must be a mapping");
  const unknownCapabilities = Object.keys(campaign.capabilities || {}).filter(key => key !== "chrome_vision");
  if (unknownCapabilities.length) throw new Error(`Unknown capabilities: ${unknownCapabilities.sort().join(", ")}`);
  const declaration = campaign.capabilities?.chrome_vision;
  if (!declaration) return { campaignPath: resolvedCampaignPath, engineConfig: campaign, resolvedCapabilities: {} };
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
  const engineConfig = structuredClone(campaign);
  engineConfig.capabilities.chrome_vision = durable;
  return { campaignPath: resolvedCampaignPath, engineConfig, resolvedCapabilities: { chrome_vision: resolved.config } };
}

if (process.argv[1] && path.resolve(process.argv[1]) === new URL(import.meta.url).pathname) {
  const campaignPath = process.argv[2];
  if (!campaignPath) throw new Error("Usage: campaign-preflight <campaign.yaml>");
  process.stdout.write(`${JSON.stringify(await preflightCampaign(campaignPath))}\n`);
}
