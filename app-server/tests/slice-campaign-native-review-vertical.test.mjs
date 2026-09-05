import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  createImplementationReviewService, createNativeReviewClosureService,
  createReviewEpisodeService, createReviewFindingBridge,
  createSliceCampaignService, openSqliteClaimEvidenceStore,
  openSqliteReviewEpisodeStore, openSqliteSliceCampaignStore,
  readClaimEvidence, reviewEpisodeDigest,
} from "../src/index.mjs";

const root = path.resolve(new URL("../..", import.meta.url).pathname);
const load = async (name) => JSON.parse(await readFile(path.join(
  root, "app-server/tests/fixtures/slice-campaign-native-review", `${name}.json`,
), "utf8"));
const exactReference = (owner, reference, revision, integritySha256) => ({
  owner, reference, revision, integrity_sha256: integritySha256,
  freshness: "exact immutable revision", status: "verified",
});
const episodeReference = (owner, reference, revision, sha256) => ({
  owner, reference, revision, sha256, freshness: "exact immutable revision",
});
const identity = {runId: "s12-run", sliceNumber: 4, attemptId: "attempt-1", planVersion: "s12-plan-v1-accepted"};
const subject = (result) => result.subject;

function findingAuthority() {
  return {
    schema_version: 1, grant_id: "grant:s12-finding", actor: "producer:s12-review-bridge",
    profile: "revision-bound-review-finding-v1",
    permissions: ["create_claim", "publish_revision", "record_reliance"],
    decision_scope: "s12-native-review",
    authority_reference: exactReference("supervisor", "authority/s12-finding.json", "accepted-plan", "a".repeat(64)),
  };
}

function episodeAuthority(result, {campaignIdentity = identity, obligationId = "generic", episodeId = "episode-1"} = {}) {
  const episodeIdentity = {...campaignIdentity, reviewObligationId: obligationId, reviewEpisodeId: episodeId};
  return {
    schemaVersion: 1, grantId: "grant:s12-episode", identity: episodeIdentity,
    source: episodeReference("slice-supervisor", "selection:s12:generic", "selection-v1", "d".repeat(64)),
    writer: {actorId: "reviewer", provider: "fixture", generation: 1,
      runtimeSession: episodeReference("reviewer-runtime", "session-1", "generation-1", "e".repeat(64))},
    readers: ["reviewer", "builder", "supervisor"],
    initialSubject: episodeReference("checkpoint", result.subject.commit, result.subject.tree, reviewEpisodeDigest(result.subject)),
    predecessorRevision: null,
  };
}

function candidate(result) {
  return {commit: result.subject.commit, tree: result.subject.tree, manifestSha256: result.subject.patchIdentity};
}

function selection(result) {
  return {
    schemaVersion: 1, owner: "slice-supervisor", selectionId: "selection:s12:v1",
    subject: result.subject,
    specialists: [{obligationId: "generic", skill: "implementation-review", selection: "selected"}],
  };
}

async function stores(t, directory, {bootstrap = false} = {}) {
  const implementationReview = createImplementationReviewService();
  const episodeStore = await openSqliteReviewEpisodeStore({filePath: path.join(directory, "episodes.sqlite")});
  const campaignStore = await openSqliteSliceCampaignStore({filePath: path.join(directory, "campaign.sqlite")});
  const authority = findingAuthority();
  const claimStore = await openSqliteClaimEvidenceStore({
    filePath: path.join(directory, "claims.sqlite"),
    ...(bootstrap ? {bootstrapAuthorities: [authority]} : {}),
  });
  t.after(() => { episodeStore.close(); campaignStore.close(); claimStore.close(); });
  return {implementationReview, episodeStore, campaignStore, claimStore, authority};
}

test("resolved native findings become reported only after every exact builder reliance", () => {
  const reliance = (finding) => Object.freeze({...finding, relianceRef: episodeReference(
    "claim-evidence", `reliance:${finding.findingId}`, `reliance:${finding.findingId}`, "f".repeat(64),
  )});
  const closure = createNativeReviewClosureService({
    reviewEpisode: {begin() {}, transition() {}, recover() {}, read() {}},
    reviewer: {review() {}},
    findingBridge: {publishFindings() {}, project() {}, recordReliance: ({finding}) => reliance(finding)},
  });
  const resolved = (findingId) => ({findingId, outcome: "verified_resolved",
    revisionRef: episodeReference("claim-evidence", `finding:${findingId}`, `revision:${findingId}`, "e".repeat(64)),
    relianceRef: null});
  const request = {authority: {}, consumer: "slice-builder:test", consumerRevision: "candidate-tree",
    decisionScope: "slice-campaign-native-review"};
  let binding = {schemaVersion: 1, obligationId: "generic", status: "awaiting_builder",
    findings: [resolved("F1"), resolved("F2")]};
  binding = closure.recordBuilderEvaluation({...request, binding,
    operationId: "rely:F1", findingId: "F1"});
  assert.equal(binding.status, "awaiting_builder");
  binding = closure.recordBuilderEvaluation({...request, binding,
    operationId: "rely:F2", findingId: "F2"});
  assert.equal(binding.status, "reported");

  const open = {...resolved("F3"), outcome: "open"};
  binding = closure.recordBuilderEvaluation({...request,
    binding: {schemaVersion: 1, obligationId: "generic", status: "awaiting_builder", findings: [open]},
    operationId: "rely:F3", findingId: "F3"});
  assert.equal(binding.status, "awaiting_builder");
});

