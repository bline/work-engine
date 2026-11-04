use serde_json::{Value, json};
use session_query::{INPUT_LIMIT, Profile, Query};
use std::{
    env,
    io::{self, Read},
    process,
};
fn main() {
    let args: Vec<String> = env::args().collect();
    let json_mode = args.get(3).is_some_and(|s| s == "--json");
    match run(&args) {
        Ok(value) => {
            if json_mode {
                println!(
                    "{}",
                    json!({"schema":"work-engine.session-query.v1","ok":true,"result":value})
                );
            } else {
                human(&args[2], &value);
            }
        }
        Err(code) => {
            if json_mode {
                println!(
                    "{}",
                    json!({"schema":"work-engine.session-query.v1","ok":false,"error":{"code":code}})
                );
            } else {
                eprintln!("Query failed: {code}");
            }
            process::exit(1);
        }
    }
}
fn run(args: &[String]) -> Result<Value, &'static str> {
    if !(args.len() == 4 || args.len() == 5) || !matches!(args[3].as_str(), "--json" | "--human") {
        return Err("invalid_invocation");
    }
    let profile: Profile =
        serde_json::from_slice(&session_query::read_file(&args[1], 16384, true)?)
            .map_err(|_| "invalid_profile")?;
    profile.validate()?;
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
    let arguments = serde_json::from_slice(&bytes).map_err(|_| "invalid_arguments")?;
    let query = Query::parse(&args[2], arguments)?;
    session_query::query(&profile, &query)
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
                if v["result"]["available"] == true {
                    "available"
                } else {
                    "unavailable"
                }
            );
            if let Some(s) = v["result"]["text"].as_str() {
                println!("{}", safe_human(s));
            }
        }
        _ => {}
    }
}
