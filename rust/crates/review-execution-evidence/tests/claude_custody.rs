#![cfg(feature = "test-support")]
mod claude_common;

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use claude_common::{counts, fixture, fixture_with_credential, sha};
use review_execution_evidence::claude::{
    CredentialSource, InitialEntryPermit, QualifiedReader, QualifiedWriter, SessionEvolutionPermit,
};
use review_execution_evidence::{ExecutionEvidenceOwner, ExecutionPublication};

async fn completed(
    parent: &Path,
) -> (
    PathBuf,
    PathBuf,
    review_execution_evidence::claude::QualifiedReaderSelection,
    review_execution_evidence::ExecutionEvidenceRef,
) {
    let f = fixture(parent, "success", Duration::from_secs(2));
    let selection = f.scope.reader_selection().unwrap();
    let permit = InitialEntryPermit::offline_peer(&f.scope);
    let mut writer = QualifiedWriter::create(&f.root, f.scope).unwrap();
    let reference = match writer.execute_initial(permit).await.unwrap() {
        ExecutionPublication::Committed(reference) => reference,
        other => panic!("unexpected: {other:?}"),
    };
    drop(writer);
    (f.root, f.witness, selection, reference)
}

#[tokio::test]
async fn root_database_and_fence_replacement_refuse_readback() {
    for kind in ["root", "database", "fence"] {
        let temp = tempfile::tempdir().unwrap();
        let (root, _, selection, reference) = completed(temp.path()).await;
        let reader = QualifiedReader::open(&root, selection).unwrap();
        reader.read_result(&reference).unwrap();
        match kind {
            "root" => {
                fs::rename(&root, temp.path().join("old-root")).unwrap();
                fs::create_dir(&root).unwrap();
            }
            "database" => {
                let db = root.join("evidence.sqlite");
                let replacement = root.join("replacement.sqlite");
                fs::copy(&db, &replacement).unwrap();
                fs::rename(&replacement, &db).unwrap();
            }
            "fence" => {
                let fence = root.join("writer.lock");
                fs::rename(&fence, root.join("old-fence")).unwrap();
                fs::write(&fence, b"").unwrap();
            }
            _ => unreachable!(),
        }
        assert!(reader.read_result(&reference).is_err(), "{kind}");
    }
}

#[tokio::test]
async fn immutable_snapshot_and_live_session_have_distinct_checks() {
    let temp = tempfile::tempdir().unwrap();
    let (root, witness, selection, reference) = completed(temp.path()).await;
    let reader = QualifiedReader::open(&root, selection.clone()).unwrap();
    let retained = reader.retained_session(&reference).unwrap();
    retained.verify_current().unwrap();
    let snapshot = root
        .join("session-snapshots")
        .join(sha(b"attempt-1"))
        .join("session.json");
    fs::write(&snapshot, b"changed").unwrap();
    assert!(reader.read_result(&reference).is_err());
    assert!(reader.retained_session(&reference).is_err());
    assert_eq!(counts(&witness), (1, 1));

    let temp = tempfile::tempdir().unwrap();
    let (root, _, selection, reference) = completed(temp.path()).await;
    let reader = QualifiedReader::open(&root, selection).unwrap();
    fs::write(
        root.join("session-live/session.json"),
        b"later but unregistered",
    )
    .unwrap();
    // The initial committed record remains readable from its immutable snapshot.
    reader.read_result(&reference).unwrap();
    assert!(reader.retained_session(&reference).is_err());
}

#[tokio::test]
async fn links_and_missing_live_session_refuse_capability() {
    for kind in ["symlink", "hardlink", "missing"] {
        let temp = tempfile::tempdir().unwrap();
        let (root, _, selection, reference) = completed(temp.path()).await;
        let live = root.join("session-live/session.json");
        match kind {
            "symlink" => {
                fs::rename(&live, root.join("saved-session")).unwrap();
                symlink(root.join("saved-session"), &live).unwrap();
            }
            "hardlink" => {
                fs::hard_link(&live, root.join("second-link")).unwrap();
            }
            "missing" => {
                fs::rename(root.join("session-live"), root.join("lost-session")).unwrap();
            }
            _ => unreachable!(),
        }
        let reader = QualifiedReader::open(&root, selection).unwrap();
        assert!(reader.retained_session(&reference).is_err(), "{kind}");
    }
}

