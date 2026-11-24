use super::*;

#[path = "tests/join.rs"]
mod join;

use artifact_contract::{CHECKPOINT, ENTRYPOINT, GROUP, PRIVATE_UMASK, USER};
use slice_campaign::offline_qualification::{
    FixtureCreateReadback, FixtureObject, JoinedMarkReadback, SelectionPins,
};
use slice_campaign::{
    AcceptedBoundary, AdmitRequest, Baseline, Campaign, CampaignIdentity, Effect, TrustedConfig,
};

pub(super) const UPGRADE: &[u8] = b"HTTP/1.1 101 UPGRADED\r\nContent-Type: application/vnd.docker.multiplexed-stream\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n";
fn hash(c: char) -> String {
    c.to_string().repeat(64)
}
fn pin(path: &str, c: char) -> FilePin {
    FilePin {
        path: path.into(),
        sha256: hash(c),
    }
}

pub(super) fn serving() -> Serving {
    Serving {
        boot: [1; 16],
        docker_socket_inode: 2,
        daemon_process_start_ticks: 3,
        daemon_artifact_sha256: hash('1'),
        moby_semantics_revision: MOBY.into(),
        runtime_generation: hash('2'),
        api: API.into(),
    }
}

pub(super) fn recipe() -> Recipe {
    Recipe {
        startup: Startup {
            store: Store::ContainerdLocalTarget,
            image_id: format!("sha256:{}", hash('3')),
            config_sha256: hash('4'),
            manifest_sha256: hash('3'),
            init: pin(ENTRYPOINT, '5'),
            elf: ElfClosure::Dynamic {
                interpreter: pin("/lib64/ld-linux-x86-64.so.2", '6'),
                dependencies: vec![pin("/lib/x86_64-linux-gnu/libc.so.6", '7')],
            },
        },
        runtime: "runc".into(),
        seccomp_sha256: hash('8'),
        apparmor_profile: "docker-default".into(),
        apparmor_policy_sha256: hash('9'),
    }
}

pub(super) fn task(serving: &Serving, object: &str) -> TaskKey {
    TaskKey {
        serving: serving.clone(),
        container: object.into(),
        task_id: object.into(),
        pid: 7183,
        start_ticks: 3000,
        pid_namespace: 4026532200,
        exec_generation: hash('b'),
        runtime_namespace: "moby".into(),
        subtree_capture: hash('4'),
        subtree_mount_id: 36,
        subtree_inode: 8295,
    }
}

fn bytes(connection: u64, value: &[u8]) -> Event {
    Event::Bytes {
        connection,
        chunk: value.into(),
    }
}
pub(super) fn mux(channel: u8, payload: &[u8]) -> Vec<u8> {
    [
        vec![channel, 0, 0, 0],
        (payload.len() as u32).to_be_bytes().to_vec(),
        payload.to_vec(),
    ]
    .concat()
}

pub(super) struct Synthetic {
    serving: Serving,
    recipe: Recipe,
    start: JoinedStartRequest,
    owner: FixtureTerminalSubject,
    pub(super) create: CreateTrace,
    samples: Vec<Sample>,
    through: u64,
}

