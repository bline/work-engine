#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SHA256 = /^[a-f0-9]{64}$/;
const KINDS = new Set(["structural_command", "route_preference", "causally_required_sequence", "bundled", "non_normative", "remediation", "reviewer_lifecycle"]);
const APPLICABILITY = new Set(["applicable", "omitted"]);
const PRECEDENCE = new Set(["ordinary", "role", "skill", "conflicting_higher_layer", "unresolved_conflict"]);
const OMISSION = new Set(["none", "declared_without_closure", "accepted_closed_no_replacement_path"]);
const FINDING_CONSEQUENCES = new Set(["none", "missing_causal_exposure", "precedence_conflict", "unsafe_omission", "bundled_clause", "layer_conflict"]);

function fail(message) { throw new Error(message); }
function text(value, label) { if (typeof value !== "string" || value.trim() === "") fail(`${label} must be a non-empty string`); }
function array(value, label) { if (!Array.isArray(value)) fail(`${label} must be an array`); }
function exactKeys(value, keys, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) fail(`${label} must be an object`);
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) fail(`${label} fields must be exactly [${expected.join(", ")}]`);
}
export function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}
export function digest(value) { return createHash("sha256").update(Buffer.isBuffer(value) ? value : canonical(value)).digest("hex"); }

function validateCase(entry, index) {
  const label = `cases[${index}]`;
  const keys = ["id", "instruction_kind", "command", "source_catalog", "effective_instructions", "relations", "expected", "limitations"];
  if (entry.pair_id !== undefined || entry.variant !== undefined) keys.push("pair_id", "variant");
  exactKeys(entry, keys, label);
  text(entry.id, `${label}.id`);
  if (!KINDS.has(entry.instruction_kind)) fail(`${label}.instruction_kind is unsupported`);
  text(entry.command, `${label}.command`);
  exactKeys(entry.source_catalog, ["reason", "failure_mode"], `${label}.source_catalog`);
  text(entry.source_catalog.reason, `${label}.source_catalog.reason`);
  text(entry.source_catalog.failure_mode, `${label}.source_catalog.failure_mode`);
  array(entry.effective_instructions, `${label}.effective_instructions`);
  entry.effective_instructions.forEach((value, item) => text(value, `${label}.effective_instructions[${item}]`));
  exactKeys(entry.relations, ["source_traceable", "reviewer_can_reconstruct", "reason_loaded", "failure_mode_loaded", "precedence", "conditional_omission"], `${label}.relations`);
  for (const key of ["source_traceable", "reviewer_can_reconstruct", "reason_loaded", "failure_mode_loaded"]) {
    if (typeof entry.relations[key] !== "boolean") fail(`${label}.relations.${key} must be boolean`);
  }
  if (!PRECEDENCE.has(entry.relations.precedence)) fail(`${label}.relations.precedence is unsupported`);
  if (!OMISSION.has(entry.relations.conditional_omission)) fail(`${label}.relations.conditional_omission is unsupported`);
  exactKeys(entry.expected, ["applicability", "finding_consequence", "missing_causal_exposure_finding", "protected_distinction"], `${label}.expected`);
  if (!APPLICABILITY.has(entry.expected.applicability)) fail(`${label}.expected.applicability is unsupported`);
  if (!FINDING_CONSEQUENCES.has(entry.expected.finding_consequence)) fail(`${label}.expected.finding_consequence is unsupported`);
  if (typeof entry.expected.missing_causal_exposure_finding !== "boolean") fail(`${label}.expected.missing_causal_exposure_finding must be boolean`);
  text(entry.expected.protected_distinction, `${label}.expected.protected_distinction`);
  array(entry.limitations, `${label}.limitations`);
  if (entry.limitations.length === 0) fail(`${label}.limitations must not be empty`);
  entry.limitations.forEach((value, item) => text(value, `${label}.limitations[${item}]`));
}

function validateVerticalPair(cases) {
  const pair = cases.filter((entry) => entry.pair_id === "vertical-causal-exposure");
  if (pair.length !== 2) fail("vertical-causal-exposure must contain exactly two cases");
  const missing = pair.find((entry) => entry.variant === "missing");
  const loaded = pair.find((entry) => entry.variant === "loaded");
  if (!missing || !loaded) fail("vertical pair must contain missing and loaded variants");
  for (const key of ["instruction_kind", "command", "source_catalog"]) {
    if (canonical(missing[key]) !== canonical(loaded[key])) fail(`vertical pair differs unexpectedly at ${key}`);
  }
  const stableRelations = ["source_traceable", "reviewer_can_reconstruct", "precedence", "conditional_omission"];
  for (const key of stableRelations) if (missing.relations[key] !== loaded.relations[key]) fail(`vertical pair differs unexpectedly at relations.${key}`);
  if (missing.relations.reason_loaded || missing.relations.failure_mode_loaded) fail("missing vertical variant cannot load causal content");
  if (!loaded.relations.reason_loaded || !loaded.relations.failure_mode_loaded) fail("loaded vertical variant must load causal content");
  if (!missing.expected.missing_causal_exposure_finding || loaded.expected.missing_causal_exposure_finding) fail("vertical pair expected finding polarity is invalid");
}

