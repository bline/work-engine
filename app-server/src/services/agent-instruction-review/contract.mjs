import { createHash } from "node:crypto";

import {
  canonicalJson,
  digest as implementationReviewDigest,
  validateImplementationReviewResult,
} from "../implementation-review/contract.mjs";

const SHA256 = /^[a-f0-9]{64}$/;
const APPLICABILITY = new Set(["applicable", "omitted"]);
const SEVERITIES = new Set(["blocker", "high", "medium", "low", "info"]);
const CONFIDENCE = new Set(["high", "medium", "low"]);
const OUTCOMES = new Set(["retain", "restate", "split", "move", "demote", "remove"]);
const PRECEDENCE = new Set(["system", "developer", "role", "skill", "subject"]);
const INCLUSION = new Set(["loaded", "omitted"]);
const MATERIAL_KINDS = new Set([
  "manifest_fragment", "role_contract", "skill_contract", "governed_instruction",
  "causal_reason", "failure_mode", "conditional_reference",
]);

export class AgentInstructionReviewError extends Error {}

export const digest = (value) => createHash("sha256")
  .update(Buffer.isBuffer(value) ? value : canonicalJson(value))
  .digest("hex");

export function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new AgentInstructionReviewError(`${label} must be an object`);
  }
  return value;
}

function exact(value, fields, label) {
  record(value, label);
  const expected = new Set(fields);
  if (Object.keys(value).some((field) => !expected.has(field))
      || fields.some((field) => !(field in value))) {
    throw new AgentInstructionReviewError(`${label} fields are invalid`);
  }
}

function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new AgentInstructionReviewError(`${label} must be non-empty text`);
  }
  return value;
}

function sha256(value, label) {
  if (typeof value !== "string" || !SHA256.test(value)) {
    throw new AgentInstructionReviewError(`${label} must be lowercase SHA-256`);
  }
  return value;
}

function texts(value, label, { required = false } = {}) {
  if (!Array.isArray(value) || (required && value.length === 0)) {
    throw new AgentInstructionReviewError(`${label} must be ${required ? "a non-empty" : "an"} array`);
  }
  value.forEach((item, index) => text(item, `${label}[${index}]`));
  return value;
}

export function validateSubject(value, label = "agent instruction review subject") {
  exact(value, ["commit", "tree", "patchIdentity"], label);
  for (const field of ["commit", "tree", "patchIdentity"]) text(value[field], `${label}.${field}`);
  return value;
}

export function validateClosureFragment(value, label = "instruction closure fragment") {
  exact(value, [
    "id", "kind", "path", "startLine", "endLine", "sha256", "precedence",
    "inclusion", "condition", "content",
  ], label);
  for (const field of ["id", "path"]) text(value[field], `${label}.${field}`);
  if (!MATERIAL_KINDS.has(value.kind)) throw new AgentInstructionReviewError(`${label}.kind is invalid`);
  if (!PRECEDENCE.has(value.precedence)) throw new AgentInstructionReviewError(`${label}.precedence is invalid`);
  if (!INCLUSION.has(value.inclusion)) throw new AgentInstructionReviewError(`${label}.inclusion is invalid`);
  if (!Number.isSafeInteger(value.startLine) || value.startLine < 1
      || !Number.isSafeInteger(value.endLine) || value.endLine < value.startLine) {
    throw new AgentInstructionReviewError(`${label} line bounds are invalid`);
  }
  sha256(value.sha256, `${label}.sha256`);
  if (value.inclusion === "loaded") {
    text(value.content, `${label}.content`);
    if (digest(Buffer.from(value.content)) !== value.sha256) {
      throw new AgentInstructionReviewError(`${label} content digest differs`);
    }
  } else if (value.content !== null) {
    throw new AgentInstructionReviewError(`${label} omitted content must not be exposed`);
  }
  if (value.condition === null) {
    if (value.inclusion === "omitted") {
      throw new AgentInstructionReviewError(`${label} omission requires an accepted condition`);
    }
  } else {
    exact(value.condition, ["reference", "decision", "authority", "sha256"], `${label}.condition`);
    for (const field of ["reference", "decision", "authority"]) {
      text(value.condition[field], `${label}.condition.${field}`);
    }
    sha256(value.condition.sha256, `${label}.condition.sha256`);
    if (value.inclusion === "omitted" && value.condition.decision !== "accepted_safe_omission") {
      throw new AgentInstructionReviewError(`${label} omission is not authorized by a closed condition`);
    }
  }
  return value;
}

