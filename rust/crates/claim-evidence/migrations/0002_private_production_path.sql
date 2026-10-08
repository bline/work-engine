BEGIN EXCLUSIVE;
CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, name TEXT NOT NULL) STRICT;
INSERT INTO schema_migrations VALUES(2,'native-review-claims-sqlite-v2');
CREATE TABLE root_binding(singleton INTEGER PRIMARY KEY CHECK(singleton=1), marker_sha256 TEXT NOT NULL, database_dev TEXT NOT NULL, database_ino TEXT NOT NULL, schema_sha256 TEXT NOT NULL) STRICT;
CREATE TABLE canonical_claim_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1), store_revision INTEGER NOT NULL CHECK(store_revision>0), store_sha256 TEXT NOT NULL, store_json TEXT NOT NULL) STRICT;
CREATE TABLE operation_admissions(operation_id TEXT PRIMARY KEY, admission_sha256 TEXT NOT NULL, grant_sha256 TEXT NOT NULL) STRICT;
CREATE TABLE production_path_observations(event_identity TEXT PRIMARY KEY, observation_id TEXT NOT NULL UNIQUE, observation_sha256 TEXT NOT NULL, observation_json TEXT NOT NULL, request_sha256 TEXT NOT NULL, admission_sha256 TEXT NOT NULL, grant_id TEXT NOT NULL, grant_sha256 TEXT NOT NULL, custody_json TEXT NOT NULL, custody_sha256 TEXT NOT NULL) STRICT;
CREATE TABLE production_path_establishments(operation_id TEXT PRIMARY KEY, establishment_id TEXT NOT NULL UNIQUE, claim_revision TEXT NOT NULL, claim_sha256 TEXT NOT NULL, observation_id TEXT REFERENCES production_path_observations(observation_id), establishment_sha256 TEXT NOT NULL, establishment_json TEXT NOT NULL, request_sha256 TEXT NOT NULL, admission_sha256 TEXT NOT NULL, grant_id TEXT NOT NULL, grant_sha256 TEXT NOT NULL, custody_sha256 TEXT NOT NULL, custody_json TEXT NOT NULL) STRICT;
PRAGMA user_version=2;
