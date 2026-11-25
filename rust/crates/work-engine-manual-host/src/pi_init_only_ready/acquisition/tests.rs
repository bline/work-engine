use super::super::{artifact_contract::CHECKPOINT, tests as ready};
use super::*;
use slice_campaign::{
    Campaign,
    offline_qualification::{
        OfflineAcceptedDisposition, OfflineCleanupTail, OfflineEntryFence,
        OfflineOldTaskTransitions, OfflineOtherWork, OfflineRecoveryBacking, TerminalFailure,
    },
};
use std::{
    fs,
    io::{Read, Write},
    net::Shutdown,
    os::{
        fd::AsRawFd,
        unix::{
            ffi::{OsStrExt, OsStringExt},
            fs::PermissionsExt,
            net::UnixListener,
        },
    },
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

const ID: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const SOURCE: &str = "8888888888888888888888888888888888888888888888888888888888888888";
const LIVE: &[u8] = b"populated 1\nfrozen 0\n";
const END: &[u8] = b"populated 0\nfrozen 0\n";
// Authored offline projection, not a capture from the installed daemon and
// not computed from the sent Create body. Include Moby's nullable fields and
// separate Config, HostConfig, State, and image identity fields.
const CREATED_INSPECT: &[u8] = br#"{
  "Id":"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
  "Image":"sha256:3333333333333333333333333333333333333333333333333333333333333333",
  "Path":"/work-engine-b-init-only", "Args":null, "Mounts":[],
  "State":{"Status":"created","Running":false,"Pid":0,"Paused":false},
  "Config":{
    "Image":"sha256:3333333333333333333333333333333333333333333333333333333333333333",
    "User":"65534:65534", "Entrypoint":["/work-engine-b-init-only"],
    "Cmd":null, "WorkingDir":"/", "Env":null, "Volumes":null,
    "Healthcheck":{"Test":["NONE"]}, "Tty":false, "OpenStdin":false,
    "StdinOnce":false, "AttachStdin":false, "AttachStdout":true, "AttachStderr":false
  },
  "HostConfig":{
    "Runtime":"runc", "NetworkMode":"none", "CgroupnsMode":"private",
    "ReadonlyRootfs":true, "Privileged":false, "CapDrop":["ALL"], "Init":false,
    "RestartPolicy":{"Name":"no","MaximumRetryCount":0}, "AutoRemove":false,
    "LogConfig":{"Type":"none","Config":{}},
    "SecurityOpt":["no-new-privileges","apparmor=docker-default"]
  }
}"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fault {
    None,
    Early,
    AmbiguousBoundary,
    HeldCutLost,
    PolicyCoverageLost,
    PolicyCoverageStartedTooLate,
    PolicyWrongCapture,
    PolicyWrongPayload,
    PolicyWrongSource,
    TerminalCutLost,
    CrossedBeforeStart,
    CrossedBeforeStop,
    StopLocalOnly,
    StopWrongChannel,
    StopWrongTarget,
    StopWrongGeneration,
    DelayedStopAfterClientLoss,
    CrossedCreate,
    CrossedStart,
    CrossedAttach,
    CrossedTerminal,
    MissingPolicy,
    LostCreate,
    LostStart,
    LostStop,
    PendingStart,
    IncompleteTerminal,
    Populated,
    MissingOriginalExit,
    Crash,
    ContestedLater,
}

#[derive(Default)]
struct Observations {
    stopped: AtomicBool,
    kernel_current_at: Mutex<Option<Instant>>,
    kernel_end_read: AtomicBool,
    policy: Mutex<Option<(Instant, serde_json::Value)>>,
    // The simulated runtime owns this restriction, not a client/fence drop
    // guard. The delayed handler consults it separately from local submission.
    resolution_scope: Mutex<Option<StopExchange>>,
    handler_resolved_after_client_loss: AtomicBool,
}

struct FakeSource {
    fault: Fault,
    serving: Serving,
    observations: Arc<Observations>,
    early_writer: Receiver<UnixStream>,
    delayed_resolution: Receiver<()>,
    stop: Option<OfflineStopSubject>,
}

struct FakeFence {
    fault: Fault,
    observations: Arc<Observations>,
}

impl StartFence for FakeFence {
    fn confirm_cut(
        &mut self,
        capture: &HeldCapture,
        _: Instant,
        _: Instant,
    ) -> Result<AppliedCoverage> {
        match self.fault {
            Fault::AmbiguousBoundary => return refuse("marker could precede submission"),
            Fault::HeldCutLost => return refuse("live launch guard broke during final collection"),
            _ => {}
        }
        let kernel_at = self.observations.kernel_current_at.lock().unwrap().unwrap();
        let policy = self.observations.policy.lock().unwrap();
        let (from, payload) = policy.as_ref().unwrap();
        let mut observed_payload = payload.clone();
        if self.fault == Fault::PolicyWrongPayload {
            // Same task, launch/root, stdout and policy label. Change one
            // effective policy field only in the coverage source's subject.
            observed_payload["effective"]["applied_apparmor_sha256"] = json!("e".repeat(64));
            assert_ne!(observed_payload, *payload);
        }
        let read_at = Instant::now();
        Ok(AppliedCoverage {
            source: if self.fault == Fault::PolicyWrongSource {
                "b".repeat(64)
            } else {
                SOURCE.into()
            },
            capture_position: capture.position + u64::from(self.fault == Fault::PolicyWrongCapture),
            applied: AppliedRecord {
                position: capture.applied.position,
                sha256: crate::digest(&serde_json::to_vec(&observed_payload)?),
            },
            evidence_sha256: "c".repeat(64),
            from: if self.fault == Fault::PolicyCoverageStartedTooLate {
                read_at
            } else {
                *from
            },
            through: if self.fault == Fault::PolicyCoverageLost {
                kernel_at
            } else {
                read_at
            },
        })
    }
}

