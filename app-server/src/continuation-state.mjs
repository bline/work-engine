import { createHash } from "node:crypto";

export const CONTINUATION_STATE_SCHEMA_VERSION = 1;
export const CONTINUATION_STATE_TYPE = "work-engine.continuation-state";

const DIGEST = /^[a-f0-9]{64}$/;
const INTERACTION_STATUSES = new Set([
  "open", "resolved", "closed_but_active", "superseded", "historical", "ambiguous",
]);
const LOADING_DISPOSITIONS = new Set([
  "compiled_consequence", "escalate", "exact", "omit_from_working_context", "reference_only",
]);

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

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

function json(value, label) {
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value)) throw new TypeError(`${label} numbers must be safe integers`);
    return value;
  }
  if (Array.isArray(value)) return value.map((item, index) => json(item, `${label}[${index}]`));
  record(value, label);
  return Object.fromEntries(Object.keys(value).sort().map((key) => [key, json(value[key], `${label}.${key}`)]));
}

function reference(value, label) {
  record(value, label);
  rejectUnknown(value, ["reference", "sha256"], label);
  text(value.sha256, `${label} sha256`);
  if (!DIGEST.test(value.sha256)) throw new TypeError(`${label} sha256 must be lowercase SHA-256`);
  return { reference: text(value.reference, `${label} reference`), sha256: value.sha256 };
}

function semanticRecord(value, index, label) {
  const itemLabel = `${label}[${index}]`;
  record(value, itemLabel);
  rejectUnknown(value, ["id", "statement", "sourceRefs"], itemLabel);
  return {
    id: text(value.id, `${itemLabel} id`),
    statement: text(value.statement, `${itemLabel} statement`),
    sourceRefs: (value.sourceRefs ?? []).map((item, refIndex) => reference(item, `${itemLabel}.sourceRefs[${refIndex}]`)),
  };
}

function unique(items, label) {
  const ids = items.map((item) => item.id);
  if (new Set(ids).size !== ids.length) throw new TypeError(`${label} ids must be unique`);
  return items;
}

function normalizeInteraction(value, index) {
  const label = `humanInteractions[${index}]`;
  record(value, label);
  rejectUnknown(value, [
    "id", "kind", "status", "authorityEffect", "durableConsequenceRef",
    "sourceRef", "nextContextDisposition",
  ], label);
  if (!INTERACTION_STATUSES.has(value.status)) throw new TypeError(`${label} status is unsupported`);
  if (!LOADING_DISPOSITIONS.has(value.nextContextDisposition)) {
    throw new TypeError(`${label} nextContextDisposition is unsupported`);
  }
  return {
    id: text(value.id, `${label} id`),
    kind: text(value.kind, `${label} kind`),
    status: value.status,
    authorityEffect: value.authorityEffect == null ? null : text(value.authorityEffect, `${label} authorityEffect`),
    durableConsequenceRef: value.durableConsequenceRef == null ? null : reference(value.durableConsequenceRef, `${label} durableConsequenceRef`),
    sourceRef: reference(value.sourceRef, `${label} sourceRef`),
    nextContextDisposition: value.nextContextDisposition,
  };
}

