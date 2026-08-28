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
      bound_by: ["INV-001"], may_invoke: [], may_observe: [], may_mutate: [], owns: [],
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
    authority_sources: [{ id: "authority.fixture", path: "authority.txt", sha256: digest(authority), role: "fixture authority" }],
    role_profile: {
      role_id: "role.builder", projection: { path: "projection.yaml", sha256: digest(projection) },
      relations: { bound_by: ["INV-001"], may_invoke: [], may_observe: [], may_mutate: [], owns: [], consumes: [], emits: [], mediated_transitions: [], forbidden_from: [] },
    }, sections,
  };
  const skillInterface = {
    schema_version: 1, status: "experimental_non_authoritative", skill_id: "vertical",
    boundaries: [{ id: "mutation.scoped", kind: "mutation", semantic_section: "authority", authority_source: "authority.fixture", enforcement: "scoped_write" }],
  };
  const result = await compileSkill({ structureSource: stringify(structure), interfaceSource: stringify(skillInterface), workspaceRoot: root });
  assert.equal(result.output.toString(), "---\nname: vertical\ndescription: Vertical proof.\n---\n\n# Vertical\n\nExact prose.\n\n## Authority\n\nMutation is scoped.\n");
  assert.deepEqual(result.ir.section_provenance.map(({ id }) => id), ["identity", "authority"]);
  assert.equal(result.ir.interface.boundaries[0].authority_source, "authority.fixture");

  const invalid = structuredClone(skillInterface);
  invalid.boundaries[0].authority_source = "authority.unknown";
  await assert.rejects(
    compileSkill({ structureSource: stringify(structure), interfaceSource: stringify(invalid), workspaceRoot: root }),
    /unresolved authority source authority\.unknown/,
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
  assert.equal(first.ir.output_sha256, "d3eb81d00b8b517a7f552e9bcbd300773e49f30c035b2f016115189d5f6fd8c0");
  assert.equal(first.ir.status, "experimental_non_authoritative");
  assert.match(first.ir.input_sha256.structure, /^[0-9a-f]{64}$/);
  assert.match(first.ir.input_sha256.interface, /^[0-9a-f]{64}$/);
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
