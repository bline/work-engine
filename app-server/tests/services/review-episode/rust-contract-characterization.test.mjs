import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { createImplementationReviewService } from "../../../src/services/implementation-review/service.mjs";
import { createProductionPathEvidenceService } from "../../../src/services/claim-evidence/production-path-service.mjs";
import { makeProductionPathClaimRevision, normalizeProductionPathObservation,
  productionPathReference } from "../../../src/services/claim-evidence/production-path-contract.mjs";
import { openSqliteClaimEvidenceStore } from "../../../src/services/claim-evidence/sqlite-store.mjs";
import { digest as claimDigest } from "../../../src/services/claim-evidence/identity.mjs";
import { createSliceCampaignService } from "../../../src/services/slice-campaign/service.mjs";
import { openSqliteSliceCampaignStore } from "../../../src/services/slice-campaign/sqlite-store.mjs";
import { createNativeReviewClosureService } from "../../../src/services/slice-campaign/native-review-closure.mjs";
import { createReviewFindingBridge } from "../../../src/services/claim-evidence/review-finding-bridge.mjs";
import { canonicalJson, digest, identityKey } from "../../../src/services/review-episode/contract.mjs";
import { createReviewEpisodeService } from "../../../src/services/review-episode/service.mjs";
import { openSqliteReviewEpisodeStore } from "../../../src/services/review-episode/sqlite-store.mjs";

const fixture = async (name) => JSON.parse(await readFile(new URL(`../../fixtures/review-episode-rust/${name}.json`, import.meta.url)));
const sha = (value) => createHash("sha256").update(value).digest("hex");
const ref = (owner, reference, revision, value = reference) => ({owner, reference, revision,
  sha256: sha(value), freshness: "exact_revision"});
const result = async (name) => JSON.parse(await readFile(new URL(`../../fixtures/implementation-review/${name}.json`, import.meta.url)));
const authority = (subject, identity, generation = 1, predecessorRevision = null) => ({
  schemaVersion: 1, grantId: `grant-${generation}`, identity,
  source: ref("human", "accepted-plan", "plan-v1"),
  writer: {actorId: "reviewer", provider: "fixture", generation,
    runtimeSession: ref("runtime", `session-${generation}`, `generation-${generation}`)},
  readers: ["reviewer", "builder", "supervisor"],
  initialSubject: {...ref("checkpoint", "candidate", subject.commit), sha256: digest(subject)},
  predecessorRevision,
});
const admission = (name, status, boundary) => ({
  claimRevisionRef: ref("claim-evidence", `claim-${name}`, `claim-revision-${name}`),
  establishmentRef: ref("claim-evidence", `establishment-${name}`, `establishment-revision-${name}`),
  observationRef: ref("claim-evidence", "observation-1", "observation-revision-1"),
  consumptionRef: ref("slice-campaign", `consumption-${name}`, `consumption-revision-${name}`),
  status, boundary, consumer: boundary === "builder_projection" ? "slice-builder:run" : "slice-campaign:run",
});
const predecessors = () => [admission("builder-old", "unestablished", "builder_projection"),
  admission("terminal-old", "unestablished", "campaign_terminalization")];
const successors = () => [admission("builder-new", "established", "builder_projection"),
  admission("terminal-new", "established", "campaign_terminalization")];

async function sqliteService(t) {
  const directory = await mkdtemp(path.join(os.tmpdir(), "review-episode-r0."));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const filePath = path.join(directory, "episode.sqlite");
  let store = await openSqliteReviewEpisodeStore({filePath});
  t.after(() => store.close());
  return {filePath, service: createReviewEpisodeService({store,
    implementationReview: createImplementationReviewService()}),
  reopen: async () => {store.close(); store = await openSqliteReviewEpisodeStore({filePath});
    return createReviewEpisodeService({store, implementationReview: createImplementationReviewService()});}};
}

