//! Error model (contract S8): the same `{valid, kind, errors[{code, at, message}]}` report and
//! exit codes as `proposal-evaluation`, re-exported so one report type crosses both crates.
//! The codes below are this crate's; shared S8 codes keep their `proposal-evaluation` names.

pub use proposal_evaluation::diag::{
    CheckReport, EXIT_BINDING, EXIT_INVALID, EXIT_OK, EXIT_USAGE, ErrorItem, Failure,
};

/// Stable error codes. Each rule has exactly one code.
pub mod codes {
    // Shared S8 codes, with the names `proposal-evaluation` uses.
    /// Contract 4.2 step 4 / Erratum 3 reuse the code comparisons already use.
    pub use proposal_evaluation::diag::codes::BASIS_REVISION_MISMATCH;
    pub use proposal_evaluation::diag::codes::{
        AUTHORITY_ASSERTED, DIGEST_MISMATCH, DUPLICATE_PROPOSAL_ID, EMPTY_STRING, ID_PATTERN,
        LIMITATION_MISSING, PATH_NOT_ABSOLUTE, PATH_NOT_NORMALISED, POINTER_UNRESOLVED,
        PROPOSAL_ID_MISMATCH, SCHEMA, SCHEMA_MISMATCH, SHA256_PATTERN, TIME_FORMAT,
        UPSTREAM_CLASS_MISMATCH,
    };

    // Contract 4.4.
    pub const SUPERSESSION_INVALID: &str = "supersession_invalid";
    pub const READINESS_PROPOSAL_UNKNOWN: &str = "readiness_proposal_unknown";
    pub const DISPOSITION_MISSING: &str = "disposition_missing";
    pub const DISPOSITION_DUPLICATE: &str = "disposition_duplicate";
    pub const DISPOSITION_UNKNOWN_PROPOSAL: &str = "disposition_unknown_proposal";
    pub const RANK_DUPLICATE: &str = "rank_duplicate";
    pub const SEQUENCE_TARGET_INVALID: &str = "sequence_target_invalid";
    pub const SEQUENCE_CYCLE: &str = "sequence_cycle";
    pub const UNLOCK_ENDPOINT_UNKNOWN: &str = "unlock_endpoint_unknown";
    pub const RELATIONSHIP_BASIS_ABSENT: &str = "relationship_basis_absent";
    pub const UNLOCK_TYPE_INVALID: &str = "unlock_type_invalid";
    pub const EXCLUSION_MEMBER_INVALID: &str = "exclusion_member_invalid";
    pub const EVALUATION_BASIS_REDERIVE_MISMATCH: &str = "evaluation_basis_rederive_mismatch";
    pub const EVALUATION_BASIS_INVALID: &str = "evaluation_basis_invalid";
    pub const ASSESSMENT_NOT_LIVE: &str = "assessment_not_live";

    // Errata 4-7.
    pub const DECISION_BINDING: &str = "decision_binding";
    pub const RELATIONSHIP_BASIS_DISCONNECTED: &str = "relationship_basis_disconnected";
    pub const READINESS_DECISION_MISMATCH: &str = "readiness_decision_mismatch";
    pub const EXCLUSION_CONTRADICTED: &str = "exclusion_contradicted";
}

/// Required limitation names (contract 4.1; Erratum 6).
pub mod limitations {
    pub const STRATEGIC_BASIS_UNVERIFIED: &str = "strategic_basis_unverified";
    pub const RELATIONSHIP_GRAPH_REVISION_UNVERIFIED: &str =
        "relationship_graph_revision_unverified";
    pub const NO_EVALUATION_BASIS: &str = "no_evaluation_basis";
    pub const READINESS_DECISION_FIT_UNVERIFIED: &str = "readiness_decision_fit_unverified";
}
