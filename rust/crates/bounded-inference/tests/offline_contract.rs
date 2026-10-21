use bounded_inference::{
    assemble_request, AttemptJournal, AttemptRecord, AttemptState, Bounds, ObservedTerminal,
    StreamError, StreamObserver, ToolFreeProfile, UsageSemantics,
};
use serde_json::{json, Value};
use std::fs;
use std::fs::OpenOptions;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_ID: AtomicU64 = AtomicU64::new(0);

fn bounds() -> Bounds {
    Bounds {
        request_bytes: 4096,
        stream_bytes: 16384,
        frame_bytes: 8192,
        output_bytes: 1024,
        events: 32,
        local_deadline_ms: 1000,
    }
}

fn profile() -> ToolFreeProfile {
    ToolFreeProfile {
        revision: "fixture-r1".into(),
        model: "selected-model".into(),
        bounds: bounds(),
    }
}

fn event(value: Value) -> Vec<u8> {
    format!(
        "event: {}\r\ndata: {}\r\n\r\n",
        value["type"].as_str().unwrap(),
        value
    )
    .into_bytes()
}

fn created() -> Value {
    json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-a","model":"observed-model"}})
}
fn added(seq: u64, index: usize, id: &str) -> Value {
    json!({"type":"response.output_item.added","sequence_number":seq,"output_index":index,"item":{"id":id,"type":"message"}})
}
fn delta(seq: u64, index: usize, id: &str, text: &str) -> Value {
    json!({"type":"response.output_text.delta","sequence_number":seq,"output_index":index,"item_id":id,"content_index":0,"delta":text})
}
fn terminal(seq: u64, status: &str, output: Value, usage: Option<Value>) -> Value {
    json!({"type":format!("response.{status}"),"sequence_number":seq,"response":{"id":"resp-a","model":"observed-model","status":status,"output":output,"usage":usage}})
}
fn item(id: &str, text: &str) -> Value {
    json!({"id":id,"type":"message","role":"assistant","content":[{"type":"output_text","text":text}]})
}

#[test]
fn exact_fixture_request_is_bounded_and_tool_free() {
    let request = assemble_request("literal\nα", "{\"candidate\":true}\n", &profile()).unwrap();
    let wire: Value = serde_json::from_slice(&request.bytes).unwrap();
    assert_eq!(wire["instructions"], "literal\nα");
    assert_eq!(
        wire["input"][0]["content"][0]["text"],
        "{\"candidate\":true}\n"
    );
    assert_eq!(wire["input"][0]["role"], "user");
    assert_eq!(wire["store"], false);
    assert_eq!(wire["stream"], true);
    for absent in [
        "tools",
        "previous_response_id",
        "max_output_tokens",
        "system",
    ] {
        assert!(wire.get(absent).is_none());
    }
    assert_eq!(request.sha256.len(), 64);
    assert_eq!(request.profile_revision, "fixture-r1");
    let mut tiny = profile();
    tiny.bounds.request_bytes = 10;
    assert!(assemble_request("x", "y", &tiny).is_err());
}

#[test]
fn fragmented_utf8_interleaved_identities_and_duplicate_event() {
    let mut observer = StreamObserver::new(bounds()).unwrap();
    let mut bytes = Vec::new();
    for v in [
        created(),
        added(2, 0, "a"),
        added(3, 1, "b"),
        delta(4, 1, "b", "β"),
        delta(5, 0, "a", "α"),
        delta(4, 1, "b", "β"),
        terminal(
            6,
            "completed",
            json!([item("a", "α"), item("b", "β")]),
            Some(json!({"input_tokens":11,"output_tokens":2,"total_tokens":13})),
        ),
    ] {
        bytes.extend(event(v));
    }
    bytes.extend(b"data: [DONE]\r\n\r\n");
    for chunk in bytes.chunks(1) {
        observer.push(chunk).unwrap();
    }
    let seen = observer.finish().unwrap();
    assert_eq!(seen.response_id.as_deref(), Some("resp-a"));
    assert_eq!(seen.observed_model.as_deref(), Some("observed-model"));
    assert_eq!(seen.terminal, Some(ObservedTerminal::Completed));
    assert_eq!(seen.output.as_deref(), Some("αβ"));
    assert_eq!(seen.unique_events, 6);
    assert_eq!(seen.usage.len(), 1);
    assert_eq!(seen.usage[0].semantics, UsageSemantics::Cumulative);
    assert_eq!(seen.usage[0].counts.cached_input, None);
}

