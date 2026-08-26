import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

import { projectObservedContext } from "./observed-context-projection.mjs";

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
  const unknown = Object.keys(value).filter((field) => !allowed.includes(field)).sort();
  if (unknown.length > 0) throw new TypeError(`${label} contains unsupported fields: ${unknown.join(", ")}`);
}

function digest(content) {
  return createHash("sha256").update(content, "utf8").digest("hex");
}

function material(value, index) {
  record(value, `visible material ${index}`);
  rejectUnknown(value, [
    "identity", "origin", "trustClass", "instructionApplicability",
    "contentRef", "content", "producer",
  ], `visible material ${index}`);
  const content = text(value.content, `visible material ${index} content`);
  const contentRef = record(value.contentRef, `visible material ${index} contentRef`);
  rejectUnknown(contentRef, ["kind", "reference"], `visible material ${index} contentRef`);
  const projected = {
    identity: text(value.identity, `visible material ${index} identity`),
    origin: text(value.origin, `visible material ${index} origin`),
    trustClass: text(value.trustClass, `visible material ${index} trustClass`),
    instructionApplicability: text(
      value.instructionApplicability,
      `visible material ${index} instructionApplicability`,
    ),
    contentRef: {
      kind: text(contentRef.kind, `visible material ${index} contentRef kind`),
      reference: text(contentRef.reference, `visible material ${index} contentRef reference`),
      sha256: digest(content),
    },
    ...(value.producer == null ? {} : {
      producer: text(value.producer, `visible material ${index} producer`),
    }),
  };
  return { projected, sourceMaterial: { contentRef: projected.contentRef, content } };
}

export async function projectManifestRoleObservedContext({
  delivery,
  lifecycleSnapshot,
  visibleMaterials,
  expectedNextWork,
  sourceInventoryCompleteness,
  omissions = [],
  signing,
}) {
  record(delivery, "manifest role delivery");
  record(delivery.binding, "manifest role delivery binding");
  const roleProjection = record(delivery.roleProjection, "manifest role projection");
  record(roleProjection.role, "manifest role runtime projection");
  if (!Array.isArray(roleProjection.skills) || roleProjection.skills.length === 0) {
    throw new TypeError("manifest role projection requires exact skill inputs");
  }
  if (!Array.isArray(visibleMaterials)) {
    throw new TypeError("manifest role visible materials must be an array");
  }
  const visible = visibleMaterials.map(material);
  const skillMaterials = await Promise.all(roleProjection.skills.map(async (skill, index) => {
    const name = text(skill.name, `manifest role skill ${index} name`);
    const skillPath = text(skill.path, `manifest role skill ${index} path`);
    const content = await readFile(skillPath, "utf8");
    const contentRef = { kind: "skill", reference: skillPath, sha256: digest(content) };
    return {
      activatedSkill: { name, path: skillPath, sha256: contentRef.sha256 },
      sourceMaterial: { contentRef, content },
    };
  }));
  const contractPath = text(
    roleProjection.roleContract?.path,
    "manifest role contract path",
  );
  const contract = skillMaterials.find(({ activatedSkill }) =>
    activatedSkill.path === contractPath
  );
  if (!contract) throw new TypeError("manifest role contract is not an activated skill");
  record(expectedNextWork, "manifest role expected next work");
  rejectUnknown(expectedNextWork, ["reference", "content"], "manifest role expected next work");
  const nextContent = text(expectedNextWork.content, "manifest role expected next work content");
  const nextRef = {
    kind: "expected_next_work",
    reference: text(expectedNextWork.reference, "manifest role expected next work reference"),
    sha256: digest(nextContent),
  };
  const projection = projectObservedContext({
    logicalRoleInstanceId: text(delivery.logicalRoleInstanceId, "manifest role logical identity"),
    runtimeBinding: {
      threadId: text(delivery.threadId, "manifest role thread id"),
      bindingRevision: delivery.binding.bindingRevision,
      environmentRevision: text(
        roleProjection.role.runtimeEnvironmentRevision,
        "manifest role environment revision",
      ),
    },
    lastCompletedTurnId: text(delivery.turnId, "manifest role completed turn id"),
    visibleItems: visible.map(({ projected }) => projected),
    governingSources: [{
      identity: "role-contract",
      owner: contractPath,
      contentRef: contract.sourceMaterial.contentRef,
    }],
    activatedSkills: skillMaterials.map(({ activatedSkill }) => activatedSkill),
    lifecycleSnapshot,
    expectedNextWork: { reference: nextRef.reference, sha256: nextRef.sha256 },
    sourceInventoryCompleteness,
    omissions,
  }, signing);
  return Object.freeze({
    projection,
    sourceMaterials: Object.freeze([
      ...visible.map(({ sourceMaterial }) => sourceMaterial),
      ...skillMaterials.map(({ sourceMaterial }) => sourceMaterial),
      { contentRef: nextRef, content: nextContent },
    ]),
  });
}
