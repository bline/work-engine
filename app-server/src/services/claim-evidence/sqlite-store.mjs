import { chmodSync, mkdirSync } from "node:fs";
import path from "node:path";

import { canonicalJson, digest } from "./identity.mjs";
import { validateObservation } from "./observation.mjs";
import { buildProjection } from "./projections.mjs";
import { applyOperation, blankStore } from "./service.mjs";
import { validateStore } from "./validation.mjs";

export const SQLITE_CLAIM_EVIDENCE_SCHEMA_VERSION = 2;

function nonemptyText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function positiveInteger(value, label) {
  if (!Number.isSafeInteger(value) || value < 1) {
    throw new TypeError(`${label} must be a positive safe integer`);
  }
  return value;
}

function hardenSqliteSidecars(filePath) {
  for (const suffix of ["-wal", "-shm"]) {
    try {
      chmodSync(`${filePath}${suffix}`, 0o600);
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
    }
  }
}

function canonicalStateFromRow(row) {
  if (!row) return null;
  let store;
  try {
    store = JSON.parse(row.store_json);
  } catch (error) {
    throw new TypeError(`stored claim-evidence state contains invalid JSON: ${error.message}`);
  }
  if (digest(store) !== row.store_sha256 || canonicalJson(store) !== row.store_json) {
    throw new TypeError("stored claim-evidence state failed its canonical integrity check");
  }
  validateStore(store);
  return { revision: Number(row.store_revision), store };
}

const MIGRATION_1 = `
  CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
  ) STRICT;

  CREATE TABLE canonical_claim_state (
    singleton_id INTEGER PRIMARY KEY CHECK(singleton_id = 1),
    store_revision INTEGER NOT NULL CHECK(store_revision > 0),
    store_sha256 TEXT NOT NULL CHECK(length(store_sha256) = 64),
    store_json TEXT NOT NULL
  ) STRICT;
`;

const MIGRATION_2 = `
  CREATE TABLE evidence_observations (
    event_identity TEXT PRIMARY KEY,
    observation_id TEXT NOT NULL UNIQUE,
    observation_sha256 TEXT NOT NULL CHECK(length(observation_sha256) = 64),
    observation_json TEXT NOT NULL
  ) STRICT;
`;

function observationFromRow(row) {
  if (!row) return null;
  let observation;
  try {
    observation = JSON.parse(row.observation_json);
  } catch (error) {
    throw new TypeError(`stored evidence observation contains invalid JSON: ${error.message}`);
  }
  if (canonicalJson(observation) !== row.observation_json
      || digest(observation) !== row.observation_sha256
      || observation.id !== row.observation_id
      || observation.event_identity !== row.event_identity) {
    throw new TypeError("stored evidence observation failed its canonical integrity check");
  }
  validateObservation(observation);
  return observation;
}

function migrate(database, filePath) {
  database.exec("BEGIN EXCLUSIVE");
  try {
    const current = Number(database.prepare("PRAGMA user_version").get().user_version);
    if (current > SQLITE_CLAIM_EVIDENCE_SCHEMA_VERSION) {
      throw new TypeError(
        `SQLite claim-evidence schema ${current} is newer than supported schema ${SQLITE_CLAIM_EVIDENCE_SCHEMA_VERSION}`,
      );
    }
    if (current < 1) {
      database.exec(MIGRATION_1);
      database.prepare("INSERT INTO schema_migrations(version, applied_at) VALUES (1, ?)")
        .run(new Date().toISOString());
      database.exec("PRAGMA user_version = 1");
    }
    if (current < 2) {
      database.exec(MIGRATION_2);
      database.prepare("INSERT INTO schema_migrations(version, applied_at) VALUES (2, ?)")
        .run(new Date().toISOString());
      database.exec("PRAGMA user_version = 2");
    }
    const versions = database.prepare("SELECT version FROM schema_migrations ORDER BY version")
      .all().map((row) => Number(row.version));
    if (versions.length !== SQLITE_CLAIM_EVIDENCE_SCHEMA_VERSION
        || versions.some((version, index) => version !== index + 1)) {
      throw new TypeError("SQLite claim-evidence migration history is incomplete");
    }
    hardenSqliteSidecars(filePath);
    database.exec("COMMIT");
  } catch (error) {
    try {
      database.exec("ROLLBACK");
    } catch {
      // The original migration failure owns the outcome.
    }
    throw error;
  }
}

