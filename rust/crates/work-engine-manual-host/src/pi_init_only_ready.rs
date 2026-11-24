//! Checking one captured Contract B init-only attempt.
//!
//! The checker accepts protected capture records, not caller authority. The
//! private acquisition feature composes this checker with a wire/reader path;
//! neither that composition nor a passing fixture selects an installation or
//! confers a capability to do anything next.

#[cfg(all(target_os = "linux", feature = "private-init-only-acquisition"))]
#[cfg_attr(
    not(test),
    allow(dead_code, reason = "no installed selector; private offline cut")
)]
#[path = "pi_init_only_ready/acquisition.rs"]
pub(crate) mod acquisition;
#[path = "pi_init_only_ready/artifact_contract.rs"]
mod artifact_contract;
#[path = "pi_init_only_ready/launch.rs"]
mod launch;
#[cfg(test)]
#[path = "pi_init_only_ready/tests.rs"]
mod tests;
#[path = "pi_init_only_ready/wire.rs"]
mod wire;

use artifact_contract::HOLD_SECONDS;
pub use launch::*;
use serde::Serialize;
use serde_json::json;
use slice_campaign::codec::campaign_digest;
use slice_campaign::offline_qualification::{
    AttemptBinding, ConfirmedRunning, FixtureCheckpoint, FixtureConfinement,
    FixtureEffectiveReadback, FixtureExecReadback, FixtureHeldReadbacks, FixtureReadyPeer,
    FixtureRunning, FixtureTerminalSubject, HoldFailure, JoinedStartRequest, OfflineStopSubject,
    OfflineStopTerminalReadbacks, QualificationPort, TerminalFailure, fixture_init_policy,
    fixture_listener_id,
};

const NS_PER_SECOND: u64 = 1_000_000_000;
const MAX_EVENTS: usize = 1024;
const MAX_STREAM_BYTES: usize = 1024;
// Pin one wire profile to keep the candidate narrow; no negotiation/fallback.
const API: &str = "v1.44";
const MOBY: &str = "6a43e3d5afddf4111da0f864bbc7cae5d7e95001";

#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    Recipe,
    Create,
    Subject,
    Prearm,
    Upgrade,
    CrossedStream,
    EarlyStream,
    ContaminatedStream,
    StreamUnavailable,
    Start,
    Task,
    Effective,
    NotCurrent,
    Owner(HoldFailure),
}

fn full_hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Protected serving identity. Exact equality is required across sampled
/// roles, with the approved serving digest selected by the existing owner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Serving {
    pub boot: [u8; 16],
    pub docker_socket_inode: u64,
    pub daemon_process_start_ticks: u64,
    pub daemon_artifact_sha256: String,
    pub moby_semantics_revision: String,
    pub runtime_generation: String,
    pub api: String,
}

