-- S6/S7 share one successor of the S5 store. The caller disables FK enforcement
-- for this table rebuild, performs the migration in one transaction, and
-- checks all foreign keys before enabling admission again.
-- A transition opts into S6 once. A stale/missing accepted package must never
-- make later facts or successor rehydration fall back to the S5 path.
ALTER TABLE transitions ADD COLUMN semantic_required INTEGER NOT NULL DEFAULT 0
    CHECK (semantic_required IN (0,1));

CREATE TABLE native_sessions_new (
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
    role TEXT NOT NULL CHECK (role IN ('domain','successor','semantic_compile','semantic_verify','semantic_successor_verify')),
    control_transition_id TEXT REFERENCES transitions(transition_id),
    CHECK ((role IN ('domain','successor') AND control_transition_id IS NULL)
        OR (role IN ('semantic_compile','semantic_verify','semantic_successor_verify') AND control_transition_id IS NOT NULL))
) STRICT;

INSERT INTO native_sessions_new(
    session_id,subject_id,context_id,incarnation_id,profile_digest,binary_sha256,
    protocol_digest,snapshot_id,snapshot_manifest_sha256,allocation_state,
    thread_id,created_wall_ms,role,control_transition_id
)
SELECT session_id,subject_id,context_id,incarnation_id,profile_digest,binary_sha256,
    protocol_digest,snapshot_id,snapshot_manifest_sha256,allocation_state,
    thread_id,created_wall_ms,
    CASE WHEN EXISTS (
        SELECT 1 FROM native_turns t WHERE t.session_id=native_sessions.session_id
            AND t.purpose='successor_rehydrate'
    ) THEN 'successor' ELSE 'domain' END,
    NULL
FROM native_sessions;

DROP TABLE native_sessions;
ALTER TABLE native_sessions_new RENAME TO native_sessions;
CREATE UNIQUE INDEX one_native_generation_session
    ON native_sessions(subject_id,context_id)
    WHERE role IN ('domain','successor');
CREATE UNIQUE INDEX one_native_control_session
    ON native_sessions(control_transition_id,role)
    WHERE role IN ('semantic_compile','semantic_verify','semantic_successor_verify');

-- v3 ingress had no explicit session lineage. Historical rows remain nullable;
-- v4 query-only qualification refuses them unless a later exact owner record
-- establishes linkage. New ingress always stores the session permit identity.
ALTER TABLE native_ingress ADD COLUMN session_id TEXT REFERENCES native_sessions(session_id);

CREATE TABLE semantic_captures (
    capture_id TEXT PRIMARY KEY,
    transition_id TEXT NOT NULL REFERENCES transitions(transition_id),
    subject_id TEXT NOT NULL REFERENCES subjects(subject_id),
    schema_id TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    basis_sha256 TEXT NOT NULL,
    basis_artifact_sha256 TEXT NOT NULL REFERENCES artifacts(digest_hex),
    dependencies_json BLOB NOT NULL,
    source_revision INTEGER NOT NULL,
    created_wall_ms INTEGER NOT NULL,
    eligibility TEXT NOT NULL CHECK (eligibility IN ('current','stale','unresolved'))
) STRICT;
CREATE INDEX semantic_captures_by_transition ON semantic_captures(transition_id,created_wall_ms);

-- Invalid schema/policy or malformed owner input has no valid SourceBasis to
-- link. Preserve a bounded, digest-bound negative result without pretending
-- it is a capture or allowing any control inference.
CREATE TABLE semantic_material_rejections (
    capture_id TEXT PRIMARY KEY,
    transition_id TEXT NOT NULL REFERENCES transitions(transition_id),
    subject_id TEXT NOT NULL REFERENCES subjects(subject_id),
    input_sha256 TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    schema_version INTEGER NOT NULL CHECK (schema_version>=0),
    semantic_revision INTEGER NOT NULL CHECK (semantic_revision>=0),
    error_code TEXT NOT NULL,
    eligibility TEXT NOT NULL CHECK (eligibility='unresolved'),
    created_wall_ms INTEGER NOT NULL
) STRICT;
CREATE INDEX semantic_material_rejections_by_transition
    ON semantic_material_rejections(transition_id,created_wall_ms);

