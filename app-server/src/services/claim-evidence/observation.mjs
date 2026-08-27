import { ClaimEvidenceError, COMPLETENESS_STATES } from "./contract.mjs";
import { digest, validateTransportSafeJson } from "./identity.mjs";
import { exactFields, nonempty, stringList, validateReference } from "./validation.mjs";

export const OBSERVATION_SCHEMA_VERSION = 1;
export const OBSERVATION_VERIFICATION_STATES = new Set(["verified", "unavailable"]);

function requireObservation(condition, message) {
  if (!condition) throw new ClaimEvidenceError(message);
}

function deepFreeze(value) {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    Object.values(value).forEach(deepFreeze);
    Object.freeze(value);
  }
  return value;
}

function validateTimestamp(value) {
  nonempty(value, "observation.observed_at");
  requireObservation(Number.isFinite(Date.parse(value)), "observation.observed_at is invalid");
  requireObservation(new Date(value).toISOString() === value, "observation.observed_at must be canonical ISO-8601");
}

function validateDigest(value, label) {
  exactFields(value, ["algorithm", "value"], label);
  requireObservation(value.algorithm === "sha256", `${label} algorithm is unsupported`);
  requireObservation(/^[0-9a-f]{64}$/.test(value.value), `${label} value is invalid`);
}

function validateSubject(subject) {
  exactFields(subject, ["namespace", "subject_kind", "stable_subject_id", "content_set"], "observation subject");
  for (const key of ["namespace", "subject_kind", "stable_subject_id"]) nonempty(subject[key], `observation.subject.${key}`);
  stringList(subject.content_set, "observation.subject.content_set", { required: true });
}

export function validateObservation(observation) {
  validateTransportSafeJson(observation, "observation");
  exactFields(observation, [
    "schema_version", "id", "event_identity", "producer", "origin", "subject",
    "evidence_baseline", "artifact", "observed_at", "provider_sequence",
    "completeness", "exclusions", "collection_failures", "executable_generation",
    "adapter_version",
  ], "observation");
  requireObservation(observation.schema_version === OBSERVATION_SCHEMA_VERSION, "unsupported observation version");
  nonempty(observation.event_identity, "observation.event_identity");
  exactFields(observation.producer, ["identity", "kind"], "observation producer");
  nonempty(observation.producer.identity, "observation.producer.identity");
  nonempty(observation.producer.kind, "observation.producer.kind");
  exactFields(observation.origin, ["kind", "reference", "trust_classification"], "observation origin");
  for (const key of ["kind", "reference", "trust_classification"]) nonempty(observation.origin[key], `observation.origin.${key}`);
  validateSubject(observation.subject);
  validateReference(observation.evidence_baseline, "observation evidence baseline");
  exactFields(observation.artifact, ["kind", "reference", "digest", "verification", "checkpoint"], "observation artifact");
  nonempty(observation.artifact.kind, "observation.artifact.kind");
  nonempty(observation.artifact.reference, "observation.artifact.reference");
  requireObservation(OBSERVATION_VERIFICATION_STATES.has(observation.artifact.verification), "unknown artifact verification state");
  exactFields(observation.artifact.checkpoint, ["object_format", "commit", "tree"], "observation checkpoint");
  for (const key of ["object_format", "commit", "tree"]) nonempty(observation.artifact.checkpoint[key], `observation.artifact.checkpoint.${key}`);
  if (observation.artifact.verification === "verified") {
    validateDigest(observation.artifact.digest, "observation artifact digest");
  } else {
    requireObservation(observation.artifact.digest === null, "unavailable artifact cannot claim a digest");
    requireObservation(observation.collection_failures.length > 0, "unavailable artifact must retain a collection failure");
  }
  validateTimestamp(observation.observed_at);
  requireObservation(observation.provider_sequence === null || (typeof observation.provider_sequence === "string" && observation.provider_sequence.length > 0), "observation provider_sequence is invalid");
  requireObservation(COMPLETENESS_STATES.has(observation.completeness), "unknown observation completeness");
  stringList(observation.exclusions, "observation exclusions");
  stringList(observation.collection_failures, "observation collection failures");
  requireObservation(observation.artifact.verification !== "verified" || observation.completeness !== "unavailable", "verified artifact cannot have unavailable completeness");
  requireObservation(observation.artifact.verification !== "unavailable" || observation.completeness === "unavailable", "unavailable artifact must have unavailable completeness");
  nonempty(observation.executable_generation, "observation.executable_generation");
  nonempty(observation.adapter_version, "observation.adapter_version");
  const withoutId = structuredClone(observation);
  delete withoutId.id;
  requireObservation(observation.id === `observation-v1@${digest(withoutId)}`, "observation identity mismatch");
  return observation;
}

export function normalizeObservation(input) {
  exactFields(input, [
    "event_identity", "producer", "origin", "subject", "evidence_baseline", "artifact",
    "observed_at", "provider_sequence", "completeness", "exclusions",
    "collection_failures", "executable_generation", "adapter_version",
  ], "observation input");
  const observation = {
    schema_version: OBSERVATION_SCHEMA_VERSION,
    ...structuredClone(input),
  };
  observation.id = `observation-v1@${digest(observation)}`;
  validateObservation(observation);
  return deepFreeze(observation);
}

export function observationEventIdentity(producerIdentity, sourceIdentity) {
  nonempty(producerIdentity, "observation producer identity");
  nonempty(sourceIdentity, "observation source identity");
  return `observation-event-v1@${digest({ producer_identity: producerIdentity, source_identity: sourceIdentity })}`;
}
