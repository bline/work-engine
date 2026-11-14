-- Physical12/13 preserve behavior6/7 and add immutable input observations.
CREATE TABLE native_input_observations (
 invocation_id TEXT PRIMARY KEY REFERENCES native_turns(invocation_id),
 session_id TEXT NOT NULL REFERENCES native_sessions(session_id),
 attempt_id TEXT NOT NULL,
 transport_session TEXT NOT NULL,
 terminal_sequence INTEGER NOT NULL,
 artifact_sha256 TEXT NOT NULL REFERENCES artifacts(digest_hex),
 observed_wall_ms INTEGER NOT NULL,
 FOREIGN KEY(transport_session,terminal_sequence) REFERENCES native_ingress(transport_session,sequence)
) STRICT;
CREATE TABLE native_input_conflicts (
 invocation_id TEXT NOT NULL REFERENCES native_input_observations(invocation_id),
 artifact_sha256 TEXT NOT NULL REFERENCES artifacts(digest_hex),
 observed_wall_ms INTEGER NOT NULL,
 PRIMARY KEY(invocation_id,artifact_sha256)
) STRICT;
CREATE TRIGGER custody_revision_native_input_observations_insert AFTER INSERT ON native_input_observations
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_native_input_observations_insert BEFORE INSERT ON native_input_observations
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER custody_revision_native_input_observations_update AFTER UPDATE ON native_input_observations
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_native_input_observations_update BEFORE UPDATE ON native_input_observations
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER custody_revision_native_input_observations_delete AFTER DELETE ON native_input_observations
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_native_input_observations_delete BEFORE DELETE ON native_input_observations
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER custody_revision_native_input_conflicts_insert AFTER INSERT ON native_input_conflicts
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_native_input_conflicts_insert BEFORE INSERT ON native_input_conflicts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER custody_revision_native_input_conflicts_update AFTER UPDATE ON native_input_conflicts
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_native_input_conflicts_update BEFORE UPDATE ON native_input_conflicts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
CREATE TRIGGER custody_revision_native_input_conflicts_delete AFTER DELETE ON native_input_conflicts
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_native_input_conflicts_delete BEFORE DELETE ON native_input_conflicts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
