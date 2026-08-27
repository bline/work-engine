import assert from "node:assert/strict";
import test from "node:test";

import {
  normalizeObservation, observationEventIdentity, validateObservation,
} from "../../../src/services/claim-evidence/observation.mjs";

const reference = {
  owner: "repository", reference: "baseline", revision: "tree:base",
  integrity_sha256: "a".repeat(64), freshness: "current", status: "verified",
};

const input = (overrides = {}) => ({
  event_identity: observationEventIdentity("producer", "source"),
  producer: { identity: "producer", kind: "deterministic_test_adapter" },
  origin: { kind: "test_receipt", reference: "receipt:one", trust_classification: "untrusted_input" },
  subject: { namespace: "tests", subject_kind: "receipt", stable_subject_id: "one", content_set: ["result.json"] },
  evidence_baseline: reference,
  artifact: {
    kind: "test_receipt", reference: "receipt:one",
    digest: { algorithm: "sha256", value: "b".repeat(64) },
    verification: "verified",
    checkpoint: { object_format: "sha256", commit: "receipt-one", tree: "suite-one" },
  },
  observed_at: "2026-08-26T12:00:00.000Z",
  provider_sequence: null,
  completeness: "available",
  exclusions: [],
  collection_failures: [],
  executable_generation: "test-generation",
  adapter_version: "test-adapter-v1",
  ...overrides,
});

test("observation normalization is exact, stable, and deeply immutable", () => {
  const first = normalizeObservation(input());
  const second = normalizeObservation(structuredClone(input()));
  assert.deepEqual(first, second);
  assert.match(first.id, /^observation-v1@[0-9a-f]{64}$/);
  assert.ok(Object.isFrozen(first));
  assert.ok(Object.isFrozen(first.artifact));
  assert.throws(() => { first.artifact.reference = "changed"; }, TypeError);
  assert.equal(validateObservation(first), first);

  const extra = structuredClone(input());
  extra.authority = "invented";
  assert.throws(() => normalizeObservation(extra), /missing or unknown fields/);
});

test("origin trust is retained as data and changes identity without granting authority", () => {
  const untrusted = normalizeObservation(input());
  const classified = normalizeObservation(input({
    origin: { ...input().origin, trust_classification: "host_verified_artifact" },
  }));
  assert.notEqual(untrusted.id, classified.id);
  assert.equal(classified.origin.trust_classification, "host_verified_artifact");
  assert.equal("authority_ref" in classified, false);
  assert.equal("profile" in classified, false);
});

test("unavailable collection retains requested artifact identity and failures without a digest", () => {
  const unavailable = normalizeObservation(input({
    artifact: {
      ...input().artifact,
      digest: null,
      verification: "unavailable",
    },
    completeness: "unavailable",
    exclusions: ["artifact content"],
    collection_failures: ["provider could not retrieve the requested artifact"],
  }));
  assert.equal(unavailable.artifact.reference, "receipt:one");
  assert.equal(unavailable.artifact.digest, null);
  assert.equal(unavailable.artifact.verification, "unavailable");

  assert.throws(() => normalizeObservation(input({
    artifact: { ...input().artifact, digest: null, verification: "unavailable" },
  })), /must retain a collection failure/);
  assert.throws(() => normalizeObservation(input({
    artifact: { ...input().artifact, verification: "unavailable" },
    completeness: "unavailable",
    collection_failures: ["failed"],
  })), /cannot claim a digest/);
  assert.throws(() => normalizeObservation(input({
    artifact: { ...input().artifact, digest: null, verification: "unavailable" },
    completeness: "available",
    collection_failures: ["provider could not retrieve the requested artifact"],
  })), /must have unavailable completeness/);

  const partial = normalizeObservation(input({
    completeness: "partial",
    exclusions: ["optional provider annotations"],
    collection_failures: ["provider sequence metadata was unavailable"],
  }));
  assert.equal(partial.artifact.verification, "verified");
  assert.equal(partial.completeness, "partial");
  assert.deepEqual(partial.exclusions, ["optional provider annotations"]);
});
