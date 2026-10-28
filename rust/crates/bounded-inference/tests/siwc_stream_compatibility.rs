use bounded_inference::{Bounds, ObservedTerminal, StreamError, StreamObservation, StreamObserver};
use serde_json::{json, Value};

fn bounds() -> Bounds {
    Bounds {
        request_bytes: 4096,
        stream_bytes: 1024 * 1024,
        frame_bytes: 128 * 1024,
        output_bytes: 16 * 1024,
        events: 4096,
        local_deadline_ms: 30_000,
    }
}

fn fixture() -> Vec<Value> {
    include_str!("fixtures/synthetic-siwc-sparse-response.sse")
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn observe(events: &[Value], mode: u8, limits: Bounds) -> StreamObservation {
    let mut observer = match mode {
        0 => StreamObserver::new(limits),
        1 => StreamObserver::selected(limits),
        _ => StreamObserver::siwc(limits),
    }
    .unwrap();
    for event in events {
        if observer
            .push(format!("data: {event}\n\n").as_bytes())
            .is_err()
        {
            break;
        }
    }
    observer.finish_evidence()
}

fn rejected(events: &[Value]) {
    let result = observe(events, 2, bounds());
    assert!(result.local_failure.is_some());
    assert!(result.output.is_none());
}

#[test]
fn siwc_metadata_and_sparse_completion_require_explicit_profile() {
    let events = fixture();
    let result = observe(&events, 2, bounds());
    assert_eq!(result.local_failure, None);
    assert_eq!(result.terminal, Some(ObservedTerminal::Completed));
    assert_eq!(result.output.as_deref(), Some("B2_TRANSPORT_OK."));
    assert_eq!(result.unique_events, 10);
    assert_eq!(result.usage.len(), 1);
    for mode in [0, 1] {
        assert_eq!(
            observe(&events, mode, bounds()).local_failure,
            Some(StreamError::MalformedEvent)
        );
        let without_metadata: Vec<_> = events
            .iter()
            .filter(|e| e["type"] != "response.metadata")
            .cloned()
            .collect();
        assert_eq!(
            observe(&without_metadata, mode, bounds()).local_failure,
            Some(StreamError::ConflictingIdentity)
        );
    }
}

#[test]
fn metadata_is_response_bound_counted_and_never_terminal_or_output() {
    let original = fixture();
    for mutation in 0..5 {
        let mut events = original.clone();
        match mutation {
            0 => events[5]["response_id"] = json!("different-response"),
            1 => events[5]["metadata"] = json!("not-an-object"),
            2 => events[5]["output"] = json!("injected"),
            3 => events[5]["type"] = json!("response.unknown"),
            _ => {
                events[5].as_object_mut().unwrap().remove("response_id");
            }
        }
        rejected(&events);
    }
    rejected(&[original[5].clone()]);
    let mut duplicate = original.clone();
    duplicate.insert(6, original[5].clone());
    assert_eq!(observe(&duplicate, 2, bounds()).unique_events, 10);
    duplicate[6]["metadata"] = json!({"different":true});
    rejected(&duplicate);
    let mut late = original.clone();
    late.push(json!({"type":"response.metadata","sequence_number":10,
        "response_id":"resp-synthetic-siwc","metadata":{}}));
    rejected(&late);
    let only_metadata = vec![original[0].clone(), original[5].clone()];
    assert_eq!(
        observe(&only_metadata, 2, bounds()).local_failure,
        Some(StreamError::MissingTerminal)
    );
}

#[test]
fn sparse_terminal_requires_complete_coherent_item_snapshots() {
    let original = fixture();
    for mutation in 0..13 {
        let mut events = original.clone();
        match mutation {
            0 => {
                events.remove(8);
            }
            1 => events[8]["item"]["status"] = json!("in_progress"),
            2 => {
                events[8]["item"].as_object_mut().unwrap().remove("status");
            }
            3 => events[8]["item"]["role"] = json!("user"),
            4 => {
                events[8]["item"].as_object_mut().unwrap().remove("role");
            }
            5 => events[8]["item"]["content"][0]["text"] = json!("conflicting text"),
            6 => {
                events.remove(2);
            }
            7 => events[8]["output_index"] = json!(1),
            8 => {
                let mut extra = events[2].clone();
                extra["sequence_number"] = json!(20);
                extra["output_index"] = json!(1);
                extra["item"]["id"] = json!("unfinished-item");
                events.insert(9, extra);
            }
            9 => events[8]["item"]["content"] = json!([]),
            10 => {
                events.remove(6);
            }
            11 => {
                events.remove(7);
            }
            _ => {
                events.remove(3);
            }
        }
        rejected(&events);
    }
}

#[test]
fn sparse_mode_does_not_override_present_terminal_or_missing_terminal() {
    let original = fixture();
    for output in [
        Value::Null,
        json!([{"id":"item-synthetic-siwc","type":"message",
        "role":"assistant","status":"completed","content":[{"type":"output_text","text":"wrong"}]}]),
    ] {
        let mut events = original.clone();
        events[9]["response"]["output"] = output;
        rejected(&events);
    }
    let mut missing = original.clone();
    missing[9]["response"]
        .as_object_mut()
        .unwrap()
        .remove("output");
    rejected(&missing);
    rejected(&original[..9]);
    for (kind, status) in [
        ("response.failed", "failed"),
        ("response.incomplete", "incomplete"),
    ] {
        let mut events = original.clone();
        events[9]["type"] = json!(kind);
        events[9]["response"]["status"] = json!(status);
        let result = observe(&events, 2, bounds());
        assert_eq!(result.local_failure, None);
        assert!(result.output.is_none());
    }
    let mut observer = StreamObserver::siwc(bounds()).unwrap();
    for event in &original {
        observer
            .push(format!("data: {event}\n\n").as_bytes())
            .unwrap();
    }
    observer.push(b"data: ").unwrap();
    assert_eq!(
        observer.finish_evidence().local_failure,
        Some(StreamError::MalformedEvent)
    );
}

#[test]
fn sparse_snapshots_preserve_limits_and_require_dense_added_items() {
    let events = vec![
        json!({"type":"response.created","sequence_number":0,"response":{"id":"r","model":"fake-model"}}),
        json!({"type":"response.output_item.added","sequence_number":1,"output_index":0,"item":{"id":"i0","type":"message","role":"assistant","content":[]}}),
        json!({"type":"response.output_item.done","sequence_number":2,"output_index":0,"item":{"id":"i0","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"abcd"}]}}),
        json!({"type":"response.output_item.added","sequence_number":3,"output_index":1,"item":{"id":"i1","type":"message","role":"assistant","content":[]}}),
        json!({"type":"response.output_item.done","sequence_number":4,"output_index":1,"item":{"id":"i1","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"efgh"}]}}),
        json!({"type":"response.completed","sequence_number":5,"response":{"id":"r","model":"fake-model","status":"completed","output":[]}}),
    ];
    let mut witnessed = Vec::new();
    for event in events {
        if event["type"] == "response.output_item.done" {
            let index = event["output_index"].clone();
            let id = event["item"]["id"].clone();
            let text = event["item"]["content"][0]["text"].clone();
            witnessed.push(json!({"type":"response.content_part.added","output_index":index,"content_index":0,"item_id":id,"part":{"type":"output_text","text":""}}));
            witnessed.push(json!({"type":"response.output_text.done","output_index":index,"content_index":0,"item_id":id,"text":text}));
            witnessed.push(json!({"type":"response.content_part.done","output_index":index,"content_index":0,"item_id":id,"part":{"type":"output_text","text":text}}));
        }
        witnessed.push(event);
    }
    for (index, event) in witnessed.iter_mut().enumerate() {
        event["sequence_number"] = json!(index);
    }
    let events = witnessed;
    assert_eq!(
        observe(&events, 2, bounds()).output.as_deref(),
        Some("abcdefgh")
    );
    let mut limit = bounds();
    limit.output_bytes = 7;
    assert_eq!(
        observe(&events, 2, limit).local_failure,
        Some(StreamError::OutputTooLarge)
    );
    let mut gap = events.clone();
    for event in &mut gap {
        if event.get("output_index") == Some(&json!(1)) {
            event["output_index"] = json!(2);
        }
    }
    rejected(&gap);
    let mut no_added = events.clone();
    no_added.remove(1);
    rejected(&no_added);
    let mut metadata = fixture();
    metadata[5]["metadata"] = json!({"padding":"x".repeat(2048)});
    let mut limit = bounds();
    limit.frame_bytes = 1024;
    assert_eq!(
        observe(&metadata, 2, limit).local_failure,
        Some(StreamError::FrameTooLarge)
    );
    let mut limit = bounds();
    limit.events = 5;
    assert_eq!(
        observe(&fixture(), 2, limit).local_failure,
        Some(StreamError::TooManyEvents)
    );
}

#[test]
fn sparse_reasoning_items_require_completion_and_never_contribute_text() {
    for before in [true, false] {
        let mut events = fixture();
        let reasoning_index = if before { 0 } else { 1 };
        if before {
            for event in &mut events {
                if event.get("output_index").is_some() {
                    event["output_index"] = json!(1);
                }
            }
        }
        let added = json!({"type":"response.output_item.added","output_index":reasoning_index,
            "item":{"id":"reasoning-synthetic","type":"reasoning","status":"in_progress","summary":[]}});
        let done = json!({"type":"response.output_item.done","output_index":reasoning_index,
            "item":{"id":"reasoning-synthetic","type":"reasoning","status":"completed","summary":[]}});
        let at = if before { 1 } else { events.len() - 1 };
        events.splice(at..at, [added, done]);
        for (index, event) in events.iter_mut().enumerate() {
            event["sequence_number"] = json!(index);
        }
        let result = observe(&events, 2, bounds());
        assert_eq!(result.local_failure, None);
        assert_eq!(result.output.as_deref(), Some("B2_TRANSPORT_OK."));
        let done = events
            .iter_mut()
            .find(|event| {
                event["type"] == "response.output_item.done" && event["item"]["type"] == "reasoning"
            })
            .unwrap();
        done["item"].as_object_mut().unwrap().remove("status");
        assert_eq!(
            observe(&events, 2, bounds()).local_failure,
            Some(StreamError::ConflictingTerminal)
        );
    }
}
