use bounded_inference::{
    ActionGrant, AuthError, DiagnosticGrant, HostError, PublicHost, SmokeGrant,
};
use lifecycle_semantic::RealDiagnosticPrepared;
use std::{
    env, fs,
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
    process::{Child, Command},
    time::{Duration, Instant},
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut args: Vec<String> = env::args().collect();
    let child = args.get(1).is_some_and(|v| v == "--bounded-child");
    if child {
        args.remove(1);
    }
    let result: Result<(), CliError> = if !child
        && matches!(
            args.get(1).map(String::as_str),
            Some("sign-in" | "models" | "refresh" | "disconnect" | "smoke" | "diagnostic")
        ) {
        run_bounded_child(&args).map_err(CliError::from)
    } else {
        run(args).await
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn run_bounded_child(args: &[String]) -> Result<(), &'static str> {
    let seconds = match args.get(1).map(String::as_str) {
        Some("sign-in") => 630,
        Some("models" | "disconnect") => 15,
        Some("refresh") => 20,
        Some("smoke" | "diagnostic") => 45,
        _ => return Err("invalid bounded action"),
    };
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let exe = env::current_exe().map_err(|_| "executable unavailable")?;
    let child = Command::new(exe)
        .arg("--bounded-child")
        .args(&args[1..])
        .spawn()
        .map_err(|_| "bounded child failed to start")?;
    let result = collect_bounded_child(child, deadline);
    if args
        .get(1)
        .is_some_and(|v| v == "smoke" || v == "diagnostic")
    {
        best_effort_child_closure(args, &result);
    }
    result
}

fn best_effort_child_closure(args: &[String], result: &Result<(), &'static str>) {
    let diagnostic = args.get(1).is_some_and(|v| v == "diagnostic");
    let (Some(grant_path), Some(work_path)) =
        (args.get(5), args.get(if diagnostic { 7 } else { 6 }))
    else {
        return;
    };
    let Ok(bytes) = read_small(grant_path) else {
        return;
    };
    let grant = if diagnostic {
        let Ok(grant) = serde_json::from_slice::<DiagnosticGrant>(&bytes) else {
            return;
        };
        grant.smoke
    } else {
        let Ok(grant) = serde_json::from_slice::<SmokeGrant>(&bytes) else {
            return;
        };
        grant
    };
    if grant.attempt_id.is_empty()
        || grant.attempt_id.len() > 128
        || !grant
            .attempt_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return;
    }
    let dir = Path::new(work_path);
    if !dir.is_absolute()
        || dir.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return;
    }
    let mut cursor = std::path::PathBuf::new();
    for component in dir.components() {
        cursor.push(component);
        if fs::symlink_metadata(&cursor).is_ok_and(|m| m.file_type().is_symlink()) {
            return;
        }
    }
    let Ok(meta) = fs::metadata(dir) else {
        return;
    };
    if !meta.is_dir()
        || meta.uid() != rustix::process::getuid().as_raw()
        || meta.permissions().mode() & 0o077 != 0
    {
        return;
    }
    let disposition = match result {
        Ok(()) => "observed_reaped_success",
        Err("bounded action failed") => "observed_reaped_failure",
        Err(_) => "collection_attempted",
    };
    let body = serde_json::json!({"owned_child_disposition":disposition,
        "remote_settlement":"unknown_from_local_process", "funding_settlement":"open_or_unestablished"});
    let path = dir.join(format!("owned-child-closure-{}.json", grant.attempt_id));
    if let Ok(mut file) = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
    {
        if file.write_all(body.to_string().as_bytes()).is_ok() {
            let _ = file.sync_all();
        }
    }
}

