use lifecycle_store::SubjectProjection;
use lifecycle_wire::{
    AdmissionV1, CursorV1, DeliveryV1, EffectSettlementV1, ExecutionOutcomeV1, FieldV1,
    LifecycleSnapshotV1, ObservationProvenanceV1, RuntimeAvailabilityV1, TransitionStageV1,
    TransitionV1,
};

pub fn into_snapshot(value: SubjectProjection, targeted: bool) -> LifecycleSnapshotV1 {
    let semantic_compile_refusal = value
        .transition
        .as_ref()
        .and_then(|item| item.semantic_compile_refusal.clone());
    let semantic_material_block = value.transition.as_ref().and_then(|item| {
        item.semantic_material_block.as_ref().map(|reason| {
            if item.semantic_basis_issues.is_empty() {
                reason.clone()
            } else {
                format!(
                    "{}:{}",
                    reason,
                    serde_json::to_string(&item.semantic_basis_issues).unwrap_or_default()
                )
            }
        })
    });
    let yielded_outcome_not_representable = value
        .delivery
        .as_ref()
        .is_some_and(|item| item.outcome == "yielded");
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
    let mut blocking_reasons = Vec::new();
    if value.unresolved_attempts > 0 {
        blocking_reasons.push("unresolved_entered_attempt".to_owned());
    }
    if yielded_outcome_not_representable {
        blocking_reasons.push("yielded_outcome_not_representable_v1".to_owned());
    }
    if let Some(reason) = &semantic_compile_refusal {
        blocking_reasons.push(reason.clone());
    }
    if let Some(reason) = &semantic_material_block {
        blocking_reasons.push(reason.clone());
    }
    if let Some(reason) = &value.task_effect_hold {
        blocking_reasons.push(reason.clone());
    }
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
        runtime_availability: if value.unresolved_attempts > 0
            || semantic_compile_refusal.is_some()
            || semantic_material_block.is_some()
            || value.task_effect_hold.is_some()
        {
            RuntimeAvailabilityV1::Unknown
        } else {
            RuntimeAvailabilityV1::Available
        },
        runtime_provenance: FieldV1::Unknown,
        blocking_reason: if blocking_reasons.is_empty() {
            FieldV1::NotApplicable
        } else {
            FieldV1::Known(blocking_reasons.join(";"))
        },
        authority_refs: Vec::new(),
        evidence_refs,
        measurement: FieldV1::NotApplicable,
    }
}

fn outcome(value: &str) -> Option<ExecutionOutcomeV1> {
    match value {
        "pending" => Some(ExecutionOutcomeV1::Pending),
        // The published v1 schema has no Yielded variant. The blocking
        // reason distinguishes this lossy projection from an unknown owner fact.
        "yielded" => Some(ExecutionOutcomeV1::Unknown),
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

#[cfg(test)]
mod tests {
    use lifecycle_store::DeliveryProjection;
    use lifecycle_wire::WaitTargetV1;

    use super::*;

    #[test]
    fn yielded_custody_survives_service_wire_projection() {
        let projection = SubjectProjection {
            store_id: "store-yielded".into(),
            subject_id: "subject-yielded".into(),
            context_id: "context-yielded".into(),
            revision: 2,
            semantic_revision: 0,
            owner_kind: "domain".into(),
            owner_ref: None,
            journal_cursor: 9,
            transition: None,
            delivery: Some(DeliveryProjection {
                delivery_id: "delivery-yielded".into(),
                effect_id: Some("effect-yielded".into()),
                attempt_id: Some("attempt-yielded".into()),
                outcome: "yielded".into(),
                settlement_kind: "established".into(),
                settlement_evidence: Some("settlement-yielded".into()),
                source_ref: Some("observation-yielded".into()),
                observed_wall_ms: Some(123),
            }),
            unresolved_attempts: 0,
            task_effect_hold: None,
        };
        let overlapping = into_snapshot(
            SubjectProjection {
                unresolved_attempts: 1,
                ..projection.clone()
            },
            true,
        );
        assert_eq!(
            overlapping.blocking_reason,
            FieldV1::Known(
                "unresolved_entered_attempt;yielded_outcome_not_representable_v1".into()
            )
        );
        let snapshot = into_snapshot(projection, true);
        let bytes = serde_json::to_vec(&snapshot).unwrap();
        let readback: LifecycleSnapshotV1 = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(readback, snapshot);
        assert!(
            !WaitTargetV1::DeliveryOutcome {
                delivery_id: "delivery-yielded".into(),
                outcome: ExecutionOutcomeV1::Completed,
            }
            .is_observed(&readback)
        );
        assert!(
            WaitTargetV1::DeliveryOutcome {
                delivery_id: "delivery-yielded".into(),
                outcome: ExecutionOutcomeV1::Unknown,
            }
            .is_observed(&readback)
        );
        assert!(matches!(
            &readback.delivery,
            FieldV1::Known(DeliveryV1 {
                outcome: ExecutionOutcomeV1::Unknown,
                settlement: EffectSettlementV1::Established { .. },
                ..
            })
        ));
        assert_eq!(
            readback.blocking_reason,
            FieldV1::Known("yielded_outcome_not_representable_v1".into())
        );
        assert_eq!(readback.evidence_refs, vec!["observation-yielded"]);
        let FieldV1::Known(delivery) = readback.delivery else {
            panic!("exact delivery identity was lost");
        };
        assert_eq!(delivery.delivery_id, "delivery-yielded");
        assert_eq!(delivery.effect_id, "effect-yielded");
        assert_eq!(delivery.attempt_id, "attempt-yielded");
        assert!(matches!(delivery.provenance, FieldV1::Known(_)));
    }
}
