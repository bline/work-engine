//! Unselected controlled pre-build startup. Only trusted owner composition may
//! supply the out-of-band current selection; request bytes cannot select
//! themselves. There is no installed selection source or runtime caller.
use std::path::{Path, PathBuf};
use std::sync::MutexGuard;

use serde::Deserialize;

use super::{AdmitRequest, Campaign, CampaignInner, TrustedConfig};
use crate::codec::raw_sha256;
use crate::contract::{
    AcceptedBoundary, AdvancePhase, Baseline, CampaignIdentity, CampaignRef, Consequence, Snapshot,
};
use crate::recovery::{Effect, OperationReceipt, RecoveryLocator};
use crate::{CampaignError, Result, require_sha, require_text};

pub(crate) const PROFILE: &str = "campaign-controlled-prebuild-v4";
const ADMIT: &str = "controlled-prebuild/admit";
const IMPLEMENTING: &str = "controlled-prebuild/implementing";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Selection {
    schema_version: u32,
    root: PathBuf,
    repository: PathBuf,
    workspace: PathBuf,
    identity: CampaignIdentity,
    accepted_boundary: AcceptedBoundary,
    baseline: Baseline,
    writer_identity: String,
}

/// Facts from this owner's fresh `Accepted → Implementing` commit. The
/// receipt's profile digest pins the separately checked selection document.
/// This does not prove later currentness or authorize Builder admission.
///
/// A reopened observation cannot substitute for this result, even if it
/// reports the same head:
///
/// ```compile_fail,E0308
/// use slice_campaign::prebuild::{FreshImplementing, Implementing};
/// fn relabel(observation: Implementing) -> FreshImplementing {
///     observation
/// }
/// ```
///
/// The fact is not a deserializable document:
///
/// ```compile_fail,E0277
/// let _ = serde_json::from_str::<slice_campaign::prebuild::FreshImplementing>("{}");
/// ```
#[derive(Debug)]
pub struct FreshImplementing {
    anchored_root: PathBuf,
    reference: CampaignRef,
    snapshot: Snapshot,
    receipt: OperationReceipt,
}

impl FreshImplementing {
    pub fn anchored_root(&self) -> &Path {
        &self.anchored_root
    }

    pub fn reference(&self) -> &CampaignRef {
        &self.reference
    }

    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub fn receipt(&self) -> &OperationReceipt {
        &self.receipt
    }
}

