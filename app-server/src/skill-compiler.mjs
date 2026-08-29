import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";

import { parseDocument } from "yaml";

import { createAgentEnvironmentGraphAdapter } from "./agent-environment-graph-adapter.mjs";

const STRUCTURE_FIELDS = new Set([
  "schema_version", "status", "skill_id", "source", "frontmatter", "document",
  "source_bindings", "role_profile", "sections",
]);
const INTERFACE_FIELDS = new Set(["schema_version", "status", "skill_id", "boundaries", "runtime_requirements"]);
const SECTION_FIELDS = new Set(["id", "kind", "heading", "content", "source_span"]);
const HEADING_FIELDS = new Set(["level", "text"]);
const SPAN_FIELDS = new Set(["start_byte", "end_byte", "sha256"]);
const SOURCE_FIELDS = new Set(["path", "repository_revision", "sha256", "producer"]);
const SOURCE_BINDING_FIELDS = new Set(["id", "kind", "path", "sha256", "role"]);
const SOURCE_BINDING_KINDS = new Set(["canonical_authority", "generated_evidence"]);
const ROLE_FIELDS = new Set([
  "role_id", "label", "objective", "context_lifetime", "projection", "relations",
]);
const PROJECTION_FIELDS = new Set(["path", "sha256"]);
const BOUNDARY_FIELDS = new Set([
  "id", "kind", "semantic_section", "authority_source", "enforcement",
]);
const RELATIONS = [
  "bound_by", "may_invoke", "may_observe", "may_mutate", "owns", "consumes",
  "emits", "mediated_transitions", "forbidden_from",
];
const SECTION_KINDS = new Set([
  "identity", "operation", "authority", "lifecycle", "planning", "implementation",
  "validation", "escalation", "boundary", "receipt",
]);
const BOUNDARY_KINDS = new Set(["authority", "mutation", "mediation", "prohibition", "interface"]);
const ENFORCEMENTS = new Set(["scoped_write", "mediated", "denied", "receipt_contract"]);
const BOUNDARY_SECTION_KINDS = Object.freeze({
  authority: new Set(["authority", "boundary"]),
  mutation: new Set(["implementation", "boundary"]),
  mediation: new Set(["implementation", "receipt"]),
  prohibition: new Set(["planning", "boundary"]),
  interface: new Set(["receipt"]),
});
const RUNTIME_REQUIREMENT_FIELDS = new Set(["required_capabilities", "capability_ceiling", "effect_ceiling", "prohibited_effects", "continuity"]);
const CONTINUITY_KINDS = new Set(["ephemeral", "retained"]);

function fail(message) {
  throw new TypeError(message);
}

function object(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) fail(`${label} must be an object`);
  return value;
}

function exactFields(value, allowed, label) {
  object(value, label);
  const unknown = Object.keys(value).filter((key) => !allowed.has(key));
  if (unknown.length) fail(`${label} contains unknown field ${unknown[0]}`);
}

function text(value, label) {
  if (typeof value !== "string" || value.length === 0) fail(`${label} must be nonempty text`);
  return value;
}

