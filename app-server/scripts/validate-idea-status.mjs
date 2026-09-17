#!/usr/bin/env node
// Deterministic validator/indexer for app-server/ideas/pending/idea-status-grammar.md.
//
// This parses the idea_status / idea_provenance YAML blocks each pending idea document
// carries and mechanically enforces the grammar's own derivation rules -- it does not
// re-judge any semantic classification (that is a per-document, human/model job), only
// whether the recorded values are internally consistent and are actually backed by the
// evidence the grammar requires them to be backed by.

import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "yaml";

const GRAMMAR_FILE = "idea-status-grammar.md";

const SUPERSESSION_VALUES = new Set(["none", "partial", "full"]);
const RESIDUE_VALUES = new Set(["none", "present", "unknown"]);
const BACKLOG_VALUES = new Set(["none", "present", "active", "unknown"]);
const COMPLETENESS_VALUES = new Set(["complete", "partial"]);
const PROVENANCE_ORIGINS = new Set(["synthesized_from_raw_material", "direct_capture", "operator_authored"]);
const LEDGER_DISPOSITIONS = ["ANSWERED", "OPEN", "MOOT", "UNCHECKED"];
const PLAN_DISPOSITIONS = ["OPEN", "PARTIAL", "ACTIVE", "COMPLETED", "SUPERSEDED", "MOOT", "UNCHECKED"];

function findYamlBlocks(text) {
  const blocks = [];
  const re = /```yaml\n([\s\S]*?)\n```/g;
  let match;
  while ((match = re.exec(text))) blocks.push(match[1]);
  return blocks;
}

// Finds every "KIND: <kind>" tag (bracketed `[KIND: X] [DISPOSITION ...]` or prose
// `KIND: X, DISPOSITION`) and returns the disposition word immediately attached to it.
function findKindDispositions(text, kind) {
  const found = [];
  const bracket = new RegExp(`\\[KIND:\\s*${kind}[^\\]]*\\]\\s*\\[\\s*(${LEDGER_DISPOSITIONS.join("|")})\\b`, "g");
  const prose = new RegExp(`KIND:\\s*${kind}\\s*,\\s*(${LEDGER_DISPOSITIONS.join("|")})\\b`, "g");
  let match;
  while ((match = bracket.exec(text))) found.push(match[1]);
  while ((match = prose.exec(text))) found.push(match[1]);
  return found;
}

function findPlanDispositions(text) {
  const found = [];
  const re = new RegExp(`\\[PLAN:\\s*(${PLAN_DISPOSITIONS.join("|")})\\b`, "g");
  let match;
  while ((match = re.exec(text))) found.push(match[1]);
  return found;
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim() !== "";
}

