//! The installed Pi Host composition boundary. Preparation is local and
//! credential-free; serving the verified job remains the runner's decision.

use super::pi_host::{PiHostConfig, sha};
use crate::{NativeConfig, PiHostComposition};
use lifecycle_store::StoreError;
use serde_json::Value;
use std::fs::{self, DirBuilder, File};
use std::io::Read;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PREPARE_TIMEOUT: Duration = Duration::from_secs(180);
const MAX_RECORD_BYTES: u64 = 256 * 1024;
const RECORD_SCHEMA: &str = "work-engine.pi-host-composition-record.v1";

pub(super) struct Prepared {
    pub host: PiHostConfig,
    pub root: PathBuf,
    pub deadline: u64,
    pub routes: Value,
}

pub(super) fn unix_seconds() -> Result<u64, StoreError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs())
        .map_err(|_| StoreError::Unavailable)
}

pub(super) fn read_json_file(path: &Path, limit: u64) -> Result<(String, Value), StoreError> {
    let rejected = |_| StoreError::Rejected;
    if !fs::symlink_metadata(path).map_err(rejected)?.is_file() {
        return Err(StoreError::Rejected);
    }
    let file = File::open(path).map_err(rejected)?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(rejected)?;
    if bytes.len() as u64 > limit {
        return Err(StoreError::Rejected);
    }
    let value = lifecycle_wire::parse_strict_json(&bytes).map_err(|_| StoreError::Rejected)?;
    Ok((sha(&bytes), value))
}

fn attempt_root(config: &NativeConfig, uid: u32) -> Result<PathBuf, StoreError> {
    let root = &config.root;
    let metadata = fs::symlink_metadata(root).map_err(|_| StoreError::Rejected)?;
    let canonical = fs::canonicalize(root).map_err(|_| StoreError::Rejected)?;
    if !metadata.is_dir()
        || metadata.uid() != uid
        || metadata.mode() & 0o7777 != 0o700
        || canonical.as_os_str() != root.as_os_str()
        || root.parent().is_none()
        || !root
            .as_os_str()
            .as_encoded_bytes()
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(b))
    {
        return Err(StoreError::Rejected);
    }
    let attempt = root.join(format!(
        "pi-attempt-{}",
        &sha(config.attempt_id.as_bytes())[..24]
    ));
    // mkdir is exclusive: even a failed prepare consumes this root.
    DirBuilder::new()
        .mode(0o700)
        .create(&attempt)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                StoreError::ClaimConflict
            } else {
                StoreError::Unavailable
            }
        })?;
    if !fs::canonicalize(&attempt).is_ok_and(|path| path.as_os_str() == attempt.as_os_str()) {
        return Err(StoreError::Rejected);
    }
    Ok(attempt)
}

fn prepare_stdout(
    config: &NativeConfig,
    composition: &PiHostComposition,
    root: &Path,
    duration: Duration,
) -> Result<Vec<u8>, StoreError> {
    let mut child = Command::new(&config.executable)
        .args(["prepare", "--profile"])
        .arg(&composition.profile)
        .arg("--root")
        .arg(root)
        .arg("--snapshot-id")
        .arg(&config.snapshot.id)
        .env_clear()
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| StoreError::Unavailable)?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(StoreError::Unavailable);
    };
    let (tx, rx) = std::sync::mpsc::channel();
    // The pinned Host's prepare command does not spawn children. The reader
    // drains at most the bound so a refusal can't flood runner memory.
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout.take(MAX_RECORD_BYTES + 1).read_to_end(&mut bytes);
        let _ = tx.send(result.map(|_| bytes));
    });
    let until = Instant::now() + duration;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) => return Err(StoreError::Rejected),
            Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(StoreError::Unavailable);
            }
        }
    }
    rx.recv_timeout(until.saturating_duration_since(Instant::now()))
        .ok()
        .and_then(Result::ok)
        .filter(|bytes| bytes.len() as u64 <= MAX_RECORD_BYTES)
        .ok_or(StoreError::Rejected)
}