CREATE TABLE semantic_attempts (
    invocation_id TEXT PRIMARY KEY REFERENCES native_turns(invocation_id),
    capture_id TEXT NOT NULL REFERENCES semantic_captures(capture_id),
    phase TEXT NOT NULL CHECK (phase IN ('compile','candidate_verify','successor_adequacy')),
    purpose TEXT NOT NULL CHECK (purpose IN ('semantic_compile','semantic_verify')),
    attempt_id TEXT NOT NULL UNIQUE,
    session_id TEXT NOT NULL REFERENCES native_sessions(session_id),
    input_sha256 TEXT NOT NULL,
    schema_id TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    exposure_sha256 TEXT NOT NULL,
    grant_id TEXT NOT NULL REFERENCES grants(grant_id),
    grant_revision INTEGER NOT NULL,
    result_artifact_sha256 TEXT REFERENCES artifacts(digest_hex),
    result_sha256 TEXT,
    state TEXT NOT NULL CHECK (state IN ('entered_uncertain','completed_settled','blocked_conflicted'))
) STRICT;

CREATE TABLE semantic_results (
    result_id TEXT PRIMARY KEY,
    transition_id TEXT NOT NULL REFERENCES transitions(transition_id),
    capture_id TEXT NOT NULL REFERENCES semantic_captures(capture_id),
    verifier_invocation_id TEXT NOT NULL REFERENCES semantic_attempts(invocation_id),
    phase TEXT NOT NULL CHECK (phase IN ('candidate_verify','successor_adequacy')),
    candidate_artifact_sha256 TEXT NOT NULL REFERENCES artifacts(digest_hex),
    candidate_record_artifact_sha256 TEXT REFERENCES artifacts(digest_hex),
    challenge_artifact_sha256 TEXT REFERENCES artifacts(digest_hex),
    successor_prompt_artifact_sha256 TEXT REFERENCES artifacts(digest_hex),
    response_artifact_sha256 TEXT REFERENCES artifacts(digest_hex),
    findings_artifact_sha256 TEXT NOT NULL REFERENCES artifacts(digest_hex),
    verdict TEXT NOT NULL CHECK (verdict IN ('accepted','rejected','unresolved')),
    eligibility TEXT NOT NULL CHECK (eligibility IN ('current','stale','unresolved')),
    basis_sha256 TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    created_wall_ms INTEGER NOT NULL
) STRICT;
CREATE INDEX semantic_results_by_transition ON semantic_results(transition_id,created_wall_ms);

CREATE TABLE admin_ownership (
    singleton INTEGER PRIMARY KEY CHECK (singleton=1),
    status TEXT NOT NULL CHECK (status IN ('active','inactive','fenced')),
    revision INTEGER NOT NULL CHECK (revision>=0),
    handoff_id TEXT,
    capsule_sha256 TEXT,
    peer_store_id TEXT,
    selection_revision INTEGER
) STRICT;
INSERT INTO admin_ownership(singleton,status,revision) VALUES (1,'active',0);

CREATE TABLE admin_store_lineage (
    singleton INTEGER PRIMARY KEY CHECK (singleton=1),
    source_store_id TEXT NOT NULL,
    capsule_sha256 TEXT NOT NULL,
    handoff_id TEXT NOT NULL,
    source_revision INTEGER NOT NULL
) STRICT;

