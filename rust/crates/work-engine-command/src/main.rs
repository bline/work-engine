use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    env,
    io::{self, Read},
    net::SocketAddr,
    path::Path,
    process,
    time::Duration,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    version: u8,
    kind: String,
    endpoint: SocketAddr,
    credential_file: String,
    timeout_seconds: u64,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    request_id: String,
    grant_credential: String,
    session_id: String,
    workspace: String,
    input_text: String,
}
const JOB_OPERATIONS: [&str; 4] = [
    "submit_coding_job",
    "read_coding_job_operation",
    "read_coding_job_result",
    "cancel_coding_job",
];
const JOB_READ_KEYS: [&str; 3] = ["request_id", "authorization_id", "job_credential"];
const JOB_SUBMIT_KEYS: [&str; 5] = [
    "request_id",
    "authorization_id",
    "job_credential",
    "job_sha256",
    "packet_sha256",
];
/// Strict client-side shape check. The server repeats it and owns the authority decision.
fn job_arguments(operation: &str, bytes: &[u8]) -> Result<Value, &'static str> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| "invalid_arguments")?;
    let keys: &[&str] = if operation == "submit_coding_job" {
        &JOB_SUBMIT_KEYS
    } else {
        &JOB_READ_KEYS
    };
    let object = value.as_object().ok_or("invalid_arguments")?;
    if object.len() != keys.len()
        || !keys
            .iter()
            .all(|k| object.get(*k).is_some_and(Value::is_string))
    {
        return Err("invalid_arguments");
    }
    Ok(value)
}
fn job_human(owner: &Value) {
    for (label, key) in [
        ("Operation", "operation_id"),
        ("Entry", "entry"),
        ("State", "state"),
        ("Job", "job_id"),
        ("Host receipt", "host_receipt_id"),
        ("Cancellation", "cancellation"),
        ("Result", "result"),
        ("Terminal", "terminal"),
        ("Validation", "validation"),
        ("Cleanup", "cleanup"),
        ("Process cleanup", "process_cleanup"),
        ("Acceptance", "acceptance"),
        ("Domain effect settlement", "domain_effect_settlement"),
    ] {
        println!("{label}: {}", observation(owner.get(key)));
    }
    println!("Settlement: not established by the adapter");
}
fn safe(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().to_string().chars().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
fn observation(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "unknown".into(),
        Some(Value::String(s)) => safe(s),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Object(map)) => {
            if map.get("available").and_then(Value::as_bool) == Some(false) {
                "unavailable".into()
            } else if let Some(s) = map.get("availability").and_then(Value::as_str) {
                safe(s)
            } else if let Some(s) = map.get("status").and_then(Value::as_str) {
                safe(s)
            } else if map.get("available").and_then(Value::as_bool) == Some(true) {
                "available (details not shown)".into()
            } else {
                "unknown".into()
            }
        }
        _ => "unknown".into(),
    }
}
fn human_owner(operation: &str, owner: &Value) {
    if JOB_OPERATIONS.contains(&operation) {
        return job_human(owner);
    }
    for (label, key) in [
        ("Operation", "operation_id"),
        ("Entry", "entry_state"),
        ("Native status", "native_status"),
        ("Native turn", "native_turn_id"),
        ("Last reconciliation", "last_reconciliation_at"),
        ("Completion", "completion"),
        ("Result", "result"),
        ("Process closure", "process_closure"),
    ] {
        println!("{label}: {}", observation(owner.get(key)));
    }
    match owner.get("limitations").and_then(Value::as_array) {
        Some(items) if items.is_empty() => println!("Limitations: none reported"),
        Some(items) => {
            println!("Limitations:");
            for item in items {
                println!("  {}", observation(Some(item)));
            }
        }
        None => println!("Limitations: unknown"),
    }
    println!("Settlement: not established by submission");
}
fn uncertainty_guidance() {
    eprintln!(
        "Entry, cancellation and settlement are unknown; inspect the original request before any explicit repeat."
    );
}
fn ambiguous_owner_error(code: &str) -> bool {
    !matches!(
        code,
        "unauthorized" | "expired" | "target_mismatch" | "payload_conflict" | "session_busy"
    )
}
fn run(args: &[String]) -> Result<Value, &'static str> {
    if args.len() != 5
        || !(args[2] == "submit_session_request" || JOB_OPERATIONS.contains(&args[2].as_str()))
        || !matches!(args[3].as_str(), "--json" | "--human")
    {
        return Err("invalid_invocation");
    }
    let job = JOB_OPERATIONS.contains(&args[2].as_str());
    let p: Profile = serde_json::from_slice(&session_query::read_file(&args[1], 16384, true)?)
        .map_err(|_| "invalid_profile")?;
    if p.version != 1
        || p.kind != "local-service"
        || !p.endpoint.ip().is_loopback()
        || p.endpoint.port() == 0
        || !Path::new(&p.credential_file).is_absolute()
        || !(1..=60).contains(&p.timeout_seconds)
    {
        return Err("invalid_profile");
    }
    let bytes = if args[4] == "-" {
        let mut b = Vec::new();
        io::stdin()
            .take(session_query::INPUT_LIMIT as u64 + 1)
            .read_to_end(&mut b)
            .map_err(|_| "input_unavailable")?;
        b
    } else {
        session_query::read_file(&args[4], session_query::INPUT_LIMIT, true)?
    };
    if bytes.len() > session_query::INPUT_LIMIT {
        return Err("input_too_large");
    }
    let (request_id, arguments) = if job {
        let value = job_arguments(&args[2], &bytes)?;
        (
            value["request_id"].as_str().unwrap_or_default().to_string(),
            value,
        )
    } else {
        let a: Arguments = serde_json::from_slice(&bytes).map_err(|_| "invalid_arguments")?;
        (
            a.request_id.clone(),
            serde_json::to_value(a).map_err(|_| "invalid_arguments")?,
        )
    };
    let id = request_id.as_bytes();
    if id.len() != 36
        || !id.iter().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                *b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
    {
        return Err("invalid_arguments");
    }
    let timeout = Duration::from_secs(p.timeout_seconds);
    let client = work_engine_server::LocalClient::new(p.endpoint, p.credential_file, timeout)?;
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let operation = args[2].clone();
    std::thread::spawn(move || {
        let _ = tx.send(if job {
            client.job(&operation, arguments, "work-engine-command")
        } else {
            client.command(&operation, arguments, "work-engine-command")
        });
    });
    rx.recv_timeout(timeout).map_err(|_| "transport_timeout")?
}
fn main() {
    let args: Vec<String> = env::args().collect();
    let json_mode = args.get(3).is_some_and(|s| s == "--json");
    let schema = if args
        .get(2)
        .is_some_and(|op| JOB_OPERATIONS.contains(&op.as_str()))
    {
        "work-engine.coding-job-command.service.v1"
    } else {
        "work-engine.session-command.service.v1"
    };
    match run(&args) {
        Ok(envelope) => {
            let owner = envelope.get("owner_result").filter(|v| !v.is_null());
            let ok = owner.is_some();
            if json_mode {
                println!("{}", json!({"schema":schema,"ok":ok,"envelope":envelope}));
            } else if let Some(owner) = owner {
                human_owner(&args[2], owner);
            } else {
                let code = envelope
                    .get("owner_error")
                    .filter(|v| !v.is_null())
                    .or_else(|| envelope.get("error"))
                    .and_then(|v| v.get("code"))
                    .and_then(Value::as_str)
                    .unwrap_or("invalid_response");
                eprintln!("Command failed: {}", safe(code));
                if ambiguous_owner_error(code) {
                    uncertainty_guidance();
                }
            }
            if !ok {
                process::exit(1);
            }
        }
        Err(code) => {
            if json_mode {
                println!(
                    "{}",
                    json!({"schema":schema,"ok":false,"error":{"code":code}})
                );
            } else {
                eprintln!("Command failed: {}", safe(code));
                if matches!(
                    code,
                    "transport_timeout"
                        | "transport_unavailable"
                        | "transport_rejected"
                        | "invalid_response"
                        | "output_too_large"
                ) {
                    uncertainty_guidance();
                }
            }
            process::exit(1);
        }
    }
}
