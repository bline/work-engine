import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { stringify as stringifyYaml } from "yaml";

import {
  CodexAppServerAdapter,
  ExactSkillResolver,
  FileRoleBindingRegistry,
  REQUEST_CONTEXT_INPUT_PREFIX,
  StrategicPlannerRuntime,
  loadRuntimeManifest,
} from "../src/index.mjs";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "../..");
const RUNTIME_MANIFEST = path.join(ROOT, "app-server/runtime-manifest.yaml");

class PlannerTransport {
  constructor() {
    this.requests = [];
    this.threadCount = 0;
    this.turnCount = 0;
    this.notificationHandlers = new Set();
  }

  onServerRequest(handler) {
    this.serverRequestHandler = handler;
  }

  onNotification(handler) {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }

  notify() {}

  async request(method, params) {
    this.requests.push({ method, params });
    if (method === "initialize") {
      return {
        userAgent: "work-engine/0.149.1 (Linux; x86_64)",
        codexHome: "/tmp/codex",
        platformFamily: "unix",
        platformOs: "linux",
      };
    }
    if (method === "thread/start") {
      this.threadCount += 1;
      return { thread: { id: `planner-thread-${this.threadCount}` } };
    }
    if (method === "thread/resume") return { thread: { id: params.threadId } };
    if (method === "turn/start") {
      this.turnCount += 1;
      const turnId = `planner-turn-${this.turnCount}`;
      assert.equal("additionalContext" in params, false);
      const contextItem = params.input.find(
        (item) => item.type === "text" && item.text.startsWith(REQUEST_CONTEXT_INPUT_PREFIX),
      );
      assert.ok(contextItem, "turn input must contain request-bound Work Engine context");
      const requestContext = JSON.parse(
        contextItem.text.slice(REQUEST_CONTEXT_INPUT_PREFIX.length),
      ).entries;
      const objective = requestContext["work-engine.strategic-objective"].value;
      const evidenceCutoff = JSON.parse(
        requestContext["work-engine.evidence-cutoff"].value,
      );
      const continuity = requestContext["work-engine.planner-continuity"].value;
      const output = stringifyYaml({
        schema_version: 1,
        strategic_objective: objective,
        evidence_cutoff: evidenceCutoff,
        continuity,
        verdict: "continue",
        current_rationale: "The current route still serves the objective.",
        assumptions: { confirmed: [], changed: [], invalidated: [] },
        route_changes: {
          priorities: [],
          dependencies: [],
          newly_important: [],
          deferred: [],
        },
        recommended_campaign: {
          disposition: "continue_current",
          objective: null,
          work_source: null,
          reason: "No strategic change is supported by the supplied evidence.",
        },
        open_uncertainties: [],
        authority_required: [],
        revisit_when: [],
      });
      queueMicrotask(() => {
        const notification = {
          method: "turn/completed",
          params: {
            threadId: params.threadId,
            turn: {
              id: turnId,
              items: [{
                type: "agentMessage",
                id: `planner-message-${this.turnCount}`,
                text: output,
                phase: "final_answer",
                memoryCitation: null,
                delivery: null,
              }],
              itemsView: "full",
              status: "completed",
              error: null,
              startedAt: 1,
              completedAt: 2,
              durationMs: 1000,
            },
          },
        };
        for (const handler of this.notificationHandlers) handler(notification);
      });
      return { turn: { id: turnId } };
    }
    throw new Error(`unexpected request ${method}`);
  }
}

function reference(revision = "roadmap-revision-1") {
  return {
    owner: "work-engine-roadmap",
    reference: "roadmap.md",
    revision,
    freshnessRule: "exact_git_revision",
  };
}

