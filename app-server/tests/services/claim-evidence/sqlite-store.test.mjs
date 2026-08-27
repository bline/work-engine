import assert from "node:assert/strict";
import { chmod, mkdtemp, readdir, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { resolveRecord } from "../../../src/services/claim-evidence/projections.mjs";
import { openSqliteClaimEvidenceStore } from "../../../src/services/claim-evidence/sqlite-store.mjs";

const exactReference = (reference = "source") => ({
  owner: "repository", reference, revision: "blob-1", integrity_sha256: "a".repeat(64),
  freshness: "current", status: "verified",
});

const authority = {
  schema_version: 1, grant_id: "grant:producer", actor: "producer",
  profile: "proposal-research-v1",
  permissions: ["create_claim", "publish_revision", "publish_lineage", "record_reliance", "retire_reliance"],
  decision_scope: "proposal-formation", authority_reference: exactReference("authority"),
};

const revision = (overrides = {}) => ({
  proposition: "A bounded proposition", support_qualification: "supported",
  assumptions: [], limitations: [], confidence: { estimate: 1 },
  evidence_references: [exactReference()], sensitivity_references: [],
  evidence_mode: "direct_source", judgment_kind: "semantic",
  decision_scope: "proposal-formation",
  profile_payload: { materiality: "material", support_qualification: "supported" },
  reopening_conditions: [], tombstone: false, ...overrides,
});

const operation = (operationId, action, payload, expectedState = null) => ({
  schema_version: 1, operation_id: operationId, action,
  profile: "proposal-research-v1", expected_state: expectedState, payload,
});

const createOperation = (operationId = "create") => operation(operationId, "create_claim", {
  subject: {
    namespace: "research", subject_kind: "proposal", stable_subject_id: "placement",
    evidence_baseline: exactReference("baseline"), content_set: ["proposal.md"],
  },
  statement_identity: "Claims are service-owned",
  initial_revision: revision(),
});

async function temporaryDatabase(t) {
  const directory = await mkdtemp(path.join(tmpdir(), "claim-evidence-sqlite-test-"));
  t.after(async () => rm(directory, { recursive: true, force: true }));
  return path.join(directory, "claims.sqlite3");
}

test("vertical SQLite claim lifecycle survives restart, rebuilds, and replays exactly", async (t) => {
  const filePath = await temporaryDatabase(t);
  let store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  const request = createOperation();
  const published = store.publish(request, authority);
  assert.equal(published.idempotent, false);
  assert.equal(store.exportStore().operations.length, 1);
  store.close();

  store = await openSqliteClaimEvidenceStore({ filePath });
  t.after(() => store.close());
  const recovered = store.exportStore();
  assert.equal(recovered.revisions[0].id, published.result_identity);
  assert.equal(resolveRecord(store.buildProjection(), published.result_identity).revision.id, published.result_identity);
  const replay = store.publish(structuredClone(request), structuredClone(authority));
  assert.equal(replay.idempotent, true);
  assert.equal(replay.result_identity, published.result_identity);
  assert.deepEqual(store.exportStore(), recovered);

  const conflict = createOperation();
  conflict.payload.statement_identity = "Different evidence under the same operation id";
  assert.throws(() => store.publish(conflict, authority), /operation identity conflict/);
  assert.deepEqual(store.exportStore(), recovered);
  assert.deepEqual(store.integrityCheck(), ["ok"]);
  assert.equal((await stat(filePath)).mode & 0o777, 0o600);
});

test("SQLite claim artifacts remain private in a permissive existing directory", async (t) => {
  const directory = await mkdtemp(path.join(tmpdir(), "claim-evidence-permissions-test-"));
  t.after(async () => rm(directory, { recursive: true, force: true }));
  await chmod(directory, 0o777);
  const filePath = path.join(directory, "claims.sqlite3");
  const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  t.after(() => store.close());
  store.publish(createOperation(), authority);

  const artifactNames = new Set([
    path.basename(filePath), `${path.basename(filePath)}-wal`, `${path.basename(filePath)}-shm`,
  ]);
  const presentArtifacts = (await readdir(directory)).filter((name) => artifactNames.has(name));
  assert.ok(presentArtifacts.includes(path.basename(filePath)));
  for (const artifact of presentArtifacts) {
    assert.equal((await stat(path.join(directory, artifact))).mode & 0o777, 0o600, artifact);
  }
});

test("failed claim operations leave no partial canonical state", async (t) => {
  const filePath = await temporaryDatabase(t);
  const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  t.after(() => store.close());
  const first = store.publish(createOperation(), authority).result_identity;
  const second = store.publish(operation("revise", "publish_revision", {
    claim_id: store.exportStore().claims[0].id,
    revision: revision({ proposition: "A successor proposition" }),
  }, first), authority).result_identity;
  const before = store.exportStore();

  assert.throws(() => store.publish(operation("cyclic", "publish_lineage", {
    relationship: "derivation", sources: [second], target: second,
  }), authority), /cyclic lineage/);
  assert.deepEqual(store.exportStore(), before);

  assert.throws(() => store.publish(operation("stale", "publish_revision", {
    claim_id: before.claims[0].id,
    revision: revision({ proposition: "A stale successor" }),
  }, first), authority), /conflicting predecessor/);
  assert.deepEqual(store.exportStore(), before);
});

test("two SQLite claim writers serialize and only one stale predecessor wins", async (t) => {
  const filePath = await temporaryDatabase(t);
  const firstStore = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  const secondStore = await openSqliteClaimEvidenceStore({ filePath });
  t.after(() => firstStore.close());
  t.after(() => secondStore.close());
  const predecessor = firstStore.publish(createOperation(), authority).result_identity;
  const claimId = firstStore.exportStore().claims[0].id;
  const firstRequest = operation("writer-one", "publish_revision", {
    claim_id: claimId, revision: revision({ proposition: "Writer one" }),
  }, predecessor);
  const secondRequest = operation("writer-two", "publish_revision", {
    claim_id: claimId, revision: revision({ proposition: "Writer two" }),
  }, predecessor);

  const winner = firstStore.publish(firstRequest, authority);
  assert.throws(() => secondStore.publish(secondRequest, authority), /conflicting predecessor/);
  const recovered = secondStore.exportStore();
  assert.equal(recovered.revisions.length, 2);
  assert.equal(recovered.revisions.at(-1).id, winner.result_identity);
  assert.equal(recovered.operations.some((item) => item.operation_id === "writer-two"), false);
});

test("authority bootstrap is explicit and cannot become later admission", async (t) => {
  const filePath = await temporaryDatabase(t);
  await assert.rejects(
    openSqliteClaimEvidenceStore({ filePath }),
    /requires explicit bootstrap authorities/,
  );
  const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  store.close();
  await assert.rejects(
    openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] }),
    /bootstrap is only valid for an uninitialized database/,
  );
  const reopened = await openSqliteClaimEvidenceStore({ filePath });
  t.after(() => reopened.close());
  assert.deepEqual(reopened.exportStore().authorities, [authority]);
});

