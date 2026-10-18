-- An execution epoch selects a native session without changing the domain
-- context or rewriting historical session, turn, ingress, or effect rows.
-- This migration is invoked only by the selected active-store restart path.
CREATE TABLE native_execution_epochs (
    subject_id TEXT NOT NULL REFERENCES subjects(subject_id),
    context_id TEXT NOT NULL,
    epoch INTEGER NOT NULL CHECK(epoch>=0),
    target_session_id TEXT NOT NULL UNIQUE,
    predecessor_session_id TEXT REFERENCES native_sessions(session_id),
    intent_sha256 TEXT,
    selection_sha256 TEXT,
    owner_revision INTEGER NOT NULL CHECK(owner_revision>=0),
    semantic_revision INTEGER NOT NULL CHECK(semantic_revision>=0),
    queue_revision INTEGER NOT NULL CHECK(queue_revision>=0),
    journal_cursor INTEGER NOT NULL CHECK(journal_cursor>=0),
    profile_digest TEXT NOT NULL,
    binary_sha256 TEXT NOT NULL,
    protocol_digest TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    snapshot_manifest_sha256 TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('authorized','active','historical')),
    PRIMARY KEY(subject_id,context_id,epoch),
    CHECK ((epoch=0 AND predecessor_session_id IS NULL AND intent_sha256 IS NULL
            AND selection_sha256 IS NULL AND state IN ('active','historical'))
        OR (epoch>0 AND predecessor_session_id IS NOT NULL AND intent_sha256 IS NOT NULL
            AND selection_sha256 IS NOT NULL))
) STRICT;

INSERT INTO native_execution_epochs(
    subject_id,context_id,epoch,target_session_id,predecessor_session_id,
    intent_sha256,selection_sha256,owner_revision,semantic_revision,queue_revision,
    journal_cursor,profile_digest,binary_sha256,protocol_digest,snapshot_id,
    snapshot_manifest_sha256,state
)
SELECT n.subject_id,n.context_id,0,n.session_id,NULL,NULL,NULL,
       s.revision,s.semantic_revision,s.queue_revision,
       (SELECT COALESCE(MAX(sequence),0) FROM journal),
       n.profile_digest,n.binary_sha256,n.protocol_digest,n.snapshot_id,
       n.snapshot_manifest_sha256,'active'
FROM native_sessions n JOIN subjects s ON s.subject_id=n.subject_id
WHERE n.role IN ('domain','successor');

DROP INDEX one_native_generation_session;
CREATE UNIQUE INDEX one_active_native_execution_epoch
    ON native_execution_epochs(subject_id,context_id) WHERE state='active';
CREATE UNIQUE INDEX one_authorized_native_execution_epoch
    ON native_execution_epochs(subject_id,context_id) WHERE state='authorized';
CREATE INDEX native_execution_epoch_predecessor
    ON native_execution_epochs(predecessor_session_id);

CREATE TRIGGER custody_revision_native_execution_epochs_insert
AFTER INSERT ON native_execution_epochs
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_native_execution_epochs_update
AFTER UPDATE ON native_execution_epochs
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
CREATE TRIGGER custody_revision_native_execution_epochs_delete
AFTER DELETE ON native_execution_epochs
BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;