impl StopFence for FakeFence {
    fn confirm_end(
        &mut self,
        (begin, end): (Instant, Instant),
        _: Instant,
    ) -> Result<OfflineContainment> {
        assert!(begin <= end && self.observations.kernel_end_read.load(Ordering::SeqCst));
        if self.fault == Fault::TerminalCutLost {
            return refuse("containment lost at original final kernel sample");
        }
        Ok(OfflineContainment::ContinuousThroughFinalRead)
    }
}

impl ProtectedSource for FakeSource {
    type StartFence = FakeFence;
    type StopFence = FakeFence;

    fn identity(&self) -> &str {
        SOURCE
    }
    fn preflight(&mut self, _: &AttemptBinding, _: &Endpoint, _: Instant) -> Result<()> {
        Ok(())
    }
    fn serving(&mut self, _: &Endpoint, _: Instant) -> Result<Serving> {
        Ok(self.serving.clone())
    }

    fn fence_start(
        &mut self,
        start: &StartRpc,
        stdout: BorrowedFd<'_>,
        request: BorrowedFd<'_>,
        _: Instant,
    ) -> Result<(Attach, FakeFence)> {
        assert_eq!(start.exchange, docker::socket_cookie(&request)?);
        let connection = docker::socket_cookie(&stdout)?;
        assert_ne!(connection, start.exchange);
        if self.fault == Fault::AmbiguousBoundary {
            // Called after the controller's final pre-write drain. The marker
            // is receivable after the write but is not caused by the Start.
            self.early_writer
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .write_all(&ready::mux(1, CHECKPOINT))
                .unwrap();
        }
        Ok((
            Attach {
                serving: self.serving.clone(),
                connection,
                resolved_full_id: if self.fault == Fault::CrossedBeforeStart {
                    "e".repeat(64)
                } else {
                    ID.into()
                },
                http_request: literal_attach(),
                registration: Registration::AuthenticatedPinnedSameObject,
            },
            FakeFence {
                fault: self.fault,
                observations: self.observations.clone(),
            },
        ))
    }

    fn selected(&mut self, start: &StartRpc, _: Instant) -> Result<SelectedTask> {
        assert_ne!(
            self.fault,
            Fault::Crash,
            "simulated loss of process-local continuation"
        );
        let key = ready::task(&self.serving, &start.full_id);
        let mut start = start.clone();
        if self.fault == Fault::CrossedStart {
            start.links.start_mark = "0".repeat(64);
        }
        Ok(SelectedTask { start, task: key })
    }

    fn applied(
        &mut self,
        start: &StartRpc,
        attach: &Attach,
        key: &TaskKey,
        _: Instant,
    ) -> Result<AppliedWitness> {
        let mut fixture = ready::Synthetic::new();
        let cut = fixture.cut();
        let mut launch = cut.launch.clone();
        let mut effective = cut.effective.clone();
        launch.task = key.clone();
        effective.task = key.clone();
        let mut attach = attach.clone();
        if self.fault == Fault::MissingPolicy {
            effective.applied_seccomp_sha256.clear();
        }
        if self.fault == Fault::CrossedAttach {
            attach.connection += 1;
        }
        let witness = AppliedWitness {
            start: start.clone(),
            attach,
            launch,
            effective,
        };
        *self.observations.policy.lock().unwrap() =
            Some((Instant::now(), serde_json::to_value(&witness)?));
        Ok(witness)
    }

    fn fence_stop(
        &mut self,
        exchange: &StopExchange,
        request: BorrowedFd<'_>,
        _: Instant,
    ) -> Result<(StopResolution, FakeFence)> {
        assert_eq!(exchange.task.container, ID);
        assert_eq!(exchange.subject.execution.live.running.object.full_id, ID);
        let actual_cookie = docker::socket_cookie(&request)?;
        assert_eq!(exchange.connection, actual_cookie);
        if self.fault == Fault::CrossedBeforeStop {
            return refuse("Stop resolution changed before send");
        }
        assert_eq!(exchange.request_target, literal_stop_target());
        let mut observed = StopExchange {
            subject: exchange.subject.clone(),
            task: ready::task(&self.serving, ID),
            request_target: literal_stop_target(),
            connection: actual_cookie,
        };
        match self.fault {
            Fault::StopWrongChannel => observed.connection += 1,
            Fault::StopWrongTarget => observed.request_target.push_str("&force=1"),
            Fault::StopWrongGeneration => observed.task.serving.daemon_process_start_ticks += 1,
            _ => {}
        }
        let scope = if self.fault == Fault::StopLocalOnly {
            // This guard does not cover the pending-handler state; a
            // submitted request could still look up a different object.
            ResolutionScope::LocalChannelOnly
        } else {
            *self.observations.resolution_scope.lock().unwrap() = Some(observed.clone());
            ResolutionScope::ThroughHandlerResolutionOrInert {
                enforcement_sha256: "1".repeat(64),
            }
        };
        self.stop = Some(exchange.subject.clone());
        Ok((
            StopResolution {
                source: SOURCE.into(),
                exchange: observed,
                scope,
            },
            FakeFence {
                fault: self.fault,
                observations: self.observations.clone(),
            },
        ))
    }