test("campaign terminalization admits a reliance-complete resolved native closure", async () => {
  const initialResult = await load("initial-finding");
  const reliance = (finding) => Object.freeze({...finding, relianceRef: episodeReference(
    "claim-evidence", `reliance:${finding.findingId}`, `reliance:${finding.findingId}`, "f".repeat(64),
  )});
  const closure = createNativeReviewClosureService({
    reviewEpisode: {begin() {}, transition() {}, recover() {}, read() {}},
    reviewer: {review() {}},
    findingBridge: {publishFindings() {}, project() {}, recordReliance: ({finding}) => reliance(finding)},
  });
  const resolved = (findingId) => ({findingId, outcome: "verified_resolved",
    revisionRef: episodeReference("claim-evidence", `finding:${findingId}`, `revision:${findingId}`, "e".repeat(64)),
    relianceRef: null});
  const nativeReview = {
    async executeInitial({obligationId}) {
      return {binding: {schemaVersion: 1, obligationId, status: "awaiting_builder",
        findings: [resolved("F1"), resolved("F2")]}, builderContext: {}};
    },
    recordBuilderEvaluation: (request) => closure.recordBuilderEvaluation(request),
  };
  const campaignIdentity = {...identity, attemptId: "reliance-complete"};
  const service = createSliceCampaignService({nativeReview,
    reviewSubject: {async createCandidate(request) { return request; },
      async createPhysicalProfile({subject: value}) { return {subject: value}; }},
    receiptFinalizer: {async finalize(value) { return value; }},
  });
  let state = service.admit({identity: campaignIdentity, workspace: "/s12-reliance-complete",
    acceptedBoundary: {reference: "plan:s12", sha256: "1".repeat(64)},
    baseline: {acceptedCommit: "baseline", acceptedTree: "baseline-tree", interSliceCommit: "inter"}});
  state = service.advance({identity: campaignIdentity, expectedRevision: state.revision,
    phase: "implementing", consequence: {}});
  state = service.advance({identity: campaignIdentity, expectedRevision: state.revision,
    phase: "gate_ready", consequence: {}});
  state = await service.bindCandidate({identity: campaignIdentity, expectedRevision: state.revision,
    request: candidate(initialResult)});
  state = service.advance({identity: campaignIdentity, expectedRevision: state.revision,
    phase: "review_ready", consequence: {}});
  state = service.bindReviewSelection({identity: campaignIdentity, expectedRevision: state.revision,
    selection: selection(initialResult)});
  state = (await service.runNativeReview({identity: campaignIdentity, expectedRevision: state.revision,
    request: {obligationId: "generic"}})).campaign;
  state = service.recordNativeFindingEvaluation({identity: campaignIdentity, expectedRevision: state.revision,
    request: {obligationId: "generic", authority: {}, operationId: "rely:F1", findingId: "F1",
      consumer: "slice-builder:test", consumerRevision: initialResult.subject.tree,
      decisionScope: "slice-campaign-native-review"}}).campaign;
  assert.equal(state.nativeReview.obligations.generic.status, "awaiting_builder");
  await assert.rejects(service.terminalize({identity: campaignIdentity, expectedRevision: state.revision,
    outcome: "accepted", receipt: {}}), /completed native closure/);
  state = service.recordNativeFindingEvaluation({identity: campaignIdentity, expectedRevision: state.revision,
    request: {obligationId: "generic", authority: {}, operationId: "rely:F2", findingId: "F2",
      consumer: "slice-builder:test", consumerRevision: initialResult.subject.tree,
      decisionScope: "slice-campaign-native-review"}}).campaign;
  assert.equal(state.nativeReview.obligations.generic.status, "reported");
  state = await service.terminalize({identity: campaignIdentity, expectedRevision: state.revision,
    outcome: "accepted", receipt: {status: "accepted"}});
  assert.equal(state.phase, "terminal");
});

