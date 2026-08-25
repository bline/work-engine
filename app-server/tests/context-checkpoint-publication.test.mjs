import assert from "node:assert/strict";
import { createHash, generateKeyPairSync } from "node:crypto";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  ContextCheckpointPublisher,
  InMemoryContextCheckpointPublicationStore,
  SemanticContextInferenceRuntime,
  projectObservedContext,
  appendLifecycleLedgerEntry,
  verifyLifecycleLedger,
} from "../src/index.mjs";

const FIXTURES = new URL("./fixtures/semantic-context/", import.meta.url);
const digest = (value) => createHash("sha256").update(value, "utf8").digest("hex");
const revision = (digit) => `sha256:${digit.repeat(64)}`;
const AUTHORITY_REVISION = revision("e");
const AUTHORITY_EVIDENCE = revision("f");

const CONTENT = Object.freeze({
  human: "The user authorized the next bounded App Server slice.",
  evidence: "The observed-context projection and continuation schema tests pass.",
  skill: "The strategic planner is advisory and preserves canonical authority.",
  next: "Build the hidden semantic compiler and independent verifier against fixtures.",
});
const SHA = Object.freeze(Object.fromEntries(
  Object.entries(CONTENT).map(([key, value]) => [key, digest(value)]),
));

async function fixture(name, replacements = {}) {
  let value = await readFile(new URL(name, FIXTURES), "utf8");
  for (const [token, replacement] of Object.entries({
    HUMAN_SHA: SHA.human,
    EVIDENCE_SHA: SHA.evidence,
    SKILL_SHA: SHA.skill,
    NEXT_SHA: SHA.next,
    ...replacements,
  })) value = value.replaceAll(`__${token}__`, replacement);
  return value;
}

function observedFixture() {
  const keys = generateKeyPairSync("ed25519");
  const projection = projectObservedContext({
    logicalRoleInstanceId: "strategic-planner:main",
    runtimeBinding: {
      threadId: "thread-semantic-1",
      bindingRevision: 4,
      environmentRevision: "runtime-environment-1",
    },
    lastCompletedTurnId: "turn-7",
    visibleItems: [{
      identity: "user-message:proceed",
      origin: "human",
      trustClass: "human_authority_input",
      instructionApplicability: "contract_defined",
      contentRef: {
        kind: "thread-item",
        reference: "user-message:proceed",
        sha256: SHA.human,
      },
    }, {
      identity: "evidence:observed-context",
      origin: "tool",
      trustClass: "attributed_evidence",
      instructionApplicability: "none",
      producer: "app-server-test-gate",
      contentRef: {
        kind: "evidence",
        reference: "evidence:observed-context",
        sha256: SHA.evidence,
      },
    }],
    governingSources: [{
      identity: "role-contract",
      owner: "skills/strategic-planner/SKILL.md",
      contentRef: {
        kind: "skill",
        reference: "skills/strategic-planner/SKILL.md",
        sha256: SHA.skill,
      },
    }],
    activatedSkills: [{
      name: "strategic-planner",
      path: "skills/strategic-planner/SKILL.md",
      sha256: SHA.skill,
    }],
    lifecycleSnapshot: {
      schemaVersion: 1,
      threadId: "thread-semantic-1",
      retainedObservationCount: 2,
    },
    expectedNextWork: {
      reference: "expected-next-work:semantic-inference",
      sha256: SHA.next,
    },
    sourceInventoryCompleteness: "partial",
    omissions: [{
      scope: "provider-effective-prompt",
      reason: "provider does not expose exact effective input",
    }],
  }, {
    componentId: "work-engine.observed-context-projector",
    buildRevision: "fixture-build-1",
    keyId: "fixture-key-1",
    privateKey: keys.privateKey,
  });
  return { keys, projection };
}

function materials() {
  return Object.entries({
    human: ["thread-item", "user-message:proceed"],
    evidence: ["evidence", "evidence:observed-context"],
    skill: ["skill", "skills/strategic-planner/SKILL.md"],
    next: ["expected_next_work", "expected-next-work:semantic-inference"],
  }).map(([key, [kind, reference]]) => ({
    contentRef: { kind, reference, sha256: SHA[key] },
    content: CONTENT[key],
  }));
}

