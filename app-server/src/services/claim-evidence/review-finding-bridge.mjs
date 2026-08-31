import { digest, stableClaimId } from "./identity.mjs";
import { projectReviewerClaimContext } from "./reviewer-projection.mjs";

const PROFILE = "revision-bound-review-finding-v1";

function text(value, label) {
  if (typeof value !== "string" || !value.trim()) throw new TypeError(`${label} must be non-empty text`);
  return value;
}

function episodeKey(identity) { return `review-episode@${digest(identity)}`; }
function campaignReference(owner, reference, revision, value) {
  return Object.freeze({owner, reference, revision, sha256: digest(value), freshness: "exact immutable revision"});
}
function sourceReference(episode) {
  return Object.freeze({
    owner: "review-episode", reference: episodeKey(episode.identity), revision: episode.revision,
    integrity_sha256: digest(episode), freshness: "exact immutable revision", status: "verified",
  });
}
function revisionPayload({finding, result, episode, source}) {
  return {
    proposition: finding.consequence,
    support_qualification: "attributed_review_finding",
    assumptions: [],
    limitations: [...result.limitations],
    confidence: {label: finding.confidence},
    evidence_references: [source], sensitivity_references: [],
    evidence_mode: "review_episode_result", judgment_kind: "review_finding",
    decision_scope: null,
    profile_payload: {
      finding_id: finding.id, severity: finding.severity,
      episode: episodeKey(episode.identity), outcome: finding.status,
    },
    reopening_conditions: finding.status === "verified_resolved" ? ["new subject revision or contradictory evidence"] : [],
    tombstone: false,
  };
}

export function createReviewFindingBridge({store} = {}) {
  if (!store || typeof store.publish !== "function" || typeof store.exportStore !== "function") {
    throw new TypeError("review finding bridge requires the canonical claim-evidence store");
  }
  const publishFindings = ({authority, operationPrefix, episode, result, previousFindings = []}) => {
    text(operationPrefix, "review finding operationPrefix");
    if (!Array.isArray(result?.findings)) throw new TypeError("review finding bridge requires an admitted review result");
    const previous = new Map(previousFindings.map((item) => [item.findingId, item]));
    const source = sourceReference(episode);
    return result.findings.map((finding) => {
      const prior = previous.get(finding.id) ?? null;
      const subject = {
        namespace: "implementation-review", subject_kind: "revision-bound-finding",
        stable_subject_id: `${episodeKey(episode.identity)}:${finding.id}`,
        evidence_baseline: source, content_set: [source.reference],
      };
      const claimId = stableClaimId(subject);
      const revision = revisionPayload({finding, result, episode, source});
      revision.decision_scope = authority.decision_scope;
      const operation = prior === null ? {
        schema_version: 1, operation_id: `${operationPrefix}:${finding.id}`, action: "create_claim",
        profile: PROFILE, expected_state: null,
        payload: {subject, statement_identity: finding.title, initial_revision: revision},
      } : {
        schema_version: 1, operation_id: `${operationPrefix}:${finding.id}`, action: "publish_revision",
        profile: PROFILE, expected_state: prior.revisionRef.revision,
        payload: {claim_id: claimId, revision},
      };
      const published = store.publish(operation, authority);
      const record = published.store.revisions.find(({id}) => id === published.result_identity);
      if (!record) throw new TypeError("review finding publication did not produce an exact revision");
      const revisionRef = campaignReference("claim-evidence", claimId, record.id, record);
      return Object.freeze({
        findingId: finding.id, outcome: finding.status,
        initialRevisionRef: prior?.initialRevisionRef ?? revisionRef,
        revisionRef,
        relianceRef: prior?.revisionRef.revision === revisionRef.revision ? prior.relianceRef : null,
      });
    });
  };
  const recordReliance = ({authority, operationId, finding, consumer, consumerRevision, decisionScope}) => {
    for (const [value, label] of [[operationId, "operationId"], [consumer, "consumer"], [consumerRevision, "consumerRevision"], [decisionScope, "decisionScope"]]) text(value, `review finding reliance ${label}`);
    if (!finding?.revisionRef) throw new TypeError("review finding reliance requires an exact finding revision");
    const published = store.publish({
      schema_version: 1, operation_id: operationId, action: "record_reliance", profile: PROFILE,
      expected_state: null,
      payload: {consumer, consumer_revision: consumerRevision, decision_scope: decisionScope,
        claim_revision_id: finding.revisionRef.revision, state: "active", predecessor_reliance: null},
    }, authority);
    const reliance = published.store.reliances.find(({id}) => id === published.result_identity);
    return Object.freeze({...finding, relianceRef: campaignReference("claim-evidence", reliance.id, reliance.id, reliance)});
  };
  const project = ({requestId, consumer, limitations, findings}) => projectReviewerClaimContext(store, {
    schema_version: 1, request_id: requestId, consumer, limitations,
    selections: findings.map(({findingId, revisionRef}) => ({
      revision_id: revisionRef.revision,
      selection_reason: `Exact revision for review finding ${findingId}; applicability and reliance remain consumer decisions.`,
    })),
  });
  return Object.freeze({publishFindings, recordReliance, project});
}
