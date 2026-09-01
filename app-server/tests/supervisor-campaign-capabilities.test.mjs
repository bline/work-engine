import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdtemp, rm, symlink, writeFile } from "node:fs/promises";
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
import { canonicalJson, digest as workspaceDigest } from "../src/services/workspace-coordination/contract.mjs";

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

function git(repository, ...args) {
  return execFileSync("git", ["-C", repository, ...args], { encoding: "utf8" }).trim();
}

function gitInput(repository, args, input, encoding = "utf8") {
  return execFileSync("git", ["-C", repository, ...args], { encoding, input }).trim();
}

function writeOfferBlob(repository, value) {
  return gitInput(repository, ["hash-object", "-w", "--stdin"], canonicalJson(value));
}

async function completionFixture(t) {
  const repository = await mkdtemp(path.join(os.tmpdir(), "work-engine-a3b-host-repo-"));
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "work-engine-a3b-host-state-"));
  t.after(async () => { await rm(repository, { recursive: true, force: true }); await rm(stateRoot, { recursive: true, force: true }); });
  git(repository, "init", "--quiet", "--initial-branch=human");
  git(repository, "config", "user.name", "A3b Host Test");
  git(repository, "config", "user.email", "a3b@example.invalid");
  await writeFile(path.join(repository, "base.txt"), "base\n");
  git(repository, "add", "base.txt"); git(repository, "commit", "--quiet", "-m", "base");
  const baseline = git(repository, "rev-parse", "HEAD");
  await writeFile(path.join(repository, "task.txt"), "accepted\n");
  git(repository, "add", "task.txt"); git(repository, "commit", "--quiet", "-m", "candidate");
  const candidate = git(repository, "rev-parse", "HEAD");
  const tree = git(repository, "rev-parse", "HEAD^{tree}");
  const patchBytes = execFileSync("git", ["-C", repository, "diff-tree", "--binary", "--no-renames", "--no-ext-diff", `${baseline}^{tree}`, tree]);
  const paths = [{ path: "task.txt", action: "include", attribution: "task_owned",
    content_digest: createHash("sha256").update("accepted\n").digest("hex") }];
  const taskPatchDigest = createHash("sha256").update(patchBytes).digest("hex");
  const manifestDigest = workspaceDigest(paths);
  const gateReceiptDigest = sha("a3b-gate");
  const candidateCheckpointId = sha("a3b-candidate-checkpoint");
  const metadata = {
    work_engine_checkpoint: 1, kind: "accepted", candidate_checkpoint_id: candidateCheckpointId,
    run_id: "a3b-host", slice_number: 1, candidate_attempt: 1, tree,
    task_patch_digest: taskPatchDigest, manifest_digest: manifestDigest,
    gate_receipt_digest: gateReceiptDigest,
    plan_version: "a3b-host-plan", scope_revision: "a3b-host-scope",
  };
  const checkpointCommit = gitInput(repository, ["commit-tree", tree, "-p", candidate], `${JSON.stringify(metadata)}\n`);
  const checkpointRef = "refs/work-engine/checkpoints/a3b-host/slice-1/accepted";
  git(repository, "update-ref", checkpointRef, checkpointCommit);
  const checkpoint = {
    schema_version: 1, checkpoint_id: workspaceDigest(metadata), checkpoint_kind: "accepted",
    repository, run_id: "a3b-host", slice_number: 1, candidate_attempt: 1,
    baseline_commit_oid: baseline, baseline_tree_oid: git(repository, "rev-parse", `${baseline}^{tree}`),
    checkpoint_commit_oid: checkpointCommit, checkpoint_tree_oid: tree,
    parent_checkpoint_commit_oid: candidate, plan_version: metadata.plan_version,
    scope_revision: metadata.scope_revision, gate_receipt_digest: gateReceiptDigest,
    task_patch_digest: taskPatchDigest, paths, ref: checkpointRef, manifest_digest: manifestDigest,
    created_at: "2026-09-01T11:30:00-06:00", limitations: [],
    candidate_checkpoint_id: candidateCheckpointId,
  };
  git(repository, "reset", "--hard", "--quiet", baseline);
  const proposal = {
    schema_version: 2, subject: "publish through host", body: "",
    paths: ["task.txt"], checkpoint_commit_oid: checkpoint.checkpoint_commit_oid,
    checkpoint_tree_oid: checkpoint.checkpoint_tree_oid, task_patch_digest: checkpoint.task_patch_digest,
    provenance: { schema_version: 1, producer: "a3b-host-test",
      evidence: [{ kind: "gate", digest: checkpoint.gate_receipt_digest }] },
  };
  const request = {
    repository, run_id: "a3b-host", slice_number: 1, expected_branch: "human",
    expected_head_oid: baseline, accepted_paths: [{ action: "include", path: "task.txt" }], proposal,
  };
  const opened = { schema_version: 2, offer_id: workspaceDigest(request), state: "open",
    request, result: null, reason: null, prior_oid: null, decision: null };
  const priorOid = writeOfferBlob(repository, opened);
  const authorized = { ...opened, state: "create_authorized", prior_oid: priorOid,
    decision: { decision: "create", authority: { kind: "human",
      reference: "user:a3b-host", observed_at: "2026-09-01T11:30:00-06:00" } } };
  const artifactOid = writeOfferBlob(repository, authorized);
  const ref = "refs/work-engine/completion-offers/a3b-host/slice-1";
  git(repository, "update-ref", ref, artifactOid);
  return { repository, stateRoot, checkpoint, offer: { ...authorized, artifact_oid: artifactOid, ref } };
}