export function validateInstructionClosure(value) {
  exact(value, [
    "schemaVersion", "subject", "manifest", "role", "sourceInventoryComplete",
    "fragments", "omissions", "limitations", "closureRevision",
  ], "agent instruction closure");
  if (value.schemaVersion !== 1) throw new AgentInstructionReviewError("instruction closure schema version is invalid");
  validateSubject(value.subject);
  exact(value.manifest, ["manifestId", "path", "sha256"], "instruction closure manifest");
  text(value.manifest.manifestId, "instruction closure manifest.manifestId");
  if (value.manifest.path !== null) text(value.manifest.path, "instruction closure manifest.path");
  sha256(value.manifest.sha256, "instruction closure manifest.sha256");
  exact(value.role, ["roleId", "runtimeEnvironmentRevision"], "instruction closure role");
  text(value.role.roleId, "instruction closure role.roleId");
  sha256(value.role.runtimeEnvironmentRevision, "instruction closure role.runtimeEnvironmentRevision");
  if (typeof value.sourceInventoryComplete !== "boolean") {
    throw new AgentInstructionReviewError("instruction closure completeness flag is invalid");
  }
  if (!Array.isArray(value.fragments) || value.fragments.length === 0) {
    throw new AgentInstructionReviewError("instruction closure fragments must be non-empty");
  }
  value.fragments.forEach((item, index) => validateClosureFragment(item, `instruction closure fragments[${index}]`));
  const ids = value.fragments.map(({ id }) => id);
  if (new Set(ids).size !== ids.length) throw new AgentInstructionReviewError("instruction closure fragment IDs must be unique");
  texts(value.omissions, "instruction closure omissions");
  const expectedOmissions = value.fragments.filter(({ inclusion }) => inclusion === "omitted").map(({ id }) => id);
  if (canonicalJson(value.omissions) !== canonicalJson(expectedOmissions)) {
    throw new AgentInstructionReviewError("instruction closure omission inventory differs");
  }
  texts(value.limitations, "instruction closure limitations", { required: !value.sourceInventoryComplete });
  const unsigned = { ...value };
  delete unsigned.closureRevision;
  if (digest(unsigned) !== value.closureRevision) {
    throw new AgentInstructionReviewError("instruction closure revision differs");
  }
  return value;
}

function validateFindingDetail(value, label) {
  exact(value, [
    "id", "severity", "instruction", "placement", "protectedDistinction",
    "causalExposure", "exactRouteNecessary", "authoritySource", "consequence",
    "confidence", "limitations", "advisoryOutcome",
  ], label);
  text(value.id, `${label}.id`);
  if (!SEVERITIES.has(value.severity)) throw new AgentInstructionReviewError(`${label}.severity is invalid`);
  exact(value.instruction, ["fragmentId", "path", "startLine", "endLine"], `${label}.instruction`);
  for (const field of ["fragmentId", "path"]) text(value.instruction[field], `${label}.instruction.${field}`);
  if (!Number.isSafeInteger(value.instruction.startLine) || value.instruction.startLine < 1
      || !Number.isSafeInteger(value.instruction.endLine)
      || value.instruction.endLine < value.instruction.startLine) {
    throw new AgentInstructionReviewError(`${label}.instruction bounds are invalid`);
  }
  exact(value.placement, ["semanticOwner", "consumer", "audience", "scope", "precedence", "loadingReach"], `${label}.placement`);
  for (const field of ["semanticOwner", "consumer", "audience", "scope", "loadingReach"]) {
    text(value.placement[field], `${label}.placement.${field}`);
  }
  if (!PRECEDENCE.has(value.placement.precedence)) throw new AgentInstructionReviewError(`${label}.placement.precedence is invalid`);
  text(value.protectedDistinction, `${label}.protectedDistinction`);
  exact(value.causalExposure, ["reasonLoaded", "failureModeLoaded", "sourceTraceable", "reviewerReconstructed"], `${label}.causalExposure`);
  for (const field of Object.keys(value.causalExposure)) {
    if (typeof value.causalExposure[field] !== "boolean") throw new AgentInstructionReviewError(`${label}.causalExposure.${field} must be boolean`);
  }
  if (typeof value.exactRouteNecessary !== "boolean") throw new AgentInstructionReviewError(`${label}.exactRouteNecessary must be boolean`);
  for (const field of ["authoritySource", "consequence"]) text(value[field], `${label}.${field}`);
  if (!CONFIDENCE.has(value.confidence)) throw new AgentInstructionReviewError(`${label}.confidence is invalid`);
  texts(value.limitations, `${label}.limitations`);
  if (!OUTCOMES.has(value.advisoryOutcome)) throw new AgentInstructionReviewError(`${label}.advisoryOutcome is invalid`);
  return value;
}

