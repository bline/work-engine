import {
  SLICE_CAMPAIGN_SCHEMA_VERSION, SLICE_PHASES, digest, freeze, identityKey,
  normalizeIdentity, requireRecord, requireSha256, requireText, validateReviewSelection,
  validateReviewSelectionSuccession,
} from "./contract.mjs";
import { validateExternalBootstrapPacket } from "./external-bootstrap-adoption-contract.mjs";

const NEXT_PHASE = new Map([["accepted", "implementing"], ["implementing", "gate_ready"], ["gate_ready", "review_ready"], ["review_ready", "terminal"]]);

export class InMemorySliceCampaignStore {
  constructor() { this.states = new Map(); this.workspaceAdmissions = new Map(); this.externalBootstrapEvidence = new Map(); }
  get(key) { return this.states.get(key) ?? null; }
  admit(key, workspace, state) {
    if (this.states.has(key)) throw new Error("slice campaign attempt already exists");
    const holder = this.workspaceAdmissions.get(workspace);
    if (holder && holder !== key) throw new Error("workspace already has an admitted mutable slice");
    this.workspaceAdmissions.set(workspace, key); this.states.set(key, state);
  }
  put(key, state, expectedRevision, { releaseWorkspace = null } = {}) {
    if (this.states.get(key)?.revision !== expectedRevision) throw new Error("slice campaign revision conflict");
    this.states.set(key, state);
    if (releaseWorkspace !== null && this.workspaceAdmissions.get(releaseWorkspace) === key) this.workspaceAdmissions.delete(releaseWorkspace);
  }
  supersede({key, expectedRevision, workspace, state, successorKey, successorState}) {
    if (this.states.get(key)?.revision !== expectedRevision) throw new Error("slice campaign revision conflict");
    if (this.states.has(successorKey)) throw new Error("slice campaign successor attempt already exists");
    if (this.workspaceAdmissions.get(workspace) !== key) {
      throw new Error("slice campaign supersession requires the source workspace admission");
    }
    this.states.set(key, state);
    this.states.set(successorKey, successorState);
    this.workspaceAdmissions.set(workspace, successorKey);
  }
  adoptExternalBootstrapEvidence(key, expectedCampaignRevision, adoption) {
    if (this.states.get(key)?.revision !== expectedCampaignRevision) throw new Error("slice campaign revision conflict");
    const existing = this.externalBootstrapEvidence.get(key);
    if (existing) {
      if (existing.packetDigest !== adoption.packetDigest) throw new Error("external bootstrap packet conflicts with adopted evidence");
      return existing;
    }
    this.externalBootstrapEvidence.set(key, adoption);
    return adoption;
  }
  getExternalBootstrapEvidence(key) { return this.externalBootstrapEvidence.get(key) ?? null; }
}

