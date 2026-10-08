#[allow(dead_code)]
mod common;

use common::{binding, invocation_count, scope};
use review_execution_evidence::{
    CheckedObservation, CleanupObservation, EvidenceReadError, EvidenceReader, EvidenceWriter,
    ExecutionEvidenceOwner, ExecutionPublication, FailureKind, REQUEST_MAX_BYTES,
};

#[tokio::test]
async fn exact_reference_binding_and_profile_refusals() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let count = temp.path().join("count");
    let selected = scope("root-1", &count, "echo");
    let pin = selected.reader_pin().unwrap();
    let mut writer = EvidenceWriter::create(&root, selected.clone()).unwrap();
    assert!(matches!(
        EvidenceWriter::resume(&root, selected.clone()),
        Err(EvidenceReadError::Unresolved(_))
    ));
    let reference = match writer.execute().await.unwrap() {
        ExecutionPublication::Committed(reference) => reference,
        other => panic!("unexpected: {other:?}"),
    };
    drop(writer);
    let reader = EvidenceReader::open(&root, pin.clone()).unwrap();
    let checked = reader.read_result(&reference).unwrap();
    let obs = match checked.observation() {
        CheckedObservation::Present(value) => value,
        _ => panic!(),
    };
    let mut wrong_ref = reference.clone();
    wrong_ref.attempt_id = "other-attempt".into();
    assert_eq!(
        reader.read_result(&wrong_ref).unwrap_err(),
        EvidenceReadError::Absent
    );
    wrong_ref = reference.clone();
    wrong_ref.sha256 = "f".repeat(64);
    assert!(matches!(
        reader.read_result(&wrong_ref),
        Err(EvidenceReadError::Conflicting(_))
    ));
    wrong_ref = reference.clone();
    wrong_ref.profile = "qualified-provider".into();
    assert!(matches!(
        reader.read_result(&wrong_ref),
        Err(EvidenceReadError::Unsupported(_))
    ));
    wrong_ref = reference.clone();
    wrong_ref.schema_version = 2;
    assert!(matches!(
        reader.read_result(&wrong_ref),
        Err(EvidenceReadError::Unsupported(_))
    ));
    let mut wrong_pin = pin;
    wrong_pin.root_id = "other-root".into();
    assert!(matches!(
        EvidenceReader::open(&root, wrong_pin),
        Err(EvidenceReadError::Conflicting(_))
    ));
    let mut wrong_binding = binding(checked.claim_sha256());
    wrong_binding.prepared_request_sha256 = "f".repeat(64);
    assert!(matches!(
        reader.read_custody(&reference, &wrong_binding, Some(obs)),
        Err(EvidenceReadError::Conflicting(_))
    ));
    assert!(matches!(
        reader.read_custody(&reference, &binding(checked.claim_sha256()), None),
        Err(EvidenceReadError::Conflicting(_))
    ));
    let mut wrong_session = binding(checked.claim_sha256());
    wrong_session.session_id = None;
    assert!(matches!(
        reader.read_custody(&reference, &wrong_session, Some(obs)),
        Err(EvidenceReadError::Conflicting(_))
    ));
    assert_eq!(invocation_count(&count), 1);
}