export function validateCorpus(corpus) {
  exactKeys(corpus, ["schema_version", "corpus_id", "authority_notice", "contract_sources", "bootstrap_contract", "cases"], "corpus");
  if (corpus.schema_version !== 1) fail("schema_version must be 1");
  text(corpus.corpus_id, "corpus_id");
  text(corpus.authority_notice, "authority_notice");
  if (!/does not grant, transfer, or accept/i.test(corpus.authority_notice)) fail("authority_notice must deny authority effects");
  array(corpus.contract_sources, "contract_sources");
  for (const [index, source] of corpus.contract_sources.entries()) {
    exactKeys(source, ["path", "role"], `contract_sources[${index}]`);
    text(source.path, `contract_sources[${index}].path`);
    if (path.isAbsolute(source.path) || source.path.includes("..")) fail(`contract_sources[${index}].path must be repository-relative`);
    text(source.role, `contract_sources[${index}].role`);
  }
  exactKeys(corpus.bootstrap_contract, ["owner", "reviewer_independence", "mutation", "authority", "required_checks"], "bootstrap_contract");
  for (const key of ["owner", "reviewer_independence", "mutation", "authority"]) text(corpus.bootstrap_contract[key], `bootstrap_contract.${key}`);
  array(corpus.bootstrap_contract.required_checks, "bootstrap_contract.required_checks");
  array(corpus.cases, "cases");
  const ids = corpus.cases.map((entry) => entry.id);
  if (new Set(ids).size !== ids.length) fail("case ids must be unique");
  corpus.cases.forEach(validateCase);
  validateVerticalPair(corpus.cases);
  return Object.freeze({ corpus_id: corpus.corpus_id, cases: corpus.cases.length, corpus_digest: digest(corpus) });
}

export function validateBehavioralResults(corpus, results) {
  exactKeys(results, ["schema_version", "result_id", "authority_notice", "corpus", "configuration", "freshness", "cases", "limitations"], "results");
  if (results.schema_version !== 1) fail("results.schema_version must be 1");
  text(results.result_id, "results.result_id");
  text(results.authority_notice, "results.authority_notice");
  exactKeys(results.corpus, ["id", "sha256"], "results.corpus");
  if (results.corpus.id !== corpus.corpus_id || results.corpus.sha256 !== digest(corpus)) fail("results corpus identity is stale");
  exactKeys(results.configuration, ["provider", "model", "effort", "prompt_contract", "temperature"], "results.configuration");
  for (const key of ["provider", "model", "effort", "prompt_contract"]) text(results.configuration[key], `results.configuration.${key}`);
  if (results.configuration.temperature !== null && typeof results.configuration.temperature !== "number") fail("results.configuration.temperature must be number or null");
  exactKeys(results.freshness, ["fresh_context", "builder_context_inherited", "session_persisted"], "results.freshness");
  if (results.freshness.fresh_context !== true || results.freshness.builder_context_inherited !== false || results.freshness.session_persisted !== false) fail("behavioral result must record a fresh disposable context");
  array(results.cases, "results.cases");
  if (results.cases.length !== corpus.cases.length) fail("results must cover every corpus case");
  const expected = new Map(corpus.cases.map((entry) => [entry.id, entry.expected]));
  const seen = new Set();
  for (const [index, entry] of results.cases.entries()) {
    exactKeys(entry, ["case_id", "applicability", "finding_consequence", "missing_causal_exposure_finding", "consequence", "limitations"], `results.cases[${index}]`);
    if (!expected.has(entry.case_id) || seen.has(entry.case_id)) fail(`results.cases[${index}].case_id is missing or duplicated`);
    seen.add(entry.case_id);
    if (entry.applicability !== expected.get(entry.case_id).applicability) fail(`${entry.case_id} applicability contradicts the revision-bound expected consequence`);
    if (entry.finding_consequence !== expected.get(entry.case_id).finding_consequence) fail(`${entry.case_id} finding consequence contradicts the revision-bound expected consequence`);
    if (entry.missing_causal_exposure_finding !== expected.get(entry.case_id).missing_causal_exposure_finding) fail(`${entry.case_id} causal-exposure result contradicts the revision-bound expected consequence`);
    text(entry.consequence, `results.cases[${index}].consequence`);
    array(entry.limitations, `results.cases[${index}].limitations`);
  }
  array(results.limitations, "results.limitations");
  if (results.limitations.length === 0) fail("results.limitations must not be empty");
  return Object.freeze({ cases: seen.size, results_digest: digest(results) });
}

async function main(argv) {
  const corpusFlag = argv.indexOf("--corpus");
  const resultsFlag = argv.indexOf("--results");
  if (corpusFlag < 0) fail("usage: --corpus <path> [--results <path>]");
  const corpus = JSON.parse(await readFile(path.resolve(argv[corpusFlag + 1]), "utf8"));
  const output = { corpus: validateCorpus(corpus) };
  if (resultsFlag >= 0) output.results = validateBehavioralResults(corpus, JSON.parse(await readFile(path.resolve(argv[resultsFlag + 1]), "utf8")));
  process.stdout.write(`${JSON.stringify(output)}\n`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  main(process.argv.slice(2)).catch((error) => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
}
