//! Fixture-only in-memory observation source (exists only with the `fixture` feature).
//!
//! Synthetic: it stands in for a protected authority source in tests. Every mutation is
//! immediately "durable" and advances the revision by one, so it meets the four source
//! obligations by construction. `inject_unknown` makes every observation return
//! [`Unobserved`] until cleared.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::ids::{Enc, IssuanceId};
use crate::observation::{AuthorityObservation, AuthorityState, Entry, IssuanceStatus, Unobserved};
use crate::records::Record;

#[derive(Debug, Default)]
struct Inner {
    sequence: u64,
    entries: BTreeMap<IssuanceId, Entry>,
    unknown: Option<String>,
}

/// An in-memory source supporting committed, revoked and superseded records and `Unknown`
/// injection.
#[derive(Debug, Default)]
pub struct FixtureSource {
    inner: RefCell<Inner>,
}

impl FixtureSource {
    pub fn new() -> Self {
        Self::default()
    }

    /// Issue a record. Returns a fresh issuance identity, distinct even for identical terms.
    pub fn issue(&self, record: Record) -> IssuanceId {
        let mut inner = self.inner.borrow_mut();
        inner.sequence += 1;
        let body = record.body_digest();
        let id = IssuanceId::from_bytes(
            Enc::new("fixture_issuance")
                .u64(inner.sequence)
                .bytes(body.as_bytes())
                .finish(),
        );
        inner.entries.insert(
            id,
            Entry {
                record,
                body,
                status: IssuanceStatus::Committed,
            },
        );
        id
    }

    /// Revoke one issuance. Panics if it is not committed (a writer refuses such a revoke).
    pub fn revoke(&self, id: &IssuanceId) {
        let mut inner = self.inner.borrow_mut();
        let entry = inner.entries.get_mut(id).expect("revoke names an issuance");
        assert_eq!(
            entry.status,
            IssuanceStatus::Committed,
            "revoke target is committed"
        );
        entry.status = IssuanceStatus::Revoked;
        inner.sequence += 1;
    }

    /// Supersede one committed issuance by a new one carrying `record`.
    pub fn supersede(&self, old: &IssuanceId, record: Record) -> IssuanceId {
        let new = self.issue(record);
        let mut inner = self.inner.borrow_mut();
        let entry = inner
            .entries
            .get_mut(old)
            .expect("supersede names an issuance");
        assert_eq!(
            entry.status,
            IssuanceStatus::Committed,
            "superseded issuance is committed"
        );
        entry.status = IssuanceStatus::Superseded { by: new };
        new
    }

    /// Make every observation return `Unobserved` (write in progress, read error, torn record,
    /// unavailable source) until [`FixtureSource::clear_unknown`].
    pub fn inject_unknown(&self, detail: &str) {
        self.inner.borrow_mut().unknown = Some(detail.to_owned());
    }

    pub fn clear_unknown(&self) {
        self.inner.borrow_mut().unknown = None;
    }
}

impl AuthorityObservation for FixtureSource {
    fn observe(&self) -> Result<AuthorityState, Unobserved> {
        let inner = self.inner.borrow();
        if let Some(detail) = &inner.unknown {
            return Err(Unobserved {
                detail: detail.clone(),
            });
        }
        Ok(AuthorityState::new(inner.sequence, inner.entries.clone()))
    }
}