#[test]
fn failed_after_text_does_not_supply_completed_output() {
    let mut observer = StreamObserver::new(bounds()).unwrap();
    for v in [
        created(),
        added(2, 0, "a"),
        delta(3, 0, "a", "looks valid"),
        json!({"type":"response.failed","sequence_number":4,"response":{"id":"resp-a","status":"failed","error":{"code":"insufficient_quota"},"usage":{"input_tokens":5}}}),
    ] {
        observer.push(&event(v)).unwrap();
    }
    let seen = observer.finish().unwrap();
    assert_eq!(
        seen.terminal,
        Some(ObservedTerminal::Failed {
            code: Some("insufficient_quota".into())
        })
    );
    assert_eq!(seen.output, None);
    assert_eq!(seen.usage[0].counts.input, Some(5));
    assert_eq!(seen.usage[0].counts.output, None);
}

#[test]
fn missing_incomplete_and_contradictory_terminal() {
    let mut missing = StreamObserver::new(bounds()).unwrap();
    missing.push(&event(created())).unwrap();
    assert_eq!(missing.finish().unwrap_err(), StreamError::MissingTerminal);

    let mut incomplete = StreamObserver::new(bounds()).unwrap();
    incomplete.push(&event(created())).unwrap();
    incomplete.push(&event(json!({"type":"response.incomplete","sequence_number":2,"response":{"id":"resp-a","status":"incomplete","incomplete_details":{"reason":"limit"}}}))).unwrap();
    assert_eq!(
        incomplete.finish().unwrap().terminal,
        Some(ObservedTerminal::Incomplete {
            reason: Some("limit".into())
        })
    );

    let mut conflicting = StreamObserver::new(bounds()).unwrap();
    conflicting.push(&event(created())).unwrap();
    conflicting
        .push(&event(terminal(2, "completed", json!([]), None)))
        .unwrap();
    assert_eq!(
        conflicting
            .push(&event(terminal(3, "failed", json!([]), None)))
            .unwrap_err(),
        StreamError::ConflictingTerminal
    );
}

#[test]
fn malformed_bounds_identity_and_tool_are_rejected() {
    let mut malformed = StreamObserver::new(bounds()).unwrap();
    assert_eq!(
        malformed.push(b"data: {oops}\n\n").unwrap_err(),
        StreamError::MalformedEvent
    );

    let mut tiny = bounds();
    tiny.frame_bytes = 8;
    let mut oversized = StreamObserver::new(tiny).unwrap();
    assert_eq!(
        oversized.push(&event(created())).unwrap_err(),
        StreamError::FrameTooLarge
    );

    let mut mismatch = StreamObserver::new(bounds()).unwrap();
    mismatch.push(&event(created())).unwrap();
    mismatch.push(&event(added(2, 0, "a"))).unwrap();
    assert_eq!(
        mismatch
            .push(&event(delta(3, 0, "wrong", "x")))
            .unwrap_err(),
        StreamError::ConflictingIdentity
    );

    let mut tool = StreamObserver::new(bounds()).unwrap();
    tool.push(&event(created())).unwrap();
    assert_eq!(tool.push(&event(json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"call","type":"function_call"}}))).unwrap_err(), StreamError::UnsupportedTool);
}

#[test]
fn timeout_and_disconnect_cannot_complete() {
    let mut timeout = StreamObserver::new(bounds()).unwrap();
    timeout.push(&event(created())).unwrap();
    assert_eq!(
        timeout.finish_due_to_timeout().unwrap_err(),
        StreamError::Timeout
    );
    let mut cut = StreamObserver::new(bounds()).unwrap();
    cut.push(b"data: {").unwrap();
    assert_eq!(cut.finish().unwrap_err(), StreamError::MalformedEvent);
}

#[test]
fn mixed_line_endings_and_distinct_usage_semantics() {
    let mut observer = StreamObserver::new(bounds()).unwrap();
    let mut bytes = event(created());
    bytes.extend(
        format!(
            "data: {}\n\n",
            json!({"type":"response.usage.delta","sequence_number":2,"usage":{"input_tokens":3}})
        )
        .as_bytes(),
    );
    bytes.extend(event(terminal(
        3,
        "completed",
        json!([]),
        Some(json!({"input_tokens":10,"output_tokens":1})),
    )));
    observer.push(&bytes).unwrap();
    let observed = observer.finish().unwrap();
    assert_eq!(observed.usage.len(), 2);
    assert_eq!(observed.usage[0].semantics, UsageSemantics::Incremental);
    assert_eq!(observed.usage[1].semantics, UsageSemantics::Cumulative);
    assert_eq!(observed.usage[1].counts.total, None);
}

