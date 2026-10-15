use crate::{
    application,
    model::{Model, View},
    task_live::{Reply, Worker},
    ui,
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    time::{Duration, Instant},
};
use tuirealm::{
    event::{Event, Key, KeyEvent, KeyModifiers},
    listener::EventListenerCfg,
};
use work_engine_terminal_ui::task_observation::{self as wire, Request};

fn request() -> Request {
    Request {
        task_id: "task:test".into(),
        delivery_id: None,
        command_id: None,
    }
}
fn raw() -> Value {
    json!({"protocol_version":2,"status":"observation","consistency":"independent_components","request":request(),"host_identity":{"pid":1,"linux_starttime_ticks":"1","incarnation":"host:1","build_id":"test","profile":"retained-repository-analysis-v1","executable_sha256":"a".repeat(64),"source_sha256":"b".repeat(64),"lock_sha256":"c".repeat(64),"toolchain_sha256":"d".repeat(64),"features":[],"provider_identity":"synthetic"},"components":{
        "owner":{"state":"present","scope":{"task_id":"task:test","subject_id":"subject:test"},"revision":{"kind":"task","owner_revision":7,"cursor_revision":3},"value":{"task_id":"task:test","subject_id":"subject:test","revision":7,"cursor_revision":3,"objective":"Inspect real task reads","inputs":[{"input_id":"input:1","engineering_question":"**Original input**"}],"questions":[{"id":"q1","recipient_ref":"operator","scope_ref":"task:test","text":"May I publish?","state":"pending"}],"completed_effects":[],"result":{"result_id":"r1","body":"## Published result\n\nEvidence retained.","attribution":"task owner","source_refs":[]}}},
        "lifecycle":{"state":"unavailable","scope":{"subject_id":"subject:test","delivery_id":null},"reason":"lifecycle_read_unavailable"},
        "native_delivery_status":{"state":"present","scope":{"subject_id":"subject:test","collection":"native_delivery_segments","filter":"subject"},"revision":{"kind":"not_provided"},"value":{"protocol_version":1,"subject_id":"subject:test","recovery_only":false,"segments":[]},"completeness":{"kind":"complete_for_scope","limit":64}},
        "command_result":{"state":"not_requested","scope":{"subject_id":"subject:test","command_id":null,"task_binding":"not_established"}},
        "control_result":{"state":"not_requested","scope":{"subject_id":"subject:test","command_id":null,"task_binding":"not_established"}}
    }})
}
fn deliver(model: &mut Model, raw: Value) {
    let job = model.task_tick().expect("queued read");
    let result = wire::decode(&serde_json::to_vec(&raw).unwrap(), &job.request);
    model.apply_task_reply(Reply {
        generation: job.generation,
        result,
    });
}
#[test]
fn task_socket_worker_projects_real_decoded_reads_into_all_views() {
    let endpoint = std::env::temp_dir().join(format!(
        "ui-task-{}-{}.sock",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let listener = UnixListener::bind(&endpoint).unwrap();
    let host = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut line = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        let request: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["op"], "task.observe.v2");
        writeln!(stream, "{}", raw()).unwrap();
    });
    let worker = Worker::start();
    let (mut app, shared) = application(EventListenerCfg::default());
    shared.borrow_mut().draft().insert_str("preserved draft");
    shared
        .borrow_mut()
        .connect_task(endpoint.clone(), request());
    let job = shared.borrow_mut().task_tick().unwrap();
    worker.request(job).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(reply) = worker.try_reply() {
            shared.borrow_mut().apply_task_reply(reply);
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    host.join().unwrap();
    for view in [
        View::Overview,
        View::Dashboard,
        View::Conversation,
        View::Lifecycle,
    ] {
        shared.borrow_mut().view = view;
        let frame = ui::snapshot(&mut app, &shared, 128, 40);
        assert!(frame.contains("TASK OBSERVATION"), "{frame}");
        assert!(!frame.contains("PROVISIONAL"));
        assert!(shared.borrow().input_read_only());
    }
    let mut model = shared.borrow_mut();
    assert_eq!(
        model.task.as_ref().unwrap().objective,
        "Inspect real task reads"
    );
    assert!(
        model
            .task
            .as_ref()
            .unwrap()
            .conversation()
            .banner
            .contains("NOT a chronological")
    );
    assert!(
        model
            .task
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .filter(|n| n.component >= 3)
            .all(|n| n.parent.is_none())
    );
    let entries = model.entries.len();
    model.handle(Event::Keyboard(KeyEvent::new(
        Key::Enter,
        KeyModifiers::NONE,
    )));
    assert_eq!(model.entries.len(), entries);
    assert_eq!(model.draft_text(), "preserved draft");
    assert!(model.task_tick().is_none(), "refresh is rate bounded");
    std::fs::remove_file(endpoint).unwrap();
}
#[test]
fn task_disconnect_generation_host_change_and_component_refusal_preserve_truth() {
    let mut model = Model::new();
    model.connect_task("/tmp/synthetic-ui.sock".into(), request());
    deliver(&mut model, raw());
    let original = model.task.as_ref().unwrap().conversation().entries;
    model.request_connection_read(crate::connection::ReadKind::Refresh);
    let old = model.task_tick().unwrap();
    model.disconnect_connection();
    model.apply_task_reply(Reply {
        generation: old.generation,
        result: wire::decode(&serde_json::to_vec(&raw()).unwrap(), &request()),
    });
    assert!(model.task_connection.as_ref().unwrap().disconnected);
    assert!(model.task_tick().is_none());
    assert!(
        model
            .task
            .as_ref()
            .unwrap()
            .components
            .iter()
            .all(|c| c.stale)
    );
    assert!(
        model.task.as_ref().unwrap().conversation().entries[0]
            .text
            .contains("STALE")
    );
    model.request_connection_read(crate::connection::ReadKind::Reconnect);
    let mut refused = raw();
    refused["components"]["owner"] =
        json!({"state":"unavailable","scope":{"task_id":"task:test"},"reason":"owner_unavailable"});
    for key in ["lifecycle", "native_delivery_status"] {
        let mut scope = refused["components"][key]["scope"].clone();
        scope["subject_id"] = Value::Null;
        refused["components"][key] =
            json!({"state":"unavailable","scope":scope,"reason":"task_dependency_unavailable"});
    }
    deliver(&mut model, refused.clone());
    assert_eq!(
        model.task.as_ref().unwrap().conversation().entries.len(),
        original.len()
    );
    model.request_connection_read(crate::connection::ReadKind::Refresh);
    refused["host_identity"]["incarnation"] = json!("host:2");
    deliver(&mut model, refused);
    assert!(
        !model
            .task
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|n| n.id.starts_with("result:")),
        "new host cannot inherit old owner content"
    );
    model.request_connection_read(crate::connection::ReadKind::Refresh);
    let mut capacity = raw();
    let scope = capacity["components"]["native_delivery_status"]["scope"].clone();
    capacity["components"]["native_delivery_status"] =
        json!({"state":"refused","scope":scope,"reason":"capacity_exceeded","limit":64});
    deliver(&mut model, capacity);
    assert_eq!(
        model.task.as_ref().unwrap().components[2].availability,
        crate::task::Availability::Refused
    );
    assert!(
        model
            .task
            .as_ref()
            .unwrap()
            .details()
            .contains("capacity_exceeded")
    );
}
#[test]
fn task_navigation_scope_and_identical_refresh_keep_identity_and_drafts() {
    let mut model = Model::new();
    model.connect_task("/tmp/synthetic-ui.sock".into(), request());
    deliver(&mut model, raw());
    model.select(2);
    let id = model.task.as_ref().unwrap().selected_node().id.clone();
    let source = model.task.as_ref().unwrap().conversation().source;
    model.view = View::Dashboard;
    model.handle(Event::Keyboard(KeyEvent::new(
        Key::Function(2),
        KeyModifiers::NONE,
    )));
    assert_eq!(model.view, View::Conversation);
    assert!(!model.task_connection.as_ref().unwrap().disconnected);
    model.request_connection_read(crate::connection::ReadKind::Refresh);
    deliver(&mut model, raw());
    assert_eq!(model.task.as_ref().unwrap().selected_node().id, id);
    assert_eq!(model.task.as_ref().unwrap().conversation().source, source);
    let conversation = model.task.as_ref().unwrap().conversation();
    assert_eq!(conversation.entries.len(), 1);
    assert!(conversation.entries[0].text.contains("May I publish?"));
    model.open_task(crate::task::Case::Pending);
    assert!(model.task_connection.as_ref().unwrap().disconnected);
    assert!(model.task_tick().is_none());
}
#[test]
fn task_cli_rejects_ambiguous_targets_and_validates_identifiers() {
    let parse = |args: &str| {
        crate::parse_task_connection(
            &args
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        )
    };
    assert!(
        parse("--connect /tmp/service --task task:1 --command c --delivery d")
            .unwrap()
            .is_some()
    );
    for args in [
        "--connect relative --task t",
        "--connect /s --task bad/id",
        "--connect /s --task t --delivery d --delivery e",
        "--connect /s --task t --subject s",
        "--connect /s --task",
    ] {
        assert!(parse(args).is_err(), "{args}");
    }
}

