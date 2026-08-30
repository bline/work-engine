import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { createImplementationReviewService } from "../../../src/services/implementation-review/service.mjs";

const fixture = async (name) => JSON.parse(await readFile(new URL(`../../fixtures/implementation-review/${name}.json`, import.meta.url)));

test("implementation-review admits exact evidence-bearing provider-neutral verdicts", async () => {
  const service = createImplementationReviewService();
  for (const name of ["acceptable-as-is", "remediation-required", "incomplete"]) {
    const result = await fixture(name);
    const admitted = service.admit({ result, expectedSubject: result.subject });
    assert.equal(admitted.result.verdict, result.verdict);
    assert.equal(admitted.resultRevision.length, 64);
    assert.equal(admitted.authority.mutationAuthorized, false);
    assert.equal(admitted.authority.implementationAcceptanceAuthorized, false);
    assert.equal(admitted.authority.reviewerSelectionAuthorized, false);
    assert.equal(admitted.authority.humanAuthorityConferred, false);
    assert.equal(admitted.authority.independenceClaimed, false);
    assert.equal(Object.isFrozen(admitted), true);
  }
});

test("implementation-review fails closed on subject, evidence, and verdict contradictions", async () => {
  const service = createImplementationReviewService();
  const accepted = await fixture("acceptable-as-is");
  assert.throws(() => service.admit({ result: accepted, expectedSubject: { ...accepted.subject, tree: "other" } }), /does not match/);
  assert.throws(() => service.admit({ result: { ...accepted, decisiveEvidence: [] }, expectedSubject: accepted.subject }), /requires decisive evidence/);
  const remediation = await fixture("remediation-required");
  assert.throws(() => service.admit({ result: { ...remediation, findings: [] }, expectedSubject: remediation.subject }), /requires an unresolved/);
  const incomplete = await fixture("incomplete");
  assert.throws(() => service.admit({ result: { ...incomplete, limitations: [] }, expectedSubject: incomplete.subject }), /requires explicit limitations/);
  const tampered = structuredClone(remediation); tampered.findings[0].evidence[0].sha256 = "forged";
  assert.throws(() => service.admit({ result: tampered, expectedSubject: tampered.subject }), /lowercase SHA-256/);
});
