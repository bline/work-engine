import { SCHEMA_VERSION } from "./contract.mjs";
import { digest, validateTransportSafeJson } from "./identity.mjs";
import {
  buildProjection, discover, projectionContext, queryReliance, requireFreshProjection,
  resolveRecord, traverse,
} from "./projections.mjs";
import { exactFields, nonempty, requireCondition } from "./validation.mjs";

const OPERATIONS = new Set([
  "resolve", "discover", "project_relevant_revisions", "traverse_lineage",
  "query_direct_reliance", "query_reverse_reliance",
]);
const PAGE_LIMIT = 100;
const TRAVERSAL_LIMIT = 1_000;
const CURSOR_VERSION = 1;

function boundedInteger(value, maximum, label) {
  requireCondition(Number.isSafeInteger(value) && value >= 1 && value <= maximum, `${label} must be an integer from 1 through ${maximum}`);
  return value;
}

function encodeCursor(operation, projection, query, lastId) {
  return Buffer.from(JSON.stringify({
    version: CURSOR_VERSION,
    operation,
    projection_sha256: projection.canonical_input.sha256,
    query_sha256: digest(query),
    last_id: lastId,
  }), "utf8").toString("base64url");
}

function decodeCursor(cursor, operation, projection, query) {
  if (cursor === null) return null;
  requireCondition(typeof cursor === "string" && cursor.length > 0, "cursor must be null or a nonempty string");
  let decoded;
  try {
    decoded = JSON.parse(Buffer.from(cursor, "base64url").toString("utf8"));
  } catch {
    requireCondition(false, "cursor is malformed");
  }
  exactFields(decoded, ["version", "operation", "projection_sha256", "query_sha256", "last_id"], "cursor");
  requireCondition(decoded.version === CURSOR_VERSION && decoded.operation === operation, "cursor operation mismatch");
  requireCondition(decoded.projection_sha256 === projection.canonical_input.sha256, "cursor projection mismatch");
  requireCondition(decoded.query_sha256 === digest(query), "cursor query mismatch");
  nonempty(decoded.last_id, "cursor.last_id");
  return decoded.last_id;
}

function paginate(items, { operation, projection, query, limit, cursor, identity }) {
  boundedInteger(limit, PAGE_LIMIT, "page limit");
  const ordered = [...items].sort((left, right) => {
    const leftId = identity(left);
    const rightId = identity(right);
    return leftId < rightId ? -1 : (leftId > rightId ? 1 : 0);
  });
  const lastId = decodeCursor(cursor, operation, projection, query);
  let start = 0;
  if (lastId !== null) {
    const index = ordered.findIndex((item) => identity(item) === lastId);
    requireCondition(index !== -1, "cursor position is unavailable");
    start = index + 1;
  }
  const page = ordered.slice(start, start + limit);
  const truncated = start + page.length < ordered.length;
  return {
    items: page,
    page: {
      total_count: ordered.length,
      returned_count: page.length,
      truncated,
      next_cursor: truncated ? encodeCursor(operation, projection, query, identity(page.at(-1))) : null,
    },
  };
}

