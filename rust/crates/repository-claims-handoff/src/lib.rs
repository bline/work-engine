//! Bounded exact claim handoff: construction, recovery from an owner checkpoint
//! and verification through the existing `work-engine-server claims-stdio`
//! read profile.
//!
//! A handoff is a selection made by someone else. It names exact claim,
//! revision and revision-digest identities (and optionally evidence artifact
//! digests). It never selects a newest revision, never assesses applicability
//! and carries no authority. Trusted server and read-profile paths are supplied
//! by the caller and are never read from a handoff or checkpoint. Recovered
//! text is data and is never executed.
use claim_evidence::{
    codec::{JsValue, canonical_json, digest, exact_fields, parse_json},
    sha256_bytes,
};
use serde_json::{Value, json};
use std::{
    fmt, fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

pub const SCHEMA: &str = "repository-claims-handoff-v1";
/// A handoff occupies exactly one line of tool output, beginning with this marker.
pub const MARKER: &str = "CLAIMS_HANDOFF_V1 ";
pub const MAX_HANDOFF_BYTES: usize = 4096;
pub const MAX_REFS: usize = 4;
pub const MAX_EVIDENCE_PER_REF: usize = 4;
pub const MAX_TEXT_BYTES: usize = 256;
/// Owner-checkpoint prompt cap for coding recovery (lifecycle-store native.rs).
pub const MAX_CHECKPOINT_BYTES: usize = 65_536;
pub const MAX_PROJECTION_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_EVIDENCE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_PAGES: usize = 4096;
/// One response frame, newline included. The reader rejects a longer frame itself.
const MAX_RESPONSE_LINE: usize = 1024 * 1024;
static LIVE_READERS: AtomicUsize = AtomicUsize::new(0);
/// Reader threads currently alive in this process (test observation only).
#[doc(hidden)]
pub fn live_reader_threads() -> usize {
    LIVE_READERS.load(Ordering::SeqCst)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    code: &'static str,
    detail: String,
}
impl Error {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
    /// Stable refusal class; callers must not fall back to another selection.
    pub fn code(&self) -> &'static str {
        self.code
    }
    pub fn detail(&self) -> &str {
        &self.detail
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}
impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, Error>;

fn is_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn is_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value
            .bytes()
            .all(|b| b.is_ascii_graphic() && b != b'"' && b != b'\\')
}
fn is_text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_TEXT_BYTES
        && !value.chars().any(char::is_control)
}

/// One exact selection. All fields are explicit; none is derived or defaulted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactRef {
    pub claim_id: String,
    pub revision_id: String,
    pub revision_sha256: String,
    /// Evidence artifact digests (as named by the revision's
    /// `evidence_references[].integrity_sha256`) the selector wants read.
    pub evidence_sha256: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    /// Who made the selection (data; not authenticated here).
    pub selected_by: String,
    /// Why these exact revisions were selected (data; not assessed here).
    pub selection_basis: String,
    pub refs: Vec<ExactRef>,
}