class SqliteClaimEvidenceStore {
  constructor(database, filePath) {
    this.database = database;
    this.filePath = filePath;
    this.closed = false;
  }

  #assertOpen() {
    if (this.closed) throw new TypeError("SQLite claim-evidence store is closed");
  }

  #transaction(operation, mode = "IMMEDIATE") {
    this.#assertOpen();
    this.database.exec(`BEGIN ${mode}`);
    try {
      const result = operation();
      hardenSqliteSidecars(this.filePath);
      this.database.exec("COMMIT");
      return result;
    } catch (error) {
      try {
        this.database.exec("ROLLBACK");
      } catch {
        // The original failure owns the outcome.
      }
      throw error;
    }
  }

  #canonicalState() {
    return canonicalStateFromRow(this.database.prepare(`
      SELECT store_revision, store_sha256, store_json
      FROM canonical_claim_state WHERE singleton_id = 1
    `).get());
  }

  initialize(authorities) {
    if (!Array.isArray(authorities)) {
      throw new TypeError("claim-evidence bootstrap authorities must be an array");
    }
    return this.#transaction(() => {
      if (this.#canonicalState()) {
        throw new TypeError("claim-evidence authority bootstrap is only valid for an uninitialized database");
      }
      const store = blankStore(authorities);
      const storeJson = canonicalJson(store);
      this.database.prepare(`
        INSERT INTO canonical_claim_state(
          singleton_id, store_revision, store_sha256, store_json
        ) VALUES (1, 1, ?, ?)
      `).run(digest(store), storeJson);
      return structuredClone(store);
    });
  }

  publish(request, authority) {
    return this.#transaction(() => {
      const current = this.#canonicalState();
      if (!current) throw new TypeError("SQLite claim-evidence store is not initialized");
      const result = applyOperation(current.store, request, authority);
      if (result.idempotent) {
        return { ...result, store: structuredClone(result.store) };
      }
      const storeJson = canonicalJson(result.store);
      const updated = this.database.prepare(`
        UPDATE canonical_claim_state
        SET store_revision = ?, store_sha256 = ?, store_json = ?
        WHERE singleton_id = 1 AND store_revision = ?
      `).run(current.revision + 1, digest(result.store), storeJson, current.revision);
      if (Number(updated.changes) !== 1) {
        throw new TypeError("claim-evidence canonical state changed during publication");
      }
      return { ...result, store: structuredClone(result.store) };
    });
  }

  recordObservation(observation) {
    validateObservation(observation);
    return this.#transaction(() => {
      const observationJson = canonicalJson(observation);
      const observationSha256 = digest(observation);
      const priorEvent = this.database.prepare(`
        SELECT event_identity, observation_id, observation_sha256, observation_json
        FROM evidence_observations WHERE event_identity = ?
      `).get(observation.event_identity);
      if (priorEvent) {
        const prior = observationFromRow(priorEvent);
        if (prior.id !== observation.id
            || priorEvent.observation_sha256 !== observationSha256
            || priorEvent.observation_json !== observationJson) {
          throw new TypeError("evidence observation event identity conflict");
        }
        return { idempotent: true, observation_id: prior.id, observation: structuredClone(prior) };
      }
      const priorIdentity = this.database.prepare(`
        SELECT event_identity FROM evidence_observations WHERE observation_id = ?
      `).get(observation.id);
      if (priorIdentity) throw new TypeError("evidence observation identity reused by another event");
      this.database.prepare(`
        INSERT INTO evidence_observations(
          event_identity, observation_id, observation_sha256, observation_json
        ) VALUES (?, ?, ?, ?)
      `).run(observation.event_identity, observation.id, observationSha256, observationJson);
      return { idempotent: false, observation_id: observation.id, observation: structuredClone(observation) };
    });
  }

  readObservation(observationId) {
    this.#assertOpen();
    nonemptyText(observationId, "evidence observation identity");
    const row = this.database.prepare(`
      SELECT event_identity, observation_id, observation_sha256, observation_json
      FROM evidence_observations WHERE observation_id = ?
    `).get(observationId);
    return row ? structuredClone(observationFromRow(row)) : null;
  }

  listObservations({ limit = 100, afterEventIdentity = null } = {}) {
    this.#assertOpen();
    positiveInteger(limit, "evidence observation list limit");
    if (limit > 1_000) throw new TypeError("evidence observation list limit exceeds 1000");
    if (afterEventIdentity !== null) nonemptyText(afterEventIdentity, "evidence observation cursor");
    const rows = afterEventIdentity === null
      ? this.database.prepare(`
        SELECT event_identity, observation_id, observation_sha256, observation_json
        FROM evidence_observations ORDER BY event_identity, observation_id LIMIT ?
      `).all(limit)
      : this.database.prepare(`
        SELECT event_identity, observation_id, observation_sha256, observation_json
        FROM evidence_observations WHERE event_identity > ?
        ORDER BY event_identity, observation_id LIMIT ?
      `).all(afterEventIdentity, limit);
    return rows.map((row) => structuredClone(observationFromRow(row)));
  }

  exportStore() {
    this.#assertOpen();
    const current = this.#canonicalState();
    if (!current) throw new TypeError("SQLite claim-evidence store is not initialized");
    return structuredClone(current.store);
  }

  buildProjection(options = {}) {
    return buildProjection(this.exportStore(), options);
  }

  integrityCheck() {
    this.#assertOpen();
    const physical = this.database.prepare("PRAGMA integrity_check").all()
      .map((row) => row.integrity_check);
    this.exportStore();
    let afterEventIdentity = null;
    while (true) {
      const page = this.listObservations({ limit: 1_000, afterEventIdentity });
      if (page.length === 0) break;
      afterEventIdentity = page.at(-1).event_identity;
      if (page.length < 1_000) break;
    }
    return Object.freeze(physical);
  }

  close() {
    if (this.closed) return;
    this.database.close();
    this.closed = true;
  }
}