test("legacy codec golden bytes and SHA-256 include UTF-16 ordering and no newline", async () => {
  const vectors = await fixture("codec-v1");
  for (const item of vectors.vectors) {
    assert.equal(canonicalJson(item.value), item.canonical, item.id);
    assert.equal(digest(item.value), item.sha256, item.id);
    assert.equal(sha(item.canonical), item.sha256, item.id);
    assert.equal(item.canonical.endsWith("\n"), false, item.id);
  }
  assert.equal(Object.hasOwn(vectors.vectors.find(({id}) => id === "missing-key").value, "absent"), false);
  assert.notEqual(digest({present: null}), digest({present: null, absent: null}));
  assert.equal(canonicalJson({"\uE000": 1, "\u{10000}": 2}), '{"𐀀":2,"":1}');
});

test("v1/v2 durable state bytes, identity, revisions and predecessor history match frozen JS oracle", async (t) => {
  const oracle = await fixture("states-v1-v2");
  const {service} = await sqliteService(t);
  const accepted = await result("acceptable-as-is");
  const grant = authority(accepted.subject, oracle.identity);
  assert.equal(identityKey(oracle.identity), oracle.identityKey);
  let state = service.begin({authority: grant, transitionId: "begin", unresolvedQuestions: []});
  assertState("v1_begin", state);
  state = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "result", action: "record_result", payload: {result: accepted,
      unresolvedQuestions: ["Who owns the claim?"], evidenceAdmissions: predecessors()}});
  assertState("v2_blocked", state);
  state = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "succession", action: "succeed_evidence",
    payload: {predecessorAdmissions: predecessors(), successorAdmissions: successors()}});
  assertState("v2_succeeded", state);
  assert.deepEqual(service.history({identity: oracle.identity}).map(({revision}) => revision),
    oracle.historyRevisions);
  assert.equal(service.read({identity: oracle.identity, revision: oracle.states.v2_blocked.revision}).phase,
    "evidence_unestablished");

  function assertState(name, value) {
    const expected = oracle.states[name];
    assert.equal(value.schemaVersion, expected.schemaVersion, name);
    assert.equal(value.revision, expected.revision, name);
    assert.equal(canonicalJson(value), expected.canonical, name);
    assert.equal(digest(Object.fromEntries(Object.entries(value).filter(([key]) => key !== "revision"))),
      expected.revision, name);
  }
});

