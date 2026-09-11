import { digest as episodeDigest } from "../review-episode/contract.mjs";
import { ReviewEpisodeResultError } from "../review-episode/service.mjs";
import { ImplementationReviewError } from "../implementation-review/contract.mjs";
import { AgentInstructionReviewError } from "../agent-instruction-review/contract.mjs";
import { digest as claimDigest } from "../claim-evidence/identity.mjs";
import {
  normalizeProductionPathObservation, productionPathReference,
} from "../claim-evidence/production-path-contract.mjs";

function reference(owner, reference, revision, value) {
  return Object.freeze({owner, reference, revision, sha256: episodeDigest(value), freshness: "exact immutable revision"});
}
function episodeRef(state) {
  return reference("review-episode", `review-episode@${episodeDigest(state.identity)}`, state.revision, state);
}
function status(episode, findings) {
  if (episode.phase === "evidence_unestablished") return "evidence_unestablished";
  return episode.phase === "reported" || findings.length === 0 ? "reported" : "awaiting_builder";
}
function relianceComplete(findings) {
  return findings.length > 0
    && findings.every((item) => item.relianceRef !== null
      && (item.builderEvaluation?.disposition === "invalid"
        || (item.builderEvaluation?.disposition === "valid" && item.outcome === "verified_resolved")));
}
function withRelianceStatus(current, findings) {
  return Object.freeze({...current,
    status: current.status === "awaiting_builder" && relianceComplete(findings) ? "reported" : current.status,
    findings});
}
function binding({obligationId, episode, findings, prior = null}) {
  return Object.freeze({
    schemaVersion: 1, obligationId, status: status(episode, findings),
    episodeRef: episodeRef(episode), runtimeSessionRef: episode.writer.runtimeSession, findings,
    claimEvidence: Object.freeze(structuredClone(episode.evidenceAdmissions ?? [])),
    initialEpisodeRef: prior?.initialEpisodeRef ?? episodeRef(episode),
    authority: Object.freeze({reviewerSelectionAuthorized: false, findingEvaluationAuthorized: false,
      reviewAcceptanceAuthorized: false, campaignAcceptanceAuthorized: false, mutationAuthorized: false}),
  });
}

async function executeReviewer(reviewer, reviewSkill, reviewerRequest, claimContext) {
  if (reviewSkill === "agent-instruction-review") {
    if (typeof reviewer.reviewAgentInstructions !== "function") {
      throw new Error("agent-instruction-review obligation requires the canonical specialist reviewer route");
    }
    const execution = await reviewer.reviewAgentInstructions({...reviewerRequest, claimContext});
    return Object.freeze({execution,
      result: execution.specialistReview?.implementationReviewResult ?? null,
      contractError: execution.contractError ?? null,
      rejectedResult: execution.contractError ? execution.result : null});
  }
  if (!["implementation-review", "claude-recon-implementation"].includes(reviewSkill)) {
    throw new Error("native review obligation names an unsupported selected skill");
  }
  const execution = await reviewer.review({...reviewerRequest, claimContext});
  return Object.freeze({execution, result: execution.result ?? null,
    contractError: null, rejectedResult: null});
}

function resultContractFailure(error, result, execution) {
  if (!(error instanceof ImplementationReviewError)
      && !(error instanceof AgentInstructionReviewError)
      && !(error instanceof ReviewEpisodeResultError)) throw error;
  const rejectedResult = Object.freeze(structuredClone(result));
  const implementationResult = rejectedResult.result ?? rejectedResult;
  const recovery = Object.freeze({schemaVersion: 1,
    failureSignature: "result_contract_rejected", providerEntry: "entered",
    sessionAvailable: true, sessionId: execution.runtimeSessionId,
    message: error.message,
    rejectedResult, rejectedResultDigest: episodeDigest(rejectedResult),
    rejectedSubjectDigest: episodeDigest(implementationResult.subject),
    transportReceiptDigest: execution.receipt?.transportReceiptDigest ?? null,
  });
  return Object.freeze({kind: "result_contract", message: error.message,
    providerEntry: "entered", failureSignature: "result_contract_rejected",
    sessionAvailable: true, recovery});
}

function recordResult({reviewEpisode, authority, episode, transitionId, result, execution, evidenceAdmissions = []}) {
  try {
    return Object.freeze({episode: reviewEpisode.transition({authority,
      expectedRevision: episode.revision, transitionId, action: "record_result",
      payload: {result, unresolvedQuestions: [], ...(evidenceAdmissions.length ? {evidenceAdmissions} : {})}}), failure: null});
  } catch (error) {
    return Object.freeze({episode, failure: resultContractFailure(error, result, execution)});
  }
}

