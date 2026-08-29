import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "../../..");
const SERVER = path.join(ROOT, "skills/work-engine-mcp/scripts/server.mjs");
const MANAGE = path.join(ROOT, "skills/slice-supervisor/scripts/manage_active_slice.py");
const IDENTITY = {
  run_id: "mcp-test-run",
  slice_number: 1,
  attempt_id: "attempt-1",
  plan_version: "plan-1",
};

async function connect(repository = ROOT, authority = null, claimRoot = null) {
  const args = [SERVER, "--repository", repository];
  if (authority) args.push("--review-authority-file", authority);
  if (claimRoot) args.push("--claim-root", claimRoot);
  const transport = new StdioClientTransport({
    command: process.execPath,
    args,
    cwd: ROOT,
    stderr: "pipe",
  });
  const client = new Client({ name: "work-engine-mcp-test", version: "0.1.0" });
  await client.connect(transport);
  return client;
}

const CLAIM_SCRIPT = path.join(ROOT, "skills/claim-evidence/scripts/claim_evidence.py");

function directClaim(root, args) {
  return JSON.parse(execFileSync("python3", [CLAIM_SCRIPT, "--root", root, ...args], {
    encoding: "utf8",
  }));
}

function createClaimRoot(root) {
  const setup = String.raw`
import importlib.util
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
module_path = Path(sys.argv[2])
test_path = Path(sys.argv[3])

def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

ce = load_module("claim_evidence", module_path)
fixtures = load_module("claim_evidence_test_helpers", test_path)
case = fixtures.ClaimEvidenceTest()
store = ce.blank_store()
authority = fixtures.authority("proposal-research-v1", "producer-candidate-a")
case.admit(store, authority)
research_payload = fixtures.revision("proposal-research-v1")
research_payload["evidence_references"][0]["status"] = "unavailable"
research_payload["confidence"] = 9007199254740991
research = ce.apply_operation(store, fixtures.request("mcp-create-research", "create_claim", "proposal-research-v1", {"subject": fixtures.subject("proposal-research", "candidate-a"), "statement_identity": "stable proposition", "initial_revision": research_payload}), authority)["result_identity"]
review, _ = case.publish_claim(store, "revision-bound-review-finding-v1", "review", "finding-a", "mcp-create-review")
claim_id = research.rsplit("@", 1)[0]
second = ce.apply_operation(store, fixtures.request("mcp-next", "publish_revision", "proposal-research-v1", {"claim_id": claim_id, "revision": fixtures.revision("proposal-research-v1")}, research), authority)["result_identity"]
ce.apply_operation(store, fixtures.request("mcp-edge", "publish_lineage", "proposal-research-v1", {"relationship": "supersession", "sources": [research], "target": second}), authority)
reliance_payload = {"consumer": "proposal:mcp-consumer", "consumer_revision": "tree-mcp", "decision_scope": "formation", "claim_revision_id": research, "state": "active", "predecessor_reliance": None}
ce.apply_operation(store, fixtures.request("mcp-rely", "record_reliance", "proposal-research-v1", reliance_payload), authority)
store["projection_boundary"].update({"actual_content_set": "bounded MCP equivalence fixture", "source_watermark": "tree-mcp", "excluded_inputs": ["excluded-source"], "failed_inputs": ["failed-source"], "freshness": "current_for_tree_mcp", "completeness": "partial"})
ce.validate_store(store)
store_path, projection_path = ce.paths(root)
store_path.parent.mkdir(parents=True, exist_ok=True)
(store_path.parent / "store.lock").touch()
ce.atomic_write(store_path, store)
ce.atomic_write(projection_path, ce.build_projection(store))
print(json.dumps({"claim": claim_id, "research": research, "second": second, "review": review}))
`;
  return JSON.parse(execFileSync("python3", [
    "-c", setup, root, CLAIM_SCRIPT,
    path.join(ROOT, "skills/claim-evidence/tests/test_claim_evidence.py"),
  ], { encoding: "utf8" }));
}

const SHA = "a".repeat(64);
function exactReference(owner, reference, revision) {
  return { owner, reference, revision, integrity_sha256: SHA, freshness_rule: "exact_revision" };
}

