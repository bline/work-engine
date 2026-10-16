#![cfg(feature = "test-support")]
mod claude_common;

use std::fs;
use std::time::Duration;

use claude_common::{counts, fixture};
use review_execution_evidence::claude::{InitialEntryPermit, QualifiedReader, QualifiedWriter};
use review_execution_evidence::{
    EvidenceClass, EvidenceReadError, EvidenceReader, ExecutionEvidenceOwner, ExecutionPublication,
};

#[tokio::test]
async fn offline_peer_capture_reopens_with_exact_result_and_session() {
    let temp = tempfile::tempdir().unwrap();
    let f = fixture(temp.path(), "success", Duration::from_secs(2));
    let selection = f.scope.reader_selection().unwrap();
    let permit = InitialEntryPermit::offline_peer(&f.scope);
    let mut writer = QualifiedWriter::create(&f.root, f.scope.clone()).unwrap();
    let reference = match writer.execute_initial(permit).await.unwrap() {
        ExecutionPublication::Committed(reference) => reference,
        other => panic!("unexpected publication: {other:?}"),
    };
    assert_eq!(counts(&f.witness), (1, 1));
    drop(writer);
    let reader = QualifiedReader::open(&f.root, selection.clone()).unwrap();
    let checked = reader.read_result(&reference).unwrap();
    assert_eq!(checked.evidence_class(), EvidenceClass::ControlledFixture);
    assert_eq!(checked.reference(), &reference);
    let retained = reader.retained_session(&reference).unwrap();
    assert_eq!(retained.locator().session_id, "session-1");
    retained.verify_current().unwrap();
    assert!(
        reader
            .read_custody(
                &reference,
                &claude_common_binding(checked.claim_sha256()),
                None
            )
            .is_err()
    );
    assert!(matches!(
        EvidenceReader::open(&f.root, selection.reader_pin().clone()),
        Err(EvidenceReadError::Unsupported(_))
    ));
    drop(reader);
    let mut reopened = QualifiedWriter::resume(&f.root, f.scope).unwrap();
    assert_eq!(
        reopened.reconcile().unwrap(),
        ExecutionPublication::Replayed(reference.clone())
    );
    assert_eq!(counts(&f.witness), (1, 1));
}

fn claude_common_binding(sha: &str) -> claim_evidence::ProductionPathAdmissionBinding {
    use claim_evidence::{ProductionPathAccess, ProductionPathStage};
    claim_evidence::ProductionPathAdmissionBinding {
        campaign_root_id: "campaign-1".into(),
        campaign_revision: "revision-1".into(),
        campaign_operation_id: "operation-1".into(),
        obligation_id: "obligation-1".into(),
        candidate_digest: "b".repeat(64),
        selection_digest: "c".repeat(64),
        profile_digest: "d".repeat(64),
        prepared_request_sha256: "e".repeat(64),
        child_request_sha256: "f".repeat(64),
        review_episode_id: "episode-1".into(),
        attempt_id: "attempt-1".into(),
        native_result_sha256: sha.into(),
        episode_revision: None,
        session_id: Some("session-1".into()),
        stage: ProductionPathStage::RecordObservation,
        access: ProductionPathAccess::Original,
    }
}

#[tokio::test]
async fn pre_entry_candidate_drift_releases_no_prompt() {
    let temp = tempfile::tempdir().unwrap();
    let f = fixture(temp.path(), "success", Duration::from_secs(2));
    let permit = InitialEntryPermit::offline_peer(&f.scope);
    let mut writer = QualifiedWriter::create(&f.root, f.scope).unwrap();
    fs::write(f.candidate.join("source.rs"), b"changed").unwrap();
    assert_eq!(
        writer.execute_initial(permit).await.unwrap(),
        ExecutionPublication::DefinitePreEntryFailure
    );
    assert_eq!(counts(&f.witness), (0, 0));
}

#[tokio::test]
async fn pre_entry_executable_drift_releases_no_prompt() {
    let temp = tempfile::tempdir().unwrap();
    let f = fixture(temp.path(), "success", Duration::from_secs(2));
    let permit = InitialEntryPermit::offline_peer(&f.scope);
    let mut writer = QualifiedWriter::create(&f.root, f.scope).unwrap();
    fs::write(temp.path().join("installed-offline-peer"), b"changed").unwrap();
    assert_eq!(
        writer.execute_initial(permit).await.unwrap(),
        ExecutionPublication::DefinitePreEntryFailure
    );
    assert_eq!(counts(&f.witness), (0, 0));
}

#[tokio::test]
async fn malformed_and_mismatched_envelopes_never_retry() {
    for mode in [
        "malformed",
        "duplicate",
        "wrong_model",
        "wrong_session",
        "mixed_model",
        "wrong_subject",
        "noncanonical_result",
        "error_route",
        "oversized",
        "exit",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let f = fixture(temp.path(), mode, Duration::from_secs(2));
        let permit = InitialEntryPermit::offline_peer(&f.scope);
        let mut writer = QualifiedWriter::create(&f.root, f.scope.clone()).unwrap();
        assert_eq!(
            writer.execute_initial(permit).await.unwrap(),
            ExecutionPublication::Unresolved,
            "{mode}"
        );
        assert_eq!(counts(&f.witness), (1, 1), "{mode}");
        drop(writer);
        let mut reopened = QualifiedWriter::resume(&f.root, f.scope).unwrap();
        assert_eq!(
            reopened.reconcile().unwrap(),
            ExecutionPublication::Unresolved,
            "{mode}"
        );
        assert_eq!(counts(&f.witness), (1, 1), "{mode}");
    }
}

#[tokio::test]
async fn held_stdout_pipe_is_bounded_and_original_attempt_remains_unresolved() {
    let temp = tempfile::tempdir().unwrap();
    let f = fixture(temp.path(), "held_pipe", Duration::from_millis(100));
    let permit = InitialEntryPermit::offline_peer(&f.scope);
    let mut writer = QualifiedWriter::create(&f.root, f.scope.clone()).unwrap();
    assert_eq!(
        writer.execute_initial(permit).await.unwrap(),
        ExecutionPublication::Unresolved
    );
    drop(writer);
    let mut reopened = QualifiedWriter::resume(&f.root, f.scope).unwrap();
    assert_eq!(
        reopened.reconcile().unwrap(),
        ExecutionPublication::Unresolved
    );
    assert_eq!(counts(&f.witness), (1, 1));
}

#[tokio::test]
async fn timeout_reaps_locally_and_keeps_original_unresolved() {
    let temp = tempfile::tempdir().unwrap();
    let f = fixture(temp.path(), "timeout", Duration::from_millis(100));
    let permit = InitialEntryPermit::offline_peer(&f.scope);
    let mut writer = QualifiedWriter::create(&f.root, f.scope.clone()).unwrap();
    assert_eq!(
        writer.execute_initial(permit).await.unwrap(),
        ExecutionPublication::Unresolved
    );
    assert_eq!(counts(&f.witness), (1, 1));
    drop(writer);
    let mut reopened = QualifiedWriter::resume(&f.root, f.scope).unwrap();
    assert_eq!(
        reopened.reconcile().unwrap(),
        ExecutionPublication::Unresolved
    );
    assert_eq!(counts(&f.witness), (1, 1));
}
