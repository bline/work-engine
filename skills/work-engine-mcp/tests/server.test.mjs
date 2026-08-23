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

async function connect(repository = ROOT, authority = null) {
  const args = [SERVER, "--repository", repository];
  if (authority) args.push("--review-authority-file", authority);
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
      "query_claim_lineage_dogfood",
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

test("rejects empty claim filters instead of implying there are no matches", async (t) => {
  const client = await connect();
  t.after(() => client.close());
  const response = await client.callTool({
    name: "query_claim_lineage_dogfood",
    arguments: { ids: [] },
  });
  assert.equal(response.isError, true);
});

test("returns validated experimental claim records with visible limits", async (t) => {
  const client = await connect();
  t.after(() => client.close());
  const response = await client.callTool({
    name: "query_claim_lineage_dogfood",
    arguments: { types: ["claim_revision"], limit: 2 },
  });
  const value = response.structuredContent.result;
  assert.equal(value.authorization_scope, "read_only_experimental");
  assert.equal(value.records.length, 2);
  assert.equal(value.truncated, true);
  assert.match(value.completeness_boundary, /two pre-bound fixtures/);
  assert.ok(value.canonical_input_manifest[0].sha256);
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
  assert.ok(tools.tools.some((tool) => tool.name === "advance_independent_review_episode"));
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
  const history = await secondClient.callTool({
    name: "list_independent_review_episode_history", arguments: { limit: 10 },
  });
  assert.equal(history.structuredContent.result.items.length, 4);

  const staleClient = await connect(repository, authority1);
  t.after(() => staleClient.close());
  const stale = await staleClient.callTool({
    name: "advance_independent_review_episode",
    arguments: { expected_revision: reevaluation.structuredContent.result.durable_revision,
      transition_id: "stale-writer", action: "mark_uncertain",
      payload: { reason: "must fail", reconciliation_action: "stop" } },
  });
  assert.equal(stale.isError, true);
  const forbidden = await secondClient.callTool({
    name: "advance_independent_review_episode",
    arguments: { expected_revision: reevaluation.structuredContent.result.durable_revision,
      transition_id: "claim-write", action: "record_re_evaluation", payload: { claim_revision: "forbidden" } },
  });
  assert.equal(forbidden.isError, true);

  const refsAfter = execFileSync("git", ["-C", repository, "for-each-ref"], { encoding: "utf8" });
  const newRefs = refsAfter.split("\n").filter((line) => line && !refsBefore.includes(line));
  assert.ok(newRefs.every((line) => line.includes("refs/work-engine/independent-review-state")));
  const statusAfter = execFileSync("git", ["-C", repository, "status", "--short"], { encoding: "utf8" });
  assert.equal(statusAfter, statusBefore);
});
