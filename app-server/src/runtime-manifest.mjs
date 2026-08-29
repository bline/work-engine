import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";

const TOP_LEVEL_FIELDS = new Set(["schema_version", "manifest_id", "roles"]);
const ROLE_FIELDS = new Set(["contract", "compiled_environment", "compiled_skill_sha256", "developer_instructions", "thread_options", "skills", "capabilities", "effects", "continuity"]);
const COMPILED_ENVIRONMENT_FIELDS = new Set(["structure", "interface"]);
const SKILL_FIELDS = new Set(["name", "path", "compiled_environment", "compiled_skill_sha256", "capabilities", "effects"]);
const IDENTIFIER_SEGMENT = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;
const APPROVAL_POLICIES = new Set(["untrusted", "on-request", "never"]);
const SANDBOX_MODES = new Set(["read-only", "workspace-write", "danger-full-access"]);
const PERSONALITIES = new Set(["none", "friendly", "pragmatic"]);
const CONTINUITY_KINDS = new Set(["ephemeral", "retained"]);
const THREAD_OPTION_FIELDS = new Map([
  ["cwd", "cwd"],
  ["approval_policy", "approvalPolicy"],
  ["sandbox", "sandbox"],
  ["model", "model"],
  ["service_tier", "serviceTier"],
  ["personality", "personality"],
]);

function requireRecord(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
}

function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
}

function requireIdentifierSegment(value, label) {
  requireText(value, label);
  if (!IDENTIFIER_SEGMENT.test(value)) {
    throw new TypeError(`${label} must contain only letters, digits, dot, underscore, or hyphen`);
  }
}

function requireSha256(value, label) {
  requireText(value, label);
  if (!/^[0-9a-f]{64}$/.test(value)) throw new TypeError(`${label} must be a SHA-256 digest`);
}

