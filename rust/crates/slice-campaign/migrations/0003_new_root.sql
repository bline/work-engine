-- Schema 3 is initialized only in a fresh private campaign root. No old-root
-- migration is performed or accepted by the writer.
CREATE TABLE store_identity (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    root_id TEXT NOT NULL,
    anchored_root_sha256 TEXT NOT NULL CHECK(length(anchored_root_sha256) = 64),
    profile TEXT NOT NULL,
    trusted_config_digest TEXT NOT NULL CHECK(length(trusted_config_digest) = 64),
    writer_identity TEXT NOT NULL
) STRICT;
CREATE TABLE campaign_state (
    identity_key TEXT PRIMARY KEY,
    revision TEXT NOT NULL CHECK(length(revision) = 64),
    state_json TEXT NOT NULL
) STRICT;
CREATE TABLE workspace_reservation (
    workspace TEXT PRIMARY KEY,
    identity_key TEXT NOT NULL UNIQUE REFERENCES campaign_state(identity_key)
) STRICT;
CREATE TABLE operation_receipt (
    operation_id TEXT PRIMARY KEY,
    identity_key TEXT NOT NULL REFERENCES campaign_state(identity_key),
    kind TEXT NOT NULL,
    request_digest TEXT NOT NULL CHECK(length(request_digest) = 64),
    receipt_json TEXT NOT NULL,
    result_json TEXT NOT NULL
) STRICT;
CREATE TABLE request_slot (
    identity_key TEXT NOT NULL REFERENCES campaign_state(identity_key),
    obligation_id TEXT NOT NULL,
    attempt_id TEXT NOT NULL,
    operation_id TEXT NOT NULL UNIQUE REFERENCES operation_receipt(operation_id),
    request_digest TEXT NOT NULL CHECK(length(request_digest) = 64),
    prepared_revision TEXT NOT NULL CHECK(length(prepared_revision) = 64),
    dispatch_operation_id TEXT UNIQUE,
    may_have_entered INTEGER NOT NULL DEFAULT 0 CHECK(may_have_entered IN (0, 1)),
    active INTEGER NOT NULL CHECK(active IN (0, 1)),
    CHECK ((dispatch_operation_id IS NULL AND may_have_entered = 0) OR
           (dispatch_operation_id IS NOT NULL AND may_have_entered = 1)),
    PRIMARY KEY(identity_key, obligation_id, attempt_id)
) STRICT;
CREATE UNIQUE INDEX request_slot_one_active
    ON request_slot(identity_key, obligation_id) WHERE active = 1;
