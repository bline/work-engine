import assert from "node:assert/strict";
import test from "node:test";

import {
  makeProductionPathClaimRevision, normalizeProductionPathObservation,
  validateProductionPathClaimRevision, validateProductionPathObservation,
} from "../../../src/services/claim-evidence/production-path-contract.mjs";

const candidate = {commit: "c".repeat(40), tree: "d".repeat(40), patchIdentity: "e".repeat(64)};
const claimInput = (boundary = "builder_projection") => ({
  proposition: "The selected native result was independently observed under the required production path.",
  subject: {candidate, reviewEpisodeId: "episode-1"}, coveredState: "admitted_native_review_result",
  consumptionBoundary: boundary,
  consumer: boundary === "builder_projection" ? "slice-builder:run-1" : "slice-campaign:run-1",
  acceptance: {owner: "operator", source: "accepted-selection", unestablishedRoute: "operator"},
  profile: {id: "production-path-v1", revision: "production-path-profile-v1",
    allowedMechanisms: ["native-review-host-receipt-v1"],
    admissibleObservers: ["app-server.reviewer-host"], integrityRequired: true,
    requiredRealization: "claude-sonnet", requiredCapabilities: ["repository_read"],
    continuity: "fresh_initial"},
});

test("production-path claim identity binds boundary and consumer and rejects self-authorization", () => {
  const builder = makeProductionPathClaimRevision(claimInput());
  const terminal = makeProductionPathClaimRevision(claimInput("campaign_terminalization"));
  assert.notEqual(builder.claimId, terminal.claimId);
  assert.equal(validateProductionPathClaimRevision(builder, candidate), builder);
  const selfAuthorized = structuredClone(builder);
  selfAuthorized.acceptance.owner = "reviewer";
  assert.throws(() => validateProductionPathClaimRevision(selfAuthorized), /self-authorized/);
  const incomplete = structuredClone(builder); delete incomplete.profile.admissibleObservers;
  assert.throws(() => validateProductionPathClaimRevision(incomplete), /fields are invalid/);
});

test("normalized host observation is integrity-bound and contains no provider path or transcript", () => {
  const observation = normalizeProductionPathObservation({
    event_identity: "event-1", selection: {id: "selection-1", revision: "a".repeat(64)},
    obligationId: "generic", subject: {candidate, reviewEpisodeId: "episode-1"},
    coveredState: "admitted_native_review_result",
    execution: {attemptId: "attempt-1", resultDigest: "b".repeat(64)},
    realization: {requested: "claude-sonnet", observed: "claude-sonnet"},
    capabilityEnvelope: {capabilities: ["repository_read"], mutationAuthorized: false},
    continuity: {mode: "fresh_initial", sessionId: "session-1"},
    transport: {mechanism: "native-review-host-receipt-v1", digest: "f".repeat(64)},
    observer: {identity: "app-server.reviewer-host", kind: "app_server_host"},
    observedAt: "2026-09-09T05:00:00.000Z", adapterVersion: "adapter-v1",
    artifacts: [{owner: "reviewer-runtime", reference: "artifact:attempt-1:transport",
      digest: "f".repeat(64), status: "verified"}],
  });
  assert.equal(validateProductionPathObservation(observation), observation);
  assert.equal(JSON.stringify(observation).includes("/home/"), false);
  assert.equal(JSON.stringify(observation).includes("transcript"), false);
  const changed = structuredClone(observation); changed.realization.observed = "other";
  assert.throws(() => validateProductionPathObservation(changed), /identity is invalid/);
});

export {candidate, claimInput};
