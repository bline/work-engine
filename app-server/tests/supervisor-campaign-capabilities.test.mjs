import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, rm, symlink } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  createSupervisorCampaignCapabilityDefinitions,
} from "../src/services/slice-campaign/capability-contract.mjs";
import {
  createSupervisorCampaignCapabilityHostRuntime,
} from "../src/services/slice-campaign/capability-host-runtime.mjs";
import { createLegacySupervisorControlAdapter } from "../src/services/slice-campaign/legacy-control-adapter.mjs";
import { SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL } from "../src/services/slice-campaign/host-effect-runtime.mjs";

const root = path.resolve(new URL("../..", import.meta.url).pathname);
const sha = (value) => createHash("sha256").update(value).digest("hex");

function effect(capability, operation, input) {
  return {
    protocol: SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL,
    capability,
    operation,
    input,
  };
}

test("six thin clients bind exact operations and never infer human authority", async () => {
  const calls = [];
  const definitions = createSupervisorCampaignCapabilityDefinitions(async (request) => {
    calls.push(request);
    return {
      schema_version: 1,
      generation_id: "generation-six",
      capability: request.capability,
      operation: request.operation,
      result: { state: "accepted" },
    };
  });
  assert.deepEqual([...definitions.keys()].sort(), [
    "capability.checkpoint_lifecycle",
    "capability.completion_offer",
    "capability.lifecycle_control",
    "capability.preflight",
    "capability.receipt_finalization",
    "capability.resume",
  ]);
  const offer = definitions.get("capability.completion_offer");
  await assert.rejects(offer.handler({ operation: "resolve", input: {
    offer: {}, decision: { decision: "create", authority: {
      kind: "agent", reference: "inferred", observed_at: "2026-09-01T09:00:00-06:00",
    } },
  } }), /kind must be human/);
  assert.equal(calls.length, 0);
  const result = await offer.handler({ operation: "resolve", input: {
    offer: {}, decision: { decision: "create", authority: {
      kind: "human", reference: "user-message-1", observed_at: "2026-09-01T09:00:00-06:00",
    } },
  } });
  assert.equal(result.generation_id, "generation-six");
  assert.equal(calls[0].capability, "capability.completion_offer");
  assert.equal(calls[0].operation, "resolve");
});

test("stable host executes preflight and native lifecycle across reconstruction", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "work-engine-six-capabilities-"));
  t.after(() => rm(stateRoot, { recursive: true, force: true }));
  let runtime = await createSupervisorCampaignCapabilityHostRuntime({
    workspaceRoot: root,
    stateRoot,
  });
  const preflight = await runtime.dispatch({
    generationId: "generation-a",
    effect: effect("capability.preflight", "run", {
      campaign_path: "app-server/skills-migration-campaign.yaml",
    }),
  });
  assert.equal(preflight.generation_id, "generation-a");
  assert.equal(preflight.result.engineConfig.version, 2);
  await assert.rejects(runtime.dispatch({
    generationId: "generation-a",
    effect: effect("capability.preflight", "run", {
      campaign_path: path.join(root, "app-server/skills-migration-campaign.yaml"),
    }),
  }), /repository-relative/);

  const identity = {
    runId: "host-six", sliceNumber: 1, attemptId: "attempt-1", planVersion: "plan-1",
  };
  let campaign = (await runtime.dispatch({
    generationId: "generation-a",
    effect: effect("capability.lifecycle_control", "admit", {
      identity,
      workspace: "/private/workspace-a",
      acceptedBoundary: { reference: "plan:a2", sha256: sha("accepted-a2") },
      baseline: { acceptedCommit: "baseline", acceptedTree: "tree", interSliceCommit: "inter" },
    }),
  })).result;
  campaign = (await runtime.dispatch({
    generationId: "generation-a",
    effect: effect("capability.lifecycle_control", "advance", {
      identity, expectedRevision: campaign.revision, phase: "implementing", consequence: {},
    }),
  })).result;
  runtime.close();

  runtime = await createSupervisorCampaignCapabilityHostRuntime({ workspaceRoot: root, stateRoot });
  t.after(() => runtime.close());
  const recovered = await runtime.dispatch({
    generationId: "generation-b",
    effect: effect("capability.resume", "recover_active", { identity }),
  });
  assert.equal(recovered.generation_id, "generation-b");
  assert.equal(recovered.result.revision, campaign.revision);
  for (const capability of [
    "capability.completion_publication",
    "capability.workspace_coordination",
    "capability.worktree_lifecycle",
    "capability.canonical_publication",
    "capability.strategic_reconciliation",
  ]) {
    await assert.rejects(runtime.dispatch({
      generationId: "generation-b",
      effect: effect(capability, "unavailable", {}),
    }), /unavailable/);
  }
});