impl Handoff {
    pub fn new(selected_by: &str, selection_basis: &str, refs: Vec<ExactRef>) -> Result<Self> {
        let h = Self {
            selected_by: selected_by.into(),
            selection_basis: selection_basis.into(),
            refs,
        };
        h.validate()?;
        Ok(h)
    }
    fn validate(&self) -> Result<()> {
        let bad = |d: &str| Err(Error::new("invalid_handoff", d));
        if !is_text(&self.selected_by) || !is_text(&self.selection_basis) {
            return bad("selected_by and selection_basis must be bounded plain text");
        }
        if self.refs.is_empty() || self.refs.len() > MAX_REFS {
            return bad("between 1 and 4 exact references required");
        }
        for (i, r) in self.refs.iter().enumerate() {
            if !is_identity(&r.claim_id)
                || !is_identity(&r.revision_id)
                || !is_sha(&r.revision_sha256)
            {
                return bad(
                    "claim_id, revision_id and revision_sha256 must be exact bounded identities",
                );
            }
            if r.evidence_sha256.len() > MAX_EVIDENCE_PER_REF
                || r.evidence_sha256.iter().any(|e| !is_sha(e))
                || r.evidence_sha256
                    .iter()
                    .enumerate()
                    .any(|(j, e)| r.evidence_sha256[..j].contains(e))
            {
                return bad(
                    "evidence digests must be distinct SHA-256 values, at most 4 per reference",
                );
            }
            if self.refs[..i]
                .iter()
                .any(|p| p.revision_id == r.revision_id && p.claim_id == r.claim_id)
            {
                return bad("duplicate exact reference");
            }
        }
        Ok(())
    }
    fn value(&self) -> JsValue {
        let refs = self
            .refs
            .iter()
            .map(|r| {
                JsValue::object([
                    ("claim_id", JsValue::text(&r.claim_id)),
                    ("revision_id", JsValue::text(&r.revision_id)),
                    ("revision_sha256", JsValue::text(&r.revision_sha256)),
                    (
                        "evidence_sha256",
                        JsValue::Array(
                            r.evidence_sha256.iter().map(|e| JsValue::text(e)).collect(),
                        ),
                    ),
                ])
            })
            .collect();
        JsValue::object([
            ("schema", JsValue::text(SCHEMA)),
            ("selection", JsValue::text("exact")),
            ("applicability", JsValue::text("not_assessed")),
            ("selected_by", JsValue::text(&self.selected_by)),
            ("selection_basis", JsValue::text(&self.selection_basis)),
            ("refs", JsValue::Array(refs)),
        ])
    }
    /// Canonical JSON (claim-evidence codec).
    pub fn canonical(&self) -> Result<String> {
        self.validate()?;
        let mut text = canonical_json(&self.value())
            .map_err(|e| Error::new("invalid_handoff", e.to_string()))?;
        // The native codec terminates its output with a newline; a handoff is one line.
        if text.ends_with('\n') {
            text.pop();
        }
        if text.len() > MAX_HANDOFF_BYTES {
            return Err(Error::new("invalid_handoff", "handoff exceeds 4096 bytes"));
        }
        Ok(text)
    }
    /// The single tool-output line carrying this handoff.
    pub fn line(&self) -> Result<String> {
        Ok(format!("{MARKER}{}", self.canonical()?))
    }
    /// Parse the canonical JSON body (without marker). Strict: closed fields,
    /// exact literals, and canonical re-encoding must equal the input.
    pub fn parse(body: &str) -> Result<Self> {
        let bad = |d: &str| Error::new("invalid_handoff", d.to_string());
        if body.len() > MAX_HANDOFF_BYTES {
            return Err(bad("handoff exceeds 4096 bytes"));
        }
        let v = parse_json(body).map_err(|e| Error::new("invalid_handoff", e.to_string()))?;
        let obj = v.as_object().map_err(|_| bad("object required"))?;
        let keys: Vec<String> = obj
            .keys()
            .map(|k| k.to_string_checked().unwrap_or_default())
            .collect();
        let expected = [
            "applicability",
            "refs",
            "schema",
            "selected_by",
            "selection",
            "selection_basis",
        ];
        if keys != expected {
            return Err(bad("closed field set required"));
        }
        let text = |k: &str| -> Result<String> {
            v.get(k)
                .and_then(|x| x.as_text().ok())
                .and_then(|s| s.to_string_checked().ok())
                .ok_or_else(|| bad("text field required"))
        };
        if text("schema")? != SCHEMA
            || text("selection")? != "exact"
            || text("applicability")? != "not_assessed"
        {
            return Err(bad("schema, selection and applicability literals required"));
        }
        let mut refs = Vec::new();
        for item in v
            .get("refs")
            .and_then(|x| x.as_array().ok())
            .ok_or_else(|| bad("refs array required"))?
        {
            let o = item
                .as_object()
                .map_err(|_| bad("reference object required"))?;
            let ks: Vec<String> = o
                .keys()
                .map(|k| k.to_string_checked().unwrap_or_default())
                .collect();
            if ks
                != [
                    "claim_id",
                    "evidence_sha256",
                    "revision_id",
                    "revision_sha256",
                ]
            {
                return Err(bad("closed reference field set required"));
            }
            let t = |k: &str| -> Result<String> {
                item.get(k)
                    .and_then(|x| x.as_text().ok())
                    .and_then(|s| s.to_string_checked().ok())
                    .ok_or_else(|| bad("reference text required"))
            };
            let mut evidence = Vec::new();
            for e in item
                .get("evidence_sha256")
                .and_then(|x| x.as_array().ok())
                .ok_or_else(|| bad("evidence array required"))?
            {
                evidence.push(
                    e.as_text()
                        .ok()
                        .and_then(|s| s.to_string_checked().ok())
                        .ok_or_else(|| bad("evidence digest text required"))?,
                );
            }
            refs.push(ExactRef {
                claim_id: t("claim_id")?,
                revision_id: t("revision_id")?,
                revision_sha256: t("revision_sha256")?,
                evidence_sha256: evidence,
            });
        }
        let h = Self::new(&text("selected_by")?, &text("selection_basis")?, refs)?;
        if h.canonical()? != body {
            return Err(bad("handoff is not in canonical form"));
        }
        Ok(h)
    }
}

fn collect_strings<'a>(v: &'a Value, depth: usize, out: &mut Vec<&'a str>) -> Result<()> {
    if depth > 16 {
        return Err(Error::new(
            "invalid_checkpoint",
            "coding_result nesting exceeds bound",
        ));
    }
    match v {
        Value::String(s) => out.push(s),
        Value::Array(a) => a
            .iter()
            .try_for_each(|x| collect_strings(x, depth + 1, out))?,
        Value::Object(m) => m
            .values()
            .try_for_each(|x| collect_strings(x, depth + 1, out))?,
        _ => {}
    }
    Ok(())
}

/// Recover the handoff from an owner checkpoint (the exact JSON the coding
/// continuity owner stages and replays as the sole recovered prompt). The
/// handoff must be exactly one distinct marker line in the exact Host
/// `coding_result`. Absence or ambiguity refuses; there is no fallback.
pub fn extract_from_checkpoint(checkpoint: &str) -> Result<Handoff> {
    if checkpoint.len() > MAX_CHECKPOINT_BYTES {
        return Err(Error::new(
            "invalid_checkpoint",
            "checkpoint exceeds owner cap",
        ));
    }
    let v: Value = serde_json::from_str(checkpoint)
        .map_err(|_| Error::new("invalid_checkpoint", "checkpoint is not JSON"))?;
    let result = v
        .get("coding_result")
        .ok_or_else(|| Error::new("invalid_checkpoint", "coding_result absent"))?;
    let mut strings = Vec::new();
    collect_strings(result, 0, &mut strings)?;
    let mut lines: Vec<&str> = Vec::new();
    for s in strings {
        for line in s.lines() {
            if line.starts_with(MARKER) && !lines.contains(&line) {
                lines.push(line);
            }
        }
    }
    match lines.as_slice() {
        [] => Err(Error::new(
            "handoff_missing",
            "no handoff line in the checkpoint coding_result",
        )),
        [line] => Handoff::parse(&line[MARKER.len()..]),
        _ => Err(Error::new(
            "handoff_ambiguous",
            "more than one distinct handoff line",
        )),
    }
}

