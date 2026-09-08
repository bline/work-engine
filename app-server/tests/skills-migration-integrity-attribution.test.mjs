import assert from "node:assert/strict";
import {execFileSync} from "node:child_process";
import {createHash} from "node:crypto";
import {mkdtemp, mkdir, rm, writeFile} from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {classifySkillsMigrationIntegrity} from "../src/services/skills-migration-integrity/attribution.mjs";

const sha = (value) => createHash("sha256").update(value).digest("hex");
const git = (root, args) => execFileSync("git", ["-C", root, ...args], {encoding: "utf8"}).trim();

async function fixture(t) {
  const root = await mkdtemp(path.join(os.tmpdir(), "skills-integrity-attribution-"));
  t.after(() => rm(root, {recursive: true, force: true}));
  git(root, ["init", "-q"]); git(root, ["config", "user.email", "test@example.invalid"]); git(root, ["config", "user.name", "Test"]);
  await mkdir(path.join(root, "skills", "owned"), {recursive: true});
  await mkdir(path.join(root, "skills", "debt"), {recursive: true});
  await mkdir(path.join(root, "app-server", "migrations", "skills"), {recursive: true});
  await writeFile(path.join(root, "skills", "owned", "SKILL.md"), "owned-v1\n");
  await writeFile(path.join(root, "skills", "debt", "SKILL.md"), "debt-v1\n");
  const inventory = (ownedDigest, debtDigest, responsibility = "fixture") => ({schema_version: 1,
    responsibility, packages: [
      {name: "owned", source: {path: "skills/owned/SKILL.md", sha256: ownedDigest}, artifacts: [], tests: []},
      {name: "debt", source: {path: "skills/debt/SKILL.md", sha256: debtDigest}, artifacts: [], tests: []},
    ]});
  await writeFile(path.join(root, "app-server", "migrations", "skills", "portfolio-inventory-v1.json"),
    JSON.stringify(inventory(sha("owned-v1\n"), sha("stale"))));
  git(root, ["add", "."]); git(root, ["commit", "-qm", "baseline"]); const baseline = git(root, ["rev-parse", "HEAD"]);
  await writeFile(path.join(root, "skills", "owned", "SKILL.md"), "owned-v2\n");
  await writeFile(path.join(root, "app-server", "migrations", "skills", "portfolio-inventory-v1.json"),
    JSON.stringify(inventory(sha("owned-v2\n"), sha("debt-v1\n"))));
  git(root, ["add", "."]); git(root, ["commit", "-qm", "candidate"]); const candidate = git(root, ["rev-parse", "HEAD"]);
  return {root, baseline, candidate, inventory};
}

test("attributes in-scope hashes and inherited debt without semantic judgment", async (t) => {
  const {root, baseline, candidate} = await fixture(t);
  const receipt = classifySkillsMigrationIntegrity({repository: root,
    baselineRevision: baseline, candidateRevision: candidate,
    acceptedPaths: ["skills/owned/SKILL.md", "app-server/migrations/skills/portfolio-inventory-v1.json"]});
  assert.equal(receipt.verdict, "mechanically_attributed");
  assert.equal(receipt.semanticProjectionEqual, true);
  assert.deepEqual(receipt.changes.map(({path: filePath, classification}) => [filePath, classification]), [
    ["skills/debt/SKILL.md", "inherited_baseline_debt_reconciliation"],
    ["skills/owned/SKILL.md", "in_scope_derived_update"],
  ]);
  assert.deepEqual(receipt.violations, []);
  assert.match(receipt.receiptDigest, /^[0-9a-f]{64}$/);
});

test("fails closed on out-of-scope source, stale candidate binding, and semantic inventory edits", async (t) => {
  const {root, baseline, candidate, inventory} = await fixture(t);
  await writeFile(path.join(root, "skills", "debt", "SKILL.md"), "debt-v2\n");
  await writeFile(path.join(root, "app-server", "migrations", "skills", "portfolio-inventory-v1.json"),
    JSON.stringify(inventory(sha("wrong"), sha("debt-v2\n"), "changed semantic claim")));
  git(root, ["add", "."]); git(root, ["commit", "-qm", "invalid"]); const invalid = git(root, ["rev-parse", "HEAD"]);
  const receipt = classifySkillsMigrationIntegrity({repository: root,
    baselineRevision: candidate, candidateRevision: invalid,
    acceptedPaths: ["app-server/migrations/skills/portfolio-inventory-v1.json"]});
  assert.equal(receipt.verdict, "requires_disposition");
  assert.equal(receipt.semanticProjectionEqual, false);
  assert.ok(receipt.violations.some(({classification}) => classification === "stale_candidate_binding"));
  assert.ok(receipt.violations.some(({classification}) => classification === "out_of_scope_source_change"));
  assert.ok(receipt.violations.some(({classification}) => classification === "semantic_inventory_change"));
  assert.notEqual(baseline, invalid);
});
