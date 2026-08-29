import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  CodexAppServerAdapter,
  DEVELOPMENT_CLAIM_CONTEXT_KIND,
  DEVELOPMENT_CLAIM_CONTEXT_NAME,
  ExactSkillResolver,
  FileRoleBindingRegistry,
  ManifestRoleRuntime,
  loadRuntimeManifest,
  projectDevelopmentClaimContext,
} from "../../../src/index.mjs";
import { readClaimEvidence } from "../../../src/services/claim-evidence/read-service.mjs";
import { openSqliteClaimEvidenceStore } from "../../../src/services/claim-evidence/sqlite-store.mjs";

const CONTEXT_PREFIX = "WORK_ENGINE_REQUEST_CONTEXT_V1\n";
const repositoryRoot = path.resolve(new URL("../../../..", import.meta.url).pathname);

const exactReference = (reference, byte = "a") => ({
  owner: "repository",
  reference,
  revision: "exact-revision-1",
  integrity_sha256: byte.repeat(64),
  freshness: "current",
  status: "verified",
});

const authority = {
  schema_version: 1,
  grant_id: "grant:development-claim-consumer",
  actor: "producer:development-claim-consumer",
  profile: "proposal-research-v1",
  permissions: ["create_claim", "record_reliance"],
  decision_scope: "slice-implementation",
  authority_reference: exactReference("authority/development-claim-consumer.json", "b"),
};

const claimOperation = {
  schema_version: 1,
  operation_id: "publish:development-claim",
  action: "create_claim",
  profile: "proposal-research-v1",
  expected_state: null,
  payload: {
    subject: {
      namespace: "proposal-research",
      subject_kind: "proposal",
      stable_subject_id: "app-server-development-placement",
      evidence_baseline: exactReference("proposals/app-server-development.md", "c"),
      content_set: ["proposals/app-server-development.md"],
    },
    statement_identity: "The selected slice may be implemented through App Server",
    initial_revision: {
      proposition: "The selected slice may be implemented through App Server",
      support_qualification: "supported",
      assumptions: [],
      limitations: ["This claim does not accept a slice plan or authorize implementation"],
      confidence: { estimate: 0.9 },
      evidence_references: [exactReference("proposals/app-server-development.md", "c")],
      sensitivity_references: [],
      evidence_mode: "direct_source",
      judgment_kind: "semantic",
      decision_scope: "slice-implementation",
      profile_payload: { materiality: "material", support_qualification: "supported" },
      reopening_conditions: [],
      tombstone: false,
    },
  },
};

class ClaimConsumerTransport {
  constructor() {
    this.requests = [];
  }

  onServerRequest() {}
  onNotification() { return () => {}; }
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
    if (method === "thread/start") return { thread: { id: "claim-consumer-thread" } };
    if (method === "turn/start") return { turn: { id: "claim-consumer-turn" } };
    throw new Error(`unexpected request ${method}`);
  }
}

async function temporaryDatabase(t) {
  const directory = await mkdtemp(path.join(tmpdir(), "development-claim-context-test-"));
  t.after(async () => rm(directory, { recursive: true, force: true }));
  return { directory, filePath: path.join(directory, "claims.sqlite3") };
}

function contextRequest(revisionId, overrides = {}) {
  return {
    schema_version: 1,
    request_id: "development-claim-context:one",
    consumer: {
      identity: "slice-builder:claim-reachability",
      revision: "candidate-tree-1",
      decision_scope: "slice-implementation",
    },
    selections: [{
      revision_id: revisionId,
      selection_reason: "governs the bounded implementation placement",
    }],
    ...overrides,
  };
}

