import { readFile } from "node:fs/promises";
import path from "node:path";

import {
  bindAgentInstructionReviewResult,
  digest,
  freeze,
  validateInstructionClosure,
  validateSubject,
} from "./contract.mjs";

function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new TypeError(`${label} must be non-empty text`);
  return value;
}

function lineCount(value) {
  return value.split("\n").length - (value.endsWith("\n") ? 1 : 0) || 1;
}

function loadedFragment({ id, kind, path: sourcePath, content, precedence, condition = null }) {
  text(content, `instruction material ${id} content`);
  return {
    id, kind, path: sourcePath, startLine: 1, endLine: lineCount(content),
    sha256: digest(Buffer.from(content)), precedence, inclusion: "loaded", condition, content,
  };
}

function conditionalFragment(value, index) {
  const label = `conditionalReferences[${index}]`;
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new TypeError(`${label} must be an object`);
  const fields = ["id", "kind", "path", "content", "precedence", "inclusion", "condition"];
  if (Object.keys(value).some((field) => !fields.includes(field)) || fields.some((field) => !(field in value))) {
    throw new TypeError(`${label} fields are invalid`);
  }
  const sourceDigest = digest(Buffer.from(text(value.content, `${label}.content`)));
  const fragment = {
    id: text(value.id, `${label}.id`), kind: value.kind,
    path: text(value.path, `${label}.path`), startLine: 1, endLine: lineCount(value.content),
    sha256: sourceDigest, precedence: value.precedence, inclusion: value.inclusion,
    condition: value.condition,
    content: value.inclusion === "loaded" ? value.content : null,
  };
  return fragment;
}

export async function projectAgentInstructionClosure({
  subject,
  roleProjection,
  conditionalReferences = [],
  sourceInventoryComplete,
  limitations = [],
} = {}) {
  validateSubject(subject);
  if (!roleProjection?.manifest || !roleProjection?.role || !Array.isArray(roleProjection.skills)) {
    throw new TypeError("agent instruction closure requires a runtime-manifest role projection");
  }
  if (typeof sourceInventoryComplete !== "boolean") throw new TypeError("sourceInventoryComplete must be boolean");
  if (!Array.isArray(conditionalReferences) || !Array.isArray(limitations)) {
    throw new TypeError("instruction closure references and limitations must be arrays");
  }
  if (!sourceInventoryComplete && limitations.length === 0) {
    throw new TypeError("incomplete instruction inventory requires limitations");
  }
  const fragments = [];
  const declaredReferences = [];
  if (roleProjection.role.developerInstructions) {
    fragments.push(loadedFragment({
      id: "manifest.developer-instructions", kind: "manifest_fragment",
      path: roleProjection.manifest.path ?? "runtime-manifest:inline",
      content: roleProjection.role.developerInstructions, precedence: "developer",
    }));
  }
  for (const [index, skill] of roleProjection.skills.entries()) {
    const content = await readFile(skill.path, "utf8");
    fragments.push(loadedFragment({
      id: `manifest.skill.${index}.${skill.name}`,
      kind: skill.path === roleProjection.roleContract.activatedPath ? "role_contract" : "skill_contract",
      path: skill.path, content, precedence: "skill",
    }));
    for (const match of content.matchAll(/\]\(([^)#]+\.md)(?:#[^)]+)?\)/g)) {
      if (/^[a-z][a-z0-9+.-]*:/i.test(match[1]) || path.isAbsolute(match[1])) continue;
      declaredReferences.push({
        sourceSkill: skill.name,
        path: path.resolve(path.dirname(skill.path), match[1]),
      });
    }
  }
  fragments.push(...conditionalReferences.map(conditionalFragment));
  if (sourceInventoryComplete) {
    const supplied = new Set(fragments
      .filter(({ kind }) => kind === "conditional_reference")
      .map(({ path: fragmentPath }) => path.resolve(fragmentPath)));
    const missing = declaredReferences.filter(({ path: referencePath }) => !supplied.has(referencePath));
    if (missing.length > 0) {
      throw new TypeError(`complete instruction inventory omits declared reference ${missing[0].path} from ${missing[0].sourceSkill}`);
    }
  }
  const body = {
    schemaVersion: 1,
    subject: structuredClone(subject),
    manifest: {
      manifestId: roleProjection.manifest.manifestId,
      path: roleProjection.manifest.path,
      sha256: roleProjection.manifest.sha256,
    },
    role: {
      roleId: roleProjection.roleId,
      runtimeEnvironmentRevision: roleProjection.role.runtimeEnvironmentRevision,
    },
    sourceInventoryComplete,
    fragments,
    omissions: fragments.filter(({ inclusion }) => inclusion === "omitted").map(({ id }) => id),
    limitations: [...limitations],
  };
  const closure = freeze({ ...body, closureRevision: digest(body) });
  validateInstructionClosure(closure);
  return closure;
}

export async function renderAgentInstructionReviewDelivery({ reviewerRoleProjection, closure }) {
  validateInstructionClosure(closure);
  const specialist = reviewerRoleProjection?.skills?.find(({ name }) => name === "agent-instruction-review");
  if (!specialist) throw new TypeError("implementation reviewer omits agent-instruction-review specialist");
  if (reviewerRoleProjection.role.threadOptions.sandbox !== "read-only"
      || reviewerRoleProjection.role.effects.length !== 0
      || (specialist.effects?.length ?? 0) !== 0) {
    throw new TypeError("agent instruction reviewer exceeds the read-only effect ceiling");
  }
  const skill = await readFile(specialist.path, "utf8");
  const findingContractPath = path.join(path.dirname(specialist.path), "references", "finding-contract.md");
  const findingContract = await readFile(findingContractPath, "utf8");
  const payload = {
    schema_version: 1,
    perspective: "agent-instruction-review",
    specialist_contract: { path: specialist.path, sha256: digest(Buffer.from(skill)), content: skill },
    finding_contract: { path: findingContractPath, sha256: digest(Buffer.from(findingContract)), content: findingContract },
    effective_instruction_closure: closure,
  };
  return freeze({
    roleInstructions: `${reviewerRoleProjection.role.developerInstructions.trim()}\n\nWORK_ENGINE_AGENT_INSTRUCTION_REVIEW_V1\n${JSON.stringify(payload)}`,
    deliveryRevision: digest(payload),
  });
}

export function createAgentInstructionReviewService() {
  return freeze({
    projectClosure: projectAgentInstructionClosure,
    renderDelivery: renderAgentInstructionReviewDelivery,
    admit({ result, closure }) { return bindAgentInstructionReviewResult(result, closure); },
  });
}
