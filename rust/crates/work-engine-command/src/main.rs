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
fn human_owner(owner: &Value) {
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
        || args[2] != "submit_session_request"
        || !matches!(args[3].as_str(), "--json" | "--human")
    {
        return Err("invalid_invocation");
    }
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
    let a: Arguments = serde_json::from_slice(&bytes).map_err(|_| "invalid_arguments")?;
    let id = a.request_id.as_bytes();
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
    let arguments = serde_json::to_value(a).map_err(|_| "invalid_arguments")?;
    let timeout = Duration::from_secs(p.timeout_seconds);
    let client = work_engine_server::LocalClient::new(p.endpoint, p.credential_file, timeout)?;
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _ = tx.send(client.command("submit_session_request", arguments, "work-engine-command"));
    });
    rx.recv_timeout(timeout).map_err(|_| "transport_timeout")?
}
fn main() {
    let args: Vec<String> = env::args().collect();
    let json_mode = args.get(3).is_some_and(|s| s == "--json");
    match run(&args) {
        Ok(envelope) => {
            let owner = envelope.get("owner_result").filter(|v| !v.is_null());
            let ok = owner.is_some();
            if json_mode {
                println!(
                    "{}",
                    json!({"schema":"work-engine.session-command.service.v1","ok":ok,"envelope":envelope})
                );
            } else if let Some(owner) = owner {
                human_owner(owner);
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
                    json!({"schema":"work-engine.session-command.service.v1","ok":false,"error":{"code":code}})
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
