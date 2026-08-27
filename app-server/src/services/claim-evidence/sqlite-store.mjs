import { chmodSync, mkdirSync } from "node:fs";
import path from "node:path";

import { canonicalJson, digest } from "./identity.mjs";
import { buildProjection } from "./projections.mjs";
import { applyOperation, blankStore } from "./service.mjs";
import { validateStore } from "./validation.mjs";

export const SQLITE_CLAIM_EVIDENCE_SCHEMA_VERSION = 1;

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