function validateDocument(relativePath, text) {
  const errors = [];
  const infos = [];

  const statusBlocks = [];
  const provenanceBlocks = [];
  for (const raw of findYamlBlocks(text)) {
    let parsed;
    try {
      parsed = parseYaml(raw);
    } catch (error) {
      errors.push(`unparseable yaml block: ${error.message}`);
      continue;
    }
    if (parsed && typeof parsed === "object" && "idea_status" in parsed) statusBlocks.push(parsed.idea_status);
    if (parsed && typeof parsed === "object" && "idea_provenance" in parsed) provenanceBlocks.push(parsed.idea_provenance);
  }

  if (statusBlocks.length === 0 && provenanceBlocks.length === 0) {
    return { path: relativePath, unaudited: true, errors, infos, status: null };
  }
  if (statusBlocks.length === 0) errors.push("has idea_provenance but no idea_status block");
  if (statusBlocks.length > 1) errors.push(`has ${statusBlocks.length} idea_status blocks, expected exactly 1`);
  if (provenanceBlocks.length === 0) errors.push("has idea_status but no idea_provenance block");
  if (provenanceBlocks.length > 1) errors.push(`has ${provenanceBlocks.length} idea_provenance blocks, expected exactly 1`);

  const status = statusBlocks[0] ?? {};
  const provenance = provenanceBlocks[0] ?? {};

  // --- Enum / required-field checks -------------------------------------------------
  const supersession = status.architectural_supersession;
  if (!SUPERSESSION_VALUES.has(supersession)) {
    errors.push(`architectural_supersession is missing or invalid: ${JSON.stringify(supersession)}`);
  }
  const residue = status.residue;
  if (!RESIDUE_VALUES.has(residue)) errors.push(`residue is missing or invalid: ${JSON.stringify(residue)}`);
  const backlog = status.backlog;
  if (!BACKLOG_VALUES.has(backlog)) errors.push(`backlog is missing or invalid: ${JSON.stringify(backlog)}`);
  const completeness = status.audit_scope_completeness;
  if (!COMPLETENESS_VALUES.has(completeness)) {
    errors.push(`audit_scope_completeness is missing or invalid: ${JSON.stringify(completeness)}`);
  }
  if (!Array.isArray(status.audit_scope) || status.audit_scope.length === 0) {
    errors.push("audit_scope is missing or empty");
  }
  if (!isNonEmptyString(status.status_as_of)) errors.push("status_as_of is missing");

  // --- Cross-field: supersession -> superseded_by ------------------------------------
  if (supersession === "partial" || supersession === "full") {
    if (!Array.isArray(status.superseded_by) || status.superseded_by.length === 0) {
      errors.push(`architectural_supersession: ${supersession} requires a non-empty superseded_by list`);
    } else {
      status.superseded_by.forEach((entry, index) => {
        if (!entry || !isNonEmptyString(entry.view)) {
          errors.push(`superseded_by[${index}] is missing a view`);
        }
        if (supersession === "partial" && !isNonEmptyString(entry?.scope)) {
          errors.push(`superseded_by[${index}] is missing scope, required for architectural_supersession: partial`);
        }
      });
    }
  }

  // --- Cross-field: residue/backlog must be backed by evidence in the document ------
  const residueDispositions = findKindDispositions(text, "RESIDUE");
  const backlogDispositions = findKindDispositions(text, "BACKLOG");
  const planDispositions = findPlanDispositions(text);

  if (residue === "present" && !residueDispositions.includes("OPEN")) {
    errors.push("residue: present has no [KIND: RESIDUE] ... OPEN tag anywhere in the document to support it");
  }
  if (residue === "unknown" && completeness !== "partial") {
    errors.push("residue: unknown requires audit_scope_completeness: partial (an unresolved audit cannot be a completed one)");
  }
  if (residue === "none" && residueDispositions.includes("OPEN")) {
    errors.push("residue: none contradicts a [KIND: RESIDUE] ... OPEN tag found in the document");
  }

  const backlogHasOpenSupport = backlogDispositions.includes("OPEN") || planDispositions.includes("OPEN") || planDispositions.includes("PARTIAL");
  if (backlog === "present" && !backlogHasOpenSupport) {
    errors.push("backlog: present has no [KIND: BACKLOG] ... OPEN or [PLAN: OPEN/PARTIAL] tag anywhere in the document to support it");
  }
  if (backlog === "active" && !planDispositions.includes("ACTIVE")) {
    errors.push("backlog: active requires a [PLAN: ACTIVE] tag citing current execution evidence");
  }
  if (backlog === "unknown" && completeness !== "partial") {
    errors.push("backlog: unknown requires audit_scope_completeness: partial (an unresolved audit cannot be a completed one)");
  }
  if (backlog === "none" && backlogHasOpenSupport) {
    errors.push("backlog: none contradicts a [KIND: BACKLOG] ... OPEN or [PLAN: OPEN/PARTIAL] tag found in the document");
  }

  // --- Cross-field: completeness: complete requires zero UNCHECKED in scope --------
  const anyUnchecked = residueDispositions.includes("UNCHECKED") || backlogDispositions.includes("UNCHECKED") || planDispositions.includes("UNCHECKED");
  if (completeness === "complete" && anyUnchecked) {
    errors.push("audit_scope_completeness: complete but an UNCHECKED tag exists in the document");
  }

  // --- Provenance -------------------------------------------------------------------
  const origin = provenance.origin;
  if (isNonEmptyString(origin)) {
    if (!PROVENANCE_ORIGINS.has(origin) && !origin.startsWith("derived_from:")) {
      errors.push(`idea_provenance.origin is not a recognized value: ${JSON.stringify(origin)}`);
    }
  } else if (provenanceBlocks.length > 0) {
    errors.push("idea_provenance.origin is missing");
  }
  if ("canonical_source" in provenance && typeof provenance.canonical_source !== "boolean") {
    errors.push(`idea_provenance.canonical_source must be true/false, got ${JSON.stringify(provenance.canonical_source)}`);
  }

  // --- Informational flags (not errors) ----------------------------------------------
  if (supersession === "full" && residue === "none" && backlog === "none") {
    infos.push("retirement-candidate: architectural_supersession: full + residue: none + backlog: none");
  }

  return { path: relativePath, unaudited: false, errors, infos, status: { supersession, residue, backlog, completeness } };
}

