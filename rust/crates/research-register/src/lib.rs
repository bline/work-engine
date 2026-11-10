//! Commit-pinned reading of the research register.
//!
//! Two typed layers. [`GitObjects`] reads Git objects from one repository and checks every
//! commit, tree, blob and tag it returns: the object's type must be the type requested, and its
//! canonical hash (`"<type> <length>\0<contents>"`) must equal the requested id. Asking gix for
//! original objects (`core.useReplaceRefs=false`) is a preference; these checks are the boundary.
//! [`PinnedSnapshot`] is built only through [`GitObjects`] and never changes afterwards.
//!
//! Register validation, anchors and decision cards are not part of this crate yet.

pub mod error;
pub mod objects;
pub mod snapshot;

pub use error::{BadRequest, CannotRead, Error, NoRegister, Unsupported};
pub use objects::{Ancestry, BlobAt, Commit, EntryKind, GitObjects, Prefix, TreeEntry};
pub use snapshot::{
    EntryIdentity, PinnedSnapshot, SNAPSHOT_SCHEMA, SnapshotEntry, SnapshotIdentity,
};
