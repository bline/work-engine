import assert from "node:assert/strict";
import { chmod, mkdtemp, readdir, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import test from "node:test";

import { observeGitCheckpoint } from "../../../src/services/claim-evidence/git-checkpoint-observation.mjs";
import { normalizeObservation, observationEventIdentity } from "../../../src/services/claim-evidence/observation.mjs";
import { digest } from "../../../src/services/claim-evidence/identity.mjs";
import { resolveRecord } from "../../../src/services/claim-evidence/projections.mjs";
import { closeSemanticShadowEpisode, SEMANTIC_SHADOW_INSTRUCTION_BUNDLE_REVISION } from "../../../src/services/claim-evidence/semantic-shadow-contract.mjs";
import { openSqliteClaimEvidenceStore } from "../../../src/services/claim-evidence/sqlite-store.mjs";

const execFileAsync = promisify(execFile);

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

const exactObservation = (overrides = {}) => normalizeObservation({
  event_identity: observationEventIdentity("test-producer", "test-source"),
  producer: { identity: "test-producer", kind: "deterministic_test_adapter" },
  origin: { kind: "test_receipt", reference: "receipt:test", trust_classification: "untrusted_input" },
  subject: { namespace: "tests", subject_kind: "receipt", stable_subject_id: "sqlite", content_set: ["receipt.json"] },
  evidence_baseline: exactReference("observation-baseline"),
  artifact: {
    kind: "test_receipt", reference: "receipt:test",
    digest: { algorithm: "sha256", value: "b".repeat(64) }, verification: "verified",
    checkpoint: { object_format: "sha256", commit: "receipt-test", tree: "suite-test" },
  },
  observed_at: "2026-08-26T12:00:00.000Z", provider_sequence: null,
  completeness: "available", exclusions: [], collection_failures: [],
  executable_generation: "test-generation", adapter_version: "test-adapter-v1",
  ...overrides,
});

async function temporaryDatabase(t) {
  const directory = await mkdtemp(path.join(tmpdir(), "claim-evidence-sqlite-test-"));
  t.after(async () => rm(directory, { recursive: true, force: true }));
  return path.join(directory, "claims.sqlite3");
}

async function temporaryGitCheckpoint(t) {
  const repositoryPath = await mkdtemp(path.join(tmpdir(), "claim-evidence-git-test-"));
  t.after(async () => rm(repositoryPath, { recursive: true, force: true }));
  await execFileAsync("git", ["init", "--quiet", repositoryPath]);
  await execFileAsync("git", ["-C", repositoryPath, "config", "user.name", "Observation Test"]);
  await execFileAsync("git", ["-C", repositoryPath, "config", "user.email", "observation@example.invalid"]);
  await writeFile(path.join(repositoryPath, "artifact.txt"), "exact checkpoint artifact\n");
  await execFileAsync("git", ["-C", repositoryPath, "add", "artifact.txt"]);
  await execFileAsync("git", ["-C", repositoryPath, "commit", "--quiet", "-m", "checkpoint"]);
  const { stdout: commitOutput } = await execFileAsync("git", ["-C", repositoryPath, "rev-parse", "HEAD"]);
  const commit = commitOutput.trim();
  const { stdout: treeOutput } = await execFileAsync("git", ["-C", repositoryPath, "rev-parse", `${commit}^{tree}`]);
  return { repositoryPath, commit, tree: treeOutput.trim() };
}

test("vertical Git checkpoint observation survives SQLite restart without semantic promotion", async (t) => {
  const filePath = await temporaryDatabase(t);
  const checkpoint = await temporaryGitCheckpoint(t);
  const observation = await observeGitCheckpoint({
    ...checkpoint,
    expectedCommit: checkpoint.commit,
    expectedTree: checkpoint.tree,
    producerIdentity: "app-server:test-checkpoint-producer",
    observedAt: "2026-08-26T12:00:00.000Z",
    providerSequence: "checkpoint-1",
    origin: { kind: "git_repository", reference: "test-repository", trust_classification: "host_verified_artifact" },
    subject: { namespace: "tests", subject_kind: "checkpoint", stable_subject_id: "vertical", content_set: ["artifact.txt"] },
    evidenceBaseline: exactReference("checkpoint-baseline"),
    executableGeneration: "test-generation-1",
    adapterVersion: "git-checkpoint-observation-v1",
  });

  let store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  const before = store.exportStore();
  const recorded = store.recordObservation(observation);
  assert.equal(recorded.idempotent, false);
  assert.deepEqual(store.readObservation(observation.id), observation);
  assert.deepEqual(store.exportStore(), before);
  store.close();

  store = await openSqliteClaimEvidenceStore({ filePath });
  t.after(() => store.close());
  assert.deepEqual(store.readObservation(observation.id), observation);
  assert.deepEqual(store.listObservations(), [observation]);
  const replay = store.recordObservation(structuredClone(observation));
  assert.equal(replay.idempotent, true);
  assert.equal(replay.observation_id, observation.id);
  const after = store.exportStore();
  for (const collection of ["claims", "revisions", "lineage", "reliances", "operations"]) {
    assert.equal(after[collection].length, before[collection].length, collection);
  }
});

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

test("SQLite observation custody replays exactly and rejects conflicting event reuse across writers", async (t) => {
  const filePath = await temporaryDatabase(t);
  const firstStore = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  const secondStore = await openSqliteClaimEvidenceStore({ filePath });
  t.after(() => firstStore.close());
  t.after(() => secondStore.close());
  const observation = exactObservation();
  assert.equal(firstStore.recordObservation(observation).idempotent, false);
  assert.equal(secondStore.recordObservation(structuredClone(observation)).idempotent, true);

  const conflict = exactObservation({
    provider_sequence: "different-payload-under-the-same-event",
  });
  assert.throws(() => secondStore.recordObservation(conflict), /event identity conflict/);
  assert.deepEqual(firstStore.listObservations(), [observation]);
  assert.deepEqual(firstStore.listObservations({ limit: 1, afterEventIdentity: observation.event_identity }), []);
  assert.throws(() => firstStore.listObservations({ limit: 1_001 }), /exceeds 1000/);
  assert.equal(firstStore.exportStore().operations.length, 0);
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
  database.exec("PRAGMA user_version = 4");
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

test("SQLite migrates canonical schema v1 to separate observation custody", async (t) => {
  const filePath = await temporaryDatabase(t);
  let store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  const canonicalBefore = store.exportStore();
  store.close();
  const { DatabaseSync } = await import("node:sqlite");
  const database = new DatabaseSync(filePath);
  database.exec("DROP TABLE evidence_observations");
  database.exec("DROP TABLE semantic_shadow_episodes");
  database.exec("DELETE FROM schema_migrations WHERE version >= 2");
  database.exec("PRAGMA user_version = 1");
  database.close();

  store = await openSqliteClaimEvidenceStore({ filePath });
  t.after(() => store.close());
  assert.deepEqual(store.exportStore(), canonicalBefore);
  assert.deepEqual(store.listObservations(), []);
  assert.equal(store.recordObservation(exactObservation()).idempotent, false);
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

test("observation reads and integrity checks detect stored payload corruption", async (t) => {
  const filePath = await temporaryDatabase(t);
  const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  const observation = exactObservation();
  store.recordObservation(observation);
  store.close();
  const { DatabaseSync } = await import("node:sqlite");
  const database = new DatabaseSync(filePath);
  database.prepare("UPDATE evidence_observations SET observation_json = ? WHERE observation_id = ?")
    .run("{}\n", observation.id);
  database.close();
  const reopened = await openSqliteClaimEvidenceStore({ filePath });
  t.after(() => reopened.close());
  assert.throws(() => reopened.readObservation(observation.id), /canonical integrity check/);
  assert.throws(() => reopened.listObservations(), /canonical integrity check/);
  assert.throws(() => reopened.integrityCheck(), /canonical integrity check/);
});

const closedShadowEpisode = (observation, overrides = {}) => closeSemanticShadowEpisode({
  episode_identity: "episode:sqlite", request_revision: "c".repeat(64),
  instruction_bundle_revision: SEMANTIC_SHADOW_INSTRUCTION_BUNDLE_REVISION,
  source_projection_revision: "d".repeat(64),
  observation_bindings: [{ observation_id: observation.id, observation_digest: digest(observation) }],
  started_at: "2026-08-27T12:00:00.000Z", closed_at: "2026-08-27T12:00:01.000Z",
  terminal_stage: "compiler", terminal_status: "failed", compiler_attempt: null,
  candidate: null, verifier_attempt: null, verification: null,
  failure: { kind: "compiler_inference_failure", message: "provider unavailable" },
  eligibility_disposition: null, ...overrides,
});

test("SQLite semantic shadow custody serializes writers and rejects conflicting replay", async (t) => {
  const filePath = await temporaryDatabase(t);
  const first = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  const second = await openSqliteClaimEvidenceStore({ filePath });
  t.after(() => first.close()); t.after(() => second.close());
  const observation = exactObservation(); first.recordObservation(observation);
  const before = first.exportStore(); const observations = first.listObservations(); const episode = closedShadowEpisode(observation);
  assert.equal(first.recordShadowEpisode(episode).idempotent, false);
  assert.equal(second.recordShadowEpisode(structuredClone(episode)).idempotent, true);
  assert.throws(() => second.recordShadowEpisode(closedShadowEpisode(observation, { closed_at: "2026-08-27T12:00:02.000Z" })), /episode identity conflict/);
  assert.deepEqual(second.readShadowEpisode(episode.episode_identity), episode);
  assert.deepEqual(first.exportStore(), before); assert.deepEqual(first.listObservations(), observations);
  const unadmitted = exactObservation({ event_identity: observationEventIdentity("other", "unadmitted") });
  assert.throws(() => first.recordShadowEpisode(closedShadowEpisode(unadmitted, { episode_identity: "episode:unadmitted" })), /unadmitted observation binding/);
});

test("SQLite migrates schema v2 to separate semantic shadow custody", async (t) => {
  const filePath = await temporaryDatabase(t); let store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] }); const before = store.exportStore(); store.close();
  const { DatabaseSync } = await import("node:sqlite"); const database = new DatabaseSync(filePath);
  database.exec("DROP TABLE semantic_shadow_episodes"); database.exec("DELETE FROM schema_migrations WHERE version = 3"); database.exec("PRAGMA user_version = 2"); database.close();
  store = await openSqliteClaimEvidenceStore({ filePath }); t.after(() => store.close());
  assert.deepEqual(store.exportStore(), before); assert.deepEqual(store.listShadowEpisodes(), []);
  const observation = exactObservation(); store.recordObservation(observation);
  assert.equal(store.recordShadowEpisode(closedShadowEpisode(observation)).idempotent, false);
});

test("semantic shadow reads and integrity checks detect stored payload corruption", async (t) => {
  const filePath = await temporaryDatabase(t); const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] }); const observation = exactObservation(); store.recordObservation(observation); const episode = closedShadowEpisode(observation); store.recordShadowEpisode(episode); store.close();
  const { DatabaseSync } = await import("node:sqlite"); const database = new DatabaseSync(filePath);
  database.prepare("UPDATE semantic_shadow_episodes SET episode_json = ? WHERE episode_identity = ?").run("{}\n", episode.episode_identity); database.close();
  const reopened = await openSqliteClaimEvidenceStore({ filePath }); t.after(() => reopened.close());
  assert.throws(() => reopened.readShadowEpisode(episode.episode_identity), /canonical integrity check/);
  assert.throws(() => reopened.integrityCheck(), /canonical integrity check/);
});