/// Recover the owner checkpoint text from the continuity owner's result file
/// (`work-engine.pi-coding-continuity-result.v1`, whose `checkpoint` field is the
/// exact string staged as the recovered prompt). The file is data; its schema
/// literal and a bounded string field are the only things read.
pub fn checkpoint_from_continuity_result(result: &str) -> Result<String> {
    if result.len() > 4 * MAX_CHECKPOINT_BYTES {
        return Err(Error::new(
            "invalid_checkpoint",
            "continuity result exceeds bound",
        ));
    }
    let v: Value = serde_json::from_str(result)
        .map_err(|_| Error::new("invalid_checkpoint", "continuity result is not JSON"))?;
    if v["schema"] != "work-engine.pi-coding-continuity-result.v1" {
        return Err(Error::new(
            "invalid_checkpoint",
            "unexpected continuity result schema",
        ));
    }
    v["checkpoint"].as_str().map(str::to_owned).ok_or_else(|| {
        Error::new(
            "invalid_checkpoint",
            "checkpoint absent from continuity result",
        )
    })
}

/// Trusted, externally supplied read capability. Digests are expectations
/// supplied by the operator, not values read from the handoff.
#[derive(Debug, Clone)]
pub struct TrustedServer {
    pub executable: PathBuf,
    pub executable_sha256: String,
    pub read_profile: PathBuf,
    pub read_profile_sha256: String,
}
#[derive(Debug, Clone)]
pub struct Options {
    /// Page size requested from the server (4..=32768). Smaller values exercise
    /// framed pagination; the digest verification is identical.
    pub page_bytes: usize,
    pub deadline: Duration,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            page_bytes: 32768,
            deadline: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRead {
    pub sha256: String,
    pub total_bytes: usize,
    pub pages: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRef {
    pub claim_id: String,
    pub revision_id: String,
    pub revision_sha256: String,
    pub projection_sha256: String,
    pub projection_bytes: usize,
    pub projection_pages: usize,
    /// Whether the selected historical revision is still the claim's head. A
    /// false value is normal and is not a refusal. Read from the verified
    /// native projection, never defaulted.
    pub current_head: bool,
    pub head_revision_ids: Vec<String>,
    pub evidence: Vec<EvidenceRead>,
    /// The complete native `repository_fact_projection_v1` exactly as the owner
    /// produced it (proposition, limitations, authority, admission, provenance,
    /// reliances and baseline included). It is owner data: this crate adds no
    /// summary and draws no conclusion from it. `projection_sha256` is the digest
    /// of its canonical claim-evidence encoding (including the codec's trailing
    /// newline); at most `MAX_PROJECTION_BYTES`.
    pub owner_projection: Value,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// Selector data from the handoff; not authenticated or assessed here.
    pub selected_by: String,
    pub selection_basis: String,
    pub refs: Vec<ResolvedRef>,
}
impl Resolution {
    /// Report JSON. Applicability and source currency stay `not_assessed`.
    pub fn report(&self) -> Value {
        json!({
            "schema": "repository-claims-handoff-resolution-v2",
            "status": "resolved",
            "applicability": "not_assessed",
            "source_currency": "not_assessed",
            "semantic_acceptance": "not_assessed",
            "selection": {
                "selected_by": self.selected_by,
                "basis": self.selection_basis,
                "authentication": "unauthenticated_descriptive_data",
            },
            "refs": self.refs.iter().map(|r| json!({
                "claim_id": r.claim_id, "revision_id": r.revision_id, "revision_sha256": r.revision_sha256,
                "projection_sha256": r.projection_sha256, "projection_bytes": r.projection_bytes,
                "projection_pages": r.projection_pages, "current_head": r.current_head,
                "head_revision_ids": r.head_revision_ids,
                "verification": {
                    "projection_digest_matches_assembled_pages": true,
                    "canonical_reencoding_equals_bytes": true,
                    "request_binding_recomputed": true,
                    "revision_digest_recomputed": true,
                    "native_envelope_fields_present": true,
                    "authority_not_authenticated_here": true,
                },
                "owner_projection": r.owner_projection,
                "evidence": r.evidence.iter().map(|e| json!({"sha256": e.sha256, "total_bytes": e.total_bytes, "pages": e.pages})).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })
    }
}

enum Frame {
    Line(String),
    Oversized,
    Io,
}
struct ReaderGuard;
impl Drop for ReaderGuard {
    fn drop(&mut self) {
        LIVE_READERS.fetch_sub(1, Ordering::SeqCst);
    }
}
/// Read response frames from the server. One outstanding RPC needs one frame; a
/// second may wait inside the reader. A full queue blocks the reader, which stops
/// draining the pipe and so blocks the server: transport memory is bounded by about
/// two frames of `MAX_RESPONSE_LINE` plus the pipe, whatever the server emits. An
/// oversized frame is rejected here, before it is queued. Dropping the receiver
/// releases a reader blocked on a full queue.
fn spawn_reader<R: Read + Send + 'static>(out: R) -> (mpsc::Receiver<Frame>, JoinHandle<()>) {
    let (tx, lines) = mpsc::sync_channel(1);
    LIVE_READERS.fetch_add(1, Ordering::SeqCst);
    let reader = std::thread::spawn(move || {
        let _guard = ReaderGuard;
        let mut reader = BufReader::new(out);
        loop {
            let mut line = String::new();
            let frame = match reader
                .by_ref()
                .take(MAX_RESPONSE_LINE as u64 + 1)
                .read_line(&mut line)
            {
                Ok(0) => break,
                Ok(_) if line.len() > MAX_RESPONSE_LINE => Frame::Oversized,
                Ok(_) => Frame::Line(line),
                Err(_) => Frame::Io,
            };
            let last = !matches!(frame, Frame::Line(_));
            if tx.send(frame).is_err() || last {
                break;
            }
        }
    });
    (lines, reader)
}

struct Session {
    child: Child,
    input: Option<ChildStdin>,
    lines: Option<mpsc::Receiver<Frame>>,
    reader: Option<JoinHandle<()>>,
    deadline: Instant,
    sequence: u64,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.input.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
        // Dropping the receiver wakes a reader blocked on a full queue; the dead
        // child's closed pipe wakes one blocked on a read.
        self.lines.take();
        if let Some(reader) = self.reader.take() {
            let end = Instant::now() + Duration::from_secs(2);
            while !reader.is_finished() && Instant::now() < end {
                std::thread::sleep(Duration::from_millis(5));
            }
            if reader.is_finished() {
                let _ = reader.join();
            }
        }
    }
}
impl Session {
    fn start(server: &TrustedServer, deadline: Duration) -> Result<Self> {
        let unavailable = |d: &str| Error::new("server_unavailable", d.to_string());
        let exe = fs::read(&server.executable)
            .map_err(|_| unavailable("server executable unreadable"))?;
        if !is_sha(&server.executable_sha256) || sha256_bytes(&exe) != server.executable_sha256 {
            return Err(Error::new(
                "server_untrusted",
                "server executable digest differs from the trusted pin",
            ));
        }
        drop(exe);
        let profile =
            fs::read(&server.read_profile).map_err(|_| unavailable("read profile unreadable"))?;
        if !is_sha(&server.read_profile_sha256)
            || sha256_bytes(&profile) != server.read_profile_sha256
        {
            return Err(Error::new(
                "profile_untrusted",
                "read profile digest differs from the trusted pin",
            ));
        }
        let mut child = Command::new(&server.executable)
            .arg("claims-stdio")
            .arg(&server.read_profile)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| unavailable("server did not start"))?;
        let input = child.stdin.take();
        let Some(out) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(unavailable("server stdout unavailable"));
        };
        let (lines, reader) = spawn_reader(out);
        let mut s = Self {
            child,
            input,
            lines: Some(lines),
            reader: Some(reader),
            deadline: Instant::now() + deadline,
            sequence: 0,
        };
        let init = s.rpc("initialize", json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"repository-claims-handoff","version":"1"}}))?;
        if init["result"]["protocolVersion"] != "2025-06-18" {
            return Err(Error::new("protocol", "unexpected MCP protocol version"));
        }
        Ok(s)
    }
    fn rpc(&mut self, method: &str, params: Value) -> Result<Value> {
        self.sequence += 1;
        let id = self.sequence;
        let input = self
            .input
            .as_mut()
            .ok_or_else(|| Error::new("server_unavailable", "server input closed"))?;
        writeln!(
            input,
            "{}",
            json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
        )
        .and_then(|_| input.flush())
        .map_err(|_| Error::new("server_unavailable", "server closed input"))?;
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        let receiver = self
            .lines
            .as_ref()
            .ok_or_else(|| Error::new("server_unavailable", "server output closed"))?;
        let line = match receiver.recv_timeout(remaining) {
            Ok(Frame::Line(line)) => line,
            Ok(Frame::Oversized) => {
                return Err(Error::new("protocol", "response line exceeds bound"));
            }
            Ok(Frame::Io) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(Error::new(
                    "server_unavailable",
                    "server closed before responding",
                ));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err(Error::new("server_unavailable", "server response deadline"));
            }
        };
        let v: Value = serde_json::from_str(&line)
            .map_err(|_| Error::new("protocol", "response is not JSON"))?;
        if v["id"] != id {
            return Err(Error::new("protocol", "response id mismatch"));
        }
        if v.get("error").is_some() {
            return Err(Error::new(
                "owner_refused",
                "server returned a protocol error",
            ));
        }
        Ok(v)
    }
    fn tool(&mut self, name: &str, args: Value) -> Result<Value> {
        let v = self.rpc("tools/call", json!({"name":name,"arguments":args}))?;
        let r = &v["result"];
        if r["isError"] != false {
            return Err(Error::new(
                "owner_refused",
                format!(
                    "{name} refused: {}",
                    r["content"][0]["text"]
                        .as_str()
                        .unwrap_or("no detail")
                        .chars()
                        .take(160)
                        .collect::<String>()
                ),
            ));
        }
        let owner = &r["structuredContent"]["owner_result"];
        if !owner.is_object() {
            return Err(Error::new("protocol", "owner_result absent"));
        }
        Ok(owner.clone())
    }
}

