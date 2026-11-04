//! Fake receiver protocol only. Never imports or executes Pi or Docker.
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, BufRead, Write},
    path::Path,
    time::{Duration, Instant},
};
fn hash(b: &[u8]) -> String {
    work_engine_manual_host::digest(b)
}
fn emit(v: Value) {
    println!("{v}");
    io::stdout().flush().unwrap();
}
fn main() {
    // Compose the accepted fake daemon for ordinary custody commands.
    let exe = std::env::current_exe().unwrap();
    let mode = fs::read_to_string(exe.with_extension("mode")).unwrap_or_default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("start") {
        let status = std::process::Command::new(exe.with_extension("base"))
            .args(&args)
            .status()
            .unwrap();
        std::process::exit(status.code().unwrap_or(1));
    }
    writeln!(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(exe.with_extension("calls"))
            .unwrap(),
        "start"
    )
    .unwrap();
    let state_path = exe.with_extension("base").with_extension("state");
    let mut daemon: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    daemon["State"]["Running"] = json!(true);
    daemon["State"]["Status"] = json!("running");
    fs::write(&state_path, serde_json::to_vec(&daemon).unwrap()).unwrap();
    let export = Path::new(
        daemon["Mounts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["Destination"] == "/export")
            .unwrap()["Source"]
            .as_str()
            .unwrap(),
    );
    let custody = export.parent().unwrap();
    let schema = "work-engine.pi-observation-sdk.v1";
    let native = "fixture-native-session";
    let state = json!({"sessionId":native,"isStreaming":mode=="busy","isCompacting":false,"pendingMessageCount":0,"autoCompactionEnabled":false,"autoRetryEnabled":false});
    emit(json!({"schema":schema,"type":"ready","state":state}));
    if mode == "busy" || mode == "no-release" {
        std::thread::sleep(Duration::from_secs(5));
        return;
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while !custody.join("receiver-ready.json").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    let barrier: Value =
        serde_json::from_slice(&fs::read(custody.join("receiver-ready.json")).unwrap()).unwrap();
    assert_eq!(barrier["native_session_id"], native);
    let journal: Value =
        serde_json::from_slice(&fs::read(custody.join("pi-state.json")).unwrap()).unwrap();
    let p = &journal["job"]["receiver"];
    let event: Value =
        serde_json::from_slice(&fs::read(p["event"]["path"].as_str().unwrap()).unwrap()).unwrap();
    let mut binding = p["recipient_without_session"].clone();
    binding["session"] = json!(native);
    let b = &binding;
    let wire = format!(
        "{{\"recipient\":{},\"principal\":{},\"job\":{},\"lane\":{},\"session\":{},\"workspace\":{},\"generation\":{},\"address\":{},\"fixture_mode\":{},\"expires_at\":{},\"active\":{}}}",
        b["recipient"],
        b["principal"],
        b["job"],
        b["lane"],
        b["session"],
        b["workspace"],
        b["generation"],
        b["address"],
        b["fixture_mode"],
        b["expires_at"],
        b["active"]
    );
    let binding_sha = hash(wire.as_bytes());
    let delivery_id = hash(
        &serde_json::to_vec(&(
            event["event_id"].as_str().unwrap(),
            binding["recipient"].as_str().unwrap(),
            binding["generation"].as_u64().unwrap(),
            &binding_sha,
        ))
        .unwrap(),
    );
    let attempt_id =
        hash(&serde_json::to_vec(&(&delivery_id, p["attempt_number"].as_u64().unwrap())).unwrap());
    let mut envelope = json!({"schema":"work-engine.delivery-envelope/v1","fixture_only":true,"authority":"observation_only","delivery_id":delivery_id,"attempt_id":attempt_id,"subscription_id":b["recipient"],"binding_sha256":hash(wire.as_bytes()),"recipient":binding,"event":event});
    if mode == "wrong-attempt" {
        envelope["attempt_id"] = json!("wrong");
    }
    let raw = serde_json::to_string(&envelope).unwrap();
    let sha = hash(raw.as_bytes());
    let input = custody.join("envelope.json");
    fs::write(&input, &raw).unwrap();
    let release = json!({"schema":schema,"input_digest":barrier["input_digest"],"container_id":barrier["container_id"],"native_session_id":native,"envelope":{"path":input,"sha256":sha}});
    let tmp = custody.join("release.tmp");
    fs::write(&tmp, serde_json::to_vec(&release).unwrap()).unwrap();
    fs::rename(tmp, custody.join("receiver-release.json")).unwrap();
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line).unwrap();
    if mode == "wrong-attempt" {
        return;
    }
    let command: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(command["rawEnvelope"], raw);
    assert_eq!(command["nativeSessionId"], native);
    let native = if mode == "wrong-session" {
        "foreign"
    } else {
        native
    };
    emit(json!({"schema":schema,"type":"admitted","sessionId":native,"envelopeSha256":sha}));
    let message = json!({"role":"custom","customType":"work-engine.delivery-observation.v1","content":[{"type":"text","text":raw}],"display":false,"details":{"rawEnvelope":raw,"envelopeSha256":sha},"timestamp":1});
    if mode != "missing-end" {
        emit(
            json!({"schema":schema,"type":"native_event","sessionId":native,"event":{"type":"message_end","message":message}}),
        );
    }
    if mode != "missing-messages" {
        emit(
            json!({"schema":schema,"type":"get_messages","sessionId":native,"messages":[message],"state":state}),
        );
    }
    let history = json!({"schema":schema,"sessionId":native,"header":{"id":native},"entries":[{"type":"custom_message","customType":"work-engine.delivery-observation.v1","details":message["details"],"content":message["content"],"display":false}],"messages":[message],"nativeSessionFile":null});
    let bytes = serde_json::to_vec(&history).unwrap();
    if mode != "missing-history" {
        fs::write(export.join("receiver-native-history.json"), &bytes).unwrap();
    }
    emit(
        json!({"schema":schema,"type":"history_export","sessionId":native,"path":"receiver-native-history.json","sha256":hash(&bytes)}),
    );
    if mode == "partial-tail" {
        print!("{{\"partial\":");
        io::stdout().flush().unwrap();
    }
    daemon["State"]["Running"] = json!(false);
    daemon["State"]["Status"] = json!("exited");
    fs::write(state_path, serde_json::to_vec(&daemon).unwrap()).unwrap();
}
