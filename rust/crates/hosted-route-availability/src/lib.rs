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

use provisional::{Class, Reset, Table, WindowReading};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub const CONFIG_SCHEMA: &str = "work-engine.hosted-route-availability-config.v1";
pub const OUTPUT_SCHEMA: &str = "work-engine.hosted-route-availability.v1";
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
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Funding {
    Subscription,
    Api,
}
#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub payer: String,
    pub endpoint: String,
}
impl Funding {
    fn label(self) -> &'static str {
        match self {
            Self::Subscription => "subscription",
            Self::Api => "api",
        }
    }
    pub fn dimensions(self) -> &'static [&'static str] {
        match self {
            Self::Subscription => &["subscription_5h", "subscription_weekly"],
            Self::Api => &["api_rate", "api_credit"],
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
    let mut out = serde_json::to_vec_pretty(&derive(&config, &digest(config_bytes))?)
        .map_err(|e| e.to_string())?;
    out.push(b'\n');
    Ok(out)
}

fn check(config: &Config) -> Result<(), String> {
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
            if !p.funding.dimensions().contains(&d.as_str()) {
                return Err(format!("pool {}: no dimension {d}", p.id));
            }
            if !(t.is_finite() && *t > 0.0 && *t <= 1.0) {
                return Err(format!("pool {}: threshold {d} must be in (0, 1]", p.id));
            }
        }
        if p.credit_cleared_ms.is_some() && p.funding != Funding::Api {
            return Err(format!(
                "pool {}: credit_cleared_ms needs api funding",
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
    /// API credit: no reset; the operator clears it.
    OperatorAction,
    Unknown(&'static str),
}
#[derive(Clone, Debug)]
struct Observation {
    pool: usize,
    dimension: &'static str,
    time: u64,
    job: String,
    request: u64,
    attempt: u64,
    kind: Kind,
    utilization: Option<f64>,
    evidence: Value,
}
impl Observation {
    fn same_reading(&self, o: &Observation) -> bool {
        self.kind == o.kind && self.utilization.map(f64::to_bits) == o.utilization.map(f64::to_bits)
    }
}

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
    attempts: Vec<(u64, u64, String, String, String, Option<u16>)>,
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
            attempts.push((
                request,
                a["attempt"].as_u64().ok_or("unexpected_journal_shape")?,
                s(&a["route"])?,
                s(&a["payer"])?,
                disposition,
                status,
            ));
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
    marks: &'a mut BTreeSet<String>,
}
impl Reader<'_> {
    fn mark(&mut self, v: Value) {
        self.marks.insert(v.to_string());
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

/// Observations of one attempt on its pool's dimensions.
#[allow(clippy::too_many_arguments)]
fn observe(
    table: &Table,
    reader: &mut Reader,
    j: &Journal,
    pool_index: usize,
    pool: &Pool,
    (request, attempt, route, payer, disposition, status): &(
        u64,
        u64,
        String,
        String,
        String,
        Option<u16>,
    ),
    endpoint: &str,
    out: &mut Vec<Observation>,
) {
    let (provider, funding, kinds) = endpoint_kind(endpoint).expect("pool member endpoint");
    let (request, attempt) = (*request, *attempt);
    let refused = disposition == "refused";
    if !(refused || disposition == "completed" || disposition == "uncertain") {
        return;
    }
    let Some(head) = reader.head(j, request, attempt) else {
        if refused || disposition == "completed" {
            reader.mark(json!({"kind":"no_head_evidence","job_id":j.job,"request":request,"attempt":attempt}));
        }
        return;
    };
    let Some(time) = head.time else {
        reader.mark(json!({"kind":"no_observation_time","job_id":j.job,"request":request,"attempt":attempt}));
        return;
    };
    let success = disposition == "completed" && (200..300).contains(&head.status);
    if !success && !refused && head.status < 400 {
        // An uncertain attempt after a success head is not a quota input.
        return;
    }
    let base = |source: String| {
        json!({"job_id":j.job,"request":request,"attempt":attempt,"route":route,"payer":payer,
            "endpoint":endpoint,"observed_ms":time,"artifact":head.artifact,"source":source})
    };
    let mut push =
        |dimension: &'static str, kind: Kind, utilization: Option<f64>, source: String| {
            out.push(Observation {
                pool: pool_index,
                dimension,
                time,
                job: j.job.clone(),
                request,
                attempt,
                kind,
                utilization,
                evidence: base(source),
            })
        };
    let dims = funding.dimensions();
    let source_kind = j.payer_sources.as_ref().and_then(|m| m.get(payer));
    let funding_reason = match source_kind {
        None => Some("funding_unverified"),
        Some(k) if !kinds.contains(&k.as_str()) => Some("funding_conflict"),
        _ => None,
    };
    let reason = funding_reason.or(match status {
        Some(s) if *s != head.status => Some("status_conflict"),
        _ => None,
    });
    if let Some(reason) = reason {
        for d in dims {
            push(d, Kind::Unknown(reason), None, format!("journal:{reason}"));
        }
        return;
    }
    let windows: Vec<WindowReading> = table.windows(provider, &head.headers, head.head_ms);
    let window = |d: &str| windows.iter().find(|w| w.dimension == d);
    if success {
        for d in dims {
            match window(d) {
                Some(w) if w.malformed.is_some() => push(
                    d,
                    Kind::Unknown(w.malformed.unwrap()),
                    None,
                    format!("header:{}", w.source),
                ),
                Some(w) if w.exhausted == Some(true) => push(
                    d,
                    Kind::Unknown("conflicting_header"),
                    None,
                    format!("header:{}", w.source),
                ),
                Some(w) => push(
                    d,
                    Kind::Available,
                    w.utilization,
                    format!("success;header:{}", w.source),
                ),
                None => push(d, Kind::Available, None, "success".into()),
            }
        }
        return;
    }
    let body = if refused {
        reader.artifact(
            j,
            &format!("coding-hosted-{request}-{attempt}.refusal.body"),
        )
    } else {
        reader.artifact(j, &format!("coding-hosted-{request}-{attempt}.stream.sse"))
    }
    .unwrap_or_default();
    let class = table.classify(provider, funding.label(), head.status, &body);
    let source = format!(
        "classification:{}",
        serde_json::to_value(class).unwrap().as_str().unwrap()
    );
    match (class, funding) {
        (Class::SubscriptionQuotaExhausted, Funding::Subscription) => {
            let exhausted: Vec<_> = windows
                .iter()
                .filter(|w| w.malformed.is_none())
                .filter(|w| w.exhausted == Some(true) || w.utilization.is_some_and(|u| u >= 1.0))
                .collect();
            for d in dims {
                if let Some(w) = window(d).filter(|w| w.malformed.is_some()) {
                    push(d, Kind::Unknown(w.malformed.unwrap()), None, format!("{source};header:{}", w.source));
                } else if exhausted.is_empty() {
                    push(d, Kind::Unknown("window_undetermined"), None, source.clone());
                } else if let Some(w) = exhausted.iter().find(|w| w.dimension == *d) {
                    let reset = w.reset.clone().unwrap_or(Reset::Unknown("reset_absent"));
                    push(d, Kind::Exhausted(reset), w.utilization, format!("{source};header:{}", w.source));
                }
            }
        }
        (Class::RateLimited, Funding::Api) => {
            let reset = table
                .retry_after(&head.headers, head.head_ms)
                .unwrap_or(Reset::Unknown("reset_absent"));
            push("api_rate", Kind::Exhausted(reset), None, format!("{source};header:{}", table.retry_after));
        }
        (Class::ApiCreditExhausted, Funding::Api) => {
            if pool.credit_cleared_ms.is_none_or(|c| time > c) {
                push("api_credit", Kind::OperatorAction, None, source);
            }
        }
        (Class::Overload | Class::AuthorizationFailure, _) => reader.mark(
            json!({"kind":"non_quota_failure","class":class,"job_id":j.job,"request":request,"attempt":attempt}),
        ),
        _ => {
            for d in dims {
                push(d, Kind::Unknown("unclassified_failure"), None, source.clone());
            }
        }
    }
}

fn derive(config: &Config, config_sha256: &str) -> Result<Value, String> {
    check(config)?;
    let table = provisional::table();
    let mut marks = BTreeSet::new();
    let mut journals = vec![];
    let mut observations = vec![];
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
                marks.insert(json!({"kind":reason,"root":root}).to_string());
                continue;
            }
        };
        journals.push(json!({"root":root,"read":true,"job_id":j.job,"journal_sha256":sha}));
        let mut reader = Reader { marks: &mut marks };
        for a in &j.attempts {
            let Some((route_payer, endpoint)) = j.routes.get(&a.2) else {
                reader.mark(
                    json!({"kind":"unknown_route","job_id":j.job,"request":a.0,"attempt":a.1}),
                );
                continue;
            };
            if *route_payer != a.3 {
                reader.mark(json!({"kind":"route_payer_mismatch","job_id":j.job,"request":a.0,"attempt":a.1}));
                continue;
            }
            let key = Member {
                payer: a.3.clone(),
                endpoint: endpoint.clone(),
            };
            if let Some(&i) = members.get(&key) {
                observe(
                    &table,
                    &mut reader,
                    &j,
                    i,
                    &config.pools[i],
                    a,
                    endpoint,
                    &mut observations,
                );
            }
        }
    }
    let now = config.now_ms;
    let mut pools = vec![];
    for (i, p) in config.pools.iter().enumerate() {
        let mut dimensions = vec![];
        for d in p.funding.dimensions() {
            let mut obs: Vec<&Observation> = observations
                .iter()
                .filter(|o| o.pool == i && o.dimension == *d && o.time <= now)
                .collect();
            obs.sort_by(|a, b| {
                (a.time, &a.job, a.request, a.attempt).cmp(&(b.time, &b.job, b.request, b.attempt))
            });
            dimensions.push(state(
                d,
                &obs,
                now,
                config.max_evidence_age_ms,
                p.low_thresholds.get(*d),
            ));
        }
        pools.push(json!({"id":p.id,"funding":p.funding.label(),"dimensions":dimensions}));
    }
    let marks: Vec<Value> = marks
        .iter()
        .map(|m| serde_json::from_str(m).unwrap())
        .collect();
    Ok(json!({
        "schema": OUTPUT_SCHEMA,
        "now_ms": now,
        "config_sha256": config_sha256,
        "table": {"version": table.version, "sha256": provisional::table_sha256(), "provisional": table.provisional},
        "journals": journals,
        "pools": pools,
        "marks": marks,
        "complete": marks.is_empty(),
    }))
}