test("a real retained builder turn receives one exact claim projection and reliance survives restart", async (t) => {
  const { directory, filePath } = await temporaryDatabase(t);
  let store = await openSqliteClaimEvidenceStore({
    filePath,
    bootstrapAuthorities: [authority],
  });
  const revisionId = store.publish(claimOperation, authority).result_identity;
  const projected = projectDevelopmentClaimContext(store, contextRequest(revisionId));
  assert.equal(projected.context.relevant_exact_revisions[0].revision.id, revisionId);
  assert.equal(projected.context.projection.freshness, "current_after_verified_rebuild");
  assert.equal(projected.context.projection.completeness, "available");
  assert.equal(Object.isFrozen(projected.context.relevant_exact_revisions[0].revision), true);
  assert.deepEqual(Object.keys(projected.request_context), [DEVELOPMENT_CLAIM_CONTEXT_NAME]);

  const transport = new ClaimConsumerTransport();
  const adapter = new CodexAppServerAdapter({
    transport,
    registry: new FileRoleBindingRegistry(path.join(directory, "bindings.json")),
    skillResolver: await ExactSkillResolver.create([path.join(repositoryRoot, "skills")]),
  });
  await adapter.initialize();
  const runtime = new ManifestRoleRuntime({
    adapter,
    manifest: await loadRuntimeManifest(path.join(repositoryRoot, "app-server/runtime-manifest.yaml")),
  });
  const delivery = await runtime.deliverTurn({
    roleId: "slice-builder",
    instanceId: "claim-reachability",
    clientUserMessageId: "claim-reachability-1",
    text: "Build only the accepted bounded slice.",
    requestContext: projected.request_context,
  });
  assert.equal(delivery.logicalRoleInstanceId, "slice-builder:claim-reachability");

  const turnStart = transport.requests.find(({ method }) => method === "turn/start");
  const contextInput = turnStart.params.input.find(({ text }) => text?.startsWith(CONTEXT_PREFIX));
  const envelope = JSON.parse(contextInput.text.slice(CONTEXT_PREFIX.length));
  const deliveredEntry = envelope.entries[DEVELOPMENT_CLAIM_CONTEXT_NAME];
  assert.equal(deliveredEntry.kind, DEVELOPMENT_CLAIM_CONTEXT_KIND);
  const deliveredContext = JSON.parse(deliveredEntry.value);
  assert.equal(deliveredContext.relevant_exact_revisions[0].revision.id, revisionId);
  assert.equal(deliveredContext.consumer.identity, delivery.logicalRoleInstanceId);
  assert.equal("permissions" in deliveredContext.relevant_exact_revisions[0], false);
  assert.equal("operation" in deliveredContext.relevant_exact_revisions[0], false);
  assert.equal("store" in deliveredContext, false);

  const reliance = store.publish({
    schema_version: 1,
    operation_id: "rely:development-claim-context:one",
    action: "record_reliance",
    profile: "proposal-research-v1",
    expected_state: null,
    payload: {
      consumer: deliveredContext.consumer.identity,
      consumer_revision: deliveredContext.consumer.revision,
      decision_scope: deliveredContext.consumer.decision_scope,
      claim_revision_id: revisionId,
      state: "active",
      predecessor_reliance: null,
    },
  }, authority);
  const direct = readClaimEvidence(store, {
    schema_version: 1,
    request_id: "direct-reliance",
    operation: "query_direct_reliance",
    parameters: { revision_id: revisionId, limit: 10, cursor: null },
  });
  assert.equal(direct.outcome, "succeeded");
  assert.equal(direct.result.reliances[0].id, reliance.result_identity);
  const reverse = readClaimEvidence(store, {
    schema_version: 1,
    request_id: "reverse-reliance",
    operation: "query_reverse_reliance",
    parameters: { consumer: deliveredContext.consumer.identity, limit: 10, cursor: null },
  });
  assert.equal(reverse.outcome, "succeeded");
  assert.equal(reverse.result.reliances[0].claim_revision_id, revisionId);
  store.close();

  store = await openSqliteClaimEvidenceStore({ filePath });
  t.after(() => store.close());
  const recovered = readClaimEvidence(store, {
    schema_version: 1,
    request_id: "recovered-reliance",
    operation: "query_reverse_reliance",
    parameters: { consumer: deliveredContext.consumer.identity, limit: 10, cursor: null },
  });
  assert.equal(recovered.outcome, "succeeded");
  assert.equal(recovered.result.reliances[0].id, reliance.result_identity);
});

test("development claim context refuses missing revisions and mismatched decision scope", async (t) => {
  const { filePath } = await temporaryDatabase(t);
  const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  t.after(() => store.close());
  const revisionId = store.publish(claimOperation, authority).result_identity;

  assert.throws(
    () => projectDevelopmentClaimContext(store, contextRequest("missing-revision")),
    /relevant revision not found/,
  );
  assert.throws(
    () => projectDevelopmentClaimContext(store, contextRequest(revisionId, {
      consumer: {
        identity: "slice-builder:claim-reachability",
        revision: "candidate-tree-1",
        decision_scope: "different-scope",
      },
    })),
    /decision scope does not match/,
  );
  assert.throws(() => store.publish({
    schema_version: 1,
    operation_id: "unauthorized-reliance",
    action: "record_reliance",
    profile: "proposal-research-v1",
    expected_state: null,
    payload: {
      consumer: "slice-builder:claim-reachability",
      consumer_revision: "candidate-tree-1",
      decision_scope: "slice-implementation",
      claim_revision_id: revisionId,
      state: "active",
      predecessor_reliance: null,
    },
  }, {
    ...authority,
    grant_id: "grant:not-admitted",
    actor: "producer:not-admitted",
  }), /not admitted/);
  assert.equal(store.exportStore().reliances.length, 0);
});
