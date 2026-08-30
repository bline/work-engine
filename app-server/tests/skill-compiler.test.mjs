import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { parse, stringify } from "yaml";

import { compileSkill } from "../src/skill-compiler.mjs";

const digest = (value) => createHash("sha256").update(value).digest("hex");

test("vertical compiler path preserves exact bytes, provenance, and authority closure", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "skill-compiler-"));
  const authority = Buffer.from("authority\n");
  await writeFile(path.join(root, "authority.txt"), authority);
  const projection = stringify({
    role_id: "role.builder",
    role: {
      bound_by: ["INV-001"], must_require: { "role.reviewer": ["INV-012"] },
      may_invoke: [], may_observe: ["artifact.plan"],
      observation_limits: { "artifact.plan": "accepted consequence only" }, may_mutate: [], owns: [],
      consumes: [], emits: [], mediated_transitions: [], forbidden_from: [],
    },
  });
  await writeFile(path.join(root, "projection.yaml"), projection);
  const sections = [
    { id: "identity", kind: "identity", heading: { level: 1, text: "Vertical" }, content: "\nExact prose.\n\n" },
    { id: "authority", kind: "boundary", heading: { level: 2, text: "Authority" }, content: "\nMutation is scoped.\n" },
  ].map((section, index) => {
    const rendered = `${"#".repeat(section.heading.level)} ${section.heading.text}\n${section.content}`;
    return { ...section, source_span: { start_byte: index * 100, end_byte: index * 100 + Buffer.byteLength(rendered), sha256: digest(rendered) } };
  });
  const legacy = Buffer.from("---\nname: vertical\ndescription: Vertical proof.\n---\n\n# Vertical\n\nExact prose.\n\n## Authority\n\nMutation is scoped.\n");
  await writeFile(path.join(root, "legacy.md"), legacy);
  const bodyOffset = legacy.indexOf(Buffer.from("# Vertical"));
  let nextOffset = bodyOffset;
  for (const section of sections) {
    const rendered = `${"#".repeat(section.heading.level)} ${section.heading.text}\n${section.content}`;
    section.source_span.start_byte = nextOffset;
    section.source_span.end_byte = nextOffset + Buffer.byteLength(rendered);
    nextOffset = section.source_span.end_byte;
  }
  const structure = {
    schema_version: 1, status: "experimental_non_authoritative", skill_id: "vertical",
    source: { path: "legacy.md", repository_revision: "fixture", sha256: digest(legacy), producer: "fixture migration" },
    frontmatter: { name: "vertical", description: "Vertical proof." },
    document: { line_ending: "lf", final_newline: true },
    source_bindings: [{
      id: "authority.fixture", kind: "canonical_authority", path: "authority.txt",
      sha256: digest(authority), role: "fixture authority",
    }],
    role_profile: {
      role_id: "role.builder", label: "Builder", objective: "Build the fixture.",
      context_lifetime: "one fixture",
      projection: { path: "projection.yaml", sha256: digest(projection) },
      relations: { bound_by: ["INV-001"], must_require: { "role.reviewer": ["INV-012"] }, may_invoke: [], may_observe: ["artifact.plan"], observation_limits: { "artifact.plan": "accepted consequence only" }, may_mutate: [], owns: [], consumes: [], emits: [], mediated_transitions: [], forbidden_from: [] },
    }, sections,
  };
  const skillInterface = {
    schema_version: 1, status: "experimental_non_authoritative", skill_id: "vertical",
    runtime_requirements: { required_capabilities: [], capability_ceiling: [], effect_ceiling: [], prohibited_effects: [], continuity: "ephemeral" },
    boundaries: [{ id: "mutation.scoped", kind: "mutation", semantic_section: "authority", authority_source: "authority.fixture", enforcement: "scoped_write" }],
  };
  const result = await compileSkill({
    structureSource: stringify(structure), interfaceSource: stringify(skillInterface), workspaceRoot: root,
    agentEnvironmentGraphAdapter: {
      projectRole: async () => ({
        backend: "fixture.aeg", backend_sha256: "0".repeat(64), canonical_role_match: true,
        projection: parse(projection),
      }),
    },
  });
  assert.equal(result.output.toString(), "---\nname: vertical\ndescription: Vertical proof.\n---\n\n# Vertical\n\nExact prose.\n\n## Authority\n\nMutation is scoped.\n");
  assert.deepEqual(result.ir.section_provenance.map(({ id }) => id), ["identity", "authority"]);
  assert.equal(result.ir.interface.boundaries[0].authority_source, "authority.fixture");
  assert.equal(result.ir.source_bindings[0].kind, "canonical_authority");
  assert.equal(result.ir.runtime_requirements.compiled_skill_sha256, result.ir.output_sha256);
  assert.match(result.ir.runtime_requirements.sha256, /^[0-9a-f]{64}$/);
  assert.deepEqual(result.ir.role_profile.relations.must_require, {
    "role.reviewer": ["INV-012"],
  });
  assert.deepEqual(result.ir.role_profile.relations.observation_limits, {
    "artifact.plan": "accepted consequence only",
  });

  for (const [value, message] of [
    [[], /must_require must be an object/],
    [{ "role.reviewer": [] }, /must contain at least one invariant/],
    [{ "role.reviewer": ["INV-012", "INV-012"] }, /contains duplicates/],
  ]) {
    const malformed = structuredClone(structure);
    malformed.role_profile.relations.must_require = value;
    await assert.rejects(
      compileSkill({
        structureSource: stringify(malformed),
        interfaceSource: stringify(skillInterface),
        verifySources: false,
      }),
      message,
    );
  }

  for (const [value, message] of [
    [[], /observation_limits must be an object/],
    [{ "": "accepted consequence only" }, /entity id must be nonempty text/],
    [{ "artifact.plan": "" }, /observation_limits\.artifact\.plan must be nonempty text/],
  ]) {
    const malformed = structuredClone(structure);
    malformed.role_profile.relations.observation_limits = value;
    await assert.rejects(
      compileSkill({
        structureSource: stringify(malformed),
        interfaceSource: stringify(skillInterface),
        verifySources: false,
      }),
      message,
    );
  }

  const invalid = structuredClone(skillInterface);
  invalid.boundaries[0].authority_source = "authority.unknown";
  await assert.rejects(
    compileSkill({
      structureSource: stringify(structure), interfaceSource: stringify(invalid), workspaceRoot: root,
      agentEnvironmentGraphAdapter: { projectRole: async () => { throw new Error("must not reach projection"); } },
    }),
    /unresolved canonical authority source authority\.unknown/,
  );
});

