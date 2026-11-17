//! Loopback recording and enforcing proxy between Claude Code and the
//! Anthropic API. Every forwarded request body is persisted create-only, with
//! its digest in the ledger, before any upstream byte is sent. The
//! `Authorization` value passes through and is never written.

use crate::http::{self, Head};
use crate::spec;
use crate::sse;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const ANTHROPIC_HOST: &str = "api.anthropic.com";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Upstream {
    Anthropic,
    /// Loopback fixture port; offline qualification only.
    Fixture(u16),
}

#[derive(Clone, Debug)]
pub struct Config {
    pub dir: PathBuf,
    pub model: String,
    pub expected_tools: Vec<Value>,
    pub max_requests: u32,
    pub max_body: usize,
    pub max_response: usize,
    pub upstream: Upstream,
}

/// One ledger record per request Claude Code sent to the proxy.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct RequestRecord {
    pub n: u32,
    pub path: String,
    pub body_sha256: Option<String>,
    pub body_len: usize,
    /// Non-secret header provenance; credential headers appear by name only.
    pub headers: Vec<(String, String)>,
    pub refused: Option<String>,
    pub forwarded: bool,
    pub status: Option<u16>,
    pub response_sha256: Option<String>,
    pub response_len: usize,
    pub message_id: Option<String>,
    pub stop_reason: Option<String>,
    pub tool_uses: Vec<(String, String, Value)>,
    pub complete: bool,
    pub failure: Option<String>,
}

pub struct Proxy {
    pub port: u16,
    pub ledger: Arc<Mutex<Vec<RequestRecord>>>,
    stop: Arc<AtomicBool>,
}

const KEEP_VALUES: &[&str] = &[
    "user-agent",
    "anthropic-version",
    "anthropic-beta",
    "x-app",
    "x-client-app",
    "content-type",
];
const HOP: &[&str] = &[
    "host",
    "connection",
    "keep-alive",
    "transfer-encoding",
    "content-length",
    "accept-encoding",
    "proxy-connection",
    "proxy-authorization",
    "te",
    "upgrade",
    "trailer",
];

impl Proxy {
    pub fn start(config: Config) -> std::io::Result<Proxy> {
        fs::create_dir_all(config.dir.join("requests"))?;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let ledger = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let tls = match config.upstream {
            Upstream::Anthropic => Some(tls_config()?),
            Upstream::Fixture(_) => None,
        };
        let shared = Arc::new(Shared {
            config,
            ledger: ledger.clone(),
            tls,
        });
        let s = stop.clone();
        std::thread::spawn(move || {
            for conn in listener.incoming() {
                if s.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(conn) = conn else { continue };
                let shared = shared.clone();
                std::thread::spawn(move || {
                    let _ = serve(&shared, conn);
                });
            }
        });
        Ok(Proxy { port, ledger, stop })
    }

    pub fn records(&self) -> Vec<RequestRecord> {
        self.ledger.lock().map(|l| l.clone()).unwrap_or_default()
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
    }
}

struct Shared {
    config: Config,
    ledger: Arc<Mutex<Vec<RequestRecord>>>,
    tls: Option<Arc<rustls::ClientConfig>>,
}

fn tls_config() -> std::io::Result<Arc<rustls::ClientConfig>> {
    use rustls_platform_verifier::ConfigVerifierExt;
    rustls::ClientConfig::with_platform_verifier()
        .map(Arc::new)
        .map_err(|e| std::io::Error::other(e.to_string()))
}

