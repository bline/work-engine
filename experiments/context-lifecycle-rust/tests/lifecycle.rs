use anyhow::{Result, bail, ensure};
use context_lifecycle_experiment::provider::*;
use context_lifecycle_experiment::*;
use serde::Deserialize;
use std::io::{BufRead, BufReader, Read};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command as OsCommand, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_context-lifecycle-experiment");
const DEADLINE: u64 = u64::MAX;
#[derive(Deserialize)]
struct Ready {
    address: SocketAddr,
    pid: u32,
}
struct OwnedProcess {
    child: Child,
    address: SocketAddr,
}
impl OwnedProcess {
    fn start(args: &[&str]) -> Result<Self> {
        let mut child = OsCommand::new(BIN)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = child.stdout.take().expect("piped stdout");
        let mut owned = Self {
            child,
            address: "127.0.0.1:0".parse()?,
        };
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
            let _ = sender.send(result);
        });
        let line = receiver.recv_timeout(Duration::from_secs(5))??;
        if line.is_empty() {
            let status = owned.wait_for_exit()?;
            let mut error = String::new();
            if let Some(stderr) = owned.child.stderr.take() {
                stderr.take(4096).read_to_string(&mut error)?;
            }
            bail!("child exited {status} before readiness: {error}");
        }
        let ready: Ready = serde_json::from_str(&line)?;
        ensure!(ready.pid == owned.child.id(), "wrong child identity");
        owned.address = ready.address;
        Ok(owned)
    }
    fn kill_and_wait(&mut self) -> Result<ExitStatus> {
        self.child.kill()?;
        Ok(self.child.wait()?)
    }
    fn wait_for_exit(&mut self) -> Result<ExitStatus> {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait()? {
                return Ok(status);
            }
            ensure!(
                Instant::now() < until,
                "child did not exit within five seconds"
            );
            std::thread::yield_now();
        }
    }
}
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
struct Fixture {
    coordinator: OwnedProcess,
    provider: OwnedProcess,
    directory: tempfile::TempDir,
    client: Client,
}
impl Fixture {
    fn new(barrier: Option<&str>) -> Result<Self> {
        let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/target"))?;
        let provider =
            OwnedProcess::start(&["provider", path_str(&directory.path().join("provider.db"))])?;
        let coordinator = start_coordinator(directory.path(), provider.address, barrier)?;
        let client = Client::new(coordinator.address);
        Ok(Self {
            coordinator,
            provider,
            directory,
            client,
        })
    }
    fn restart(&mut self) -> Result<(u32, ExitStatus, u32)> {
        let prior = self.coordinator.child.id();
        let status = self.coordinator.kill_and_wait()?;
        self.coordinator = start_coordinator(self.directory.path(), self.provider.address, None)?;
        self.client = Client::new(self.coordinator.address);
        Ok((prior, status, self.coordinator.child.id()))
    }
    fn provider_command(&self, key: &str, action: ProviderAction) -> Result<ProviderReply> {
        process::rpc(
            self.provider.address,
            &ProviderRequestMessage::Command(ProviderCommand {
                key: CommandId::new(key),
                action,
            }),
        )
    }
    fn ledger(&self) -> Result<(Vec<ProviderRecord>, u64)> {
        match process::rpc(self.provider.address, &ProviderRequestMessage::Snapshot)? {
            ProviderReply::Snapshot {
                records,
                submit_calls,
            } => Ok((records, submit_calls)),
            _ => bail!("provider snapshot"),
        }
    }
    fn release(&self, effect: &str, outcome: Outcome) -> Result<()> {
        self.provider_command(
            &format!("release:{effect}"),
            ProviderAction::Publish {
                effect: EffectId::new(effect),
                outcome,
                sealed: true,
            },
        )?;
        Ok(())
    }
    fn register_two(&self) -> Result<Owner> {
        let owner = self
            .client
            .register(CommandId::new("register"), ContextId::new("context"))?;
        for (id, payload) in [
            ("one", vec![0, 255, 42, 0]),
            ("two", b"second input".to_vec()),
        ] {
            self.client.execute(command(
                &format!("enqueue:{id}"),
                Action::Enqueue {
                    owner: owner.stamp(),
                    input: InputId::new(id),
                    payload,
                    lineage: format!("synthetic:{id}"),
                },
            ))?;
        }
        Ok(owner)
    }
    fn wait(&self, effect: &str, state: EffectState) -> Result<Snapshot> {
        self.client
            .observer()
            .wait(Condition::Effect(EffectId::new(effect), state), 5000)
    }
    fn detached_admit(&self, owner: &Owner, mode: Mode) -> Result<()> {
        let mut socket = TcpStream::connect(self.coordinator.address)?;
        process::write_message(
            &mut socket,
            &Request::Command(admission("admit-one", owner.stamp(), "one", "e-one", mode)),
        )?;
        // Close before receiving any ticket. The actor still owns accepted responsibility.
        drop(socket);
        Ok(())
    }
}
fn path_str(path: &Path) -> &str {
    path.to_str().expect("test path UTF-8")
}
fn start_coordinator(
    path: &Path,
    provider: SocketAddr,
    barrier: Option<&str>,
) -> Result<OwnedProcess> {
    let database: PathBuf = path.join("coordinator.db");
    let address = provider.to_string();
    let mut args = vec!["coordinator", path_str(&database), &address];
    if let Some(barrier) = barrier {
        args.push(barrier);
    }
    OwnedProcess::start(&args)
}
fn command(key: &str, action: Action) -> Command {
    Command {
        key: CommandId::new(key),
        action,
    }
}
fn admission(key: &str, owner: Stamp, input: &str, effect: &str, mode: Mode) -> Command {
    command(
        key,
        Action::Admit {
            owner,
            input: InputId::new(input),
            delivery: DeliveryId::new(format!("delivery:{input}")),
            effect: EffectId::new(effect),
            authority: SyntheticAuthority {
                approval: format!("synthetic:explicit:{input}"),
                deadline: DEADLINE,
                mode,
            },
        },
    )
}
fn continuation() -> Continuation {
    Continuation {
        objective: "finish two synthetic inputs in order".into(),
        authority: "only separately admitted fake deliveries".into(),
        commitments: vec!["preserve held effect and binary payload".into()],
        references: vec!["synthetic:source:one".into(), "synthetic:authority".into()],
    }
}
fn capture(client: &Client, permit: &CapturePermit) -> Result<()> {
    client.execute(command(
        "capture",
        Action::Capture {
            permit: permit.clone(),
            continuation: continuation(),
        },
    ))?;
    Ok(())
}