class RecordedInference {
  constructor(outputFactory, producer, inferenceId) {
    this.outputFactory = outputFactory;
    this.producer = producer;
    this.inferenceId = inferenceId;
  }

  async infer(request) {
    return {
      outputText: await this.outputFactory(request),
      provenance: {
        producer: this.producer,
        model: "fixture-model",
        version: "1",
        inferenceId: this.inferenceId,
      },
    };
  }
}

async function inspection(verifierFixture = "verifier-accepted.yaml") {
  const { keys, projection } = observedFixture();
  const compiler = new RecordedInference(
    () => fixture("compiler-valid.yaml"),
    "recorded-semantic-compiler",
    "compiler-inference-1",
  );
  const verifier = new RecordedInference(
    (request) => fixture(verifierFixture, {
      SOURCE_SHA: request.input.sourceRevision.slice("sha256:".length),
      CANDIDATE_SHA: request.input.candidate.candidateRevision.slice("sha256:".length),
    }),
    "recorded-semantic-verifier",
    "verifier-inference-1",
  );
  const timestamps = ["2026-08-25T18:00:00.000Z", "2026-08-25T18:00:01.000Z"];
  const result = await new SemanticContextInferenceRuntime({
    compiler,
    verifier,
    resolvePublicKey: (keyId) => keyId === "fixture-key-1" ? keys.publicKey : null,
    now: () => timestamps.shift(),
  }).inspect({ projection, sourceMaterials: materials() });
  return { ...result, keys, projection };
}

function storeFor(projection, overrides = {}) {
  return new InMemoryContextCheckpointPublicationStore([{
    logicalRoleInstanceId: projection.observedContext.logicalRoleInstanceId,
    threadId: projection.observedContext.runtimeBinding.threadId,
    bindingRevision: projection.observedContext.runtimeBinding.bindingRevision,
    sourceRevision: projection.sourceRevision,
    authorityRevision: AUTHORITY_REVISION,
    publicationRevision: null,
    ledgerRevision: null,
    ...overrides,
  }]);
}

function authorityResult(status = "current") {
  return async ({ references }) => ({
    status,
    authorityRevision: AUTHORITY_REVISION,
    checkedReferences: references,
    evidenceRefs: [AUTHORITY_EVIDENCE],
  });
}

function publisher({ store, keys, revalidateAuthority = authorityResult() }) {
  return new ContextCheckpointPublisher({
    store,
    resolvePublicKey: (keyId) => keyId === "fixture-key-1" ? keys.publicKey : null,
    revalidateAuthority,
    now: () => "2026-08-25T18:00:02.000Z",
  });
}

test("accepted verification publishes one revision-bound checkpoint and ledger entry", async () => {
  const inspected = await inspection();
  const store = storeFor(inspected.projection);
  const runtime = publisher({ store, keys: inspected.keys });
  const result = await runtime.publish({
    projection: inspected.projection,
    candidate: inspected.candidate,
    verification: inspected.verification,
  });

  assert.equal(result.status, "published");
  assert.match(result.publication.checkpointRevision, /^sha256:[a-f0-9]{64}$/);
  assert.equal(result.publication.continuationState, inspected.candidate);
  assert.equal(result.publication.verification, inspected.verification);
  assert.equal(result.ledgerEntry.eventType, "checkpoint_published");
  assert.equal(result.ledgerEntry.status, "observed");
  assert.equal(verifyLifecycleLedger([result.ledgerEntry]), true);
  assert.equal(
    store.snapshot("strategic-planner:main").publication.checkpointRevision,
    result.publication.checkpointRevision,
  );

  const duplicate = await runtime.publish({
    projection: inspected.projection,
    candidate: inspected.candidate,
    verification: inspected.verification,
    expectedPublicationRevision: result.publication.checkpointRevision,
    previousLedgerEntry: result.ledgerEntry,
  });
  assert.equal(duplicate.status, "rejected");
  assert.equal(duplicate.reason, "duplicate_candidate");
});

