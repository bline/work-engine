import assert from "node:assert/strict";
import {mkdtemp, rm} from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  createProductionPathEvidenceService, makeProductionPathClaimRevision,
  normalizeProductionPathObservation, openSqliteClaimEvidenceStore,
} from "../../../src/index.mjs";

const candidate = {commit: "c".repeat(40), tree: "d".repeat(40), patchIdentity: "e".repeat(64)};
const claim = () => makeProductionPathClaimRevision({
  proposition: "Observed native review production path", subject: {candidate, reviewEpisodeId: "episode-1"},
  coveredState: "admitted_native_review_result", consumptionBoundary: "builder_projection",
  consumer: "slice-builder:run-1",
  acceptance: {owner: "operator", source: "selection-1", unestablishedRoute: "operator"},
  profile: {id: "production-path-v1", revision: "production-path-profile-v1",
    allowedMechanisms: ["native-review-host-receipt-v1"], admissibleObservers: ["app-server.reviewer-host"],
    integrityRequired: true, requiredRealization: "claude-sonnet",
    requiredCapabilities: ["repository_read"], continuity: "fresh_initial"},
});
const observation = (overrides = {}) => normalizeProductionPathObservation({
  event_identity: overrides.event_identity ?? "event-1", selection: {id: "selection-1", revision: "a".repeat(64)},
  obligationId: "generic", subject: {candidate, reviewEpisodeId: "episode-1"},
  coveredState: "admitted_native_review_result", execution: {attemptId: "attempt-1", resultDigest: "b".repeat(64)},
  realization: {requested: "claude-sonnet", observed: overrides.observed ?? "claude-sonnet"},
  capabilityEnvelope: {capabilities: ["repository_read"], mutationAuthorized: overrides.mutationAuthorized ?? false},
  continuity: {mode: "fresh_initial", sessionId: "session-1"},
  transport: {mechanism: "native-review-host-receipt-v1", digest: "f".repeat(64)},
  observer: {identity: overrides.observer ?? "app-server.reviewer-host", kind: "app_server_host"},
  observedAt: "2026-09-09T05:00:00.000Z", adapterVersion: "adapter-v1",
  artifacts: [{owner: "reviewer-runtime", reference: "artifact:transport", digest: "f".repeat(64), status: "verified"}],
});

test("production-path admission persists established, unestablished, and contradictory evidence", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "ppce-production-path."));
  t.after(() => rm(directory, {recursive: true, force: true}));
  let store = await openSqliteClaimEvidenceStore({filePath: path.join(directory, "claims.sqlite"), bootstrapAuthorities: []});
  const service = createProductionPathEvidenceService({store});
  const established = service.admit({operationId: "establish-1", claim: claim(), observation: observation()});
  assert.equal(established.status, "established");
  assert.deepEqual(service.admit({operationId: "establish-1", claim: claim(), observation: observation()}), established);
  const missing = service.admit({operationId: "missing-1", claim: claim(), observation: null});
  assert.equal(missing.status, "unestablished");
  const mismatched = service.admit({operationId: "mismatch-1", claim: claim(), observation: observation({event_identity: "event-2", observer: "reviewer"})});
  assert.equal(mismatched.status, "unestablished");
  const contradicted = service.admit({operationId: "false-1", claim: claim(), observation: observation({event_identity: "event-3", mutationAuthorized: true})});
  assert.equal(contradicted.status, "false");
  const id = established.id; store.close();
  store = await openSqliteClaimEvidenceStore({filePath: path.join(directory, "claims.sqlite")});
  t.after(() => store.close());
  assert.deepEqual(store.readProductionPathEstablishment(id), established);
});