#[test]
fn task_execution_result_and_answer_are_independent_and_failures_are_visible() {
    let mut model = Model::new();
    model.connect_task("/tmp/synthetic-ui.sock".into(), request());
    let mut observed = raw();
    let mut lifecycle: Value = serde_json::from_slice(include_bytes!(
        "../../lifecycle-wire/tests/fixtures/failed-unsettled-v1.json"
    ))
    .unwrap();
    lifecycle["subject_id"] = json!("subject:test");
    observed["components"]["lifecycle"] = json!({"state":"present","scope":{"subject_id":"subject:test","delivery_id":null},"revision":{"kind":"lifecycle","cursor":lifecycle["cursor"]},"value":lifecycle});
    observed["components"]["owner"]["value"]["questions"][0]["state"] = json!({"answered":{"answer":"Yes, keep settlement separate.","authority_ref":"operator:1"}});
    deliver(&mut model, observed);
    let task = model.task.as_ref().unwrap();
    assert!(task.execution.contains("outcome failed"));
    assert!(task.execution.contains("settlement unresolved"));
    assert!(!task.question_pending);
    assert!(
        task.conversation().entries[1]
            .text
            .contains("Yes, keep settlement separate.")
    );
    assert!(task.overview()[2].body.contains("Published result"));
    for error in [
        wire::ReadError::UnsupportedVersion,
        wire::ReadError::Deadline,
        wire::ReadError::Refused,
    ] {
        model.request_connection_read(crate::connection::ReadKind::Refresh);
        let job = model.task_tick().unwrap();
        model.apply_task_reply(Reply {
            generation: job.generation,
            result: Err(error.clone()),
        });
        let task = model.task.as_ref().unwrap();
        assert!(task.overview()[1].body.contains(&format!("{error:?}")));
        assert!(task.overview()[2].body.starts_with("LAST KNOWN"));
        assert!(task.overview()[3].body.starts_with("LAST KNOWN"));
    }
}

#[test]
fn task_target_change_clears_prior_anchors_and_late_results() {
    let mut model = Model::new();
    model.connect_task("/tmp/synthetic-ui.sock".into(), request());
    deliver(&mut model, raw());
    model.request_connection_read(crate::connection::ReadKind::Refresh);
    let old = model.task_tick().unwrap();
    let mut target = request();
    target.task_id = "task:other".into();
    model.connect_task("/tmp/other-ui.sock".into(), target);
    model.apply_task_reply(Reply {
        generation: old.generation,
        result: wire::decode(&serde_json::to_vec(&raw()).unwrap(), &request()),
    });
    let task = model.task.as_ref().unwrap();
    assert!(!task.nodes.iter().any(|n| n.id.starts_with("result:")));
    assert_eq!(task.live.as_ref().unwrap().request.task_id, "task:other");
    assert_eq!(task.live.as_ref().unwrap().host, "not observed");
}
