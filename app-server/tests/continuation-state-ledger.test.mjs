import assert from "node:assert/strict";
import test from "node:test";

import {
  appendLifecycleLedgerEntry,
  validateContinuationState,
  verifyContinuationState,
  verifyLifecycleLedger,
} from "../src/index.mjs";

const sha = (digit) => digit.repeat(64);
const ref = (reference, digit) => ({ reference, sha256: sha(digit) });

function continuationFixture() {
  return {
    schemaVersion: 1,
    type: "work-engine.continuation-state",
    subject: {
      logicalRoleInstanceId: "strategic-planner:main",
      runtimeBindingRevision: 4,
      sourceRevision: `sha256:${sha("a")}`,
    },
    compiledAt: "2026-08-25T12:00:00.000Z",
    compiler: {
      producer: "work-engine.semantic-context-compiler",
      model: "compiler-model",
      version: "1",
      inferenceId: "compiler-inference-1",
    },
    objective: {
      statement: "Keep the roadmap coherent",
      authorityRef: ref("user-message:objective", "b"),
    },
    workPosition: {
      phase: "implementation",
      currentUnit: "continuation-state-ledger",
    },
    completedConsequences: [{
      id: "consequence-1",
      statement: "Observed context was projected",
      sourceRefs: [ref("git:observed-context", "c")],
    }],
    activeCommitments: [{
      id: "commitment-1",
      statement: "Preserve human authority",
      sourceRefs: [ref("user-message:authority", "d")],
    }],
    decisions: [],
    humanInteractions: [{
      id: "interaction-1",
      kind: "implementation_authorization",
      status: "closed_but_active",
      authorityEffect: "authorizes the accepted App Server slice",
      durableConsequenceRef: ref("authority-record:slice", "e"),
      sourceRef: ref("user-message:proceed", "f"),
      nextContextDisposition: "compiled_consequence",
    }],
    authorityDependencies: {
      canonicalRecords: [ref("authority-record:slice", "e")],
      revalidationRequired: ["authorized next action"],
    },
    unresolved: [],
    governingEnvironment: {
      roleContract: ref("skills/strategic-planner/SKILL.md", "1"),
      instructionsToReload: [ref("AGENTS.md", "2")],
      activatedSkills: [ref("skills/strategic-planner/SKILL.md", "1")],
    },
    canonicalReferences: [ref("docs/roadmap.md", "3")],
    authorizedNextAction: {
      kind: "implement",
      objective: "Validate continuation state and ledger records",
      authorityRef: ref("authority-record:slice", "e"),
    },
    roleState: {
      schema: "strategic-planner-continuation-v1",
      value: { planningRevision: 7, disposition: "continue" },
    },
    uncertainty: [{
      id: "uncertainty-1",
      statement: "Durable storage is not implemented",
      sourceRefs: [ref("architecture-gap:ledger-storage", "4")],
    }],
  };
}

test("continuation-state-v1 validation is deterministic, immutable, and tamper evident", () => {
  const first = validateContinuationState(continuationFixture());
  const second = validateContinuationState(continuationFixture());

  assert.deepEqual(first, second);
  assert.match(first.candidateRevision, /^sha256:[a-f0-9]{64}$/);
  assert.equal(Object.hasOwn(first, "checkpointRevision"), false);
  assert.equal(first.humanInteractions[0].status, "closed_but_active");
  assert.equal(first.humanInteractions[0].nextContextDisposition, "compiled_consequence");
  assert.equal(Object.isFrozen(first), true);
  assert.equal(Object.isFrozen(first.roleState.value), true);
  assert.equal(verifyContinuationState(first), true);

  const tampered = structuredClone(first);
  tampered.authorizedNextAction.objective = "Act without canonical authority";
  assert.equal(verifyContinuationState(tampered), false);
});

