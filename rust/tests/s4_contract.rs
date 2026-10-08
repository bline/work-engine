use std::{
    fs,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use lifecycle_store::SqliteLifecycleStore;
use lifecycle_wire::{parse_command_v2, schema_bundle_v2};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use tempfile::tempdir;
use work_engine_types::CodecContract;

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn public_v1_identity_and_v2_schema_bundle_are_exact() {
    let root = workspace().join("rust/crates/lifecycle-wire");
    let v1 = fs::read(root.join("schemas/v1/manifest.json")).unwrap();
    assert_eq!(
        CodecContract::BinaryArtifactV1
            .digest_binary(&v1)
            .unwrap()
            .hex(),
        "f4c73a45602284989fc352f5529f421a5e2c73cdaece2c4678035825032727be"
    );
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("schemas/v2/manifest.json")).unwrap()).unwrap();
    assert_eq!(
        manifest["v1_compatibility_manifest_sha256"],
        "f4c73a45602284989fc352f5529f421a5e2c73cdaece2c4678035825032727be"
    );
    for (name, schema) in schema_bundle_v2() {
        let mut bytes = serde_json::to_vec_pretty(&schema).unwrap();
        bytes.push(b'\n');
        assert_eq!(fs::read(root.join("schemas/v2").join(name)).unwrap(), bytes);
    }
    for (group, prefix) in [
        ("schema_sha256", "schemas/v2"),
        ("fixture_sha256", "tests/fixtures"),
        ("source_sha256", ""),
    ] {
        for (name, expected) in manifest[group].as_object().unwrap() {
            let bytes = fs::read(root.join(prefix).join(name)).unwrap();
            assert_eq!(
                CodecContract::BinaryArtifactV1
                    .digest_binary(&bytes)
                    .unwrap()
                    .hex(),
                expected.as_str().unwrap(),
                "v2 manifest drift: {group}/{name}"
            );
        }
    }
    assert!(
        parse_command_v2(&fs::read(root.join("tests/fixtures/command-v2.json")).unwrap()).is_ok()
    );
}

