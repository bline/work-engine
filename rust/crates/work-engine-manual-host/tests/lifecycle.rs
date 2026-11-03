use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
use tempfile::TempDir;
use work_engine_manual_host::{Host, Request, SCHEMA, digest};
const BIN: &str = env!("CARGO_BIN_EXE_work-engine-manual-host");
fn artifact(p: &Path) -> Value {
    json!({"path":p,"sha256":digest(&fs::read(p).unwrap())})
}
struct Fixture {
    dir: TempDir,
    request: Value,
}
impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let p = dir.path();
        for n in [
            "role_host.py",
            "scope_budget.py",
            "child_context.py",
            "usage_ledger.py",
            "chatboard.py",
            "profile.py",
        ] {
            fs::write(p.join(n), include_str!("adapter.py")).unwrap();
        }
        fs::write(p.join("context-binary"), "fixture").unwrap();
        fs::write(
            p.join("authority.md"),
            "explicit local fixture authorization",
        )
        .unwrap();
        fs::write(p.join("packet.md"), "produce the authorized result").unwrap();
        fs::write(
            p.join("root.jsonl"),
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"root\"}}\n",
        )
        .unwrap();
        fs::write(
            p.join("assignment.json"),
            r#"{"scope_budget":{"objective_id":"objective","bindings":[],"sessions":[{"session_id":"root","provider":"codex"}]}}"#,
        )
        .unwrap();
        fs::write(
            p.join("config.json"),
            json!({"sid":"child","rollout":p.join("child.jsonl"),"result":p.join("result.json")})
                .to_string(),
        )
        .unwrap();
        let manifest: serde_json::Map<String, Value> = [
            "role_host.py",
            "scope_budget.py",
            "child_context.py",
            "usage_ledger.py",
            "chatboard.py",
            "profile.py",
        ]
        .into_iter()
        .map(|n| (n.to_string(), json!(digest(&fs::read(p.join(n)).unwrap()))))
        .collect();
        fs::write(
            p.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let request = json!({"schema":SCHEMA,"operation_id":"one","revision":1,"state_dir":p.join("state"),"repository":p,"cwd":p,
   "authority":artifact(&p.join("authority.md")),"packet":artifact(&p.join("packet.md")),"assignment":p.join("assignment.json"),"objective_id":"objective",
   "root":{"session_id":"root","rollout":p.join("root.jsonl")},
   "tools":{"python":"/usr/bin/python3","bundle_manifest":artifact(&p.join("manifest.json")),"role_host":artifact(&p.join("role_host.py")),"scope_budget":artifact(&p.join("scope_budget.py")),
    "usage_ledger":artifact(&p.join("usage_ledger.py")),"child_context":artifact(&p.join("child_context.py")),"context_binary":artifact(&p.join("context-binary")),"chatboard":artifact(&p.join("chatboard.py")),"task_profile":artifact(&p.join("profile.py"))},
   "model":"fixture-model","effort":"medium","service_tier":"default","sandbox":"read-only","require_read_only":true,"bootstrap_context":true,
   "result_path":p.join("result.json"),"artifacts":[],"custody":[],"claim":{"resource":"manual-host:objective:one","claim_id":"fixture-claim","ttl_seconds":600},"profile":null,"poll_seconds":1,"wait_seconds":2});
        Self { dir, request }
    }
    fn config(&self, key: &str, v: Value) {
        let p = self.dir.path().join("config.json");
        let mut cfg: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
        cfg[key] = v;
        fs::write(p, cfg.to_string()).unwrap();
    }
    fn count(&self, key: &str) -> u64 {
        let c: Value =
            serde_json::from_slice(&fs::read(self.dir.path().join("config.json")).unwrap())
                .unwrap();
        c[key].as_u64().unwrap_or(0)
    }
    fn invoke(&self, args: &[&str]) -> std::process::Output {
        Command::new(BIN).args(args).output().unwrap()
    }
    fn dispatch(&self) -> std::process::Output {
        let p = self.dir.path().join("request.json");
        fs::write(&p, self.request.to_string()).unwrap();
        self.invoke(&["dispatch", "--request", p.to_str().unwrap()])
    }
    fn collect(&self) -> std::process::Output {
        self.invoke(&[
            "collect",
            "--state",
            self.dir.path().join("state").to_str().unwrap(),
            "--operation",
            "one",
        ])
    }
    fn state(&self) -> Value {
        serde_json::from_slice(&fs::read(self.dir.path().join("state/state.json")).unwrap())
            .unwrap()
    }
    fn close(&self, ack: &str, objective: bool) -> std::process::Output {
        let c = json!({"schema":SCHEMA,"operation_id":"one","authority":self.request["authority"],"ack_receipt":ack,"objective_end":objective,"disposition":"accepted","reason":"explicit fixture closure"});
        let p = self.dir.path().join("close.json");
        fs::write(&p, c.to_string()).unwrap();
        self.invoke(&[
            "close",
            "--state",
            self.dir.path().join("state").to_str().unwrap(),
            "--request",
            p.to_str().unwrap(),
        ])
    }
}
fn success(o: std::process::Output) -> Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
#[test]
fn normal_collection_ack_and_explicit_objective_close() {
    let f = Fixture::new();
    success(f.dispatch());
    assert_eq!(f.count("starts"), 1);
    let receipt = success(f.collect());
    assert_eq!(receipt["summary"]["phase"], "collected");
    assert_eq!(receipt["delivery"], "pending");
    assert!(f.state()["operations"]["one"]["evidence"]["settings"].is_object());
    let again = success(f.collect());
    assert_eq!(again["receipt_id"], receipt["receipt_id"]);
    assert!(
        serde_json::from_slice::<Value>(&fs::read(f.dir.path().join("assignment.json")).unwrap())
            .unwrap()["scope_budget"]["closed_at"]
            .is_null()
    );
    let closed = success(f.close(receipt["receipt_id"].as_str().unwrap(), true));
    assert_eq!(closed["summary"]["phase"], "closed");
    assert_eq!(f.count("closes"), 1);
    assert_eq!(f.count("releases"), 1);
    let repeated = success(f.close(receipt["receipt_id"].as_str().unwrap(), true));
    assert_eq!(repeated["receipt_id"], closed["receipt_id"]);
    assert_eq!(f.count("closes"), 1);
}
#[test]
fn duplicate_and_conflicting_input_cannot_start_again() {
    let mut f = Fixture::new();
    success(f.dispatch());
    success(f.dispatch());
    assert_eq!(f.count("creates"), 1);
    assert_eq!(f.count("starts"), 1);
    f.request["revision"] = json!(2);
    assert!(!f.dispatch().status.success());
    assert_eq!(f.count("starts"), 1);
}
#[test]
fn unknown_external_effects_never_retry() {
    for (fault, phase, count) in [
        ("unknown_create", "create_unknown", "creates"),
        ("unknown_start", "start_unknown", "starts"),
    ] {
        let f = Fixture::new();
        f.config(fault, json!(true));
        success(f.dispatch());
        success(f.dispatch());
        success(f.collect());
        assert_eq!(f.count(count), 1);
        assert_eq!(f.state()["operations"]["one"]["phase"], phase);
    }
}
#[test]
fn crashes_preserve_unknown_effect_and_process_custody() {
    for (fault, phase, key) in [
        ("crash_create", "create_unknown", "creates"),
        ("crash_start", "start_unknown", "starts"),
    ] {
        let f = Fixture::new();
        f.config(fault, json!(true));
        assert!(!f.dispatch().status.success());
        std::thread::sleep(Duration::from_millis(100));
        success(f.collect());
        assert_eq!(f.state()["operations"]["one"]["phase"], phase);
        success(f.dispatch());
        assert_eq!(f.count(key), 1);
    }
}
#[test]
fn absent_readonly_grant_or_missing_bootstrap_refuses_paid_turn() {
    let f = Fixture::new();
    f.config("no_grant", json!(true));
    success(f.dispatch());
    assert_eq!(f.count("starts"), 0);
    let mut f = Fixture::new();
    f.request["bootstrap_context"] = json!(false);
    success(f.dispatch());
    assert_eq!(f.count("starts"), 0);
}
#[test]
fn unavailable_or_exhausted_budget_stops_before_create() {
    for (k, v) in [
        ("budget_fail", json!(true)),
        ("headroom", json!(0)),
        ("context_fail", json!(true)),
    ] {
        let f = Fixture::new();
        f.config(k, v);
        success(f.dispatch());
        assert_eq!(f.count("creates"), 0);
    }
}
#[test]
fn missing_result_retains_claim_and_unclosed_objective() {
    let f = Fixture::new();
    f.config("missing_result", json!(true));
    success(f.dispatch());
    let v = success(f.collect());
    assert_eq!(v["summary"]["phase"], "terminal");
    assert!(
        !f.close(v["receipt_id"].as_str().unwrap(), true)
            .status
            .success()
    );
    assert_eq!(f.count("releases"), 0);
}
#[test]
fn changed_artifact_invalidates_prior_collection_without_rehashing_success() {
    let f = Fixture::new();
    f.config("with_artifact", json!(true));
    success(f.dispatch());
    success(f.collect());
    fs::write(f.dir.path().join("output.txt"), "changed").unwrap();
    assert!(!f.collect().status.success());
}
#[test]
fn observer_failure_is_unavailable_and_does_not_fake_completion() {
    let f = Fixture::new();
    f.config("observer_fail", json!(true));
    success(f.dispatch());
    let v = success(f.collect());
    assert_eq!(v["summary"]["phase"], "running");
    assert_eq!(
        f.state()["operations"]["one"]["evidence"]["turn"]["status"],
        "unavailable"
    );
}
#[test]
fn concurrent_executors_admit_one_child() {
    let f = Fixture::new();
    f.config("prepare_delay", json!(0.4));
    let p = f.dir.path().join("request.json");
    fs::write(&p, f.request.to_string()).unwrap();
    let mut one = Command::new(BIN)
        .args(["dispatch", "--request", p.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let two = f.dispatch();
    assert!(!two.status.success());
    assert!(one.wait().unwrap().success());
    assert_eq!(f.count("starts"), 1);
}
#[test]
fn incompatible_binary_and_unknown_ack_are_refused() {
    let f = Fixture::new();
    success(f.dispatch());
    let v = success(f.collect());
    assert!(!f.close("unknown", false).status.success());
    let path = f.dir.path().join("state/state.json");
    let mut s = f.state();
    s["binary_sha256"] = json!("other");
    fs::write(path, s.to_string()).unwrap();
    assert!(!f.collect().status.success());
    assert!(v["receipt_id"].is_string());
}
#[test]
fn actual_subprocess_timeout_collects_descendants() {
    let f = Fixture::new();
    success(f.dispatch());
    let r: Request = serde_json::from_value(f.request.clone()).unwrap();
    let state_path = r.state_dir.join("state.json");
    let mut state = f.state();
    state["binary_sha256"] = json!(digest(&fs::read(std::env::current_exe().unwrap()).unwrap()));
    fs::write(state_path, state.to_string()).unwrap();
    let mut h = Host::open(&r.state_dir, None).unwrap();
    let pidpath = f.dir.path().join("sleep.pid");
    let code = format!(
        "import subprocess,time; p=subprocess.Popen(['sleep','60']);open({:?},'w').write(str(p.pid));time.sleep(60)",
        pidpath.to_str().unwrap()
    );
    assert!(
        h.run(
            "one",
            "timeout",
            vec!["/usr/bin/python3".into(), "-c".into(), code],
            Duration::from_millis(150)
        )
        .is_err()
    );
    let rec = h.state.operations["one"].processes.last().unwrap();
    assert!(rec.timed_out);
    assert!(rec.leftovers.is_empty());
    let pid = fs::read_to_string(pidpath).unwrap();
    let stat = fs::read_to_string(format!("/proc/{}/stat", pid.trim())).unwrap_or_default();
    assert!(
        stat.is_empty()
            || stat
                .split(')')
                .nth(1)
                .unwrap()
                .trim_start()
                .starts_with('Z')
    );
}
#[test]
fn artifact_digest_conflict_prevents_admission() {
    let mut f = Fixture::new();
    f.request["packet"]["sha256"] = json!("bad");
    assert!(!f.dispatch().status.success());
    assert_eq!(f.count("creates"), 0);
}
#[test]
fn profile_effect_is_recorded_without_exposing_sidecar() {
    let mut f = Fixture::new();
    f.request["profile"] =
        json!({"run_dir":f.dir.path(),"episode":"already-enrolled","role":"builder"});
    success(f.dispatch());
    assert_eq!(f.count("profiles"), 1);
}
#[test]
fn retained_delivery_reuses_verified_collected_thread_without_create() {
    let mut f = Fixture::new();
    success(f.dispatch());
    success(f.collect());
    f.request["operation_id"] = json!("two");
    f.request["claim"] = Value::Null;
    f.request["retained"] =
        json!({"session_id":"child","rollout":f.dir.path().join("child.jsonl")});
    f.request["bootstrap_context"] = json!(false);
    f.request["require_read_only"] = json!(false);
    f.request["result_path"] = json!(f.dir.path().join("second-result.json"));
    f.config("result", f.request["result_path"].clone());
    let v = success(f.dispatch());
    assert_eq!(v["summary"]["phase"], "running");
    assert_eq!(f.count("creates"), 1);
    assert_eq!(f.count("starts"), 2);
}
#[test]
fn explicitly_abandoned_preexecution_refusal_can_close_without_acceptance() {
    let f = Fixture::new();
    f.config("no_grant", json!(true));
    let r = success(f.dispatch());
    let c = json!({"schema":SCHEMA,"operation_id":"one","authority":f.request["authority"],"ack_receipt":r["receipt_id"],"objective_end":false,"disposition":"abandoned","reason":"explicitly abandon unsupported realization"});
    let p = f.dir.path().join("close.json");
    fs::write(&p, c.to_string()).unwrap();
    let v = success(f.invoke(&[
        "close",
        "--state",
        f.dir.path().join("state").to_str().unwrap(),
        "--request",
        p.to_str().unwrap(),
    ]));
    assert_eq!(v["summary"]["phase"], "closed");
    assert_eq!(f.count("starts"), 0);
    assert_eq!(f.count("releases"), 1);
    assert_eq!(f.count("closes"), 0);
}
#[test]
fn mutation_during_snapshot_collection_is_a_conflict() {
    let f = Fixture::new();
    f.config("with_artifact", json!(true));
    success(f.dispatch());
    let result = f.dir.path().join("result.json");
    let mut doc: Value = serde_json::from_slice(&fs::read(&result).unwrap()).unwrap();
    let original_sha = doc["artifacts"][0]["sha256"].as_str().unwrap().to_string();
    for i in 0..6 {
        let path = f.dir.path().join(format!("large-{i}"));
        let content = vec![i as u8; 6 * 1024 * 1024];
        fs::write(&path, content).unwrap();
        doc["artifacts"]
            .as_array_mut()
            .unwrap()
            .push(artifact(&path));
    }
    fs::write(result, doc.to_string()).unwrap();
    let mut child = Command::new(BIN)
        .args([
            "collect",
            "--state",
            f.dir.path().join("state").to_str().unwrap(),
            "--operation",
            "one",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let snapshot = f.dir.path().join("state/snapshots").join(original_sha);
    let started = std::time::Instant::now();
    while !snapshot.exists() {
        assert!(started.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(1));
    }
    fs::write(f.dir.path().join("output.txt"), "concurrently changed").unwrap();
    assert!(!child.wait().unwrap().success());
}
#[test]
fn ended_turn_failure_is_collected_without_semantic_success() {
    let f = Fixture::new();
    f.config("turn_status", json!("failed"));
    success(f.dispatch());
    let v = success(f.collect());
    assert_eq!(v["summary"]["phase"], "collected");
    assert_eq!(
        f.state()["operations"]["one"]["evidence"]["turn"]["status"],
        "failed"
    );
    assert_eq!(v["summary"]["acceptance"], "root_owned");
}
#[test]
fn settings_mismatch_is_not_a_configuration_acceptance_claim() {
    let f = Fixture::new();
    f.config("observed_tier", json!("priority"));
    success(f.dispatch());
    let v = success(f.collect());
    assert!(
        v["summary"]["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s.as_str().unwrap().contains("settings"))
    );
}
#[test]
fn lost_observation_output_and_reader_failure_preserve_events_and_cursor() {
    let f = Fixture::new();
    success(f.dispatch());
    f.config("turn_status", json!("inProgress"));
    f.config("context_fail", json!(true));
    let first = success(f.collect());
    let state = f.state();
    assert!(
        !state["operations"]["one"]["observer_events"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    f.config("budget_fail", json!(true));
    success(f.collect());
    assert_eq!(
        f.state()["operations"]["one"]["budget_cursor"],
        "stable-cursor"
    );
    f.config("budget_fail", json!(false));
    f.config("context_fail", json!(false));
    let later = success(f.collect());
    let state = f.state();
    assert!(
        state["operations"]["one"]["observer_events"]
            .as_object()
            .unwrap()
            .len()
            >= 2
    );
    let dir = f.dir.path().join("state");
    success(f.invoke(&[
        "collect",
        "--state",
        dir.to_str().unwrap(),
        "--operation",
        "one",
        "--ack-receipt",
        first["receipt_id"].as_str().unwrap(),
    ]));
    assert!(
        !f.state()["acknowledged_events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_ne!(later["receipt_id"], Value::Null);
}
#[test]
fn changed_imported_dependency_refuses_further_execution() {
    let f = Fixture::new();
    success(f.dispatch());
    fs::write(f.dir.path().join("role_host.py"), "changed").unwrap();
    let receipt = success(f.collect());
    assert_eq!(receipt["summary"]["phase"], "running");
    assert_eq!(
        f.state()["operations"]["one"]["evidence"]["turn"]["status"],
        "unavailable"
    );
    assert_eq!(f.count("starts"), 1);
}
#[test]
fn draft_binding_freezes_tool_and_authority_identities_without_effects() {
    let f = Fixture::new();
    let p = f.dir.path();
    let paths = [
        (
            "role_host.py",
            "skills/manual-orchestration/scripts/role_host.py",
        ),
        (
            "scope_budget.py",
            "skills/manual-orchestration/scripts/scope_budget.py",
        ),
        (
            "child_context.py",
            "skills/manual-orchestration/scripts/child_context.py",
        ),
        ("usage_ledger.py", "metrics/scripts/usage_ledger.py"),
        (
            "chatboard.py",
            "skills/durable-state/scripts/codex_chatboard.py",
        ),
    ];
    let mut manifest = serde_json::Map::new();
    for (source, target) in paths {
        let to = p.join(target);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(p.join(source), &to).unwrap();
        manifest.insert(target.into(), json!(digest(&fs::read(to).unwrap())));
    }
    fs::write(
        p.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let mut draft = f.request.clone();
    draft["authority"] = draft["authority"]["path"].clone();
    draft["packet"] = draft["packet"]["path"].clone();
    draft["tools"] =
        json!({"bundle":p,"python":"/usr/bin/python3","context_binary":p.join("context-binary")});
    draft["result_path"] = json!(p.join("bind-only-must-not-create/result.json"));
    let bound = work_engine_manual_host::bind_draft(draft).unwrap();
    assert_eq!(bound.packet.sha256, f.request["packet"]["sha256"]);
    assert_eq!(f.count("starts"), 0);
    assert!(!p.join("bind-only-must-not-create").exists());
}
#[test]
fn public_schema_and_parser_preserve_version_and_unknown_field_boundaries() {
    let f = Fixture::new();
    let schema = work_engine_manual_host::public_schemas();
    assert_eq!(schema["request"]["properties"]["schema"]["const"], SCHEMA);
    assert_eq!(schema["request"]["additionalProperties"], false);
    let mut request = f.request.clone();
    request["invented_authority"] = json!(true);
    assert!(serde_json::from_value::<Request>(request).is_err());
}
#[test]
fn readonly_child_final_json_is_collected_without_child_file_write() {
    let mut f = Fixture::new();
    f.request["result_from_turn"] = json!(true);
    f.config("missing_result", json!(true));
    success(f.dispatch());
    let receipt = success(f.collect());
    assert_eq!(receipt["summary"]["phase"], "collected");
    assert_eq!(
        f.state()["operations"]["one"]["evidence"]["native_result_source"]["source"],
        "explicit_final_answer"
    );
    let doc: Value =
        serde_json::from_slice(&fs::read(f.dir.path().join("result.json")).unwrap()).unwrap();
    assert_eq!(doc["native"], true);
}
#[test]
fn native_capture_rejects_foreign_result_even_when_file_appears_after_dispatch() {
    let mut f = Fixture::new();
    f.request["result_from_turn"] = json!(true);
    success(f.dispatch());
    assert!(!f.collect().status.success());
    assert!(
        !f.state()["operations"]["one"]["evidence"]
            .as_object()
            .unwrap()
            .contains_key("collection")
    );
}
#[test]
fn admitted_operations_cannot_reserve_one_output_path() {
    let mut f = Fixture::new();
    f.config("missing_result", json!(true));
    success(f.dispatch());
    f.request["operation_id"] = json!("two");
    f.request["claim"] = Value::Null;
    assert!(!f.dispatch().status.success());
    assert_eq!(f.count("creates"), 1);
}
#[test]
fn native_capture_recovery_revalidates_turn_item_and_exact_publication() {
    let mut f = Fixture::new();
    f.request["result_from_turn"] = json!(true);
    f.config("missing_result", json!(true));
    success(f.dispatch());
    success(f.collect());
    let path = f.dir.path().join("state/state.json");
    let mut state = f.state();
    state["operations"]["one"]["phase"] = json!("terminal");
    state["operations"]["one"]["evidence"]
        .as_object_mut()
        .unwrap()
        .remove("collection");
    fs::write(&path, state.to_string()).unwrap();
    success(f.collect());
    let mut state = f.state();
    state["operations"]["one"]["evidence"]["native_result_source"]["turn_id"] = json!("other-turn");
    fs::write(path, state.to_string()).unwrap();
    assert!(!f.collect().status.success());
    assert_eq!(f.count("starts"), 1);
}
#[test]
fn precreate_abandonment_completes_task_and_objective_without_claim_release() {
    for objective in [false, true] {
        let f = Fixture::new();
        f.config("headroom", json!(0));
        let receipt = success(f.dispatch());
        assert_eq!(f.count("creates"), 0);
        let c = json!({"schema":SCHEMA,"operation_id":"one","authority":f.request["authority"],"ack_receipt":receipt["receipt_id"],"objective_end":objective,"disposition":"abandoned","reason":"no admission headroom"});
        let p = f.dir.path().join("abandon.json");
        fs::write(&p, c.to_string()).unwrap();
        let closed = success(f.invoke(&[
            "close",
            "--state",
            f.dir.path().join("state").to_str().unwrap(),
            "--request",
            p.to_str().unwrap(),
        ]));
        assert_eq!(closed["summary"]["phase"], "closed");
        assert_eq!(f.count("releases"), 0);
        assert_eq!(f.count("closes"), u64::from(objective));
    }
}
#[test]
fn task_then_objective_closure_are_separate_idempotent_transitions() {
    let f = Fixture::new();
    success(f.dispatch());
    let result = success(f.collect());
    let task = success(f.close(result["receipt_id"].as_str().unwrap(), false));
    let objective = success(f.close(task["receipt_id"].as_str().unwrap(), true));
    success(f.close(task["receipt_id"].as_str().unwrap(), true));
    assert_eq!(f.count("closes"), 1);
    assert_eq!(f.count("releases"), 1);
    assert_eq!(objective["summary"]["phase"], "closed");
}
#[test]
fn post_closure_accounting_refresh_preserves_historical_receipt_and_effect_counts() {
    let f = Fixture::new();
    success(f.dispatch());
    let result = success(f.collect());
    let closed = success(f.close(result["receipt_id"].as_str().unwrap(), true));
    let old = fs::read(closed["receipt"].as_str().unwrap()).unwrap();
    let counts = (
        f.count("starts"),
        f.count("creates"),
        f.count("releases"),
        f.count("closes"),
    );
    f.config("tail_candidate", json!(5));
    let refresh = success(f.invoke(&[
        "collect",
        "--state",
        f.dir.path().join("state").to_str().unwrap(),
        "--operation",
        "one",
        "--reconcile-accounting",
    ]));
    assert_eq!(
        refresh["accounting"]["snapshot"]["post_close"]["candidate_totals"],
        5
    );
    assert_eq!(
        refresh["accounting"]["snapshot"]["post_close"]["settlement"]["status"],
        "not_established"
    );
    assert_eq!(old, fs::read(closed["receipt"].as_str().unwrap()).unwrap());
    assert_eq!(
        counts,
        (
            f.count("starts"),
            f.count("creates"),
            f.count("releases"),
            f.count("closes")
        )
    );
}
#[test]
fn stale_session_settings_cannot_satisfy_selected_turn_configuration() {
    let f = Fixture::new();
    success(f.dispatch());
    f.config("stale_settings", json!(true));
    let v = success(f.collect());
    assert!(
        v["summary"]["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str().unwrap().contains("settings"))
    );
}
#[test]
fn scoped_root_capacity_decision_allows_pressure_for_fresh_and_retained_work() {
    let mut f = Fixture::new();
    f.config("root_recommendation", json!("fresh"));
    f.config("child_recommendation", json!("fresh"));
    let admitted = success(f.dispatch());
    assert_eq!(admitted["summary"]["phase"], "running");
    success(f.collect());
    assert!(
        f.state()["operations"]["one"]["observer_events"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v["kind"] == "context_pressure")
    );
    f.request["operation_id"] = json!("retained-pressure");
    f.request["claim"] = Value::Null;
    f.request["bootstrap_context"] = json!(false);
    f.request["require_read_only"] = json!(false);
    f.request["retained"] =
        json!({"session_id":"child","rollout":f.dir.path().join("child.jsonl")});
    f.request["result_path"] = json!(f.dir.path().join("retained-pressure-result.json"));
    f.config("result", f.request["result_path"].clone());
    let retained = success(f.dispatch());
    assert_eq!(retained["summary"]["phase"], "running");
    assert_eq!(f.count("starts"), 2);
}
#[test]
fn acknowledged_pressure_does_not_turn_foreground_collection_into_model_polling() {
    let f = Fixture::new();
    f.config("root_recommendation", json!("fresh"));
    f.config("child_recommendation", json!("fresh"));
    f.config("turn_status", json!("inProgress"));
    success(f.dispatch());
    let first = success(f.collect());
    let before = f.count("turn_reads");
    success(f.invoke(&[
        "collect",
        "--state",
        f.dir.path().join("state").to_str().unwrap(),
        "--operation",
        "one",
        "--wait",
        "--ack-receipt",
        first["receipt_id"].as_str().unwrap(),
    ]));
    assert!(f.count("turn_reads") > before + 1);
    assert_eq!(f.count("starts"), 1);
}

#[test]
fn ack_only_preserves_receipts_and_does_not_run_observers_or_native_effects() {
    let f = Fixture::new();
    success(f.dispatch());
    let collected = success(f.collect());
    let closed = success(f.close(collected["receipt_id"].as_str().unwrap(), true));
    f.config("tail_candidate", json!(7));
    let state_dir = f.dir.path().join("state");
    let state_arg = state_dir.to_str().unwrap();
    let accounting = success(f.invoke(&[
        "collect",
        "--state",
        state_arg,
        "--operation",
        "one",
        "--reconcile-accounting",
    ]));
    let before = f.state();
    let config_before = fs::read(f.dir.path().join("config.json")).unwrap();
    let receipt_bytes = fs::read(closed["receipt"].as_str().unwrap()).unwrap();
    let accounting_bytes = fs::read(accounting["receipt"].as_str().unwrap()).unwrap();
    let receipt_count = fs::read_dir(state_dir.join("receipts")).unwrap().count();
    let ack_accounting = success(f.invoke(&[
        "ack",
        "--state",
        state_arg,
        "--ack-receipt",
        accounting["receipt_id"].as_str().unwrap(),
    ]));
    assert_eq!(ack_accounting["delivery"], "acknowledged");
    let ack = success(f.invoke(&[
        "ack",
        "--state",
        state_arg,
        "--ack-receipt",
        closed["receipt_id"].as_str().unwrap(),
    ]));
    assert_eq!(
        ack["summary"]["operations"][0]["receipt"]["delivery"],
        "acknowledged"
    );
    let repeated = success(f.invoke(&[
        "ack",
        "--state",
        state_arg,
        "--ack-receipt",
        closed["receipt_id"].as_str().unwrap(),
    ]));
    assert_eq!(repeated, ack);
    assert_eq!(f.state()["operations"], before["operations"]);
    assert_eq!(
        config_before,
        fs::read(f.dir.path().join("config.json")).unwrap()
    );
    assert_eq!(
        receipt_count,
        fs::read_dir(state_dir.join("receipts")).unwrap().count()
    );
    assert_eq!(
        receipt_bytes,
        fs::read(closed["receipt"].as_str().unwrap()).unwrap()
    );
    assert_eq!(
        accounting_bytes,
        fs::read(accounting["receipt"].as_str().unwrap()).unwrap()
    );
}

#[test]
fn acknowledging_historical_receipt_does_not_claim_latest_task_delivery() {
    let f = Fixture::new();
    success(f.dispatch());
    let collected = success(f.collect());
    let closed = success(f.close(collected["receipt_id"].as_str().unwrap(), true));
    f.config("tail_candidate", json!(8));
    let dir = f.dir.path().join("state");
    let path = dir.to_str().unwrap();
    success(f.invoke(&[
        "collect",
        "--state",
        path,
        "--operation",
        "one",
        "--reconcile-accounting",
    ]));
    let newer = success(f.collect());
    assert_ne!(newer["receipt_id"], closed["receipt_id"]);
    let ack = success(f.invoke(&[
        "ack",
        "--state",
        path,
        "--ack-receipt",
        closed["receipt_id"].as_str().unwrap(),
    ]));
    assert_eq!(ack["delivery"], "acknowledged");
    assert_eq!(
        ack["summary"]["operations"][0]["receipt"]["receipt_id"],
        newer["receipt_id"]
    );
    assert_eq!(
        ack["summary"]["operations"][0]["receipt"]["delivery"],
        "pending"
    );
    let invalid = "0".repeat(64);
    assert!(
        !f.invoke(&["ack", "--state", path, "--ack-receipt", &invalid])
            .status
            .success()
    );
    let receipt = newer["receipt"].as_str().unwrap();
    fs::write(receipt, "corrupt").unwrap();
    assert!(
        !f.invoke(&[
            "ack",
            "--state",
            path,
            "--ack-receipt",
            newer["receipt_id"].as_str().unwrap()
        ])
        .status
        .success()
    );
    assert!(
        !f.state()["acknowledged"]
            .as_array()
            .unwrap()
            .contains(&newer["receipt_id"])
    );
}

#[test]
fn compact_inspection_exposes_evidence_and_controls_without_refresh_or_mutation() {
    let f = Fixture::new();
    success(f.dispatch());
    success(f.collect());
    let state_path = f.dir.path().join("state/state.json");
    let mut state = f.state();
    state["operations"]["one"]["evidence"]["budget"]["snapshot"]["cost_controls"] = json!([
        {"node_id":null,"max_usd":30,"remaining_usd":25},
        {"node_id":"child","max_usd":0.75,"remaining_usd":-0.5}
    ]);
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    let before = fs::read(&state_path).unwrap();
    let dir = f.dir.path().join("state");
    let view = success(f.invoke(&[
        "inspect",
        "--state",
        dir.to_str().unwrap(),
        "--summary",
        "--operation",
        "one",
    ]));
    let op = &view["operations"][0];
    assert_eq!(view["evidence_status"], "recorded_state_not_refreshed");
    assert_eq!(op["configuration"]["status"], "matched");
    assert_eq!(op["configuration"]["observed"]["model"], "fixture-model");
    assert_eq!(op["realization"]["sandbox"]["type"], "readOnly");
    assert_eq!(op["accounting"]["role_controls"][0]["remaining_usd"], -0.5);
    assert_eq!(
        op["accounting"]["role_control_status"],
        "configured_in_recorded_snapshot"
    );
    assert_eq!(op["receipt"]["delivery"], "pending");
    assert_eq!(op["context"]["root"]["input_tokens"], 100);
    assert_eq!(op["monitoring"], "not_established_by_recorded_state");
    assert!(!op.to_string().contains("process_records"));
    assert_eq!(before, fs::read(&state_path).unwrap());
    assert_eq!(
        success(f.invoke(&["inspect", "--state", dir.to_str().unwrap()])),
        state
    );
    assert!(
        !f.invoke(&[
            "inspect",
            "--state",
            dir.to_str().unwrap(),
            "--summary",
            "--operation",
            "unknown"
        ])
        .status
        .success()
    );
    state["binary_sha256"] = json!("another-version");
    state["operations"]["one"]["evidence"]
        .as_object_mut()
        .unwrap()
        .remove("settings");
    state["operations"]["one"]["evidence"]["budget"]["snapshot"]["cost_controls"] = json!([]);
    state["operations"]["one"]["evidence"]
        .as_object_mut()
        .unwrap()
        .remove("prepared");
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    let missing = success(f.invoke(&["inspect", "--state", dir.to_str().unwrap(), "--summary"]));
    assert_eq!(
        missing["operations"][0]["configuration"]["status"],
        "not_established"
    );
    assert_eq!(
        missing["operations"][0]["accounting"]["role_control_status"],
        "none_in_recorded_snapshot"
    );
    assert_eq!(
        missing["operations"][0]["realization"]["field_presence"]["sandbox"],
        false
    );
    assert_eq!(
        missing["operations"][0]["realization"]["field_presence"]["forked_from"],
        false
    );
    assert!(
        !f.invoke(&[
            "ack",
            "--state",
            dir.to_str().unwrap(),
            "--ack-receipt",
            op["receipt"]["receipt_id"].as_str().unwrap()
        ])
        .status
        .success()
    );
}

#[test]
fn authorized_nested_output_directory_is_prepared_before_native_start() {
    use std::os::unix::fs::PermissionsExt;
    let mut f = Fixture::new();
    let output = f.dir.path().join("new/task/output/result.json");
    f.request["result_path"] = json!(output);
    f.config("result", json!(output));
    assert!(!output.parent().unwrap().exists());
    success(f.dispatch());
    assert!(output.exists());
    assert_eq!(
        fs::metadata(output.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let result = success(f.collect());
    assert!(result["summary"]["failures"].as_array().unwrap().is_empty());
    success(f.dispatch());
    assert_eq!(f.count("creates"), 1);
    assert_eq!(f.count("starts"), 1);
}

#[test]
fn output_preflight_validates_authority_and_reports_blocked_parent_without_launch() {
    let mut f = Fixture::new();
    let parent = f.dir.path().join("unapproved");
    f.request["result_path"] = json!(parent.join("result.json"));
    f.request["authority"]["sha256"] = json!("invalid");
    assert!(!f.dispatch().status.success());
    assert!(!parent.exists());
    assert_eq!(f.count("creates"), 0);
    let mut f = Fixture::new();
    let blocked = f.dir.path().join("blocked");
    fs::write(&blocked, "preserve").unwrap();
    f.request["result_path"] = json!(blocked.join("nested/result.json"));
    let result = f.dispatch();
    assert!(!result.status.success());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains("cannot prepare authorized result directory"));
    assert!(error.contains(blocked.to_str().unwrap()));
    assert_eq!(fs::read_to_string(blocked).unwrap(), "preserve");
    assert_eq!(f.count("creates"), 0);
    assert!(f.state()["operations"].as_object().unwrap().is_empty());
}

#[test]
fn preparing_parent_cannot_expose_and_overwrite_an_existing_result() {
    let mut f = Fixture::new();
    let existing = f.dir.path().join("result.json");
    fs::write(&existing, "preserve prior result").unwrap();
    let missing = f.dir.path().join("new");
    f.request["result_path"] = json!(missing.join("../result.json"));
    assert!(!Path::new(f.request["result_path"].as_str().unwrap()).exists());
    let output = f.dispatch();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("result already exists"));
    assert_eq!(
        fs::read_to_string(existing).unwrap(),
        "preserve prior result"
    );
    assert!(f.state()["operations"].as_object().unwrap().is_empty());
    assert_eq!(f.count("creates"), 0);
    assert_eq!(f.count("starts"), 0);
}

#[test]
fn dangling_output_link_remains_an_occupied_reservation() {
    let f = Fixture::new();
    std::os::unix::fs::symlink(
        f.dir.path().join("not-created"),
        f.dir.path().join("result.json"),
    )
    .unwrap();
    let output = f.dispatch();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("result already exists"));
    assert!(f.dir.path().join("result.json").is_symlink());
    assert!(!f.dir.path().join("not-created").exists());
    assert_eq!(f.count("creates"), 0);
}

#[test]
fn help_is_available_before_state_or_provider_effects() {
    let f = Fixture::new();
    for args in [
        &["--help"][..],
        &["dispatch", "--help"][..],
        &["ack", "--help"][..],
    ] {
        let output = f.invoke(args);
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("ack --state DIR --ack-receipt ID"));
        assert!(help.contains("inspect --state DIR [--summary"));
    }
    assert!(!f.dir.path().join("state").exists());
    assert_eq!(f.count("creates"), 0);
}

#[test]
fn summary_flag_on_dispatch_cannot_be_silently_ignored() {
    let f = Fixture::new();
    let p = f.dir.path().join("request.json");
    fs::write(&p, f.request.to_string()).unwrap();
    let output = f.invoke(&["dispatch", "--request", p.to_str().unwrap(), "--summary"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("only by inspect"));
    assert!(!f.dir.path().join("state").exists());
    assert_eq!(f.count("starts"), 0);
}

#[test]
fn raw_inspect_remains_available_for_oversized_other_version_state() {
    let f = Fixture::new();
    let dir = f.dir.path().join("historical");
    fs::create_dir(&dir).unwrap();
    let value = json!({"schema":"future-or-unknown","opaque":"x".repeat(8*1024*1024)});
    fs::write(dir.join("state.json"), value.to_string()).unwrap();
    assert_eq!(
        success(f.invoke(&["inspect", "--state", dir.to_str().unwrap()])),
        value
    );
    assert!(
        !f.invoke(&["inspect", "--state", dir.to_str().unwrap(), "--summary"])
            .status
            .success()
    );
}

#[test]
fn large_owned_records_reopen_inspect_acknowledge_and_close() {
    let f = Fixture::new();
    let r: Request = serde_json::from_value(f.request.clone()).unwrap();
    let mut h = Host::open(&r.state_dir, Some(&r)).unwrap();
    h.dispatch(r.clone()).unwrap();
    h.collect("one", false).unwrap();
    // Model accumulated observation history beyond the external-input limit.
    h.state
        .operations
        .get_mut("one")
        .unwrap()
        .observer_events
        .insert(
            "large-history".into(),
            json!({"recorded": "x".repeat(9 * 1024 * 1024)}),
        );
    h.save().unwrap();
    assert!(fs::metadata(r.state_dir.join("state.json")).unwrap().len() > 8 * 1024 * 1024);
    drop(h);

    let mut h = Host::open(&r.state_dir, None).unwrap();
    let before_inspect = fs::read(r.state_dir.join("state.json")).unwrap();
    let summary = work_engine_manual_host::inspect(&r.state_dir, Some("one"), true).unwrap();
    assert_eq!(summary["operations"][0]["phase"], "collected");
    assert_eq!(
        before_inspect,
        fs::read(r.state_dir.join("state.json")).unwrap()
    );
    let task = h.receipt("one").unwrap();
    let task_path = Path::new(task["receipt"].as_str().unwrap());
    let task_bytes = fs::read(task_path).unwrap();
    assert!(task_bytes.len() > 8 * 1024 * 1024);
    let config_before_ack = fs::read(f.dir.path().join("config.json")).unwrap();
    h.acknowledge_receipt(task["receipt_id"].as_str().unwrap())
        .unwrap();
    assert_eq!(
        config_before_ack,
        fs::read(f.dir.path().join("config.json")).unwrap()
    );
    assert_eq!(task_bytes, fs::read(task_path).unwrap());

    let close = serde_json::from_value(json!({
        "schema":SCHEMA, "operation_id":"one", "authority":f.request["authority"],
        "ack_receipt":task["receipt_id"], "objective_end":true,
        "disposition":"accepted", "reason":"explicit large registry closure",
    }))
    .unwrap();
    let closed = h.close(close).unwrap();
    h.acknowledge_receipt(closed["receipt_id"].as_str().unwrap())
        .unwrap();
    assert_eq!(f.count("creates"), 1);
    assert_eq!(f.count("starts"), 1);
    assert_eq!(f.count("releases"), 1);
    assert_eq!(f.count("closes"), 1);

    let accounting = h.reconcile_accounting("one").unwrap();
    assert!(
        fs::metadata(accounting["receipt"].as_str().unwrap())
            .unwrap()
            .len()
            > 8 * 1024 * 1024
    );
    h.acknowledge_receipt(accounting["receipt_id"].as_str().unwrap())
        .unwrap();
    let accounting_bytes = fs::read(accounting["receipt"].as_str().unwrap()).unwrap();
    let repeated = h.reconcile_accounting("one").unwrap();
    h.acknowledge_receipt(repeated["receipt_id"].as_str().unwrap())
        .unwrap();
    assert_eq!(
        accounting_bytes,
        fs::read(accounting["receipt"].as_str().unwrap()).unwrap()
    );
    drop(h);
    let mut h = Host::open(&r.state_dir, None).unwrap();
    assert!(matches!(
        h.state.operations["one"].phase,
        work_engine_manual_host::Phase::Closed
    ));
    assert!(
        h.state
            .acknowledged
            .contains(closed["receipt_id"].as_str().unwrap())
    );
    assert!(
        h.state
            .acknowledged
            .contains(accounting["receipt_id"].as_str().unwrap())
    );
    // Large records retain exact-byte identity checking.
    let mut tampered = task_bytes.clone();
    tampered.push(b' ');
    fs::write(task_path, tampered).unwrap();
    assert!(
        h.acknowledge_receipt(task["receipt_id"].as_str().unwrap())
            .unwrap_err()
            .to_string()
            .contains("receipt identity conflict")
    );
}

#[test]
fn large_external_artifacts_remain_bounded() {
    let f = Fixture::new();
    let path = f.dir.path().join("large-packet.md");
    fs::write(&path, "x".repeat(9 * 1024 * 1024)).unwrap();
    let artifact: work_engine_manual_host::Artifact =
        serde_json::from_value(artifact(&path)).unwrap();
    assert!(
        artifact
            .verify()
            .unwrap_err()
            .to_string()
            .contains("oversize input")
    );
    assert_eq!(f.count("creates"), 0);
    assert_eq!(f.count("starts"), 0);
}

#[test]
fn waiting_returns_unavailable_turn_evidence_without_polling_or_relaunch() {
    for fault in ["observer_fail", "unknown_status"] {
        let f = Fixture::new();
        success(f.dispatch());
        if fault == "observer_fail" {
            f.config("observer_fail", json!(true));
        } else {
            f.config("turn_status", json!("not_established"));
        }
        let dir = f.dir.path().join("state");
        let receipt = success(f.invoke(&[
            "collect",
            "--state",
            dir.to_str().unwrap(),
            "--operation",
            "one",
            "--wait",
        ]));
        assert_eq!(receipt["summary"]["phase"], "running");
        assert_eq!(receipt["summary"]["turn_outcome"], "unavailable");
        assert_eq!(f.count("turn_reads"), 1);
        assert!(
            f.state()["operations"]["one"]["evidence"]
                .get("collection")
                .is_none()
        );
        assert_eq!(f.count("creates"), 1);
        assert_eq!(f.count("starts"), 1);
        let historical = fs::read(receipt["receipt"].as_str().unwrap()).unwrap();
        f.config("observer_fail", json!(false));
        f.config("turn_status", json!("completed"));
        let recovered = success(f.collect());
        assert_eq!(recovered["summary"]["phase"], "collected");
        assert_eq!(f.count("creates"), 1);
        assert_eq!(f.count("starts"), 1);
        assert_eq!(
            historical,
            fs::read(receipt["receipt"].as_str().unwrap()).unwrap()
        );
    }
}