function reviewAuthority(generation, predecessorRevision = null) {
  return {
    schema_version: 1,
    profile: "independent-adversarial-review-episode-v1",
    grant_id: `review-grant-${generation}`,
    identity: { ...IDENTITY, review_obligation_id: "review-1", review_episode_id: "episode-1" },
    source: exactReference("human-authority", "accepted-review-profile", "decision-1"),
    writer: {
      logical_actor_id: "reviewer-1",
      provider: "claude",
      generation,
      runtime_session_ref: exactReference("claude-runtime", `session-${generation}`, `generation-${generation}`),
    },
    readers: ["independent_reviewer", "coordinating_builder", "slice_supervisor"],
    initial_subject: [exactReference("checkpoint", "candidate", "tree-1")],
    expires_at: null,
    predecessor_revision: predecessorRevision,
  };
}

function begin(repository) {
  return JSON.parse(execFileSync("python3", [
    MANAGE,
    "--repository",
    repository,
    "begin",
    "--identity-json",
    JSON.stringify(IDENTITY),
    "--actor-binding-json",
    JSON.stringify({
      logical_actor_id: "builder-1",
      provider: "codex",
      runtime_session_id: null,
    }),
    "--phase",
    "planning",
    "--obligation-json",
    JSON.stringify({
      obligation_id: "implementation-1",
      kind: "implementation",
      summary: "Implement the accepted slice",
    }),
    "--authoritative-refs-json",
    JSON.stringify([{ kind: "accepted_plan", reference: "plan:plan-1" }]),
  ], { encoding: "utf8" }));
}

test("lists only the bounded read-only tool surface", async (t) => {
  const client = await connect();
  t.after(() => client.close());
  const listed = await client.listTools();
  assert.deepEqual(
    listed.tools.map((tool) => tool.name).sort(),
    [
      "list_active_slice_history",
      "read_active_slice_state",
    ],
  );
});

test("reads current state and retained history without mutating the repository", async (t) => {
  const repository = await mkdtemp(path.join(os.tmpdir(), "work-engine-mcp-state."));
  t.after(() => rm(repository, { recursive: true, force: true }));
  execFileSync("git", ["init", "-q", repository]);
  const active = begin(repository);
  execFileSync("python3", [
    MANAGE,
    "--repository",
    repository,
    "wait",
    "--identity-json",
    JSON.stringify(IDENTITY),
    "--expected-revision",
    active.durable_revision,
    "--event-id",
    "provider-wait-1",
    "--capability",
    "independent_review",
    "--provider",
    "claude",
    "--reason",
    "provider unavailable",
  ]);
  const before = execFileSync("git", ["-C", repository, "for-each-ref"], { encoding: "utf8" });
  const client = await connect(repository);
  t.after(() => client.close());

  const current = await client.callTool({
    name: "read_active_slice_state",
    arguments: { identity: IDENTITY },
  });
  assert.equal(current.structuredContent.result.authorization_scope, "read_only");
  assert.equal(current.structuredContent.result.state.phase, "planning");
  assert.equal(current.structuredContent.result.state.status, "waiting_on_capability");

  const history = await client.callTool({
    name: "list_active_slice_history",
    arguments: { identity: IDENTITY, limit: 10 },
  });
  assert.equal(history.structuredContent.result.items.length, 2);
  assert.equal(history.structuredContent.result.ordering, "newest_first");

  const selected = await client.callTool({
    name: "read_active_slice_state",
    arguments: { identity: IDENTITY, revision: active.durable_revision },
  });
  assert.equal(selected.structuredContent.result.query, "exact_revision");
  assert.equal(selected.structuredContent.result.state.status, "active");
  assert.equal(selected.structuredContent.result.state.durable_revision, active.durable_revision);
  const after = execFileSync("git", ["-C", repository, "for-each-ref"], { encoding: "utf8" });
  assert.equal(after, before);
});