test("pinned slice-builder decomposition regenerates exact canonical bytes and AEG relations", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const structureSource = await readFile(path.join(root, "app-server/migrations/skills/slice-builder/structure.yaml"), "utf8");
  const interfaceSource = await readFile(path.join(root, "app-server/migrations/skills/slice-builder/interface.yaml"), "utf8");
  const expected = await readFile(path.join(root, "skills/slice-builder/SKILL.md"));
  const first = await compileSkill({ structureSource, interfaceSource, workspaceRoot: root });
  const second = await compileSkill({ structureSource, interfaceSource, workspaceRoot: root });
  assert.ok(first.output.equals(expected));
  assert.ok(second.output.equals(first.output));
  assert.equal(first.ir.section_provenance.length, 9);
  assert.equal(first.ir.output_sha256, "561d9bc9234af444745f0eea2add061b8f154a07d837dea319aee24c855c383e");
  assert.equal(first.ir.status, "experimental_non_authoritative");
  assert.match(first.ir.input_sha256.structure, /^[0-9a-f]{64}$/);
  assert.match(first.ir.input_sha256.interface, /^[0-9a-f]{64}$/);
  assert.equal(first.ir.role_projection.canonical_role_match, true);
  assert.deepEqual(first.ir.runtime_requirements.required_capabilities, [
    "capability.deterministic_gate",
    "capability.direct_source_observation",
    "capability.independent_review",
    "capability.repository_evidence",
    "capability.repository_mutation",
  ]);
  assert.deepEqual(first.ir.runtime_requirements.capability_ceiling, first.ir.runtime_requirements.required_capabilities);
  assert.deepEqual(first.ir.runtime_requirements.effect_ceiling, ["state.task_changes", "state.worktree"]);
  assert.equal(first.ir.role_projection.projection.role_id, "role.builder");
  assert.deepEqual(first.ir.role_projection.projection.role, first.ir.role_profile && {
    label: first.ir.role_profile.label,
    objective: first.ir.role_profile.objective,
    context_lifetime: first.ir.role_profile.context_lifetime,
    ...first.ir.role_profile.relations,
  });
  assert.ok(first.ir.role_projection.projection.invariants["INV-001"]);
  assert.ok(first.ir.role_projection.projection.mechanisms);
});

