import { createHash, randomUUID } from "node:crypto";

import { parseStrategicPlanningHandoff } from "../../../roles/strategic-planning-handoff.mjs";

const CONTINUITY = new Set(["initialized", "retained", "reconstructed"]);

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function exact(value, required, optional, label) {
  const allowed = new Set([...required, ...optional]);
  const unsupported = Object.keys(value).find((field) => !allowed.has(field));
  if (unsupported) throw new TypeError(`${label} contains unsupported field ${unsupported}`);
  const missing = [...required].find((field) => !Object.hasOwn(value, field));
  if (missing) throw new TypeError(`${label} requires field ${missing}`);
  return value;
}

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

function digest(value) {
  return createHash("sha256").update(canonical(value)).digest("hex");
}

export function validateStrategicReconciliationRequest(value) {
  const request = exact(record(value, "strategic reconciliation request"), new Set([
    "instance_id", "client_user_message_id", "strategic_objective", "evidence_cutoff",
    "canonical_references", "continuity",
  ]), new Set(), "strategic reconciliation request");
  text(request.instance_id, "strategic reconciliation instance_id");
  text(request.client_user_message_id, "strategic reconciliation client_user_message_id");
  text(request.strategic_objective, "strategic reconciliation strategic_objective");
  const cutoff = exact(record(request.evidence_cutoff, "strategic reconciliation evidence_cutoff"),
    new Set(["roadmap_revision", "repository_revision"]), new Set(["campaign_terminals"]),
    "strategic reconciliation evidence_cutoff");
  text(cutoff.roadmap_revision, "strategic reconciliation roadmap_revision");
  text(cutoff.repository_revision, "strategic reconciliation repository_revision");
  if (cutoff.campaign_terminals !== undefined && !Array.isArray(cutoff.campaign_terminals)) {
    throw new TypeError("strategic reconciliation campaign_terminals must be an array");
  }
  for (const [index, terminal] of (cutoff.campaign_terminals ?? []).entries()) {
    exact(record(terminal, `strategic reconciliation campaign_terminals[${index}]`),
      new Set(["run_id", "slice_number", "status"]), new Set(),
      `strategic reconciliation campaign_terminals[${index}]`);
    text(terminal.run_id, `strategic reconciliation campaign_terminals[${index}].run_id`);
    if (!Number.isInteger(terminal.slice_number) || terminal.slice_number < 0) {
      throw new TypeError(`strategic reconciliation campaign_terminals[${index}].slice_number must be non-negative`);
    }
    text(terminal.status, `strategic reconciliation campaign_terminals[${index}].status`);
  }
  if (!Array.isArray(request.canonical_references) || request.canonical_references.length === 0) {
    throw new TypeError("strategic reconciliation canonical_references must be a non-empty array");
  }
  for (const [index, reference] of request.canonical_references.entries()) {
    exact(record(reference, `strategic reconciliation canonical_references[${index}]`),
      new Set(["owner", "reference", "revision", "freshness_rule"]),
      new Set(["integrity_sha256"]), `strategic reconciliation canonical_references[${index}]`);
    for (const field of ["owner", "reference", "revision", "freshness_rule"]) {
      text(reference[field], `strategic reconciliation canonical_references[${index}].${field}`);
    }
    if (reference.integrity_sha256 !== undefined
        && !/^[0-9a-f]{64}$/.test(reference.integrity_sha256)) {
      throw new TypeError(`strategic reconciliation canonical_references[${index}].integrity_sha256 must be sha256`);
    }
  }
  if (!CONTINUITY.has(request.continuity)) {
    throw new TypeError("strategic reconciliation continuity is unsupported");
  }
  return structuredClone(request);
}

export function strategicPlannerRequest(value) {
  const request = validateStrategicReconciliationRequest(value);
  return {
    instanceId: request.instance_id,
    clientUserMessageId: request.client_user_message_id,
    strategicObjective: request.strategic_objective,
    evidenceCutoff: {
      roadmapRevision: request.evidence_cutoff.roadmap_revision,
      repositoryRevision: request.evidence_cutoff.repository_revision,
      ...(request.evidence_cutoff.campaign_terminals === undefined ? {} : {
        campaignTerminals: request.evidence_cutoff.campaign_terminals.map((terminal) => ({
          runId: terminal.run_id, sliceNumber: terminal.slice_number, status: terminal.status,
        })),
      }),
    },
    canonicalReferences: request.canonical_references.map((reference) => ({
      owner: reference.owner,
      reference: reference.reference,
      revision: reference.revision,
      freshnessRule: reference.freshness_rule,
      ...(reference.integrity_sha256 === undefined ? {} : {
        integritySha256: reference.integrity_sha256,
      }),
    })),
    continuity: request.continuity,
  };
}

export function createStrategicReconciliationHost() {
  const admissions = new Map();
  let closed = false;
  return Object.freeze({
    validateInput(value) {
      const input = exact(record(value, "strategic reconciliation host input"),
        new Set(["phase"]), new Set(["request", "admission", "output_text"]),
        "strategic reconciliation host input");
      if (input.phase === "admit") {
        exact(input, new Set(["phase", "request"]), new Set(), "strategic reconciliation admit input");
        return { phase: "admit", request: validateStrategicReconciliationRequest(input.request) };
      }
      if (input.phase === "complete") {
        exact(input, new Set(["phase", "admission", "output_text"]), new Set(),
          "strategic reconciliation complete input");
        record(input.admission, "strategic reconciliation admission");
        text(input.admission.admission_id, "strategic reconciliation admission_id");
        text(input.admission.generation_id, "strategic reconciliation admission generation_id");
        text(input.admission.request_digest, "strategic reconciliation admission request_digest");
        text(input.output_text, "strategic reconciliation output_text");
        return structuredClone(input);
      }
      throw new TypeError("strategic reconciliation host phase is unsupported");
    },
    async handle({ generationId, input }) {
      if (closed) throw new Error("strategic reconciliation host is closed");
      if (input.phase === "admit") {
        const admission = Object.freeze({
          admission_id: randomUUID(), generation_id: generationId,
          request_digest: digest(input.request),
        });
        admissions.set(admission.admission_id, { admission, request: input.request });
        return { status: "admitted", admission };
      }
      const retained = admissions.get(input.admission.admission_id);
      admissions.delete(input.admission.admission_id);
      if (!retained || canonical(retained.admission) !== canonical(input.admission)) {
        throw new Error("strategic reconciliation admission is stale or unavailable");
      }
      if (retained.admission.generation_id !== generationId) {
        throw new Error("strategic reconciliation admission belongs to another executable generation");
      }
      const planner = strategicPlannerRequest(retained.request);
      const handoff = parseStrategicPlanningHandoff(input.output_text, {
        strategicObjective: planner.strategicObjective,
        evidenceCutoff: planner.evidenceCutoff,
        continuity: planner.continuity,
      });
      return { status: "completed", admission: retained.admission, handoff };
    },
    validateOutput(value) { return structuredClone(record(value, "strategic reconciliation host result")); },
    close() { closed = true; admissions.clear(); },
  });
}