impl Synthetic {
    fn with(
        recipe: Recipe,
        serving: Serving,
        start: JoinedStartRequest,
        owner: FixtureTerminalSubject,
    ) -> Self {
        let object = &start.target.full_id;
        let body = recipe.create_body().unwrap();
        let mut config: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let host = config
            .as_object_mut()
            .unwrap()
            .remove("HostConfig")
            .unwrap();
        let create = CreateTrace {
            serving: serving.clone(), campaign_request: start.create_request_digest.clone(),
            target: format!("POST /{API}/containers/create?platform=linux%2Famd64"), body,
            status: 201, reply: serde_json::to_vec(&json!({"Id":object,"Warnings":[]})).unwrap(),
            inspect: serde_json::to_vec(&json!({
                "Id": object, "Image": recipe.startup.image_id, "Path": ENTRYPOINT, "Args":[],
                "Config":config, "HostConfig":host, "Mounts":[], "State":{"Status":"created","Running":false,"Pid":0}
            })).unwrap(),
        };
        let rpc = StartRpc {
            serving: serving.clone(),
            full_id: object.clone(),
            links: OperationLinks {
                selection: owner.selection.clone(),
                create_request: start.create_request_digest.clone(),
                create_mark: start.create_mark_digest.clone(),
                start_request: start.request_digest.clone(),
                start_mark: owner.start_mark.clone(),
            },
            exchange: 23,
            request_target: format!("POST /{API}/containers/{object}/start"),
        };
        let task = task(&serving, object);
        let launch = ResolvedLaunch {
            task: task.clone(),
            runtime: recipe.runtime.clone(),
            platform: "linux/amd64".into(),
            argv: vec![ENTRYPOINT.into()],
            cwd: "/".into(),
            env: vec![
                "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
                format!("HOSTNAME={}", &object[..12]),
            ],
            image: recipe.startup.clone(),
            root_mount_id: 17,
            io_pipe_inode: 587,
            startup_backing: StartupBacking::ImmutableImageAndDependencies,
            stdout_route: StdoutRoute::ExclusiveSelectedTaskLivePipe,
        };
        let effective = Effective {
            task: task.clone(),
            uid: [USER; 4],
            gid: [GROUP; 4],
            groups: vec![GROUP],
            caps: [0; 5],
            nnp: true,
            umask: PRIVATE_UMASK,
            seccomp_filters: 1,
            applied_seccomp_sha256: recipe.seccomp_sha256.clone(),
            apparmor_enforcing: true,
            applied_apparmor_profile: recipe.apparmor_profile.clone(),
            applied_apparmor_sha256: recipe.apparmor_policy_sha256.clone(),
            root_mount_id: 17,
            read_only_root: true,
            namespaces: Namespaces {
                task: [task.pid_namespace, 12, 13, 14, 15, 16],
                host: [1, 2, 3, 4, 5, 6],
            },
            inventory: Some(Inventory {
                mounts: ["/dev", "/proc", "/sys", "/sys/fs/cgroup"]
                    .into_iter()
                    .map(|p| MountView {
                        path: p.into(),
                        kind: MountKind::PrivateVirtual,
                        read_only: p.starts_with("/sys"),
                    })
                    .collect(),
                devices: vec![(1, 3), (1, 5)],
                interfaces: vec!["lo".into()],
                non_loopback_route_count: 0,
                externally_reachable_control_handles: 0,
                writable_cgroup_control: false,
            }),
            fds: vec![
                Descriptor::NullInput,
                Descriptor::StdoutPipe(587),
                Descriptor::NonInputStderr,
            ],
        };
        // Upgrade split mid-header. A single app record is split across two
        // mux frames, then split again at a partial mux header and payload.
        let wire = [mux(1, &CHECKPOINT[..8]), mux(1, &CHECKPOINT[8..])].concat();
        let events = [
            Event::Open(Attach {
                serving: serving.clone(),
                connection: 7,
                resolved_full_id: object.clone(),
                http_request: attach_request(&serving, object),
                registration: Registration::AuthenticatedPinnedSameObject,
            }),
            bytes(7, &UPGRADE[..13]),
            bytes(7, &UPGRADE[13..]),
            Event::Issued(rpc.clone()),
            bytes(7, &wire[..3]),
            bytes(7, &wire[3..10]),
            bytes(7, &wire[10..]),
            Event::Reply {
                rpc,
                status: 204,
                body: vec![],
            },
            Event::Selected(Box::new(task.clone())),
            Event::Cut(Box::new(CurrentCut {
                launch,
                effective,
                anchors: Anchors::SameExecutionLiveAndRetained,
                drain: Drain::UninterruptedAtCut,
            })),
        ];
        let samples: Vec<_> = events
            .into_iter()
            .enumerate()
            .map(|(i, event)| Sample {
                at_ns: (i as u64 + 1) * 1_000_000,
                event,
            })
            .collect();
        let through = samples.last().unwrap().at_ns;
        Self {
            serving,
            recipe,
            start,
            owner,
            create,
            samples,
            through,
        }
    }

