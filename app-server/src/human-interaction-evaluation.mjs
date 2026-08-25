export const HUMAN_INTERACTION_CLOSURE_ASSESSMENTS = Object.freeze([
  "supported",
  "contradicted",
  "uncertain",
]);
export const HUMAN_INTERACTION_LOADING_ASSESSMENTS = Object.freeze([
  "supported",
  "insufficient",
  "uncertain",
]);

const CLOSURE_ASSESSMENTS = new Set(HUMAN_INTERACTION_CLOSURE_ASSESSMENTS);
const LOADING_ASSESSMENTS = new Set(HUMAN_INTERACTION_LOADING_ASSESSMENTS);

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

function rejectUnknown(value, allowed, label) {
  const unknown = Object.keys(value).filter((key) => !allowed.includes(key)).sort();
  if (unknown.length) throw new TypeError(`${label} contains unsupported fields: ${unknown.join(", ")}`);
}

function referenceKey(value) {
  return JSON.stringify([value.reference, value.sha256]);
}

export function validateHumanInteractionBoundary(candidate, projection) {
  record(candidate, "continuation candidate");
  record(projection?.observedContext, "observed context projection");
  if (!Array.isArray(candidate.humanInteractions)) {
    throw new TypeError("continuation candidate humanInteractions must be an array");
  }
  const humanSources = new Set(projection.observedContext.visibleItems
    .filter((item) => item.origin === "human")
    .map((item) => referenceKey(item.contentRef)));
  for (const [index, interaction] of candidate.humanInteractions.entries()) {
    const label = `humanInteractions[${index}]`;
    if (!humanSources.has(referenceKey(interaction.sourceRef))) {
      throw new TypeError(`${label}.sourceRef is not an observed human-authored source`);
    }
    if (interaction.nextContextDisposition === "compiled_consequence"
        && interaction.durableConsequenceRef === null) {
      throw new TypeError(`${label} compiled_consequence requires a durable consequence`);
    }
    if (interaction.status === "ambiguous"
        && !["exact", "escalate"].includes(interaction.nextContextDisposition)) {
      throw new TypeError(`${label} ambiguous meaning must remain exact or escalate`);
    }
    if (interaction.status === "open"
        && interaction.nextContextDisposition === "omit_from_working_context") {
      throw new TypeError(`${label} open meaning cannot be omitted from working context`);
    }
    if (interaction.status === "closed_but_active"
        && ["omit_from_working_context", "reference_only"].includes(
          interaction.nextContextDisposition,
        )) {
      throw new TypeError(`${label} active governing meaning must be loaded or escalate`);
    }
  }
}

export function normalizeHumanInteractionEvaluations(
  value,
  { candidate, authorizeReference },
) {
  if (!Array.isArray(value)) {
    throw new TypeError("semantic verifier interactionEvaluations must be an array");
  }
  if (typeof authorizeReference !== "function") {
    throw new TypeError("interaction evaluation requires a reference authorizer");
  }
  const byId = new Map();
  for (const [index, evaluation] of value.entries()) {
    const label = `semantic verifier interactionEvaluations[${index}]`;
    record(evaluation, label);
    rejectUnknown(evaluation, [
      "interactionId", "sourceRef", "status", "nextContextDisposition",
      "closure", "loading", "rationale", "sourceRefs",
    ], label);
    const interactionId = text(evaluation.interactionId, `${label} interactionId`);
    if (byId.has(interactionId)) {
      throw new TypeError(`semantic verifier duplicates interaction evaluation ${interactionId}`);
    }
    const interaction = candidate.humanInteractions.find((item) => item.id === interactionId);
    if (!interaction) throw new TypeError(`${label} does not match a candidate interaction`);
    const sourceRef = authorizeReference(evaluation.sourceRef, `${label}.sourceRef`);
    if (referenceKey(sourceRef) !== referenceKey(interaction.sourceRef)) {
      throw new TypeError(`${label}.sourceRef does not match the candidate interaction`);
    }
    if (evaluation.status !== interaction.status) {
      throw new TypeError(`${label}.status does not match the candidate interaction`);
    }
    if (evaluation.nextContextDisposition !== interaction.nextContextDisposition) {
      throw new TypeError(
        `${label}.nextContextDisposition does not match the candidate interaction`,
      );
    }
    if (!CLOSURE_ASSESSMENTS.has(evaluation.closure)) {
      throw new TypeError(`${label}.closure is unsupported`);
    }
    if (!LOADING_ASSESSMENTS.has(evaluation.loading)) {
      throw new TypeError(`${label}.loading is unsupported`);
    }
    const sourceRefs = (evaluation.sourceRefs ?? []).map((sourceReference, refIndex) =>
      authorizeReference(sourceReference, `${label}.sourceRefs[${refIndex}]`)
    );
    if (sourceRefs.length === 0) {
      throw new TypeError(`${label} must cite at least one source reference`);
    }
    byId.set(interactionId, {
      interactionId,
      sourceRef,
      status: interaction.status,
      nextContextDisposition: interaction.nextContextDisposition,
      closure: evaluation.closure,
      loading: evaluation.loading,
      rationale: text(evaluation.rationale, `${label}.rationale`),
      sourceRefs,
    });
  }
  if (byId.size !== candidate.humanInteractions.length) {
    throw new TypeError(
      "semantic verifier must evaluate every candidate human interaction exactly once",
    );
  }
  return candidate.humanInteractions.map((interaction) => byId.get(interaction.id));
}

export function deriveHumanInteractionCheckStatus(interactions, evaluations) {
  if (interactions.length !== evaluations.length) {
    throw new TypeError("human interaction check requires complete evaluations");
  }
  if (evaluations.some((evaluation) =>
    evaluation.closure === "contradicted" || evaluation.loading === "insufficient"
  )) return "fail";
  if (evaluations.some((evaluation, index) =>
    evaluation.closure === "uncertain"
    || evaluation.loading === "uncertain"
    || interactions[index].status === "ambiguous"
    || interactions[index].nextContextDisposition === "escalate"
  )) return "uncertain";
  return "pass";
}
