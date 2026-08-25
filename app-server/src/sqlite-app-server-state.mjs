import { chmodSync, mkdirSync } from "node:fs";
import path from "node:path";

import {
  validateContextCheckpointFence,
  validateContextCheckpointPublicationAttempt,
  verifyContextCheckpointPublication,
} from "./context-checkpoint-publication.mjs";
import {
  verifyLifecycleLedgerEntryIntegrity,
} from "./context-lifecycle-ledger.mjs";
import {
  verifyContextLifecycleEpisode,
} from "./context-lifecycle-episode.mjs";

export const SQLITE_APP_SERVER_STATE_SCHEMA_VERSION = 1;

function text(value, label) {
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

function parseJson(value, label) {
  try {
    return JSON.parse(value);
  } catch (error) {
    throw new TypeError(`${label} contains invalid JSON: ${error.message}`);
  }
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

function rejected(reason, currentFence = null) {
  return freeze({
    status: "rejected",
    reason,
    ...(currentFence ? { currentFence } : {}),
  });
}

function rowFence(row) {
  if (!row) return null;
  return freeze({
    logicalRoleInstanceId: row.logical_role_instance_id,
    threadId: row.thread_id,
    bindingRevision: row.binding_revision,
    sourceRevision: row.source_revision,
    authorityRevision: row.authority_revision,
    publicationRevision: row.publication_revision,
    ledgerRevision: row.ledger_revision,
  });
}

function sameFence(left, right) {
  return [
    "logicalRoleInstanceId", "threadId", "bindingRevision", "sourceRevision",
    "authorityRevision", "publicationRevision", "ledgerRevision",
  ].every((field) => left[field] === right[field]);
}

function episodeFromRow(row) {
  const episode = parseJson(row.payload_json, "stored lifecycle episode");
  if (!verifyContextLifecycleEpisode(episode)
      || episode.episodeId !== row.episode_id
      || episode.subject.logicalRoleInstanceId !== row.logical_role_instance_id
      || episode.pressure.policyRevision !== row.policy_revision
      || episode.scheduleRevision !== row.schedule_revision
      || episode.completedAt !== row.completed_at
      || episode.requestRevision !== row.request_revision
      || episode.episodeRevision !== row.episode_revision) {
    throw new TypeError("stored lifecycle episode failed its integrity check");
  }
  return episode;
}

const MIGRATION_1 = `
  CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
  ) STRICT;

  CREATE TABLE lifecycle_episodes (
    episode_id TEXT PRIMARY KEY,
    logical_role_instance_id TEXT NOT NULL,
    policy_revision TEXT NOT NULL,
    schedule_revision TEXT NOT NULL,
    completed_at TEXT NOT NULL,
    request_revision TEXT NOT NULL,
    episode_revision TEXT NOT NULL UNIQUE,
    payload_json TEXT NOT NULL
  ) STRICT;

  CREATE INDEX lifecycle_episodes_role_completed
    ON lifecycle_episodes(logical_role_instance_id, completed_at, episode_id);
  CREATE INDEX lifecycle_episodes_policy_schedule
    ON lifecycle_episodes(policy_revision, schedule_revision);

  CREATE TABLE checkpoint_fences (
    logical_role_instance_id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL,
    binding_revision INTEGER NOT NULL CHECK(binding_revision > 0),
    source_revision TEXT NOT NULL,
    authority_revision TEXT NOT NULL,
    publication_revision TEXT,
    ledger_revision TEXT,
    updated_at TEXT NOT NULL
  ) STRICT;

  CREATE TABLE checkpoint_publications (
    checkpoint_revision TEXT PRIMARY KEY,
    logical_role_instance_id TEXT NOT NULL,
    candidate_revision TEXT NOT NULL,
    ledger_revision TEXT NOT NULL UNIQUE,
    published_at TEXT NOT NULL,
    publication_json TEXT NOT NULL,
    ledger_json TEXT NOT NULL,
    UNIQUE(logical_role_instance_id, candidate_revision),
    FOREIGN KEY(logical_role_instance_id)
      REFERENCES checkpoint_fences(logical_role_instance_id)
  ) STRICT;

  CREATE TABLE lifecycle_ledger_heads (
    logical_role_instance_id TEXT PRIMARY KEY,
    entry_revision TEXT NOT NULL UNIQUE,
    sequence INTEGER NOT NULL CHECK(sequence > 0),
    entry_json TEXT NOT NULL,
    FOREIGN KEY(logical_role_instance_id)
      REFERENCES checkpoint_fences(logical_role_instance_id)
  ) STRICT;
`;

class SqliteAppServerStateStore {
  constructor(database, filePath) {
    this.database = database;
    this.filePath = filePath;
    this.closed = false;
  }

  #assertOpen() {
    if (this.closed) throw new TypeError("SQLite App Server state store is closed");
  }

  #transaction(operation) {
    this.#assertOpen();
    this.database.exec("BEGIN IMMEDIATE");
    try {
      const result = operation();
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

  #fence(logicalRoleInstanceId) {
    return rowFence(this.database.prepare(`
      SELECT logical_role_instance_id, thread_id, binding_revision,
             source_revision, authority_revision, publication_revision,
             ledger_revision
      FROM checkpoint_fences
      WHERE logical_role_instance_id = ?
    `).get(logicalRoleInstanceId));
  }

  close() {
    if (this.closed) return;
    this.database.close();
    this.closed = true;
  }

  integrityCheck() {
    this.#assertOpen();
    const rows = this.database.prepare("PRAGMA integrity_check").all();
    return Object.freeze(rows.map((row) => row.integrity_check));
  }

  get(episodeId) {
    this.#assertOpen();
    text(episodeId, "lifecycle episode id");
    const row = this.database.prepare(`
      SELECT episode_id, logical_role_instance_id, policy_revision,
             schedule_revision, completed_at, request_revision,
             episode_revision, payload_json
      FROM lifecycle_episodes WHERE episode_id = ?
    `).get(episodeId);
    if (!row) return null;
    return episodeFromRow(row);
  }

  append(episode) {
    this.#assertOpen();
    if (!verifyContextLifecycleEpisode(episode)) {
      throw new TypeError("episode store requires an integrity-valid lifecycle episode");
    }
    return this.#transaction(() => {
      const existing = this.database.prepare(`
        SELECT episode_id, logical_role_instance_id, policy_revision,
               schedule_revision, completed_at, request_revision,
               episode_revision, payload_json
        FROM lifecycle_episodes
        WHERE episode_id = ?
      `).get(episode.episodeId);
      if (existing) {
        if (existing.episode_revision !== episode.episodeRevision) {
          throw new TypeError("lifecycle episode id was reused for different evidence");
        }
        const stored = episodeFromRow(existing);
        return freeze({ status: "replayed", episode: stored });
      }
      this.database.prepare(`
        INSERT INTO lifecycle_episodes (
          episode_id, logical_role_instance_id, policy_revision, schedule_revision,
          completed_at, request_revision, episode_revision, payload_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      `).run(
        episode.episodeId,
        episode.subject.logicalRoleInstanceId,
        episode.pressure.policyRevision,
        episode.scheduleRevision,
        episode.completedAt,
        episode.requestRevision,
        episode.episodeRevision,
        JSON.stringify(episode),
      );
      return freeze({ status: "appended", episode });
    });
  }

  receipts({ logicalRoleInstanceId = null } = {}) {
    this.#assertOpen();
    if (logicalRoleInstanceId !== null) {
      text(logicalRoleInstanceId, "logical role instance id");
    }
    const rows = logicalRoleInstanceId === null
      ? this.database.prepare(`
          SELECT episode_id, logical_role_instance_id, policy_revision,
                 schedule_revision, completed_at, request_revision,
                 episode_revision, payload_json
          FROM lifecycle_episodes ORDER BY completed_at, episode_id
        `).all()
      : this.database.prepare(`
          SELECT episode_id, logical_role_instance_id, policy_revision,
                 schedule_revision, completed_at, request_revision,
                 episode_revision, payload_json
          FROM lifecycle_episodes
          WHERE logical_role_instance_id = ?
          ORDER BY completed_at, episode_id
        `).all(logicalRoleInstanceId);
    return Object.freeze(rows.map(episodeFromRow));
  }

  initializeCheckpointFence(value, { initializedAt = new Date().toISOString() } = {}) {
    this.#assertOpen();
    const fence = validateContextCheckpointFence(value);
    text(initializedAt, "checkpoint fence initialization timestamp");
    if (Number.isNaN(Date.parse(initializedAt))) {
      throw new TypeError("checkpoint fence initialization timestamp must be ISO formatted");
    }
    if (fence.publicationRevision !== null || fence.ledgerRevision !== null) {
      throw new TypeError("new SQLite checkpoint fences cannot begin with publication state");
    }
    return this.#transaction(() => {
      const existing = this.#fence(fence.logicalRoleInstanceId);
      if (existing) {
        if (!sameFence(existing, fence)) {
          throw new TypeError("checkpoint fence already exists with different evidence");
        }
        return freeze({ status: "replayed", fence: existing });
      }
      this.database.prepare(`
        INSERT INTO checkpoint_fences (
          logical_role_instance_id, thread_id, binding_revision, source_revision,
          authority_revision, publication_revision, ledger_revision, updated_at
        ) VALUES (?, ?, ?, ?, ?, NULL, NULL, ?)
      `).run(
        fence.logicalRoleInstanceId,
        fence.threadId,
        fence.bindingRevision,
        fence.sourceRevision,
        fence.authorityRevision,
        initializedAt,
      );
      return freeze({ status: "initialized", fence });
    });
  }

  compareAndSwapCheckpointFence({
    expectedFence,
    nextFence,
    updatedAt = new Date().toISOString(),
  }) {
    this.#assertOpen();
    const expected = validateContextCheckpointFence(expectedFence);
    const next = validateContextCheckpointFence(nextFence);
    text(updatedAt, "checkpoint fence update timestamp");
    if (Number.isNaN(Date.parse(updatedAt))) {
      throw new TypeError("checkpoint fence update timestamp must be ISO formatted");
    }
    if (next.logicalRoleInstanceId !== expected.logicalRoleInstanceId) {
      throw new TypeError("checkpoint fence update cannot change logical role identity");
    }
    if (next.publicationRevision !== expected.publicationRevision
        || next.ledgerRevision !== expected.ledgerRevision) {
      throw new TypeError("checkpoint fence update cannot rewrite publication or ledger heads");
    }
    return this.#transaction(() => {
      const current = this.#fence(expected.logicalRoleInstanceId);
      if (!current) return rejected("missing_lifecycle_fence");
      if (!sameFence(current, expected)) return rejected("fence_conflict", current);
      const update = this.database.prepare(`
        UPDATE checkpoint_fences
        SET thread_id = ?, binding_revision = ?, source_revision = ?,
            authority_revision = ?, updated_at = ?
        WHERE logical_role_instance_id = ?
          AND thread_id = ?
          AND binding_revision = ?
          AND source_revision = ?
          AND authority_revision = ?
          AND publication_revision IS ?
          AND ledger_revision IS ?
      `).run(
        next.threadId,
        next.bindingRevision,
        next.sourceRevision,
        next.authorityRevision,
        updatedAt,
        expected.logicalRoleInstanceId,
        expected.threadId,
        expected.bindingRevision,
        expected.sourceRevision,
        expected.authorityRevision,
        expected.publicationRevision,
        expected.ledgerRevision,
      );
      if (Number(update.changes) !== 1) {
        throw new TypeError("checkpoint fence changed inside its SQLite transaction");
      }
      return freeze({ status: "committed", currentFence: this.#fence(expected.logicalRoleInstanceId) });
    });
  }

  snapshot(logicalRoleInstanceId) {
    this.#assertOpen();
    text(logicalRoleInstanceId, "logical role instance id");
    const fence = this.#fence(logicalRoleInstanceId);
    if (!fence) return null;
    if (fence.publicationRevision === null) {
      if (fence.ledgerRevision !== null) {
        throw new TypeError("stored checkpoint fence has a ledger head without a publication");
      }
      return freeze({ fence, publication: null, ledgerEntry: null });
    }
    const publicationRow = this.database.prepare(`
      SELECT checkpoint_revision, logical_role_instance_id, candidate_revision,
             ledger_revision, published_at, publication_json, ledger_json
      FROM checkpoint_publications
      WHERE checkpoint_revision = ? AND logical_role_instance_id = ?
    `).get(fence.publicationRevision, logicalRoleInstanceId);
    const headRow = this.database.prepare(`
      SELECT entry_revision, sequence, entry_json
      FROM lifecycle_ledger_heads
      WHERE logical_role_instance_id = ?
    `).get(logicalRoleInstanceId);
    if (!publicationRow || !headRow || headRow.entry_revision !== fence.ledgerRevision) {
      throw new TypeError("stored checkpoint publication or ledger head is incomplete");
    }
    const publication = parseJson(publicationRow.publication_json, "stored checkpoint publication");
    const ledgerEntry = parseJson(headRow.entry_json, "stored lifecycle ledger head");
    const publicationLedgerEntry = parseJson(
      publicationRow.ledger_json,
      "stored checkpoint publication ledger entry",
    );
    if (!verifyContextCheckpointPublication(publication)
        || !verifyLifecycleLedgerEntryIntegrity(ledgerEntry)
        || !verifyLifecycleLedgerEntryIntegrity(publicationLedgerEntry)
        || publicationRow.checkpoint_revision !== publication.checkpointRevision
        || publicationRow.logical_role_instance_id !== publication.subject.logicalRoleInstanceId
        || publicationRow.candidate_revision !== publication.subject.candidateRevision
        || publicationRow.ledger_revision !== publicationLedgerEntry.entryRevision
        || publicationRow.published_at !== publication.publishedAt
        || headRow.sequence !== ledgerEntry.sequence
        || publication.checkpointRevision !== fence.publicationRevision
        || ledgerEntry.entryRevision !== fence.ledgerRevision
        || publicationLedgerEntry.entryRevision !== ledgerEntry.entryRevision) {
      throw new TypeError("stored checkpoint publication failed its integrity check");
    }
    return freeze({ fence, publication, ledgerEntry });
  }

  compareAndSwapPublication(input) {
    this.#assertOpen();
    const {
      expected,
      publication,
      ledgerEntry,
    } = validateContextCheckpointPublicationAttempt(input);
    return this.#transaction(() => {
      const current = this.#fence(expected.logicalRoleInstanceId);
      if (!current) return rejected("missing_lifecycle_fence");
      for (const [field, reason] of [
        ["threadId", "stale_runtime_binding"],
        ["bindingRevision", "stale_runtime_binding"],
        ["sourceRevision", "stale_source_revision"],
        ["authorityRevision", "stale_authority_revision"],
        ["publicationRevision", "publication_conflict"],
        ["ledgerRevision", "ledger_conflict"],
      ]) {
        if (current[field] !== expected[field]) return rejected(reason, current);
      }
      const duplicate = this.database.prepare(`
        SELECT checkpoint_revision FROM checkpoint_publications
        WHERE logical_role_instance_id = ? AND candidate_revision = ?
      `).get(expected.logicalRoleInstanceId, publication.subject.candidateRevision);
      if (duplicate) return rejected("duplicate_candidate", current);

      this.database.prepare(`
        INSERT INTO checkpoint_publications (
          checkpoint_revision, logical_role_instance_id, candidate_revision,
          ledger_revision, published_at, publication_json, ledger_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
      `).run(
        publication.checkpointRevision,
        expected.logicalRoleInstanceId,
        publication.subject.candidateRevision,
        ledgerEntry.entryRevision,
        publication.publishedAt,
        JSON.stringify(publication),
        JSON.stringify(ledgerEntry),
      );
      this.database.prepare(`
        INSERT INTO lifecycle_ledger_heads (
          logical_role_instance_id, entry_revision, sequence, entry_json
        ) VALUES (?, ?, ?, ?)
        ON CONFLICT(logical_role_instance_id) DO UPDATE SET
          entry_revision = excluded.entry_revision,
          sequence = excluded.sequence,
          entry_json = excluded.entry_json
      `).run(
        expected.logicalRoleInstanceId,
        ledgerEntry.entryRevision,
        ledgerEntry.sequence,
        JSON.stringify(ledgerEntry),
      );
      const update = this.database.prepare(`
        UPDATE checkpoint_fences
        SET publication_revision = ?, ledger_revision = ?, updated_at = ?
        WHERE logical_role_instance_id = ?
          AND thread_id = ?
          AND binding_revision = ?
          AND source_revision = ?
          AND authority_revision = ?
          AND publication_revision IS ?
          AND ledger_revision IS ?
      `).run(
        publication.checkpointRevision,
        ledgerEntry.entryRevision,
        publication.publishedAt,
        expected.logicalRoleInstanceId,
        expected.threadId,
        expected.bindingRevision,
        expected.sourceRevision,
        expected.authorityRevision,
        expected.publicationRevision,
        expected.ledgerRevision,
      );
      if (Number(update.changes) !== 1) {
        throw new TypeError("checkpoint fence changed inside its SQLite transaction");
      }
      return freeze({ status: "committed", currentFence: this.#fence(expected.logicalRoleInstanceId) });
    });
  }
}