    pub(super) fn new() -> Self {
        let serving = serving();
        let start = JoinedStartRequest {
            create_request_digest: hash('a'),
            create_mark_digest: hash('b'),
            request_digest: hash('c'),
            target: FixtureObject {
                full_id: hash('d'),
                serving_generation: serving.digest().unwrap(),
            },
        };
        let running = task(&serving, &start.target.full_id)
            .fixture_running(&start)
            .unwrap();
        let owner = FixtureTerminalSubject {
            selection: hash('e'),
            create_mark: start.create_mark_digest.clone(),
            start_mark: hash('f'),
            frontier: hash('0'),
            running,
            boot: hash('1'),
            namespace: hash('2'),
            subtree: hash('3'),
        };
        Self::with(recipe(), serving, start, owner)
    }
    fn input(&self) -> FixtureInput<'_> {
        FixtureInput {
            recipe: &self.recipe,
            serving: &self.serving,
            create: &self.create,
            start: &self.start,
            samples: &self.samples,
            observed_through_ns: self.through,
        }
    }
    fn check(&self) -> Result<HeldHistory, Refusal> {
        capture(
            self.input(),
            &self.owner.selection,
            &self.recipe.digest().unwrap(),
            &self.owner,
        )
    }
    pub(super) fn cut(&mut self) -> &mut CurrentCut {
        let Event::Cut(cut) = &mut self.samples.last_mut().unwrap().event else {
            panic!("cut")
        };
        cut
    }
    fn append_before_cut(&mut self, event: Event) {
        let i = self.samples.len() - 1;
        self.samples.insert(
            i,
            Sample {
                at_ns: self.through,
                event,
            },
        );
    }
}

#[test]
fn qualified_early_to_reply_fragmented_stdout_keeps_exact_historical_key() {
    let x = Synthetic::new();
    let checked = x.check().unwrap();
    assert_eq!(checked.links().create_mark, x.owner.create_mark);
    assert_eq!(checked.links().start_request, x.start.request_digest);
    assert_eq!(checked.links().start_mark, x.owner.start_mark);
    assert_eq!(
        checked.execution().fixture_running(&x.start).unwrap(),
        x.owner.running
    );
    assert_eq!(checked.observed_at_ns(), x.through);
    assert_eq!(CHECKPOINT, b"work-engine.b.init-only-held.v1\n");
    assert!(artifact_contract::reduced_groups(&[]));
    assert!(artifact_contract::reduced_groups(&[GROUP]));
    assert!(!artifact_contract::reduced_groups(&[GROUP, 0]));
}

#[test]
fn prearm_requires_same_protocol_peer_exact_object_and_registered_before_start() {
    for change in 0..5 {
        let mut x = Synthetic::new();
        let Event::Open(open) = &mut x.samples[0].event else {
            panic!("open")
        };
        match change {
            0 => open.registration = Registration::UnboundOrDifferent,
            1 => open.serving.daemon_process_start_ticks += 1,
            2 => open.resolved_full_id = hash('0'),
            3 => {
                open.http_request = String::from_utf8_lossy(&open.http_request)
                    .replace("stdin=0", "stdin=1")
                    .into_bytes()
            }
            _ => {
                open.http_request = String::from_utf8_lossy(&open.http_request)
                    .replace("logs=0", "logs=1")
                    .into_bytes()
            }
        }
        assert_eq!(x.check(), Err(Refusal::Prearm));
    }
    let mut x = Synthetic::new();
    x.samples.remove(2);
    assert_eq!(x.check(), Err(Refusal::Start));
    let mut x = Synthetic::new();
    x.samples[2].event = bytes(7, &[&UPGRADE[13..], &mux(1, CHECKPOINT)].concat());
    assert_eq!(x.check(), Err(Refusal::EarlyStream));
}

