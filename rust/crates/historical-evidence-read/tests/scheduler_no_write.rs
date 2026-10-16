//! A subprocess is required because Landlock confinement is irreversible for a process.
#[path = "../src/source_read/mod.rs"]
#[allow(dead_code)]
mod source_read;

use rusqlite::{Connection, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture(directory: &Path, wal: bool) -> (Connection, Value) {
    let path = directory.join("scheduler.sqlite3");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TABLE metadata(key TEXT PRIMARY KEY,value INTEGER);
        CREATE TABLE scheduled_items(id TEXT PRIMARY KEY,repository_id TEXT,logical_role TEXT,
        logical_agent_id TEXT,intent TEXT,due_at TEXT,on_due TEXT,approval_policy TEXT,
        catch_up_policy TEXT,result_policy TEXT,payload_json TEXT,status TEXT,
        created_at TEXT,updated_at TEXT,presented_at TEXT,result_ref TEXT);",
    )
    .unwrap();
    if wal {
        db.pragma_update(None, "journal_mode", "WAL").unwrap();
    }
    let intent = "candidate-4 session-test episode-test";
    db.execute("INSERT INTO metadata VALUES ('revision',15)", [])
        .unwrap();
    db.execute("INSERT INTO scheduled_items VALUES (?1,?2,?3,NULL,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,NULL,?14)",
        params!["test-item", "/test/repository", "supervisor", intent,
        "2026-08-29T12:00:00Z", "present", "none", "latest", "retain", "{}",
        "acknowledged", "2026-08-29T01:00:00Z", "2026-08-29T02:00:00Z",
        "episode-test@revision-test"]).unwrap();
    let row = json!({"id":"test-item","repository_id":"/test/repository",
        "logical_role":"supervisor","logical_agent_id":null,"intent":intent,
        "due_at":"2026-08-29T12:00:00Z","on_due":"present",
        "approval_policy":"none","catch_up_policy":"latest","result_policy":"retain",
        "payload":{},"status":"acknowledged","created_at":"2026-08-29T01:00:00Z",
        "updated_at":"2026-08-29T02:00:00Z","presented_at":null,
        "result_ref":"episode-test@revision-test"});
    let grant = json!({"scheduler":{"db_path":path,"store_revision":15,"item_id":"test-item",
        "row_sha256":source_read::canonical_row_sha(&row).unwrap(),
        "repository_id":"/test/repository","logical_role":"supervisor",
        "due_at":"2026-08-29T12:00:00Z","result_ref":"episode-test@revision-test",
        "intent_sha256":source_read::sha(intent.as_bytes()),
        "session_id":"session-test","episode_ref":"episode-test"}});
    (db, grant)
}

fn snapshot(directory: &Path) -> BTreeMap<String, String> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = fs::read(&path).unwrap();
            (
                path.file_name().unwrap().to_string_lossy().to_string(),
                format!("{:x}", Sha256::digest(bytes)),
            )
        })
        .collect()
}

fn child(directory: &Path, grant: &Value, expect_success: bool) {
    let grant_path = directory.join("grant.json");
    fs::write(&grant_path, serde_json::to_vec(grant).unwrap()).unwrap();
    let before = snapshot(directory);
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_scheduler_inspection", "--nocapture"])
        .env("S5_SCHEDULER_TEST_GRANT", &grant_path)
        .env(
            "S5_SCHEDULER_TEST_SUCCESS",
            if expect_success { "1" } else { "0" },
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(
        before,
        snapshot(directory),
        "scheduler read changed source files"
    );
}

#[test]
fn child_scheduler_inspection() {
    let Ok(path) = std::env::var("S5_SCHEDULER_TEST_GRANT") else {
        return;
    };
    let grant: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let result = source_read::scheduler_item::inspect(&grant);
    if let Ok(ref row) = result {
        assert_eq!(row["store_revision"], 15);
        assert_eq!(row["row_sha256"], grant["scheduler"]["row_sha256"]);
        println!("SOURCE_STATUS:observed");
    } else {
        println!("SOURCE_STATUS:unavailable");
    }
    if std::env::var("S5_SCHEDULER_TEST_SUCCESS").unwrap() == "1" {
        assert!(result.is_ok(), "{result:?}");
    }
}

#[test]
fn concurrent_revision_change_is_exact_snapshot_or_unavailable() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("s5-scheduler-race-{}-{unique}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let (db, grant) = fixture(&root, true);
    let grant_path = root.join("grant.json");
    fs::write(&grant_path, serde_json::to_vec(&grant).unwrap()).unwrap();
    let mut reader = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_scheduler_inspection", "--nocapture"])
        .env("S5_SCHEDULER_TEST_GRANT", &grant_path)
        .env("S5_SCHEDULER_TEST_SUCCESS", "0")
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(1));
    db.execute("UPDATE metadata SET value=16 WHERE key='revision'", [])
        .unwrap();
    assert!(reader.wait().unwrap().success());
    drop(db);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn scheduler_read_cannot_create_or_modify_sidecars() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "s5-scheduler-no-write-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    for (name, wal, keep_open, expected_success) in [
        ("rollback", false, false, true),
        ("wal-absent", true, false, false),
        ("wal-existing", true, true, false),
    ] {
        let directory = root.join(name);
        fs::create_dir(&directory).unwrap();
        let (db, grant) = fixture(&directory, wal);
        if keep_open {
            child(&directory, &grant, expected_success);
        } else {
            drop(db);
            child(&directory, &grant, expected_success);
            continue;
        }
        drop(db);
    }
    fs::remove_dir_all(root).unwrap();
}
