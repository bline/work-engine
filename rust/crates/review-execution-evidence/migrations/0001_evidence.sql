CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE events (
    attempt_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    stage TEXT NOT NULL,
    predecessor_sha256 TEXT NOT NULL,
    payload BLOB NOT NULL,
    sha256 TEXT NOT NULL,
    PRIMARY KEY (attempt_id, ordinal),
    UNIQUE (attempt_id, stage)
);
CREATE TABLE artifacts (
    attempt_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    bytes BLOB NOT NULL,
    sha256 TEXT NOT NULL,
    PRIMARY KEY (attempt_id, kind)
);