#[test]
fn exact_upgrade_and_ordered_stream_refuse_contamination_even_after_valid_marker() {
    for replacement in [
        "HTTP/1.1 200 OK",
        "HTTP/1.1 101 Switching Protocols",
        "HTTP/1.1 101 UPGRADED\r\nUpgrade: tcp",
        "HTTP/1.1 101 UPGRADED\r\nTransfer-Encoding: chunked",
    ] {
        let mut x = Synthetic::new();
        x.samples.remove(1);
        let header = String::from_utf8_lossy(UPGRADE).replace("HTTP/1.1 101 UPGRADED", replacement);
        x.samples[1].event = bytes(7, header.as_bytes());
        assert_eq!(x.check(), Err(Refusal::Upgrade));
    }
    let mut x = Synthetic::new();
    x.samples[4].event = bytes(11, &mux(1, CHECKPOINT));
    assert_eq!(x.check(), Err(Refusal::CrossedStream));
    for bad in [mux(1, CHECKPOINT), mux(2, b"stderr"), b"\n".to_vec()] {
        let mut x = Synthetic::new();
        x.append_before_cut(bytes(7, &bad));
        assert_eq!(x.check(), Err(Refusal::ContaminatedStream));
    }
    for wrong in [
        mux(2, CHECKPOINT),
        mux(3, CHECKPOINT),
        mux(1, b"not the record"),
        vec![1, 0, 0, 0, 255, 255, 255, 255],
    ] {
        let mut x = Synthetic::new();
        x.samples[4].event = bytes(7, &wrong);
        assert_eq!(x.check(), Err(Refusal::ContaminatedStream));
    }
    let mut x = Synthetic::new();
    x.samples.remove(6);
    assert_eq!(x.check(), Err(Refusal::StreamUnavailable));
    let mut x = Synthetic::new();
    x.append_before_cut(Event::EofResetGapOrBackpressure);
    assert_eq!(x.check(), Err(Refusal::StreamUnavailable));
    let mut x = Synthetic::new();
    x.cut().drain = Drain::Unestablished;
    assert_eq!(x.check(), Err(Refusal::StreamUnavailable));
    let mut x = Synthetic::new();
    x.append_before_cut(bytes(7, &[0; MAX_STREAM_BYTES]));
    assert_eq!(x.check(), Err(Refusal::StreamUnavailable));
}

#[test]
fn create_recipe_full_id_and_start_rpc_are_not_rebound_by_marker() {
    let mut x = Synthetic::new();
    x.create.reply = serde_json::to_vec(&json!({"Id":"dddddddddddd"})).unwrap();
    assert_eq!(x.check(), Err(Refusal::Create));
    let mut x = Synthetic::new();
    x.create.body.push(b' ');
    assert_eq!(x.check(), Err(Refusal::Create));
    let mut x = Synthetic::new();
    let mut inspect: serde_json::Value = serde_json::from_slice(&x.create.inspect).unwrap();
    inspect["Config"]["Env"] = json!(["LD_PRELOAD=/hostile.so"]);
    x.create.inspect = serde_json::to_vec(&inspect).unwrap();
    assert_eq!(x.check(), Err(Refusal::Create));
    let mut x = Synthetic::new();
    let Event::Issued(rpc) = &mut x.samples[3].event else {
        panic!("issue")
    };
    rpc.links.start_mark = hash('5');
    assert_eq!(x.check(), Err(Refusal::Start));
    let mut x = Synthetic::new();
    let Event::Reply { rpc, .. } = &mut x.samples[7].event else {
        panic!("reply")
    };
    rpc.exchange += 1;
    assert_eq!(x.check(), Err(Refusal::Start));
}