test("ten thin clients bind exact operations and never infer human authority", async () => {
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
    "capability.canonical_publication",
    "capability.checkpoint_lifecycle",
    "capability.completion_offer",
    "capability.completion_publication",
    "capability.lifecycle_control",
    "capability.preflight",
    "capability.receipt_finalization",
    "capability.resume",
    "capability.workspace_coordination",
    "capability.worktree_lifecycle",
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
    canonicalBranches: ["main"],
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

  runtime = await createSupervisorCampaignCapabilityHostRuntime({ workspaceRoot: root, stateRoot, canonicalBranches: ["main"] });
  t.after(() => runtime.close());
  const recovered = await runtime.dispatch({
    generationId: "generation-b",
    effect: effect("capability.resume", "recover_active", { identity }),
  });
  assert.equal(recovered.generation_id, "generation-b");
  assert.equal(recovered.result.revision, campaign.revision);
  for (const capability of [
    "capability.completion_publication",
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
    canonicalBranches: ["main"],
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

test("stable host executes real workspace, worktree, and completion capability registrations", async (t) => {
  const stateRoot = await mkdtemp(path.join(os.tmpdir(), "work-engine-a3a-host-"));
  t.after(() => rm(stateRoot, { recursive: true, force: true }));
  const legacy = {
    identity: {}, preflight() {}, finalize() {}, validateReceipt(value) { return value; },
    checkpoint() {}, offer() {}, resumeTerminal() {},
  };
  const runtime = await createSupervisorCampaignCapabilityHostRuntime({
    workspaceRoot: root, stateRoot, canonicalBranches: ["main"],
    legacyAdapterFactory: async () => legacy,
  });
  t.after(() => runtime.close());
  const dispatch = (capability, operation, input) => runtime.dispatch({
    generationId: "generation-a3a", effect: effect(capability, operation, input),
  });
  const acquired = (await dispatch("capability.workspace_coordination", "acquire", {
    resource: { type: "port", id: "test:49199" }, holder: "supervisor-test",
    intent_id: "a3a-vertical", ttl_ms: 60_000,
  })).result;
  assert.equal(acquired.status, "acquired");
  assert.equal((await dispatch("capability.workspace_coordination", "inspect", {
    resource: { type: "port", id: "test:49199" },
  })).result.lease.fencingToken, acquired.lease.fencingToken);
  assert.equal((await dispatch("capability.workspace_coordination", "release", {
    lease: acquired.lease,
  })).result.released, true);

  const baseline = execFileSync("git", ["-C", root, "rev-parse", "HEAD"], { encoding: "utf8" }).trim();
  const allocation = (await dispatch("capability.worktree_lifecycle", "allocate", {
    operation_id: "a3a-host-allocation", agent_id: "builder-test",
    intent_id: "a3a-host", baseline_commit: baseline,
  })).result;
  assert.equal(allocation.status, "allocated");
  assert.equal((await dispatch("capability.worktree_lifecycle", "cleanup", {
    allocation,
  })).result.status, "removed");
  await assert.rejects(dispatch("capability.canonical_publication", "prepare", {
    operation_id: "a3a-invalid-checkpoint", target_branch: "main", expected_parent: baseline,
    checkpoint: {}, manifest: [], authorization: {}, message: {},
  }), /accepted checkpoint commit|manifest/);
  await assert.rejects(runtime.dispatch({
    generationId: "generation-a3a",
    effect: effect("capability.completion_publication", "prepare", {}),
  }), /requires field offer/);
  await assert.rejects(runtime.dispatch({
    generationId: "generation-a3a",
    effect: effect("capability.strategic_reconciliation", "reconcile", {}),
  }), /unavailable/);
});

test("stable host completes exact offer authority only after explicit checkout transition", async (t) => {
  const fixture = await completionFixture(t);
  const legacy = {
    identity: {}, preflight() {}, finalize() {}, validateReceipt(value) { return value; },
    checkpoint() {}, offer() {}, resumeTerminal() {},
  };
  let runtime = await createSupervisorCampaignCapabilityHostRuntime({
    workspaceRoot: fixture.repository, stateRoot: fixture.stateRoot,
    canonicalBranches: ["human"], legacyAdapterFactory: async () => legacy,
  });
  t.after(() => runtime.close());
  const dispatch = (operation, input) => runtime.dispatch({
    generationId: "generation-a3b", effect: effect("capability.completion_publication", operation, input),
  });
  const prepared = (await dispatch("prepare", {
    offer: fixture.offer, accepted_checkpoint: fixture.checkpoint,
  })).result;
  assert.equal(prepared.vocabulary, "private_prepared_publication");
  const validation = {
    schemaVersion: 1, status: "passed", tree: prepared.integratedTree,
    profile: "a3b-host", requirements: ["exact_tree"], gateResult: { status: "passed" },
  };
  validation.receiptDigest = workspaceDigest(validation);
  const transition = (await dispatch("complete", {
    offer: fixture.offer, preparation_revision: prepared.preparationRevision, validation,
  })).result;
  assert.equal(transition.status, "checkout_transition_required");
  runtime.close();

  git(fixture.repository, "switch", "--detach", "--quiet", fixture.offer.request.expected_head_oid);
  runtime = await createSupervisorCampaignCapabilityHostRuntime({
    workspaceRoot: fixture.repository, stateRoot: fixture.stateRoot,
    canonicalBranches: ["human"], legacyAdapterFactory: async () => legacy,
  });
  const published = (await dispatch("complete", {
    offer: fixture.offer, preparation_revision: prepared.preparationRevision, validation,
  })).result;
  assert.equal(published.status, "published");
  assert.equal(published.vocabulary, "human_visible_ref_observed");
  assert.equal(git(fixture.repository, "rev-parse", "human"), published.publication.commit);
  const reconciled = (await dispatch("reconcile", {
    offer: fixture.offer, preparation_revision: prepared.preparationRevision,
  })).result;
  assert.equal(reconciled.recordDigest, published.recordDigest);
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