test("projects production claim reads with exact CLI semantics and no mutation surface", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-mcp-claims."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const ids = createClaimRoot(directory);
  const client = await connect(ROOT, null, directory);
  t.after(() => client.close());

  const listed = await client.listTools();
  const claimTools = listed.tools.map((tool) => tool.name).filter((name) => name.includes("claim"));
  assert.deepEqual(claimTools.sort(), [
    "discover_claim_evidence",
    "query_claim_evidence_reliance",
    "resolve_claim_evidence",
    "traverse_claim_evidence",
  ]);
  assert.ok(claimTools.every((name) => !/(apply|init|publish|rebuild|write)/.test(name)));

  const cases = [
    {
      tool: "discover_claim_evidence",
      arguments: { criteria: { namespace: "proposal-research", profile: "proposal-research-v1" } },
      cli: ["discover", "--criteria-json", JSON.stringify({ namespace: "proposal-research", profile: "proposal-research-v1" })],
    },
    {
      tool: "resolve_claim_evidence",
      arguments: { identity: ids.research },
      cli: ["resolve", "--identity", ids.research],
    },
    {
      tool: "traverse_claim_evidence",
      arguments: { revision: ids.research, direction: "successors" },
      cli: ["traverse", "--revision", ids.research, "--direction", "successors"],
    },
    {
      tool: "query_claim_evidence_reliance",
      arguments: { revision: ids.research },
      cli: ["reliance", "--revision", ids.research],
    },
    {
      tool: "query_claim_evidence_reliance",
      arguments: { consumer: "proposal:mcp-consumer" },
      cli: ["reliance", "--consumer", "proposal:mcp-consumer"],
    },
  ];
  const before = execFileSync("find", [directory, "-printf", "%P %s %T@\n"], { encoding: "utf8" });
  for (const item of cases) {
    const response = await client.callTool({ name: item.tool, arguments: item.arguments });
    assert.equal(response.isError, undefined);
    assert.deepEqual(response.structuredContent.result, directClaim(directory, item.cli));
    assert.equal(response.structuredContent.result.projection_schema_version, 1);
    assert.equal(response.structuredContent.result.completeness, "partial");
    assert.deepEqual(response.structuredContent.result.excluded_inputs, ["excluded-source"]);
    assert.deepEqual(response.structuredContent.result.failed_inputs, ["failed-source"]);
    assert.equal(response.structuredContent.result.unresolved_references[0].status, "unavailable");
  }
  const exactResolution = await client.callTool({
    name: "resolve_claim_evidence", arguments: { identity: ids.research },
  });
  assert.equal(exactResolution.structuredContent.result.revision.confidence, Number.MAX_SAFE_INTEGER);
  const after = execFileSync("find", [directory, "-printf", "%P %s %T@\n"], { encoding: "utf8" });
  assert.equal(after, before);

  const emptyDiscovery = await client.callTool({
    name: "discover_claim_evidence", arguments: { criteria: {} },
  });
  assert.equal(emptyDiscovery.isError, true);
  const ambiguousReliance = await client.callTool({
    name: "query_claim_evidence_reliance",
    arguments: { revision: ids.research, consumer: "proposal:mcp-consumer" },
  });
  assert.equal(ambiguousReliance.isError, true);
});

