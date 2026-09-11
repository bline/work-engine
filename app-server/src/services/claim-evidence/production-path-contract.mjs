import { digest } from "./identity.mjs";

export const PRODUCTION_PATH_PROFILE = "production-path-v1";
export const PRODUCTION_PATH_PROFILE_REVISION = "production-path-profile-v1";
export const PRODUCTION_PATH_OBSERVATION_SCHEMA_VERSION = 2;
export const PRODUCTION_PATH_ESTABLISHMENT_SCHEMA_VERSION = 1;
export const PRODUCTION_PATH_SUCCESSION_SCHEMA_VERSION = 1;
export const PRODUCTION_PATH_BOUNDARIES = new Set(["builder_projection", "campaign_terminalization"]);
export const PRODUCTION_PATH_STATUSES = new Set(["established", "false", "unestablished"]);

export class ProductionPathEvidenceError extends TypeError {}

const record = (value, label) => {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new ProductionPathEvidenceError(`${label} must be an object`);
  return value;
};
const text = (value, label) => {
  if (typeof value !== "string" || !value.trim()) throw new ProductionPathEvidenceError(`${label} must be non-empty text`);
  return value;
};
const sha256 = (value, label) => {
  if (typeof value !== "string" || !/^[0-9a-f]{64}$/.test(value)) throw new ProductionPathEvidenceError(`${label} must be lowercase SHA-256`);
  return value;
};
const exact = (value, fields, label) => {
  record(value, label);
  const expected = [...fields].sort();
  if (JSON.stringify(Object.keys(value).sort()) !== JSON.stringify(expected)) {
    throw new ProductionPathEvidenceError(`${label} fields are invalid`);
  }
};
const stringList = (value, label, {nonempty = false} = {}) => {
  if (!Array.isArray(value) || (nonempty && value.length === 0)
      || value.some((item) => typeof item !== "string" || !item.trim())
      || new Set(value).size !== value.length) throw new ProductionPathEvidenceError(`${label} must be a unique string array`);
};

export const productionPathClaimId = (claim) => `production-path-claim-v1@${digest({
  proposition: claim.proposition, subject: claim.subject, coveredState: claim.coveredState,
  consumptionBoundary: claim.consumptionBoundary, consumer: claim.consumer,
})}`;
export const productionPathClaimRevision = (claim) => {
  const value = structuredClone(claim); delete value.revision;
  return `production-path-claim-revision-v1@${digest(value)}`;
};

export function validateProductionPathClaimRevision(value, expectedSubject = null) {
  exact(value, ["schemaVersion", "claimId", "revision", "proposition", "subject", "coveredState",
    "consumptionBoundary", "consumer", "acceptance", "profile"], "production-path claim revision");
  if (value.schemaVersion !== 1) throw new ProductionPathEvidenceError("production-path claim schema version is invalid");
  text(value.proposition, "production-path claim proposition");
  exact(value.subject, ["candidate", "reviewEpisodeId"], "production-path claim subject");
  exact(value.subject.candidate, ["commit", "tree", "patchIdentity"], "production-path claim candidate");
  Object.entries(value.subject.candidate).forEach(([key, item]) => text(item, `production-path claim candidate.${key}`));
  text(value.subject.reviewEpisodeId, "production-path claim reviewEpisodeId");
  text(value.coveredState, "production-path claim coveredState");
  if (!PRODUCTION_PATH_BOUNDARIES.has(value.consumptionBoundary)) throw new ProductionPathEvidenceError("production-path claim boundary is invalid");
  text(value.consumer, "production-path claim consumer");
  exact(value.acceptance, ["owner", "source", "unestablishedRoute"], "production-path claim acceptance");
  Object.entries(value.acceptance).forEach(([key, item]) => text(item, `production-path claim acceptance.${key}`));
  if (["reviewer", "builder", "adapter", "terminalizer"].includes(value.acceptance.owner)) {
    throw new ProductionPathEvidenceError("production-path claim is self-authorized");
  }
  exact(value.profile, ["id", "revision", "allowedMechanisms", "admissibleObservers", "integrityRequired",
    "requiredRealization", "requiredCapabilities", "continuity"], "production-path evidence profile");
  if (value.profile.id !== PRODUCTION_PATH_PROFILE || value.profile.revision !== PRODUCTION_PATH_PROFILE_REVISION) {
    throw new ProductionPathEvidenceError("production-path evidence profile is unsupported");
  }
  stringList(value.profile.allowedMechanisms, "production-path allowed mechanisms", {nonempty: true});
  stringList(value.profile.admissibleObservers, "production-path admissible observers", {nonempty: true});
  stringList(value.profile.requiredCapabilities, "production-path required capabilities");
  text(value.profile.requiredRealization, "production-path required realization");
  if (typeof value.profile.integrityRequired !== "boolean") throw new ProductionPathEvidenceError("production-path integrity requirement is invalid");
  if (!['fresh_initial', 'retained'].includes(value.profile.continuity)) throw new ProductionPathEvidenceError("production-path continuity requirement is invalid");
  if (value.claimId !== productionPathClaimId(value) || value.revision !== productionPathClaimRevision(value)) {
    throw new ProductionPathEvidenceError("production-path claim identity is invalid");
  }
  if (expectedSubject !== null && digest(value.subject.candidate) !== digest(expectedSubject)) {
    throw new ProductionPathEvidenceError("production-path claim subject differs from selection subject");
  }
  const expectedConsumer = value.consumptionBoundary === "builder_projection" ? "slice-builder" : "slice-campaign";
  if (!value.consumer.startsWith(`${expectedConsumer}:`)) throw new ProductionPathEvidenceError("production-path claim consumer does not own its boundary");
  return value;
}