test("unresolved verification and invalid authority cannot publish", async () => {
  const unresolved = await inspection("verifier-unresolved.yaml");
  const unresolvedStore = storeFor(unresolved.projection);
  const unresolvedResult = await publisher({
    store: unresolvedStore,
    keys: unresolved.keys,
  }).publish({
    projection: unresolved.projection,
    candidate: unresolved.candidate,
    verification: unresolved.verification,
  });
  assert.deepEqual(unresolvedResult.status, "rejected");
  assert.equal(unresolvedResult.reason, "verification_not_accepted");
  assert.equal(unresolvedStore.snapshot("strategic-planner:main").publication, null);

  const accepted = await inspection();
  const authorityStore = storeFor(accepted.projection);
  const authorityResultValue = await publisher({
    store: authorityStore,
    keys: accepted.keys,
    revalidateAuthority: authorityResult("invalid"),
  }).publish({
    projection: accepted.projection,
    candidate: accepted.candidate,
    verification: accepted.verification,
  });
  assert.equal(authorityResultValue.status, "rejected");
  assert.equal(authorityResultValue.reason, "authority_invalid");
  assert.equal(authorityStore.snapshot("strategic-planner:main").publication, null);
});

test("atomic publication rejects stale source, binding, authority, and predecessor fences", async () => {
  const inspected = await inspection();
  for (const [overrides, expectedReason] of [
    [{ sourceRevision: revision("1") }, "stale_source_revision"],
    [{ bindingRevision: 5 }, "stale_runtime_binding"],
    [{ authorityRevision: revision("2") }, "stale_authority_revision"],
    [{ publicationRevision: revision("3") }, "publication_conflict"],
    [{ ledgerRevision: revision("4") }, "ledger_conflict"],
  ]) {
    const store = storeFor(inspected.projection, overrides);
    const result = await publisher({ store, keys: inspected.keys }).publish({
      projection: inspected.projection,
      candidate: inspected.candidate,
      verification: inspected.verification,
    });
    assert.equal(result.status, "rejected");
    assert.equal(result.reason, expectedReason);
    assert.equal(store.snapshot("strategic-planner:main").publication, null);
  }
});

test("authority revalidation must cover the exact candidate authority set", async () => {
  const inspected = await inspection();
  const store = storeFor(inspected.projection);
  await assert.rejects(
    publisher({
      store,
      keys: inspected.keys,
      revalidateAuthority: async ({ references }) => ({
        status: "current",
        authorityRevision: AUTHORITY_REVISION,
        checkedReferences: references.slice(1),
        evidenceRefs: [AUTHORITY_EVIDENCE],
      }),
    }).publish({
      projection: inspected.projection,
      candidate: inspected.candidate,
      verification: inspected.verification,
    }),
    /must cover the exact authority reference set/,
  );
  assert.equal(store.snapshot("strategic-planner:main").publication, null);
});

test("publication rejects a predecessor ledger entry owned by another lifecycle subject", async () => {
  const inspected = await inspection();
  const previous = appendLifecycleLedgerEntry(null, {
    eventType: "observation_recorded",
    status: "observed",
    recordedAt: "2026-08-25T17:59:59.000Z",
    subject: {
      logicalRoleInstanceId: "another-role:main",
      threadId: "another-thread",
      bindingRevision: 1,
    },
    evidenceRefs: [revision("9")],
    details: {},
  });
  const store = storeFor(inspected.projection, { ledgerRevision: previous.entryRevision });
  await assert.rejects(
    publisher({ store, keys: inspected.keys }).publish({
      projection: inspected.projection,
      candidate: inspected.candidate,
      verification: inspected.verification,
      previousLedgerEntry: previous,
    }),
    /ledger predecessor belongs to another subject/,
  );
  assert.equal(store.snapshot("strategic-planner:main").publication, null);
});

test("a fabricated store commit response cannot become publication evidence", async () => {
  const inspected = await inspection();
  const runtime = publisher({
    store: {
      compareAndSwapPublication: async ({ expectedFence }) => ({
        status: "committed",
        currentFence: expectedFence,
      }),
    },
    keys: inspected.keys,
  });
  await assert.rejects(
    runtime.publish({
      projection: inspected.projection,
      candidate: inspected.candidate,
      verification: inspected.verification,
    }),
    (error) => error.code === "invalid_store_receipt",
  );
});
