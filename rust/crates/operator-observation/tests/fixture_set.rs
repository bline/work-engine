//! RC5: the one deterministic generator of the consumer fixture set.
//!
//! decode -> Selection -> project (fixed clock) -> serialize. The default suite
//! regenerates in memory and fails on any byte difference from the checked-in set
//! under `tests/fixtures/selected-task/`. Files are written only when
//! `SELECTED_TASK_FIXTURES=update` is set. devwits-auth checks in the same bytes.
use operator_observation::{
    digest,
    selected_task::{Presentation, ResultSummary, SelectedTask},
    task::{self, ReadError, Request, Selection},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf};

const T: u64 = 1_794_600_000_000;
const ENDPOINT: &str = "/private/u2/host.sock";
const MANIFEST: &str = "MANIFEST.sha256";

fn request() -> Request {
    Request {
        task_id: "task:u2".into(),
        delivery_id: Some("delivery:u2".into()),
        command_id: None,
    }
}
fn source() -> String {
    digest(b"synthetic")
}
/// A TO2 frame shaped like the task host's v2 observation.
fn frame(owner_revision: u64, cursor_revision: u64) -> Value {
    let subject = "subject:u2";
    let cursor = json!({"store_id":"store:u2","stream_id":"stream:u2","commit_sequence":"1"});
    let segment = json!({"delivery_id":"delivery:u2","input_id":"input:u2","sequence":1,"delivery_state":"active","effect_id":"effect:u2","queue_attempt_id":"attempt:u2","segment_no":1,"invocation_id":"invocation:u2","native_attempt_id":"native:u2","predecessor_invocation_id":null,"transition_id":null,"segment_state":"completed","native_state":"completed","send_intent":true,"thread_id":"01a142ba-b70b-7eb0-9900-b53bece80f30","turn_id":"01a142ba-b722-7272-affc-928d3a20aeed","context_generation":"context:u2","final_sha256":null,"history_status":"complete"});
    json!({"protocol_version":2,"status":"observation","consistency":"independent_components","request":request(),
    "host_identity":{"pid":1,"linux_starttime_ticks":"1","incarnation":"host:u2","build_id":"build:u2","profile":"retained-repository-analysis-v1","executable_sha256":"a".repeat(64),"source_sha256":"b".repeat(64),"lock_sha256":"c".repeat(64),"toolchain_sha256":"d".repeat(64),"features":[],"provider_identity":"synthetic_no_provider"},
    "components":{
    "owner":{"state":"present","scope":{"task_id":"task:u2","subject_id":subject},"revision":{"kind":"task","owner_revision":owner_revision,"cursor_revision":cursor_revision},"value":{"task_id":"task:u2","subject_id":subject,"revision":owner_revision,"cursor_revision":cursor_revision,"objective":"Verify selected task across clients","engineering_question":"What remains pending?","inputs":[{"input_id":"input:u2"}],
        "questions":[
            {"id":"question:u2","recipient_ref":"operator:u2","scope_ref":"task:u2","text":"Publish the private synthetic result?","state":"pending"},
            {"id":"question:u2:scope","recipient_ref":"operator:u2","scope_ref":"task:u2","text":"Keep the review scope?","state":{"answered":{"answer":"Yes, publish","authority_ref":"operator:u2"}}}],
        "result":null}},
    "lifecycle":{"state":"present","scope":{"subject_id":subject,"delivery_id":"delivery:u2"},"revision":{"kind":"lifecycle","cursor":cursor},"value":{"protocol_version":1,"subject_id":subject,"context_generation":"context:u2","cursor":cursor,"transition":{"state":"unknown"},"current_admission":{"kind":"unknown"},"delivery":{"state":"unknown"},"runtime_availability":"unknown","runtime_provenance":{"state":"unknown"},"blocking_reason":{"state":"unknown"},"authority_refs":[],"evidence_refs":[],"measurement":{"state":"unknown"}}},
    "native_delivery_status":{"state":"present","scope":{"subject_id":subject,"collection":"native_delivery_segments","filter":"subject"},"revision":{"kind":"not_provided"},"completeness":{"kind":"complete_for_scope","limit":64},"value":{"protocol_version":1,"subject_id":subject,"recovery_only":false,"segments":[segment]}},
    "command_result":{"state":"not_requested","scope":{"subject_id":subject,"command_id":null,"task_binding":"not_established"}},
    "control_result":{"state":"not_requested","scope":{"subject_id":subject,"command_id":null,"task_binding":"not_established"}}}})
}
fn published(raw: &mut Value) {
    raw["components"]["owner"]["value"]["result"] = json!({"body":"Independent synthetic result","result_id":"result:joined","operation_id":"operation:joined","attribution":"operator:u2","source_refs":[]});
}
fn owner_failure(raw: &Value, state: &str, scope: Value) -> Value {
    let mut out = raw.clone();
    out["components"]["owner"] = match state {
        "absent" => json!({"state":"absent","scope":scope,"record_kind":"task"}),
        "refused" => {
            json!({"state":"refused","scope":scope,"reason":"capacity_exceeded","limit":64})
        }
        _ => json!({"state":"unavailable","scope":scope,"reason":"task_read_unavailable"}),
    };
    for name in ["lifecycle", "native_delivery_status"] {
        let mut scope = raw["components"][name]["scope"].clone();
        scope["subject_id"] = Value::Null;
        out["components"][name] = json!({"state":if state=="absent" {"not_applicable"}else{"unavailable"},"scope":scope,"reason":if state=="absent" {"task_absent"}else{"task_dependency_unavailable"}});
    }
    out
}
fn read(s: &mut Selection, raw: &Value, at: u64) {
    let g = s.begin(ENDPOINT.into(), request());
    let decoded = task::decode(&serde_json::to_vec(raw).unwrap(), &request());
    assert!(s.apply_at(g, decoded, at));
}
fn fail(s: &mut Selection, at: u64) {
    let g = s.begin(ENDPOINT.into(), request());
    assert!(s.apply_at(g, Err(ReadError::Unavailable), at));
}
fn project(s: &Selection, at: u64) -> SelectedTask {
    SelectedTask::project_at(s, &source(), None, true, false, at).unwrap()
}

