//! Session-owned read-only client of the pinned session-events MCP owner.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawFd,
        process::CommandExt,
    },
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

// Approved query owner and its accepted disabled coding-job consumer extension (d44b1bbf).
// Keep existing operator profiles valid; each invocation still verifies its selected exact pin.
const OWNER_SHAS: &[&str] = &[
    "470a625b76d17a62a68659709d99d2767c944441be4cd2ddbff96bc6be12a12d",
    "933eda1c2af1f3f76819fbe2099269c6a04c33fcdb8adf52969cdf80f5beeefd",
];
const HELPER: &str = include_str!("../owner.py");
const OWNER_LIMIT: usize = 512 * 1024;
pub const INPUT_LIMIT: usize = 256 * 1024;
const OUTPUT_LIMIT: usize = 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeProfile {
    pub binary: String,
    pub binary_sha256: String,
    pub config: String,
    pub config_sha256: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextProfile {
    pub budget_root: Option<String>,
    pub role_ledger: Option<String>,
    pub ledger_path: Option<String>,
    pub ledger_sha256: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub version: u8,
    pub owner: String,
    pub owner_sha256: String,
    pub codex_home: String,
    pub claude_home: String,
    pub state_dir: String,
    pub bridge: Option<BridgeProfile>,
    pub context: Option<ContextProfile>,
}
impl Profile {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != 1
            || !OWNER_SHAS.contains(&self.owner_sha256.as_str())
            || [
                &self.owner,
                &self.codex_home,
                &self.claude_home,
                &self.state_dir,
            ]
            .iter()
            .any(|s| !Path::new(s).is_absolute())
        {
            return Err("invalid_profile");
        }
        if let Some(c) = &self.context
            && ([&c.budget_root, &c.role_ledger, &c.ledger_path]
                .into_iter()
                .flatten()
                .any(|s| !Path::new(s).is_absolute())
                || c.ledger_path.is_some() != c.ledger_sha256.is_some()
                || c.ledger_sha256.as_ref().is_some_and(|s| {
                    s.len() != 64
                        || !s
                            .bytes()
                            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                }))
        {
            return Err("invalid_profile");
        }
        if let Some(b) = &self.bridge
            && (!Path::new(&b.binary).is_absolute()
                || !Path::new(&b.config).is_absolute()
                || ![&b.binary_sha256, &b.config_sha256].iter().all(|s| {
                    s.len() == 64
                        && s.bytes()
                            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                }))
        {
            return Err("invalid_profile");
        }
        Ok(())
    }
}