fn collect_bounded_child(mut child: Child, deadline: Instant) -> Result<(), &'static str> {
    loop {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("bounded action deadline exceeded; remote outcome may remain unknown");
        }
        let status = match child.try_wait() {
            Ok(status) => status,
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("bounded child status unavailable");
            }
        };
        match status {
            Some(status) if status.success() => return Ok(()),
            Some(_) => return Err("bounded action failed"),
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parent_records_attempted_collection_without_claiming_remote_closure() {
        let root = std::env::temp_dir().join(format!(
            "b2-parent-closure-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let grant_path = root.join("grant.json");
        fs::write(&grant_path, serde_json::json!({
            "release_id":"release","operation_id":"operation","attempt_id":"attempt",
            "client_id":"client","subject":"subject","credential_revision":1,
            "selected_model":"model","model_observation_sha256":"0".repeat(64),
            "funding_assertion_ref":"ref","funding_actor":"actor","funding_custodian":"custodian",
            "funding_setting_scope":"scope","funding_observed_at_unix":1,
            "credit_use_disabled":true,"included_plan_enabled":true,
            "funding_observation_valid_until_unix":2,"account_owner_accepts_open_custody":true,
            "accepted_closure_basis":"basis","release_actor":"actor"
        }).to_string()).unwrap();
        let args = vec![
            "b2-public-host".into(),
            "smoke".into(),
            "credential".into(),
            "host".into(),
            "action".into(),
            grant_path.to_string_lossy().into(),
            root.to_string_lossy().into(),
        ];
        best_effort_child_closure(
            &args,
            &Err("bounded action deadline exceeded; remote outcome may remain unknown"),
        );
        let marker: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("owned-child-closure-attempt.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(marker["owned_child_disposition"], "collection_attempted");
        assert_eq!(marker["remote_settlement"], "unknown_from_local_process");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn model_error_projection_contains_only_typed_evidence() {
        let error = CliError::Model(HostError::Https(
            bounded_inference::PublicHttpsError::HttpStatus(429),
        ));
        assert_eq!(
            error.to_string(),
            "model discovery failed: public HTTPS failed: public HTTPS rejected HTTP status 429"
        );
        let error = CliError::Model(HostError::Https(
            bounded_inference::PublicHttpsError::ModelJson,
        ));
        assert_eq!(
            error.to_string(),
            "model discovery failed: public HTTPS failed: public HTTPS model response is not JSON"
        );
    }

    #[test]
    fn deadline_terminates_and_collects_owned_child() {
        let child = Command::new("sleep").arg("2").spawn().unwrap();
        let pid = child.id();
        let started = Instant::now();
        assert!(collect_bounded_child(child, started + Duration::from_millis(100)).is_err());
        assert!(started.elapsed() < Duration::from_millis(500));
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
    }
}

enum CliError {
    Message(&'static str),
    Model(HostError),
    Readiness(AuthError),
}

impl From<&'static str> for CliError {
    fn from(message: &'static str) -> Self {
        Self::Message(message)
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(message) => f.write_str(message),
            Self::Model(error) => write!(f, "model discovery failed: {error}"),
            Self::Readiness(error) => write!(f, "local readiness inspection failed: {error}"),
        }
    }
}

fn read_bounded(path: &str, limit: usize) -> Result<Vec<u8>, &'static str> {
    let path = Path::new(path);
    if !path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err("input path must be absolute and normalized");
    }
    let bytes = fs::read(path).map_err(|_| "input read failed")?;
    if bytes.len() > limit {
        return Err("input too large");
    }
    Ok(bytes)
}
fn read_small(path: &str) -> Result<Vec<u8>, &'static str> {
    read_bounded(path, 16 * 1024)
}

