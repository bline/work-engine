import {
  chmodSync, closeSync, constants, fstatSync, lstatSync, mkdirSync, openSync,
} from "node:fs";
import path from "node:path";

export class SqliteSliceCampaignStore {
  constructor(database, filePath) { this.database = database; this.filePath = filePath; this.closed = false; }
  #transaction(operation) {
    if (this.closed) throw new Error("slice campaign store is closed");
    this.database.exec("BEGIN IMMEDIATE");
    try { const result = operation(); this.database.exec("COMMIT"); return result; }
    catch (error) { try { this.database.exec("ROLLBACK"); } catch {} throw error; }
  }
  get(key) {
    const row = this.database.prepare("SELECT state_json FROM slice_campaign_state WHERE identity_key = ?").get(key);
    return row ? JSON.parse(row.state_json) : null;
  }
  admit(key, workspace, state) {
    return this.#transaction(() => {
      const holder = this.database.prepare("SELECT identity_key FROM slice_campaign_admission WHERE workspace = ?").get(workspace);
      if (holder && holder.identity_key !== key) throw new Error("workspace already has an admitted mutable slice");
      try {
        this.database.prepare("INSERT INTO slice_campaign_state(identity_key, revision, state_json) VALUES(?,?,?)")
          .run(key, state.revision, JSON.stringify(state));
        this.database.prepare("INSERT INTO slice_campaign_admission(workspace, identity_key) VALUES(?,?)").run(workspace, key);
      } catch (error) {
        if (String(error.message).includes("UNIQUE")) throw new Error("slice campaign attempt already exists");
        throw error;
      }
    });
  }
  put(key, state, expectedRevision, { releaseWorkspace = null } = {}) {
    return this.#transaction(() => {
      const result = this.database.prepare(
        "UPDATE slice_campaign_state SET revision = ?, state_json = ? WHERE identity_key = ? AND revision = ?",
      ).run(state.revision, JSON.stringify(state), key, expectedRevision);
      if (result.changes !== 1) throw new Error("slice campaign revision conflict");
      if (releaseWorkspace !== null) this.database.prepare(
        "DELETE FROM slice_campaign_admission WHERE workspace = ? AND identity_key = ?",
      ).run(releaseWorkspace, key);
    });
  }
  supersede({key, expectedRevision, workspace, state, successorKey, successorState}) {
    return this.#transaction(() => {
      const current = this.database.prepare(
        "SELECT revision FROM slice_campaign_state WHERE identity_key = ?",
      ).get(key);
      if (current?.revision !== expectedRevision) throw new Error("slice campaign revision conflict");
      if (this.database.prepare(
        "SELECT identity_key FROM slice_campaign_state WHERE identity_key = ?",
      ).get(successorKey)) throw new Error("slice campaign successor attempt already exists");
      const holder = this.database.prepare(
        "SELECT identity_key FROM slice_campaign_admission WHERE workspace = ?",
      ).get(workspace);
      if (holder?.identity_key !== key) {
        throw new Error("slice campaign supersession requires the source workspace admission");
      }
      const updated = this.database.prepare(
        "UPDATE slice_campaign_state SET revision = ?, state_json = ? WHERE identity_key = ? AND revision = ?",
      ).run(state.revision, JSON.stringify(state), key, expectedRevision);
      if (updated.changes !== 1) throw new Error("slice campaign revision conflict");
      this.database.prepare(
        "INSERT INTO slice_campaign_state(identity_key, revision, state_json) VALUES(?,?,?)",
      ).run(successorKey, successorState.revision, JSON.stringify(successorState));
      const transferred = this.database.prepare(
        "UPDATE slice_campaign_admission SET identity_key = ? WHERE workspace = ? AND identity_key = ?",
      ).run(successorKey, workspace, key);
      if (transferred.changes !== 1) {
        throw new Error("slice campaign supersession failed to transfer workspace admission");
      }
    });
  }
  adoptExternalBootstrapEvidence(key, expectedCampaignRevision, adoption) {
    return this.#transaction(() => {
      const campaign = this.database.prepare(
        "SELECT revision FROM slice_campaign_state WHERE identity_key = ?",
      ).get(key);
      if (campaign?.revision !== expectedCampaignRevision) throw new Error("slice campaign revision conflict");
      const existing = this.database.prepare(
        "SELECT packet_digest, adoption_json FROM slice_campaign_external_bootstrap_evidence WHERE identity_key = ?",
      ).get(key);
      if (existing) {
        if (existing.packet_digest !== adoption.packetDigest) throw new Error("external bootstrap packet conflicts with adopted evidence");
        return JSON.parse(existing.adoption_json);
      }
      this.database.prepare(
        "INSERT INTO slice_campaign_external_bootstrap_evidence(identity_key, campaign_revision, packet_digest, adoption_json) VALUES(?,?,?,?)",
      ).run(key, expectedCampaignRevision, adoption.packetDigest, JSON.stringify(adoption));
      return adoption;
    });
  }
  getExternalBootstrapEvidence(key) {
    const row = this.database.prepare(
      "SELECT adoption_json FROM slice_campaign_external_bootstrap_evidence WHERE identity_key = ?",
    ).get(key);
    return row ? JSON.parse(row.adoption_json) : null;
  }
  close() { if (!this.closed) { this.closed = true; this.database.close(); } }
}

