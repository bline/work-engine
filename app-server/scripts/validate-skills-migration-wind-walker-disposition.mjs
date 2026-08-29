#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SHA256 = /^[a-f0-9]{64}$/;
const RESIDUAL_IDS = new Set([
  "context-is-not-canonical-state", "continuation-independence",
  "human-interaction-preservation", "fail-closed-rehydration",
]);
const RETIREMENT_IDS = new Set([
  "server-lifecycle-coverage", "effective-instruction-parity", "consumer-closure",
  "compatibility-evidence", "human-ownership-transition",
]);

function fail(message) { throw new Error(message); }
function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") fail(`${label} must be non-empty`);
}
function exact(value, fields, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) fail(`${label} must be an object`);
  if (JSON.stringify(Object.keys(value).sort()) !== JSON.stringify([...fields].sort())) {
    fail(`${label} fields are not schema-exact`);
  }
}
function clauses(value, expected, label) {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  const ids = new Set();
  value.forEach((entry, index) => {
    exact(entry, ["id", "statement", "cause", "failure_mode"], `${label}[${index}]`);
    for (const field of ["id", "statement", "cause", "failure_mode"]) {
      text(entry[field], `${label}[${index}].${field}`);
    }
    ids.add(entry.id);
  });
  if (ids.size !== value.length || ids.size !== expected.size
      || [...expected].some((id) => !ids.has(id))) fail(`${label} does not close its required identities`);
}

export async function validateWindWalkerDisposition({ repository, record }) {
  exact(record, [
    "schema_version", "record_id", "authority_notice", "classification_state",
    "authority_status", "subject", "target_owner", "legacy_status",
    "residual_projection", "retirement", "compatibility",
  ], "record");
  if (record.schema_version !== 1 || record.record_id !== "wind-walker-disposition-v1") {
    fail("unsupported Wind Walker disposition identity");
  }
  if (!/does not grant, transfer, or accept/i.test(record.authority_notice)) {
    fail("authority_notice must deny authority effects");
  }
  if (record.classification_state !== "provisional" || record.authority_status !== "not_accepted") {
    fail("Wind Walker ownership cannot be accepted by migration evidence");
  }
  exact(record.subject, ["path", "sha256"], "subject");
  if (record.subject.path !== "skills/wind-walker/SKILL.md" || !SHA256.test(record.subject.sha256)) {
    fail("subject must bind the exact legacy Wind Walker skill");
  }
  const bytes = await readFile(path.resolve(repository, record.subject.path));
  if (createHash("sha256").update(bytes).digest("hex") !== record.subject.sha256) {
    fail("Wind Walker subject digest is stale");
  }
  if (record.target_owner !== "app-server semantic context lifecycle service"
      || record.legacy_status !== "retained_compatibility") {
    fail("legacy ownership must remain retained compatibility state");
  }
  exact(record.residual_projection, ["status", "owner", "instructions"], "residual_projection");
  if (record.residual_projection.status !== "required"
      || record.residual_projection.owner !== "canonical role and lifecycle contracts") {
    fail("residual projection ownership is invalid");
  }
  clauses(record.residual_projection.instructions, RESIDUAL_IDS, "residual_projection.instructions");
  exact(record.retirement, ["status", "prerequisites"], "retirement");
  if (record.retirement.status !== "blocked_pending_authority") {
    fail("legacy retirement must remain blocked pending authority");
  }
  clauses(record.retirement.prerequisites, RETIREMENT_IDS, "retirement.prerequisites");
  exact(record.compatibility, ["required", "evidence", "difference_authority"], "compatibility");
  if (record.compatibility.required !== true
      || record.compatibility.difference_authority !== "human-owned explicit ownership transition"
      || !Array.isArray(record.compatibility.evidence)
      || !record.compatibility.evidence.includes("skills/wind-walker/SKILL.md")
      || !record.compatibility.evidence.includes("app-server/migrations/skills/instruction-review-regression-v1.json")) {
    fail("compatibility evidence and difference authority are incomplete");
  }
  return Object.freeze({
    record_id: record.record_id,
    subject_sha256: record.subject.sha256,
    residual_instructions: record.residual_projection.instructions.length,
    retirement_prerequisites: record.retirement.prerequisites.length,
  });
}

async function main(argv) {
  const repositoryFlag = argv.indexOf("--repository");
  const recordFlag = argv.indexOf("--record");
  if (repositoryFlag < 0 || recordFlag < 0) fail("usage: --repository <path> --record <path>");
  const repository = path.resolve(argv[repositoryFlag + 1]);
  const record = JSON.parse(await readFile(path.resolve(argv[recordFlag + 1]), "utf8"));
  process.stdout.write(`${JSON.stringify(await validateWindWalkerDisposition({ repository, record }))}\n`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  main(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