export function createSliceCampaignService({
  store = new InMemorySliceCampaignStore(), reviewSubject, legacyReview,
  implementationReview = null, nativeReview = null, receiptFinalizer, completionOffer = null,
} = {}) {
  for (const [owner, methods] of [[reviewSubject, ["createCandidate", "createPhysicalProfile"]], [receiptFinalizer, ["finalize"]]]) {
    if (!owner || methods.some((method) => typeof owner[method] !== "function")) throw new TypeError(`slice campaign requires composed owner ${methods.join("/")}`);
  }
  if (legacyReview !== undefined && legacyReview !== null
      && typeof legacyReview.review !== "function") {
    throw new TypeError("slice campaign legacy review owner requires review");
  }
  if (completionOffer !== null && typeof completionOffer.open !== "function") {
    throw new TypeError("slice campaign completion-offer owner requires open");
  }

  const publish = (state, expectedRevision, options = {}) => {
    const revision = digest({ ...state, revision: undefined });
    const published = freeze({ ...state, revision });
    store.put(identityKey(state.identity), published, expectedRevision, options);
    return published;
  };
  const current = (identity) => {
    const normalized = normalizeIdentity(identity);
    const state = store.get(identityKey(normalized));
    if (!state) throw new Error("slice campaign attempt does not exist");
    return state;
  };
  const requireRevision = (state, expectedRevision) => {
    requireSha256(expectedRevision, "expected campaign revision");
    if (state.revision !== expectedRevision) throw new Error("slice campaign revision conflict");
  };
  const nativeObligations = (state) => state.nativeReview?.obligations ?? {};
  const reportedZeroFindingClosure = (obligation) => obligation?.status === "reported"
    && Array.isArray(obligation.findings) && obligation.findings.length === 0;
  const completedNativeRemediationBinding = (value) => {
    const {remediationExecuting: _remediationExecuting, ...binding} = value;
    return freeze(binding);
  };
  const nativeRemediationReady = (state) => {
    const selected = state.reviewSelection?.specialists
      ?.filter(({selection}) => selection === "selected")
      .map(({obligationId}) => nativeObligations(state)[obligationId]) ?? [];
    return selected.length > 0
      && selected.some((obligation) => obligation?.status === "awaiting_builder")
      && selected.every((obligation) => ["awaiting_builder", "reported"].includes(obligation?.status));
  };
  const nativeEnvelope = (obligations) => freeze({schemaVersion: 1, obligations: freeze({...obligations})});
  const initialState = ({identity, workspace, acceptedBoundary, expectedImpact, baseline}) => ({
    schemaVersion: SLICE_CAMPAIGN_SCHEMA_VERSION, identity, workspace,
    acceptedBoundary: freeze(structuredClone(acceptedBoundary)),
    expectedImpact: expectedImpact && freeze(structuredClone(expectedImpact)),
    baseline: freeze(structuredClone(baseline)), phase: "accepted", latestConsequence: null,
    candidateRequestDigest: null, candidate: null, physicalProfile: null,
    review: null, implementationReview: null, reviewSelection: null, nativeReview: null,
    terminal: null,
  });
  const replacementNativeEnvelope = (state) => {
    const selected = new Set(state.reviewSelection.specialists
      .filter(({selection}) => selection === "selected").map(({obligationId}) => obligationId));
    return nativeEnvelope(Object.fromEntries(Object.entries(nativeObligations(state)).map(
      ([obligationId, obligation]) => [obligationId,
        selected.has(obligationId) && obligation.status === "reported"
          ? freeze({...obligation, status: "awaiting_builder"})
          : obligation],
    )));
  };
  const failedNativeObligation = ({obligationId, requestDigest, outcome, prior = null}) => {
    const failure = freeze(structuredClone(outcome.failure));
    const status = failure.failureSignature === "result_contract_rejected"
      ? "correction_required"
      : failure.providerEntry === "not_entered" ? "retryable_failure" : "executing";
    return freeze({...structuredClone(prior ?? {}), schemaVersion: 1, obligationId,
      status,
      requestDigest, failure,
      attempt: freeze({attemptId: outcome.execution?.attemptId ?? null,
        runtimeSessionId: outcome.execution?.runtimeSessionId ?? null,
        transportReceiptDigest: outcome.execution?.transportReceipt
          ? digest(outcome.execution.transportReceipt) : null}),
      priorAttempts: freeze([...(prior?.priorAttempts ?? []),
        ...(prior?.attempt ? [prior.attempt] : [])])});
  };

  return Object.freeze({
    admit({ identity, workspace, acceptedBoundary, expectedImpact = null, baseline }) {
      const normalized = normalizeIdentity(identity);
      requireText(workspace, "workspace");
      requireRecord(acceptedBoundary, "accepted boundary");
      requireText(acceptedBoundary.reference, "accepted boundary reference");
      requireSha256(acceptedBoundary.sha256, "accepted boundary sha256");
      requireRecord(baseline, "campaign baseline");
      for (const field of ["acceptedCommit", "acceptedTree", "interSliceCommit"]) requireText(baseline[field], `campaign baseline ${field}`);
      if (expectedImpact !== null) { requireRecord(expectedImpact, "expected impact"); requireSha256(expectedImpact.sha256, "expected impact sha256"); }
      const key = identityKey(normalized);
      const initial = initialState({identity: normalized, workspace, acceptedBoundary, expectedImpact, baseline});
      const published = freeze({ ...initial, revision: digest(initial) });
      store.admit(key, workspace, published);
      return published;
    },
    recover(identity) { return current(identity); },
    adoptExternalBootstrapEvidence({identity, expectedRevision, packet}) {
      const state = current(identity);
      requireRevision(state, expectedRevision);
      const verified = validateExternalBootstrapPacket(packet);
      if (identityKey(verified.identity) !== identityKey(state.identity)) {
        throw new Error("external bootstrap packet campaign identity conflicts with target");
      }
      if (verified.expected_campaign_revision !== state.revision) {
        throw new Error("external bootstrap packet expected campaign revision is stale");
      }
      if (verified.objective_boundary_baseline.accepted_boundary.reference !== state.acceptedBoundary.reference
          || verified.objective_boundary_baseline.accepted_boundary.sha256 !== state.acceptedBoundary.sha256
          || verified.objective_boundary_baseline.baseline.commit !== state.baseline.acceptedCommit
          || verified.objective_boundary_baseline.baseline.tree !== state.baseline.acceptedTree) {
        throw new Error("external bootstrap packet boundary or baseline conflicts with campaign");
      }
      const adoption = freeze({schemaVersion: 1, provenance: verified.provenance,
        packetDigest: verified.whole_packet_sha256, packet: verified,
        authority: freeze({phaseAdvance: false, candidateBinding: false, reviewAcceptance: false,
          terminalization: false, publication: false})});
      const stored = store.adoptExternalBootstrapEvidence(identityKey(state.identity), state.revision, adoption);
      return freeze({campaign: state, adoption: stored});
    },
    recoverExternalBootstrapEvidence(identity) {
      const normalized = normalizeIdentity(identity);
      current(normalized);
      return store.getExternalBootstrapEvidence(identityKey(normalized));
    },
    supersede({identity, expectedRevision, operationId, successor}) {
      const state = current(identity);
      requireText(operationId, "slice campaign supersession operation identity");
      requireRecord(successor, "slice campaign successor");
      const successorIdentity = normalizeIdentity(successor.identity);
      requireRecord(successor.acceptedBoundary, "successor accepted boundary");
      requireText(successor.acceptedBoundary.reference, "successor accepted boundary reference");
      requireSha256(successor.acceptedBoundary.sha256, "successor accepted boundary sha256");
      requireRecord(successor.baseline, "successor campaign baseline");
      for (const field of ["acceptedCommit", "acceptedTree", "interSliceCommit"]) {
        requireText(successor.baseline[field], `successor campaign baseline ${field}`);
      }
      if (successor.expectedImpact !== undefined && successor.expectedImpact !== null) {
        requireRecord(successor.expectedImpact, "successor expected impact");
        requireSha256(successor.expectedImpact.sha256, "successor expected impact sha256");
      }
      const sourceKey = identityKey(state.identity);
      const successorKey = identityKey(successorIdentity);
      if (sourceKey === successorKey) throw new Error("slice campaign successor identity must be distinct");
      const requestDigest = digest({operationId, identity: state.identity, expectedRevision,
        successor: {...successor, identity: successorIdentity}});
      if (state.phase === "superseded") {
        if (state.supersession?.requestDigest !== requestDigest) {
          throw new Error("slice campaign supersession request conflicts with durable superseded state");
        }
        return freeze({superseded: state, successor: store.get(successorKey)});
      }
      requireRevision(state, expectedRevision);
      if (state.phase !== "review_ready") {
        throw new Error("slice campaign supersession requires review-ready source state");
      }
      const reconciliations = [];
      const selectedSkills = new Map(state.reviewSelection?.specialists
        ?.filter(({selection}) => selection === "selected")
        .map(({obligationId, skill}) => [obligationId, skill]) ?? []);
      const obligations = Object.fromEntries(Object.entries(nativeObligations(state)).map(
        ([obligationId, obligation]) => {
          const exactZeroFinding = obligation?.status === "awaiting_builder"
            && ["implementation-review", "claude-recon-implementation"].includes(
              selectedSkills.get(obligationId))
            && Array.isArray(obligation.findings) && obligation.findings.length === 0
            && obligation.episodeRef?.owner === "review-episode"
            && typeof obligation.episodeRef?.revision === "string"
            && obligation.episodeRef.revision.length > 0
            && obligation.remediationExecuting !== true
            && obligation.failure === undefined;
          if (!exactZeroFinding) return [obligationId, obligation];
          reconciliations.push(freeze({schemaVersion: 1, obligationId,
            reason: "existing_reported_zero_finding_binding", episodeRef: obligation.episodeRef,
            providerEntry: false, findingCreated: false, relianceCreated: false,
            reviewAcceptanceImplied: false}));
          return [obligationId, freeze({...obligation, status: "reported",
            reconciliation: reconciliations.at(-1)})];
        },
      ));
      const supersession = freeze({schemaVersion: 1, outcome: "superseded", operationId,
        requestDigest, successorIdentity, reconciliations: freeze(reconciliations),
        authority: freeze({reviewAcceptanceAuthorized: false, campaignAcceptanceAuthorized: false,
          publicationAuthorized: false, providerEntryAuthorized: false})});
      const oldUnpublished = {...state, phase: "superseded", nativeReview: state.nativeReview
        ? nativeEnvelope(obligations) : null, supersession};
      const superseded = freeze({...oldUnpublished, revision: digest({...oldUnpublished, revision: undefined})});
      const successorUnpublished = initialState({identity: successorIdentity, workspace: state.workspace,
        acceptedBoundary: successor.acceptedBoundary, expectedImpact: successor.expectedImpact ?? null,
        baseline: successor.baseline});
      const successorState = freeze({...successorUnpublished,
        revision: digest({...successorUnpublished, revision: undefined})});
      store.supersede({key: sourceKey, expectedRevision: state.revision, workspace: state.workspace,
        state: superseded, successorKey, successorState});
      return freeze({superseded, successor: successorState});
    },
    advance({ identity, expectedRevision, phase, consequence }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      const remediationCycle = state.phase === "review_ready" && phase === "gate_ready"
        && nativeRemediationReady(state);
      if (!SLICE_PHASES.includes(phase) || (NEXT_PHASE.get(state.phase) !== phase && !remediationCycle)) {
        throw new Error("slice campaign phase transition is invalid");
      }
      requireRecord(consequence, "phase consequence");
      return publish({ ...state, phase, latestConsequence: freeze(structuredClone(consequence)) }, state.revision);
    },
    async bindCandidate({ identity, expectedRevision, request }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (state.phase !== "gate_ready") throw new Error("candidate requires gate-ready campaign state");
      const requestDigest = digest(request);
      const replacement = state.candidateRequestDigest !== null
        && state.candidateRequestDigest !== requestDigest
        && nativeRemediationReady(state);
      if (state.candidateRequestDigest && state.candidateRequestDigest !== requestDigest && !replacement) {
        throw new Error("candidate request conflicts with bound candidate");
      }
      const candidate = state.candidate && !replacement
        ? state.candidate
        : await reviewSubject.createCandidate(request);
      const candidateState = state.candidate && !replacement ? state : publish({
        ...state, candidateRequestDigest: requestDigest, candidate, physicalProfile: null,
        ...(replacement ? {nativeReview: replacementNativeEnvelope(state)} : {}),
      }, state.revision);
      const physicalProfile = await reviewSubject.createPhysicalProfile({ subject: {
        schema_version: 2,
        construction_method: "slice_checkpoint_candidate_receipt",
        evidence_cutoff: candidate.created_at,
        checkpoint: candidate,
      } });
      return publish({ ...candidateState, physicalProfile }, candidateState.revision);
    },
    async runLegacyReview({ identity, expectedRevision, selectionPlan }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (state.phase !== "review_ready" || state.review || state.implementationReview
          || state.reviewSelection || state.nativeReview) {
        throw new Error("legacy review requires review-ready state without a prior review");
      }
      if (!legacyReview) throw new Error("legacy review compatibility owner is unavailable");
      if (!state.candidate || !state.physicalProfile) throw new Error("legacy review requires an immutable candidate and physical profile");
      const review = await legacyReview.review({ subject: state.candidate, profile: state.physicalProfile, selectionPlan });
      return publish({ ...state, review }, state.revision);
    },
    bindImplementationReview({ identity, expectedRevision, result }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (state.phase !== "review_ready" || state.review || state.implementationReview
          || state.reviewSelection || state.nativeReview) {
        throw new Error("implementation review requires review-ready state without a prior review");
      }
      if (!state.candidate || !state.physicalProfile) throw new Error("implementation review requires an immutable candidate and physical profile");
      if (!implementationReview || typeof implementationReview.admit !== "function") {
        throw new Error("native implementation-review service is unavailable");
      }
      const subject = {
        commit: state.candidate.checkpoint_commit_oid ?? state.candidate.commit,
        tree: state.candidate.checkpoint_tree_oid ?? state.candidate.tree,
        patchIdentity: state.candidate.task_patch_digest ?? state.candidate.manifestSha256,
      };
      const admitted = implementationReview.admit({ result, expectedSubject: subject });
      return publish({ ...state, implementationReview: admitted }, state.revision);
    },
    bindReviewSelection({ identity, expectedRevision, selection }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (state.phase !== "review_ready" || state.review || state.implementationReview || state.nativeReview) {
        throw new Error("native review selection requires unused review-ready state");
      }
      if (!state.candidate || !state.physicalProfile) throw new Error("native review selection requires immutable candidate and profile");
      const subject = {
        commit: state.candidate.checkpoint_commit_oid ?? state.candidate.commit,
        tree: state.candidate.checkpoint_tree_oid ?? state.candidate.tree,
        patchIdentity: state.candidate.task_patch_digest ?? state.candidate.manifestSha256,
      };
      validateReviewSelection(selection, subject, state.identity);
      if (state.reviewSelection) {
        if (digest(state.reviewSelection) === digest(selection)) return state;
        throw new Error("native review selection conflicts with the bound disposition");
      }
      return publish({ ...state, reviewSelection: freeze(structuredClone(selection)) }, state.revision);
    },
    succeedReviewSelection({identity, expectedRevision, operationId, obligationId,
      authority, successorSelection, observationId, episodeAuthority}) {
      const state = current(identity);
      requireText(operationId, "review selection succession operation identity");
      if (state.reviewSelectionSuccession?.operationId === operationId) {
        if (state.reviewSelectionSuccession.requestDigest === digest({authority, successorSelection, observationId})) {
          return freeze({campaign: state, builderContext: state.reviewSelectionSuccession.builderContext});
        }
        throw new Error("review selection succession operation identity conflicts with durable content");
      }
      requireRevision(state, expectedRevision);
      const obligation = nativeObligations(state)[obligationId];
      if (state.phase !== "review_ready" || obligation?.status !== "evidence_unestablished") {
        throw new Error("review selection succession requires review-ready evidence-unestablished obligation");
      }
      if (!nativeReview?.succeedProductionPathEvidence) {
        throw new Error("production-path correction owner is unavailable");
      }
      const subject = {commit: state.candidate.checkpoint_commit_oid ?? state.candidate.commit,
        tree: state.candidate.checkpoint_tree_oid ?? state.candidate.tree,
        patchIdentity: state.candidate.task_patch_digest ?? state.candidate.manifestSha256};
      validateReviewSelectionSuccession(state.reviewSelection, successorSelection, subject, state.identity);
      const priorSpecialist = state.reviewSelection.specialists.find((item) => item.obligationId === obligationId);
      const nextSpecialist = successorSelection.specialists.find((item) => item.obligationId === obligationId);
      if (!priorSpecialist || !nextSpecialist || priorSpecialist.requiredClaims.length !== 2) {
        throw new Error("review selection succession requires both predecessor claims");
      }
      const predecessorClaims = Object.fromEntries(priorSpecialist.requiredClaims.map((claim) =>
        [claim.consumptionBoundary === "builder_projection" ? "builder" : "terminal", claim]));
      const outcome = nativeReview.succeedProductionPathEvidence({binding: obligation, obligationId,
        authority: episodeAuthority, operationId, correctionAuthority: authority, campaignRevision: state.revision,
        predecessorSelection: state.reviewSelection, successorSelection, predecessorClaims,
        observationId, candidate: subject});
      const requestDigest = digest({authority, successorSelection, observationId});
      const succession = freeze({schemaVersion: 1, operationId, requestDigest,
        predecessorRevision: digest(state.reviewSelection), successorRevision: digest(successorSelection),
        correction: outcome.correction.succession, builderContext: outcome.builderContext,
        consequences: freeze({builder: "projection_enabled",
          terminal: "eligible_for_later_consumption", reviewAccepted: false, campaignAccepted: false})});
      const campaign = publish({...state, reviewSelection: freeze(structuredClone(successorSelection)),
        reviewSelectionSuccession: succession,
        nativeReview: nativeEnvelope({...nativeObligations(state), [obligationId]: outcome.binding}),
        latestConsequence: succession.consequences}, state.revision);
      return freeze({campaign, builderContext: outcome.builderContext});
    },
    async runNativeReview({ identity, expectedRevision, request }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (state.phase !== "review_ready" || state.review || state.implementationReview) {
        throw new Error("native review requires unused review-ready state");
      }
      if (!nativeReview?.executeInitial) throw new Error("native review closure service is unavailable");
      const disposition = state.reviewSelection?.specialists.find(({obligationId}) => obligationId === request?.obligationId);
      if (!disposition || disposition.selection !== "selected") throw new Error("native review obligation is not selected by the supervisor");
      const requestDigest = digest(request);
      const obligations = nativeObligations(state);
      const currentObligation = obligations[request.obligationId] ?? null;
      let prepared = state;
      let allowProviderEntry = false;
      if (currentObligation === null) {
        prepared = publish({...state, nativeReview: nativeEnvelope({...obligations,
          [request.obligationId]: freeze({schemaVersion: 1, obligationId: request.obligationId,
            status: "executing", requestDigest})})}, state.revision);
        allowProviderEntry = true;
      } else if (currentObligation.status !== "executing"
          || currentObligation.requestDigest !== requestDigest) {
        throw new Error("native review request conflicts with durable execution admission");
      }
      const outcome = await nativeReview.executeInitial({...request, reviewSkill: disposition.skill, allowProviderEntry});
      if (outcome.failure) {
        const failed = failedNativeObligation({obligationId: request.obligationId,
          requestDigest, outcome, prior: currentObligation});
        const campaign = publish({...prepared, nativeReview: nativeEnvelope({...nativeObligations(prepared),
          [request.obligationId]: failed})}, prepared.revision);
        return freeze({campaign, builderContext: null, failure: failed.failure});
      }
      const campaign = publish({...prepared, nativeReview: nativeEnvelope({...nativeObligations(prepared),
        [request.obligationId]: outcome.binding})}, prepared.revision);
      return freeze({campaign, builderContext: outcome.builderContext, failure: null});
    },
    async retryNativeReview({ identity, expectedRevision, request, recovery }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      const currentObligation = nativeObligations(state)[request?.obligationId] ?? null;
      if (state.phase !== "review_ready"
          || !["executing", "retryable_failure", "retry_executing"].includes(currentObligation?.status)) {
        throw new Error("native review retry requires an unresolved admitted obligation");
      }
      if (!nativeReview?.executeInitial) throw new Error("native review closure service is unavailable");
      const disposition = state.reviewSelection?.specialists.find(({obligationId}) => obligationId === request.obligationId);
      if (!disposition || disposition.selection !== "selected") throw new Error("native review retry obligation is not selected by the supervisor");
      requireRecord(recovery, "native review retry recovery");
      const retainedAuthentication = recovery.failureSignature === "authentication_required"
        && recovery.sessionAvailable === true
        && recovery.sessionId === request.reviewerRequest?.continuationSessionId;
      const preSpawnAuthentication = recovery.failureSignature === "authentication_unavailable"
        && recovery.sessionAvailable === false
        && recovery.sessionId === request.retrySessionId
        && request.reviewerRequest?.continuationSessionId === undefined;
      const preSpawnProcess = recovery.failureSignature === "process_start_failed"
        && recovery.sessionAvailable === false
        && recovery.sessionId === request.retrySessionId
        && request.reviewerRequest?.continuationSessionId === undefined
        && request.reviewerRequest?.preSpawnRetry === true
        && typeof recovery.errorCode === "string" && Boolean(recovery.errorCode.trim());
      if (recovery.providerEntry !== "not_entered"
          || (!retainedAuthentication && !preSpawnAuthentication && !preSpawnProcess)) {
        throw new Error("native review retry lacks exact definite pre-provider failure evidence");
      }
      const recoveryChanged = currentObligation.status === "retry_executing"
        && digest(currentObligation.recovery) !== digest(recovery);
      const recoveredAuthenticationRetry = recoveryChanged
        && currentObligation.recovery?.failureSignature === "authentication_required"
        && currentObligation.recovery?.providerEntry === "not_entered"
        && currentObligation.recovery?.sessionAvailable === true
        && currentObligation.recovery?.sessionId === recovery.sessionId
        && retainedAuthentication;
      if (recoveryChanged && !recoveredAuthenticationRetry) {
        throw new Error("native review retry execution recovery differs from durable pre-provider evidence");
      }
      const executionRequest = retainedAuthentication || preSpawnProcess
        ? freeze({...request, reviewerRequest: freeze({...request.reviewerRequest, refreshCredentials: true})})
        : request;
      const requestDigest = digest(executionRequest);
      const preparedObligation = freeze({...currentObligation, status: "retry_executing",
        requestDigest, recovery: freeze(structuredClone(recovery)),
        ...(recoveredAuthenticationRetry ? {priorRecoveries: freeze([
          ...(currentObligation.priorRecoveries ?? []),
          freeze(structuredClone(currentObligation.recovery)),
        ])} : {})});
      const prepared = publish({...state, nativeReview: nativeEnvelope({...nativeObligations(state),
        [request.obligationId]: preparedObligation})}, state.revision);
      const outcome = await nativeReview.executeInitial({...executionRequest, reviewSkill: disposition.skill,
        allowProviderEntry: true, resumeExistingEpisode: true});
      if (outcome.failure) {
        const failed = failedNativeObligation({obligationId: request.obligationId,
          requestDigest, outcome, prior: preparedObligation});
        const campaign = publish({...prepared, nativeReview: nativeEnvelope({...nativeObligations(prepared),
          [request.obligationId]: failed})}, prepared.revision);
        return freeze({campaign, builderContext: null, failure: failed.failure});
      }
      const campaign = publish({...prepared, nativeReview: nativeEnvelope({...nativeObligations(prepared),
        [request.obligationId]: outcome.binding})}, prepared.revision);
      return freeze({campaign, builderContext: outcome.builderContext, failure: null});
    },
    async correctNativeReviewResult({ identity, expectedRevision, request, recovery }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      const currentObligation = nativeObligations(state)[request?.obligationId] ?? null;
      if (state.phase !== "review_ready"
          || !["correction_required", "retry_executing", "correction_executing",
            "remediation_executing"].includes(currentObligation?.status)) {
        throw new Error("native review result correction requires an admitted contract-rejected result");
      }
      if (!nativeReview?.executeCorrection || !nativeReview?.recoverCorrection) {
        throw new Error("native review result correction service is unavailable");
      }
      const disposition = state.reviewSelection?.specialists.find(({obligationId}) => obligationId === request.obligationId);
      if (!disposition || disposition.selection !== "selected") throw new Error("native review result correction obligation is not selected by the supervisor");
      requireRecord(recovery, "native review result correction recovery");
      requireSha256(recovery.transportReceiptDigest,
        "native review result correction transport receipt digest");
      const correcting = recovery.failureSignature === "result_contract_rejected";
      const recovered = recovery.correctedResult ?? recovery.rejectedResult;
      const recoveredImplementation = recovered?.result ?? recovered;
      const expectedResultDigest = correcting
        ? recovery.rejectedResultDigest : recovery.correctedResultDigest;
      const expectedSubjectDigest = correcting
        ? recovery.rejectedSubjectDigest : recovery.correctedSubjectDigest;
      if (!["result_contract_rejected", "result_contract_corrected"].includes(recovery.failureSignature)
          || recovery.providerEntry !== "entered" || recovery.sessionAvailable !== true
          || recovery.sessionId !== request.reviewerRequest?.continuationSessionId
          || digest(recovered) !== expectedResultDigest
          || digest(recoveredImplementation?.subject) !== expectedSubjectDigest
          || (!correcting && !["retry_executing", "correction_executing",
            "remediation_executing"].includes(currentObligation.status))) {
        throw new Error("native review result correction lacks exact provider result and retained-session evidence");
      }
      const requestDigest = digest(request);
      if (!correcting) {
        const outcome = nativeReview.recoverCorrection({...request, binding: currentObligation,
          recoveredResult: recoveredImplementation,
          recoveredExecution: freeze({attemptId: null, runtimeSessionId: recovery.sessionId,
            receipt: freeze({transportReceiptDigest: recovery.transportReceiptDigest})})});
        if (outcome.failure) {
          const failed = failedNativeObligation({obligationId: request.obligationId,
            requestDigest, outcome, prior: currentObligation});
          const campaign = publish({...state, nativeReview: nativeEnvelope({...nativeObligations(state),
            [request.obligationId]: failed})}, state.revision);
          return freeze({campaign, builderContext: null, failure: failed.failure});
        }
        const campaign = publish({...state, nativeReview: nativeEnvelope({...nativeObligations(state),
          [request.obligationId]: outcome.binding})}, state.revision);
        return freeze({campaign, builderContext: outcome.builderContext, failure: null});
      }
      const preparedObligation = freeze({...currentObligation, status: "correction_executing",
        requestDigest, recovery: freeze(structuredClone(recovery))});
      const prepared = publish({...state, nativeReview: nativeEnvelope({...nativeObligations(state),
        [request.obligationId]: preparedObligation})}, state.revision);
      const outcome = await nativeReview.executeCorrection({...request,
        binding: preparedObligation, reviewSkill: disposition.skill, allowProviderEntry: true});
      if (outcome.failure) {
        const failed = failedNativeObligation({obligationId: request.obligationId,
          requestDigest, outcome, prior: preparedObligation});
        const campaign = publish({...prepared, nativeReview: nativeEnvelope({...nativeObligations(prepared),
          [request.obligationId]: failed})}, prepared.revision);
        return freeze({campaign, builderContext: null, failure: failed.failure});
      }
      const campaign = publish({...prepared, nativeReview: nativeEnvelope({...nativeObligations(prepared),
        [request.obligationId]: outcome.binding})}, prepared.revision);
      return freeze({campaign, builderContext: outcome.builderContext, failure: null});
    },
    recordNativeFindingEvaluation({ identity, expectedRevision, request }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      const currentObligation = nativeObligations(state)[request?.obligationId] ?? null;
      if (state.phase !== "review_ready"
          || !["awaiting_builder", "reported"].includes(currentObligation?.status)) {
        throw new Error("native finding evaluation requires a review closure with an exact current finding revision");
      }
      if (!nativeReview?.recordBuilderEvaluation) throw new Error("native review closure service is unavailable");
      const binding = nativeReview.recordBuilderEvaluation({binding: currentObligation, ...request});
      return freeze({campaign: publish({...state, nativeReview: nativeEnvelope({...nativeObligations(state),
        [request.obligationId]: binding})}, state.revision)});
    },
    async runNativeRemediation({ identity, expectedRevision, request, remediationSubjectReference = null,
      legacyAdmittedRequest = null }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      const currentObligation = nativeObligations(state)[request?.obligationId] ?? null;
      if (state.phase !== "review_ready"
          || (!["awaiting_builder", "remediation_executing"].includes(currentObligation?.status)
            && !reportedZeroFindingClosure(currentObligation))) {
        throw new Error("native remediation requires an awaiting-builder or reported zero-finding closure");
      }
      if (!nativeReview?.executeRemediation || !nativeReview?.recoverRemediation) {
        throw new Error("native review closure service is unavailable");
      }
      const disposition = state.reviewSelection?.specialists.find(({obligationId}) => obligationId === request?.obligationId);
      if (!disposition || disposition.selection !== "selected") throw new Error("native remediation obligation is not selected by the supervisor");
      const requestDigest = digest(request);
      let prepared = state;
      let allowProviderEntry = false;
      if (currentObligation.status === "awaiting_builder"
          || (reportedZeroFindingClosure(currentObligation)
            && currentObligation.remediationExecuting !== true)) {
        prepared = publish({...state, nativeReview: nativeEnvelope({...nativeObligations(state),
          [request.obligationId]: freeze({...currentObligation,
            ...(currentObligation.status === "awaiting_builder" ? {status: "remediation_executing"} : {}),
            ...(currentObligation.status === "reported" ? {remediationExecuting: true} : {}),
            requestDigest})})}, state.revision);
        allowProviderEntry = true;
      } else {
        const recoveryRequest = {
          binding: currentObligation,
          authority: request.authority,
          subjectTransitionId: request.subjectTransitionId,
          resultTransitionId: request.resultTransitionId,
        };
        if (currentObligation.requestDigest !== requestDigest) {
          const legacyAuthority = legacyAdmittedRequest?.authority;
          const reconstructed = legacyAuthority && request.authority ? {
            ...legacyAdmittedRequest,
            authority: {...legacyAuthority, initialSubject: request.authority.initialSubject},
          } : null;
          const exactLegacyAuthorityDefect = Boolean(legacyAdmittedRequest !== null
            && legacyAuthority?.initialSubject
            && digest(legacyAdmittedRequest) === currentObligation.requestDigest
            && digest(reconstructed) === requestDigest
            && digest(legacyAuthority.initialSubject) === digest(remediationSubjectReference));
          if (!exactLegacyAuthorityDefect) {
            throw new Error("native remediation request conflicts with durable execution admission");
          }
          const recovery = nativeReview.recoverRemediation(recoveryRequest);
          if (recovery.providerEntry !== "not_entered") {
            throw new Error("native remediation request conflicts with durable execution admission");
          }
          prepared = publish({...state, nativeReview: nativeEnvelope({...nativeObligations(state),
            [request.obligationId]: freeze({...currentObligation, requestDigest,
              admissionCorrection: freeze({schemaVersion: 1,
                reason: "legacy_authority_initial_subject_reconstructed_pre_provider",
                previousRequestDigest: currentObligation.requestDigest})})})}, state.revision);
          allowProviderEntry = true;
        } else {
          const recovery = nativeReview.recoverRemediation(recoveryRequest);
          if (recovery.providerEntry === "not_entered") allowProviderEntry = true;
        }
      }
      const outcome = await nativeReview.executeRemediation({binding: nativeObligations(prepared)[request.obligationId],
        ...request,
        remediationSubject: remediationSubjectReference ?? request.remediationSubject,
        reviewSkill: disposition.skill, allowProviderEntry});
      if (outcome.failure) {
        const failed = failedNativeObligation({obligationId: request.obligationId,
          requestDigest, outcome, prior: nativeObligations(prepared)[request.obligationId]});
        const campaign = publish({...prepared, nativeReview: nativeEnvelope({...nativeObligations(prepared),
          [request.obligationId]: failed})}, prepared.revision);
        return freeze({campaign, builderContext: null, failure: failed.failure});
      }
      const campaign = publish({...prepared, nativeReview: nativeEnvelope({...nativeObligations(prepared),
        [request.obligationId]: completedNativeRemediationBinding(outcome.binding)})}, prepared.revision);
      return freeze({campaign, builderContext: outcome.builderContext, failure: null});
    },
    async terminalize({ identity, expectedRevision, outcome, receipt }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      const terminalRequestDigest = digest({ outcome, receipt, candidate: state.candidate });
      if (state.phase === "terminal") {
        if (state.terminal?.requestDigest === terminalRequestDigest) return state;
        throw new Error("terminal campaign request conflicts with durable terminal state");
      }
      const selected = state.reviewSelection?.specialists.filter(({selection}) => selection === "selected") ?? [];
      const obligations = nativeObligations(state);
      const productionPathSelected = state.reviewSelection?.schemaVersion === 2;
      const terminalCandidateSubject = {commit: state.candidate?.checkpoint_commit_oid ?? state.candidate?.commit,
        tree: state.candidate?.checkpoint_tree_oid ?? state.candidate?.tree,
        patchIdentity: state.candidate?.task_patch_digest ?? state.candidate?.manifestSha256};
      const requiredEvidence = productionPathSelected ? selected.flatMap(({obligationId, requiredClaims}) => {
        const admissions = obligations[obligationId]?.claimEvidence ?? [];
        return requiredClaims.map((claim) => admissions.find(({claimRevisionRef}) =>
          claimRevisionRef.revision === claim.revision) ?? null);
      }) : [];
      const productionPathComplete = !productionPathSelected || (requiredEvidence.length === selected.length * 2
        && selected.every(({requiredClaims}) => requiredClaims.every((claim) =>
          digest(claim.subject.candidate) === digest(terminalCandidateSubject)))
        && requiredEvidence.every((item) => item !== null && item.status === "established"
          && item.claimRevisionRef && item.establishmentRef && item.consumptionRef)
        && requiredEvidence.filter(({boundary}) => boundary === "campaign_terminalization").length === selected.length
        && requiredEvidence.filter(({boundary}) => boundary === "builder_projection").length === selected.length);
      const nativeComplete = state.reviewSelection !== null && selected.length > 0
        && selected.every(({obligationId}) => obligations[obligationId]?.status === "reported"
          && obligations[obligationId].remediationExecuting !== true)
        && Object.keys(obligations).every((obligationId) => selected.some((item) => item.obligationId === obligationId));
      const compatibilityComplete = state.review !== null && state.reviewSelection === null && state.nativeReview === null;
      if (state.phase === "review_ready" && !productionPathComplete) {
        throw new Error("terminalization requires established production-path claim evidence");
      }
      if (state.phase !== "review_ready" || (!compatibilityComplete && !nativeComplete)) {
        throw new Error("terminalization requires completed native closure or compatibility review");
      }
      requireText(outcome, "terminal outcome"); requireRecord(receipt, "terminal receipt");
      const terminalReceipt = productionPathSelected ? freeze({...structuredClone(receipt),
        productionPathClaimEvidence: freeze(structuredClone(requiredEvidence))}) : receipt;
      const finalizedReceipt = await receiptFinalizer.finalize({ identity: state.identity, outcome,
        receipt: terminalReceipt, candidate: state.candidate });
      return publish({ ...state, phase: "terminal", terminal: freeze({
        outcome, finalizedReceipt, completionOffer: null,
        completionOfferRequestDigest: null, completionOfferSupersession: null,
        requestDigest: terminalRequestDigest,
      }) }, state.revision, { releaseWorkspace: state.workspace });
    },
    async openCompletionOffer({ identity, expectedRevision, request }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (state.phase !== "terminal") {
        throw new Error("completion offer requires terminal campaign state");
      }
      if (!completionOffer) throw new Error("completion-offer owner is unavailable");
      requireRecord(request, "completion offer request");
      const requestDigest = digest(request);
      if (state.terminal.completionOffer !== null) {
        if (state.terminal.completionOfferRequestDigest === requestDigest) return state;
        throw new Error("completion offer request conflicts with durable terminal state");
      }
      const offer = await completionOffer.open({
        identity: state.identity,
        outcome: state.terminal.outcome,
        candidate: state.candidate,
        request,
      });
      return publish({ ...state, terminal: freeze({
        ...state.terminal,
        completionOffer: freeze(structuredClone(offer)),
        completionOfferRequestDigest: requestDigest,
      }) }, state.revision);
    },
    async supersedeCompletionOffer({ identity, expectedRevision, expectedOfferId,
      operationId, request, reason, publicationState }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (state.phase !== "terminal" || state.terminal?.completionOffer === null) {
        throw new Error("completion offer supersession requires a terminal campaign offer");
      }
      if (!completionOffer || typeof completionOffer.supersede !== "function") {
        throw new Error("completion-offer supersession owner is unavailable");
      }
      requireText(expectedOfferId, "expected completion offer identity");
      requireText(operationId, "completion offer supersession operation identity");
      requireRecord(request, "completion offer successor request");
      requireText(reason, "completion offer supersession reason");
      const requestDigest = digest(request);
      if (state.terminal.completionOfferRequestDigest === requestDigest) {
        if (state.terminal.completionOfferSupersession?.operation_id === operationId) return state;
        throw new Error("completion offer supersession operation conflicts with durable terminal state");
      }
      if (state.terminal.completionOffer.offer_id !== expectedOfferId) {
        throw new Error("completion offer supersession expected offer conflicts with durable terminal state");
      }
      const result = await completionOffer.supersede({
        offer: state.terminal.completionOffer, request, operationId, reason, publicationState,
      });
      return publish({ ...state, terminal: freeze({
        ...state.terminal,
        completionOffer: freeze(structuredClone(result.offer)),
        completionOfferRequestDigest: requestDigest,
        completionOfferSupersession: freeze(structuredClone(result.supersession)),
      }) }, state.revision);
    },
  });
}
