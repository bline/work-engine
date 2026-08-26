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
import { normalizeContextTransitionInput } from "./context-input-custody.mjs";

export const SQLITE_APP_SERVER_STATE_SCHEMA_VERSION = 2;

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

function inputAdmissionFromRow(row) {
  if (!row) return null;
  return freeze({
    logicalRoleInstanceId: row.logical_role_instance_id,
    threadId: row.thread_id,
    bindingRevision: row.binding_revision,
    transitionRevision: row.transition_revision,
    status: row.status,
    reconciliationRevision: row.reconciliation_revision,
    closedAt: row.closed_at,
    reopenedAt: row.reopened_at,
  });
}

function queuedInputFromRow(row) {
  if (!row) return null;
  const input = normalizeContextTransitionInput(
    parseJson(row.input_json, "stored context transition input"),
  );
  if (input.inputRevision !== row.input_revision
      || input.logicalRoleInstanceId !== row.logical_role_instance_id
      || input.clientUserMessageId !== row.client_user_message_id) {
    throw new TypeError("stored context transition input failed its integrity check");
  }
  return freeze({
    queueId: row.queue_id,
    sequence: row.sequence,
    transitionRevision: row.transition_revision,
    status: row.status,
    input,
    queuedAt: row.queued_at,
    releasedAt: row.released_at,
    delivery: row.delivery_json === null
      ? null
      : parseJson(row.delivery_json, "stored context input delivery"),
  });
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

const MIGRATION_2 = `
  CREATE TABLE context_input_admissions (
    logical_role_instance_id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL,
    binding_revision INTEGER NOT NULL CHECK(binding_revision > 0),
    transition_revision TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('closed', 'releasing', 'open')),
    reconciliation_revision TEXT,
    closed_at TEXT NOT NULL,
    reopened_at TEXT
  ) STRICT;

  CREATE TABLE context_input_queue (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    queue_id TEXT NOT NULL UNIQUE,
    logical_role_instance_id TEXT NOT NULL,
    transition_revision TEXT NOT NULL,
    client_user_message_id TEXT NOT NULL,
    input_revision TEXT NOT NULL,
    input_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('queued', 'releasing', 'released')),
    queued_at TEXT NOT NULL,
    released_at TEXT,
    delivery_json TEXT,
    UNIQUE(logical_role_instance_id, client_user_message_id)
  ) STRICT;

  CREATE INDEX context_input_queue_release_order
    ON context_input_queue(logical_role_instance_id, transition_revision, status, sequence);
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

  contextInputAdmission(logicalRoleInstanceId) {
    this.#assertOpen();
    text(logicalRoleInstanceId, "context input admission role");
    return inputAdmissionFromRow(this.database.prepare(`
      SELECT logical_role_instance_id, thread_id, binding_revision,
             transition_revision, status, reconciliation_revision,
             closed_at, reopened_at
      FROM context_input_admissions
      WHERE logical_role_instance_id = ?
    `).get(logicalRoleInstanceId));
  }

  closeContextInputAdmission({
    logicalRoleInstanceId,
    threadId,
    bindingRevision,
    transitionRevision,
    closedAt = new Date().toISOString(),
  }) {
    this.#assertOpen();
    text(logicalRoleInstanceId, "context input admission role");
    text(threadId, "context input admission thread");
    positiveInteger(bindingRevision, "context input admission binding revision");
    text(transitionRevision, "context input admission transition revision");
    text(closedAt, "context input admission close timestamp");
    if (Number.isNaN(Date.parse(closedAt))) {
      throw new TypeError("context input admission close timestamp must be ISO formatted");
    }
    return this.#transaction(() => {
      const existing = this.contextInputAdmission(logicalRoleInstanceId);
      if (existing && existing.status !== "open") {
        if (existing.threadId === threadId
            && existing.bindingRevision === bindingRevision
            && existing.transitionRevision === transitionRevision) {
          return freeze({ status: "replayed", admission: existing });
        }
        throw new TypeError("context input admission is already closed by another transition");
      }
      this.database.prepare(`
        INSERT INTO context_input_admissions (
          logical_role_instance_id, thread_id, binding_revision,
          transition_revision, status, reconciliation_revision,
          closed_at, reopened_at
        ) VALUES (?, ?, ?, ?, 'closed', NULL, ?, NULL)
        ON CONFLICT(logical_role_instance_id) DO UPDATE SET
          thread_id = excluded.thread_id,
          binding_revision = excluded.binding_revision,
          transition_revision = excluded.transition_revision,
          status = 'closed',
          reconciliation_revision = NULL,
          closed_at = excluded.closed_at,
          reopened_at = NULL
      `).run(
        logicalRoleInstanceId,
        threadId,
        bindingRevision,
        transitionRevision,
        closedAt,
      );
      return freeze({
        status: "closed",
        admission: this.contextInputAdmission(logicalRoleInstanceId),
      });
    });
  }

  queueContextInput(value, { queuedAt = new Date().toISOString() } = {}) {
    this.#assertOpen();
    const input = normalizeContextTransitionInput(value);
    text(queuedAt, "context input queue timestamp");
    if (Number.isNaN(Date.parse(queuedAt))) {
      throw new TypeError("context input queue timestamp must be ISO formatted");
    }
    return this.#transaction(() => {
      const admission = this.contextInputAdmission(input.logicalRoleInstanceId);
      if (!admission || admission.status === "open") return rejected("admission_open");
      if (admission.threadId !== input.threadId
          || admission.bindingRevision !== input.bindingRevision) {
        return rejected("stale_runtime_binding");
      }
      const existing = this.database.prepare(`
        SELECT sequence, queue_id, logical_role_instance_id, transition_revision,
               client_user_message_id, input_revision, input_json, status,
               queued_at, released_at, delivery_json
        FROM context_input_queue
        WHERE logical_role_instance_id = ? AND client_user_message_id = ?
      `).get(input.logicalRoleInstanceId, input.clientUserMessageId);
      if (existing) {
        if (existing.input_revision !== input.inputRevision
            || existing.transition_revision !== admission.transitionRevision) {
          throw new TypeError("context input client message id was reused for different input");
        }
        return freeze({ status: "replayed", item: queuedInputFromRow(existing) });
      }
      const queueId = `context-input:${input.inputRevision.slice("sha256:".length)}`;
      this.database.prepare(`
        INSERT INTO context_input_queue (
          queue_id, logical_role_instance_id, transition_revision,
          client_user_message_id, input_revision, input_json, status,
          queued_at, released_at, delivery_json
        ) VALUES (?, ?, ?, ?, ?, ?, 'queued', ?, NULL, NULL)
      `).run(
        queueId,
        input.logicalRoleInstanceId,
        admission.transitionRevision,
        input.clientUserMessageId,
        input.inputRevision,
        JSON.stringify(input),
        queuedAt,
      );
      const row = this.database.prepare(`
        SELECT sequence, queue_id, logical_role_instance_id, transition_revision,
               client_user_message_id, input_revision, input_json, status,
               queued_at, released_at, delivery_json
        FROM context_input_queue WHERE queue_id = ?
      `).get(queueId);
      return freeze({ status: "queued", item: queuedInputFromRow(row) });
    });
  }

  pendingContextInputs({ logicalRoleInstanceId, transitionRevision = null }) {
    this.#assertOpen();
    text(logicalRoleInstanceId, "context input queue role");
    if (transitionRevision !== null) {
      text(transitionRevision, "context input queue transition revision");
    }
    const sql = `
      SELECT sequence, queue_id, logical_role_instance_id, transition_revision,
             client_user_message_id, input_revision, input_json, status,
             queued_at, released_at, delivery_json
      FROM context_input_queue
      WHERE logical_role_instance_id = ?
        AND status != 'released'
        ${transitionRevision === null ? "" : "AND transition_revision = ?"}
      ORDER BY sequence
    `;
    const rows = transitionRevision === null
      ? this.database.prepare(sql).all(logicalRoleInstanceId)
      : this.database.prepare(sql).all(logicalRoleInstanceId, transitionRevision);
    return freeze(rows.map(queuedInputFromRow));
  }

  beginContextInputRelease({
    logicalRoleInstanceId,
    transitionRevision,
    reconciliationRevision,
  }) {
    this.#assertOpen();
    text(logicalRoleInstanceId, "context input release role");
    text(transitionRevision, "context input release transition revision");
    text(reconciliationRevision, "context input release reconciliation revision");
    return this.#transaction(() => {
      const admission = this.contextInputAdmission(logicalRoleInstanceId);
      if (!admission || admission.transitionRevision !== transitionRevision) {
        return rejected("transition_mismatch", admission);
      }
      if (admission.status === "open") {
        if (admission.reconciliationRevision !== reconciliationRevision) {
          return rejected("reconciliation_mismatch", admission);
        }
        return freeze({ status: "replayed", admission });
      }
      if (admission.reconciliationRevision !== null
          && admission.reconciliationRevision !== reconciliationRevision) {
        return rejected("reconciliation_mismatch", admission);
      }
      this.database.prepare(`
        UPDATE context_input_admissions
        SET status = 'releasing', reconciliation_revision = ?
        WHERE logical_role_instance_id = ? AND transition_revision = ?
      `).run(reconciliationRevision, logicalRoleInstanceId, transitionRevision);
      return freeze({
        status: admission.status === "releasing" ? "resuming" : "releasing",
        admission: this.contextInputAdmission(logicalRoleInstanceId),
      });
    });
  }

  nextContextInputForRelease({ logicalRoleInstanceId, transitionRevision }) {
    this.#assertOpen();
    text(logicalRoleInstanceId, "context input release role");
    text(transitionRevision, "context input release transition revision");
    return this.#transaction(() => {
      const admission = this.contextInputAdmission(logicalRoleInstanceId);
      if (!admission || admission.status !== "releasing"
          || admission.transitionRevision !== transitionRevision) {
        throw new TypeError("context input release requires the active releasing admission");
      }
      const row = this.database.prepare(`
        SELECT sequence, queue_id, logical_role_instance_id, transition_revision,
               client_user_message_id, input_revision, input_json, status,
               queued_at, released_at, delivery_json
        FROM context_input_queue
        WHERE logical_role_instance_id = ? AND transition_revision = ?
          AND status IN ('queued', 'releasing')
        ORDER BY sequence LIMIT 1
      `).get(logicalRoleInstanceId, transitionRevision);
      if (!row) return null;
      if (row.status === "queued") {
        this.database.prepare(`
          UPDATE context_input_queue SET status = 'releasing'
          WHERE queue_id = ? AND status = 'queued'
        `).run(row.queue_id);
        row.status = "releasing";
      }
      return queuedInputFromRow(row);
    });
  }

  completeContextInputRelease({
    queueId,
    inputRevision,
    delivery,
    releasedAt = new Date().toISOString(),
  }) {
    this.#assertOpen();
    text(queueId, "context input queue id");
    text(inputRevision, "context input revision");
    if (!delivery || typeof delivery !== "object" || Array.isArray(delivery)) {
      throw new TypeError("context input delivery receipt must be an object");
    }
    text(releasedAt, "context input release timestamp");
    if (Number.isNaN(Date.parse(releasedAt))) {
      throw new TypeError("context input release timestamp must be ISO formatted");
    }
    return this.#transaction(() => {
      const row = this.database.prepare(`
        SELECT sequence, queue_id, logical_role_instance_id, transition_revision,
               client_user_message_id, input_revision, input_json, status,
               queued_at, released_at, delivery_json
        FROM context_input_queue WHERE queue_id = ?
      `).get(queueId);
      if (!row || row.input_revision !== inputRevision) {
        return rejected("queued_input_mismatch");
      }
      if (row.status === "released") {
        if (row.delivery_json !== JSON.stringify(delivery)) {
          throw new TypeError("released context input has a different delivery receipt");
        }
        return freeze({ status: "replayed", item: queuedInputFromRow(row) });
      }
      if (row.status !== "releasing") {
        throw new TypeError("context input must be claimed before release completion");
      }
      this.database.prepare(`
        UPDATE context_input_queue
        SET status = 'released', released_at = ?, delivery_json = ?
        WHERE queue_id = ? AND status = 'releasing'
      `).run(releasedAt, JSON.stringify(delivery), queueId);
      const completed = this.database.prepare(`
        SELECT sequence, queue_id, logical_role_instance_id, transition_revision,
               client_user_message_id, input_revision, input_json, status,
               queued_at, released_at, delivery_json
        FROM context_input_queue WHERE queue_id = ?
      `).get(queueId);
      return freeze({ status: "released", item: queuedInputFromRow(completed) });
    });
  }

  reopenContextInputAdmission({
    logicalRoleInstanceId,
    transitionRevision,
    reconciliationRevision,
    reopenedAt = new Date().toISOString(),
  }) {
    this.#assertOpen();
    text(logicalRoleInstanceId, "context input admission role");
    text(transitionRevision, "context input admission transition revision");
    text(reconciliationRevision, "context input admission reconciliation revision");
    text(reopenedAt, "context input admission reopen timestamp");
    if (Number.isNaN(Date.parse(reopenedAt))) {
      throw new TypeError("context input admission reopen timestamp must be ISO formatted");
    }
    return this.#transaction(() => {
      const admission = this.contextInputAdmission(logicalRoleInstanceId);
      if (!admission || admission.transitionRevision !== transitionRevision
          || admission.reconciliationRevision !== reconciliationRevision) {
        return rejected("transition_or_reconciliation_mismatch", admission);
      }
      if (admission.status === "open") return freeze({ status: "replayed", admission });
      if (admission.status !== "releasing") {
        return rejected("release_not_started", admission);
      }
      const pending = this.database.prepare(`
        SELECT COUNT(*) AS count FROM context_input_queue
        WHERE logical_role_instance_id = ? AND transition_revision = ?
          AND status != 'released'
      `).get(logicalRoleInstanceId, transitionRevision).count;
      if (Number(pending) !== 0) return rejected("queued_inputs_pending", admission);
      this.database.prepare(`
        UPDATE context_input_admissions
        SET status = 'open', reopened_at = ?
        WHERE logical_role_instance_id = ? AND transition_revision = ?
      `).run(reopenedAt, logicalRoleInstanceId, transitionRevision);
      return freeze({
        status: "open",
        admission: this.contextInputAdmission(logicalRoleInstanceId),
      });
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
    if (current < 2) {
      database.exec(MIGRATION_2);
      database.prepare(`
        INSERT INTO schema_migrations(version, applied_at) VALUES (2, ?)
      `).run(new Date().toISOString());
      database.exec("PRAGMA user_version = 2");
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