test("native campaign closes one claim-backed finding through remediation and restart", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "s12-native-review."));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const [initialResult, remediatedResult] = await Promise.all([load("initial-finding"), load("remediated-finding")]);
  let owners = await stores(t, directory, {bootstrap: true});
  let delivered = initialResult;
  const reviewer = {async review({subject: expected, claimContext = null}) {
    assert.deepEqual(expected, subject(delivered));
    if (claimContext !== null) assert.equal(claimContext.context_kind, "reviewer_relevant_exact_claim_revisions");
    return {attemptId: `attempt:${delivered.subject.commit}`, result: structuredClone(delivered)};
  }};
  const reviewEpisode = createReviewEpisodeService({store: owners.episodeStore, implementationReview: owners.implementationReview});
  const findingBridge = createReviewFindingBridge({store: owners.claimStore});
  const closure = createNativeReviewClosureService({reviewEpisode, reviewer, findingBridge});
  let interruptBeforeRemediationTransition = true;
  const nativeReview = {...closure, async executeRemediation(request) {
    if (interruptBeforeRemediationTransition) {
      interruptBeforeRemediationTransition = false;
      throw new Error("simulated host loss before remediation subject transition");
    }
    return closure.executeRemediation(request);
  }};
  const service = createSliceCampaignService({
    store: owners.campaignStore, implementationReview: owners.implementationReview, nativeReview,
    reviewSubject: {async createCandidate(request) { return request; }, async createPhysicalProfile({subject: value}) { return {subject: value}; }},
    legacyReview: {async review() { return {status: "passed"}; }},
    receiptFinalizer: {async finalize(value) { return {terminalDigest: reviewEpisodeDigest(value)}; }},
  });
  let state = service.admit({identity, workspace: directory,
    acceptedBoundary: {reference: "plan:s12", sha256: "1".repeat(64)},
    baseline: {acceptedCommit: "baseline", acceptedTree: "baseline-tree", interSliceCommit: "inter"}});
  state = service.advance({identity, expectedRevision: state.revision, phase: "implementing", consequence: {implementation: "ready"}});
  state = service.advance({identity, expectedRevision: state.revision, phase: "gate_ready", consequence: {gate: "passed"}});
  state = await service.bindCandidate({identity, expectedRevision: state.revision, request: candidate(initialResult)});
  state = service.advance({identity, expectedRevision: state.revision, phase: "review_ready", consequence: {candidate: "bound"}});
  state = service.bindReviewSelection({identity, expectedRevision: state.revision, selection: selection(initialResult)});
  const authority = episodeAuthority(initialResult);
  let outcome = await service.runNativeReview({identity, expectedRevision: state.revision, request: {
    obligationId: "generic", authority, beginTransitionId: "episode:begin", resultTransitionId: "episode:initial",
    reviewerRequest: {subject: initialResult.subject}, findingAuthority: owners.authority,
    operationPrefix: "s12:initial", contextRequest: {requestId: "s12:builder:initial", consumer: {
      identity: "slice-builder:s12", revision: initialResult.subject.tree, decision_scope: "s12-native-review"},
      limitations: ["Claims are evidence records, not review acceptance."]},
  }});
  state = outcome.campaign;
  assert.equal(outcome.builderContext.relevant_exact_revisions.length, 1);
  assert.equal("permissions" in outcome.builderContext.relevant_exact_revisions[0], false);
  assert.equal("store" in outcome.builderContext, false);
  assert.equal(state.nativeReview.obligations.generic.status, "awaiting_builder");
  assert.equal("result" in state.nativeReview.obligations.generic, false);

  outcome = service.recordNativeFindingEvaluation({identity, expectedRevision: state.revision, request: {
    obligationId: "generic", authority: owners.authority, operationId: "s12:rely:S12-001",
    findingId: "S12-001", consumer: "slice-builder:s12", consumerRevision: initialResult.subject.tree,
    decisionScope: "s12-native-review",
  }});
  state = outcome.campaign;
  assert.equal(state.nativeReview.obligations.generic.findings[0].relianceRef !== null, true);
  assert.throws(() => service.recordNativeFindingEvaluation({identity, expectedRevision: state.revision, request: {
    obligationId: "generic", authority: owners.authority, operationId: "s12:rely:S12-001:second",
    findingId: "S12-001", consumer: "slice-builder:s12", consumerRevision: initialResult.subject.tree,
    decisionScope: "s12-native-review",
  }}), /already has builder reliance/);
  assert.equal(readClaimEvidence(owners.claimStore, {schema_version: 1, request_id: "reliance", operation: "query_reverse_reliance",
    parameters: {consumer: "slice-builder:s12", limit: 10, cursor: null}}).result.reliances.length, 1);

  const lineageOmittingResult = structuredClone(remediatedResult);
  lineageOmittingResult.findings = [];
  delivered = lineageOmittingResult;
  let reviewerConsumer = null;
  reviewer.review = async ({subject: expected, claimContext = null}) => {
    assert.deepEqual(expected, subject(delivered));
    if (claimContext !== null) reviewerConsumer = claimContext.consumer;
    return {attemptId: `attempt:${delivered.subject.commit}`, result: structuredClone(delivered),
      runtimeSessionId: "session-1", receipt: {transportReceiptDigest: "f".repeat(64)}};
  };
  const remediationRequest = {
    obligationId: "generic", authority, subjectTransitionId: "episode:candidate-2",
    resultTransitionId: "episode:remediated", remediationSubject: remediatedResult.subject,
    reviewerRequest: {subject: remediatedResult.subject, continuationSessionId: "session-1"},
    findingAuthority: owners.authority, operationPrefix: "s12:remediated",
    contextRequest: {requestId: "s12:builder:remediated", consumer: {
      identity: "slice-builder:s12", revision: remediatedResult.subject.tree, decision_scope: "s12-native-review"},
      limitations: ["Claims are evidence records, not review acceptance."]},
  };
  const remediationSubjectReference = episodeReference(
    "checkpoint", remediatedResult.subject.commit, remediatedResult.subject.tree,
    reviewEpisodeDigest(remediatedResult.subject),
  );
  await assert.rejects(service.runNativeRemediation({identity, expectedRevision: state.revision,
    request: remediationRequest, remediationSubjectReference}),
  /simulated host loss before remediation subject transition/);
  state = service.recover(identity);
  assert.equal(state.nativeReview.obligations.generic.status, "remediation_executing");
  assert.equal(closure.recoverRemediation({binding: state.nativeReview.obligations.generic,
    authority, subjectTransitionId: remediationRequest.subjectTransitionId,
    resultTransitionId: remediationRequest.resultTransitionId}).providerEntry, "not_entered");
  outcome = await service.runNativeRemediation({identity, expectedRevision: state.revision,
    request: remediationRequest, remediationSubjectReference});
  state = outcome.campaign;
  assert.equal(outcome.failure.failureSignature, "result_contract_rejected");
  assert.equal(state.nativeReview.obligations.generic.status, "correction_required");
  assert.equal(state.nativeReview.obligations.generic.failure.recovery.rejectedResult.findings.length, 0);
  assert.equal(state.nativeReview.obligations.generic.findings[0].findingId, "S12-001");
  delivered = remediatedResult;
  outcome = await service.correctNativeReviewResult({identity, expectedRevision: state.revision,
    request: {...remediationRequest, resultTransitionId: "episode:remediated:corrected"},
    recovery: state.nativeReview.obligations.generic.failure.recovery});
  state = outcome.campaign;
  assert.deepEqual(reviewerConsumer, {identity: "reviewer", revision: remediatedResult.subject.tree,
    decision_scope: "s12-native-review"});
  assert.equal(outcome.builderContext.consumer.identity, "slice-builder:s12");
  assert.equal(state.nativeReview.obligations.generic.status, "reported");
  assert.equal(state.nativeReview.obligations.generic.findings[0].outcome, "verified_resolved");
  assert.notEqual(state.nativeReview.obligations.generic.findings[0].revisionRef.revision,
    state.nativeReview.obligations.generic.findings[0].initialRevisionRef.revision);
  assert.equal(state.nativeReview.obligations.generic.findings[0].relianceRef, null);
  outcome = service.recordNativeFindingEvaluation({identity, expectedRevision: state.revision, request: {
    obligationId: "generic", authority: owners.authority, operationId: "s12:rely:S12-001:remediated",
    findingId: "S12-001", consumer: "slice-builder:s12", consumerRevision: remediatedResult.subject.tree,
    decisionScope: "s12-native-review",
  }});
  state = outcome.campaign;
  assert.equal(state.nativeReview.obligations.generic.findings[0].relianceRef !== null, true);
  assert.equal(readClaimEvidence(owners.claimStore, {schema_version: 1, request_id: "reliance-after-remediation",
    operation: "query_reverse_reliance", parameters: {consumer: "slice-builder:s12", limit: 10, cursor: null},
  }).result.reliances.length, 2);
  const accepted = await service.terminalize({identity, expectedRevision: state.revision, outcome: "accepted", receipt: {status: "accepted"}});
  assert.equal(accepted.phase, "terminal");

  owners.episodeStore.close(); owners.campaignStore.close(); owners.claimStore.close();
  owners = await stores(t, directory);
  const recoveredEpisode = createReviewEpisodeService({store: owners.episodeStore, implementationReview: owners.implementationReview});
  const recoveredCampaign = createSliceCampaignService({store: owners.campaignStore,
    implementationReview: owners.implementationReview,
    nativeReview: createNativeReviewClosureService({reviewEpisode: recoveredEpisode, reviewer, findingBridge: createReviewFindingBridge({store: owners.claimStore})}),
    reviewSubject: {async createCandidate(value) { return value; }, async createPhysicalProfile({subject: value}) { return {subject: value}; }},
    legacyReview: {async review() { return {status: "passed"}; }}, receiptFinalizer: {async finalize(value) { return value; }}});
  assert.equal(recoveredCampaign.recover(identity).revision, accepted.revision);
  assert.equal(recoveredEpisode.recover(authority.identity).currentResult.findings[0].status, "verified_resolved");
  assert.equal(owners.claimStore.exportStore().revisions.length, 2);
});