fn validate_record(
    config: &NativeConfig,
    composition: &PiHostComposition,
    root: &Path,
    stdout: &[u8],
    started: u64,
    now: u64,
) -> Result<Prepared, StoreError> {
    let mut response =
        lifecycle_wire::parse_strict_json(stdout).map_err(|_| StoreError::Rejected)?;
    let pin = response
        .as_object_mut()
        .ok_or(StoreError::Rejected)?
        .remove("composition_sha256")
        .ok_or(StoreError::Rejected)?;
    let (digest, record) = read_json_file(&root.join("composition.json"), MAX_RECORD_BYTES)?;
    let prepared = record["prepared_unix_seconds"]
        .as_u64()
        .ok_or(StoreError::Rejected)?;
    let deadline = record["deadline_unix_seconds"]
        .as_u64()
        .ok_or(StoreError::Rejected)?;
    if pin != digest
        || response != record
        || record["schema"] != RECORD_SCHEMA
        || record["root"].as_str().map(Path::new) != Some(root)
        || record["profile"]["path"].as_str().map(Path::new) != Some(composition.profile.as_path())
        || record["profile"]["sha256"] != composition.profile_sha256
        || record["snapshot_id"] != config.snapshot.id
        || record["host_sha256"] != composition.host_sha256
        || !(started..=now).contains(&prepared)
        || deadline <= now
        || deadline.saturating_sub(prepared) > 3600
    {
        return Err(StoreError::Rejected);
    }
    let (digest, host) = read_json_file(&root.join("pi-host.json"), MAX_RECORD_BYTES)?;
    if record["pi_host_json_sha256"] != digest {
        return Err(StoreError::Rejected);
    }
    let host: PiHostConfig = serde_json::from_value(host).map_err(|_| StoreError::Rejected)?;
    if host.host_sha256 != composition.host_sha256
        || record["job_sha256"] != host.job_sha256
        || record["serve_config_sha256"] != host.serve_config_sha256
        || host.serve_config != root.join("serve-config.json")
        || host.state_dir != root.join("pi-host-state")
        || !fs::symlink_metadata(&host.state_dir).is_ok_and(|meta| meta.is_dir())
        || host.provider != "claude-sdk"
        || deadline - prepared < host.turn_deadline_secs.saturating_add(300)
    {
        return Err(StoreError::Rejected);
    }
    let routes = super::pi_host::check_composed_job(&host, root, deadline, &record["job_id"])?;
    if routes != record["routes"] {
        return Err(StoreError::Rejected);
    }
    Ok(Prepared {
        host,
        root: root.to_path_buf(),
        deadline,
        routes,
    })
}

