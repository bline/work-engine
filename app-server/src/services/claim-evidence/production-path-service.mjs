import { digest } from "./identity.mjs";
import {
  PRODUCTION_PATH_PROFILE_REVISION, makeProductionPathClaimRevision,
  validateProductionPathClaimRevision, validateProductionPathCorrectionAuthority,
  validateProductionPathEstablishment, validateProductionPathObservation,
  validateProductionPathSuccession,
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
    correct({operationId, authority, campaignRevision, selection, claims, observationId,
      reviewEpisode, candidate, succeedEpisode}) {
      if (typeof store.recordProductionPathCorrection !== "function") {
        throw new TypeError("production-path correction requires the transactional canonical store");
      }
      validateProductionPathCorrectionAuthority(authority);
      if (authority.campaign !== claims.builder.consumer.replace(/^slice-builder:/, "")
          || authority.campaign !== claims.terminal.consumer.replace(/^slice-campaign:/, "")) {
        throw new TypeError("production-path correction authority does not own the claim consumers");
      }
      const observation = store.readObservation(observationId);
      if (!observation) throw new TypeError("production-path correction observation is unavailable");
      validateProductionPathObservation(observation);
      const successors = {};
      const establishments = {};
      for (const [name, predecessor] of Object.entries(claims)) {
        validateProductionPathClaimRevision(predecessor, candidate);
        const successor = makeProductionPathClaimRevision({...structuredClone(predecessor),
          profile: {...structuredClone(predecessor.profile), requiredRealization: "claude-sonnet-5",
            continuity: "retained"}});
        if (successor.claimId !== predecessor.claimId || successor.revision === predecessor.revision) {
          throw new TypeError("production-path correction claim lineage is invalid");
        }
        const priorAdmission = store.listProductionPathEstablishments({limit: 1_000})
          .find(({claimRevision}) => claimRevision === predecessor.revision);
        if (!priorAdmission || priorAdmission.status !== "unestablished") {
          throw new TypeError("production-path correction requires an immutable unestablished predecessor");
        }
        const result = evaluate(successor, observation);
        const establishment = {schemaVersion: 1,
          operationId: `${operationId}:establish:${successor.revision}`, claimId: successor.claimId,
          claimRevision: successor.revision, observationId: observation.id,
          observationDigest: digest(observation), evaluator,
          profileRevision: PRODUCTION_PATH_PROFILE_REVISION, status: result.status,
          reasons: result.reasons, predecessor: priorAdmission.id};
        establishment.id = `production-path-establishment-v1@${digest(establishment)}`;
        validateProductionPathEstablishment(establishment);
        if (establishment.status !== "established") {
          throw new TypeError("production-path corrected claim is not established by the retained observation");
        }
        successors[name] = successor; establishments[name] = establishment;
      }
      if (new Set(Object.values(successors).map(({consumptionBoundary}) => consumptionBoundary)).size !== 2) {
        throw new TypeError("production-path correction requires builder and terminal claims atomically");
      }
      if (typeof succeedEpisode !== "function") throw new TypeError("production-path correction requires Review Episode owner");
      const episodeSuccessorRevision = succeedEpisode({successors, establishments, observation});
      const succession = {schemaVersion: 1, operationId, authorityDigest: digest(authority),
        campaignRevision, selection: structuredClone(selection), claims: Object.fromEntries(
          Object.entries(successors).map(([name, successor]) => [name, {
            claimId: successor.claimId, predecessorRevision: claims[name].revision,
            successorRevision: successor.revision,
            predecessorEstablishment: establishments[name].predecessor,
            successorEstablishment: establishments[name].id,
          }])), observationId: observation.id, observationDigest: digest(observation),
        reviewEpisode: {...structuredClone(reviewEpisode), successorRevision: episodeSuccessorRevision}, candidate: structuredClone(candidate),
        consequences: {builder: "projection_enabled", terminal: "eligible_for_later_consumption",
          reviewAccepted: false, campaignAccepted: false}};
      succession.id = `production-path-succession-v1@${digest(succession)}`;
      validateProductionPathSuccession(succession);
      const recorded = store.recordProductionPathCorrection({succession, establishments: Object.values(establishments)});
      return Object.freeze({...recorded, successors: Object.freeze(successors),
        establishments: Object.freeze(establishments), observation: Object.freeze(observation)});
    },
  });
}
