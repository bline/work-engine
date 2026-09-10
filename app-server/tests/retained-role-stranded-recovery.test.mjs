import assert from "node:assert/strict";
import test from "node:test";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { createSliceCampaignService } from "../src/services/slice-campaign/service.mjs";
import { openSqliteSliceCampaignStore } from "../src/services/slice-campaign/sqlite-store.mjs";
import { buildExternalBootstrapPacket } from "../src/services/slice-campaign/external-bootstrap-adoption-contract.mjs";
import { identity, unsignedPacket } from "./external-bootstrap-adoption.test.mjs";

const owners = {
  reviewSubject: {async createCandidate() { throw new Error("candidate must not be called"); },
    async createPhysicalProfile() { throw new Error("profile must not be called"); }},
  receiptFinalizer: {async finalize() { throw new Error("terminalization must not be called"); }},
};

test("exact stranded-state evidence adoption is append-only, CAS-bound, replayable, and non-advancing", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "stranded-bootstrap-adoption."));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const database = path.join(directory, "campaign.sqlite");
  let store = await openSqliteSliceCampaignStore({filePath: database});
  let service = createSliceCampaignService({store, ...owners});
  let campaign = service.admit({identity, workspace: "/private-fixture-only",
    acceptedBoundary: {reference: "wind-walker:context-lifecycle-bootstrap-recovery-20260910-builder-plan-v1@9601e2377122d6c707196b7a1b27e2f9007f6e30",
      sha256: "07fc193ab4ab76a938334ae939ddcfe5151711234d5918a36e829301b6325e61"},
    baseline: {acceptedCommit: "d7fee69cd2bb78e56736ceeb8463fb37a3687627",
      acceptedTree: "c39438f617495af8dabb0974e07ccb41a3d64a88", interSliceCommit: "fixture"}});
  campaign = service.advance({identity, expectedRevision: campaign.revision, phase: "implementing",
    consequence: {strandedPhase: "implementing"}});
  const before = structuredClone(campaign);
  const packet = buildExternalBootstrapPacket(unsignedPacket(campaign.revision));
  const adopted = service.adoptExternalBootstrapEvidence({identity,
    expectedRevision: campaign.revision, packet});
  assert.deepEqual(adopted.campaign, before);
  for (const field of ["phase", "latestConsequence", "candidate", "review", "terminal"]) {
    assert.deepEqual(adopted.campaign[field], before[field]);
  }
  assert.equal(adopted.adoption.authority.phaseAdvance, false);
  assert.equal(adopted.adoption.authority.publication, false);
  assert.deepEqual(service.adoptExternalBootstrapEvidence({identity,
    expectedRevision: campaign.revision, packet}), adopted);
  assert.throws(() => service.adoptExternalBootstrapEvidence({identity,
    expectedRevision: "0".repeat(64), packet}), /revision conflict/);
  const conflict = buildExternalBootstrapPacket({...unsignedPacket(campaign.revision),
    checkpoint: {...unsignedPacket(campaign.revision).checkpoint, commit: "conflict"}});
  assert.throws(() => service.adoptExternalBootstrapEvidence({identity,
    expectedRevision: campaign.revision, packet: conflict}), /conflicts with adopted evidence/);

  store.close();
  store = await openSqliteSliceCampaignStore({filePath: database});
  t.after(() => store.close());
  service = createSliceCampaignService({store, ...owners});
  assert.deepEqual(service.recover(identity), before);
  assert.equal(service.recoverExternalBootstrapEvidence(identity).packetDigest,
    packet.whole_packet_sha256);
});