test("native closure refuses stale campaign CAS, raw-result terminalization, and unselected obligations", async () => {
  const initialResult = await load("initial-finding");
  const implementationReview = createImplementationReviewService();
  const service = createSliceCampaignService({implementationReview,
    nativeReview: {async executeInitial() { throw new Error("must not enter native owner"); }},
    reviewSubject: {async createCandidate() { return candidate(initialResult); }, async createPhysicalProfile({subject: value}) { return {subject: value}; }},
    legacyReview: {async review() { return {status: "passed"}; }}, receiptFinalizer: {async finalize(value) { return value; }}});
  let state = service.admit({identity: {...identity, attemptId: "refusal"}, workspace: "/s12-refusal",
    acceptedBoundary: {reference: "plan:s12", sha256: "1".repeat(64)}, baseline: {acceptedCommit: "b", acceptedTree: "t", interSliceCommit: "i"}});
  const refusalIdentity = state.identity;
  state = service.advance({identity: refusalIdentity, expectedRevision: state.revision, phase: "implementing", consequence: {}});
  state = service.advance({identity: refusalIdentity, expectedRevision: state.revision, phase: "gate_ready", consequence: {}});
  state = await service.bindCandidate({identity: refusalIdentity, expectedRevision: state.revision, request: candidate(initialResult)});
  state = service.advance({identity: refusalIdentity, expectedRevision: state.revision, phase: "review_ready", consequence: {}});
  await assert.rejects(service.terminalize({identity: refusalIdentity, expectedRevision: state.revision, outcome: "accepted", receipt: {}}), /completed native closure or compatibility/);
  state = service.bindReviewSelection({identity: refusalIdentity, expectedRevision: state.revision, selection: selection(initialResult)});
  await assert.rejects(service.runLegacyReview({identity: refusalIdentity, expectedRevision: state.revision, selectionPlan: {}}), /without a prior review/);
  assert.throws(() => service.bindImplementationReview({identity: refusalIdentity, expectedRevision: state.revision, result: initialResult}), /without a prior review/);
  await assert.rejects(service.runNativeReview({identity: refusalIdentity, expectedRevision: "0".repeat(64), request: {obligationId: "generic"}}), /revision conflict/);
  await assert.rejects(service.runNativeReview({identity: refusalIdentity, expectedRevision: state.revision, request: {obligationId: "unselected"}}), /not selected/);
});

