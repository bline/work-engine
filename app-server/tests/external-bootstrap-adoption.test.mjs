import assert from "node:assert/strict";
import test from "node:test";
import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { promisify } from "node:util";

import { buildExternalBootstrapPacket, validateExternalBootstrapPacket } from
  "../src/services/slice-campaign/external-bootstrap-adoption-contract.mjs";

const sha = (value) => createHash("sha256").update(value).digest("hex");
const identity = {runId: "context-pressure-active-turn-recovery-20260909", sliceNumber: 1,
  attemptId: "active-turn-pressure-repair-plan-v1", planVersion: "context-pressure-active-turn-repair-v1"};

function unsignedPacket(expectedRevision) {
  return {
    schema_version: 1, kind: "work-engine.external-bootstrap-evidence-packet",
    provenance: "external_bootstrap_wind_walker", identity,
    expected_campaign_revision: expectedRevision,
    objective_boundary_baseline: {
      objective: "Recover active-turn admission reserve and stranded-role reconciliation without mutating stranded state.",
      accepted_boundary: {reference: "wind-walker:context-lifecycle-bootstrap-recovery-20260910-builder-plan-v1@9601e2377122d6c707196b7a1b27e2f9007f6e30",
        sha256: "07fc193ab4ab76a938334ae939ddcfe5151711234d5918a36e829301b6325e61"},
      baseline: {commit: "d7fee69cd2bb78e56736ceeb8463fb37a3687627",
        tree: "c39438f617495af8dabb0974e07ccb41a3d64a88"},
    },
    task_manifest: [{path: "app-server/src/active-turn-lifecycle-scheduler.mjs", action: "add"}],
    checkpoint: {parent_commit: "d7fee69cd2bb78e56736ceeb8463fb37a3687627",
      commit: "private-checkpoint", tree: "private-tree", ref: "refs/work-engine/checkpoints/private",
      task_patch_sha256: sha("patch")},
    gate: {manifest_sha256: sha("gate-manifest"), receipt_sha256: sha("gate-receipt"),
      test_counts: {passed: 47, failed: 0}, workspace_integrity_sha256: sha("workspace")},
    incident_refs: {
      ppce_supervisor: "01a08469-412b-7a60-a397-396d5591064a",
      failed_turn: "01a08882-4389-7cb2-8d85-6a0af14f57cc",
      failed_preparation: "sha256:41d325d8f2697833378c4c0af2fd5062ca52c1e3c3e3ca6b67e2e9bc6c6c219d",
      chatboard_sequence: "442",
      stranded_revision: "45bd087176dbf9b4f4dcc40752b7333acfd9f5b923a13f6b6702b94fdc5330ea",
      supervisor_thread: "01a08900-41d7-7590-8e74-e8e29f2e1a8d",
      builder_role: "slice-builder:context-pressure-active-turn-recovery-20260909-slice-1-builder",
      builder_thread: "01a08902-b90a-7802-99ec-900451c1b4ac",
      delivery: "18d6b63d-814f-4a03-82f2-80e23f1d503e",
      stranded_preparation: "sha256:00cf6679030da3cf7b8b010e2b9735da28a425238d348362bdd4211489523f54",
      workspace_lease: "0c1da61a-0701-47f1-9616-a3c66d9382b7",
      workspace_fence: "1",
    },
    limitations: ["app_server_lifecycle_proof_unavailable",
      "app_server_mailbox_proof_unavailable", "app_server_delivery_output_proof_unavailable"],
  };
}

test("packet build and verify tooling binds whole bytes and explicit proof limitations", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "external-bootstrap-packet."));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const unsigned = unsignedPacket(sha("revision"));
  const unsignedPath = path.join(directory, "unsigned.json");
  const packetPath = path.join(directory, "packet.json");
  await writeFile(unsignedPath, JSON.stringify(unsigned));
  const exec = promisify(execFile);
  const root = path.resolve(new URL("..", import.meta.url).pathname);
  const built = await exec(process.execPath, [path.join(root, "scripts/build-external-bootstrap-packet.mjs"), unsignedPath]);
  const packet = JSON.parse(built.stdout);
  await writeFile(packetPath, JSON.stringify(packet));
  const verified = await exec(process.execPath, [path.join(root, "scripts/verify-external-bootstrap-packet.mjs"), packetPath]);
  assert.deepEqual(JSON.parse(verified.stdout), {valid: true, whole_packet_sha256: packet.whole_packet_sha256});
  assert.throws(() => validateExternalBootstrapPacket({...packet, gate: {...packet.gate, receipt_sha256: sha("mutated")}}), /integrity failed/);
  const {limitations: _removed, ...missing} = unsigned;
  assert.throws(() => buildExternalBootstrapPacket(missing), /fields are invalid/);
});

export { identity, unsignedPacket };