#[test]
fn integrated_replacement_restart_and_fifo() -> Result<()> {
    let mut fixture = Fixture::new(None)?;
    let owner = fixture.register_two()?;
    let old = owner.stamp();
    let runtime_pid = std::process::id();
    let provider_pid = fixture.provider.child.id();
    let mut runtime = SyntheticRuntime::new(RuntimeId::new("surviving-runtime"), &owner);
    fixture.client.execute(admission(
        "admit-one",
        old.clone(),
        "one",
        "e-one",
        Mode::Hold,
    ))?;
    println!(
        "TRACE 1 context epoch=1 runtime_pid={runtime_pid} provider_pid={provider_pid}: first delivery entered; second queued"
    );
    let permit = fixture.client.replace(
        CommandId::new("replace"),
        owner,
        TransitionId::new("transition"),
        DEADLINE,
    )?;
    capture(&fixture.client, &permit)?;
    let package = fixture
        .client
        .observer()
        .bootstrap(permit.transition.clone())?;
    assert_eq!(package.unresolved.len(), 1);
    assert_eq!(package.queue[0].payload, vec![0, 255, 42, 0]);
    assert_eq!(package.queue[0].digest, digest(&package.queue[0].payload));
    assert_eq!(package.queue[0].lineage, "synthetic:one");
    let successor = fixture
        .client
        .activate(CommandId::new("activate"), runtime.load(package))?;
    assert_eq!(runtime.context_count(), 2);
    assert_eq!(std::process::id(), runtime_pid);
    assert_eq!(successor.stamp().epoch, Epoch(2));
    println!(
        "TRACE 2 replacement complete: two model contexts in surviving runtime; epoch=2 carries unresolved e-one"
    );
    assert!(
        fixture
            .client
            .execute(admission(
                "stale",
                old,
                "two",
                "stale-effect",
                Mode::Succeed
            ))
            .is_err()
    );
    assert!(
        fixture
            .client
            .execute(admission(
                "blocked",
                successor.stamp(),
                "two",
                "e-two",
                Mode::Succeed
            ))
            .is_err()
    );
    let (prior_pid, exit, next_pid) = fixture.restart()?;
    assert!(!exit.success());
    assert_ne!(prior_pid, next_pid);
    assert_eq!(fixture.provider.child.id(), provider_pid);
    let recovered = fixture.client.observer().result(EffectId::new("e-one"))?;
    assert_eq!(recovered.state, EffectState::Unknown);
    let (records, calls) = fixture.ledger()?;
    assert_eq!(calls, 1);
    assert_eq!(records.len(), 1);
    println!(
        "TRACE 3 coordinator pid={prior_pid} exited={exit}; new pid={next_pid}; provider unchanged; submit_calls={calls}; e-one=Unknown, no resend"
    );
    fixture.release("e-one", Outcome::Success("held effect sealed".into()))?;
    fixture.wait("e-one", EffectState::Success)?;
    let same_result_api = fixture.client.observer().result(EffectId::new("e-one"))?;
    assert_eq!(same_result_api.evidence.len(), 1);
    println!(
        "TRACE 4 exact sealed evidence reconciled through owned executor; same result API reports Success"
    );
    fixture.client.admit(
        CommandId::new("admit-two"),
        &successor,
        InputId::new("two"),
        DeliveryId::new("delivery:two"),
        EffectId::new("e-two"),
        SyntheticAuthority {
            approval: "synthetic:explicit:two".into(),
            deadline: DEADLINE,
            mode: Mode::Succeed,
        },
    )?;
    fixture.wait("e-two", EffectState::Success)?;
    let (records, calls) = fixture.ledger()?;
    assert_eq!(calls, 2);
    assert_eq!(records.len(), 2);
    assert!(
        records
            .iter()
            .all(|r| r.executions == 1 && r.evidence.iter().any(|e| e.sealed))
    );
    println!(
        "TRACE 5 separately authorized second input succeeded; independent provider ledger: two submissions, two effects, one execution each"
    );
    Ok(())
}