function migrate(database) {
  database.exec("BEGIN EXCLUSIVE");
  try {
    const current = Number(database.prepare("PRAGMA user_version").get().user_version);
    if (current > SQLITE_APP_SERVER_STATE_SCHEMA_VERSION) {
      throw new TypeError(
        `SQLite App Server state schema ${current} is newer than supported schema ${SQLITE_APP_SERVER_STATE_SCHEMA_VERSION}`,
      );
    }
    if (current < 1) {
      database.exec(MIGRATION_1);
      database.prepare(`
        INSERT INTO schema_migrations(version, applied_at) VALUES (1, ?)
      `).run(new Date().toISOString());
      database.exec("PRAGMA user_version = 1");
    }
    const versions = database.prepare(`
      SELECT version FROM schema_migrations ORDER BY version
    `).all().map((row) => row.version);
    if (versions.length !== SQLITE_APP_SERVER_STATE_SCHEMA_VERSION
        || versions.some((version, index) => version !== index + 1)) {
      throw new TypeError("SQLite App Server state migration history is incomplete");
    }
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

export async function openSqliteAppServerStateStore({
  filePath,
  busyTimeoutMs = 5_000,
} = {}) {
  const resolvedPath = path.resolve(text(filePath, "SQLite App Server state path"));
  positiveInteger(busyTimeoutMs, "SQLite App Server state busy timeout");
  mkdirSync(path.dirname(resolvedPath), { recursive: true, mode: 0o700 });
  const { DatabaseSync } = await import("node:sqlite");
  const database = new DatabaseSync(resolvedPath);
  try {
    database.exec("PRAGMA foreign_keys = ON");
    database.exec("PRAGMA trusted_schema = OFF");
    database.exec(`PRAGMA busy_timeout = ${busyTimeoutMs}`);
    database.exec("PRAGMA journal_mode = WAL");
    database.exec("PRAGMA synchronous = FULL");
    migrate(database);
    chmodSync(resolvedPath, 0o600);
    return new SqliteAppServerStateStore(database, resolvedPath);
  } catch (error) {
    database.close();
    throw error;
  }
}
