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

#[test]
fn immutable_base_v1_opener_refuses_packaged_service_v2_image_before_mutation() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
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
    let snapshot_root = root.join("coherent-v2");
    fs::create_dir(&snapshot_root).unwrap();
    store
        .snapshot_to(&snapshot_root.join("lifecycle.sqlite"))
        .unwrap();
    drop(store);

    let base = root.join("base-source");
    fs::create_dir(&base).unwrap();
    let mut archive = Command::new("git")
        .arg("archive")
        .arg("b2efdbfa0354a997b23ac4e183b89ca0dbf36cc2")
        .arg("--")
        .arg("rust")
        .current_dir(workspace())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let status = Command::new("tar")
        .args(["-x", "-C"])
        .arg(&base)
        .stdin(Stdio::from(archive.stdout.take().unwrap()))
        .status()
        .unwrap();
    assert!(status.success());
    assert!(archive.wait().unwrap().success());
    let examples = base.join("rust/crates/lifecycle-store/examples");
    fs::create_dir(&examples).unwrap();
    fs::copy(
        workspace().join("rust/crates/lifecycle-test-support/fixtures/v1_store_opener.rs"),
        examples.join("v1_store_opener.rs"),
    )
    .unwrap();
    let status = Command::new("cargo")
        .arg("+1.92.0")
        .args(["run", "--manifest-path"])
        .arg(base.join("rust/Cargo.toml"))
        .args([
            "-p",
            "lifecycle-store",
            "--example",
            "v1_store_opener",
            "--locked",
            "--target-dir",
        ])
        .arg(root.join("base-target"))
        .arg("--")
        .arg(&snapshot_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .status()
        .unwrap();
    assert!(
        status.success(),
        "immutable base v1 opener failed to prove refusal"
    );
}