/// Observation from reopening, not evidence of a fresh commit or a grant.
#[derive(Debug)]
pub struct Implementing {
    pub reference: CampaignRef,
    pub snapshot: Snapshot,
}

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error(transparent)]
    Campaign(#[from] CampaignError),
    #[error("{step} refused: {reason}")]
    Refused { step: &'static str, reason: String },
    #[error("{step} outcome unknown; reconcile the existing root, do not recreate it")]
    OutcomeUnknown {
        step: &'static str,
        locator: Box<RecoveryLocator>,
    },
}

fn config(owner_selected_sha256: Option<&str>, bytes: &[u8]) -> Result<TrustedConfig> {
    let digest = owner_selected_sha256.ok_or_else(|| {
        CampaignError::Unsupported("controlled pre-build selection is not configured".into())
    })?;
    require_sha(digest, "independent owner selection digest")?;
    if bytes.len() > 16 * 1024 || raw_sha256(bytes) != digest {
        return Err(CampaignError::Contract(
            "pre-build document differs from the current owner selection".into(),
        ));
    }
    let selection: Selection = serde_json::from_slice(bytes)
        .map_err(|error| CampaignError::Contract(error.to_string()))?;
    if selection.schema_version != 1 {
        return Err(CampaignError::Unsupported(
            "pre-build selection version is unsupported".into(),
        ));
    }
    selection.identity.validate()?;
    require_text(&selection.writer_identity, "writer identity")?;
    require_text(
        &selection.accepted_boundary.reference,
        "accepted objective reference",
    )?;
    require_sha(
        &selection.accepted_boundary.sha256,
        "accepted objective digest",
    )?;
    if !selection.workspace.is_dir()
        || !selection.workspace.is_absolute()
        || std::fs::canonicalize(&selection.workspace).ok().as_deref()
            != Some(Path::new(&selection.workspace))
    {
        return Err(CampaignError::Root("workspace is not anchored".into()));
    }
    crate::subject_binding::verify_prebuild_baseline(&selection.repository, &selection.baseline)?;
    Ok(TrustedConfig {
        schema_version: 4,
        profile: PROFILE.into(),
        acceptance_digest: None,
        root: selection.root,
        repository: selection.repository,
        workspace: selection.workspace,
        identity: selection.identity,
        accepted_boundary: selection.accepted_boundary,
        baseline: selection.baseline,
        writer_identity: selection.writer_identity,
        gate: None,
        digest: digest.into(),
    })
}

/// Enter a fresh root declared by the exact owner-pinned document. The digest
/// must come from a separately protected, current source, not the document
/// submitter. Missing selection refuses before opening or creating a store.
/// An existing root is never recreated or taken over by this entry point.
pub fn start(
    owner_selected_sha256: Option<&str>,
    document: &[u8],
) -> std::result::Result<FreshImplementing, StartupError> {
    let config = config(owner_selected_sha256, document)?;
    let identity = config.identity.clone();
    let consequence = Consequence {
        owner: "slice-supervisor".into(),
        reference: config.accepted_boundary.reference.clone(),
        sha256: config.accepted_boundary.sha256.clone(),
    };
    let admission = AdmitRequest {
        identity: identity.clone(),
        repository: config.repository.clone(),
        workspace: config.workspace.clone(),
        accepted_boundary: config.accepted_boundary.clone(),
        baseline: config.baseline.clone(),
        expected_impact: None,
    };
    let campaign = Campaign::initialize(config)?;
    let mut owner = campaign.owner()?;
    let (admitted, _) = newly_applied(ADMIT, owner.admit(ADMIT, admission)?)?;
    let (snapshot, receipt) = newly_applied(
        IMPLEMENTING,
        owner.advance(
            &identity,
            &admitted.revision,
            IMPLEMENTING,
            AdvancePhase::Implementing,
            consequence,
        )?,
    )?;
    #[cfg(test)]
    tests::record_owner_state(&owner, &snapshot, &receipt);
    // Bind the result of this commit, not a subsequent Store observation.
    let fresh = FreshImplementing {
        anchored_root: owner.store.root.clone(),
        reference: CampaignRef {
            root_id: owner.store.root_id.clone(),
            identity: snapshot.identity.clone(),
            revision: snapshot.revision.clone(),
        },
        snapshot,
        receipt,
    };
    let detached = finish_under_owner(owner, fresh);
    drop(campaign);
    Ok(detached)
}

// This private continuation receives the same guard that made the commit.
// There is no admission decision here; the only output is historical fact.
fn finish_under_owner(
    owner: MutexGuard<'_, CampaignInner>,
    fresh: FreshImplementing,
) -> FreshImplementing {
    #[cfg(test)]
    tests::record_owner_state(&owner, &fresh.snapshot, &fresh.receipt);
    drop(owner);
    fresh
}

/// Open the same campaign owner under the current pin and read back the exact
/// head. Does not run admission again, adopt historical replay, or advance.
pub fn reopen(
    owner_selected_sha256: Option<&str>,
    document: &[u8],
    expected: &CampaignRef,
) -> Result<Implementing> {
    let config = config(owner_selected_sha256, document)?;
    if config.identity != expected.identity {
        return Err(CampaignError::Conflict(
            "expected campaign is outside the owner selection".into(),
        ));
    }
    let campaign = Campaign::open(config)?;
    observe(&campaign, expected)
}

fn newly_applied(
    step: &'static str,
    effect: Effect<Snapshot>,
) -> std::result::Result<(Snapshot, OperationReceipt), StartupError> {
    match effect {
        Effect::Applied { value, receipt } => Ok((value, receipt)),
        Effect::NoEffect(reason) => Err(StartupError::Refused { step, reason }),
        Effect::Replayed { .. } => Err(StartupError::Refused {
            step,
            reason: "expected a fresh startup operation".into(),
        }),
        Effect::OutcomeUnknown(locator) => Err(StartupError::OutcomeUnknown {
            step,
            locator: Box::new(locator),
        }),
    }
}

fn observe(campaign: &Campaign, expected: &CampaignRef) -> Result<Implementing> {
    if expected.root_id != campaign.root_id() {
        return Err(CampaignError::Conflict("campaign root differs".into()));
    }
    let snapshot = campaign
        .read(&expected.identity)?
        .ok_or_else(|| CampaignError::Conflict("no admitted campaign".into()))?;
    if snapshot.revision != expected.revision || snapshot.phase != "implementing" {
        return Err(CampaignError::Conflict(
            "expected current implementing revision differs".into(),
        ));
    }
    Ok(Implementing {
        reference: expected.clone(),
        snapshot,
    })
}

#[cfg(test)]
mod tests;
