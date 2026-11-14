-- Physical14/15 retain immutable causal companions; accepted input bytes are untouched.
CREATE TABLE native_causal_evidence (
 operation_id TEXT PRIMARY KEY REFERENCES native_task_operations(operation_id),
 invocation_id TEXT NOT NULL REFERENCES native_input_observations(invocation_id),
 input_artifact_sha256 TEXT NOT NULL REFERENCES artifacts(digest_hex),
 artifact_sha256 TEXT NOT NULL REFERENCES artifacts(digest_hex),
 observed_wall_ms INTEGER NOT NULL
) STRICT;
CREATE TABLE native_causal_conflicts (
 operation_id TEXT NOT NULL REFERENCES native_causal_evidence(operation_id),
 artifact_sha256 TEXT NOT NULL REFERENCES artifacts(digest_hex),
 observed_wall_ms INTEGER NOT NULL,
 PRIMARY KEY(operation_id,artifact_sha256)
) STRICT;
CREATE TRIGGER custody_revision_native_causal_evidence_insert AFTER INSERT ON native_causal_evidence
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_native_causal_evidence_insert BEFORE INSERT ON native_causal_evidence
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER custody_revision_native_causal_evidence_update AFTER UPDATE ON native_causal_evidence
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_native_causal_evidence_update BEFORE UPDATE ON native_causal_evidence
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER custody_revision_native_causal_evidence_delete AFTER DELETE ON native_causal_evidence
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_native_causal_evidence_delete BEFORE DELETE ON native_causal_evidence
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER custody_revision_native_causal_conflicts_insert AFTER INSERT ON native_causal_conflicts
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_native_causal_conflicts_insert BEFORE INSERT ON native_causal_conflicts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER custody_revision_native_causal_conflicts_update AFTER UPDATE ON native_causal_conflicts
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_native_causal_conflicts_update BEFORE UPDATE ON native_causal_conflicts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER custody_revision_native_causal_conflicts_delete AFTER DELETE ON native_causal_conflicts
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_native_causal_conflicts_delete BEFORE DELETE ON native_causal_conflicts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
