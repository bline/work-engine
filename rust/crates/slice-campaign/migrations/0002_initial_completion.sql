-- New private roots only. Historical schema-1 roots are never upgraded on open.
DROP TABLE request_slot;
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
