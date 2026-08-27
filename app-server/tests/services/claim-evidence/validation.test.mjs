import assert from "node:assert/strict";
import test from "node:test";

import { blankStore } from "../../../src/services/claim-evidence/service.mjs";
import { validateAuthority, validateProfilePayload, validateReference, validateStore } from "../../../src/services/claim-evidence/validation.mjs";

function reference(status = "verified") {
  return { owner: "repository", reference: "path", revision: "blob", integrity_sha256: "f".repeat(64), freshness: "current", status };
}
function authority(overrides = {}) {
  return {
    schema_version: 1, grant_id: "grant", actor: "actor", profile: "proposal-research-v1",
    permissions: ["create_claim"], decision_scope: "scope", authority_reference: reference(), ...overrides,
  };
}

test("references, authorities, and profile payloads are closed", () => {
  validateReference(reference());
  validateAuthority(authority());
  validateProfilePayload("proposal-research-v1", { materiality: "material", support_qualification: "supported" });
  validateProfilePayload("revision-bound-review-finding-v1", { finding_id: "F-1", severity: "high", episode: "review", outcome: "open" });
  assert.throws(() => validateReference({ ...reference(), extra: true }), /missing or unknown/);
  assert.throws(() => validateAuthority(authority({ permissions: ["create_claim", "create_claim"] })), /unique/);
  assert.throws(() => validateAuthority(authority({ authority_reference: reference("unavailable") })), /not verified/);
  assert.throws(() => validateProfilePayload("proposal-research-v1", { materiality: "material" }), /missing or unknown/);
});

test("store validation rejects unknown versions and globally colliding identities", () => {
  const store = blankStore([authority()]);
  assert.equal(validateStore(store).authorities.get("grant").actor, "actor");
  assert.throws(() => validateStore({ ...store, schema_version: 2 }), /unsupported store version/);
  const collision = structuredClone(store);
  collision.operations.push({ operation_id: "grant", action: "create_claim", payload_sha256: "0".repeat(64), result_identity: "missing", authority_ref: "grant" });
  assert.throws(() => validateStore(collision), /duplicate record identity/);
});