#[test]
fn immediate_failure_precedes_ticket_and_survives_observer_cancellation() -> Result<()> {
    let fixture = Fixture::new(Some("after-result"))?;
    let owner = fixture.register_two()?;
    fixture.detached_admit(&owner, Mode::Fail)?;
    let snapshot = fixture.client.observer().wait(Condition::Barrier, 5000)?;
    assert_eq!(snapshot.barrier.expect("barrier").stage, Stage::AfterResult);
    assert_eq!(snapshot.effects[0].state, EffectState::Failure);
    assert_eq!(snapshot.deliveries[0].outcome, DeliveryOutcome::Failure);
    assert_eq!(snapshot.effects[0].evidence.len(), 1);
    fixture.client.release_barrier()?;
    let result_before_timeout = fixture.client.observer().result(EffectId::new("e-one"))?;
    assert!(
        fixture
            .client
            .observer()
            .wait(
                Condition::Effect(EffectId::new("e-one"), EffectState::Failure),
                0
            )
            .is_err()
    );
    assert_eq!(
        fixture.client.observer().result(EffectId::new("e-one"))?,
        result_before_timeout
    );
    let replay = fixture.client.execute(admission(
        "admit-one",
        owner.stamp(),
        "one",
        "e-one",
        Mode::Fail,
    ))?;
    assert_eq!(replay, Reply::Ticket(EffectId::new("e-one")));
    assert_eq!(fixture.ledger()?.1, 1);
    Ok(())
}

