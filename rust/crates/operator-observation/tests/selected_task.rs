use operator_observation::{
    Snapshot, digest,
    selected_task::{SelectedTask, linked_sessions, safe_text},
    task::{self, ReadError, Request, Selection},
};
use serde_json::{Value, json};
fn fixture() -> (Request, Value) {
    let request = Request {
        task_id: "task:u2".into(),
        delivery_id: Some("delivery:u2".into()),
        command_id: None,
    };
    let subject = "subject:u2";
    let cursor = json!({"store_id":"store:u2","stream_id":"stream:u2","commit_sequence":"1"});
    let segment = json!({"delivery_id":"delivery:u2","input_id":"input:u2","sequence":1,"delivery_state":"active","effect_id":"effect:u2","queue_attempt_id":"attempt:u2","segment_no":1,"invocation_id":"invocation:u2","native_attempt_id":"native:u2","predecessor_invocation_id":null,"transition_id":null,"segment_state":"completed","native_state":"completed","send_intent":true,"thread_id":"01a142ba-b70b-7eb0-9900-b53bece80f30","turn_id":"01a142ba-b722-7272-affc-928d3a20aeed","context_generation":"context:u2","final_sha256":null,"history_status":"complete"});
    let raw = json!({"protocol_version":2,"status":"observation","consistency":"independent_components","request":request,"host_identity":{"pid":1,"linux_starttime_ticks":"1","incarnation":"host:u2","build_id":"build:u2","profile":"retained-repository-analysis-v1","executable_sha256":"a".repeat(64),"source_sha256":"b".repeat(64),"lock_sha256":"c".repeat(64),"toolchain_sha256":"d".repeat(64),"features":[],"provider_identity":"synthetic_no_provider"},"components":{
    "owner":{"state":"present","scope":{"task_id":"task:u2","subject_id":subject},"revision":{"kind":"task","owner_revision":1,"cursor_revision":1},"value":{"task_id":"task:u2","subject_id":subject,"revision":1,"cursor_revision":1,"objective":"Verify selected task across clients","engineering_question":"What remains pending?","inputs":[{"input_id":"input:u2"}],"questions":[{"id":"question:u2","recipient_ref":"operator:u2","scope_ref":"task:u2","text":"Publish the private synthetic result?","state":"pending"}],"result":null}},
    "lifecycle":{"state":"present","scope":{"subject_id":subject,"delivery_id":"delivery:u2"},"revision":{"kind":"lifecycle","cursor":cursor},"value":{"protocol_version":1,"subject_id":subject,"context_generation":"context:u2","cursor":cursor,"transition":{"state":"unknown"},"current_admission":{"kind":"unknown"},"delivery":{"state":"unknown"},"runtime_availability":"unknown","runtime_provenance":{"state":"unknown"},"blocking_reason":{"state":"unknown"},"authority_refs":[],"evidence_refs":[],"measurement":{"state":"unknown"}}},
    "native_delivery_status":{"state":"present","scope":{"subject_id":subject,"collection":"native_delivery_segments","filter":"subject"},"revision":{"kind":"not_provided"},"completeness":{"kind":"complete_for_scope","limit":64},"value":{"protocol_version":1,"subject_id":subject,"recovery_only":false,"segments":[segment]}},
    "command_result":{"state":"not_requested","scope":{"subject_id":subject,"command_id":null,"task_binding":"not_established"}},"control_result":{"state":"not_requested","scope":{"subject_id":subject,"command_id":null,"task_binding":"not_established"}}
    }});
    (request, raw)
}
fn apply(s: &mut Selection, r: &Request, v: &Value) {
    let g = s.begin("/private/u2/host.sock".into(), r.clone());
    assert!(s.apply(g, task::decode(&serde_json::to_vec(v).unwrap(), r)));
}
#[test]
fn exact_join_conflicts_capacity_and_source_fencing() {
    let (r, raw) = fixture();
    let mut s = Selection::default();
    apply(&mut s, &r, &raw);
    assert_eq!(linked_sessions(&s).len(), 1);
    for pointer in [
        "/components/native_delivery_status/value/segments/0/input_id",
        "/components/native_delivery_status/value/segments/0/delivery_id",
        "/components/native_delivery_status/value/segments/0/thread_id",
    ] {
        let mut bad = raw.clone();
        *bad.pointer_mut(pointer).unwrap() = json!("conflict");
        let mut s = Selection::default();
        apply(&mut s, &r, &bad);
        assert!(linked_sessions(&s).is_empty());
    }
    for pointer in [
        "/components/owner/value/task_id",
        "/components/owner/value/subject_id",
        "/components/native_delivery_status/value/subject_id",
    ] {
        let mut bad = raw.clone();
        *bad.pointer_mut(pointer).unwrap() = json!("conflict");
        assert!(task::decode(&serde_json::to_vec(&bad).unwrap(), &r).is_err());
    }
    let mut capacity = raw.clone();
    let row = capacity["components"]["native_delivery_status"]["value"]["segments"][0].clone();
    capacity["components"]["native_delivery_status"]["value"]["segments"] = json!(vec![row; 65]);
    assert!(task::decode(&serde_json::to_vec(&capacity).unwrap(), &r).is_err());
    let source = digest(b"synthetic");
    let projected = SelectedTask::project(&s, &source, None, true).unwrap();
    assert_eq!(projected.activity.len(), 0);
    let sessions = operator_observation::collect(
        &digest(b"other-source"),
        "Owner-linked session activity",
        &linked_sessions(&s),
        |_| Err("source_unavailable"),
    )
    .unwrap();
    assert!(SelectedTask::project(&s, &source, Some(&sessions), true).is_err());
}
#[test]
fn changes_without_activity_and_stale_original_times() {
    let (r, mut raw) = fixture();
    let mut s = Selection::default();
    apply(&mut s, &r, &raw);
    let source = digest(b"synthetic");
    let first = SelectedTask::project(&s, &source, None, true).unwrap();
    raw["components"]["owner"]["value"]["questions"][0]["state"] =
        json!({"answered":{"answer":"Yes, publish","authority_ref":"operator:u2"}});
    raw["components"]["owner"]["value"]["result"] = json!({"body":"Synthetic result","result_id":"result:u2","operation_id":"operation:u2","attribution":"operator:u2","source_refs":[]});
    apply(&mut s, &r, &raw);
    let second = SelectedTask::project(&s, &source, None, true).unwrap();
    assert_ne!(
        first.components["owner"].rows,
        second.components["owner"].rows
    );
    assert!(second.activity.is_empty());
    let at = second.components["owner"].last_known_at_ms;
    let g = s.begin("/private/u2/host.sock".into(), r.clone());
    s.apply(g, Err(ReadError::Unavailable));
    let failed = SelectedTask::project(&s, &source, None, true).unwrap();
    assert_eq!(failed.components["owner"].last_known_at_ms, at);
    assert!(failed.components["owner"].stale);
    assert!(linked_sessions(&s).is_empty());
    assert!(!s.apply(g - 1, task::decode(&serde_json::to_vec(&raw).unwrap(), &r)));
    apply(&mut s, &r, &raw);
    assert!(!s.components["owner"].stale);
    raw["host_identity"]["incarnation"] = json!("host:replacement");
    apply(&mut s, &r, &raw);
    let third = SelectedTask::project(&s, &source, None, true).unwrap();
    assert_ne!(first.scope_id, third.scope_id);
}
#[test]
fn shared_projection_fixture_and_private_redaction() {
    let (r, mut raw) = fixture();
    // Stable joined-client control includes a pending question and a separately
    // published result. U3's unchanged lifecycle harness requires this result.
    raw["components"]["owner"]["value"]["result"] = json!({"body":"Independent synthetic result","result_id":"result:joined","operation_id":"operation:joined","attribution":"operator:u2","source_refs":[]});
    let mut s = Selection::default();
    apply(&mut s, &r, &raw);
    let source = digest(b"synthetic");
    let ids = linked_sessions(&s);
    let activity=operator_observation::collect(&source,"Owner-linked session activity",&ids, |_|Ok(json!({"sessionId":ids[0],"host":"codex","sessionTitle":"Private synthetic activity","entries":[]}))).unwrap();
    let p = SelectedTask::project(&s, &source, Some(&activity), true).unwrap();
    assert_eq!(p.summary.value.as_ref().unwrap().pending_count, 1);
    assert!(matches!(
        p.summary.value.as_ref().unwrap().result,
        operator_observation::selected_task::ResultSummary::Published { .. }
    ));
    let bytes = serde_json::to_vec_pretty(&p).unwrap();
    SelectedTask::parse(&bytes).unwrap();
    assert!(
        !safe_text("Bearer secret-value sk-private ghp_private password=hunter2")
            .contains("secret-value")
    );
    assert!(!safe_text(r#"{"body":"sk-private"}"#).contains("sk-private"));
    if let Some(dir) = std::env::var_os("U2_FIXTURE_OUTPUT") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("selected-task.json"), &bytes).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            dir.join("selected-task.json"),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        std::fs::write(
            dir.join("to2.json"),
            serde_json::to_vec_pretty(&raw).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("activity.json"),
            serde_json::to_vec_pretty(&activity).unwrap(),
        )
        .unwrap();
    }
    let mut invalid = p.clone();
    invalid.endpoint = "/changed".into();
    assert!(invalid.validate().is_err());
    let _: Option<Snapshot> = None;
}

