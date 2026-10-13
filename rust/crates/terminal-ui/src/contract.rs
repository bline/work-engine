//! Read-only projection of the published S3 contract and passive client results.
//! No transport, command grant, or domain state machine lives here.

use crate::presentation::{Conversation, Entry, EntryKind, SourceId};
use lifecycle_client::ClientError;
use lifecycle_wire::{
    AdmissionV1, EffectSettlementV1, ExecutionOutcomeV1, FieldV1, LifecycleSnapshotV1,
    RuntimeAvailabilityV1, WaitResultV1, WireError, WireErrorCode,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Case {
    FailedUnsettled,
    HistoricalBlocked,
    Unavailable,
    Unsupported,
}

impl Case {
    pub const ALL: [Self; 4] = [
        Self::FailedUnsettled,
        Self::HistoricalBlocked,
        Self::Unavailable,
        Self::Unsupported,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::FailedUnsettled => "failed unsettled",
            Self::HistoricalBlocked => "historical complete current blocked",
            Self::Unavailable => "observation unavailable",
            Self::Unsupported => "unsupported variant",
        }
    }
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::FailedUnsettled => {
                include_bytes!("../../lifecycle-wire/tests/fixtures/failed-unsettled-v1.json")
            }
            Self::HistoricalBlocked => include_bytes!(
                "../../lifecycle-wire/tests/fixtures/historical-complete-current-blocked-v1.json"
            ),
            Self::Unavailable => include_bytes!(
                "../../lifecycle-wire/tests/fixtures/observation-unavailable-v1.json"
            ),
            Self::Unsupported => include_bytes!(
                "../../lifecycle-wire/tests/fixtures/unsupported-observation-v2.json"
            ),
        }
    }
}

fn row(text: impl Into<String>, details: impl Into<String>) -> Entry {
    Entry {
        kind: EntryKind::Recorded {
            details: details.into(),
        },
        recipient: None,
        subject: "lifecycle:observation",
        speaker: "Lifecycle observation",
        text: text.into(),
        detail: 0,
    }
}

fn field<T>(field: &FieldV1<T>, known: impl FnOnce(&T) -> String) -> String {
    match field {
        FieldV1::Known(value) => known(value),
        FieldV1::Unknown => "unknown".into(),
        FieldV1::NotApplicable => "not applicable".into(),
        FieldV1::Unavailable => "unavailable".into(),
    }
}

fn admission(value: &AdmissionV1) -> String {
    match value {
        AdmissionV1::Domain { context_generation } => {
            format!("domain / context {context_generation}")
        }
        AdmissionV1::Transition { transition_id } => {
            format!("blocked by current transition {transition_id}")
        }
        AdmissionV1::Fenced { reason } => format!("fenced / {reason}"),
        AdmissionV1::Unknown => "unknown".into(),
    }
}

fn outcome(value: &ExecutionOutcomeV1) -> &'static str {
    match value {
        ExecutionOutcomeV1::Pending => "pending",
        ExecutionOutcomeV1::Completed => "completed",
        ExecutionOutcomeV1::Failed => "failed",
        ExecutionOutcomeV1::Interrupted => "interrupted",
        ExecutionOutcomeV1::Unknown => "unknown",
    }
}

fn settlement(value: &EffectSettlementV1) -> String {
    match value {
        EffectSettlementV1::Unresolved => "unresolved".into(),
        EffectSettlementV1::Established { evidence_ref } => format!("established / {evidence_ref}"),
        EffectSettlementV1::Conflict { evidence_ref } => format!("conflict / {evidence_ref}"),
    }
}

fn provenance(value: &FieldV1<lifecycle_wire::ObservationProvenanceV1>) -> String {
    field(value, |v| {
        format!(
            "source {} / observed Unix ms {}",
            v.source_ref, v.observed_wall_ms
        )
    })
}

fn code_label(code: WireErrorCode) -> &'static str {
    match code {
        WireErrorCode::MalformedJson => "malformed_json",
        WireErrorCode::UnsupportedVersion => "unsupported_version",
        WireErrorCode::SemanticInvalidity => "semantic_invalidity",
        WireErrorCode::StaleRevision => "stale_revision",
        WireErrorCode::Conflict => "conflict",
        WireErrorCode::ObservationUnavailable => "observation_unavailable",
        WireErrorCode::CursorGap => "cursor_gap",
        WireErrorCode::UnsupportedCapability => "unsupported_capability",
        WireErrorCode::UnsupportedObservationVariant => "unsupported_observation_variant",
    }
}

