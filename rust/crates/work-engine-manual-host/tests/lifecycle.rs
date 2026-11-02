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
    let bound = work_engine_manual_host::bind_draft(draft).unwrap();
    assert_eq!(bound.packet.sha256, f.request["packet"]["sha256"]);
    assert_eq!(f.count("starts"), 0);
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
