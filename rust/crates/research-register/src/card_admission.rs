//! Host-owned admission. Only the explicit synthetic qualification feature installs authority.
//! No production protection/authentication route is implemented.
use crate::{
    card_wire::CurrentCardRequest,
    use_bundle::{BlobPin, Record, ValidUseBundle, revision},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_issuers: Option<BTreeMap<String, String>>,
}
/// Not deserializable. A request cannot install this value.
#[derive(Debug, Clone)]
pub struct Installation {
    pub(crate) data: Option<InstallationData>,
}
/// Pure checked data, without authentication or permission to use it.
/// Private fields prevent callers from skipping record validation.
#[derive(Debug, Clone)]
pub struct ValidatedInstallationRecords {
    data: InstallationData,
}

impl Installation {
    pub fn unavailable() -> Self {
        Self { data: None }
    }
    /// Internal trusted-composition installer. The caller must already hold the
    /// actual externally authenticated owner-admitted startup consequence for
    /// these exact sources, roles and scope. Deserializing records, checking UID
    /// or matching hashes does not satisfy that caller contract. No socket/native
    /// operation exposes this Rust entry. `verify` still checks exact Git objects,
    /// bundle/request/use/scope and role approvals before producing any card.
    pub fn install_from_admitted_startup(records: ValidatedInstallationRecords) -> Self {
        Self {
            data: Some(records.data),
        }
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
                role_issuers: None,
            }),
        }
    }
}
pub fn validate_installation_records(
    delegation: String,
    purpose: String,
    scope: DisclosureScope,
    receipts: Vec<(String, String, BlobPin, Vec<u8>)>,
) -> Result<ValidatedInstallationRecords, String> {
    let bounded = |s: &str| !s.is_empty() && s.len() <= 128 && !s.contains('\0');
    if delegation.is_empty()
        || delegation.len() > 8192
        || delegation.contains('\0')
        || purpose.is_empty()
        || purpose.len() > 8192
        || purpose.contains('\0')
        || receipts.len() != 2
        || scope.decision_ids.len() > 256
        || scope.finding_ids.len() > 256
        || scope
            .decision_ids
            .iter()
            .chain(&scope.finding_ids)
            .any(|s| !bounded(s))
    {
        return Err("authority_mismatch".into());
    }
    let mut role_issuers = BTreeMap::new();
    let mut retained = Vec::new();
    let mut issuers = BTreeSet::new();
    for (role, issuer, pin, bytes) in receipts {
        if !matches!(role.as_str(), "qualification" | "applicability")
            || !bounded(&issuer)
            || role_issuers.insert(role.clone(), issuer.clone()).is_some()
            || bytes.is_empty()
            || bytes.len() > 128 * 1024
            || revision(&bytes) != pin.sha256
            || pin.commit.len() != 40
            || pin.blob.len() != 40
            || !pin
                .commit
                .bytes()
                .chain(pin.blob.bytes())
                .all(|b| b.is_ascii_hexdigit())
            || pin.path.contains('\0')
            || pin.path.is_empty()
            || pin.path.len() > 8192
            || std::path::Path::new(&pin.path).is_absolute()
            || std::path::Path::new(&pin.path)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("authority_mismatch".into());
        }
        let mut issues = vec![];
        crate::parse::document("approval", &bytes, &mut issues).ok_or("authority_mismatch")?;
        let event: ApprovalEvent =
            serde_json::from_slice(&bytes).map_err(|_| "authority_mismatch")?;
        if event.role != role
            || event.issuer != issuer
            || event.delegation != delegation
            || event.purpose != purpose
        {
            return Err("authority_mismatch".into());
        }
        issuers.insert(issuer);
        retained.push((pin, bytes));
    }
    Ok(ValidatedInstallationRecords {
        data: InstallationData {
            issuers,
            delegation,
            purpose,
            scope,
            receipts: retained,
            role_issuers: Some(role_issuers),
        },
    })
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
        if data
            .role_issuers
            .as_ref()
            .is_some_and(|roles| roles.get(&event.role) != Some(&event.issuer))
            || !data.issuers.contains(&event.issuer)
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