test("recovers an authority-bound review episode and fences the prior writer", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-mcp-review."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const repository = path.join(directory, "repository");
  execFileSync("git", ["init", "-q", repository]);
  const authority1 = path.join(directory, "authority-1.json");
  await writeFile(authority1, JSON.stringify(reviewAuthority(1)));
  const refsBefore = execFileSync("git", ["-C", repository, "for-each-ref"], { encoding: "utf8" });
  const statusBefore = execFileSync("git", ["-C", repository, "status", "--short"], { encoding: "utf8" });

  const firstClient = await connect(repository, authority1);
  const tools = await firstClient.listTools();
  const advanceTool = tools.tools.find((tool) => tool.name === "advance_independent_review_episode");
  assert.ok(advanceTool);
  const payloadVariants = advanceTool.inputSchema.properties.payload.anyOf;
  const findingVariant = payloadVariants.find((variant) =>
    variant.properties?.findings?.items?.properties?.finding_id);
  assert.ok(findingVariant);
  assert.ok(findingVariant.properties.findings.items.required.includes("evidence_references"));
  const begun = await firstClient.callTool({
    name: "begin_independent_review_episode",
    arguments: { transition_id: "begin-1", evidence_references: [], claim_references: [], unresolved_questions: [] },
  });
  const first = begun.structuredContent.result;
  const evidence = exactReference("repository", "skills/subject.py", "blob-1");
  const initial = await firstClient.callTool({
    name: "advance_independent_review_episode",
    arguments: {
      expected_revision: first.durable_revision,
      transition_id: "initial-result-1",
      action: "record_initial_result",
      payload: {
        findings: [{ finding_id: "finding-1", attributed_reviewer: "reviewer-1", reviewer_generation: 1,
          severity: "high", observation: "bounded issue", evidence_references: [evidence],
          status: "open", remediation_references: [] }],
        unresolved_questions: [], evidence_references: [evidence], claim_references: [],
      },
    },
  });
  assert.equal(initial.structuredContent.result.pending_next_action, "await_remediation");
  const reviewedRevision = initial.structuredContent.result.durable_revision;
  await firstClient.close();

  const authority2 = path.join(directory, "authority-2.json");
  await writeFile(authority2, JSON.stringify(reviewAuthority(2, reviewedRevision)));
  const secondClient = await connect(repository, authority2);
  t.after(() => secondClient.close());
  const recovered = await secondClient.callTool({ name: "read_independent_review_episode", arguments: {} });
  assert.equal(recovered.structuredContent.result.findings[0].finding_id, "finding-1");
  const replaced = await secondClient.callTool({
    name: "advance_independent_review_episode",
    arguments: { expected_revision: reviewedRevision, transition_id: "replace-1", action: "replace_writer",
      payload: { reason: "reviewer runtime replaced", pending_next_action: "reconcile_then_resume" } },
  });
  assert.equal(replaced.structuredContent.result.continuity, "reconstructed_continuation");
  const replacementRevision = replaced.structuredContent.result.durable_revision;
  const remediation = exactReference("builder", "remediation-delta", "delta-2");
  const reevaluation = await secondClient.callTool({
    name: "advance_independent_review_episode",
    arguments: { expected_revision: replacementRevision, transition_id: "remediation-1",
      action: "record_remediation_subject",
      payload: { reviewed_subject: [remediation], evidence_references: [remediation] } },
  });
  assert.equal(reevaluation.structuredContent.result.pending_next_action, "re_evaluate_delta_in_same_session");
  const reported = await secondClient.callTool({
    name: "advance_independent_review_episode",
    arguments: { expected_revision: reevaluation.structuredContent.result.durable_revision,
      transition_id: "re-evaluation-1", action: "record_re_evaluation",
      payload: {
        findings: [{ finding_id: "finding-1", attributed_reviewer: "reviewer-1", reviewer_generation: 1,
          severity: "high", observation: "bounded issue", evidence_references: [evidence],
          status: "verified_resolved", remediation_references: [remediation] }],
        unresolved_questions: [], evidence_references: [remediation], claim_references: [],
      } },
  });
  assert.equal(reported.structuredContent.result.current_review_phase, "reported");
  const laterSubject = exactReference("checkpoint", "candidate", "tree-3");
  const laterReevaluation = await secondClient.callTool({
    name: "advance_independent_review_episode",
    arguments: { expected_revision: reported.structuredContent.result.durable_revision,
      transition_id: "remediation-2", action: "record_remediation_subject",
      payload: { reviewed_subject: [laterSubject], evidence_references: [laterSubject] } },
  });
  assert.equal(laterReevaluation.structuredContent.result.current_review_phase, "re_evaluation");
  assert.equal(laterReevaluation.structuredContent.result.continuity, "same_session");
  assert.equal(laterReevaluation.structuredContent.result.writer_binding.generation, 2);
  assert.equal(laterReevaluation.structuredContent.result.writer_binding.runtime_session_ref.reference,
    "session-2");
  assert.deepEqual(laterReevaluation.structuredContent.result.reviewed_subject, [laterSubject]);
  assert.equal(laterReevaluation.structuredContent.result.findings[0].status, "verified_resolved");
  const history = await secondClient.callTool({
    name: "list_independent_review_episode_history", arguments: { limit: 10 },
  });
  assert.equal(history.structuredContent.result.items.length, 6);

  const staleClient = await connect(repository, authority1);
  t.after(() => staleClient.close());
  const stale = await staleClient.callTool({
    name: "advance_independent_review_episode",
    arguments: { expected_revision: laterReevaluation.structuredContent.result.durable_revision,
      transition_id: "stale-writer", action: "mark_uncertain",
      payload: { reason: "must fail", reconciliation_action: "stop" } },
  });
  assert.equal(stale.isError, true);
  const forbidden = await secondClient.callTool({
    name: "advance_independent_review_episode",
    arguments: { expected_revision: laterReevaluation.structuredContent.result.durable_revision,
      transition_id: "claim-write", action: "record_re_evaluation", payload: { claim_revision: "forbidden" } },
  });
  assert.equal(forbidden.isError, true);

  const refsAfter = execFileSync("git", ["-C", repository, "for-each-ref"], { encoding: "utf8" });
  const newRefs = refsAfter.split("\n").filter((line) => line && !refsBefore.includes(line));
  assert.ok(newRefs.every((line) => line.includes("refs/work-engine/independent-review-state")));
  const statusAfter = execFileSync("git", ["-C", repository, "status", "--short"], { encoding: "utf8" });
  assert.equal(statusAfter, statusBefore);
});
