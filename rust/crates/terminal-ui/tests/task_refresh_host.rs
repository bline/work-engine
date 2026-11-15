#![cfg(target_os = "linux")]

use lifecycle_proof::{
    host::{HostClient, HostReportedIdentity},
    identity::HostProcess,
    provider::{Reply, ScriptedProvider, Step},
};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[path = "support/terminal.rs"]
mod terminal;
const TASK: &str = "task:ui:sim";
const QUESTION: &str = "question:ui:sim";
const AUTH: &str = "operator:s8:authorized";
fn save(root: &Path, name: &str, v: &Value) {
    fs::write(root.join(name), serde_json::to_vec_pretty(v).unwrap()).unwrap();
}
fn wait(root: &Path, c: &HostClient, d: &str, p: impl Fn(&Value) -> bool) -> Value {
    let until = Instant::now() + Duration::from_secs(45);
    loop {
        let v = c.observe(TASK, Some(d)).unwrap();
        if p(&v) {
            return v;
        }
        if Instant::now() > until {
            save(root, "timeout-observation.json", &v);
            panic!("observation deadline");
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn shared_cli(root: &Path, stage: &str, delivery: &str) {
    let Some(binary) = std::env::var_os("U2_OBSERVE_BINARY") else {
        return;
    };
    let output = std::process::Command::new(binary)
        .arg("selected-task")
        .arg(root.join("host.sock"))
        .arg(TASK)
        .args(["--delivery", delivery, "--profile"])
        .arg(root.join("events-profile.json"))
        .arg("--output")
        .arg(root.join(format!("shared-{stage}.json")))
        .arg("--human")
        .output()
        .unwrap();
    fs::write(root.join(format!("cli-{stage}.stdout")), &output.stdout).unwrap();
    fs::write(root.join(format!("cli-{stage}.stderr")), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "shared CLI stage failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let shared = operator_observation::selected_task::SelectedTask::parse(
        &fs::read(root.join(format!("shared-{stage}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(shared.request.task_id, TASK);
    assert_eq!(shared.standing, "new_endpoint_read");
    assert_eq!(shared.components["owner"].state, "present");
    if stage == "completed" {
        assert_eq!(shared.activity.len(), 1);
        assert!(
            matches!(&shared.activity[0].observation,operator_observation::Observation::Available{entries,..} if !entries.is_empty())
        );
    }
}

struct HostGuard(Option<HostProcess>);
impl std::ops::Deref for HostGuard {
    type Target = HostProcess;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}
impl std::ops::DerefMut for HostGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap()
    }
}
impl Drop for HostGuard {
    fn drop(&mut self) {
        if let Some(host) = self.0.take()
            && let Err(error) = host.terminate_and_reap(Duration::from_secs(15))
        {
            eprintln!("Host cleanup failed: {error}");
        }
    }
}

/// Opt-in Linux proof using a pinned installed host and auth-free local simulator.
#[test]
#[ignore = "requires UI_TEST_HOST, UI_TEST_STARTUP_TEMPLATE, UI_TEST_EVIDENCE_DIR, TMPDIR and CARGO_TARGET_DIR"]
fn installed_host_refreshes_one_running_ui() {
    let output = std::path::PathBuf::from(
        std::env::var_os("UI_TEST_EVIDENCE_DIR").expect("UI_TEST_EVIDENCE_DIR"),
    );
    assert!(output.is_absolute() && output.is_dir());
    for key in ["TMPDIR", "CARGO_TARGET_DIR"] {
        assert!(Path::new(&std::env::var_os(key).expect(key)).is_absolute());
    }
    let retained = tempfile::Builder::new()
        .prefix("ui-refresh-")
        .tempdir_in(output)
        .unwrap()
        .keep();
    let root = retained.as_path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    eprintln!("UI refresh evidence retained at {}", root.display());
    let executable =
        std::path::PathBuf::from(std::env::var_os("UI_TEST_HOST").expect("UI_TEST_HOST"));
    let template = std::path::PathBuf::from(
        std::env::var_os("UI_TEST_STARTUP_TEMPLATE").expect("UI_TEST_STARTUP_TEMPLATE"),
    );
    assert!(executable.is_absolute() && template.is_absolute());
    let args = root.join("result-arguments.json");
    let question_step = Step {
        required: vec!["input:command:ui:sim".into()],
        forbidden: vec![],
        reply: Reply::ToolCall {
            call_id: "call:ui:question".into(),
            namespace: "task".into(),
            name: "question_open".into(),
            arguments: json!({
                "question_id": QUESTION, "recipient_ref": AUTH,
                "scope_ref": "scope:ui:sim",
                "text": "May I publish this controlled UI validation result?"
            }),
        },
        release_path: None,
    };
    let result_step = Step {
        required: vec![QUESTION.into()],
        forbidden: vec![],
        reply: Reply::ToolCallFromFile {
            call_id: "call:ui:result".into(),
            namespace: "task".into(),
            name: "result_publish".into(),
            arguments_path: args.clone(),
        },
        // The host answer and its visible UI projection must precede this response.
        release_path: Some(args.clone()),
    };
    let final_step = Step {
        required: vec!["result:ui:sim".into()],
        forbidden: vec![],
        reply: Reply::Text("Controlled UI validation result published.".into()),
        release_path: None,
    };
    let provider = ScriptedProvider::start(
        vec![question_step, result_step, final_step],
        root.join("provider-ledger.jsonl"),
    )
    .unwrap();
    let mut config: Value = serde_json::from_slice(&fs::read(&template).unwrap()).unwrap();
    assert_eq!(config["provider_identity"], "native-sim-proof-v1");
    assert_eq!(config["profile"], "retained-repository-analysis-v1");
    assert_eq!(config["features"], json!(["controlled-proof"]));
    assert_eq!(
        config["answer_authority_ref"], AUTH,
        "requires the S8 controlled fixture template"
    );
    assert_eq!(config["lifecycle"]["subject_id"], "subject:s8:task");
    assert!(config["lifecycle"]["verifier_executable"].is_null());
    assert!(config["lifecycle"]["verifier_ledger_path"].is_null());
    for key in ["task_store_path", "socket_path"] {
        let name = Path::new(config[key].as_str().unwrap())
            .file_name()
            .unwrap()
            .to_owned();
        config[key] = json!(root.join(name));
    }
    for key in [
        "socket_path",
        "store_root",
        "peer_socket_path",
        "proof_barrier_dir",
    ] {
        let name = Path::new(config["lifecycle"][key].as_str().unwrap())
            .file_name()
            .unwrap()
            .to_owned();
        config["lifecycle"][key] = json!(root.join(name));
    }
    for dir in ["native", "barrier"] {
        fs::create_dir(root.join(dir)).unwrap();
        fs::set_permissions(root.join(dir), fs::Permissions::from_mode(0o700)).unwrap();
    }
    for barrier in [
        "native_before_turn_entry",
        "native_after_turn_send_intent",
        "native_after_result_commit",
        "semantic_before_turn_entry",
        "semantic_before_commit",
    ] {
        fs::write(
            root.join("barrier").join(format!("{barrier}.release")),
            b"release\n",
        )
        .unwrap();
    }
    // U2-ENV-002: the same qualified native package, privately retrieved and pinned.
    // Never replace the lifecycle owner's companion guard or global installation.
    if let Some(selected) = std::env::var_os("U2_TEST_NATIVE") {
        let selected = std::path::PathBuf::from(selected);
        assert!(selected.is_absolute());
        assert_eq!(
            lifecycle_proof::identity::sha256_file(&selected).unwrap(),
            std::env::var("U2_TEST_NATIVE_SHA256").unwrap()
        );
        assert_eq!(
            lifecycle_proof::identity::sha256_file(
                &selected.with_file_name("codex-code-mode-host")
            )
            .unwrap(),
            "8724d73ec190536233ab4628d24a071bfbcf27e5124f08fe35129f26fe48b916"
        );
        config["lifecycle"]["native"]["executable"] = json!(selected);
    }
    config["lifecycle"]["native"]["root"] = json!(root.join("native"));
    config["lifecycle"]["native"]["simulator_base_url"] = json!(provider.base_url());
    config["lifecycle"]["proof_run_id"] = json!("terminal-ui-installed-refresh");
    let expiry = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        + 600_000;
    for g in config["lifecycle"]["grants"].as_array_mut().unwrap() {
        g["expires_wall_ms"] = json!(expiry);
    }
    save(root, "host-startup.json", &config);
    fs::set_permissions(
        root.join("host-startup.json"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    if std::env::var_os("U2_OBSERVE_BINARY").is_some() {
        for name in ["events-state", "events-claude"] {
            fs::create_dir(root.join(name)).unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o700)).unwrap();
        }
        let owner = std::env::var("U2_EVENTS_OWNER").expect("U2_EVENTS_OWNER");
        assert_eq!(
            lifecycle_proof::identity::sha256_file(Path::new(&owner)).unwrap(),
            "586128d82ebd1fa0c8813e5583924cd9ce812e6ffe76de0c2e63d396414e2a8b"
        );
        save(
            root,
            "events-profile.json",
            &json!({"version":1,"owner":owner,"owner_sha256":"586128d82ebd1fa0c8813e5583924cd9ce812e6ffe76de0c2e63d396414e2a8b","codex_home":root.join("native/codex"),"claude_home":root.join("events-claude"),"state_dir":root.join("events-state"),"bridge":null,"context":null}),
        );
        fs::set_permissions(
            root.join("events-profile.json"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }
    let mut host = HostGuard(Some(
        HostProcess::launch(&executable, &root.join("host-startup.json")).unwrap(),
    ));
    let client = HostClient::new(root.join("host.sock"), Duration::from_secs(5)).unwrap();
    client.await_ready(Duration::from_secs(10)).unwrap();
    let submit = json!({"command_id":"command:ui:sim",
            "task_id":TASK,
            "subject_id":"subject:s8:task",
            "kind":"repository_analysis",
            "objective":"Verify a controlled question, authorized answer and published result in the terminal UI.",
            "engineering_question":"Open the controlled UI validation question and publish the attributed result after the operator answers.",
            "snapshot_ref":"snapshot:s8:approved",
            "facts":[]});
    save(root, "submit-request.json", &submit);
    let admitted = client.submit(submit).unwrap();
    save(root, "admitted.json", &admitted);
    assert_eq!(admitted["status"], "admitted");
    let identity = HostReportedIdentity::from_reply(&admitted).unwrap();
    identity
        .bind_process(host.identity(), "build:s8:external-proof")
        .unwrap();
    identity.bind_startup(&config).unwrap();
    let d = admitted["lifecycle"]["outcome_ref"].as_str().unwrap();
    let pending = wait(root, &client, d, |v| {
        v["components"]["owner"]["value"]["questions"][0]["state"] == "pending"
    });
    save(root, "pending.json", &pending);
    shared_cli(root, "pending", d);
    let mut ui = terminal::Terminal::launch(&root.join("host.sock"), TASK, d, root);
    let ui_pid = ui.pid();
    ui.wait_for("TASK OBSERVATION", "initial");
    ui.input(b"\x10conversation\r");
    ui.wait_for(
        "May I publish this controlled UI validation result?",
        "pending",
    );
    assert!(!ui.screen().contains("Yes, publish"));
    assert!(!ui.screen().contains("Controlled UI validation complete"));
    let answer = json!({"command_id":"command:ui:sim:answer",
            "task_id":TASK,
            "question_id":QUESTION,
            "recipient_ref":AUTH,
            "scope_ref":"scope:ui:sim",
            "answer":"Yes, publish the controlled UI validation result; no repository mutation is authorized.",
            "expected_revision":pending["components"]["owner"]["value"]["revision"],
            "authority_ref":AUTH});
    save(root, "answer-request.json", &answer);
    let answered = client.answer(answer).unwrap();
    save(root, "answer-response.json", &answered);
    assert_eq!(answered["status"], "answered");
    let after = client.observe(TASK, Some(d)).unwrap();
    save(root, "answered.json", &after);
    shared_cli(root, "answered", d);
    let answer_wait = ui.wait_for(
        "Yes, publish the controlled UI validation result",
        "answered",
    );
    assert!(!ui.screen().contains("Controlled UI validation complete"));
    let sources = after["components"]["owner"]["value"]["sources"]
        .as_array()
        .unwrap();
    let refs: Vec<Value> = sources
        .iter()
        .filter(|s| s["current"] == true)
        .map(|s| {
            json!({"id":s["id"],
            "owner_namespace":s["owner_namespace"],
            "owner_ref":s["owner_ref"],
            "revision":s["revision"],"sha256":s["sha256"]})
        })
        .collect();
    save(
        root,
        "arguments-staging.json",
        &json!({"result_id":"result:ui:sim",
            "body":"Controlled UI validation complete: question answered and result published without repository mutation.",
            "source_refs":refs,
            "attribution":"controlled-local-simulator"}),
    );
    fs::rename(root.join("arguments-staging.json"), args).unwrap();
    let complete = wait(root, &client, d, |v| {
        !v["components"]["owner"]["value"]["result"].is_null()
            && v["components"]["lifecycle"]["value"]["delivery"]["value"]["outcome"] == "completed"
    });
    save(root, "completed.json", &complete);
    shared_cli(root, "completed", d);
    assert_eq!(
        complete["components"]["owner"]["value"]["result"]["result_id"],
        "result:ui:sim"
    );
    let result_wait = ui.wait_for("Controlled UI validation complete", "completed");
    assert_eq!(ui.pid(), ui_pid);
    assert_eq!(
        complete["components"]["native_delivery_status"]["value"]["segments"][0]["segment_state"],
        "completed"
    );
    assert_eq!(
        complete["components"]["owner"]["value"]["questions"][0]["state"]["answered"]["authority_ref"],
        AUTH
    );
    ui.close();
    host.assert_same_execution().unwrap();
    let close = host
        .0
        .take()
        .unwrap()
        .terminate_and_reap(Duration::from_secs(15))
        .unwrap();
    assert_eq!(close.exit_code, Some(0));
    assert!(close.reaped);
    let entries = provider.finish().unwrap();
    assert_eq!(entries.len(), 3);
    save(
        root,
        "result.json",
        &json!({"status":"passed",
            "ui_pid":ui_pid,"ui_binary_sha256":lifecycle_proof::identity::sha256_file(Path::new(env!("CARGO_BIN_EXE_work-engine-terminal-ui"))).unwrap(),
            "answer_screen_wait_ms":answer_wait.as_millis(),
            "result_screen_wait_ms":result_wait.as_millis(),
            "ui_launches":1,
            "manual_refreshes":0,
            "terminal_restored":true,
            "provider_entries":entries.len(),
            "host_close":close,
            "host_identity":identity,
            "stages":["pending","answered","completed"],
            "scope":"Controlled loopback simulator and installed host with actual terminal UI PTY captures; no paid provider or production workload qualification."}),
    );
    println!("PASS pending -> answered -> published result, host reaped, simulator finished");
}
