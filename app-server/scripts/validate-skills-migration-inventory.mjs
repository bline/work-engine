#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SHA256 = /^[a-f0-9]{64}$/;
const DISPOSITIONS = new Set(["retain", "relocate", "rewrite", "compatibility", "historical", "remove"]);
const SHAPES = new Set(["agent_skill", "role_like", "deterministic_capability", "service_state", "provider_adapter", "experiment", "projection", "mixed"]);
const GENERIC_PROTECTED_DISTINCTION = /^Preserve the observable consequence exercised by .+ while its owning boundary is classified\.$/;

function fail(message) { throw new Error(message); }
function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") fail(`${label} must be a non-empty string`);
}
function requireArray(value, label) {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
}
function digest(bytes) { return createHash("sha256").update(bytes).digest("hex"); }
const execute = promisify(execFile);
async function git(repository, args, options = {}) {
  try {
    return (await execute("git", ["-C", repository, ...args], { encoding: options.encoding ?? "utf8", maxBuffer: 32 * 1024 * 1024 })).stdout;
  } catch (error) {
    fail(`Git inventory source failed: ${error.stderr?.toString().trim() || error.message}`);
  }
}
async function resolveTree(repository, treeish) {
  requireText(treeish, "treeish");
  return (await git(repository, ["rev-parse", "--verify", `${treeish}^{tree}`])).trim();
}
async function treeFiles(repository, treeOid, prefix = "skills/") {
  const output = await git(repository, ["ls-tree", "-r", "-z", "--name-only", treeOid, "--", prefix]);
  return output.split("\0").filter(Boolean).sort();
}
async function treeBlob(repository, treeOid, relativePath, label) {
  const line = (await git(repository, ["ls-tree", treeOid, "--", relativePath])).trim();
  const match = /^(\d+) (\w+) ([a-f0-9]+)\t/.exec(line);
  if (!match) fail(`${label} declared path absent from Git tree: ${relativePath}`);
  if (match[1] === "120000" || match[2] !== "blob") fail(`${label} evidence must be a non-symlink Git blob: ${relativePath}`);
  return git(repository, ["cat-file", "blob", match[3]], { encoding: "buffer" });
}

function validateEvidenceEntry(entry, label, expectedPrefix) {
  if (!entry || typeof entry !== "object" || Array.isArray(entry)) fail(`${label} must be an object`);
  requireText(entry.path, `${label}.path`);
  if (path.isAbsolute(entry.path) || entry.path.includes("..")) fail(`${label}.path must be repository-relative`);
  if (expectedPrefix && !entry.path.startsWith(expectedPrefix)) fail(`${label}.path must remain under ${expectedPrefix}`);
  if (!SHA256.test(entry.sha256)) fail(`${label}.sha256 must be a lowercase SHA-256 digest`);
}