test("SQLite command baseline: CAS, replay, replacement attribution and restart", async (t) => {
  const commands = await fixture("commands");
  const {service, reopen} = await sqliteService(t);
  const initial = await result("remediation-required");
  const grant = authority(initial.subject, commands.identity);
  let state = service.begin({authority: grant, transitionId: "begin"});
  assert.equal(state.revision, commands.revisions.begin);
  const first = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "initial-result", action: "record_result",
    payload: {result: initial, unresolvedQuestions: []}});
  assert.equal(first.revision, commands.revisions.initialResult);
  assert.equal(first.phase, "remediation");
  assert.equal(service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "initial-result", action: "record_result",
    payload: {result: initial, unresolvedQuestions: []}}).revision, first.revision);
  assert.throws(() => service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "initial-result", action: "mark_uncertain",
    payload: {reason: "other", reconciliationAction: "read"}}), /conflicts/);
  assert.throws(() => service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "stale", action: "mark_uncertain",
    payload: {reason: "lost", reconciliationAction: "read"}}), /expected revision/);
  state = service.transition({authority: grant, expectedRevision: first.revision,
    transitionId: "uncertain", action: "mark_uncertain",
    payload: {reason: "session lost", reconciliationAction: "replace"}});
  assert.throws(() => service.transition({authority: authority(initial.subject, commands.identity, 3, state.revision),
    expectedRevision: state.revision, transitionId: "wrong-generation", action: "replace_writer",
    payload: {reason: "wrong generation", pendingAction: "stop"}}), /successor authority is invalid/);
  const successor = authority(initial.subject, commands.identity, 2, state.revision);
  state = service.transition({authority: successor, expectedRevision: state.revision,
    transitionId: "replace", action: "replace_writer",
    payload: {reason: "session unavailable", pendingAction: "reconcile exact state"}});
  assert.equal(state.revision, commands.revisions.replaced);
  assert.equal(state.continuity, "reconstructed_continuation");
  assert.equal(state.writer.generation, 2);
  assert.throws(() => service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "stale-writer", action: "mark_uncertain",
    payload: {reason: "x", reconciliationAction: "y"}}), /current writer generation/);
  const changed = structuredClone(initial); changed.findings[0].title = "rewritten attribution";
  assert.throws(() => service.transition({authority: successor, expectedRevision: state.revision,
    transitionId: "changed-finding", action: "record_result",
    payload: {result: changed, unresolvedQuestions: []}}), /result transition is invalid|cannot be rewritten/);
  const recovered = await reopen();
  assert.equal(recovered.recover(commands.identity).revision, state.revision);
  assert.deepEqual(recovered.history({identity: commands.identity}).map(({revision}) => revision),
    commands.historyRevisions);
  assert.equal(recovered.read({identity: commands.identity, revision: first.revision}).writer.generation, 1);
  assert.equal(recovered.recover(commands.identity).writer.generation, 2);
  const retired = recovered.transition({authority: successor, expectedRevision: state.revision,
    transitionId: "retire", action: "retire",
    payload: {outcome: "closed", reason: "review complete", protectedReferences: [grant.initialSubject]}});
  assert.equal(retired.status, "retired");
  assert.throws(() => recovered.transition({authority: successor, expectedRevision: retired.revision,
    transitionId: "after-retire", action: "mark_uncertain",
    payload: {reason: "late", reconciliationAction: "stop"}}), /retired/);
});

test("legacy replay exposes current state before stale writer/revision checks; ordinary new write stays fenced", async (t) => {
  const deltas = await fixture("semantic-deltas");
  const {service} = await sqliteService(t);
  const accepted = await result("acceptable-as-is");
  const grant = authority(accepted.subject, deltas.identity);
  let state = service.begin({authority: grant, transitionId: "begin"});
  state = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "result", action: "record_result", payload: {result: accepted, unresolvedQuestions: []}});
  const staleRevision = state.revision;
  state = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "uncertain", action: "mark_uncertain",
    payload: {reason: "session lost", reconciliationAction: "replace"}});
  const successor = authority(accepted.subject, deltas.identity, 2, state.revision);
  state = service.transition({authority: successor, expectedRevision: state.revision,
    transitionId: "replace", action: "replace_writer",
    payload: {reason: "lost", pendingAction: "reconcile"}});
  const staleReplay = service.transition({authority: grant, expectedRevision: staleRevision,
    transitionId: "result", action: "record_result", payload: {result: accepted, unresolvedQuestions: []}});
  assert.equal(staleReplay.revision, state.revision);
  assert.equal(service.begin({authority: grant, transitionId: "begin"}).revision, state.revision);
  assert.throws(() => service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "new", action: "mark_uncertain", payload: {reason: "x", reconciliationAction: "y"}}),
  /current writer generation/);
  assert.equal(deltas.decisions.replay.legacy, "matching replay returns latest state before revision/writer checks");
});

test("legacy succession reports acceptable result despite retained unresolved question", async (t) => {
  const deltas = await fixture("semantic-deltas");
  const {service} = await sqliteService(t);
  const accepted = await result("acceptable-as-is");
  const grant = authority(accepted.subject, {...deltas.identity, reviewEpisodeId: "questions"});
  let state = service.begin({authority: grant, transitionId: "begin"});
  state = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "result", action: "record_result", payload: {result: accepted,
      unresolvedQuestions: ["Who resolves this?"], evidenceAdmissions: predecessors()}});
  assert.equal(state.phase, "evidence_unestablished");
  state = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "succession", action: "succeed_evidence",
    payload: {predecessorAdmissions: predecessors(), successorAdmissions: successors()}});
  assert.equal(state.phase, "reported");
  assert.deepEqual(state.unresolvedQuestions, ["Who resolves this?"]);
  assert.equal(deltas.decisions.questions.future.phase, "remediation");
});

