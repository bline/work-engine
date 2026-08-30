import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import test from "node:test";

const workspaceRoot = new URL("../../../..", import.meta.url).pathname;

test("canonical finding vocabulary remains consumable by the legacy episode validator", async () => {
  const result = JSON.parse(await readFile(new URL("../../fixtures/implementation-review/remediation-required.json", import.meta.url)));
  const finding = result.findings[0];
  const legacy = {
    finding_id: finding.id,
    attributed_reviewer: "reviewer",
    reviewer_generation: 1,
    severity: finding.severity,
    observation: finding.observed,
    evidence_references: finding.evidence.map((item) => ({
      owner: "repository", reference: `${item.path}:${item.startLine}-${item.endLine}`,
      revision: result.subject.commit, integrity_sha256: item.sha256, freshness_rule: "exact_revision",
    })),
    status: finding.status,
    remediation_references: [],
  };
  const script = `
import importlib.util, json, pathlib, sys
p = pathlib.Path("skills/independent-review-state/scripts/independent_review_state.py")
s = importlib.util.spec_from_file_location("legacy_review_episode", p)
m = importlib.util.module_from_spec(s); s.loader.exec_module(m)
value = json.load(sys.stdin); m.validate_finding(value, "finding")
print(json.dumps({"valid": True, "status": value["status"]}))
`;
  const process = spawnSync("python3", ["-c", script], { cwd: workspaceRoot, input: JSON.stringify(legacy), encoding: "utf8" });
  assert.equal(process.status, 0, process.stderr);
  assert.deepEqual(JSON.parse(process.stdout), { valid: true, status: "open" });
});