// The enum makes the read-only surface explicit; no mutation variant exists.
pub struct QueryArgs(Value);
pub enum Query {
    ListSessions(QueryArgs),
    ReadSession(QueryArgs),
    ReadOperation(QueryArgs),
    ReadResult(QueryArgs),
}
fn uuid(v: &Value) -> bool {
    v.as_str().is_some_and(|s| {
        s.len() == 36
            && s.bytes().enumerate().all(|(i, c)| {
                if [8, 13, 18, 23].contains(&i) {
                    c == b'-'
                } else {
                    c.is_ascii_digit() || (b'a'..=b'f').contains(&c)
                }
            })
    })
}
fn num(v: &Value, min: u64, max: u64) -> bool {
    v.as_u64().is_some_and(|n| n >= min && n <= max)
}
impl Query {
    pub fn parse(name: &str, args: Value) -> Result<Self, &'static str> {
        let fields: &[&str] = match name {
            "list_sessions" => &["query", "host", "limit"],
            "read_session" => &[
                "session_id",
                "entry_id",
                "before",
                "after",
                "tail",
                "roles",
                "text_offset",
                "max_chars",
            ],
            "read_session_operation" => &["request_id", "grant_credential"],
            "read_session_result" => {
                &["request_id", "grant_credential", "text_offset", "max_chars"]
            }
            _ => return Err("invalid_query"),
        };
        let a = args.as_object().ok_or("invalid_arguments")?;
        if a.keys().any(|k| !fields.contains(&k.as_str())) {
            return Err("invalid_arguments");
        }
        for (k, v) in a {
            let valid = match k.as_str() {
                "session_id" | "request_id" => uuid(v),
                "grant_credential" => v.as_str().is_some_and(|s| !s.is_empty() && s.len() <= 4096),
                "query" => v.as_str().is_some_and(|s| s.len() <= 4096),
                "host" => matches!(v.as_str(), Some("codex" | "claude")),
                "limit" => num(v, 1, 100),
                "before" | "after" => num(v, 0, 50),
                "tail" => num(v, 1, 50),
                "text_offset" => num(v, 0, 1_u64 << 62),
                "max_chars" => {
                    if name == "read_session" {
                        num(v, 100, 50000)
                    } else {
                        num(v, 1, 16000)
                    }
                }
                "entry_id" => v.as_str().is_some_and(|s| {
                    s.len() <= 40 && {
                        let mut pieces = s.split('.');
                        let first = pieces.next().unwrap_or("");
                        let second = pieces.next();
                        !first.is_empty()
                            && first.bytes().all(|x| x.is_ascii_digit())
                            && second.is_none_or(|part| {
                                !part.is_empty() && part.bytes().all(|x| x.is_ascii_digit())
                            })
                            && pieces.next().is_none()
                    }
                }),
                "roles" => v.as_array().is_some_and(|a| {
                    a.len() <= 8
                        && a.iter().all(|r| {
                            matches!(
                                r.as_str(),
                                Some(
                                    "user"
                                        | "assistant"
                                        | "tool_call"
                                        | "tool_result"
                                        | "reasoning"
                                        | "system"
                                        | "agent_message"
                                        | "turn_end"
                                )
                            )
                        })
                }),
                _ => false,
            };
            if !valid {
                return Err("invalid_arguments");
            }
        }
        match name {
            "read_session" if !a.contains_key("session_id") => Err("invalid_arguments"),
            "read_session_operation" | "read_session_result"
                if !a.contains_key("request_id") || !a.contains_key("grant_credential") =>
            {
                Err("invalid_arguments")
            }
            "list_sessions" => Ok(Self::ListSessions(QueryArgs(args))),
            "read_session" => Ok(Self::ReadSession(QueryArgs(args))),
            "read_session_operation" => Ok(Self::ReadOperation(QueryArgs(args))),
            "read_session_result" => Ok(Self::ReadResult(QueryArgs(args))),
            _ => Err("invalid_query"),
        }
    }
    fn parts(&self) -> (&'static str, &Value) {
        match self {
            Self::ListSessions(v) => ("list_sessions", &v.0),
            Self::ReadSession(v) => ("read_session", &v.0),
            Self::ReadOperation(v) => ("read_session_operation", &v.0),
            Self::ReadResult(v) => ("read_session_result", &v.0),
        }
    }
}

/// Read bounded bytes through one no-follow regular-file handle; private files must belong to this user.
pub fn read_file(path: &str, limit: usize, private: bool) -> Result<Vec<u8>, &'static str> {
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| "file_unavailable")?;
    let m = f.metadata().map_err(|_| "file_unavailable")?;
    if !m.is_file()
        || m.len() > limit as u64
        || (private && (m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0))
    {
        return Err("invalid_file");
    }
    let mut b = Vec::new();
    f.take(limit as u64 + 1)
        .read_to_end(&mut b)
        .map_err(|_| "file_unavailable")?;
    if b.len() > limit {
        return Err("input_too_large");
    }
    Ok(b)
}