#[test]
fn crash_boundaries_preserve_uncertainty_without_resend() -> Result<()> {
    for barrier in ["before-entry", "after-entry", "after-send", "after-result"] {
        let mut fixture = Fixture::new(Some(barrier))?;
        let owner = fixture.register_two()?;
        let mode = if barrier == "after-result" {
            Mode::Succeed
        } else {
            Mode::Hold
        };
        fixture.detached_admit(&owner, mode.clone())?;
        fixture.client.observer().wait(Condition::Barrier, 5000)?;
        let calls_before = fixture.ledger()?.1;
        let expected_before = u64::from(matches!(barrier, "after-send" | "after-result"));
        assert_eq!(calls_before, expected_before, "{barrier}");
        let (_, status, _) = fixture.restart()?;
        assert!(!status.success());
        match barrier {
            "before-entry" => {
                // Prepared work has never entered and may safely enter after restart.
                fixture.client.execute(admission(
                    "admit-one",
                    owner.stamp(),
                    "one",
                    "e-one",
                    mode,
                ))?;
                assert_eq!(fixture.ledger()?.1, 1);
            }
            "after-entry" => {
                fixture.wait("e-one", EffectState::Unknown)?;
                assert_eq!(fixture.ledger()?.1, 0);
                assert!(
                    fixture
                        .client
                        .execute(admission(
                            "blocked",
                            owner.stamp(),
                            "two",
                            "e-two",
                            Mode::Succeed
                        ))
                        .is_err()
                );
            }
            "after-send" => {
                fixture.wait("e-one", EffectState::Unknown)?;
                assert_eq!(fixture.ledger()?.1, 1);
                fixture.release(
                    "e-one",
                    Outcome::Failure("late authoritative failure".into()),
                )?;
                fixture.wait("e-one", EffectState::Failure)?;
            }
            "after-result" => {
                fixture.wait("e-one", EffectState::Success)?;
                let reply = fixture.client.execute(admission(
                    "admit-one",
                    owner.stamp(),
                    "one",
                    "e-one",
                    mode,
                ))?;
                assert_eq!(reply, Reply::Ticket(EffectId::new("e-one")));
                assert_eq!(fixture.ledger()?.1, 1);
            }
            _ => unreachable!(),
        }
        println!(
            "BOUNDARY {barrier}: actual process exit={status}, provider calls before={calls_before}, after={}",
            fixture.ledger()?.1
        );
    }
    Ok(())
}

#[test]
fn stale_owners_capture_misuse_readiness_and_key_conflicts() -> Result<()> {
    let fixture = Fixture::new(None)?;
    let owner = fixture.register_two()?;
    let stale = owner.stamp();
    assert!(
        fixture
            .client
            .execute(admission(
                "overtake",
                stale.clone(),
                "two",
                "e-two",
                Mode::Succeed
            ))
            .is_err()
    );
    assert!(
        fixture
            .client
            .execute(admission(
                "overtake",
                stale.clone(),
                "one",
                "e-one",
                Mode::Succeed
            ))
            .unwrap_err()
            .to_string()
            .contains("command key conflict")
    );
    assert!(
        fixture
            .client
            .execute(command(
                "enqueue:one",
                Action::Enqueue {
                    owner: stale.clone(),
                    input: InputId::new("one"),
                    payload: vec![9],
                    lineage: "synthetic:one".into()
                }
            ))
            .is_err()
    );
    let mut runtime = SyntheticRuntime::new(RuntimeId::new("runtime"), &owner);
    let permit = fixture.client.replace(
        CommandId::new("replace"),
        owner,
        TransitionId::new("t"),
        DEADLINE,
    )?;
    assert!(
        fixture
            .client
            .execute(admission(
                "capture-as-owner",
                Stamp {
                    context: permit.context.clone(),
                    epoch: permit.epoch
                },
                "one",
                "e-one",
                Mode::Succeed
            ))
            .is_err()
    );
    let mut wrong_permit = permit.clone();
    wrong_permit.epoch = Epoch(99);
    assert!(
        fixture
            .client
            .execute(command(
                "bad-permit",
                Action::Capture {
                    permit: wrong_permit,
                    continuation: continuation()
                }
            ))
            .is_err()
    );
    assert!(
        fixture
            .client
            .observer()
            .bootstrap(permit.transition.clone())
            .is_err()
    );
    capture(&fixture.client, &permit)?;
    let package = fixture
        .client
        .observer()
        .bootstrap(permit.transition.clone())?;
    let attestation = runtime.load(package);
    let mut wrong = attestation.clone();
    wrong.context = ContextId::new("wrong");
    assert!(
        fixture
            .client
            .activate(CommandId::new("wrong-id"), wrong)
            .is_err()
    );
    let mut wrong = attestation.clone();
    wrong.revision = Revision(0);
    assert!(
        fixture
            .client
            .activate(CommandId::new("wrong-revision"), wrong)
            .is_err()
    );
    let mut wrong = attestation.clone();
    wrong.digest = "different bytes".into();
    assert!(
        fixture
            .client
            .activate(CommandId::new("wrong-digest"), wrong)
            .is_err()
    );
    let successor = fixture
        .client
        .activate(CommandId::new("activate"), attestation)?;
    assert!(
        fixture
            .client
            .execute(admission("stale", stale, "one", "e-one", Mode::Succeed))
            .is_err()
    );
    assert!(
        fixture
            .client
            .execute(command(
                "late-capture",
                Action::Capture {
                    permit,
                    continuation: continuation()
                }
            ))
            .is_err()
    );
    fixture.client.execute(admission(
        "new",
        successor.stamp(),
        "one",
        "e-one",
        Mode::Succeed,
    ))?;
    Ok(())
}