test("strategic planner binds a durable read-only role thread with exact evidence", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "strategic-planner-role."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const transport = new PlannerTransport();
  const registry = new FileRoleBindingRegistry(path.join(directory, "bindings.json"));
  const skillResolver = await ExactSkillResolver.create([path.join(ROOT, "skills")]);
  const adapter = new CodexAppServerAdapter({ transport, registry, skillResolver });
  await adapter.initialize();
  const planner = new StrategicPlannerRuntime({
    adapter,
    manifest: await loadRuntimeManifest(RUNTIME_MANIFEST),
  });

  const first = await planner.requestReview({
    instanceId: "standalone-roadmap",
    clientUserMessageId: "planner-review-1",
    strategicObjective: "Keep the standalone Work Engine route coherent",
    evidenceCutoff: {
      roadmapRevision: "roadmap-revision-1",
      repositoryRevision: "tree-revision-1",
    },
    canonicalReferences: [reference()],
    continuity: "initialized",
  });
  const second = await planner.requestReview({
    instanceId: "standalone-roadmap",
    clientUserMessageId: "planner-review-2",
    strategicObjective: "Keep the standalone Work Engine route coherent",
    evidenceCutoff: {
      roadmapRevision: "roadmap-revision-2",
      repositoryRevision: "tree-revision-2",
      campaignTerminals: [{ runId: "run-14", sliceNumber: 14, status: "accepted" }],
    },
    canonicalReferences: [reference("roadmap-revision-2")],
    continuity: "retained",
  });

  assert.equal(first.logicalRoleInstanceId, "strategic-planner:standalone-roadmap");
  assert.equal(first.threadId, second.threadId);
  assert.equal(first.completion.status, "completed");
  assert.equal(first.handoff.schema_version, 1);
  assert.equal(first.handoff.strategic_objective, "Keep the standalone Work Engine route coherent");
  assert.equal(second.handoff.continuity, "retained");
  const start = transport.requests.find(({ method }) => method === "thread/start");
  assert.equal(start.params.cwd, ROOT);
  assert.equal(start.params.approvalPolicy, "never");
  assert.equal(start.params.sandbox, "read-only");
  assert.match(start.params.developerInstructions, /planning handoff is advisory/);
  assert.match(start.params.developerInstructions, /thread history is reasoning context/);
  const turns = transport.requests.filter(({ method }) => method === "turn/start");
  assert.equal(turns.length, 2);
  assert.deepEqual(turns[0].params.input[0], {
    type: "skill",
    name: "strategic-planner",
    path: path.join(ROOT, "skills/strategic-planner/SKILL.md"),
  });
  const firstContext = JSON.parse(
    turns[0].params.input[1].text.slice(REQUEST_CONTEXT_INPUT_PREFIX.length),
  ).entries;
  assert.equal(
    firstContext["work-engine.planner-continuity"].value,
    "initialized",
  );
  const secondContext = JSON.parse(
    turns[1].params.input[1].text.slice(REQUEST_CONTEXT_INPUT_PREFIX.length),
  ).entries;
  assert.deepEqual(
    JSON.parse(secondContext["work-engine.evidence-cutoff"].value),
    {
      roadmap_revision: "roadmap-revision-2",
      repository_revision: "tree-revision-2",
      campaign_terminals: [{ run_id: "run-14", slice_number: 14, status: "accepted" }],
    },
  );
  assert.deepEqual(
    transport.requests.map(({ method }) => method),
    ["initialize", "thread/start", "turn/start", "thread/resume", "turn/start"],
  );
});

test("strategic planner rejects an unbound evidence request before provider delivery", async () => {
  let deliveries = 0;
  const planner = new StrategicPlannerRuntime({
    adapter: {
      deliverTurn: async () => { deliveries += 1; },
      waitForTurnCompletion: async () => { throw new Error("should not wait"); },
    },
    manifest: await loadRuntimeManifest(RUNTIME_MANIFEST),
  });
  await assert.rejects(
    planner.requestReview({
      instanceId: "standalone-roadmap",
      clientUserMessageId: "planner-review-invalid",
      strategicObjective: "Choose the next campaign",
      evidenceCutoff: {},
      canonicalReferences: [],
      continuity: "initialized",
    }),
    /at least one durable reference/,
  );
  assert.equal(deliveries, 0);
});

test("strategic planner rejects a structurally valid handoff bound to stale evidence", async () => {
  const output = stringifyYaml({
    schema_version: 1,
    strategic_objective: "A different objective",
    evidence_cutoff: {
      roadmap_revision: "roadmap-revision-1",
      repository_revision: "tree-revision-1",
    },
    continuity: "initialized",
    verdict: "continue",
    current_rationale: "This valid-looking handoff belongs to another request.",
    recommended_campaign: {
      disposition: "none",
      objective: null,
      work_source: null,
      reason: "No campaign recommendation.",
    },
  });
  const planner = new StrategicPlannerRuntime({
    adapter: {
      deliverTurn: async () => ({
        logicalRoleInstanceId: "strategic-planner:standalone-roadmap",
        threadId: "thread-1",
        turnId: "turn-1",
        replayedDelivery: false,
      }),
      waitForTurnCompletion: async () => ({ outputText: output }),
    },
    manifest: await loadRuntimeManifest(RUNTIME_MANIFEST),
  });
  await assert.rejects(
    planner.requestReview({
      instanceId: "standalone-roadmap",
      clientUserMessageId: "planner-review-stale",
      strategicObjective: "Keep the standalone Work Engine route coherent",
      evidenceCutoff: {
        roadmapRevision: "roadmap-revision-1",
        repositoryRevision: "tree-revision-1",
      },
      canonicalReferences: [reference()],
      continuity: "initialized",
    }),
    /does not match the requested objective/,
  );
});