    fn settlement(
        &mut self,
        stop: &OfflineStopSubject,
        key: &TaskKey,
        phase: Phase,
        _: Instant,
    ) -> Result<ScopeWitness> {
        use OfflineAcceptedDisposition::Completed;
        use OfflineEntryFence::Enforced;
        if self.fault == Fault::DelayedStopAfterClientLoss && phase == Phase::Ingress {
            self.delayed_resolution
                .recv_timeout(Duration::from_secs(2))
                .unwrap();
        }
        let fact = match phase {
            Phase::Ingress => ScopeFact::Ingress(OfflineNewIngress {
                controller: Enforced,
                engine: Enforced,
                runtime: Enforced,
            }),
            Phase::OldTask => ScopeFact::OldTask(OfflineOldTask::Stopped(
                OfflineOldTaskTransitions::PinnedContainerd233RuncStopped,
            )),
            Phase::Accepted => ScopeFact::Accepted(OfflineClosedFrontier {
                create: Completed,
                start: if self.fault == Fault::PendingStart {
                    OfflineAcceptedDisposition::Pending
                } else {
                    Completed
                },
                stop: Completed,
                other_work: OfflineOtherWork::ExcludedByClosedAcceptanceCut,
                cleanup_tail: OfflineCleanupTail::CannotExecuteOrReopen,
            }),
            Phase::Recovery if self.fault == Fault::IncompleteTerminal => {
                return refuse("no scope-bound recovery witness");
            }
            Phase::Recovery => ScopeFact::Recovery(OfflineRecoveryAuthority {
                backing: OfflineRecoveryBacking::RetainedExecutionInert,
                controller_reconcile: Enforced,
                runtime_reconcile: Enforced,
            }),
        };
        let mut task = key.clone();
        if self.fault == Fault::CrossedTerminal && phase == Phase::Recovery {
            task.start_ticks += 1;
        }
        Ok(ScopeWitness {
            subject: stop.clone(),
            task,
            fact,
        })
    }
}

struct FakeAnchor {
    observations: Arc<Observations>,
    fault: Fault,
}
impl Anchor for FakeAnchor {
    fn check_current(&self, _: &Effective) -> io::Result<kernel::KernelCut> {
        let sample = kernel::KernelCut {
            cgroup_events: self.live_events()?,
            status: vec![],
            apparmor: vec![],
            procfs: None,
        };
        *self.observations.kernel_current_at.lock().unwrap() = Some(Instant::now());
        Ok(sample)
    }
    fn live_events(&self) -> io::Result<Vec<u8>> {
        if self.observations.stopped.load(Ordering::SeqCst) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(LIVE.into())
    }
    fn end_events(&self) -> io::Result<Vec<u8>> {
        if !self.observations.stopped.load(Ordering::SeqCst)
            || self.fault == Fault::MissingOriginalExit
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.observations
            .kernel_end_read
            .store(true, Ordering::SeqCst);
        Ok(if self.fault == Fault::Populated {
            LIVE
        } else {
            END
        }
        .into())
    }
}

fn accept(listener: &UnixListener) -> UnixStream {
    let mut p = libc::pollfd {
        fd: listener.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // Safety: p points to one valid pollfd for the still-open fake listener.
    assert_eq!(
        unsafe { libc::poll(&mut p, 1, 4500) },
        1,
        "expected exact fake request"
    );
    let (io, _) = listener.accept().unwrap();
    io.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    io.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
    io
}

// Expectations for this peer do not invoke the production encoder.
fn expect_wire(listener: &UnixListener, header: &[u8], body: &[u8]) -> UnixStream {
    let mut io = accept(listener);
    let mut captured = vec![0; header.len()];
    io.read_exact(&mut captured).unwrap();
    assert_eq!(captured, header);
    captured.resize(body.len(), 0);
    io.read_exact(&mut captured).unwrap();
    assert_eq!(captured, body);
    io
}

fn literal_attach() -> Vec<u8> {
    format!("POST /v1.44/containers/{ID}/attach?logs=0&stream=1&stdin=0&stdout=1&stderr=0 HTTP/1.1\r\nHost: docker\r\nConnection: Upgrade\r\nUpgrade: tcp\r\nContent-Length: 0\r\n\r\n").into_bytes()
}

fn literal_stop_target() -> String {
    format!("POST /v1.44/containers/{ID}/stop?signal=SIGTERM&t=2")
}

fn expect_no_bytes(listener: &UnixListener) {
    let mut io = accept(listener);
    let mut b = [0];
    assert_eq!(
        io.read(&mut b).unwrap(),
        0,
        "fence refused but request was sent"
    );
}

fn await_marker_in_archive(attempt: &Path) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let mut entries: Vec<_> = fs::read_dir(attempt)
            .unwrap()
            .map(|r| r.unwrap().path())
            .collect();
        entries.sort_unstable();
        let mut bytes = Vec::new();
        for f in entries {
            if let Ok(record) = serde_json::from_slice::<serde_json::Value>(&fs::read(f).unwrap())
                && record["kind"] == "Stdout"
                && let Ok(chunk) = serde_json::from_value::<Vec<u8>>(
                    record["data"]["event"]["Bytes"]["chunk"].clone(),
                )
            {
                bytes.extend(chunk);
            }
        }
        if bytes.ends_with(CHECKPOINT) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "no complete marker captured while reply was pending"
        );
        thread::park_timeout(Duration::from_millis(5));
    }
}

fn reply(mut io: UnixStream, status: &str, body: &[u8]) {
    io.write_all(
        format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .as_bytes(),
    )
    .unwrap();
    io.write_all(body).unwrap();
}

