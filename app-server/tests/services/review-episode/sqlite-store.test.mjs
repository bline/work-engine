import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { createImplementationReviewService } from "../../../src/services/implementation-review/service.mjs";
import { digest } from "../../../src/services/review-episode/contract.mjs";
import { createReviewEpisodeService } from "../../../src/services/review-episode/service.mjs";
import { openSqliteReviewEpisodeStore } from "../../../src/services/review-episode/sqlite-store.mjs";

const sha = (value) => createHash("sha256").update(value).digest("hex");
const ref = (owner, reference, revision, value) => ({ owner, reference, revision, sha256: sha(value), freshness: "exact_revision" });

test("SQLite review-episode state survives restart and preserves CAS history", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "review-episode-sqlite."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const filePath = path.join(directory, "episode.sqlite");
  const result = JSON.parse(await readFile(new URL("../../fixtures/implementation-review/acceptable-as-is.json", import.meta.url)));
  const identity = { runId: "s9", sliceNumber: 1, attemptId: "attempt", planVersion: "plan", reviewObligationId: "review", reviewEpisodeId: "episode" };
  const authority = { schemaVersion: 1, grantId: "grant", identity,
    source: ref("human", "plan", "r1", "plan"),
    writer: { actorId: "reviewer", provider: "claude", generation: 1, runtimeSession: ref("runtime", "session", "g1", "session") },
    readers: ["reviewer", "builder"], initialSubject: { ...ref("checkpoint", "candidate", "r1", "candidate"), sha256: digest(result.subject) }, predecessorRevision: null };
  let store = await openSqliteReviewEpisodeStore({ filePath });
  let service = createReviewEpisodeService({ store, implementationReview: createImplementationReviewService() });
  let state = service.begin({ authority, transitionId: "begin" });
  state = service.transition({ authority, expectedRevision: state.revision, transitionId: "result",
    action: "record_result", payload: { result, unresolvedQuestions: [] } });
  const expected = state.revision; store.close();
  store = await openSqliteReviewEpisodeStore({ filePath }); t.after(() => store.close());
  service = createReviewEpisodeService({ store, implementationReview: createImplementationReviewService() });
  assert.equal(service.recover(identity).revision, expected);
  assert.equal(service.history({ identity }).length, 2);
  assert.throws(() => service.transition({ authority, expectedRevision: sha("stale"), transitionId: "uncertain",
    action: "mark_uncertain", payload: { reason: "lost", reconciliationAction: "read" } }), /expected revision/);
});
