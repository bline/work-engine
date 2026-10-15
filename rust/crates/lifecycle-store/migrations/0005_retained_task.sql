-- S8 task-host control receipts. A missing reply is recovered by exact
-- command identity; an absent row is not permission to retry a contraction.
ALTER TABLE transitions ADD COLUMN successor_intent_context TEXT;

CREATE TABLE grant_contractions (
    command_id TEXT PRIMARY KEY,
    subject_id TEXT NOT NULL REFERENCES subjects(subject_id),
    principal TEXT NOT NULL,
    grant_id TEXT NOT NULL REFERENCES grants(grant_id),
    expected_revision INTEGER NOT NULL CHECK (expected_revision>=0),
    resulting_revision INTEGER NOT NULL CHECK (resulting_revision>=0),
    canonical_command BLOB NOT NULL,
    committed_wall_ms INTEGER NOT NULL,
    UNIQUE(grant_id, resulting_revision)
) STRICT;

-- One immutable logical FIFO attempt may span distinct native execution
-- attempts. Each native turn retains its own send/terminal record; this table
-- makes the physical attempt lineage and yielded consequence explicit.
CREATE TABLE native_delivery_segments (
    invocation_id TEXT PRIMARY KEY REFERENCES native_turns(invocation_id),
    queue_attempt_id TEXT NOT NULL REFERENCES attempts(attempt_id),
    segment_no INTEGER NOT NULL CHECK(segment_no>=0),
    predecessor_invocation_id TEXT REFERENCES native_delivery_segments(invocation_id),
    transition_id TEXT REFERENCES transitions(transition_id),
    state TEXT NOT NULL CHECK(state IN ('entered','yielded','completed')),
    settled_wall_ms INTEGER,
    UNIQUE(queue_attempt_id,segment_no)
) STRICT;

CREATE TABLE native_task_operations (
    operation_id TEXT PRIMARY KEY,
    parent_invocation_id TEXT NOT NULL REFERENCES native_turns(invocation_id),
    call_id TEXT NOT NULL,
    transport_session TEXT NOT NULL,
    transport_sequence INTEGER NOT NULL,
    contract_id TEXT NOT NULL CHECK(contract_id IN ('task.question.open.v1','task.result.publish.v1')),
    implementation_id TEXT NOT NULL,
    executor_id TEXT NOT NULL,
    grant_id TEXT NOT NULL REFERENCES grants(grant_id),
    grant_revision INTEGER NOT NULL,
    payload_sha256 TEXT NOT NULL,
    entry_sha256 TEXT NOT NULL,
    entry_json BLOB NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('entered_uncertain','completed_settled')),
    owner_commit_json BLOB,
    owner_commit_sha256 TEXT,
    entered_wall_ms INTEGER NOT NULL,
    settled_wall_ms INTEGER,
    UNIQUE(parent_invocation_id,call_id),
    FOREIGN KEY(transport_session,transport_sequence)
      REFERENCES native_ingress(transport_session,sequence)
) STRICT;

CREATE TRIGGER custody_revision_grant_contractions_insert AFTER INSERT ON grant_contractions
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_grant_contractions_update AFTER UPDATE ON grant_contractions
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_grant_contractions_delete AFTER DELETE ON grant_contractions
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_native_delivery_segments_insert AFTER INSERT ON native_delivery_segments
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_native_delivery_segments_update AFTER UPDATE ON native_delivery_segments
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_native_delivery_segments_delete AFTER DELETE ON native_delivery_segments
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_native_task_operations_insert AFTER INSERT ON native_task_operations
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_native_task_operations_update AFTER UPDATE ON native_task_operations
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_native_task_operations_delete AFTER DELETE ON native_task_operations
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