function execute(projection, operation, parameters) {
  if (operation === "resolve") {
    exactFields(parameters, ["identity"], "resolve parameters");
    nonempty(parameters.identity, "resolve identity");
    return resolveRecord(projection, parameters.identity);
  }
  if (operation === "discover") {
    exactFields(parameters, ["criteria", "limit", "cursor"], "discover parameters");
    const result = discover(projection, parameters.criteria);
    const bounded = paginate(result.candidates, {
      operation, projection, query: parameters.criteria, limit: parameters.limit,
      cursor: parameters.cursor, identity: (item) => item.claim.id,
    });
    return { applicability: result.applicability, criteria: result.criteria, candidates: bounded.items, page: bounded.page };
  }
  if (operation === "project_relevant_revisions") {
    exactFields(parameters, ["selections"], "relevant revision parameters");
    requireCondition(Array.isArray(parameters.selections) && parameters.selections.length > 0 && parameters.selections.length <= PAGE_LIMIT, "relevant revision selections must contain from 1 through 100 items");
    const revisionIds = new Set();
    const relevantExactRevisions = parameters.selections.map((selection) => {
      exactFields(selection, ["revision_id", "selection_reason"], "relevant revision selection");
      nonempty(selection.revision_id, "relevant revision selection revision_id");
      nonempty(selection.selection_reason, "relevant revision selection selection_reason");
      requireCondition(!revisionIds.has(selection.revision_id), "relevant revision selections must be unique");
      revisionIds.add(selection.revision_id);
      const revision = projection.revisions.find((item) => item.id === selection.revision_id);
      requireCondition(revision, "relevant revision not found");
      const claim = projection.claims.find((item) => item.id === revision.claim_id);
      const authority = projection.authorities.find((item) => item.grant_id === revision.authority_ref);
      requireCondition(claim && authority, "relevant revision has incomplete provenance");
      return {
        selection_reason: selection.selection_reason,
        claim: {
          id: claim.id, profile: claim.profile, subject: claim.subject,
          statement_identity: claim.statement_identity,
        },
        revision,
        authority_ref: revision.authority_ref,
        authority_reference: authority.authority_reference,
      };
    });
    return { relevant_exact_revisions: relevantExactRevisions, ...projectionContext(projection) };
  }
  if (operation === "traverse_lineage") {
    exactFields(parameters, ["revision_id", "direction", "max_revision_ids", "max_lineage_edges"], "traversal parameters");
    nonempty(parameters.revision_id, "traversal revision_id");
    boundedInteger(parameters.max_revision_ids, TRAVERSAL_LIMIT, "max_revision_ids");
    boundedInteger(parameters.max_lineage_edges, TRAVERSAL_LIMIT, "max_lineage_edges");
    const result = traverse(projection, parameters.revision_id, parameters.direction);
    const revisionIds = result.revision_ids.slice(0, parameters.max_revision_ids);
    const lineage = result.lineage.slice(0, parameters.max_lineage_edges);
    return {
      revision_id: result.revision_id,
      direction: result.direction,
      revision_ids: revisionIds,
      lineage,
      bounds: {
        revision_ids: {
          total_count: result.revision_ids.length,
          returned_count: revisionIds.length,
          truncated: revisionIds.length < result.revision_ids.length,
        },
        lineage_edges: {
          total_count: result.lineage.length,
          returned_count: lineage.length,
          truncated: lineage.length < result.lineage.length,
        },
      },
    };
  }
  const direct = operation === "query_direct_reliance";
  exactFields(parameters, [direct ? "revision_id" : "consumer", "limit", "cursor"], "reliance parameters");
  const selector = direct ? parameters.revision_id : parameters.consumer;
  nonempty(selector, direct ? "reliance revision_id" : "reliance consumer");
  const query = direct ? { revision_id: selector } : { consumer: selector };
  const result = queryReliance(projection, direct ? selector : null, direct ? null : selector);
  const bounded = paginate(result.reliances, {
    operation, projection, query, limit: parameters.limit, cursor: parameters.cursor,
    identity: (item) => item.id,
  });
  return { query: result.query, reliances: bounded.items, page: bounded.page };
}

function receipt(request, outcome, projection, result, refusal) {
  return {
    schema_version: SCHEMA_VERSION,
    request_id: request?.request_id ?? null,
    operation: request?.operation ?? null,
    outcome,
    projection: projection ? projectionContext(projection) : null,
    result,
    refusal,
  };
}

export function readClaimEvidence(store, request) {
  let projection = null;
  let requestAccepted = false;
  try {
    validateTransportSafeJson(request, "read request");
    exactFields(request, ["schema_version", "request_id", "operation", "parameters"], "read request");
    requireCondition(request.schema_version === SCHEMA_VERSION, "unsupported read request version");
    nonempty(request.request_id, "read request_id");
    requireCondition(OPERATIONS.has(request.operation), "unknown read operation");
    requestAccepted = true;
    const canonicalSnapshot = store.exportStore();
    projection = buildProjection(canonicalSnapshot);
    requireFreshProjection(canonicalSnapshot, projection);
    requireCondition(projection.completeness !== "unavailable", "projection completeness is unavailable");
    const result = execute(projection, request.operation, request.parameters);
    return receipt(request, "succeeded", projection, result, null);
  } catch (error) {
    return receipt(request, "refused", projection, null, {
      code: !requestAccepted || projection ? "request_refused" : "projection_unavailable",
      message: error instanceof Error ? error.message : "claim-evidence read failed",
    });
  }
}