#[test]
fn changed_current_state_invalidates_readiness_and_restart_bootstrap_is_live() -> Result<()> {
    let mut fixture = Fixture::new(None)?;
    let owner = fixture.register_two()?;
    fixture.client.execute(admission(
        "admit-one",
        owner.stamp(),
        "one",
        "e-one",
        Mode::Hold,
    ))?;
    let mut runtime = SyntheticRuntime::new(RuntimeId::new("runtime"), &owner);
    let permit = fixture.client.replace(
        CommandId::new("replace"),
        owner,
        TransitionId::new("t"),
        DEADLINE,
    )?;
    capture(&fixture.client, &permit)?;
    let package = fixture
        .client
        .observer()
        .bootstrap(permit.transition.clone())?;
    assert_eq!(package.unresolved.len(), 1);
    let old_attestation = runtime.load(package);
    fixture.restart()?;
    fixture.release(
        "e-one",
        Outcome::Success("settled during replacement".into()),
    )?;
    fixture.wait("e-one", EffectState::Success)?;
    assert!(
        fixture
            .client
            .activate(CommandId::new("stale-ready"), old_attestation)
            .is_err()
    );
    let current = fixture.client.observer().bootstrap(permit.transition)?;
    assert!(current.unresolved.is_empty());
    fixture
        .client
        .activate(CommandId::new("current-ready"), runtime.load(current))?;
    Ok(())
}

#[test]
fn capture_terminal_unknown_and_failed_cannot_be_resurrected() -> Result<()> {
    for outcome in [
        CaptureEnd::Unknown("turn ended without meaning".into()),
        CaptureEnd::Failed("explicit capture failure".into()),
    ] {
        let fixture = Fixture::new(None)?;
        let owner = fixture.register_two()?;
        let permit = fixture.client.replace(
            CommandId::new("replace"),
            owner,
            TransitionId::new("t"),
            DEADLINE,
        )?;
        fixture.client.execute(command(
            "turn-ended",
            Action::EndCapture {
                permit: permit.clone(),
                outcome: outcome.clone(),
            },
        ))?;
        assert!(capture(&fixture.client, &permit).is_err());
        assert!(
            fixture
                .client
                .observer()
                .bootstrap(permit.transition)
                .is_err()
        );
        let snapshot = fixture.client.observer().snapshot()?;
        assert!(!snapshot.contexts[0].active);
        assert!(!snapshot.transitions[0].completed);
        match outcome {
            CaptureEnd::Unknown(_) => assert!(matches!(
                snapshot.transitions[0].capture,
                CaptureState::Unknown(_)
            )),
            CaptureEnd::Failed(_) => assert!(matches!(
                snapshot.transitions[0].capture,
                CaptureState::Failed(_)
            )),
        }
    }
    Ok(())
}

