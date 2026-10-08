CREATE TABLE store_identity (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    store_id TEXT NOT NULL
) STRICT;

CREATE TABLE task_results (
    attempt_id TEXT PRIMARY KEY REFERENCES attempts(attempt_id),
    termination TEXT NOT NULL,
    local_task_ended INTEGER NOT NULL CHECK (local_task_ended IN (0,1)),
    child_exit_code INTEGER,
    finished_wall_ms INTEGER,
    observation_source TEXT,
    final_text_sha256 TEXT
) STRICT;

CREATE TABLE verifier_results (
    transition_id TEXT PRIMARY KEY REFERENCES transitions(transition_id),
    source_id TEXT NOT NULL REFERENCES observations(source_id),
    activation TEXT NOT NULL,
    disposition TEXT NOT NULL,
    primary_error TEXT NOT NULL,
    exit_code INTEGER,
    exit_success INTEGER NOT NULL CHECK (exit_success IN (0,1)),
    unsafe_at_close INTEGER NOT NULL CHECK (unsafe_at_close IN (0,1)),
    late_at_close INTEGER NOT NULL CHECK (late_at_close IN (0,1))
) STRICT;

CREATE TABLE service_closes (
    incarnation_id TEXT PRIMARY KEY,
    subject_id TEXT NOT NULL REFERENCES subjects(subject_id),
    joined_json TEXT NOT NULL,
    unresolved_json TEXT NOT NULL,
    pending_results_json TEXT NOT NULL,
    unsafe_at_close INTEGER NOT NULL CHECK (unsafe_at_close IN (0,1)),
    wall_ms INTEGER NOT NULL
) STRICT;

ALTER TABLE observations ADD COLUMN final_text BLOB;
ALTER TABLE observation_conflicts ADD COLUMN final_text BLOB;
ALTER TABLE transitions ADD COLUMN basis_source TEXT REFERENCES observations(source_id);
DROP INDEX observation_conflict_variant;
CREATE UNIQUE INDEX observation_conflict_variant ON observation_conflicts (
    source_id,effect_id,attempt_id,incarnation_id,provider_thread_id,
    provider_turn_id,outcome,settlement_kind,COALESCE(settlement_evidence,''),
    COALESCE(hex(final_text),'')
);