#[test]
fn matching_inspect_and_marker_do_not_establish_lifetime_immutable_launch_or_policy() {
    for change in 0..6 {
        let mut x = Synthetic::new();
        let c = x.cut();
        match change {
            0 => c.launch.task.start_ticks += 1,
            1 => c.launch.task.exec_generation = hash('8'),
            2 => c.launch.task.serving.runtime_generation = hash('8'),
            3 => c.launch.image.elf = ElfClosure::Static,
            4 => c.launch.startup_backing = StartupBacking::OverrideOrUnknown,
            _ => c.launch.stdout_route = StdoutRoute::AlternateOrUnknown,
        }
        assert_eq!(x.check(), Err(Refusal::Task));
    }
    for change in 0..9 {
        let mut x = Synthetic::new();
        let e = &mut x.cut().effective;
        match change {
            0 => e.task.subtree_inode += 1,
            1 => e.nnp = false,
            2 => e.seccomp_filters = 0,
            3 => e.applied_seccomp_sha256.clear(),
            4 => e.namespaces.task[4] = e.namespaces.host[4],
            5 => e.inventory = None,
            6 => e.inventory.as_mut().unwrap().writable_cgroup_control = true,
            7 => e.fds[1] = Descriptor::StdoutPipe(9),
            _ => e.apparmor_enforcing = false,
        }
        assert_eq!(x.check(), Err(Refusal::Effective));
    }
}

#[test]
fn cut_is_current_live_same_anchor_and_not_cancelled() {
    let mut x = Synthetic::new();
    x.append_before_cut(Event::CancelledOrRevoked);
    assert_eq!(x.check(), Err(Refusal::NotCurrent));
    let mut x = Synthetic::new();
    x.through += 1;
    assert_eq!(x.check(), Err(Refusal::NotCurrent));
    let mut x = Synthetic::new();
    x.through = x.samples[3].at_ns + HOLD_SECONDS * NS_PER_SECOND;
    x.samples.last_mut().unwrap().at_ns = x.through;
    assert_eq!(x.check(), Err(Refusal::NotCurrent));
    let mut x = Synthetic::new();
    x.cut().anchors = Anchors::GoneChangedOrUnestablished;
    assert_eq!(x.check(), Err(Refusal::NotCurrent));
    let mut x = Synthetic::new();
    let Event::Selected(task) = &mut x.samples[8].event else {
        panic!("selected")
    };
    task.subtree_inode += 1;
    assert_eq!(x.check(), Err(Refusal::Task));

    let mut x = Synthetic::new();
    let Event::Selected(task) = &mut x.samples[8].event else {
        panic!("selected")
    };
    // Same mount and inode, but not the retained original slot.
    task.subtree_capture = hash('5');
    assert_eq!(x.check(), Err(Refusal::Task));
}

fn campaign(
    recipe: &Recipe,
    serving: &Serving,
) -> (
    tempfile::TempDir,
    impl Fn() -> TrustedConfig + use<>,
    Campaign,
    AttemptBinding,
) {
    campaign_with_intent(recipe, serving, hash('b'))
}

