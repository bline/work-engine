//! Private operator ingress for the existing Campaign qualification owner.
//! The protected decision is not a serialized binding or spend receipt. Only
//! the opening that registers it may deliver the fresh spend to the Host.

use super::{
    AttemptBinding, Campaign, EnteredQualification, QualificationPort, QualificationPurpose,
    SelectionPins, SpentQualification, refused,
};
use crate::{
    AcceptedBoundary, Baseline, CampaignError, CampaignIdentity, CampaignRevision, Result,
    TrustedConfig, codec::raw_sha256,
};
use rustix::{fs::OFlags, process::geteuid};
use serde::Deserialize;
use std::{
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

const ROOT: &str = "/var/lib/work-engine/contract-b-init-qualification-v4/natural-exit";
const MAX_APPROVAL: u64 = 16 * 1024;
const MAX_GATE: u64 = 1024 * 1024;
const MAX_HOST_INPUT: u64 = 32 * 1024;

struct Inputs {
    path: PathBuf,
    uid: u32,
    node: (u64, u64),
}

impl Inputs {
    fn new(path: &Path, uid: u32) -> Result<Self> {
        let m = private_dir(path, uid)?;
        Ok(Self {
            path: path.into(),
            uid,
            node: (m.dev(), m.ino()),
        })
    }

    fn check(&self) -> Result<()> {
        let m = private_dir(&self.path, self.uid)?;
        if (m.dev(), m.ino()) != self.node {
            return Err(refused("foreground input root replaced"));
        }
        Ok(())
    }

    fn read(&self, name: &'static str, limit: u64) -> Result<Vec<u8>> {
        self.check()?;
        let path = self.path.join(name);
        let file = OpenOptions::new()
            .read(true)
            .custom_flags((OFlags::NOFOLLOW | OFlags::NONBLOCK).bits() as i32)
            .open(&path)
            .map_err(io)?;
        let m = file.metadata().map_err(io)?;
        if !m.is_file()
            || m.uid() != self.uid
            || m.nlink() != 1
            || m.mode() & 0o7177 != 0
            || !(1..=limit).contains(&m.len())
        {
            return Err(refused(
                "foreground input is not a bounded owner-only regular file",
            ));
        }
        let same = |n: &fs::Metadata| {
            (
                m.dev(),
                m.ino(),
                m.uid(),
                m.nlink(),
                m.mode(),
                m.len(),
                m.ctime(),
                m.ctime_nsec(),
            ) == (
                n.dev(),
                n.ino(),
                n.uid(),
                n.nlink(),
                n.mode(),
                n.len(),
                n.ctime(),
                n.ctime_nsec(),
            )
        };
        let mut raw = Vec::new();
        (&file).take(limit + 1).read_to_end(&mut raw).map_err(io)?;
        if raw.len() as u64 != m.len()
            || !same(&file.metadata().map_err(io)?)
            || !same(&fs::symlink_metadata(&path).map_err(io)?)
        {
            return Err(refused("foreground input changed during the read"));
        }
        self.check()?;
        Ok(raw)
    }
}

fn io(error: std::io::Error) -> CampaignError {
    CampaignError::Io(error.to_string())
}

fn private_dir(path: &Path, uid: u32) -> Result<fs::Metadata> {
    let m = fs::symlink_metadata(path).map_err(io)?;
    if !m.is_dir() || m.uid() != uid || m.mode() & 0o7077 != 0 {
        return Err(refused("foreground directory is not owner-only"));
    }
    Ok(m)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Decision {
    action: String,
    root_id: String,
    identity: CampaignIdentity,
    head: CampaignRevision,
    grant_id: String,
    attempt_id: String,
    intent_sha256: String,
    recipe_sha256: String,
    serving_sha256: String,
    expires_at_unix_ms: i64,
    repository: PathBuf,
    workspace: PathBuf,
    accepted_boundary: AcceptedBoundary,
    baseline: Baseline,
    writer_identity: String,
}

/// Fixed installed input boundary. There is no path, input bytes, or grant
/// constructor on the public API. The administrator commissions these files
/// separately; this executable only reads and checks them.
pub struct ProtectedApproval {
    inputs: Inputs,
    raw: String,
    sha256: String,
    decision: Decision,
}

pub struct HostInputs {
    pub selector: Vec<u8>,
    pub recipe: Vec<u8>,
    pub manifest: Vec<u8>,
    pub image_config: Vec<u8>,
}

impl ProtectedApproval {
    pub fn load() -> Result<Self> {
        if !geteuid().is_root() {
            return Err(refused("foreground approval requires the administrator"));
        }
        let path = Path::new(ROOT);
        for ancestor in path.ancestors().skip(1) {
            let m = fs::symlink_metadata(ancestor).map_err(io)?;
            if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
                return Err(refused("foreground ancestry is not root-protected"));
            }
        }
        Self::read(Inputs::new(path, 0)?)
    }

    fn read(inputs: Inputs) -> Result<Self> {
        let bytes = inputs.read("approval.json", MAX_APPROVAL)?;
        let sha256 = raw_sha256(&bytes);
        let raw = String::from_utf8(bytes).map_err(|_| refused("approval must be UTF-8 JSON"))?;
        let decision: Decision =
            serde_json::from_str(&raw).map_err(|_| refused("invalid exact foreground approval"))?;
        if decision.action != "approve-one-b-init-natural-exit-v1" {
            return Err(refused("no natural-exit approval"));
        }
        Ok(Self {
            inputs,
            raw,
            sha256,
            decision,
        })
    }

    pub fn host_inputs(&self) -> Result<HostInputs> {
        let read = |name| self.inputs.read(name, MAX_HOST_INPUT);
        Ok(HostInputs {
            selector: read("host.json")?,
            recipe: read("recipe.json")?,
            manifest: read("image-manifest.json")?,
            image_config: read("image-config.json")?,
        })
    }

    /// Open the pre-provisioned owner. No initialization, attachment, or
    /// admission; an absent or different Campaign may never supply a binding.
    pub fn open(self, intent: &str, recipe: &str, serving: &str) -> Result<ForegroundSelection> {
        let d = &self.decision;
        let campaign_root = self.campaign_root()?;
        let read = |name| self.inputs.read(name, MAX_GATE);
        let config = TrustedConfig::controlled_sc0(
            campaign_root,
            d.repository.clone(),
            d.workspace.clone(),
            d.identity.clone(),
            d.accepted_boundary.clone(),
            d.baseline.clone(),
            d.writer_identity.clone(),
            read("gate-capture.json")?,
            read("gate-manifest.json")?,
            read("gate-receipt.json")?,
        )?;
        let campaign = Campaign::open(config)?;
        self.select(&campaign, intent, recipe, serving)
    }

    fn campaign_root(&self) -> Result<PathBuf> {
        self.inputs.check()?;
        let root = self.inputs.path.join("campaign");
        private_dir(&root, self.inputs.uid)?;
        crate::store::verify_foreground_files(&root, self.inputs.uid)?;
        Ok(root)
    }

    fn check_current(&self) -> Result<()> {
        self.campaign_root()?;
        // Recheck under the Campaign owner gate at entry and again immediately
        // before spend. An observed withdrawal is a refusal, not a way to
        // reset the one-shot. Durable revocation is checked in the Store.
        if self.inputs.read("approval.json", MAX_APPROVAL)? != self.raw.as_bytes() {
            return Err(refused("foreground approval changed or was withdrawn"));
        }
        Ok(())
    }

    fn select(
        self,
        campaign: &Campaign,
        intent: &str,
        recipe: &str,
        serving: &str,
    ) -> Result<ForegroundSelection> {
        let d = &self.decision;
        let binding = {
            let mut owner = campaign.owner()?;
            self.check_current()?;
            if campaign.root_id() != d.root_id
                || campaign.anchored_root() != self.inputs.path.join("campaign")
                || (intent, recipe, serving)
                    != (&d.intent_sha256, &d.recipe_sha256, &d.serving_sha256)
            {
                return Err(refused("foreground Campaign or Host pins differ"));
            }
            let binding = owner.bind_qualification(
                &d.identity,
                &d.head,
                SelectionPins {
                    grant_id: d.grant_id.clone(),
                    intent_sha256: d.intent_sha256.clone(),
                    recipe_sha256: d.recipe_sha256.clone(),
                    serving_sha256: d.serving_sha256.clone(),
                    expires_at_unix_ms: d.expires_at_unix_ms,
                },
                QualificationPurpose::NaturalExit,
                Some(&self.sha256),
            )?;
            if binding.attempt_id() != d.attempt_id {
                return Err(refused("foreground attempt differs from approval"));
            }
            owner.store.register_qualification(&binding)?;
            binding
        };
        Ok(ForegroundSelection {
            approval: self,
            binding,
            port: campaign.offline_qualification_port(),
        })
    }
}

/// Fresh authority bound to the original owner opening. No serialization or
/// constructor from a binding, archive, history key, or committed spend row.
pub struct ForegroundSelection {
    approval: ProtectedApproval,
    binding: AttemptBinding,
    port: QualificationPort,
}

impl ForegroundSelection {
    pub fn binding(&self) -> &AttemptBinding {
        &self.binding
    }

    pub fn port(&self) -> &QualificationPort {
        &self.port
    }

    pub fn approval_raw(&self) -> &str {
        &self.approval.raw
    }

    fn enter(&self) -> Result<EnteredQualification> {
        self.port
            .enter_checked(&self.binding, || self.approval.check_current())
    }

    /// The acceptor runs only after exact committed-spend readback. No SQLite
    /// transaction or owner mutex is retained; dropping the handoff or losing
    /// its result cannot refund, re-enter, or reconstruct the continuation.
    pub fn handoff<T>(&self, accept: impl FnOnce(SpentQualification) -> T) -> Result<T> {
        self.deliver(self.enter()?, true, accept)
    }

    fn deliver<T>(
        &self,
        entry: EnteredQualification,
        confirm: bool,
        accept: impl FnOnce(SpentQualification) -> T,
    ) -> Result<T> {
        let spent = entry.consume_checked(confirm, || self.approval.check_current())?;
        Ok(accept(spent))
    }
}

#[cfg(test)]
mod tests;