test("legacy later result retains earlier admissions even when subject changes", async (t) => {
  const deltas = await fixture("semantic-deltas");
  const {service} = await sqliteService(t);
  const first = await result("remediation-required");
  const grant = authority(first.subject, {...deltas.identity, reviewEpisodeId: "scope"});
  let state = service.begin({authority: grant, transitionId: "begin"});
  state = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "result-1", action: "record_result", payload: {result: first,
      unresolvedQuestions: [], evidenceAdmissions: successors()}});
  const nextSubject = {commit: "candidate-2", tree: "tree-2", patchIdentity: "patch-2"};
  const nextRef = {...ref("checkpoint", "candidate-2", "candidate-2"), sha256: digest(nextSubject)};
  state = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "subject-2", action: "record_remediation_subject", payload: {subject: nextRef}});
  const second = structuredClone(first); second.subject = nextSubject;
  state = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "result-2", action: "record_result", payload: {result: second, unresolvedQuestions: []}});
  assert.equal(state.schemaVersion, 2);
  assert.deepEqual(state.evidenceAdmissions, successors());
  assert.notEqual(digest(first.subject), digest(second.subject));
  assert.equal(deltas.decisions.scope.legacy, "later result retains earlier admissions without revalidation");
});

test("finding attribution remains immutable across a valid remediation result", async (t) => {
  const {service} = await sqliteService(t);
  const first = await result("remediation-required");
  const identity = {runId: "r0", sliceNumber: 1, attemptId: "findings", planVersion: "v1",
    reviewObligationId: "generic", reviewEpisodeId: "finding-lineage"};
  const grant = authority(first.subject, identity);
  let state = service.begin({authority: grant, transitionId: "begin"});
  state = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "result-1", action: "record_result", payload: {result: first, unresolvedQuestions: []}});
  const newSubject = {commit: "candidate-2", tree: "tree-2", patchIdentity: "patch-2"};
  state = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "subject-2", action: "record_remediation_subject",
    payload: {subject: {...ref("checkpoint", "candidate-2", "candidate-2"), sha256: digest(newSubject)}}});
  const changed = structuredClone(first); changed.subject = newSubject;
  changed.findings[0].title = "rewritten attribution";
  assert.throws(() => service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "changed-finding", action: "record_result",
    payload: {result: changed, unresolvedQuestions: []}}), /finding attribution or evidence cannot be rewritten/);
  const preserved = structuredClone(first); preserved.subject = newSubject;
  preserved.verdict = "acceptable_as_is";
  preserved.decisiveEvidence = first.findings[0].evidence;
  preserved.findings[0].status = "verified_resolved";
  preserved.findings[0].remediationEvidence = first.findings[0].evidence;
  state = service.transition({authority: grant, expectedRevision: state.revision,
    transitionId: "resolved", action: "record_result",
    payload: {result: preserved, unresolvedQuestions: []}});
  assert.equal(state.currentResult.findings[0].title, first.findings[0].title);
  assert.equal(state.currentResult.findings[0].status, "verified_resolved");
});