test("native remediation can cycle through a newly bound immutable candidate", async () => {
  const [initialResult, remediatedResult] = await Promise.all([
    load("initial-finding"), load("remediated-finding"),
  ]);
  const implementationReview = createImplementationReviewService();
  const service = createSliceCampaignService({implementationReview,
    nativeReview: {
      async executeInitial({obligationId}) {
        return {binding: {schemaVersion: 1, obligationId,
          status: obligationId === "generic" ? "awaiting_builder" : "reported",
          findings: obligationId === "generic" ? [{findingId: "S12-001"}] : []}, builderContext: {}};
      },
      recoverRemediation() { return {providerEntry: "not_entered"}; },
      async executeRemediation({binding}) {
        return {binding: {...binding, status: "reported"}, builderContext: null, failure: null};
      },
    },
    reviewSubject: {
      async createCandidate(request) { return request; },
      async createPhysicalProfile({subject: value}) { return {subject: value}; },
    },
    legacyReview: {async review() { return {status: "passed"}; }},
    receiptFinalizer: {async finalize(value) { return value; }},
  });
  const remediationIdentity = {...identity, attemptId: "candidate-cycle"};
  let state = service.admit({identity: remediationIdentity, workspace: "/s12-candidate-cycle",
    acceptedBoundary: {reference: "plan:s12", sha256: "1".repeat(64)},
    baseline: {acceptedCommit: "baseline", acceptedTree: "baseline-tree", interSliceCommit: "inter"}});
  state = service.advance({identity: remediationIdentity, expectedRevision: state.revision,
    phase: "implementing", consequence: {}});
  state = service.advance({identity: remediationIdentity, expectedRevision: state.revision,
    phase: "gate_ready", consequence: {}});
  state = await service.bindCandidate({identity: remediationIdentity, expectedRevision: state.revision,
    request: candidate(initialResult)});
  state = service.advance({identity: remediationIdentity, expectedRevision: state.revision,
    phase: "review_ready", consequence: {}});
  assert.throws(() => service.advance({identity: remediationIdentity, expectedRevision: state.revision,
    phase: "gate_ready", consequence: {}}), /phase transition is invalid/);
  state = service.bindReviewSelection({identity: remediationIdentity, expectedRevision: state.revision,
    selection: {...selection(initialResult), specialists: [
      {obligationId: "generic", skill: "implementation-review", selection: "selected"},
      {obligationId: "instructions", skill: "agent-instruction-review", selection: "selected"},
    ]}});
  state = (await service.runNativeReview({identity: remediationIdentity, expectedRevision: state.revision,
    request: {obligationId: "generic", authority: {}, beginTransitionId: "unused",
      resultTransitionId: "unused", reviewerRequest: {subject: initialResult.subject}}})).campaign;
  state = (await service.runNativeReview({identity: remediationIdentity, expectedRevision: state.revision,
    request: {obligationId: "instructions", authority: {}, beginTransitionId: "unused-instructions",
      resultTransitionId: "unused-instructions", reviewerRequest: {subject: initialResult.subject}}})).campaign;
  const preservedSelection = state.reviewSelection;
  state = service.advance({identity: remediationIdentity, expectedRevision: state.revision,
    phase: "gate_ready", consequence: {gate: "remediation-passed"}});
  state = await service.bindCandidate({identity: remediationIdentity, expectedRevision: state.revision,
    request: candidate(remediatedResult)});
  assert.deepEqual(state.candidate, candidate(remediatedResult));
  assert.deepEqual(state.physicalProfile.subject.checkpoint, candidate(remediatedResult));
  assert.deepEqual(state.reviewSelection, preservedSelection);
  assert.equal(state.nativeReview.obligations.generic.status, "awaiting_builder");
  assert.equal(state.nativeReview.obligations.instructions.status, "awaiting_builder");
  state = service.advance({identity: remediationIdentity, expectedRevision: state.revision,
    phase: "review_ready", consequence: {candidate: "remediation-bound"}});
  assert.equal(state.phase, "review_ready");
  await assert.rejects(service.terminalize({identity: remediationIdentity, expectedRevision: state.revision,
    outcome: "accepted", receipt: {}}), /completed native closure/);
  state = (await service.runNativeRemediation({identity: remediationIdentity, expectedRevision: state.revision,
    request: {obligationId: "generic"}})).campaign;
  await assert.rejects(service.terminalize({identity: remediationIdentity, expectedRevision: state.revision,
    outcome: "accepted", receipt: {}}), /completed native closure/);
  state = (await service.runNativeRemediation({identity: remediationIdentity, expectedRevision: state.revision,
    request: {obligationId: "instructions"}})).campaign;
  state = await service.terminalize({identity: remediationIdentity, expectedRevision: state.revision,
    outcome: "accepted", receipt: {status: "accepted"}});
  assert.equal(state.phase, "terminal");
});

