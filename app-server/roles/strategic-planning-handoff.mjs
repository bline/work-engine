import { isDeepStrictEqual } from "node:util";

import { parseDocument } from "yaml";

const CONTINUITY_STATES = new Set(["initialized", "retained", "reconstructed"]);
const VERDICTS = new Set([
  "continue",
  "revise",
  "pause",
  "reorder",
  "split_campaign",
  "stop_campaign",
]);
const CAMPAIGN_DISPOSITIONS = new Set([
  "continue_current",
  "amend_current",
  "start_new",
  "none",
]);

export class StrategicPlanningHandoffError extends Error {
  constructor(message) {
    super(message);
    this.name = "StrategicPlanningHandoffError";
  }
}

function fail(message) {
  throw new StrategicPlanningHandoffError(message);
}

function requireRecord(value, label) {
  if (!value || Array.isArray(value) || typeof value !== "object") {
    fail(`${label} must be an object`);
  }
  return value;
}

function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    fail(`${label} must be a non-empty string`);
  }
  return value;
}

function requireNullableText(value, label) {
  if (value !== null) requireText(value, label);
}

function requireEnum(value, allowed, label) {
  if (!allowed.has(value)) {
    fail(`${label} is not a supported value`);
  }
  return value;
}

function requireArray(value, label) {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  return value;
}

function rejectUnknownKeys(value, allowed, label) {
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) fail(`${label} contains unsupported field ${key}`);
  }
}

function normalizeCampaignTerminal(value, label, inputStyle) {
  const terminal = requireRecord(value, label);
  const fields = inputStyle
    ? { runId: "run_id", sliceNumber: "slice_number", status: "status" }
    : { run_id: "run_id", slice_number: "slice_number", status: "status" };
  rejectUnknownKeys(terminal, new Set(Object.keys(fields)), label);
  const runId = requireText(terminal[inputStyle ? "runId" : "run_id"], `${label}.run_id`);
  const sliceNumber = terminal[inputStyle ? "sliceNumber" : "slice_number"];
  if (!Number.isInteger(sliceNumber) || sliceNumber < 0) {
    fail(`${label}.slice_number must be a non-negative integer`);
  }
  const status = requireText(terminal.status, `${label}.status`);
  return { run_id: runId, slice_number: sliceNumber, status };
}

export function normalizePlanningEvidenceCutoff(value) {
  const cutoff = requireRecord(value, "evidenceCutoff");
  rejectUnknownKeys(
    cutoff,
    new Set(["roadmapRevision", "repositoryRevision", "campaignTerminals"]),
    "evidenceCutoff",
  );
  const campaignTerminals = cutoff.campaignTerminals == null
    ? []
    : requireArray(cutoff.campaignTerminals, "evidenceCutoff.campaignTerminals")
      .map((terminal, index) =>
        normalizeCampaignTerminal(terminal, `evidenceCutoff.campaignTerminals[${index}]`, true),
      );
  return {
    roadmap_revision: requireText(cutoff.roadmapRevision, "evidenceCutoff.roadmapRevision"),
    repository_revision: requireText(
      cutoff.repositoryRevision,
      "evidenceCutoff.repositoryRevision",
    ),
    ...(campaignTerminals.length > 0 ? { campaign_terminals: campaignTerminals } : {}),
  };
}

function parseEvidenceCutoff(value) {
  const cutoff = requireRecord(value, "planning handoff evidence_cutoff");
  rejectUnknownKeys(
    cutoff,
    new Set(["roadmap_revision", "repository_revision", "campaign_terminals"]),
    "planning handoff evidence_cutoff",
  );
  const campaignTerminals = cutoff.campaign_terminals == null
    ? []
    : requireArray(
      cutoff.campaign_terminals,
      "planning handoff evidence_cutoff.campaign_terminals",
    ).map((terminal, index) =>
      normalizeCampaignTerminal(
        terminal,
        `planning handoff evidence_cutoff.campaign_terminals[${index}]`,
        false,
      ),
    );
  return {
    roadmap_revision: requireText(
      cutoff.roadmap_revision,
      "planning handoff evidence_cutoff.roadmap_revision",
    ),
    repository_revision: requireText(
      cutoff.repository_revision,
      "planning handoff evidence_cutoff.repository_revision",
    ),
    ...(campaignTerminals.length > 0 ? { campaign_terminals: campaignTerminals } : {}),
  };
}

