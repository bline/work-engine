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

const SUPERSESSION_VALUES = new Set(["none", "partial", "full", "unknown", "not_applicable"]);
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

// KIND/PLAN evidence must come from the document's own body -- a status block's prose
// (residue_ledger, backlog_note, architectural_supersession_note, ...) describing or
// quoting a tag is not itself the tag, and must not let a document validate its own
// claim by restating it inside the very field that claim is supposed to be evidence for.
// Blanking (not deleting) the yaml blocks keeps every other character offset unchanged.
function stripYamlBlocks(text) {
  return text.replace(/```yaml\n[\s\S]*?\n```/g, (block) => block.replace(/[^\n]/g, " "));
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

// A declared audit_scope entry is either a bare string ("open-question-ledger") or a
// single-key object ({"keyword-scan": "full_document"}, {"close-read": "SS1-6"}).
// "formal-ledger" is a synonym for "open-question-ledger" seen once in the corpus
// (incremental-architecture-intake-and-seam-reconciliation.md) for a ledger discovered
// outside the usual "Open Questions" heading convention -- treated as equivalent scope
// declaration, not a distinct scope kind.
function scopeHas(auditScope, key, exactValue) {
  return (auditScope ?? []).some((entry) => {
    if (typeof entry === "string") return entry === key;
    if (entry && typeof entry === "object" && key in entry) {
      return exactValue === undefined || entry[key] === exactValue;
    }
    return false;
  });
}

function scopeDeclaresLedgerCoverage(auditScope) {
  return scopeHas(auditScope, "open-question-ledger") || scopeHas(auditScope, "formal-ledger") || scopeHas(auditScope, "keyword-scan", "full_document");
}

function scopeDeclaresPlanCoverage(auditScope) {
  return scopeHas(auditScope, "staged-plan-section") || scopeHas(auditScope, "keyword-scan", "full_document");
}

function validateDocument(relativePath, text) {
  const errors = [];
  const infos = [];
  const bodyText = stripYamlBlocks(text);

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
        if (!isNonEmptyString(entry?.scope)) {
          errors.push(`superseded_by[${index}] is missing scope, required for architectural_supersession: ${supersession}`);
        }
      });
    }
  }

  // --- Cross-field: residue/backlog must be backed by evidence in the document ------
  // KIND tags only carry ANSWERED/OPEN/MOOT/UNCHECKED (never ACTIVE -- SS4.2); PLAN tags
  // carry the fuller staged-plan set including ACTIVE/COMPLETED/SUPERSEDED (SS4.3). Only
  // PLAN's OPEN/PARTIAL/ACTIVE feed the backlog axis (SS4.3); PLAN tags never feed residue
  // -- a staged plan is never itself an architectural-ownership question.
  const residueDispositions = findKindDispositions(bodyText, "RESIDUE");
  const backlogDispositions = findKindDispositions(bodyText, "BACKLOG");
  const planDispositions = findPlanDispositions(bodyText);

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
    errors.push("backlog: present has no [KIND: BACKLOG] ... OPEN tag or [PLAN: OPEN/PARTIAL] tag anywhere in the document to support it");
  }
  if (backlog === "active" && !planDispositions.includes("ACTIVE")) {
    errors.push("backlog: active requires a [PLAN: ACTIVE] tag citing current execution evidence");
  }
  if (backlog === "unknown" && completeness !== "partial") {
    errors.push("backlog: unknown requires audit_scope_completeness: partial (an unresolved audit cannot be a completed one)");
  }
  if (backlog === "none" && backlogHasOpenSupport) {
    errors.push("backlog: none contradicts a [KIND: BACKLOG] ... OPEN tag or [PLAN: OPEN/PARTIAL] tag found in the document");
  }

  // --- Cross-field: architectural_supersession requires recorded evidence -----------
  // audit_scope_completeness tracks ledger/plan-item coverage (SS5) -- a categorically
  // different question from whether canonical views were actually read for semantic
  // coverage of this document's claims (SS3.1). Coupling supersession: unknown to
  // completeness: partial conflated those two independent checks; removed. The
  // mechanical proxy for "semantic coverage was actually checked" is that the check's
  // reasoning was recorded, not merely that a value was asserted -- so every value on
  // this axis requires a non-empty architectural_supersession_note explaining what was
  // compared against what.
  if (!isNonEmptyString(status.architectural_supersession_note)) {
    errors.push(`architectural_supersession: ${JSON.stringify(supersession)} requires a non-empty architectural_supersession_note recording what semantic-coverage check was performed (or, for unknown, what was not)`);
  }

  // --- Cross-field: audit_scope_completeness is scope-relative (SS5.2), not global --
  // "complete" means every source actually NAMED in audit_scope has no UNCHECKED item --
  // a document that never declared staged-plan-section (and never keyword-scanned the
  // full document) is not made incomplete by an UNCHECKED tag inside some section it
  // never claimed to have checked; conversely, a document IS incomplete if it carries
  // ledger/plan tags of a kind its own declared audit_scope does not cover at all -- that
  // is silence being misrepresented as a checked scope (SS5.1's own "silence must not read
  // as clean" rule, applied to the scope declaration itself, not just to residue/backlog).
  const auditScope = status.audit_scope;
  const ledgerScopeDeclared = scopeDeclaresLedgerCoverage(auditScope);
  const planScopeDeclared = scopeDeclaresPlanCoverage(auditScope);
  const hasLedgerTags = residueDispositions.length > 0 || backlogDispositions.length > 0;
  const hasPlanTags = planDispositions.length > 0;

  if (hasLedgerTags && !ledgerScopeDeclared) {
    errors.push("document has [KIND: RESIDUE/BACKLOG] tags but audit_scope declares neither open-question-ledger nor keyword-scan: full_document");
  }
  if (hasPlanTags && !planScopeDeclared) {
    errors.push("document has [PLAN: ...] tags but audit_scope declares neither staged-plan-section nor keyword-scan: full_document");
  }

  const ledgerUnchecked = ledgerScopeDeclared && (residueDispositions.includes("UNCHECKED") || backlogDispositions.includes("UNCHECKED"));
  const planUnchecked = planScopeDeclared && planDispositions.includes("UNCHECKED");
  if (completeness === "complete" && (ledgerUnchecked || planUnchecked)) {
    errors.push("audit_scope_completeness: complete but a source named in audit_scope (ledger and/or staged-plan section) still carries an UNCHECKED tag");
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
  if ((supersession === "full" || supersession === "not_applicable") && residue === "none" && backlog === "none") {
    infos.push(`retirement-candidate: architectural_supersession: ${supersession} + residue: none + backlog: none`);
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
    architectural_supersession: { none: 0, partial: 0, full: 0, unknown: 0, not_applicable: 0 },
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
