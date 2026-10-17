use crate::{Result, Workflow, refuse};
use review_execution_evidence::claude::{QualifiedReader, QualifiedWriter};
use review_execution_evidence::{EvidenceReader, EvidenceWriter, ExecutionPublication};
use serde_json::{Value, json};
use slice_campaign::{
    AdmitRequest, Effect, NativeReviewRequestRef, OperationReceipt, RecoveryLocator,
};

impl Workflow {
    pub(crate) fn dispatch_receipt(
        &mut self,
        request: &NativeReviewRequestRef,
    ) -> Result<OperationReceipt> {
        // The selected campaign must already exist. Replaying its exact original
        // admission returns the owner's config digest and historical receipt;
        // this does not reconstruct a live review/dispatch capability.
        self.snapshot()?;
        let d = &self.startup.document;
        let original = self.campaign.admit(
            &self.startup.id("admit"),
            AdmitRequest {
                identity: d.identity.clone(),
                repository: d.repository.clone(),
                workspace: d.workspace.clone(),
                accepted_boundary: d.accepted_boundary.clone(),
                baseline: d.baseline.clone(),
                expected_impact: None,
            },
        )?;
        let profile_digest = match original {
            Effect::Replayed { receipt, .. } => receipt.profile_digest,
            _ => {
                return Err(refuse(
                    "unresolved",
                    "original campaign admission was not an exact replay",
                ));
            }
        };
        let command = self.command();
        let operation = self.startup.id("dispatch");
        let digest = slice_campaign::codec::campaign_digest(
            &json!({"identity":d.identity,"obligationId":d.obligation_id,"preparationOperationId":request.operation_id,"requestDigest":request.request_digest,"expectedRevision":request.prepared_revision,"operationId":operation,"command":command}),
        )?;
        let locator = RecoveryLocator {
            root_id: self.campaign.root_id().into(),
            anchored_root: self.campaign.anchored_root().into(),
            identity: d.identity.clone(),
            operation_id: operation,
            kind: "authorize_dispatch".into(),
            request_digest: digest,
            expected_revision: Some(request.prepared_revision.clone()),
            profile_digest,
        };
        let (record, receipt) = self.campaign.replay_dispatch(&locator)?;
        if record.command != command || record.request_digest != request.request_digest {
            return Err(refuse("conflict", "original dispatch differs from startup"));
        }
        Ok(receipt)
    }
    pub(crate) fn recover_initial(&mut self) -> Result<Value> {
        let state = self.snapshot()?;
        let d = self.startup.document.clone();
        let Some(request) = state
            .obligations
            .iter()
            .find(|o| o.obligation_id == d.obligation_id)
            .and_then(|o| o.request.as_ref())
        else {
            return Ok(
                json!({"status":"not_dispatched","reason":"no original preparation","launchAuthorized":false}),
            );
        };
        let dispatch = state
            .progress
            .attempts
            .iter()
            .find(|a| a.attempt_id == d.identity.attempt_id)
            .and_then(|a| a.dispatch.as_ref());
        if dispatch.is_none() {
            return Ok(
                json!({"status":"unresolved","reason":"prepared without a retained live dispatch permit","launchAuthorized":false}),
            );
        }
        let receipt = self.dispatch_receipt(request)?;
        if !d.roots.evidence.exists() {
            return Ok(
                json!({"status":"unresolved","reason":"dispatch committed but execution root absent; entry not disproven","dispatch":receipt,"launchAuthorized":false}),
            );
        }
        if self.startup.qualified.is_some() {
            let scope = self.qualified_scope(request, receipt.result_revision.as_str())?;
            let mut writer = QualifiedWriter::resume(&d.roots.evidence, scope.clone())?;
            let publication = writer.reconcile()?;
            drop(writer);
            match publication {
                ExecutionPublication::Committed(reference)
                | ExecutionPublication::Replayed(reference) => self.join(reference),
                other => {
                    let reader =
                        QualifiedReader::open(&d.roots.evidence, scope.reader_selection()?)?;
                    let failure = match reader.read_local_failure(&d.identity.attempt_id) {
                        Ok(value) => json!({"status":"observed","failure":value}),
                        Err(error) => json!({"status":"unavailable","reason":error.to_string()}),
                    };
                    Ok(
                        json!({"status":"unresolved","reason":format!("qualified custody {other:?}"),
                        "failure":failure,"dispatch":receipt,"launchAuthorized":false}),
                    )
                }
            }
        } else {
            let scope = self.scope(request, receipt.result_revision.as_str())?;
            let writer = EvidenceWriter::resume(&d.roots.evidence, scope.clone())?;
            let publication = writer.reconcile()?;
            drop(writer);
            match publication {
                ExecutionPublication::Committed(reference)
                | ExecutionPublication::Replayed(reference) => self.join(reference),
                other => {
                    let reader = EvidenceReader::open(&d.roots.evidence, scope.reader_pin()?)?;
                    let failure = match reader.read_local_failure(&d.identity.attempt_id) {
                        Ok(value) => json!({"status":"observed","failure":value}),
                        Err(error) => json!({"status":"unavailable","reason":error.to_string()}),
                    };
                    Ok(
                        json!({"status":"unresolved","reason":format!("execution custody {other:?}"),
                        "failure":failure,"dispatch":receipt,"launchAuthorized":false}),
                    )
                }
            }
        }
    }
}
