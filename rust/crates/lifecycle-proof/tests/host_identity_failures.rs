#![recursion_limit = "256"]

mod common;

use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use lifecycle_proof::{identity::sha256_file, receipt::ArtifactBinding};
use serde::Serialize;
use serde_json::{Value, json};

use common::{create_fixture, preflight_installed_host};

#[derive(Serialize)]
struct FailedStartup {
    case: String,
    startup: ArtifactBinding,
    exit_code: Option<i32>,
    stderr: String,
}

#[test]
fn installed_host_refuses_tampered_identity_before_entry() {
    let build = preflight_installed_host();
    let fixture = create_fixture("http://127.0.0.1:9/v1", build.clone());
    assert!(fixture.startup_path.is_file());
    assert_eq!(
        fixture.build_provenance.executable_sha256,
        build.executable_sha256
    );
    let mut cases: Vec<(String, Value)> = Vec::new();
    for (name, field, value) in [
        (
            "executable-digest",
            "executable_sha256",
            json!("0".repeat(64)),
        ),
        ("source-digest", "source_sha256", json!("0".repeat(64))),
        ("lock-digest", "lock_sha256", json!("0".repeat(64))),
        (
            "toolchain-digest",
            "toolchain_sha256",
            json!("0".repeat(64)),
        ),
        ("compiled-feature", "features", json!(["native-sim-proof"])),
        (
            "provider-identity",
            "provider_identity",
            json!("unexpected-provider"),
        ),
    ] {
        let mut startup = fixture.startup.clone();
        startup[field] = value;
        cases.push((name.into(), startup));
    }
    let mut manifest_entries: Vec<Value> =
        serde_json::from_slice(&fs::read(&fixture.source_manifest.path).unwrap()).unwrap();
    manifest_entries[0]["sha256"] = json!("0".repeat(64));
    let alternate_manifest = fixture.root.path().join("source-manifest-tampered.json");
    fs::write(
        &alternate_manifest,
        serde_json::to_vec(&manifest_entries).unwrap(),
    )
    .unwrap();
    let mut startup = fixture.startup.clone();
    startup["source_manifest_path"] = json!(alternate_manifest);
    startup["source_sha256"] = json!(sha256_file(&alternate_manifest).unwrap());
    cases.push(("source-entry".into(), startup));
    let mut startup = fixture.startup.clone();
    startup["lifecycle"]["native"]["same_native_session_required"] = json!(true);
    cases.push(("same-session-required".into(), startup));

    let mut outcomes = Vec::new();
    for (name, startup) in cases {
        let path = fixture.root.path().join(format!("invalid-{name}.json"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        file.write_all(&serde_json::to_vec(&startup).unwrap())
            .unwrap();
        file.sync_all().unwrap();
        let mut child = Command::new(&fixture.host_executable)
            .args(["serve", "--startup"])
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("invalid startup did not terminate: {name}");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success(), "invalid startup accepted: {name}");
        assert!(
            output.stdout.is_empty(),
            "invalid startup wrote stdout: {name}"
        );
        assert!(
            !fixture.host_socket.exists(),
            "invalid startup opened host socket: {name}"
        );
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        assert!(stderr.len() < 4096, "unbounded startup diagnostic: {name}");
        outcomes.push(FailedStartup {
            case: name,
            startup: ArtifactBinding::read(&path).unwrap(),
            exit_code: output.status.code(),
            stderr,
        });
    }
    assert_eq!(outcomes.len(), 8);
    for source in &fixture.sources {
        source.verify().unwrap();
    }
    assert_eq!(
        sha256_file(&fixture.host_executable).unwrap(),
        build.executable_sha256
    );
    if let Some(destination) = std::env::var_os("S8_EVIDENCE_DIR") {
        let destination = std::path::PathBuf::from(destination);
        assert!(destination.is_absolute());
        fs::create_dir_all(&destination).unwrap();
        let evidence = json!({
            "schema":"s8-installed-host-identity-failures-v1",
            "build_provenance":build,
            "host_executable":ArtifactBinding::read(&fixture.host_executable).unwrap(),
            "source_manifest":fixture.source_manifest,
            "source_entries":fixture.sources,
            "faults":outcomes,
            "provider_entries":0,
            "host_socket_absent":true
        });
        let path = destination.join(format!(
            "s8-host-identity-failures-{}.json",
            std::process::id()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        file.write_all(&serde_json::to_vec(&evidence).unwrap())
            .unwrap();
        file.sync_all().unwrap();
        eprintln!(
            "S8 identity fault receipt {} sha256 {}",
            path.display(),
            sha256_file(&path).unwrap()
        );
        let _ = fixture.root.keep();
    }
}
