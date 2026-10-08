#[allow(dead_code)]
mod common;

use common::{binding, invocation_count, scope};
use review_execution_evidence::{
    CheckedObservation, EvidenceClass, EvidenceReader, EvidenceWriter, ExecutionEvidenceOwner,
    ExecutionPublication,
};

#[tokio::test]
async fn actual_child_commits_and_reopens_without_reinvocation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("evidence-root");
    let count = temp.path().join("invocations");
    let selected = scope("root-1", &count, "echo");
    let pin = selected.reader_pin().unwrap();
    let mut writer = EvidenceWriter::create(&root, selected.clone()).unwrap();
    let reference = match writer.execute().await.unwrap() {
        ExecutionPublication::Committed(reference) => reference,
        other => panic!("unexpected publication: {other:?}"),
    };
    assert_eq!(invocation_count(&count), 1);
    assert_eq!(
        writer.execute().await.unwrap(),
        ExecutionPublication::Replayed(reference.clone())
    );
    assert_eq!(invocation_count(&count), 1);
    drop(writer);
    let reader = EvidenceReader::open(&root, pin.clone()).unwrap();
    let checked = reader.read_result(&reference).unwrap();
    assert_eq!(checked.evidence_class(), EvidenceClass::ControlledProcess);
    assert_eq!(
        checked.value().get("request").and_then(|v| v.get("task")),
        Some(&claim_evidence::codec::JsValue::text("inspect"))
    );
    let observation = match checked.observation() {
        CheckedObservation::Present(value) => value,
        _ => panic!("owned observation missing"),
    };
    let custody = reader
        .read_custody(
            &reference,
            &binding(checked.claim_sha256()),
            Some(observation),
        )
        .unwrap();
    assert_eq!(custody.artifact_references.len(), 2);
    assert_eq!(custody.session_id.as_deref(), Some("controlled-session-1"));
    assert_eq!(invocation_count(&count), 1);
    let probe = std::process::Command::new(env!("CARGO_BIN_EXE_reader_probe"))
        .arg(&root)
        .arg(&pin.root_id)
        .arg(&pin.configuration_sha256)
        .arg(&pin.executable_sha256)
        .arg(&pin.source_sha256)
        .arg(&pin.profile)
        .arg(&reference.attempt_id)
        .arg(&reference.record_id)
        .arg(&reference.revision)
        .arg(&reference.sha256)
        .output()
        .unwrap();
    assert!(
        probe.status.success(),
        "fresh reader failed: {}",
        String::from_utf8_lossy(&probe.stderr)
    );
    assert_eq!(
        String::from_utf8(probe.stdout).unwrap().trim(),
        checked.claim_sha256()
    );
    assert_eq!(invocation_count(&count), 1);
}
