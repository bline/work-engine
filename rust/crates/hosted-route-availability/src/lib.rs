//! Read-only route availability view over hosted Pi journals.
//!
//! Given explicit journal roots, an operator configuration and a fixed `now`,
//! derives per-pool, per-dimension availability from each journal's attempt
//! records and digest-verified retained artifacts. Holds no credentials and
//! never changes dispatch. Header names and failure classification are the
//! provisional, digest-identified table in [`provisional`].
//!
//! Each journal is read as data (`pi-state.json`), checked against the fields
//! this view relies on; anything unexpected fails closed: the journal is
//! skipped and marked, and a dimension without usable evidence is `unknown`.
pub mod provisional;

use provisional::{Class, Effect, Reset, Table, WindowReading};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub const CONFIG_SCHEMA: &str = "work-engine.hosted-route-availability-config.v1";
/// v2 adds each pool's declaration (members, thresholds, credit clear) and
/// the evidence age bound, so a consumer can check which operator pools an
/// output was derived under.
pub const OUTPUT_SCHEMA: &str = "work-engine.hosted-route-availability.v2";
/// The Host journal schema this view reads.
pub const JOURNAL_SCHEMA: &str = "work-engine.pi-docker-custody.v1";
/// Hosted job profiles whose journals carry hosted attempts.
pub const HOSTED_PROFILES: [&str; 3] = [
    "work-engine.pi-coding-hosted.v1",
    "work-engine.pi-analysis-hosted.v1",
    "work-engine.pi-lifecycle-hosted.v1",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub now_ms: u64,
    /// Absolute per-job state directories, each holding `pi-state.json`.
    pub journal_roots: Vec<PathBuf>,
    /// Evidence older than this (relative to `now_ms`) is stale.
    pub max_evidence_age_ms: u64,
    pub pools: Vec<Pool>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pool {
    pub id: String,
    pub funding: Funding,
    /// Operator-declared routes, by payer name and endpoint kind, that share
    /// this pool's state in every listed journal.
    pub members: Vec<Member>,
    /// Per-dimension low-quota threshold on utilization.
    #[serde(default)]
    pub low_thresholds: BTreeMap<String, f64>,
    /// The operator cleared API credit exhaustion at this time: earlier API
    /// credit evidence is not used.
    #[serde(default)]
    pub credit_cleared_ms: Option<u64>,
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Funding {
    Subscription,
    Api,
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub payer: String,
    pub endpoint: String,
}
impl Funding {
    /// The funding's name in [`provisional::TABLE`].
    pub fn label(self) -> &'static str {
        match self {
            Self::Subscription => "subscription",
            Self::Api => "api",
        }
    }
}
/// Provider, funding and the payer-source kinds the Host records for an
/// endpoint kind (`journal.coding.hosted_transport.payer_sources`).
fn endpoint_kind(endpoint: &str) -> Option<(&'static str, Funding, &'static [&'static str])> {
    Some(match endpoint {
        "https:anthropic-oauth" => ("anthropic", Funding::Subscription, &["claude-code-auth"]),
        "https:anthropic-api" => (
            "anthropic",
            Funding::Api,
            &["anthropic-api-key-env", "anthropic-api-key-file"],
        ),
        "https:chatgpt-codex" => ("openai", Funding::Subscription, &["chatgpt-pi-auth"]),
        "https:openai-responses" => ("openai", Funding::Api, &["api-key-env", "api-key-file"]),
        _ => return None,
    })
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// The view's output for `config_bytes`, as pretty JSON with a trailing
/// newline. Fails only on an invalid configuration.
pub fn view(config_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let config: Config =
        serde_json::from_slice(config_bytes).map_err(|e| format!("config: {e}"))?;
    let mut out = serde_json::to_vec_pretty(&derive(
        &config,
        &digest(config_bytes),
        &provisional::table(),
    )?)
    .map_err(|e| e.to_string())?;
    out.push(b'\n');
    Ok(out)
}

/// Validate a configuration against the pinned table, as [`view`] does.
pub fn check_config(config: &Config) -> Result<(), String> {
    check(config, &provisional::table())
}

/// A pool's declaration as reported in the output.
pub fn pool_declaration(p: &Pool) -> Value {
    json!({"members": p.members, "low_thresholds": p.low_thresholds, "credit_cleared_ms": p.credit_cleared_ms})
}

fn check(config: &Config, table: &Table) -> Result<(), String> {
    if config.schema != CONFIG_SCHEMA {
        return Err(format!("config schema must be {CONFIG_SCHEMA}"));
    }
    if let Some(r) = config.journal_roots.iter().find(|r| !r.is_absolute()) {
        return Err(format!("journal root is not absolute: {}", r.display()));
    }
    let mut ids = BTreeSet::new();
    let mut members = BTreeSet::new();
    for p in &config.pools {
        if !ids.insert(&p.id) {
            return Err(format!("duplicate pool {}", p.id));
        }
        for m in &p.members {
            match endpoint_kind(&m.endpoint) {
                Some((_, f, _)) if f == p.funding => {}
                _ => {
                    return Err(format!(
                        "pool {}: endpoint {} is not {}",
                        p.id,
                        m.endpoint,
                        p.funding.label()
                    ));
                }
            }
            if !members.insert(m) {
                return Err(format!(
                    "route {}/{} is in more than one pool",
                    m.payer, m.endpoint
                ));
            }
        }
        for (d, t) in &p.low_thresholds {
            if !table.dimensions(p.funding.label()).contains(d) {
                return Err(format!("pool {}: no dimension {d}", p.id));
            }
            if !table.threshold_valid(*t) {
                let u = &table.utilization;
                return Err(format!(
                    "pool {}: threshold {d} must be in ({}, {}]",
                    p.id, u.threshold_min_exclusive, u.threshold_max
                ));
            }
        }
        if p.credit_cleared_ms.is_some()
            && table
                .operator_action_dimensions(p.funding.label())
                .is_empty()
        {
            return Err(format!(
                "pool {}: credit_cleared_ms needs a dimension the operator clears",
                p.id
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Available,
    Exhausted(Reset),
    /// No reset; the operator clears it.
    OperatorAction,
    Unknown(&'static str),
}
/// A status observation of one dimension.
#[derive(Clone, Debug)]
struct Observation {
    pool: usize,
    dimension: String,
    time: u64,
    job: String,
    request: u64,
    attempt: u64,
    kind: Kind,
    evidence: Value,
}
/// A utilization reading of one window from any valid head, successful or
/// not, with the reset of the window it was read in.
#[derive(Clone, Debug)]
struct Reading {
    pool: usize,
    dimension: String,
    time: u64,
    job: String,
    request: u64,
    attempt: u64,
    /// The reported share, or why the window's headers were unusable
    /// (`malformed_header`, `conflicting_header`).
    utilization: Result<f64, &'static str>,
    reset: Option<Reset>,
    evidence: Value,
}
/// The deterministic order of observations and readings.
fn order(time: u64, job: &str, request: u64, attempt: u64) -> (u64, &str, u64, u64) {
    (time, job, request, attempt)
}

/// Blocking marks: evidence that could not be read or verified, so the view
/// may be missing something (`complete` is false). Informational marks:
/// classified evidence that changes no quota dimension.
#[derive(Default)]
struct Marks {
    blocking: BTreeSet<String>,
    informational: BTreeSet<String>,
}

/// One attempt record as read from the journal.
struct Attempt {
    request: u64,
    attempt: u64,
    route: String,
    payer: String,
    disposition: String,
    status: Option<u16>,
    /// The Host's settlement reason, when recorded.
    reason: Option<String>,
}
/// The Host's settlement reason for a provider failure event it found in a
/// relayed stream (`pi_coding_provider.rs`).
const HOST_STREAM_FAILURE: &str = "provider failure event in relayed stream:";

/// A response head as retained for one attempt.
struct Head {
    status: u16,
    headers: Vec<(String, String)>,
    /// From `head.json`; `relay.json` has no head time.
    head_ms: Option<u64>,
    time: Option<u64>,
    artifact: &'static str,
}

/// One journal's data needed here, checked; `Err` is the reason to skip it.
struct Journal {
    root: PathBuf,
    job: String,
    routes: BTreeMap<String, (String, String)>,
    payer_sources: Option<BTreeMap<String, String>>,
    artifacts: BTreeMap<String, String>,
    attempts: Vec<Attempt>,
}

fn s(v: &Value) -> Result<String, &'static str> {
    v.as_str()
        .map(str::to_owned)
        .ok_or("unexpected_journal_shape")
}
fn read_journal(root: &Path) -> Result<(Journal, String), &'static str> {
    let raw = fs::read(root.join("pi-state.json")).map_err(|_| "journal_unreadable")?;
    let sha = digest(&raw);
    let j: Value = serde_json::from_slice(&raw).map_err(|_| "journal_malformed")?;
    if j["schema"] != JOURNAL_SCHEMA {
        return Err("journal_schema");
    }
    if j["job"]["state_dir"].as_str() != root.to_str() {
        return Err("journal_root_mismatch");
    }
    let job = s(&j["job"]["job_id"])?;
    let profile = j["job"]["coding"]["schema"].as_str().unwrap_or_default();
    if !HOSTED_PROFILES.contains(&profile) {
        return Err("journal_not_hosted");
    }
    let mut routes = BTreeMap::new();
    for r in j["job"]["coding"]["provider"]["hosted"]["routes"]
        .as_array()
        .ok_or("unexpected_journal_shape")?
    {
        routes.insert(s(&r["id"])?, (s(&r["payer"])?, s(&r["endpoint"])?));
    }
    let mut artifacts = BTreeMap::new();
    for a in j["artifacts"]
        .as_array()
        .ok_or("unexpected_journal_shape")?
    {
        artifacts.insert(s(&a["path"])?, s(&a["sha256"])?);
    }
    let coding = &j["coding"];
    let payer_sources = match &coding["hosted_transport"]["payer_sources"] {
        Value::Null => None,
        Value::Object(m) => Some(
            m.iter()
                .map(|(k, v)| Ok((k.clone(), s(v)?)))
                .collect::<Result<_, &'static str>>()?,
        ),
        _ => return Err("unexpected_journal_shape"),
    };
    let mut attempts = vec![];
    let requests = match &coding["provider_requests"] {
        Value::Null => serde_json::Map::new(),
        Value::Object(m) => m.clone(),
        _ => return Err("unexpected_journal_shape"),
    };
    for (key, r) in &requests {
        let request: u64 = key.parse().map_err(|_| "unexpected_journal_shape")?;
        let list = match &r["attempts"] {
            Value::Null => continue,
            Value::Array(a) => a,
            _ => return Err("unexpected_journal_shape"),
        };
        for a in list {
            let disposition = s(&a["disposition"])?;
            if ![
                "pending",
                "refused",
                "not_dispatched",
                "completed",
                "uncertain",
            ]
            .contains(&disposition.as_str())
            {
                return Err("unexpected_journal_shape");
            }
            let status = match &a["http_status"] {
                Value::Null => None,
                v => Some(
                    v.as_u64()
                        .and_then(|n| u16::try_from(n).ok())
                        .ok_or("unexpected_journal_shape")?,
                ),
            };
            attempts.push(Attempt {
                request,
                attempt: a["attempt"].as_u64().ok_or("unexpected_journal_shape")?,
                route: s(&a["route"])?,
                payer: s(&a["payer"])?,
                disposition,
                status,
                reason: match &a["reason"] {
                    Value::Null => None,
                    v => Some(s(v)?),
                },
            });
        }
    }
    Ok((
        Journal {
            root: root.to_owned(),
            job,
            routes,
            payer_sources,
            artifacts,
            attempts,
        },
        sha,
    ))
}

struct Reader<'a> {
    marks: &'a mut Marks,
}
impl Reader<'_> {
    fn mark(&mut self, v: Value) {
        self.marks.blocking.insert(v.to_string());
    }
    fn note(&mut self, v: Value) {
        self.marks.informational.insert(v.to_string());
    }
    /// A registered artifact whose bytes match the journal's digest. An
    /// unregistered file is no evidence; a mismatch is ignored and marked.
    fn artifact(&mut self, j: &Journal, name: &str) -> Option<Vec<u8>> {
        let path = j.root.join(name);
        let recorded = j.artifacts.get(path.to_str()?)?;
        match fs::read(&path) {
            Ok(b) if digest(&b) == *recorded => Some(b),
            _ => {
                self.mark(json!({"kind":"artifact_digest_mismatch","job_id":j.job,"path":path}));
                None
            }
        }
    }
    fn head(&mut self, j: &Journal, request: u64, attempt: u64) -> Option<Head> {
        // Kept headers as `[name, value]` string pairs; anything else is invalid.
        let pairs = |v: &Value| -> Option<Vec<(String, String)>> {
            v.as_array()?
                .iter()
                .map(|p| match p.as_array()?.as_slice() {
                    [k, v] => Some((k.as_str()?.to_owned(), v.as_str()?.to_owned())),
                    _ => None,
                })
                .collect()
        };
        let name = format!("coding-hosted-{request}-{attempt}.head.json");
        if let Some(b) = self.artifact(j, &name) {
            let v: Value = serde_json::from_slice(&b).unwrap_or(Value::Null);
            let valid = v["jobId"].as_str() == Some(j.job.as_str())
                && v["request"].as_u64() == Some(request)
                && v["attempt"].as_u64() == Some(attempt);
            match (valid, v["status"].as_u64(), v["headMs"].as_u64(), pairs(&v["headHeaders"])) {
                (true, Some(status), Some(ms), Some(headers)) if status <= u16::MAX as u64 => {
                    return Some(Head {
                        status: status as u16,
                        headers,
                        head_ms: Some(ms),
                        time: Some(ms),
                        artifact: "head.json",
                    });
                }
                _ => self.mark(json!({"kind":"invalid_head_evidence","job_id":j.job,"request":request,"attempt":attempt})),
            }
        }
        let name = format!("coding-hosted-{request}-{attempt}.relay.json");
        let b = self.artifact(j, &name)?;
        let v: Value = serde_json::from_slice(&b).ok()?;
        let status = v["status"].as_u64().filter(|s| *s <= u16::MAX as u64)? as u16;
        Some(Head {
            status,
            headers: pairs(&v["headHeaders"])?,
            head_ms: None,
            time: v["upstreamEndMs"].as_u64().or(v["openedMs"].as_u64()),
            artifact: "relay.json",
        })
    }
}

/// Where one attempt's observations and readings go.
struct Sink<'a> {
    observations: &'a mut Vec<Observation>,
    readings: &'a mut Vec<Reading>,
}

fn class_label(class: Class) -> String {
    serde_json::to_value(class)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Status observations and utilization readings of one attempt on its
/// pool's dimensions.
fn observe(
    table: &Table,
    reader: &mut Reader,
    j: &Journal,
    pool: usize,
    a: &Attempt,
    endpoint: &str,
    sink: Sink,
) {
    let (provider, funding, kinds) = endpoint_kind(endpoint).expect("pool member endpoint");
    let f = funding.label();
    let (request, attempt) = (a.request, a.attempt);
    let id = json!({"job_id":j.job,"request":request,"attempt":attempt});
    let with_id = |mut v: Value| {
        v.as_object_mut()
            .unwrap()
            .extend(id.as_object().unwrap().clone());
        v
    };
    let refused = a.disposition == "refused";
    if !matches!(
        a.disposition.as_str(),
        "refused" | "completed" | "uncertain"
    ) {
        return;
    }
    let Some(head) = reader.head(j, request, attempt) else {
        if a.disposition != "uncertain" {
            reader.mark(with_id(json!({"kind":"no_head_evidence"})));
        }
        return;
    };
    let Some(time) = head.time else {
        reader.mark(with_id(json!({"kind":"no_observation_time"})));
        return;
    };
    let base = |source: String| {
        json!({"job_id":j.job,"request":request,"attempt":attempt,"route":a.route,"payer":a.payer,
            "endpoint":endpoint,"observed_ms":time,"artifact":head.artifact,"source":source})
    };
    let dims = table.dimensions(f);
    let funding_reason = match j.payer_sources.as_ref().and_then(|m| m.get(&a.payer)) {
        None => Some("funding_unverified"),
        Some(k) if !kinds.contains(&k.as_str()) => Some("funding_conflict"),
        _ => None,
    };
    let reason = funding_reason.or(match a.status {
        Some(s) if s != head.status => Some("status_conflict"),
        _ => None,
    });
    let windows: Vec<WindowReading> = table.windows(provider, &head.headers, head.head_ms);
    if reason.is_none() {
        // Every window utilization is a reading, whatever the outcome; an
        // unusable window is a reading of why, so no older share stays in
        // force.
        for w in &windows {
            let utilization = match (w.malformed, w.utilization) {
                (Some(m), _) => Err(m),
                (None, Some(u)) => Ok(u),
                (None, None) => continue,
            };
            let mut evidence = base(format!(
                "header:{}",
                if w.malformed.is_some() {
                    &w.source
                } else {
                    &w.utilization_header
                }
            ));
            evidence["reset_at"] = match &w.reset {
                Some(Reset::At(t)) => json!(t),
                _ => Value::Null,
            };
            sink.readings.push(Reading {
                pool,
                dimension: w.dimension.clone(),
                time,
                job: j.job.clone(),
                request,
                attempt,
                utilization,
                reset: w.reset.clone(),
                evidence,
            });
        }
    }
    let mut push = |dimension: &str, kind: Kind, source: String| {
        sink.observations.push(Observation {
            pool,
            dimension: dimension.to_owned(),
            time,
            job: j.job.clone(),
            request,
            attempt,
            kind,
            evidence: base(source),
        })
    };
    if let Some(reason) = reason {
        for d in dims {
            push(d, Kind::Unknown(reason), format!("journal:{reason}"));
        }
        return;
    }
    let window = |d: &str| windows.iter().find(|w| w.dimension == d);
    if a.disposition == "completed" && (200..300).contains(&head.status) {
        for d in dims {
            match window(d) {
                Some(w) if w.malformed.is_some() => push(
                    d,
                    Kind::Unknown(w.malformed.unwrap()),
                    format!("header:{}", w.source),
                ),
                Some(w) if w.exhausted == Some(true) => push(
                    d,
                    Kind::Unknown("conflicting_header"),
                    format!("header:{}", w.source),
                ),
                Some(w) => push(d, Kind::Available, format!("success;header:{}", w.source)),
                None => push(d, Kind::Available, "success".into()),
            }
        }
        return;
    }
    let class = if refused || head.status >= 400 {
        let kind = if refused {
            "refusal.body"
        } else {
            "stream.sse"
        };
        let body = reader
            .artifact(j, &format!("coding-hosted-{request}-{attempt}.{kind}"))
            .unwrap_or_default();
        table.classify_head(provider, f, head.status, &body)
    } else {
        // A provider failure after a success head (the attempt stayed
        // uncertain): classified from the retained stream's failure event.
        let Some(stream) =
            reader.artifact(j, &format!("coding-hosted-{request}-{attempt}.stream.sse"))
        else {
            // Unregistered or unverified: the view cannot see the stream.
            reader.mark(with_id(
                json!({"kind":"post_head_failure_unclassified","reason":a.reason,"stream":"unverified"}),
            ));
            return;
        };
        match table.classify_stream(provider, f, &stream) {
            None if a
                .reason
                .as_deref()
                .is_some_and(|r| r.starts_with(HOST_STREAM_FAILURE)) =>
            {
                // The Host found a failure event the view cannot.
                reader.mark(with_id(
                    json!({"kind":"post_head_failure_unclassified","reason":a.reason,"stream":"verified"}),
                ));
                return;
            }
            None => {
                // A verified stream with no failure event (guard close,
                // cancel, Host close, crash reconcile): no quota signal.
                reader.note(with_id(
                    json!({"kind":"post_head_no_failure_event","reason":a.reason}),
                ));
                return;
            }
            Some(class) => {
                reader.note(with_id(json!({"kind":"post_head_failure","class":class})));
                class
            }
        }
    };
    let source = format!("classification:{}", class_label(class));
    let effect = table
        .effect(class, f)
        .map(|e| (e.effect, e.dimension.as_deref()));
    match effect {
        Some((Effect::Windows, _)) => {
            let exhausted: Vec<_> = windows
                .iter()
                .filter(|w| table.window_exhausted(w))
                .collect();
            for d in dims {
                if let Some(w) = window(d).filter(|w| w.malformed.is_some()) {
                    push(
                        d,
                        Kind::Unknown(w.malformed.unwrap()),
                        format!("{source};header:{}", w.source),
                    );
                } else if exhausted.is_empty() {
                    push(d, Kind::Unknown("window_undetermined"), source.clone());
                } else if let Some(w) = exhausted.iter().find(|w| w.dimension == *d) {
                    let reset = w.reset.clone().unwrap_or(Reset::Unknown("reset_absent"));
                    push(
                        d,
                        Kind::Exhausted(reset),
                        format!("{source};header:{}", w.source),
                    );
                }
            }
        }
        Some((Effect::RetryAfter, Some(d))) => {
            let reset = table
                .retry_after(&head.headers, head.head_ms)
                .unwrap_or(Reset::Unknown("reset_absent"));
            push(
                d,
                Kind::Exhausted(reset),
                format!("{source};header:{}", table.retry_after),
            );
        }
        Some((Effect::OperatorAction, Some(d))) => push(d, Kind::OperatorAction, source),
        Some((Effect::NonQuota, _)) => {
            reader.note(with_id(json!({"kind":"non_quota_failure","class":class})))
        }
        _ => {
            for d in dims {
                push(d, Kind::Unknown("unclassified_failure"), source.clone());
            }
        }
    }
}

fn derive(config: &Config, config_sha256: &str, table: &Table) -> Result<Value, String> {
    check(config, table)?;
    let mut marks = Marks::default();
    let mut journals = vec![];
    let mut observations = vec![];
    let mut readings = vec![];
    let roots: BTreeSet<&PathBuf> = config.journal_roots.iter().collect();
    let members: BTreeMap<&Member, usize> = config
        .pools
        .iter()
        .enumerate()
        .flat_map(|(i, p)| p.members.iter().map(move |m| (m, i)))
        .collect();
    for root in roots {
        let (j, sha) = match read_journal(root) {
            Ok(v) => v,
            Err(reason) => {
                journals.push(json!({"root":root,"read":false,"reason":reason}));
                marks
                    .blocking
                    .insert(json!({"kind":reason,"root":root}).to_string());
                continue;
            }
        };
        journals.push(json!({"root":root,"read":true,"job_id":j.job,"journal_sha256":sha}));
        let mut reader = Reader { marks: &mut marks };
        for a in &j.attempts {
            let id = |kind: &str| json!({"kind":kind,"job_id":j.job,"request":a.request,"attempt":a.attempt});
            let Some((route_payer, endpoint)) = j.routes.get(&a.route) else {
                reader.mark(id("unknown_route"));
                continue;
            };
            if *route_payer != a.payer {
                reader.mark(id("route_payer_mismatch"));
                continue;
            }
            let key = Member {
                payer: a.payer.clone(),
                endpoint: endpoint.clone(),
            };
            if let Some(&i) = members.get(&key) {
                let sink = Sink {
                    observations: &mut observations,
                    readings: &mut readings,
                };
                observe(table, &mut reader, &j, i, a, endpoint, sink);
            }
        }
    }
    let now = config.now_ms;
    let mut pools = vec![];
    for (i, p) in config.pools.iter().enumerate() {
        let f = p.funding.label();
        let operator = table.operator_action_dimensions(f);
        let mut dimensions = vec![];
        for d in table.dimensions(f) {
            let mut obs: Vec<&Observation> = observations
                .iter()
                .filter(|o| o.pool == i && o.dimension == *d && o.time <= now)
                .collect();
            obs.sort_by(|a, b| {
                order(a.time, &a.job, a.request, a.attempt)
                    .cmp(&order(b.time, &b.job, b.request, b.attempt))
            });
            let mut reads: Vec<&Reading> = readings
                .iter()
                .filter(|r| r.pool == i && r.dimension == *d && r.time <= now)
                .collect();
            reads.sort_by(|a, b| {
                order(a.time, &a.job, a.request, a.attempt)
                    .cmp(&order(b.time, &b.job, b.request, b.attempt))
            });
            // A clear after `now` has not happened yet.
            let cleared = p
                .credit_cleared_ms
                .filter(|c| *c <= now && operator.contains(&d.as_str()));
            let st = status(&obs, now, config.max_evidence_age_ms, cleared);
            let (utilization, utilization_reason, reading) =
                utilization(&reads, now, config.max_evidence_age_ms);
            let low = match (utilization, p.low_thresholds.get(d)) {
                (Some(u), Some(t)) => json!(u >= *t),
                _ => Value::Null,
            };
            dimensions.push(json!({
                "dimension": d,
                "status": st.status,
                "reason": st.reason,
                "available_again_at": st.at,
                "available_again_reason": st.at_reason,
                "evidence": st.evidence.map(|o| &o.evidence),
                "utilization": utilization,
                "utilization_reason": utilization_reason,
                "utilization_evidence": reading.map(|r| &r.evidence),
                "low": low,
            }));
        }
        let mut pool = json!({"id":p.id,"funding":f,"dimensions":dimensions});
        if let (Value::Object(m), Value::Object(d)) = (&mut pool, pool_declaration(p)) {
            m.extend(d);
        }
        pools.push(pool);
    }
    let list = |set: &BTreeSet<String>| -> Vec<Value> {
        set.iter()
            .map(|m| serde_json::from_str(m).unwrap())
            .collect()
    };
    Ok(json!({
        "schema": OUTPUT_SCHEMA,
        "now_ms": now,
        "max_evidence_age_ms": config.max_evidence_age_ms,
        "config_sha256": config_sha256,
        "table": {"version": table.version, "sha256": provisional::table_sha256(), "provisional": table.provisional},
        "journals": journals,
        "pools": pools,
        "marks": {"blocking": list(&marks.blocking), "informational": list(&marks.informational)},
        "complete": marks.blocking.is_empty(),
    }))
}

struct Status<'a> {
    status: &'static str,
    reason: Option<&'static str>,
    evidence: Option<&'a Observation>,
    at: Option<u64>,
    at_reason: Option<&'static str>,
}

/// A dimension's status from its observations, oldest first. `cleared`: the
/// operator cleared this operator-action dimension at that time.
fn status<'a>(obs: &[&'a Observation], now: u64, max_age: u64, cleared: Option<u64>) -> Status<'a> {
    let unknown = |reason, evidence| Status {
        status: "unknown",
        reason: Some(reason),
        evidence,
        at: None,
        at_reason: Some("status_unknown"),
    };
    let mut obs = obs;
    if let Some(c) = cleared
        && let Some(exhaustion) = obs
            .iter()
            .rev()
            .find(|o| o.time <= c && o.kind == Kind::OperatorAction)
    {
        // Nothing at or before the clear counts after it; the clear is the
        // reset, verified only by a later success.
        let after = obs.partition_point(|o| o.time <= c);
        if after == obs.len() {
            return Status {
                status: "reset_elapsed",
                reason: Some("operator_cleared"),
                evidence: Some(exhaustion),
                at: Some(c),
                at_reason: None,
            };
        }
        obs = &obs[after..];
    }
    let Some(last) = obs.last().copied() else {
        return unknown("no_evidence", None);
    };
    if obs
        .iter()
        .any(|o| o.time == last.time && o.kind != last.kind)
    {
        return unknown("conflicting", Some(last));
    }
    // Operator-action exhaustion never expires by age.
    if now.saturating_sub(last.time) > max_age && last.kind != Kind::OperatorAction {
        return unknown("stale", Some(last));
    }
    let known = |status, at, at_reason| Status {
        status,
        reason: None,
        evidence: Some(last),
        at,
        at_reason,
    };
    match &last.kind {
        Kind::Available => known("available", None, None),
        Kind::Exhausted(Reset::At(t)) if now >= *t => known("reset_elapsed", Some(*t), None),
        Kind::Exhausted(Reset::At(t)) => known("exhausted", Some(*t), None),
        Kind::Exhausted(Reset::Unknown(r)) => known("exhausted", None, Some(r)),
        Kind::OperatorAction => known("exhausted", None, Some("operator_action")),
        Kind::Unknown(r) => unknown(r, Some(last)),
    }
}

/// The last reported share of a window, from its newest reading, oldest
/// first. It applies only within the window it was read in: once that
/// window's reset has passed, utilization is null until a new reading.
fn utilization<'a>(
    reads: &[&'a Reading],
    now: u64,
    max_age: u64,
) -> (Option<f64>, Option<&'static str>, Option<&'a Reading>) {
    let Some(last) = reads.last().copied() else {
        return (None, Some("no_reading"), None);
    };
    let bits = |r: &Reading| r.utilization.map(f64::to_bits);
    if reads
        .iter()
        .any(|r| r.time == last.time && bits(r) != bits(last))
    {
        return (None, Some("conflicting"), Some(last));
    }
    let utilization = match last.utilization {
        Ok(u) => u,
        Err(reason) => return (None, Some(reason), Some(last)),
    };
    if now.saturating_sub(last.time) > max_age {
        return (None, Some("stale"), Some(last));
    }
    if matches!(last.reset, Some(Reset::At(t)) if now >= t) {
        return (None, Some("window_reset_since_reading"), Some(last));
    }
    (Some(utilization), None, Some(last))
}
