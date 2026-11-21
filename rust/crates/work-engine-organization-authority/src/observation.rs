//! Authority observation and the reservation seal (addendum items 3 and 5; 6.1 F3).

use std::collections::BTreeMap;

use crate::ids::{BodyDigest, Boundary, Enc, IssuanceId, ObservationRevision};
use crate::records::Record;

/// The standing of one issuance in one observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssuanceStatus {
    /// Issued and durable, not revoked, not superseded.
    Committed,
    /// Withdrawn by a durable revocation, or by a revocation whose outcome is uncertain (an
    /// uncertain revocation keeps fencing).
    Revoked,
    /// Replaced by a later issuance. The terms are no longer current; this is not a revocation.
    Superseded { by: IssuanceId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) record: Record,
    pub(crate) body: BodyDigest,
    pub(crate) status: IssuanceStatus,
}

/// How an observation resolves a referenced issuance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution<'a> {
    Committed(&'a Record),
    Revoked,
    Superseded {
        by: IssuanceId,
    },
    /// No such issuance at this revision.
    Missing,
}

/// One state of organizational authority at one revision. It has no public constructor: only an
/// observation source in this crate produces one.
#[derive(Debug, PartialEq, Eq)]
pub struct AuthorityState {
    revision: ObservationRevision,
    entries: BTreeMap<IssuanceId, Entry>,
}

impl AuthorityState {
    /// Builds a state and computes its revision digest from its entries.
    #[cfg_attr(not(feature = "fixture"), allow(dead_code))]
    pub(crate) fn new(sequence: u64, entries: BTreeMap<IssuanceId, Entry>) -> Self {
        let mut enc = Enc::new("authority_state");
        enc.u64(sequence).u64(entries.len() as u64);
        for (id, entry) in &entries {
            enc.bytes(id.as_bytes()).bytes(entry.body.as_bytes());
            match &entry.status {
                IssuanceStatus::Committed => enc.u64(0),
                IssuanceStatus::Revoked => enc.u64(1),
                IssuanceStatus::Superseded { by } => enc.u64(2).bytes(by.as_bytes()),
            };
        }
        AuthorityState {
            revision: ObservationRevision::new(sequence, enc.finish()),
            entries,
        }
    }

    pub fn revision(&self) -> ObservationRevision {
        self.revision
    }

    pub fn resolve(&self, id: &IssuanceId) -> Resolution<'_> {
        match self.entries.get(id) {
            None => Resolution::Missing,
            Some(entry) => match &entry.status {
                IssuanceStatus::Committed => Resolution::Committed(&entry.record),
                IssuanceStatus::Revoked => Resolution::Revoked,
                IssuanceStatus::Superseded { by } => Resolution::Superseded { by: *by },
            },
        }
    }

    pub fn body_digest(&self, id: &IssuanceId) -> Option<BodyDigest> {
        self.entries.get(id).map(|entry| entry.body)
    }

    /// Look up both the historical record and its standing in this same observation.
    #[cfg(feature = "fixture")]
    pub(crate) fn observed_entry(&self, id: &IssuanceId) -> Option<&Entry> {
        self.entries.get(id)
    }

    /// Every issuance with its record and status, in issuance-id order.
    pub(crate) fn entries(&self) -> impl Iterator<Item = (&IssuanceId, &Entry)> {
        self.entries.iter()
    }
}

/// No observation exists: the source is unavailable, a read failed, a record is torn, or the
/// state is unpublished or uncertain. No gate outcome exists without an observation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("no authority observation: {detail}")]
pub struct Unobserved {
    pub detail: String,
}

/// A source of organizational-authority observations.
///
/// # Source obligations (the contract every implementation must meet)
///
/// These four obligations are acceptance criteria for any future source (owner decision O6).
/// No production source exists; the in-memory fixture source meets them by construction. They
/// are contract text here, not properties this trait can enforce.
///
/// 1. **Committed only once durable.** An issuance is reported [`IssuanceStatus::Committed`] only
///    once its issuing is durable. A state whose durability is unknown is not reported: the
///    source returns [`Unobserved`].
/// 2. **Never retracted, totally ordered.** Revisions are totally ordered by
///    [`ObservationRevision::sequence`]. A reported revision is never retracted by later
///    recovery: a later observation never reports a lower sequence, never reports a different
///    state at a reported sequence, and never loses an issuance it reported.
/// 3. **Uncertain revocation keeps fencing.** A revocation whose outcome is uncertain is reported
///    as [`IssuanceStatus::Revoked`] (or the source reports [`Unobserved`]); it never reverts to
///    committed without the owner's explicit decision.
/// 4. **One observation is one state.** Every resolution in one [`AuthorityState`] belongs to the
///    single state at its revision, never a mixture of reads at different times.
///
/// How a source achieves these (serialized log, protected service, or anything else) is the
/// later storage decision's; this contract states outcomes, not mechanism.
pub trait AuthorityObservation {
    /// One observation, or no observation.
    fn observe(&self) -> Result<AuthorityState, Unobserved>;
}

/// A reservation of one effect boundary by the host that owns it. The host allocates the epoch
/// and records single use; this value only names the boundary an observation is sealed to.
/// Not `Clone`: a reservation is a single act.
#[derive(Debug, PartialEq, Eq)]
pub struct Reservation {
    boundary: Boundary,
}

impl Reservation {
    pub fn boundary(&self) -> &Boundary {
        &self.boundary
    }
}

/// Reserve a boundary (6.1 F3 step 1). The host owns epoch allocation and single use.
pub fn reserve(boundary: Boundary) -> Reservation {
    Reservation { boundary }
}

/// An observation sealed to one reservation's boundary (6.1 F3 step 2). Gates decide only the
/// boundary it is sealed to, so one observation cannot decide a later boundary. Not `Clone`, no
/// public constructor.
#[derive(Debug)]
pub struct SealedObservation {
    boundary: Boundary,
    state: AuthorityState,
}

impl SealedObservation {
    pub fn boundary(&self) -> &Boundary {
        &self.boundary
    }
    pub fn state(&self) -> &AuthorityState {
        &self.state
    }
    pub fn revision(&self) -> ObservationRevision {
        self.state.revision
    }
}

/// Observe for a reservation: one observation from the source, sealed to the reservation's
/// boundary.
pub fn observe_for<S: AuthorityObservation + ?Sized>(
    source: &S,
    reservation: &Reservation,
) -> Result<SealedObservation, Unobserved> {
    let state = source.observe()?;
    Ok(SealedObservation {
        boundary: reservation.boundary.clone(),
        state,
    })
}
