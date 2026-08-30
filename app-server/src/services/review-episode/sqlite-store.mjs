import { chmodSync, closeSync, constants, fstatSync, lstatSync, mkdirSync, openSync } from "node:fs";
import path from "node:path";

import { canonicalJson, digest, validateState } from "./contract.mjs";

export class SqliteReviewEpisodeStore {
  constructor(database) { this.database = database; this.closed = false; }
  #assertOpen() { if (this.closed) throw new Error("review episode store is closed"); }
  #decode(row) {
    if (!row) return null;
    const state = JSON.parse(row.state_json);
    if (canonicalJson(state) !== row.state_json || state.revision !== row.revision
        || digest(Object.fromEntries(Object.entries(state).filter(([key]) => key !== "revision"))) !== row.revision) {
      throw new Error("stored review episode state failed its integrity check");
    }
    validateState(Object.fromEntries(Object.entries(state).filter(([key]) => key !== "revision")));
    return state;
  }
  get(key) {
    this.#assertOpen();
    return this.#decode(this.database.prepare("SELECT revision, state_json FROM review_episode_current WHERE identity_key = ?").get(key));
  }
  put(key, state, expectedRevision) {
    this.#assertOpen(); this.database.exec("BEGIN IMMEDIATE");
    try {
      const current = this.database.prepare("SELECT revision FROM review_episode_current WHERE identity_key = ?").get(key);
      if ((current?.revision ?? null) !== expectedRevision) throw new Error("review episode revision conflict");
      const stateJson = canonicalJson(state);
      this.database.prepare("INSERT INTO review_episode_history(identity_key, revision, predecessor_revision, state_json) VALUES(?,?,?,?)")
        .run(key, state.revision, expectedRevision, stateJson);
      this.database.prepare(`INSERT INTO review_episode_current(identity_key, revision, state_json) VALUES(?,?,?)
        ON CONFLICT(identity_key) DO UPDATE SET revision=excluded.revision,state_json=excluded.state_json`)
        .run(key, state.revision, stateJson);
      this.database.exec("COMMIT");
    } catch (error) { try { this.database.exec("ROLLBACK"); } catch {} throw error; }
  }
  history(key) {
    this.#assertOpen();
    return this.database.prepare("SELECT revision, state_json FROM review_episode_history WHERE identity_key = ? ORDER BY sequence")
      .all(key).map((row) => this.#decode(row));
  }
  close() { if (!this.closed) { this.closed = true; this.database.close(); } }
}

function privateDatabase(filePath, DatabaseSync) {
  let descriptor; let expected;
  try {
    descriptor = openSync(filePath, constants.O_CREAT | constants.O_EXCL | constants.O_RDWR | constants.O_NOFOLLOW, 0o600);
    expected = fstatSync(descriptor);
  } catch (error) {
    if (error?.code !== "EEXIST") throw error;
    expected = lstatSync(filePath);
    if (!expected.isFile() || expected.isSymbolicLink()) throw new Error("SQLite review-episode path must be a regular file");
    chmodSync(filePath, 0o600);
  }
  if (!expected.isFile()) throw new Error("SQLite review-episode path must be a regular file");
  try {
    const database = new DatabaseSync(filePath);
    const observed = lstatSync(filePath);
    if (!observed.isFile() || observed.isSymbolicLink() || observed.dev !== expected.dev || observed.ino !== expected.ino) {
      database.close(); throw new Error("SQLite review-episode path changed while opening");
    }
    return database;
  } finally { if (descriptor !== undefined) closeSync(descriptor); }
}

export async function openSqliteReviewEpisodeStore({ filePath, busyTimeoutMs = 5_000 } = {}) {
  if (typeof filePath !== "string" || filePath.trim() === "") throw new TypeError("SQLite review-episode path must be nonempty");
  if (!Number.isSafeInteger(busyTimeoutMs) || busyTimeoutMs < 1) throw new TypeError("SQLite review-episode busy timeout must be positive");
  const resolved = path.resolve(filePath); mkdirSync(path.dirname(resolved), { recursive: true, mode: 0o700 });
  const { DatabaseSync } = await import("node:sqlite");
  const database = privateDatabase(resolved, DatabaseSync);
  try {
    database.exec("PRAGMA trusted_schema = OFF"); database.exec(`PRAGMA busy_timeout = ${busyTimeoutMs}`);
    database.exec("PRAGMA journal_mode = WAL"); database.exec("PRAGMA synchronous = FULL");
    database.exec(`CREATE TABLE IF NOT EXISTS review_episode_current (
      identity_key TEXT PRIMARY KEY, revision TEXT NOT NULL UNIQUE CHECK(length(revision)=64), state_json TEXT NOT NULL
    ) STRICT`);
    database.exec(`CREATE TABLE IF NOT EXISTS review_episode_history (
      sequence INTEGER PRIMARY KEY AUTOINCREMENT, identity_key TEXT NOT NULL, revision TEXT NOT NULL UNIQUE CHECK(length(revision)=64),
      predecessor_revision TEXT, state_json TEXT NOT NULL
    ) STRICT`);
    return new SqliteReviewEpisodeStore(database);
  } catch (error) { database.close(); throw error; }
}
