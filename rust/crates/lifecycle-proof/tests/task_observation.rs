#![recursion_limit = "256"]

mod common;
use common::{TASK_ID, create_fixture, initial_submit, preflight_installed_host, ui_observe};
use lifecycle_proof::{
    host::HostClient,
    identity::HostProcess,
    provider::{Reply, ScriptedProvider, Step},
    receipt::ArtifactBinding,
};
use serde_json::json;
use std::{
    fs, thread,
    time::{Duration, Instant},
};

#[test]
fn installed_producer_to_production_ui_client() {
    let build = preflight_installed_host();
    let provider_root = tempfile::Builder::new()
        .prefix("to2-provider-")
        .tempdir_in(std::env::var_os("TMPDIR").unwrap())
        .unwrap();
    let provider = ScriptedProvider::start(
        vec![Step {
            required: vec!["input:command:s8:d0".into()],
            forbidden: vec![],
            reply: Reply::Text(
                "The approved snapshot contains ORION-7; no mutation authorized.".into(),
            ),
            release_path: None,
        }],
        provider_root.path().join("ledger.jsonl"),
    )
    .unwrap();
    let fixture = create_fixture(&provider.base_url(), build);
    let serialized_startup: serde_json::Value =
        serde_json::from_slice(&fs::read(&fixture.startup_path).unwrap()).unwrap();
    assert_eq!(serialized_startup, fixture.startup);
    let release = fixture
        .root
        .path()
        .join("barrier/native_before_turn_entry.release");
    fs::remove_file(&release).unwrap(); // Owned fixture latch, before any host exists.
    let host = HostProcess::launch(&fixture.host_executable, &fixture.startup_path).unwrap();
    let client = HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
    client.await_ready(Duration::from_secs(10)).unwrap();
    let absent = client.observe(TASK_ID, None).unwrap();
    let identity = absent["host_identity"].clone();
    let mut consumer = vec![ui_observe(
        &fixture,
        json!({"task_id":TASK_ID}),
        "absent",
        identity.clone(),
    )];
    assert_eq!(
        client
            .exchange(&json!({"op":"task.observe.v1","task_id":TASK_ID}))
            .unwrap(),
        json!({"status":"error","code":"unsupported_observation_version"})
    );
    let admitted = client.submit(initial_submit()).unwrap();
    assert_eq!(admitted["status"], "admitted", "{admitted}");
    let delivery = admitted["lifecycle"]["outcome_ref"].as_str().unwrap();
    let before = client.observe(TASK_ID, Some(delivery)).unwrap();
    consumer.push(ui_observe(
        &fixture,
        json!({"task_id":TASK_ID,"delivery_id":delivery}),
        "empty",
        identity.clone(),
    ));
    let after = client.observe(TASK_ID, Some(delivery)).unwrap();
    assert_eq!(before["components"]["owner"], after["components"]["owner"]);
    assert_eq!(
        before["components"]["native_delivery_status"],
        after["components"]["native_delivery_status"]
    );
    assert_eq!(provider.entry_count(), 0);
    fs::write(&release, b"release\n").unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let settled = loop {
        let value = client.observe(TASK_ID, Some(delivery)).unwrap();
        if value["components"]["lifecycle"]["value"]["delivery"]["value"]["outcome"] == "completed"
        {
            break value;
        }
        assert!(
            Instant::now() < deadline,
            "delivery did not settle: {value}"
        );
        thread::sleep(Duration::from_millis(20));
    };
    consumer.push(ui_observe(
        &fixture,
        json!({"task_id":TASK_ID,"delivery_id":delivery}),
        "segment",
        identity.clone(),
    ));
    let close = host.terminate_and_reap(Duration::from_secs(15)).unwrap();
    assert!(close.reaped);
    consumer.push(ui_observe(
        &fixture,
        json!({"task_id":TASK_ID}),
        "transport_unavailable",
        identity,
    ));
    for source in &fixture.sources {
        source.verify().unwrap();
    }
    let receipt = json!({"schema":"s8-to2-installed-ui-join-v1","build":fixture.build_provenance,"startup":ArtifactBinding::read(&fixture.startup_path).unwrap(),"source_manifest":fixture.source_manifest,"absent":absent,"admitted":admitted,"empty_before":before,"empty_after":after,"settled":settled,"consumer":consumer,"close":close,"provider_entries":provider.entry_count(),"provider_is_controlled_local_simulator":true});
    let destination =
        std::path::PathBuf::from(std::env::var_os("S8_EVIDENCE_DIR").expect("evidence root"));
    fs::create_dir_all(&destination).unwrap();
    let path = destination.join(format!("to2-join-{}.json", std::process::id()));
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    use std::io::Write;
    output
        .write_all(&serde_json::to_vec(&receipt).unwrap())
        .unwrap();
    output.sync_all().unwrap();
    let _ = fixture.root.keep();
}