export async function openSqliteClaimEvidenceStore({
  filePath,
  bootstrapAuthorities,
  busyTimeoutMs = 5_000,
} = {}) {
  const resolvedPath = path.resolve(nonemptyText(filePath, "SQLite claim-evidence path"));
  positiveInteger(busyTimeoutMs, "SQLite claim-evidence busy timeout");
  mkdirSync(path.dirname(resolvedPath), { recursive: true, mode: 0o700 });
  const { DatabaseSync } = await import("node:sqlite");
  const database = new DatabaseSync(resolvedPath);
  const store = new SqliteClaimEvidenceStore(database, resolvedPath);
  try {
    chmodSync(resolvedPath, 0o600);
    database.exec("PRAGMA foreign_keys = ON");
    database.exec("PRAGMA trusted_schema = OFF");
    database.exec(`PRAGMA busy_timeout = ${busyTimeoutMs}`);
    database.exec("PRAGMA journal_mode = WAL");
    hardenSqliteSidecars(resolvedPath);
    database.exec("PRAGMA synchronous = FULL");
    migrate(database, resolvedPath);
    const initialized = database.prepare(
      "SELECT 1 AS present FROM canonical_claim_state WHERE singleton_id = 1",
    ).get();
    if (bootstrapAuthorities !== undefined) {
      if (initialized) {
        throw new TypeError("claim-evidence authority bootstrap is only valid for an uninitialized database");
      }
      store.initialize(bootstrapAuthorities);
    } else if (!initialized) {
      throw new TypeError("an uninitialized claim-evidence database requires explicit bootstrap authorities");
    }
    return store;
  } catch (error) {
    store.close();
    throw error;
  }
}
