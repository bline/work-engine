import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  appendLifecycleLedgerEntry,
  createContextLifecycleEpisode,
  openSqliteAppServerStateStore,
  verifyContextLifecycleEpisode,
} from "../src/index.mjs";

const revision = (character) => `sha256:${character.repeat(64)}`;

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonical(value[key])}`
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}

function digest(value) {
  return `sha256:${createHash("sha256").update(canonical(value)).digest("hex")}`;
}

function episode(id, { requestCharacter = "1", completedSecond = 1 } = {}) {
  return createContextLifecycleEpisode({
    schemaVersion: 1,
    type: "context-lifecycle-shadow-episode",
    episodeId: id,
    requestRevision: revision(requestCharacter),
    scheduleRevision: revision("2"),
    mode: "shadow",
    subject: {
      logicalRoleInstanceId: "strategic-planner:main",
      threadId: "thread-sqlite-1",
      bindingRevision: 1,
    },
    startedAt: "2026-08-26T01:00:00.000Z",
    completedAt: `2026-08-26T01:00:0${completedSecond}.000Z`,
    pressure: {
      policyRevision: revision("3"),
      observationId: `observation-${id}`,
      observationSequence: completedSecond,
      pressureBasisPoints: 5_000,
      previousDisposition: "comfortable",
      disposition: "comfortable",
      transitionReason: "below_entry_threshold",
      measurementSource: "sqlite-fixture",
      measurementSourceRevision: revision("4"),
    },
    sourceRevision: null,
    inference: {
      status: "not_scheduled",
      candidateRevision: null,
      verificationRevision: null,
      compiler: null,
      verifier: null,
      blockerCount: 0,
      uncertaintyCount: 0,
    },
    checkpoint: {
      status: "not_attempted",
      checkpointRevision: null,
      ledgerRevision: null,
      reason: "inspection_not_scheduled",
    },
    measurements: {
      context: {
        retainedInputTokens: 20_000,
        reportedContextWindowTokens: 100_000,
        growthSincePreviousTokens: 1_000,
        estimatedAvoidedInputTokens: null,
        estimateMethod: null,
        estimateRevision: null,
      },
      compiler: {
        durationMs: null,
        inputTokens: null,
        cachedInputTokens: null,
        outputTokens: null,
        costMicrounits: null,
      },
      verifier: {
        durationMs: null,
        inputTokens: null,
        cachedInputTokens: null,
        outputTokens: null,
        costMicrounits: null,
      },
      inspectionDurationMs: null,
      publicationDurationMs: null,
      checkpointBytes: null,
    },
    transition: { status: "not_requested", retirementAttempted: false },
    failure: null,
  });
}

function initialFence() {
  return {
    logicalRoleInstanceId: "strategic-planner:main",
    threadId: "thread-sqlite-1",
    bindingRevision: 1,
    sourceRevision: revision("5"),
    authorityRevision: revision("6"),
    publicationRevision: null,
    ledgerRevision: null,
  };
}

function publicationAttempt(expectedFence, candidateCharacter, previousLedgerEntry = null) {
  const checkpoint = {
    schemaVersion: 1,
    type: "work-engine.context-checkpoint",
    subject: {
      logicalRoleInstanceId: expectedFence.logicalRoleInstanceId,
      threadId: expectedFence.threadId,
      bindingRevision: expectedFence.bindingRevision,
      sourceRevision: expectedFence.sourceRevision,
      candidateRevision: revision(candidateCharacter),
      verificationRevision: revision("7"),
      authorityRevision: expectedFence.authorityRevision,
    },
    publishedAt: "2026-08-26T01:10:00.000Z",
    predecessorCheckpointRevision: expectedFence.publicationRevision,
    authority: {
      status: "current",
      authorityRevision: expectedFence.authorityRevision,
      references: [],
      evidenceRefs: [revision("8")],
    },
    continuationState: { fixture: candidateCharacter },
    verification: { fixture: true },
  };
  const publication = Object.freeze({
    ...checkpoint,
    checkpointRevision: digest(checkpoint),
  });
  const ledgerEntry = appendLifecycleLedgerEntry(previousLedgerEntry, {
    eventType: "checkpoint_published",
    status: "observed",
    recordedAt: checkpoint.publishedAt,
    subject: {
      logicalRoleInstanceId: expectedFence.logicalRoleInstanceId,
      threadId: expectedFence.threadId,
      bindingRevision: expectedFence.bindingRevision,
    },
    evidenceRefs: [publication.checkpointRevision],
    details: {
      checkpointRevision: publication.checkpointRevision,
      candidateRevision: publication.subject.candidateRevision,
      verificationRevision: publication.subject.verificationRevision,
      authorityRevision: publication.subject.authorityRevision,
      predecessorCheckpointRevision: publication.predecessorCheckpointRevision,
    },
  });
  return { expectedFence, publication, ledgerEntry, previousLedgerEntry };
}

async function temporaryDatabase(t) {
  const directory = await mkdtemp(path.join(tmpdir(), "work-engine-state-test-"));
  t.after(async () => rm(directory, { recursive: true, force: true }));
  return path.join(directory, "app-server.sqlite3");
}

test("SQLite lifecycle episodes survive restart and preserve exact replay", async (t) => {
  const filePath = await temporaryDatabase(t);
  let store = await openSqliteAppServerStateStore({ filePath });
  const first = episode("episode-sqlite-1");
  assert.equal(store.append(first).status, "appended");
  assert.equal(store.append(structuredClone(first)).status, "replayed");
  assert.deepEqual(store.integrityCheck(), ["ok"]);
  store.close();

  store = await openSqliteAppServerStateStore({ filePath });
  t.after(() => store.close());
  const recovered = store.get(first.episodeId);
  assert.equal(recovered.episodeRevision, first.episodeRevision);
  assert.equal(verifyContextLifecycleEpisode(recovered), true);
  assert.equal(store.receipts({ logicalRoleInstanceId: "strategic-planner:main" }).length, 1);
  assert.throws(
    () => store.append(episode("episode-sqlite-1", { requestCharacter: "9" })),
    /reused for different evidence/,
  );
  assert.equal((await stat(filePath)).mode & 0o777, 0o600);
});

test("SQLite checkpoint publication atomically restores checkpoint and ledger head", async (t) => {
  const filePath = await temporaryDatabase(t);
  let store = await openSqliteAppServerStateStore({ filePath });
  const fence = initialFence();
  assert.equal(store.initializeCheckpointFence(fence).status, "initialized");
  assert.equal(store.initializeCheckpointFence(structuredClone(fence)).status, "replayed");
  const first = publicationAttempt(fence, "a");
  const committed = store.compareAndSwapPublication(first);
  assert.equal(committed.status, "committed");
  assert.equal(committed.currentFence.publicationRevision, first.publication.checkpointRevision);
  store.close();

  store = await openSqliteAppServerStateStore({ filePath });
  t.after(() => store.close());
  const recovered = store.snapshot(fence.logicalRoleInstanceId);
  assert.equal(recovered.publication.checkpointRevision, first.publication.checkpointRevision);
  assert.equal(recovered.ledgerEntry.entryRevision, first.ledgerEntry.entryRevision);
  assert.equal(recovered.fence.ledgerRevision, first.ledgerEntry.entryRevision);

  const nextFence = {
    ...recovered.fence,
    sourceRevision: revision("9"),
  };
  const fenceUpdate = store.compareAndSwapCheckpointFence({
    expectedFence: recovered.fence,
    nextFence,
  });
  assert.equal(fenceUpdate.status, "committed");
  assert.equal(fenceUpdate.currentFence.publicationRevision, first.publication.checkpointRevision);

  const second = publicationAttempt(fenceUpdate.currentFence, "b", recovered.ledgerEntry);
  const advanced = store.compareAndSwapPublication(second);
  assert.equal(advanced.status, "committed");
  assert.equal(store.snapshot(fence.logicalRoleInstanceId).ledgerEntry.sequence, 2);
});

test("two SQLite writers serialize and stale checkpoint CAS loses truthfully", async (t) => {
  const filePath = await temporaryDatabase(t);
  const firstStore = await openSqliteAppServerStateStore({ filePath });
  const secondStore = await openSqliteAppServerStateStore({ filePath });
  t.after(() => firstStore.close());
  t.after(() => secondStore.close());
  const fence = initialFence();
  firstStore.initializeCheckpointFence(fence);
  const firstView = firstStore.snapshot(fence.logicalRoleInstanceId).fence;
  const secondView = secondStore.snapshot(fence.logicalRoleInstanceId).fence;
  const winner = publicationAttempt(firstView, "c");
  const loser = publicationAttempt(secondView, "d");
  assert.equal(firstStore.compareAndSwapPublication(winner).status, "committed");
  const rejected = secondStore.compareAndSwapPublication(loser);
  assert.equal(rejected.status, "rejected");
  assert.equal(rejected.reason, "publication_conflict");
  assert.equal(rejected.currentFence.publicationRevision, winner.publication.checkpointRevision);
  assert.equal(
    secondStore.snapshot(fence.logicalRoleInstanceId).publication.checkpointRevision,
    winner.publication.checkpointRevision,
  );
  const staleFenceUpdate = secondStore.compareAndSwapCheckpointFence({
    expectedFence: secondView,
    nextFence: { ...secondView, sourceRevision: revision("e") },
  });
  assert.equal(staleFenceUpdate.status, "rejected");
  assert.equal(staleFenceUpdate.reason, "fence_conflict");
});

test("two SQLite writers preserve episode identity across connections", async (t) => {
  const filePath = await temporaryDatabase(t);
  const firstStore = await openSqliteAppServerStateStore({ filePath });
  const secondStore = await openSqliteAppServerStateStore({ filePath });
  t.after(() => firstStore.close());
  t.after(() => secondStore.close());
  const receipt = episode("episode-shared-writers");
  assert.equal(firstStore.append(receipt).status, "appended");
  assert.equal(secondStore.append(structuredClone(receipt)).status, "replayed");
  assert.throws(
    () => secondStore.append(episode("episode-shared-writers", { requestCharacter: "e" })),
    /reused for different evidence/,
  );
});

test("SQLite state refuses an unknown future schema revision", async (t) => {
  const filePath = await temporaryDatabase(t);
  const store = await openSqliteAppServerStateStore({ filePath });
  store.close();
  const { DatabaseSync } = await import("node:sqlite");
  const database = new DatabaseSync(filePath);
  database.exec("PRAGMA user_version = 2");
  database.close();
  await assert.rejects(
    openSqliteAppServerStateStore({ filePath }),
    /newer than supported schema/,
  );
});
