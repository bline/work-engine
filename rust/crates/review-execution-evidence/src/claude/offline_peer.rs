//! Feature-gated deterministic transport peer. It never represents Claude or a
//! production provider route and is absent from ordinary package builds.
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};

fn witness(label: &str) {
    if let Ok(path) = std::env::var("WORK_ENGINE_PEER_WITNESS")
        && let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path)
    {
        let _ = writeln!(file, "{label}");
    }
}

fn main() {
    witness("start");
    if std::env::var("WORK_ENGINE_OFFLINE_PEER").ok().as_deref() != Some("1") {
        std::process::exit(41);
    }
    let mut session = None;
    let mut model = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--session-id" => session = args.next(),
            "--model" => model = args.next(),
            "--effort" | "--output-format" | "--mcp-config" | "--tools" | "--json-schema" => {
                let _ = args.next();
            }
            "--strict-mcp-config" | "--dangerously-skip-permissions" => {}
            "-p" => {}
            _ => std::process::exit(42),
        }
    }
    let Some(session) = session else {
        std::process::exit(43)
    };
    let Some(model) = model else {
        std::process::exit(44)
    };
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).unwrap();
    if !input.is_empty() {
        witness("prompt");
    }
    let prompt: Value = serde_json::from_slice(&input).unwrap();
    if let Ok(project) = std::env::var("WORK_ENGINE_CBM_PROJECT") {
        let mut stream = UnixStream::connect("/cbm/socket/analysis.sock").unwrap();
        let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"index_status","arguments":{"project":project,
                "format":"json","diagnostics":"none"}}});
        writeln!(stream, "{request}").unwrap();
        let mut response = String::new();
        BufReader::new(stream).read_line(&mut response).unwrap();
        if response.is_empty() {
            std::process::exit(46);
        }
    }
    let mode = prompt
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("success");
    match mode {
        "timeout" => std::thread::sleep(Duration::from_secs(10)),
        "exit" => std::process::exit(7),
        "malformed" => {
            print!("not JSON");
            return;
        }
        "duplicate" => {
            print!("{{\"type\":\"result\",\"type\":\"result\"}}");
            return;
        }
        "oversized" => {
            print!("{}", "X".repeat(5 * 1024 * 1024));
            return;
        }
        "held_pipe" => {
            hold_pipe_after_parent_exit();
            return;
        }
        "leak" => {
            print!(
                "{}",
                std::env::var("CLAUDE_CODE_OAUTH_TOKEN").unwrap_or_default()
            );
            return;
        }
        "oversized_session" => {
            let home = PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap());
            let error = fs::write(home.join("too-large.bin"), vec![b'X'; 20 * 1024 * 1024])
                .expect_err("bounded session write must fail");
            std::process::exit(if error.raw_os_error() == Some(libc::ENOSPC) {
                72
            } else {
                73
            });
        }
        "sparse_session" => {
            let home = PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap());
            let file = fs::File::create(home.join("sparse.bin")).unwrap();
            file.set_len(1024 * 1024 * 1024).unwrap();
        }
        "fifo_session" => {
            let home = PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap());
            let name =
                std::ffi::CString::new(home.join("waiting.fifo").as_os_str().as_encoded_bytes())
                    .unwrap();
            assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        }
        "inode_growth" => {
            let home = PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap());
            for index in 0..1_000_000u32 {
                match fs::File::create(home.join(format!("empty-{index:08}"))) {
                    Ok(_) => {}
                    Err(error) => {
                        std::process::exit(if error.raw_os_error() == Some(libc::ENOSPC) {
                            76
                        } else {
                            77
                        })
                    }
                }
            }
            std::process::exit(78);
        }
        "hostile_write" => {
            // These two destinations bypassed the charged session mount when
            // Bubblewrap's synthetic root and /dev were left writable.
            for path in ["/escape", "/dev/escape"] {
                if fs::write(path, b"unbounded").is_ok() {
                    std::process::exit(74);
                }
            }
            let quota = fs::read("/quota/memory.max").unwrap();
            if fs::write("/quota/memory.max", quota).is_ok() {
                std::process::exit(74);
            }
        }
        _ => {}
    }
    let home = PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap());
    fs::write(
        home.join("session.json"),
        json!({"session_id":session,"turn":1}).to_string(),
    )
    .unwrap();
    if mode == "nested" {
        fs::create_dir_all(home.join("history/turn")).unwrap();
        fs::write(home.join("history/turn/state.json"), b"{\"turn\":1}").unwrap();
    }
    let returned_session = if mode == "wrong_session" {
        "wrong-session"
    } else {
        &session
    };
    let returned_model = if mode == "wrong_model" {
        "unaccepted-model"
    } else {
        &model
    };
    let mut usage = serde_json::Map::new();
    usage.insert(
        returned_model.into(),
        json!({"inputTokens":1,"outputTokens":1}),
    );
    if mode == "mixed_model" {
        usage.insert("other-model".into(), json!({"inputTokens":1}));
    }
    let mut result = prompt.get("result").cloned().unwrap_or(Value::Null);
    if mode == "wrong_subject" {
        result["subject"]["commit"] = json!("different");
    }
    if mode == "noncanonical_result" {
        result["verdict"] = json!("invalid-verdict");
    }
    let output = json!({"type":"result","subtype":if mode == "error_route" {"error"} else {"success"},"session_id":returned_session,
        "model":returned_model,"modelUsage":usage,"structured_output":result});
    println!("{output}");
}

/// Deliberately orphan one bounded child so stdout stays open after the peer
/// exits; the holder exits in one second and is reaped by the process init.
#[allow(clippy::zombie_processes)]
fn hold_pipe_after_parent_exit() {
    let _ = std::process::Command::new("sleep")
        .arg("1")
        .stdout(std::process::Stdio::inherit())
        .spawn()
        .unwrap();
}