pub fn snapshot_conversation(
    snapshot: &LifecycleSnapshotV1,
    source: SourceId,
    health: &str,
) -> Conversation {
    let identity = format!(
        "subject {} / context {} / store {} / stream {} / commit {}",
        snapshot.subject_id,
        snapshot.context_generation,
        snapshot.cursor.store_id,
        snapshot.cursor.stream_id,
        snapshot.cursor.commit_sequence
    );
    let delivery = field(&snapshot.delivery, |v| {
        format!(
            "{} / outcome {} / settlement {}",
            v.delivery_id,
            outcome(&v.outcome),
            settlement(&v.settlement)
        )
    });
    let transition = field(&snapshot.transition, |v| {
        format!(
            "{} / stage {:?} / completed {}",
            v.transition_id, v.stage, v.completed
        )
    });
    let admission = admission(&snapshot.current_admission);
    let runtime = match snapshot.runtime_availability {
        RuntimeAvailabilityV1::Available => "available",
        RuntimeAvailabilityV1::Unavailable => "unavailable",
        RuntimeAvailabilityV1::Unknown => "unknown",
    };
    let full = format!(
        "Protocol v{}\nScope: current snapshot, not event history; native operations and transcript unavailable\nIdentity: {identity}\nObservation: {health}\nCurrent admission: {admission}\nBlocking reason: {}\nHistorical/target transition: {transition}\nTarget delivery: {delivery}\nRuntime availability: {runtime}\nRuntime provenance: {}\nMeasurement (raw, unqualified; no percentage): {}\nAuthority references: {}\nEvidence references: {}",
        snapshot.protocol_version,
        field(&snapshot.blocking_reason, Clone::clone),
        provenance(&snapshot.runtime_provenance),
        field(&snapshot.measurement, Clone::clone),
        if snapshot.authority_refs.is_empty() {
            "none supplied".into()
        } else {
            snapshot.authority_refs.join(", ")
        },
        if snapshot.evidence_refs.is_empty() {
            "none supplied".into()
        } else {
            snapshot.evidence_refs.join(", ")
        },
    );
    let delivery_detail = field(&snapshot.delivery, |v| {
        format!(
            "Delivery {}\nEffect {}\nAttempt {}\nOutcome {}\nSettlement {}\nDelivery provenance {}",
            v.delivery_id,
            v.effect_id,
            v.attempt_id,
            outcome(&v.outcome),
            settlement(&v.settlement),
            provenance(&v.provenance)
        )
    });
    Conversation {
        source,
        entries: vec![
            row(format!("Observation {health} · {identity}"), full.clone()),
            row(
                format!("Current admission: {admission}"),
                format!(
                    "{full}\nCurrent admission describes the current read, independently of targeted historical facts."
                ),
            ),
            row(
                format!("Target delivery: {delivery}"),
                format!("{delivery_detail}\n{full}"),
            ),
            row(
                format!("Historical/target transition: {transition}"),
                format!(
                    "{full}\nA completed historical transition does not grant current admission."
                ),
            ),
        ],
        usage: format!(
            "Context {} · usage unavailable",
            snapshot.context_generation
        ),
        banner: format!("{health} · current snapshot · read-only"),
        simulated_activity: false,
        context: Some(("Lifecycle observation / exact scope", full)),
    }
}

fn error_conversation(source: SourceId, label: &str, reason: &str) -> Conversation {
    let details = format!(
        "{label}: {reason}\nNo coherent snapshot is displayed for this result. This is observation health, not lifecycle success or failure. No command authority is supplied."
    );
    Conversation {
        source,
        entries: vec![row(format!("Observation error: {reason}"), details.clone())],
        usage: "Context unknown · usage unavailable".into(),
        banner: format!("{label} · {reason} · read-only"),
        simulated_activity: false,
        context: Some(("Observation error", details)),
    }
}

pub fn client_pending(source: SourceId) -> Conversation {
    error_conversation(
        source,
        "PASSIVE CLIENT",
        "connection read pending; no observation yet",
    )
}

pub fn client_status(
    last_known: Option<&LifecycleSnapshotV1>,
    source: SourceId,
    status: &str,
) -> Conversation {
    match last_known {
        Some(snapshot) => snapshot_conversation(snapshot, source, status),
        None => error_conversation(source, "PASSIVE CLIENT", status),
    }
}

