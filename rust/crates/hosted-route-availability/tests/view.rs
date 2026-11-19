//! Route availability view over fixture journals shaped like the Host's
//! `pi-state.json` and its retained per-attempt artifacts.
use hosted_route_availability::{provisional, view};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs};

const NOW: u64 = 1_790_000_000_000;
const HOUR: u64 = 3_600_000;
const DAY: u64 = 24 * HOUR;
const SUB: (&str, &str, &str) = ("sub", "claude-sub", "https:anthropic-oauth");
const API: (&str, &str, &str) = ("api", "anthropic-api", "https:anthropic-api");
const H5S: &str = "anthropic-ratelimit-unified-5h-status";
const H5U: &str = "anthropic-ratelimit-unified-5h-utilization";
const H5R: &str = "anthropic-ratelimit-unified-5h-reset";
const H7S: &str = "anthropic-ratelimit-unified-7d-status";
const H7U: &str = "anthropic-ratelimit-unified-7d-utilization";
const H7R: &str = "anthropic-ratelimit-unified-7d-reset";

fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn error(kind: &str) -> Vec<u8> {
    json!({"type":"error","error":{"type":kind,"message":"fixture"}})
        .to_string()
        .into_bytes()
}

struct Job {
    dir: tempfile::TempDir,
    id: String,
    routes: Vec<(&'static str, &'static str, &'static str)>,
    sources: BTreeMap<String, String>,
    requests: BTreeMap<u64, Vec<Value>>,
    artifacts: Vec<Value>,
}
impl Job {
    fn new(id: &str, routes: &[(&'static str, &'static str, &'static str)]) -> Self {
        let sources = routes
            .iter()
            .map(|(_, payer, endpoint)| {
                let kind = match *endpoint {
                    "https:anthropic-oauth" => "claude-code-auth",
                    _ => "anthropic-api-key-env",
                };
                (payer.to_string(), kind.to_string())
            })
            .collect();
        Self {
            dir: tempfile::tempdir().unwrap(),
            id: id.into(),
            routes: routes.to_vec(),
            sources,
            requests: BTreeMap::new(),
            artifacts: vec![],
        }
    }
    fn root(&self) -> String {
        self.dir.path().to_str().unwrap().to_owned()
    }
    /// Write a file and register it with `recorded` (its digest by default).
    fn file_recorded(&mut self, name: &str, bytes: &[u8], recorded: Option<String>) {
        let path = self.dir.path().join(name);
        fs::write(&path, bytes).unwrap();
        self.artifacts
            .push(json!({"path":path,"sha256":recorded.unwrap_or_else(|| sha(bytes))}));
    }
    fn file(&mut self, name: &str, bytes: &[u8]) {
        self.file_recorded(name, bytes, None);
    }
    fn attempt(
        &mut self,
        r: u64,
        a: u64,
        route: (&str, &str, &str),
        disposition: &str,
        status: u16,
    ) {
        self.requests.entry(r).or_default().push(json!({
            "attempt":a,"route":route.0,"payer":route.1,"reserved_microusd":1000,
            "disposition":disposition,"http_status":status,"relay_sha256":null,
            "refusal_body_sha256":null,"settled_microusd":null,"reason":null}));
    }
    fn head_bytes(
        &self,
        r: u64,
        a: u64,
        status: u16,
        head_ms: u64,
        headers: &[(&str, &str)],
    ) -> Vec<u8> {
        json!({"jobId":self.id,"request":r,"attempt":a,"status":status,"headMs":head_ms,
            "headHeaders":headers.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>()})
        .to_string()
        .into_bytes()
    }
    fn head(&mut self, r: u64, a: u64, status: u16, head_ms: u64, headers: &[(&str, &str)]) {
        let b = self.head_bytes(r, a, status, head_ms, headers);
        self.file(&format!("coding-hosted-{r}-{a}.head.json"), &b);
    }
    #[allow(clippy::too_many_arguments)]
    fn refusal(
        &mut self,
        r: u64,
        a: u64,
        route: (&str, &str, &str),
        status: u16,
        head_ms: u64,
        headers: &[(&str, &str)],
        body: &[u8],
    ) {
        self.attempt(r, a, route, "refused", status);
        self.head(r, a, status, head_ms, headers);
        self.file(&format!("coding-hosted-{r}-{a}.refusal.body"), body);
    }
    fn success(
        &mut self,
        r: u64,
        a: u64,
        route: (&str, &str, &str),
        head_ms: u64,
        headers: &[(&str, &str)],
    ) {
        self.attempt(r, a, route, "completed", 200);
        self.head(r, a, 200, head_ms, headers);
    }
    fn save(&self) {
        let requests: serde_json::Map<String, Value> = self
            .requests
            .iter()
            .map(|(r, attempts)| {
                (
                    r.to_string(),
                    json!({"reserved_microusd":0,"attempts":attempts}),
                )
            })
            .collect();
        let journal = json!({
            "schema":"work-engine.pi-docker-custody.v1",
            "job":{"job_id":self.id,"state_dir":self.root(),"coding":{
                "schema":"work-engine.pi-coding-hosted.v1",
                "provider":{"hosted":{"max_attempts_per_request":2,"routes":
                    self.routes.iter().map(|(id, payer, endpoint)| json!({"id":id,"payer":payer,"endpoint":endpoint})).collect::<Vec<_>>()}}}},
            "coding":{"provider_requests":requests,
                "hosted_transport":{"transport":"https","payer_sources":self.sources}},
            "artifacts":self.artifacts,
        });
        fs::write(self.dir.path().join("pi-state.json"), journal.to_string()).unwrap();
    }
}

fn config(jobs: &[&Job], now: u64) -> Value {
    for j in jobs {
        j.save();
    }
    json!({
        "schema":"work-engine.hosted-route-availability-config.v1",
        "now_ms":now,
        "journal_roots":jobs.iter().map(|j| j.root()).collect::<Vec<_>>(),
        "max_evidence_age_ms":7 * DAY,
        "pools":[
            {"id":"claude-sub","funding":"subscription",
             "members":[{"payer":"claude-sub","endpoint":"https:anthropic-oauth"}],
             "low_thresholds":{"subscription_5h":0.9,"subscription_weekly":0.95}},
            {"id":"anthropic-api","funding":"api",
             "members":[{"payer":"anthropic-api","endpoint":"https:anthropic-api"}],
             "low_thresholds":{"api_rate":0.5}}
        ]
    })
}
fn run(config: &Value) -> Value {
    serde_json::from_slice(&view(&serde_json::to_vec(config).unwrap()).unwrap()).unwrap()
}
fn dim(out: &Value, pool: &str, dimension: &str) -> Value {
    out["pools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == pool)
        .unwrap()["dimensions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["dimension"] == dimension)
        .unwrap()
        .clone()
}
fn has_mark(out: &Value, kind: &str) -> bool {
    out["marks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["kind"] == kind)
}

/// 5h refused at NOW-1h (reset R5); weekly refused at NOW-30m (reset R7).
fn two_windows() -> (Job, u64, u64) {
    let r5 = NOW / 1000 + 1800;
    let r7 = NOW / 1000 + 5 * 86_400;
    let mut j = Job::new("job-a", &[SUB]);
    let (r5s, r7s) = (r5.to_string(), r7.to_string());
    j.refusal(
        1,
        1,
        SUB,
        429,
        NOW - HOUR,
        &[
            (H5S, "rejected"),
            (H5R, &r5s),
            (H7S, "allowed"),
            (H7U, "0.6"),
            (H7R, &r7s),
        ],
        &error("rate_limit_error"),
    );
    j.refusal(
        2,
        1,
        SUB,
        429,
        NOW - HOUR / 2,
        &[
            (H5S, "allowed"),
            (H7S, "rejected"),
            (H7U, "1.0"),
            (H7R, &r7s),
        ],
        &error("rate_limit_error"),
    );
    (j, r5 * 1000, r7 * 1000)
}

#[test]
fn windows_are_tracked_separately_each_with_its_own_reset() {
    let (j, r5, r7) = two_windows();
    let out = run(&config(&[&j], NOW));
    assert!(out["complete"].as_bool().unwrap(), "{out}");
    let h5 = dim(&out, "claude-sub", "subscription_5h");
    assert_eq!(h5["status"], "exhausted");
    assert_eq!(h5["available_again_at"], r5);
    assert_eq!(h5["evidence"]["request"], 1);
    assert_eq!(h5["evidence"]["observed_ms"], NOW - HOUR);
    assert_eq!(
        h5["evidence"]["source"],
        format!("classification:subscription_quota_exhausted;header:{H5S}")
    );
    let wk = dim(&out, "claude-sub", "subscription_weekly");
    assert_eq!(wk["status"], "exhausted");
    assert_eq!(wk["available_again_at"], r7);
    assert_eq!(wk["evidence"]["request"], 2);
    assert_eq!(wk["utilization"], 1.0);
    assert_eq!(wk["low"], true);
    // The 5h reset passes; the weekly window is still exhausted.
    let out = run(&config(&[&j], r5 + 1));
    assert_eq!(
        dim(&out, "claude-sub", "subscription_5h")["status"],
        "reset_elapsed"
    );
    assert_eq!(
        dim(&out, "claude-sub", "subscription_5h")["available_again_at"],
        r5
    );
    assert_eq!(
        dim(&out, "claude-sub", "subscription_weekly")["status"],
        "exhausted"
    );
}

#[test]
fn only_a_later_success_makes_a_window_available_and_low_follows_the_threshold() {
    let (mut j, r5, _) = two_windows();
    // A reset passing is not availability.
    let out = run(&config(&[&j], r5 + 2 * 60_000));
    assert_eq!(
        dim(&out, "claude-sub", "subscription_5h")["status"],
        "reset_elapsed"
    );
    j.success(
        3,
        1,
        SUB,
        r5 + 60_000,
        &[
            (H5S, "allowed"),
            (H5U, "0.95"),
            (H7S, "allowed"),
            (H7U, "0.5"),
        ],
    );
    let mut api = Job::new("job-api", &[API]);
    api.success(1, 1, API, r5 + 60_000, &[]);
    let out = run(&config(&[&j, &api], r5 + 2 * 60_000));
    let h5 = dim(&out, "claude-sub", "subscription_5h");
    assert_eq!(
        (h5["status"].as_str(), h5["utilization"].as_f64()),
        (Some("available"), Some(0.95))
    );
    assert_eq!(h5["low"], true, "0.95 >= 0.9");
    assert_eq!(h5["available_again_at"], Value::Null);
    let wk = dim(&out, "claude-sub", "subscription_weekly");
    assert_eq!(
        (wk["status"].as_str(), wk["low"].as_bool()),
        (Some("available"), Some(false))
    );
    assert_eq!(wk["evidence"]["request"], 3);
    // Utilization unknown: low is unknown even with a threshold.
    let rate = dim(&out, "anthropic-api", "api_rate");
    assert_eq!(rate["status"], "available");
    assert_eq!(
        (rate["utilization"].clone(), rate["low"].clone()),
        (Value::Null, Value::Null)
    );
}

#[test]
fn api_credit_exhaustion_has_no_reset_until_the_operator_clears_it() {
    let mut j = Job::new("job-api", &[API]);
    j.refusal(
        1,
        1,
        API,
        402,
        NOW - HOUR,
        &[("retry-after", "60")],
        &error("billing_error"),
    );
    let mut c = config(&[&j], NOW + 6 * DAY);
    let out = run(&c);
    let credit = dim(&out, "anthropic-api", "api_credit");
    assert_eq!(credit["status"], "exhausted");
    assert_eq!(credit["available_again_at"], Value::Null);
    assert_eq!(credit["available_again_reason"], "operator_action");
    assert_eq!(
        credit["evidence"]["source"],
        "classification:api_credit_exhausted"
    );
    assert_eq!(
        dim(&out, "anthropic-api", "api_rate")["reason"],
        "no_evidence"
    );
    // Cleared by the operator after the observation: no evidence remains.
    c["pools"][1]["credit_cleared_ms"] = json!(NOW - HOUR / 2);
    let credit = dim(&run(&c), "anthropic-api", "api_credit");
    assert_eq!(
        (credit["status"].as_str(), credit["reason"].as_str()),
        (Some("unknown"), Some("no_evidence"))
    );
}

#[test]
fn unknown_stale_malformed_and_conflicting_evidence_is_unknown() {
    // No evidence.
    let empty = Job::new("job-empty", &[SUB]);
    let out = run(&config(&[&empty], NOW));
    let h5 = dim(&out, "claude-sub", "subscription_5h");
    assert_eq!(
        (h5["status"].as_str(), h5["reason"].as_str()),
        (Some("unknown"), Some("no_evidence"))
    );
    assert_eq!(h5["available_again_reason"], "status_unknown");
    // Stale: older than the operator's bound.
    let mut old = Job::new("job-old", &[SUB]);
    let reset = (NOW / 1000 + 3600).to_string();
    old.refusal(
        1,
        1,
        SUB,
        429,
        NOW - 8 * DAY,
        &[(H5S, "rejected"), (H5R, &reset)],
        &error("rate_limit_error"),
    );
    let h5 = dim(&run(&config(&[&old], NOW)), "claude-sub", "subscription_5h");
    assert_eq!(
        (h5["status"].as_str(), h5["reason"].as_str()),
        (Some("unknown"), Some("stale"))
    );
    assert_eq!(h5["available_again_at"], Value::Null);
    // Malformed header.
    let mut bad = Job::new("job-bad", &[SUB]);
    bad.success(1, 1, SUB, NOW - HOUR, &[(H5U, "ninety"), (H7U, "0.4")]);
    let out = run(&config(&[&bad], NOW));
    let h5 = dim(&out, "claude-sub", "subscription_5h");
    assert_eq!(
        (h5["status"].as_str(), h5["reason"].as_str()),
        (Some("unknown"), Some("malformed_header"))
    );
    assert_eq!(h5["low"], Value::Null);
    assert_eq!(
        dim(&out, "claude-sub", "subscription_weekly")["status"],
        "available"
    );
    // Conflicting: two jobs in the pool disagree at the same time.
    let mut a = Job::new("job-a", &[SUB]);
    a.success(1, 1, SUB, NOW - HOUR, &[(H5U, "0.5")]);
    let mut b = Job::new("job-b", &[SUB]);
    b.refusal(
        1,
        1,
        SUB,
        429,
        NOW - HOUR,
        &[(H5S, "rejected"), (H5R, &reset)],
        &error("rate_limit_error"),
    );
    let h5 = dim(
        &run(&config(&[&a, &b], NOW)),
        "claude-sub",
        "subscription_5h",
    );
    assert_eq!(
        (h5["status"].as_str(), h5["reason"].as_str()),
        (Some("unknown"), Some("conflicting"))
    );
    // A success head that says the window is rejected conflicts with itself.
    let mut c = Job::new("job-c", &[SUB]);
    c.success(1, 1, SUB, NOW - HOUR, &[(H5S, "rejected")]);
    let h5 = dim(&run(&config(&[&c], NOW)), "claude-sub", "subscription_5h");
    assert_eq!(h5["reason"], "conflicting_header");
    // A malformed journal is skipped and marked; its pools stay unknown.
    let broken = tempfile::tempdir().unwrap();
    fs::write(broken.path().join("pi-state.json"), b"{not json").unwrap();
    let mut cfg = config(&[], NOW);
    cfg["journal_roots"] = json!([broken.path()]);
    let out = run(&cfg);
    assert_eq!(out["journals"][0]["read"], false);
    assert_eq!(out["journals"][0]["reason"], "journal_malformed");
    assert!(has_mark(&out, "journal_malformed") && out["complete"] == false);
    assert_eq!(
        dim(&out, "claude-sub", "subscription_5h")["status"],
        "unknown"
    );
}

#[test]
fn relative_reset_without_head_time_is_unknown() {
    // An uncertain relayed non-success head: relay.json carries the kept
    // headers but no head time; the error body is the retained stream.
    let relayed = |with_head: bool| {
        let mut j = Job::new("job-api", &[API]);
        j.attempt(1, 1, API, "uncertain", 429);
        let relay = json!({"status":429,"headHeaders":[["retry-after","30"]],
            "openedMs":NOW - HOUR,"upstreamEndMs":NOW - HOUR + 5});
        j.file("coding-hosted-1-1.relay.json", relay.to_string().as_bytes());
        j.file("coding-hosted-1-1.stream.sse", &error("rate_limit_error"));
        if with_head {
            j.head(1, 1, 429, NOW - HOUR, &[("retry-after", "30")]);
        }
        j
    };
    let j = relayed(false);
    let rate = dim(
        &run(&config(&[&j], NOW - HOUR + 10)),
        "anthropic-api",
        "api_rate",
    );
    assert_eq!(rate["status"], "exhausted");
    assert_eq!(rate["available_again_at"], Value::Null);
    assert_eq!(
        rate["available_again_reason"],
        "relative_reset_without_head_time"
    );
    assert_eq!(rate["evidence"]["artifact"], "relay.json");
    let j = relayed(true);
    let rate = dim(
        &run(&config(&[&j], NOW - HOUR + 10)),
        "anthropic-api",
        "api_rate",
    );
    assert_eq!(rate["available_again_at"], NOW - HOUR + 30_000);
    assert_eq!(rate["evidence"]["artifact"], "head.json");
}

#[test]
fn two_jobs_declared_in_one_pool_share_state() {
    let reset = (NOW / 1000 + 1800).to_string();
    let mut a = Job::new("job-a", &[SUB]);
    a.refusal(
        1,
        1,
        SUB,
        429,
        NOW - HOUR,
        &[(H5S, "rejected"), (H5R, &reset)],
        &error("rate_limit_error"),
    );
    let mut b = Job::new("job-b", &[SUB]);
    b.success(1, 1, SUB, NOW - 2 * HOUR, &[(H5U, "0.3")]);
    // Another payer, not declared in any pool, has no effect.
    let other = ("sub2", "claude-other", "https:anthropic-oauth");
    let mut c = Job::new("job-c", &[other]);
    c.success(1, 1, other, NOW - HOUR / 4, &[(H5U, "0.1")]);
    let out = run(&config(&[&a, &b, &c], NOW));
    let h5 = dim(&out, "claude-sub", "subscription_5h");
    assert_eq!(h5["status"], "exhausted");
    assert_eq!(h5["evidence"]["job_id"], "job-a");
    // Job B's later success in the same pool makes it available for both.
    b.success(2, 1, SUB, NOW - HOUR / 2, &[(H5U, "0.4")]);
    let h5 = dim(
        &run(&config(&[&a, &b, &c], NOW)),
        "claude-sub",
        "subscription_5h",
    );
    assert_eq!(
        (h5["status"].as_str(), h5["evidence"]["job_id"].as_str()),
        (Some("available"), Some("job-b"))
    );
    // Declared in the pool, job C's evidence is shared too.
    let mut cfg = config(&[&a, &b, &c], NOW);
    cfg["pools"][0]["members"]
        .as_array_mut()
        .unwrap()
        .push(json!({"payer":"claude-other","endpoint":"https:anthropic-oauth"}));
    let h5 = dim(&run(&cfg), "claude-sub", "subscription_5h");
    assert_eq!(h5["evidence"]["job_id"], "job-c");
}

#[test]
fn same_inputs_and_now_give_byte_identical_output() {
    let (j, _, _) = two_windows();
    let mut api = Job::new("job-api", &[API]);
    api.refusal(1, 1, API, 402, NOW - HOUR, &[], &error("billing_error"));
    let cfg = config(&[&j, &api], NOW);
    let bytes = serde_json::to_vec(&cfg).unwrap();
    let first = view(&bytes).unwrap();
    assert_eq!(view(&bytes).unwrap(), first, "re-read");
    // Root order and repetition do not change the state.
    let mut reordered = cfg.clone();
    reordered["journal_roots"] = json!([api.root(), j.root(), api.root()]);
    let strip = |b: &[u8]| {
        let mut v: Value = serde_json::from_slice(b).unwrap();
        v.as_object_mut().unwrap().remove("config_sha256");
        v
    };
    assert_eq!(
        strip(&view(&serde_json::to_vec(&reordered).unwrap()).unwrap()),
        strip(&first)
    );
    // The binary prints the same bytes.
    let path = j.dir.path().join("view-config.json");
    fs::write(&path, &bytes).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_hosted-route-availability"))
        .arg(&path)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(out.stdout, first);
}

#[test]
fn an_unclassified_failure_is_never_a_known_window() {
    let reset = (NOW / 1000 + 1800).to_string();
    let mut j = Job::new("job-a", &[SUB, API]);
    // Rejected window headers, but an error the table does not know.
    j.refusal(
        1,
        1,
        SUB,
        429,
        NOW - HOUR,
        &[(H5S, "rejected"), (H5R, &reset)],
        &error("brand_new_error"),
    );
    // An uncertain non-success relay with no classifiable body.
    j.attempt(2, 1, API, "uncertain", 500);
    j.head(2, 1, 500, NOW - HOUR, &[]);
    j.file("coding-hosted-2-1.stream.sse", b"upstream exploded");
    let out = run(&config(&[&j], NOW));
    for (pool, d) in [
        ("claude-sub", "subscription_5h"),
        ("claude-sub", "subscription_weekly"),
        ("anthropic-api", "api_rate"),
        ("anthropic-api", "api_credit"),
    ] {
        let s = dim(&out, pool, d);
        assert_eq!(
            (s["status"].as_str(), s["reason"].as_str()),
            (Some("unknown"), Some("unclassified_failure")),
            "{d}"
        );
        assert_eq!(s["available_again_at"], Value::Null);
    }
    // A known class with no identifiable window is not a guessed window.
    let mut k = Job::new("job-k", &[SUB]);
    k.refusal(
        1,
        1,
        SUB,
        429,
        NOW - HOUR,
        &[("retry-after", "60")],
        &error("rate_limit_error"),
    );
    let h5 = dim(&run(&config(&[&k], NOW)), "claude-sub", "subscription_5h");
    assert_eq!(
        (h5["status"].as_str(), h5["reason"].as_str()),
        (Some("unknown"), Some("window_undetermined"))
    );
}

#[test]
fn an_artifact_whose_digest_mismatches_is_ignored_and_marked() {
    let reset = (NOW / 1000 + 1800).to_string();
    // The head evidence does not match its recorded digest: no head, no time.
    let mut j = Job::new("job-a", &[SUB]);
    j.attempt(1, 1, SUB, "refused", 429);
    let head = j.head_bytes(1, 1, 429, NOW - HOUR, &[(H5S, "rejected"), (H5R, &reset)]);
    j.file_recorded("coding-hosted-1-1.head.json", &head, Some(sha(b"other")));
    j.file("coding-hosted-1-1.refusal.body", &error("rate_limit_error"));
    let out = run(&config(&[&j], NOW));
    assert!(
        has_mark(&out, "artifact_digest_mismatch") && out["complete"] == false,
        "{out}"
    );
    assert_eq!(
        dim(&out, "claude-sub", "subscription_5h")["reason"],
        "no_evidence"
    );
    // A mismatched refusal body is not classified.
    let mut k = Job::new("job-k", &[SUB]);
    k.attempt(1, 1, SUB, "refused", 429);
    k.head(1, 1, 429, NOW - HOUR, &[(H5S, "rejected"), (H5R, &reset)]);
    k.file_recorded(
        "coding-hosted-1-1.refusal.body",
        &error("rate_limit_error"),
        Some(sha(b"other")),
    );
    let out = run(&config(&[&k], NOW));
    assert!(has_mark(&out, "artifact_digest_mismatch"));
    assert_eq!(
        dim(&out, "claude-sub", "subscription_5h")["reason"],
        "unclassified_failure"
    );
    // Head evidence naming another attempt is not evidence.
    let mut m = Job::new("job-m", &[SUB]);
    m.attempt(1, 1, SUB, "refused", 429);
    let other = m.head_bytes(1, 2, 429, NOW - HOUR, &[(H5S, "rejected")]);
    m.file("coding-hosted-1-1.head.json", &other);
    m.file("coding-hosted-1-1.refusal.body", &error("rate_limit_error"));
    let out = run(&config(&[&m], NOW));
    assert!(has_mark(&out, "invalid_head_evidence"));
    assert_eq!(
        dim(&out, "claude-sub", "subscription_5h")["reason"],
        "no_evidence"
    );
}

#[test]
fn the_classification_table_is_pinned_and_reported() {
    let t = provisional::table();
    assert!(t.provisional && t.version.contains("provisional"));
    assert_eq!(
        provisional::table_sha256(),
        "cf10be0e78f064a56b232cb7c48d540a4d7ad176bef13b3fed131aaa9a689d1e",
        "a table change must update this pin"
    );
    let out = run(&config(&[], NOW));
    assert_eq!(out["table"]["sha256"], provisional::table_sha256());
    assert_eq!(out["table"]["provisional"], true);
}

#[test]
fn an_invalid_config_is_refused() {
    let mut c = config(&[], NOW);
    c["pools"][0]["members"][0]["endpoint"] = json!("https:anthropic-api");
    assert!(
        view(&serde_json::to_vec(&c).unwrap())
            .unwrap_err()
            .contains("is not subscription")
    );
    let mut c = config(&[], NOW);
    c["journal_roots"] = json!(["relative/root"]);
    assert!(view(&serde_json::to_vec(&c).unwrap()).is_err());
}