fn uint(v: &Value, what: &str) -> Result<usize> {
    v.as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| {
            Error::new(
                "projection_corrupt",
                format!("{what} must be an unsigned integer"),
            )
        })
}

/// Read one exact projection through framed pages and verify its complete
/// digest. Returns (bytes, digest, pages).
fn read_projection(
    s: &mut Session,
    r: &ExactRef,
    page_bytes: usize,
) -> Result<(String, String, usize)> {
    let mut assembled = String::new();
    let mut digest: Option<String> = None;
    let mut total: Option<usize> = None;
    let mut pages = 0;
    loop {
        let mut page = json!({"offset": assembled.len(), "max_bytes": page_bytes});
        if let Some(d) = &digest {
            page["sha256"] = json!(d);
        }
        let o = s.tool("resolve_repository_fact", json!({"claim_id":r.claim_id,"revision_id":r.revision_id,"revision_sha256":r.revision_sha256,"projection_page":page}))?;
        pages += 1;
        let corrupt = |d: &str| Error::new("projection_corrupt", d.to_string());
        if pages > MAX_PAGES || o["encoding"] != "claim-evidence-canonical-json-v1" {
            return Err(corrupt("page bound or encoding"));
        }
        let sha = o["projection_sha256"]
            .as_str()
            .filter(|d| is_sha(d))
            .ok_or_else(|| corrupt("projection digest absent"))?;
        let text = o["canonical_projection_page"]
            .as_str()
            .ok_or_else(|| corrupt("page text absent"))?;
        let t = uint(&o["total_bytes"], "total_bytes")?;
        if digest.as_deref().is_some_and(|d| d != sha)
            || total.is_some_and(|x| x != t)
            || uint(&o["offset"], "offset")? != assembled.len()
            || text.is_empty()
            || t > MAX_PROJECTION_BYTES
            || text.len() > page_bytes
            || assembled.len() + text.len() > t
        {
            return Err(corrupt("page framing conflict"));
        }
        digest = Some(sha.into());
        total = Some(t);
        assembled.push_str(text);
        let more = o["has_more"]
            .as_bool()
            .ok_or_else(|| corrupt("has_more absent"))?;
        let next = &o["next_offset"];
        if more != (assembled.len() < t)
            || (more && uint(next, "next_offset")? != assembled.len())
            || (!more && !next.is_null())
        {
            return Err(corrupt("continuation conflict"));
        }
        if !more {
            break;
        }
    }
    let digest = digest.unwrap();
    if assembled.len() != total.unwrap() || sha256_bytes(assembled.as_bytes()) != digest {
        return Err(Error::new(
            "projection_corrupt",
            "assembled projection does not match its digest",
        ));
    }
    Ok((assembled, digest, pages))
}