export async function validateInventory({ repository, inventory, treeish = "HEAD" }) {
  if (inventory?.schema_version !== 1) fail("inventory schema_version must be 1");
  requireText(inventory.inventory_id, "inventory_id");
  requireText(inventory.evidence_only_notice, "evidence_only_notice");
  if (!/does not (grant|change|transfer).*authority/i.test(inventory.evidence_only_notice)) {
    fail("evidence_only_notice must explicitly deny an authority transition");
  }
  if (inventory.classification_state !== "provisional") fail("classification_state must be provisional");
  if (inventory.authority_status !== "not_accepted") fail("authority_status must be not_accepted");
  requireText(inventory.baseline?.git_head, "baseline.git_head");
  requireText(inventory.baseline?.worktree_observation, "baseline.worktree_observation");
  requireArray(inventory.graph_limitations, "graph_limitations");
  requireArray(inventory.packages, "packages");

  const tree_oid = await resolveTree(repository, treeish);
  const gitPaths = await treeFiles(repository, tree_oid);
  const liveNames = gitPaths.filter((entry) => /^skills\/[^/]+\/SKILL\.md$/.test(entry)).map((entry) => entry.split("/")[1]).sort();
  const names = inventory.packages.map((record) => record.name);
  if (new Set(names).size !== names.length) fail("inventory contains duplicate package records");
  if (JSON.stringify([...names].sort()) !== JSON.stringify(liveNames)) {
    fail(`closed inventory mismatch: expected [${liveNames.join(", ")}], received [${[...names].sort().join(", ")}]`);
  }

  let verifiedFiles = 0;
  for (const record of inventory.packages) {
    const label = `package ${record.name}`;
    requireText(record.name, `${label}.name`);
    if (record.classification_state !== "provisional" || record.authority_status !== "not_accepted") {
      fail(`${label} classification cannot be accepted without an authority transition`);
    }
    requireText(record.responsibility, `${label}.responsibility`);
    requireArray(record.shapes, `${label}.shapes`);
    if (record.shapes.length === 0 || record.shapes.some((shape) => !SHAPES.has(shape))) fail(`${label}.shapes is unsupported`);
    requireText(record.candidate_destination, `${label}.candidate_destination`);
    requireText(record.compatibility_posture, `${label}.compatibility_posture`);
    requireText(record.reviewer_applicability, `${label}.reviewer_applicability`);
    requireArray(record.consumers, `${label}.consumers`);
    requireArray(record.server_service_dependencies, `${label}.server_service_dependencies`);
    requireArray(record.unresolved_authority_questions, `${label}.unresolved_authority_questions`);
    requireArray(record.evidence_sources, `${label}.evidence_sources`);
    if (record.evidence_sources.length === 0) fail(`${label} must cite evidence sources`);

    const prefix = `skills/${record.name}/`;
    validateEvidenceEntry(record.source, `${label}.source`, prefix);
    if (record.source.path !== `${prefix}SKILL.md`) fail(`${label}.source must identify its exact SKILL.md`);
    requireArray(record.artifacts, `${label}.artifacts`);
    requireArray(record.tests, `${label}.tests`);
    const entries = [record.source, ...record.artifacts, ...record.tests];
    const entryPaths = entries.map((entry) => entry.path);
    if (new Set(entryPaths).size !== entryPaths.length) fail(`${label} contains duplicate evidence paths`);

    for (const [index, entry] of record.artifacts.entries()) validateEvidenceEntry(entry, `${label}.artifacts[${index}]`, prefix);
    for (const [index, test] of record.tests.entries()) {
      validateEvidenceEntry(test, `${label}.tests[${index}]`, prefix);
      requireText(test.protected_distinction, `${label}.tests[${index}].protected_distinction`);
      if (GENERIC_PROTECTED_DISTINCTION.test(test.protected_distinction)) {
        fail(`${label}.tests[${index}].protected_distinction must identify a bounded consequence or explicitly record that none was identified`);
      }
      if (!DISPOSITIONS.has(test.candidate_disposition)) fail(`${label}.tests[${index}].candidate_disposition is unsupported`);
    }

    const entryBlobs = new Map();
    for (const entry of entries) entryBlobs.set(entry.path, await treeBlob(repository, tree_oid, entry.path, label));
    const livePaths = gitPaths.filter((entry) => entry.startsWith(prefix));
    if (JSON.stringify([...entryPaths].sort()) !== JSON.stringify(livePaths)) {
      fail(`${label} file closure mismatch`);
    }
    for (const entry of entries) {
      if (digest(entryBlobs.get(entry.path)) !== entry.sha256) fail(`${label} stale exact-byte digest: ${entry.path}`);
      verifiedFiles += 1;
    }
  }
  return Object.freeze({ packages: names.length, verified_files: verifiedFiles, tree_oid });
}

async function main(argv) {
  const repositoryFlag = argv.indexOf("--repository");
  const inventoryFlag = argv.indexOf("--inventory");
  const treeFlag = argv.indexOf("--tree-ish");
  if (repositoryFlag < 0 || inventoryFlag < 0) fail("usage: --repository <path> --inventory <path>");
  const repository = path.resolve(argv[repositoryFlag + 1]);
  const inventoryPath = path.resolve(argv[inventoryFlag + 1]);
  const inventory = JSON.parse(await readFile(inventoryPath, "utf8"));
  const treeish = treeFlag < 0 ? "HEAD" : argv[treeFlag + 1];
  process.stdout.write(`${JSON.stringify(await validateInventory({ repository, inventory, treeish }))}\n`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  main(process.argv.slice(2)).catch((error) => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
}
