import { chmodSync, closeSync, constants, fstatSync, lstatSync, mkdirSync, openSync } from "node:fs";
import path from "node:path";

function openPrivateDatabase(filePath, DatabaseSync) {
  let descriptor;
  let expected;
  try {
    descriptor = openSync(filePath, constants.O_CREAT | constants.O_EXCL | constants.O_RDWR | constants.O_NOFOLLOW, 0o600);
    expected = fstatSync(descriptor);
  } catch (error) {
    if (error?.code !== "EEXIST") throw error;
    expected = lstatSync(filePath);
    if (!expected.isFile() || expected.isSymbolicLink()) throw new Error("SQLite workspace-coordination path must be a regular file");
    chmodSync(filePath, 0o600);
  }
  if (!expected.isFile()) throw new Error("SQLite workspace-coordination path must be a regular file");
  try {
    const database = new DatabaseSync(filePath);
    const observed = lstatSync(filePath);
    if (!observed.isFile() || observed.isSymbolicLink() || observed.dev !== expected.dev || observed.ino !== expected.ino) {
      database.close(); throw new Error("SQLite workspace-coordination path changed while opening");
    }
    return database;
  } finally { if (descriptor !== undefined) closeSync(descriptor); }
}

export class SqliteWorkspaceCoordinationStore {
  constructor(database, filePath) { this.database = database; this.filePath = filePath; this.closed = false; }
  #transaction(operation) {
    if (this.closed) throw new Error("workspace coordination store is closed");
    this.database.exec("BEGIN IMMEDIATE");
    try { const result = operation(); this.database.exec("COMMIT"); return result; }
    catch (error) { try { this.database.exec("ROLLBACK"); } catch {} throw error; }
  }
  inspect(resourceKey) {
    const row = this.database.prepare("SELECT generation, lease_json FROM workspace_resource WHERE resource_key = ?").get(resourceKey);
    return row ? { generation: row.generation, lease: row.lease_json ? JSON.parse(row.lease_json) : null } : null;
  }
  acquire(request) {
    return this.#transaction(() => {
      const current = this.inspect(request.resource.key) ?? { generation: 0, lease: null };
      if (current.lease && Date.parse(current.lease.expiresAt) > Date.parse(request.issuedAt)) {
        return { status: "blocked", current: current.lease };
      }
      const lease = { ...request, fencingToken: current.generation + 1 };
      this.database.prepare(`INSERT INTO workspace_resource(resource_key, generation, lease_json)
        VALUES(?,?,?) ON CONFLICT(resource_key) DO UPDATE SET generation=excluded.generation, lease_json=excluded.lease_json`)
        .run(request.resource.key, lease.fencingToken, JSON.stringify(lease));
      return { status: "acquired", lease };
    });
  }
  release(lease) {
    return this.#transaction(() => this.database.prepare(`UPDATE workspace_resource SET lease_json = NULL
      WHERE resource_key = ? AND generation = ? AND json_extract(lease_json, '$.leaseId') = ?`)
      .run(lease.resource.key, lease.fencingToken, lease.leaseId).changes === 1);
  }
  withAdmission({ lease, operationId, admittedAt }, mutate) {
    return this.#transaction(() => {
      if (this.database.prepare("SELECT 1 FROM workspace_mutation_admission WHERE operation_id = ?").get(operationId)) {
        throw new Error("mutation operation id was already admitted");
      }
      const current = this.inspect(lease.resource.key);
      if (!current?.lease || current.generation !== lease.fencingToken
          || current.lease.leaseId !== lease.leaseId || current.lease.holder !== lease.holder) {
        throw new Error("resource lease is absent or superseded");
      }
      if (Date.parse(current.lease.expiresAt) <= Date.parse(admittedAt)) throw new Error("resource lease is expired");
      const result = mutate();
      if (result && typeof result.then === "function") throw new TypeError("mutation admission callback returned an asynchronous result");
      this.database.prepare(`INSERT INTO workspace_mutation_admission
        (operation_id, resource_key, fencing_token, lease_id, admitted_at, result_json) VALUES(?,?,?,?,?,?)`)
        .run(operationId, lease.resource.key, lease.fencingToken, lease.leaseId, admittedAt, JSON.stringify(result));
      return result;
    });
  }
  inspectAdmission(operationId) {
    const row = this.database.prepare(`SELECT resource_key, fencing_token, lease_id,
      admitted_at, result_json FROM workspace_mutation_admission WHERE operation_id = ?`).get(operationId);
    return row ? {
      operationId, resourceKey: row.resource_key, fencingToken: row.fencing_token,
      leaseId: row.lease_id, admittedAt: row.admitted_at, result: JSON.parse(row.result_json),
    } : null;
  }
  loadPublication(operationId) {
    const row = this.database.prepare(`SELECT revision, record_json
      FROM workspace_prepared_publication WHERE operation_id = ?`).get(operationId);
    return row ? { revision: row.revision, record: JSON.parse(row.record_json) } : null;
  }
  savePublication({ operationId, expectedRevision, revision, record }) {
    return this.#transaction(() => {
      const current = this.loadPublication(operationId);
      if (current?.revision === revision) return current;
      if ((current?.revision ?? null) !== expectedRevision) {
        throw new Error("prepared publication revision conflict");
      }
      if (current) {
        this.database.prepare(`UPDATE workspace_prepared_publication
          SET revision = ?, record_json = ? WHERE operation_id = ? AND revision = ?`)
          .run(revision, JSON.stringify(record), operationId, expectedRevision);
      } else {
        this.database.prepare(`INSERT INTO workspace_prepared_publication
          (operation_id, revision, record_json) VALUES(?,?,?)`)
          .run(operationId, revision, JSON.stringify(record));
      }
      return { revision, record };
    });
  }
  close() { if (!this.closed) { this.closed = true; this.database.close(); } }
}

export async function openSqliteWorkspaceCoordinationStore({ filePath, busyTimeoutMs = 5_000 } = {}) {
  if (typeof filePath !== "string" || filePath.trim() === "") throw new TypeError("SQLite workspace-coordination path must be nonempty");
  if (!Number.isInteger(busyTimeoutMs) || busyTimeoutMs < 1) throw new TypeError("SQLite busy timeout must be positive");
  const resolved = path.resolve(filePath); mkdirSync(path.dirname(resolved), { recursive: true, mode: 0o700 });
  const { DatabaseSync } = await import("node:sqlite");
  const database = openPrivateDatabase(resolved, DatabaseSync);
  try {
    database.exec("PRAGMA trusted_schema = OFF"); database.exec(`PRAGMA busy_timeout = ${busyTimeoutMs}`);
    database.exec("PRAGMA journal_mode = WAL"); database.exec("PRAGMA synchronous = FULL");
    database.exec(`CREATE TABLE IF NOT EXISTS workspace_resource (
      resource_key TEXT PRIMARY KEY, generation INTEGER NOT NULL, lease_json TEXT
    ) STRICT`);
    database.exec(`CREATE TABLE IF NOT EXISTS workspace_mutation_admission (
      operation_id TEXT PRIMARY KEY, resource_key TEXT NOT NULL, fencing_token INTEGER NOT NULL,
      lease_id TEXT NOT NULL, admitted_at TEXT NOT NULL, result_json TEXT NOT NULL
    ) STRICT`);
    database.exec(`CREATE TABLE IF NOT EXISTS workspace_prepared_publication (
      operation_id TEXT PRIMARY KEY, revision TEXT NOT NULL, record_json TEXT NOT NULL
    ) STRICT`);
    return new SqliteWorkspaceCoordinationStore(database, resolved);
  } catch (error) { database.close(); throw error; }
}