pub fn query(profile: &Profile, query: &Query) -> Result<Value, &'static str> {
    profile.validate()?;
    let (name, args) = query.parts();
    Query::parse(name, args.clone())?; // Public enum values cannot bypass validation.
    if matches!(query, Query::ReadOperation(_) | Query::ReadResult(_)) && profile.bridge.is_none() {
        return Err("bridge_unavailable");
    }
    let owner = read_file(&profile.owner, OWNER_LIMIT, false).map_err(|_| "owner_unavailable")?;
    if format!("{:x}", Sha256::digest(&owner)) != profile.owner_sha256 {
        return Err("owner_pin_mismatch");
    }
    let source = String::from_utf8(owner).map_err(|_| "owner_unavailable")?;
    let payload = serde_json::to_vec(
        &json!({"profile":profile,"name":name,"arguments":args,"owner_source":source}),
    )
    .map_err(|_| "invalid_arguments")?;
    if payload.len() > OWNER_LIMIT + INPUT_LIMIT {
        return Err("input_too_large");
    }
    transport(&payload, Duration::from_secs(35), HELPER)
}
fn transport(payload: &[u8], timeout: Duration, program: &str) -> Result<Value, &'static str> {
    let mut child = Command::new("/usr/bin/python3")
        .args(["-I", "-c", program])
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "owner_unavailable")?;
    let mut stdin = child.stdin.take();
    let mut stdout = child.stdout.take().unwrap();
    let set_nonblock = |fd| unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) } == 0;
    if !set_nonblock(stdin.as_ref().unwrap().as_raw_fd()) || !set_nonblock(stdout.as_raw_fd()) {
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        let _ = child.wait();
        return Err("owner_unavailable");
    }
    let deadline = Instant::now() + timeout;
    let mut sent = 0;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 16384];
    let mut drain = |stdout: &mut std::process::ChildStdout| -> Result<(), &'static str> {
        loop {
            match stdout.read(&mut chunk) {
                Ok(0) => return Ok(()),
                Ok(n) => {
                    bytes.extend_from_slice(&chunk[..n]);
                    if bytes.len() > OUTPUT_LIMIT {
                        return Err("output_too_large");
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err("owner_unavailable"),
            }
        }
    };
    let status = loop {
        if let Err(e) = drain(&mut stdout) {
            break Err(e);
        }
        if let Some(pipe) = stdin.as_mut() {
            match pipe.write(&payload[sent..]) {
                Ok(n) => {
                    sent += n;
                    if sent == payload.len() {
                        stdin.take();
                    }
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) => {}
                Err(_) => break Err("owner_unavailable"),
            }
        }
        // Observe exit without reaping: the process-group id cannot be reused before cleanup.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let observed = unsafe {
            libc::waitid(
                libc::P_PID,
                child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if observed != 0 {
            break Err("owner_unavailable");
        }
        if unsafe { info.si_pid() } != 0 {
            break if sent == payload.len()
                && info.si_code == libc::CLD_EXITED
                && unsafe { info.si_status() } == 0
            {
                Ok(())
            } else {
                Err("owner_unavailable")
            };
        }
        if Instant::now() >= deadline {
            break Err("timeout");
        }
        thread::sleep(Duration::from_millis(10));
    };
    // Kill the owned group while its leader is still unreaped. No background I/O workers exist.
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
    let final_drain = drain(&mut stdout);
    status?;
    final_drain?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "owner_unavailable")?;
    if let Some(reason) = value.get("error").and_then(Value::as_str) {
        return Err(match reason {
            "query_rejected" => "query_rejected",
            "unauthorized" => "unauthorized",
            "history_unavailable" => "history_unavailable",
            "bridge_unavailable" => "bridge_unavailable",
            "identity_conflict" => "identity_conflict",
            "expired" => "expired",
            "target_mismatch" => "target_mismatch",
            "custody_lost" => "custody_lost",
            _ => "owner_unavailable",
        });
    }
    value.get("result").cloned().ok_or("owner_unavailable")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_approved_owner_pins_preserve_profile_compatibility() {
        let mut profile = Profile {
            version: 1,
            owner: "/private/owner.py".into(),
            owner_sha256: String::new(),
            codex_home: "/private/codex".into(),
            claude_home: "/private/claude".into(),
            state_dir: "/private/events".into(),
            bridge: None,
            context: None,
        };
        for pin in OWNER_SHAS {
            profile.owner_sha256 = (*pin).into();
            assert!(profile.validate().is_ok());
        }
        profile.owner_sha256 = "a".repeat(64);
        assert_eq!(profile.validate(), Err("invalid_profile"));
    }

    #[test]
    fn protocol_limits_and_cleanup() {
        assert_eq!(
            transport(
                b"{}",
                Duration::from_millis(150),
                "import time; time.sleep(10)"
            ),
            Err("timeout")
        );
        assert_eq!(
            transport(
                b"{}",
                Duration::from_secs(3),
                "import sys; sys.stdout.write('x' * 1100000); sys.stdout.flush()"
            ),
            Err("output_too_large")
        );
        assert_eq!(
            transport(b"{}", Duration::from_secs(3), "raise Exception('secret')"),
            Err("owner_unavailable")
        );
        // A child retaining stdout must not keep the caller or its I/O thread alive.
        assert_eq!(
            transport(
                b"{}",
                Duration::from_secs(3),
                "import os,time,sys; p=os.fork(); (time.sleep(10) if p==0 else (print('\"ok\"'),sys.stdout.flush()))"
            ),
            Err("owner_unavailable")
        );
        assert_eq!(
            transport(
                b"{}",
                Duration::from_secs(3),
                "print('{\"result\":{\"value\":1}}')"
            )
            .unwrap()["value"],
            1
        );
    }
    #[test]
    fn strict_queries() {
        for (name, args) in [
            ("submit_session_request", json!({})),
            ("read_session", json!({"session_id":"bad"})),
            (
                "read_session",
                json!({"session_id":"11111111-1111-4111-8111-111111111111","tail":true}),
            ),
            (
                "read_session_result",
                json!({"request_id":"11111111-1111-4111-8111-111111111111","grant_credential":"x","max_chars":0}),
            ),
        ] {
            assert!(Query::parse(name, args).is_err());
        }
    }
}