pub fn example(case: Case) -> Conversation {
    let source = SourceId::Contract(case);
    let label = "S3 CONTRACT EXAMPLE · NOT LIVE OR ARCHIVE EVIDENCE";
    match case {
        Case::FailedUnsettled | Case::HistoricalBlocked => {
            match lifecycle_wire::parse_snapshot(case.bytes(), "subject-1") {
                Ok(snapshot) => snapshot_conversation(&snapshot, source, label),
                Err(error) => error_conversation(
                    source,
                    label,
                    &format!("invalid published fixture: {error}"),
                ),
            }
        }
        Case::Unavailable => {
            // Use the owner's strict JSON parser before decoding its published wait-result DTO.
            let result = lifecycle_wire::parse_strict_json(case.bytes()).and_then(|value| {
                serde_json::from_value::<WaitResultV1<LifecycleSnapshotV1>>(value)
                    .map_err(|_| WireError::SemanticInvalidity)
            });
            match result {
                Ok(WaitResultV1::ObservationUnavailable { code }) => error_conversation(
                    source,
                    label,
                    &format!("observation unavailable ({})", code_label(code)),
                ),
                Ok(_) => error_conversation(
                    source,
                    label,
                    "invalid published unavailable fixture: unexpected result",
                ),
                Err(error) => error_conversation(
                    source,
                    label,
                    &format!("invalid published fixture: {error}"),
                ),
            }
        }
        Case::Unsupported => match lifecycle_wire::parse_snapshot(case.bytes(), "subject-1") {
            Err(WireError::UnsupportedVersion) => {
                error_conversation(source, label, "unsupported protocol version")
            }
            Err(WireError::UnsupportedObservationVariant) => {
                error_conversation(source, label, "unsupported observation variant")
            }
            Err(error) => error_conversation(
                source,
                label,
                &format!("invalid published fixture: {error}"),
            ),
            Ok(_) => error_conversation(
                source,
                label,
                "invalid published fixture: unsupported case accepted",
            ),
        },
    }
}

#[allow(dead_code)]
pub fn client_snapshot(
    result: Result<LifecycleSnapshotV1, ClientError>,
    last_known: Option<&LifecycleSnapshotV1>,
    source: SourceId,
    action: &str,
) -> Conversation {
    match result {
        Ok(snapshot) => {
            snapshot_conversation(&snapshot, source, &format!("{action} · coherent read"))
        }
        Err(error @ (ClientError::ObservationUnavailable | ClientError::DeadlineExpired))
            if last_known.is_some() =>
        {
            snapshot_conversation(
                last_known.unwrap(),
                source,
                &format!("{action}: {error} · last known only; connection state unknown"),
            )
        }
        Err(error) => error_conversation(source, "PASSIVE CLIENT", &format!("{action}: {error}")),
    }
}

#[allow(dead_code)]
pub fn client_wait(
    result: WaitResultV1<LifecycleSnapshotV1>,
    last_known: Option<&LifecycleSnapshotV1>,
    source: SourceId,
) -> Conversation {
    match result {
        WaitResultV1::Observed(snapshot) => {
            snapshot_conversation(&snapshot, source, "wait target observed")
        }
        WaitResultV1::DeadlineExpired {
            last_seen: Some(snapshot),
        } => snapshot_conversation(
            &snapshot,
            source,
            "deadline expired · last known snapshot; target unobserved",
        ),
        WaitResultV1::DeadlineExpired { last_seen: None } => match last_known {
            Some(snapshot) => snapshot_conversation(
                snapshot,
                source,
                "deadline expired · prior last known snapshot; target unobserved",
            ),
            None => error_conversation(
                source,
                "PASSIVE CLIENT",
                "deadline expired · no snapshot seen",
            ),
        },
        WaitResultV1::ObservationUnavailable { code } => {
            let reason = match code {
                WireErrorCode::UnsupportedVersion => "unsupported protocol version".to_string(),
                WireErrorCode::UnsupportedObservationVariant => {
                    "unsupported observation variant".to_string()
                }
                _ => format!("observation unavailable ({})", code_label(code)),
            };
            if code == WireErrorCode::ObservationUnavailable
                && let Some(snapshot) = last_known
            {
                return snapshot_conversation(
                    snapshot,
                    source,
                    "observation unavailable · prior last known snapshot only; cause not supplied",
                );
            }
            error_conversation(source, "PASSIVE CLIENT", &reason)
        }
    }
}