test("real episode and claims SQLite owners expose the episode-before-claims correction cut", async (t) => {
  const {service: episode} = await sqliteService(t);
  const directory = await mkdtemp(path.join(os.tmpdir(), "review-episode-r0-claims."));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const claimStore = await openSqliteClaimEvidenceStore({filePath: path.join(directory, "claims.sqlite"),
    bootstrapAuthorities: []});
  t.after(() => claimStore.close());
  const claimsOwner = createProductionPathEvidenceService({store: claimStore});
  const accepted = await result("acceptable-as-is");
  const identity = {runId: "r0", sliceNumber: 1, attemptId: "join", planVersion: "v1",
    reviewObligationId: "generic", reviewEpisodeId: "episode-join"};
  const grant = authority(accepted.subject, identity);
  const campaign = "r0:1:join:v1";
  const claimFor = (boundary) => makeProductionPathClaimRevision({
    proposition: "Exact native result used the accepted production path.",
    subject: {candidate: accepted.subject, reviewEpisodeId: identity.reviewEpisodeId},
    coveredState: "admitted_native_review_result", consumptionBoundary: boundary,
    consumer: `${boundary === "builder_projection" ? "slice-builder" : "slice-campaign"}:${campaign}`,
    acceptance: {owner: "operator", source: "selection-v1", unestablishedRoute: "operator"},
    profile: {id: "production-path-v1", revision: "production-path-profile-v1",
      allowedMechanisms: ["native-review-host-receipt-v1"],
      admissibleObservers: ["app-server.reviewer-host"], integrityRequired: true,
      requiredRealization: "claude-sonnet", requiredCapabilities: ["repository_read"],
      continuity: "fresh_initial"},
  });
  const claims = {builder: claimFor("builder_projection"), terminal: claimFor("campaign_terminalization")};
  const observation = normalizeProductionPathObservation({event_identity: "r0-correction-observation",
    selection: {id: "selection-v2", revision: "a".repeat(64)}, obligationId: "generic",
    subject: {candidate: accepted.subject, reviewEpisodeId: identity.reviewEpisodeId},
    coveredState: "admitted_native_review_result",
    execution: {attemptId: "provider-attempt-1", resultDigest: claimDigest(accepted)},
    realization: {requested: "claude-sonnet-5", observed: "claude-sonnet-5"},
    capabilityEnvelope: {capabilities: ["repository_read"], mutationAuthorized: false},
    continuity: {mode: "same_session_resume", sessionId: "session-1"},
    transport: {mechanism: "native-review-host-receipt-v1", digest: "f".repeat(64)},
    observer: {identity: "app-server.reviewer-host", kind: "app_server_host"},
    observedAt: "2026-10-07T00:00:00.000Z", adapterVersion: "adapter-v1",
    artifacts: [{owner: "reviewer-runtime", reference: "transport", digest: "f".repeat(64), status: "verified"}],
  });
  const oldAdmissions = Object.values(claims).map((claim) => {
    const establishment = claimsOwner.admit({operationId: `r0:prior:${claim.consumptionBoundary}`,
      claim, observation});
    assert.equal(establishment.status, "unestablished");
    return {claimRevisionRef: productionPathReference("claim-evidence", claim.claimId, claim.revision, claim),
      establishmentRef: productionPathReference("claim-evidence", establishment.id, establishment.id, establishment),
      observationRef: productionPathReference("claim-evidence", observation.id, observation.id, observation),
      consumptionRef: productionPathReference("slice-campaign", `consumption:${claim.claimId}`,
        claimDigest({claimRevision: claim.revision, establishment: establishment.id}),
        {claimRevision: claim.revision, establishment: establishment.id}),
      status: establishment.status, boundary: claim.consumptionBoundary, consumer: claim.consumer};
  });
  let current = episode.begin({authority: grant, transitionId: "begin"});
  current = episode.transition({authority: grant, expectedRevision: current.revision,
    transitionId: "result", action: "record_result",
    payload: {result: accepted, unresolvedQuestions: [], evidenceAdmissions: oldAdmissions}});
  assert.equal(current.phase, "evidence_unestablished");
  const blockedRevision = current.revision;
  const operationId = "r0:correction:1";
  const correction = {operationId,
    authority: {schemaVersion: 1, owner: "slice-supervisor", source: "operator-acceptance",
      sequence: 1, campaign, acceptanceOwner: "operator"}, campaignRevision: "campaign-revision-1",
    selection: {id: "selection-v2", predecessorRevision: "a".repeat(64),
      successorRevision: "b".repeat(64)}, claims, observationId: observation.id,
    reviewEpisode: {id: identity.reviewEpisodeId, predecessorRevision: blockedRevision},
    candidate: accepted.subject};
  const succeedEpisode = ({successors: nextClaims, establishments}) => {
    const nextAdmissions = Object.entries(nextClaims).map(([name, claim]) => {
      const establishment = establishments[name];
      return {claimRevisionRef: productionPathReference("claim-evidence", claim.claimId, claim.revision, claim),
        establishmentRef: productionPathReference("claim-evidence", establishment.id, establishment.id, establishment),
        observationRef: productionPathReference("claim-evidence", observation.id, observation.id, observation),
        consumptionRef: productionPathReference("slice-campaign", `consumption:${claim.claimId}`,
          claimDigest({claimRevision: claim.revision, establishment: establishment.id}),
          {claimRevision: claim.revision, establishment: establishment.id}),
        status: establishment.status, boundary: claim.consumptionBoundary, consumer: claim.consumer};
    });
    return episode.transition({authority: grant, expectedRevision: blockedRevision,
      transitionId: `${operationId}:episode-succession`, action: "succeed_evidence",
      payload: {predecessorAdmissions: oldAdmissions, successorAdmissions: nextAdmissions}}).revision;
  };
  assert.throws(() => claimsOwner.correct({...correction, succeedEpisode: (args) => {
    succeedEpisode(args); throw new Error("controlled interruption after episode commit");
  }}), /controlled interruption/);
  const episodeAfterCut = episode.recover(identity);
  assert.equal(episodeAfterCut.evidenceAdmissions.length, 4);
  assert.equal(episodeAfterCut.phase, "reported");
  assert.equal(claimStore.readProductionPathSuccession(operationId), null);
  assert.equal(claimStore.listProductionPathEstablishments().length, 2);
  assert.equal(episode.read({identity, revision: blockedRevision}).phase, "evidence_unestablished");
  const reconciled = claimsOwner.correct({...correction, succeedEpisode});
  assert.equal(reconciled.succession.reviewEpisode.successorRevision, episodeAfterCut.revision);
  assert.equal(claimStore.readProductionPathSuccession(operationId).id, reconciled.succession.id);
  assert.equal(claimStore.listProductionPathEstablishments().length, 4);
  assert.equal(episode.recover(identity).revision, episodeAfterCut.revision);
});