pub(super) fn campaign_with_intent(
    recipe: &Recipe,
    serving: &Serving,
    intent_sha256: String,
) -> (
    tempfile::TempDir,
    impl Fn() -> TrustedConfig + use<>,
    Campaign,
    AttemptBinding,
) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let repository = root.join("repo");
    let workspace = root.join("workspace");
    std::fs::create_dir(&repository).unwrap();
    std::fs::create_dir(&workspace).unwrap();
    let identity = CampaignIdentity {
        run_id: "sc0-oracle-c2".into(),
        slice_number: 1,
        attempt_id: "offline-ready-capture".into(),
        plan_version: "sc0-controlled-v1".into(),
    };
    let accepted_boundary = AcceptedBoundary {
        reference: "offline Ready capture fixture".into(),
        sha256: "f45b94d3ef985a446002e21a795a51004a5c732e5b150208be09f964152662ef".into(),
    };
    let baseline = Baseline {
        accepted_commit: "a8e074d765d94a7e816b7ab49db428fd4a760a9f".into(),
        accepted_tree: "17868afbf46ea53715981fce83171001c632b957".into(),
        inter_slice_commit: "a8e074d765d94a7e816b7ab49db428fd4a760a9f".into(),
    };
    macro_rules! fixture {
        ($name:literal) => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../slice-campaign/tests/fixtures/vectors/",
                $name,
                ".json"
            ))
            .to_vec()
        };
    }
    let (root, repo, work, id, accepted, base) = (
        root.join("authority"),
        repository.clone(),
        workspace.clone(),
        identity.clone(),
        accepted_boundary.clone(),
        baseline.clone(),
    );
    let config = move || {
        TrustedConfig::controlled_sc0(
            root.clone(),
            repo.clone(),
            work.clone(),
            id.clone(),
            accepted.clone(),
            base.clone(),
            "offline-fixture-owner".into(),
            fixture!("f52792e22d2a1b73e007bc7078607d6657989248a35de1371a1180d23101d206"),
            fixture!("d155cc2ac1cf1ab26b0e186d6a6ac86f547b1a0b9c3ce74f0ba69eecd37a2f34"),
            fixture!("c927b51b887e37f6fdc02e1fabfadddd211d0ac30712cbc6c5028993ad3889c2"),
        )
        .unwrap()
    };
    let mut app = Campaign::initialize(config()).unwrap();
    let Effect::Applied { value, .. } = app
        .admit(
            "offline-ready-campaign",
            AdmitRequest {
                identity: identity.clone(),
                repository,
                workspace,
                accepted_boundary,
                baseline,
                expected_impact: None,
            },
        )
        .unwrap()
    else {
        panic!("admit")
    };
    app.attach_joined_offline_qualification_fixture().unwrap();
    let binding = app
        .select_offline_qualification(
            &identity,
            &value.revision,
            SelectionPins {
                grant_id: hash('a'),
                intent_sha256,
                recipe_sha256: recipe.digest().unwrap(),
                serving_sha256: serving.digest().unwrap(),
                expires_at_unix_ms: 4_102_444_800_000,
            },
        )
        .unwrap();
    (temp, config, app, binding)
}

fn running_capture(
    port: &QualificationPort,
    binding: &AttemptBinding,
) -> (Synthetic, ConfirmedRunning) {
    let recipe = recipe();
    let serving = serving();
    let spent = port.enter(binding).unwrap().consume().unwrap();
    let create = binding.create_request_digest().unwrap();
    let object = FixtureObject {
        full_id: hash('c'),
        serving_generation: serving.digest().unwrap(),
    };
    let marked = spent
        .mark_joined_create(&create, JoinedMarkReadback::Check)
        .unwrap();
    let fresh = marked
        .confirm_created(
            &create,
            &object.full_id,
            FixtureCreateReadback {
                request_digest: create.clone(),
                object: object.clone(),
            },
        )
        .unwrap();
    let start = fresh.request().clone();
    let running = task(&serving, &object.full_id)
        .fixture_running(&start)
        .unwrap();
    let confirmation = fresh
        .mark(JoinedMarkReadback::Check)
        .unwrap()
        .confirm_running(running.clone())
        .unwrap();
    let owner = port.fixture_terminal_subject(binding, &running).unwrap();
    (Synthetic::with(recipe, serving, start, owner), confirmation)
}

#[test]
fn capture_joins_existing_durable_create_start_and_owner_current_held() {
    for revoke in [false, true] {
        let (_temp, _, app, binding) = campaign(&recipe(), &serving());
        let port = app.offline_qualification_port();
        let (fixture, confirmation) = running_capture(&port, &binding);
        let running = &fixture.owner.running;
        if revoke {
            app.revoke_offline_qualification(&binding).unwrap();
        }
        let result = verify_fixture(&port, &binding, running, confirmation, fixture.input());
        if revoke {
            assert_eq!(result, Err(Refusal::Owner(HoldFailure::NotCurrent)));
        } else {
            assert_eq!(result.unwrap().links().selection, binding.binding_digest());
        }
        assert!(port.enter(&binding).is_err());
    }
}