test("legacy remediation admission correction refuses uncertain provider entry", async () => {
  const [initialResult, remediatedResult] = await Promise.all([
    load("initial-finding"), load("remediated-finding"),
  ]);
  let remediationEntries = 0;
  const nativeReview = {
    async executeInitial({obligationId}) {
      return {binding: {schemaVersion: 1, obligationId, status: "awaiting_builder",
        findings: [{findingId: "S12-001"}]}, builderContext: {}};
    },
    recoverRemediation() { return {providerEntry: "unknown"}; },
    async executeRemediation() {
      remediationEntries += 1;
      throw new Error("fixture interruption before remediation subject transition");
    },
  };
  const service = createSliceCampaignService({implementationReview: createImplementationReviewService(), nativeReview,
    reviewSubject: {async createCandidate(request) { return request; },
      async createPhysicalProfile({subject: value}) { return {subject: value}; }},
    legacyReview: {async review() { return {status: "passed"}; }},
    receiptFinalizer: {async finalize(value) { return value; }}});
  const recoveryIdentity = {...identity, attemptId: "legacy-authority-uncertain"};
  let state = service.admit({identity: recoveryIdentity, workspace: "/s12-legacy-authority-uncertain",
    acceptedBoundary: {reference: "plan:s12", sha256: "1".repeat(64)},
    baseline: {acceptedCommit: "baseline", acceptedTree: "baseline-tree", interSliceCommit: "inter"}});
  state = service.advance({identity: recoveryIdentity, expectedRevision: state.revision,
    phase: "implementing", consequence: {}});
  state = service.advance({identity: recoveryIdentity, expectedRevision: state.revision,
    phase: "gate_ready", consequence: {}});
  state = await service.bindCandidate({identity: recoveryIdentity, expectedRevision: state.revision,
    request: candidate(initialResult)});
  state = service.advance({identity: recoveryIdentity, expectedRevision: state.revision,
    phase: "review_ready", consequence: {}});
  state = service.bindReviewSelection({identity: recoveryIdentity, expectedRevision: state.revision,
    selection: selection(initialResult)});
  state = (await service.runNativeReview({identity: recoveryIdentity, expectedRevision: state.revision,
    request: {obligationId: "generic", authority: {initialSubject: initialResult.subject},
      beginTransitionId: "legacy-authority:begin", resultTransitionId: "legacy-authority:result",
      reviewerRequest: {subject: initialResult.subject}}})).campaign;
  state = service.advance({identity: recoveryIdentity, expectedRevision: state.revision,
    phase: "gate_ready", consequence: {}});
  state = await service.bindCandidate({identity: recoveryIdentity, expectedRevision: state.revision,
    request: candidate(remediatedResult)});
  state = service.advance({identity: recoveryIdentity, expectedRevision: state.revision,
    phase: "review_ready", consequence: {}});
  const remediationSubjectReference = episodeReference("checkpoint", remediatedResult.subject.commit,
    remediatedResult.subject.tree, reviewEpisodeDigest(remediatedResult.subject));
  const correctedRequest = {obligationId: "generic", authority: {initialSubject: episodeReference(
    "checkpoint", initialResult.subject.commit, initialResult.subject.tree, reviewEpisodeDigest(initialResult.subject))},
  subjectTransitionId: "legacy-authority:subject", resultTransitionId: "legacy-authority:remediated",
  remediationSubject: remediatedResult.subject, reviewerRequest: {subject: remediatedResult.subject}};
  const legacyAdmittedRequest = {...correctedRequest,
    authority: {...correctedRequest.authority, initialSubject: remediationSubjectReference}};
  await assert.rejects(service.runNativeRemediation({identity: recoveryIdentity,
    expectedRevision: state.revision, request: legacyAdmittedRequest, remediationSubjectReference}),
  /fixture interruption before remediation subject transition/);
  state = service.recover(recoveryIdentity);
  await assert.rejects(service.runNativeRemediation({identity: recoveryIdentity,
    expectedRevision: state.revision, request: correctedRequest, remediationSubjectReference,
    legacyAdmittedRequest}), /durable execution admission/);
  assert.equal(remediationEntries, 1);
});