#[tokio::test]
async fn synthetic_credential_canary_never_enters_custody() {
    let temp = tempfile::tempdir().unwrap();
    let credential_path = temp.path().join("credential.json");
    let token = "synthetic-credential-canary-74f2";
    let expiry = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + 3_600_000;
    fs::write(
        &credential_path,
        serde_json::json!({"claudeAiOauth":{
        "accessToken":token,"expiresAt":expiry,"refreshToken":"must-stay-out"}})
        .to_string(),
    )
    .unwrap();
    fs::set_permissions(&credential_path, fs::Permissions::from_mode(0o600)).unwrap();
    let source = CredentialSource::new(&credential_path).unwrap();
    let f = fixture_with_credential(temp.path(), "leak", Duration::from_secs(2), Some(source));
    let permit = InitialEntryPermit::offline_peer(&f.scope);
    let mut writer = QualifiedWriter::create(&f.root, f.scope).unwrap();
    assert_eq!(
        writer.execute_initial(permit).await.unwrap(),
        ExecutionPublication::Unresolved
    );
    drop(writer);
    assert_eq!(counts(&f.witness), (1, 1));
    for path in [
        f.root.join("evidence.sqlite"),
        f.root.join("evidence.sqlite-wal"),
    ]
    .into_iter()
    .filter(|path| path.exists())
    {
        let bytes = fs::read(path).unwrap();
        assert!(
            !bytes
                .windows(token.len())
                .any(|part| part == token.as_bytes())
        );
        assert!(
            !bytes
                .windows("must-stay-out".len())
                .any(|part| part == b"must-stay-out")
        );
    }
    assert!(
        CredentialSource::new(&credential_path)
            .unwrap()
            .read_access_only(expiry, 1)
            .is_err()
    );
}

#[tokio::test]
async fn expired_missing_and_replaced_credentials_refuse_before_launch() {
    for kind in ["expired", "missing_expiry", "replaced"] {
        let temp = tempfile::tempdir().unwrap();
        let credential_path = temp.path().join("credential.json");
        let expiry = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
            + 3_600_000;
        let value = match kind {
            "expired" => {
                serde_json::json!({"claudeAiOauth":{"accessToken":"synthetic", "expiresAt":1}})
            }
            "missing_expiry" => serde_json::json!({"claudeAiOauth":{"accessToken":"synthetic"}}),
            _ => {
                serde_json::json!({"claudeAiOauth":{"accessToken":"synthetic", "expiresAt":expiry}})
            }
        };
        fs::write(&credential_path, value.to_string()).unwrap();
        fs::set_permissions(&credential_path, fs::Permissions::from_mode(0o600)).unwrap();
        let source = CredentialSource::new(&credential_path).unwrap();
        let f =
            fixture_with_credential(temp.path(), "success", Duration::from_secs(2), Some(source));
        let permit = InitialEntryPermit::offline_peer(&f.scope);
        let mut writer = QualifiedWriter::create(&f.root, f.scope.clone()).unwrap();
        if kind == "replaced" {
            let replacement = temp.path().join("replacement.json");
            fs::write(&replacement, value.to_string()).unwrap();
            fs::set_permissions(&replacement, fs::Permissions::from_mode(0o600)).unwrap();
            fs::rename(replacement, credential_path).unwrap();
        }
        assert_eq!(
            writer.execute_initial(permit).await.unwrap(),
            ExecutionPublication::DefinitePreEntryFailure,
            "{kind}"
        );
        drop(writer);
        let mut reopened = QualifiedWriter::resume(&f.root, f.scope).unwrap();
        assert_eq!(
            reopened.reconcile().unwrap(),
            ExecutionPublication::DefinitePreEntryFailure
        );
        assert_eq!(counts(&f.witness), (0, 0), "{kind}");
    }
}