/// The checked-in consumer set: name -> envelope.
fn generate() -> BTreeMap<&'static str, SelectedTask> {
    let mut set = BTreeMap::new();
    // U3 control: fresh, one pending and one answered question, published result,
    // exact linked activity read.
    let mut raw = frame(2, 2);
    published(&mut raw);
    let mut s = Selection::default();
    read(&mut s, &raw, T);
    let ids = operator_observation::selected_task::linked_sessions(&s);
    let mut activity = operator_observation::collect(&source(), "Owner-linked session activity", &ids, |_| {
        Ok(json!({"sessionId":ids[0],"host":"codex","sessionTitle":"Private synthetic activity","entries":[]}))
    })
    .unwrap();
    activity.observed_at_ms = T + 500;
    for session in &mut activity.sessions {
        session.observed_at_ms = T + 500;
    }
    set.insert(
        "joined-fresh",
        SelectedTask::project_at(&s, &source(), Some(&activity), true, true, T + 1_000).unwrap(),
    );
    // Stale present: a later transport failure keeps the original read as basis.
    let mut s = Selection::default();
    read(&mut s, &frame(7, 3), T);
    fail(&mut s, T + 30_000);
    set.insert("stale-present", project(&s, T + 31_000));
    // Retained anchor: a completed read refuses the owner with subject omitted.
    let mut s = Selection::default();
    let raw = frame(4, 2);
    read(&mut s, &raw, T);
    read(
        &mut s,
        &owner_failure(&raw, "refused", json!({"task_id":"task:u2"})),
        T + 60_000,
    );
    set.insert("retained-anchor-refused", project(&s, T + 61_000));
    // Observed absence clears the earlier anchor.
    let mut s = Selection::default();
    read(&mut s, &raw, T);
    read(
        &mut s,
        &owner_failure(&raw, "absent", json!({"task_id":"task:u2"})),
        T + 10_000,
    );
    set.insert("observed-absent", project(&s, T + 11_000));
    // Unobserved: the first read failed before any host identity was established.
    let mut s = Selection::default();
    fail(&mut s, T);
    set.insert("unobserved", project(&s, T + 1_000));
    // Summary unavailable: owner present, owner fields malformed.
    let mut raw = frame(5, 5);
    raw["components"]["owner"]["value"]["objective"] = json!(42);
    let mut s = Selection::default();
    read(&mut s, &raw, T);
    set.insert("summary-unavailable", project(&s, T + 1_000));
    // Full-u64 revisions, a clipped excerpt and a redacted credential.
    let mut s = Selection::default();
    read(&mut s, &large_frame(), T);
    set.insert("large-revision-redacted", project(&s, T + 1_000));
    set
}
fn large_frame() -> Value {
    let mut raw = frame(9_007_199_254_740_993, u64::MAX);
    let owner = &mut raw["components"]["owner"]["value"];
    owner["objective"] = json!(format!("Long objective {}", "x".repeat(1100)));
    owner["engineering_question"] =
        json!("Rotate the deploy credential? password=hunter2 Bearer abc.def");
    published(&mut raw);
    raw
}
fn encode(p: &SelectedTask) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(p).unwrap();
    bytes.push(b'\n');
    bytes
}
fn files() -> BTreeMap<String, Vec<u8>> {
    let mut out: BTreeMap<String, Vec<u8>> = generate()
        .iter()
        .map(|(name, p)| (format!("{name}.json"), encode(p)))
        .collect();
    let manifest = out
        .iter()
        .map(|(name, bytes)| format!("{}  {name}\n", digest(bytes)))
        .collect::<String>();
    out.insert(MANIFEST.into(), manifest.into_bytes());
    out
}
fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/selected-task")
}

