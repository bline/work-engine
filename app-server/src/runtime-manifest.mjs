import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";

const TOP_LEVEL_FIELDS = new Set(["schema_version", "manifest_id", "roles"]);
const ROLE_FIELDS = new Set(["contract", "developer_instructions", "thread_options", "skills", "capabilities"]);
const SKILL_FIELDS = new Set(["name", "path"]);
const IDENTIFIER_SEGMENT = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;
const APPROVAL_POLICIES = new Set(["untrusted", "on-request", "never"]);
const SANDBOX_MODES = new Set(["read-only", "workspace-write", "danger-full-access"]);
const PERSONALITIES = new Set(["none", "friendly", "pragmatic"]);
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
    if (names.has(skill.name)) {
      throw new TypeError(`role ${roleId} contains duplicate skill name ${skill.name}`);
    }
    names.add(skill.name);
    return {
      name: skill.name,
      path: path.resolve(baseDirectory, skill.path),
    };
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

export class RuntimeManifest {
  constructor({ manifestId, source, roles }) {
    this.manifestId = manifestId;
    this.source = freezeProjection(source);
    this.roles = freezeProjection(roles);
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
  sourcePath = null,
  sourceSha256 = null,
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
  const roles = {};
  for (const roleId of Object.keys(document.roles).sort()) {
    requireIdentifierSegment(roleId, "runtime role id");
    const role = document.roles[roleId];
    requireRecord(role, `role ${roleId}`);
    rejectUnknownFields(role, ROLE_FIELDS, `role ${roleId}`);
    requireText(role.contract, `role ${roleId} contract`);
    requireText(role.developer_instructions, `role ${roleId} developer_instructions`);
    const contractPath = path.resolve(resolvedIdentityBase, role.contract);
    const identitySkills = normalizeSkills(role.skills, resolvedIdentityBase, roleId);
    const skills = normalizeSkills(role.skills, resolvedBase, roleId);
    const contractSkillIndex = identitySkills.findIndex((skill) => skill.path === contractPath);
    if (contractSkillIndex === -1) {
      throw new TypeError(`role ${roleId} contract must be present in its exact skill inputs`);
    }
    const identityRoleContract = { path: contractPath };
    const roleTemplate = {
      roleContract: {
        ...identityRoleContract,
        activatedPath: skills[contractSkillIndex].path,
      },
      developerInstructions: role.developer_instructions,
      threadOptions: normalizeThreadOptions(
        role.thread_options,
        resolvedIdentityBase,
        roleId,
      ),
      skills,
      capabilities: normalizeCapabilities(role.capabilities, roleId),
    };
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

export async function loadRuntimeManifest(manifestPath) {
  const loaded = await loadRuntimeManifestDocument(manifestPath);
  return projectRuntimeManifest(loaded.document, {
    baseDirectory: path.dirname(loaded.sourcePath),
    sourcePath: loaded.sourcePath,
    sourceSha256: loaded.sourceSha256,
  });
}

export class ManifestRoleRuntime {
  constructor({ adapter, manifest }) {
    if (!adapter || typeof adapter.deliverTurn !== "function") {
      throw new TypeError("ManifestRoleRuntime requires an App Server adapter");
    }
    if (!(manifest instanceof RuntimeManifest)) {
      throw new TypeError("ManifestRoleRuntime requires a projected runtime manifest");
    }
    this.adapter = adapter;
    this.manifest = manifest;
  }

  async deliverTurn({ roleId, instanceId, ...turn }) {
    const projection = this.manifest.projectRole(roleId, instanceId);
    const delivery = await this.adapter.deliverTurn({
      ...turn,
      role: projection.role,
      skills: projection.skills,
    });
    return Object.freeze({ ...delivery, roleProjection: projection });
  }
}