/// One dimension's state from its observations, oldest first.
fn state(
    dimension: &str,
    obs: &[&Observation],
    now: u64,
    max_age: u64,
    threshold: Option<&f64>,
) -> Value {
    let entry = |status: &str,
                 reason: Option<&str>,
                 o: Option<&Observation>,
                 at: Option<u64>,
                 at_reason: Option<&str>| {
        let utilization = o
            .and_then(|o| o.utilization)
            .filter(|_| status != "unknown");
        let low = match (utilization, threshold) {
            (Some(u), Some(t)) => json!(u >= *t),
            _ => Value::Null,
        };
        json!({"dimension":dimension,"status":status,"reason":reason,"utilization":utilization,"low":low,
            "available_again_at":at,"available_again_reason":at_reason,"evidence":o.map(|o| &o.evidence)})
    };
    let Some(last) = obs.last().copied() else {
        return entry(
            "unknown",
            Some("no_evidence"),
            None,
            None,
            Some("status_unknown"),
        );
    };
    if obs
        .iter()
        .any(|o| o.time == last.time && !o.same_reading(last))
    {
        return entry(
            "unknown",
            Some("conflicting"),
            Some(last),
            None,
            Some("status_unknown"),
        );
    }
    if now.saturating_sub(last.time) > max_age {
        return entry(
            "unknown",
            Some("stale"),
            Some(last),
            None,
            Some("status_unknown"),
        );
    }
    match &last.kind {
        Kind::Available => entry("available", None, Some(last), None, None),
        Kind::Exhausted(Reset::At(t)) if now >= *t => {
            entry("reset_elapsed", None, Some(last), Some(*t), None)
        }
        Kind::Exhausted(Reset::At(t)) => entry("exhausted", None, Some(last), Some(*t), None),
        Kind::Exhausted(Reset::Unknown(r)) => entry("exhausted", None, Some(last), None, Some(r)),
        Kind::OperatorAction => entry("exhausted", None, Some(last), None, Some("operator_action")),
        Kind::Unknown(r) => entry("unknown", Some(r), Some(last), None, Some("status_unknown")),
    }
}