impl Serving {
    pub fn digest(&self) -> Result<String, Refusal> {
        if self.boot == [0; 16]
            || self.docker_socket_inode == 0
            || self.daemon_process_start_ticks == 0
            || !full_hex(&self.daemon_artifact_sha256)
            || self.moby_semantics_revision != MOBY
            || !full_hex(&self.runtime_generation)
            || self.api != API
        {
            return Err(Refusal::Prearm);
        }
        campaign_digest(&json!(["b-init-only-serving-v1", self])).map_err(|_| Refusal::Subject)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationLinks {
    pub selection: String,
    pub create_request: String,
    pub create_mark: String,
    pub start_request: String,
    pub start_mark: String,
}

/// Comparable physical identity: read while live, never inferred from an
/// inspect PID or from the desired task. A retained anchor is a premise of the
/// capture (the IDs here are its historical coordinates, not handles).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TaskKey {
    pub serving: Serving,
    pub container: String,
    pub task_id: String,
    pub pid: u32,
    pub start_ticks: u64,
    pub pid_namespace: u64,
    pub exec_generation: String,
    pub runtime_namespace: String,
    /// Protected slot retaining the original cgroup handle, not a hash of
    /// a desired task, an inode, or a lookup of the current path.
    pub subtree_capture: String,
    pub subtree_mount_id: u64,
    pub subtree_inode: u64,
}

impl TaskKey {
    /// Bridge into the *existing* synthetic Campaign type for the owner handoff.
    /// The lifetime encoding hashes the observed kernel coordinates, not the
    /// requested object. Ready's comparable key still retains the raw tuple.
    pub fn fixture_running(&self, start: &JoinedStartRequest) -> Result<FixtureRunning, Refusal> {
        if !full_hex(&self.container)
            || !full_hex(&self.task_id)
            || !full_hex(&self.exec_generation)
            || self.pid == 0
            || self.start_ticks == 0
            || self.pid_namespace == 0
            || self.runtime_namespace != "moby"
            || !full_hex(&self.subtree_capture)
            || self.subtree_mount_id == 0
            || self.subtree_inode == 0
            || self.container != start.target.full_id
            || self.serving.digest()? != start.target.serving_generation
        {
            return Err(Refusal::Task);
        }
        let lifetime = campaign_digest(&json!([
            "b-init-only-kernel-lifetime-v1",
            self.serving.boot,
            self.serving.runtime_generation,
            self.pid,
            self.start_ticks,
            self.pid_namespace
        ]))
        .map_err(|_| Refusal::Task)?;
        Ok(FixtureRunning {
            start_request_digest: start.request_digest.clone(),
            object: start.target.clone(),
            task_id: self.task_id.clone(),
            lifetime,
            exec_generation: self.exec_generation.clone(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Attach {
    pub serving: Serving,
    /// One capture-local stream ID allocated on open (not an fd number).
    pub connection: u64,
    pub resolved_full_id: String,
    pub http_request: Vec<u8>,
    pub registration: Registration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Registration {
    /// Protected source associated the authenticated peer, serving code, full
    /// object and this connection with the reviewed registration semantics.
    AuthenticatedPinnedSameObject,
    UnboundOrDifferent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StartRpc {
    pub serving: Serving,
    pub full_id: String,
    pub links: OperationLinks,
    /// Protected request/reply correspondence, independently of attach stream.
    pub exchange: u64,
    pub request_target: String,
}

#[derive(Serialize)]
pub struct Sample {
    /// One protected monotonic clock. Vector order is the capture order,
    /// including events with the same tick.
    pub at_ns: u64,
    pub event: Event,
}

#[derive(Serialize)]
pub enum Event {
    Open(Attach),
    Bytes {
        connection: u64,
        chunk: Vec<u8>,
    },
    Issued(StartRpc),
    Reply {
        rpc: StartRpc,
        status: u16,
        body: Vec<u8>,
    },
    /// Original runtime-selected, kernel-anchored task while live. Retain
    /// these exact coordinates through the independent current Held cut.
    Selected(Box<TaskKey>),
    /// These invalidate the capture even if the expected marker was seen.
    EofResetGapOrBackpressure,
    CancelledOrRevoked,
    Cut(Box<CurrentCut>),
}

#[derive(Serialize)]
pub struct CurrentCut {
    pub launch: ResolvedLaunch,
    pub effective: Effective,
    pub anchors: Anchors,
    pub drain: Drain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Anchors {
    /// Protected runtime/kernel cut retains the original live task + exec,
    /// mount and subtree during the applied sample and through the cut.
    SameExecutionLiveAndRetained,
    GoneChangedOrUnestablished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Drain {
    /// Same continuously drained protected stream, no feed gap, queue loss,
    /// backpressure, replay or reset through the cut. No collector cancellation
    /// won at the cut. Not derivable merely from seeing these marker bytes.
    UninterruptedAtCut,
    Unestablished,
}

pub struct FixtureInput<'a> {
    pub recipe: &'a Recipe,
    pub serving: &'a Serving,
    pub create: &'a CreateTrace,
    pub start: &'a JoinedStartRequest,
    pub samples: &'a [Sample],
    /// End of the protected capture/drain horizon. Cannot choose an older cut
    /// while ignoring a later fault already present in the transcript.
    pub observed_through_ns: u64,
}

/// Historical key only (no operation/release methods or current permit).
#[derive(Debug, PartialEq, Eq)]
pub struct HeldHistory {
    links: OperationLinks,
    execution: TaskKey,
    running: FixtureRunning,
    observed_at_ns: u64,
}

impl HeldHistory {
    pub fn links(&self) -> &OperationLinks {
        &self.links
    }
    pub fn execution(&self) -> &TaskKey {
        &self.execution
    }
    pub fn observed_at_ns(&self) -> u64 {
        self.observed_at_ns
    }

    fn capture_sha256(&self) -> Result<String, Refusal> {
        campaign_digest(&json!([
            "b-init-only-held-capture-v1",
            self.links,
            self.execution,
            self.observed_at_ns
        ]))
        .map_err(|_| Refusal::Subject)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum JoinRefusal {
    CrossedExecution,
    HeldProjection(Refusal),
    Owner(TerminalFailure),
}

/// Boot ID supplied to the Terminal fixture: 32 lowercase hex characters,
/// in order from the same 16 observed bytes (no dashes or endian conversion).
fn boot_key(boot: &[u8; 16]) -> String {
    use std::fmt::Write as _;
    let mut hex = String::with_capacity(32);
    for b in boot {
        write!(&mut hex, "{b:02x}").expect("write to String");
    }
    hex
}

/// Check historical Held and the exact archived scoped Terminal on one
/// execution. Not a currentness check, a Stop receipt or an effect permit.
/// Terminal capture positions and Ready's monotonic timestamps are distinct
/// source coordinates; only the owner's persisted frontier orders Held and
/// the later Close here.
pub fn qualify_fixture(
    port: &QualificationPort,
    binding: &AttemptBinding,
    held: &HeldHistory,
    stop: &OfflineStopSubject,
    terminal: &OfflineStopTerminalReadbacks,
) -> Result<(), JoinRefusal> {
    let links = &held.links;
    let key = &held.execution;
    let execution = &stop.execution;
    let live = &execution.live;
    if links.selection != binding.binding_digest()
        || links.selection != execution.selection
        || links.create_request != execution.create_request
        || links.create_mark != execution.create_mark
        || links.start_mark != execution.start_mark
        || links.start_request != live.running.start_request_digest
        || held.running != live.running
        || boot_key(&key.serving.boot) != live.boot
        || key.runtime_namespace != live.runtime_namespace
        || key.subtree_capture != live.cgroup.capture
        || key.subtree_mount_id != live.cgroup.mount_id
        || key.subtree_inode != live.cgroup.inode
    {
        return Err(JoinRefusal::CrossedExecution);
    }
    let projection = fixture_held_projection(binding, held).map_err(JoinRefusal::HeldProjection)?;
    let capture_sha256 = held.capture_sha256().map_err(JoinRefusal::HeldProjection)?;
    port.confirm_offline_init_only_join(binding, stop, terminal, &projection, &capture_sha256)
        .map_err(JoinRefusal::Owner)
}

fn attach_request(s: &Serving, id: &str) -> Vec<u8> {
    format!("POST /{}/containers/{id}/attach?logs=0&stream=1&stdin=0&stdout=1&stderr=0 HTTP/1.1\r\nHost: docker\r\nConnection: Upgrade\r\nUpgrade: tcp\r\nContent-Length: 0\r\n\r\n", s.api).into_bytes()
}

fn capture(
    input: FixtureInput<'_>,
    selection: &str,
    recipe_digest: &str,
    owner: &FixtureTerminalSubject,
) -> Result<HeldHistory, Refusal> {
    let FixtureInput {
        recipe,
        serving,
        create,
        start,
        samples,
        observed_through_ns,
    } = input;
    let object = &start.target.full_id;
    if !full_hex(object)
        || serving.digest()? != start.target.serving_generation
        || recipe.digest()? != recipe_digest
        || owner.selection != selection
        || owner.create_mark != start.create_mark_digest
        || owner.running.object != start.target
        || owner.running.start_request_digest != start.request_digest
    {
        return Err(Refusal::Subject);
    }
    let links = OperationLinks {
        selection: selection.to_owned(),
        create_request: start.create_request_digest.clone(),
        create_mark: owner.create_mark.clone(),
        start_request: start.request_digest.clone(),
        start_mark: owner.start_mark.clone(),
    };
    if ![
        &links.selection,
        &links.create_request,
        &links.create_mark,
        &links.start_request,
        &links.start_mark,
    ]
    .into_iter()
    .all(|v| full_hex(v))
    {
        return Err(Refusal::Subject);
    }
    recipe.verify_create(create, serving, object, &links.create_request)?;

    if samples.len() > MAX_EVENTS {
        return Err(Refusal::StreamUnavailable);
    }
    let Some((open, tail)) = samples.split_first() else {
        return Err(Refusal::Prearm);
    };
    let Event::Open(attach) = &open.event else {
        return Err(Refusal::Prearm);
    };
    if attach.serving != *serving
        || attach.connection == 0
        || attach.resolved_full_id != *object
        || attach.http_request != attach_request(serving, object)
        || attach.registration != Registration::AuthenticatedPinnedSameObject
    {
        return Err(Refusal::Prearm);
    }
    let mut wire = wire::Stream::default();
    let mut taken = 0usize;
    let mut issued = None;
    let mut acknowledged = false;
    let mut selected = None;
    let mut last = open.at_ns;
    for (i, sample) in tail.iter().enumerate() {
        if sample.at_ns < last || sample.at_ns > observed_through_ns {
            return Err(Refusal::NotCurrent);
        }
        last = sample.at_ns;
        match &sample.event {
            Event::Open(_) => return Err(Refusal::CrossedStream),
            Event::EofResetGapOrBackpressure => return Err(Refusal::StreamUnavailable),
            Event::CancelledOrRevoked => return Err(Refusal::NotCurrent),
            Event::Bytes { connection, chunk } => {
                if *connection != attach.connection {
                    return Err(Refusal::CrossedStream);
                }
                if chunk.is_empty() || chunk.len() > MAX_STREAM_BYTES - taken {
                    return Err(Refusal::StreamUnavailable);
                }
                taken += chunk.len();
                wire.feed(chunk, issued.is_some())?;
            }
            Event::Issued(rpc) => {
                if issued.is_some()
                    || !wire.upgraded()
                    || rpc.serving != *serving
                    || rpc.full_id != *object
                    || rpc.links != links
                    || rpc.exchange == 0
                    || rpc.request_target
                        != format!("POST /{}/containers/{object}/start", serving.api)
                    || sample.at_ns - open.at_ns > HOLD_SECONDS * NS_PER_SECOND
                {
                    return Err(Refusal::Start);
                }
                issued = Some((rpc, sample.at_ns));
            }
            Event::Reply { rpc, status, body } => {
                if acknowledged
                    || issued.map(|(r, _)| r) != Some(rpc)
                    || *status != 204
                    || !body.is_empty()
                {
                    return Err(Refusal::Start);
                }
                acknowledged = true;
            }
            Event::Selected(task) => {
                if issued.is_none()
                    || selected.is_some()
                    || task.serving != *serving
                    || task.fixture_running(start)? != owner.running
                {
                    return Err(Refusal::Task);
                }
                selected = Some(task.as_ref());
            }
            Event::Cut(cut) => {
                let Some((_, issued_at)) = issued else {
                    return Err(Refusal::Start);
                };
                if !acknowledged
                    || i + 1 != tail.len()
                    || sample.at_ns != observed_through_ns
                    || sample.at_ns - issued_at >= HOLD_SECONDS * NS_PER_SECOND
                    || cut.anchors != Anchors::SameExecutionLiveAndRetained
                {
                    return Err(Refusal::NotCurrent);
                }
                if !wire.complete() || cut.drain != Drain::UninterruptedAtCut {
                    return Err(Refusal::StreamUnavailable);
                }
                if selected != Some(&cut.launch.task) || !cut.launch.matches(recipe) {
                    return Err(Refusal::Task);
                }
                if !cut.effective.matches(recipe, &cut.launch) {
                    return Err(Refusal::Effective);
                }
                return Ok(HeldHistory {
                    links,
                    execution: cut.launch.task.clone(),
                    running: owner.running.clone(),
                    observed_at_ns: sample.at_ns,
                });
            }
        }
    }
    Err(Refusal::NotCurrent)
}

/// Compose the checked capture with the exact one-shot Campaign continuation.
/// The caller has already separately confirmed the running task (so a bad
/// stream does not erase a known Start). The Campaign store owns the marks,
/// currentness and final fixture fact. The fixture projection below is only a
/// bridge into its pre-existing synthetic test policy; it is not OS evidence.
pub fn verify_fixture(
    port: &QualificationPort,
    binding: &AttemptBinding,
    running: &FixtureRunning,
    confirmation: ConfirmedRunning,
    input: FixtureInput<'_>,
) -> Result<HeldHistory, Refusal> {
    if input.serving.digest()? != binding.serving_sha256()
        || binding
            .create_request_digest()
            .map_err(|_| Refusal::Subject)?
            != input.start.create_request_digest
    {
        return Err(Refusal::Subject);
    }
    let owner = port
        .fixture_terminal_subject(binding, running)
        .map_err(|_| Refusal::Subject)?;
    let checked = capture(
        input,
        binding.binding_digest(),
        binding.recipe_sha256(),
        &owner,
    )?;
    let projection = fixture_held_projection(binding, &checked)?;
    confirmation
        .verify_init_only_held(
            port,
            binding,
            running,
            &projection,
            &checked.capture_sha256()?,
        )
        .map_err(Refusal::Owner)?;
    Ok(checked)
}

// Same projection for initial checked write and historical readback. These
// are the existing synthetic policy values, not the protected applied inputs
// verified above, nor a reconstruction of an execution or a live handle.
fn fixture_held_projection(
    binding: &AttemptBinding,
    checked: &HeldHistory,
) -> Result<FixtureHeldReadbacks, Refusal> {
    let policy = fixture_init_policy();
    let listener =
        fixture_listener_id(binding, &checked.links.start_request).map_err(|_| Refusal::Subject)?;
    let running = &checked.running;
    Ok(FixtureHeldReadbacks {
        exec: Some(FixtureExecReadback {
            running: running.clone(),
            executable_sha256: policy.executable_sha256,
            startup_closure_sha256: policy.startup_closure_sha256,
        }),
        ready: Some(FixtureReadyPeer {
            listener,
            peer: running.clone(),
            checkpoint: FixtureCheckpoint::HeldBeforeWorkload,
        }),
        confinement: FixtureConfinement::Observed(FixtureEffectiveReadback {
            running: running.clone(),
            credentials: Some(policy.credentials),
            isolation: Some(policy.isolation),
        }),
    })
}