fn serve(shared: &Shared, conn: TcpStream) -> std::io::Result<()> {
    conn.set_read_timeout(Some(Duration::from_secs(120)))?;
    let mut reader = BufReader::new(conn.try_clone()?);
    let mut client = conn;
    let Some(head) = http::read_head(&mut reader)? else {
        return Ok(());
    };
    let mut parts = head.start.split(' ');
    let method = parts.next().unwrap_or("").to_owned();
    let target = parts.next().unwrap_or("").to_owned();
    let n = {
        let mut l = shared.ledger.lock().map_err(|_| http::invalid("ledger"))?;
        let n = l.len() as u32 + 1;
        l.push(RequestRecord {
            n,
            path: target.clone(),
            headers: provenance(&head),
            ..Default::default()
        });
        n
    };
    let refuse = |client: &mut TcpStream, status: u16, reason: &str| -> std::io::Result<()> {
        update(shared, n, |r| r.refused = Some(reason.to_owned()));
        respond_error(client, status, reason)
    };
    let path = target.split('?').next().unwrap_or("");
    if method != "POST" || path != "/v1/messages" {
        return refuse(&mut client, 404, "only POST /v1/messages is forwarded");
    }
    if head.get("transfer-encoding").is_some() {
        return refuse(&mut client, 411, "chunked request bodies are refused");
    }
    let len = head
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    if len > shared.config.max_body {
        return refuse(&mut client, 413, "request body exceeds the job bound");
    }
    let body = http::read_body(&mut reader, len, shared.config.max_body)?;
    let digest = spec::sha256_hex(&body);
    update(shared, n, |r| {
        r.body_sha256 = Some(digest.clone());
        r.body_len = body.len();
    });
    // Retain every request body, refused ones included, create-only.
    persist(&shared.config.dir, &format!("{n:04}.body"), &body)?;
    let value: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => return refuse(&mut client, 400, "request body is not JSON"),
    };
    if value["model"] != shared.config.model {
        return refuse(&mut client, 400, "request model differs from the job model");
    }
    if let Err(reason) = spec::check_request_tools(&shared.config.expected_tools, &value["tools"]) {
        return refuse(&mut client, 400, reason);
    }
    let forwarded_before = shared
        .ledger
        .lock()
        .map(|l| l.iter().filter(|r| r.forwarded).count() as u32)
        .unwrap_or(u32::MAX);
    if forwarded_before >= shared.config.max_requests {
        return refuse(&mut client, 400, "request count bound reached");
    }
    update(shared, n, |r| r.forwarded = true);
    append_ledger(
        &shared.config.dir,
        &json!({"event":"forward","n":n,"body_sha256":digest,"body_len":body.len()}),
    )?;
    let result = forward(shared, n, &head, &target, &body, &mut client);
    if let Err(e) = &result {
        update(shared, n, |r| r.failure = Some(e.to_string()));
    }
    let record = shared
        .ledger
        .lock()
        .ok()
        .and_then(|l| l.iter().find(|r| r.n == n).cloned());
    if let Some(record) = record {
        append_ledger(
            &shared.config.dir,
            &json!({"event":"settle","record":record}),
        )?;
    }
    let _ = client.shutdown(Shutdown::Both);
    result
}

