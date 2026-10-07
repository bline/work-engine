use std::time::Duration;

use lifecycle_client::{ClientError, ObservationTransport, ReadClient, SubjectRef, WaitHintV1};
use lifecycle_wire::{
    AdmissionV1, CursorV1, FieldV1, LifecycleSnapshotV1, RuntimeAvailabilityV1, TransitionStageV1,
    WaitResultV1, WaitTargetV1,
};

struct LocalReadTransport;

impl ObservationTransport for LocalReadTransport {
    fn snapshot(
        &self,
        subject: &SubjectRef,
        _: Option<&WaitTargetV1>,
        _: Duration,
    ) -> Result<LifecycleSnapshotV1, ClientError> {
        Ok(LifecycleSnapshotV1 {
            protocol_version: 1,
            subject_id: subject.as_str().to_owned(),
            context_generation: "context-1".into(),
            cursor: CursorV1 {
                store_id: "store-1".into(),
                stream_id: "subject-1".into(),
                commit_sequence: "7".into(),
            },
            transition: FieldV1::NotApplicable,
            current_admission: AdmissionV1::Domain {
                context_generation: "context-1".into(),
            },
            delivery: FieldV1::NotApplicable,
            runtime_availability: RuntimeAvailabilityV1::Unknown,
            runtime_provenance: FieldV1::Unknown,
            blocking_reason: FieldV1::NotApplicable,
            authority_refs: vec![],
            evidence_refs: vec![],
            measurement: FieldV1::Unavailable,
        })
    }

    fn wait_hint(
        &self,
        _: &SubjectRef,
        _: &WaitTargetV1,
        _: &CursorV1,
        _: Duration,
    ) -> Result<WaitHintV1, ClientError> {
        Ok(WaitHintV1::NoChange)
    }
}

#[test]
fn public_consumer_has_only_read_and_wait_operations() {
    let client = ReadClient::new(LocalReadTransport);
    let subject = SubjectRef::parse("subject-1").unwrap();
    let snapshot = client.snapshot(&subject, Duration::from_secs(1)).unwrap();
    assert_eq!(snapshot.cursor.commit_sequence, "7");
    let target = WaitTargetV1::TransitionStage {
        transition_id: "transition-1".into(),
        stage: TransitionStageV1::Reconciled,
    };
    assert!(matches!(
        client.wait(&subject, &target, Duration::from_secs(1)),
        WaitResultV1::DeadlineExpired { .. }
    ));
}