fn fake(
    listener: &UnixListener,
    fault: Fault,
    observations: Arc<Observations>,
    early_writer: Sender<UnixStream>,
    delayed_resolution: Sender<()>,
    attempt: &Path,
) -> Vec<Operation> {
    let trace = ready::Synthetic::new().create;
    let header = format!(
        "POST /v1.44/containers/create?platform=linux%2Famd64 HTTP/1.1\r\nHost: docker\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        trace.body.len()
    );
    let create = expect_wire(listener, header.as_bytes(), &trace.body);
    let mut mutations = vec![Operation::Create];
    if fault == Fault::LostCreate {
        // Partial positive response: must not guess/adopt the hinted ID.
        (&create)
            .write_all(b"HTTP/1.1 201 Created\r\nContent-Length: 300\r\n\r\n{\"Id\":\"dd")
            .unwrap();
        return mutations;
    }
    reply(create, "201 Created", &trace.reply);
    let mut inspect = expect_wire(listener,
        format!("GET /v1.44/containers/{ID}/json HTTP/1.1\r\nHost: docker\r\nConnection: close\r\nContent-Length: 0\r\n\r\n").as_bytes(), &[]);
    let mut data = CREATED_INSPECT.to_vec();
    if fault == Fault::CrossedCreate {
        let mut v: serde_json::Value = serde_json::from_slice(&data).unwrap();
        v["Id"] = json!("e".repeat(64));
        data = serde_json::to_vec(&v).unwrap();
    }
    // This is how the larger Moby JSON can be returned. Split chunk syntax
    // and payload across writes; retain the full exact decoded body.
    inspect
        .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n")
        .unwrap();
    for bytes in data.chunks(127) {
        inspect
            .write_all(format!("{:x}\r\n", bytes.len()).as_bytes())
            .unwrap();
        inspect.write_all(bytes).unwrap();
        inspect.write_all(b"\r\n").unwrap();
    }
    inspect.write_all(b"0\r\n\r\n").unwrap();
    drop(inspect);
    if fault == Fault::CrossedCreate {
        return mutations;
    }

    let mut stdout = expect_wire(listener, &literal_attach(), &[]);
    let frame = ready::mux(1, CHECKPOINT);
    if fault == Fault::Early {
        stdout
            .write_all(&[ready::UPGRADE, &frame].concat())
            .unwrap();
        return mutations;
    }
    stdout.write_all(ready::UPGRADE).unwrap();
    if fault == Fault::CrossedBeforeStart {
        expect_no_bytes(listener);
        return mutations;
    }
    if fault == Fault::AmbiguousBoundary {
        early_writer.send(stdout.try_clone().unwrap()).unwrap();
    }
    let start = expect_wire(listener,
        format!("POST /v1.44/containers/{ID}/start HTTP/1.1\r\nHost: docker\r\nConnection: close\r\nContent-Length: 0\r\n\r\n").as_bytes(), &[]);
    mutations.push(Operation::Start);
    if fault != Fault::AmbiguousBoundary {
        stdout.write_all(&frame[..6]).unwrap();
        stdout.write_all(&frame[6..]).unwrap();
    }
    await_marker_in_archive(attempt); // A blocked 204 cannot be used to delay collection.
    let mut start = start;
    if fault == Fault::LostStart {
        drop(start);
    } else {
        start.write_all(b"HTTP/1.1 204 No Content\r\n\r\n").unwrap();
    }
    // Keep the original stdout socket open until the controller closes it.
    let mut b = [0];
    assert_eq!(stdout.read(&mut b).unwrap(), 0);
    if held_coverage_fault(fault)
        || matches!(
            fault,
            Fault::CrossedStart
                | Fault::Crash
                | Fault::CrossedAttach
                | Fault::MissingPolicy
                | Fault::LostStart
                | Fault::AmbiguousBoundary
                | Fault::HeldCutLost
        )
    {
        return mutations;
    }
    if stop_resolution_fault(fault) {
        expect_no_bytes(listener);
        return mutations;
    }
    let target = literal_stop_target();
    let mut stop = expect_wire(
        listener,
        format!(
            "{target} HTTP/1.1\r\nHost: docker\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
        )
        .as_bytes(),
        &[],
    );
    mutations.push(Operation::Stop);
    if fault == Fault::DelayedStopAfterClientLoss {
        // Lose the reply and wait for actual client teardown, *without*
        // resolving yet. In the fixture, the ordinary mutable lookup would
        // now select a replacement. The runtime-owned scope excludes it.
        stop.shutdown(Shutdown::Write).unwrap();
        assert_eq!(stop.read(&mut b).unwrap(), 0);
        resolve_delayed_stop(&observations, &target);
        delayed_resolution.send(()).unwrap();
    } else {
        observations.stopped.store(true, Ordering::SeqCst);
        if fault != Fault::LostStop {
            stop.write_all(b"HTTP/1.1 204 No Content\r\n\r\n").unwrap();
        }
    }
    mutations
}

fn resolve_delayed_stop(observations: &Observations, actual_target: &str) {
    let enforced = observations.resolution_scope.lock().unwrap();
    let resolved = match enforced.as_ref() {
        Some(scope) if scope.request_target == actual_target => scope.task.container.as_str(),
        _ => "replacement",
    };
    assert_eq!(
        resolved, ID,
        "late resolution escaped the runtime restriction"
    );
    observations.stopped.store(true, Ordering::SeqCst);
    observations
        .handler_resolved_after_client_loss
        .store(true, Ordering::SeqCst);
}

fn private_root() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

const HIGH_ID: u64 = 1 << 53;

fn synthetic_endpoint(path: PathBuf) -> Endpoint {
    Endpoint {
        path,
        device: HIGH_ID,
        inode: HIGH_ID,
        listener_pid: 3,
        listener_uid: 4,
        listener_gid: 5,
    }
}