export function makeProductionPathClaimRevision(input) {
  const claim = {schemaVersion: 1, ...structuredClone(input), claimId: "", revision: ""};
  claim.claimId = productionPathClaimId(claim);
  claim.revision = productionPathClaimRevision(claim);
  validateProductionPathClaimRevision(claim);
  return Object.freeze(claim);
}

export function validateProductionPathObservation(value) {
  exact(value, ["schema_version", "id", "event_identity", "kind", "selection", "obligationId", "subject",
    "coveredState", "execution", "realization", "capabilityEnvelope", "continuity", "transport",
    "observer", "observedAt", "adapterVersion", "artifacts"], "production-path observation");
  if (value.schema_version !== PRODUCTION_PATH_OBSERVATION_SCHEMA_VERSION || value.kind !== "production_path") {
    throw new ProductionPathEvidenceError("production-path observation version or kind is invalid");
  }
  exact(value.selection, ["id", "revision"], "production-path observation selection");
  text(value.selection.id, "production-path observation selection.id"); sha256(value.selection.revision, "production-path observation selection.revision");
  text(value.obligationId, "production-path observation obligationId");
  exact(value.subject, ["candidate", "reviewEpisodeId"], "production-path observation subject");
  exact(value.subject.candidate, ["commit", "tree", "patchIdentity"], "production-path observation candidate");
  Object.values(value.subject.candidate).forEach((item) => text(item, "production-path observation candidate identity"));
  text(value.subject.reviewEpisodeId, "production-path observation reviewEpisodeId");
  text(value.coveredState, "production-path observation coveredState");
  exact(value.execution, ["attemptId", "resultDigest"], "production-path observation execution");
  text(value.execution.attemptId, "production-path observation attemptId"); sha256(value.execution.resultDigest, "production-path observation resultDigest");
  exact(value.realization, ["requested", "observed"], "production-path observation realization");
  text(value.realization.requested, "production-path observation requested realization"); text(value.realization.observed, "production-path observation observed realization");
  exact(value.capabilityEnvelope, ["capabilities", "mutationAuthorized"], "production-path capability envelope");
  stringList(value.capabilityEnvelope.capabilities, "production-path capabilities");
  if (typeof value.capabilityEnvelope.mutationAuthorized !== "boolean") throw new ProductionPathEvidenceError("production-path mutation authorization is invalid");
  exact(value.continuity, ["mode", "sessionId"], "production-path observation continuity");
  if (!['fresh_initial', 'same_session_resume'].includes(value.continuity.mode)) throw new ProductionPathEvidenceError("production-path observation continuity is invalid");
  text(value.continuity.sessionId, "production-path observation sessionId");
  exact(value.transport, ["mechanism", "digest"], "production-path observation transport");
  text(value.transport.mechanism, "production-path observation mechanism"); sha256(value.transport.digest, "production-path observation transport digest");
  exact(value.observer, ["identity", "kind"], "production-path observation observer");
  text(value.observer.identity, "production-path observation observer identity"); text(value.observer.kind, "production-path observation observer kind");
  text(value.observedAt, "production-path observation observedAt");
  if (!Number.isFinite(Date.parse(value.observedAt)) || new Date(value.observedAt).toISOString() !== value.observedAt) throw new ProductionPathEvidenceError("production-path observation time is invalid");
  text(value.adapterVersion, "production-path observation adapterVersion");
  if (!Array.isArray(value.artifacts) || value.artifacts.some((item) => {
    try { exact(item, ["owner", "reference", "digest", "status"], "production-path artifact"); text(item.owner, "artifact owner"); text(item.reference, "artifact reference"); if (!['verified', 'unavailable'].includes(item.status)) return true; if (item.status === 'verified') sha256(item.digest, "artifact digest"); else if (item.digest !== null) return true; return false; } catch { return true; }
  })) throw new ProductionPathEvidenceError("production-path artifacts are invalid");
  const withoutId = structuredClone(value); delete withoutId.id;
  if (value.id !== `production-path-observation-v1@${digest(withoutId)}`) throw new ProductionPathEvidenceError("production-path observation identity is invalid");
  return value;
}

export function normalizeProductionPathObservation(input) {
  const value = {schema_version: PRODUCTION_PATH_OBSERVATION_SCHEMA_VERSION, kind: "production_path", ...structuredClone(input)};
  value.id = `production-path-observation-v1@${digest(value)}`;
  validateProductionPathObservation(value);
  return Object.freeze(value);
}