#[test]
fn provisional_duplicate_and_conflicting_evidence() -> Result<()> {
    let fixture = Fixture::new(None)?;
    let owner = fixture.register_two()?;
    fixture.client.execute(admission(
        "admit-one",
        owner.stamp(),
        "one",
        "e-one",
        Mode::Hold,
    ))?;
    fixture.provider_command(
        "provisional",
        ProviderAction::Publish {
            effect: EffectId::new("e-one"),
            outcome: Outcome::Success("provisional".into()),
            sealed: false,
        },
    )?;
    fixture.wait("e-one", EffectState::Unknown)?;
    assert!(
        fixture
            .client
            .execute(admission(
                "blocked",
                owner.stamp(),
                "two",
                "e-two",
                Mode::Succeed
            ))
            .is_err()
    );
    fixture.release("e-one", Outcome::Success("sealed".into()))?;
    fixture.wait("e-one", EffectState::Success)?;
    fixture.provider_command(
        "duplicate",
        ProviderAction::Publish {
            effect: EffectId::new("e-one"),
            outcome: Outcome::Success("sealed".into()),
            sealed: true,
        },
    )?;
    assert_eq!(
        fixture
            .client
            .observer()
            .result(EffectId::new("e-one"))?
            .evidence
            .len(),
        2
    );
    fixture.client.execute(admission(
        "admit-two",
        owner.stamp(),
        "two",
        "e-two",
        Mode::Succeed,
    ))?;
    assert!(
        fixture
            .provider_command(
                "normal-conflict",
                ProviderAction::Publish {
                    effect: EffectId::new("e-one"),
                    outcome: Outcome::Failure("contradiction".into()),
                    sealed: true
                }
            )
            .is_err()
    );
    fixture.provider_command(
        "broken-provider",
        ProviderAction::Contradict {
            effect: EffectId::new("e-one"),
            outcome: Outcome::Failure("contradiction".into()),
        },
    )?;
    let snapshot = fixture.wait("e-one", EffectState::Conflict)?;
    assert!(snapshot.contexts[0].fenced);
    assert_eq!(
        snapshot
            .effects
            .iter()
            .find(|e| e.id == EffectId::new("e-one"))
            .expect("effect")
            .evidence
            .len(),
        3
    );
    assert!(
        fixture
            .client
            .execute(command(
                "fenced",
                Action::Enqueue {
                    owner: owner.stamp(),
                    input: InputId::new("three"),
                    payload: vec![],
                    lineage: "synthetic".into()
                }
            ))
            .is_err()
    );
    assert_eq!(fixture.ledger()?.0.len(), 2); // Historical effects cannot be undone by a later fence.
    Ok(())
}

#[test]
fn normal_process_exit_is_not_settlement_and_unsafe_history_survives() -> Result<()> {
    let mut fixture = Fixture::new(None)?;
    let owner = fixture.register_two()?;
    fixture.client.execute(admission(
        "admit-one",
        owner.stamp(),
        "one",
        "e-one",
        Mode::Hold,
    ))?;
    let mut worker = OwnedProcess {
        child: OsCommand::new(BIN).arg("synthetic-worker-exit").spawn()?,
        address: "127.0.0.1:0".parse()?,
    };
    let pid = worker.child.id();
    let status = worker.wait_for_exit()?;
    assert!(status.success());
    fixture.client.execute(command(
        "observed-exit",
        Action::ObserveLocalExit {
            owner: owner.stamp(),
            pid,
            status: status.to_string(),
        },
    ))?;
    let snapshot = fixture.client.observer().snapshot()?;
    assert_eq!(snapshot.effects[0].state, EffectState::Entered);
    assert!(snapshot.exits[0].unsafe_at_time);
    fixture.release("e-one", Outcome::Success("late after local exit".into()))?;
    fixture.wait("e-one", EffectState::Success)?;
    fixture.restart()?;
    let snapshot = fixture.client.observer().snapshot()?;
    assert!(snapshot.exits[0].unsafe_at_time);
    assert_eq!(snapshot.exits[0].unresolved, vec![EffectId::new("e-one")]);
    assert!(snapshot.effects.iter().all(|e| !e.state.unresolved()));
    println!(
        "EXIT worker_pid={pid}, observed={status}; unsafe-at-time remains after authoritative settlement and coordinator restart"
    );
    Ok(())
}

#[test]
fn exclusive_process_lock_and_bounded_observer_timeout() -> Result<()> {
    let fixture = Fixture::new(Some("before-entry"))?;
    let owner = fixture.register_two()?;
    let contender = start_coordinator(fixture.directory.path(), fixture.provider.address, None);
    assert!(
        contender
            .err()
            .expect("contender must fail")
            .to_string()
            .contains("exclusive coordinator lock unavailable")
    );
    fixture.detached_admit(&owner, Mode::Hold)?;
    fixture.client.observer().wait(Condition::Barrier, 5000)?;
    assert!(
        fixture
            .client
            .observer()
            .wait(
                Condition::Effect(EffectId::new("e-one"), EffectState::Success),
                0
            )
            .is_err()
    );
    assert_eq!(
        fixture
            .client
            .observer()
            .result(EffectId::new("e-one"))?
            .state,
        EffectState::Prepared
    );
    fixture.client.release_barrier()?;
    fixture.client.execute(admission(
        "admit-one",
        owner.stamp(),
        "one",
        "e-one",
        Mode::Hold,
    ))?;
    assert_eq!(fixture.ledger()?.1, 1);
    Ok(())
}
