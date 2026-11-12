//! Commit-pinned reading of the research register.
//!
//! Two typed layers. [`GitObjects`] reads Git objects from one repository and checks every
//! commit, tree, blob and tag it returns: the object's type must be the type requested, and its
//! canonical hash (`"<type> <length>\0<contents>"`) must equal the requested id. Asking gix for
//! original objects (`core.useReplaceRefs=false`) is a preference; these checks are the boundary.
//! [`PinnedSnapshot`] is built only through [`GitObjects`] and never changes afterwards.
//!
//! [`model::decode`] parses the register files ([`parse`]) and decodes typed records with their
//! Git-free checks; violations carry a [`RuleId`]. [`validate`] checks a whole snapshot, including
//! the Git-backed rules, and yields a [`ValidRegister`] or the violations. [`anchor::anchors`]
//! classifies each decision's anchor of a [`ValidRegister`] at its own commit (read-only; rename
//! detection is an open owner question, Q-R2-2). Decision cards are not part of this crate yet.

pub mod anchor;
pub mod bounded_read;
pub mod error;
pub mod model;
pub mod objects;
pub mod parse;
pub mod read_host;
pub mod rules;
pub mod snapshot;
pub mod use_bundle;
pub mod validate;

pub use anchor::{AnchorClass, AnchorEntry, AnchorError, AnchorReport, anchors};
pub use error::{BadRequest, CannotRead, Error, NoRegister, Unsupported};
pub use model::{Decoded, Index, Records, decode};
pub use objects::{
    Ancestry, BlobAt, Commit, EntryKind, GitObjects, Prefix, TreeEntry, TreeListing,
};
pub use parse::{Json, Number};
pub use rules::{Location, RuleId, Violation};
pub use snapshot::{
    EntryIdentity, PinnedSnapshot, SNAPSHOT_SCHEMA, SnapshotEntry, SnapshotIdentity,
};
pub use validate::{Outcome, Unreadable, ValidRegister, validate, validate_report};