export function validateProductionPathEstablishment(value) {
  exact(value, ["schemaVersion", "id", "operationId", "claimId", "claimRevision", "observationId",
    "observationDigest", "evaluator", "profileRevision", "status", "reasons", "predecessor"], "production-path establishment");
  if (value.schemaVersion !== PRODUCTION_PATH_ESTABLISHMENT_SCHEMA_VERSION || !PRODUCTION_PATH_STATUSES.has(value.status)) throw new ProductionPathEvidenceError("production-path establishment version or status is invalid");
  for (const field of ["operationId", "claimId", "claimRevision", "observationId", "evaluator", "profileRevision"]) text(value[field], `production-path establishment.${field}`);
  sha256(value.observationDigest, "production-path establishment observationDigest");
  stringList(value.reasons, "production-path establishment reasons");
  if (value.predecessor !== null) text(value.predecessor, "production-path establishment predecessor");
  const withoutId = structuredClone(value); delete withoutId.id;
  if (value.id !== `production-path-establishment-v1@${digest(withoutId)}`) throw new ProductionPathEvidenceError("production-path establishment identity is invalid");
  return value;
}

export function validateProductionPathCorrectionAuthority(value) {
  exact(value, ["schemaVersion", "owner", "source", "sequence", "campaign", "acceptanceOwner"],
    "production-path correction authority");
  if (value.schemaVersion !== 1 || value.owner !== "slice-supervisor") {
    throw new ProductionPathEvidenceError("production-path correction authority owner is invalid");
  }
  text(value.source, "production-path correction authority source");
  if (!Number.isSafeInteger(value.sequence) || value.sequence < 1) {
    throw new ProductionPathEvidenceError("production-path correction authority sequence is invalid");
  }
  text(value.campaign, "production-path correction authority campaign");
  text(value.acceptanceOwner, "production-path correction acceptance owner");
  if (value.acceptanceOwner !== "operator") {
    throw new ProductionPathEvidenceError("production-path correction must preserve operator acceptance ownership");
  }
  return value;
}

export function validateProductionPathSuccession(value) {
  exact(value, ["schemaVersion", "id", "operationId", "authorityDigest", "campaignRevision",
    "selection", "claims", "observationId", "observationDigest", "reviewEpisode",
    "candidate", "consequences"], "production-path succession");
  if (value.schemaVersion !== PRODUCTION_PATH_SUCCESSION_SCHEMA_VERSION) {
    throw new ProductionPathEvidenceError("production-path succession schema version is invalid");
  }
  for (const field of ["operationId", "campaignRevision", "observationId"]) {
    text(value[field], `production-path succession.${field}`);
  }
  sha256(value.authorityDigest, "production-path succession authorityDigest");
  sha256(value.observationDigest, "production-path succession observationDigest");
  exact(value.selection, ["id", "predecessorRevision", "successorRevision"], "production-path succession selection");
  text(value.selection.id, "production-path succession selection.id");
  sha256(value.selection.predecessorRevision, "production-path succession selection.predecessorRevision");
  sha256(value.selection.successorRevision, "production-path succession selection.successorRevision");
  exact(value.claims, ["builder", "terminal"], "production-path succession claims");
  for (const [boundary, item] of Object.entries(value.claims)) {
    exact(item, ["claimId", "predecessorRevision", "successorRevision", "predecessorEstablishment",
      "successorEstablishment"], `production-path succession ${boundary} claim`);
    text(item.claimId, `production-path succession ${boundary} claimId`);
    for (const field of ["predecessorRevision", "successorRevision", "predecessorEstablishment",
      "successorEstablishment"]) text(item[field], `production-path succession ${boundary}.${field}`);
  }
  exact(value.reviewEpisode, ["id", "predecessorRevision", "successorRevision"], "production-path succession review episode");
  Object.values(value.reviewEpisode).forEach((item) => text(item, "production-path succession review episode value"));
  exact(value.candidate, ["commit", "tree", "patchIdentity"], "production-path succession candidate");
  Object.values(value.candidate).forEach((item) => text(item, "production-path succession candidate value"));
  exact(value.consequences, ["builder", "terminal", "reviewAccepted", "campaignAccepted"], "production-path succession consequences");
  if (value.consequences.builder !== "projection_enabled"
      || value.consequences.terminal !== "eligible_for_later_consumption"
      || value.consequences.reviewAccepted !== false || value.consequences.campaignAccepted !== false) {
    throw new ProductionPathEvidenceError("production-path succession consequences are invalid");
  }
  const withoutId = structuredClone(value); delete withoutId.id;
  if (value.id !== `production-path-succession-v1@${digest(withoutId)}`) {
    throw new ProductionPathEvidenceError("production-path succession identity is invalid");
  }
  return value;
}

export const productionPathReference = (owner, reference, revision, value) => Object.freeze({
  owner, reference, revision, sha256: digest(value), freshness: "exact immutable revision",
});
