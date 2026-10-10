use anyhow::{Result, ensure};
use context_lifecycle_experiment::codex::{
    Endpoint, InterruptState, Launch, Observation, ObservationState,
};
use context_lifecycle_experiment::provider::*;
use context_lifecycle_experiment::vertical::{TextVerticalConfig, run_text_vertical};
use context_lifecycle_experiment::*;
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const BIN: &str = env!("CARGO_BIN_EXE_context-lifecycle-experiment");
const FIXTURE: &str = env!("CARGO_BIN_EXE_codex-fixture");
#[path = "support/loopback.rs"]
mod loopback;
struct Owned {
    child: Child,
    address: SocketAddr,
}
impl Owned {
    fn start(args: &[&str]) -> Result<Self> {
        let mut child = ProcessCommand::new(BIN)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = child.stdout.take().expect("stdout");
        let mut owned = Self {
            child,
            address: "127.0.0.1:0".parse()?,
        };
        let (send, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let read = BufReader::new(stdout).read_line(&mut line).map(|_| line);
            let _ = send.send(read);
        });
        let line = receive.recv_timeout(Duration::from_secs(10))??;
        if line.is_empty() {
            let mut error = String::new();
            if let Some(stderr) = owned.child.stderr.take() {
                stderr.take(8192).read_to_string(&mut error)?;
            }
            anyhow::bail!("process exited before readiness: {error}");
        }
        let ready: Value = serde_json::from_str(&line)?;
        ensure!(
            ready["pid"].as_u64() == Some(owned.child.id().into()),
            "child identity"
        );
        owned.address = ready["address"].as_str().expect("address").parse()?;
        Ok(owned)
    }
    fn kill(&mut self) -> Result<String> {
        self.child.kill()?;
        let status = self.child.wait()?;
        ensure!(!status.success(), "expected real kill");
        Ok(status.to_string())
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
struct Fixture {
    coordinator: Owned,
    adapter: Owned,
    root: tempfile::TempDir,
    launch: PathBuf,
    client: Client,
    previous_exits: Vec<Value>,
}
impl Fixture {
    fn new(scenario: &str) -> Result<Self> {
        Self::with_endpoint(
            FIXTURE.into(),
            Endpoint::Fixture {
                scenario: scenario.into(),
            },
        )
    }
    fn with_endpoint(executable: PathBuf, endpoint: Endpoint) -> Result<Self> {
        let root = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/target"))?;
        let launch = root.path().join("launch.json");
        fs::write(
            &launch,
            serde_json::to_vec(&Launch {
                executable,
                home: root.path().join("adapter"),
                endpoint,
            })?,
        )?;
        let adapter = Owned::start(&["codex-provider", path(&launch)])?;
        let coordinator = start_coordinator(root.path(), adapter.address)?;
        let client = Client::new(coordinator.address);
        Ok(Self {
            coordinator,
            adapter,
            root,
            launch,
            client,
            previous_exits: vec![],
        })
    }
    fn setup(&self) -> Result<Owner> {
        let owner = self
            .client
            .register(CommandId::new("register"), ContextId::new("context"))?;
        for input in ["one", "two"] {
            self.client.execute(Command {
                key: CommandId::new(format!("queue:{input}")),
                action: Action::Enqueue {
                    owner: owner.stamp(),
                    input: InputId::new(input),
                    payload: format!("Synthetic text {input}").into_bytes(),
                    lineage: format!("synthetic:{input}"),
                },
            })?;
        }
        Ok(owner)
    }
    fn admit(&self, owner: &Owner) -> Result<()> {
        self.client.execute(admission(owner.stamp()))?;
        Ok(())
    }
    fn wait(&self, state: ObservationState) -> Result<Effect> {
        self.client
            .observer()
            .wait(Condition::Native(EffectId::new("effect"), state), 5000)?;
        self.client.observer().result(EffectId::new("effect"))
    }
    fn control(&self, request: Value) -> Result<Value> {
        let address: SocketAddr =
            serde_json::from_slice(&fs::read(self.root.path().join("adapter/control.json"))?)?;
        process::rpc(address, &request)
    }
    fn native(&self) -> Result<Observation> {
        match process::rpc(
            self.adapter.address,
            &ProviderRequestMessage::Lookup(EffectId::new("effect")),
        )? {
            ProviderReply::Codex(observation) => Ok(observation),
            _ => anyhow::bail!("native observation reply"),
        }
    }
    fn restart(&mut self) -> Result<()> {
        let coordinator_pid = self.coordinator.child.id();
        let adapter_pid = self.adapter.child.id();
        let coordinator_exit = self.coordinator.kill()?;
        let adapter_exit = self.adapter.kill()?;
        self.previous_exits.push(json!({"coordinator":{"pid":coordinator_pid,"exit":coordinator_exit},"adapter":{"pid":adapter_pid,"exit":adapter_exit}}));
        self.adapter = Owned::start(&["codex-provider", path(&self.launch)])?;
        self.coordinator = start_coordinator(self.root.path(), self.adapter.address)?;
        self.client = Client::new(self.coordinator.address);
        Ok(())
    }
}
fn path(path: &Path) -> &str {
    path.to_str().expect("path")
}
fn start_coordinator(root: &Path, address: SocketAddr) -> Result<Owned> {
    Owned::start(&[
        "coordinator",
        path(&root.join("coordinator.db")),
        &address.to_string(),
    ])
}
fn admission(owner: Stamp) -> Command {
    Command {
        key: CommandId::new("admit"),
        action: Action::Admit {
            owner,
            input: InputId::new("one"),
            delivery: DeliveryId::new("delivery"),
            effect: EffectId::new("effect"),
            authority: SyntheticAuthority {
                approval: "explicit synthetic native observation".into(),
                deadline: u64::MAX,
                mode: Mode::ObserveCodex,
            },
        },
    }
}

#[test]
fn native_terminal_with_uncertain_tool_work_never_settles_delivery() -> Result<()> {
    let fixture = Fixture::new("delayed")?;
    for effect in ["missing", "different"] {
        let rejected: Result<ProviderReply> = process::rpc(
            fixture.adapter.address,
            &ProviderRequestMessage::Command(ProviderCommand {
                key: CommandId::new("rejected-interrupt"),
                action: ProviderAction::Interrupt {
                    effect: EffectId::new(effect),
                },
            }),
        );
        let expected = if effect == "missing" {
            "unknown native effect"
        } else {
            "native command key conflict"
        };
        assert!(rejected.unwrap_err().to_string().contains(expected));
    }
    let owner = fixture.setup()?;
    fixture.admit(&owner)?;
    fixture.wait(ObservationState::Pending)?;
    fixture.control(json!({"action":"wrong"}))?;
    fixture.control(json!({"action":"tool"}))?;
    let reply: ProviderReply = process::rpc(
        fixture.adapter.address,
        &ProviderRequestMessage::Command(ProviderCommand {
            key: CommandId::new("interrupt"),
            action: ProviderAction::Interrupt {
                effect: EffectId::new("effect"),
            },
        }),
    )?;
    let ProviderReply::Codex(ack) = reply else {
        anyhow::bail!("ack")
    };
    assert_eq!(ack.interrupt, Some(InterruptState::Acknowledged));
    assert_eq!(ack.state, ObservationState::Pending);
    assert!(ack.terminals.is_empty());
    fixture.control(json!({"action":"complete","status":"interrupted"}))?;
    let effect = fixture.wait(ObservationState::Interrupted)?;
    assert_eq!(effect.state, EffectState::Unknown);
    assert!(effect.evidence.is_empty());
    assert!(!effect.codex.expect("observation").activity.is_empty());
    assert!(
        fixture
            .client
            .admit(
                CommandId::new("next"),
                &owner,
                InputId::new("two"),
                DeliveryId::new("next-delivery"),
                EffectId::new("next-effect"),
                SyntheticAuthority {
                    approval: "synthetic".into(),
                    deadline: u64::MAX,
                    mode: Mode::ObserveCodex
                }
            )
            .is_err()
    );
    println!(
        "NATIVE interrupted observed; interrupt acknowledgment was separate; effect=Unknown, no seal, FIFO remains closed"
    );
    Ok(())
}

#[test]
fn early_terminal_exact_association_duplicates_and_conflict() -> Result<()> {
    let fixture = Fixture::new("early")?;
    let owner = fixture.setup()?;
    fixture.admit(&owner)?;
    let effect = fixture.wait(ObservationState::Completed)?;
    let observation = effect.codex.expect("native");
    assert_eq!(observation.terminals.len(), 1);
    assert_eq!(
        observation.turn_id.as_ref(),
        Some(&observation.terminals[0].turn.id)
    );
    fixture.control(json!({"action":"duplicate"}))?;
    fixture.control(json!({"action":"wrong"}))?;
    fixture.control(json!({"action":"conflict"}))?;
    let effect = fixture.wait(ObservationState::Conflicted)?;
    assert_eq!(effect.codex.expect("native").terminals.len(), 2);
    assert_eq!(effect.state, EffectState::Unknown);
    assert_eq!(fixture.control(json!({"action":"snapshot"}))?["starts"], 1);
    Ok(())
}

#[test]
fn adapter_restart_readback_requires_known_exact_turn_and_never_replays() -> Result<()> {
    let mut fixture = Fixture::new("delayed")?;
    let owner = fixture.setup()?;
    fixture.admit(&owner)?;
    fixture.wait(ObservationState::Pending)?;
    fixture.control(json!({"action":"silent-complete","status":"completed"}))?;
    assert_eq!(fixture.native()?.state, ObservationState::Pending);
    fixture.restart()?;
    let effect = fixture.wait(ObservationState::Completed)?;
    let observation = effect.codex.expect("native");
    assert_eq!(observation.processes.len(), 2);
    assert_eq!(fixture.control(json!({"action":"snapshot"}))?["starts"], 1);
    assert_eq!(effect.state, EffectState::Unknown);
    println!(
        "NATIVE restart exact-ID legacy readback=Completed, native start count=1, strict delivery=Unknown"
    );
    Ok(())
}

#[test]
fn lost_identity_eof_malformed_and_nonterminal_terminal_stay_unknown() -> Result<()> {
    for scenario in ["lost-response", "eof", "malformed", "inprogress-terminal"] {
        let mut fixture = Fixture::new(scenario)?;
        let owner = fixture.setup()?;
        fixture.admit(&owner)?;
        let effect = fixture.wait(ObservationState::Unknown)?;
        let observation = effect.codex.expect("native");
        assert!(observation.terminals.is_empty(), "{scenario}");
        if scenario != "inprogress-terminal" {
            assert!(observation.turn_id.is_none(), "{scenario}");
        }
        fixture.restart()?;
        let observation = fixture.native()?;
        assert_eq!(observation.state, ObservationState::Unknown, "{scenario}");
        assert_eq!(
            fixture.control(json!({"action":"snapshot"}))?["starts"],
            1,
            "{scenario}"
        );
        fixture.client.execute(admission(owner.stamp()))?;
        assert_eq!(
            fixture.control(json!({"action":"snapshot"}))?["starts"],
            1,
            "{scenario}"
        );
    }
    Ok(())
}

#[test]
fn disconnected_observer_does_not_own_native_turn_responsibility() -> Result<()> {
    let fixture = Fixture::new("delayed")?;
    let owner = fixture.setup()?;
    let mut stream = TcpStream::connect(fixture.coordinator.address)?;
    process::write_message(&mut stream, &Request::Command(admission(owner.stamp())))?;
    drop(stream);
    fixture.wait(ObservationState::Pending)?;
    fixture.control(json!({"action":"complete","status":"completed"}))?;
    let effect = fixture.wait(ObservationState::Completed)?;
    assert_eq!(effect.state, EffectState::Unknown);
    assert_eq!(fixture.control(json!({"action":"snapshot"}))?["starts"], 1);
    Ok(())
}

#[test]
fn server_user_input_requests_are_denied_during_rpc_wait_and_drain() -> Result<()> {
    let fixture = Fixture::new("request-wait")?;
    let owner = fixture.setup()?;
    fixture.admit(&owner)?;
    fixture.wait(ObservationState::Pending)?;
    fixture.control(json!({"action":"request-drain"}))?;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let denials = loop {
        let snapshot = fixture.control(json!({"action":"snapshot"}))?;
        let denials = snapshot["denied_requests"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if denials.len() >= 2 || std::time::Instant::now() >= deadline {
            break denials;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    ensure!(denials.len() == 2, "both native request paths must reject");
    for (index, id) in ["input-wait", "input-drain"].iter().enumerate() {
        assert_eq!(denials[index]["id"], *id);
        assert_eq!(denials[index]["error"]["code"], -32601);
        assert_eq!(
            denials[index]["error"]["message"],
            "observer adapter does not grant server-request authority"
        );
    }
    let observation = fixture.native()?;
    assert!(
        observation
            .notes
            .iter()
            .any(|note| note.contains("input-wait"))
    );
    assert!(
        observation
            .notes
            .iter()
            .any(|note| note.contains("input-drain"))
    );
    assert_eq!(
        fixture
            .client
            .observer()
            .result(EffectId::new("effect"))?
            .state,
        EffectState::Unknown
    );
    Ok(())
}

#[test]
fn same_status_final_content_conflict_fences_native_text_domain() -> Result<()> {
    let fixture = Fixture::new("delayed")?;
    let owner = fixture.setup()?;
    fixture.client.admit(
        CommandId::new("text:admit"),
        &owner,
        InputId::new("one"),
        DeliveryId::new("text:delivery"),
        EffectId::new("text:effect"),
        SyntheticAuthority {
            approval: "explicit synthetic text fixture".into(),
            deadline: u64::MAX,
            mode: Mode::CodexText,
        },
    )?;
    fixture.client.observer().wait(
        Condition::Native(EffectId::new("text:effect"), ObservationState::Pending),
        5000,
    )?;
    fixture.control(json!({"action":"content-conflict"}))?;
    fixture.client.observer().wait(
        Condition::Effect(EffectId::new("text:effect"), EffectState::Conflict),
        5000,
    )?;
    let effect = fixture
        .client
        .observer()
        .result(EffectId::new("text:effect"))?;
    assert_eq!(effect.state, EffectState::Conflict);
    assert_eq!(
        effect.codex.as_ref().expect("native").state,
        ObservationState::Conflicted
    );
    assert!(fixture.client.observer().snapshot()?.contexts[0].fenced);
    assert_eq!(fixture.control(json!({"action":"snapshot"}))?["starts"], 1);
    Ok(())
}

#[test]
fn text_unknown_entry_and_incomplete_final_never_replay_or_unlock_fifo() -> Result<()> {
    for scenario in ["lost-response", "full-empty"] {
        let mut fixture = Fixture::new(scenario)?;
        let owner = fixture.setup()?;
        let admission = Command {
            key: CommandId::new("text:admit"),
            action: Action::Admit {
                owner: owner.stamp(),
                input: InputId::new("one"),
                delivery: DeliveryId::new("delivery"),
                effect: EffectId::new("effect"),
                authority: SyntheticAuthority {
                    approval: "explicit synthetic text fixture".into(),
                    deadline: u64::MAX,
                    mode: Mode::CodexText,
                },
            },
        };
        fixture.client.execute(admission.clone())?;
        if scenario == "full-empty" {
            fixture.wait(ObservationState::Pending)?;
            fixture.control(json!({"action":"complete","status":"completed"}))?;
            fixture.wait(ObservationState::Completed)?;
        } else {
            fixture.wait(ObservationState::Unknown)?;
        }
        let effect = fixture.client.observer().result(EffectId::new("effect"))?;
        assert_eq!(effect.state, EffectState::Unknown, "{scenario}");
        assert!(
            effect
                .codex
                .as_ref()
                .expect("native")
                .text_receipt
                .is_none(),
            "{scenario}"
        );
        if scenario != "lost-response" {
            assert_eq!(fixture.control(json!({"action":"snapshot"}))?["starts"], 1);
        } else {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while fs::read(fixture.root.path().join("adapter/fixture.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .map(|state| state["starts"].clone())
                != Some(json!(1))
            {
                ensure!(
                    std::time::Instant::now() < deadline,
                    "lost-response start not observed"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        fixture.restart()?;
        fixture.client.execute(admission)?;
        assert_eq!(
            fixture.control(json!({"action":"snapshot"}))?["starts"],
            1,
            "{scenario}"
        );
        assert!(
            fixture
                .client
                .admit(
                    CommandId::new("text:second"),
                    &owner,
                    InputId::new("two"),
                    DeliveryId::new("second-delivery"),
                    EffectId::new("second-effect"),
                    SyntheticAuthority {
                        approval: "explicit synthetic text fixture".into(),
                        deadline: u64::MAX,
                        mode: Mode::CodexText
                    }
                )
                .is_err(),
            "{scenario}"
        );
    }
    Ok(())
}

#[test]
#[ignore = "explicit installed pinned Codex probe; no credentials, loopback simulated inference only"]
fn installed_cli_loopback_contract_probe() -> Result<()> {
    let backend = loopback::Backend::start()?;
    let mut fixture = Fixture::with_endpoint(
        context_lifecycle_experiment::codex::NATIVE_BINARY.into(),
        Endpoint::Loopback(backend.address),
    )?;
    let owner = fixture.setup()?;
    fixture.admit(&owner)?;
    let effect = fixture.wait(ObservationState::Completed)?;
    assert_eq!(effect.state, EffectState::Unknown);
    assert!(effect.evidence.is_empty());
    let observation = effect.codex.expect("native observation");
    let requests = backend.requests()?;
    assert_eq!(
        requests.len(),
        4,
        "three forced tool attempts and a final response"
    );
    assert!(requests.iter().all(|request| !request.authorization));
    let tools: Vec<_> = requests[0].body["input"][0]["tools"]
        .as_array()
        .expect("developer-message tool inventory")
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or("unknown"))
        .collect();
    let nested_tools: Vec<_> = requests[0].body["input"][0]["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|namespace| namespace["tools"].as_array().into_iter().flatten())
        .map(|tool| tool["name"].as_str().unwrap_or("unknown"))
        .collect();
    assert_eq!(tools, ["functions", "clock"]);
    assert_eq!(
        nested_tools,
        [
            "exec",
            "wait",
            "request_user_input",
            "request_user_input_async",
            "sleep"
        ]
    );
    println!(
        "CLI_PROBE model={} responses_tools={} developer_namespaces={:?} developer_nested_tools={:?}",
        requests[0].body["model"], requests[0].body["tools"], tools, nested_tools
    );
    let outcomes: Vec<_> = [
        (1, "disabled-shell-probe"),
        (2, "disabled-create-goal-probe"),
        (3, "disabled-update-goal-probe"),
    ]
    .into_iter()
    .map(|(index, call_id)| {
        requests[index].body["input"]
            .as_array()
            .expect("input")
            .iter()
            .find(|item| item["type"] == "function_call_output" && item["call_id"] == call_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("forced {call_id} has no observable result"))
    })
    .collect::<Result<_>>()?;
    println!(
        "CLI_PROBE version=0.160.1 model=gpt-6-sol pid={} thread={} turn={} developer_namespaces={} attempted_tool_outputs={} native={:?} effect=Unknown sealed=false",
        observation.processes[0].pid,
        observation.thread_id.as_deref().unwrap_or("unknown"),
        observation.turn_id.as_deref().unwrap_or("unknown"),
        serde_json::to_string(&tools)?,
        serde_json::to_string(&outcomes)?,
        observation.state
    );
    assert_eq!(outcomes[0]["output"], "unsupported call: exec_command");
    assert_eq!(outcomes[1]["output"], "unsupported call: create_goal");
    assert_eq!(outcomes[2]["output"], "unsupported call: update_goal");
    fixture.restart()?;
    let effect = fixture.wait(ObservationState::Completed)?;
    assert_eq!(effect.state, EffectState::Unknown);
    assert_eq!(
        backend.requests()?.len(),
        4,
        "readback must not make another inference request"
    );
    println!(
        "CLI_PROBE restart exact-ID observation retained; inference requests=4, real-provider turns=0"
    );
    Ok(())
}

#[test]
#[ignore = "explicit pinned Codex pure-discovery probe; loopback simulated inference only"]
fn installed_cli_namespaced_exec_discovery() -> Result<()> {
    let backend = loopback::Backend::start_pure_discovery()?;
    let fixture = Fixture::with_endpoint(
        context_lifecycle_experiment::codex::NATIVE_BINARY.into(),
        Endpoint::Loopback(backend.address),
    )?;
    let owner = fixture.setup()?;
    fixture.admit(&owner)?;
    let completion = fixture.wait(ObservationState::Completed);
    let requests = backend.requests()?;
    let first = requests
        .first()
        .ok_or_else(|| anyhow::anyhow!("no simulated inference request"))?;
    let response_tools = first.body["tools"].as_array().map(|tools| {
        tools
            .iter()
            .map(|tool| tool["name"].clone())
            .collect::<Vec<_>>()
    });
    let developer_tools = first.body["input"][0]["tools"].as_array().map(|tools| {
        tools
            .iter()
            .map(|tool| tool["name"].clone())
            .collect::<Vec<_>>()
    });
    let second_items = requests
        .get(1)
        .and_then(|request| request.body["input"].as_array());
    let second_output = second_items.and_then(|items| {
        items
            .iter()
            .find(|item| {
                item["call_id"] == "pure-discovery-call" && item["type"] != "custom_tool_call"
            })
            .cloned()
    });
    let second_item_types = second_items.map(|items| {
        items
            .iter()
            .map(|item| item["type"].clone())
            .collect::<Vec<_>>()
    });
    println!(
        "PURE_DISCOVERY first_model={} responses_tools={} developer_tools={} request_count={} second_item_types={} second_output={} completion={:?}",
        first.body["model"],
        serde_json::to_string(&response_tools)?,
        serde_json::to_string(&developer_tools)?,
        requests.len(),
        serde_json::to_string(&second_item_types)?,
        serde_json::to_string(&second_output)?,
        completion
            .as_ref()
            .map(|effect| effect.codex.as_ref().map(|o| &o.state))
    );
    ensure!(
        requests.iter().all(|request| !request.authorization),
        "authorization forbidden"
    );
    assert_eq!(first.body["tools"], Value::Null);
    assert_eq!(
        developer_tools,
        Some(vec![json!("functions"), json!("clock")])
    );
    assert_eq!(
        second_output.as_ref().map(|item| &item["call_id"]),
        Some(&json!("pure-discovery-call"))
    );
    assert_eq!(
        second_output.as_ref().map(|item| &item["output"]),
        Some(&json!("code-mode host is disabled"))
    );
    completion?;
    Ok(())
}

#[test]
#[ignore = "explicit pinned Codex user-input denial probe; loopback simulated inference only"]
fn installed_cli_namespaced_user_input_denial() -> Result<()> {
    let backend = loopback::Backend::start_user_input()?;
    let fixture = Fixture::with_endpoint(
        context_lifecycle_experiment::codex::NATIVE_BINARY.into(),
        Endpoint::Loopback(backend.address),
    )?;
    let owner = fixture.setup()?;
    fixture.admit(&owner)?;
    let completion = fixture.wait(ObservationState::Completed);
    let requests = backend.requests()?;
    let output = requests.get(1).and_then(|request| {
        request.body["input"].as_array().and_then(|items| {
            items
                .iter()
                .find(|item| {
                    item["call_id"] == "user-input-call" && item["type"] == "function_call_output"
                })
                .cloned()
        })
    });
    println!(
        "USER_INPUT request_count={} auth_headers={} output={} completion={:?} notes={:?}",
        requests.len(),
        requests
            .iter()
            .filter(|request| request.authorization)
            .count(),
        serde_json::to_string(&output)?,
        completion
            .as_ref()
            .map(|effect| effect.codex.as_ref().map(|o| &o.state)),
        completion
            .as_ref()
            .ok()
            .and_then(|effect| effect.codex.as_ref())
            .map(|o| &o.notes)
    );
    ensure!(
        requests.iter().all(|request| !request.authorization),
        "authorization forbidden"
    );
    assert_eq!(
        output.as_ref().map(|item| &item["output"]),
        Some(&json!("request_user_input is unavailable in Default mode"))
    );
    completion?;
    Ok(())
}

#[test]
#[ignore = "explicit pinned Codex async user-input routing probe; loopback simulated inference only"]
fn installed_cli_namespaced_async_user_input_routing() -> Result<()> {
    let backend = loopback::Backend::start_async_user_input()?;
    let mut fixture = Fixture::with_endpoint(
        context_lifecycle_experiment::codex::NATIVE_BINARY.into(),
        Endpoint::Loopback(backend.address),
    )?;
    let owner = fixture.setup()?;
    fixture.admit(&owner)?;
    let completion = fixture.wait(ObservationState::Completed);
    let requests = backend.requests()?;
    let output = requests.get(1).and_then(|request| {
        request.body["input"].as_array().and_then(|items| {
            items
                .iter()
                .find(|item| {
                    item["call_id"] == "async-user-input-call"
                        && item["type"] == "function_call_output"
                })
                .cloned()
        })
    });
    println!(
        "ASYNC_USER_INPUT request_count={} auth_headers={} output={} completion={:?} notes={:?}",
        requests.len(),
        requests
            .iter()
            .filter(|request| request.authorization)
            .count(),
        serde_json::to_string(&output)?,
        completion
            .as_ref()
            .map(|effect| effect.codex.as_ref().map(|o| &o.state)),
        completion
            .as_ref()
            .ok()
            .and_then(|effect| effect.codex.as_ref())
            .map(|o| &o.notes)
    );
    ensure!(
        requests.iter().all(|request| !request.authorization),
        "authorization forbidden"
    );
    assert_eq!(
        output.as_ref().map(|item| &item["output"]),
        Some(&json!("{\"accepted\":true}"))
    );
    let first_effect = completion?;
    retain_async_observability(&mut fixture, &requests, &first_effect)?;
    Ok(())
}

fn retain_async_observability(
    fixture: &mut Fixture,
    requests: &[loopback::Captured],
    first_effect: &Effect,
) -> Result<()> {
    let first_observation = first_effect
        .codex
        .as_ref()
        .expect("first native observation");
    let exact_thread = first_observation.thread_id.clone().expect("native thread");
    let exact_turn = first_observation.turn_id.clone().expect("native turn");
    fixture.restart()?;
    let readback_effect = fixture.wait(ObservationState::Completed)?;
    let readback_observation = readback_effect
        .codex
        .as_ref()
        .expect("readback observation");
    let trace_bytes = fs::read(fixture.root.path().join("adapter/native-stdio.jsonl"))?;
    ensure!(
        trace_bytes.len() <= 32 * 1024 * 1024,
        "bounded native trace"
    );
    let trace: Vec<Value> = trace_bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(serde_json::from_slice)
        .collect::<std::result::Result<_, _>>()?;
    let read_requests: Vec<_> = trace
        .iter()
        .filter_map(|record| {
            let raw: Value = serde_json::from_str(record["raw"].as_str()?).ok()?;
            (record["direction"] == "adapter_to_native" && raw["method"] == "thread/read")
                .then_some(raw)
        })
        .collect();
    ensure!(
        !read_requests.is_empty(),
        "native readback was not requested"
    );
    let read_id = &read_requests.last().expect("read request")["id"];
    let exact_readback = trace
        .iter()
        .filter_map(|record| {
            let raw: Value = serde_json::from_str(record["raw"].as_str()?).ok()?;
            (record["direction"] == "native_to_adapter" && raw["id"] == *read_id).then_some(raw)
        })
        .next_back()
        .ok_or_else(|| anyhow::anyhow!("readback response missing"))?;
    ensure!(
        exact_readback["result"]["thread"]["id"] == exact_thread,
        "readback thread mismatch"
    );
    let exact_turn_value = exact_readback["result"]["thread"]["turns"]
        .as_array()
        .and_then(|turns| turns.iter().find(|turn| turn["id"] == exact_turn))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("exact readback turn missing"))?;

    let evidence_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("evidence/async-user-input-observability");
    fs::create_dir_all(&evidence_dir)?;
    fs::write(evidence_dir.join("native-stdio.jsonl"), &trace_bytes)?;
    let exchanges: Vec<_> = requests
        .iter()
        .map(|request| {
            json!({
                "authorization_header_present":request.authorization,
                "request":request.body,
                "response_events":request.response_events
            })
        })
        .collect();
    fs::write(
        evidence_dir.join("simulator-exchanges.json"),
        serde_json::to_vec_pretty(&exchanges)?,
    )?;
    fs::write(
        evidence_dir.join("exact-thread-readback.json"),
        serde_json::to_vec_pretty(&exact_readback)?,
    )?;
    fs::write(
        evidence_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&json!({
            "native_version":context_lifecycle_experiment::codex::CLI_VERSION,
            "native_sha256":context_lifecycle_experiment::codex::NATIVE_SHA256,
            "model":"gpt-6-sol",
            "provider":"local_probe_loopback_responses",
            "feature_goals":false,
            "feature_code_mode_host":false,
            "thread_id":exact_thread,
            "turn_id":exact_turn,
            "native_processes":readback_observation.processes,
            "coordinator_pid":fixture.coordinator.child.id(),
            "adapter_pid":fixture.adapter.child.id(),
            "observed_prior_process_exits":fixture.previous_exits,
            "simulated_request_count":requests.len(),
            "authorization_header_count":requests.iter().filter(|request| request.authorization).count(),
            "first_observation":first_observation,
            "readback_observation":readback_observation,
            "exact_turn_readback":exact_turn_value,
            "strict_delivery":"Unknown",
            "real_provider_turns":0
        }))?,
    )?;
    println!(
        "ASYNC_OBSERVABILITY evidence={} native_items={} readback_items={}",
        evidence_dir.display(),
        first_observation
            .terminals
            .first()
            .map(|n| n.turn.items.len())
            .unwrap_or(0),
        exact_turn_value["items"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0)
    );
    Ok(())
}

#[test]
#[ignore = "explicit pinned Codex controlled three-turn lifecycle; loopback simulated inference only"]
fn installed_cli_controlled_text_vertical() -> Result<()> {
    let backend = loopback::Backend::start_text_vertical()?;
    let mut fixture = Fixture::with_endpoint(
        context_lifecycle_experiment::codex::NATIVE_BINARY.into(),
        Endpoint::Loopback(backend.address),
    )?;
    let coordinator_pid = fixture.coordinator.child.id();
    let deadline = SystemClock.now() + 600_000;
    let result = run_text_vertical(
        &fixture.client,
        TextVerticalConfig {
            first_input: b"Synthetic text one".to_vec(),
            second_input: b"Use the continuation fact already carried in this thread. Reply with SECOND: followed by its value, and nothing else.".to_vec(),
            first_fact: "FACT:ORION-7".into(),
            second_fact: "SECOND:ORION-7".into(),
            continuation_objective: "Finish queued input two".into(),
            limited_package_adequacy:
                "caller confirms exact continuation fact and limited package adequacy".into(),
            runtime: RuntimeId::new(format!("coordinator:{coordinator_pid}")),
            authority: SyntheticAuthority {
                approval: "explicit controlled native text simulation".into(),
                deadline,
                mode: Mode::CodexText,
            },
            transition_deadline: deadline,
            observation_timeout_ms: 10_000,
        },
    )?;
    let first_receipt = &result.first;
    let readiness_receipt = &result.readiness;
    let second_receipt = &result.second;
    let package = &result.package;
    let thread_a = first_receipt.thread_id.clone();
    let thread_b = readiness_receipt.thread_id.clone();
    assert!(
        fixture
            .client
            .execute(Command {
                key: CommandId::new("native:stale-old-admit"),
                action: Action::Admit {
                    owner: result.old_owner,
                    input: InputId::new("vertical-input-2"),
                    delivery: DeliveryId::new("stale-delivery"),
                    effect: EffectId::new("stale-effect"),
                    authority: SyntheticAuthority {
                        approval: "stale".into(),
                        deadline,
                        mode: Mode::CodexText
                    }
                },
            })
            .is_err()
    );
    assert_eq!(fixture.coordinator.child.id(), coordinator_pid);
    let requests = backend.requests()?;
    assert_eq!(requests.len(), 3);
    assert!(requests.iter().all(|request| !request.authorization));
    let evidence_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("evidence/controlled-text-vertical");
    fs::create_dir_all(&evidence_dir)?;
    let trace_bytes = fs::read(fixture.root.path().join("adapter/native-stdio.jsonl"))?;
    ensure!(
        trace_bytes.len() <= 16 * 1024 * 1024,
        "bounded native trace"
    );
    fs::write(evidence_dir.join("native-stdio.jsonl"), trace_bytes)?;
    let exchanges: Vec<_> = requests
        .iter()
        .map(|request| {
            json!({
                "authorization_header_present":request.authorization,
                "request":request.body,
                "response_events":request.response_events
            })
        })
        .collect();
    fs::write(
        evidence_dir.join("simulator-exchanges.json"),
        serde_json::to_vec_pretty(&exchanges)?,
    )?;
    let snapshot = fixture.client.observer().snapshot()?;
    for effect in snapshot
        .effects
        .iter()
        .filter(|effect| effect.domain == EvidenceDomain::CodexTextTurnCompletion)
    {
        let observation = effect.codex.as_ref().expect("native text observation");
        assert!(observation.full_turn.as_ref().is_some_and(|turn| {
            turn["items"]
                .as_array()
                .is_some_and(|items| items.iter().any(|item| item["type"] == "reasoning"))
        }));
        assert!(
            observation
                .activity
                .iter()
                .any(|frame| frame["method"] == "item/agentMessage/delta")
        );
        assert!(
            observation
                .activity
                .iter()
                .any(|frame| frame["method"] == "item/reasoning/summaryTextDelta")
        );
    }
    let adapter_pid = fixture.adapter.child.id();
    let adapter_exit = fixture.adapter.kill()?;
    let coordinator_exit = fixture.coordinator.kill()?;
    fs::write(
        evidence_dir.join("result.json"),
        serde_json::to_vec_pretty(&json!({
            "native_version":context_lifecycle_experiment::codex::CLI_VERSION,
            "native_sha256":context_lifecycle_experiment::codex::NATIVE_SHA256,
            "profile_digest":context_lifecycle_experiment::codex::profile_digest(),
            "model":"gpt-6-sol","provider":"local_probe_loopback_responses",
            "feature_goals":false,"feature_code_mode_host":false,
            "coordinator_pid":coordinator_pid,"coordinator_exit":coordinator_exit,
            "adapter_pid":adapter_pid,"adapter_exit":adapter_exit,
            "thread_a":thread_a,"thread_b":thread_b,
            "first_receipt":first_receipt,"readiness_receipt":readiness_receipt,"second_receipt":second_receipt,
            "package_digest":digest(&package),"snapshot":snapshot,
            "simulated_inference_requests":requests.len(),
            "authorization_headers":0,"real_provider_turns":0,
            "claim_scope":"local single dispatch per entered turn; no network-wide exactly-once guarantee"
        }))?,
    )?;
    println!(
        "NATIVE_VERTICAL coordinator_pid={coordinator_pid} thread_a={thread_a} thread_b={thread_b} turns=3 outputs={:?} readiness_package_digest={}",
        [
            first_receipt.output.as_str(),
            readiness_receipt.output.as_str(),
            second_receipt.output.as_str()
        ],
        digest(&package)
    );
    Ok(())
}

#[test]
#[ignore = "explicit pinned Codex adverse native text turn; loopback simulated inference only"]
fn installed_cli_async_input_attempt_never_completes_text_delivery() -> Result<()> {
    let backend = loopback::Backend::start_async_user_input()?;
    let fixture = Fixture::with_endpoint(
        context_lifecycle_experiment::codex::NATIVE_BINARY.into(),
        Endpoint::Loopback(backend.address),
    )?;
    let owner = fixture.setup()?;
    let effect_id = fixture.client.admit(
        CommandId::new("adverse:admit"),
        &owner,
        InputId::new("one"),
        DeliveryId::new("adverse:delivery"),
        EffectId::new("adverse:effect"),
        SyntheticAuthority {
            approval: "explicit adverse native text simulation".into(),
            deadline: u64::MAX,
            mode: Mode::CodexText,
        },
    )?;
    fixture.client.observer().wait(
        Condition::Native(effect_id.clone(), ObservationState::Completed),
        10_000,
    )?;
    let until = std::time::Instant::now() + Duration::from_secs(5);
    let effect = loop {
        let effect = fixture.client.observer().result(effect_id.clone())?;
        if effect
            .codex
            .as_ref()
            .and_then(|observation| observation.text_violation.as_ref())
            .is_some()
            || std::time::Instant::now() >= until
        {
            break effect;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(effect.state, EffectState::Unknown);
    let observation = effect.codex.as_ref().expect("native observation");
    assert!(observation.text_receipt.is_none());
    assert!(
        observation
            .text_violation
            .as_deref()
            .is_some_and(|reason| reason.contains("UserInputRequested"))
    );
    let requests = backend.requests()?;
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|request| !request.authorization));
    let evidence_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("evidence/async-text-violation");
    fs::create_dir_all(&evidence_dir)?;
    fs::write(
        evidence_dir.join("native-stdio.jsonl"),
        fs::read(fixture.root.path().join("adapter/native-stdio.jsonl"))?,
    )?;
    let exchanges: Vec<_> = requests.iter().map(|request| json!({"authorization_header_present":request.authorization,"request":request.body,"response_events":request.response_events})).collect();
    fs::write(
        evidence_dir.join("simulator-exchanges.json"),
        serde_json::to_vec_pretty(&exchanges)?,
    )?;
    fs::write(
        evidence_dir.join("result.json"),
        serde_json::to_vec_pretty(
            &json!({"effect":effect,"simulated_inference_requests":2,"authorization_headers":0,"real_provider_turns":0}),
        )?,
    )?;
    println!(
        "ASYNC_TEXT_VIOLATION reason={:?} strict=Unknown evidence={}",
        observation.text_violation,
        evidence_dir.display()
    );
    Ok(())
}

#[test]
#[ignore = "explicit pinned Codex disabled-host rejection; loopback simulated inference only"]
fn installed_cli_disabled_exec_rejection_then_text_completion() -> Result<()> {
    let backend = loopback::Backend::start_pure_discovery()?;
    let fixture = Fixture::with_endpoint(
        context_lifecycle_experiment::codex::NATIVE_BINARY.into(),
        Endpoint::Loopback(backend.address),
    )?;
    let owner = fixture.setup()?;
    let effect_id = fixture.client.admit(
        CommandId::new("tool:admit"),
        &owner,
        InputId::new("one"),
        DeliveryId::new("tool:delivery"),
        EffectId::new("tool:effect"),
        SyntheticAuthority {
            approval: "explicit adverse native tool simulation".into(),
            deadline: u64::MAX,
            mode: Mode::CodexText,
        },
    )?;
    fixture.client.observer().wait(
        Condition::Native(effect_id.clone(), ObservationState::Completed),
        10_000,
    )?;
    let until = std::time::Instant::now() + Duration::from_secs(5);
    let effect = loop {
        let effect = fixture.client.observer().result(effect_id.clone())?;
        if effect
            .codex
            .as_ref()
            .is_some_and(|observation| observation.readback_attempted)
            || std::time::Instant::now() >= until
        {
            break effect;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let observation = effect.codex.as_ref().expect("native observation");
    let requests = backend.requests()?;
    let denied = requests
        .get(1)
        .map(|request| &request.body["input"])
        .and_then(Value::as_array)
        .and_then(|items| {
            items.iter().find(|item| {
                item["call_id"] == "pure-discovery-call"
                    && item["type"] == "custom_tool_call_output"
            })
        })
        .cloned();
    let evidence_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("evidence/disabled-exec-text-rejection");
    fs::create_dir_all(&evidence_dir)?;
    fs::write(
        evidence_dir.join("native-stdio.jsonl"),
        fs::read(fixture.root.path().join("adapter/native-stdio.jsonl"))?,
    )?;
    fs::write(
        evidence_dir.join("observation.json"),
        serde_json::to_vec_pretty(observation)?,
    )?;
    fs::write(
        evidence_dir.join("simulator-exchanges.json"),
        serde_json::to_vec_pretty(
            &requests
                .iter()
                .map(|request| {
                    json!({"authorization":request.authorization,"body":request.body,"response_events":request.response_events})
                })
                .collect::<Vec<_>>(),
        )?,
    )?;
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|request| !request.authorization));
    assert_eq!(
        denied.as_ref().map(|item| &item["output"]),
        Some(&json!("code-mode host is disabled"))
    );
    assert_eq!(effect.state, EffectState::Success);
    assert!(observation.text_violation.is_none());
    assert!(observation.text_receipt.is_some());
    println!(
        "DISABLED_EXEC_REJECTED text_success=true evidence={}",
        evidence_dir.display()
    );
    Ok(())
}
