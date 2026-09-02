import { digest as episodeDigest } from "../review-episode/contract.mjs";

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
    return Object.freeze({execution, result: execution.specialistReview?.implementationReviewResult ?? null});
  }
  if (!["implementation-review", "claude-recon-implementation"].includes(reviewSkill)) {
    throw new Error("native review obligation names an unsupported selected skill");
  }
  const execution = await reviewer.review({...reviewerRequest, claimContext});
  return Object.freeze({execution, result: execution.result ?? null});
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
        const {execution, result} = await executeReviewer(reviewer, reviewSkill, reviewerRequest, null);
        if (execution.failure || !result) {
          return Object.freeze({binding: null, builderContext: null,
            failure: execution.failure ?? Object.freeze({kind: "output", message: "native reviewer produced no result",
              providerEntry: "unknown", failureSignature: null, sessionAvailable: false}), execution});
        }
        episode = reviewEpisode.transition({authority, expectedRevision: episode.revision,
          transitionId: resultTransitionId, action: "record_result", payload: {result, unresolvedQuestions: []}});
      }
      if (!episode.currentResult) throw new Error("native reviewer result transition has no durable result");
      const findings = findingBridge.publishFindings({authority: findingAuthority, operationPrefix, episode, result: episode.currentResult});
      const nativeBinding = binding({obligationId, episode, findings});
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
        const {execution, result} = await executeReviewer(reviewer, reviewSkill, reviewerRequest, reviewerContext);
        if (execution.failure || !result) {
          return Object.freeze({binding: null, builderContext: null,
            failure: execution.failure ?? Object.freeze({kind: "output", message: "native remediation reviewer produced no result",
              providerEntry: "unknown", failureSignature: null, sessionAvailable: false}), execution});
        }
        episode = reviewEpisode.transition({authority, expectedRevision: episode.revision,
          transitionId: resultTransitionId, action: "record_result", payload: {result, unresolvedQuestions: []}});
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