function integer(value, label) {
  if (!Number.isInteger(value) || value < 0) fail(`${label} must be a nonnegative integer`);
  return value;
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
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

function digest(value, label) {
  if (typeof value !== "string" || !/^[0-9a-f]{64}$/.test(value)) fail(`${label} must be a SHA-256 digest`);
  return value;
}

function uniqueTexts(value, label) {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  const result = value.map((item, index) => text(item, `${label}[${index}]`));
  if (new Set(result).size !== result.length) fail(`${label} contains duplicates`);
  return result;
}

function relationEntries(value, relation, label) {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  const result = value.map((entry, index) => {
    if (relation === "may_mutate") {
      exactFields(entry, new Set(["target", "boundary"]), `${label}[${index}]`);
      return { target: text(entry.target, `${label}[${index}].target`), boundary: text(entry.boundary, `${label}[${index}].boundary`) };
    }
    if (relation === "mediated_transitions") {
      exactFields(entry, new Set(["transition", "mediated_by"]), `${label}[${index}]`);
      return { transition: text(entry.transition, `${label}[${index}].transition`), mediated_by: text(entry.mediated_by, `${label}[${index}].mediated_by`) };
    }
    return text(entry, `${label}[${index}]`);
  });
  if (new Set(result.map((entry) => JSON.stringify(entry))).size !== result.length) fail(`${label} contains duplicates`);
  return result;
}

function parseYaml(source, label) {
  const document = parseDocument(source, { uniqueKeys: true, maxAliasCount: 0 });
  if (document.errors.length) fail(`invalid ${label} YAML: ${document.errors[0].message}`);
  return object(document.toJS({ maxAliasCount: 0 }), label);
}

function validateStructure(raw) {
  exactFields(raw, STRUCTURE_FIELDS, "structure");
  if (raw.schema_version !== 1) fail("structure.schema_version must be 1");
  if (raw.status !== "experimental_non_authoritative") fail("structure.status must preserve experimental ownership");
  const skillId = text(raw.skill_id, "structure.skill_id");
  exactFields(raw.source, SOURCE_FIELDS, "structure.source");
  const source = {
    path: text(raw.source.path, "structure.source.path"),
    repository_revision: text(raw.source.repository_revision, "structure.source.repository_revision"),
    sha256: digest(raw.source.sha256, "structure.source.sha256"),
    producer: text(raw.source.producer, "structure.source.producer"),
  };
  exactFields(raw.frontmatter, new Set(["name", "description"]), "structure.frontmatter");
  if (raw.frontmatter.name !== skillId) fail("structure.frontmatter.name must equal skill_id");
  const frontmatter = {
    name: text(raw.frontmatter.name, "structure.frontmatter.name"),
    description: text(raw.frontmatter.description, "structure.frontmatter.description"),
  };
  exactFields(raw.document, new Set(["line_ending", "final_newline"]), "structure.document");
  if (raw.document.line_ending !== "lf") fail("only LF line endings are supported by schema v1");
  if (typeof raw.document.final_newline !== "boolean") fail("structure.document.final_newline must be boolean");

  if (!Array.isArray(raw.source_bindings) || raw.source_bindings.length === 0) fail("structure.source_bindings must be nonempty");
  const sourceBindings = raw.source_bindings.map((entry, index) => {
    exactFields(entry, SOURCE_BINDING_FIELDS, `structure.source_bindings[${index}]`);
    const kind = text(entry.kind, `structure.source_bindings[${index}].kind`);
    if (!SOURCE_BINDING_KINDS.has(kind)) fail(`unsupported source binding kind ${kind}`);
    return {
      id: text(entry.id, `structure.source_bindings[${index}].id`), kind,
      path: text(entry.path, `structure.source_bindings[${index}].path`),
      sha256: digest(entry.sha256, `structure.source_bindings[${index}].sha256`),
      role: text(entry.role, `structure.source_bindings[${index}].role`),
    };
  });
  if (new Set(sourceBindings.map(({ id }) => id)).size !== sourceBindings.length) fail("structure.source_bindings contains duplicate ids");

  if (!Array.isArray(raw.sections) || raw.sections.length < 2) fail("structure.sections must meaningfully decompose the document");
  const sections = raw.sections.map((entry, index) => {
    exactFields(entry, SECTION_FIELDS, `structure.sections[${index}]`);
    exactFields(entry.heading, HEADING_FIELDS, `structure.sections[${index}].heading`);
    exactFields(entry.source_span, SPAN_FIELDS, `structure.sections[${index}].source_span`);
    const kind = text(entry.kind, `structure.sections[${index}].kind`);
    if (!SECTION_KINDS.has(kind)) fail(`unsupported section kind ${kind}`);
    const level = integer(entry.heading.level, `structure.sections[${index}].heading.level`);
    if (level < 1 || level > 6) fail("section heading level must be from 1 through 6");
    if (typeof entry.content !== "string") fail(`structure.sections[${index}].content must be text`);
    return {
      id: text(entry.id, `structure.sections[${index}].id`), kind,
      heading: { level, text: text(entry.heading.text, `structure.sections[${index}].heading.text`) },
      content: entry.content,
      source_span: {
        start_byte: integer(entry.source_span.start_byte, `structure.sections[${index}].source_span.start_byte`),
        end_byte: integer(entry.source_span.end_byte, `structure.sections[${index}].source_span.end_byte`),
        sha256: digest(entry.source_span.sha256, `structure.sections[${index}].source_span.sha256`),
      },
    };
  });
  if (new Set(sections.map(({ id }) => id)).size !== sections.length) fail("structure.sections contains duplicate ids");
  for (const section of sections) {
    if (section.source_span.end_byte <= section.source_span.start_byte) fail(`section ${section.id} has an empty source span`);
    const rendered = `${"#".repeat(section.heading.level)} ${section.heading.text}\n${section.content}`;
    if (sha256(rendered) !== section.source_span.sha256) fail(`section ${section.id} source span digest mismatch`);
  }

  exactFields(raw.role_profile, ROLE_FIELDS, "structure.role_profile");
  exactFields(raw.role_profile.projection, PROJECTION_FIELDS, "structure.role_profile.projection");
  exactFields(raw.role_profile.relations, new Set(RELATIONS), "structure.role_profile.relations");
  const relations = Object.fromEntries(RELATIONS.map((relation) => [relation, relationEntries(raw.role_profile.relations[relation], relation, `structure.role_profile.relations.${relation}`)]));
  return {
    schema_version: 1, status: raw.status, skill_id: skillId, source, frontmatter,
    document: raw.document, source_bindings: sourceBindings,
    role_profile: {
      role_id: text(raw.role_profile.role_id, "structure.role_profile.role_id"),
      label: text(raw.role_profile.label, "structure.role_profile.label"),
      objective: text(raw.role_profile.objective, "structure.role_profile.objective"),
      context_lifetime: text(raw.role_profile.context_lifetime, "structure.role_profile.context_lifetime"),
      projection: {
        path: text(raw.role_profile.projection.path, "structure.role_profile.projection.path"),
        sha256: digest(raw.role_profile.projection.sha256, "structure.role_profile.projection.sha256"),
      }, relations,
    }, sections,
  };
}

function validateInterface(raw, structure) {
  exactFields(raw, INTERFACE_FIELDS, "interface");
  if (raw.schema_version !== 1) fail("interface.schema_version must be 1");
  if (raw.status !== "experimental_non_authoritative") fail("interface.status must preserve experimental ownership");
  if (raw.skill_id !== structure.skill_id) fail("interface.skill_id must match structure.skill_id");
  if (!Array.isArray(raw.boundaries) || raw.boundaries.length === 0) fail("interface.boundaries must be nonempty");
  const sectionsById = new Map(structure.sections.map((section) => [section.id, section]));
  const authorityIds = new Set(structure.source_bindings
    .filter(({ kind }) => kind === "canonical_authority")
    .map(({ id }) => id));
  const boundaries = raw.boundaries.map((entry, index) => {
    exactFields(entry, BOUNDARY_FIELDS, `interface.boundaries[${index}]`);
    const kind = text(entry.kind, `interface.boundaries[${index}].kind`);
    const enforcement = text(entry.enforcement, `interface.boundaries[${index}].enforcement`);
    if (!BOUNDARY_KINDS.has(kind)) fail(`unsupported boundary kind ${kind}`);
    if (!ENFORCEMENTS.has(enforcement)) fail(`unsupported enforcement primitive ${enforcement}`);
    const semanticSection = text(entry.semantic_section, `interface.boundaries[${index}].semantic_section`);
    const authoritySource = text(entry.authority_source, `interface.boundaries[${index}].authority_source`);
    const section = sectionsById.get(semanticSection);
    if (!section) fail(`unresolved semantic section ${semanticSection}`);
    if (!BOUNDARY_SECTION_KINDS[kind].has(section.kind)) {
      fail(`boundary kind ${kind} is incompatible with semantic section kind ${section.kind}`);
    }
    if (!authorityIds.has(authoritySource)) fail(`unresolved canonical authority source ${authoritySource}`);
    return { id: text(entry.id, `interface.boundaries[${index}].id`), kind, semantic_section: semanticSection, authority_source: authoritySource, enforcement };
  });
  if (new Set(boundaries.map(({ id }) => id)).size !== boundaries.length) fail("interface.boundaries contains duplicate ids");
  exactFields(raw.runtime_requirements, RUNTIME_REQUIREMENT_FIELDS, "interface.runtime_requirements");
  const requiredCapabilities = uniqueTexts(raw.runtime_requirements.required_capabilities, "interface.runtime_requirements.required_capabilities").sort();
  const capabilityCeiling = uniqueTexts(raw.runtime_requirements.capability_ceiling, "interface.runtime_requirements.capability_ceiling").sort();
  const invokableCapabilities = new Set(structure.role_profile.relations.may_invoke);
  for (const capability of requiredCapabilities) {
    if (!invokableCapabilities.has(capability)) fail(`runtime requirement capability ${capability} is not declared by the role`);
  }
  for (const capability of capabilityCeiling) {
    if (!invokableCapabilities.has(capability)) fail(`runtime capability ceiling ${capability} is not declared by the role`);
  }
  for (const capability of requiredCapabilities) {
    if (!capabilityCeiling.includes(capability)) fail(`required capability ${capability} exceeds the runtime capability ceiling`);
  }
  const effectCeiling = uniqueTexts(raw.runtime_requirements.effect_ceiling, "interface.runtime_requirements.effect_ceiling").sort();
  const mutableTargets = new Set(structure.role_profile.relations.may_mutate.map(({ target }) => target));
  for (const effect of effectCeiling) {
    if (!mutableTargets.has(effect)) fail(`runtime effect ceiling ${effect} is not declared by the role`);
  }
  const prohibitedEffects = uniqueTexts(raw.runtime_requirements.prohibited_effects, "interface.runtime_requirements.prohibited_effects").sort();
  for (const effect of prohibitedEffects) {
    if (effectCeiling.includes(effect)) fail(`runtime effect ${effect} cannot be both permitted and prohibited`);
  }
  const continuity = text(raw.runtime_requirements.continuity, "interface.runtime_requirements.continuity");
  if (!CONTINUITY_KINDS.has(continuity)) fail(`unsupported runtime continuity ${continuity}`);
  return {
    schema_version: 1, status: raw.status, skill_id: raw.skill_id, boundaries,
    runtime_requirements: {
      required_capabilities: requiredCapabilities, capability_ceiling: capabilityCeiling, effect_ceiling: effectCeiling,
      prohibited_effects: prohibitedEffects, continuity,
    },
  };
}

function projectRuntimeRequirements(structure, skillInterface, outputSha256, verifiedSources) {
  const requirements = {
    schema_version: 1,
    role_id: structure.role_profile.role_id,
    skill_id: structure.skill_id,
    contract: {
      source_binding_id: structure.source_bindings.find(({ kind }) => kind === "canonical_authority")?.id,
      path: structure.source.path,
      sha256: structure.source.sha256,
      must_be_activated: true,
    },
    ...skillInterface.runtime_requirements,
    compiled_skill_sha256: outputSha256,
    verified_sources: verifiedSources,
  };
  return Object.freeze({ ...requirements, sha256: sha256(Buffer.from(canonicalJson(requirements))) });
}

function renderSchemaV1Frontmatter(frontmatter) {
  const description = frontmatter.description;
  if (description.includes("\n")) fail("schema v1 frontmatter description must be one line");
  return Buffer.from(`---\nname: ${frontmatter.name}\ndescription: ${description}\n---\n\n`);
}

function renderCodexSkill(structure) {
  const frontmatter = renderSchemaV1Frontmatter(structure.frontmatter);
  let body = structure.sections.map((section) => `${"#".repeat(section.heading.level)} ${section.heading.text}\n${section.content}`).join("");
  if (!structure.document.final_newline && body.endsWith("\n")) body = body.slice(0, -1);
  if (structure.document.final_newline && !body.endsWith("\n")) body += "\n";
  return Buffer.concat([frontmatter, Buffer.from(body)]);
}

async function verifySourceBindings(structure, workspaceRoot) {
  for (const source of structure.source_bindings) {
    const bytes = await readFile(path.resolve(workspaceRoot, source.path));
    if (sha256(bytes) !== source.sha256) fail(`source binding ${source.id} digest mismatch`);
  }
}

async function verifyLegacySource(structure, workspaceRoot) {
  const bytes = await readFile(path.resolve(workspaceRoot, structure.source.path));
  if (sha256(bytes) !== structure.source.sha256) fail("legacy source digest mismatch");
  const frontmatter = renderSchemaV1Frontmatter(structure.frontmatter);
  if (!bytes.subarray(0, frontmatter.length).equals(frontmatter)) {
    fail("structure frontmatter does not match canonical source prefix");
  }
  if (structure.sections[0].source_span.start_byte !== frontmatter.length) {
    fail("first section source span must begin immediately after canonical frontmatter");
  }
  let previousEnd = null;
  for (const section of structure.sections) {
    const { start_byte: start, end_byte: end } = section.source_span;
    if (previousEnd !== null && start !== previousEnd) fail(`section ${section.id} source span is not contiguous`);
    const rendered = Buffer.from(`${"#".repeat(section.heading.level)} ${section.heading.text}\n${section.content}`);
    if (!bytes.subarray(start, end).equals(rendered)) fail(`section ${section.id} source span does not match legacy bytes`);
    previousEnd = end;
  }
  if (previousEnd !== bytes.length) fail("section source spans do not reach the end of the legacy source");
}

async function verifyRoleProjection(structure, workspaceRoot, aegAdapter) {
  const projectionPath = path.resolve(workspaceRoot, structure.role_profile.projection.path);
  const bytes = await readFile(projectionPath);
  if (sha256(bytes) !== structure.role_profile.projection.sha256) fail("role projection digest mismatch");
  const oracle = parseYaml(bytes.toString("utf8"), "role projection");
  const role = {
    label: structure.role_profile.label,
    objective: structure.role_profile.objective,
    context_lifetime: structure.role_profile.context_lifetime,
    ...structure.role_profile.relations,
  };
  const envelope = await aegAdapter.projectRole({
    roleId: structure.role_profile.role_id,
    role,
  });
  if (canonicalJson(envelope.projection) !== canonicalJson(oracle)) {
    fail("complete role projection differs from pinned generated oracle");
  }
  return envelope;
}

export async function compileSkill({
  structureSource,
  interfaceSource,
  workspaceRoot = process.cwd(),
  verifySources = true,
  agentEnvironmentGraphAdapter = null,
}) {
  const structure = validateStructure(parseYaml(structureSource, "structure"));
  const skillInterface = validateInterface(parseYaml(interfaceSource, "interface"), structure);
  if (verifySources) {
    await verifyLegacySource(structure, workspaceRoot);
    await verifySourceBindings(structure, workspaceRoot);
    agentEnvironmentGraphAdapter ??= createAgentEnvironmentGraphAdapter({ workspaceRoot });
  }
  const roleProjection = verifySources
    ? await verifyRoleProjection(structure, workspaceRoot, agentEnvironmentGraphAdapter)
    : null;
  const output = renderCodexSkill(structure);
  const outputSha256 = sha256(output);
  const runtimeRequirements = projectRuntimeRequirements(structure, skillInterface, outputSha256, roleProjection !== null);
  const ir = {
    schema_version: 1,
    compiler: "work-engine.skill-compiler.bootstrap-v1",
    skill_id: structure.skill_id,
    status: "experimental_non_authoritative",
    input_sha256: {
      structure: sha256(Buffer.from(structureSource)),
      interface: sha256(Buffer.from(interfaceSource)),
      role_projection: structure.role_profile.projection.sha256,
    },
    source: structure.source,
    section_provenance: structure.sections.map(({ id, kind, source_span }) => ({ id, kind, source_span })),
    source_bindings: structure.source_bindings,
    role_profile: structure.role_profile,
    role_projection: roleProjection ? {
      backend: roleProjection.backend,
      backend_sha256: roleProjection.backend_sha256,
      canonical_role_match: roleProjection.canonical_role_match,
      projection: roleProjection.projection,
    } : null,
    interface: skillInterface,
    runtime_requirements: runtimeRequirements,
    output_sha256: outputSha256,
  };
  return { ir, output };
}

export const skillCompilerInternals = Object.freeze({ parseYaml, validateStructure, validateInterface, renderCodexSkill, projectRuntimeRequirements, sha256 });
