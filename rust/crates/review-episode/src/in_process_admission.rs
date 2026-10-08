//! Exact, in-process read admission for a selected native episode root.
//! The caller that constructs the scope owns the trusted startup provenance.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use review_episode_core::codec::{JsString, JsValue, canonical_json, digest, field};
use review_episode_core::identity::{Authority, Identity, Revision};
use review_episode_core::state::ReviewEpisodeState;
use review_episode_store::{EpisodeStore, NativeRootSelection, StoreOptions};

use crate::admission::{AdmittedResult, HostAdmissionPort};
use crate::protocol::{DEFAULT_REQUEST_LIMIT, Request};
use crate::{AppError, AppResult, Application};

const MAX_TARGETS: usize = 1024;
const READER: &str = "native-review-host";

/// Only an exact revision or first introduction of an exact transition is in scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EpisodeReadTarget {
    Revision(Revision),
    Transition {
        transition_id: JsString,
        content_digest: Revision,
    },
}

impl EpisodeReadTarget {
    fn value(&self) -> JsValue {
        match self {
            Self::Revision(revision) => JsValue::object([
                ("kind", JsValue::text("revision")),
                ("revision", revision.value()),
            ]),
            Self::Transition {
                transition_id,
                content_digest,
            } => JsValue::object([
                ("kind", JsValue::text("transition")),
                ("transitionId", JsValue::String(transition_id.clone())),
                ("contentDigest", content_digest.value()),
            ]),
        }
    }

    fn validate(&self) -> AppResult<()> {
        match self {
            Self::Revision(revision) => {
                Revision::parse(&revision.value(), "read revision")?;
            }
            Self::Transition {
                transition_id,
                content_digest,
            } => {
                JsValue::String(transition_id.clone()).nonempty_text("transitionId")?;
                Revision::parse(&content_digest.value(), "contentDigest")?;
            }
        }
        if canonical_json(&self.value()).len() > DEFAULT_REQUEST_LIMIT {
            return Err(AppError::Admission(
                "episode read target exceeds limit".into(),
            ));
        }
        Ok(())
    }
}

/// Trusted startup data. Parsing an authority document alone does not establish
/// that this scope came from the selected host's authority owner.
pub struct TrustedEpisodeReadScope {
    authority: Authority,
    targets: Vec<EpisodeReadTarget>,
    authority_digest: Revision,
}

impl TrustedEpisodeReadScope {
    pub fn new(authority: Authority, permitted: Vec<EpisodeReadTarget>) -> AppResult<Self> {
        let authority = Authority::parse(authority.value().clone())?;
        if authority.writer().generation != 1 || authority.predecessor_revision().is_some() {
            return Err(AppError::Admission(
                "read scope requires initial generation".into(),
            ));
        }
        let readers = field(authority.value(), "readers")?.as_array()?;
        if !readers.contains(&JsValue::text(READER)) {
            return Err(AppError::Admission(
                "native read principal is absent".into(),
            ));
        }
        if permitted.is_empty() || permitted.len() > MAX_TARGETS {
            return Err(AppError::Admission(
                "read scope target count is invalid".into(),
            ));
        }
        let mut seen = BTreeSet::new();
        let mut encoded = 2usize;
        for target in &permitted {
            target.validate()?;
            let key = canonical_json(&target.value());
            encoded = encoded.saturating_add(key.len() + 1);
            if encoded > DEFAULT_REQUEST_LIMIT || !seen.insert(key) {
                return Err(AppError::Admission(
                    "read scope is duplicate or oversized".into(),
                ));
            }
        }
        if canonical_json(authority.value())
            .len()
            .saturating_add(encoded)
            > DEFAULT_REQUEST_LIMIT
        {
            return Err(AppError::Admission("read scope exceeds limit".into()));
        }
        let authority_digest = Revision(digest(authority.value()));
        Ok(Self {
            authority,
            targets: permitted,
            authority_digest,
        })
    }