test("stable host routes each legacy-owned operation only through its fixed adapter", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "work-engine-legacy-routes-"));
  t.after(() => rm(stateRoot, { recursive: true, force: true }));
  const calls = [];
  const legacy = {
    preflight(input) { calls.push(["preflight", input]); return { owner: "preflight" }; },
    finalize(input) { calls.push(["finalize", input]); return { owner: "receipt" }; },
    validateReceipt(receipt) { calls.push(["validateReceipt", receipt]); return receipt; },
    checkpoint(operation, input) {
      calls.push([`checkpoint.${operation}`, input]);
      return { owner: `checkpoint.${operation}` };
    },
    offer(operation, input) {
      calls.push([`offer.${operation}`, input]);
      return operation === "reconcile" ? null : { owner: `offer.${operation}` };
    },
    resumeTerminal(input) { calls.push(["resume.terminal", input]); return { owner: "resume" }; },
    identity: { backend: "fixed-test-double" },
  };
  const runtime = await createSupervisorCampaignCapabilityHostRuntime({
    workspaceRoot: root,
    stateRoot,
    legacyAdapterFactory: async () => legacy,
  });
  t.after(() => runtime.close());
  const dispatch = (capability, operation, input) => runtime.dispatch({
    generationId: "generation-routes",
    effect: effect(capability, operation, input),
  });

  await dispatch("capability.preflight", "run", { campaign_path: "campaign.yaml" });
  await dispatch("capability.receipt_finalization", "finalize_named_campaign", {
    semantic_receipt: {}, telemetry_ingress: {}, campaign_preflight: {}, handoff_receipt: {},
  });
  const candidate = {};
  await dispatch("capability.checkpoint_lifecycle", "accept", {
    candidate, review_result: {}, gate_receipt_digest: "digest",
  });
  await dispatch("capability.checkpoint_lifecycle", "stop", { candidate });
  const offer = {};
  await dispatch("capability.completion_offer", "load", {
    repository: root, run_id: "run", slice_number: 1,
  });
  await dispatch("capability.completion_offer", "resolve", { offer, decision: {
    decision: "decline",
    authority: { kind: "human", reference: "user-message", observed_at: "2026-09-01T09:00:00-06:00" },
  } });
  await dispatch("capability.completion_offer", "reconcile", { offer });
  await dispatch("capability.completion_offer", "expire", { offer, reason: "bounded expiry" });
  await dispatch("capability.resume", "recover_terminal", {
    campaign_preflight: {}, run_id: "run",
  });

  assert.deepEqual(calls.map(([operation]) => operation), [
    "preflight", "finalize", "checkpoint.accept", "checkpoint.stop",
    "offer.load", "offer.resolve", "offer.reconcile", "offer.expire", "resume.terminal",
  ]);
});

test("fixed legacy adapter rejects a metrics destination through an escaping symlink", async (t) => {
  const local = await mkdtemp(path.join(root, ".tmp-a2-metrics-"));
  const external = await mkdtemp(path.join(os.tmpdir(), "work-engine-a2-external-"));
  t.after(() => rm(local, { recursive: true, force: true }));
  t.after(() => rm(external, { recursive: true, force: true }));
  await symlink(external, path.join(local, "metrics"));
  const adapter = await createLegacySupervisorControlAdapter({ workspaceRoot: root });
  await assert.rejects(adapter.finalize({
    semantic_receipt: {}, telemetry_ingress: {}, handoff_receipt: {},
    campaign_preflight: {
      campaignSource: { resolvedPath: path.join(root, "app-server/skills-migration-campaign.yaml") },
      engineConfig: { metrics: {
        path: path.join(path.relative(root, local), "metrics", "receipts.jsonl"),
      } },
    },
  }), /escapes the repository/);
});