export function validateAgentInstructionReviewResult(value) {
  exact(value, [
    "schemaVersion", "perspective", "subject", "closureRevision", "applicability",
    "applicabilityReason", "result", "findingDetails", "limitations",
  ], "agent instruction review result");
  if (value.schemaVersion !== 1 || value.perspective !== "agent-instruction-review") {
    throw new AgentInstructionReviewError("agent instruction review result identity is invalid");
  }
  validateSubject(value.subject);
  sha256(value.closureRevision, "agent instruction review result.closureRevision");
  if (!APPLICABILITY.has(value.applicability)) throw new AgentInstructionReviewError("agent instruction review applicability is invalid");
  text(value.applicabilityReason, "agent instruction review applicabilityReason");
  validateImplementationReviewResult(value.result);
  if (implementationReviewDigest(value.subject) !== implementationReviewDigest(value.result.subject)) {
    throw new AgentInstructionReviewError("nested implementation review result subject differs");
  }
  if (!Array.isArray(value.findingDetails)) throw new AgentInstructionReviewError("agent instruction findingDetails must be an array");
  value.findingDetails.forEach((item, index) => validateFindingDetail(item, `agent instruction findingDetails[${index}]`));
  const genericIds = value.result.findings.map(({ id }) => id);
  const detailIds = value.findingDetails.map(({ id }) => id);
  if (new Set(detailIds).size !== detailIds.length || canonicalJson(genericIds) !== canonicalJson(detailIds)) {
    throw new AgentInstructionReviewError("specialist finding details must exactly match generic finding lineage");
  }
  if (value.applicability === "omitted" && value.result.findings.length !== 0) {
    throw new AgentInstructionReviewError("omitted specialist review cannot report findings");
  }
  texts(value.limitations, "agent instruction review limitations", { required: true });
  return value;
}

export function bindAgentInstructionReviewResult(value, closure) {
  validateInstructionClosure(closure);
  validateAgentInstructionReviewResult(value);
  if (implementationReviewDigest(value.subject) !== implementationReviewDigest(closure.subject)
      || value.closureRevision !== closure.closureRevision) {
    throw new AgentInstructionReviewError("agent instruction review result differs from exact closure subject");
  }
  const fragments = new Map(closure.fragments.map((fragment) => [fragment.id, fragment]));
  for (const finding of value.findingDetails) {
    const fragment = fragments.get(finding.instruction.fragmentId);
    if (!fragment || finding.instruction.path !== fragment.path
        || finding.instruction.startLine < fragment.startLine
        || finding.instruction.endLine > fragment.endLine) {
      throw new AgentInstructionReviewError(`finding ${finding.id} instruction identity is outside the exact closure`);
    }
  }
  const admitted = structuredClone(value);
  return freeze({
    schemaVersion: 1,
    result: admitted,
    resultRevision: digest(admitted),
    implementationReviewResult: admitted.result,
    authority: freeze({
      mutationAuthorized: false,
      architectureChoiceAuthorized: false,
      proposalAcceptanceAuthorized: false,
      reviewerSelectionAuthorized: false,
      implementationAcceptanceAuthorized: false,
      selfCertificationAuthorized: false,
      humanAuthorityConferred: false,
      independenceClaimed: false,
    }),
  });
}