fn wait_connect(socket: &Path) {
    let started = Instant::now();
    while UnixStream::connect(socket).is_err() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "service unavailable"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn archive_rust_source(commit: &str, destination: &Path) {
    fs::create_dir(destination).unwrap();
    let mut archive = Command::new("git")
        .arg("archive")
        .arg(commit)
        .arg("--")
        .arg("rust")
        .current_dir(workspace())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let status = Command::new("tar")
        .args(["-x", "-C"])
        .arg(destination)
        .stdin(Stdio::from(archive.stdout.take().unwrap()))
        .status()
        .unwrap();
    assert!(status.success());
    assert!(archive.wait().unwrap().success());
}

fn schema_version(root: &Path) -> i64 {
    let database = root.join("lifecycle.sqlite");
    let connection =
        Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap()
}

fn run_store_example(source: &Path, target: &Path, name: &str, args: &[&Path]) {
    let mut command = Command::new("cargo");
    command
        .arg("+1.92.0")
        .args(["run", "--manifest-path"])
        .arg(source.join("rust/Cargo.toml"))
        .args([
            "-p",
            "lifecycle-store",
            "--example",
            name,
            "--locked",
            "--target-dir",
        ])
        .arg(target)
        .arg("--");
    for arg in args {
        command.arg(arg);
    }
    let status = command
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .status()
        .unwrap();
    assert!(status.success(), "historical {name} failed");
}

#[test]
fn immutable_base_v1_refuses_genuine_s4_v2_image_and_s4_refuses_current_v3() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();

    // This archive is the published S4 source, whose store opener creates schema v2.
    let s4 = root.join("published-s4-source");
    archive_rust_source("41992a22d86b041d2db357d16345c477804af63a", &s4);
    let s4_examples = s4.join("rust/crates/lifecycle-store/examples");
    fs::create_dir(&s4_examples).unwrap();
    fs::write(
        s4_examples.join("s4_store_contract.rs"),
        r#"use std::{env, fs, path::PathBuf, process};
use lifecycle_store::SqliteLifecycleStore;
use rusqlite::{Connection, OpenFlags};

fn version(root: &PathBuf) -> i64 {
    let connection = Connection::open_with_flags(root.join("lifecycle.sqlite"), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    connection.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap()
}

fn main() {
    let mode = env::args().nth(1).unwrap();
    let root = PathBuf::from(env::args_os().nth(2).unwrap());
    match mode.as_str() {
        "create-v2" => {
            fs::create_dir_all(&root).unwrap();
            let store = SqliteLifecycleStore::open(&root.join("seed"), "issuer-contract".into()).unwrap();
            store.snapshot_to(&root.join("lifecycle.sqlite")).unwrap();
            assert_eq!(version(&root), 2);
        }
        "refuse-v3" => {
            assert_eq!(version(&root), 3);
            let database = root.join("lifecycle.sqlite");
            let before = fs::read(&database).unwrap();
            if SqliteLifecycleStore::open(&root, "issuer-contract".into()).is_ok() {
                eprintln!("published S4 opener accepted schema v3"); process::exit(2);
            }
            assert_eq!(fs::read(&database).unwrap(), before, "published S4 opener mutated schema v3");
        }
        _ => panic!("unknown mode"),
    }
}
"#,
    )
    .unwrap();
    let s4_target = root.join("s4-target");
    let genuine_v2 = root.join("genuine-s4-v2");
    let create_v2 = Path::new("create-v2");
    run_store_example(
        &s4,
        &s4_target,
        "s4_store_contract",
        &[create_v2, &genuine_v2],
    );
    assert_eq!(schema_version(&genuine_v2), 2);

    let base = root.join("base-source");
    archive_rust_source("b2efdbfa0354a997b23ac4e183b89ca0dbf36cc2", &base);
    let examples = base.join("rust/crates/lifecycle-store/examples");
    fs::create_dir(&examples).unwrap();
    fs::copy(
        workspace().join("rust/crates/lifecycle-test-support/fixtures/v1_store_opener.rs"),
        examples.join("v1_store_opener.rs"),
    )
    .unwrap();
    run_store_example(
        &base,
        &root.join("base-target"),
        "v1_store_opener",
        &[&genuine_v2],
    );

    // A v2 image opens under the current store and migrates to the current schema.
    let upgraded = root.join("upgraded-v2");
    fs::create_dir(&upgraded).unwrap();
    fs::copy(
        genuine_v2.join("lifecycle.sqlite"),
        upgraded.join("lifecycle.sqlite"),
    )
    .unwrap();
    let store = SqliteLifecycleStore::open(&upgraded, "issuer-contract".into()).unwrap();
    assert_eq!(store.load_recovery().unwrap().subjects, 0);
    drop(store);
    assert_eq!(schema_version(&upgraded), 3);

    let db = root.join("live-store");
    let socket = root.join("read.sock");
    let config = json!({"profile":"read_only","store_root":db,"socket_path":socket,
        "peer_socket_path":root.join("absent-peer"),"trusted_issuer":"issuer-contract",
        "principal":{"uid":fs::metadata(root).unwrap().uid(),"principal_ref":"principal-contract","producer_ref":"producer-contract"},
        "subject_id":"subject-contract","context_generation":"context-contract","build_id":"build-contract","proof_run_id":"proof-contract",
        "grants":[],"max_executions":1,"max_wait_ms":1000});
    let config_path = root.join("config.json");
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let mut service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    service.0.kill().unwrap();
    service.0.wait().unwrap();
    let store = SqliteLifecycleStore::open(&db, "issuer-contract".into()).unwrap();
    let current_v3 = root.join("current-service-v3");
    fs::create_dir(&current_v3).unwrap();
    store
        .snapshot_to(&current_v3.join("lifecycle.sqlite"))
        .unwrap();
    drop(store);
    assert_eq!(schema_version(&current_v3), 3);
    let refuse_v3 = Path::new("refuse-v3");
    run_store_example(
        &s4,
        &s4_target,
        "s4_store_contract",
        &[refuse_v3, &current_v3],
    );
}