fn read_evidence(s: &mut Session, sha: &str, page_bytes: usize) -> Result<EvidenceRead> {
    let corrupt = |d: &str| Error::new("evidence_corrupt", d.to_string());
    let mut bytes: Vec<u8> = Vec::new();
    let mut total: Option<usize> = None;
    let mut pages = 0;
    loop {
        let o = s.tool(
            "read_repository_evidence",
            json!({"sha256":sha,"offset":bytes.len(),"max_bytes":page_bytes}),
        )?;
        pages += 1;
        let text = o["utf8"]
            .as_str()
            .ok_or_else(|| corrupt("page text absent"))?;
        let t = uint(&o["total_bytes"], "total_bytes").map_err(|_| corrupt("total_bytes"))?;
        if pages > MAX_PAGES
            || o["artifact_sha256"] != sha
            || total.is_some_and(|x| x != t)
            || o["offset"].as_u64() != Some(bytes.len() as u64)
            || text.is_empty()
            || t > MAX_EVIDENCE_BYTES
            || text.len() > page_bytes
            || bytes.len() + text.len() > t
        {
            return Err(corrupt("page framing conflict"));
        }
        total = Some(t);
        bytes.extend_from_slice(text.as_bytes());
        let more = o["has_more"]
            .as_bool()
            .ok_or_else(|| corrupt("has_more absent"))?;
        let next = &o["next_offset"];
        if more != (bytes.len() < t)
            || (more && next.as_u64() != Some(bytes.len() as u64))
            || (!more && !next.is_null())
        {
            return Err(corrupt("continuation conflict"));
        }
        if !more {
            break;
        }
    }
    if Some(bytes.len()) != total || sha256_bytes(&bytes) != sha {
        return Err(corrupt("assembled evidence does not match its digest"));
    }
    Ok(EvidenceRead {
        sha256: sha.into(),
        total_bytes: bytes.len(),
        pages,
    })
}

fn text_of(v: Option<&JsValue>) -> Option<String> {
    v?.as_text().ok()?.to_string_checked().ok()
}

fn nested_mismatch(d: &str) -> Error {
    Error::new("revision_mismatch", d.to_string())
}
fn nested_text(v: &JsValue, k: &str) -> Result<String> {
    text_of(v.get(k))
        .filter(|t| !t.is_empty())
        .ok_or_else(|| nested_mismatch(&format!("{k} must be non-empty text")))
}
/// Native evidence/baseline/authority reference (claim-evidence `reference` schema).
fn reference_record(v: &JsValue) -> Result<()> {
    exact_fields(
        v,
        &[
            "freshness",
            "integrity_sha256",
            "owner",
            "reference",
            "revision",
            "status",
        ],
        "reference",
    )
    .map_err(|e| nested_mismatch(&e.to_string()))?;
    for k in ["freshness", "owner", "reference", "revision", "status"] {
        nested_text(v, k)?;
    }
    if !is_sha(&nested_text(v, "integrity_sha256")?) {
        return Err(nested_mismatch(
            "reference integrity_sha256 must be a SHA-256",
        ));
    }
    Ok(())
}
/// Native authority grant record: structure and types only. Whether the grant is
/// authentic or sufficient stays with the claims owner.
fn authority_record(v: &JsValue) -> Result<()> {
    exact_fields(
        v,
        &[
            "actor",
            "authority_reference",
            "decision_scope",
            "grant_id",
            "permissions",
            "profile",
            "schema_version",
        ],
        "authority",
    )
    .map_err(|e| nested_mismatch(&e.to_string()))?;
    for k in ["actor", "decision_scope", "grant_id", "profile"] {
        nested_text(v, k)?;
    }
    if v.get("schema_version") != Some(&JsValue::Number(1.0))
        || !v.get("permissions").is_some_and(|p| {
            p.as_array()
                .is_ok_and(|a| a.iter().all(|t| text_of(Some(t)).is_some()))
        })
    {
        return Err(nested_mismatch(
            "authority schema_version or permissions invalid",
        ));
    }
    reference_record(v.get("authority_reference").unwrap_or(&JsValue::Null))
}

struct VerifiedProjection {
    listed: Vec<String>,
    heads: Vec<String>,
    current_head: bool,
    owner_projection: Value,
}

