use std::path::PathBuf;
use work_engine_claude_host::{driver, fixture, relay, snapshot};

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn need(args: &[String], name: &str) -> String {
    flag(args, name).unwrap_or_else(|| {
        eprintln!("missing {name}");
        std::process::exit(64)
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("run") => {
            let job_path = PathBuf::from(need(&args, "--job"));
            let out = PathBuf::from(need(&args, "--out"));
            let job: driver::Job = match std::fs::read(&job_path)
                .map_err(|e| e.to_string())
                .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
            {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("job: {e}");
                    std::process::exit(64)
                }
            };
            let mut server =
                match snapshot::SnapshotServer::load(&job.snapshot.id, &job.snapshot.root) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("snapshot: {e}");
                        std::process::exit(64)
                    }
                };
            match driver::run(&job, &out, &mut server) {
                Ok(c) => {
                    let summary = serde_json::json!({"eligible":c.eligible,"outcome":c.outcome,
                        "requests":c.requests.len(),"forwarded":c.payloads.len(),"calls":c.calls.len(),
                        "published":server.published.len(),"reasons":c.reasons});
                    println!("{summary}");
                    std::process::exit(if c.eligible { 0 } else { 2 })
                }
                Err(e) => {
                    eprintln!("run: {e}");
                    std::process::exit(1)
                }
            }
        }
        Some("mcp-relay") => {
            let r = relay::run(
                &PathBuf::from(need(&args, "--socket")),
                &PathBuf::from(need(&args, "--tools")),
                &PathBuf::from(need(&args, "--log")),
            );
            if let Err(e) = r {
                eprintln!("relay: {e}");
                std::process::exit(1)
            }
        }
        Some("fixture-upstream") => {
            let script = fixture::Script {
                snapshot_id: need(&args, "--snapshot-id"),
                member_id: need(&args, "--member"),
                length: need(&args, "--length").parse().unwrap_or(1),
                tool_name: flag(&args, "--tool-name")
                    .unwrap_or_else(|| "mcp__we__snapshot_read".into()),
            };
            let port: u16 = need(&args, "--port").parse().unwrap_or(0);
            match fixture::Fixture::start(port, script) {
                Ok(f) => {
                    println!("{{\"port\":{}}}", f.port);
                    loop {
                        std::thread::park();
                    }
                }
                Err(e) => {
                    eprintln!("fixture: {e}");
                    std::process::exit(1)
                }
            }
        }
        _ => {
            eprintln!(
                "usage: work-engine-claude-host run --job <job.json> --out <dir> | mcp-relay --socket <s> --tools <t> --log <l> | fixture-upstream --port <p> --snapshot-id <id> --member <m> --length <n> [--tool-name <name>]"
            );
            std::process::exit(64)
        }
    }
}