#[tokio::test]
async fn registered_session_evolution_links_immutable_predecessors_without_new_entry() {
    let temp = tempfile::tempdir().unwrap();
    let f = fixture(temp.path(), "success", Duration::from_secs(2));
    let selection = f.scope.reader_selection().unwrap();
    let permit = InitialEntryPermit::offline_peer(&f.scope);
    let mut writer = QualifiedWriter::create(&f.root, f.scope.clone()).unwrap();
    let reference = match writer.execute_initial(permit).await.unwrap() {
        ExecutionPublication::Committed(reference) => reference,
        other => panic!("unexpected initial: {other:?}"),
    };
    drop(writer);
    let reader = QualifiedReader::open(&f.root, selection.clone()).unwrap();
    let initial = reader.retained_session(&reference).unwrap();
    assert!(
        reader
            .retained_session_after(&reference, "unknown-attempt")
            .is_err()
    );
    drop(reader);
    let first_snapshot = f
        .root
        .join("session-snapshots")
        .join(sha(b"attempt-1"))
        .join("session.json");
    let original = fs::read(&first_snapshot).unwrap();
    fs::write(
        f.root.join("session-live/session.json"),
        b"{\"session_id\":\"session-1\",\"turn\":2}",
    )
    .unwrap();
    let permit = SessionEvolutionPermit::offline_peer(&f.scope, &initial, "attempt-2").unwrap();
    let mut writer = QualifiedWriter::resume(&f.root, f.scope.clone()).unwrap();
    let second = writer.record_session_evolution(initial, permit).unwrap();
    assert_eq!(second.locator().attempt_id, "attempt-2");
    second.verify_current().unwrap();
    fs::write(
        f.root.join("session-live/session.json"),
        b"{\"session_id\":\"session-1\",\"turn\":3}",
    )
    .unwrap();
    let permit = SessionEvolutionPermit::offline_peer(&f.scope, &second, "attempt-3").unwrap();
    let third = writer.record_session_evolution(second, permit).unwrap();
    assert_eq!(third.locator().attempt_id, "attempt-3");
    third.verify_current().unwrap();
    drop(writer);
    let reader = QualifiedReader::open(&f.root, selection).unwrap();
    reader.read_result(&reference).unwrap();
    assert!(reader.retained_session(&reference).is_err());
    assert!(
        reader
            .retained_session_after(&reference, "attempt-2")
            .is_err()
    );
    reader
        .retained_session_after(&reference, "attempt-3")
        .unwrap()
        .verify_current()
        .unwrap();
    assert_eq!(fs::read(&first_snapshot).unwrap(), original);
    assert_eq!(counts(&f.witness), (1, 1));
    fs::write(
        f.root
            .join("session-snapshots")
            .join(sha(b"attempt-2"))
            .join("session.json"),
        b"changed",
    )
    .unwrap();
    assert!(
        reader
            .retained_session_after(&reference, "attempt-3")
            .is_err()
    );
    reader.read_result(&reference).unwrap();
}

#[tokio::test]
async fn cumulative_retained_and_live_storage_refuses_successor_before_new_custody() {
    let temp = tempfile::tempdir().unwrap();
    let f = fixture(temp.path(), "success", Duration::from_secs(2));
    let selection = f.scope.reader_selection().unwrap();
    let permit = InitialEntryPermit::offline_peer(&f.scope);
    let mut writer = QualifiedWriter::create(&f.root, f.scope.clone()).unwrap();
    let reference = match writer.execute_initial(permit).await.unwrap() {
        ExecutionPublication::Committed(reference) => reference,
        other => panic!("unexpected initial: {other:?}"),
    };
    let reader = QualifiedReader::open(&f.root, selection).unwrap();
    let mut prior = reader.retained_session(&reference).unwrap();
    let content = vec![b'X'; 12 * 1024 * 1024];
    for number in 2..=4 {
        fs::write(f.root.join("session-live/session.json"), &content).unwrap();
        let attempt = format!("attempt-{number}");
        let permit = SessionEvolutionPermit::offline_peer(&f.scope, &prior, &attempt).unwrap();
        prior = writer.record_session_evolution(prior, permit).unwrap();
        prior.verify_current().unwrap();
    }
    let permit = SessionEvolutionPermit::offline_peer(&f.scope, &prior, "attempt-5").unwrap();
    assert!(writer.record_session_evolution(prior, permit).is_err());
    assert!(
        !f.root
            .join("session-snapshots")
            .join(sha(b"attempt-5"))
            .exists()
    );
    let reader = QualifiedReader::open(&f.root, f.scope.reader_selection().unwrap()).unwrap();
    reader.read_result(&reference).unwrap();
    reader
        .retained_session_after(&reference, "attempt-4")
        .unwrap()
        .verify_current()
        .unwrap();
    assert_eq!(counts(&f.witness), (1, 1));
}
