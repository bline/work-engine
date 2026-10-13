//! Controlled SC0 initial-review composition. Domain stores and reducers remain
//! owned by campaign, CE3, episode and execution evidence.
//!
//! Launch capability is private and cannot be constructed or decoded by callers.
//! ```compile_fail
//! let _ = review_workflow::initial::InitialLaunch;
//! ```
//! ```compile_fail
//! let _: review_workflow::initial::InitialLaunch = serde_json::from_str("{}").unwrap();
//! ```
mod initial;
pub mod protocol;
mod recovery;
pub mod startup;

use serde::Serialize;
use serde_json::Value;
use slice_campaign::{Campaign, Effect, Snapshot};
use startup::TrustedStartup;

#[derive(Debug, thiserror::Error, Serialize)]
#[error("{kind}: {message}")]
pub struct Error {
    pub kind: &'static str,
    pub message: String,
    pub owner: Option<Value>,
}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn refuse(kind: &'static str, message: impl Into<String>) -> Error {
    Error {
        kind,
        message: message.into(),
        owner: None,
    }
}
macro_rules! owner_error {
    ($ty:ty, $kind:literal) => {
        impl From<$ty> for Error {
            fn from(value: $ty) -> Self {
                refuse($kind, value.to_string())
            }
        }
    };
}
owner_error!(std::io::Error, "io");
owner_error!(serde_json::Error, "contract");
owner_error!(slice_campaign::CampaignError, "campaign");
impl From<review_episode::AppError> for Error {
    fn from(value: review_episode::AppError) -> Self {
        use review_episode_core::codec::{JsValue, canonical_json};
        let owner = value.possible_commit().map(|context| serde_json::json!({
            "owner":"review-episode", "root":context.root, "operation":context.operation,
            "identityCanonicalJson":canonical_json(&context.identity.value()),
            "transitionIdCanonicalJson":canonical_json(&JsValue::String(context.transition_id.clone())),
            "contentDigest":context.content_digest.0,
        }));
        Self {
            kind: "episode",
            message: value.to_string(),
            owner,
        }
    }
}
owner_error!(review_episode_core::EpisodeError, "episode");
owner_error!(review_episode_store::StoreError, "episode_store");
owner_error!(
    review_execution_evidence::EvidenceReadError,
    "execution_evidence"
);
owner_error!(claim_evidence::ClaimError, "claims");

pub(crate) fn effect<T>(value: Effect<T>) -> Result<T> {
    match value {
        Effect::Applied { value, .. } | Effect::Replayed { value, .. } => Ok(value),
        Effect::NoEffect(reason) => Err(refuse("no_effect", reason)),
        Effect::OutcomeUnknown(locator) => Err(Error {
            kind: "outcome_unknown",
            message: "reconcile original owner operation".into(),
            owner: Some(serde_json::to_value(locator)?),
        }),
    }
}

pub struct Workflow {
    startup: TrustedStartup,
    campaign: Campaign,
}
impl Workflow {
    pub fn initialize(startup: TrustedStartup) -> Result<Snapshot> {
        initial::initialize(startup)
    }
    pub fn open(startup: TrustedStartup) -> Result<Self> {
        let campaign = Campaign::open(startup.campaign_config()?)?;
        review_episode_store::EpisodeStore::open_native(
            &startup.document.roots.episode,
            review_episode_store::StoreOptions::default(),
        )?;
        let value = Self { startup, campaign };
        value.snapshot()?;
        Ok(value)
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        self.campaign
            .read(&self.startup.document.identity)?
            .ok_or_else(|| refuse("unresolved", "campaign not initialized"))
    }
    pub async fn run(&mut self, command: &protocol::Command) -> Result<Value> {
        self.startup.revalidate()?;
        command.check_scope(&self.startup.document)?;
        review_episode_store::EpisodeStore::open_native(
            &self.startup.document.roots.episode,
            review_episode_store::StoreOptions::default(),
        )?;
        let state = self.snapshot()?;
        if command.operation == protocol::Operation::InspectInitial {
            return Ok(serde_json::json!({"status":"observed","campaign":state}));
        }
        if command.expected_revision.as_deref() != Some(state.revision.as_str()) {
            return Err(refuse("conflict", "expected campaign revision differs"));
        }
        match command.operation {
            protocol::Operation::ExecuteInitial => self.execute_initial(&state.revision).await,
            protocol::Operation::RecoverInitial => self.recover_initial(),
            _ => Err(refuse("unsupported", "operation belongs to host control")),
        }
    }
}

#[cfg(feature = "test-faults")]
pub(crate) fn fault_cut(name: &str) {
    if std::env::var("REVIEW_HOST_FAULT_CUT").ok().as_deref() != Some(name) {
        return;
    }
    if let Some(directory) = std::env::var_os("REVIEW_HOST_FAULT_DIR") {
        std::fs::write(
            std::path::Path::new(&directory).join(format!("{name}.ready")),
            b"ready\n",
        )
        .expect("fault signal");
        loop {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
#[cfg(not(feature = "test-faults"))]
pub(crate) fn fault_cut(_: &str) {}
