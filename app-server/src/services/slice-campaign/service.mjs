import {
  SLICE_CAMPAIGN_SCHEMA_VERSION, SLICE_PHASES, digest, freeze, identityKey,
  normalizeIdentity, requireRecord, requireSha256, requireText, validateReviewSelection,
} from "./contract.mjs";

const NEXT_PHASE = new Map([["accepted", "implementing"], ["implementing", "gate_ready"], ["gate_ready", "review_ready"], ["review_ready", "terminal"]]);

export class InMemorySliceCampaignStore {
  constructor() { this.states = new Map(); this.workspaceAdmissions = new Map(); }
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
  const nativeEnvelope = (obligations) => freeze({schemaVersion: 1, obligations: freeze({...obligations})});
  const failedNativeObligation = ({obligationId, requestDigest, outcome, prior = null}) => {
    const failure = freeze(structuredClone(outcome.failure));
    const status = failure.failureSignature === "result_contract_rejected"
      ? "correction_required"
      : failure.providerEntry === "not_entered" ? "retryable_failure" : "executing";
    return freeze({schemaVersion: 1, obligationId,
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
      const initial = { schemaVersion: SLICE_CAMPAIGN_SCHEMA_VERSION, identity: normalized, workspace,
        acceptedBoundary: freeze(structuredClone(acceptedBoundary)), expectedImpact: expectedImpact && freeze(structuredClone(expectedImpact)),
        baseline: freeze(structuredClone(baseline)), phase: "accepted", latestConsequence: null,
        candidateRequestDigest: null, candidate: null, physicalProfile: null,
        review: null, implementationReview: null, reviewSelection: null, nativeReview: null, terminal: null };
      const published = freeze({ ...initial, revision: digest(initial) });
      store.admit(key, workspace, published);
      return published;
    },
    recover(identity) { return current(identity); },
    advance({ identity, expectedRevision, phase, consequence }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (!SLICE_PHASES.includes(phase) || NEXT_PHASE.get(state.phase) !== phase) throw new Error("slice campaign phase transition is invalid");
      requireRecord(consequence, "phase consequence");
      return publish({ ...state, phase, latestConsequence: freeze(structuredClone(consequence)) }, state.revision);
    },
    async bindCandidate({ identity, expectedRevision, request }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (state.phase !== "gate_ready") throw new Error("candidate requires gate-ready campaign state");
      const requestDigest = digest(request);
      if (state.candidateRequestDigest && state.candidateRequestDigest !== requestDigest) throw new Error("candidate request conflicts with bound candidate");
      const candidate = state.candidate ?? await reviewSubject.createCandidate(request);
      const candidateState = state.candidate ? state : publish({ ...state, candidateRequestDigest: requestDigest, candidate }, state.revision);
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
      validateReviewSelection(selection, subject);
      if (state.reviewSelection) {
        if (digest(state.reviewSelection) === digest(selection)) return state;
        throw new Error("native review selection conflicts with the bound disposition");
      }
      return publish({ ...state, reviewSelection: freeze(structuredClone(selection)) }, state.revision);
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
      if (state.phase !== "review_ready" || !["executing", "retryable_failure"].includes(currentObligation?.status)) {
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
      if (recovery.providerEntry !== "not_entered"
          || (!retainedAuthentication && !preSpawnAuthentication)) {
        throw new Error("native review retry lacks exact definite pre-provider failure evidence");
      }
      const executionRequest = retainedAuthentication
        ? freeze({...request, reviewerRequest: freeze({...request.reviewerRequest, refreshCredentials: true})})
        : request;
      const requestDigest = digest(executionRequest);
      const preparedObligation = freeze({...currentObligation, status: "retry_executing",
        requestDigest, recovery: freeze(structuredClone(recovery))});
      const prepared = publish({...state, nativeReview: nativeEnvelope({...nativeObligations(state),
        [request.obligationId]: preparedObligation})}, state.revision);
      const outcome = await nativeReview.executeInitial({...executionRequest, reviewSkill: disposition.skill,
        allowProviderEntry: true});
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
          || !["correction_required", "retry_executing", "correction_executing"].includes(currentObligation?.status)) {
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
          || (!correcting && !["retry_executing", "correction_executing"].includes(currentObligation.status))) {
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
    async runNativeRemediation({ identity, expectedRevision, request }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      const currentObligation = nativeObligations(state)[request?.obligationId] ?? null;
      if (state.phase !== "review_ready"
          || !["awaiting_builder", "remediation_executing"].includes(currentObligation?.status)) {
        throw new Error("native remediation requires an awaiting-builder closure");
      }
      if (!nativeReview?.executeRemediation) throw new Error("native review closure service is unavailable");
      const disposition = state.reviewSelection?.specialists.find(({obligationId}) => obligationId === request?.obligationId);
      if (!disposition || disposition.selection !== "selected") throw new Error("native remediation obligation is not selected by the supervisor");
      const requestDigest = digest(request);
      let prepared = state;
      let allowProviderEntry = false;
      if (currentObligation.status === "awaiting_builder") {
        prepared = publish({...state, nativeReview: nativeEnvelope({...nativeObligations(state),
          [request.obligationId]: freeze({...currentObligation,
            status: "remediation_executing", requestDigest})})}, state.revision);
        allowProviderEntry = true;
      } else if (currentObligation.requestDigest !== requestDigest) {
        throw new Error("native remediation request conflicts with durable execution admission");
      }
      const outcome = await nativeReview.executeRemediation({binding: nativeObligations(prepared)[request.obligationId],
        ...request, reviewSkill: disposition.skill, allowProviderEntry});
      const campaign = publish({...prepared, nativeReview: nativeEnvelope({...nativeObligations(prepared),
        [request.obligationId]: outcome.binding})}, prepared.revision);
      return freeze({campaign, builderContext: outcome.builderContext});
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
      const nativeComplete = state.reviewSelection !== null && selected.length > 0
        && selected.every(({obligationId}) => obligations[obligationId]?.status === "reported")
        && Object.keys(obligations).every((obligationId) => selected.some((item) => item.obligationId === obligationId));
      const compatibilityComplete = state.review !== null && state.reviewSelection === null && state.nativeReview === null;
      if (state.phase !== "review_ready" || (!compatibilityComplete && !nativeComplete)) {
        throw new Error("terminalization requires completed native closure or compatibility review");
      }
      requireText(outcome, "terminal outcome"); requireRecord(receipt, "terminal receipt");
      const finalizedReceipt = await receiptFinalizer.finalize({ identity: state.identity, outcome, receipt, candidate: state.candidate });
      return publish({ ...state, phase: "terminal", terminal: freeze({
        outcome, finalizedReceipt, completionOffer: null,
        completionOfferRequestDigest: null, requestDigest: terminalRequestDigest,
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
  });
}
