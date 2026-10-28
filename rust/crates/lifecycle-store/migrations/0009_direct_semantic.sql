-- Direct-origin semantic control (additive). Native tables, rows and encodings
-- are untouched. Physical schema ids 10 and 11 add this to 8 and 9, which
-- dispatch as behavior 6 and 7. A direct operation has no native session or
-- thread, so it cannot be a semantic_attempts row; it keeps its own identity.

CREATE TABLE direct_semantic_attempts (
    operation_id TEXT PRIMARY KEY,
    capture_id TEXT NOT NULL REFERENCES semantic_captures(capture_id),
    role TEXT NOT NULL CHECK (role IN ('compile','verify')),
    attempt_id TEXT NOT NULL UNIQUE,
    exposure_id TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    grant_id TEXT NOT NULL REFERENCES grants(grant_id),
    grant_revision INTEGER NOT NULL,
    -- The trusted acting principal under which the call was entered. The grant
    -- tuple is compared to it again at publication and at consumption.
    principal TEXT NOT NULL,
    evidence_class TEXT NOT NULL CHECK (evidence_class IN ('synthetic_local','public_api')),
    evidence_ref TEXT NOT NULL,
    requested_provider TEXT NOT NULL,
    requested_model TEXT NOT NULL,
    input_sha256 TEXT NOT NULL,
    prepared_binding_sha256 TEXT NOT NULL UNIQUE,
    prepared_wire_sha256 TEXT NOT NULL UNIQUE,
    prepared_artifact_sha256 TEXT NOT NULL REFERENCES artifacts(digest_hex),
    state TEXT NOT NULL CHECK (state IN
        ('intent_recorded','terminal_recorded','completed_settled','refused','not_sent')),
    terminal_artifact_sha256 TEXT REFERENCES artifacts(digest_hex),
    output_sha256 TEXT,
    result_artifact_sha256 TEXT REFERENCES artifacts(digest_hex),
    refusal_code TEXT,
    created_wall_ms INTEGER NOT NULL
) STRICT;
CREATE INDEX direct_semantic_attempts_by_capture ON direct_semantic_attempts(capture_id,role);

CREATE TABLE direct_semantic_results (
    result_id TEXT PRIMARY KEY,
    transition_id TEXT NOT NULL REFERENCES transitions(transition_id),
    capture_id TEXT NOT NULL REFERENCES semantic_captures(capture_id),
    compile_operation_id TEXT NOT NULL REFERENCES direct_semantic_attempts(operation_id),
    verifier_operation_id TEXT NOT NULL REFERENCES direct_semantic_attempts(operation_id),
    candidate_artifact_sha256 TEXT NOT NULL REFERENCES artifacts(digest_hex),
    candidate_record_artifact_sha256 TEXT REFERENCES artifacts(digest_hex),
    challenge_artifact_sha256 TEXT REFERENCES artifacts(digest_hex),
    successor_prompt_artifact_sha256 TEXT REFERENCES artifacts(digest_hex),
    findings_artifact_sha256 TEXT NOT NULL REFERENCES artifacts(digest_hex),
    verdict TEXT NOT NULL CHECK (verdict IN ('accepted','rejected','unresolved')),
    eligibility TEXT NOT NULL CHECK (eligibility IN ('current','stale','unresolved')),
    basis_sha256 TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    evidence_class TEXT NOT NULL CHECK (evidence_class IN ('synthetic_local','public_api')),
    evidence_ref TEXT NOT NULL,
    -- Owner admission of the direct checkpoint profile, fixed in the publish
    -- transaction from a current trusted grant. NULL means proof-scoped only.
    admission_grant_id TEXT REFERENCES grants(grant_id),
    admission_grant_revision INTEGER,
    created_wall_ms INTEGER NOT NULL
) STRICT;
CREATE INDEX direct_semantic_results_by_transition ON direct_semantic_results(transition_id,created_wall_ms);

CREATE TRIGGER custody_revision_direct_semantic_attempts_insert AFTER INSERT ON direct_semantic_attempts
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_direct_semantic_attempts_insert BEFORE INSERT ON direct_semantic_attempts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;

CREATE TRIGGER custody_revision_direct_semantic_attempts_update AFTER UPDATE ON direct_semantic_attempts
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_direct_semantic_attempts_update BEFORE UPDATE ON direct_semantic_attempts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;

CREATE TRIGGER custody_revision_direct_semantic_attempts_delete AFTER DELETE ON direct_semantic_attempts
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_direct_semantic_attempts_delete BEFORE DELETE ON direct_semantic_attempts
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;

CREATE TRIGGER custody_revision_direct_semantic_results_insert AFTER INSERT ON direct_semantic_results
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_direct_semantic_results_insert BEFORE INSERT ON direct_semantic_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;

CREATE TRIGGER custody_revision_direct_semantic_results_update AFTER UPDATE ON direct_semantic_results
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_direct_semantic_results_update BEFORE UPDATE ON direct_semantic_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;

CREATE TRIGGER custody_revision_direct_semantic_results_delete AFTER DELETE ON direct_semantic_results
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER held_quarantine_direct_semantic_results_delete BEFORE DELETE ON direct_semantic_results
WHEN (SELECT mode FROM legacy_held_state WHERE singleton=1)!='ordinary'
BEGIN SELECT RAISE(ABORT,'held quarantine'); END;
