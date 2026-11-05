use serde_json::{Value, json};
use session_query::{INPUT_LIMIT, Profile, Query};
use std::{
    env,
    io::{self, Read},
    net::SocketAddr,
    path::Path,
    process,
    time::Duration,
};
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ServiceProfile {
    version: u8,
    kind: String,
    endpoint: SocketAddr,
    credential_file: String,
    timeout_seconds: u64,
}
struct Output {
    value: Value,
    service: bool,
}
fn main() {
    let args: Vec<String> = env::args().collect();
    let json_mode = args.get(3).is_some_and(|s| s == "--json");
    let mut service = false;
    match run(&args, &mut service) {
        Ok(output) => {
            let value = output.value;
            let failed = output.service && value.get("owner_result").is_none_or(Value::is_null);
            if json_mode {
                println!(
                    "{}",
                    if output.service {
                        json!({"schema":"work-engine.session-query.service.v1","ok":!failed,"envelope":value})
                    } else {
                        json!({"schema":"work-engine.session-query.v1","ok":true,"result":value})
                    }
                );
            } else if output.service {
                if let Some(owner) = value.get("owner_result").filter(|v| !v.is_null()) {
                    service_human(&args[2], owner);
                } else {
                    eprintln!(
                        "Query failed: {}",
                        safe_human(
                            value
                                .get("owner_error")
                                .filter(|v| !v.is_null())
                                .or_else(|| value.get("error").filter(|v| !v.is_null()))
                                .and_then(|e| e.get("code"))
                                .and_then(Value::as_str)
                                .unwrap_or("invalid_response")
                        )
                    );
                }
            } else {
                human(&args[2], &value);
            }
            if failed {
                process::exit(1);
            }
        }
        Err(code) => {
            if json_mode {
                if service {
                    let kind = if matches!(
                        code,
                        "transport_timeout"
                            | "transport_unavailable"
                            | "transport_rejected"
                            | "invalid_response"
                            | "output_too_large"
                    ) {
                        "transport_error"
                    } else {
                        "error"
                    };
                    println!(
                        "{}",
                        json!({"schema":"work-engine.session-query.service.v1","ok":false,kind:{"code":code}})
                    );
                } else {
                    println!(
                        "{}",
                        json!({"schema":"work-engine.session-query.v1","ok":false,"error":{"code":code}})
                    );
                }
            } else {
                eprintln!("Query failed: {}", safe_human(code));
            }
            process::exit(1);
        }
    }
}
fn run(args: &[String], service: &mut bool) -> Result<Output, &'static str> {
    if !(args.len() == 4 || args.len() == 5) || !matches!(args[3].as_str(), "--json" | "--human") {
        return Err("invalid_invocation");
    }
    let profile_bytes = session_query::read_file(&args[1], 16384, true)?;
    let raw: Value = serde_json::from_slice(&profile_bytes).map_err(|_| "invalid_profile")?;
    *service = raw.get("kind").is_some();
    let service_profile = if *service {
        let p: ServiceProfile =
            serde_json::from_slice(&profile_bytes).map_err(|_| "invalid_profile")?;
        if p.version != 1
            || p.kind != "local-service"
            || !p.endpoint.ip().is_loopback()
            || p.endpoint.port() == 0
            || !Path::new(&p.credential_file).is_absolute()
            || !(1..=60).contains(&p.timeout_seconds)
        {
            return Err("invalid_profile");
        }
        if !matches!(
            args[2].as_str(),
            "list_sessions" | "read_session" | "read_session_operation" | "read_session_result"
        ) {
            return Err("invalid_query");
        }
        Some(p)
    } else {
        None
    };
    let direct_profile = if service_profile.is_none() {
        let p: Profile = serde_json::from_slice(&profile_bytes).map_err(|_| "invalid_profile")?;
        p.validate()?;
        Some(p)
    } else {
        None
    };
    let bytes = if args.len() == 4 {
        b"{}".to_vec()
    } else if args[4] == "-" {
        let mut b = Vec::new();
        io::stdin()
            .take(INPUT_LIMIT as u64 + 1)
            .read_to_end(&mut b)
            .map_err(|_| "input_unavailable")?;
        b
    } else {
        session_query::read_file(&args[4], INPUT_LIMIT, true)?
    };
    if bytes.len() > INPUT_LIMIT {
        return Err("input_too_large");
    }
    let arguments: Value = serde_json::from_slice(&bytes).map_err(|_| "invalid_arguments")?;
    let query = Query::parse(&args[2], arguments.clone())?;
    if let Some(p) = service_profile {
        let timeout = Duration::from_secs(p.timeout_seconds);
        let client = work_engine_server::LocalClient::new(p.endpoint, p.credential_file, timeout)?;
        let operation = args[2].clone();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result = match operation.as_str() {
                "list_sessions" | "read_session" => {
                    client.query(&operation, arguments, "work-engine-query")
                }
                "read_session_operation" | "read_session_result" => {
                    client.command(&operation, arguments, "work-engine-query")
                }
                _ => Err("invalid_query"),
            };
            let _ = tx.send(result);
        });
        let value = rx
            .recv_timeout(timeout)
            .map_err(|_| "transport_timeout")??;
        Ok(Output {
            value,
            service: true,
        })
    } else {
        Ok(Output {
            value: session_query::query(&direct_profile.ok_or("invalid_profile")?, &query)?,
            service: false,
        })
    }
}
fn safe_human(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().to_string().chars().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
// Render owner observations without converting a missing or partial observation
// into a negative claim. The direct v1 presentation remains unchanged.
fn service_human(name: &str, v: &Value) {
    human(name, v);
    if matches!(name, "read_session_operation" | "read_session_result") {
        for (key, label) in [
            ("workers", "Workers"),
            ("usage", "Usage"),
            ("process_closure", "Process closure"),
        ] {
            let observation = &v[key];
            let state = observation
                .get("availability")
                .and_then(Value::as_str)
                .map(safe_human)
                .unwrap_or_else(
                    || match observation.get("available").and_then(Value::as_bool) {
                        Some(true) => "available".into(),
                        Some(false) => "unavailable".into(),
                        None => "unknown".into(),
                    },
                );
            println!("{label}: {state}");
        }
        if name == "read_session_result" {
            let completion = &v["completion"];
            let state = completion
                .get("status")
                .and_then(Value::as_str)
                .map(safe_human)
                .unwrap_or_else(
                    || match completion.get("available").and_then(Value::as_bool) {
                        Some(false) => "unavailable".into(),
                        _ => "unknown".into(),
                    },
                );
            println!("Completion: {state}");
        }
    }
}

fn human(name: &str, v: &Value) {
    match name {
        "list_sessions" => {
            let rows = v["sessions"].as_array();
            println!(
                "{} session(s)",
                v["matched"]
                    .as_u64()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            );
            if let Some(rows) = rows {
                for r in rows {
                    println!(
                        "{}  {}  {}",
                        safe_human(r["sessionId"].as_str().unwrap_or("?")),
                        safe_human(r["host"].as_str().unwrap_or("?")),
                        safe_human(r["title"].as_str().unwrap_or(""))
                    );
                }
            }
        }
        "read_session" => {
            println!(
                "Session {}",
                safe_human(v["sessionId"].as_str().unwrap_or("?"))
            );
            if let Some(rows) = v["entries"].as_array() {
                for r in rows {
                    println!(
                        "{}: {}",
                        safe_human(r["role"].as_str().unwrap_or("?")),
                        safe_human(r["text"].as_str().unwrap_or(""))
                    );
                }
            }
        }
        "read_session_operation" => println!(
            "Operation {}: {}",
            safe_human(v["operation_id"].as_str().unwrap_or("?")),
            safe_human(v["entry_state"].as_str().unwrap_or("unknown"))
        ),
        "read_session_result" => {
            println!(
                "Result: {}",
                match v["result"]["available"].as_bool() {
                    Some(true) => "available",
                    Some(false) => "unavailable",
                    None => "unknown",
                }
            );
            if let Some(s) = v["result"]["text"].as_str() {
                println!("{}", safe_human(s));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, net::TcpListener, os::unix::fs::PermissionsExt};

    #[test]
    fn service_profile_rejects_duplicate_and_unknown_fields() {
        for text in [
            r#"{"version":1,"version":1,"kind":"local-service","endpoint":"127.0.0.1:1","credential_file":"/tmp/c","timeout_seconds":1}"#,
            r#"{"version":1,"kind":"local-service","endpoint":"127.0.0.1:1","credential_file":"/tmp/c","timeout_seconds":1,"extra":true}"#,
        ] {
            assert!(serde_json::from_str::<ServiceProfile>(text).is_err());
        }
    }

    #[test]
    fn unsupported_service_operations_do_not_connect() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let path = std::env::temp_dir().join(format!("query-profile-{}", process::id()));
        let text = json!({"version":1,"kind":"local-service","endpoint":listener.local_addr().unwrap().to_string(),"credential_file":"/unread/credential","timeout_seconds":1});
        fs::write(&path, text.to_string()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        for name in ["submit_session_request", "unknown_method"] {
            let args = vec![
                "query".into(),
                path.to_string_lossy().into_owned(),
                name.into(),
                "--json".into(),
            ];
            let mut service = false;
            assert!(matches!(run(&args, &mut service), Err("invalid_query")) && service);
            assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
        }
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn human_text_escapes_terminal_controls() {
        assert_eq!(
            safe_human("a\u{1b}\u{9b}\r\u{7}b"),
            "a\\u{1b}\\u{9b}\\r\\u{7}b"
        );
    }
}