export function openPrivateSqliteDatabase(filePath, DatabaseSync, { beforeDatabaseOpen = null } = {}) {
  let descriptor;
  let expected;
  try {
    descriptor = openSync(filePath, constants.O_CREAT | constants.O_EXCL | constants.O_RDWR | constants.O_NOFOLLOW, 0o600);
    expected = fstatSync(descriptor);
  } catch (error) {
    if (error?.code !== "EEXIST") throw error;
    expected = lstatSync(filePath);
    if (!expected.isFile() || expected.isSymbolicLink()) throw new Error("SQLite slice-campaign path must be a regular file");
    chmodSync(filePath, 0o600);
  }
  if (!expected.isFile()) {
    if (descriptor !== undefined) closeSync(descriptor);
    throw new Error("SQLite slice-campaign path must be a regular file");
  }
  let database;
  try {
    if (beforeDatabaseOpen !== null) beforeDatabaseOpen({ filePath, stat: lstatSync(filePath) });
    database = new DatabaseSync(filePath);
    const observed = lstatSync(filePath);
    if (!observed.isFile() || observed.isSymbolicLink()
        || observed.dev !== expected.dev || observed.ino !== expected.ino) {
      database.close();
      throw new Error("SQLite slice-campaign path changed while opening");
    }
    return database;
  } finally {
    if (descriptor !== undefined) closeSync(descriptor);
  }
}

export async function openSqliteSliceCampaignStore({ filePath, busyTimeoutMs = 5_000 } = {}) {
  if (typeof filePath !== "string" || filePath.trim() === "") throw new TypeError("SQLite slice-campaign path must be nonempty");
  if (!Number.isInteger(busyTimeoutMs) || busyTimeoutMs < 1) throw new TypeError("SQLite busy timeout must be positive");
  const resolved = path.resolve(filePath);
  mkdirSync(path.dirname(resolved), { recursive: true, mode: 0o700 });
  const { DatabaseSync } = await import("node:sqlite");
  const database = openPrivateSqliteDatabase(resolved, DatabaseSync);
  try {
    database.exec("PRAGMA trusted_schema = OFF");
    database.exec(`PRAGMA busy_timeout = ${busyTimeoutMs}`);
    database.exec("PRAGMA journal_mode = WAL");
    database.exec("PRAGMA synchronous = FULL");
    database.exec(`CREATE TABLE IF NOT EXISTS slice_campaign_state (
      identity_key TEXT PRIMARY KEY, revision TEXT NOT NULL, state_json TEXT NOT NULL
    ) STRICT`);
    database.exec(`CREATE TABLE IF NOT EXISTS slice_campaign_admission (
      workspace TEXT PRIMARY KEY, identity_key TEXT NOT NULL UNIQUE,
      FOREIGN KEY(identity_key) REFERENCES slice_campaign_state(identity_key)
    ) STRICT`);
    database.exec("CREATE TABLE IF NOT EXISTS slice_campaign_external_bootstrap_evidence (identity_key TEXT PRIMARY KEY, campaign_revision TEXT NOT NULL, packet_digest TEXT NOT NULL, adoption_json TEXT NOT NULL, FOREIGN KEY(identity_key) REFERENCES slice_campaign_state(identity_key)) STRICT");
    database.exec("PRAGMA foreign_keys = ON");
    return new SqliteSliceCampaignStore(database, resolved);
  } catch (error) { database.close(); throw error; }
}
