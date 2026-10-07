CREATE TABLE subjects (
    subject_id TEXT PRIMARY KEY,
    context_id TEXT NOT NULL,
    build_id TEXT NOT NULL,
    proof_run_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 0),
    semantic_revision INTEGER NOT NULL CHECK (semantic_revision >= 0),
    queue_revision INTEGER NOT NULL CHECK (queue_revision >= 0),
    owner_kind TEXT NOT NULL CHECK (owner_kind IN ('domain','transition','fenced')),
    owner_ref TEXT,
    last_wall_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE transitions (
    transition_id TEXT PRIMARY KEY,
    subject_id TEXT NOT NULL REFERENCES subjects(subject_id),
    predecessor_context TEXT NOT NULL,
    stage TEXT NOT NULL CHECK (stage IN ('quiescing','capturing','verifying','checkpointed','switching','rehydrating','ready_to_commit','reconciled','recovery_required','aborted_before_entry')),
    basis_revision INTEGER NOT NULL,
    checkpoint_id TEXT,
    successor_context TEXT,
    successor_thread TEXT,
    created_wall_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE transition_evidence (
    source_id TEXT PRIMARY KEY,
    transition_id TEXT NOT NULL REFERENCES transitions(transition_id),
    stage_after TEXT NOT NULL,
    observed_wall_ms INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX one_open_transition_per_subject ON transitions(subject_id)
    WHERE stage NOT IN ('reconciled','aborted_before_entry');

CREATE TABLE interruption_requests (
    principal TEXT NOT NULL,
    command_id TEXT NOT NULL,
    subject_id TEXT NOT NULL REFERENCES subjects(subject_id),
    attempt_id TEXT NOT NULL,
    requested_wall_ms INTEGER NOT NULL,
    PRIMARY KEY (principal, command_id)
) STRICT;

CREATE TABLE grants (
    grant_id TEXT PRIMARY KEY,
    issuer TEXT NOT NULL,
    principal TEXT NOT NULL,
    subject_id TEXT NOT NULL REFERENCES subjects(subject_id),
    context_id TEXT NOT NULL,
    build_id TEXT NOT NULL,
    proof_run_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    expires_ms INTEGER NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 0),
    revoked INTEGER NOT NULL CHECK (revoked IN (0,1))
) STRICT;

-- Authenticated UnknownSubject is a stable command result, so its subject ID
-- intentionally need not have a registered subjects row yet.
CREATE TABLE commands (
    principal TEXT NOT NULL,
    command_id TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    request_digest TEXT NOT NULL,
    canonical_bytes BLOB NOT NULL,
    outcome_kind TEXT NOT NULL,
    outcome_ref TEXT,
    rejection_code TEXT,
    resulting_revision INTEGER NOT NULL,
    PRIMARY KEY (principal, command_id)
) STRICT;

CREATE TABLE inputs (
    input_id TEXT PRIMARY KEY,
    subject_id TEXT NOT NULL REFERENCES subjects(subject_id),
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    delivery_id TEXT NOT NULL UNIQUE,
    producer_ref TEXT NOT NULL,
    text BLOB NOT NULL,
    text_digest TEXT NOT NULL,
    grant_id TEXT NOT NULL REFERENCES grants(grant_id),
    entry_grant_id TEXT REFERENCES grants(grant_id),
    state TEXT NOT NULL CHECK (state IN ('queued','prepared','entered','succeeded','failed_blocked','unknown','retired')),
    UNIQUE (subject_id, sequence)
) STRICT;

CREATE TABLE effects (
    effect_id TEXT PRIMARY KEY,
    subject_id TEXT NOT NULL REFERENCES subjects(subject_id),
    input_id TEXT NOT NULL REFERENCES inputs(input_id),
    expected_revision INTEGER NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('prepared','entered','applied','cancelled'))
) STRICT;

CREATE UNIQUE INDEX one_active_effect_per_input ON effects(input_id)
    WHERE state!='cancelled';

CREATE TABLE attempts (
    attempt_id TEXT PRIMARY KEY,
    effect_id TEXT NOT NULL UNIQUE REFERENCES effects(effect_id),
    incarnation_id TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('entered','observed')),
    entered_wall_ms INTEGER NOT NULL,
    provider_thread_id TEXT,
    provider_turn_id TEXT,
    outcome TEXT NOT NULL DEFAULT 'pending',
    settlement_kind TEXT NOT NULL DEFAULT 'unresolved',
    settlement_evidence TEXT
) STRICT;

CREATE TABLE observations (
    source_id TEXT PRIMARY KEY,
    effect_id TEXT NOT NULL REFERENCES effects(effect_id),
    attempt_id TEXT NOT NULL REFERENCES attempts(attempt_id),
    incarnation_id TEXT NOT NULL,
    provider_thread_id TEXT NOT NULL,
    provider_turn_id TEXT NOT NULL,
    outcome TEXT NOT NULL,
    settlement_kind TEXT NOT NULL,
    settlement_evidence TEXT,
    applied_kind TEXT NOT NULL,
    observed_wall_ms INTEGER NOT NULL
) STRICT;

-- A source coordinate may arrive again with contradictory bytes. Preserve each
-- exact variant and its application result without rewriting the original fact.
CREATE TABLE observation_conflicts (
    conflict_id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id TEXT NOT NULL REFERENCES observations(source_id),
    effect_id TEXT NOT NULL,
    attempt_id TEXT NOT NULL,
    incarnation_id TEXT NOT NULL,
    provider_thread_id TEXT NOT NULL,
    provider_turn_id TEXT NOT NULL,
    outcome TEXT NOT NULL,
    settlement_kind TEXT NOT NULL,
    settlement_evidence TEXT,
    attribution_kind TEXT NOT NULL CHECK (attribution_kind IN ('exact','mismatch')),
    observed_wall_ms INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX observation_conflict_variant ON observation_conflicts (
    source_id,effect_id,attempt_id,incarnation_id,provider_thread_id,
    provider_turn_id,outcome,settlement_kind,COALESCE(settlement_evidence,'')
);

CREATE TABLE artifacts (
    digest_hex TEXT PRIMARY KEY,
    subject_id TEXT NOT NULL REFERENCES subjects(subject_id),
    byte_length INTEGER NOT NULL CHECK (byte_length >= 0),
    relative_path TEXT NOT NULL UNIQUE,
    committed_wall_ms INTEGER NOT NULL
) STRICT;

-- The corresponding rejection event must remain durable under that same ID.
CREATE TABLE journal (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    subject_id TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    event_ref TEXT NOT NULL,
    wall_ms INTEGER NOT NULL
) STRICT;