function normalizeCandidate(candidate) {
  record(candidate, "continuation state");
  rejectUnknown(candidate, [
    "schemaVersion", "type", "subject", "compiledAt", "compiler", "objective",
    "workPosition", "completedConsequences", "activeCommitments", "decisions",
    "humanInteractions", "authorityDependencies", "unresolved", "governingEnvironment",
    "canonicalReferences", "authorizedNextAction", "roleState", "uncertainty",
  ], "continuation state");
  if (candidate.schemaVersion !== 1 || candidate.type !== CONTINUATION_STATE_TYPE) {
    throw new TypeError("unsupported continuation state schema or type");
  }
  const subject = record(candidate.subject, "continuation state subject");
  rejectUnknown(subject, ["logicalRoleInstanceId", "runtimeBindingRevision", "sourceRevision"], "continuation state subject");
  if (!Number.isSafeInteger(subject.runtimeBindingRevision) || subject.runtimeBindingRevision < 1) {
    throw new TypeError("continuation state runtime binding revision must be positive");
  }
  text(subject.sourceRevision, "continuation state source revision");
  if (!/^sha256:[a-f0-9]{64}$/.test(subject.sourceRevision)) {
    throw new TypeError("continuation state source revision must be sha256-bound");
  }
  const compiledAt = text(candidate.compiledAt, "continuation state compiledAt");
  if (Number.isNaN(Date.parse(compiledAt))) throw new TypeError("continuation state compiledAt must be an ISO timestamp");
  const compiler = record(candidate.compiler, "continuation state compiler");
  rejectUnknown(
    compiler,
    ["producer", "model", "version", "inferenceId"],
    "continuation state compiler",
  );
  const objective = record(candidate.objective, "continuation state objective");
  rejectUnknown(objective, ["statement", "authorityRef"], "continuation state objective");
  const workPosition = record(candidate.workPosition, "continuation state workPosition");
  rejectUnknown(workPosition, ["phase", "currentUnit"], "continuation state workPosition");
  const authorityDependencies = record(candidate.authorityDependencies, "authority dependencies");
  rejectUnknown(authorityDependencies, ["canonicalRecords", "revalidationRequired"], "authority dependencies");
  const governing = record(candidate.governingEnvironment, "governing environment");
  rejectUnknown(governing, ["roleContract", "instructionsToReload", "activatedSkills"], "governing environment");
  const action = record(candidate.authorizedNextAction, "authorized next action");
  rejectUnknown(action, ["kind", "objective", "authorityRef"], "authorized next action");
  const roleState = record(candidate.roleState, "role state");
  rejectUnknown(roleState, ["schema", "value"], "role state");
  return {
    schemaVersion: CONTINUATION_STATE_SCHEMA_VERSION,
    type: CONTINUATION_STATE_TYPE,
    subject: {
      logicalRoleInstanceId: text(subject.logicalRoleInstanceId, "logical role instance id"),
      runtimeBindingRevision: subject.runtimeBindingRevision,
      sourceRevision: subject.sourceRevision,
    },
    compiledAt,
    compiler: {
      producer: text(compiler.producer, "compiler producer"),
      model: compiler.model == null ? null : text(compiler.model, "compiler model"),
      version: text(compiler.version, "compiler version"),
      inferenceId: text(compiler.inferenceId, "compiler inference id"),
    },
    objective: {
      statement: text(objective.statement, "objective statement"),
      authorityRef: reference(objective.authorityRef, "objective authorityRef"),
    },
    workPosition: {
      phase: text(workPosition.phase, "work position phase"),
      currentUnit: text(workPosition.currentUnit, "work position currentUnit"),
    },
    completedConsequences: unique((candidate.completedConsequences ?? []).map((item, index) => semanticRecord(item, index, "completedConsequences")), "completedConsequences"),
    activeCommitments: unique((candidate.activeCommitments ?? []).map((item, index) => semanticRecord(item, index, "activeCommitments")), "activeCommitments"),
    decisions: unique((candidate.decisions ?? []).map((item, index) => semanticRecord(item, index, "decisions")), "decisions"),
    humanInteractions: unique((candidate.humanInteractions ?? []).map(normalizeInteraction), "humanInteractions"),
    authorityDependencies: {
      canonicalRecords: (authorityDependencies.canonicalRecords ?? []).map((item, index) => reference(item, `authorityDependencies.canonicalRecords[${index}]`)),
      revalidationRequired: (authorityDependencies.revalidationRequired ?? []).map((item, index) => text(item, `authorityDependencies.revalidationRequired[${index}]`)),
    },
    unresolved: unique((candidate.unresolved ?? []).map((item, index) => semanticRecord(item, index, "unresolved")), "unresolved"),
    governingEnvironment: {
      roleContract: reference(governing.roleContract, "governingEnvironment.roleContract"),
      instructionsToReload: (governing.instructionsToReload ?? []).map((item, index) => reference(item, `governingEnvironment.instructionsToReload[${index}]`)),
      activatedSkills: (governing.activatedSkills ?? []).map((item, index) => reference(item, `governingEnvironment.activatedSkills[${index}]`)),
    },
    canonicalReferences: (candidate.canonicalReferences ?? []).map((item, index) => reference(item, `canonicalReferences[${index}]`)),
    authorizedNextAction: {
      kind: text(action.kind, "authorized next action kind"),
      objective: text(action.objective, "authorized next action objective"),
      authorityRef: reference(action.authorityRef, "authorized next action authorityRef"),
    },
    roleState: { schema: text(roleState.schema, "role state schema"), value: json(roleState.value, "role state value") },
    uncertainty: unique((candidate.uncertainty ?? []).map((item, index) => semanticRecord(item, index, "uncertainty")), "uncertainty"),
  };
}

export function validateContinuationState(candidate) {
  const normalized = normalizeCandidate(candidate);
  const candidateRevision = `sha256:${createHash("sha256").update(canonicalJson(normalized)).digest("hex")}`;
  return freeze({ ...normalized, candidateRevision });
}

export function verifyContinuationState(state) {
  try {
    record(state, "continuation state candidate");
    const { candidateRevision, ...candidate } = state;
    return validateContinuationState(candidate).candidateRevision === candidateRevision;
  } catch {
    return false;
  }
}
