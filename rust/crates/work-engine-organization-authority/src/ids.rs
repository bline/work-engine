//! Identities, defined by property rather than by any log layout (review F-10).

use std::fmt;

use sha2::{Digest as _, Sha256};

/// Domain separator for every digest this crate computes.
pub(crate) const DOMAIN: &str = "work-engine.organization-authority";

/// A small length-prefixed encoder: the canonical bytes of a record body or state for hashing.
/// It is not a wire or storage codec; storage chooses its own codec later.
pub(crate) struct Enc(Sha256);

impl Enc {
    pub(crate) fn new(kind: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN.as_bytes());
        hasher.update([0u8]);
        hasher.update(kind.as_bytes());
        hasher.update([0u8]);
        hasher.update(b"v1");
        hasher.update([0u8]);
        Enc(hasher)
    }
    pub(crate) fn str(&mut self, text: &str) -> &mut Self {
        self.u64(text.len() as u64);
        self.0.update(text.as_bytes());
        self
    }
    pub(crate) fn u64(&mut self, value: u64) -> &mut Self {
        self.0.update(value.to_be_bytes());
        self
    }
    pub(crate) fn bytes(&mut self, value: &[u8; 32]) -> &mut Self {
        self.0.update(value);
        self
    }
    pub(crate) fn opt_u64(&mut self, value: Option<u64>) -> &mut Self {
        match value {
            None => self.u64(0),
            Some(v) => self.u64(1).u64(v),
        }
    }
    pub(crate) fn strs<'a>(&mut self, items: impl ExactSizeIterator<Item = &'a str>) -> &mut Self {
        self.u64(items.len() as u64);
        for item in items {
            self.str(item);
        }
        self
    }
    pub(crate) fn finish(&mut self) -> [u8; 32] {
        std::mem::take(&mut self.0).finalize().into()
    }
}

fn hex(bytes: &[u8; 32], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }
    Ok(())
}

/// Issuance identity: "which act of issuing". Unique per act of issuing, even when two issuances
/// carry identical terms (6.1 F5). Never interchangeable with [`BodyDigest`]. Only an observation
/// source in this crate assigns one.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IssuanceId([u8; 32]);

impl IssuanceId {
    #[cfg_attr(not(feature = "fixture"), allow(dead_code))]
    pub(crate) fn from_bytes(bytes: [u8; 32]) -> Self {
        IssuanceId(bytes)
    }
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for IssuanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("IssuanceId(")?;
        hex(&self.0, f)?;
        f.write_str(")")
    }
}

/// Content identity: "which terms". The digest of a record body's canonical bytes in this crate's
/// domain with the record kind. Display and comparison only; no cross-reference uses it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BodyDigest(pub(crate) [u8; 32]);

impl BodyDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for BodyDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BodyDigest(")?;
        hex(&self.0, f)?;
        f.write_str(")")
    }
}

/// One revision of organizational authority: a position in a total order plus the digest of the
/// one state at that position. Two observations at the same `sequence` with different
/// `state_digest`s violate the source contract (one revision is one state).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObservationRevision {
    sequence: u64,
    state_digest: [u8; 32],
}

impl ObservationRevision {
    pub(crate) fn new(sequence: u64, state_digest: [u8; 32]) -> Self {
        ObservationRevision {
            sequence,
            state_digest,
        }
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn state_digest(&self) -> &[u8; 32] {
        &self.state_digest
    }
}

impl fmt::Debug for ObservationRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObservationRevision({}, ", self.sequence)?;
        hex(&self.state_digest, f)?;
        f.write_str(")")
    }
}

/// The kind of effect boundary. Delivery (spawn and contract exposure) and consumption are
/// separate boundaries (owner decision O4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoundaryKind {
    Delivery,
    Consumption,
}

/// One effect boundary `(operation, kind, epoch)`. The consuming host allocates `epoch`, strictly
/// increasing per `(operation, kind)`; this crate does not allocate or record epochs (single use
/// of a boundary is the host's, owner decision O10).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Boundary {
    operation: String,
    kind: BoundaryKind,
    epoch: u64,
}

impl Boundary {
    pub fn new(operation: impl Into<String>, kind: BoundaryKind, epoch: u64) -> Self {
        Boundary {
            operation: operation.into(),
            kind,
            epoch,
        }
    }
    pub fn operation(&self) -> &str {
        &self.operation
    }
    pub fn kind(&self) -> BoundaryKind {
        self.kind
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    /// Same operation and kind, later epoch.
    pub(crate) fn supersedes(&self, earlier: &Boundary) -> bool {
        self.operation == earlier.operation
            && self.kind == earlier.kind
            && self.epoch > earlier.epoch
    }
}

/// A concrete realization's identity, as the host computes it. An opaque value: its definition for
/// any realization is outside this crate. A value, not authority.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RealizationDigest(String);

impl RealizationDigest {
    pub fn new(value: impl Into<String>) -> Self {
        RealizationDigest(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An installed executable's digest (the installation pin), as the host reports it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutableDigest(String);

impl ExecutableDigest {
    pub fn new(value: impl Into<String>) -> Self {
        ExecutableDigest(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