test("production campaign succession exposes claims-before-campaign cut with exact three-store lineage", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "review-episode-r0-campaign-cut."));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const episodeStore = await openSqliteReviewEpisodeStore({filePath: path.join(directory, "episodes.sqlite")});
  const claimStore = await openSqliteClaimEvidenceStore({filePath: path.join(directory, "claims.sqlite"),
    bootstrapAuthorities: []});
  const campaignStore = await openSqliteSliceCampaignStore({filePath: path.join(directory, "campaign.sqlite")});
  t.after(() => {episodeStore.close(); claimStore.close(); campaignStore.close();});
  const accepted = await result("acceptable-as-is");
  const campaignIdentity = {runId: "r0", sliceNumber: 2, attemptId: "campaign-cut", planVersion: "v1"};
  const campaignKey = `${campaignIdentity.runId}:${campaignIdentity.sliceNumber}:${campaignIdentity.attemptId}:${campaignIdentity.planVersion}`;
  const episodeAuthority = authority(accepted.subject, {...campaignIdentity,
    reviewObligationId: "generic", reviewEpisodeId: "episode-campaign-cut"});
  const claimFor = (boundary) => makeProductionPathClaimRevision({
    proposition: "The native result used the accepted production path.",
    subject: {candidate: accepted.subject, reviewEpisodeId: episodeAuthority.identity.reviewEpisodeId},
    coveredState: "admitted_native_review_result", consumptionBoundary: boundary,
    consumer: `${boundary === "builder_projection" ? "slice-builder" : "slice-campaign"}:${campaignKey}`,
    acceptance: {owner: "operator", source: "selection-v2", unestablishedRoute: "operator"},
    profile: {id: "production-path-v1", revision: "production-path-profile-v1",
      allowedMechanisms: ["native-review-host-receipt-v1"],
      admissibleObservers: ["app-server.reviewer-host"], integrityRequired: true,
      requiredRealization: "claude-sonnet", requiredCapabilities: ["repository_read"],
      continuity: "fresh_initial"},
  });
  const selection = {schemaVersion: 2, owner: "slice-supervisor", selectionId: "r0-selection",
    subject: accepted.subject, specialists: [{obligationId: "generic", skill: "implementation-review",
      selection: "selected", requiredClaims: [claimFor("builder_projection"),
        claimFor("campaign_terminalization")]}]};
  let providerEntries = 0;
  const reviewer = {async review() {providerEntries += 1; return {attemptId: "synthetic-provider-1",
    result: structuredClone(accepted), runtimeSessionId: "session-1",
    receipt: {requestedModel: "claude-sonnet-5", observedModel: "claude-sonnet-5",
      capabilities: ["repository_read"], mutationAuthorized: false,
      continuity: "same_session_resume", sessionId: "session-1",
      transportReceiptDigest: "f".repeat(64), evidenceMechanism: "native-review-host-receipt-v1",
      observerIdentity: "app-server.reviewer-host", observedAt: "2026-10-07T00:00:00.000Z",
      profileConfigurationDigest: "adapter-v1",
      artifacts: [{owner: "reviewer-runtime", reference: "transport", digest: "f".repeat(64),
        status: "verified"}]}};}};
  const episode = createReviewEpisodeService({store: episodeStore,
    implementationReview: createImplementationReviewService()});
  const nativeReview = createNativeReviewClosureService({reviewEpisode: episode, reviewer,
    findingBridge: createReviewFindingBridge({store: claimStore}),
    productionPathEvidence: createProductionPathEvidenceService({store: claimStore})});
  let cutBeforeCampaignCommit = false;
  const campaignWriteBoundary = {
    get: (...args) => campaignStore.get(...args),
    admit: (...args) => campaignStore.admit(...args),
    put: (...args) => {
      if (cutBeforeCampaignCommit && args[1].reviewSelectionSuccession) {
        throw new Error("controlled interruption after claims commit before campaign commit");
      }
      return campaignStore.put(...args);
    },
  };
  const campaignOwner = createSliceCampaignService({store: campaignWriteBoundary, nativeReview,
    reviewSubject: {async createCandidate(request) {return request;},
      async createPhysicalProfile({subject}) {return {subject};}},
    receiptFinalizer: {async finalize(value) {return value;}}});
  let campaign = campaignOwner.admit({identity: campaignIdentity, workspace: directory,
    acceptedBoundary: {reference: "r0-selection", sha256: "a".repeat(64)},
    baseline: {acceptedCommit: "baseline", acceptedTree: "baseline-tree", interSliceCommit: "inter"}});
  for (const phase of ["implementing", "gate_ready"]) campaign = campaignOwner.advance({
    identity: campaignIdentity, expectedRevision: campaign.revision, phase, consequence: {}});
  campaign = await campaignOwner.bindCandidate({identity: campaignIdentity,
    expectedRevision: campaign.revision, request: {commit: accepted.subject.commit,
      tree: accepted.subject.tree, manifestSha256: accepted.subject.patchIdentity}});
  campaign = campaignOwner.advance({identity: campaignIdentity, expectedRevision: campaign.revision,
    phase: "review_ready", consequence: {}});
  campaign = campaignOwner.bindReviewSelection({identity: campaignIdentity,
    expectedRevision: campaign.revision, selection});
  const reviewed = await campaignOwner.runNativeReview({identity: campaignIdentity,
    expectedRevision: campaign.revision, request: {obligationId: "generic", authority: episodeAuthority,
      beginTransitionId: "r0:begin", resultTransitionId: "r0:result",
      reviewerRequest: {subject: accepted.subject}, findingAuthority: {},
      operationPrefix: "r0:campaign-cut", requiredClaims: selection.specialists[0].requiredClaims,
      selection: {id: selection.selectionId, revision: digest(selection)},
      contextRequest: {requestId: "r0:campaign-cut:context",
        consumer: {identity: `slice-builder:${campaignKey}`, revision: accepted.subject.tree,
          decision_scope: "r0-native-review"}, limitations: ["Claims do not grant review acceptance."]}}});
  campaign = reviewed.campaign;
  assert.equal(campaign.nativeReview.obligations.generic.status, "evidence_unestablished");
  const episodePredecessor = campaign.nativeReview.obligations.generic.episodeRef.revision;
  const observation = claimStore.listObservations()[0];
  const successorSelection = structuredClone(selection);
  successorSelection.specialists[0].requiredClaims = selection.specialists[0].requiredClaims.map((prior) =>
    makeProductionPathClaimRevision({...prior, profile: {...prior.profile,
      requiredRealization: "claude-sonnet-5", continuity: "retained"}}));
  const operationId = "r0:campaign-cut:correction";
  cutBeforeCampaignCommit = true;
  assert.throws(() => campaignOwner.succeedReviewSelection({identity: campaignIdentity,
    expectedRevision: campaign.revision, operationId, obligationId: "generic",
    authority: {schemaVersion: 1, owner: "slice-supervisor", source: "operator-acceptance",
      sequence: 2, campaign: campaignKey, acceptanceOwner: "operator"},
    successorSelection, observationId: observation.id, episodeAuthority}),
  /controlled interruption after claims commit before campaign commit/);
  const episodeAfterCut = episode.recover(episodeAuthority.identity);
  const claimsAfterCut = claimStore.readProductionPathSuccession(operationId);
  const campaignAfterCut = campaignOwner.recover(campaignIdentity);
  assert.equal(providerEntries, 1);
  assert.equal(episodeAfterCut.phase, "reported");
  assert.equal(episodeAfterCut.evidenceAdmissions.length, 4);
  assert.ok(claimsAfterCut);
  assert.equal(claimStore.listProductionPathEstablishments().length, 4);
  assert.equal(claimsAfterCut.reviewEpisode.predecessorRevision, episodePredecessor);
  assert.equal(claimsAfterCut.reviewEpisode.successorRevision, episodeAfterCut.revision);
  assert.equal(claimsAfterCut.campaignRevision, campaign.revision);
  assert.equal(claimsAfterCut.selection.predecessorRevision, digest(selection));
  assert.equal(claimsAfterCut.selection.successorRevision, digest(successorSelection));
  assert.equal(claimStore.readObservation(observation.id).execution.resultDigest, claimDigest(accepted));
  for (const [name, linked] of Object.entries(claimsAfterCut.claims)) {
    const establishment = claimStore.readProductionPathEstablishment(linked.successorEstablishment);
    assert.equal(establishment.status, "established", name);
    assert.equal(establishment.claimRevision, linked.successorRevision, name);
    assert.equal(establishment.predecessor, linked.predecessorEstablishment, name);
    assert.ok(episodeAfterCut.evidenceAdmissions.some((entry) =>
      entry.establishmentRef.reference === establishment.id
      && entry.claimRevisionRef.revision === linked.successorRevision), name);
  }
  assert.equal(campaignAfterCut.revision, campaign.revision);
  assert.equal(campaignAfterCut.nativeReview.obligations.generic.episodeRef.revision, episodePredecessor);
  assert.equal(campaignAfterCut.reviewSelectionSuccession, undefined);
});
