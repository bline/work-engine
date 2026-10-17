//! Qualified initial-only E1 adapter. The campaign entry capability is held
//! through checked episode begin and consumed exactly once by E1.
use crate::{Result, Workflow, refuse};
use claim_evidence::codec::JsValue;
use review_execution_evidence::claude::{
    QualifiedScope, QualifiedSelection, QualifiedWriter, compose_applied_initial_prompt,
};
use review_execution_evidence::{
    ExecutionBinding, ExecutionEvidenceRef, ExecutionProvenance, ExecutionPublication,
};
#[cfg(feature = "test-support")]
use slice_campaign::OfflineInitialEntry;
use slice_campaign::{FreshInitialEntry, NativeReviewRequestRef};
use std::time::Duration;

pub(crate) enum QualifiedInitialLaunch {
    Installed(FreshInitialEntry),
    #[cfg(feature = "test-support")]
    Offline(OfflineInitialEntry),
}

impl Workflow {
    pub(crate) fn qualified_scope(
        &self,
        request: &NativeReviewRequestRef,
        dispatch_revision: &str,
    ) -> Result<QualifiedScope> {
        let d = &self.startup.document;
        let runtime = self
            .startup
            .qualified
            .as_ref()
            .ok_or_else(|| refuse("unsupported", "qualified runtime absent"))?;
        let command = self.command();
        let prompt = compose_applied_initial_prompt(&runtime.prompt_template, request)?;
        let subject = JsValue::object([
            ("commit", JsValue::text(&request.candidate.commit)),
            ("tree", JsValue::text(&request.candidate.tree)),
            (
                "patchIdentity",
                JsValue::text(&request.candidate.patch_identity),
            ),
        ]);
        let selection = QualifiedSelection {
            root_id: d.peer.evidence_root_id.clone(),
            provenance: ExecutionProvenance {
                binding: ExecutionBinding {
                    campaign_root_id: self.campaign.root_id().into(),
                    obligation_id: d.obligation_id.clone(),
                    candidate_digest: request.candidate.receipt_sha256.clone(),
                    selection_digest: request.selection.campaign_digest.clone(),
                    profile_digest: request.profile_digest.clone(),
                    execution_profile_digest: Some(runtime.profile.profile_digest().into()),
                    prepared_request_sha256: request.request_digest.clone(),
                    review_episode_id: command.episode_id,
                    attempt_id: d.identity.attempt_id.clone(),
                },
                dispatch_operation_id: self.startup.id("dispatch"),
                dispatch_revision: dispatch_revision.into(),
                execution_session_id: Some(d.peer.session.clone()),
            },
            selection_id: request.selection.selection_id.clone(),
            candidate_commit: request.candidate.commit.clone(),
            candidate_tree: request.candidate.tree.clone(),
            candidate_patch_identity: request.candidate.patch_identity.clone(),
            candidate_path: d.repository.clone(),
            candidate_tree_sha256: runtime.boundary.index().candidate_tree_sha256().into(),
            cbm_project: runtime.boundary.index().project().into(),
            cbm_store_sha256: runtime.boundary.index().store_sha256().into(),
            cbm_generation: runtime.boundary.index().indexed_at().into(),
            cbm_coverage_sha256: runtime.boundary.index().coverage_sha256().into(),
            prompt,
            subject,
            credential: Some(runtime.credential.clone()),
            #[cfg(feature = "test-support")]
            offline_witness: runtime.offline_witness.clone(),
            #[cfg(not(feature = "test-support"))]
            offline_witness: None,
            horizon: Duration::from_millis(runtime.horizon_millis),
        };
        if runtime.installed() {
            Ok(QualifiedScope::bind_installed(
                runtime.profile.clone(),
                selection,
                runtime.boundary.clone(),
            )?
            .with_applied_physical_profile(&request.profile_digest)?
            .with_applied_request(&runtime.prompt_template, request.clone())?)
        } else {
            #[cfg(feature = "test-support")]
            {
                Ok(QualifiedScope::bind_offline_guarded_peer(
                    runtime.profile.clone(),
                    selection,
                    runtime.boundary.clone(),
                )?
                .with_applied_physical_profile(&request.profile_digest)?
                .with_applied_request(&runtime.prompt_template, request.clone())?)
            }
            #[cfg(not(feature = "test-support"))]
            {
                Err(refuse("unsupported", "offline provider unavailable"))
            }
        }
    }

    pub(crate) async fn launch_qualified(
        &self,
        entry: QualifiedInitialLaunch,
        scope: QualifiedScope,
    ) -> Result<ExecutionEvidenceRef> {
        let root = &self.startup.document.roots.evidence;
        if root.exists() {
            return Err(refuse(
                "unresolved",
                "qualified evidence root exists; recover original",
            ));
        }
        let mut writer = QualifiedWriter::create(root, scope)?;
        let publication = match entry {
            QualifiedInitialLaunch::Installed(value) => {
                writer.execute_applied_initial(value).await?
            }
            #[cfg(feature = "test-support")]
            QualifiedInitialLaunch::Offline(value) => writer.execute_offline_applied(value).await?,
        };
        drop(writer);
        match publication {
            ExecutionPublication::Committed(reference) => Ok(reference),
            other => Err(refuse(
                "unresolved",
                format!("qualified publication {other:?}; recover original"),
            )),
        }
    }
}
