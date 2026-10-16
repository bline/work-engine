-- S9 held historical custody. No normal lifecycle row is derived from legacy evidence.
CREATE TABLE legacy_held_state (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    mode TEXT NOT NULL CHECK(mode IN ('ordinary','held_quarantine')),
    handoff_id TEXT,
    source_capture_sha256 TEXT,
    capsule_sha256 TEXT,
    inventory_sha256 TEXT,
    row_count INTEGER NOT NULL DEFAULT 0 CHECK(row_count>=0),
    unclassified_count INTEGER NOT NULL DEFAULT 0 CHECK(unclassified_count>=0),
    CHECK((mode='ordinary' AND handoff_id IS NULL AND source_capture_sha256 IS NULL
           AND capsule_sha256 IS NULL AND inventory_sha256 IS NULL
           AND row_count=0 AND unclassified_count=0)
       OR (mode='held_quarantine' AND handoff_id IS NOT NULL
           AND source_capture_sha256 IS NOT NULL AND capsule_sha256 IS NOT NULL
           AND inventory_sha256 IS NOT NULL AND row_count>0 AND unclassified_count>0))
) STRICT;
INSERT INTO legacy_held_state(singleton,mode) VALUES(1,'ordinary');

CREATE TRIGGER held_mode_no_generic_release BEFORE UPDATE OF mode ON legacy_held_state
WHEN OLD.mode='held_quarantine' AND NEW.mode!='held_quarantine'
BEGIN SELECT RAISE(ABORT,'held quarantine release requires new contract'); END;
CREATE TRIGGER held_mode_no_delete BEFORE DELETE ON legacy_held_state
BEGIN SELECT RAISE(ABORT,'held state is permanent'); END;

CREATE TABLE legacy_held_artifacts (
    name TEXT PRIMARY KEY,
    sha256 TEXT NOT NULL,
    byte_length INTEGER NOT NULL CHECK(byte_length>=0),
    raw_bytes BLOB NOT NULL
) STRICT;

CREATE TABLE legacy_held_rows (
    ordinal INTEGER PRIMARY KEY CHECK(ordinal>=0),
    table_name TEXT NOT NULL,
    source_identity TEXT NOT NULL,
    source_revision TEXT,
    source_projection_sha256 TEXT NOT NULL,
    classification TEXT NOT NULL,
    row_json BLOB NOT NULL
) STRICT;

-- Append-only recovery custody chain. The normal singleton lineage remains the
-- immediate predecessor projection; this table retains every prior adoption.
CREATE TABLE legacy_held_lineage (
    ordinal INTEGER PRIMARY KEY CHECK(ordinal>=0),
    source_store_id TEXT NOT NULL,
    destination_store_id TEXT NOT NULL,
    source_revision INTEGER NOT NULL CHECK(source_revision>=0),
    capsule_sha256 TEXT NOT NULL,
    handoff_id TEXT NOT NULL
) STRICT;

CREATE TRIGGER held_artifacts_immutable_insert BEFORE INSERT ON legacy_held_artifacts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)='held_quarantine'
BEGIN SELECT RAISE(ABORT,'sealed held artifact'); END;
CREATE TRIGGER held_artifacts_immutable_update BEFORE UPDATE ON legacy_held_artifacts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)='held_quarantine'
BEGIN SELECT RAISE(ABORT,'sealed held artifact'); END;
CREATE TRIGGER held_artifacts_immutable_delete BEFORE DELETE ON legacy_held_artifacts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)='held_quarantine'
BEGIN SELECT RAISE(ABORT,'sealed held artifact'); END;
CREATE TRIGGER held_rows_immutable_insert BEFORE INSERT ON legacy_held_rows
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)='held_quarantine'
BEGIN SELECT RAISE(ABORT,'sealed held row'); END;
CREATE TRIGGER held_rows_immutable_update BEFORE UPDATE ON legacy_held_rows
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)='held_quarantine'
BEGIN SELECT RAISE(ABORT,'sealed held row'); END;
CREATE TRIGGER held_rows_immutable_delete BEFORE DELETE ON legacy_held_rows
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)='held_quarantine'
BEGIN SELECT RAISE(ABORT,'sealed held row'); END;
CREATE TRIGGER held_lineage_immutable_update BEFORE UPDATE ON legacy_held_lineage
BEGIN SELECT RAISE(ABORT,'held lineage is append-only'); END;
CREATE TRIGGER held_lineage_immutable_delete BEFORE DELETE ON legacy_held_lineage
BEGIN SELECT RAISE(ABORT,'held lineage is append-only'); END;