#[test]
fn checked_in_fixture_set_is_generator_output() {
    let expected = files();
    let dir = dir();
    if std::env::var("SELECTED_TASK_FIXTURES").as_deref() == Ok("update") {
        std::fs::create_dir_all(&dir).unwrap();
        for entry in std::fs::read_dir(&dir).unwrap() {
            std::fs::remove_file(entry.unwrap().path()).unwrap();
        }
        for (name, bytes) in &expected {
            std::fs::write(dir.join(name), bytes).unwrap();
        }
    }
    let mut present = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    present.sort();
    assert_eq!(present, expected.keys().cloned().collect::<Vec<_>>());
    for (name, bytes) in &expected {
        assert!(
            std::fs::read(dir.join(name)).unwrap() == *bytes,
            "{name} differs from generator output; regenerate with SELECTED_TASK_FIXTURES=update"
        );
    }
}

#[test]
fn generator_is_deterministic_and_each_fixture_parses() {
    assert_eq!(files(), files());
    for (name, p) in generate() {
        let parsed = SelectedTask::parse(&encode(&p)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(encode(&parsed), encode(&p), "{name}");
    }
}

/// Facts every client must show identically for one fixture.
fn facts(p: &SelectedTask) -> (String, String, String, String, u64, String, String) {
    let c = &p.components["owner"];
    let retained = p.summary.basis == "last_known_owner";
    let revision = if retained {
        &c.last_known_revision
    } else {
        &c.revision
    };
    let digits = if revision.is_null() {
        "none".into()
    } else {
        format!(
            "{}/{}",
            revision["owner_revision"].as_str().unwrap(),
            revision["cursor_revision"].as_str().unwrap()
        )
    };
    let read = if retained {
        c.last_known_at_ms.unwrap()
    } else {
        c.read_at_ms
    };
    let (counts, result) = match &p.summary.value {
        Some(v) => (
            format!(
                "{} pending · {} answered",
                v.pending_count, v.answered_count
            ),
            match v.result {
                ResultSummary::NotPublished => "not_published".into(),
                ResultSummary::Published { .. } => "published".into(),
            },
        ),
        None => ("unavailable".into(), "unavailable".into()),
    };
    (
        p.summary.basis.clone(),
        c.state.clone(),
        digits,
        p.transport.clone(),
        read,
        counts,
        result,
    )
}

#[test]
fn fixture_set_semantics_and_cli_human_facts() {
    let set = generate();
    let expect = |name: &str,
                  basis: &str,
                  state: &str,
                  digits: &str,
                  transport: &str,
                  read: u64,
                  counts: &str,
                  result: &str| {
        let f = facts(&set[name]);
        assert_eq!(
            f,
            (
                basis.into(),
                state.into(),
                digits.into(),
                transport.into(),
                read,
                counts.into(),
                result.into()
            ),
            "{name}"
        );
    };
    let both = "1 pending · 1 answered";
    expect(
        "joined-fresh",
        "latest_owner",
        "present",
        "2/2",
        "read_completed",
        T,
        both,
        "published",
    );
    expect(
        "stale-present",
        "last_known_owner",
        "present",
        "7/3",
        "unavailable",
        T,
        both,
        "not_published",
    );
    expect(
        "retained-anchor-refused",
        "last_known_owner",
        "refused",
        "4/2",
        "read_completed",
        T,
        both,
        "not_published",
    );
    expect(
        "observed-absent",
        "none",
        "absent",
        "none",
        "read_completed",
        T + 10_000,
        "unavailable",
        "unavailable",
    );
    expect(
        "unobserved",
        "none",
        "unavailable",
        "none",
        "unavailable",
        0,
        "unavailable",
        "unavailable",
    );
    expect(
        "summary-unavailable",
        "latest_owner",
        "present",
        "5/5",
        "read_completed",
        T,
        "unavailable",
        "unavailable",
    );
    expect(
        "large-revision-redacted",
        "latest_owner",
        "present",
        "9007199254740993/18446744073709551615",
        "read_completed",
        T,
        both,
        "published",
    );
    assert_eq!(
        set["joined-fresh"].activity_coverage.state,
        "read_completed"
    );
    assert_eq!(set["joined-fresh"].activity.len(), 1);
    assert_eq!(
        set["summary-unavailable"].summary.reason.as_deref(),
        Some("malformed_owner_fields")
    );
    assert_eq!(
        set["retained-anchor-refused"].components["owner"].read_at_ms,
        T + 60_000
    );
    assert!(set["unobserved"].host_identity.is_none());
    let large = &set["large-revision-redacted"];
    let v = large.summary.value.as_ref().unwrap();
    assert!(v.objective.truncated);
    let text = String::from_utf8(encode(large)).unwrap();
    assert!(!text.contains("hunter2") && !text.contains("abc.def") && text.contains("[REDACTED]"));
    // The CLI's `--human` renderer shows the same facts from each envelope.
    for (name, p) in &set {
        let (basis, state, digits, _, read, counts, result) = facts(p);
        let overview = p.human_view(None, T + 120_000, false);
        let owner = p.human_view(Some("owner"), T + 120_000, false);
        assert!(
            overview.contains(&format!("Owner record {state} · {basis}")),
            "{name}\n{overview}"
        );
        if counts != "unavailable" {
            assert!(overview.contains(&format!("Questions: {counts}")), "{name}");
        }
        match result.as_str() {
            "published" => assert!(overview.contains("Published result:"), "{name}"),
            "not_published" => assert!(overview.contains("No result published"), "{name}"),
            _ => assert!(overview.contains("Compact summary unavailable"), "{name}"),
        }
        for part in digits.split('/').filter(|d| *d != "none") {
            assert!(owner.contains(&format!("\"{part}\"")), "{name}");
        }
        assert!(owner.contains(&read.to_string()), "{name}");
        println!("FACTS {name} | {basis} | {state} | {digits} | read {read} | {counts} | {result}");
    }
}

/// Replaces the former environment-gated typed-host test with a default check:
/// the CLI replays a full-u64 TO2 frame and prints the exact digits.
#[test]
fn cli_replay_preserves_full_u64_revision_digits() {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::temp_dir().join(format!("s1-large-to2-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_vec(&large_frame()).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let run = |extra: &[&str]| {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_work-engine-observe"))
            .args([
                "selected-task",
                ENDPOINT,
                "task:u2",
                "--delivery",
                "delivery:u2",
                "--replay",
            ])
            .arg(&path)
            .args(extra)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    };
    let json: Value = serde_json::from_str(&run(&[])).unwrap();
    assert_eq!(
        json["components"]["owner"]["revision"]["owner_revision"],
        "9007199254740993"
    );
    assert_eq!(
        json["components"]["owner"]["revision"]["cursor_revision"],
        "18446744073709551615"
    );
    let human = run(&["--human", "--inspect", "owner"]);
    assert!(human.contains("9007199254740993") && human.contains("18446744073709551615"));
    std::fs::remove_file(path).unwrap();
}

