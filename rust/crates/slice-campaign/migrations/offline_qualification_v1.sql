-- Explicitly attached only by the offline qualification fixture. This
-- extension does not migrate existing campaign profiles or change user_version.
CREATE TABLE offline_qualification_schema (
    version INTEGER PRIMARY KEY CHECK (version = 1),
    joined INTEGER NOT NULL CHECK (joined IN (0, 1))
) STRICT;

CREATE TABLE offline_qualification_selection (
    grant_id TEXT PRIMARY KEY CHECK (length(grant_id) = 64),
    root_id TEXT NOT NULL,
    identity_key TEXT NOT NULL REFERENCES campaign_state(identity_key),
    head TEXT NOT NULL CHECK (length(head) = 64),
    attempt_id TEXT NOT NULL UNIQUE CHECK (length(attempt_id) = 64),
    binding_digest TEXT NOT NULL CHECK (length(binding_digest) = 64),
    expires_at INTEGER NOT NULL CHECK (expires_at > 0),
    store_profile TEXT NOT NULL,
    opening_epoch TEXT NOT NULL,
    issuer_id TEXT NOT NULL,
    purpose TEXT NOT NULL,
    recipe_digest TEXT NOT NULL CHECK (length(recipe_digest) = 64),
    server_operation TEXT NOT NULL,
    serving_generation TEXT NOT NULL CHECK (length(serving_generation) = 64),
    evidence_ingress TEXT NOT NULL CHECK (length(evidence_ingress) = 64),
    entered INTEGER NOT NULL DEFAULT 0 CHECK (entered IN (0, 1)),
    revoked INTEGER NOT NULL DEFAULT 0 CHECK (revoked IN (0, 1)),
    UNIQUE (grant_id, attempt_id, binding_digest)
) STRICT;

-- Immutable exact spend fact. A lifecycle consumer must join by the complete
-- spend binding, not infer spend from existence of a selection or entry row.
CREATE TABLE offline_qualification_spend (
    grant_id TEXT PRIMARY KEY,
    attempt_id TEXT NOT NULL,
    binding_digest TEXT NOT NULL,
    spend_receipt TEXT NOT NULL UNIQUE CHECK (length(spend_receipt) = 64),
    FOREIGN KEY (grant_id, attempt_id, binding_digest)
        REFERENCES offline_qualification_selection(grant_id, attempt_id, binding_digest)
) STRICT;