test("native closure recovers post-result claim failure without replaying provider entry", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "s12-native-recovery."));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const initialResult = await load("initial-finding");
  const owners = await stores(t, directory, {bootstrap: true});
  const reviewEpisode = createReviewEpisodeService({store: owners.episodeStore, implementationReview: owners.implementationReview});
  const canonicalBridge = createReviewFindingBridge({store: owners.claimStore});
  let failAfterPublication = true;
  const findingBridge = {
    ...canonicalBridge,
    publishFindings(request) {
      const published = canonicalBridge.publishFindings(request);
      if (failAfterPublication) {
        failAfterPublication = false;
        throw new Error("fixture interruption after claim publication");
      }
      return published;
    },
  };
  let reviewerEntries = 0;
  const reviewer = {async review() {
    reviewerEntries += 1;
    return {attemptId: "provider-attempt-1", result: structuredClone(initialResult)};
  }};
  const nativeReview = createNativeReviewClosureService({reviewEpisode, reviewer, findingBridge});
  const service = createSliceCampaignService({store: owners.campaignStore, nativeReview,
    implementationReview: owners.implementationReview,
    reviewSubject: {async createCandidate(request) { return request; }, async createPhysicalProfile({subject: value}) { return {subject: value}; }},
    legacyReview: {async review() { return {status: "passed"}; }}, receiptFinalizer: {async finalize(value) { return value; }}});
  const recoveryIdentity = {...identity, attemptId: "partial-success"};
  let state = service.admit({identity: recoveryIdentity, workspace: directory,
    acceptedBoundary: {reference: "plan:s12", sha256: "1".repeat(64)},
    baseline: {acceptedCommit: "baseline", acceptedTree: "baseline-tree", interSliceCommit: "inter"}});
  state = service.advance({identity: recoveryIdentity, expectedRevision: state.revision, phase: "implementing", consequence: {}});
  state = service.advance({identity: recoveryIdentity, expectedRevision: state.revision, phase: "gate_ready", consequence: {}});
  state = await service.bindCandidate({identity: recoveryIdentity, expectedRevision: state.revision, request: candidate(initialResult)});
  state = service.advance({identity: recoveryIdentity, expectedRevision: state.revision, phase: "review_ready", consequence: {}});
  state = service.bindReviewSelection({identity: recoveryIdentity, expectedRevision: state.revision, selection: selection(initialResult)});
  const authority = episodeAuthority(initialResult, {campaignIdentity: recoveryIdentity});
  const request = {
    obligationId: "generic", authority, beginTransitionId: "recovery:begin", resultTransitionId: "recovery:result",
    reviewerRequest: {subject: initialResult.subject}, findingAuthority: owners.authority,
    operationPrefix: "s12:recovery", contextRequest: {requestId: "s12:recovery:context", consumer: {
      identity: "slice-builder:s12", revision: initialResult.subject.tree, decision_scope: "s12-native-review"},
      limitations: ["Claims are evidence records, not review acceptance."]},
  };
  await assert.rejects(service.runNativeReview({identity: recoveryIdentity, expectedRevision: state.revision, request}), /fixture interruption/);
  state = service.recover(recoveryIdentity);
  assert.equal(state.nativeReview.obligations.generic.status, "executing");
  assert.equal(reviewerEntries, 1);
  const recovered = await service.runNativeReview({identity: recoveryIdentity, expectedRevision: state.revision, request});
  assert.equal(recovered.campaign.nativeReview.obligations.generic.status, "awaiting_builder");
  assert.equal(reviewerEntries, 1);
  assert.equal(owners.claimStore.exportStore().operations.length, 1);
});

