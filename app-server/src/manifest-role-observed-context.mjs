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

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonical(value[key])}`
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}

function runtimeEnvironmentMaterial(delivery, roleProjection, contract) {
  const role = roleProjection.role;
  const satisfaction = delivery.runtimeSatisfaction ?? null;
  if (role.compiledSkillSha256 === null) {
    if (satisfaction === null) return null;
    throw new TypeError("uncompiled manifest role cannot claim runtime satisfaction");
  }
  record(satisfaction, "manifest role runtime satisfaction");
  const unsigned = { ...satisfaction };
  delete unsigned.sha256;
  if (satisfaction.schema_version !== 1
      || satisfaction.role_id !== roleProjection.roleId
      || satisfaction.compiled_skill_sha256 !== role.compiledSkillSha256
      || satisfaction.manifest_sha256 !== roleProjection.manifest.sha256
      || satisfaction.runtime_environment_revision !== role.runtimeEnvironmentRevision
      || satisfaction.sha256 !== digest(canonical(unsigned))) {
    throw new TypeError("manifest role runtime satisfaction does not match its role environment");
  }
  const satisfactionSha256 = satisfaction.sha256;
  const contractPath = contract.activatedSkill.path;
  const content = canonical({
    schema_version: 1,
    logical_role_instance_id: role.logicalRoleInstanceId,
    runtime_environment_revision: role.runtimeEnvironmentRevision,
    compiled_skill_sha256: role.compiledSkillSha256,
    runtime_satisfaction_sha256: satisfactionSha256,
    developer_instructions: role.developerInstructions,
    role_contract: {
      path: contractPath,
      sha256: contract.sourceMaterial.contentRef.sha256,
    },
  });
  const reference = [
    "work-engine.runtime-role-environment", "v1",
    encodeURIComponent(role.logicalRoleInstanceId),
    role.runtimeEnvironmentRevision,
    satisfactionSha256,
    encodeURIComponent(contractPath),
  ].join("/");
  return {
    contentRef: { kind: "runtime-role-environment", reference, sha256: digest(content) },
    content,
  };
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
  const role = record(roleProjection.role, "manifest role runtime projection");
  const deliveryLogicalRoleInstanceId = text(
    delivery.logicalRoleInstanceId,
    "manifest role logical identity",
  );
  if (deliveryLogicalRoleInstanceId !== text(
    role.logicalRoleInstanceId,
    "manifest role projected logical identity",
  )) {
    throw new TypeError("manifest role delivery does not match projected logical identity");
  }
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
  const activatedContractPath = roleProjection.roleContract?.activatedPath == null
    ? contractPath
    : text(
      roleProjection.roleContract.activatedPath,
      "manifest role activated contract path",
    );
  const contract = skillMaterials.find(({ activatedSkill }) =>
    activatedSkill.path === activatedContractPath
  );
  if (!contract) throw new TypeError("manifest role contract is not an activated skill");
  const environmentMaterial = runtimeEnvironmentMaterial(delivery, roleProjection, contract);
  record(expectedNextWork, "manifest role expected next work");
  rejectUnknown(expectedNextWork, ["reference", "content"], "manifest role expected next work");
  const nextContent = text(expectedNextWork.content, "manifest role expected next work content");
  const nextRef = {
    kind: "expected_next_work",
    reference: text(expectedNextWork.reference, "manifest role expected next work reference"),
    sha256: digest(nextContent),
  };
  const projection = projectObservedContext({
    logicalRoleInstanceId: deliveryLogicalRoleInstanceId,
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
      contentRef: environmentMaterial?.contentRef ?? contract.sourceMaterial.contentRef,
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
      ...(environmentMaterial === null ? [] : [environmentMaterial]),
      { contentRef: nextRef, content: nextContent },
    ]),
  });
}