CREATE TABLE admin_recovery_observations (
    command_id TEXT PRIMARY KEY,
    selector_sha256 TEXT NOT NULL,
    fact_sha256 TEXT NOT NULL,
    fact_json BLOB NOT NULL,
    claim_id TEXT NOT NULL,
    observation_revision INTEGER NOT NULL,
    classification TEXT NOT NULL,
    availability TEXT NOT NULL,
    outcome TEXT,
    settlement TEXT,
    effect_coverage TEXT NOT NULL,
    observed_wall_ms INTEGER NOT NULL,
    UNIQUE(claim_id,observation_revision)
) STRICT;
CREATE TABLE admin_dispositions (
    command_id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL UNIQUE,
    selector_sha256 TEXT NOT NULL,
    request_sha256 TEXT NOT NULL,
    grant_sha256 TEXT NOT NULL,
    observation_revision INTEGER NOT NULL,
    original_outcome TEXT,
    original_settlement TEXT,
    disposition TEXT NOT NULL CHECK (disposition='retired_no_retry'),
    wall_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE admin_handoffs (
    handoff_id TEXT PRIMARY KEY,
    capsule_sha256 TEXT NOT NULL,
    source_store_id TEXT NOT NULL,
    destination_store_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('accepted_inactive','source_fenced','destination_owned')),
    revision INTEGER NOT NULL,
    adoption_revision INTEGER NOT NULL,
    source_revision INTEGER NOT NULL,
    selection_revision INTEGER,
    selection_digest TEXT,
    launch_guard_id TEXT
) STRICT;

-- This monotonic token covers every durable custody mutation. Export and source
-- fence compare it so an old capsule cannot outlive a retirement, recovery
-- observation, native ingress or child-entry write that did not append journal.
CREATE TABLE admin_custody_revision (
    singleton INTEGER PRIMARY KEY CHECK (singleton=1),
    revision INTEGER NOT NULL CHECK (revision>=0)
) STRICT;
INSERT INTO admin_custody_revision(singleton,revision) VALUES (1,0);