/// Bind the assembled projection bytes to the exact selection using the native
/// claim-evidence codec. The outer projection digest comes from the same
/// response as the bytes, so it proves page assembly only; this proves the
/// returned content is the selected revision, in canonical form, carrying the
/// native envelope, admission, authority and head metadata. Missing or mistyped
/// material refuses; nothing is defaulted. It does not authenticate authority or
/// judge truth, applicability or currency (those stay with their owners).
fn verify_projection(text: &str, r: &ExactRef) -> Result<VerifiedProjection> {
    let corrupt = |d: &str| Error::new("projection_corrupt", d.to_string());
    let mismatch = |d: &str| Error::new("revision_mismatch", d.to_string());
    let projection = parse_json(text).map_err(|e| corrupt(&e.to_string()))?;
    if canonical_json(&projection).map_err(|e| corrupt(&e.to_string()))? != text {
        return Err(corrupt("projection is not the canonical encoding"));
    }
    let fields = |v: &JsValue, names: &[&str], label: &str| -> Result<()> {
        exact_fields(v, names, label).map_err(|e| mismatch(&e.to_string()))
    };
    let get = |v: &JsValue, k: &str| -> Result<JsValue> {
        v.get(k)
            .cloned()
            .ok_or_else(|| mismatch(&format!("{k} absent")))
    };
    let lit = |v: &JsValue, k: &str, want: &str| -> Result<()> {
        if text_of(v.get(k)).as_deref() == Some(want) {
            Ok(())
        } else {
            Err(mismatch(&format!("{k} is not {want}")))
        }
    };
    let name = |v: &JsValue, k: &str| -> Result<String> {
        text_of(v.get(k))
            .filter(|t| !t.is_empty())
            .ok_or_else(|| mismatch(&format!("{k} must be non-empty text")))
    };
    let hash = |v: &JsValue, k: &str| -> Result<String> {
        text_of(v.get(k))
            .filter(|t| is_sha(t))
            .ok_or_else(|| mismatch(&format!("{k} must be a SHA-256")))
    };
    let objects = |v: &JsValue, k: &str| -> Result<Vec<JsValue>> {
        let items = get(v, k)?
            .as_array()
            .map(<[JsValue]>::to_vec)
            .map_err(|_| mismatch(&format!("{k} must be an array")))?;
        if items.iter().any(|i| i.as_object().is_err()) {
            return Err(mismatch(&format!("{k} entries must be objects")));
        }
        Ok(items)
    };
    fields(
        &projection,
        &[
            "schema_version",
            "kind",
            "repository_id",
            "decision_scope",
            "applicability",
            "source_currency",
            "projection",
            "items",
            "next_cursor",
            "limitations",
        ],
        "projection",
    )?;
    if get(&projection, "schema_version")? != JsValue::Number(1.0) {
        return Err(mismatch("projection schema_version must be 1"));
    }
    lit(&projection, "kind", "repository_fact_projection_v1")?;
    lit(&projection, "applicability", "not_assessed")?;
    lit(&projection, "source_currency", "not_assessed")?;
    name(&projection, "repository_id")?;
    name(&projection, "decision_scope")?;
    if get(&projection, "next_cursor")? != JsValue::Null {
        return Err(mismatch("exact resolution has no continuation cursor"));
    }
    let limitations = get(&projection, "limitations")?;
    if limitations.as_array().map_or(true, |l| {
        l.is_empty()
            || l.iter()
                .any(|t| text_of(Some(t)).is_none_or(|t| t.is_empty()))
    }) {
        return Err(mismatch("limitations must be non-empty text entries"));
    }
    let envelope = get(&projection, "projection")?;
    fields(
        &envelope,
        &[
            "build_version",
            "canonical_input_sha256",
            "canonical_state_sha256",
            "root_id",
            "bootstrap_config_sha256",
            "bootstrap_source_sha256",
            "store_revision",
            "request_sha256",
            "freshness",
            "completeness",
        ],
        "projection envelope",
    )?;
    lit(&envelope, "build_version", "claim-evidence-repository-v1")?;
    lit(
        &envelope,
        "freshness",
        "current_relative_to_canonical_input",
    )?;
    lit(&envelope, "completeness", "available")?;
    name(&envelope, "root_id")?;
    for k in [
        "canonical_input_sha256",
        "canonical_state_sha256",
        "bootstrap_config_sha256",
        "bootstrap_source_sha256",
    ] {
        hash(&envelope, k)?;
    }
    if !matches!(get(&envelope, "store_revision")?, JsValue::Number(n) if n >= 0.0 && n.fract() == 0.0)
    {
        return Err(mismatch("store_revision must be a non-negative integer"));
    }
    let request = JsValue::object([
        ("claim_id", JsValue::text(&r.claim_id)),
        ("revision_id", JsValue::text(&r.revision_id)),
        ("revision_sha256", JsValue::text(&r.revision_sha256)),
    ]);
    if hash(&envelope, "request_sha256")?
        != digest(&request).map_err(|e| corrupt(&e.to_string()))?
    {
        return Err(mismatch("projection is not bound to the selected request"));
    }

    let items = get(&projection, "items")?;
    let [item] = items
        .as_array()
        .map_err(|_| mismatch("items must be an array"))?
    else {
        return Err(mismatch("exact resolution must return exactly one item"));
    };
    fields(
        item,
        &[
            "admitted_operation",
            "claim",
            "revision",
            "revision_sha256",
            "authority",
            "creation_authority",
            "current_head",
            "head_revision_ids",
            "unresolved_references",
            "reliances",
            "applicability",
        ],
        "projection item",
    )?;
    lit(item, "applicability", "not_assessed")?;
    let claim = get(item, "claim")?;
    let revision = get(item, "revision")?;
    if text_of(claim.get("id")).as_deref() != Some(&r.claim_id)
        || text_of(revision.get("id")).as_deref() != Some(&r.revision_id)
        || text_of(revision.get("claim_id")).as_deref() != Some(&r.claim_id)
        || text_of(item.get("revision_sha256")).as_deref() != Some(&r.revision_sha256)
    {
        return Err(mismatch(
            "returned revision is not the selected exact revision",
        ));
    }
    if digest(&revision).map_err(|e| corrupt(&e.to_string()))? != r.revision_sha256 {
        return Err(mismatch(
            "returned revision content does not hash to the selected digest",
        ));
    }
    for k in ["proposition", "decision_scope"] {
        name(&revision, k)?;
    }
    let authority = get(item, "authority")?;
    let creation = get(item, "creation_authority")?;
    for g in [&authority, &creation] {
        authority_record(g)?;
    }
    if authority.get("grant_id") != revision.get("authority_ref")
        || creation.get("grant_id") != claim.get("authority_ref")
    {
        return Err(mismatch(
            "authority records do not match the revision and claim authority references",
        ));
    }
    exact_fields(
        &claim,
        &[
            "authority_ref",
            "created_by",
            "id",
            "profile",
            "schema_version",
            "statement_identity",
            "subject",
        ],
        "claim",
    )
    .map_err(|e| mismatch(&e.to_string()))?;
    for k in ["created_by", "profile", "statement_identity"] {
        name(&claim, k)?;
    }
    let subject = get(&claim, "subject")?;
    fields(
        &subject,
        &[
            "content_set",
            "evidence_baseline",
            "namespace",
            "stable_subject_id",
            "subject_kind",
        ],
        "claim subject",
    )?;
    for k in ["namespace", "stable_subject_id", "subject_kind"] {
        name(&subject, k)?;
    }
    if get(&subject, "content_set")?
        .as_array()
        .map_or(true, |c| c.iter().any(|t| text_of(Some(t)).is_none()))
    {
        return Err(mismatch("claim subject content_set must be text entries"));
    }
    reference_record(&get(&subject, "evidence_baseline")?)?;
    for k in ["evidence_references", "sensitivity_references"] {
        for r in objects(&revision, k)? {
            reference_record(&r)?;
        }
    }
    for r in objects(item, "unresolved_references")? {
        reference_record(&r)?;
    }
    let admitted = |v: &JsValue, result_id: &str, label: &str| -> Result<()> {
        fields(v, &["operation", "binding", "request"], label)?;
        let operation = get(v, "operation")?;
        fields(
            &operation,
            &[
                "action",
                "authority_ref",
                "operation_id",
                "payload_sha256",
                "result_identity",
            ],
            "admitted operation",
        )?;
        if text_of(operation.get("result_identity")).as_deref() != Some(result_id) {
            return Err(mismatch(
                "admitted operation is not for the returned record",
            ));
        }
        for k in ["action", "authority_ref", "operation_id"] {
            name(&operation, k)?;
        }
        hash(&operation, "payload_sha256")?;
        let binding = get(v, "binding")?;
        fields(
            &binding,
            &[
                "authority_ref_sha256",
                "decision_scope",
                "operation_id",
                "producer",
                "repository_id",
                "request_sha256",
                "root_id",
            ],
            "admission binding",
        )?;
        for k in [
            "decision_scope",
            "operation_id",
            "producer",
            "repository_id",
            "root_id",
        ] {
            name(&binding, k)?;
        }
        for k in ["authority_ref_sha256", "request_sha256"] {
            hash(&binding, k)?;
        }
        // The request payload and expected_state are owner-defined opaque values; they must
        // be present (expected_state may be null) and the envelope closed.
        let request = get(v, "request")?;
        fields(
            &request,
            &[
                "action",
                "expected_state",
                "operation_id",
                "payload",
                "profile",
                "schema_version",
            ],
            "admission request",
        )?;
        for k in ["action", "operation_id", "profile"] {
            name(&request, k)?;
        }
        if get(&request, "schema_version")? != JsValue::Number(1.0)
            || get(&request, "payload")?.as_object().is_err()
        {
            return Err(mismatch(
                "admission request schema_version or payload invalid",
            ));
        }
        let id = name(&operation, "operation_id")?;
        if name(&binding, "operation_id")? != id
            || name(&request, "operation_id")? != id
            || name(&request, "action")? != name(&operation, "action")?
        {
            return Err(mismatch("admission operation identities disagree"));
        }
        Ok(())
    };
    admitted(
        &get(item, "admitted_operation")?,
        &r.revision_id,
        "admitted_operation",
    )?;
    for reliance in objects(item, "reliances")? {
        fields(
            &reliance,
            &[
                "admitted_operation",
                "record",
                "sha256",
                "attributed_producer",
                "authority",
                "attestation",
            ],
            "reliance",
        )?;
        let record = get(&reliance, "record")?;
        let rid = name(&record, "id")?;
        if text_of(record.get("claim_revision_id")).as_deref() != Some(&r.revision_id)
            || hash(&reliance, "sha256")? != digest(&record).map_err(|e| corrupt(&e.to_string()))?
        {
            return Err(mismatch("reliance is not bound to the selected revision"));
        }
        let rel_authority = get(&reliance, "authority")?;
        authority_record(&rel_authority)?;
        if rel_authority.get("grant_id") != record.get("authority_ref")
            || text_of(reliance.get("attributed_producer")).as_deref()
                != text_of(rel_authority.get("actor")).as_deref()
            || text_of(reliance.get("attributed_producer")).is_none()
        {
            return Err(mismatch(
                "reliance authority and attributed producer are not consistent",
            ));
        }
        lit(
            &reliance,
            "attestation",
            "producer_attributed_not_direct_consumer_attestation",
        )?;
        admitted(
            &get(&reliance, "admitted_operation")?,
            &rid,
            "reliance admitted_operation",
        )?;
    }

    let mut heads = Vec::new();
    for h in get(item, "head_revision_ids")?
        .as_array()
        .map_err(|_| mismatch("head_revision_ids must be an array"))?
    {
        heads.push(
            text_of(Some(h))
                .filter(|t| is_identity(t))
                .ok_or_else(|| mismatch("head_revision_ids entries must be identities"))?,
        );
    }
    if heads.is_empty() || heads.windows(2).any(|w| w[0] >= w[1]) {
        return Err(mismatch(
            "head_revision_ids must be non-empty, sorted and distinct",
        ));
    }
    let JsValue::Bool(current_head) = get(item, "current_head")? else {
        return Err(mismatch("current_head must be a boolean"));
    };
    if current_head != heads.contains(&r.revision_id) {
        return Err(mismatch("current_head contradicts head_revision_ids"));
    }
    let mut listed = Vec::new();
    for reference in objects(&revision, "evidence_references")? {
        listed.push(hash(&reference, "integrity_sha256")?);
    }
    let owner_projection: Value = serde_json::from_str(text)
        .map_err(|_| corrupt("projection is not representable as JSON for the report"))?;
    Ok(VerifiedProjection {
        listed,
        heads,
        current_head,
        owner_projection,
    })
}

