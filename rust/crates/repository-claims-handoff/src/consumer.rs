//! Bounded read-only consumer API for an agent that must recover exact claim
//! facts after its context was replaced.
//!
//! The consumer holds one stdio session to the trusted `claims-stdio` server
//! (path and digest pins and the read profile come from the embedding Host, never
//! from tool arguments) and exposes four tools that mirror the server's four
//! read-only operations. It adds no claims semantics:
//! - a resolved revision is the exact selected revision; its complete native
//!   projection is assembled, digest-checked and re-verified with the same
//!   `verify_projection` used by `resolve_handoff` before any byte is returned;
//! - evidence is readable only if its digest is listed by a verified selected
//!   revision, and is digest-checked as a whole before any page is returned;
//! - nothing is replaced by "latest"; nothing is published, retired or
//!   invalidated; `applicability`, `source_currency` and
//!   `semantic_acceptance` stay `not_assessed`;
//! - an explicit [`RelianceDecision`] is a consumer statement, never inferred
//!   from reading, and carries no grant.
use super::*;
use std::collections::BTreeMap;

pub const SCHEMA_PAGE: &str = "claims-consumer-page-v1";
pub const SCHEMA_DECISION: &str = "claims-consumer-reliance-decision-v1";
pub const DECISION_MARKER: &str = "CLAIMS_RELIANCE_DECISION_V1 ";
pub const TOOL_DISCOVER: &str = "claims_discover";
pub const TOOL_RESOLVE: &str = "claims_resolve";
pub const TOOL_EVIDENCE: &str = "claims_evidence";
pub const TOOL_CURRENCY: &str = "claims_currency";
pub const TOOLS: [&str; 4] = [TOOL_DISCOVER, TOOL_RESOLVE, TOOL_EVIDENCE, TOOL_CURRENCY];
pub const MAX_PAGE_BYTES: usize = 8192;
pub const DEFAULT_PAGE_BYTES: usize = 4096;
pub const MAX_ARGUMENT_BYTES: usize = 2048;
pub const MAX_DISCOVER_LIMIT: usize = 20;
pub const MAX_CURRENCY_BYTES: usize = 16 * 1024;
/// Verified blobs retained per consumer (resolved projections + evidence +
/// discoveries); a bound on memory, not a freshness rule.
pub const MAX_CACHED_BLOBS: usize = 12;

/// Model-facing JSON schemas for the four tools. Property names are the only
/// accepted argument names; server, profile, scope and producer cannot be named.
pub fn tool_schemas() -> Value {
    let sha = json!({"type":"string","pattern":"^[0-9a-f]{64}$"});
    let ident = json!({"type":"string","minLength":1,"maxLength":200});
    let page = json!({"type":"integer","minimum":4,"maximum":MAX_PAGE_BYTES});
    let offset = json!({"type":"integer","minimum":0});
    json!([
        {"name":TOOL_DISCOVER,"description":"Read-only discovery of bounded repository fact candidates in the Host-selected claims scope. Returns the owner's native canonical JSON in pages; applicability is not assessed. Repeat with offset and blob_sha256 to page one result.",
         "parameters":{"type":"object","additionalProperties":false,"required":["query"],"properties":{"query":{"type":"string","minLength":1,"maxLength":512},"limit":{"type":"integer","minimum":1,"maximum":MAX_DISCOVER_LIMIT},"cursor":{"type":"object"},"offset":offset,"blob_sha256":sha,"max_bytes":page}}},
        {"name":TOOL_RESOLVE,"description":"Read-only exact resolution of one selected claim revision (claim_id, revision_id, revision_sha256 all required; never latest). Returns the complete native projection in digest-verified pages with head/currency fields from the owner.",
         "parameters":{"type":"object","additionalProperties":false,"required":["claim_id","revision_id","revision_sha256"],"properties":{"claim_id":ident,"revision_id":ident,"revision_sha256":sha,"offset":offset,"max_bytes":page}}},
        {"name":TOOL_EVIDENCE,"description":"Read-only paging of one evidence artifact that is listed by the exact selected revision. Bytes are verified against the digest before any page is returned.",
         "parameters":{"type":"object","additionalProperties":false,"required":["claim_id","revision_id","revision_sha256","sha256"],"properties":{"claim_id":ident,"revision_id":ident,"revision_sha256":sha,"sha256":sha,"offset":offset,"max_bytes":page}}},
        {"name":TOOL_CURRENCY,"description":"Read-only comparison of the captured source anchors of a listed evidence artifact with the Host-selected checkout. Reports equality, change or unavailability exactly as the owner does; semantic applicability stays not assessed.",
         "parameters":{"type":"object","additionalProperties":false,"required":["claim_id","revision_id","revision_sha256","sha256"],"properties":{"claim_id":ident,"revision_id":ident,"revision_sha256":sha,"sha256":sha}}}
    ])
}