test("schema v1 rejects duplicate keys and whole-document escape hatches", async () => {
  const duplicate = "schema_version: 1\nschema_version: 1\n";
  await assert.rejects(
    compileSkill({ structureSource: duplicate, interfaceSource: duplicate, verifySources: false }),
    /Map keys must be unique/,
  );
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const valid = await readFile(path.join(root, "app-server/migrations/skills/slice-builder/structure.yaml"), "utf8");
  const invalid = `${valid}whole_document: opaque\n`;
  const skillInterface = await readFile(path.join(root, "app-server/migrations/skills/slice-builder/interface.yaml"), "utf8");
  await assert.rejects(
    compileSkill({ structureSource: invalid, interfaceSource: skillInterface, verifySources: false }),
    /unknown field whole_document/,
  );
});

test("unverified compilation cannot produce runtime-admissible requirements", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const result = await compileSkill({
    structureSource: await readFile(path.join(root, "app-server/migrations/skills/slice-builder/structure.yaml"), "utf8"),
    interfaceSource: await readFile(path.join(root, "app-server/migrations/skills/slice-builder/interface.yaml"), "utf8"),
    verifySources: false,
  });
  assert.equal(result.ir.runtime_requirements.verified_sources, false);
});

test("role projection divergence fails closed", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const structureSource = await readFile(path.join(root, "app-server/migrations/skills/slice-builder/structure.yaml"), "utf8");
  const interfaceSource = await readFile(path.join(root, "app-server/migrations/skills/slice-builder/interface.yaml"), "utf8");
  await assert.rejects(
    compileSkill({ structureSource, interfaceSource, workspaceRoot: root, agentEnvironmentGraphAdapter: {
      projectRole: async () => ({ backend: "fixture", backend_sha256: "0".repeat(64), canonical_role_match: true, projection: { role_id: "role.builder", role: {} } }),
    } }),
    /complete role projection differs from pinned generated oracle/,
  );
});

test("schema v1 requires a closed source binding kind", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const structure = parse(await readFile(path.join(root, "app-server/migrations/skills/slice-builder/structure.yaml"), "utf8"));
  const interfaceSource = await readFile(path.join(root, "app-server/migrations/skills/slice-builder/interface.yaml"), "utf8");
  delete structure.source_bindings[0].kind;
  await assert.rejects(
    compileSkill({ structureSource: stringify(structure), interfaceSource, verifySources: false }),
    /source_bindings\[0\]\.kind must be nonempty text/,
  );
  structure.source_bindings[0].kind = "projection_maybe_authority";
  await assert.rejects(
    compileSkill({ structureSource: stringify(structure), interfaceSource, verifySources: false }),
    /unsupported source binding kind projection_maybe_authority/,
  );
});

test("generated evidence cannot satisfy an interface authority reference", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const structureSource = await readFile(path.join(root, "app-server/migrations/skills/slice-builder/structure.yaml"), "utf8");
  const skillInterface = parse(await readFile(path.join(root, "app-server/migrations/skills/slice-builder/interface.yaml"), "utf8"));
  skillInterface.boundaries[0].authority_source = "evidence.role-projection";
  await assert.rejects(
    compileSkill({ structureSource, interfaceSource: stringify(skillInterface), verifySources: false }),
    /unresolved canonical authority source evidence\.role-projection/,
  );
});

