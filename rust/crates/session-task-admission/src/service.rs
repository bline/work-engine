//! Local admission inspection and separately credentialled coordinator registry.
//! It cannot mint human approval, commit admission, or dispatch native effects.
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::Digest;
use std::{
    io::{Read, Write},
    path::Path,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    pub(crate) schema: String,
    pub(crate) database: String,
    pub(crate) sources: crate::inspect::Sources,
    #[serde(default)]
    coordinator_token_sha256: Option<String>,
}
#[cfg(unix)]
pub(crate) fn protected(path: &Path) -> Option<Vec<u8>> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    if !path.is_absolute() {
        return None;
    }
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .ok()?;
    let m = f.metadata().ok()?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o077 != 0
        || m.len() > 128 * 1024
    {
        return None;
    }
    let mut data = Vec::new();
    std::io::Read::by_ref(&mut f)
        .take(128 * 1024 + 1)
        .read_to_end(&mut data)
        .ok()?;
    let after = f.metadata().ok()?;
    if data.len() as u64 != m.len()
        || after.len() != m.len()
        || after.mtime() != m.mtime()
        || after.mtime_nsec() != m.mtime_nsec()
    {
        return None;
    }
    Some(data)
}
#[cfg(unix)]
pub(crate) fn call_at(config_path: &str, input: &str, now: time::OffsetDateTime) -> Option<Value> {
    let config: Config = serde_json::from_slice(&protected(Path::new(config_path))?).ok()?;
    if config.schema != "dot.admission-config/v1-proposal"
        || !Path::new(&config.database).is_absolute()
    {
        return None;
    }
    let meta = std::fs::symlink_metadata(&config.database).ok()?;
    use std::os::unix::fs::MetadataExt;
    if !meta.is_file()
        || meta.file_type().is_symlink()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o077 != 0
    {
        return None;
    }
    let db = rusqlite::Connection::open_with_flags(
        &config.database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .ok()?;
    crate::store::initialize(&db).ok()?;
    let request: crate::request::AdmissionRequest = serde_json::from_str(input).ok()?;
    let existing = crate::store::retrieve_existing(&db, &request, &config.sources.actor).ok()?;
    if let Some(existing) = existing {
        let at = now
            .format(&time::format_description::well_known::Rfc3339)
            .ok()?;
        let revision = crate::hash(&format!("{}\0{}", existing.admission.id, existing.status));
        return Some(
            json!({"schema":"dot.admission-view/v1-proposal","admission_id":existing.admission.id,"task":request.task,"decision":request.decision,"answer_id":request.decision.id,"revision":"1","status":existing.status,"hold_reasons":[],"reservation_ref":null,"authority_revision":null,"native_operation_id":null,"observed_at":at,"source_revision":revision}),
        );
    }
    let evidence = crate::inspect::collect(&config.sources, &db, input, now).ok()?;
    let id = crate::hash(&format!(
        "dot-admission/v1\0{}\0{}",
        evidence.decision.id, evidence.task.sha256
    ));
    let at = now
        .format(&time::format_description::well_known::Rfc3339)
        .ok()?;
    Some(
        json!({"schema":"dot.admission-view/v1-proposal","admission_id":id,"task":evidence.task,"decision":evidence.decision,"answer_id":evidence.decision.id,"revision":"held","status":"held","hold_reasons":["exclusive_controller_unqualified","mode_freshness_unqualified","native_profile_unqualified"],"reservation_ref":null,"authority_revision":null,"native_operation_id":null,"observed_at":at,"source_revision":evidence.board_revision}),
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinatorCommand {
    schema: String,
    operation: String,
    credential: String,
    #[serde(deserialize_with = "crate::required_option")]
    expected_generation: Option<i64>,
    state: String,
    #[serde(deserialize_with = "crate::required_option")]
    proposal: Option<crate::Ref>,
    #[serde(deserialize_with = "crate::required_option")]
    request_json: Option<String>,
}
/// Coordinator capability is distinct from the owner's signed human decision.
/// No MCP operation forwards this command or credential. The configured token
/// and DB must be isolated from worker access by a qualified OS boundary;
/// same-UID permissions alone are not such a boundary.
#[cfg(unix)]
pub(crate) fn coordinator(config_path: &str, input: &str) -> Option<Value> {
    use std::os::unix::fs::MetadataExt;
    let config: Config = serde_json::from_slice(&protected(Path::new(config_path))?).ok()?;
    if config.schema != "dot.admission-config/v1-proposal"
        || !Path::new(&config.database).is_absolute()
    {
        return None;
    }
    let command: CoordinatorCommand = serde_json::from_str(input).ok()?;
    let expected = config.coordinator_token_sha256.as_ref()?;
    if command.schema != "dot.coordinator-command/v1-proposal"
        || command.credential.len() < 32
        || command.credential.len() > 4096
        || expected.len() != 64
        || crate::hash(&command.credential) != *expected
    {
        return None;
    }
    let meta = std::fs::symlink_metadata(&config.database).ok()?;
    if !meta.is_file()
        || meta.file_type().is_symlink()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o077 != 0
    {
        return None;
    }
    let mut db = rusqlite::Connection::open_with_flags(
        &config.database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
    )
    .ok()?;
    crate::store::initialize(&db).ok()?;
    let generation = match command.operation.as_str() {
        "register_proposal" if command.request_json.is_none() => crate::store::register_proposal(
            &mut db,
            command.proposal.as_ref()?,
            &command.state,
            command.expected_generation,
        )
        .ok()?,
        "register_task" if command.proposal.is_none() => crate::store::register_task(
            &mut db,
            command.request_json.as_ref()?,
            &command.state,
            command.expected_generation,
        )
        .ok()?,
        _ => return None,
    };
    Some(json!({"generation":generation}))
}
/// Operator CLI projection of the existing signed-control owner. A profile
/// pin or coordinator credential is not authority: current board/key proof is
/// verified again by ingest_control before the registry transition.
fn scoped_control(config_path: &str, input: &str) -> Option<Value> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Request {
        schema: String,
        profile: crate::canonical_source::Pin,
    }
    let request: Request = serde_json::from_str(input).ok()?;
    if request.schema != "dot.scoped-control-ingest/v1" {
        return None;
    }
    let raw = protected(Path::new(config_path))?;
    let pin = crate::canonical_source::Pin {
        path: Path::new(config_path).to_path_buf(),
        sha256: format!("{:x}", sha2::Sha256::digest(&raw)),
    };
    crate::scoped_pilot::ingest_control(pin, request.profile, time::OffsetDateTime::now_utc()).ok()
}
/// CLI input is a bounded task admission request; config is an operator startup
/// parameter, never selected by the request. Any unavailable source holds.
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let mut bytes = Vec::new();
    let ok = std::io::stdin()
        .take(256 * 1024 + 1)
        .read_to_end(&mut bytes)
        .is_ok();
    let result = if ok && bytes.len() <= 256 * 1024 && args.len() == 4 && args[1] == "--config" {
        std::str::from_utf8(&bytes)
            .ok()
            .and_then(|s| match args[3].as_str() {
                "rpc" => call_at(&args[2], s, time::OffsetDateTime::now_utc())
                    .map(|v| json!({"schema_version":1,"admission":v})),
                "scoped-control" => scoped_control(&args[2], s)
                    .map(|v| json!({"schema_version":1,"scoped_control":v})),
                "coordinator" => {
                    coordinator(&args[2], s).map(|v| json!({"schema_version":1,"coordinator":v}))
                }
                _ => None,
            })
    } else {
        None
    };
    let output = match result {
        Some(result) => {
            json!({"schema_version":1,"protocol":"task-admission-rpc-v1","ok":true,"result":result})
        }
        None => {
            json!({"schema_version":1,"protocol":"task-admission-rpc-v1","ok":false,"error":"bridge_unavailable"})
        }
    };
    let _ = writeln!(std::io::stdout(), "{output}");
}
