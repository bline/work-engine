//! Private natural-exit continuation of the existing V4 controller. There
//! is no default selector and no normal signal or Stop/Close on this path.

use super::super::{
    Attach, CreateTrace, HeldHistory, Inventory, JoinRefusal, MountKind, Namespaces,
    StartupBacking, StdoutRoute, fixture_held_projection, same_held_execution,
};
use super::{
    Anchor, AppliedCoverage, AppliedWitness, AttemptBinding, Custody, Endpoint, EndpointIdentity,
    HOLD_CAPTURE, HeldCapture, Kind, OfflineCgroupHandle, OfflineLiveSubtree, PREPARE, Path,
    ProtectedSource, QualificationPort, RPC_LIMIT, Recipe, Result, SelectedTask, Serving, StartRpc,
    TaskKey, Verdict, boot_key, campaign_digest, capture_ns,
    containerd::{PendingWait, Tasks, check_socket},
    docker::{self, Sender},
    full_hex,
    image::ImageMetadata,
    invalid, json,
    kernel::{
        DeviceCut, KernelCut, KernelTask, StartupCut, StdioCut, exited, lifetime,
        selected_coordinates,
    },
    launch_state::LaunchState,
    policy::AppArmor,
    refuse,
    retirement::RetirementCapture,
    run_mode, serving_now, within,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use slice_campaign::offline_qualification::{
    FixtureRunning, OfflineAcceptedDisposition, OfflineCleanupTail, OfflineContainment,
    OfflineEntryFence, OfflineNaturalClosure, OfflineNaturalExit, OfflineNaturalRetirement,
    OfflineNaturalTerminal, OfflineNewIngress, OfflineOtherWork, OfflineRecoveryAuthority,
    OfflineRecoveryBacking, OfflineTerminalExecution,
};
use std::{
    fs::{self, File, Metadata},
    io::Read,
    os::{
        fd::OwnedFd,
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::PathBuf,
    time::{Duration, Instant},
};

#[path = "natural/authority.rs"]
mod authority;
#[path = "natural/oci.rs"]
mod oci;
use oci::OciPin;
#[path = "natural/selector.rs"]
mod selector;
#[cfg(feature = "private-natural-exit-foreground")]
pub use selector::run_natural_exit_foreground;
#[path = "natural/start.rs"]
mod start;

const NATURAL_END: Duration = Duration::from_secs(50);
const SERVING_EXECUTABLE_MAX: u64 = 256 * 1024 * 1024;

#[derive(PartialEq, Eq, Serialize)]
struct ExecutableNode {
    dev: u64,
    inode: u64,
    bytes: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl ExecutableNode {
    fn observe(m: &Metadata) -> Result<Self> {
        if !m.is_file()
            || m.uid() != 0
            || m.mode() & 0o022 != 0
            || m.len() == 0
            || m.len() > SERVING_EXECUTABLE_MAX
        {
            return refuse("loaded serving executable is not a protected bounded file");
        }
        Ok(Self {
            dev: m.dev(),
            inode: m.ino(),
            bytes: m.len(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
        })
    }

    fn check_hash(&self, exe: &File, sha256: &str, deadline: Instant) -> Result<()> {
        let mut buf = [0; 65536];
        let mut hash = Sha256::new();
        let mut input = exe.take(self.bytes + 1);
        let mut count = 0;
        loop {
            within(deadline)?;
            let n = input.read(&mut buf)?;
            if n == 0 {
                break;
            }
            count += n as u64;
            hash.update(&buf[..n]);
        }
        if count != self.bytes
            || *self != Self::observe(&exe.metadata()?)?
            || format!("{:x}", hash.finalize()) != sha256
        {
            return refuse("runtime executable differs from the selected artifact");
        }
        within(deadline)
    }
}

struct Daemon {
    sender: Sender,
    exe_path: PathBuf,
    pidfd: OwnedFd,
    node: ExecutableNode,
    ticks: u64,
}

impl Daemon {
    fn pin(sender: Sender, executable_sha256: &str, ticks: u64, deadline: Instant) -> Result<Self> {
        let pid = rustix::process::Pid::from_raw(sender.pid)
            .filter(|pid| pid.as_raw_pid() > 1 && sender.uid == 0 && sender.gid == 0)
            .ok_or_else(|| invalid("invalid intended root serving process"))?;
        let pidfd = rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::empty())
            .map_err(std::io::Error::from)?;
        let exe_path = PathBuf::from(format!("/proc/{}/exe", sender.pid));
        let exe = File::open(&exe_path)?;
        let node = ExecutableNode::observe(&exe.metadata()?)?;
        node.check_hash(&exe, executable_sha256, deadline)?;
        let daemon = Self {
            sender,
            exe_path,
            pidfd,
            node,
            ticks,
        };
        daemon.check()?;
        within(deadline)?;
        Ok(daemon)
    }

    fn check(&self) -> Result<()> {
        if exited(&self.pidfd)?
            || lifetime(self.sender.pid as u32)?.0 != self.ticks
            || ExecutableNode::observe(&fs::metadata(&self.exe_path)?)? != self.node
            || exited(&self.pidfd)?
        {
            return refuse("the original serving executable or process lifetime changed");
        }
        Ok(())
    }
}

/// Exact approved loaded containerd artifact and its non-reused process
/// lifetime. A systemd listener or a version string cannot fill these slots.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContainerdPin {
    pub(crate) artifact_sha256: String,
    pub(crate) start_ticks: u64,
}

impl ContainerdPin {
    fn check(&self, tasks: &Endpoint) -> Result<Sender> {
        if tasks.listener_pid <= 1
            || (tasks.listener_uid, tasks.listener_gid) != (0, 0)
            || self.start_ticks == 0
            || !full_hex(&self.artifact_sha256)
        {
            return refuse("containerd listener is not the bound root serving generation");
        }
        Ok(Sender {
            pid: tasks.listener_pid,
            uid: 0,
            gid: 0,
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Identity {
    docker_pid: i32,
    tasks: EndpointIdentity,
    runtime: ContainerdPin,
    oci: OciPin,
    curl_sha256: String,
    authority_sha256: String,
    trusted_gid0: bool,
}

fn covers_held(from: Instant, cut: Instant, observed: Instant, deadline: Instant) -> bool {
    from <= cut && cut < observed && observed < deadline
}

#[test]
fn natural_held_interval_requires_a_post_cut_observation_within_budget() {
    let from = Instant::now();
    let cut = from + Duration::from_secs(1);
    let observed = cut + Duration::from_secs(1);
    let deadline = observed + Duration::from_secs(1);
    assert!(covers_held(from, cut, observed, deadline));
    assert!(!covers_held(observed, cut, observed, deadline));
    assert!(!covers_held(from, cut, cut, deadline));
    assert!(!covers_held(from, cut, deadline, deadline));
}

impl Identity {
    pub(super) fn intent(
        &self,
        root: &Path,
        docker: &EndpointIdentity,
        source: &str,
    ) -> Result<String> {
        if self.docker_pid <= 1
            || !full_hex(source)
            || !full_hex(&self.curl_sha256)
            || !full_hex(&self.authority_sha256)
            || !self.oci.check()
        {
            return refuse("natural-exit source artifacts are not bound");
        }
        let root_meta = super::custody::check_root(root)?.metadata()?;
        campaign_digest(&json!([
            "b-init-natural-exit-v1",
            [
                root.as_os_str().as_bytes(),
                &root_meta.dev().to_be_bytes(),
                &root_meta.ino().to_be_bytes()
            ],
            docker,
            self,
            source,
            PREPARE.as_secs(),
            RPC_LIMIT.as_secs(),
            HOLD_CAPTURE.as_secs(),
            NATURAL_END.as_secs(),
        ]))
        .map_err(invalid)
    }
}

pub(super) struct Attempt<'a> {
    docker_pid: i32,
    docker: &'a Endpoint,
    authority_sha256: &'a str,
    scope: Option<(u64, authority::Scope)>,
    daemon: Option<Daemon>,
    tasks_endpoint: &'a Endpoint,
    runtime_pin: &'a ContainerdPin,
    runtime: Option<Daemon>,
    oci_pin: &'a OciPin,
    oci: Option<oci::Artifacts>,
    shim: Option<Daemon>,
    curl_sha256: &'a str,
    trusted_gid0: bool,
    retirement: Option<RetirementCapture>,
    start_route: Option<start::RegisteredStart>,
    start: Option<(u64, Instant)>,
    launch: Option<LaunchState>,
    apparmor: Option<AppArmor>,
    image: Option<ImageMetadata>,
    local_image_read: Option<u64>,
    wait: Option<WaitBinding<'a>>,
}

struct WaitBinding<'a> {
    start: StartRpc,
    execution: OfflineTerminalExecution,
    position: u64,
    pending: PendingWait<'a>,
}

#[derive(Serialize)]
pub(super) struct OriginalHeld {
    kernel_current: KernelCut,
    stock_startup: StartupCut,
    stock_root: super::kernel::MaterializedRoot,
    local_image_read: u64,
    checked_create_sha256: String,
    stock_isolation: super::kernel::IsolationCut,
    stock_stdio: StdioCut,
    stock_devices: DeviceCut,
}

impl<'a> Attempt<'a> {
    fn new(
        docker: &'a Endpoint,
        docker_pid: i32,
        authority_sha256: &'a str,
        tasks_endpoint: &'a Endpoint,
        runtime_pin: &'a ContainerdPin,
        oci_pin: &'a OciPin,
        curl_sha256: &'a str,
        trusted_gid0: bool,
    ) -> Result<Self> {
        Tasks::pinned(tasks_endpoint, curl_sha256)?;
        runtime_pin.check(tasks_endpoint)?;
        if docker.path != Path::new("/run/docker.sock") || docker.listener_uid != 0 {
            return refuse("natural exit requires the pinned stock Docker endpoint");
        }
        Ok(Self {
            docker_pid,
            docker,
            authority_sha256,
            scope: None,
            daemon: None,
            tasks_endpoint,
            runtime_pin,
            runtime: None,
            oci_pin,
            oci: None,
            shim: None,
            curl_sha256,
            trusted_gid0,
            retirement: None,
            start_route: None,
            start: None,
            launch: None,
            apparmor: None,
            image: None,
            local_image_read: None,
            wait: None,
        })
    }

    pub(super) fn identity(&self) -> Identity {
        Identity {
            docker_pid: self.docker_pid,
            tasks: self.tasks_endpoint.into(),
            runtime: self.runtime_pin.clone(),
            oci: self.oci_pin.clone(),
            curl_sha256: self.curl_sha256.into(),
            authority_sha256: self.authority_sha256.into(),
            trusted_gid0: self.trusted_gid0,
        }
    }

    pub(super) fn bind_scope(
        &mut self,
        root: &Path,
        binding: &AttemptBinding,
        serving: &Serving,
        journal: &mut Custody,
        deadline: Instant,
    ) -> Result<()> {
        if self.scope.is_some() {
            return refuse("natural authority scope is not repeatable");
        }
        let scope = authority::Scope::load(
            root,
            self.authority_sha256,
            binding,
            self.docker,
            self.tasks_endpoint,
            serving,
        )?;
        let position = journal.record(
            Kind::NaturalAuthority,
            json!({
                "selection":binding.binding_digest(), "sha256":self.authority_sha256,
                "raw_operator_readout":scope.readout(),
            }),
        )?;
        self.scope = Some((position, scope));
        within(deadline)
    }

    pub(super) fn pin_serving(
        &mut self,
        serving: &Serving,
        journal: &mut Custody,
        prep: Instant,
    ) -> Result<()> {
        let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")?;
        if boot.trim().replace('-', "") != boot_key(&serving.boot) {
            return refuse("pinned serving boot changed");
        }
        let daemon = Daemon::pin(
            Sender {
                pid: self.docker_pid,
                uid: 0,
                gid: 0,
            },
            &serving.daemon_artifact_sha256,
            serving.daemon_process_start_ticks,
            prep,
        )?;
        let sender = self.runtime_pin.check(self.tasks_endpoint)?;
        if sender.pid == daemon.sender.pid {
            return refuse("stock Docker and containerd cannot share a serving process");
        }
        let runtime = Daemon::pin(
            sender,
            &self.runtime_pin.artifact_sha256,
            self.runtime_pin.start_ticks,
            prep,
        )?;
        // This is a read-only listener check, not a Tasks operation. A
        // different peer on the root-owned canonical socket is a refusal.
        check_socket(self.tasks_endpoint)?;
        self.tasks_endpoint.connect()?;
        runtime.check()?;
        let oci = self.oci_pin.prepare(prep)?;
        let mut probe = self.docker.connect()?;
        let deadline = prep.min(Instant::now() + RPC_LIMIT);
        docker::authenticate(&probe, Some(daemon.sender))?;
        docker::write(&mut probe, docker::STOCK_PROBE, deadline)?;
        let version = docker::receive(&mut probe, deadline, Some(daemon.sender));
        if !version
            .response
            .as_ref()
            .is_some_and(docker::is_stock_version)
        {
            return refuse("pre-effect Docker probe did not identify the selected stock daemon");
        }
        daemon.check()?;
        runtime.check()?;
        journal.record(Kind::Serving, json!({
            "boot":boot, "version_probe":version,
            "stock_docker_process":daemon.sender, "stock_docker_start_ticks":daemon.ticks,
            "stock_docker_loaded_executable":daemon.node,
            "stock_docker_sha256":serving.daemon_artifact_sha256,
            "stock_containerd_process":runtime.sender, "stock_containerd_start_ticks":runtime.ticks,
            "stock_containerd_loaded_executable":runtime.node,
            "stock_containerd_sha256":self.runtime_pin.artifact_sha256,
            "stock_containerd_endpoint":EndpointIdentity::from(self.tasks_endpoint),
            "stock_oci_disk_artifacts":oci,
        }))?;
        self.daemon = Some(daemon);
        self.runtime = Some(runtime);
        self.oci = Some(oci);
        within(prep)
    }

    fn check_shim(&self, original: &TaskKey) -> Result<()> {
        let shim = self
            .shim
            .as_ref()
            .ok_or_else(|| invalid("no pinned original runtime shim"))?;
        shim.check()?;
        if super::kernel::ticks_and_parent(original.pid)?
            != (original.start_ticks, shim.sender.pid as u32)
        {
            return refuse("original init no longer belongs to its selected runtime shim");
        }
        Ok(())
    }

    fn live_runtime(&self) -> Result<()> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| invalid("original containerd not pinned"))?;
        check_socket(self.tasks_endpoint)?;
        runtime.check()
    }

    pub(super) fn live_sender(&self) -> Result<Sender> {
        let daemon = self
            .daemon
            .as_ref()
            .ok_or_else(|| invalid("original Dockerd not pinned"))?;
        let (_, scope) = self
            .scope
            .as_ref()
            .ok_or_else(|| invalid("no commissioned admission scope"))?;
        scope.check(self.docker, self.tasks_endpoint)?;
        daemon.check()?;
        self.live_runtime()?;
        Ok(daemon.sender)
    }

    pub(super) fn pin_policy(
        &mut self,
        recipe: &Recipe,
        journal: &mut Custody,
        prep: Instant,
    ) -> Result<()> {
        let pin = AppArmor::pin(
            &recipe.apparmor_profile,
            &recipe.apparmor_policy_sha256,
            prep,
        )?;
        journal.record(
            Kind::NaturalPolicy,
            json!({"pre_effect_apparmor":pin.generation()}),
        )?;
        self.apparmor = Some(pin);
        Ok(())
    }

    pub(super) fn check_image(
        &mut self,
        endpoint: &Endpoint,
        serving: &Serving,
        journal: &mut Custody,
        deadline: Instant,
    ) -> Result<()> {
        let image = self
            .image
            .as_ref()
            .ok_or_else(|| invalid("missing Campaign-pinned image metadata"))?;
        let sender = self.live_sender()?;
        let target = format!("GET /{}/images/{}/json", serving.api, image.image.image_id);
        let request = docker::request(&target, &[]);
        let mut io = endpoint.connect()?;
        docker::authenticate(&io, Some(sender))?;
        let deadline = deadline.min(Instant::now() + RPC_LIMIT);
        docker::write(&mut io, &request, deadline)?;
        let observed = docker::receive(&mut io, deadline, Some(sender));
        let read = journal.record(
            Kind::NaturalImage,
            json!({"metadata":image,"request":request,"observed":observed}),
        )?;
        let response = observed
            .response
            .filter(|r| r.status == 200)
            .ok_or_else(|| {
                invalid("the pinned serving daemon did not resolve the exact local image")
            })?;
        image.check_local_inspect(&response.body)?;
        self.live_sender()?;
        within(deadline)?;
        if self.local_image_read.replace(read).is_some() {
            return refuse("selected local image readback repeated");
        }
        Ok(())
    }

    /// Read the actual original kernel and launch records. Startup, policy
    /// and isolation do not come from the inherited broadcaster association.
    pub(super) fn capture_original_held(
        &self,
        original: &KernelTask,
        recipe: &Recipe,
        create: &CreateTrace,
        rpc: &StartRpc,
        attach: &Attach,
        deadline: Instant,
    ) -> Result<(AppliedWitness, OriginalHeld)> {
        let retained = self
            .launch
            .as_ref()
            .ok_or_else(|| invalid("no original runc launch inputs"))?;
        let task = original.retirement_key();
        self.start_route()?.check_stdout(rpc, attach)?;
        if task.serving != rpc.serving || task.container != rpc.full_id {
            return refuse("stock Held launch crossed the checked original Start or stdout");
        }
        self.live_sender()?;
        self.check_shim(task)?;
        let image = self
            .image
            .as_ref()
            .ok_or_else(|| invalid("missing selected stock image"))?;
        let local_image_read = self
            .local_image_read
            .ok_or_else(|| invalid("no authenticated selected local image"))?;
        recipe
            .verify_create(
                create,
                &task.serving,
                &task.container,
                &rpc.links.create_request,
            )
            .map_err(invalid)?;
        // Pinned Moby Create resolves this exact manifest/platform and derives
        // the fresh RWLayer from it. In the stock Start path the mounted
        // RWLayer supplies BaseFS in the original OCI spec; runc's applied
        // root must still join that mount. Neither the image readback nor the
        // recipe alone establishes this result.
        let root = retained.materialized_root()?;
        let launch = retained.project_launch(
            task,
            &image.image,
            &recipe.apparmor_profile,
            original.stock_root_mount_id()?,
            StartupBacking::ImmutableImageAndDependencies,
            // Conditional on the authenticated prearm and exclusive Start
            // cut succeeding below the caller's consumption boundary.
            StdoutRoute::ExclusiveSelectedTaskLivePipe,
        )?;
        if !launch.matches(recipe) {
            return refuse("observed stock launch differs from the approved recipe");
        }
        let uid = retained.check_launch(&launch, &recipe.apparmor_profile)?;
        let stock_startup = original.stock_startup(&launch, uid, deadline)?;
        let stock_root = original.stock_materialized_root(&root, &stock_startup, deadline)?;
        let stock_isolation = original.observe_stock_isolation(deadline)?;
        let (mountinfo, mounts) =
            original.observe_stock_mounts(&launch, stock_isolation.task[1], deadline)?;
        let stock_devices = original.stock_devices(&mountinfo, deadline)?;
        let stock_stdio = original.stock_stdio(retained.output_pipes()?)?;
        let (_, label, credentials) = original.stock_identity()?;
        let (applied_apparmor_profile, applied_apparmor_sha256) = self
            .apparmor
            .as_ref()
            .ok_or_else(|| invalid("no original AppArmor policy"))?
            .check(deadline)?
            .identify_task(&label)?;
        let applied_seccomp_sha256 = retained.seccomp_pin()?;
        // No task-local cgroup control through files: the closed inventory
        // must have the original read-only cgroup2 mount and no second view.
        // No inherited control/input FD or reachable host network/filesystem:
        // these are projections of the verified descriptors, private net,
        // closed mounts/devices and the approved fresh image-backed root.
        // This says nothing about *external* admission; that is a separate
        // obligation of the exclusive Start/coverage cut.
        if !mounts.mounts.iter().any(|m| {
            m.path == "/sys/fs/cgroup" && m.kind == MountKind::PrivateVirtual && m.read_only
        }) {
            return refuse("stock task's only cgroup view was not proven read-only");
        }
        let effective = super::Effective {
            task: task.clone(),
            uid: credentials.uid,
            gid: credentials.gid,
            groups: credentials.groups,
            caps: credentials.caps,
            nnp: credentials.nnp,
            umask: credentials.umask,
            seccomp_filters: credentials.seccomp_filters,
            applied_seccomp_sha256,
            apparmor_enforcing: true,
            applied_apparmor_profile,
            applied_apparmor_sha256,
            root_mount_id: launch.root_mount_id,
            read_only_root: true,
            namespaces: Namespaces {
                task: stock_isolation.task,
                host: stock_isolation.host,
            },
            inventory: Some(Inventory {
                mounts: mounts.mounts,
                devices: stock_devices.devices(),
                interfaces: vec!["lo".into()], // checked over the complete raw network cut
                non_loopback_route_count: 0,
                externally_reachable_control_handles: 0,
                writable_cgroup_control: false,
            }),
            fds: stock_stdio.descriptors(),
        };
        if !effective.matches(recipe, &launch, true) {
            return refuse("observed original applied state is not the approved confined launch");
        }
        let kernel_current = original.check_stock_current(&launch, &effective, deadline)?;
        // Store only the rejoined raw reads in one Held record. Equality
        // across these reads is not by itself the interval coverage proof.
        if kernel_current.mountinfo() != Some(mountinfo.as_slice())
            || kernel_current.apparmor != label
        {
            return refuse("original mount or policy readback changed during Held collection");
        }
        self.live_sender()?;
        within(deadline)?;
        Ok((
            AppliedWitness {
                start: rpc.clone(),
                attach: attach.clone(),
                launch,
                effective,
            },
            OriginalHeld {
                kernel_current,
                stock_startup,
                stock_root,
                local_image_read,
                checked_create_sha256: crate::digest(&serde_json::to_vec(create)?),
                stock_isolation,
                stock_stdio,
                stock_devices,
            },
        ))
    }

    /// Consume the commissioned scope with a real post-cut sample.
    /// The authenticated marker is emitted after the fixed init's last state
    /// change. From there it only sleeps and exits; in the excluded-ingress
    /// scope the original kernel attributes cannot be changed by a competing
    /// execution. The AppArmor revision covers external policy replacement.
    /// A procfs read or a still-existing shim is not itself proof of liveness.
    pub(super) fn recheck_held(
        &self,
        recipe: &Recipe,
        applied: &AppliedWitness,
        original: &KernelTask,
        capture: &HeldCapture,
        collection_from: Instant,
        cut: Instant,
        clock: Instant,
        journal: &mut Custody,
        deadline: Instant,
        source_identity: &str,
        check_stream: impl FnOnce(&mut Custody) -> Result<()>,
    ) -> Result<AppliedCoverage> {
        self.start_route()?
            .check_stdout(&applied.start, &applied.attach)?;
        let (authority_position, _) = self
            .scope
            .as_ref()
            .ok_or_else(|| invalid("missing the commissioned original Start scope"))?;
        if original.retirement_key() != &applied.launch.task
            || applied.effective.task != applied.launch.task
            || capture.at_ns != capture_ns(clock, cut)?
        {
            return refuse("natural Held recheck crossed the original execution or cut");
        }
        let apparmor = self
            .apparmor
            .as_ref()
            .ok_or_else(|| invalid("no original AppArmor policy"))?;
        let launch = self
            .launch
            .as_ref()
            .ok_or_else(|| invalid("no original runc policy inputs"))?;
        let converted_seccomp = launch.seccomp_pin()?;
        if converted_seccomp != recipe.seccomp_sha256
            || converted_seccomp != applied.effective.applied_seccomp_sha256
            || applied.effective.applied_apparmor_profile != recipe.apparmor_profile
            || applied.effective.applied_apparmor_sha256 != recipe.apparmor_policy_sha256
        {
            return refuse("Held applied policy witness differs from the stock source and pins");
        }
        let policy = apparmor.check(deadline)?;
        self.check_shim(&applied.launch.task)?;
        check_stream(journal)?;
        self.live_sender()?;
        let cgroup_events = original.live_events()?;
        let observed = Instant::now();
        if !covers_held(collection_from, cut, observed, deadline) {
            return refuse("natural Held source does not bracket the consumed cut");
        }
        let evidence = json!({
            "scope_position":authority_position, "scope_sha256":self.authority_sha256,
            "capture":capture, "task":original.retirement_key(),
            "start":applied.start, "stdout":applied.attach,
            "collection_from_ns":capture_ns(clock, collection_from)?,
            "observed_through_ns":capture_ns(clock, observed)?,
            "original_live_cgroup_events":cgroup_events,
            "held_apparmor":policy, "converted_seccomp_sha256":converted_seccomp,
        });
        let evidence_sha256 = crate::digest(&serde_json::to_vec(&evidence)?);
        journal.record(Kind::NaturalHeldRecheck, evidence)?;
        within(deadline)?;
        Ok(AppliedCoverage {
            source: source_identity.into(),
            capture_position: capture.position,
            applied: capture.applied.clone(),
            evidence_sha256,
            from: collection_from,
            through: observed,
        })
    }

    fn start_route(&self) -> Result<&start::RegisteredStart> {
        self.start_route
            .as_ref()
            .ok_or_else(|| invalid("natural Start was not prearmed"))
    }

    fn start_route_mut(&mut self) -> Result<&mut start::RegisteredStart> {
        self.start_route
            .as_mut()
            .ok_or_else(|| invalid("natural Start was not prearmed"))
    }

    pub(super) fn arm(
        &mut self,
        journal: &mut Custody,
        checked_create: u64,
        rpc: &StartRpc,
        attach: &Attach,
        prep: Instant,
    ) -> Result<()> {
        if self.start_route.is_some() || self.retirement.is_some() {
            return refuse("natural prearm is not repeatable");
        }
        self.live_sender()?;
        self.start_route = Some(start::RegisteredStart::register(
            journal,
            checked_create,
            rpc,
            attach,
        )?);
        self.retirement = Some(RetirementCapture::arm(journal, self.trusted_gid0, prep)?);
        Ok(())
    }

    pub(super) fn start_armed(&mut self, position: u64, at: Instant) -> Result<()> {
        let retirement = self
            .retirement
            .as_ref()
            .ok_or_else(|| invalid("retirement was not prearmed"))?;
        if self.start.is_some() || retirement.arm_position() >= position {
            return refuse("natural exit requires one prearmed Start");
        }
        self.start_route_mut()?.arm(position, at)?;
        self.start = Some((position, at + NATURAL_END));
        Ok(())
    }

    pub(super) fn start_written(&mut self, position: u64, at: Instant) -> Result<()> {
        self.start_route_mut()?.written(position, at)
    }

    pub(super) fn start_accepted(
        &mut self,
        position: u64,
        at: Instant,
        reply: Option<&docker::Response>,
    ) -> Result<()> {
        self.start_route_mut()?.accepted(position, at, reply)
    }

    pub(super) fn selected(
        &mut self,
        journal: &mut Custody,
        rpc: &StartRpc,
        hold: Instant,
    ) -> Result<SelectedTask> {
        self.start_route()?.require_accepted(rpc)?;
        self.live_runtime()?;
        let observed = Tasks::pinned(self.tasks_endpoint, self.curl_sha256)?
            .running(&rpc.full_id, hold.min(Instant::now() + RPC_LIMIT))?;
        self.live_runtime()?;
        journal.record(
            Kind::NaturalTasksGet,
            json!({"start":rpc,"running":observed}),
        )?;
        let launch = LaunchState::read(rpc, hold)?;
        let launch_payload = serde_json::to_value(&launch)?;
        let generation = crate::digest(&serde_json::to_vec(&launch_payload)?);
        let position = journal.record(
            Kind::NaturalLaunchState,
            json!({"start":rpc,"runtime_launch":launch_payload,"sha256":generation}),
        )?;
        let pid = observed.view.pid;
        let init = launch.init_coordinates(&rpc.full_id, pid)?;
        let (pid_namespace, subtree_mount_id, subtree_inode) =
            selected_coordinates(pid, init.ticks, &init.cgroup)?;
        let subtree_capture = campaign_digest(&json!([
            "b-natural-original-handle-v1",
            rpc.links.selection,
            rpc.links.start_mark,
            position.to_be_bytes(),
        ]))
        .map_err(invalid)?;
        within(hold)?;
        if self.launch.is_some() || self.shim.is_some() {
            return refuse("natural task already selected");
        }
        let artifacts = self
            .oci
            .as_ref()
            .ok_or_else(|| invalid("no pre-effect OCI artifact selection"))?;
        let (shim, cut) = self.oci_pin.original_shim(
            artifacts,
            &launch,
            &rpc.full_id,
            (pid, init.ticks),
            &self.tasks_endpoint.path,
            hold,
        )?;
        journal.record(
            Kind::NaturalOciRuntime,
            json!({"start":rpc,"original_oci":cut,"pins":self.oci_pin}),
        )?;
        self.shim = Some(shim);
        self.launch = Some(launch);
        Ok(SelectedTask {
            start: rpc.clone(),
            task: TaskKey {
                serving: rpc.serving.clone(),
                container: rpc.full_id.clone(),
                task_id: rpc.full_id.clone(),
                pid,
                start_ticks: init.ticks,
                pid_namespace,
                exec_generation: generation,
                runtime_namespace: "moby".into(),
                subtree_capture,
                subtree_mount_id,
                subtree_inode,
            },
        })
    }

    pub(super) fn bind_wait(
        &mut self,
        journal: &mut Custody,
        rpc: &StartRpc,
        key: &TaskKey,
        running: &FixtureRunning,
        anchor: &impl Anchor,
        hold: Instant,
    ) -> Result<()> {
        let (_, end) = self.start.ok_or_else(|| invalid("Start was not armed"))?;
        if self.wait.is_some()
            || rpc.full_id != key.container
            || key.task_id != key.container
            || !anchor
                .original_kernel_task()
                .is_some_and(|a| a.retirement_key() == key)
        {
            return refuse("natural terminal task is not the exact retained original");
        }
        let tasks = Tasks::pinned(self.tasks_endpoint, self.curl_sha256)?;
        let execution = OfflineTerminalExecution {
            selection: rpc.links.selection.clone(),
            create_request: rpc.links.create_request.clone(),
            create_mark: rpc.links.create_mark.clone(),
            start_mark: rpc.links.start_mark.clone(),
            live: OfflineLiveSubtree {
                running: running.clone(),
                boot: boot_key(&key.serving.boot),
                runtime_namespace: key.runtime_namespace.clone(),
                cgroup: OfflineCgroupHandle {
                    capture: key.subtree_capture.clone(),
                    mount_id: key.subtree_mount_id,
                    inode: key.subtree_inode,
                },
                capture_position: journal.next(),
                cgroup_events: anchor.live_events()?,
            },
        };
        journal.record(Kind::LiveSubtree, json!(&execution.live))?;
        // A local Wait arm isn't admission by the runtime. The joined
        // closure must exclude retargeting to any replacement for the whole
        // interval in which the service could resolve this request.
        let position = journal.record(Kind::NaturalWaitArm, json!(&execution))?;
        self.live_runtime()?;
        let pending = tasks.arm_wait(&rpc.full_id, end)?;
        self.live_runtime()?;
        anchor.live_events()?;
        within(hold)?;
        self.wait = Some(WaitBinding {
            start: rpc.clone(),
            execution,
            position,
            pending,
        });
        Ok(())
    }

    pub(super) fn finish(
        &mut self,
        port: &QualificationPort,
        binding: &AttemptBinding,
        docker: &Endpoint,
        source: &mut impl ProtectedSource,
        journal: &mut Custody,
        anchor: &impl Anchor,
        held: &HeldHistory,
        held_position: u64,
    ) -> Result<Verdict> {
        let (start_arm, deadline) = self.start.ok_or_else(|| invalid("missing Start arm"))?;
        let WaitBinding {
            start,
            execution,
            position: wait_arm,
            pending,
        } = self
            .wait
            .take()
            .ok_or_else(|| invalid("missing original Wait"))?;
        let key = held.execution();
        let disposition = self.start_route()?.settlement(&start)?;
        if !same_held_execution(binding, held, &execution)
            || held.links() != &start.links
            || key.container != start.full_id
            || key.task_id != start.full_id
            || key.serving != start.serving
            || disposition.arm != start_arm
        {
            return refuse("Held crossed natural-exit Wait or original Start binding");
        }
        let exit = match pending.receive() {
            Ok(exit) => exit,
            Err(error) => {
                journal.record(
                    Kind::NaturalWaitReply,
                    json!({"execution":execution,"error":error.to_string()}),
                )?;
                return Err(error);
            }
        };
        let wait_reply = journal.record(
            Kind::NaturalWaitReply,
            json!({"execution":execution,"exit":exit}),
        )?;
        // Preserve the raw response even if its serving/lifetime join fails.
        self.live_runtime()?;
        let original = anchor
            .original_kernel_task()
            .ok_or_else(|| invalid("missing original retirement anchor"))?;
        if original.retirement_key() != key {
            return refuse("retirement anchor is not the Held execution");
        }
        self.retirement
            .as_ref()
            .ok_or_else(|| invalid("missing subtree retirement capture"))?
            .await_original(original, deadline)?;
        // On an error above, preserve_unresolved still owns the raw capture.
        let retired = self
            .retirement
            .take()
            .expect("same retained capture")
            .finish(journal, Some(original), deadline)?;
        let retirement_archive_sha256 = journal.tail_sha256()?;
        serving_now(source, docker, &key.serving, journal, deadline)?;
        let daemon = self.live_sender()?;
        let runtime = self.runtime_pin.check(self.tasks_endpoint)?;
        let (scope_position, _) = self
            .scope
            .as_ref()
            .ok_or_else(|| invalid("missing original admission scope"))?;
        let cut = SettlementCut {
            scope: *scope_position,
            retirement_arm: retired.arm,
            start: disposition,
            live: execution.live.capture_position,
            wait_arm,
            held: held_position,
            wait_reply,
            retirement_read: retired.capture,
            final_position: journal.next(),
        };
        let original_exited = original.original_exited()?;
        cut.check(exit.status, original_exited)?;
        within(deadline)?;
        let evidence = json!({
            "basis": STOCK_SETTLEMENT_BASIS,
            "execution": execution, "task": key, "start": start,
            "cut": cut, "scope_sha256": self.authority_sha256,
            "final_docker": daemon, "final_containerd": runtime,
            "original_pidfd_exited": original_exited,
            "retirement_archive_sha256": retirement_archive_sha256,
            "original_wait_wire_sha256": crate::digest(&exit.wire),
        });
        let closure = stock_closure(crate::digest(&serde_json::to_vec(&evidence)?));
        let closure_read = journal.record(Kind::NaturalClosure, evidence)?;
        let end = OfflineNaturalTerminal {
            retirement_arm: retired.arm,
            start_arm,
            wait_arm,
            held_position,
            wait_reply,
            retirement_read: retired.capture,
            closure_read,
            exit: OfflineNaturalExit {
                lifetime: execution.live.running.lifetime.clone(),
                status: exit.status,
                runtime_wire_sha256: crate::digest(&exit.wire),
            },
            retirement: OfflineNaturalRetirement {
                cgroup: execution.live.cgroup.clone(),
                original_pidfd_exited: original_exited,
                retirement_archive_sha256,
            },
            execution,
            closure,
        };
        journal.record(Kind::Settlement, json!(&end))?;
        within(deadline)?;
        record_settlement(port, binding, held, &end).map_err(invalid)?;
        // The collected cut, not the time of the durable write's reply, is
        // subject to the observation deadline. Never relabel a committed cut
        // as unqualified just because its commit acknowledgement was slow.
        let verdict = Verdict {
            held: true,
            terminal: true,
            qualified: true,
        };
        journal.record(Kind::Verdict, json!({"result":verdict}))?;
        Ok(verdict)
    }

    pub(super) fn preserve_unresolved(
        &mut self,
        binding: &AttemptBinding,
        source: &mut impl ProtectedSource,
        journal: &mut Custody,
        anchor: Option<&impl Anchor>,
        outcome: &Result<Verdict>,
    ) -> Result<()> {
        if outcome.as_ref().is_ok_and(|v| v.qualified) {
            return Ok(());
        }
        let original = anchor.and_then(Anchor::original_kernel_task);
        let abort = match (self.start, original) {
            (Some((start_arm, _)), Some(original)) => {
                explicit_abort(binding, source, journal, start_arm, original)
            }
            _ => Ok(()), // Nothing to target; no numeric PID or runtime fallback.
        };
        // Failure, lost reply or intervention does not make this attempt
        // qualify, or justify erasing its only possible evidence. Never replay.
        let retained = self.retirement.take().map(|capture| {
            capture.finish(
                journal,
                original,
                self.start
                    .map(|(_, deadline)| deadline)
                    .unwrap_or_else(Instant::now),
            )
        });
        journal.record(
            Kind::Verdict,
            json!({"natural_exit":"spent/unresolved",
                "error":outcome.as_ref().err().map(ToString::to_string),
                "abort_error":abort.as_ref().err().map(ToString::to_string),
                "retirement_error":retained.and_then(|r| r.err()).map(|e| e.to_string()),
            }),
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct AbortSubject {
    pub selection: String,
    pub start_arm: u64,
    pub original: TaskKey,
}

#[derive(Serialize)]
pub(crate) struct AbortApproval {
    pub subject: AbortSubject,
    /// Custody reference to an explicit operator authorization, distinct
    /// from the launch grant. This is not inferred by the controller.
    pub operator_approval_sha256: String,
    pub raw_operator_decision: String,
}

fn apply_abort<T>(
    subject: &AbortSubject,
    approval: &AbortApproval,
    sync_arm: impl FnOnce() -> Result<()>,
    signal_original: impl FnOnce() -> T,
) -> Result<T> {
    if &approval.subject != subject
        || approval.raw_operator_decision.is_empty()
        || crate::digest(approval.raw_operator_decision.as_bytes())
            != approval.operator_approval_sha256
    {
        return refuse("abort approval does not bind the exact spent attempt and original task");
    }
    sync_arm()?;
    Ok(signal_original())
}

fn explicit_abort(
    binding: &AttemptBinding,
    source: &mut impl ProtectedSource,
    journal: &mut Custody,
    start_arm: u64,
    original: &KernelTask,
) -> Result<()> {
    if original.original_exited()? {
        return Ok(());
    }
    let subject = AbortSubject {
        selection: binding.binding_digest().into(),
        start_arm,
        original: original.retirement_key().clone(),
    };
    let deadline = Instant::now() + RPC_LIMIT;
    let Some(approval) = source.natural_abort_approval(&subject, deadline)? else {
        return Ok(());
    };
    within(deadline)?;
    let disposition = apply_abort(
        &subject,
        &approval,
        || {
            journal.record(
                Kind::NaturalAbortArmed,
                json!({"approval":approval,"may_intervene":true}),
            )?;
            Ok(())
        },
        || -> std::io::Result<bool> {
            if original.original_exited()? {
                return Ok(false);
            }
            original.abort_original()?;
            Ok(true)
        },
    )?;
    journal.record(
        Kind::NaturalAbortDisposition,
        json!({"subject":subject,"sent":disposition.as_ref().ok(),
            "error":disposition.as_ref().err().map(ToString::to_string)}),
    )?;
    // Neither a positive signal result nor ESRCH establishes settlement.
    Ok(())
}

#[cfg(test)]
#[test]
fn natural_containerd_pin_refuses_listener_without_original_serving_artifact_and_lifetime() {
    let mut e = Endpoint {
        path: "/run/containerd/containerd.sock".into(),
        device: 1,
        inode: 2,
        listener_pid: 90,
        listener_uid: 0,
        listener_gid: 0,
    };
    let mut pin = ContainerdPin {
        artifact_sha256: "a".repeat(64),
        start_ticks: 123,
    };
    assert_eq!(pin.check(&e).unwrap().pid, 90);
    e.listener_pid = 1;
    assert!(pin.check(&e).is_err());
    e.listener_pid = 90;
    e.listener_gid = 2;
    assert!(pin.check(&e).is_err());
    e.listener_gid = 0;
    pin.start_ticks = 0;
    assert!(pin.check(&e).is_err());
    pin.start_ticks = 123;
    pin.artifact_sha256 = "version-not-loaded-bytes".into();
    assert!(pin.check(&e).is_err());
}

#[cfg(test)]
#[test]
fn natural_abort_requires_exact_authorization_and_durable_arm_before_signal() {
    let task = super::super::tests::task(&super::super::tests::serving(), &"a".repeat(64));
    let subject = AbortSubject {
        selection: "b".repeat(64),
        start_arm: 11,
        original: task,
    };
    let mut approval = AbortApproval {
        subject: subject.clone(),
        operator_approval_sha256: crate::digest(b"test operator decision"),
        raw_operator_decision: "test operator decision".into(),
    };
    let no_signal = || panic!("unapproved or unsynced signal");
    approval.subject.start_arm += 1;
    assert!(
        apply_abort(
            &subject,
            &approval,
            || panic!("crossed approval"),
            no_signal
        )
        .is_err()
    );
    approval.subject = subject.clone();
    assert!(apply_abort(&subject, &approval, || refuse("disk failure"), no_signal).is_err());
    let armed = std::cell::Cell::new(false);
    apply_abort(
        &subject,
        &approval,
        || {
            armed.set(true);
            Ok(())
        },
        || assert!(armed.get()),
    )
    .unwrap();
}

// Source semantics are part of commissioning these pinned loaded artifacts;
// version strings or a similar-looking wire reply do not authorize this join.
const STOCK_SETTLEMENT_BASIS: [&str; 4] = [
    "b-natural-stock-closed-frontier-v1",
    "moby:6a43e3d5afddf4111da0f864bbc7cae5d7e95001",
    "containerd:aad11006b869517fcd3009450b6f82da282e1a9b",
    "commissioned-no-out-of-band; checked-create; synchronous-start; original-init-wait; original-pidfd-and-subtree-retired",
];

#[derive(Serialize)]
struct SettlementCut {
    scope: u64,
    retirement_arm: u64,
    start: start::Disposition,
    live: u64,
    wait_arm: u64,
    held: u64,
    wait_reply: u64,
    retirement_read: u64,
    final_position: u64,
}

impl SettlementCut {
    fn check(&self, status: u32, original_exited: bool) -> Result<()> {
        let order = [
            self.scope,
            self.start.checked_create,
            self.start.registration,
            self.retirement_arm,
            self.start.arm,
            self.start.written,
            self.start.accepted,
            self.live,
            self.wait_arm,
            self.held,
            self.wait_reply,
            self.retirement_read,
            self.final_position,
        ];
        if self.scope == 0
            || !order.windows(2).all(|p| p[0] < p[1])
            || status != 0
            || !original_exited
        {
            return refuse("natural settlement lacks ordered original successful completion");
        }
        Ok(())
    }
}

// Only after the observed cut above. Campaign independently checks the
// durable one-shot frontier (exact Start and Held, no contest or later mark).
// Under the commissioned scope no other client, broker, reconciler or
// administrator has admitted work for this object. In these pinned sources:
// * Moby daemon/start.go waits for task creation and Start before the 204;
//   health.go and restartmanager.go exclude autonomous Exec/restart for the
//   checked NONE/no recipe. The exit monitor can still clean up or Delete.
// * Moby's fresh containerd object sets only its bundle label, not the
//   restart plugin's required containerd.io/restart.status. The scope excludes
//   other writers; the retained daemons exclude restart/restore generations.
// * Tasks.Wait (empty exec ID) is the original shim init Wait, not an event
//   or registry lookup result. Its status/time readers take the same lock as
//   the stopped transition. Stopped/deleted init cannot Start/Exec/resume;
//   remaining cleanup cannot create a replacement. No shim-death or Delete
//   acknowledgement is required, nor do we claim to observe either.
// The source read, retained kernel lifetimes, checked protected scope and
// dedicated no-Stop path (abort only on failure) jointly exclude intervention;
// this is not an inference from the absence of an abort record.
fn stock_closure(evidence_sha256: String) -> OfflineNaturalClosure {
    OfflineNaturalClosure {
        ingress: OfflineNewIngress {
            controller: OfflineEntryFence::Enforced,
            engine: OfflineEntryFence::Enforced,
            runtime: OfflineEntryFence::Enforced,
        },
        create: OfflineAcceptedDisposition::Completed,
        start: OfflineAcceptedDisposition::Completed,
        other_work: OfflineOtherWork::ExcludedByClosedAcceptanceCut,
        cleanup_tail: OfflineCleanupTail::CannotExecuteOrReopen,
        recovery: OfflineRecoveryAuthority {
            backing: OfflineRecoveryBacking::RetainedExecutionInert,
            controller_reconcile: OfflineEntryFence::Enforced,
            runtime_reconcile: OfflineEntryFence::Enforced,
        },
        containment: OfflineContainment::ContinuousThroughFinalRead,
        no_intervention: true,
        evidence_sha256,
    }
}

fn record_settlement(
    port: &QualificationPort,
    binding: &AttemptBinding,
    held: &HeldHistory,
    terminal: &OfflineNaturalTerminal,
) -> std::result::Result<(), JoinRefusal> {
    if !same_held_execution(binding, held, &terminal.execution) {
        return Err(JoinRefusal::CrossedExecution);
    }
    let projection = fixture_held_projection(binding, held).map_err(JoinRefusal::HeldProjection)?;
    let held_digest = held.capture_sha256().map_err(JoinRefusal::HeldProjection)?;
    port.record_offline_natural_terminal(binding, terminal, &projection, &held_digest)
        .map_err(JoinRefusal::Owner)
}

#[cfg(test)]
#[path = "natural/join_tests.rs"]
mod join_tests;

#[test]
fn natural_settlement_rejects_early_crossed_or_missing_original_end() {
    let mut cut = SettlementCut {
        scope: 2,
        retirement_arm: 11,
        start: start::Disposition {
            checked_create: 8,
            registration: 10,
            arm: 12,
            written: 13,
            accepted: 14,
        },
        live: 15,
        wait_arm: 16,
        held: 20,
        wait_reply: 21,
        retirement_read: 22,
        final_position: 24,
    };
    assert!(cut.check(0, true).is_ok());
    assert!(cut.check(1, true).is_err());
    assert!(cut.check(0, false).is_err());
    cut.start.accepted = 17;
    assert!(cut.check(0, true).is_err());
    cut.start.accepted = 14;
    cut.wait_reply = cut.held;
    assert!(cut.check(0, true).is_err());
    cut.wait_reply = 21;
    cut.retirement_read = 0;
    assert!(cut.check(0, true).is_err());
}

/// The selector owns fixed sources. No public effect API accepts a source
/// provider or substitutes the task/retirement handle.
#[cfg(feature = "private-natural-exit-foreground")]
fn run_natural(
    admission: super::Admission<'_>,
    root: &Path,
    docker: &Endpoint,
    docker_pid: i32,
    authority_sha256: &str,
    tasks: &Endpoint,
    runtime_pin: &ContainerdPin,
    oci_pin: &OciPin,
    curl_sha256: &str,
    trusted_gid0: bool,
    recipe: &Recipe,
    manifest: Vec<u8>,
    image_config: Vec<u8>,
    source: &mut impl ProtectedSource,
) -> Result<Verdict> {
    let mut attempt = Attempt::new(
        docker,
        docker_pid,
        authority_sha256,
        tasks,
        runtime_pin,
        oci_pin,
        curl_sha256,
        trusted_gid0,
    )?;
    attempt.image = Some(ImageMetadata::read(
        &recipe.startup,
        manifest,
        image_config,
    )?);
    run_mode(
        admission,
        root,
        docker,
        recipe,
        source,
        Some(attempt),
        KernelTask::open,
    )
}