CREATE TRIGGER custody_revision_legacy_held_state_insert AFTER INSERT ON legacy_held_state
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_legacy_held_state_update AFTER UPDATE ON legacy_held_state
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_legacy_held_state_delete AFTER DELETE ON legacy_held_state
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_legacy_held_artifacts_insert AFTER INSERT ON legacy_held_artifacts
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_legacy_held_artifacts_update AFTER UPDATE ON legacy_held_artifacts
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_legacy_held_artifacts_delete AFTER DELETE ON legacy_held_artifacts
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_legacy_held_rows_insert AFTER INSERT ON legacy_held_rows
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_legacy_held_rows_update AFTER UPDATE ON legacy_held_rows
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_legacy_held_rows_delete AFTER DELETE ON legacy_held_rows
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_legacy_held_lineage_insert AFTER INSERT ON legacy_held_lineage
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_legacy_held_lineage_update AFTER UPDATE ON legacy_held_lineage
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_legacy_held_lineage_delete AFTER DELETE ON legacy_held_lineage
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;

-- A store in held quarantine cannot acquire or mutate normal runnable state,
-- including state with unrelated subject/grant identifiers. Administrative
-- ownership changes and sealed held-evidence readback are separate paths.
CREATE TRIGGER held_quarantine_artifacts_insert BEFORE INSERT ON artifacts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_artifacts_update BEFORE UPDATE ON artifacts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_artifacts_delete BEFORE DELETE ON artifacts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_attempts_insert BEFORE INSERT ON attempts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_attempts_update BEFORE UPDATE ON attempts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_attempts_delete BEFORE DELETE ON attempts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_commands_insert BEFORE INSERT ON commands
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_commands_update BEFORE UPDATE ON commands
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_commands_delete BEFORE DELETE ON commands
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_effects_insert BEFORE INSERT ON effects
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_effects_update BEFORE UPDATE ON effects
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_effects_delete BEFORE DELETE ON effects
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_grant_contractions_insert BEFORE INSERT ON grant_contractions
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_grant_contractions_update BEFORE UPDATE ON grant_contractions
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_grant_contractions_delete BEFORE DELETE ON grant_contractions
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_grants_insert BEFORE INSERT ON grants
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_grants_update BEFORE UPDATE ON grants
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_grants_delete BEFORE DELETE ON grants
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_inputs_insert BEFORE INSERT ON inputs
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_inputs_update BEFORE UPDATE ON inputs
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_inputs_delete BEFORE DELETE ON inputs
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_interruption_requests_insert BEFORE INSERT ON interruption_requests
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_interruption_requests_update BEFORE UPDATE ON interruption_requests
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_interruption_requests_delete BEFORE DELETE ON interruption_requests
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_journal_insert BEFORE INSERT ON journal
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_journal_update BEFORE UPDATE ON journal
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_journal_delete BEFORE DELETE ON journal
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_delivery_segments_insert BEFORE INSERT ON native_delivery_segments
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_delivery_segments_update BEFORE UPDATE ON native_delivery_segments
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_delivery_segments_delete BEFORE DELETE ON native_delivery_segments
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_ingress_insert BEFORE INSERT ON native_ingress
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_ingress_update BEFORE UPDATE ON native_ingress
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_ingress_delete BEFORE DELETE ON native_ingress
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_ingress_conflicts_insert BEFORE INSERT ON native_ingress_conflicts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_ingress_conflicts_update BEFORE UPDATE ON native_ingress_conflicts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_ingress_conflicts_delete BEFORE DELETE ON native_ingress_conflicts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_operations_insert BEFORE INSERT ON native_operations
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_operations_update BEFORE UPDATE ON native_operations
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_operations_delete BEFORE DELETE ON native_operations
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_process_exits_insert BEFORE INSERT ON native_process_exits
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_process_exits_update BEFORE UPDATE ON native_process_exits
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_process_exits_delete BEFORE DELETE ON native_process_exits
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_sessions_insert BEFORE INSERT ON native_sessions
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_sessions_update BEFORE UPDATE ON native_sessions
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_sessions_delete BEFORE DELETE ON native_sessions
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_task_operations_insert BEFORE INSERT ON native_task_operations
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_task_operations_update BEFORE UPDATE ON native_task_operations
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_task_operations_delete BEFORE DELETE ON native_task_operations
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_turns_insert BEFORE INSERT ON native_turns
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_turns_update BEFORE UPDATE ON native_turns
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_turns_delete BEFORE DELETE ON native_turns
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_usage_insert BEFORE INSERT ON native_usage
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_usage_update BEFORE UPDATE ON native_usage
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_usage_delete BEFORE DELETE ON native_usage
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_verifier_results_insert BEFORE INSERT ON native_verifier_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_verifier_results_update BEFORE UPDATE ON native_verifier_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_native_verifier_results_delete BEFORE DELETE ON native_verifier_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_observation_conflicts_insert BEFORE INSERT ON observation_conflicts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_observation_conflicts_update BEFORE UPDATE ON observation_conflicts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_observation_conflicts_delete BEFORE DELETE ON observation_conflicts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_observations_insert BEFORE INSERT ON observations
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_observations_update BEFORE UPDATE ON observations
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_observations_delete BEFORE DELETE ON observations
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_semantic_attempts_insert BEFORE INSERT ON semantic_attempts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_semantic_attempts_update BEFORE UPDATE ON semantic_attempts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_semantic_attempts_delete BEFORE DELETE ON semantic_attempts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_semantic_captures_insert BEFORE INSERT ON semantic_captures
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_semantic_captures_update BEFORE UPDATE ON semantic_captures
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_semantic_captures_delete BEFORE DELETE ON semantic_captures
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_semantic_material_rejections_insert BEFORE INSERT ON semantic_material_rejections
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_semantic_material_rejections_update BEFORE UPDATE ON semantic_material_rejections
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_semantic_material_rejections_delete BEFORE DELETE ON semantic_material_rejections
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_semantic_results_insert BEFORE INSERT ON semantic_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_semantic_results_update BEFORE UPDATE ON semantic_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_semantic_results_delete BEFORE DELETE ON semantic_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_service_closes_insert BEFORE INSERT ON service_closes
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_service_closes_update BEFORE UPDATE ON service_closes
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_service_closes_delete BEFORE DELETE ON service_closes
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_subjects_insert BEFORE INSERT ON subjects
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_subjects_update BEFORE UPDATE ON subjects
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_subjects_delete BEFORE DELETE ON subjects
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_task_results_insert BEFORE INSERT ON task_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_task_results_update BEFORE UPDATE ON task_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_task_results_delete BEFORE DELETE ON task_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_transition_evidence_insert BEFORE INSERT ON transition_evidence
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_transition_evidence_update BEFORE UPDATE ON transition_evidence
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_transition_evidence_delete BEFORE DELETE ON transition_evidence
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_transitions_insert BEFORE INSERT ON transitions
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_transitions_update BEFORE UPDATE ON transitions
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_transitions_delete BEFORE DELETE ON transitions
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_verifier_results_insert BEFORE INSERT ON verifier_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_verifier_results_update BEFORE UPDATE ON verifier_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER held_quarantine_verifier_results_delete BEFORE DELETE ON verifier_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
