#![recursion_limit = "256"]

mod common;

use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    time::Duration,
};

use lifecycle_proof::{
    host::{HostClient, HostReportedIdentity},
    identity::{HostProcess, sha256_file},
    receipt::ArtifactBinding,
};
use serde_json::json;

use common::{BUILD_ID, TASK_ID, create_fixture, preflight_installed_host};

#[test]
fn same_build_restart_has_new_execution_identity_and_owned_close() {
    let build = preflight_installed_host();
    let fixture = create_fixture("http://127.0.0.1:9/v1", build.clone());
    assert_eq!(
        fixture.build_provenance.executable_sha256,
        build.executable_sha256
    );
    let client = HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
    let mut first = HostProcess::launch(&fixture.host_executable, &fixture.startup_path).unwrap();
    client.await_ready(Duration::from_secs(10)).unwrap();
    let first_reply = client.observe(TASK_ID, None).unwrap();
    assert_eq!(first_reply["status"], "observation");
    common::ui_observe(
        &fixture,
        json!({"task_id":TASK_ID}),
        "absent",
        first_reply["host_identity"].clone(),
    );
    let first_reported = HostReportedIdentity::from_reply(&first_reply).unwrap();
    first_reported
        .bind_process(first.identity(), BUILD_ID)
        .unwrap();
    first_reported.bind_startup(&fixture.startup).unwrap();
    first.assert_same_execution().unwrap();
    let first_process = first.identity().clone();
    let first_close = first.terminate_and_reap(Duration::from_secs(15)).unwrap();
    assert!(first_close.reaped);

    let mut second = HostProcess::launch(&fixture.host_executable, &fixture.startup_path).unwrap();
    client.await_ready(Duration::from_secs(10)).unwrap();
    let second_reply = client.observe(TASK_ID, None).unwrap();
    assert_eq!(second_reply["status"], "observation");
    common::ui_observe(
        &fixture,
        json!({"task_id":TASK_ID}),
        "absent",
        second_reply["host_identity"].clone(),
    );
    let second_reported = HostReportedIdentity::from_reply(&second_reply).unwrap();
    second_reported
        .bind_process(second.identity(), BUILD_ID)
        .unwrap();
    second_reported.bind_startup(&fixture.startup).unwrap();
    second.assert_same_execution().unwrap();
    assert_ne!(first_reported.incarnation, second_reported.incarnation);
    assert_ne!(first_process, *second.identity());
    assert!(
        first_reported
            .bind_process(second.identity(), BUILD_ID)
            .is_err()
    );
    assert_eq!(
        first_reported.executable_sha256,
        second_reported.executable_sha256
    );
    let second_process = second.identity().clone();
    let second_close = second.terminate_and_reap(Duration::from_secs(15)).unwrap();
    assert!(second_close.reaped);
    assert_eq!(
        sha256_file(&fixture.host_executable).unwrap(),
        build.executable_sha256
    );
    for source in &fixture.sources {
        source.verify().unwrap();
    }

    if let Some(destination) = std::env::var_os("S8_EVIDENCE_DIR") {
        let destination = std::path::PathBuf::from(destination);
        assert!(destination.is_absolute());
        fs::create_dir_all(&destination).unwrap();
        let evidence = json!({
            "schema":"s8-installed-host-restart-identity-v1",
            "build_provenance":build,
            "host_executable":ArtifactBinding::read(&fixture.host_executable).unwrap(),
            "source_manifest":fixture.source_manifest,
            "source_entries":fixture.sources,
            "startup":ArtifactBinding::read(&fixture.startup_path).unwrap(),
            "first":{"process":first_process,"reported":first_reported,"observation":first_reply,"close":first_close},
            "second":{"process":second_process,"reported":second_reported,"observation":second_reply,"close":second_close},
            "provider_entries":0
        });
        let path = destination.join(format!(
            "s8-host-restart-identity-{}.json",
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
            "S8 host restart identity receipt {} sha256 {}",
            path.display(),
            sha256_file(&path).unwrap()
        );
        let _ = fixture.root.keep();
    }
}