/// Resolve every selected reference exactly. Any failure refuses the whole
/// handoff; nothing is substituted, retried or repaired, and the client holds
/// only a read capability.
pub fn resolve_handoff(
    handoff: &Handoff,
    server: &TrustedServer,
    options: &Options,
) -> Result<Resolution> {
    handoff.validate()?;
    if !(4..=32768).contains(&options.page_bytes) {
        return Err(Error::new(
            "invalid_options",
            "page_bytes must be within 4..=32768",
        ));
    }
    let mut s = Session::start(server, options.deadline)?;
    let tools = s.rpc("tools/list", json!({}))?;
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .map(|a| a.iter().filter_map(|t| t["name"].as_str()).collect())
        .unwrap_or_default();
    if !names.contains(&"resolve_repository_fact") {
        return Err(Error::new(
            "server_unavailable",
            "resolve_repository_fact not offered",
        ));
    }
    let mut refs = Vec::new();
    for r in &handoff.refs {
        let (text, projection_sha256, projection_pages) =
            read_projection(&mut s, r, options.page_bytes)?;
        let verified = verify_projection(&text, r)?;
        let (listed, heads, current_head) =
            (verified.listed, verified.heads, verified.current_head);
        let mut evidence = Vec::new();
        for e in &r.evidence_sha256 {
            if !listed.contains(e) {
                return Err(Error::new(
                    "evidence_not_listed",
                    "selected evidence digest is not an evidence reference of the selected revision",
                ));
            }
            if !names.contains(&"read_repository_evidence") {
                return Err(Error::new(
                    "evidence_unavailable",
                    "server profile offers no evidence reads",
                ));
            }
            evidence.push(read_evidence(&mut s, e, options.page_bytes)?);
        }
        refs.push(ResolvedRef {
            claim_id: r.claim_id.clone(),
            revision_id: r.revision_id.clone(),
            revision_sha256: r.revision_sha256.clone(),
            projection_sha256,
            projection_bytes: text.len(),
            projection_pages,
            current_head,
            head_revision_ids: heads,
            evidence,
            owner_projection: verified.owner_projection,
        });
    }
    Ok(Resolution {
        selected_by: handoff.selected_by.clone(),
        selection_basis: handoff.selection_basis.clone(),
        refs,
    })
}