test("native terminalization waits for every selected specialist obligation", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "s12-native-multi."));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const acceptable = await load("remediated-finding");
  const owners = await stores(t, directory, {bootstrap: true});
  const calls = [];
  const reviewer = {
    async review() {
      calls.push("generic");
      return {attemptId: "multi-attempt", result: structuredClone(acceptable)};
    },
    async reviewAgentInstructions({closure}) {
      calls.push(`specialist:${closure.closureRevision}`);
      return {attemptId: "multi-specialist-attempt", specialistReview: {
        implementationReviewResult: structuredClone(acceptable),
      }};
    },
  };
  const reviewEpisode = createReviewEpisodeService({store: owners.episodeStore, implementationReview: owners.implementationReview});
  const nativeReview = createNativeReviewClosureService({reviewEpisode, reviewer,
    findingBridge: createReviewFindingBridge({store: owners.claimStore})});
  const service = createSliceCampaignService({store: owners.campaignStore, nativeReview,
    implementationReview: owners.implementationReview,
    reviewSubject: {async createCandidate(request) { return request; }, async createPhysicalProfile({subject: value}) { return {subject: value}; }},
    legacyReview: {async review() { return {status: "passed"}; }}, receiptFinalizer: {async finalize(value) { return value; }}});
  const multiIdentity = {...identity, attemptId: "multi-obligation"};
  let state = service.admit({identity: multiIdentity, workspace: directory,
    acceptedBoundary: {reference: "plan:s12", sha256: "1".repeat(64)},
    baseline: {acceptedCommit: "baseline", acceptedTree: "baseline-tree", interSliceCommit: "inter"}});
  state = service.advance({identity: multiIdentity, expectedRevision: state.revision, phase: "implementing", consequence: {}});
  state = service.advance({identity: multiIdentity, expectedRevision: state.revision, phase: "gate_ready", consequence: {}});
  state = await service.bindCandidate({identity: multiIdentity, expectedRevision: state.revision, request: candidate(acceptable)});
  state = service.advance({identity: multiIdentity, expectedRevision: state.revision, phase: "review_ready", consequence: {}});
  const multiSelection = selection(acceptable);
  multiSelection.specialists.push({obligationId: "instructions", skill: "agent-instruction-review", selection: "selected"});
  state = service.bindReviewSelection({identity: multiIdentity, expectedRevision: state.revision, selection: multiSelection});
  const requestFor = (obligationId) => ({obligationId,
    authority: episodeAuthority(acceptable, {campaignIdentity: multiIdentity, obligationId, episodeId: `episode-${obligationId}`}),
    beginTransitionId: `${obligationId}:begin`, resultTransitionId: `${obligationId}:result`,
    reviewerRequest: {subject: acceptable.subject, ...(obligationId === "instructions" ? {
      closure: {closureRevision: "closure-v1"},
    } : {})}, findingAuthority: owners.authority,
    operationPrefix: `s12:multi:${obligationId}`, contextRequest: {requestId: `s12:multi:${obligationId}:context`,
      consumer: {identity: "slice-builder:s12", revision: acceptable.subject.tree, decision_scope: "s12-native-review"},
      limitations: ["Claims are evidence records, not review acceptance."]}});
  state = (await service.runNativeReview({identity: multiIdentity, expectedRevision: state.revision,
    request: requestFor("generic")})).campaign;
  await assert.rejects(service.terminalize({identity: multiIdentity, expectedRevision: state.revision,
    outcome: "accepted", receipt: {}}), /completed native closure/);
  state = (await service.runNativeReview({identity: multiIdentity, expectedRevision: state.revision,
    request: requestFor("instructions")})).campaign;
  const terminal = await service.terminalize({identity: multiIdentity, expectedRevision: state.revision,
    outcome: "accepted", receipt: {status: "accepted"}});
  assert.equal(terminal.phase, "terminal");
  assert.deepEqual(Object.keys(terminal.nativeReview.obligations).sort(), ["generic", "instructions"]);
  assert.deepEqual(calls, ["generic", "specialist:closure-v1"]);
});

test("native closure refuses replay after an outcome-ambiguous provider exception", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "s12-native-provider-failure."));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const acceptable = await load("remediated-finding");
  const owners = await stores(t, directory, {bootstrap: true});
  let entries = 0;
  const reviewer = {async review() { entries += 1; throw new Error("provider outcome unavailable"); }};
  const reviewEpisode = createReviewEpisodeService({store: owners.episodeStore, implementationReview: owners.implementationReview});
  const service = createSliceCampaignService({store: owners.campaignStore,
    nativeReview: createNativeReviewClosureService({reviewEpisode, reviewer,
      findingBridge: createReviewFindingBridge({store: owners.claimStore})}),
    implementationReview: owners.implementationReview,
    reviewSubject: {async createCandidate(request) { return request; }, async createPhysicalProfile({subject: value}) { return {subject: value}; }},
    legacyReview: {async review() { return {status: "passed"}; }}, receiptFinalizer: {async finalize(value) { return value; }}});
  const failureIdentity = {...identity, attemptId: "provider-failure"};
  let state = service.admit({identity: failureIdentity, workspace: directory,
    acceptedBoundary: {reference: "plan:s12", sha256: "1".repeat(64)},
    baseline: {acceptedCommit: "baseline", acceptedTree: "baseline-tree", interSliceCommit: "inter"}});
  state = service.advance({identity: failureIdentity, expectedRevision: state.revision, phase: "implementing", consequence: {}});
  state = service.advance({identity: failureIdentity, expectedRevision: state.revision, phase: "gate_ready", consequence: {}});
  state = await service.bindCandidate({identity: failureIdentity, expectedRevision: state.revision, request: candidate(acceptable)});
  state = service.advance({identity: failureIdentity, expectedRevision: state.revision, phase: "review_ready", consequence: {}});
  state = service.bindReviewSelection({identity: failureIdentity, expectedRevision: state.revision, selection: selection(acceptable)});
  const request = {obligationId: "generic", authority: episodeAuthority(acceptable, {campaignIdentity: failureIdentity}),
    beginTransitionId: "failure:begin", resultTransitionId: "failure:result",
    reviewerRequest: {subject: acceptable.subject}, findingAuthority: owners.authority,
    operationPrefix: "s12:failure", contextRequest: {requestId: "s12:failure:context",
      consumer: {identity: "slice-builder:s12", revision: acceptable.subject.tree, decision_scope: "s12-native-review"},
      limitations: ["Claims are evidence records, not review acceptance."]}};
  await assert.rejects(service.runNativeReview({identity: failureIdentity, expectedRevision: state.revision, request}), /provider outcome unavailable/);
  state = service.recover(failureIdentity);
  await assert.rejects(service.runNativeReview({identity: failureIdentity, expectedRevision: state.revision, request}), /provider replay is refused/);
  assert.equal(entries, 1);
});
