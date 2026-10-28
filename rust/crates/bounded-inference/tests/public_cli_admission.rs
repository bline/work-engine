use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn fixture_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "b2-cli-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn invalid_command_and_action_grant_stop_before_store_creation() {
    let bin = env!("CARGO_BIN_EXE_b2-public-host");
    let root = fixture_dir("admission");
    fs::create_dir(&root).unwrap();
    let credentials = root.join("credentials");
    let bad = Command::new(bin)
        .arg("unknown")
        .arg(&credentials)
        .arg("host")
        .output()
        .unwrap();
    assert!(!bad.status.success());
    assert!(!credentials.exists());

    let identity = Command::new(bin).arg("identity").output().unwrap();
    assert!(identity.status.success());
    let identity: serde_json::Value = serde_json::from_slice(&identity.stdout).unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let grant_path = root.join("action.json");
    let grant = json!({
        "release_id":"synthetic-release", "actor":"synthetic-owner",
        "action":"models", "credential_dir":credentials, "host_id":"host",
        "work_dir":null, "smoke_grant_sha256":null,
        "source_sha256":identity["source_sha256"],
        "executable_sha256":identity["executable_sha256"],
        "issued_at_unix":now, "valid_until_unix":now+15
    });
    fs::write(&grant_path, serde_json::to_vec(&grant).unwrap()).unwrap();
    let wrong = Command::new(bin)
        .arg("disconnect")
        .arg(&credentials)
        .arg("host")
        .arg(&grant_path)
        .output()
        .unwrap();
    assert!(!wrong.status.success());
    assert!(!credentials.exists());

    let mut altered = grant;
    altered["executable_sha256"] = json!("0".repeat(64));
    fs::write(&grant_path, serde_json::to_vec(&altered).unwrap()).unwrap();
    let forged = Command::new(bin)
        .arg("models")
        .arg(&credentials)
        .arg("host")
        .arg(&grant_path)
        .output()
        .unwrap();
    assert!(!forged.status.success());
    assert!(!credentials.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn account_reads_local_store_without_discovery() {
    let bin = env!("CARGO_BIN_EXE_b2-public-host");
    let root = fixture_dir("account-local");
    fs::create_dir(&root).unwrap();
    let credentials = root.join("credentials");
    let outcome = Command::new(bin)
        .arg("account")
        .arg(&credentials)
        .arg("synthetic_host")
        .output()
        .unwrap();
    assert!(!outcome.status.success());
    assert!(credentials.exists());
    assert!(String::from_utf8_lossy(&outcome.stderr).contains("no ready selected account"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn status_inspects_existing_store_without_creation_cleanup_or_identity_output() {
    use bounded_inference::{AuthStore, Issuer};
    use rustix::fs::{flock, FlockOperation};
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let bin = env!("CARGO_BIN_EXE_b2-public-host");
    let root = fixture_dir("status-read-only");
    fs::create_dir(&root).unwrap();
    let credentials = root.join("credentials");
    let inspect = || {
        Command::new(bin)
            .arg("status")
            .arg(&credentials)
            .arg("synthetic_host")
            .output()
            .unwrap()
    };
    let absent = inspect();
    assert!(!absent.status.success());
    assert!(String::from_utf8_lossy(&absent.stderr).contains("local credential state is absent"));
    assert!(!credentials.exists());

    let issuer = Issuer::new(
        "https://auth.openai.com",
        "https://auth.openai.com/api/accounts/authorize",
        "https://auth.openai.com/api/accounts/oauth/token",
        "https://auth.openai.com/api/accounts/oauth/revoke",
    )
    .unwrap();
    drop(AuthStore::open(&credentials, "synthetic_host", issuer).unwrap());
    let orphan = credentials.join(format!(".state-{}", "A".repeat(43)));
    fs::write(&orphan, b"private-orphan-marker").unwrap();
    let snapshot = || {
        let mut files: Vec<_> = fs::read_dir(&credentials)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (
                    entry.file_name().to_string_lossy().to_string(),
                    fs::read(entry.path()).unwrap(),
                )
            })
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        files
    };
    let before = snapshot();
    let signed_out = inspect();
    assert!(
        signed_out.status.success(),
        "{}",
        String::from_utf8_lossy(&signed_out.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&signed_out.stdout).unwrap(),
        json!({"readiness":"signed_out"})
    );
    assert!(signed_out.stderr.is_empty());
    assert_eq!(snapshot(), before);
    assert_eq!(fs::read(&orphan).unwrap(), b"private-orphan-marker");

    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .mode(0o600)
        .open(credentials.join(".lock"))
        .unwrap();
    flock(&lock, FlockOperation::NonBlockingLockExclusive).unwrap();
    let unavailable = inspect();
    assert!(!unavailable.status.success());
    assert!(String::from_utf8_lossy(&unavailable.stderr).contains("store lock is unavailable"));
    assert!(unavailable.stdout.is_empty());
    assert_eq!(snapshot(), before);
    drop(lock);

    fs::remove_file(credentials.join("state.json")).unwrap();
    let without_state = snapshot();
    let missing_state = inspect();
    assert!(!missing_state.status.success());
    assert!(
        String::from_utf8_lossy(&missing_state.stderr).contains("local credential state is absent")
    );
    assert_eq!(snapshot(), without_state);

    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(credentials.join("state.json"))
        .unwrap()
        .write_all(b"private-corrupt-marker")
        .unwrap();
    let corrupted = snapshot();
    let failure = inspect();
    assert!(!failure.status.success());
    let stderr = String::from_utf8_lossy(&failure.stderr);
    assert!(stderr.contains("local credential state is corrupt"));
    assert!(!stderr.contains("private-corrupt-marker"));
    assert!(failure.stdout.is_empty());
    assert_eq!(snapshot(), corrupted);
    fs::remove_file(credentials.join(".lock")).unwrap();
    let without_lock = snapshot();
    let missing_lock = inspect();
    assert!(!missing_lock.status.success());
    assert!(String::from_utf8_lossy(&missing_lock.stderr).contains("store lock is unavailable"));
    assert_eq!(snapshot(), without_lock);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn refresh_rejects_ineligible_targets_without_initialization_or_cleanup() {
    use bounded_inference::{AuthStore, Issuer};
    let bin = env!("CARGO_BIN_EXE_b2-public-host");
    let root = fixture_dir("refresh-preopen");
    fs::create_dir(&root).unwrap();
    let credentials = root.join("credentials");
    let identity = Command::new(bin).arg("identity").output().unwrap();
    assert!(identity.status.success());
    let identity: serde_json::Value = serde_json::from_slice(&identity.stdout).unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let grant_path = root.join("grant.json");
    let target = json!({"client_id":"synthetic-client","subject":"synthetic-subject","credential_revision":1});
    let grant = json!({
        "release_id":"synthetic-refresh-preopen", "actor":"synthetic-owner",
        "custody_owner":"synthetic-owner", "custody_custodian":"synthetic-custodian",
        "custody_contract_ref":"synthetic-contract", "retention_purpose":"synthetic-test",
        "cleanup_disposition":"retain-until-owner-disconnect",
        "action":"refresh", "credential_dir":credentials, "host_id":"synthetic_host",
        "work_dir":null, "smoke_grant_sha256":null, "refresh_registration":target,
        "source_sha256":identity["source_sha256"], "executable_sha256":identity["executable_sha256"],
        "issued_at_unix":now, "valid_until_unix":now+20
    });
    let invoke = || {
        let mut fresh = grant.clone();
        let issued = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        fresh["issued_at_unix"] = json!(issued);
        fresh["valid_until_unix"] = json!(issued + 20);
        fs::write(&grant_path, serde_json::to_vec(&fresh).unwrap()).unwrap();
        Command::new(bin)
            .arg("refresh")
            .arg(&credentials)
            .arg("synthetic_host")
            .arg(&grant_path)
            .output()
            .unwrap()
    };
    assert!(!invoke().status.success());
    assert!(!credentials.exists());

    let issuer = Issuer::new(
        "https://auth.openai.com",
        "https://auth.openai.com/api/accounts/authorize",
        "https://auth.openai.com/api/accounts/oauth/token",
        "https://auth.openai.com/api/accounts/oauth/revoke",
    )
    .unwrap();
    drop(AuthStore::open(&credentials, "synthetic_host", issuer).unwrap());
    let custody = json!({"owner":"synthetic-owner", "custodian":"synthetic-custodian",
        "contract_ref":"synthetic-contract", "retention_purpose":"synthetic-test"});
    fs::write(
        credentials.join("credential-custody-contract.json"),
        serde_json::to_vec(&custody).unwrap(),
    )
    .unwrap();
    let orphan = credentials.join(format!(".state-{}", "A".repeat(43)));
    fs::write(&orphan, b"synthetic-orphan").unwrap();
    let snapshot = || {
        let mut files: Vec<_> = fs::read_dir(&credentials)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (
                    entry.file_name().to_string_lossy().to_string(),
                    fs::read(entry.path()).unwrap(),
                )
            })
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        files
    };
    let signed_out = snapshot();
    assert!(!invoke().status.success());
    assert_eq!(snapshot(), signed_out);
    assert_eq!(fs::read(&orphan).unwrap(), b"synthetic-orphan");

    let state_path = credentials.join("state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["selected_client_id"] = json!("synthetic-client");
    state["registrations"] = json!([{"client_id":"synthetic-client", "subject":"synthetic-subject",
        "email":null, "revision":2, "refresh_state":"Idle",
        "credential":{"access":"synthetic-access","refresh":"synthetic-refresh","id":"synthetic-id",
        "scopes":[],"expires_at":now+3600,"earliest_refresh_at":null}}]);
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    let stale = snapshot();
    assert!(!invoke().status.success());
    assert_eq!(snapshot(), stale);
    assert!(!credentials
        .join("action-synthetic-refresh-preopen.json")
        .exists());
    for refresh in [None, Some("")] {
        let mut ineligible = state.clone();
        match refresh {
            None => ineligible["registrations"][0]["credential"] = serde_json::Value::Null,
            Some(value) => ineligible["registrations"][0]["credential"]["refresh"] = json!(value),
        }
        fs::write(&state_path, serde_json::to_vec(&ineligible).unwrap()).unwrap();
        let before = snapshot();
        let inspected = Command::new(bin)
            .arg("renewal-target")
            .arg(&credentials)
            .arg("synthetic_host")
            .output()
            .unwrap();
        assert!(!inspected.status.success());
        assert!(inspected.stdout.is_empty());
        assert!(!invoke().status.success());
        assert_eq!(snapshot(), before);
    }

    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    let mut spent_grant = grant.clone();
    spent_grant["refresh_registration"]["credential_revision"] = json!(2);
    let spent_marker = credentials.join("action-synthetic-refresh-preopen.json");
    fs::write(&spent_marker, b"synthetic-spent-action").unwrap();
    let before_spent = snapshot();
    let issued = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    spent_grant["issued_at_unix"] = json!(issued);
    spent_grant["valid_until_unix"] = json!(issued + 20);
    fs::write(&grant_path, serde_json::to_vec(&spent_grant).unwrap()).unwrap();
    let spent = Command::new(bin)
        .arg("refresh")
        .arg(&credentials)
        .arg("synthetic_host")
        .arg(&grant_path)
        .output()
        .unwrap();
    assert!(!spent.status.success());
    assert!(spent.stdout.is_empty());
    assert_eq!(snapshot(), before_spent);
    assert_eq!(fs::read(&orphan).unwrap(), b"synthetic-orphan");
    fs::remove_dir_all(root).unwrap();
}