struct Resolved {
    exact: ExactRef,
    text: String,
    sha256: String,
    listed: Vec<String>,
    heads: Vec<String>,
    current_head: bool,
}

pub struct ClaimsConsumer {
    session: Session,
    page_bytes: usize,
    call_deadline: Duration,
    max_calls: usize,
    calls: usize,
    names: Vec<String>,
    resolved: BTreeMap<(String, String, String), Resolved>,
    evidence: BTreeMap<String, Vec<u8>>,
    discovered: BTreeMap<String, (String, String)>,
    order: Vec<String>,
}

/// Sizes of one consumer's work, for qualification counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub calls: usize,
}

fn args_object<'a>(
    args: &'a Value,
    allowed: &[&str],
) -> Result<&'a serde_json::Map<String, Value>> {
    let bad = |d: &str| Error::new("invalid_arguments", d.to_string());
    if args.to_string().len() > MAX_ARGUMENT_BYTES {
        return Err(bad("arguments exceed bound"));
    }
    let o = args.as_object().ok_or_else(|| bad("object required"))?;
    if o.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err(bad("unknown argument"));
    }
    Ok(o)
}
fn arg_text(o: &serde_json::Map<String, Value>, k: &str) -> Result<String> {
    o.get(k)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| Error::new("invalid_arguments", format!("{k} text required")))
}
fn arg_uint(o: &serde_json::Map<String, Value>, k: &str, default: usize) -> Result<usize> {
    match o.get(k) {
        None => Ok(default),
        Some(v) => v
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| Error::new("invalid_arguments", format!("{k} unsigned integer"))),
    }
}
fn exact_of(o: &serde_json::Map<String, Value>) -> Result<ExactRef> {
    let r = ExactRef {
        claim_id: arg_text(o, "claim_id")?,
        revision_id: arg_text(o, "revision_id")?,
        revision_sha256: arg_text(o, "revision_sha256")?,
        evidence_sha256: vec![],
    };
    Handoff::new("consumer", "exact tool selection", vec![r.clone()])
        .map_err(|_| Error::new("invalid_arguments", "exact reference identities malformed"))?;
    Ok(r)
}