#[test]
fn conflicting_replay_status_and_output_cap_fail_closed() {
    let mut conflicting = StreamObserver::new(bounds()).unwrap();
    conflicting.push(&event(created())).unwrap();
    let mut forged = created();
    forged["response"]["id"] = json!("resp-other");
    assert_eq!(
        conflicting.push(&event(forged)).unwrap_err(),
        StreamError::ConflictingIdentity
    );

    let mut status = StreamObserver::new(bounds()).unwrap();
    status.push(&event(created())).unwrap();
    let mut wrong = terminal(2, "completed", json!([]), None);
    wrong["response"]["status"] = json!("failed");
    assert_eq!(
        status.push(&event(wrong)).unwrap_err(),
        StreamError::ConflictingTerminal
    );
    assert_eq!(
        status.finish().unwrap_err(),
        StreamError::ConflictingTerminal
    );

    let mut cap = bounds();
    cap.output_bytes = 3;
    let mut output = StreamObserver::new(cap).unwrap();
    output.push(&event(created())).unwrap();
    assert_eq!(
        output
            .push(&event(terminal(
                2,
                "completed",
                json!([item("x", "four")]),
                None
            )))
            .unwrap_err(),
        StreamError::OutputTooLarge
    );
}

#[test]
fn ignored_parser_error_cannot_become_completed_observation() {
    let mut observer = StreamObserver::new(bounds()).unwrap();
    observer.push(&event(created())).unwrap();
    observer
        .push(&event(terminal(2, "completed", json!([]), None)))
        .unwrap();
    assert_eq!(
        observer.push(b"data: {bad}\n\n").unwrap_err(),
        StreamError::MalformedEvent
    );
    assert_eq!(observer.finish().unwrap_err(), StreamError::MalformedEvent);
}

