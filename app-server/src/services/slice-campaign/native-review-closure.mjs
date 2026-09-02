import { digest as episodeDigest } from "../review-episode/contract.mjs";
import { ImplementationReviewError } from "../implementation-review/contract.mjs";
import { AgentInstructionReviewError } from "../agent-instruction-review/contract.mjs";

function reference(owner, reference, revision, value) {
  return Object.freeze({owner, reference, revision, sha256: episodeDigest(value), freshness: "exact immutable revision"});
}
function episodeRef(state) {
  return reference("review-episode", `review-episode@${episodeDigest(state.identity)}`, state.revision, state);
}
function status(episode) { return episode.phase === "reported" ? "reported" : "awaiting_builder"; }
function binding({obligationId, episode, findings, prior = null}) {
  return Object.freeze({
    schemaVersion: 1, obligationId, status: status(episode),
    episodeRef: episodeRef(episode), runtimeSessionRef: episode.writer.runtimeSession, findings,
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
      && !(error instanceof AgentInstructionReviewError)) throw error;
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

function recordResult({reviewEpisode, authority, episode, transitionId, result, execution}) {
  try {
    return Object.freeze({episode: reviewEpisode.transition({authority,
      expectedRevision: episode.revision, transitionId, action: "record_result",
      payload: {result, unresolvedQuestions: []}}), failure: null});
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

export function createNativeReviewClosureService({reviewEpisode, reviewer, findingBridge} = {}) {
  if (!reviewEpisode?.begin || !reviewEpisode?.transition || !reviewEpisode?.recover) throw new TypeError("native review closure requires Review Episode");
  if (!reviewer?.review) throw new TypeError("native review closure requires canonical reviewer runtime");
  if (!findingBridge?.publishFindings || !findingBridge?.recordReliance || !findingBridge?.project) throw new TypeError("native review closure requires review finding bridge");
  return Object.freeze({
    async executeInitial({obligationId, reviewSkill, authority, beginTransitionId, resultTransitionId, reviewerRequest, findingAuthority, operationPrefix, contextRequest, allowProviderEntry = true}) {
      let episode = reviewEpisode.begin({authority, transitionId: beginTransitionId});
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
        const recorded = recordResult({reviewEpisode, authority, episode,
          transitionId: resultTransitionId, result, execution});
        if (recorded.failure) return Object.freeze({binding: null, builderContext: null,
          failure: recorded.failure, execution});
        episode = recorded.episode;
      }
      if (!episode.currentResult) throw new Error("native reviewer result transition has no durable result");
      const findings = findingBridge.publishFindings({authority: findingAuthority, operationPrefix, episode, result: episode.currentResult});
      const nativeBinding = binding({obligationId, episode, findings});
      const builderContext = findings.length ? findingBridge.project({...contextRequest, findings}) : null;
      return Object.freeze({binding: nativeBinding, builderContext, failure: null});
    },
    async executeCorrection({binding: current, obligationId, reviewSkill, authority,
      resultTransitionId, reviewerRequest, findingAuthority, operationPrefix, contextRequest,
      allowProviderEntry = true}) {
      const episode = reviewEpisode.recover(authority.identity);
      if (!episode || episode.currentResult) {
        throw new Error("native review result correction requires an unresolved review episode");
      }
      if (!allowProviderEntry) throw new Error("native review result correction outcome is unavailable; provider replay is refused");
      const {execution, result, contractError, rejectedResult} = await executeReviewer(
        reviewer, reviewSkill, reviewerRequest, null);
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
        operationPrefix, episode: recorded.episode, result: recorded.episode.currentResult});
      const nativeBinding = binding({obligationId, episode: recorded.episode, findings, prior: current});
      const builderContext = findings.length ? findingBridge.project({...contextRequest, findings}) : null;
      return Object.freeze({binding: nativeBinding, builderContext, failure: null});
    },
    recoverCorrection({binding: current, obligationId, authority, resultTransitionId,
      recoveredResult, recoveredExecution, findingAuthority, operationPrefix, contextRequest}) {
      const episode = reviewEpisode.recover(authority.identity);
      if (!episode || episode.currentResult) {
        throw new Error("native review corrected-result recovery requires an unresolved review episode");
      }
      const recorded = recordResult({reviewEpisode, authority, episode,
        transitionId: resultTransitionId, result: recoveredResult, execution: recoveredExecution});
      if (recorded.failure) return Object.freeze({binding: null, builderContext: null,
        failure: recorded.failure, execution: recoveredExecution});
      const findings = findingBridge.publishFindings({authority: findingAuthority,
        operationPrefix, episode: recorded.episode, result: recorded.episode.currentResult});
      const nativeBinding = binding({obligationId, episode: recorded.episode, findings, prior: current});
      const builderContext = findings.length ? findingBridge.project({...contextRequest, findings}) : null;
      return Object.freeze({binding: nativeBinding, builderContext, failure: null});
    },
    recordBuilderEvaluation({binding: current, authority, operationId, findingId, consumer, consumerRevision, decisionScope}) {
      const finding = current.findings.find((item) => item.findingId === findingId);
      if (!finding) throw new Error("native review finding evaluation names an unknown finding");
      if (finding.relianceRef !== null) throw new Error("native review finding already has builder reliance");
      const evaluated = findingBridge.recordReliance({authority, operationId, finding, consumer, consumerRevision, decisionScope});
      const findings = current.findings.map((item) => item.findingId === findingId ? evaluated : item);
      return Object.freeze({...current, findings});
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