/// Return one page of `bytes` at a character boundary; refuses an offset past
/// the end, off a boundary, or a page size outside the bound.
/// Static page-size bound, checked before any owner I/O.
fn page_arg(o: &serde_json::Map<String, Value>) -> Result<usize> {
    let n = arg_uint(o, "max_bytes", DEFAULT_PAGE_BYTES)?;
    if !(4..=MAX_PAGE_BYTES).contains(&n) {
        return Err(Error::new(
            "invalid_arguments",
            "max_bytes outside 4..=8192",
        ));
    }
    Ok(n)
}
fn page_of(bytes: &[u8], offset: usize, max_bytes: usize) -> Result<(String, Option<usize>)> {
    if !(4..=MAX_PAGE_BYTES).contains(&max_bytes) {
        return Err(Error::new(
            "invalid_arguments",
            "max_bytes outside 4..=8192",
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| Error::new("evidence_corrupt", "artifact is not UTF-8 text"))?;
    if offset >= text.len() || !text.is_char_boundary(offset) {
        return Err(Error::new("invalid_arguments", "offset outside content"));
    }
    let mut end = (offset + max_bytes).min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    Ok((
        text[offset..end].to_string(),
        (end < text.len()).then_some(end),
    ))
}

fn page_envelope(
    tool: &str,
    blob_sha256: &str,
    total: usize,
    offset: usize,
    text: String,
    next: Option<usize>,
    extra: Value,
) -> Value {
    json!({
        "schema": SCHEMA_PAGE,
        "tool": tool,
        "authority": "native_owner_bytes_digest_verified_only",
        "applicability": "not_assessed",
        "source_currency": "not_assessed",
        "semantic_acceptance": "not_assessed",
        "blob_sha256": blob_sha256,
        "total_bytes": total,
        "offset": offset,
        "has_more": next.is_some(),
        "next_offset": next,
        "text": text,
        "owner": extra,
    })
}

impl ClaimsConsumer {
    /// Start a consumer against the trusted server. The executable and profile
    /// digests are checked before spawn. `max_calls` bounds total tool calls.
    pub fn open(server: &TrustedServer, options: &Options, max_calls: usize) -> Result<Self> {
        if !(4..=32768).contains(&options.page_bytes) || !(1..=64).contains(&max_calls) {
            return Err(Error::new(
                "invalid_options",
                "page_bytes or max_calls bound",
            ));
        }
        let mut session = Session::start(server, options.deadline)?;
        let tools = session.rpc("tools/list", json!({}))?;
        let names: Vec<String> = tools["result"]["tools"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|t| t["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        for need in ["discover_repository_facts", "resolve_repository_fact"] {
            if !names.iter().any(|n| n == need) {
                return Err(Error::new(
                    "server_unavailable",
                    format!("{need} not offered"),
                ));
            }
        }
        Ok(Self {
            session,
            page_bytes: options.page_bytes,
            call_deadline: options.deadline,
            max_calls,
            calls: 0,
            names,
            resolved: BTreeMap::new(),
            evidence: BTreeMap::new(),
            discovered: BTreeMap::new(),
            order: vec![],
        })
    }
    pub fn counts(&self) -> Counts {
        Counts { calls: self.calls }
    }
    fn offers(&self, name: &str) -> bool {
        self.names.iter().any(|n| n == name)
    }
    fn arm(&mut self, until: Option<Instant>) -> Result<()> {
        let now = Instant::now();
        let mut deadline = now + self.call_deadline;
        if let Some(u) = until {
            if now >= u {
                return Err(Error::new(
                    "deadline_expired",
                    "consumption deadline expired",
                ));
            }
            deadline = deadline.min(u);
        }
        self.session.deadline = deadline;
        Ok(())
    }
    fn remember(&mut self, key: String) {
        self.order.retain(|k| k != &key);
        self.order.push(key);
        while self.order.len() > MAX_CACHED_BLOBS {
            let old = self.order.remove(0);
            self.evidence.remove(&old);
            self.discovered.remove(&old);
            self.resolved
                .retain(|(c, r, s), _| format!("{c}\u{0}{r}\u{0}{s}") != old);
        }
    }
    /// Execute one tool call. Unknown tools, extra arguments, oversized input,
    /// an exhausted call budget and an expired `until` refuse before any read.
    pub fn call(&mut self, tool: &str, args: &Value, until: Option<Instant>) -> Result<Value> {
        if !TOOLS.contains(&tool) {
            return Err(Error::new(
                "unknown_tool",
                "tool is not a claims consumer tool",
            ));
        }
        if self.calls >= self.max_calls {
            return Err(Error::new(
                "budget_exhausted",
                "claims call budget exhausted",
            ));
        }
        if self.session.is_poisoned() {
            return Err(Error::new(
                "server_unavailable",
                "transport closed after an earlier failure",
            ));
        }
        self.arm(until)?;
        self.calls += 1;
        match tool {
            TOOL_DISCOVER => self.discover(args),
            TOOL_RESOLVE => self.resolve_tool(args),
            TOOL_EVIDENCE => self.evidence_tool(args),
            _ => self.currency_tool(args),
        }
    }
    fn discover(&mut self, args: &Value) -> Result<Value> {
        let o = args_object(
            args,
            &[
                "query",
                "limit",
                "cursor",
                "offset",
                "blob_sha256",
                "max_bytes",
            ],
        )?;
        let query = arg_text(o, "query")?;
        let limit = arg_uint(o, "limit", MAX_DISCOVER_LIMIT)?;
        if query.is_empty() || query.len() > 512 || !(1..=MAX_DISCOVER_LIMIT).contains(&limit) {
            return Err(Error::new("invalid_arguments", "query or limit bound"));
        }
        let offset = arg_uint(o, "offset", 0)?;
        let max_bytes = page_arg(o)?;
        let mut base = json!({"query": query, "limit": limit});
        if let Some(c) = o.get("cursor") {
            base["cursor"] = c.clone();
        }
        let key = format!("discover\u{0}{}", base);
        if offset == 0 || !self.discovered.contains_key(&key) {
            let (text, sha, _) = read_paged(
                &mut self.session,
                "discover_repository_facts",
                base,
                self.page_bytes,
            )?;
            if offset != 0 && o.get("blob_sha256").and_then(Value::as_str) != Some(sha.as_str()) {
                return Err(Error::new(
                    "claim_projection_changed",
                    "discovery result differs from the paged result",
                ));
            }
            self.discovered.insert(key.clone(), (text, sha));
        }
        self.remember(key.clone());
        let (text, sha) = &self.discovered[&key];
        if offset != 0 && o.get("blob_sha256").and_then(Value::as_str) != Some(sha.as_str()) {
            return Err(Error::new(
                "claim_projection_changed",
                "blob_sha256 differs from the paged result",
            ));
        }
        let (page, next) = page_of(text.as_bytes(), offset, max_bytes)?;
        Ok(page_envelope(
            TOOL_DISCOVER,
            sha,
            text.len(),
            offset,
            page,
            next,
            json!({}),
        ))
    }
    fn ensure_resolved(&mut self, r: &ExactRef) -> Result<(String, String, String)> {
        let key = (
            r.claim_id.clone(),
            r.revision_id.clone(),
            r.revision_sha256.clone(),
        );
        if !self.resolved.contains_key(&key) {
            let (text, sha, _) = read_projection(&mut self.session, r, self.page_bytes)?;
            let v = verify_projection(&text, r)?;
            self.resolved.insert(
                key.clone(),
                Resolved {
                    exact: r.clone(),
                    text,
                    sha256: sha,
                    listed: v.listed,
                    heads: v.heads,
                    current_head: v.current_head,
                },
            );
        }
        self.remember(format!("{}\u{0}{}\u{0}{}", key.0, key.1, key.2));
        Ok(key)
    }
    fn resolve_tool(&mut self, args: &Value) -> Result<Value> {
        let o = args_object(
            args,
            &[
                "claim_id",
                "revision_id",
                "revision_sha256",
                "offset",
                "max_bytes",
            ],
        )?;
        let r = exact_of(o)?;
        let offset = arg_uint(o, "offset", 0)?;
        let max_bytes = page_arg(o)?;
        let key = self.ensure_resolved(&r)?;
        let v = &self.resolved[&key];
        let (page, next) = page_of(v.text.as_bytes(), offset, max_bytes)?;
        Ok(page_envelope(
            TOOL_RESOLVE,
            &v.sha256,
            v.text.len(),
            offset,
            page,
            next,
            json!({
                "claim_id": v.exact.claim_id,
                "revision_id": v.exact.revision_id,
                "revision_sha256": v.exact.revision_sha256,
                "current_head": v.current_head,
                "head_revision_ids": v.heads,
                "listed_evidence_sha256": v.listed,
                "verification": {
                    "projection_digest_matches_assembled_pages": true,
                    "canonical_reencoding_equals_bytes": true,
                    "request_binding_recomputed": true,
                    "revision_digest_recomputed": true,
                    "authority_not_authenticated_here": true
                }
            }),
        ))
    }
    fn listed_evidence(
        &mut self,
        o: &serde_json::Map<String, Value>,
    ) -> Result<(ExactRef, String)> {
        let r = exact_of(o)?;
        let sha = arg_text(o, "sha256")?;
        if !is_sha(&sha) {
            return Err(Error::new("invalid_arguments", "sha256 malformed"));
        }
        let key = self.ensure_resolved(&r)?;
        if !self.resolved[&key].listed.contains(&sha) {
            return Err(Error::new(
                "evidence_not_listed",
                "evidence digest is not listed by the exact selected revision",
            ));
        }
        if !self.offers("read_repository_evidence") {
            return Err(Error::new(
                "evidence_unavailable",
                "server profile offers no evidence reads",
            ));
        }
        Ok((r, sha))
    }
    fn evidence_tool(&mut self, args: &Value) -> Result<Value> {
        let o = args_object(
            args,
            &[
                "claim_id",
                "revision_id",
                "revision_sha256",
                "sha256",
                "offset",
                "max_bytes",
            ],
        )?;
        let offset = arg_uint(o, "offset", 0)?;
        let max_bytes = page_arg(o)?;
        let (_, sha) = self.listed_evidence(o)?;
        if !self.evidence.contains_key(&sha) {
            let (bytes, _) = read_evidence_bytes(&mut self.session, &sha, self.page_bytes)?;
            self.evidence.insert(sha.clone(), bytes);
        }
        self.remember(sha.clone());
        let bytes = &self.evidence[&sha];
        let (page, next) = page_of(bytes, offset, max_bytes)?;
        Ok(page_envelope(
            TOOL_EVIDENCE,
            &sha,
            bytes.len(),
            offset,
            page,
            next,
            json!({"integrity_sha256_verified": true}),
        ))
    }
    fn currency_tool(&mut self, args: &Value) -> Result<Value> {
        let o = args_object(
            args,
            &["claim_id", "revision_id", "revision_sha256", "sha256"],
        )?;
        let (_, sha) = self.listed_evidence(o)?;
        if !self.offers("check_repository_evidence_currency") {
            return Err(Error::new(
                "evidence_unavailable",
                "server profile offers no currency checks",
            ));
        }
        let owner = self
            .session
            .tool("check_repository_evidence_currency", json!({"sha256": sha}))?;
        let text = owner.to_string();
        if text.len() > MAX_CURRENCY_BYTES {
            return Err(Error::new("protocol", "currency result exceeds bound"));
        }
        // The owner's result is returned unmodified: changed or unavailable
        // currency is never coerced into current.
        Ok(json!({
            "schema": SCHEMA_PAGE,
            "tool": TOOL_CURRENCY,
            "authority": "native_owner_currency_result",
            "applicability": "not_assessed",
            "semantic_acceptance": "not_assessed",
            "evidence_sha256": sha,
            "owner_result": owner,
        }))
    }
}

/// The consumer's explicit decision about one exact selected revision. It is
/// task-local data: reading or selecting never implies it, and it is neither a
/// claims reliance record nor a lifecycle grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelianceDecision {
    pub claim_id: String,
    pub revision_id: String,
    pub revision_sha256: String,
    /// `relied_for_task`, `declined`, or `needs_more_evidence`.
    pub decision: String,
    pub basis: String,
}

impl RelianceDecision {
    pub const VALUES: [&'static str; 3] = ["relied_for_task", "declined", "needs_more_evidence"];
    pub fn new(subject: &ExactRef, decision: &str, basis: &str) -> Result<Self> {
        let d = Self {
            claim_id: subject.claim_id.clone(),
            revision_id: subject.revision_id.clone(),
            revision_sha256: subject.revision_sha256.clone(),
            decision: decision.into(),
            basis: basis.into(),
        };
        d.validate()?;
        Ok(d)
    }
    fn validate(&self) -> Result<()> {
        let bad = |d: &str| Err(Error::new("invalid_decision", d.to_string()));
        if !is_identity(&self.claim_id)
            || !is_identity(&self.revision_id)
            || !is_sha(&self.revision_sha256)
        {
            return bad("subject must be an exact bounded revision identity");
        }
        if !Self::VALUES.contains(&self.decision.as_str()) {
            return bad("decision value not recognised");
        }
        if !is_text(&self.basis) {
            return bad("basis must be bounded plain text");
        }
        Ok(())
    }
    fn canonical(&self) -> Result<String> {
        let v = JsValue::object([
            ("schema", JsValue::text(SCHEMA_DECISION)),
            (
                "subject",
                JsValue::object([
                    ("claim_id", JsValue::text(&self.claim_id)),
                    ("revision_id", JsValue::text(&self.revision_id)),
                    ("revision_sha256", JsValue::text(&self.revision_sha256)),
                ]),
            ),
            ("decision", JsValue::text(&self.decision)),
            ("basis", JsValue::text(&self.basis)),
            ("grant", JsValue::text("none")),
            (
                "authoritative_reliance_record",
                JsValue::text("not_written"),
            ),
        ]);
        let mut text =
            canonical_json(&v).map_err(|e| Error::new("invalid_decision", e.to_string()))?;
        // The native codec newline-terminates; a decision is one line.
        if text.ends_with('\n') {
            text.pop();
        }
        Ok(text)
    }
    pub fn line(&self) -> Result<String> {
        Ok(format!("{DECISION_MARKER}{}", self.canonical()?))
    }
    /// Strict parse of one marker line body; canonical re-encoding must equal
    /// the input.
    pub fn parse(body: &str) -> Result<Self> {
        let bad = |d: &str| Error::new("invalid_decision", d.to_string());
        if body.len() > MAX_HANDOFF_BYTES {
            return Err(bad("decision exceeds bound"));
        }
        let v: Value = serde_json::from_str(body).map_err(|_| bad("not JSON"))?;
        let text = |p: &[&str]| -> Result<String> {
            let mut c = &v;
            for k in p {
                c = c.get(*k).ok_or_else(|| bad("field absent"))?;
            }
            c.as_str()
                .map(String::from)
                .ok_or_else(|| bad("text required"))
        };
        let d = Self {
            claim_id: text(&["subject", "claim_id"])?,
            revision_id: text(&["subject", "revision_id"])?,
            revision_sha256: text(&["subject", "revision_sha256"])?,
            decision: text(&["decision"])?,
            basis: text(&["basis"])?,
        };
        d.validate()?;
        if d.canonical()? != body {
            return Err(bad("decision is not the canonical encoding"));
        }
        Ok(d)
    }
    /// Find exactly one decision line in assistant text; none or several refuse.
    pub fn extract(text: &str) -> Result<Self> {
        let mut found = text.lines().filter_map(|l| l.strip_prefix(DECISION_MARKER));
        let first = found
            .next()
            .ok_or_else(|| Error::new("invalid_decision", "no decision line"))?;
        if found.next().is_some() {
            return Err(Error::new(
                "invalid_decision",
                "more than one decision line",
            ));
        }
        Self::parse(first)
    }
    /// The decision's subject must be one of the selected exact revisions.
    pub fn check_subject(&self, selected: &[ExactRef]) -> Result<()> {
        if selected.iter().any(|r| {
            r.claim_id == self.claim_id
                && r.revision_id == self.revision_id
                && r.revision_sha256 == self.revision_sha256
        }) {
            Ok(())
        } else {
            Err(Error::new(
                "invalid_decision",
                "decision subject is not a selected exact revision",
            ))
        }
    }
}