    fn admits(&self, identity: &Identity, target: &EpisodeReadTarget) -> AppResult<()> {
        Identity::parse(&identity.value())?;
        target.validate()?;
        if identity != self.authority.identity() || !self.targets.contains(target) {
            return Err(AppError::Admission(
                "identity or exact target is outside read scope".into(),
            ));
        }
        let request_bytes = canonical_json(&JsValue::object([
            ("identity", identity.value()),
            ("target", target.value()),
        ]))
        .len();
        if request_bytes > DEFAULT_REQUEST_LIMIT {
            return Err(AppError::Admission(
                "episode read request exceeds limit".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum EpisodeReadRequest {
    Revision {
        identity: Identity,
        revision: Revision,
    },
    Transition {
        identity: Identity,
        transition_id: JsString,
        content_digest: Revision,
    },
}

impl EpisodeReadRequest {
    fn identity(&self) -> &Identity {
        match self {
            Self::Revision { identity, .. } | Self::Transition { identity, .. } => identity,
        }
    }
    fn target(&self) -> EpisodeReadTarget {
        match self {
            Self::Revision { revision, .. } => EpisodeReadTarget::Revision(revision.clone()),
            Self::Transition {
                transition_id,
                content_digest,
                ..
            } => EpisodeReadTarget::Transition {
                transition_id: transition_id.clone(),
                content_digest: content_digest.clone(),
            },
        }
    }
}

/// Private port construction ensures that `Application::new` cannot mint this
/// admission merely from a public request or result value.
///
/// ```compile_fail
/// use review_episode::in_process_admission::InProcessReadAdmission;
/// let _forged = InProcessReadAdmission {};
/// ```
pub struct InProcessReadAdmission {
    scope: TrustedEpisodeReadScope,
    selection: NativeRootSelection,
    root: PathBuf,
}

impl HostAdmissionPort for InProcessReadAdmission {
    fn request_version(&self) -> u8 {
        2
    }

    fn precheck(&self, _request: &Request) -> AppResult<()> {
        Err(AppError::Admission(
            "use checked in-process episode readback".into(),
        ))
    }

    fn admit(
        &self,
        _request: &Request,
        _observed_revision: Option<&str>,
    ) -> AppResult<AdmittedResult> {
        Err(AppError::Admission(
            "use checked in-process episode readback".into(),
        ))
    }
}

/// Checked evidence from a single validated snapshot; no continuing permit.
///
/// ```compile_fail
/// use review_episode::in_process_admission::CheckedEpisodeReadback;
/// let _forged = CheckedEpisodeReadback {};
/// ```
///
/// ```compile_fail
/// use review_episode::in_process_admission::CheckedEpisodeReadback;
/// fn requires_deserializer<T: serde::de::DeserializeOwned>() {}
/// requires_deserializer::<CheckedEpisodeReadback>();
/// ```
pub struct CheckedEpisodeReadback {
    selection: NativeRootSelection,
    root: PathBuf,
    identity: Identity,
    authority_digest: Revision,
    target: EpisodeReadTarget,
    observed_revision: Option<Revision>,
    resolved_revision: Option<Revision>,
    state: Option<ReviewEpisodeState>,
    result_digest: Option<Revision>,
}

impl CheckedEpisodeReadback {
    pub fn selection(&self) -> &NativeRootSelection {
        &self.selection
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn authority_digest(&self) -> &Revision {
        &self.authority_digest
    }
    pub fn target(&self) -> &EpisodeReadTarget {
        &self.target
    }
    pub fn observed_revision(&self) -> Option<&Revision> {
        self.observed_revision.as_ref()
    }
    pub fn resolved_revision(&self) -> Option<&Revision> {
        self.resolved_revision.as_ref()
    }
    pub fn state(&self) -> Option<&ReviewEpisodeState> {
        self.state.as_ref()
    }
    pub fn result_digest(&self) -> Option<&Revision> {
        self.result_digest.as_ref()
    }
}

impl Application<InProcessReadAdmission> {
    pub fn open_read_only(
        root: &Path,
        expected: NativeRootSelection,
        scope: TrustedEpisodeReadScope,
        options: StoreOptions,
        response_limit: usize,
    ) -> AppResult<Self> {
        if !root.is_absolute()
            || response_limit == 0
            || response_limit > crate::protocol::DEFAULT_RESPONSE_LIMIT
        {
            return Err(AppError::Admission(
                "read root or response limit is invalid".into(),
            ));
        }
        let (store, actual) = EpisodeStore::open_native_read_only(root, options)?;
        if actual != expected || actual.root != store.root().to_string_lossy() {
            return Err(AppError::Admission("native root selection differs".into()));
        }
        let root = store.root().to_path_buf();
        Ok(Self::new(
            store,
            InProcessReadAdmission {
                scope,
                selection: actual,
                root,
            },
            response_limit,
        ))
    }

    pub fn read_checked(
        &mut self,
        request: &EpisodeReadRequest,
    ) -> AppResult<CheckedEpisodeReadback> {
        let identity = request.identity();
        let target = request.target();
        self.port.scope.admits(identity, &target)?;
        let actual = review_episode_store::read_native_selection(self.store.root())?;
        if actual != self.port.selection || self.store.root() != self.port.root {
            return Err(AppError::Admission("native read selection changed".into()));
        }
        let snapshot = self.store.read(&identity.key().0)?;
        let expected_binding = self.port.scope.authority.binding();
        let expected_writer = self.port.scope.authority.writer_value();
        let expected_subject = self.port.scope.authority.initial_subject();
        for entry in &snapshot.history {
            let state = &entry.state;
            if state.identity() != identity
                || state.get("authority") != &expected_binding
                || state.get("writer") != &expected_writer
                || state.get("subject") != expected_subject
            {
                return Err(AppError::Admission(
                    "episode history differs from read authority".into(),
                ));
            }
        }
        let observed_revision = snapshot
            .current
            .as_ref()
            .map(|state| state.revision().clone());
        let state = match &target {
            EpisodeReadTarget::Revision(revision) => snapshot
                .history
                .into_iter()
                .find(|entry| entry.revision == revision.0)
                .map(|entry| entry.state),
            EpisodeReadTarget::Transition {
                transition_id,
                content_digest,
            } => {
                let mut resolved = None;
                for entry in snapshot.history {
                    if let Some(actual_digest) = entry.state.handled(transition_id) {
                        if actual_digest != &content_digest.value() {
                            return Err(AppError::Admission(
                                "transition identity conflicts with durable content".into(),
                            ));
                        }
                        resolved = Some(entry.state);
                        break;
                    }
                }
                resolved
            }
        };
        let resolved_revision = state.as_ref().map(|state| state.revision().clone());
        let result_digest = state
            .as_ref()
            .map(|state| {
                state
                    .current_result()
                    .map(|result| result.map(|result| Revision(digest(&result.value))))
            })
            .transpose()?
            .flatten();
        let encoded = JsValue::object([
            (
                "selection",
                JsValue::object([
                    ("root", JsValue::text(&self.port.selection.root)),
                    (
                        "executableSha256",
                        JsValue::text(&self.port.selection.executable_sha256),
                    ),
                    (
                        "selectionDigest",
                        JsValue::text(&self.port.selection.selection_digest),
                    ),
                ]),
            ),
            ("identity", identity.value()),
            ("authorityDigest", self.port.scope.authority_digest.value()),
            ("target", target.value()),
            (
                "observedRevision",
                observed_revision
                    .as_ref()
                    .map_or(JsValue::Null, Revision::value),
            ),
            (
                "resolvedRevision",
                resolved_revision
                    .as_ref()
                    .map_or(JsValue::Null, Revision::value),
            ),
            (
                "state",
                state
                    .as_ref()
                    .map_or(JsValue::Null, |state| state.value().clone()),
            ),
            (
                "resultDigest",
                result_digest
                    .as_ref()
                    .map_or(JsValue::Null, Revision::value),
            ),
        ]);
        if canonical_json(&encoded).len() > self.response_limit {
            return Err(AppError::ResponseTooLarge);
        }
        Ok(CheckedEpisodeReadback {
            selection: self.port.selection.clone(),
            root: self.port.root.clone(),
            identity: identity.clone(),
            authority_digest: self.port.scope.authority_digest.clone(),
            target,
            observed_revision,
            resolved_revision,
            state,
            result_digest,
        })
    }
}
