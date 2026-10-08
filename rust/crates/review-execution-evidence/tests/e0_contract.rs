#![cfg(feature = "test-support")]

use claim_evidence::codec::{JsValue, digest, parse_json};
use claim_evidence::{ProductionPathAccess, ProductionPathAdmissionBinding, ProductionPathStage};
use review_execution_evidence::test_support::{ControlledFixtureOwner, FixtureRecord};
use review_execution_evidence::{
    CheckedObservation, EvidenceClass, EvidenceReadError, EvidenceReaderProfile, ExecutionBinding,
    ExecutionEvidenceOwner, ExecutionProvenance,
};

fn fixture(
    observed: bool,
) -> (
    FixtureRecord,
    ProductionPathAdmissionBinding,
    Option<JsValue>,
) {
    let raw = b"{\"ok\": 1 }".to_vec();
    let native = parse_json(std::str::from_utf8(&raw).unwrap()).unwrap();
    let result_digest = digest(&native).unwrap();
    let provenance = ExecutionProvenance {
        binding: ExecutionBinding {
            campaign_root_id: "campaign-1".into(),
            obligation_id: "obligation-1".into(),
            candidate_digest: "a".repeat(64),
            selection_digest: "b".repeat(64),
            profile_digest: "c".repeat(64),
            prepared_request_sha256: "d".repeat(64),
            review_episode_id: "episode-1".into(),
            attempt_id: "attempt-1".into(),
        },
        dispatch_operation_id: "dispatch-1".into(),
        dispatch_revision: "revision-1".into(),
        execution_session_id: Some("session-1".into()),
    };
    let observation = JsValue::object([
        (
            "execution",
            JsValue::object([
                ("attemptId", JsValue::text("attempt-1")),
                ("resultDigest", JsValue::text(&result_digest)),
            ]),
        ),
        ("obligationId", JsValue::text("obligation-1")),
        (
            "subject",
            JsValue::object([("reviewEpisodeId", JsValue::text("episode-1"))]),
        ),
        (
            "selection",
            JsValue::object([("revision", JsValue::text(&"b".repeat(64)))]),
        ),
        (
            "continuity",
            JsValue::object([("sessionId", JsValue::text("session-1"))]),
        ),
        ("artifacts", JsValue::Array(vec![])),
    ]);
    let record = FixtureRecord::controlled(
        "root-1",
        "record-1",
        "revision-1",
        raw,
        provenance,
        if observed {
            CheckedObservation::Present(observation.clone())
        } else {
            CheckedObservation::Absent {
                reason: "owned absence event".into(),
            }
        },
        vec![],
    )
    .unwrap();
    let binding = ProductionPathAdmissionBinding {
        campaign_root_id: "campaign-1".into(),
        campaign_revision: "later-campaign-revision".into(),
        campaign_operation_id: "later-completion-operation".into(),
        obligation_id: "obligation-1".into(),
        candidate_digest: "a".repeat(64),
        selection_digest: "b".repeat(64),
        profile_digest: "c".repeat(64),
        prepared_request_sha256: "d".repeat(64),
        child_request_sha256: "e".repeat(64),
        review_episode_id: "episode-1".into(),
        attempt_id: "attempt-1".into(),
        native_result_sha256: result_digest,
        episode_revision: None,
        session_id: observed.then(|| "session-1".into()),
        stage: ProductionPathStage::RecordObservation,
        access: ProductionPathAccess::Original,
    };
    (record, binding, observed.then_some(observation))
}

#[test]
fn fixture_reader_preserves_native_result_and_distinct_digests() {
    let (record, binding, observation) = fixture(true);
    let reference = record.reference.clone();
    let owner = ControlledFixtureOwner::new(vec![record]).unwrap();
    let checked = owner.read_result(&reference).unwrap();
    assert_eq!(checked.evidence_class(), EvidenceClass::ControlledFixture);
    assert_eq!(checked.value().get("ok"), Some(&JsValue::Number(1.0)));
    assert_ne!(checked.claim_sha256(), checked.raw_sha256());
    assert_eq!(checked.claim_canonical_bytes(), b"{\"ok\":1}\n");
    assert_eq!(checked.provenance().dispatch_operation_id, "dispatch-1");
    let custody = owner
        .read_custody(&reference, &binding, observation.as_ref())
        .unwrap();
    assert_eq!(custody.native_result_sha256, checked.claim_sha256());
    assert_eq!(
        custody.observation_sha256,
        Some(digest(observation.as_ref().unwrap()).unwrap())
    );
    assert_eq!(custody.session_id.as_deref(), Some("session-1"));
}

#[test]
fn checked_absence_is_explicit_even_when_execution_had_a_session() {
    let (record, binding, _) = fixture(false);
    let reference = record.reference.clone();
    let owner = ControlledFixtureOwner::new(vec![record]).unwrap();
    let checked = owner.read_result(&reference).unwrap();
    assert!(matches!(
        checked.observation(),
        CheckedObservation::Absent { .. }
    ));
    assert_eq!(
        checked.provenance().execution_session_id.as_deref(),
        Some("session-1")
    );
    let custody = owner.read_custody(&reference, &binding, None).unwrap();
    assert_eq!(custody.session_id, None);
    assert_eq!(custody.observation_sha256, None);
    assert!(custody.artifact_references.is_empty());
}

#[test]
fn missing_conflicting_and_mismatched_evidence_never_becomes_absence() {
    let (record, binding, observation) = fixture(true);
    let reference = record.reference.clone();
    let owner = ControlledFixtureOwner::new(vec![record]).unwrap();
    let mut missing = reference.clone();
    missing.record_id = "other-record".into();
    assert_eq!(
        owner.read_result(&missing).unwrap_err(),
        EvidenceReadError::Absent
    );
    let mut conflict = reference.clone();
    conflict.sha256 = "f".repeat(64);
    assert!(matches!(
        owner.read_result(&conflict),
        Err(EvidenceReadError::Conflicting(_))
    ));
    assert!(matches!(
        owner.read_custody(&reference, &binding, None),
        Err(EvidenceReadError::Conflicting(_))
    ));
    let mut wrong = binding.clone();
    wrong.prepared_request_sha256 = "f".repeat(64);
    assert!(matches!(
        owner.read_custody(&reference, &wrong, observation.as_ref()),
        Err(EvidenceReadError::Conflicting(_))
    ));
}

#[test]
fn fixture_cannot_select_an_actual_process_or_provider_profile() {
    let (mut record, _, _) = fixture(false);
    record.reference.profile = "controlled-process-v1".into();
    assert!(matches!(
        ControlledFixtureOwner::new(vec![record]),
        Err(EvidenceReadError::Unsupported(_))
    ));
    assert!(
        EvidenceReaderProfile::QualifiedProvider
            .require_class(EvidenceClass::ControlledFixture)
            .is_err()
    );
    assert!(
        EvidenceReaderProfile::QualifiedProvider
            .require_class(EvidenceClass::ControlledProcess)
            .is_err()
    );
    assert!(
        EvidenceReaderProfile::ControlledProcess
            .require_class(EvidenceClass::ControlledFixture)
            .is_err()
    );
}

#[test]
fn mutation_after_record_creation_is_refused() {
    let (mut record, _, _) = fixture(false);
    record.raw_result.push(b' ');
    assert!(matches!(
        ControlledFixtureOwner::new(vec![record]),
        Err(EvidenceReadError::Conflicting(_))
    ));
}
