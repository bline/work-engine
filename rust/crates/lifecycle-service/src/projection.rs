use lifecycle_store::SubjectProjection;
use lifecycle_wire::{
    AdmissionV1, CursorV1, DeliveryV1, EffectSettlementV1, ExecutionOutcomeV1, FieldV1,
    LifecycleSnapshotV1, ObservationProvenanceV1, RuntimeAvailabilityV1, TransitionStageV1,
    TransitionV1,
};

pub fn into_snapshot(value: SubjectProjection, targeted: bool) -> LifecycleSnapshotV1 {
    let admission = match value.owner_kind.as_str() {
        "domain" => AdmissionV1::Domain {
            context_generation: value.context_id.clone(),
        },
        "transition" => AdmissionV1::Transition {
            transition_id: value.owner_ref.clone().unwrap_or_default(),
        },
        "fenced" => AdmissionV1::Fenced {
            reason: "durable_recovery_required".into(),
        },
        _ => AdmissionV1::Unknown,
    };
    let transition = value
        .transition
        .as_ref()
        .and_then(|item| {
            stage(&item.stage).map(|stage| {
                FieldV1::Known(TransitionV1 {
                    transition_id: item.transition_id.clone(),
                    completed: matches!(
                        stage,
                        TransitionStageV1::Reconciled | TransitionStageV1::AbortedBeforeEntry
                    ),
                    stage,
                })
            })
        })
        .unwrap_or(if targeted {
            FieldV1::Unknown
        } else {
            FieldV1::NotApplicable
        });
    let delivery = value
        .delivery
        .as_ref()
        .and_then(|item| {
            Some(FieldV1::Known(DeliveryV1 {
                delivery_id: item.delivery_id.clone(),
                effect_id: item.effect_id.clone()?,
                attempt_id: item.attempt_id.clone()?,
                outcome: outcome(&item.outcome)?,
                settlement: settlement(&item.settlement_kind, item.settlement_evidence.as_deref())?,
                provenance: match (&item.source_ref, item.observed_wall_ms) {
                    (Some(source), Some(wall)) => FieldV1::Known(ObservationProvenanceV1 {
                        source_ref: source.clone(),
                        observed_wall_ms: wall.to_string(),
                    }),
                    _ => FieldV1::Unknown,
                },
            }))
        })
        .unwrap_or(if targeted {
            FieldV1::Unknown
        } else {
            FieldV1::NotApplicable
        });
    let evidence_refs = value
        .delivery
        .as_ref()
        .and_then(|item| item.source_ref.clone())
        .into_iter()
        .collect();
    LifecycleSnapshotV1 {
        protocol_version: 1,
        subject_id: value.subject_id.clone(),
        context_generation: value.context_id,
        cursor: CursorV1 {
            store_id: value.store_id,
            stream_id: format!("stream:{}", value.subject_id),
            commit_sequence: value.journal_cursor.to_string(),
        },
        transition,
        current_admission: admission,
        delivery,
        runtime_availability: if value.unresolved_attempts > 0 {
            RuntimeAvailabilityV1::Unknown
        } else {
            RuntimeAvailabilityV1::Available
        },
        runtime_provenance: FieldV1::Unknown,
        blocking_reason: if value.unresolved_attempts > 0 {
            FieldV1::Known("unresolved_entered_attempt".into())
        } else {
            FieldV1::NotApplicable
        },
        authority_refs: Vec::new(),
        evidence_refs,
        measurement: FieldV1::NotApplicable,
    }
}

fn outcome(value: &str) -> Option<ExecutionOutcomeV1> {
    match value {
        "pending" => Some(ExecutionOutcomeV1::Pending),
        "completed" => Some(ExecutionOutcomeV1::Completed),
        "failed" => Some(ExecutionOutcomeV1::Failed),
        "interrupted" => Some(ExecutionOutcomeV1::Interrupted),
        "unknown" => Some(ExecutionOutcomeV1::Unknown),
        _ => None,
    }
}

fn settlement(kind: &str, evidence: Option<&str>) -> Option<EffectSettlementV1> {
    match (kind, evidence) {
        ("unresolved", _) => Some(EffectSettlementV1::Unresolved),
        ("established", Some(id)) => Some(EffectSettlementV1::Established {
            evidence_ref: id.into(),
        }),
        ("conflict", Some(id)) => Some(EffectSettlementV1::Conflict {
            evidence_ref: id.into(),
        }),
        _ => None,
    }
}

fn stage(value: &str) -> Option<TransitionStageV1> {
    match value {
        "quiescing" => Some(TransitionStageV1::Quiescing),
        "capturing" => Some(TransitionStageV1::Capturing),
        "verifying" => Some(TransitionStageV1::Verifying),
        "checkpointed" => Some(TransitionStageV1::Checkpointed),
        "switching" => Some(TransitionStageV1::Switching),
        "rehydrating" => Some(TransitionStageV1::Rehydrating),
        "ready_to_commit" => Some(TransitionStageV1::ReadyToCommit),
        "reconciled" => Some(TransitionStageV1::Reconciled),
        "recovery_required" => Some(TransitionStageV1::RecoveryRequired),
        "aborted_before_entry" => Some(TransitionStageV1::AbortedBeforeEntry),
        _ => None,
    }
}