pub(super) fn prepare(
    config: &NativeConfig,
    composition: &PiHostComposition,
    uid: u32,
) -> Result<Prepared, StoreError> {
    let root = attempt_root(config, uid)?;
    let started = unix_seconds()?;
    let stdout = prepare_stdout(config, composition, &root, PREPARE_TIMEOUT)?;
    validate_record(
        config,
        composition,
        &root,
        &stdout,
        started,
        unix_seconds()?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;

    fn config(root: &Path) -> NativeConfig {
        serde_json::from_value(json!({
            "executable":"/bin/false","root":root,"provider_route":"pi-host-v1",
            "simulator_base_url":"","session_id":"session:t","invocation_id":"invocation:t",
            "attempt_id":"attempt:t","prompt":"p","turn_grant_ref":"g","turn_grant_revision":1,
            "snapshot_grant_ref":"g","snapshot_grant_revision":1,
            "snapshot":{"id":"snapshot:t","source_revision":"r","members":[]}
        }))
        .unwrap()
    }

    fn put(root: &Path, name: &str, value: &Value) -> String {
        let bytes = serde_json::to_vec(value).unwrap();
        fs::write(root.join(name), &bytes).unwrap();
        sha(&bytes)
    }

    fn record_stdout(root: &Path, mut record: Value) -> Vec<u8> {
        record["composition_sha256"] = json!(put(root, "composition.json", &record));
        serde_json::to_vec(&record).unwrap()
    }

    #[test]
    fn prepared_record_binds_snapshot_pins_routes_files_and_deadline() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let config = config(root);
        let composition = PiHostComposition {
            profile: root.join("profile.json"),
            profile_sha256: "a".repeat(64),
            host_sha256: "b".repeat(64),
            credentials_file: root.join("credentials.json"),
        };
        let state = root.join("pi-host-state");
        fs::create_dir(&state).unwrap();
        let routes = json!([{"id":"main","payer":"p","endpoint":"https:anthropic-oauth"}]);
        let job = json!({"job_id":"job","state_dir":state,"deadline_unix_seconds":701,
            "coding":{"schema":"work-engine.pi-lifecycle-hosted.v1","provider":{
                "provider":"claude-sdk","model":"claude-haiku-5-5","hosted":{"routes":routes}}}});
        let job_digest = put(root, "job.json", &job);
        let serve = json!({"schema":"work-engine.pi-host-serve.v1","turn_deadline_secs":300,
            "job":{"sha256":job_digest,"path":root.join("job.json")}});
        let serve_digest = put(root, "serve-config.json", &serve);
        let host = json!({"host_sha256":composition.host_sha256,
            "serve_config":root.join("serve-config.json"),"serve_config_sha256":serve_digest,
            "job_sha256":job_digest,"state_dir":state,"provider":"claude-sdk",
            "model":"claude-haiku-5-5","turn_deadline_secs":300});
        let host_digest = put(root, "pi-host.json", &host);
        let record = json!({"schema":RECORD_SCHEMA,
            "profile":{"path":composition.profile,"sha256":composition.profile_sha256},
            "root":root,"job_id":"job","job_sha256":job_digest,"serve_config_sha256":serve_digest,
            "pi_host_json_sha256":host_digest,"host_sha256":composition.host_sha256,
            "snapshot_id":config.snapshot.id,"prepared_unix_seconds":100,"deadline_unix_seconds":701,"routes":routes});
        let stdout = record_stdout(root, record.clone());
        let check =
            |stdout: &[u8], now| validate_record(&config, &composition, root, stdout, 100, now);
        assert_eq!(check(&stdout, 101).unwrap().deadline, 701);
        assert!(check(&stdout, 701).is_err(), "expired job");
        for (field, value) in [
            ("root", json!(root.join("other"))),
            ("snapshot_id", json!("snapshot:other")),
            ("host_sha256", json!("c".repeat(64))),
            (
                "profile",
                json!({"path":composition.profile,"sha256":"c".repeat(64)}),
            ),
            ("deadline_unix_seconds", json!(702)),
            ("prepared_unix_seconds", json!(99)),
            ("pi_host_json_sha256", json!("c".repeat(64))),
            ("routes", json!([])),
        ] {
            let mut changed = record.clone();
            changed[field] = value;
            assert!(
                check(&record_stdout(root, changed), 101).is_err(),
                "{field}"
            );
        }
        let stdout = record_stdout(root, record.clone());
        fs::write(root.join("composition.json"), b"{}").unwrap();
        assert!(check(&stdout, 101).is_err(), "record changed after receipt");
        let stdout = record_stdout(root, record);
        fs::write(root.join("job.json"), b"{}").unwrap();
        assert!(check(&stdout, 101).is_err(), "job changed after receipt");
    }

    #[test]
    fn attempt_directory_is_private_and_single_use_even_when_empty() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let config = config(&root);
        let uid = fs::metadata(&root).unwrap().uid();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(attempt_root(&config, uid).is_err());
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let attempt = attempt_root(&config, uid).unwrap();
        assert_eq!(
            attempt,
            root.join(format!("pi-attempt-{}", &sha(b"attempt:t")[..24]))
        );
        assert_eq!(fs::metadata(attempt).unwrap().mode() & 0o7777, 0o700);
        assert!(matches!(
            attempt_root(&config, uid),
            Err(StoreError::ClaimConflict)
        ));
    }
}
