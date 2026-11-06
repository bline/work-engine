//! Offline actual-process facade tests. Synthetic journals do not qualify live execution.
use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command};
use work_engine_manual_host::{digest, pi_docker::Job};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> Fixture {
    let dir = std::env::temp_dir().join(format!(
        "coding-driver-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(dir.join("receipts")).unwrap();
    fs::write(dir.join("pi-lock"), b"").unwrap();
    let artifact = json!({"path":"/bin/true","sha256":digest(&fs::read("/bin/true").unwrap())});
    let tree = json!({"root":"/unused","manifest":artifact});
    let job: Job = serde_json::from_value(json!({"schema":"work-engine.pi-docker-custody.v1","job_id":"fixture","state_dir":dir,"authority":artifact,"packet":artifact,"docker":artifact,"image":"unused","runtime":tree,"workspace":tree,"deadline_unix_seconds":1,"command_timeout_seconds":1})).unwrap();
    let input = digest(&serde_json::to_vec(&job).unwrap());
    let journal = json!({"schema":job.schema,"binary_sha256":digest(&fs::read(env!("CARGO_BIN_EXE_coding-qualification")).unwrap()),"job":job,"input_digest":input,"container_name":format!("we-pi-{}", &input[..24]),"container_id":null,"session_id":null,"phase":"collected","processes":[],"failures":["synthetic fixture incomplete"],"rpc_verified":false,"daemon_exit":null,"removed":true,"artifacts":[],"exported_files":{},"receipt_id":null,"acknowledged":[]});
    fs::write(
        dir.join("pi-state.json"),
        serde_json::to_vec(&journal).unwrap(),
    )
    .unwrap();
    Fixture(dir)
}
fn invoke(action: &str, dir: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_coding-qualification"))
        .args(["pi-docker", action, "--state"])
        .arg(dir)
        .output()
        .unwrap()
}
#[test]
fn same_producer_inspect_reconcile_repeat_and_cancel_keep_original_identity() {
    let f = fixture();
    let original = fs::read(f.0.join("pi-state.json")).unwrap();
    let out = invoke("inspect", &f.0);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(fs::read(f.0.join("pi-state.json")).unwrap(), original);
    assert!(!f.0.join("lifecycle").exists());
    let inspected: Value = serde_json::from_slice(&out.stdout).unwrap();
    let out = invoke("reconcile", &f.0);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wrapper: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        wrapper["receipt"]["input_digest"],
        inspected["input_digest"]
    );
    assert_eq!(wrapper["receipt"]["status"], "not_established");
    assert_eq!(wrapper["receipt"]["acceptance"], "owner_pending");
    let id = wrapper["receipt_id"].as_str().unwrap();
    assert_eq!(
        digest(&fs::read(f.0.join("receipts").join(format!("{id}.json"))).unwrap()),
        id
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&invoke("reconcile", &f.0).stdout).unwrap(),
        wrapper
    );
    assert!(invoke("cancel", &f.0).status.success());
    assert!(f.0.join("cancel.json").exists());
    assert!(!f.0.join("lifecycle").exists());
    let journal: Value =
        serde_json::from_slice(&fs::read(f.0.join("pi-state.json")).unwrap()).unwrap();
    assert_eq!(journal["acknowledged"], json!([]));
    // Same journal cannot be reopened after the producing binary identity drifts.
    let mut wrong = journal;
    wrong["binary_sha256"] = json!("0".repeat(64));
    fs::write(f.0.join("pi-state.json"), wrong.to_string()).unwrap();
    assert!(!invoke("inspect", &f.0).status.success());
    assert!(!invoke("reconcile", &f.0).status.success());
    assert!(!invoke("cancel", &f.0).status.success());
}
#[test]
fn producer_refuses_unbounded_and_missing_job_inputs_without_state() {
    let f = fixture();
    for args in [
        vec!["pi-docker", "run", "--request"],
        vec!["pi-docker", "ack", "--state", "/unused"],
        vec!["--execute", "/missing/pinned-job.json"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_coding-qualification"))
            .args(args)
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
    }
    assert!(!f.0.join("lifecycle").exists());
}