test("SQLite claim store refuses future or incomplete migration history", async (t) => {
  const futurePath = await temporaryDatabase(t);
  let store = await openSqliteClaimEvidenceStore({
    filePath: futurePath, bootstrapAuthorities: [authority],
  });
  store.close();
  const { DatabaseSync } = await import("node:sqlite");
  let database = new DatabaseSync(futurePath);
  database.exec("PRAGMA user_version = 2");
  database.close();
  await assert.rejects(
    openSqliteClaimEvidenceStore({ filePath: futurePath }),
    /newer than supported schema/,
  );

  const incompletePath = await temporaryDatabase(t);
  store = await openSqliteClaimEvidenceStore({
    filePath: incompletePath, bootstrapAuthorities: [authority],
  });
  store.close();
  database = new DatabaseSync(incompletePath);
  database.exec("DELETE FROM schema_migrations");
  database.close();
  await assert.rejects(
    openSqliteClaimEvidenceStore({ filePath: incompletePath }),
    /migration history is incomplete/,
  );
});

test("canonical export detects stored payload corruption", async (t) => {
  const filePath = await temporaryDatabase(t);
  const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  store.close();
  const { DatabaseSync } = await import("node:sqlite");
  const database = new DatabaseSync(filePath);
  database.prepare("UPDATE canonical_claim_state SET store_json = ? WHERE singleton_id = 1")
    .run("{}\n");
  database.close();
  const reopened = await openSqliteClaimEvidenceStore({ filePath });
  t.after(() => reopened.close());
  assert.throws(() => reopened.exportStore(), /canonical integrity check/);
  assert.throws(() => reopened.integrityCheck(), /canonical integrity check/);
});