#[test]
fn clock_independent_owner_changes_and_refused_anchor_provenance() {
    let (request, mut raw) = fixture();
    let mut selection = Selection::default();
    let generation = selection.begin("/private/u2/host.sock".into(), request.clone());
    selection.apply_at(
        generation,
        task::decode(&serde_json::to_vec(&raw).unwrap(), &request),
        1000,
    );
    let source = digest(b"synthetic");
    let first = SelectedTask::project(&selection, &source, None, true).unwrap();
    let bytes = serde_json::to_vec(&first).unwrap();
    assert_eq!(
        operator_observation::selected_task::age_label(1000, 121000),
        "Observed 120s ago"
    );
    assert!(operator_observation::selected_task::age_label(1000, 122000).contains("stale"));
    assert!(operator_observation::selected_task::age_label(0, 122000).contains("unavailable"));
    assert_eq!(serde_json::to_vec(&first).unwrap(), bytes);
    // A lifecycle revision changes independently while task and Events stay unchanged.
    raw["components"]["lifecycle"]["revision"]["cursor"]["commit_sequence"] = json!("2");
    raw["components"]["lifecycle"]["value"]["cursor"]["commit_sequence"] = json!("2");
    let g = selection.begin("/private/u2/host.sock".into(), request.clone());
    selection.apply_at(
        g,
        task::decode(&serde_json::to_vec(&raw).unwrap(), &request),
        2000,
    );
    let second = SelectedTask::project(&selection, &source, None, true).unwrap();
    assert_ne!(
        first.components["lifecycle"].revision,
        second.components["lifecycle"].revision
    );
    assert_eq!(
        first.components["owner"].rows,
        second.components["owner"].rows
    );
    let scope = raw["components"]["native_delivery_status"]["scope"].clone();
    raw["components"]["native_delivery_status"] =
        json!({"state":"refused","scope":scope,"reason":"capacity_exceeded","limit":64});
    let g = selection.begin("/private/u2/host.sock".into(), request.clone());
    selection.apply_at(
        g,
        task::decode(&serde_json::to_vec(&raw).unwrap(), &request),
        3000,
    );
    let refused = SelectedTask::project(&selection, &source, None, true).unwrap();
    let c = &refused.components["native_delivery_status"];
    assert_eq!(c.state, "refused");
    assert!(c.stale);
    assert_eq!(c.read_at_ms, 3000);
    assert_eq!(c.last_known_at_ms, Some(2000));
    assert_eq!(
        c.last_known_completeness,
        json!({"kind":"complete_for_scope","limit":64})
    );
    assert_eq!(c.last_known_scope, scope);
    assert_eq!(c.completeness, Value::Null);
    assert!(linked_sessions(&selection).is_empty());
    let changed = SelectedTask::project(&selection, &digest(b"other-account"), None, true).unwrap();
    assert_ne!(refused.scope_id, changed.scope_id);
    let mut impossible = refused.clone();
    impossible.components.get_mut("owner").unwrap().read_at_ms = impossible.assembled_at_ms + 1;
    assert!(impossible.validate().is_err());
}
#[test]
fn structured_private_fields_are_redacted_before_rows_are_serialized() {
    let (r, mut raw) = fixture();
    raw["components"]["owner"]["value"]["result"] = json!({"body":"safe controlled result","nested":{"password":"two word credential","api_key":"another private phrase"}});
    let mut selection = Selection::default();
    apply(&mut selection, &r, &raw);
    let projection = SelectedTask::project(&selection, &digest(b"synthetic"), None, true).unwrap();
    let text = projection.components["owner"].rows.join("\n");
    assert!(text.contains("safe controlled result"));
    assert!(text.contains("[REDACTED]"));
    assert!(!text.contains("two word"));
    assert!(!text.contains("private phrase"));
}

#[test]
fn browser_vectors_are_shared_rust_records_with_independent_cuts() {
    let (request, raw) = fixture();
    let source = digest(b"synthetic");
    let mut selection = Selection::default();
    apply(&mut selection, &request, &raw);
    let initial = SelectedTask::project(&selection, &source, None, true).unwrap();
    let mut changed = raw.clone();
    changed["components"]["owner"]["revision"]["owner_revision"] = json!(2);
    changed["components"]["owner"]["revision"]["cursor_revision"] = json!(2);
    changed["components"]["owner"]["value"]["revision"] = json!(2);
    changed["components"]["owner"]["value"]["cursor_revision"] = json!(2);
    changed["components"]["owner"]["value"]["questions"][0]["state"] =
        json!({"answered":{"answer":"Yes, publish","authority_ref":"operator:u2"}});
    changed["components"]["owner"]["value"]["result"] = json!({"body":"Independent synthetic result","result_id":"result:u2","operation_id":"operation:u2","attribution":"operator:u2","source_refs":[],"password":"private multiword phrase"});
    apply(&mut selection, &request, &changed);
    let answered = SelectedTask::project(&selection, &source, None, true).unwrap();
    assert_ne!(
        initial.components["owner"].rows,
        answered.components["owner"].rows
    );
    changed["components"]["lifecycle"]["revision"]["cursor"]["commit_sequence"] = json!("2");
    changed["components"]["lifecycle"]["value"]["cursor"]["commit_sequence"] = json!("2");
    apply(&mut selection, &request, &changed);
    let lifecycle = SelectedTask::project(&selection, &source, None, true).unwrap();
    assert_eq!(
        answered.components["owner"].rows,
        lifecycle.components["owner"].rows
    );
    let g = selection.begin("/private/u2/host.sock".into(), request.clone());
    selection.apply(g, Err(ReadError::Unavailable));
    let failed = SelectedTask::project(&selection, &source, None, true).unwrap();
    assert!(failed.components.values().all(|c| c.stale));
    assert!(failed.activity.is_empty());
    apply(&mut selection, &request, &changed);
    let reconnected = SelectedTask::project(&selection, &source, None, true).unwrap();
    assert!(reconnected.components.values().all(|c| !c.stale));
    changed["host_identity"]["incarnation"] = json!("host:replacement");
    apply(&mut selection, &request, &changed);
    let host = SelectedTask::project(&selection, &source, None, true).unwrap();
    assert_ne!(host.scope_id, reconnected.scope_id);
    let source_changed =
        SelectedTask::project(&selection, &digest(b"other-source"), None, true).unwrap();
    assert_ne!(host.scope_id, source_changed.scope_id);
    for (name, projection) in [
        ("initial", initial),
        ("answered", answered),
        ("lifecycle", lifecycle),
        ("failed", failed),
        ("reconnected", reconnected),
        ("host", host),
        ("source", source_changed),
    ] {
        let bytes = serde_json::to_vec_pretty(&projection).unwrap();
        SelectedTask::parse(&bytes).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("private multiword phrase"));
        if let Some(dir) = std::env::var_os("U2_FIXTURE_OUTPUT") {
            std::fs::write(
                std::path::Path::new(&dir).join(format!("vector-{name}.json")),
                bytes,
            )
            .unwrap();
        }
    }
}