function validateOptionalCollections(value, fields, label) {
  if (value == null) return;
  const section = requireRecord(value, label);
  rejectUnknownKeys(section, new Set(fields), label);
  for (const field of fields) {
    if (section[field] != null) requireArray(section[field], `${label}.${field}`);
  }
}

function unwrapYaml(text) {
  requireText(text, "completed planner output");
  const trimmed = text.trim();
  if (!trimmed.startsWith("```")) return trimmed;
  const match = /^```(?:yaml|yml)?[ \t]*\r?\n([\s\S]*?)\r?\n```$/.exec(trimmed);
  if (!match) fail("completed planner output must contain only one YAML document");
  return match[1];
}

function parseYaml(text) {
  const document = parseDocument(unwrapYaml(text), {
    prettyErrors: false,
    uniqueKeys: true,
  });
  if (document.errors.length > 0) {
    fail(`completed planner output is invalid YAML: ${document.errors[0].message}`);
  }
  try {
    return document.toJS({ maxAliasCount: 0 });
  } catch (error) {
    fail(`completed planner output cannot be decoded safely: ${error.message}`);
  }
}

function deepFreeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const nested of Object.values(value)) deepFreeze(nested);
  return Object.freeze(value);
}

export function parseStrategicPlanningHandoff(text, {
  strategicObjective,
  evidenceCutoff,
  continuity,
}) {
  requireText(strategicObjective, "strategic objective");
  requireEnum(continuity, CONTINUITY_STATES, "continuity");
  const expectedCutoff = normalizePlanningEvidenceCutoff(evidenceCutoff);
  const handoff = requireRecord(parseYaml(text), "planning handoff");
  rejectUnknownKeys(handoff, new Set([
    "schema_version",
    "strategic_objective",
    "evidence_cutoff",
    "continuity",
    "verdict",
    "current_rationale",
    "assumptions",
    "route_changes",
    "recommended_campaign",
    "open_uncertainties",
    "authority_required",
    "revisit_when",
  ]), "planning handoff");

  if (handoff.schema_version !== 1) fail("planning handoff schema_version must be 1");
  requireText(handoff.strategic_objective, "planning handoff strategic_objective");
  if (handoff.strategic_objective !== strategicObjective) {
    fail("planning handoff strategic_objective does not match the requested objective");
  }
  const actualCutoff = parseEvidenceCutoff(handoff.evidence_cutoff);
  if (!isDeepStrictEqual(actualCutoff, expectedCutoff)) {
    fail("planning handoff evidence_cutoff does not match the supplied cutoff");
  }
  requireEnum(handoff.continuity, CONTINUITY_STATES, "planning handoff continuity");
  if (handoff.continuity !== continuity) {
    fail("planning handoff continuity does not match the requested continuity");
  }
  requireEnum(handoff.verdict, VERDICTS, "planning handoff verdict");
  requireText(handoff.current_rationale, "planning handoff current_rationale");

  validateOptionalCollections(
    handoff.assumptions,
    ["confirmed", "changed", "invalidated"],
    "planning handoff assumptions",
  );
  validateOptionalCollections(
    handoff.route_changes,
    ["priorities", "dependencies", "newly_important", "deferred"],
    "planning handoff route_changes",
  );
  for (const field of ["open_uncertainties", "authority_required", "revisit_when"]) {
    if (handoff[field] != null) requireArray(handoff[field], `planning handoff ${field}`);
  }

  const campaign = requireRecord(
    handoff.recommended_campaign,
    "planning handoff recommended_campaign",
  );
  rejectUnknownKeys(
    campaign,
    new Set(["disposition", "objective", "work_source", "reason"]),
    "planning handoff recommended_campaign",
  );
  requireEnum(
    campaign.disposition,
    CAMPAIGN_DISPOSITIONS,
    "planning handoff recommended_campaign.disposition",
  );
  requireNullableText(campaign.objective, "planning handoff recommended_campaign.objective");
  requireNullableText(
    campaign.work_source,
    "planning handoff recommended_campaign.work_source",
  );
  requireText(campaign.reason, "planning handoff recommended_campaign.reason");

  return deepFreeze(handoff);
}
