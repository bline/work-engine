import {
  normalizePlanningEvidenceCutoff,
  parseStrategicPlanningHandoff,
} from "./strategic-planning-handoff.mjs";

const CONTINUITY_STATES = new Set(["initialized", "retained", "reconstructed"]);

function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
}

function requireRecord(value, label) {
  if (!value || Array.isArray(value) || typeof value !== "object") {
    throw new TypeError(`${label} must be an object`);
  }
}

function validateReferences(references) {
  if (!Array.isArray(references) || references.length === 0) {
    throw new TypeError("canonicalReferences must contain at least one durable reference");
  }
  return references.map((reference, index) => {
    requireRecord(reference, `canonicalReferences[${index}]`);
    for (const field of ["owner", "reference", "revision", "freshnessRule"]) {
      requireText(reference[field], `canonicalReferences[${index}].${field}`);
    }
    return {
      owner: reference.owner,
      reference: reference.reference,
      revision: reference.revision,
      freshnessRule: reference.freshnessRule,
      ...(reference.integritySha256 ? { integritySha256: reference.integritySha256 } : {}),
    };
  });
}

export class StrategicPlannerRuntime {
  constructor({ adapter, manifest }) {
    if (
      !adapter ||
      typeof adapter.deliverTurn !== "function" ||
      typeof adapter.waitForTurnCompletion !== "function"
    ) {
      throw new TypeError("StrategicPlannerRuntime requires an App Server adapter");
    }
    if (!manifest || typeof manifest.projectRole !== "function") {
      throw new TypeError("StrategicPlannerRuntime requires a projected runtime manifest");
    }
    this.adapter = adapter;
    this.manifest = manifest;
  }

  async requestReview({
    instanceId,
    clientUserMessageId,
    strategicObjective,
    evidenceCutoff,
    canonicalReferences,
    continuity,
    signal,
  }) {
    requireText(clientUserMessageId, "client user message id");
    requireText(strategicObjective, "strategic objective");
    requireRecord(evidenceCutoff, "evidenceCutoff");
    if (!CONTINUITY_STATES.has(continuity)) {
      throw new TypeError(
        "continuity must be initialized, retained, or reconstructed",
      );
    }
    const references = validateReferences(canonicalReferences);
    const normalizedEvidenceCutoff = normalizePlanningEvidenceCutoff(evidenceCutoff);
    const roleProjection = this.manifest.projectRole("strategic-planner", instanceId);
    const delivery = await this.adapter.deliverTurn({
      role: roleProjection.role,
      clientUserMessageId,
      skills: roleProjection.skills,
      requestContext: {
        "work-engine.strategic-objective": {
          kind: "application",
          value: strategicObjective,
        },
        "work-engine.evidence-cutoff": {
          kind: "application",
          value: JSON.stringify(normalizedEvidenceCutoff),
        },
        "work-engine.canonical-references": {
          kind: "application",
          value: JSON.stringify(references),
        },
        "work-engine.planner-continuity": {
          kind: "application",
          value: continuity,
        },
      },
      text:
        "Reconcile the supplied strategic objective with the referenced durable evidence. " +
        "Copy strategic_objective, evidence_cutoff, and continuity from request context " +
        "exactly; request-binding validation rejects any rephrasing of those fields. " +
        "Return one compact version-1 strategic planning handoff; do not mutate its owners.",
    });
    const completion = await this.adapter.waitForTurnCompletion({
      ...delivery,
      signal,
    });
    const handoff = parseStrategicPlanningHandoff(completion.outputText, {
      strategicObjective,
      evidenceCutoff,
      continuity,
    });
    return Object.freeze({ ...delivery, completion, handoff });
  }
}
