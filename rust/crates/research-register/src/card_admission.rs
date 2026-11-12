//! Host-owned admission. Only the explicit synthetic qualification feature installs authority.
//! No production protection/authentication route is implemented.
use crate::{
    card_wire::CurrentCardRequest,
    use_bundle::{BlobPin, Record, ValidUseBundle, revision},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DisclosureScope {
    pub decision_ids: BTreeSet<String>,
    pub finding_ids: BTreeSet<String>,
    pub cross_decision_metadata: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalEvent {
    pub issuer: String,
    pub role: String,
    pub delegation: String,
    pub purpose: String,
    pub decision_id: String,
    pub decision_revision: String,
    pub request_sha256: String,
    pub bundle_sha256: String,
    pub records: BTreeSet<String>,
    pub scope_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InstallationData {
    pub issuers: BTreeSet<String>,
    pub delegation: String,
    pub purpose: String,
    pub scope: DisclosureScope,
    pub receipts: Vec<(BlobPin, Vec<u8>)>,
}
/// Not deserializable. A request cannot install this value.
#[derive(Debug, Clone)]
pub struct Installation {
    pub(crate) data: Option<InstallationData>,
}
impl Installation {
    pub fn unavailable() -> Self {
        Self { data: None }
    }
    /// Synthetic trusted-startup adapter; never an authenticated production installation.
    #[cfg(feature = "synthetic-card-host")]
    pub fn synthetic(
        issuers: BTreeSet<String>,
        delegation: String,
        purpose: String,
        scope: DisclosureScope,
        receipts: Vec<(BlobPin, Vec<u8>)>,
    ) -> Self {
        Self {
            data: Some(InstallationData {
                issuers,
                delegation,
                purpose,
                scope,
                receipts,
            }),
        }
    }
}
/// Internal non-deserializable proof. It cannot be created by publisher owner labels.
pub(crate) struct AdmittedUse {
    pub profile_sha256: String,
    pub receipt_pins: Vec<BlobPin>,
    pub scope: DisclosureScope,
    pub bundle_sha256: String,
    pub request: CurrentCardRequest,
}
pub(crate) fn verify(
    installation: &Installation,
    bundle: &ValidUseBundle,
    q: &CurrentCardRequest,
    objects: &crate::GitObjects,
) -> Result<AdmittedUse, String> {
    let data = installation.data.as_ref().ok_or("authority_unavailable")?;
    let expected_records = bundle.records().keys().cloned().collect::<BTreeSet<_>>();
    let scope_sha = revision(&serde_json::to_vec(&data.scope).map_err(|_| "authority_mismatch")?);
    let mut roles = BTreeSet::new();
    for (pin, bytes) in &data.receipts {
        if revision(bytes) != pin.sha256 {
            return Err("authority_mismatch".into());
        }
        let oid = gix::hash::ObjectId::from_hex(pin.commit.as_bytes())
            .map_err(|_| "authority_mismatch")?;
        match objects.blob_at(oid, &pin.path).map_err(|_| "unreadable")? {
            crate::BlobAt::Blob { id, bytes: actual }
                if id
                    == gix::hash::ObjectId::from_hex(pin.blob.as_bytes())
                        .map_err(|_| "authority_mismatch")?
                    && actual == *bytes => {}
            _ => return Err("authority_mismatch".into()),
        }
        let mut issues = vec![];
        crate::parse::document("approval", bytes, &mut issues).ok_or("authority_mismatch")?;
        let event: ApprovalEvent =
            serde_json::from_slice(bytes).map_err(|_| "authority_mismatch")?;
        if !data.issuers.contains(&event.issuer)
            || event.delegation != data.delegation
            || event.purpose != data.purpose
            || event.decision_id != q.decision_id
            || event.decision_revision != q.decision_revision
            || event.request_sha256 != q.request_sha256
            || event.bundle_sha256 != bundle.manifest_pin().sha256
            || event.records != expected_records
            || event.scope_sha256 != scope_sha
            || !["qualification", "applicability"].contains(&event.role.as_str())
            || !roles.insert(event.role)
        {
            return Err("authority_mismatch".into());
        }
    }
    if roles != BTreeSet::from(["qualification".into(), "applicability".into()]) {
        return Err("authority_mismatch".into());
    }
    let Some(stored) = bundle.records().get(&q.request_sha256) else {
        return Err("authority_mismatch".into());
    };
    if bundle.manifest().request != q.request_sha256
        || !matches!(stored.record(),Record::Request{decision,decision_revision,purpose,..} if decision==&q.decision_id && decision_revision==&q.decision_revision && purpose==&data.purpose)
    {
        return Err("authority_mismatch".into());
    }
    // Exact revisions include every study/subject, all eight facets, competing assessments,
    // qualification purpose/disposition/reviewer/capabilities and applicability relation/bounds.
    for id in &bundle.manifest().applicability {
        let Record::Applicability {
            subject,
            request,
            decision,
            decision_revision,
            ..
        } = bundle.records()[id].record()
        else {
            return Err("authority_mismatch".into());
        };
        if request != &q.request_sha256
            || decision != &q.decision_id
            || decision_revision != &q.decision_revision
        {
            return Err("authority_mismatch".into());
        }
        let mut qualified = false;
        for assessment in bundle.records().values() {
            if let Record::Assessment {
                subject: s,
                qualification,
                ..
            } = assessment.record()
                && s == subject
            {
                for k in qualification {
                    if matches!(bundle.records()[k].record(),Record::Qualification{purpose,disposition,..} if purpose==&data.purpose && disposition=="accepted")
                    {
                        qualified = true;
                    }
                }
            }
        }
        if !qualified {
            return Err("authority_mismatch".into());
        }
    }
    Ok(AdmittedUse {
        profile_sha256: revision(&serde_json::to_vec(data).map_err(|_| "authority_mismatch")?),
        receipt_pins: data.receipts.iter().map(|(p, _)| p.clone()).collect(),
        scope: data.scope.clone(),
        bundle_sha256: bundle.manifest_pin().sha256.clone(),
        request: q.clone(),
    })
}
