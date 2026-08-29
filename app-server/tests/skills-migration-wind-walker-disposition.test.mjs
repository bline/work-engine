import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import {
  validateWindWalkerDisposition,
} from "../scripts/validate-skills-migration-wind-walker-disposition.mjs";

const repository = path.resolve(new URL("../..", import.meta.url).pathname);
const recordUrl = new URL("../migrations/skills/wind-walker-disposition-v1.json", import.meta.url);

async function record() {
  return JSON.parse(await readFile(recordUrl, "utf8"));
}

test("checked-in Wind Walker disposition preserves residual causality and blocks retirement", async () => {
  const schema = JSON.parse(await readFile(
    new URL("../migrations/skills/wind-walker-disposition-v1.schema.json", import.meta.url),
    "utf8",
  ));
  assert.equal(schema.$id, "work-engine.skills-migration.wind-walker-disposition-v1");
  assert.equal(schema.additionalProperties, false);
  assert.deepEqual(await validateWindWalkerDisposition({ repository, record: await record() }), {
    record_id: "wind-walker-disposition-v1",
    subject_sha256: "bfc453242d01b625ecc96469c39f20afc43d8340bfc51523d4371a973c189005",
    residual_instructions: 4,
    retirement_prerequisites: 5,
  });
});

for (const [name, mutate, pattern] of [
  ["accepted authority", (value) => { value.authority_status = "accepted"; }, /cannot be accepted/],
  ["stale subject", (value) => { value.subject.sha256 = "0".repeat(64); }, /digest is stale/],
  ["omitted causal residual", (value) => { value.residual_projection.instructions.pop(); }, /required identities/],
  ["premature retirement", (value) => { value.retirement.status = "retired"; }, /blocked pending authority/],
  ["missing human transition", (value) => { value.retirement.prerequisites.pop(); }, /required identities/],
]) {
  test(`rejects ${name}`, async () => {
    const value = await record();
    mutate(value);
    await assert.rejects(validateWindWalkerDisposition({ repository, record: value }), pattern);
  });
}