#[test]
fn journal_intent_recovery_and_separate_attempts() {
    let dir = std::env::temp_dir().join(format!(
        "bounded-inference-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    let request = assemble_request("instruction", "input", &profile()).unwrap();
    let build = |attempt: &str| {
        AttemptRecord::prepared(
            "op-1",
            attempt,
            &request.sha256,
            &request.profile_revision,
            &request.selected_model,
            request.bounds.clone(),
        )
        .unwrap()
    };
    let path1 = dir.join("attempt-1.json");
    let mut first = AttemptJournal::create(&path1, build("attempt-1")).unwrap();
    assert_eq!(
        AttemptJournal::open(&path1).unwrap().record().state,
        AttemptState::Prepared
    );
    first.mark_send_intent().unwrap();
    assert!(AttemptJournal::open(&path1)
        .unwrap()
        .record()
        .remote_outcome_unknown());
    assert!(first.mark_send_intent().is_err());
    assert!(first
        .mark_terminal(bounded_inference::StreamObservation {
            response_id: None,
            observed_model: None,
            terminal: None,
            output: None,
            usage: vec![],
            unique_events: 0,
            local_failure: None,
        })
        .is_err());

    let mut second =
        AttemptJournal::create(dir.join("attempt-2.json"), build("attempt-2")).unwrap();
    second.mark_send_intent().unwrap();
    let mut observer = StreamObserver::new(bounds()).unwrap();
    observer.push(&event(created())).unwrap();
    observer
        .push(&event(terminal(
            2,
            "completed",
            json!([item("x", "{\"verdict\":true}")]),
            None,
        )))
        .unwrap();
    let result = observer.finish().unwrap();
    assert_eq!(result.output.as_deref(), Some("{\"verdict\":true}"));
    second.mark_terminal(result).unwrap();
    assert_eq!(
        AttemptJournal::open(dir.join("attempt-2.json"))
            .unwrap()
            .record()
            .state,
        AttemptState::TerminalObserved
    );
    assert!(AttemptJournal::create(&path1, build("attempt-1")).is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn streamed_text_must_match_done_and_terminal_at_content_identity() {
    let setup = || {
        let mut observer = StreamObserver::new(bounds()).unwrap();
        observer.push(&event(created())).unwrap();
        observer.push(&event(added(2, 0, "a"))).unwrap();
        observer.push(&event(json!({"type":"response.content_part.added","sequence_number":3,"output_index":0,"content_index":0,"item_id":"a","part":{"type":"output_text","text":""}}))).unwrap();
        observer.push(&event(delta(4, 0, "a", "REJECT"))).unwrap();
        observer
    };
    let mut done_conflict = setup();
    assert_eq!(done_conflict.push(&event(json!({"type":"response.output_text.done","sequence_number":5,"output_index":0,"content_index":0,"item_id":"a","text":"ALSO REJECT"}))).unwrap_err(), StreamError::ConflictingIdentity);
    let mut terminal_conflict = setup();
    assert_eq!(
        terminal_conflict
            .push(&event(terminal(
                5,
                "completed",
                json!([item("a", "ACCEPT")]),
                None
            )))
            .unwrap_err(),
        StreamError::ConflictingIdentity
    );
    let mut matched = setup();
    matched.push(&event(json!({"type":"response.output_text.done","sequence_number":5,"output_index":0,"content_index":0,"item_id":"a","text":"REJECT"}))).unwrap();
    matched.push(&event(json!({"type":"response.content_part.done","sequence_number":6,"output_index":0,"content_index":0,"item_id":"a","part":{"type":"output_text","text":"REJECT"}}))).unwrap();
    matched
        .push(&event(terminal(
            7,
            "completed",
            json!([item("a", "REJECT")]),
            None,
        )))
        .unwrap();
    assert_eq!(matched.finish().unwrap().output.as_deref(), Some("REJECT"));
}

#[test]
fn duplicate_item_ids_and_refusal_reasoning_are_explicitly_unsupported() {
    let mut duplicate = StreamObserver::new(bounds()).unwrap();
    duplicate.push(&event(created())).unwrap();
    duplicate.push(&event(added(2, 0, "a"))).unwrap();
    assert_eq!(
        duplicate.push(&event(added(3, 1, "a"))).unwrap_err(),
        StreamError::ConflictingIdentity
    );
    let mut refusal = StreamObserver::new(bounds()).unwrap();
    refusal.push(&event(created())).unwrap();
    refusal.push(&event(added(2, 0, "a"))).unwrap();
    assert_eq!(refusal.push(&event(json!({"type":"response.content_part.added","sequence_number":3,"output_index":0,"content_index":0,"item_id":"a","part":{"type":"refusal","refusal":"no"}}))).unwrap_err(), StreamError::UnsupportedContent);
    let mut reasoning = StreamObserver::new(bounds()).unwrap();
    reasoning.push(&event(created())).unwrap();
    assert_eq!(reasoning.push(&event(json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"r","type":"reasoning"}}))).unwrap_err(), StreamError::UnsupportedContent);
    let mut refusal_delta = StreamObserver::new(bounds()).unwrap();
    refusal_delta.push(&event(created())).unwrap();
    assert_eq!(refusal_delta.push(&event(json!({"type":"response.refusal.delta","sequence_number":2,"output_index":0,"content_index":0,"item_id":"a","delta":"no"}))).unwrap_err(), StreamError::UnsupportedContent);
}

#[test]
fn in_progress_and_usage_shapes_are_attributed() {
    let mut observer = StreamObserver::new(bounds()).unwrap();
    observer.push(&event(created())).unwrap();
    observer.push(&event(json!({"type":"response.in_progress","sequence_number":2,"response":{"id":"resp-a","model":"observed-model","status":"in_progress"}}))).unwrap();
    observer
        .push(&event(terminal(3, "completed", json!([]), None)))
        .unwrap();
    assert_eq!(
        observer.finish().unwrap().terminal,
        Some(ObservedTerminal::Completed)
    );
    for invalid in [
        json!("wrong"),
        json!({"input_tokens_details":"wrong"}),
        json!({"input_tokens":-1}),
    ] {
        let mut bad = StreamObserver::new(bounds()).unwrap();
        bad.push(&event(created())).unwrap();
        assert_eq!(
            bad.push(&event(terminal(2, "completed", json!([]), Some(invalid))))
                .unwrap_err(),
            StreamError::MalformedEvent
        );
    }
}

#[test]
fn eof_timeout_and_parse_failure_preserve_identity_usage_without_output() {
    let setup = || {
        let mut observer = StreamObserver::new(bounds()).unwrap();
        observer.push(&event(created())).unwrap();
        observer.push(&event(json!({"type":"response.usage.delta","sequence_number":2,"usage":{"input_tokens":4}}))).unwrap();
        observer
    };
    let eof = setup().finish_evidence();
    assert_eq!(eof.response_id.as_deref(), Some("resp-a"));
    assert_eq!(eof.usage[0].counts.input, Some(4));
    assert_eq!(eof.local_failure, Some(StreamError::MissingTerminal));
    assert_eq!(eof.output, None);
    let timeout = setup().timeout_evidence();
    assert_eq!(timeout.local_failure, Some(StreamError::Timeout));
    let mut malformed = setup();
    assert_eq!(
        malformed.push(b"data: {bad}\n\n").unwrap_err(),
        StreamError::MalformedEvent
    );
    let evidence = malformed.finish_evidence();
    assert_eq!(evidence.usage[0].counts.input, Some(4));
    assert_eq!(evidence.local_failure, Some(StreamError::MalformedEvent));
}

#[test]
fn frame_cap_is_independent_of_fragmentation() {
    let frame = event(created());
    let payload_len = frame.len() - 4;
    let mut cap = bounds();
    cap.frame_bytes = payload_len;
    let mut whole = StreamObserver::new(cap.clone()).unwrap();
    whole.push(&frame).unwrap();
    let mut split = StreamObserver::new(cap.clone()).unwrap();
    for b in &frame {
        split.push(std::slice::from_ref(b)).unwrap();
    }
    let mut too_small = cap;
    too_small.frame_bytes -= 1;
    let mut rejected = StreamObserver::new(too_small).unwrap();
    assert_eq!(
        rejected.push(&frame).unwrap_err(),
        StreamError::FrameTooLarge
    );
}

#[test]
fn stale_journal_handle_cannot_rewrite_terminal_and_corrupt_file_fails_closed() {
    let dir = std::env::temp_dir().join(format!(
        "bounded-journal-stale-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = dir.join("attempt.json");
    let record = AttemptRecord::prepared(
        "op",
        "attempt",
        &request.sha256,
        &request.profile_revision,
        &request.selected_model,
        bounds(),
    )
    .unwrap();
    let mut active = AttemptJournal::create(&path, record).unwrap();
    let mut stale = AttemptJournal::open(&path).unwrap();
    active.mark_send_intent().unwrap();
    let mut observer = StreamObserver::new(bounds()).unwrap();
    observer.push(&event(created())).unwrap();
    observer
        .push(&event(terminal(2, "completed", json!([]), None)))
        .unwrap();
    active.mark_terminal(observer.finish().unwrap()).unwrap();
    assert!(stale.mark_send_intent().is_err());
    assert_eq!(
        AttemptJournal::open(&path).unwrap().record().state,
        AttemptState::TerminalObserved
    );
    fs::write(&path, b"{torn").unwrap();
    assert!(AttemptJournal::open(&path).is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unknown_observation_is_durable_and_never_completed() {
    let dir = std::env::temp_dir().join(format!(
        "bounded-journal-unknown-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = dir.join("attempt.json");
    let record = AttemptRecord::prepared(
        "op",
        "attempt",
        &request.sha256,
        &request.profile_revision,
        &request.selected_model,
        bounds(),
    )
    .unwrap();
    let mut journal = AttemptJournal::create(&path, record).unwrap();
    journal.mark_send_intent().unwrap();
    let mut observer = StreamObserver::new(bounds()).unwrap();
    observer.push(&event(created())).unwrap();
    observer
        .push(&event(
            json!({"type":"response.usage.delta","sequence_number":2,"usage":{"input_tokens":5}}),
        ))
        .unwrap();
    journal.mark_unknown(observer.timeout_evidence()).unwrap();
    let reopened = AttemptJournal::open(&path).unwrap();
    assert_eq!(reopened.record().state, AttemptState::OutcomeUnknown);
    assert!(reopened.record().remote_outcome_unknown());
    assert_eq!(
        reopened.record().observation.as_ref().unwrap().usage[0]
            .counts
            .input,
        Some(5)
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn exact_synthetic_request_and_stream_artifacts_replay() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/synthetic-request-v1.json")).unwrap();
    let request = assemble_request(
        fixture["instructions"].as_str().unwrap(),
        fixture["input"].as_str().unwrap(),
        &profile(),
    )
    .unwrap();
    let wire: Value = serde_json::from_slice(&request.bytes).unwrap();
    assert_eq!(wire["instructions"], fixture["instructions"]);
    assert_eq!(wire["input"][0]["content"][0]["text"], fixture["input"]);
    let mut observer = StreamObserver::new(bounds()).unwrap();
    for chunk in include_bytes!("fixtures/synthetic-response-v1.sse").chunks(3) {
        observer.push(chunk).unwrap();
    }
    let observed = observer.finish().unwrap();
    assert_eq!(observed.terminal, Some(ObservedTerminal::Completed));
    assert_eq!(
        observed.output.as_deref(),
        fixture["expected_output"].as_str()
    );
    assert_eq!(observed.response_id.as_deref(), Some("resp-fixture"));
}

#[test]
fn persistence_fault_before_intent_and_terminal_preserves_recovery_state() {
    let dir = std::env::temp_dir().join(format!(
        "bounded-journal-fault-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = dir.join("attempt.json");
    let record = AttemptRecord::prepared(
        "op",
        "attempt",
        &request.sha256,
        &request.profile_revision,
        &request.selected_model,
        bounds(),
    )
    .unwrap();
    let mut journal = AttemptJournal::create(&path, record).unwrap();
    let lock = path.with_extension("writer-lock");
    let guard = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock)
        .unwrap();
    guard.try_lock().unwrap();
    assert!(journal.mark_send_intent().is_err());
    assert_eq!(
        AttemptJournal::open(&path).unwrap().record().state,
        AttemptState::Prepared
    );
    drop(guard);
    journal.mark_send_intent().unwrap();
    let guard = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock)
        .unwrap();
    guard.try_lock().unwrap();
    let mut observer = StreamObserver::new(bounds()).unwrap();
    observer.push(&event(created())).unwrap();
    observer
        .push(&event(terminal(2, "completed", json!([]), None)))
        .unwrap();
    assert!(journal.mark_terminal(observer.finish().unwrap()).is_err());
    assert!(AttemptJournal::open(&path)
        .unwrap()
        .record()
        .remote_outcome_unknown());
    drop(guard);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn crash_lock_child() {
    let Ok(path) = std::env::var("BOUNDED_INFERENCE_CRASH_LOCK_PATH") else {
        return;
    };
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .unwrap();
    file.try_lock().unwrap();
    file.sync_all().unwrap();
    std::process::exit(0);
}

#[test]
fn dead_writer_releases_lock_and_prepared_attempt_can_continue() {
    let dir = std::env::temp_dir().join(format!(
        "bounded-journal-dead-writer-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = dir.join("attempt.json");
    let record = AttemptRecord::prepared(
        "op",
        "attempt",
        &request.sha256,
        &request.profile_revision,
        &request.selected_model,
        bounds(),
    )
    .unwrap();
    let mut journal = AttemptJournal::create(&path, record).unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("crash_lock_child")
        .env(
            "BOUNDED_INFERENCE_CRASH_LOCK_PATH",
            path.with_extension("writer-lock"),
        )
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        AttemptJournal::open(&path).unwrap().record().state,
        AttemptState::Prepared
    );
    journal.mark_send_intent().unwrap();
    assert!(AttemptJournal::open(&path)
        .unwrap()
        .record()
        .remote_outcome_unknown());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn item_done_snapshot_and_terminal_status_must_be_coherent() {
    let mut conflicting = StreamObserver::new(bounds()).unwrap();
    conflicting.push(&event(created())).unwrap();
    conflicting.push(&event(added(2, 0, "a"))).unwrap();
    conflicting.push(&event(json!({"type":"response.output_item.done","sequence_number":3,"output_index":0,"item":{"id":"a","type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":"REJECT"}]}}))).unwrap();
    assert_eq!(
        conflicting
            .push(&event(terminal(
                4,
                "completed",
                json!([item("a", "ACCEPT")]),
                None
            )))
            .unwrap_err(),
        StreamError::ConflictingIdentity
    );

    let mut refusal = StreamObserver::new(bounds()).unwrap();
    refusal.push(&event(created())).unwrap();
    refusal.push(&event(added(2, 0, "a"))).unwrap();
    assert_eq!(refusal.push(&event(json!({"type":"response.output_item.done","sequence_number":3,"output_index":0,"item":{"id":"a","type":"message","status":"completed","role":"assistant","content":[{"type":"refusal","refusal":"no"}]}}))).unwrap_err(), StreamError::UnsupportedContent);

    let mut incomplete = StreamObserver::new(bounds()).unwrap();
    incomplete.push(&event(created())).unwrap();
    let mut bad_item = item("a", "ACCEPT");
    bad_item["status"] = json!("incomplete");
    assert_eq!(
        incomplete
            .push(&event(terminal(2, "completed", json!([bad_item]), None)))
            .unwrap_err(),
        StreamError::ConflictingTerminal
    );

    let mut good = StreamObserver::new(bounds()).unwrap();
    good.push(&event(created())).unwrap();
    good.push(&event(added(2, 0, "a"))).unwrap();
    good.push(&event(json!({"type":"response.output_item.done","sequence_number":3,"output_index":0,"item":{"id":"a","type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":"ACCEPT"}]}}))).unwrap();
    good.push(&event(terminal(
        4,
        "completed",
        json!([item("a", "ACCEPT")]),
        None,
    )))
    .unwrap();
    assert_eq!(good.finish().unwrap().output.as_deref(), Some("ACCEPT"));
}

#[test]
fn nonempty_content_start_cannot_be_replaced_by_terminal_text() {
    let mut observer = StreamObserver::new(bounds()).unwrap();
    observer.push(&event(created())).unwrap();
    observer.push(&event(added(2, 0, "a"))).unwrap();
    assert_eq!(observer.push(&event(json!({"type":"response.content_part.added","sequence_number":3,"output_index":0,"content_index":0,"item_id":"a","part":{"type":"output_text","text":"REJECT"}}))).unwrap_err(), StreamError::UnsupportedContent);
    assert_eq!(
        observer
            .push(&event(terminal(
                4,
                "completed",
                json!([item("a", "ACCEPT")]),
                None
            )))
            .unwrap_err(),
        StreamError::UnsupportedContent
    );
    let evidence = observer.finish_evidence();
    assert_eq!(evidence.output, None);
    assert_eq!(evidence.response_id.as_deref(), Some("resp-a"));
}

#[test]
fn response_envelope_matrix_rejects_hidden_state_and_preserves_terminal_details() {
    for invalid in [
        json!({"status":"failed"}),
        json!({"status":"incomplete"}),
        json!({"status":42}),
        json!({"output":[item("x", "REJECT")]}),
        json!({"output":{"bad":true}}),
        json!({"error":{"code":"quota"}}),
    ] {
        let mut value = created();
        for (key, field) in invalid.as_object().unwrap() {
            value["response"][key] = field.clone();
        }
        let mut observer = StreamObserver::new(bounds()).unwrap();
        assert!(observer.push(&event(value)).is_err());
        assert_eq!(observer.finish_evidence().output, None);
    }
    for invalid in [
        json!({"output":[item("x", "REJECT")]}),
        json!({"output":"wrong"}),
        json!({"error":{"code":"quota"}}),
        json!({"incomplete_details":{"reason":"limit"}}),
    ] {
        let mut observer = StreamObserver::new(bounds()).unwrap();
        observer.push(&event(created())).unwrap();
        let mut value = json!({"type":"response.in_progress","sequence_number":2,"response":{"id":"resp-a","status":"in_progress"}});
        for (key, field) in invalid.as_object().unwrap() {
            value["response"][key] = field.clone();
        }
        assert!(observer.push(&event(value)).is_err());
        assert_eq!(observer.finish_evidence().output, None);
    }
    let mut completed = StreamObserver::new(bounds()).unwrap();
    completed.push(&event(created())).unwrap();
    let mut value = terminal(2, "completed", json!([]), Some(json!({"input_tokens":3})));
    value["response"]["error"] = json!({"code":"quota"});
    assert_eq!(
        completed.push(&event(value)).unwrap_err(),
        StreamError::ConflictingTerminal
    );
    let evidence = completed.finish_evidence();
    assert_eq!(evidence.output, None);
    assert_eq!(evidence.usage[0].counts.input, Some(3));

    for (status, detail_key, detail_value) in [
        ("failed", "error", json!({"code":"quota"})),
        (
            "incomplete",
            "incomplete_details",
            json!({"reason":"limit"}),
        ),
    ] {
        let mut observer = StreamObserver::new(bounds()).unwrap();
        observer.push(&event(created())).unwrap();
        let mut value = terminal(2, status, json!([]), Some(json!({"input_tokens":5})));
        value["response"][detail_key] = detail_value;
        observer.push(&event(value)).unwrap();
        let observed = observer.finish().unwrap();
        assert_eq!(observed.usage[0].counts.input, Some(5));
        assert_eq!(observed.output, None);
        match observed.terminal.unwrap() {
            ObservedTerminal::Failed { code } => assert_eq!(code.as_deref(), Some("quota")),
            ObservedTerminal::Incomplete { reason } => assert_eq!(reason.as_deref(), Some("limit")),
            ObservedTerminal::Completed => panic!("unexpected completion"),
        }
    }
    for (status, detail_key) in [("failed", "error"), ("incomplete", "incomplete_details")] {
        for malformed in [
            json!("bad"),
            json!(42),
            json!(["bad"]),
            json!({"code":42,"reason":42}),
        ] {
            let mut observer = StreamObserver::new(bounds()).unwrap();
            observer.push(&event(created())).unwrap();
            let mut value = terminal(2, status, json!([]), Some(json!({"input_tokens":7})));
            value["response"][detail_key] = malformed;
            assert!(observer.push(&event(value)).is_err());
            let evidence = observer.finish_evidence();
            assert_eq!(evidence.output, None);
            assert_eq!(evidence.usage[0].counts.input, Some(7));
            assert!(matches!(
                evidence.terminal,
                Some(ObservedTerminal::Failed { .. } | ObservedTerminal::Incomplete { .. })
            ));
            assert_eq!(evidence.local_failure, Some(StreamError::MalformedEvent));
        }
    }
}

#[test]
fn item_snapshot_role_is_checked_even_without_content() {
    for kind in ["response.output_item.added", "response.output_item.done"] {
        let mut observer = StreamObserver::new(bounds()).unwrap();
        observer.push(&event(created())).unwrap();
        let value = json!({"type":kind,"sequence_number":2,"output_index":0,"item":{"id":"a","type":"message","role":"user"}});
        assert_eq!(
            observer.push(&event(value)).unwrap_err(),
            StreamError::MalformedEvent
        );
    }
}

#[test]
fn item_added_cannot_claim_final_status_then_extend_or_regress() {
    for finalized_status in ["completed", "incomplete", "failed"] {
        let mut observer = StreamObserver::new(bounds()).unwrap();
        observer.push(&event(created())).unwrap();
        let value = json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"a","type":"message","status":finalized_status,"role":"assistant","content":[{"type":"output_text","text":"A"}]}});
        assert_eq!(
            observer.push(&event(value)).unwrap_err(),
            StreamError::ConflictingTerminal
        );
        assert_eq!(
            observer
                .push(&event(terminal(
                    3,
                    "completed",
                    json!([item("a", "AB")]),
                    None
                )))
                .unwrap_err(),
            StreamError::ConflictingTerminal
        );
        let evidence = observer.finish_evidence();
        assert_eq!(evidence.output, None);
        assert_eq!(evidence.response_id.as_deref(), Some("resp-a"));
    }
    let mut valid = StreamObserver::new(bounds()).unwrap();
    valid.push(&event(created())).unwrap();
    valid.push(&event(json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"a","type":"message","status":"in_progress","role":"assistant","content":[{"type":"output_text","text":"A"}]}}))).unwrap();
    valid
        .push(&event(terminal(
            3,
            "completed",
            json!([item("a", "AB")]),
            None,
        )))
        .unwrap();
    assert_eq!(valid.finish().unwrap().output.as_deref(), Some("AB"));
}

#[test]
fn incomplete_item_preserves_incomplete_or_failed_terminal_usage() {
    for (terminal_kind, expected_terminal) in [
        ("incomplete", ObservedTerminal::Incomplete { reason: None }),
        ("failed", ObservedTerminal::Failed { code: None }),
    ] {
        let mut observer = StreamObserver::new(bounds()).unwrap();
        observer.push(&event(created())).unwrap();
        observer.push(&event(added(2, 0, "a"))).unwrap();
        observer.push(&event(delta(3, 0, "a", "partial"))).unwrap();
        observer.push(&event(json!({"type":"response.output_item.done","sequence_number":4,"output_index":0,"item":{"id":"a","type":"message","status":"incomplete","role":"assistant","content":[{"type":"output_text","text":"partial"}]}}))).unwrap();
        let mut partial = item("a", "partial");
        partial["status"] = json!("incomplete");
        observer
            .push(&event(terminal(
                5,
                terminal_kind,
                json!([partial]),
                Some(json!({"input_tokens":7,"output_tokens":2})),
            )))
            .unwrap();
        let observed = observer.finish().unwrap();
        assert_eq!(observed.terminal, Some(expected_terminal));
        assert_eq!(observed.output, None);
        assert_eq!(observed.usage[0].counts.input, Some(7));
    }

    let mut invalid = StreamObserver::new(bounds()).unwrap();
    invalid.push(&event(created())).unwrap();
    invalid.push(&event(added(2, 0, "a"))).unwrap();
    invalid.push(&event(json!({"type":"response.output_item.done","sequence_number":3,"output_index":0,"item":{"id":"a","type":"message","status":"incomplete","role":"assistant","content":[{"type":"output_text","text":"partial"}]}}))).unwrap();
    let mut partial = item("a", "partial");
    partial["status"] = json!("incomplete");
    assert_eq!(
        invalid
            .push(&event(terminal(4, "completed", json!([partial]), None)))
            .unwrap_err(),
        StreamError::ConflictingTerminal
    );

    for status in ["failed", "incomplete"] {
        let mut omitted = StreamObserver::new(bounds()).unwrap();
        omitted.push(&event(created())).unwrap();
        omitted.push(&event(added(2, 0, "a"))).unwrap();
        omitted.push(&event(delta(3, 0, "a", "partial"))).unwrap();
        omitted
            .push(&event(terminal(
                4,
                status,
                json!([]),
                Some(json!({"input_tokens":9})),
            )))
            .unwrap();
        let observed = omitted.finish().unwrap();
        assert_eq!(observed.output, None);
        assert_eq!(observed.usage[0].counts.input, Some(9));
    }
}