#[test]
fn intent_canonical_bytes_and_pin_preserve_all_filesystem_identity_fields() {
    use slice_campaign::codec::canonical_campaign;

    let root = Path::new(std::ffi::OsStr::from_bytes(b"/root-\xff"));
    let other_root = Path::new(std::ffi::OsStr::from_bytes(b"/root-\xfe"));
    let socket = Path::new(std::ffi::OsStr::from_bytes(b"/fake-\xff.sock"));
    let other_socket = Path::new(std::ffi::OsStr::from_bytes(b"/fake-\xfe.sock"));
    assert_eq!(root.to_string_lossy(), other_root.to_string_lossy());
    assert_eq!(socket.to_string_lossy(), other_socket.to_string_lossy());

    let endpoint = synthetic_endpoint(socket.to_owned());
    let preimage = |path, dev, ino, endpoint: &Endpoint| {
        intent_preimage(path, dev, ino, &endpoint.into(), SOURCE).unwrap()
    };
    let original = preimage(root, HIGH_ID, HIGH_ID, &endpoint);
    let base_bytes = canonical_campaign(&original).unwrap();
    let base_pin = campaign_digest(&original).unwrap();
    let alternatives = [
        preimage(root, HIGH_ID + 1, HIGH_ID, &endpoint),
        preimage(root, HIGH_ID, HIGH_ID + 1, &endpoint),
        preimage(
            root,
            HIGH_ID,
            HIGH_ID,
            &Endpoint {
                device: HIGH_ID + 1,
                ..endpoint.clone()
            },
        ),
        preimage(
            root,
            HIGH_ID,
            HIGH_ID,
            &Endpoint {
                inode: HIGH_ID + 1,
                ..endpoint.clone()
            },
        ),
        preimage(other_root, HIGH_ID, HIGH_ID, &endpoint),
        preimage(
            root,
            HIGH_ID,
            HIGH_ID,
            &synthetic_endpoint(other_socket.to_owned()),
        ),
    ];
    for (position, value) in alternatives.into_iter().enumerate() {
        assert_ne!(
            canonical_campaign(&value).unwrap(),
            base_bytes,
            "identity field {position}"
        );
        assert_ne!(
            campaign_digest(&value).unwrap(),
            base_pin,
            "identity field {position}"
        );
    }

    let descriptor = serde_json::to_vec(&EndpointIdentity::from(&endpoint)).unwrap();
    let decoded: EndpointIdentity = serde_json::from_slice(&descriptor).unwrap();
    let decoded_path = PathBuf::from(std::ffi::OsString::from_vec(decoded.path_bytes.clone()));
    assert_eq!(
        decoded_path.as_os_str().as_bytes(),
        socket.as_os_str().as_bytes()
    );
    assert_eq!(u64::from_be_bytes(decoded.device_be), HIGH_ID);
    assert_eq!(u64::from_be_bytes(decoded.inode_be), HIGH_ID);
    let rebuilt = intent_preimage(root, HIGH_ID, HIGH_ID, &decoded, SOURCE).unwrap();
    assert_eq!(canonical_campaign(&rebuilt).unwrap(), base_bytes);
    assert_eq!(campaign_digest(&rebuilt).unwrap(), base_pin);
}