#[test]
fn generated_selected_task_schema_matches_checked_contract() {
    let generated = serde_json::to_value(schemars::schema_for!(SelectedTask)).unwrap();
    let checked: Value =
        serde_json::from_slice(include_bytes!("../docs/selected-task.schema.json")).unwrap();
    assert_eq!(generated, checked);
}

#[test]
fn owner_question_id_matches_fresh_control() {
    let raw: Value =
        serde_json::from_slice(include_bytes!("fixtures/fresh-completed-control.json")).unwrap();
    let r: Request = serde_json::from_value(raw["request"].clone()).unwrap();
    let mut s = Selection::default();
    let g = s.begin("/private/control.sock".into(), r.clone());
    assert!(s.apply(g, task::decode(&serde_json::to_vec(&raw).unwrap(), &r)));
    let p = SelectedTask::project(&s, &digest(b"control"), None, true).unwrap();
    assert!(
        p.components["owner"]
            .rows
            .iter()
            .any(|r| r.starts_with("Question question:ui:sim:"))
    );
    assert!(
        !p.components["owner"]
            .rows
            .join("\n")
            .contains("Question unprovided")
    );
}

#[test]
fn cli_interval_independent_capacity_and_retained_provenance() {
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::{fs::PermissionsExt, net::UnixListener},
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let root = std::env::var_os("U2_REPAIR_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("u2-cli-repair-{}", std::process::id()))
        });
    std::fs::create_dir_all(&root).unwrap();
    let socket = root.join("counterexample.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let profile = root.join("profile.json");
    std::fs::write(&profile, serde_json::to_vec(&json!({"version":1,"owner":"/private/nonexistent-owner","owner_sha256":"586128d82ebd1fa0c8813e5583924cd9ce812e6ffe76de0c2e63d396414e2a8b","codex_home":"/private/codex","claude_home":"/private/claude","state_dir":"/private/state","bridge":null,"context":null})).unwrap()).unwrap();
    std::fs::set_permissions(&profile, std::fs::Permissions::from_mode(0o600)).unwrap();
    let output = root.join("published.json");
    let _ = std::fs::remove_file(&output);
    let log = std::fs::File::create(root.join("cli.stdout")).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-observe"))
        .arg("selected-task")
        .arg(&socket)
        .args(["task:u2", "--delivery", "delivery:u2", "--profile"])
        .arg(&profile)
        .arg("--output")
        .arg(&output)
        .args([
            "--interval",
            "2",
            "--human",
            "--inspect",
            "native_delivery_status",
        ])
        .stdout(log)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    struct Guard<'a>(&'a mut std::process::Child);
    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let guard = Guard(&mut child);
    let (r, base) = fixture();
    let mut last = 0;
    let mut anchor = Value::Null;
    for (stage, n) in [
        ("20", 20),
        ("21", 21),
        ("64", 64),
        ("65-refusal", 65),
        ("failure1", 0),
        ("failure2", 0),
    ] {
        let mut raw = base.clone();
        let row = raw["components"]["native_delivery_status"]["value"]["segments"][0].clone();
        raw["components"]["native_delivery_status"]["value"]["segments"] = json!(
            (0..n)
                .map(|i| {
                    let mut row = row.clone();
                    row["thread_id"] = json!(format!("00000000-0000-0000-0000-{i:012x}"));
                    row["segment_no"] = json!(i + 1);
                    row["invocation_id"] = json!(format!("invocation:u2:{i}"));
                    row["native_attempt_id"] = json!(format!("native:u2:{i}"));
                    row
                })
                .collect::<Vec<_>>()
        );
        if n == 65 {
            let scope = raw["components"]["native_delivery_status"]["scope"].clone();
            raw["components"]["native_delivery_status"] =
                json!({"state":"refused","scope":scope,"reason":"capacity_exceeded","limit":64});
        }
        if n == 0 {
            raw["components"]["owner"] = json!({"state":"unavailable","scope":{"task_id":"task:u2"},"reason":"task_read_unavailable"});
            for name in ["lifecycle", "native_delivery_status"] {
                let mut scope = raw["components"][name]["scope"].clone();
                scope["subject_id"] = Value::Null;
                raw["components"][name] = json!({"state":"unavailable","scope":scope,"reason":"task_dependency_unavailable"});
            }
        }
        task::decode(&serde_json::to_vec(&raw).unwrap(), &r).unwrap();
        let until = Instant::now() + Duration::from_secs(40);
        let (mut stream, _) = loop {
            if let Ok(s) = listener.accept() {
                break s;
            }
            assert!(Instant::now() < until, "accept {stage}");
            assert!(guard.0.try_wait().unwrap().is_none(), "CLI aborted {stage}");
            std::thread::sleep(Duration::from_millis(10));
        };
        let mut line = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        stream
            .write_all(&serde_json::to_vec(&raw).unwrap())
            .unwrap();
        stream.write_all(b"\n").unwrap();
        drop(stream);
        let p = loop {
            if let Ok(bytes) = std::fs::read(&output)
                && let Ok(p) = SelectedTask::parse(&bytes)
                && p.assembled_at_ms > last
            {
                break p;
            }
            assert!(Instant::now() < until, "publish {stage}");
            assert!(guard.0.try_wait().unwrap().is_none(), "CLI aborted {stage}");
            std::thread::sleep(Duration::from_millis(10));
        };
        last = p.assembled_at_ms;
        assert_eq!(p.components.len(), 5);
        if n == 20 {
            assert_eq!(p.activity.len(), 20);
            assert_eq!(p.activity_coverage.state, "read_completed");
        }
        if n == 21 || n == 64 {
            assert_eq!(p.activity_coverage.state, "refused_capacity");
            assert_eq!(p.activity_coverage.linked_session_ids.len(), n);
            assert!(p.activity.is_empty());
            assert!(
                p.components["native_delivery_status"].rows[0]
                    .contains(&format!("{n} native segments"))
            );
            anchor = serde_json::to_value(&p.components["native_delivery_status"]).unwrap();
        }
        if n == 65 || n == 0 {
            let c = &p.components["native_delivery_status"];
            assert!(c.stale);
            assert_eq!(c.last_known_at_ms, anchor["last_known_at_ms"].as_u64());
            assert_eq!(c.last_known_scope, anchor["scope"]);
            assert_eq!(c.last_known_completeness, anchor["completeness"]);
        }
        if n == 0 {
            assert!(
                p.components["owner"]
                    .rows
                    .join("\n")
                    .contains("Question question:u2:")
            );
        }
        std::fs::write(
            root.join(format!("{stage}.json")),
            serde_json::to_vec_pretty(&p).unwrap(),
        )
        .unwrap();
    }
    drop(guard);
    let stdout = std::fs::read_to_string(root.join("cli.stdout")).unwrap();
    assert!(stdout.contains("native_delivery_status detail"));
    assert!(stdout.contains("last_known_at_ms"));
    assert!(stdout.contains("complete_for_scope"));
    assert!(stdout.contains("capacity_exceeded"));
    std::fs::remove_file(socket).unwrap();
}

