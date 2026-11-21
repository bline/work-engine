//! Pure organizational compilation (Slice 1A).
//!
//! This crate is meaning only: it has no I/O, no clock, no environment access and
//! no path to any effect. Its identities are digests of a canonical form
//! (contract revision 5.1, section 17).
//!
//! The canonicalizer and its value type are crate-private. The public surface is the interface
//! of contract section 10.1 and the types needed to name its arguments, results and errors.

mod derive;
mod digest;
mod enforce;
mod error;
mod ids;
mod json;
mod schema;
mod view;

pub use derive::{
    AuthorityRequirement, CandidateCondition, ComponentBindingRequirement, DerivationResult,
    OrganizationCandidate, Participant, ParticipantKind, Receipt, Relationship, RelationshipKind,
    SeparationRequirement, TraceRef, TraceRow, Unresolved, UnresolvedKind, UnresolvedReason,
    VantageRequirement, consumer_class_of, derive,
};
pub use digest::{CODEC_DOMAIN, CODEC_VERSION, Digest, DigestKind};
pub use enforce::{
    EnforceStatus, EnforceabilityReport, EnforceabilityRow, EvidenceClass, enforceability,
};
pub use error::{CodecError, IdError, Position, ValidationCode, ValidationError};
pub use ids::Id;
pub use schema::{
    Condition, Continuity, Dimension, EvidenceCustody, ExecutionConfinement, IndependenceSource,
    Integrity, Mutation, Normative, Obligation, ObserverClass, OrganizationalRequirements,
    ProblemScope, Profile, ProviderFamily, RealizationClass, RealizationConstraint,
    RequiredAuthority, ResultContract, Vocabulary,
};
pub use view::{
    EpisodeAccess, Hosting, OrganizationView, Terminalization, ViewError, ViewObligation,
    ViewVantage, view,
};

#[cfg(test)]
mod codec_rules_tests;
#[cfg(test)]
mod rfc_vectors_tests;
