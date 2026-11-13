//! Schema. The `domain` column and its CHECKs and composite foreign keys are the storage-level
//! half of the fixture boundary: a fixture row can never satisfy an ordinary reference.

pub const SCHEMA_VERSION: i64 = 1;
pub const FIXTURE_LABEL: &str = "FIXTURE EXPERIMENT RESULT: not an independently accepted organizational review and not a qualified production execution";
pub const FIXTURE_SCOPE: &str = "fixture:publish-fixture-experiment";

pub const DDL: &str = "
CREATE TABLE meta(store_id TEXT NOT NULL, schema_version INTEGER NOT NULL);
CREATE TABLE log(seq INTEGER PRIMARY KEY AUTOINCREMENT, kind TEXT NOT NULL, ref TEXT NOT NULL);
CREATE TABLE org_state(
  domain TEXT PRIMARY KEY CHECK(domain IN ('ordinary','fixture')),
  org_revision INTEGER NOT NULL, requirements_digest TEXT NOT NULL,
  acceptance_ref TEXT NOT NULL, candidate_digest TEXT NOT NULL,
  provenance TEXT NOT NULL,
  CHECK(domain <> 'fixture' OR provenance = 'fixture-only'));
CREATE TABLE admissions(
  admission_id TEXT PRIMARY KEY,
  domain TEXT NOT NULL CHECK(domain IN ('ordinary','fixture')),
  provenance TEXT NOT NULL,
  org_revision INTEGER NOT NULL, requirements_digest TEXT NOT NULL,
  candidate_digest TEXT NOT NULL, binding_digest TEXT NOT NULL,
  report_json TEXT NOT NULL, unestablished_json TEXT NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('admitted','withdrawn')),
  seq INTEGER NOT NULL,
  UNIQUE(admission_id, domain),
  CHECK(domain <> 'fixture' OR provenance = 'fixture-only'));
CREATE TABLE grants(
  grant_id TEXT PRIMARY KEY, admission_id TEXT NOT NULL,
  domain TEXT NOT NULL CHECK(domain IN ('ordinary','fixture')),
  provenance TEXT NOT NULL, scope TEXT NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('active','revoked')),
  revision INTEGER NOT NULL, revoked_seq INTEGER,
  UNIQUE(grant_id, domain),
  FOREIGN KEY(admission_id, domain) REFERENCES admissions(admission_id, domain),
  CHECK(domain <> 'fixture' OR (provenance = 'fixture-only' AND scope LIKE 'fixture:%')));
CREATE TABLE executions(
  execution_id TEXT PRIMARY KEY, admission_id TEXT NOT NULL,
  domain TEXT NOT NULL CHECK(domain IN ('ordinary','fixture')),
  obligation_id TEXT NOT NULL, job_id TEXT NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('prepared','finished','unknown')),
  host_receipt_digest TEXT,
  UNIQUE(admission_id, obligation_id), UNIQUE(execution_id, domain),
  FOREIGN KEY(admission_id, domain) REFERENCES admissions(admission_id, domain));
CREATE TABLE operations(
  operation_id TEXT PRIMARY KEY, request_digest TEXT NOT NULL,
  outcome TEXT NOT NULL CHECK(outcome IN ('published','refused')),
  reason TEXT, result_id TEXT, seq INTEGER NOT NULL UNIQUE);
CREATE TABLE results(
  result_id TEXT PRIMARY KEY,
  operation_id TEXT NOT NULL UNIQUE REFERENCES operations(operation_id),
  domain TEXT NOT NULL CHECK(domain = 'fixture'),
  provenance TEXT NOT NULL CHECK(provenance = 'fixture-only'),
  result_class TEXT NOT NULL CHECK(result_class = 'fixture_experiment'),
  attestation TEXT NOT NULL CHECK(attestation = 'none'),
  label TEXT NOT NULL CHECK(label = 'FIXTURE EXPERIMENT RESULT: not an independently accepted organizational review and not a qualified production execution'),
  execution_id TEXT NOT NULL UNIQUE, admission_id TEXT NOT NULL,
  grant_id TEXT NOT NULL, grant_revision INTEGER NOT NULL,
  org_revision INTEGER NOT NULL, candidate_digest TEXT NOT NULL,
  requirements_digest TEXT NOT NULL, obligation_id TEXT NOT NULL,
  body BLOB NOT NULL CHECK(length(body) <= 65536),
  unestablished_json TEXT NOT NULL CHECK(length(unestablished_json) > 2),
  seq INTEGER NOT NULL,
  FOREIGN KEY(execution_id, domain) REFERENCES executions(execution_id, domain),
  FOREIGN KEY(grant_id, domain) REFERENCES grants(grant_id, domain),
  FOREIGN KEY(admission_id, domain) REFERENCES admissions(admission_id, domain));
CREATE TABLE receipts(
  receipt_id TEXT PRIMARY KEY,
  result_id TEXT NOT NULL UNIQUE REFERENCES results(result_id),
  bytes BLOB NOT NULL CHECK(length(bytes) <= 1048576));
CREATE TRIGGER results_immutable_u BEFORE UPDATE ON results BEGIN SELECT RAISE(ABORT,'results are immutable'); END;
CREATE TRIGGER results_immutable_d BEFORE DELETE ON results BEGIN SELECT RAISE(ABORT,'results are immutable'); END;
CREATE TRIGGER receipts_immutable_u BEFORE UPDATE ON receipts BEGIN SELECT RAISE(ABORT,'receipts are immutable'); END;
CREATE TRIGGER receipts_immutable_d BEFORE DELETE ON receipts BEGIN SELECT RAISE(ABORT,'receipts are immutable'); END;
";