function rejectUnknownFields(value, allowed, label) {
  const unknown = Object.keys(value).filter((field) => !allowed.has(field)).sort();
  if (unknown.length > 0) {
    throw new TypeError(`${label} contains unsupported fields: ${unknown.join(", ")}`);
  }
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonicalJson(value[key])}`,
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}

function freezeProjection(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freezeProjection(child);
  return Object.freeze(value);
}

function normalizeThreadOptions(value, baseDirectory, roleId) {
  if (value == null) return {};
  requireRecord(value, `role ${roleId} thread_options`);
  rejectUnknownFields(value, new Set(THREAD_OPTION_FIELDS.keys()), `role ${roleId} thread_options`);
  const options = {};
  for (const [authoredName, runtimeName] of THREAD_OPTION_FIELDS) {
    if (!(authoredName in value)) continue;
    requireText(value[authoredName], `role ${roleId} thread option ${authoredName}`);
    if (authoredName === "approval_policy" && !APPROVAL_POLICIES.has(value[authoredName])) {
      throw new TypeError(`role ${roleId} thread option approval_policy is unsupported`);
    }
    if (authoredName === "sandbox" && !SANDBOX_MODES.has(value[authoredName])) {
      throw new TypeError(`role ${roleId} thread option sandbox is unsupported`);
    }
    if (authoredName === "personality" && !PERSONALITIES.has(value[authoredName])) {
      throw new TypeError(`role ${roleId} thread option personality is unsupported`);
    }
    options[runtimeName] = authoredName === "cwd"
      ? path.resolve(baseDirectory, value[authoredName])
      : value[authoredName];
  }
  return options;
}

function normalizeSkills(value, baseDirectory, roleId) {
  if (!Array.isArray(value)) throw new TypeError(`role ${roleId} skills must be an array`);
  const names = new Set();
  return value.map((skill, index) => {
    requireRecord(skill, `role ${roleId} skills[${index}]`);
    rejectUnknownFields(skill, SKILL_FIELDS, `role ${roleId} skills[${index}]`);
    requireText(skill.name, `role ${roleId} skills[${index}].name`);
    requireText(skill.path, `role ${roleId} skills[${index}].path`);
    if (skill.compiled_environment != null) {
      requireRecord(skill.compiled_environment, `role ${roleId} skills[${index}] compiled_environment`);
      rejectUnknownFields(skill.compiled_environment, COMPILED_ENVIRONMENT_FIELDS, `role ${roleId} skills[${index}] compiled_environment`);
      requireText(skill.compiled_environment.structure, `role ${roleId} skills[${index}] compiled_environment structure`);
      requireText(skill.compiled_environment.interface, `role ${roleId} skills[${index}] compiled_environment interface`);
      requireSha256(skill.compiled_skill_sha256, `role ${roleId} skills[${index}] compiled_skill_sha256`);
    } else if (skill.compiled_skill_sha256 != null) {
      throw new TypeError(`role ${roleId} skills[${index}] compiled_skill_sha256 requires compiled_environment`);
    }
    if (names.has(skill.name)) {
      throw new TypeError(`role ${roleId} contains duplicate skill name ${skill.name}`);
    }
    names.add(skill.name);
    const normalized = {
      name: skill.name,
      path: path.resolve(baseDirectory, skill.path),
    };
    if (skill.compiled_environment != null) {
      normalized.compiledEnvironment = {
        structure: path.resolve(baseDirectory, skill.compiled_environment.structure),
        interface: path.resolve(baseDirectory, skill.compiled_environment.interface),
      };
      normalized.compiledSkillSha256 = skill.compiled_skill_sha256;
      normalized.capabilities = normalizeCapabilities(skill.capabilities, `${roleId} skill ${skill.name}`);
      normalized.effects = normalizeEffects(skill.effects, `${roleId} skill ${skill.name}`);
    }
    return normalized;
  });
}

function normalizeCapabilities(value, roleId) {
  if (value == null) return [];
  if (!Array.isArray(value)) throw new TypeError(`role ${roleId} capabilities must be an array`);
  const capabilities = value.map((capability, index) => {
    requireIdentifierSegment(capability, `role ${roleId} capabilities[${index}]`);
    return capability;
  });
  if (new Set(capabilities).size !== capabilities.length) {
    throw new TypeError(`role ${roleId} capabilities must be unique`);
  }
  return capabilities.sort();
}

function normalizeEffects(value, roleId) {
  if (value == null) return [];
  if (!Array.isArray(value)) throw new TypeError(`role ${roleId} effects must be an array`);
  const effects = value.map((effect, index) => {
    requireIdentifierSegment(effect, `role ${roleId} effects[${index}]`);
    return effect;
  });
  if (new Set(effects).size !== effects.length) throw new TypeError(`role ${roleId} effects must be unique`);
  return effects.sort();
}

export class RuntimeManifest {
  constructor({ manifestId, source, roles, requirementsBaseDirectory }) {
    this.manifestId = manifestId;
    this.source = freezeProjection(source);
    this.roles = freezeProjection(roles);
    this.requirementsBaseDirectory = requirementsBaseDirectory;
    Object.freeze(this);
  }

  get roleIds() {
    return Object.freeze(Object.keys(this.roles));
  }

  projectRole(roleId, instanceId) {
    requireIdentifierSegment(roleId, "runtime role id");
    requireIdentifierSegment(instanceId, "runtime role instance id");
    const template = this.roles[roleId];
    if (!template) throw new Error(`runtime manifest does not define role ${roleId}`);
    return freezeProjection({
      manifest: this.source,
      roleId,
      roleContract: { ...template.roleContract },
      role: {
        logicalRoleInstanceId: `${roleId}:${instanceId}`,
        capabilities: [...template.capabilities],
        effects: [...template.effects],
        continuity: template.continuity,
        compiledSkillSha256: template.compiledSkillSha256,
        developerInstructions: template.developerInstructions,
        runtimeEnvironmentRevision: template.runtimeEnvironmentRevision,
        threadOptions: { ...template.threadOptions },
      },
      skills: template.skills.map((skill) => ({ ...skill })),
    });
  }
}

export function projectRuntimeManifest(document, {
  baseDirectory = process.cwd(),
  identityBaseDirectory = baseDirectory,
  requirementsBaseDirectory = identityBaseDirectory,
  sourcePath = null,
  sourceSha256 = null,
  runtimeRequirementsByRole = {},
} = {}) {
  requireRecord(document, "runtime manifest");
  rejectUnknownFields(document, TOP_LEVEL_FIELDS, "runtime manifest");
  if (document.schema_version !== 1) {
    throw new TypeError("runtime manifest schema_version must be 1");
  }
  requireIdentifierSegment(document.manifest_id, "runtime manifest id");
  requireRecord(document.roles, "runtime manifest roles");
  if (Object.keys(document.roles).length === 0) {
    throw new TypeError("runtime manifest must define at least one role");
  }

  const resolvedBase = path.resolve(baseDirectory);
  const resolvedIdentityBase = path.resolve(identityBaseDirectory);
  const resolvedRequirementsBase = path.resolve(requirementsBaseDirectory);
  const roles = {};
  for (const roleId of Object.keys(document.roles).sort()) {
    requireIdentifierSegment(roleId, "runtime role id");
    const role = document.roles[roleId];
    requireRecord(role, `role ${roleId}`);
    rejectUnknownFields(role, ROLE_FIELDS, `role ${roleId}`);
    requireText(role.contract, `role ${roleId} contract`);
    if (role.compiled_skill_sha256 != null) requireSha256(role.compiled_skill_sha256, `role ${roleId} compiled_skill_sha256`);
    if (role.developer_instructions != null) requireText(role.developer_instructions, `role ${roleId} developer_instructions`);
    const contractPath = path.resolve(resolvedIdentityBase, role.contract);
    const identitySkills = normalizeSkills(role.skills, resolvedIdentityBase, roleId);
    const skills = normalizeSkills(role.skills, resolvedBase, roleId);
    const contractSkillIndex = identitySkills.findIndex((skill) => skill.path === contractPath);
    if (contractSkillIndex === -1) {
      throw new TypeError(`role ${roleId} contract must be present in its exact skill inputs`);
    }
    const identityRoleContract = { path: contractPath };
    const projectedSkills = skills.map((skill) => {
      const runtimeRequirements = runtimeRequirementsByRole[`${roleId}:${skill.name}`];
      return runtimeRequirements == null ? skill : { ...skill, runtimeRequirements };
    });
    const roleTemplate = {
      roleContract: {
        ...identityRoleContract,
        activatedPath: skills[contractSkillIndex].path,
      },
      developerInstructions: role.developer_instructions ?? "",
      threadOptions: normalizeThreadOptions(
        role.thread_options,
        resolvedIdentityBase,
        roleId,
      ),
      skills: projectedSkills,
      capabilities: normalizeCapabilities(role.capabilities, roleId),
      effects: normalizeEffects(role.effects, roleId),
      continuity: role.continuity == null ? "ephemeral" : role.continuity,
      compiledSkillSha256: role.compiled_skill_sha256 ?? null,
      runtimeRequirements: runtimeRequirementsByRole[roleId] ?? null,
    };
    if (!CONTINUITY_KINDS.has(roleTemplate.continuity)) {
      throw new TypeError(`role ${roleId} continuity is unsupported`);
    }
    roles[roleId] = {
      ...roleTemplate,
      runtimeEnvironmentRevision: createHash("sha256")
        .update(canonicalJson({
          ...roleTemplate,
          roleContract: identityRoleContract,
          skills: identitySkills,
        }))
        .digest("hex"),
    };
  }

  const digest = sourceSha256 ?? createHash("sha256")
    .update(canonicalJson(document))
    .digest("hex");
  return new RuntimeManifest({
    manifestId: document.manifest_id,
    source: {
      manifestId: document.manifest_id,
      schemaVersion: 1,
      path: sourcePath ? path.resolve(sourcePath) : null,
      sha256: digest,
    },
    roles,
    requirementsBaseDirectory: resolvedRequirementsBase,
  });
}

export async function loadRuntimeManifestDocument(manifestPath) {
  const resolvedPath = path.resolve(manifestPath);
  const source = await readFile(resolvedPath, "utf8");
  const { parseDocument } = await import("yaml");
  const parsed = parseDocument(source, { uniqueKeys: true });
  if (parsed.errors.length > 0) {
    throw new TypeError(`invalid runtime manifest YAML: ${parsed.errors[0].message}`);
  }
  const document = parsed.toJS({ maxAliasCount: 0 });
  return Object.freeze({
    document,
    source,
    sourcePath: resolvedPath,
    sourceSha256: createHash("sha256").update(source).digest("hex"),
  });
}

export async function hydrateRuntimeRequirements(document, {
  baseDirectory,
  workspaceRoot,
} = {}) {
  requireRecord(document?.roles, "runtime manifest roles");
  const resolvedBase = path.resolve(baseDirectory ?? process.cwd());
  const resolvedWorkspace = path.resolve(workspaceRoot ?? resolvedBase);
  const runtimeRequirementsByRole = {};
  for (const [roleId, role] of Object.entries(document.roles)) {
    const { compileSkill } = await import("./skill-compiler.mjs");
    if (role.compiled_environment != null) {
      requireRecord(role.compiled_environment, `role ${roleId} compiled_environment`);
      rejectUnknownFields(role.compiled_environment, COMPILED_ENVIRONMENT_FIELDS, `role ${roleId} compiled_environment`);
      const compiled = await compileSkill({
        structureSource: await readFile(path.resolve(resolvedBase, role.compiled_environment.structure), "utf8"),
        interfaceSource: await readFile(path.resolve(resolvedBase, role.compiled_environment.interface), "utf8"),
        workspaceRoot: resolvedWorkspace,
      });
      if (!compiled.ir.role_profile) throw new Error(`role ${roleId} compiled environment does not define a role profile`);
      runtimeRequirementsByRole[roleId] = compiled.ir.runtime_requirements;
    }
    for (const [index, skill] of (role.skills ?? []).entries()) {
      if (skill.compiled_environment == null) continue;
      const compiled = await compileSkill({
        structureSource: await readFile(path.resolve(resolvedBase, skill.compiled_environment.structure), "utf8"),
        interfaceSource: await readFile(path.resolve(resolvedBase, skill.compiled_environment.interface), "utf8"),
        workspaceRoot: resolvedWorkspace,
      });
      if (compiled.ir.role_profile) throw new Error(`role ${roleId} skills[${index}] secondary compiled skill must not define a role profile`);
      if (compiled.ir.skill_id !== skill.name) throw new Error(`role ${roleId} skills[${index}] compiled skill identity differs`);
      if (compiled.ir.output_sha256 !== skill.compiled_skill_sha256) throw new Error(`role ${roleId} skills[${index}] compiled skill fingerprint differs`);
      runtimeRequirementsByRole[`${roleId}:${skill.name}`] = compiled.ir.runtime_requirements;
    }
  }
  return freezeProjection(runtimeRequirementsByRole);
}

export async function loadRuntimeManifest(manifestPath) {
  const loaded = await loadRuntimeManifestDocument(manifestPath);
  const base = path.dirname(loaded.sourcePath);
  const workspaceRoot = path.resolve(base, "..");
  const runtimeRequirementsByRole = await hydrateRuntimeRequirements(loaded.document, {
    baseDirectory: base,
    workspaceRoot,
  });
  return projectRuntimeManifest(loaded.document, {
    baseDirectory: base,
    requirementsBaseDirectory: workspaceRoot,
    sourcePath: loaded.sourcePath,
    sourceSha256: loaded.sourceSha256,
    runtimeRequirementsByRole,
  });
}

export function satisfyRuntimeRequirements({ manifest, roleId, requirements, skillName = null }) {
  if (!(manifest instanceof RuntimeManifest)) throw new TypeError("runtime satisfaction requires a projected runtime manifest");
  requireRecord(requirements, "runtime requirements");
  if (requirements.schema_version !== 1) throw new TypeError("runtime requirements schema_version must be 1");
  if (requirements.verified_sources !== true) throw new Error("runtime requirements are not bound to verified sources");
  for (const field of ["sha256", "compiled_skill_sha256"]) requireText(requirements[field], `runtime requirements ${field}`);
  const unsignedRequirements = { ...requirements };
  delete unsignedRequirements.sha256;
  const observedRequirementsSha256 = createHash("sha256").update(canonicalJson(unsignedRequirements)).digest("hex");
  if (observedRequirementsSha256 !== requirements.sha256) throw new Error("runtime requirements digest mismatch");
  const projection = manifest.projectRole(roleId, "admission");
  const activatedSkill = skillName == null
    ? null
    : projection.skills.find(({ name }) => name === skillName);
  if (skillName != null && !activatedSkill) throw new Error(`runtime role ${roleId} omits compiled skill ${skillName}`);
  const grantedCapabilities = new Set(skillName == null ? projection.role.capabilities : activatedSkill.capabilities);
  if (skillName != null) {
    for (const capability of grantedCapabilities) {
      if (!projection.role.capabilities.includes(capability)) throw new Error(`compiled skill ${skillName} capability ${capability} is not granted to runtime role ${roleId}`);
    }
  }
  for (const capability of requirements.required_capabilities ?? []) {
    if (!grantedCapabilities.has(capability)) throw new Error(`runtime role ${roleId} is missing required capability ${capability}`);
  }
  const capabilityCeiling = new Set(requirements.capability_ceiling ?? []);
  for (const capability of grantedCapabilities) {
    if (!capabilityCeiling.has(capability)) throw new Error(`runtime role ${roleId} exceeds capability ceiling with ${capability}`);
  }
  const ceiling = new Set(requirements.effect_ceiling ?? []);
  const prohibited = new Set(requirements.prohibited_effects ?? []);
  const grantedEffects = skillName == null ? projection.role.effects : activatedSkill.effects;
  if (skillName != null) {
    for (const effect of grantedEffects) {
      if (!projection.role.effects.includes(effect)) throw new Error(`compiled skill ${skillName} effect ${effect} is not granted to runtime role ${roleId}`);
    }
  }
  for (const effect of grantedEffects) {
    if (prohibited.has(effect)) throw new Error(`runtime role ${roleId} grants prohibited effect ${effect}`);
    if (!ceiling.has(effect)) throw new Error(`runtime role ${roleId} exceeds effect ceiling with ${effect}`);
  }
  if (skillName == null && projection.role.continuity !== requirements.continuity) {
    throw new Error(`runtime role ${roleId} has incompatible continuity`);
  }
  const compiledSkillSha256 = skillName == null ? projection.role.compiledSkillSha256 : activatedSkill.compiledSkillSha256;
  if (compiledSkillSha256 !== requirements.compiled_skill_sha256) {
    throw new Error(`runtime role ${roleId} compiled skill fingerprint differs from requirements`);
  }
  if (!requirements.contract?.must_be_activated) throw new TypeError("runtime requirements must require the exact contract input");
  const requiredPath = path.resolve(manifest.requirementsBaseDirectory, requirements.contract.path);
  if (skillName == null) {
    if (requirements.contract.kind !== "role_contract") throw new Error("role requirements must identify a role contract");
    if (projection.roleContract.path !== requiredPath) throw new Error(`runtime role ${roleId} contract differs from compiled requirements`);
    if (!projection.skills.some(({ path: skillPath }) => skillPath === projection.roleContract.activatedPath)) {
      throw new Error(`runtime role ${roleId} omits the compiled contract input`);
    }
  } else {
    if (requirements.contract.kind !== "skill") throw new Error(`compiled skill ${skillName} requirements must identify a secondary skill`);
    if (activatedSkill.path !== requiredPath) throw new Error(`runtime role ${roleId} compiled skill ${skillName} path differs from requirements`);
  }
  const receipt = {
    schema_version: 1,
    role_id: roleId,
    requirements_sha256: requirements.sha256,
    compiled_skill_sha256: requirements.compiled_skill_sha256,
    ...(skillName == null ? {} : { skill_id: skillName }),
    manifest_sha256: projection.manifest.sha256,
    runtime_environment_revision: projection.role.runtimeEnvironmentRevision,
  };
  return freezeProjection({ ...receipt, sha256: createHash("sha256").update(canonicalJson(receipt)).digest("hex") });
}

export class ManifestRoleRuntime {
  constructor({ adapter, manifest, runtimeRequirements = null }) {
    if (!adapter || typeof adapter.deliverTurn !== "function") {
      throw new TypeError("ManifestRoleRuntime requires an App Server adapter");
    }
    if (!(manifest instanceof RuntimeManifest)) {
      throw new TypeError("ManifestRoleRuntime requires a projected runtime manifest");
    }
    this.adapter = adapter;
    this.manifest = manifest;
    this.runtimeRequirements = runtimeRequirements;
  }

  async deliverTurn({ roleId, instanceId, ...turn }) {
    const projection = this.manifest.projectRole(roleId, instanceId);
    const requirements = this.runtimeRequirements?.[roleId] ?? this.manifest.roles[roleId].runtimeRequirements;
    if (projection.role.compiledSkillSha256 && !requirements) {
      throw new Error(`runtime role ${roleId} has no verified compiled requirements`);
    }
    const runtimeSatisfaction = requirements
      ? satisfyRuntimeRequirements({ manifest: this.manifest, roleId, requirements })
      : null;
    const skillRuntimeSatisfactions = {};
    for (const skill of projection.skills) {
      const skillRequirements = this.runtimeRequirements?.[`${roleId}:${skill.name}`] ?? skill.runtimeRequirements;
      if (skill.compiledSkillSha256 && !skillRequirements) {
        throw new Error(`runtime role ${roleId} compiled skill ${skill.name} has no verified compiled requirements`);
      }
      if (skillRequirements) {
        skillRuntimeSatisfactions[skill.name] = satisfyRuntimeRequirements({
          manifest: this.manifest, roleId, requirements: skillRequirements, skillName: skill.name,
        });
      }
    }
    const delivery = await this.adapter.deliverTurn({
      ...turn,
      role: projection.role,
      skills: projection.skills,
    });
    return Object.freeze({
      ...delivery,
      roleProjection: projection,
      runtimeSatisfaction,
      skillRuntimeSatisfactions: freezeProjection(skillRuntimeSatisfactions),
    });
  }
}