fn forward(
    shared: &Shared,
    n: u32,
    head: &Head,
    target: &str,
    body: &[u8],
    client: &mut TcpStream,
) -> std::io::Result<()> {
    let (host, mut upstream): (String, Box<dyn ReadWrite>) =
        match (&shared.config.upstream, &shared.tls) {
            (Upstream::Anthropic, Some(tls)) => {
                let tcp = TcpStream::connect((ANTHROPIC_HOST, 443))?;
                tcp.set_read_timeout(Some(Duration::from_secs(600)))?;
                let name = rustls::pki_types::ServerName::try_from(ANTHROPIC_HOST)
                    .map_err(|_| http::invalid("server name"))?;
                let conn = rustls::ClientConnection::new(tls.clone(), name)
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                (
                    ANTHROPIC_HOST.to_owned(),
                    Box::new(rustls::StreamOwned::new(conn, tcp)),
                )
            }
            (Upstream::Fixture(port), _) => {
                let tcp = TcpStream::connect(("127.0.0.1", *port))?;
                tcp.set_read_timeout(Some(Duration::from_secs(600)))?;
                (format!("127.0.0.1:{port}"), Box::new(tcp))
            }
            _ => return Err(http::invalid("upstream not configured")),
        };
    let mut out = format!("POST {target} HTTP/1.1\r\nHost: {host}\r\n");
    for (k, v) in &head.headers {
        if !HOP.contains(&k.to_ascii_lowercase().as_str()) {
            out.push_str(&format!("{k}: {v}\r\n"));
        }
    }
    out.push_str(&format!(
        "Accept-Encoding: identity\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    upstream.write_all(out.as_bytes())?;
    upstream.write_all(body)?;
    upstream.flush()?;
    let mut reader = BufReader::new(upstream);
    let resp =
        http::read_head(&mut reader)?.ok_or_else(|| http::invalid("no upstream response"))?;
    let status: u16 = resp
        .start
        .split(' ')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| http::invalid("upstream status"))?;
    update(shared, n, |r| r.status = Some(status));
    let mut reply = format!("HTTP/1.1 {status} {}\r\n", reason(status));
    for (k, v) in &resp.headers {
        let lower = k.to_ascii_lowercase();
        if !HOP.contains(&lower.as_str()) && lower != "set-cookie" {
            reply.push_str(&format!("{k}: {v}\r\n"));
        }
    }
    reply.push_str("Transfer-Encoding: chunked\r\nConnection: close\r\n\r\n");
    client.write_all(reply.as_bytes())?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(
        shared
            .config
            .dir
            .join("requests")
            .join(format!("{n:04}.response")),
    )?;
    let mut retained = Vec::new();
    let max = shared.config.max_response;
    let copied = {
        let mut sink = |piece: &[u8]| -> std::io::Result<()> {
            file.write_all(piece)?;
            retained.extend_from_slice(piece);
            http::write_chunk(client, piece)
        };
        if resp
            .get("transfer-encoding")
            .is_some_and(|v| v.eq_ignore_ascii_case("chunked"))
        {
            http::read_chunked(&mut reader, max, &mut sink)
        } else {
            let len = resp.get("content-length").and_then(|v| v.parse().ok());
            http::read_plain(&mut reader, len, max, &mut sink)
        }
    };
    file.sync_all()?;
    let parsed = sse::parse(&retained);
    let digest = spec::sha256_hex(&retained);
    update(shared, n, |r| {
        r.response_sha256 = Some(digest.clone());
        r.response_len = retained.len();
        r.message_id = parsed.message_id.clone();
        r.stop_reason = parsed.stop_reason.clone();
        r.tool_uses = parsed
            .tool_uses
            .iter()
            .map(|t| (t.id.clone(), t.name.clone(), t.input.clone()))
            .collect();
        r.complete = copied.is_ok() && status == 200 && parsed.complete;
    });
    // The record is settled before the client sees the end of the stream.
    let _ = client.write_all(b"0\r\n\r\n");
    let _ = client.flush();
    copied.map(|_| ())
}

trait ReadWrite: std::io::Read + Write + Send {}
impl<T: std::io::Read + Write + Send> ReadWrite for T {}

fn update(shared: &Shared, n: u32, f: impl FnOnce(&mut RequestRecord)) {
    if let Ok(mut l) = shared.ledger.lock()
        && let Some(r) = l.iter_mut().find(|r| r.n == n)
    {
        f(r);
    }
}

fn provenance(head: &Head) -> Vec<(String, String)> {
    head.headers
        .iter()
        .map(|(k, v)| {
            let lower = k.to_ascii_lowercase();
            if KEEP_VALUES.contains(&lower.as_str()) {
                (k.clone(), v.clone())
            } else {
                (k.clone(), String::new())
            }
        })
        .collect()
}

fn persist(dir: &Path, name: &str, bytes: &[u8]) -> std::io::Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join("requests").join(name))?;
    f.write_all(bytes)?;
    f.sync_all()
}

fn append_ledger(dir: &Path, value: &Value) -> std::io::Result<()> {
    let mut f: File = OpenOptions::new()
        .append(true)
        .create(true)
        .open(dir.join("requests").join("ledger.jsonl"))?;
    writeln!(f, "{value}")?;
    f.sync_all()
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        411 => "Length Required",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        529 => "Overloaded",
        _ => "Status",
    }
}

fn respond_error(client: &mut TcpStream, status: u16, message: &str) -> std::io::Result<()> {
    let body = json!({"type":"error","error":{"type":"invalid_request_error",
        "message":format!("work-engine claude host refused: {message}")}})
    .to_string();
    write!(
        client,
        "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        reason(status),
        body.len()
    )?;
    client.flush()?;
    let _ = client.shutdown(Shutdown::Both);
    Ok(())
}