async fn run(args: Vec<String>) -> Result<(), CliError> {
    if args.len() == 2 && args[1] == "usage" {
        println!("https://chatgpt.com/settings/usage");
        return Ok(());
    }
    if args.len() == 2 && args[1] == "identity" {
        let executable = ActionGrant::executable_sha256().map_err(|_| "identity failed")?;
        println!(
            "{}",
            serde_json::json!({
                "source_sha256": ActionGrant::source_sha256(),
                "executable_sha256": executable
            })
        );
        return Ok(());
    }
    let command = args.get(1).map(String::as_str).ok_or("missing command")?;
    let expected = match command {
        "account" | "status" | "renewal-target" => 4,
        "sign-in" | "models" | "refresh" | "disconnect" => 5,
        "smoke" => 7,
        "diagnostic" => 8,
        _ => return Err(CliError::Message("invalid command")),
    };
    if args.len() != expected {
        return Err(CliError::Message(
            "usage: b2-public-host <account|status|renewal-target|sign-in|refresh|models|disconnect|smoke|diagnostic> <absolute-credential-dir> <host-id> [action-grant.json] [grant.json prepared.json absolute-work-dir]",
        ));
    }
    let credential_dir = Path::new(&args[2]);
    if !credential_dir.is_absolute() {
        return Err(CliError::Message("credential directory must be absolute"));
    }
    let work_dir = if command == "smoke" || command == "diagnostic" {
        Some(Path::new(
            &args[if command == "diagnostic" { 7 } else { 6 }],
        ))
    } else {
        None
    };
    let smoke_bytes = if command == "smoke" || command == "diagnostic" {
        Some(read_small(&args[5])?)
    } else {
        None
    };
    let action = if command != "account" && command != "status" && command != "renewal-target" {
        let bytes = read_small(&args[4])?;
        let grant: ActionGrant =
            serde_json::from_slice(&bytes).map_err(|_| "invalid action grant")?;
        grant
            .validate(
                command,
                credential_dir,
                &args[3],
                work_dir,
                smoke_bytes.as_deref(),
            )
            .map_err(|error| {
                if command == "models" {
                    CliError::Model(error)
                } else {
                    CliError::Message("action admission failed")
                }
            })?;
        Some(grant)
    } else {
        None
    };
    if command == "status" {
        let readiness = PublicHost::inspect_local_readiness(credential_dir, &args[3])
            .map_err(CliError::Readiness)?;
        println!("{}", serde_json::json!({"readiness": readiness}));
        return Ok(());
    }
    if command == "renewal-target" {
        let target = PublicHost::inspect_local_renewal_target(credential_dir, &args[3])
            .map_err(CliError::Readiness)?;
        println!(
            "{}",
            serde_json::json!({"client_id":target.client_id,
            "subject":target.subject,"credential_revision":target.credential_revision})
        );
        return Ok(());
    }
    if command == "refresh" {
        PublicHost::validate_refresh_preopen(action.as_ref().unwrap(), credential_dir, &args[3])
            .map_err(|_| CliError::Message("refresh target or custody admission failed"))?;
    }
    let host = PublicHost::open(credential_dir, &args[3])
        .await
        .map_err(|error| {
            if command == "models" {
                CliError::Model(error)
            } else {
                CliError::Message("local host initialization failed")
            }
        })?;
    match command {
        "sign-in" => {
            host.sign_in_admitted(action.as_ref().unwrap(), "Work Engine", |url| {
                if url.contains("id_token_hint=") {
                    return Err(bounded_inference::HostError::Policy);
                }
                println!("Open this authorization URL in your browser:\n{url}");
                Ok(())
            })
            .await
            .map_err(|_| "sign-in failed")?;
            println!("Connected. Use account to inspect the selected registration.");
        }
        "account" => {
            let selected = host.selected().map_err(|_| "no ready selected account")?;
            println!(
                "{}",
                serde_json::json!({
                    "client_id": selected.client_id, "subject": selected.subject,
                    "credential_revision": selected.credential_revision
                })
            );
        }
        "refresh" => {
            let updated = host
                .refresh_admitted(action.as_ref().unwrap())
                .await
                .map_err(|error| match error {
                    HostError::RefreshConfiguration => "refresh client configuration rejected; repair configuration before another admitted attempt",
                    _ => "refresh failed; inspect local status before any further action",
                })?;
            println!(
                "{}",
                serde_json::json!({"result":"rotated",
                "credential_revision":updated.credential_revision})
            );
        }
        "models" => {
            let observation = host
                .models_admitted(action.as_ref().unwrap())
                .await
                .map_err(CliError::Model)?;
            println!(
                "{}",
                serde_json::json!({
                    "response_sha256": observation.response_sha256,
                    "observation_id": observation.observation_id,
                    "response_bytes": observation.response_bytes,
                    "observed_at_unix": observation.observed_at_unix,
                    "credential_revision": observation.registration.credential_revision,
                    "model_count": observation.models.len()
                })
            );
        }
        "disconnect" => {
            let confirmed = host
                .disconnect_admitted(action.as_ref().unwrap())
                .await
                .map_err(|_| "local disconnect failed")?;
            println!(
                "{}",
                if confirmed {
                    "local credentials removed; remote revocation confirmed"
                } else {
                    "local credentials removed; remote revocation unconfirmed"
                }
            );
        }
        "smoke" => {
            let bytes = smoke_bytes.as_deref().ok_or("missing smoke grant")?;
            let grant: SmokeGrant =
                serde_json::from_slice(bytes).map_err(|_| "invalid smoke grant")?;
            let projection = host.smoke_admitted(action.as_ref().unwrap(), &grant, bytes,
                work_dir.ok_or("missing work directory")?)
                .await
                .map_err(|_| "smoke failed; inspect private attempt and funding obligation before any further action")?;
            println!(
                "{}",
                serde_json::to_string(&projection).map_err(|_| "projection failed")?
            );
        }
        "diagnostic" => {
            let bytes = smoke_bytes.as_deref().ok_or("missing diagnostic grant")?;
            let prepared_bytes = read_bounded(&args[6], 256 * 1024)?;
            let prepared: RealDiagnosticPrepared =
                serde_json::from_slice(&prepared_bytes).map_err(|_| "invalid prepared request")?;
            let projection = host.diagnostic_admitted(action.as_ref().unwrap(), bytes,
                &prepared, work_dir.ok_or("missing work directory")?)
                .await.map_err(|_| "diagnostic failed; inspect private attempt and funding obligation before further action")?;
            println!(
                "{}",
                serde_json::to_string(&projection).map_err(|_| "projection failed")?
            );
        }
        _ => unreachable!(),
    }
    Ok(())
}