CREATE TRIGGER custody_revision_subjects_insert AFTER INSERT ON subjects
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_subjects_update AFTER UPDATE ON subjects
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_subjects_delete AFTER DELETE ON subjects
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_transitions_insert AFTER INSERT ON transitions
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_transitions_update AFTER UPDATE ON transitions
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_transitions_delete AFTER DELETE ON transitions
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_transition_evidence_insert AFTER INSERT ON transition_evidence
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_transition_evidence_update AFTER UPDATE ON transition_evidence
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_transition_evidence_delete AFTER DELETE ON transition_evidence
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_interruption_requests_insert AFTER INSERT ON interruption_requests
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_interruption_requests_update AFTER UPDATE ON interruption_requests
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_interruption_requests_delete AFTER DELETE ON interruption_requests
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_grants_insert AFTER INSERT ON grants
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_grants_update AFTER UPDATE ON grants
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_grants_delete AFTER DELETE ON grants
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_commands_insert AFTER INSERT ON commands
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_commands_update AFTER UPDATE ON commands
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_commands_delete AFTER DELETE ON commands
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_inputs_insert AFTER INSERT ON inputs
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_inputs_update AFTER UPDATE ON inputs
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_inputs_delete AFTER DELETE ON inputs
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_effects_insert AFTER INSERT ON effects
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_effects_update AFTER UPDATE ON effects
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_effects_delete AFTER DELETE ON effects
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_attempts_insert AFTER INSERT ON attempts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_attempts_update AFTER UPDATE ON attempts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_attempts_delete AFTER DELETE ON attempts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_observations_insert AFTER INSERT ON observations
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_observations_update AFTER UPDATE ON observations
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_observations_delete AFTER DELETE ON observations
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_observation_conflicts_insert AFTER INSERT ON observation_conflicts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_observation_conflicts_update AFTER UPDATE ON observation_conflicts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_observation_conflicts_delete AFTER DELETE ON observation_conflicts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_artifacts_insert AFTER INSERT ON artifacts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_artifacts_update AFTER UPDATE ON artifacts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_artifacts_delete AFTER DELETE ON artifacts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_journal_insert AFTER INSERT ON journal
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_journal_update AFTER UPDATE ON journal
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_journal_delete AFTER DELETE ON journal
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_store_identity_insert AFTER INSERT ON store_identity
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_store_identity_update AFTER UPDATE ON store_identity
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_store_identity_delete AFTER DELETE ON store_identity
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_task_results_insert AFTER INSERT ON task_results
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_task_results_update AFTER UPDATE ON task_results
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_task_results_delete AFTER DELETE ON task_results
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_verifier_results_insert AFTER INSERT ON verifier_results
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_verifier_results_update AFTER UPDATE ON verifier_results
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_verifier_results_delete AFTER DELETE ON verifier_results
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_service_closes_insert AFTER INSERT ON service_closes
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_service_closes_update AFTER UPDATE ON service_closes
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_service_closes_delete AFTER DELETE ON service_closes
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_sessions_insert AFTER INSERT ON native_sessions
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_sessions_update AFTER UPDATE ON native_sessions
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_sessions_delete AFTER DELETE ON native_sessions
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_turns_insert AFTER INSERT ON native_turns
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_turns_update AFTER UPDATE ON native_turns
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_turns_delete AFTER DELETE ON native_turns
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_ingress_insert AFTER INSERT ON native_ingress
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_ingress_update AFTER UPDATE ON native_ingress
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_ingress_delete AFTER DELETE ON native_ingress
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_operations_insert AFTER INSERT ON native_operations
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_operations_update AFTER UPDATE ON native_operations
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_operations_delete AFTER DELETE ON native_operations
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_ingress_conflicts_insert AFTER INSERT ON native_ingress_conflicts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_ingress_conflicts_update AFTER UPDATE ON native_ingress_conflicts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_ingress_conflicts_delete AFTER DELETE ON native_ingress_conflicts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_usage_insert AFTER INSERT ON native_usage
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_usage_update AFTER UPDATE ON native_usage
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_usage_delete AFTER DELETE ON native_usage
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_process_exits_insert AFTER INSERT ON native_process_exits
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_process_exits_update AFTER UPDATE ON native_process_exits
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_process_exits_delete AFTER DELETE ON native_process_exits
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_verifier_results_insert AFTER INSERT ON native_verifier_results
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_verifier_results_update AFTER UPDATE ON native_verifier_results
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_native_verifier_results_delete AFTER DELETE ON native_verifier_results
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_semantic_captures_insert AFTER INSERT ON semantic_captures
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_semantic_captures_update AFTER UPDATE ON semantic_captures
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_semantic_captures_delete AFTER DELETE ON semantic_captures
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_semantic_material_rejections_insert AFTER INSERT ON semantic_material_rejections
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_semantic_material_rejections_update AFTER UPDATE ON semantic_material_rejections
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_semantic_material_rejections_delete AFTER DELETE ON semantic_material_rejections
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_semantic_attempts_insert AFTER INSERT ON semantic_attempts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_semantic_attempts_update AFTER UPDATE ON semantic_attempts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_semantic_attempts_delete AFTER DELETE ON semantic_attempts
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_semantic_results_insert AFTER INSERT ON semantic_results
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_semantic_results_update AFTER UPDATE ON semantic_results
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_semantic_results_delete AFTER DELETE ON semantic_results
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_ownership_insert AFTER INSERT ON admin_ownership
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_ownership_update AFTER UPDATE ON admin_ownership
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_ownership_delete AFTER DELETE ON admin_ownership
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_store_lineage_insert AFTER INSERT ON admin_store_lineage
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_store_lineage_update AFTER UPDATE ON admin_store_lineage
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_store_lineage_delete AFTER DELETE ON admin_store_lineage
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_recovery_observations_insert AFTER INSERT ON admin_recovery_observations
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_recovery_observations_update AFTER UPDATE ON admin_recovery_observations
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_recovery_observations_delete AFTER DELETE ON admin_recovery_observations
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_dispositions_insert AFTER INSERT ON admin_dispositions
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_dispositions_update AFTER UPDATE ON admin_dispositions
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_dispositions_delete AFTER DELETE ON admin_dispositions
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_handoffs_insert AFTER INSERT ON admin_handoffs
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_handoffs_update AFTER UPDATE ON admin_handoffs
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;

CREATE TRIGGER custody_revision_admin_handoffs_delete AFTER DELETE ON admin_handoffs
BEGIN
    UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1;
END;
