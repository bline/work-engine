import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";

import { canonicalJson, digest, stableClaimId } from "../../../src/services/claim-evidence/identity.mjs";
import { BUILD_VERSION } from "../../../src/services/claim-evidence/contract.mjs";
import { LEGACY_BUILD_VERSION, NATIVE_BUILD_VERSION, buildLegacyProjection } from "../../../src/services/claim-evidence/legacy-compatibility.mjs";
import { blankStore } from "../../../src/services/claim-evidence/service.mjs";

const vector = {
  value: { z: [true, null, "é"], a: { beta: 2, alpha: 1 } },
  subject: {
    namespace: "proposal-research", subject_kind: "placement", stable_subject_id: "claims-service",
    evidence_baseline: { owner: "repository", reference: "proposal.md", revision: "blob-1", integrity_sha256: "a".repeat(64), freshness: "current", status: "verified" },
    content_set: ["proposal.md"],
  },
};

test("JavaScript canonical bytes, digest, and stable claim ID match the Python oracle", (t) => {
  const script = String.raw`
import importlib.util, json, pathlib, sys
path = pathlib.Path('skills/claim-evidence/scripts/claim_evidence.py')
spec = importlib.util.spec_from_file_location('claim_evidence_oracle', path)
module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
value = json.load(sys.stdin)
print(json.dumps({'canonical_hex': module.canonical(value['value']).hex(), 'digest': module.digest(value['value']), 'claim_id': module.stable_claim_id(value['subject'])}, sort_keys=True))
`;
  const result = spawnSync("python3", ["-c", script], {
    cwd: process.cwd(), input: JSON.stringify(vector), encoding: "utf8",
    env: { ...process.env, PYTHONDONTWRITEBYTECODE: "1" },
  });
  if (result.error?.code === "ENOENT") return t.skip("python3 is unavailable");
  assert.equal(result.status, 0, result.stderr);
  const oracle = JSON.parse(result.stdout);
  assert.equal(Buffer.from(canonicalJson(vector.value)).toString("hex"), oracle.canonical_hex);
  assert.equal(digest(vector.value), oracle.digest);
  assert.equal(stableClaimId(vector.subject), oracle.claim_id);
});

test("legacy projection compatibility changes only implementation build metadata", () => {
  const projection = buildLegacyProjection(blankStore());
  assert.equal(LEGACY_BUILD_VERSION, "claim-evidence-python-v1");
  assert.equal(NATIVE_BUILD_VERSION, BUILD_VERSION);
  assert.equal(projection.build_version, LEGACY_BUILD_VERSION);
  assert.equal(projection.canonical_input.path, "canonical/store.json");
});

