import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { validateInventory } from "../scripts/validate-skills-migration-inventory.mjs";

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

async function fixture(t) {
  const repository = await mkdtemp(path.join(os.tmpdir(), "skills-migration-inventory."));
  t.after(() => rm(repository, { recursive: true, force: true }));
  const packages = [];
  for (const name of ["alpha", "beta"]) {
    const directory = path.join(repository, "skills", name);
    await mkdir(path.join(directory, "tests"), { recursive: true });
    const source = `# ${name}\n`;
    const artifact = `${name} artifact\n`;
    const testSource = `${name} test\n`;
    await writeFile(path.join(directory, "SKILL.md"), source);
    await writeFile(path.join(directory, "reference.md"), artifact);
    await writeFile(path.join(directory, "tests", `${name}.test`), testSource);
    packages.push({
      name,
      classification_state: "provisional",
      authority_status: "not_accepted",
      responsibility: `Exercise ${name} responsibility.`,
      shapes: ["agent_skill"],
      candidate_destination: "reusable skill; provisional",
      compatibility_posture: "preserve exact behavior pending successor evidence",
      reviewer_applicability: "advisory classification review required",
      consumers: ["fixture consumer"],
      server_service_dependencies: [],
      unresolved_authority_questions: ["Canonical destination is not accepted."],
      evidence_sources: [`skills/${name}/SKILL.md`],
      source: { path: `skills/${name}/SKILL.md`, sha256: sha256(source) },
      artifacts: [{ path: `skills/${name}/reference.md`, sha256: sha256(artifact) }],
      tests: [{ path: `skills/${name}/tests/${name}.test`, sha256: sha256(testSource), protected_distinction: "Fixture consequence remains observable.", candidate_disposition: "retain" }],
    });
  }
  return { repository, inventory: { schema_version: 1, inventory_id: "fixture", evidence_only_notice: "This evidence does not grant or transfer authority.", classification_state: "provisional", authority_status: "not_accepted", baseline: { git_head: "fixture", worktree_observation: "exact fixture bytes" }, graph_limitations: ["fixture"], packages } };
}

test("accepts a complete two-skill exact-byte inventory", async (t) => {
  const value = await fixture(t);
  assert.deepEqual(await validateInventory(value), { packages: 2, verified_files: 6 });
});

const cases = [
  ["omitted skill", (value) => value.inventory.packages.pop(), /closed inventory mismatch/],
  ["stale digest", (value) => { value.inventory.packages[0].source.sha256 = "0".repeat(64); }, /stale exact-byte digest/],
  ["dangling reference", (value) => { value.inventory.packages[0].artifacts[0].path = "skills/alpha/missing.md"; }, /file closure mismatch/],
  ["missing protected distinction", (value) => { value.inventory.packages[0].tests[0].protected_distinction = ""; }, /protected_distinction/],
  ["generic protected-distinction boilerplate", (value) => { value.inventory.packages[0].tests[0].protected_distinction = "Preserve the observable consequence exercised by alpha.test while its owning boundary is classified."; }, /must identify a bounded consequence/],
  ["missing disposition", (value) => { delete value.inventory.packages[0].tests[0].candidate_disposition; }, /candidate_disposition/],
  ["accepted without authority", (value) => { value.inventory.packages[0].classification_state = "accepted"; }, /cannot be accepted/],
];

for (const [name, mutate, pattern] of cases) {
  test(`rejects ${name}`, async (t) => {
    const value = await fixture(t);
    mutate(value);
    await assert.rejects(validateInventory(value), pattern);
  });
}

test("validates the checked-in live portfolio", async () => {
  const repository = path.resolve(new URL("../..", import.meta.url).pathname);
  const inventory = JSON.parse(await readFile(new URL("../migrations/skills/portfolio-inventory-v1.json", import.meta.url), "utf8"));
  const result = await validateInventory({ repository, inventory });
  assert.equal(result.packages > 0, true);
  assert.equal(result.verified_files > result.packages, true);
});