fn redigest(p: &mut SelectedTask) {
    p.scope_id = digest(
        &serde_json::to_vec(&json!([
            p.endpoint,
            p.request,
            p.source_id,
            p.host_identity
        ]))
        .unwrap(),
    );
}
fn rejects(name: &str, p: &SelectedTask) -> &'static str {
    let err = SelectedTask::parse(&serde_json::to_vec(p).unwrap()).expect_err(name);
    assert_eq!(p.validate().unwrap_err(), err, "{name}");
    err
}

/// RC2 counterexamples from the R4 design review (REPORT.md §Q1) beside P0/P1.
#[test]
fn rc2_unobserved_equality_counterexamples() {
    let set = generate();
    let p0 = set["joined-fresh"].clone();
    let p1 = set["unobserved"].clone();
    SelectedTask::parse(&encode(&p0)).expect("P0 producer-shaped fresh envelope");
    SelectedTask::parse(&encode(&p1)).expect("P1 producer-shaped unobserved envelope");
    for c in p1.components.values() {
        assert_eq!(c, &Presentation::unobserved());
    }

    let mut ce1 = p1.clone();
    ce1.host_identity = p0.host_identity.clone();
    redigest(&mut ce1);
    assert_eq!(
        rejects("CE1 unobserved + established host_identity", &ce1),
        "invalid_projection"
    );

    let mut ce2 = p0.clone();
    ce2.host_identity = None;
    redigest(&mut ce2);
    assert_eq!(
        rejects("CE2 fresh owner + host_identity null", &ce2),
        "invalid_unobserved_envelope"
    );

    let mut ce3 = p1.clone();
    ce3.components
        .insert("lifecycle".into(), p0.components["lifecycle"].clone());
    // A fresh lifecycle under an unavailable transport already fails RC2's stale rule.
    assert_eq!(
        rejects("CE3 unobserved + lifecycle present", &ce3),
        "invalid_projection"
    );
    ce3.components.get_mut("lifecycle").unwrap().stale = true;
    assert_eq!(
        rejects("CE3 unobserved + stale lifecycle present", &ce3),
        "invalid_unobserved_envelope"
    );

    let mut ce4 = p0.clone();
    ce4.components
        .insert("lifecycle".into(), Presentation::unobserved());
    assert_eq!(
        rejects("CE4 fresh + lifecycle sentinel under read_completed", &ce4),
        "invalid_projection"
    );

    let mut ce5 = p0.clone();
    ce5.transport = "unavailable".into();
    ce5.components.get_mut("owner").unwrap().stale = true;
    ce5.summary.basis = "last_known_owner".into();
    ce5.activity.clear();
    ce5.activity_source_id = None;
    ce5.activity_coverage.state = "unavailable".into();
    assert_eq!(
        rejects("CE5 transport unavailable, other components fresh", &ce5),
        "invalid_projection"
    );

    let mut ce6 = p1.clone();
    ce6.activity_coverage.state = "not_requested".into();
    ce6.activity_coverage.linked_session_ids = vec!["01a142ba-b70b-7eb0-9900-b53bece80f30".into()];
    ce6.linkage = "exact_task_input_native_session; session_wide_not_per_turn".into();
    assert_eq!(
        rejects("CE6 unobserved + asserted linkage", &ce6),
        "invalid_unobserved_envelope"
    );

    let mut ce7 = p1.clone();
    ce7.transport = "read_completed".into();
    assert_eq!(
        rejects("CE7 unobserved under read_completed", &ce7),
        "invalid_unobserved_envelope"
    );

    // Unobserved summary and activity source are fixed by the constructor value too.
    let mut bad = p1.clone();
    bad.summary.reason = Some("malformed_owner_fields".into());
    rejects("unobserved summary differs", &bad);
    let mut bad = p1.clone();
    bad.components.get_mut("owner").unwrap().read_at_ms = 1;
    rejects("unobserved component read time", &bad);
}
