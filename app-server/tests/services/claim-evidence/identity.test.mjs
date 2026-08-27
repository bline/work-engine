import assert from "node:assert/strict";
import test from "node:test";

import { canonicalJson, digest, stableClaimId, validateTransportSafeJson } from "../../../src/services/claim-evidence/identity.mjs";

const reference = {
  owner: "répository", reference: "proposal.md", revision: "blob-1",
  integrity_sha256: "0".repeat(64), freshness: "current", status: "verified",
};

test("canonical JSON is compact, Unicode-preserving, sorted, and newline terminated", () => {
  const value = { z: [true, null, "é"], a: { beta: 2, alpha: 1 } };
  assert.equal(canonicalJson(value), '{"a":{"alpha":1,"beta":2},"z":[true,null,"é"]}\n');
  assert.equal(digest(value), "0a0622ea8160d234ca6d26d9f01b7617c5bc79c0c6ba439c380b9b92be2097ac");
});

test("stable claim identity validates the whole subject but hashes only stable coordinates", () => {
  const left = { namespace: "a:b", subject_kind: "c", stable_subject_id: "d", evidence_baseline: reference, content_set: ["one"] };
  const right = { namespace: "a", subject_kind: "b", stable_subject_id: "c:d", evidence_baseline: reference, content_set: ["two"] };
  assert.notEqual(stableClaimId(left), stableClaimId(right));
  assert.equal(stableClaimId(left), stableClaimId({ ...left, content_set: ["changed"] }));
  assert.throws(() => stableClaimId({ ...left, extra: true }), /missing or unknown/);
});

test("transport validation rejects unsafe and non-JSON numeric values", () => {
  assert.throws(() => validateTransportSafeJson({ value: Number.MAX_SAFE_INTEGER + 1 }), /not lossless/);
  assert.throws(() => validateTransportSafeJson({ value: Infinity }), /finite/);
  assert.throws(() => validateTransportSafeJson({ value: undefined }), /not a JSON value/);
});
