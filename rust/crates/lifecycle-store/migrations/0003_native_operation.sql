-- S5 native execution and bounded operation custody. All tables live under
-- the one lifecycle SQLite writer and its existing durable lock.
ALTER TABLE service_closes ADD COLUMN native_close_json TEXT NOT NULL DEFAULT '[]';
CREATE TABLE native_sessions (
    session_id TEXT PRIMARY KEY,
    subject_id TEXT NOT NULL REFERENCES subjects(subject_id),
    context_id TEXT NOT NULL,
    incarnation_id TEXT NOT NULL,
    profile_digest TEXT NOT NULL,
    binary_sha256 TEXT NOT NULL,
    protocol_digest TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    snapshot_manifest_sha256 TEXT NOT NULL,
    allocation_state TEXT NOT NULL CHECK (allocation_state IN ('intent','bound','conflicted')),
    thread_id TEXT UNIQUE,
    created_wall_ms INTEGER NOT NULL,
    UNIQUE(subject_id,context_id)
) STRICT;

CREATE TABLE native_turns (
    invocation_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES native_sessions(session_id),
    attempt_id TEXT NOT NULL UNIQUE,
    grant_id TEXT NOT NULL REFERENCES grants(grant_id),
    grant_revision INTEGER NOT NULL,
    purpose TEXT NOT NULL CHECK (purpose IN ('domain_work','semantic_compile','semantic_verify','successor_rehydrate')),
    prompt_sha256 TEXT NOT NULL,
    prompt BLOB NOT NULL,
    profile_digest TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('entered_uncertain','completed_settled','blocked_conflicted')),
    thread_id TEXT NOT NULL,
    turn_id TEXT UNIQUE,
    send_intent INTEGER NOT NULL DEFAULT 0 CHECK (send_intent IN (0,1)),
    terminal_status TEXT,
    final_text BLOB,
    final_sha256 TEXT,
    history_status TEXT NOT NULL DEFAULT 'unavailable',
    history_transport_session TEXT,
    history_sequence INTEGER,
    entered_wall_ms INTEGER NOT NULL,
    settled_wall_ms INTEGER,
    FOREIGN KEY(history_transport_session,history_sequence)
      REFERENCES native_ingress(transport_session,sequence)
) STRICT;

CREATE TABLE native_ingress (
    transport_session TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK(sequence > 0),
    raw_sha256 TEXT NOT NULL,
    raw BLOB NOT NULL,
    decode_status TEXT NOT NULL,
    method TEXT,
    thread_id TEXT,
    turn_id TEXT,
    item_id TEXT,
    observed_wall_ms INTEGER NOT NULL,
    PRIMARY KEY(transport_session,sequence)
) STRICT;

CREATE TABLE native_operations (
    operation_attempt_id TEXT PRIMARY KEY,
    parent_invocation_id TEXT NOT NULL REFERENCES native_turns(invocation_id),
    call_id TEXT NOT NULL,
    request_transport_session TEXT NOT NULL,
    request_sequence INTEGER NOT NULL,
    grant_id TEXT NOT NULL REFERENCES grants(grant_id),
    grant_revision INTEGER NOT NULL,
    contract_id TEXT NOT NULL,
    implementation_id TEXT NOT NULL,
    executor_id TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    member_id TEXT NOT NULL,
    range_start INTEGER NOT NULL CHECK(range_start >= 0),
    range_length INTEGER NOT NULL CHECK(range_length > 0),
    result_sha256 TEXT,
    result BLOB,
    state TEXT NOT NULL CHECK(state IN ('entered_uncertain','completed_settled','blocked_conflicted')),
    entered_wall_ms INTEGER NOT NULL,
    settled_wall_ms INTEGER,
    UNIQUE(parent_invocation_id,call_id),
    FOREIGN KEY(request_transport_session,request_sequence)
      REFERENCES native_ingress(transport_session,sequence)
) STRICT;

CREATE TABLE native_ingress_conflicts (
    conflict_id INTEGER PRIMARY KEY AUTOINCREMENT,
    transport_session TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    raw_sha256 TEXT NOT NULL,
    raw BLOB NOT NULL,
    observed_wall_ms INTEGER NOT NULL,
    UNIQUE(transport_session,sequence,raw_sha256),
    FOREIGN KEY(transport_session,sequence)
      REFERENCES native_ingress(transport_session,sequence)
) STRICT;

CREATE TABLE native_usage (
    transport_session TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    invocation_id TEXT NOT NULL REFERENCES native_turns(invocation_id),
    scope TEXT NOT NULL,
    raw_json BLOB NOT NULL,
    PRIMARY KEY(transport_session,sequence),
    FOREIGN KEY(transport_session,sequence)
      REFERENCES native_ingress(transport_session,sequence)
) STRICT;

CREATE TABLE native_process_exits (
    session_id TEXT PRIMARY KEY REFERENCES native_sessions(session_id),
    pid INTEGER,
    exit_code INTEGER,
    forced INTEGER NOT NULL CHECK(forced IN (0,1)),
    outcome_kind TEXT NOT NULL,
    observed_wall_ms INTEGER NOT NULL
) STRICT;

-- S4's basis_source has an observation FK. Native predecessor custody has a
-- separate FK so no fabricated S4 observation can launder a native result.
ALTER TABLE transitions ADD COLUMN native_basis_source TEXT REFERENCES native_turns(invocation_id);

CREATE TABLE native_verifier_results (
    transition_id TEXT PRIMARY KEY REFERENCES transitions(transition_id),
    source_id TEXT NOT NULL,
    activation TEXT NOT NULL,
    disposition TEXT NOT NULL,
    primary_error TEXT NOT NULL,
    exit_code INTEGER,
    exit_success INTEGER NOT NULL,
    unsafe_at_close INTEGER NOT NULL,
    late_at_close INTEGER NOT NULL
) STRICT;