test("continuation-state-v1 rejects unbound, duplicate, and unsupported semantic claims", () => {
  const unbound = continuationFixture();
  unbound.subject.sourceRevision = "observed-context-latest";
  assert.throws(() => validateContinuationState(unbound), /must be sha256-bound/);

  const duplicate = continuationFixture();
  duplicate.activeCommitments.push(structuredClone(duplicate.activeCommitments[0]));
  assert.throws(() => validateContinuationState(duplicate), /ids must be unique/);

  const inventedStatus = continuationFixture();
  inventedStatus.humanInteractions[0].status = "closed";
  assert.throws(() => validateContinuationState(inventedStatus), /status is unsupported/);

  const inventedDisposition = continuationFixture();
  inventedDisposition.humanInteractions[0].nextContextDisposition = "discard";
  assert.throws(() => validateContinuationState(inventedDisposition), /Disposition is unsupported/);

  const missingAuthority = continuationFixture();
  delete missingAuthority.authorizedNextAction.authorityRef;
  assert.throws(() => validateContinuationState(missingAuthority), /authorityRef must be an object/);
});

const ledgerSubject = {
  logicalRoleInstanceId: "strategic-planner:main",
  threadId: "thread-1",
  bindingRevision: 4,
};

function ledgerInput(eventType, status, second, details = {}) {
  return {
    eventType,
    status,
    recordedAt: `2026-08-25T12:00:0${second}.000Z`,
    subject: ledgerSubject,
    evidenceRefs: [`sha256:${sha(String(second))}`],
    details,
  };
}

test("lifecycle ledger appends and verifies a three-entry tamper-evident chain", () => {
  const observation = appendLifecycleLedgerEntry(
    null,
    ledgerInput("observation_recorded", "observed", 1, { sourceRevision: `sha256:${sha("a")}` }),
  );
  const candidate = appendLifecycleLedgerEntry(
    observation,
    ledgerInput("checkpoint_candidate_recorded", "observed", 2, { candidateRevision: `sha256:${sha("b")}` }),
  );
  const verification = appendLifecycleLedgerEntry(
    candidate,
    ledgerInput("verification_recorded", "accepted", 3, { verifier: "semantic-verifier-v1" }),
  );

  assert.equal(observation.sequence, 1);
  assert.equal(candidate.sequence, 2);
  assert.equal(verification.sequence, 3);
  assert.equal(candidate.previousRevision, observation.entryRevision);
  assert.equal(verification.previousRevision, candidate.entryRevision);
  assert.equal(Object.isFrozen(verification.details), true);
  assert.equal(verifyLifecycleLedger([observation, candidate, verification]), true);
  assert.equal(verifyLifecycleLedger([]), true);

  const tampered = structuredClone([observation, candidate, verification]);
  tampered[1].details.candidateRevision = `sha256:${sha("c")}`;
  assert.equal(verifyLifecycleLedger(tampered), false);

  const relinked = structuredClone([observation, candidate, verification]);
  relinked[2].previousRevision = observation.entryRevision;
  assert.equal(verifyLifecycleLedger(relinked), false);
});

test("lifecycle ledger status constraints prevent attempts from becoming success claims", () => {
  assert.throws(
    () => appendLifecycleLedgerEntry(null, ledgerInput("actuation_requested", "accepted", 1)),
    /cannot claim status accepted/,
  );
  assert.throws(
    () => appendLifecycleLedgerEntry(null, ledgerInput("checkpoint_published", "attempted", 1)),
    /cannot claim status attempted/,
  );
  assert.throws(
    () => appendLifecycleLedgerEntry(null, ledgerInput("transition_observed", "accepted", 1)),
    /cannot claim status accepted/,
  );
  assert.throws(
    () => appendLifecycleLedgerEntry(null, ledgerInput("failure_recorded", "observed", 1)),
    /cannot claim status observed/,
  );
});

test("ledger verification rejects undeclared fields and broken predecessor entries", () => {
  const first = appendLifecycleLedgerEntry(
    null,
    ledgerInput("observation_recorded", "observed", 1),
  );
  const withExtraField = { ...first, authoritative: true };
  assert.equal(verifyLifecycleLedger([withExtraField]), false);

  const brokenPrevious = structuredClone(first);
  brokenPrevious.details = { altered: true };
  assert.throws(
    () => appendLifecycleLedgerEntry(
      brokenPrevious,
      ledgerInput("checkpoint_candidate_recorded", "observed", 2),
    ),
    /revision does not match/,
  );
});