fn summary_projection(raw: &Value, r: &Request) -> SelectedTask {
    let mut s = Selection::default();
    apply(&mut s, r, raw);
    SelectedTask::project(&s, &digest(b"synthetic"), None, true).unwrap()
}
#[test]
fn typed_summary_owner_shapes_and_capacity() {
    let (r, base) = fixture();
    let first = summary_projection(&base, &r);
    let v = first.summary.value.as_ref().unwrap();
    assert_eq!(v.pending_count, 1);
    assert_eq!(v.answered_count, 0);
    assert!(matches!(
        v.result,
        operator_observation::selected_task::ResultSummary::NotPublished
    ));
    assert!(
        first
            .human_view(None, first.assembled_at_ms, false)
            .contains("No result published")
    );
    assert!(
        !first
            .human_view(None, first.assembled_at_ms, false)
            .contains("segment_state")
    );
    for (change, reason) in [
        ("missing", "missing_owner_fields"),
        ("malformed", "malformed_owner_fields"),
        ("unknown", "unsupported_owner_value"),
        ("duplicate", "malformed_owner_fields"),
        ("id", "missing_owner_fields"),
        ("null", "malformed_owner_fields"),
    ] {
        let mut raw = base.clone();
        let o = &mut raw["components"]["owner"]["value"];
        match change {
            "missing" => {
                o.as_object_mut().unwrap().remove("result");
            }
            "malformed" => o["result"] = json!({"body":"legacy"}),
            "unknown" => o["questions"][0]["state"] = json!("future"),
            "duplicate" => {
                o["questions"] = json!([o["questions"][0].clone(), o["questions"][0].clone()])
            }
            "id" => {
                o["questions"][0].as_object_mut().unwrap().remove("id");
                o["questions"][0]["question_id"] = json!("wrong");
            }
            _ => o["questions"] = Value::Null,
        }
        let p = summary_projection(&raw, &r);
        assert_eq!(p.summary.state, "unavailable", "{change}");
        // Partial result is missing required fields, not an unpublished result.
        assert_eq!(
            p.summary.reason.as_deref(),
            Some(if change == "malformed" {
                "missing_owner_fields"
            } else {
                reason
            })
        );
        assert_eq!(p.components.len(), 5);
    }
    for n in [0, 64, 65] {
        let mut raw = base.clone();
        let q = raw["components"]["owner"]["value"]["questions"][0].clone();
        raw["components"]["owner"]["value"]["questions"] = json!(
            (0..n)
                .map(|i| {
                    let mut q = q.clone();
                    q["id"] = json!(format!("q:{i}"));
                    q
                })
                .collect::<Vec<_>>()
        );
        let p = summary_projection(&raw, &r);
        if n <= 64 {
            assert_eq!(p.summary.value.unwrap().pending_count, n);
        } else {
            assert_eq!(p.summary.reason.as_deref(), Some("summary_capacity"));
        }
        assert_eq!(p.components.len(), 5);
    }
    let mut raw = base.clone();
    raw["components"]["owner"]["value"]["result"] = json!({"result_id":"r","operation_id":"op","body":"published while pending","attribution":"operator","source_refs":[]});
    let p = summary_projection(&raw, &r);
    assert_eq!(p.summary.value.as_ref().unwrap().pending_count, 1);
    assert!(
        p.human_view(None, p.assembled_at_ms, false)
            .contains("Published result")
    );
    let mut forged = p.clone();
    forged.summary.basis = "last_known_owner".into();
    assert!(forged.validate().is_err());
    let mut lost_metadata = p.clone();
    lost_metadata.components.get_mut("owner").unwrap().revision = Value::Null;
    assert!(lost_metadata.validate().is_err());
    let mut old = serde_json::to_value(&p).unwrap();
    old["schema"] = json!("work-engine.selected-task.v1");
    assert!(SelectedTask::parse(&serde_json::to_vec(&old).unwrap()).is_err());
}
#[test]
fn whole_string_sanitization_clipping_and_full_envelope() {
    let (r, mut raw) = fixture();
    // Secret recognition must inspect the entire word before the excerpt cap.
    raw["components"]["owner"]["value"]["objective"] =
        json!(format!("{}sk-private", "界".repeat(1020)));
    raw["components"]["owner"]["value"]["engineering_question"] = json!("界".repeat(1100));
    raw["components"]["owner"]["value"]["questions"][0]["text"] = json!(format!(
        "{} Bearer hidden-token \u{1b}[2J",
        "word ".repeat(1100)
    ));
    raw["components"]["owner"]["value"]["result"] = json!({"result_id":"r","operation_id":"o","body":"safe","attribution":"operator","source_refs":[],"authorization":"multi word private"});
    let p = summary_projection(&raw, &r);
    let v = p.summary.value.as_ref().unwrap();
    assert_eq!(v.objective.text, "[REDACTED]");
    assert!(v.engineering_question.truncated);
    assert_eq!(v.engineering_question.text.chars().count(), 1024);
    assert!(v.questions[0].text.truncated);
    assert!(
        p.human_view(None, p.assembled_at_ms, false)
            .contains("excerpt clipped")
    );
    let bytes = serde_json::to_string(&p).unwrap();
    for secret in [
        "sk-private",
        "hidden-token",
        "multi word private",
        "\\u001b",
    ] {
        assert!(!bytes.contains(secret), "{secret}");
    }
    assert!(bytes.contains("[REDACTED]"));
    assert!(
        p.human_view(Some("questions"), p.assembled_at_ms, false)
            .contains("excerpt clipped")
    );
    SelectedTask::parse(bytes.as_bytes()).unwrap();
}
#[test]
fn summary_retains_original_anchor_and_clears_on_absence() {
    let (r, raw) = fixture();
    let mut s = Selection::default();
    let g = s.begin("/private/u2/host.sock".into(), r.clone());
    s.apply_at(
        g,
        task::decode(&serde_json::to_vec(&raw).unwrap(), &r),
        1000,
    );
    let initial = SelectedTask::project(&s, &digest(b"s"), None, true).unwrap();
    for _ in 0..2 {
        let g = s.begin("/private/u2/host.sock".into(), r.clone());
        s.apply_at(g, Err(ReadError::Unavailable), 2000);
        let p = SelectedTask::project(&s, &digest(b"s"), None, true).unwrap();
        assert_eq!(p.summary.basis, "last_known_owner");
        assert_eq!(p.components["owner"].read_at_ms, 1000);
        assert_eq!(p.components["owner"].last_known_at_ms, Some(1000));
        assert_eq!(
            serde_json::to_value(&p.summary.value).unwrap(),
            serde_json::to_value(&initial.summary.value).unwrap()
        );
    }
    // The interrupted test directly replaced this component while leaving the
    // failed transport in place. That synthetic publication must be rejected.
    let absent = owner_failure(&raw, "absent", json!({"task_id":"task:u2"}));
    let owner: task::Component =
        serde_json::from_value(absent["components"]["owner"].clone()).unwrap();
    let c = s.components.get_mut("owner").unwrap();
    c.latest = owner;
    c.last_known = None;
    c.last_known_at_ms = None;
    c.stale = false;
    assert_eq!(
        SelectedTask::project(&s, &digest(b"s"), None, true).unwrap_err(),
        "invalid_summary_basis"
    );
    // A real completed absence runs through decoding and Selection::apply_at,
    // clears the transport error, and clears every retained owner field.
    let p = project_at(&mut s, &r, &absent, 3000);
    assert_eq!(p.transport, "read_completed");
    assert!(!p.components["owner"].stale);
    assert_eq!(p.summary.basis, "none");
    assert!(p.summary.value.is_none());
    let g = s.begin("/private/u2/host.sock".into(), r.clone());
    s.apply_at(g, Err(ReadError::Unavailable), 4000);
    let p = SelectedTask::project(&s, &digest(b"s"), None, true).unwrap();
    assert_eq!(p.summary.basis, "none");
    assert!(p.components["owner"].last_known_at_ms.is_none());
    assert!(p.components["owner"].rows.is_empty());
}

#[test]
fn cli_compact_overview_and_typed_inspectors() {
    use std::{os::unix::fs::PermissionsExt, process::Command};
    let (_, raw) = fixture();
    let path = std::env::temp_dir().join(format!("s1-replay-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_vec(&raw).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let run = |inspect: Option<&str>| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_work-engine-observe"));
        c.args([
            "selected-task",
            "/private/u2/host.sock",
            "task:u2",
            "--delivery",
            "delivery:u2",
            "--human",
            "--replay",
        ])
        .arg(&path);
        if let Some(route) = inspect {
            c.args(["--inspect", route]);
        }
        c.output().unwrap()
    };
    let result = run(None);
    assert!(result.status.success());
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("Objective: Verify selected task"));
    assert!(text.contains("1 pending"));
    assert!(text.contains("No result published"));
    assert!(!text.contains("segment_state"));
    assert!(!text.contains("Latest scope"));
    for (route, expected) in [
        ("questions", "Recipient operator:u2"),
        ("result", "No result published"),
        ("owner", "owner detail"),
        ("activity", "Activity read: not_requested"),
    ] {
        let result = run(Some(route));
        assert!(result.status.success(), "{route}");
        assert!(
            String::from_utf8(result.stdout).unwrap().contains(expected),
            "{route}"
        );
    }
    assert!(!run(Some("success")).status.success());
    std::fs::remove_file(path).unwrap();
}

