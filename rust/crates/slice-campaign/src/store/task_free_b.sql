-- Attached by Campaign schema assembly only after the A/B owner join.
-- task_free_b_committed_spend_v1 is an owner-provided projection, NOT defined here.
CREATE TABLE b_lifecycle_attempt (
    attempt_id TEXT PRIMARY KEY NOT NULL,
    issuer_id TEXT NOT NULL,
    grant_id TEXT NOT NULL,
    binding_json TEXT NOT NULL CHECK(length(binding_json) BETWEEN 1 AND 4096),
    UNIQUE(issuer_id, grant_id)
) STRICT;

CREATE TABLE b_lifecycle_mark (
    attempt_id TEXT NOT NULL REFERENCES b_lifecycle_attempt(attempt_id),
    sequence INTEGER NOT NULL CHECK(sequence BETWEEN 1 AND 3),
    kind INTEGER NOT NULL CHECK(kind BETWEEN 1 AND 3),
    mark_json TEXT NOT NULL CHECK(length(mark_json) BETWEEN 1 AND 4096),
    PRIMARY KEY(attempt_id, sequence),
    UNIQUE(attempt_id, kind)
) STRICT;

CREATE TABLE b_lifecycle_fact (
    attempt_id TEXT NOT NULL REFERENCES b_lifecycle_attempt(attempt_id),
    slot INTEGER NOT NULL CHECK(slot BETWEEN 1 AND 5),
    ordinal INTEGER NOT NULL CHECK(ordinal IN (0, 1)),
    fact_json TEXT NOT NULL CHECK(length(fact_json) BETWEEN 1 AND 4096),
    PRIMARY KEY(attempt_id, slot, ordinal)
) STRICT;