function reviewerContextRequest({contextRequest, authority, reviewerRequest}) {
  return Object.freeze({...contextRequest, requestId: `${contextRequest.requestId}:reviewer`, consumer: Object.freeze({
    identity: authority.writer.actorId,
    revision: reviewerRequest.subject.tree,
    decision_scope: contextRequest.consumer.decision_scope,
  })});
}

export function createNativeReviewClosureService({reviewEpisode, reviewer, findingBridge,
  productionPathEvidence = null} = {}) {
  if (!reviewEpisode?.begin || !reviewEpisode?.transition || !reviewEpisode?.recover
      || !reviewEpisode?.read) throw new TypeError("native review closure requires Review Episode");
  if (!reviewer?.review) throw new TypeError("native review closure requires canonical reviewer runtime");
  if (!findingBridge?.publishFindings || !findingBridge?.recordReliance || !findingBridge?.project) throw new TypeError("native review closure requires review finding bridge");
  const admitProductionPath = ({requiredClaims = [], execution, result, authority, operationPrefix,
    selection}) => {
    if (!requiredClaims.length) return [];
    if (!productionPathEvidence?.admit) throw new Error("production-path evidence service is unavailable");
    const receipt = execution.receipt ?? null;
    let observation = null;
    if (receipt?.transportReceiptDigest) {
      observation = normalizeProductionPathObservation({
        event_identity: `production-path-event-v1@${claimDigest({selection, obligationId: authority.identity.reviewObligationId, attemptId: execution.attemptId})}`,
        selection, obligationId: authority.identity.reviewObligationId,
        subject: {candidate: result.subject, reviewEpisodeId: authority.identity.reviewEpisodeId},
        coveredState: "admitted_native_review_result",
        execution: {attemptId: execution.attemptId, resultDigest: claimDigest(result)},
        realization: {requested: receipt.requestedModel, observed: receipt.observedModel},
        capabilityEnvelope: {capabilities: receipt.capabilities ?? [], mutationAuthorized: receipt.mutationAuthorized},
        continuity: {mode: receipt.continuity, sessionId: receipt.sessionId},
        transport: {mechanism: receipt.evidenceMechanism, digest: receipt.transportReceiptDigest},
        observer: {identity: receipt.observerIdentity, kind: "app_server_host"},
        observedAt: receipt.observedAt, adapterVersion: receipt.profileConfigurationDigest,
        artifacts: receipt.artifacts ?? [],
      });
    }
    return requiredClaims.map((claim) => {
      const establishment = productionPathEvidence.admit({
        operationId: `${operationPrefix}:establish:${claim.revision}`, claim, observation,
      });
      const claimRef = productionPathReference("claim-evidence", claim.claimId, claim.revision, claim);
      const establishmentRef = productionPathReference("claim-evidence", establishment.id, establishment.id, establishment);
      const observationRef = observation === null ? null
        : productionPathReference("claim-evidence", observation.id, observation.id, observation);
      const consumption = {schemaVersion: 1, claimRevision: claim.revision,
        establishment: establishment.id, boundary: claim.consumptionBoundary, consumer: claim.consumer};
      const consumptionRef = productionPathReference("slice-campaign", `claim-consumption:${claim.claimId}`,
        claimDigest(consumption), consumption);
      return Object.freeze({claimRevisionRef: claimRef, establishmentRef, observationRef,
        consumptionRef, status: establishment.status, boundary: claim.consumptionBoundary,
        consumer: claim.consumer});
    });
  };
  return Object.freeze({
    succeedProductionPathEvidence({binding: current, obligationId, authority, operationId,
      correctionAuthority, campaignRevision, predecessorSelection, successorSelection,
      predecessorClaims, observationId, candidate}) {
      const episode = reviewEpisode.recover(authority.identity);
      if (!episode || episode.revision !== current?.episodeRef?.revision
          || episode.phase !== "evidence_unestablished") {
        throw new Error("production-path correction requires the exact evidence-unestablished Episode");
      }
      const originalResultDigest = episodeDigest(episode.currentResult);
      const originalAdmissions = structuredClone(episode.evidenceAdmissions);
      const corrected = productionPathEvidence.correct({operationId, authority: correctionAuthority,
        campaignRevision, selection: {id: successorSelection.selectionId,
          predecessorRevision: episodeDigest(predecessorSelection),
          successorRevision: episodeDigest(successorSelection)}, claims: predecessorClaims,
        observationId, reviewEpisode: {id: authority.identity.reviewEpisodeId,
          predecessorRevision: episode.revision}, candidate,
        succeedEpisode({successors, establishments, observation}) {
          const admissions = Object.entries(successors).map(([name, claim]) => {
            const establishment = establishments[name];
            const consumption = {schemaVersion: 1, claimRevision: claim.revision,
              establishment: establishment.id, boundary: claim.consumptionBoundary,
              consumer: claim.consumer};
            return Object.freeze({
              claimRevisionRef: productionPathReference("claim-evidence", claim.claimId, claim.revision, claim),
              establishmentRef: productionPathReference("claim-evidence", establishment.id, establishment.id, establishment),
              observationRef: productionPathReference("claim-evidence", observation.id, observation.id, observation),
              consumptionRef: productionPathReference("slice-campaign", `claim-consumption:${claim.claimId}`,
                claimDigest(consumption), consumption), status: establishment.status,
              boundary: claim.consumptionBoundary, consumer: claim.consumer,
            });
          });
          const succeeded = reviewEpisode.transition({authority, expectedRevision: episode.revision,
            transitionId: `${operationId}:episode-succession`, action: "succeed_evidence",
            payload: {predecessorAdmissions: originalAdmissions, successorAdmissions: admissions}});
          return succeeded.revision;
        }});
      const succeededEpisode = reviewEpisode.recover(authority.identity);
      if (episodeDigest(succeededEpisode.currentResult) !== originalResultDigest) {
        throw new Error("production-path correction rewrote the immutable review result");
      }
      const findings = current.findings ?? [];
      const nextBinding = binding({obligationId, episode: succeededEpisode, findings, prior: current});
      const builderClaims = succeededEpisode.evidenceAdmissions.filter(({boundary, status}) =>
        boundary === "builder_projection" && status === "established").slice(-1);
      return Object.freeze({binding: nextBinding, correction: corrected,
        builderContext: Object.freeze({schemaVersion: 1, findings: structuredClone(findings),
          requiredClaimEvidence: structuredClone(builderClaims)})});
    },
    recoverInitialSubject({binding: current, identity}) {
      const reference = current?.initialEpisodeRef;
      if (!reference) throw new Error("native review initial episode binding is unavailable");
      const initial = reviewEpisode.read({identity, revision: reference.revision});
      if (!initial || episodeDigest(episodeRef(initial)) !== episodeDigest(reference)) {
        throw new Error("native review initial episode binding is invalid");
      }
      return initial.subject;
    },
    validateRecoveredResult({binding: current, authority, result}) {
      const bound = reviewEpisode.read({identity: authority.identity,
        revision: current.episodeRef.revision});
      if (!bound) throw new Error("native review episode binding is unavailable");
      const episode = reviewEpisode.recover(authority.identity);
      if (!episode) throw new Error("native review episode binding is unavailable");
      return reviewEpisode.validateResult({authority,
        expectedRevision: episode.revision, result});
    },
    recoverRemediation({binding: current, authority, subjectTransitionId, resultTransitionId}) {
      const episode = reviewEpisode.recover(authority.identity);
      if (!episode) throw new Error("native review episode binding is unavailable");
      const subjectRecorded = Boolean(episode.handledTransitions[subjectTransitionId]);
      const resultRecorded = Boolean(episode.handledTransitions[resultTransitionId]);
      if (episode.revision !== current.episodeRef.revision && !subjectRecorded) {
        throw new Error("native review episode binding is stale");
      }
      return Object.freeze({
        providerEntry: resultRecorded ? "entered" : subjectRecorded ? "unknown" : "not_entered",
        subjectRecorded,
        resultRecorded,
      });
    },
    async executeInitial({obligationId, reviewSkill, authority, beginTransitionId, resultTransitionId,
      reviewerRequest, findingAuthority, operationPrefix, contextRequest, allowProviderEntry = true,
      resumeExistingEpisode = false, requiredClaims = [], selection = null}) {
      if (typeof resumeExistingEpisode !== "boolean") {
        throw new TypeError("native review initial episode resume flag must be boolean");
      }
      if (resumeExistingEpisode && typeof reviewEpisode.resumeInitial !== "function") {
        throw new Error("native review initial episode resume service is unavailable");
      }
      let episode = resumeExistingEpisode
        ? reviewEpisode.resumeInitial({authority})
        : reviewEpisode.begin({authority, transitionId: beginTransitionId});
      if (!episode.handledTransitions[resultTransitionId]) {
        if (!allowProviderEntry) throw new Error("native reviewer outcome is unavailable; provider replay is refused");
        const {execution, result, contractError, rejectedResult} = await executeReviewer(
          reviewer, reviewSkill, reviewerRequest, null);
        if (contractError) return Object.freeze({binding: null, builderContext: null,
          failure: resultContractFailure(contractError, rejectedResult, execution), execution});
        if (execution.failure || !result) {
          return Object.freeze({binding: null, builderContext: null,
            failure: execution.failure ?? Object.freeze({kind: "output", message: "native reviewer produced no result",
              providerEntry: "unknown", failureSignature: null, sessionAvailable: false}), execution});
        }
        const evidenceAdmissions = admitProductionPath({requiredClaims, execution, result, authority,
          operationPrefix, selection});
        const recorded = recordResult({reviewEpisode, authority, episode,
          transitionId: resultTransitionId, result, execution, evidenceAdmissions});
        if (recorded.failure) return Object.freeze({binding: null, builderContext: null,
          failure: recorded.failure, execution});
        episode = recorded.episode;
      }
      if (!episode.currentResult) throw new Error("native reviewer result transition has no durable result");
      if (episode.phase === "evidence_unestablished") {
        return Object.freeze({binding: binding({obligationId, episode, findings: []}),
          builderContext: null, failure: null});
      }
      const findings = findingBridge.publishFindings({authority: findingAuthority, operationPrefix, episode, result: episode.currentResult});
      const nativeBinding = binding({obligationId, episode, findings});
      const builderClaims = (episode.evidenceAdmissions ?? [])
        .filter(({boundary}) => boundary === "builder_projection");
      const builderContext = findings.length ? Object.freeze({...findingBridge.project({...contextRequest, findings}),
        requiredClaimEvidence: Object.freeze(structuredClone(builderClaims))})
        : builderClaims.length ? Object.freeze({schemaVersion: 1, findings: [],
          requiredClaimEvidence: Object.freeze(structuredClone(builderClaims))}) : null;
      return Object.freeze({binding: nativeBinding, builderContext, failure: null});
    },
    async executeCorrection({binding: current, obligationId, reviewSkill, authority,
      resultTransitionId, reviewerRequest, findingAuthority, operationPrefix, contextRequest,
      allowProviderEntry = true}) {
      const episode = reviewEpisode.recover(authority.identity);
      const remediationCorrection = episode?.phase === "re_evaluation"
        && episode.currentResult !== null;
      if (!episode || (episode.currentResult && !remediationCorrection)) {
        throw new Error("native review result correction requires an unresolved review episode");
      }
      if (!allowProviderEntry) throw new Error("native review result correction outcome is unavailable; provider replay is refused");
      const correctionRequest = remediationCorrection && reviewerRequest.resultCorrection
        ? Object.freeze({...reviewerRequest, resultCorrection: Object.freeze({
          ...reviewerRequest.resultCorrection,
          requiredPriorResult: episode.currentResult,
        })})
        : reviewerRequest;
      const {execution, result, contractError, rejectedResult} = await executeReviewer(
        reviewer, reviewSkill, correctionRequest, null);
      if (contractError) return Object.freeze({binding: null, builderContext: null,
        failure: resultContractFailure(contractError, rejectedResult, execution), execution});
      if (execution.failure || !result) {
        return Object.freeze({binding: null, builderContext: null,
          failure: execution.failure ?? Object.freeze({kind: "output",
            message: "native reviewer produced no corrected result", providerEntry: "unknown",
            failureSignature: null, sessionAvailable: false}), execution});
      }
      const recorded = recordResult({reviewEpisode, authority, episode,
        transitionId: resultTransitionId, result, execution});
      if (recorded.failure) return Object.freeze({binding: null, builderContext: null,
        failure: recorded.failure, execution});
      const findings = findingBridge.publishFindings({authority: findingAuthority,
        operationPrefix, episode: recorded.episode, result: recorded.episode.currentResult,
        ...(remediationCorrection ? {previousFindings: current.findings} : {})});
      const nativeBinding = binding({obligationId, episode: recorded.episode, findings, prior: current});
      const builderContext = findings.length ? findingBridge.project({...contextRequest, findings}) : null;
      return Object.freeze({binding: nativeBinding, builderContext, failure: null});
    },
    recoverCorrection({binding: current, obligationId, authority, resultTransitionId,
      recoveredResult, recoveredExecution, findingAuthority, operationPrefix, contextRequest}) {
      const episode = reviewEpisode.recover(authority.identity);
      const remediationRecovery = episode?.phase === "re_evaluation"
        && episode.currentResult !== null;
      if (!episode || (episode.currentResult && !remediationRecovery)) {
        throw new Error("native review corrected-result recovery requires an unresolved review episode");
      }
      const recorded = recordResult({reviewEpisode, authority, episode,
        transitionId: resultTransitionId, result: recoveredResult, execution: recoveredExecution});
      if (recorded.failure) return Object.freeze({binding: null, builderContext: null,
        failure: recorded.failure, execution: recoveredExecution});
      const findings = findingBridge.publishFindings({authority: findingAuthority,
        operationPrefix, episode: recorded.episode, result: recorded.episode.currentResult,
        ...(remediationRecovery ? {previousFindings: current.findings} : {})});
      const nativeBinding = binding({obligationId, episode: recorded.episode, findings, prior: current});
      const builderContext = findings.length ? findingBridge.project({...contextRequest, findings}) : null;
      return Object.freeze({binding: nativeBinding, builderContext, failure: null});
    },
    recordBuilderEvaluation({binding: current, authority, operationId, findingId, consumer,
      consumerRevision, decisionScope, disposition}) {
      if (!["valid", "invalid"].includes(disposition)) {
        throw new TypeError("native review finding disposition must be valid or invalid");
      }
      const finding = current.findings.find((item) => item.findingId === findingId);
      if (!finding) throw new Error("native review finding evaluation names an unknown finding");
      if (finding.builderEvaluation !== undefined) {
        throw new Error("native review finding already has a builder disposition");
      }
      const relied = finding.relianceRef === null
        ? findingBridge.recordReliance({authority, operationId, finding, consumer, consumerRevision, decisionScope})
        : finding;
      const evaluated = Object.freeze({...relied, builderEvaluation: Object.freeze({
        schemaVersion: 1, disposition, operationId, consumer, consumerRevision, decisionScope,
        relianceRef: relied.relianceRef,
      })});
      const findings = current.findings.map((item) => item.findingId === findingId ? evaluated : item);
      return withRelianceStatus(current, findings);
    },
    async executeRemediation({binding: current, reviewSkill, authority, subjectTransitionId, resultTransitionId, remediationSubject, reviewerRequest, findingAuthority, operationPrefix, contextRequest, allowProviderEntry = true}) {
      let episode = reviewEpisode.recover(authority.identity);
      if (!episode) throw new Error("native review episode binding is unavailable");
      if (episode.revision !== current.episodeRef.revision && !episode.handledTransitions[subjectTransitionId]) {
        throw new Error("native review episode binding is stale");
      }
      episode = reviewEpisode.transition({authority, expectedRevision: episode.revision,
        transitionId: subjectTransitionId, action: "record_remediation_subject", payload: {subject: remediationSubject}});
      if (!episode.handledTransitions[resultTransitionId]) {
        if (!allowProviderEntry) throw new Error("native remediation outcome is unavailable; provider replay is refused");
        const reviewerContext = current.findings.length ? findingBridge.project({
          ...reviewerContextRequest({contextRequest, authority, reviewerRequest}), findings: current.findings,
        }) : null;
        const {execution, result, contractError, rejectedResult} = await executeReviewer(
          reviewer, reviewSkill, reviewerRequest, reviewerContext);
        if (contractError) return Object.freeze({binding: null, builderContext: null,
          failure: resultContractFailure(contractError, rejectedResult, execution), execution});
        if (execution.failure || !result) {
          return Object.freeze({binding: null, builderContext: null,
            failure: execution.failure ?? Object.freeze({kind: "output", message: "native remediation reviewer produced no result",
              providerEntry: "unknown", failureSignature: null, sessionAvailable: false}), execution});
        }
        const recorded = recordResult({reviewEpisode, authority, episode,
          transitionId: resultTransitionId, result, execution});
        if (recorded.failure) return Object.freeze({binding: null, builderContext: null,
          failure: recorded.failure, execution});
        episode = recorded.episode;
      }
      if (!episode.currentResult) throw new Error("native remediation result transition has no durable result");
      const findings = findingBridge.publishFindings({authority: findingAuthority, operationPrefix, episode,
        result: episode.currentResult, previousFindings: current.findings});
      const nativeBinding = binding({obligationId: current.obligationId, episode, findings, prior: current});
      const builderContext = findings.length ? findingBridge.project({...contextRequest, findings}) : null;
      return Object.freeze({binding: nativeBinding, builderContext, failure: null});
    },
  });
}
