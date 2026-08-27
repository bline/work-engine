import { BUILD_VERSION, ClaimEvidenceError, SCHEMA_VERSION } from "./contract.mjs";
import { digest } from "./identity.mjs";
import { requireCondition, validateStore } from "./validation.mjs";
import { revisionHeads } from "./service.mjs";

export function projectionContext(projection) {
  return {
    projection_schema_version: projection.projection_schema_version,
    build_version: projection.build_version,
    projection_identity: projection.canonical_input,
    freshness: projection.freshness,
    completeness: projection.completeness,
    actual_content_set: projection.actual_content_set,
    excluded_inputs: projection.excluded_inputs,
    failed_inputs: projection.failed_inputs,
    unresolved_references: projection.unresolved_references,
  };
}

export function buildProjection(store, { buildVersion = BUILD_VERSION } = {}) {
  validateStore(store);
  const boundary = store.projection_boundary;
  const references = [
    ...store.authorities.map((item) => item.authority_reference),
    ...store.claims.map((item) => item.subject.evidence_baseline),
    ...store.revisions.flatMap((item) => [...item.evidence_references, ...item.sensitivity_references]),
  ];
  const unresolved = references.filter((item) => item.status !== "verified").map(({ owner, reference, revision, status }) => ({ owner, reference, revision, status }))
    .sort((left, right) => ["owner", "reference", "revision", "status"].map((key) => left[key].localeCompare(right[key])).find((value) => value !== 0) ?? 0);
  const claims = store.claims.map((claim) => {
    const revisions = store.revisions.filter((item) => item.claim_id === claim.id);
    const heads = [...revisionHeads(store, claim.id)].sort();
    return { ...claim, revision_ids: revisions.map((item) => item.id).sort(), head_revision_ids: heads, branch_state: heads.length > 1 ? "conflict_or_branch" : "single_head" };
  });
  return {
    projection_schema_version: SCHEMA_VERSION, build_version: buildVersion,
    canonical_input: { path: "canonical/store.json", sha256: digest(store), source_watermark: boundary.source_watermark },
    freshness: boundary.freshness, completeness: boundary.completeness,
    actual_content_set: boundary.actual_content_set, excluded_inputs: structuredClone(boundary.excluded_inputs),
    failed_inputs: structuredClone(boundary.failed_inputs), unresolved_references: unresolved,
    authorities: structuredClone(store.authorities), claims, revisions: structuredClone(store.revisions),
    lineage: structuredClone(store.lineage), reliances: structuredClone(store.reliances),
  };
}

export function requireFreshProjection(store, projection) {
  requireCondition(projection?.canonical_input?.sha256 === digest(store), "projection is stale for the canonical store");
}

export function discover(projection, criteria) {
  const allowed = new Set(["namespace", "subject_kind", "stable_subject_id", "profile", "producer", "support_qualification", "sensitivity_reference", "evidence_baseline", "content_reference", "consumer"]);
  requireCondition(criteria && typeof criteria === "object" && Object.keys(criteria).length > 0 && Object.keys(criteria).every((key) => allowed.has(key)), "discovery criteria are empty or unknown");
  requireCondition(projection.completeness !== "unavailable", "projection completeness is unavailable");
  const revisions = new Map(projection.revisions.map((item) => [item.id, item]));
  const candidates = projection.claims.filter((claim) => {
    const subject = claim.subject;
    if (["namespace", "subject_kind", "stable_subject_id"].some((key) => key in criteria && criteria[key] !== subject[key])) return false;
    if ("profile" in criteria && criteria.profile !== claim.profile) return false;
    if ("evidence_baseline" in criteria && criteria.evidence_baseline !== subject.evidence_baseline.reference) return false;
    if ("content_reference" in criteria && !subject.content_set.includes(criteria.content_reference)) return false;
    const claimRevisions = claim.revision_ids.map((item) => revisions.get(item));
    if ("producer" in criteria && !claimRevisions.some((item) => item.producer === criteria.producer)) return false;
    if ("support_qualification" in criteria && !claimRevisions.some((item) => item.support_qualification === criteria.support_qualification)) return false;
    if ("sensitivity_reference" in criteria && !claimRevisions.some((item) => item.sensitivity_references.some((reference) => reference.reference === criteria.sensitivity_reference))) return false;
    if ("consumer" in criteria && !projection.reliances.some((item) => item.consumer === criteria.consumer && claim.revision_ids.includes(item.claim_revision_id))) return false;
    return true;
  }).map((claim) => ({ claim, revisions: claim.revision_ids.map((item) => revisions.get(item)) }));
  return { applicability: "not_assessed", criteria, candidates, ...projectionContext(projection) };
}

export function resolveRecord(projection, identity) {
  const claim = projection.claims.find((item) => item.id === identity);
  const revision = projection.revisions.find((item) => item.id === identity);
  requireCondition(Boolean(claim) !== Boolean(revision), "claim or revision not found");
  if (claim) return { kind: "claim", claim, authority: projection.authorities.find((item) => item.grant_id === claim.authority_ref), revisions: projection.revisions.filter((item) => item.claim_id === identity), ...projectionContext(projection) };
  return { kind: "revision", revision, authority: projection.authorities.find((item) => item.grant_id === revision.authority_ref), direct_reliance: projection.reliances.filter((item) => item.claim_revision_id === identity), lineage: projection.lineage.filter((item) => item.target === identity || item.sources.includes(identity)), ...projectionContext(projection) };
}

export function traverse(projection, revisionId, direction) {
  requireCondition(["predecessors", "successors", "both"].includes(direction), "unknown traversal direction");
  requireCondition(projection.revisions.some((item) => item.id === revisionId), "revision not found");
  const implicit = projection.revisions.filter((item) => item.predecessor_revision !== null).map((item) => ({ id: `revision-predecessor@${item.id}`, relationship: "revision_predecessor", sources: [item.predecessor_revision], target: item.id, derived: true }));
  const relationships = [...projection.lineage, ...implicit];
  const seen = new Set([revisionId]); let frontier = new Set([revisionId]); const edges = new Map();
  while (frontier.size > 0) {
    const next = new Set();
    for (const edge of relationships) {
      const predecessors = new Set(edge.sources); const successors = new Set([edge.target]);
      const fromPredecessor = [...frontier].some((item) => predecessors.has(item));
      const fromSuccessor = frontier.has(edge.target);
      if ((["successors", "both"].includes(direction) && fromPredecessor) || (["predecessors", "both"].includes(direction) && fromSuccessor)) {
        edges.set(edge.id, edge);
        if (fromPredecessor) for (const item of successors) if (!seen.has(item)) next.add(item);
        if (fromSuccessor) for (const item of predecessors) if (!seen.has(item)) next.add(item);
      }
    }
    for (const item of next) seen.add(item); frontier = next;
  }
  return { revision_id: revisionId, direction, revision_ids: [...seen].sort(), lineage: [...edges.values()].sort((a, b) => a.id.localeCompare(b.id)), ...projectionContext(projection) };
}

export function queryReliance(projection, revisionId, consumer) {
  requireCondition(Boolean(revisionId) !== Boolean(consumer), "supply exactly one reliance query key");
  return { query: { revision_id: revisionId, consumer }, reliances: projection.reliances.filter((item) => (revisionId && item.claim_revision_id === revisionId) || (consumer && item.consumer === consumer)), ...projectionContext(projection) };
}

