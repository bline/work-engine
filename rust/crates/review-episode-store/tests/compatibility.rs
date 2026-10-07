use std::fs;
use std::path::Path;
use std::process::Command;

use review_episode_core::codec::{JsValue, canonical_json, digest, field, parse_json};
use review_episode_core::command::BeginCommand;
use review_episode_core::identity::Authority;
use review_episode_core::reducer;
use review_episode_store::{
    CopyManifest, EpisodeStore, StoreOptions, WriteDisposition, import_closed_copy,
    init_offline_root, reconcile_closed_copy,
};
use sha2::{Digest, Sha256};

const STATES: &str =
    include_str!("../../../../app-server/tests/fixtures/review-episode-rust/states-v1-v2.json");
const ACCEPTABLE: &str = include_str!(
    "../../../../app-server/tests/fixtures/implementation-review/acceptable-as-is.json"
);

fn sha(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

fn rust_begin_authority() -> Authority {
    let fixture = parse_json(STATES).unwrap();
    let result = parse_json(ACCEPTABLE).unwrap();
    let subject = field(&result, "subject").unwrap();
    let reference = |owner: &str, name: &str, revision: &str, raw: &str| {
        JsValue::object([
            ("owner", JsValue::text(owner)),
            ("reference", JsValue::text(name)),
            ("revision", JsValue::text(revision)),
            (
                "sha256",
                JsValue::text(&format!("{:x}", Sha256::digest(raw.as_bytes()))),
            ),
            ("freshness", JsValue::text("exact_revision")),
        ])
    };
    let mut initial = reference(
        "checkpoint",
        "candidate",
        "candidate-commit",
        "candidate-commit",
    );
    if let JsValue::Object(map) = &mut initial {
        map.insert("revision".into(), field(subject, "commit").unwrap().clone());
        map.insert("sha256".into(), JsValue::text(&digest(subject)));
    }
    Authority::parse(JsValue::object([
        ("schemaVersion", JsValue::Number(1.0)),
        ("grantId", JsValue::text("grant-1")),
        ("identity", field(&fixture, "identity").unwrap().clone()),
        (
            "source",
            reference("human", "accepted-plan", "plan-v1", "accepted-plan"),
        ),
        (
            "writer",
            JsValue::object([
                ("actorId", JsValue::text("reviewer")),
                ("provider", JsValue::text("fixture")),
                ("generation", JsValue::Number(1.0)),
                (
                    "runtimeSession",
                    reference("runtime", "session-1", "generation-1", "session-1"),
                ),
            ]),
        ),
        (
            "readers",
            JsValue::Array(vec![
                JsValue::text("reviewer"),
                JsValue::text("builder"),
                JsValue::text("supervisor"),
            ]),
        ),
        ("initialSubject", initial),
        ("predecessorRevision", JsValue::Null),
    ]))
    .unwrap()
}

#[test]
fn rust_reducer_row_is_read_by_legacy_js_with_exact_bytes_and_revision() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("rust-root");
    init_offline_root(&root).unwrap();
    let authority = rust_begin_authority();
    let key = authority.identity().key().0;
    let state = reducer::begin(
        None,
        &BeginCommand::new(authority, "begin", vec![]).unwrap(),
    )
    .unwrap()
    .into_state();
    let expected_json = canonical_json(state.value());
    let expected_revision = state.revision().0.clone();
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    store
        .write(&key, None, 32 * 1024 * 1024, |_| {
            Ok(WriteDisposition::Applied {
                state: Box::new(state.clone()),
                reply_json: "{}".into(),
            })
        })
        .unwrap();
    let history = store.read(&key).unwrap().history;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].state_json, expected_json);
    store.checkpoint().unwrap();
    drop(store);

    let module = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../app-server/src/services/review-episode/sqlite-store.mjs");
    let script = r#"import {pathToFileURL} from 'node:url';
      const {openSqliteReviewEpisodeStore}=await import(pathToFileURL(process.argv[1]));
      const store=await openSqliteReviewEpisodeStore({filePath:process.argv[2]});
      const current=store.get(process.argv[3]);const history=store.history(process.argv[3]);
      const raw=store.database.prepare('SELECT revision,state_json FROM review_episode_history WHERE identity_key=?').get(process.argv[3]);
      if(current?.revision!==process.argv[4]||history.length!==1||history[0].revision!==process.argv[4])throw Error('legacy read revision differs');
      if(raw?.revision!==process.argv[4]||raw?.state_json!==process.argv[5])throw Error('legacy read raw bytes differ');
      store.close();"#;
    let output = Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script)
        .arg(module)
        .arg(root.join("review-episodes.sqlite"))
        .arg(&key)
        .arg(&expected_revision)
        .arg(&expected_json)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "legacy JS rejected Rust row: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn js_sqlite_rows_copy_without_rewriting_canonical_history() {
    let temp = tempfile::tempdir().unwrap();
    let active = temp.path().join("active.sqlite");
    let source = temp.path().join("legacy.sqlite");
    let destination = temp.path().join("imported");
    let module = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../app-server/src/services/review-episode/sqlite-store.mjs");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../app-server/tests/fixtures/review-episode-rust/states-v1-v2.json");
    let create = r#"import fs from 'node:fs'; import {pathToFileURL} from 'node:url';
      const {openSqliteReviewEpisodeStore}=await import(pathToFileURL(process.argv[1]));
      const x=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
      const store=await openSqliteReviewEpisodeStore({filePath:process.argv[3]});
      let prior=null;for(const name of ['v1_begin','v2_blocked','v2_succeeded']){
        const state=JSON.parse(x.states[name].canonical);store.put(x.identityKey,state,prior);prior=state.revision;
      }store.close();"#;
    let output = Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(create)
        .arg(&module)
        .arg(&fixture)
        .arg(&active)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Node source failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let backup=Command::new("python3").arg("-c").arg("import sqlite3,sys; a=sqlite3.connect(sys.argv[1]); b=sqlite3.connect(sys.argv[2]); a.backup(b); b.execute('PRAGMA journal_mode=DELETE'); b.close(); a.close()").arg(&active).arg(&source).output().unwrap();
    assert!(
        backup.status.success(),
        "closed copy failed: {}",
        String::from_utf8_lossy(&backup.stderr)
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let conn = rusqlite::Connection::open(&source).unwrap();
    conn.execute_batch("PRAGMA user_version=7; UPDATE sqlite_sequence SET seq=9 WHERE name='review_episode_history'").unwrap();
    drop(conn);
    let source_hash = sha(&source);
    let report = import_closed_copy(
        &CopyManifest {
            source: source.clone(),
            sha256: source_hash.clone(),
            provenance: "generated JS offline fixture".into(),
        },
        &destination,
    )
    .unwrap();
    assert_eq!(report.source_sha256, source_hash);
    assert_eq!(report.episodes, 1);
    assert_eq!(report.rows, 3);
    assert!(report.byte_equal);
    let reconciled = reconcile_closed_copy(
        &CopyManifest {
            source: source.clone(),
            sha256: source_hash.clone(),
            provenance: "generated JS offline fixture".into(),
        },
        &destination,
    )
    .unwrap();
    assert_eq!((reconciled.episodes, reconciled.rows), (1, 3));
    let fixture = parse_json(STATES).unwrap();
    let key = field(&fixture, "identityKey")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let mut store = EpisodeStore::open(&destination, StoreOptions::default()).unwrap();
    let snapshot = store.read(&key).unwrap();
    assert_eq!(snapshot.history.len(), 3);
    let states = field(&fixture, "states").unwrap();
    for (row, name) in snapshot
        .history
        .iter()
        .zip(["v1_begin", "v2_blocked", "v2_succeeded"])
    {
        let expected = field(field(states, name).unwrap(), "canonical")
            .unwrap()
            .as_text()
            .unwrap()
            .to_string_checked()
            .unwrap();
        assert_eq!(row.state_json, expected);
    }
    store.checkpoint().unwrap();
    drop(store);
    let imported = rusqlite::Connection::open(destination.join("review-episodes.sqlite")).unwrap();
    let highwater: i64 = imported
        .query_row(
            "SELECT seq FROM sqlite_sequence WHERE name='review_episode_history'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let user_version: i64 = imported
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!((highwater, user_version), (9, 7));
    drop(imported);
    let read = r#"import {pathToFileURL} from 'node:url';const {openSqliteReviewEpisodeStore}=await import(pathToFileURL(process.argv[1]));
      const store=await openSqliteReviewEpisodeStore({filePath:process.argv[2]});
      const history=store.history(process.argv[3]);if(history.length!==3)throw Error('history differs');store.close();"#;
    let output = Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(read)
        .arg(&module)
        .arg(destination.join("review-episodes.sqlite"))
        .arg(&key)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Node imported read failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let destination_db = destination.join("review-episodes.sqlite");
    assert!(
        reconcile_closed_copy(
            &CopyManifest {
                source: destination_db.clone(),
                sha256: sha(&destination_db),
                provenance: "alias".into(),
            },
            &destination
        )
        .is_err()
    );
    assert!(
        import_closed_copy(
            &CopyManifest {
                source: source.clone(),
                sha256: source_hash.clone(),
                provenance: "generated JS offline fixture".into()
            },
            &destination
        )
        .is_err()
    );
    let wrong = temp.path().join("wrong-digest");
    assert!(
        import_closed_copy(
            &CopyManifest {
                source: source.clone(),
                sha256: "0".repeat(64),
                provenance: "fixture".into()
            },
            &wrong
        )
        .is_err()
    );
    assert!(!wrong.exists());
    let sidecar = source.with_extension("sqlite-wal");
    fs::write(&sidecar, b"pending").unwrap();
    let sidecar_dest = temp.path().join("sidecar-reject");
    assert!(
        import_closed_copy(
            &CopyManifest {
                source: source.clone(),
                sha256: source_hash.clone(),
                provenance: "fixture".into()
            },
            &sidecar_dest
        )
        .is_err()
    );
    assert!(!sidecar_dest.exists());
    fs::remove_file(&sidecar).unwrap();
    let source_db = rusqlite::Connection::open(&source).unwrap();
    source_db
        .execute(
            "UPDATE review_episode_history SET state_json='{}' WHERE sequence=2",
            [],
        )
        .unwrap();
    drop(source_db);
    let corrupt = temp.path().join("corrupt-reject");
    assert!(
        import_closed_copy(
            &CopyManifest {
                source: source.clone(),
                sha256: sha(&source),
                provenance: "fixture".into()
            },
            &corrupt
        )
        .is_err()
    );
    assert!(!corrupt.exists());
}

#[test]
fn empty_history_copy_preserves_sequence_row_absence_zero_and_nonzero() {
    use rusqlite::OptionalExtension;
    let temp = tempfile::tempdir().unwrap();
    for (name, highwater) in [("absent", None), ("zero", Some(0)), ("nonzero", Some(7))] {
        let source = temp.path().join(format!("{name}.sqlite"));
        let destination = temp.path().join(format!("{name}-imported"));
        let conn = rusqlite::Connection::open(&source).unwrap();
        conn.execute_batch("CREATE TABLE review_episode_current (identity_key TEXT PRIMARY KEY, revision TEXT NOT NULL UNIQUE CHECK(length(revision)=64), state_json TEXT NOT NULL) STRICT; CREATE TABLE review_episode_history (sequence INTEGER PRIMARY KEY AUTOINCREMENT, identity_key TEXT NOT NULL, revision TEXT NOT NULL UNIQUE CHECK(length(revision)=64), predecessor_revision TEXT, state_json TEXT NOT NULL) STRICT;").unwrap();
        if let Some(value) = highwater {
            conn.execute(
                "INSERT INTO sqlite_sequence(name,seq) VALUES('review_episode_history',?1)",
                [value],
            )
            .unwrap();
        }
        drop(conn);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let manifest = CopyManifest {
            source: source.clone(),
            sha256: sha(&source),
            provenance: format!("generated empty-history {name} fixture"),
        };
        let report = import_closed_copy(&manifest, &destination).unwrap();
        assert_eq!((report.episodes, report.rows), (0, 0));
        assert_eq!(report.history_highwater, highwater);
        let reconciled = reconcile_closed_copy(&manifest, &destination).unwrap();
        assert_eq!(reconciled.history_highwater, highwater);
        let db = rusqlite::Connection::open(destination.join("review-episodes.sqlite")).unwrap();
        let retained: Option<i64> = db
            .query_row(
                "SELECT seq FROM sqlite_sequence WHERE name='review_episode_history'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(retained, highwater, "{name}");
    }
}

#[cfg(feature = "test-faults")]
#[test]
#[ignore]
fn import_child() {
    let source = std::path::PathBuf::from(std::env::var("R2_IMPORT_SOURCE").unwrap());
    let destination = std::path::PathBuf::from(std::env::var("R2_IMPORT_DEST").unwrap());
    let sha256 = std::env::var("R2_IMPORT_SHA").unwrap();
    let manifest = CopyManifest {
        source,
        sha256,
        provenance: "generated offline crash fixture".into(),
    };
    import_closed_copy(&manifest, &destination).unwrap();
}

#[cfg(feature = "test-faults")]
#[test]
fn import_process_kills_before_and_after_publish_reconcile_exact_copy() {
    use std::time::{Duration, Instant};
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("closed-source.sqlite");
    let fixture = parse_json(STATES).unwrap();
    let key = field(&fixture, "identityKey")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let states = field(&fixture, "states").unwrap();
    let conn = rusqlite::Connection::open(&source).unwrap();
    conn.execute_batch("CREATE TABLE review_episode_current (identity_key TEXT PRIMARY KEY, revision TEXT NOT NULL UNIQUE CHECK(length(revision)=64), state_json TEXT NOT NULL) STRICT; CREATE TABLE review_episode_history (sequence INTEGER PRIMARY KEY AUTOINCREMENT, identity_key TEXT NOT NULL, revision TEXT NOT NULL UNIQUE CHECK(length(revision)=64), predecessor_revision TEXT, state_json TEXT NOT NULL) STRICT;").unwrap();
    let mut predecessor: Option<String> = None;
    for name in ["v1_begin", "v2_blocked", "v2_succeeded"] {
        let row = field(states, name).unwrap();
        let revision = field(row, "revision")
            .unwrap()
            .as_text()
            .unwrap()
            .to_string_checked()
            .unwrap();
        let json = field(row, "canonical")
            .unwrap()
            .as_text()
            .unwrap()
            .to_string_checked()
            .unwrap();
        conn.execute("INSERT INTO review_episode_history(identity_key,revision,predecessor_revision,state_json) VALUES(?1,?2,?3,?4)",rusqlite::params![key,revision,predecessor,json]).unwrap();
        predecessor = Some(revision);
        if name == "v2_succeeded" {
            conn.execute("INSERT INTO review_episode_current(identity_key,revision,state_json) VALUES(?1,?2,?3)",rusqlite::params![key,predecessor,json]).unwrap();
        }
    }
    drop(conn);
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
    let source_hash = sha(&source);
    let manifest = CopyManifest {
        source: source.clone(),
        sha256: source_hash.clone(),
        provenance: "generated offline crash fixture".into(),
    };
    for cut in [
        "import_after_source_validation",
        "import_before_commit",
        "import_after_commit",
        "import_before_publish",
        "import_after_publish",
    ] {
        let destination = temp.path().join(format!("destination-{cut}"));
        let barrier = temp.path().join(format!("barrier-{cut}"));
        fs::create_dir(&barrier).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "import_child", "--nocapture"])
            .env("R2_IMPORT_SOURCE", &source)
            .env("R2_IMPORT_DEST", &destination)
            .env("R2_IMPORT_SHA", &source_hash)
            .env("REVIEW_EPISODE_FAULT_CUT", cut)
            .env("REVIEW_EPISODE_FAULT_DIR", &barrier)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let ready = barrier.join(format!("{cut}.ready"));
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            if let Some(status) = child.try_wait().unwrap() {
                panic!("import child exited before {cut}: {status}");
            }
            if Instant::now() > deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("import child did not reach {cut}");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        child.kill().unwrap();
        let _ = child.wait_with_output().unwrap();
        if cut == "import_after_publish" {
            assert!(destination.exists());
            let report = reconcile_closed_copy(&manifest, &destination).unwrap();
            assert_eq!((report.episodes, report.rows), (1, 3));
        } else {
            assert!(!destination.exists(), "{cut} published early");
            let report = import_closed_copy(&manifest, &destination).unwrap();
            assert_eq!((report.episodes, report.rows), (1, 3));
        }
        assert_eq!(sha(&source), source_hash);
    }
}
