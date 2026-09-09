import { digest } from "./identity.mjs";
import {
  PRODUCTION_PATH_PROFILE_REVISION, validateProductionPathClaimRevision,
  validateProductionPathEstablishment, validateProductionPathObservation,
} from "./production-path-contract.mjs";

const sorted = (value) => [...value].sort();

function evaluate(claim, observation) {
  const reasons = [];
  if (!observation) reasons.push("observation_unavailable");
  else {
    if (digest(claim.subject) !== digest(observation.subject)) reasons.push("subject_mismatch");
    if (claim.coveredState !== observation.coveredState) reasons.push("covered_state_mismatch");
    if (!claim.profile.allowedMechanisms.includes(observation.transport.mechanism)) reasons.push("mechanism_mismatch");
    if (!claim.profile.admissibleObservers.includes(observation.observer.identity)) reasons.push("observer_not_admissible");
    if (claim.profile.requiredRealization !== observation.realization.observed) reasons.push("realization_mismatch");
    if (observation.capabilityEnvelope.mutationAuthorized) reasons.push("mutation_authorized");
    if (!claim.profile.requiredCapabilities.every((item) => observation.capabilityEnvelope.capabilities.includes(item))) reasons.push("capability_mismatch");
    const continuity = observation.continuity.mode === "fresh_initial" ? "fresh_initial" : "retained";
    if (claim.profile.continuity !== continuity) reasons.push("continuity_mismatch");
    if (claim.profile.integrityRequired && (!observation.transport.digest
        || observation.artifacts.some(({status}) => status !== "verified"))) reasons.push("integrity_or_artifact_unavailable");
  }
  return {status: reasons.includes("mutation_authorized") ? "false"
    : reasons.length ? "unestablished" : "established", reasons: sorted(reasons)};
}

export function createProductionPathEvidenceService({store, evaluator = "claim-evidence.production-path-v1"} = {}) {
  if (!store?.recordObservation || !store?.readObservation
      || !store?.recordProductionPathEstablishment || !store?.readProductionPathEstablishment) {
    throw new TypeError("production-path evidence service requires the canonical claim-evidence store");
  }
  return Object.freeze({
    admit({operationId, claim, observation, predecessor = null}) {
      validateProductionPathClaimRevision(claim);
      if (observation !== null) validateProductionPathObservation(observation);
      const admittedObservation = observation === null ? null : store.recordObservation(observation).observation;
      const result = evaluate(claim, admittedObservation);
      const establishment = {schemaVersion: 1, operationId,
        claimId: claim.claimId, claimRevision: claim.revision,
        observationId: admittedObservation?.id ?? "observation:unavailable",
        observationDigest: admittedObservation ? digest(admittedObservation) : "0".repeat(64),
        evaluator, profileRevision: PRODUCTION_PATH_PROFILE_REVISION,
        status: result.status, reasons: result.reasons, predecessor};
      establishment.id = `production-path-establishment-v1@${digest(establishment)}`;
      validateProductionPathEstablishment(establishment);
      return Object.freeze(store.recordProductionPathEstablishment(establishment).establishment);
    },
    read(id) { return store.readProductionPathEstablishment(id); },
  });
}
