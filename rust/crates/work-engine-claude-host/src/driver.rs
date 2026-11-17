//! One bounded Claude Code turn: proxy, relay socket, child process, tool
//! service, capture.

use crate::capture::{self, Capture, RelayCall};
use crate::proxy::{self, Proxy, Upstream};
use crate::spec;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub const JOB_SCHEMA: &str = "work-engine.claude-host-job.v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub max_requests: u32,
    pub max_body: usize,
    pub max_response: usize,
    pub deadline_secs: u64,
    pub max_tool_calls: u32,
}

impl Bounds {
    fn validate(&self) -> Result<(), String> {
        if !(1..=64).contains(&self.max_requests)
            || !(1024..=8 * 1024 * 1024).contains(&self.max_body)
            || !(1024..=64 * 1024 * 1024).contains(&self.max_response)
            || !(10..=3600).contains(&self.deadline_secs)
            || !(1..=256).contains(&self.max_tool_calls)
        {
            return Err("job bounds out of range".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotRef {
    pub id: String,
    pub root: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub schema: String,
    pub claude: PathBuf,
    #[serde(default)]
    pub claude_sha256: Option<String>,
    pub model: String,
    pub system_prompt: String,
    pub prompt: String,
    pub snapshot: SnapshotRef,
    pub upstream: Upstream,
    pub bounds: Bounds,
}

impl Job {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != JOB_SCHEMA
            || self.model.is_empty()
            || self.model.len() > 128
            || self.system_prompt.is_empty()
            || self.prompt.is_empty()
            || self.prompt.len() > 256 * 1024
            || self.system_prompt.len() > 256 * 1024
        {
            return Err("job fields invalid".into());
        }
        self.bounds.validate()?;
        if let Some(pin) = &self.claude_sha256 {
            let bytes =
                std::fs::read(&self.claude).map_err(|e| format!("claude executable: {e}"))?;
            if &spec::sha256_hex(&bytes) != pin {
                return Err("claude executable digest differs from the job pin".into());
            }
        }
        Ok(())
    }
}

/// Serves relayed tool calls. Returns the tool text and whether it is an error.
pub trait ToolServer {
    fn call(&mut self, tool: &str, arguments: &Value) -> (String, bool);
}

enum Event {
    Stdout(Vec<u8>),
    StdoutEnd,
    Tool {
        message: Value,
        reply: mpsc::Sender<Value>,
    },
}

/// Environment for Claude Code: inherited, minus every switch that could move
/// it off the subscription or out of headless defaults, plus the proxy.
fn child_env(cmd: &mut Command, port: u16) {
    for k in [
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_BASE_URL",
        "CLAUDE_CODE_USE_BEDROCK",
        "CLAUDE_CODE_USE_VERTEX",
        "CLAUDE_CODE_USE_FOUNDRY",
        "CLAUDECODE",
        "CLAUDE_CODE_ENTRYPOINT",
        "CLAUDE_CONFIG_DIR",
    ] {
        cmd.env_remove(k);
    }
    cmd.env("ANTHROPIC_BASE_URL", format!("http://127.0.0.1:{port}"))
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .env("DISABLE_AUTO_COMPACT", "1")
        .env("DISABLE_COMPACT", "1")
        .env("CLAUDE_CODE_DISABLE_AUTO_MEMORY", "1")
        .env("CLAUDE_CODE_DISABLE_CLAUDE_MDS", "1")
        .env("MAX_MCP_OUTPUT_TOKENS", "10000000");
}

pub fn run(job: &Job, out: &Path, server: &mut dyn ToolServer) -> Result<Capture, String> {
    job.validate()?;
    std::fs::create_dir(out).map_err(|e| format!("output directory must be new: {e}"))?;
    let cwd = out.join("cwd");
    std::fs::create_dir(&cwd).map_err(|e| e.to_string())?;
    let tools = spec::analysis_tools();
    let expected = spec::expected_request_tools(&tools);
    let tools_path = out.join("tools.json");
    std::fs::write(
        &tools_path,
        serde_json::to_vec_pretty(&tools).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;
    // Short private socket path: Unix socket paths are limited to 108 bytes.
    let sock_dir = std::env::temp_dir().join(format!("we-claude-host-{}", std::process::id()));
    std::fs::create_dir(&sock_dir).map_err(|e| format!("socket dir: {e}"))?;
    std::fs::set_permissions(&sock_dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| e.to_string())?;
    let sock = sock_dir.join("relay.sock");
    let listener = UnixListener::bind(&sock).map_err(|e| format!("relay socket: {e}"))?;
    let proxy = Proxy::start(proxy::Config {
        dir: out.to_path_buf(),
        model: job.model.clone(),
        expected_tools: expected.clone(),
        max_requests: job.bounds.max_requests,
        max_body: job.bounds.max_body,
        max_response: job.bounds.max_response,
        upstream: job.upstream.clone(),
    })
    .map_err(|e| format!("proxy: {e}"))?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let relay_log = out.join("relay.jsonl");
    let mcp = json!({"mcpServers":{spec::SERVER:{"type":"stdio","command":exe,
        "args":["mcp-relay","--socket",sock,"--tools",tools_path,"--log",relay_log]}}});
    let mcp_path = out.join("mcp.json");
    std::fs::write(&mcp_path, mcp.to_string()).map_err(|e| e.to_string())?;
    let allowed: Vec<String> = tools
        .iter()
        .map(|t| {
            format!(
                "mcp__{}__{}",
                spec::SERVER,
                t["name"].as_str().unwrap_or("")
            )
        })
        .collect();
    let stderr = std::fs::File::create(out.join("claude.stderr")).map_err(|e| e.to_string())?;
    let mut cmd = Command::new(&job.claude);
    cmd.current_dir(&cwd)
        .args([
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
        ])
        .args([
            "--verbose",
            "--model",
            &job.model,
            "--tools",
            "",
            "--strict-mcp-config",
        ])
        .arg("--mcp-config")
        .arg(&mcp_path)
        .arg("--allowedTools")
        .arg(allowed.join(","))
        .args(["--setting-sources", "", "--no-session-persistence"])
        .arg("--system-prompt")
        .arg(&job.system_prompt)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr))
        .process_group(0);
    child_env(&mut cmd, proxy.port);
    let started = Instant::now();
    let mut child = cmd.spawn().map_err(|e| format!("spawn claude: {e}"))?;
    let pgid = child.id() as i32;
    let user = json!({"type":"user","message":{"role":"user","content":[{"type":"text","text":job.prompt}]}});
    {
        let mut stdin = child.stdin.take().ok_or("claude stdin")?;
        writeln!(stdin, "{user}").map_err(|e| e.to_string())?;
    } // EOF after the one message: one turn.
    let (tx, rx) = mpsc::channel::<Event>();
    let stdout = child.stdout.take().ok_or("claude stdout")?;
    let t = tx.clone();
    std::thread::spawn(move || {
        let mut r = BufReader::new(stdout);
        loop {
            let mut line = Vec::new();
            match std::io::Read::take(&mut r, crate::relay::MAX_LINE as u64)
                .read_until(b'\n', &mut line)
            {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if t.send(Event::Stdout(line)).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = t.send(Event::StdoutEnd);
    });
    let t = tx.clone();
    std::thread::spawn(move || {
        // One relay process per turn; serve its connection until it closes.
        let Ok((conn, _)) = listener.accept() else {
            return;
        };
        let Ok(read_half) = conn.try_clone() else {
            return;
        };
        let mut r = BufReader::new(read_half);
        let mut w = conn;
        loop {
            let mut line = String::new();
            match std::io::Read::take(&mut r, crate::relay::MAX_LINE as u64).read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                break;
            };
            let (reply_tx, reply_rx) = mpsc::channel();
            if t.send(Event::Tool {
                message,
                reply: reply_tx,
            })
            .is_err()
            {
                break;
            }
            let Ok(answer) = reply_rx.recv() else { break };
            if writeln!(w, "{answer}").is_err() {
                break;
            }
        }
    });
    drop(tx);
    let mut stream = std::fs::File::create(out.join("stream.jsonl")).map_err(|e| e.to_string())?;
    let mut result_event: Option<Value> = None;
    let mut relay_calls: Vec<RelayCall> = Vec::new();
    let mut failure: Option<String> = None;
    let deadline = started + Duration::from_secs(job.bounds.deadline_secs);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            failure = Some("deadline".into());
            break;
        }
        match rx.recv_timeout(left) {
            Ok(Event::Stdout(line)) => {
                stream.write_all(&line).map_err(|e| e.to_string())?;
                if let Ok(v) = serde_json::from_slice::<Value>(&line)
                    && v["type"] == "result"
                {
                    result_event = Some(v);
                }
            }
            Ok(Event::StdoutEnd) => break,
            Ok(Event::Tool { message, reply }) => {
                let seq = message["seq"].as_u64().unwrap_or(0);
                let tool = message["tool"].as_str().unwrap_or("").to_owned();
                let (text, is_error) = if relay_calls.len() as u32 >= job.bounds.max_tool_calls {
                    ("tool call bound reached".to_owned(), true)
                } else {
                    server.call(&tool, &message["arguments"])
                };
                relay_calls.push(RelayCall {
                    seq,
                    tool_use_id: message["tool_use_id"].as_str().unwrap_or("").to_owned(),
                    tool,
                    arguments: message["arguments"].clone(),
                    text: text.clone(),
                    is_error,
                });
                let _ = reply.send(json!({"seq":seq,"text":text,"is_error":is_error}));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                failure = Some("deadline".into());
                break;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    stream.sync_all().map_err(|e| e.to_string())?;
    // Reap: give a finished child a moment, then kill the whole group.
    let exit = wait_or_kill(&mut child, pgid, if failure.is_some() { 0 } else { 10 });
    proxy.stop();
    let _ = std::fs::remove_file(&sock);
    let _ = std::fs::remove_dir(&sock_dir);
    let records = proxy.records();
    let capture = capture::build(
        out,
        job,
        &spec::tools_digest(&expected),
        records,
        relay_calls,
        result_event,
        failure,
        exit,
    );
    std::fs::write(
        out.join("capture.json"),
        serde_json::to_vec_pretty(&capture).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;
    Ok(capture)
}

fn wait_or_kill(child: &mut std::process::Child, pgid: i32, grace_secs: u64) -> Option<i32> {
    let until = Instant::now() + Duration::from_secs(grace_secs);
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            // Members of the group (the relay) may outlive the leader.
            unsafe { libc::kill(-pgid, libc::SIGKILL) };
            return status.code();
        }
        if Instant::now() >= until {
            unsafe { libc::kill(-pgid, libc::SIGKILL) };
            return child.wait().ok().and_then(|s| s.code());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