/// Hex SHA-256 helper for operators pinning an executable or profile.
pub fn file_sha256(path: &Path) -> Result<String> {
    fs::read(path)
        .map(|b| sha256_bytes(&b))
        .map_err(|_| Error::new("server_unavailable", "file unreadable"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::AtomicU64};

    /// Never-ending source of 64 KiB frames that counts the bytes pulled from it.
    struct Endless {
        pulled: Arc<AtomicU64>,
        line: Vec<u8>,
        at: usize,
    }
    impl Read for Endless {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let n = buf.len().min(self.line.len() - self.at);
            buf[..n].copy_from_slice(&self.line[self.at..self.at + n]);
            self.at = (self.at + n) % self.line.len();
            self.pulled.fetch_add(n as u64, Ordering::SeqCst);
            Ok(n)
        }
    }

    #[test]
    fn unconsumed_flood_is_backpressured_and_dropping_the_receiver_releases_the_reader() {
        let mut line = vec![b'x'; 65_535];
        line.push(b'\n');
        let pulled = Arc::new(AtomicU64::new(0));
        let (frames, reader) = spawn_reader(Endless {
            pulled: pulled.clone(),
            line,
            at: 0,
        });
        // The consumer reads nothing. An unbounded queue would pull gigabytes in this time.
        std::thread::sleep(Duration::from_millis(300));
        let held = pulled.load(Ordering::SeqCst);
        assert!(held > 0, "reader should have started");
        assert!(
            held < 2 * MAX_RESPONSE_LINE as u64,
            "reader pulled {held} bytes with nothing consumed"
        );
        assert!(
            !reader.is_finished(),
            "reader waits blocked on the full queue"
        );
        let before = pulled.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(
            before,
            pulled.load(Ordering::SeqCst),
            "no progress while blocked"
        );
        assert!(matches!(frames.recv().unwrap(), Frame::Line(_)));
        drop(frames);
        let end = Instant::now() + Duration::from_secs(5);
        while !reader.is_finished() && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            reader.is_finished(),
            "dropping the receiver must release the reader"
        );
        reader.join().unwrap();
    }

    #[test]
    fn oversized_frame_is_rejected_by_the_reader_before_queueing() {
        let pulled = Arc::new(AtomicU64::new(0));
        let (frames, reader) = spawn_reader(Endless {
            pulled: pulled.clone(),
            line: vec![b'y'; 4 * MAX_RESPONSE_LINE],
            at: 0,
        });
        assert!(matches!(frames.recv().unwrap(), Frame::Oversized));
        reader.join().unwrap();
        assert!(
            frames.recv().is_err(),
            "reader stops after an oversized frame"
        );
        assert!(pulled.load(Ordering::SeqCst) < 2 * MAX_RESPONSE_LINE as u64 + 8192);
    }
}