fn owner_failure(raw: &Value, state: &str, scope: Value) -> Value {
    let mut out = raw.clone();
    out["components"]["owner"] = if state == "absent" {
        json!({"state":"absent","scope":scope,"record_kind":"task"})
    } else if state == "refused" {
        json!({"state":"refused","scope":scope,"reason":"capacity_exceeded","limit":64})
    } else {
        json!({"state":"unavailable","scope":scope,"reason":"task_read_unavailable"})
    };
    for name in ["lifecycle", "native_delivery_status"] {
        let mut scope = raw["components"][name]["scope"].clone();
        scope["subject_id"] = Value::Null;
        out["components"][name] = json!({"state":if state=="absent" {"not_applicable"}else{"unavailable"},"scope":scope,"reason":if state=="absent" {"task_absent"}else{"task_dependency_unavailable"}});
    }
    out
}
fn project_at(s: &mut Selection, r: &Request, raw: &Value, at: u64) -> SelectedTask {
    let g = s.begin("/private/u2/host.sock".into(), r.clone());
    let decoded = task::decode(&serde_json::to_vec(raw).unwrap(), r).unwrap();
    assert!(s.apply_at(g, Ok(decoded), at));
    let p = SelectedTask::project(s, &digest(b"synthetic"), None, true).unwrap();
    SelectedTask::parse(&serde_json::to_vec(&p).unwrap()).unwrap()
}
fn present_tuple(p: &SelectedTask, retained: bool) -> Value {
    let c = &p.components["owner"];
    if retained {
        json!([
            c.last_known_scope,
            c.last_known_revision,
            c.last_known_completeness,
            c.last_known_at_ms
        ])
    } else {
        json!([c.scope, c.revision, c.completeness, c.read_at_ms])
    }
}
#[test]
fn reviewed_owner_basis_contract_and_shared_vectors() {
    use std::collections::BTreeMap;
    let (r, raw) = fixture();
    let mut s = Selection::default();
    let mut vectors = BTreeMap::<String, Value>::new();
    let fresh = project_at(&mut s, &r, &raw, 1000);
    assert_eq!(present_tuple(&fresh, false), present_tuple(&fresh, true));
    vectors.insert("fresh".into(), serde_json::to_value(&fresh).unwrap());
    for (name, at) in [("transport_failed", 2000), ("repeated_failed", 3000)] {
        let g = s.begin("/private/u2/host.sock".into(), r.clone());
        s.apply_at(g, Err(ReadError::Unavailable), at);
        let p = SelectedTask::project(&s, &digest(b"synthetic"), None, true).unwrap();
        assert_eq!(p.summary.basis, "last_known_owner");
        assert_eq!(present_tuple(&p, false), present_tuple(&fresh, false));
        assert_eq!(present_tuple(&p, true), present_tuple(&fresh, true));
        assert_eq!(
            serde_json::to_value(&p.summary.value).unwrap(),
            serde_json::to_value(&fresh.summary.value).unwrap()
        );
        vectors.insert(name.into(), serde_json::to_value(&p).unwrap());
    }
    for (index, (name, state, scope)) in [
        (
            "refused_anchor",
            "refused",
            raw["components"]["owner"]["scope"].clone(),
        ),
        (
            "unavailable_anchor",
            "unavailable",
            raw["components"]["owner"]["scope"].clone(),
        ),
        (
            "unresolved_anchor",
            "unavailable",
            json!({"task_id":"task:u2"}),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let at = 4000 + index as u64 * 100;
        let p = project_at(&mut s, &r, &owner_failure(&raw, state, scope), at);
        assert_eq!(p.summary.basis, "last_known_owner");
        assert_eq!(present_tuple(&p, true), present_tuple(&fresh, true));
        assert_eq!(p.components["owner"].read_at_ms, at);
        assert_ne!(
            p.components["owner"].revision,
            p.components["owner"].last_known_revision
        );
        vectors.insert(name.into(), serde_json::to_value(p).unwrap());
        let g = s.begin("/private/u2/host.sock".into(), r.clone());
        s.apply_at(g, Err(ReadError::Unavailable), at + 50);
        let p = SelectedTask::project(&s, &digest(b"synthetic"), None, true).unwrap();
        assert_eq!(present_tuple(&p, true), present_tuple(&fresh, true));
        assert_eq!(p.components["owner"].read_at_ms, at);
        vectors.insert(format!("{name}_failed"), serde_json::to_value(p).unwrap());
    }
    let mut empty = Selection::default();
    let g = empty.begin("/private/u2/host.sock".into(), r.clone());
    empty.apply_at(g, Err(ReadError::Unavailable), 5000);
    let p = SelectedTask::project(&empty, &digest(b"synthetic"), None, true).unwrap();
    assert_eq!(p.summary.basis, "none");
    vectors.insert("initial_failure".into(), serde_json::to_value(p).unwrap());
    let absent = owner_failure(&raw, "absent", json!({"task_id":"task:u2"}));
    let p = project_at(&mut s, &r, &absent, 6000);
    assert!(p.components["owner"].rows.is_empty());
    assert_eq!(p.summary.basis, "none");
    vectors.insert("absent".into(), serde_json::to_value(p).unwrap());
    let g = s.begin("/private/u2/host.sock".into(), r.clone());
    s.apply_at(g, Err(ReadError::Unavailable), 7000);
    let p = SelectedTask::project(&s, &digest(b"synthetic"), None, true).unwrap();
    assert_eq!(p.summary.basis, "none");
    vectors.insert("absent_failed".into(), serde_json::to_value(p).unwrap());
    project_at(&mut s, &r, &raw, 8000);
    let different = owner_failure(
        &raw,
        "unavailable",
        json!({"task_id":"task:u2","subject_id":"subject:other"}),
    );
    let p = project_at(&mut s, &r, &different, 9000);
    assert_eq!(p.summary.basis, "none");
    vectors.insert(
        "explicit_subject_clears".into(),
        serde_json::to_value(p).unwrap(),
    );
    project_at(&mut s, &r, &raw, 10000);
    let mut replaced = owner_failure(&raw, "unavailable", json!({"task_id":"task:u2"}));
    replaced["host_identity"]["incarnation"] = json!("host:replacement");
    let p = project_at(&mut s, &r, &replaced, 11000);
    assert_eq!(p.summary.basis, "none");
    vectors.insert("identity_reset".into(), serde_json::to_value(p).unwrap());
    vectors.insert(
        "zero_time".into(),
        serde_json::to_value(project_at(&mut Selection::default(), &r, &raw, 0)).unwrap(),
    );
    for (name, kind) in [
        ("summary_missing", "missing"),
        ("summary_malformed", "malformed"),
        ("summary_unsupported", "unsupported"),
        ("summary_capacity", "capacity"),
    ] {
        let mut raw = raw.clone();
        let owner = &mut raw["components"]["owner"]["value"];
        match kind {
            "missing" => {
                owner.as_object_mut().unwrap().remove("result");
            }
            "malformed" => owner["objective"] = json!(42),
            "unsupported" => owner["questions"][0]["state"] = json!("future"),
            _ => {
                let q = owner["questions"][0].clone();
                owner["questions"] = json!(
                    (0..65)
                        .map(|i| {
                            let mut q = q.clone();
                            q["id"] = json!(format!("q:{i}"));
                            q
                        })
                        .collect::<Vec<_>>()
                );
            }
        }
        let p = project_at(&mut Selection::default(), &r, &raw, 12000);
        assert!(p.summary.value.is_none());
        vectors.insert(name.into(), serde_json::to_value(p).unwrap());
    }
    for (name, path, value) in [
        (
            "anchor_subject",
            "/components/owner/last_known_scope/subject_id",
            json!("subject:different"),
        ),
        (
            "anchor_task",
            "/components/owner/last_known_scope/task_id",
            json!("task:different"),
        ),
        (
            "anchor_revision",
            "/components/owner/last_known_revision/owner_revision",
            json!(2),
        ),
        (
            "anchor_completeness",
            "/components/owner/last_known_completeness",
            json!({}),
        ),
        (
            "anchor_time",
            "/components/owner/last_known_at_ms",
            json!(999),
        ),
        (
            "partial_scope",
            "/components/owner/last_known_scope",
            Value::Null,
        ),
        (
            "partial_revision",
            "/components/owner/last_known_revision",
            Value::Null,
        ),
        (
            "partial_time",
            "/components/owner/last_known_at_ms",
            Value::Null,
        ),
        (
            "stale_present_completed",
            "/transport",
            json!("read_completed"),
        ),
    ] {
        let mut bad = vectors["transport_failed"].clone();
        *bad.pointer_mut(path).unwrap() = value;
        vectors.insert(name.into(), bad);
    }
    let mut bad = vectors["fresh"].clone();
    bad["transport"] = json!("unavailable");
    vectors.insert("transport_latest".into(), bad);
    let mut bad = vectors["fresh"].clone();
    bad["components"]["owner"]["last_known_revision"]["cursor_revision"] = json!(2);
    vectors.insert("fresh_copy_revision".into(), bad);
    let mut bad = vectors["unavailable_anchor"].clone();
    bad["components"]["owner"]["scope"]["subject_id"] = json!("subject:different");
    vectors.insert("failure_explicit_subject".into(), bad);
    let mut bad = vectors["absent"].clone();
    for key in [
        "last_known_scope",
        "last_known_revision",
        "last_known_completeness",
        "last_known_at_ms",
    ] {
        bad["components"]["owner"][key] = vectors["fresh"]["components"]["owner"][key].clone();
    }
    vectors.insert("absence_retains".into(), bad);
    let mut bad = vectors["initial_failure"].clone();
    bad["components"]["owner"]["rows"] = json!(["retained data"]);
    vectors.insert("no_anchor_rows".into(), bad);
    let mut bad = vectors["initial_failure"].clone();
    bad["components"]["owner"]["last_known_scope"] =
        vectors["fresh"]["components"]["owner"]["scope"].clone();
    vectors.insert("no_anchor_scope".into(), bad);
    for name in [
        "summary_missing",
        "summary_malformed",
        "summary_unsupported",
        "summary_capacity",
    ] {
        let mut bad = vectors[name].clone();
        bad["transport"] = json!("unavailable");
        bad["components"]["owner"]["stale"] = json!(true);
        bad["summary"]["basis"] = json!("last_known_owner");
        bad["components"]["owner"]["last_known_scope"]["subject_id"] = json!("subject:wrong");
        vectors.insert(format!("{name}_bad_basis"), bad);
    }
    let contract: Value = serde_json::from_str(include_str!(
        "fixtures/selected-task-consistency-contract.json"
    ))
    .unwrap();
    assert_eq!(vectors.len(), contract["cases"].as_array().unwrap().len());
    for case in contract["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        assert_eq!(
            SelectedTask::parse(&serde_json::to_vec(&vectors[name]).unwrap()).is_ok(),
            case["accept"].as_bool().unwrap(),
            "{name}"
        );
    }
    if let Some(dir) = std::env::var_os("U2_FIXTURE_OUTPUT") {
        let cli = std::path::Path::new(&dir).join("cli-consistency");
        std::fs::create_dir_all(&cli).unwrap();
        for case in contract["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["accept"] == true)
        {
            let name = case["name"].as_str().unwrap();
            let p = SelectedTask::parse(&serde_json::to_vec(&vectors[name]).unwrap()).unwrap();
            for route in [None, Some("owner"), Some("questions"), Some("result")] {
                std::fs::write(
                    cli.join(format!("{name}-{}.txt", route.unwrap_or("overview"))),
                    p.human_view(route, 15000, false),
                )
                .unwrap();
            }
        }
        std::fs::write(
            std::path::Path::new(&dir).join("consistency-vectors.json"),
            serde_json::to_vec_pretty(&json!({"contract":contract,"vectors":vectors})).unwrap(),
        )
        .unwrap();
    }
}

const REPAIR_FRAGMENTS: &[&str] = &[
    "OBJalpha",
    "OBJbeta",
    "ENGalpha",
    "ENGbeta",
    "Qalpha",
    "Qbeta",
    "ANSalpha",
    "ANSbeta",
    "ANScorner",
    "ANSslash",
    "RESalpha",
    "RESbeta",
    "RESnext",
    "REStail",
    "ROWalpha",
    "ROWbeta",
    "ENDalpha",
    "ENDbeta",
];
#[test]
fn lexical_credentials_and_equal_consumer_vectors() {
    let controls = [
        (
            r#"before password: "blue green" after"#,
            vec!["blue", "green"],
            vec!["before", "after"],
        ),
        (
            r#"'API-KEY' = 'violet amber' color=blue"#,
            vec!["violet", "amber"],
            vec!["color=blue"],
        ),
        (
            r#"TOKEN="one \"two"credential='three \\four' username=Alice"#,
            vec!["one", "two", "three", "four"],
            vec!["username=Alice"],
        ),
        (
            r#"secret : 'unclosed orange yellow"#,
            vec!["orange", "yellow", "unclosed"],
            vec!["secret"],
        ),
        (
            r#"authorization = Bearer "cedar pine" note=kept"#,
            vec!["cedar", "pine"],
            vec!["note=kept"],
        ),
        (
            r#"Bearer 'silver gold' safe"#,
            vec!["silver", "gold"],
            vec!["safe"],
        ),
        (
            "password=first;token=second&api_key=third, color=blue",
            vec!["first", "second", "third"],
            vec!["color=blue"],
        ),
        (
            "username='Alice Bob' color=blue count=3",
            vec![],
            vec!["Alice Bob", "color=blue", "count=3"],
        ),
    ];
    for (raw, fragments, kept) in controls {
        let clean = safe_text(raw);
        for fragment in fragments {
            assert!(!clean.contains(fragment), "{raw}: {fragment} in {clean}");
        }
        for control in kept {
            assert!(clean.contains(control), "lost {control} in {clean}");
        }
    }
    let (r, mut raw) = fixture();
    let o = &mut raw["components"]["owner"]["value"];
    o["objective"] = json!(format!(
        "keep-objective {} Password : \"OBJalpha OBJbeta\"",
        "界".repeat(1020)
    ));
    o["engineering_question"] =
        json!("keep-engineering 'API-KEY' = 'ENGalpha ENGbeta' safe-engineering");
    o["questions"][0]["text"] =
        json!("keep-question \"authorization\" : \"Bearer Qalpha Qbeta\" safe-question");
    o["questions"][0]["state"] = json!({"answered":{"answer":r#"keep-answer secret='ANSalpha \'ANSbeta' credential="ANScorner\\ANSslash" safe-answer"#,"authority_ref":"operator:u2"}});
    let mut pending = o["questions"][0].clone();
    pending["id"] = json!("question:pending");
    pending["state"] = json!("pending");
    pending["text"] = json!("Unanswered independent question");
    o["questions"].as_array_mut().unwrap().push(pending);
    o["result"] = json!({"result_id":"result:repair","operation_id":"operation:repair","body":r#"keep-result TOKEN="RESalpha \"RESbeta"password='RESnext REStail' color=blue secret='ENDalpha ENDbeta"#,"attribution":"operator:u2","source_refs":[],"inspection":"keep-row credential : 'ROWalpha ROWbeta' safe-row"});
    let p = project_at(&mut Selection::default(), &r, &raw, 1000);
    let bytes = serde_json::to_vec_pretty(&p).unwrap();
    let serialized = String::from_utf8(bytes.clone()).unwrap();
    let v = p.summary.value.as_ref().unwrap();
    assert_eq!(v.pending_count, 1);
    assert_eq!(v.answered_count, 1);
    assert!(v.objective.truncated);
    assert!(matches!(
        v.result,
        operator_observation::selected_task::ResultSummary::Published { .. }
    ));
    for fragment in REPAIR_FRAGMENTS {
        assert!(
            !serialized.contains(fragment),
            "serialized secret {fragment}"
        );
    }
    for control in [
        "keep-objective",
        "keep-engineering",
        "safe-engineering",
        "keep-question",
        "safe-question",
        "keep-answer",
        "safe-answer",
        "keep-result",
        "color=blue",
        "keep-row",
        "safe-row",
    ] {
        assert!(serialized.contains(control), "lost {control}");
    }
    for route in [
        None,
        Some("questions"),
        Some("result"),
        Some("owner"),
        Some("lifecycle"),
        Some("native_delivery_status"),
        Some("command_result"),
        Some("control_result"),
        Some("activity"),
    ] {
        let text = p.human_view(route, 2000, false);
        for fragment in REPAIR_FRAGMENTS {
            assert!(!text.contains(fragment), "terminal {route:?}: {fragment}");
        }
    }
    use std::{os::unix::fs::PermissionsExt, process::Command};
    let path =
        std::env::temp_dir().join(format!("s1-repair-credentials-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_vec(&raw).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    for route in [None, Some("questions"), Some("result"), Some("owner")] {
        let mut c = Command::new(env!("CARGO_BIN_EXE_work-engine-observe"));
        c.args([
            "selected-task",
            "/private/u2/host.sock",
            "task:u2",
            "--delivery",
            "delivery:u2",
            "--human",
            "--replay",
        ])
        .arg(&path);
        if let Some(route) = route {
            c.args(["--inspect", route]);
        }
        let result = c.output().unwrap();
        assert!(result.status.success());
        let text = String::from_utf8(result.stdout).unwrap();
        assert!(text.contains("[REDACTED]") || text.contains("excerpt clipped"));
        for fragment in REPAIR_FRAGMENTS {
            assert!(!text.contains(fragment), "CLI {route:?}: {fragment}");
        }
        if let Some(dir) = std::env::var_os("U2_FIXTURE_OUTPUT") {
            std::fs::write(
                std::path::Path::new(&dir)
                    .join(format!("cli-lexical-{}.txt", route.unwrap_or("overview"))),
                &text,
            )
            .unwrap();
        }
    }
    std::fs::remove_file(path).unwrap();
    if let Some(dir) = std::env::var_os("U2_FIXTURE_OUTPUT") {
        let dir = std::path::Path::new(&dir);
        std::fs::write(dir.join("vector-lexical.json"), &bytes).unwrap();
        std::fs::write(
            dir.join("lexical-fragments.json"),
            serde_json::to_vec_pretty(REPAIR_FRAGMENTS).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn r2_full_u64_producer_and_raw_json_contract() {
    let (request, base) = fixture();
    let mut vectors = serde_json::Map::new();
    let mut cases = Vec::new();
    let mut add = |name: String, bytes: String, accept: bool| {
        assert_eq!(SelectedTask::parse(bytes.as_bytes()).is_ok(), accept, "{name}");
        cases.push(json!({"name":name,"json":bytes,"accept":accept}));
    };
    for field in ["owner_revision", "cursor_revision"] {
        for value in [0u64, 9007199254740991, 9007199254740992, 9007199254740993, 9007199254740994, u64::MAX] {
            let mut raw = base.clone();
            raw["components"]["owner"]["revision"][field] = json!(value);
            raw["components"]["owner"]["value"][if field == "owner_revision" { "revision" } else {field}] = json!(value);
            let mut selection = Selection::default();
            eprintln!("R3 synthetic TO2 {field}={value}");
            emit_to2(&format!("{field}_{value}"), &serde_json::to_vec(&raw).unwrap());
            let fresh = project_at(&mut selection, &request, &raw, 1000);
            assert_eq!(fresh.components["owner"].revision[field], value.to_string());
            assert_eq!(fresh.components["owner"].last_known_revision[field], value.to_string());
            let identity = fresh.owner_basis_identity();
            for (state, at) in [("fresh",1000), ("retained",2000), ("repeated_failure",3000), ("reconnect",4000)] {
                let p = if state == "fresh" {fresh.clone()} else if state == "reconnect" { project_at(&mut selection, &request, &raw, at) } else {
                    let generation = selection.begin("/private/u2/host.sock".into(), request.clone());
                    selection.apply_at(generation, Err(ReadError::Unavailable), at);
                    SelectedTask::project(&selection, &digest(b"synthetic"), None, true).unwrap()
                };
                assert_eq!(p.components["owner"].last_known_revision[field], value.to_string());
                let name = format!("{field}_{value}_{state}");
                let bytes = serde_json::to_string(&p).unwrap();
                add(name.clone(),bytes,true);
                if value >= 9007199254740992 { assert!(p.human_view(Some("owner"),5000,false).contains(&value.to_string())); }
                vectors.insert(name,serde_json::to_value(p).unwrap());
            }
            if value == 9007199254740993 {
                let mut next = fresh.clone();
                next.components.get_mut("owner").unwrap().revision[field] = json!("9007199254740994");
                next.components.get_mut("owner").unwrap().last_known_revision[field] = json!("9007199254740994");
                assert_ne!(identity,next.owner_basis_identity());
            }
        }
    }
    let positive = vectors["owner_revision_9007199254740993_fresh"].clone();
    for field in ["owner_revision","cursor_revision"] {
        for (label, value) in [("numeric",json!(9007199254740993u64)),("overflow",json!("18446744073709551616")),("leading_zero",json!("01")),("sign",json!("+1")),("space",json!(" 1")),("fraction",json!("1.0")),("exponent",json!("1e2")),("unicode",json!("١")),("null",Value::Null),("bool",json!(true)),("array",json!([])),("object",json!({}))] {
            let mut bad = positive.clone();
            bad["components"]["owner"]["revision"][field]=value.clone();
            bad["components"]["owner"]["last_known_revision"][field]=value;
            add(format!("{field}_{label}"),serde_json::to_string(&bad).unwrap(),false);
        }
        let mut bad=positive.clone();bad["components"]["owner"]["last_known_revision"][field]=json!("9007199254740992");
        add(format!("{field}_string_mismatch"),serde_json::to_string(&bad).unwrap(),false);
        let mut numeric=positive.clone();numeric["components"]["owner"]["revision"][field]=json!(9007199254740993u64);numeric["components"]["owner"]["last_known_revision"][field]=json!(9007199254740992u64);
        add(format!("{field}_numeric_collapse"),serde_json::to_string(&numeric).unwrap(),false);
    }
    for key in ["scope","revision"] {
        let mut bad=positive.clone();bad["components"]["owner"][key]["extra"]=json!(9007199254740993u64);
        bad["components"]["owner"][format!("last_known_{key}")]["extra"]=json!(9007199254740992u64);
        add(format!("{key}_extra_collapse"),serde_json::to_string(&bad).unwrap(),false);
    }
    let mut bad=positive.clone();bad["components"]["owner"]["revision"].as_object_mut().unwrap().remove("cursor_revision");add("partial_revision".into(),serde_json::to_string(&bad).unwrap(),false);
    for state in ["absent","unavailable","refused"] {
        for subject in [None,Some(Value::Null),Some(json!("subject:different"))] {
            let mut scope=json!({"task_id":"task:u2"});if let Some(subject)=subject {scope["subject_id"]=subject;}
            let mut selection=Selection::default();
            let p=project_at(&mut selection,&request,&owner_failure(&base,state,scope),5000);
            let mut v=serde_json::to_value(p).unwrap();
            add(format!("{state}_scope_{}",cases_len_tag(&v)),serde_json::to_string(&v).unwrap(),true);
            for (label,id) in [("mismatch",json!("task:wrong")),("null",Value::Null),("number",json!(3))] {
                v["components"]["owner"]["scope"]["task_id"]=id;
                add(format!("{state}_{label}_{}",cases_len_tag(&v)),serde_json::to_string(&v).unwrap(),false);
            }
        }
    }
    let mut selection=Selection::default();let generation=selection.begin("/private/u2/host.sock".into(),request.clone());selection.apply_at(generation,Err(ReadError::Unavailable),0);
    let sentinel=SelectedTask::project(&selection,&digest(b"synthetic"),None,true).unwrap();
    add("exact_sentinel".into(),serde_json::to_string(&sentinel).unwrap(),true);
    for (label,scope) in [("missing_task",json!({"subject_id":"subject:u2"})),("extra",json!({"extra":1}))] {let mut v=serde_json::to_value(&sentinel).unwrap();v["components"]["owner"]["scope"]=scope;add(label.into(),serde_json::to_string(&v).unwrap(),false);}
    if let Some(dir)=std::env::var_os("U2_FIXTURE_OUTPUT") {
        std::fs::write(std::path::Path::new(&dir).join("r2-raw-vectors.json"),serde_json::to_vec_pretty(&cases).unwrap()).unwrap();
        for (name,v) in vectors {std::fs::write(std::path::Path::new(&dir).join(format!("r2-{name}.json")),serde_json::to_vec_pretty(&v).unwrap()).unwrap();}
    }
}
fn cases_len_tag(v:&Value)->String { v["components"]["owner"]["scope"]["subject_id"].to_string() }

fn emit_to2(name: &str, bytes: &[u8]) {
    if let Some(dir) = std::env::var_os("U2_FIXTURE_OUTPUT") {
        let dir = std::path::Path::new(&dir).join("raw-to2");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{name}.json")), bytes).unwrap();
    }
}

#[test]
fn r3_strict_task_domain_separation() {
    let (request, base) = fixture();
    let check = |name: &str, bytes: Vec<u8>, accept: bool| {
        emit_to2(name, &bytes);
        assert_eq!(operator_observation::task::decode(&bytes, &request).is_ok(), accept, "{name}");
    };
    for field in ["owner_revision", "cursor_revision"] {
        let value_field = if field == "owner_revision" { "revision" } else {field};
        for token in ["-1", "1.0", "1e0", "18446744073709551616", "\"1\"", "null", "true"] {
            let mut raw=base.clone();
            raw["components"]["owner"]["revision"][field]=json!("REPLACE_TOKEN");
            raw["components"]["owner"]["value"][value_field]=json!("REPLACE_TOKEN");
            let bytes=serde_json::to_string(&raw).unwrap().replace("\"REPLACE_TOKEN\"",token).into_bytes();
            check(&format!("negative_{field}_{token:?}" ).replace('/',"_"),bytes,false);
        }
        let mut raw=base.clone();raw["components"]["owner"]["revision"][field]=json!(u64::MAX);
        check(&format!("unequal_{field}"),serde_json::to_vec(&raw).unwrap(),false);
    }
    let mut raw=base.clone();raw["components"]["owner"]["value"]["nested_integer"] = json!({"items":[u64::MAX,9007199254740993u64]});
    check("nested_task_integers",serde_json::to_vec(&raw).unwrap(),true);
    for name in ["lifecycle","native_delivery_status","command_result","control_result"] {
        let mut raw=base.clone();raw["components"][name]["ignored_integer"]=json!(9007199254740992u64);
        check(&format!("other_domain_{name}"),serde_json::to_vec(&raw).unwrap(),false);
    }
    for name in ["owner","lifecycle","native_delivery_status","command_result","control_result"] {
        let mut raw=base.clone();raw["components"][name]["scope"]["integer_probe"]=json!(1);
        check(&format!("scope_policy_control_{name}"),serde_json::to_vec(&raw).unwrap(),true);
        raw["components"][name]["scope"]["integer_probe"]=json!(9007199254740992u64);
        check(&format!("scope_policy_large_{name}"),serde_json::to_vec(&raw).unwrap(),false);
    }
    let mut ignored=base.clone();ignored["components"]["owner"]["value"]["ignored"]=json!("DUPLICATE_MARKER");
    for (name,value) in [("ignored_duplicate",r#"{"x":1,"x":2}"#),("ignored_escaped_duplicate",r#"{"x":1,"\u0078":2}"#)] {
        check(name,serde_json::to_string(&ignored).unwrap().replace("\"DUPLICATE_MARKER\"",value).into_bytes(),false);
    }
    for token in ["9007199254740991", "9007199254740992", "18446744073709551615"] {
        assert_eq!(lifecycle_wire::parse_strict_json(token.as_bytes()).is_ok(), token=="9007199254740991");
    }
    let bytes=serde_json::to_string(&base).unwrap();
    for (name,suffix) in [("trailing_json"," {}"),("trailing_garbage","x")] {check(name,format!("{bytes}{suffix}").into_bytes(),false);}
    for (name,extra) in [("duplicate","\"x\":1,\"x\":2"),("escaped_duplicate","\"x\":1,\"\\u0078\":2"),("nested_duplicate","\"nested\":{\"x\":1,\"x\":2}")] {
        check(name,bytes.replacen('{',&format!("{{{extra},"),1).into_bytes(),false);
    }
    check("frame_overflow",vec![b' ';operator_observation::task::MAX_FRAME+1],false);
}

#[test]
fn r3_typed_host_construction_through_selection() {
    let Some(dir)=std::env::var_os("U2_FIXTURE_OUTPUT") else {return};
    let dir=std::path::Path::new(&dir);
    if !dir.join("typed-owner").exists() {return;}
    let (request,base)=fixture();
    for field in ["owner_revision","cursor_revision"] {
        for n in [0u64,9007199254740991,9007199254740992,9007199254740993,9007199254740994,u64::MAX] {
            let mut raw=base.clone();
            raw["components"]["owner"]=serde_json::from_slice(&std::fs::read(dir.join("typed-owner").join(format!("{field}_{n}.json"))).unwrap()).unwrap();
            emit_to2(&format!("typed_{field}_{n}"),&serde_json::to_vec(&raw).unwrap());
            let replay=dir.join("raw-to2").join(format!("typed_{field}_{n}.json"));
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&replay,std::fs::Permissions::from_mode(0o600)).unwrap();
            let cli=std::process::Command::new(env!("CARGO_BIN_EXE_work-engine-observe"))
                .args(["selected-task","/private/u2/host.sock","task:u2","--delivery","delivery:u2","--replay"])
                .arg(&replay).output().unwrap();
            assert!(cli.status.success(),"CLI stderr {}",String::from_utf8_lossy(&cli.stderr));
            let cli_value:Value=serde_json::from_slice(&cli.stdout).unwrap();
            assert_eq!(cli_value["components"]["owner"]["revision"][field],n.to_string());
            std::fs::write(dir.join(format!("cli-{field}_{n}.json")),&cli.stdout).unwrap();
            let human=std::process::Command::new(env!("CARGO_BIN_EXE_work-engine-observe"))
                .args(["selected-task","/private/u2/host.sock","task:u2","--delivery","delivery:u2","--replay"])
                .arg(&replay).args(["--human","--inspect","owner"]).output().unwrap();
            assert!(human.status.success());assert!(String::from_utf8_lossy(&human.stdout).contains(&n.to_string()));
            std::fs::write(dir.join(format!("cli-{field}_{n}.txt")),human.stdout).unwrap();
            let mut selection=Selection::default();let p=project_at(&mut selection,&request,&raw,1000);
            assert_eq!(p.components["owner"].revision[field],n.to_string());
            let bytes=serde_json::to_vec(&p).unwrap();SelectedTask::parse(&bytes).unwrap();
            std::fs::write(dir.join(format!("typed-{field}_{n}.json")),&bytes).unwrap();
            if field=="owner_revision" && n==u64::MAX {std::fs::write(dir.join("canonical-typed-selected-task.json"),bytes).unwrap();}
        }
    }
}