async function collectMarkdownFiles(root) {
  const out = [];
  async function walk(dir) {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      if (entry.isDirectory()) {
        await walk(path.join(dir, entry.name));
      } else if (entry.isFile() && entry.name.endsWith(".md") && entry.name !== GRAMMAR_FILE) {
        out.push(path.join(dir, entry.name));
      }
    }
  }
  await walk(root);
  return out.sort();
}

function tally(results) {
  const counts = {
    architectural_supersession: { none: 0, partial: 0, full: 0 },
    residue: { none: 0, present: 0, unknown: 0 },
    backlog: { none: 0, present: 0, active: 0, unknown: 0 },
    audit_scope_completeness: { complete: 0, partial: 0 },
  };
  let audited = 0;
  let unaudited = 0;
  for (const result of results) {
    if (result.unaudited) {
      unaudited += 1;
      continue;
    }
    audited += 1;
    if (result.status.supersession in counts.architectural_supersession) counts.architectural_supersession[result.status.supersession] += 1;
    if (result.status.residue in counts.residue) counts.residue[result.status.residue] += 1;
    if (result.status.backlog in counts.backlog) counts.backlog[result.status.backlog] += 1;
    if (result.status.completeness in counts.audit_scope_completeness) counts.audit_scope_completeness[result.status.completeness] += 1;
  }
  return { audited, unaudited, counts };
}

async function run(root) {
  const files = await collectMarkdownFiles(root);
  const results = [];
  for (const file of files) {
    const text = await readFile(file, "utf8");
    results.push(validateDocument(path.relative(root, file), text));
  }
  return { results, summary: tally(results) };
}

function printReport({ results, summary }) {
  const withErrors = results.filter((result) => result.errors.length > 0);
  const withInfo = results.filter((result) => result.infos.length > 0);
  const unaudited = results.filter((result) => result.unaudited);

  for (const result of withErrors) {
    process.stdout.write(`ERROR ${result.path}\n`);
    for (const error of result.errors) process.stdout.write(`  - ${error}\n`);
  }
  for (const result of withInfo) {
    process.stdout.write(`INFO  ${result.path}\n`);
    for (const info of result.infos) process.stdout.write(`  - ${info}\n`);
  }
  if (unaudited.length > 0) {
    process.stdout.write(`UNAUDITED (${unaudited.length}): not yet run through idea-status-grammar.md, no idea_status block -- not an error\n`);
    for (const result of unaudited) process.stdout.write(`  - ${result.path}\n`);
  }

  process.stdout.write("\n=== Corpus summary ===\n");
  process.stdout.write(`audited: ${summary.audited}    unaudited: ${summary.unaudited}\n`);
  for (const [axis, values] of Object.entries(summary.counts)) {
    process.stdout.write(`${axis}: ${Object.entries(values).map(([k, v]) => `${k}=${v}`).join(" ")}\n`);
  }
  process.stdout.write(`\ndocuments with errors: ${withErrors.length}\n`);

  return withErrors.length === 0;
}

async function main(argv) {
  const rootFlag = argv.indexOf("--root");
  const jsonFlag = argv.includes("--json");
  const root = path.resolve(rootFlag < 0 ? path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "ideas", "pending") : argv[rootFlag + 1]);
  const report = await run(root);
  if (jsonFlag) {
    process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
    process.exitCode = report.results.some((result) => result.errors.length > 0) ? 1 : 0;
    return;
  }
  const clean = printReport(report);
  process.exitCode = clean ? 0 : 1;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  main(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${error.stack ?? error.message}\n`);
    process.exitCode = 1;
  });
}

export { validateDocument, collectMarkdownFiles, tally, run };