#[tokio::test]
async fn artifact_marker_and_manifest_tampering_refuse_checked_readback() {
    for target in ["artifact", "request", "marker", "manifest", "schema"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let count = temp.path().join("count");
        let selected = scope("root-1", &count, "echo");
        let pin = selected.reader_pin().unwrap();
        let mut writer = EvidenceWriter::create(&root, selected).unwrap();
        let reference = match writer.execute().await.unwrap() {
            ExecutionPublication::Committed(reference) => reference,
            other => panic!("{other:?}"),
        };
        drop(writer);
        let reader = EvidenceReader::open(&root, pin).unwrap();
        reader.read_result(&reference).unwrap();
        let conn = rusqlite::Connection::open(root.join("evidence.sqlite")).unwrap();
        match target {
            "artifact" => {
                conn.execute("UPDATE artifacts SET bytes=x'00' WHERE kind='result'", [])
                    .unwrap();
            }
            "request" => {
                conn.execute("UPDATE artifacts SET bytes=x'00' WHERE kind='request'", [])
                    .unwrap();
            }
            "marker" => {
                conn.execute("UPDATE metadata SET value='wrong' WHERE key='profile'", [])
                    .unwrap();
            }
            "manifest" => {
                conn.execute("UPDATE events SET payload=x'00' WHERE stage='terminal'", [])
                    .unwrap();
            }
            "schema" => {
                conn.execute(
                    "UPDATE metadata SET value='2' WHERE key='schema_version'",
                    [],
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        drop(conn);
        assert!(
            reader.read_result(&reference).is_err(),
            "tampered {target} was accepted"
        );
        assert_eq!(invocation_count(&count), 1);
    }
}

#[tokio::test]
async fn request_and_output_limits_never_create_checked_success() {
    let temp = tempfile::tempdir().unwrap();
    let count = temp.path().join("count");
    let mut oversized = scope("root-1", &count, "echo");
    oversized.request_bytes = vec![b'x'; REQUEST_MAX_BYTES + 1];
    assert!(matches!(
        EvidenceWriter::create(&temp.path().join("oversized"), oversized),
        Err(EvidenceReadError::Capacity(_))
    ));
    let selected = scope("root-2", &count, "overflow");
    let pin = selected.reader_pin().unwrap();
    let root = temp.path().join("overflow");
    let mut writer = EvidenceWriter::create(&root, selected).unwrap();
    assert_eq!(
        writer.execute().await.unwrap(),
        ExecutionPublication::Unresolved
    );
    assert_eq!(invocation_count(&count), 1);
    drop(writer);
    let reader = EvidenceReader::open(&root, pin).unwrap();
    let failure = reader.read_local_failure("attempt-1").unwrap().unwrap();
    assert_eq!(failure.kind, FailureKind::StdoutOverflow);
    assert_eq!(
        failure.stdout_bytes,
        review_execution_evidence::STREAM_MAX_BYTES + 1
    );
    assert!(matches!(
        failure.cleanup,
        CleanupObservation::NotNeeded
            | CleanupObservation::Reaped
            | CleanupObservation::KillFailedButReaped
    ));
    let mut fake = review_execution_evidence::ExecutionEvidenceRef {
        schema_version: 1,
        owner: "review-execution-evidence".into(),
        root_id: "root-2".into(),
        profile: "controlled-process-v1".into(),
        attempt_id: "attempt-1".into(),
        record_id: "terminal".into(),
        revision: "1".into(),
        sha256: "a".repeat(64),
    };
    assert!(reader.read_result(&fake).is_err());
    fake.attempt_id = "absent".into();
    assert_eq!(
        reader.read_result(&fake).unwrap_err(),
        EvidenceReadError::Absent
    );
}

#[tokio::test]
async fn malformed_result_and_timeout_remain_unresolved_after_restart() {
    for mode in ["invalid", "sleep", "exit1"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let count = temp.path().join("count");
        let selected = scope("root-1", &count, mode);
        let pin = selected.reader_pin().unwrap();
        let mut writer = EvidenceWriter::create(&root, selected.clone()).unwrap();
        assert_eq!(
            writer.execute().await.unwrap(),
            ExecutionPublication::Unresolved
        );
        drop(writer);
        let before = invocation_count(&count);
        assert_eq!(before, 1, "controlled peer did not enter for {mode}");
        let mut resumed = EvidenceWriter::resume(&root, selected).unwrap();
        assert_eq!(
            resumed.reconcile().unwrap(),
            ExecutionPublication::Unresolved
        );
        assert_eq!(
            resumed.execute().await.unwrap(),
            ExecutionPublication::Unresolved
        );
        drop(resumed);
        assert_eq!(
            invocation_count(&count),
            before,
            "retry occurred for {mode}"
        );
        let reader = EvidenceReader::open(&root, pin).unwrap();
        let failure = reader.read_local_failure("attempt-1").unwrap().unwrap();
        match mode {
            "invalid" => {
                assert_eq!(failure.kind, FailureKind::MalformedResult);
                assert_eq!(failure.exit_code, Some(0));
                assert_eq!(failure.cleanup, CleanupObservation::NotNeeded);
            }
            "sleep" => {
                assert_eq!(failure.kind, FailureKind::Timeout);
                assert!(matches!(
                    failure.cleanup,
                    CleanupObservation::Reaped | CleanupObservation::KillFailedButReaped
                ));
            }
            "exit1" => {
                assert_eq!(failure.kind, FailureKind::NonzeroExit);
                assert_eq!(failure.exit_code, Some(1));
                assert_eq!(failure.cleanup, CleanupObservation::NotNeeded);
            }
            _ => unreachable!(),
        }
        let reference = review_execution_evidence::ExecutionEvidenceRef {
            schema_version: 1,
            owner: "review-execution-evidence".into(),
            root_id: "root-1".into(),
            profile: "controlled-process-v1".into(),
            attempt_id: "attempt-1".into(),
            record_id: "terminal".into(),
            revision: "1".into(),
            sha256: "a".repeat(64),
        };
        assert!(reader.read_result(&reference).is_err());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn substituted_database_and_linked_root_refuse_readback() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let count = temp.path().join("count");
    let selected = scope("root-1", &count, "echo");
    let pin = selected.reader_pin().unwrap();
    let mut writer = EvidenceWriter::create(&root, selected).unwrap();
    let reference = match writer.execute().await.unwrap() {
        ExecutionPublication::Committed(reference) => reference,
        other => panic!("{other:?}"),
    };
    drop(writer);
    let linked_root = temp.path().join("linked-root");
    symlink(&root, &linked_root).unwrap();
    assert!(EvidenceReader::open(&linked_root, pin.clone()).is_err());
    let reader = EvidenceReader::open(&root, pin).unwrap();
    let db = root.join("evidence.sqlite");
    let moved = root.join("moved.sqlite");
    std::fs::rename(&db, &moved).unwrap();
    symlink(&moved, &db).unwrap();
    assert!(reader.read_result(&reference).is_err());
    assert_eq!(invocation_count(&count), 1);
}

#[cfg(unix)]
#[tokio::test]
async fn regular_replacement_of_database_root_and_fence_is_refused() {
    use std::os::unix::fs::MetadataExt;

    for target in ["database", "root", "fence"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let count = temp.path().join("count");
        let selected = scope("root-1", &count, "echo");
        let pin = selected.reader_pin().unwrap();
        let mut writer = EvidenceWriter::create(&root, selected.clone()).unwrap();
        let reference = match writer.execute().await.unwrap() {
            ExecutionPublication::Committed(reference) => reference,
            other => panic!("{other:?}"),
        };
        let checkpoint = rusqlite::Connection::open(root.join("evidence.sqlite")).unwrap();
        checkpoint
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .unwrap();
        drop(checkpoint);
        let reader = EvidenceReader::open(&root, pin.clone()).unwrap();
        reader.read_result(&reference).unwrap();
        match target {
            "database" => {
                let db = root.join("evidence.sqlite");
                let copy = root.join("copy.sqlite");
                std::fs::copy(&db, &copy).unwrap();
                let id = std::fs::metadata(&copy).unwrap();
                let copied = rusqlite::Connection::open(&copy).unwrap();
                for (key, value) in [("db_dev", id.dev()), ("db_ino", id.ino())] {
                    copied
                        .execute(
                            "UPDATE metadata SET value=?1 WHERE key=?2",
                            rusqlite::params![value.to_string(), key],
                        )
                        .unwrap();
                }
                copied
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                    .unwrap();
                drop(copied);
                std::fs::rename(&copy, &db).unwrap();
            }
            "fence" => {
                let lock = root.join("writer.lock");
                let copy = root.join("copy.lock");
                std::fs::copy(&lock, &copy).unwrap();
                let id = std::fs::metadata(&copy).unwrap();
                std::fs::rename(&copy, &lock).unwrap();
                let marker = rusqlite::Connection::open(root.join("evidence.sqlite")).unwrap();
                for (key, value) in [("fence_dev", id.dev()), ("fence_ino", id.ino())] {
                    marker
                        .execute(
                            "UPDATE metadata SET value=?1 WHERE key=?2",
                            rusqlite::params![value.to_string(), key],
                        )
                        .unwrap();
                }
                drop(marker);
            }
            "root" => {
                let old = temp.path().join("old-root");
                let replacement = temp.path().join("replacement");
                std::fs::create_dir(&replacement).unwrap();
                for name in ["evidence.sqlite", "writer.lock"] {
                    std::fs::copy(root.join(name), replacement.join(name)).unwrap();
                }
                std::fs::rename(&root, &old).unwrap();
                std::fs::rename(&replacement, &root).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            reader.read_result(&reference).is_err(),
            "old reader accepted replaced {target}"
        );
        assert!(
            writer.reconcile().is_err(),
            "old writer accepted replaced {target}"
        );
        let second = EvidenceWriter::resume(&root, selected);
        assert!(second.is_err(), "second writer accepted replaced {target}");
        if target == "fence" {
            assert!(matches!(
                second,
                Err(EvidenceReadError::Unresolved(
                    "another root writer is active"
                ))
            ));
        }
        assert_eq!(invocation_count(&count), 1);
    }
}

#[cfg(unix)]
#[test]
fn wrong_pin_resume_leaves_delete_mode_and_writer_resources_untouched() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let count = temp.path().join("count");
    let selected = scope("root-1", &count, "echo");
    drop(EvidenceWriter::create(&root, selected.clone()).unwrap());
    let db = root.join("evidence.sqlite");
    let conn = rusqlite::Connection::open(&db).unwrap();
    let mode: String = conn
        .query_row("PRAGMA journal_mode=DELETE", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode.to_lowercase(), "delete");
    drop(conn);
    let before_lock = std::fs::metadata(root.join("writer.lock")).unwrap();
    let mut wrong = selected;
    wrong.root_id = "wrong-root".into();
    assert!(matches!(
        EvidenceWriter::resume(&root, wrong),
        Err(EvidenceReadError::Conflicting(_))
    ));
    let conn = rusqlite::Connection::open(&db).unwrap();
    let after: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(after.to_lowercase(), "delete");
    assert!(!root.join("evidence.sqlite-wal").exists());
    assert_eq!(
        std::fs::metadata(root.join("writer.lock")).unwrap().len(),
        before_lock.len()
    );
}

#[cfg(unix)]
#[test]
fn linked_writer_fence_and_sqlite_sidecar_are_refused() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let count = temp.path().join("count");
    let selected = scope("root-1", &count, "echo");
    let pin = selected.reader_pin().unwrap();
    drop(EvidenceWriter::create(&root, selected.clone()).unwrap());
    let lock = root.join("writer.lock");
    std::fs::rename(&lock, root.join("old.lock")).unwrap();
    symlink(root.join("old.lock"), &lock).unwrap();
    assert!(EvidenceWriter::resume(&root, selected.clone()).is_err());
    assert!(EvidenceReader::open(&root, pin.clone()).is_err());
    std::fs::remove_file(&lock).unwrap();
    std::fs::rename(root.join("old.lock"), &lock).unwrap();
    let checkpoint = rusqlite::Connection::open(root.join("evidence.sqlite")).unwrap();
    checkpoint
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .unwrap();
    drop(checkpoint);
    let wal = root.join("evidence.sqlite-wal");
    if wal.exists() {
        std::fs::remove_file(&wal).unwrap();
    }
    symlink(&lock, &wal).unwrap();
    assert!(EvidenceReader::open(&root, pin).is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn validated_prelaunch_refusal_is_distinct_from_spawn_uncertainty() {
    use std::os::unix::fs::PermissionsExt;
    for mode in ["changed-bytes", "spawn-permission"] {
        let temp = tempfile::tempdir().unwrap();
        let count = temp.path().join("count");
        let mut selected = scope("root-1", &count, "echo");
        let executable_copy = temp.path().join("selected-peer");
        std::fs::copy(&selected.executable, &executable_copy).unwrap();
        selected.executable = executable_copy.clone();
        selected.executable_sha256 = common::sha(&std::fs::read(&executable_copy).unwrap());
        let root = temp.path().join("root");
        let mut writer = EvidenceWriter::create(&root, selected.clone()).unwrap();
        if mode == "changed-bytes" {
            let mut bytes = std::fs::read(&executable_copy).unwrap();
            bytes[0] ^= 1;
            std::fs::write(&executable_copy, bytes).unwrap();
        } else {
            let mut permissions = std::fs::metadata(&executable_copy).unwrap().permissions();
            permissions.set_mode(0o600);
            std::fs::set_permissions(&executable_copy, permissions).unwrap();
        }
        let outcome = writer.execute().await.unwrap();
        if mode == "changed-bytes" {
            assert_eq!(outcome, ExecutionPublication::DefinitePreEntryFailure);
        } else {
            assert_eq!(outcome, ExecutionPublication::Unresolved);
        }
        assert_eq!(invocation_count(&count), 0);
        assert_eq!(writer.reconcile().unwrap(), outcome);
        drop(writer);
        let resumed = EvidenceWriter::resume(&root, selected).unwrap();
        assert_eq!(resumed.reconcile().unwrap(), outcome);
        assert_eq!(invocation_count(&count), 0);
    }
}