test("schema v1 rejects mediation mapped to a lifecycle section", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const structureSource = await readFile(path.join(root, "app-server/migrations/skills/slice-builder/structure.yaml"), "utf8");
  const interfaceSource = await readFile(path.join(root, "app-server/migrations/skills/slice-builder/interface.yaml"), "utf8");
  const mismapped = interfaceSource.replace(
    "id: acceptance.supervisor-mediated\n    kind: mediation\n    semantic_section: implementation-turn",
    "id: acceptance.supervisor-mediated\n    kind: mediation\n    semantic_section: builder-lifetime",
  );
  await assert.rejects(
    compileSkill({ structureSource, interfaceSource: mismapped, verifySources: false }),
    /boundary kind mediation is incompatible with semantic section kind lifecycle/,
  );
});

test("source verification rejects authored frontmatter that differs from the canonical prefix", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const structure = parse(await readFile(path.join(root, "app-server/migrations/skills/slice-builder/structure.yaml"), "utf8"));
  const interfaceSource = await readFile(path.join(root, "app-server/migrations/skills/slice-builder/interface.yaml"), "utf8");
  structure.frontmatter.description = "Altered experimental description.";
  await assert.rejects(
    compileSkill({ structureSource: stringify(structure), interfaceSource, workspaceRoot: root }),
    /structure frontmatter does not match canonical source prefix/,
  );
});

test("source verification rejects an unverified gap before the first section", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const structure = parse(await readFile(path.join(root, "app-server/migrations/skills/slice-builder/structure.yaml"), "utf8"));
  const interfaceSource = await readFile(path.join(root, "app-server/migrations/skills/slice-builder/interface.yaml"), "utf8");
  structure.sections[0].source_span.start_byte += 1;
  await assert.rejects(
    compileSkill({ structureSource: stringify(structure), interfaceSource, workspaceRoot: root }),
    /first section source span must begin immediately after canonical frontmatter/,
  );
});

test("repo-search compiles byte-exactly as a role-free evidence skill", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const structureSource = await readFile(path.join(root, "app-server/migrations/skills/repo-search/structure.yaml"), "utf8");
  const interfaceSource = await readFile(path.join(root, "app-server/migrations/skills/repo-search/interface.yaml"), "utf8");
  const result = await compileSkill({ structureSource, interfaceSource, workspaceRoot: root });
  const canonical = await readFile(path.join(root, "skills/repo-search/SKILL.md"));

  assert.deepEqual(result.output, canonical);
  assert.equal(result.ir.output_sha256, digest(canonical));
  assert.equal("role_profile" in result.ir, false);
  assert.equal("role_projection" in result.ir, false);
  assert.equal("role_projection" in result.ir.input_sha256, false);
  assert.equal("role_id" in result.ir.runtime_requirements, false);
  assert.equal(result.ir.runtime_requirements.contract.kind, "skill");
  assert.deepEqual(result.ir.runtime_requirements.required_capabilities, [
    "capability.direct_source_observation",
    "capability.repository_evidence",
  ]);
  assert.deepEqual(result.ir.runtime_requirements.effect_ceiling, []);
});

test("role-free skills reject role placeholders and source drift", async () => {
  const root = path.resolve(new URL("../..", import.meta.url).pathname);
  const structure = parse(await readFile(path.join(root, "app-server/migrations/skills/repo-search/structure.yaml"), "utf8"));
  const interfaceSource = await readFile(path.join(root, "app-server/migrations/skills/repo-search/interface.yaml"), "utf8");
  structure.role_profile = {};
  await assert.rejects(
    compileSkill({ structureSource: stringify(structure), interfaceSource, verifySources: false }),
    /structure\.role_profile\.projection must be an object/,
  );

  delete structure.role_profile;
  structure.source.sha256 = "0".repeat(64);
  await assert.rejects(
    compileSkill({ structureSource: stringify(structure), interfaceSource, workspaceRoot: root }),
    /legacy source digest mismatch/,
  );
});