#[test]
fn adjacent_endpoint_identity_refuses_before_live_preflight_and_spend() {
    let files = tempfile::tempdir().unwrap();
    let socket = files.path().join("fake.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let selected = synthetic_endpoint(socket);
    let crossed = Endpoint {
        inode: selected.inode + 1,
        ..selected.clone()
    };
    let root = private_root();
    let recipe = ready::recipe();
    let serving = ready::serving();
    let (_inputs, _config, app, binding) = ready::campaign_with_intent(
        &recipe,
        &serving,
        intent(root.path(), &selected, SOURCE).unwrap(),
    );
    let port = app.offline_qualification_port();
    let mut source = FakeSource {
        fault: Fault::None,
        serving,
        observations: Arc::new(Observations::default()),
        early_writer: mpsc::channel().1,
        delayed_resolution: mpsc::channel().1,
        stop: None,
    };
    let mut invoke = |endpoint| {
        run_at::<FakeAnchor>(
            &port,
            &binding,
            root.path(),
            endpoint,
            &recipe,
            &mut source,
            |_| panic!("identity-only fixture reached a task lookup"),
        )
    };
    assert!(matches!(
        invoke(&crossed),
        Err(Error::Refused(message)) if message == "private wire/source selection differs from Campaign"
    ));
    // Equal identity passes the intent check, but the synthetic coordinates
    // still do not describe the real fake listener. Live verification stays on.
    assert!(matches!(
        invoke(&selected),
        Err(Error::Io(error)) if error.to_string() == "Docker endpoint identity differs"
    ));
    assert!(fs::read_dir(root.path()).unwrap().next().is_none());
    assert!(
        !port
            .read_qualification_spend(&binding.history_key())
            .unwrap()
    );
    let db = rusqlite::Connection::open_with_flags(
        app.anchored_root().join("slice-campaign.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let attempts: u32 = db
        .query_row(
            "SELECT count(*) FROM b_lifecycle_attempt WHERE attempt_id = ?1",
            [binding.attempt_id()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(attempts, 0);
    listener.set_nonblocking(true).unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}

#[test]
fn cold_recovery_preserves_original_spend_but_does_not_read_crossed_prefix() {
    let files = private_root();
    let root = files
        .path()
        .join(std::ffi::OsStr::from_bytes(b"archive-\xff"));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let selected = synthetic_endpoint(root.join(std::ffi::OsStr::from_bytes(b"absent-\xfe.sock")));
    let crossed = Endpoint {
        device: selected.device + 1,
        ..selected.clone()
    };
    let (_inputs, config, app, binding) = ready::campaign_with_intent(
        &ready::recipe(),
        &ready::serving(),
        intent(&root, &selected, SOURCE).unwrap(),
    );
    let correct = RecoverySelection::new(&binding, &selected, SOURCE);
    let wrong = RecoverySelection::new(&binding, &crossed, SOURCE);
    let port = app.offline_qualification_port();
    let spent = port.enter(&binding).unwrap().consume().unwrap();
    let create = spent
        .mark_joined_create(
            &binding.create_request_digest().unwrap(),
            JoinedMarkReadback::Check,
        )
        .unwrap();
    let mut archive = Custody::begin(&root, binding.attempt_id()).unwrap();
    archive
        .record(
            Kind::Selection,
            json!({"history_key":correct.key, "recovery_selection":correct}),
        )
        .unwrap();
    archive
        .record(
            Kind::Armed(Operation::Create),
            json!({"mark":create.mark_digest().unwrap()}),
        )
        .unwrap();
    drop((create, archive, binding, port, app));

    let first: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join(&correct.key.attempt).join("001.json")).unwrap(),
    )
    .unwrap();
    let correct: RecoverySelection =
        serde_json::from_value(first["data"]["recovery_selection"].clone()).unwrap();
    let reopened = Campaign::open(config()).unwrap();
    let port = reopened.offline_qualification_port();
    assert!(!selected.path.exists());
    assert_eq!(
        correct.endpoint.path_bytes.as_slice(),
        selected.path.as_os_str().as_bytes()
    );
    let history = recover(&port, &correct, &root).unwrap();
    assert!(history.spent);
    assert_eq!(history.archive, ArchiveState::VerifiedPrefix);
    assert_eq!(history.possibly_submitted, CREATE);
    assert_eq!(
        history.disposition,
        CurrentDisposition::SpentCurrentDispositionUnknown
    );
    let wrong = serde_json::from_slice(&serde_json::to_vec(&wrong).unwrap()).unwrap();
    assert_eq!(
        recover(&port, &wrong, &root).unwrap(),
        History::unavailable(true)
    );
}

fn exercise(fault: Fault, expected_mutations: &[Operation], expected: Option<Verdict>) {
    let files = tempfile::tempdir().unwrap();
    let socket = files
        .path()
        .join(std::ffi::OsStr::from_bytes(b"fake-\xff.sock"));
    let listener = UnixListener::bind(&socket).unwrap();
    let m = fs::metadata(&socket).unwrap();
    let endpoint = Endpoint {
        path: socket,
        device: m.dev(),
        inode: m.ino(),
        listener_pid: std::process::id() as i32,
        // Safety: these getters have no preconditions.
        listener_uid: unsafe { libc::geteuid() },
        listener_gid: unsafe { libc::getegid() },
    };
    let recipe = ready::recipe();
    let serving = Serving {
        docker_socket_inode: endpoint.inode,
        ..ready::serving()
    };
    let root = private_root();
    let (_inputs, config, app, binding) = ready::campaign_with_intent(
        &recipe,
        &serving,
        intent(root.path(), &endpoint, SOURCE).unwrap(),
    );
    let path = root.path().join(binding.attempt_id());
    let peer_path = path.clone();
    let observations = Arc::new(Observations::default());
    let peer_observations = observations.clone();
    let (tx, rx) = mpsc::channel();
    let (resolved, delayed_resolution) = mpsc::channel();
    let server = thread::spawn(move || {
        let seen = fake(
            &listener,
            fault,
            peer_observations,
            tx,
            resolved,
            &peer_path,
        );
        (seen, listener)
    });
    let mut source = FakeSource {
        fault,
        serving,
        observations: observations.clone(),
        early_writer: rx,
        delayed_resolution,
        stop: None,
    };
    let port = app.offline_qualification_port();
    let result = catch_unwind(AssertUnwindSafe(|| {
        run_at(
            &port,
            &binding,
            root.path(),
            &endpoint,
            &recipe,
            &mut source,
            |_| {
                Ok(FakeAnchor {
                    observations: observations.clone(),
                    fault,
                })
            },
        )
    }));
    let (seen, listener) = server.join().unwrap();
    assert_eq!(seen, expected_mutations, "{fault:?}");
    match expected {
        Some(verdict) => assert_eq!(result.unwrap().unwrap(), verdict, "{fault:?}"),
        None if fault == Fault::Crash => assert!(result.is_err()),
        None => assert!(result.unwrap().is_err(), "{fault:?}"),
    }
    assert_eq!(
        observations
            .handler_resolved_after_client_loss
            .load(Ordering::SeqCst),
        fault == Fault::DelayedStopAfterClientLoss
    );
    check_cut_records(&path, fault, expected_mutations);
    let held_and_close = if held_coverage_fault(fault) {
        Some((0, 0))
    } else if stop_resolution_fault(fault) || expected_mutations == ALL {
        Some((1, 1))
    } else {
        None
    };
    if let Some(expected) = held_and_close {
        let db = rusqlite::Connection::open_with_flags(
            app.anchored_root().join("slice-campaign.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let actual: (u32, u32, u32) = db.query_row(
            "SELECT (SELECT count(*) FROM b_lifecycle_attempt WHERE attempt_id = ?1),\n                    (SELECT count(*) FROM b_lifecycle_fact WHERE attempt_id = ?1 AND slot = 4),\n                    (SELECT count(*) FROM b_lifecycle_mark WHERE attempt_id = ?1 AND kind = 3)",
            [binding.attempt_id()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();
        assert_eq!(
            actual,
            (1, expected.0, expected.1),
            "Campaign attempt/Held/Close {fault:?}"
        );
    }
    if fault == Fault::ContestedLater {
        let stop = source.stop.take().unwrap();
        let evidence = OfflineStopTerminalReadbacks {
            recovery: Some(OfflineStopReadback {
                subject: stop.clone(),
                position: fs::read_dir(&path).unwrap().count() as u64 + 1,
                observed: OfflineRecoveryAuthority {
                    backing: OfflineRecoveryBacking::Restorable,
                    controller_reconcile: OfflineEntryFence::Enforced,
                    runtime_reconcile: OfflineEntryFence::Enforced,
                },
            }),
            ..Default::default()
        };
        assert_eq!(
            port.record_offline_scoped_terminal(&binding, &stop, &evidence),
            Err(TerminalFailure::Contested)
        );
    }
    listener.set_nonblocking(true).unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    // No socket or original in-memory binding, anchor, or continuation is
    // available to recovery; only the protected selection record and DB.
    drop(listener);
    fs::remove_file(&endpoint.path).unwrap();
    drop(source);
    drop(endpoint);
    drop(binding);
    drop(port);
    drop(app);
    let raw = fs::read(path.join("001.json")).unwrap();
    let selection = serde_json::from_value::<RecoverySelection>(
        serde_json::from_slice::<serde_json::Value>(&raw).unwrap()["data"]["recovery_selection"]
            .clone(),
    )
    .unwrap();
    let reopened = Campaign::open(config()).unwrap();
    let record = recover(
        &reopened.offline_qualification_port(),
        &selection,
        root.path(),
    )
    .unwrap();
    assert!(record.spent);
    assert_eq!(record.archive, ArchiveState::VerifiedPrefix);
    assert_eq!(
        record.disposition,
        CurrentDisposition::SpentCurrentDispositionUnknown
    );
    assert_eq!(record.possibly_submitted, expected_mutations, "{fault:?}");
    assert_eq!(
        record.checked_scope,
        matches!(
            fault,
            Fault::None
                | Fault::LostStop
                | Fault::DelayedStopAfterClientLoss
                | Fault::ContestedLater
        )
    );
    assert_eq!(
        record.observed_create_id.as_deref(),
        (fault != Fault::LostCreate).then_some(ID)
    );
}

fn held_coverage_fault(fault: Fault) -> bool {
    matches!(
        fault,
        Fault::PolicyCoverageLost
            | Fault::PolicyCoverageStartedTooLate
            | Fault::PolicyWrongCapture
            | Fault::PolicyWrongPayload
            | Fault::PolicyWrongSource
    )
}

fn stop_resolution_fault(fault: Fault) -> bool {
    matches!(
        fault,
        Fault::CrossedBeforeStop
            | Fault::StopLocalOnly
            | Fault::StopWrongChannel
            | Fault::StopWrongTarget
            | Fault::StopWrongGeneration
    )
}

fn check_cut_records(path: &Path, fault: Fault, operations: &[Operation]) {
    let mut files: Vec<_> = fs::read_dir(path)
        .unwrap()
        .map(|r| r.unwrap().path())
        .collect();
    files.sort_unstable();
    let records: Vec<serde_json::Value> = files
        .iter()
        .map(|f| serde_json::from_slice(&fs::read(f).unwrap()).unwrap())
        .collect();
    let find = |kind: serde_json::Value| records.iter().find(|r| r["kind"] == kind);
    if held_coverage_fault(fault) {
        assert!(find(json!("HeldCapture")).is_some());
        assert!(find(json!("HeldCoverage")).is_none());
        assert!(!records.iter().any(|r| !r["data"]["event"]["Cut"].is_null()));
        assert!(find(json!({"Intended":"Stop"})).is_none());
    }
    let armed_stop = find(json!({"Armed":"Stop"}));
    assert_eq!(armed_stop.is_some(), operations.contains(&Operation::Stop));
    if let Some(armed) = armed_stop {
        let protected = find(json!("StopResolution")).unwrap();
        assert_eq!(
            protected["position"].as_u64().unwrap() + 1,
            armed["position"].as_u64().unwrap()
        );
        assert_eq!(
            protected["data"]["exchange"]["request_target"],
            literal_stop_target()
        );
        assert_eq!(
            protected["data"]["exchange"]["subject"]["close_mark"],
            armed["data"]["mark"]
        );
        assert_eq!(protected["data"]["source"], SOURCE);
    } else {
        assert!(find(json!("StopResolution")).is_none());
    }
    if let Some(proof) = find(json!("HeldCoverage")) {
        let capture = find(json!("HeldCapture")).unwrap();
        let at_ns = capture["data"]["at_ns"].as_u64().unwrap();
        let cut = records
            .iter()
            .find(|r| !r["data"]["event"]["Cut"].is_null())
            .unwrap();
        let applied_position = capture["data"]["applied"]["position"].as_u64().unwrap();
        let kernel_position = capture["data"]["kernel_position"].as_u64().unwrap();
        assert!(applied_position < kernel_position);
        assert!(kernel_position < capture["position"].as_u64().unwrap());
        let kernel = &records[(kernel_position - 1) as usize];
        assert_eq!(kernel["kind"], "Applied");
        assert_eq!(kernel["data"].as_object().unwrap().len(), 1);
        assert!(kernel["data"]["kernel_current"].is_object());
        let payload = &records[(applied_position - 1) as usize]["data"];
        assert_eq!(
            capture["data"]["applied"]["sha256"],
            crate::digest(&serde_json::to_vec(payload).unwrap())
        );
        assert_eq!(proof["data"]["applied"], capture["data"]["applied"]);
        assert_eq!(proof["data"]["capture_position"], capture["position"]);
        assert!(proof["data"]["covered_from_ns"].as_u64().unwrap() <= at_ns);
        assert!(proof["data"]["covered_through_ns"].as_u64().unwrap() >= at_ns);
        assert_eq!(cut["data"]["at_ns"], at_ns);
        assert_eq!(
            cut["data"]["event"]["Cut"]["effective"],
            payload["effective"]
        );
        assert!(capture["position"].as_u64().unwrap() < proof["position"].as_u64().unwrap());
        assert!(proof["position"].as_u64().unwrap() < cut["position"].as_u64().unwrap());
    }
}

const ALL: &[Operation] = &[Operation::Create, Operation::Start, Operation::Stop];
const CREATE: &[Operation] = &[Operation::Create];
const CREATE_START: &[Operation] = &[Operation::Create, Operation::Start];
fn verdict(held: bool, terminal: bool, qualified: bool) -> Option<Verdict> {
    Some(Verdict {
        held,
        terminal,
        qualified,
    })
}

#[test]
fn captured_docker_wire_joins_same_init_with_or_without_stop_reply() {
    for f in [Fault::None, Fault::LostStop] {
        exercise(f, ALL, verdict(true, true, true));
    }
}

#[test]
fn crossed_inspect_start_attach_and_terminal_do_not_qualify() {
    exercise(Fault::CrossedCreate, CREATE, None);
    exercise(Fault::CrossedStart, CREATE_START, None);
    exercise(
        Fault::CrossedAttach,
        CREATE_START,
        verdict(false, false, false),
    );
    exercise(Fault::CrossedTerminal, ALL, verdict(true, false, false));
}

#[test]
fn early_and_ambiguous_stdout_lost_cut_and_missing_policy_cannot_authorize_stop() {
    exercise(Fault::Early, CREATE, None);
    for fault in [
        Fault::AmbiguousBoundary,
        Fault::HeldCutLost,
        Fault::MissingPolicy,
    ] {
        exercise(fault, CREATE_START, verdict(false, false, false));
    }
}

#[test]
fn applied_coverage_must_join_the_actual_cut_source_and_full_effective_payload() {
    for fault in [
        Fault::PolicyCoverageLost,
        Fault::PolicyCoverageStartedTooLate,
        Fault::PolicyWrongCapture,
        Fault::PolicyWrongPayload,
        Fault::PolicyWrongSource,
    ] {
        exercise(fault, CREATE_START, verdict(false, false, false));
    }
}

#[test]
fn stop_requires_exact_scope_surviving_client_loss_at_handler_resolution() {
    for fault in [
        Fault::StopLocalOnly,
        Fault::StopWrongChannel,
        Fault::StopWrongTarget,
        Fault::StopWrongGeneration,
    ] {
        exercise(fault, CREATE_START, None);
    }
    exercise(
        Fault::DelayedStopAfterClientLoss,
        ALL,
        verdict(true, true, true),
    );
}

#[test]
fn changed_object_resolution_is_refused_before_each_effect() {
    exercise(Fault::CrossedBeforeStart, CREATE, None);
    exercise(Fault::CrossedBeforeStop, CREATE_START, None);
}

#[test]
fn lost_create_and_start_never_replay_and_pending_start_prevents_terminal() {
    exercise(Fault::LostCreate, CREATE, None);
    exercise(Fault::LostStart, CREATE_START, verdict(false, false, false));
    exercise(Fault::PendingStart, ALL, verdict(true, false, false));
}

#[test]
fn stop_ack_never_substitutes_for_recovery_original_exit_or_empty_subtree() {
    for f in [
        Fault::IncompleteTerminal,
        Fault::Populated,
        Fault::MissingOriginalExit,
        Fault::TerminalCutLost,
    ] {
        exercise(f, ALL, verdict(true, false, false));
    }
}

#[test]
fn crash_after_start_recovers_spent_unresolved_and_cannot_send() {
    exercise(Fault::Crash, CREATE_START, None);
}

#[test]
fn reopened_archive_does_not_mask_a_later_owner_contest() {
    exercise(Fault::ContestedLater, ALL, verdict(true, true, true));
}

#[test]
fn protected_archive_corroborates_spend_even_if_spend_observation_was_not_written() {
    let root = private_root();
    let endpoint = Endpoint {
        path: root.path().join("never-opened.sock"),
        device: 1,
        inode: 2,
        listener_pid: 3,
        listener_uid: 4,
        listener_gid: 5,
    };
    let (_inputs, config, app, binding) = ready::campaign_with_intent(
        &ready::recipe(),
        &ready::serving(),
        intent(root.path(), &endpoint, SOURCE).unwrap(),
    );
    let selection = RecoverySelection::new(&binding, &endpoint, SOURCE);
    let mut archive = Custody::begin(root.path(), binding.attempt_id()).unwrap();
    archive
        .record(Kind::Selection, json!({"history_key":selection.key}))
        .unwrap();
    let port = app.offline_qualification_port();
    assert!(!recover(&port, &selection, root.path()).unwrap().spent);
    let spent = port.enter(&binding).unwrap().consume().unwrap();
    let create = spent
        .mark_joined_create(
            &binding.create_request_digest().unwrap(),
            JoinedMarkReadback::Check,
        )
        .unwrap();
    archive
        .record(
            Kind::Armed(Operation::Create),
            json!({"mark":create.mark_digest().unwrap()}),
        )
        .unwrap();
    // Complete 201 durability, cut before the separate CreateIdentity record.
    archive
        .record(
            Kind::Reply(Operation::Create),
            json!({"response":{
                "status":201,"body":format!("{{\"Id\":\"{ID}\",\"Warnings\":[]}}").into_bytes()
            }}),
        )
        .unwrap();
    let path = root.path().join(&selection.key.attempt);
    drop(create);
    drop(binding);
    drop(archive);
    drop(port);
    drop(app);
    let reopen = Campaign::open(config()).unwrap();
    let port = reopen.offline_qualification_port();
    let history = recover(&port, &selection, root.path()).unwrap();
    assert!(history.spent);
    assert_eq!(history.possibly_submitted, CREATE);
    assert_eq!(history.observed_create_id.as_deref(), Some(ID));
    assert!(!history.checked_create);

    let wrong = private_root();
    fs::create_dir(wrong.path().join(&selection.key.attempt)).unwrap();
    fs::set_permissions(
        wrong.path().join(&selection.key.attempt),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    fs::copy(
        path.join("001.json"),
        wrong.path().join(&selection.key.attempt).join("001.json"),
    )
    .unwrap();
    let misplaced = recover(&port, &selection, wrong.path()).unwrap();
    assert!(misplaced.spent);
    assert_eq!(misplaced.archive, ArchiveState::UnavailableOrDamaged);
    assert_eq!(
        misplaced.disposition,
        CurrentDisposition::SpentCurrentDispositionUnknown
    );

    let file = path.join("004.json");
    fs::write(&file, b"torn").unwrap();
    let torn = recover(&port, &selection, root.path()).unwrap();
    assert_eq!(torn.archive, ArchiveState::UnavailableOrDamaged);
    assert_eq!(torn.observed_create_id.as_deref(), Some(ID));
    assert_eq!(torn.possibly_submitted, CREATE);
    assert_eq!(
        torn.disposition,
        CurrentDisposition::SpentCurrentDispositionUnknown
    );

    for i in 2..=4 {
        fs::remove_file(path.join(format!("{i:03}.json"))).unwrap();
    }
    let prefix = recover(&port, &selection, root.path()).unwrap();
    assert_eq!(prefix.archive, ArchiveState::VerifiedPrefix);
    assert!(prefix.spent && prefix.possibly_submitted.is_empty());
    assert_eq!(
        prefix.disposition,
        CurrentDisposition::SpentCurrentDispositionUnknown
    );

    let first = path.join("001.json");
    fs::set_permissions(&first, fs::Permissions::from_mode(0o644)).unwrap();
    let unprotected = recover(&port, &selection, root.path()).unwrap();
    assert!(unprotected.spent);
    assert_eq!(unprotected.archive, ArchiveState::UnavailableOrDamaged);

    let mut wrong_key = selection.key;
    wrong_key.original_opening.push('x');
    assert!(port.read_qualification_spend(&wrong_key).is_err());
}
