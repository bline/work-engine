//! Private init-only controller. No Server entry, Builder, release, recovery
//! sender, or default selection. Only the opt-in foreground selector supplies
//! the installed natural-exit source.

#[cfg(target_arch = "x86_64")]
#[path = "acquisition/containerd.rs"]
mod containerd;
#[path = "acquisition/custody.rs"]
mod custody;
#[path = "acquisition/docker.rs"]
mod docker;
#[cfg(target_arch = "x86_64")]
#[path = "acquisition/image.rs"]
mod image;
#[path = "acquisition/kernel.rs"]
mod kernel;
#[cfg(target_arch = "x86_64")]
#[path = "acquisition/launch_state.rs"]
mod launch_state;
#[cfg(target_arch = "x86_64")]
#[path = "acquisition/natural.rs"]
mod natural;
#[cfg(all(target_arch = "x86_64", feature = "private-natural-exit-foreground"))]
pub use natural::run_natural_exit_foreground;
#[cfg(target_arch = "x86_64")]
#[path = "acquisition/policy.rs"]
mod policy;
#[cfg(target_arch = "x86_64")]
#[path = "acquisition/retirement.rs"]
mod retirement;
#[cfg(test)]
#[path = "acquisition/tests.rs"]
mod tests;

use super::{
    Anchors, Attach, CreateTrace, CurrentCut, Drain, Effective, Event, FixtureInput,
    OperationLinks, Recipe, Registration, ResolvedLaunch, Sample, Serving, StartRpc, TaskKey,
    attach_request, boot_key, full_hex, launch::create_reply_id, qualify_fixture, verify_fixture,
    wire,
};
use crate::{Error, Result, refuse};
pub(crate) use custody::{ArchiveState, CurrentDisposition, History, Operation};
use custody::{Custody, Kind};
pub(crate) use docker::Endpoint;
use kernel::{Anchor, KernelTask};
use serde::{Deserialize, Serialize};
use serde_json::json;
#[cfg(all(target_arch = "x86_64", feature = "private-natural-exit-foreground"))]
use slice_campaign::offline_qualification::foreground::ForegroundSelection;
use slice_campaign::{
    codec::campaign_digest,
    offline_qualification::{
        AttemptBinding, FixtureCreateReadback, FixtureObject, JoinedMarkReadback,
        OfflineCgroupHandle, OfflineClosedFrontier, OfflineContainment, OfflineLiveSubtree,
        OfflineNewIngress, OfflineOldTask, OfflineRecoveryAuthority, OfflineStopReadback,
        OfflineStopResponse, OfflineStopSubject, OfflineStopTerminalReadbacks, OfflineSubtreeEnd,
        QualificationHistoryKey, QualificationPort, QualificationPurpose, SpentQualification,
    },
};
use std::{
    io,
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd},
        unix::{ffi::OsStrExt, fs::MetadataExt, net::UnixStream},
    },
    path::Path,
    time::{Duration, Instant},
};

const PREPARE: Duration = Duration::from_secs(8);
const RPC_LIMIT: Duration = Duration::from_secs(4);
const HOLD_CAPTURE: Duration = Duration::from_secs(12);
const SETTLEMENT: Duration = Duration::from_secs(8);
const STOP_QUERY: &str = "signal=SIGTERM&t=2";

/// An in-process, reviewed, attempt-scoped reader wired by a future trusted
/// selector, never a host/client-supplied JSON assertion. No Docker or
/// containerd mutation authority is conferred here. Return unavailable for
/// anything not independently supported by the trusted runtime/kernel. The
/// implementation must honor the deadlines and preserve evidence coordinates.
/// In particular stock Inspect cannot implement `selected`/`applied` or the
/// accepted-work/recovery cut. The current tree provides no such selector.
pub(crate) trait ProtectedSource {
    type StartFence: StartFence;
    type StopFence: StopFence;

    fn identity(&self) -> &str;
    fn preflight(
        &mut self,
        binding: &AttemptBinding,
        endpoint: &Endpoint,
        deadline: Instant,
    ) -> Result<()>;
    fn serving(&mut self, endpoint: &Endpoint, deadline: Instant) -> Result<Serving>;
    /// Establish actual broadcaster registration and exclusive object/Start
    /// resolution on these live sockets. A cookie is a correlation handle,
    /// not proof of what the daemon resolved. Do not read/write either socket.
    /// From registration, no task/output may precede acceptance of *this*
    /// Start; any unobservable boundary or competing producer is a refusal.
    fn fence_start(
        &mut self,
        start: &StartRpc,
        stdout: BorrowedFd<'_>,
        request: BorrowedFd<'_>,
        deadline: Instant,
    ) -> Result<(Attach, Self::StartFence)>;
    fn selected(&mut self, start: &StartRpc, deadline: Instant) -> Result<SelectedTask>;
    fn applied(
        &mut self,
        start: &StartRpc,
        attach: &Attach,
        task: &TaskKey,
        deadline: Instant,
    ) -> Result<AppliedWitness>;
    /// Read the already-effective resolution restriction for this exchange.
    /// Do not send on the descriptor. The restriction survives loss of this
    /// descriptor, client, returned fence and controller. The fence separately
    /// supplies the containment readback for Terminal.
    fn fence_stop(
        &mut self,
        exchange: &StopExchange,
        request: BorrowedFd<'_>,
        deadline: Instant,
    ) -> Result<(StopResolution, Self::StopFence)>;
    /// Distinct explicit authorization from the qualification operator. The
    /// private selector may consult that owner, but must not approve from a
    /// timeout, failure, normal launch grant, or controller default.
    #[cfg(target_arch = "x86_64")]
    fn natural_abort_approval(
        &mut self,
        _subject: &natural::AbortSubject,
        _deadline: Instant,
    ) -> Result<Option<natural::AbortApproval>> {
        Ok(None)
    }
    /// Each response is a new checked observation in the original capture.
    fn settlement(
        &mut self,
        stop: &OfflineStopSubject,
        task: &TaskKey,
        phase: Phase,
        deadline: Instant,
    ) -> Result<ScopeWitness>;
}

pub(crate) trait StartFence {
    /// Corroborate the controller's original kernel/stream cut: this Start
    /// was the sole possible producer, with the original live execution,
    /// launch/root, and uninterrupted, uncancelled stream through the cut.
    /// The caller retains the original handles and this fence. Return coverage
    /// independently attributable to the *entire* applied witness, including
    /// effective policy, inventory and descriptors at that cut.
    fn confirm_cut(
        &mut self,
        capture: &HeldCapture,
        cut: Instant,
        deadline: Instant,
    ) -> Result<AppliedCoverage>;
}

pub(crate) trait StopFence {
    /// Confirm actual continuous containment and closed ingress/recovery
    /// across the evidence collection, including this original end read.
    /// Checked only *after* the kernel read completes, not a promise about it.
    fn confirm_end(
        &mut self,
        read: (Instant, Instant),
        deadline: Instant,
    ) -> Result<OfflineContainment>;
}

/// Readbacks carry correspondence from the protected source, not just the
/// expected identity supplied in the query.
#[derive(Serialize)]
pub(crate) struct SelectedTask {
    pub start: StartRpc,
    pub task: TaskKey,
}

#[derive(Serialize)]
pub(crate) struct AppliedWitness {
    pub start: StartRpc,
    pub attach: Attach,
    pub launch: ResolvedLaunch,
    pub effective: Effective,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct AppliedRecord {
    pub position: u64,
    /// Hash of the archived AppliedWitness payload (Start, attachment,
    /// launch/pipe/root, task/subtree and complete effective state).
    pub sha256: String,
}

#[derive(Serialize)]
pub(crate) struct HeldCapture {
    pub position: u64,
    pub applied: AppliedRecord,
    /// The raw original-task readbacks linked to this cut. In natural mode
    /// this same record includes startup (including effective env), isolation,
    /// and stdio; none of these are inferred from the applied payload.
    pub kernel_position: u64,
    pub at_ns: u64,
}

pub(crate) struct AppliedCoverage {
    pub source: String,
    pub capture_position: u64,
    pub applied: AppliedRecord,
    /// Reference to the authoritative observation/stability evidence in
    /// protected custody. Neither this digest nor payload equality proves
    /// coverage by itself. The selected reader must establish the interval
    /// from the source's evidence, with a checked mapping to this clock, not
    /// by copying the query's timestamp or with two unjoined point readings.
    pub evidence_sha256: String,
    pub from: Instant,
    pub through: Instant,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct StopExchange {
    pub subject: OfflineStopSubject,
    pub task: TaskKey,
    pub request_target: String,
    pub connection: u64,
}

#[derive(Serialize)]
pub(crate) struct StopResolution {
    pub source: String,
    pub exchange: StopExchange,
    pub scope: ResolutionScope,
}

#[derive(Serialize)]
pub(crate) enum ResolutionScope {
    /// An already enforced original-only-or-inert restriction on the actual
    /// serving handler. It lasts until the handler has fixed the original
    /// target with no later retargeting, or this submission can no longer
    /// select or act. Serving changes must preserve that property or make
    /// the request inert. A local timeout, error, task exit or drop is not
    /// such a boundary. The reference identifies the actual enforced scope.
    ThroughHandlerResolutionOrInert {
        enforcement_sha256: String,
    },
    LocalChannelOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Ingress,
    OldTask,
    Accepted,
    Recovery,
}

#[derive(Serialize)]
pub(crate) enum ScopeFact {
    Ingress(OfflineNewIngress),
    OldTask(OfflineOldTask),
    Accepted(OfflineClosedFrontier),
    Recovery(OfflineRecoveryAuthority),
}

#[derive(Serialize)]
pub(crate) struct ScopeWitness {
    pub subject: OfflineStopSubject,
    pub task: TaskKey,
    pub fact: ScopeFact,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Verdict {
    pub held: bool,
    pub terminal: bool,
    pub qualified: bool,
}

fn invalid(e: impl std::fmt::Debug) -> Error {
    Error::Refused(format!("private init-only acquisition: {e:?}"))
}

/// Descriptive only. Use the same representation in the intent preimage
/// and the protected recovery descriptor; never deserialize a live endpoint.
#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct EndpointIdentity {
    path_bytes: Vec<u8>,
    device_be: [u8; 8],
    inode_be: [u8; 8],
    listener_pid: i32,
    listener_uid: u32,
    listener_gid: u32,
}

impl From<&Endpoint> for EndpointIdentity {
    fn from(endpoint: &Endpoint) -> Self {
        Self {
            path_bytes: endpoint.path.as_os_str().as_bytes().to_vec(),
            device_be: endpoint.device.to_be_bytes(),
            inode_be: endpoint.inode.to_be_bytes(),
            listener_pid: endpoint.listener_pid,
            listener_uid: endpoint.listener_uid,
            listener_gid: endpoint.listener_gid,
        }
    }
}

fn intent_preimage(
    root: &Path,
    device: u64,
    inode: u64,
    endpoint: &EndpointIdentity,
    source: &str,
) -> Result<serde_json::Value> {
    if !full_hex(source) {
        return refuse("unbound protected reader");
    }
    // Campaign canonicalizes numbers as f64. Encode each filesystem u64
    // in its own fixed-width big-endian byte field, never as a JSON number.
    // Unix paths are byte strings, not necessarily Unicode.
    Ok(serde_json::to_value((
        "b-private-init-only-acquisition-v4",
        (
            root.as_os_str().as_bytes(),
            device.to_be_bytes(),
            inode.to_be_bytes(),
        ),
        endpoint,
        source,
        STOP_QUERY,
        PREPARE.as_secs(),
        RPC_LIMIT.as_secs(),
        HOLD_CAPTURE.as_secs(),
        SETTLEMENT.as_secs(),
    ))?)
}

fn pinned_intent(root: &Path, endpoint: &EndpointIdentity, source: &str) -> Result<String> {
    let m = custody::check_root(root)?.metadata()?;
    campaign_digest(&intent_preimage(root, m.dev(), m.ino(), endpoint, source)?).map_err(invalid)
}

/// Existing Campaign intent pin, not a second grant. Couples the exact
/// observer and endpoint to the small fixed wire/timeout budget.
pub(crate) fn intent(root: &Path, endpoint: &Endpoint, source: &str) -> Result<String> {
    pinned_intent(root, &endpoint.into(), source)
}

/// This is the sole real invocation shape; KernelTask::open is fixed, not a
/// selectable fallback. The private owner must supply a selected binding.
pub(crate) fn run(
    port: &QualificationPort,
    binding: &AttemptBinding,
    root: &Path,
    endpoint: &Endpoint,
    recipe: &Recipe,
    source: &mut impl ProtectedSource,
) -> Result<Verdict> {
    run_at(
        port,
        binding,
        root,
        endpoint,
        recipe,
        source,
        KernelTask::open,
    )
}

/// Read the exact Campaign spend and the archive. No socket, task lookup,
/// owner write, spend/mark continuation, or inferred success on this path.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecoverySelection {
    key: QualificationHistoryKey,
    endpoint: EndpointIdentity,
    source: String,
    #[cfg(target_arch = "x86_64")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    natural: Option<natural::Identity>,
}

impl RecoverySelection {
    /// Description only; the future protected owner may retain it outside
    /// the attempt directory so even a missing first record is diagnosable.
    pub(crate) fn new(binding: &AttemptBinding, endpoint: &Endpoint, source: &str) -> Self {
        Self {
            key: binding.history_key(),
            endpoint: endpoint.into(),
            source: source.into(),
            #[cfg(target_arch = "x86_64")]
            natural: None,
        }
    }
}

pub(crate) fn recover(
    port: &QualificationPort,
    selection: &RecoverySelection,
    root: &Path,
) -> Result<History> {
    let spent = port
        .read_qualification_spend(&selection.key)
        .map_err(invalid)?;
    // Recompute the Campaign-pinned intent from archive filesystem identity
    // and protected selection descriptions; never resolve/contact the endpoint.
    let pinned = match selection.key.purpose {
        QualificationPurpose::OneStop => {
            pinned_intent(root, &selection.endpoint, &selection.source)
        }
        QualificationPurpose::NaturalExit => {
            #[cfg(target_arch = "x86_64")]
            {
                selection.natural.as_ref().map_or_else(
                    || refuse("missing historical natural-exit intent"),
                    |natural| natural.intent(root, &selection.endpoint, &selection.source),
                )
            }
            #[cfg(not(target_arch = "x86_64"))]
            refuse("natural-exit custody is unavailable on this architecture")
        }
    };
    match pinned {
        Ok(digest) if digest == selection.key.intent => {
            Ok(custody::history(root, &selection.key, spent))
        }
        _ => Ok(History::unavailable(spent)),
    }
}

fn capture_ns(clock: Instant, time: Instant) -> Result<u64> {
    let elapsed = time
        .checked_duration_since(clock)
        .ok_or_else(|| invalid("observation precedes this capture clock"))?;
    u64::try_from(elapsed.as_nanos()).map_err(invalid)
}

fn within(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        return refuse("attempt phase deadline exceeded");
    }
    Ok(())
}

fn serving_now(
    source: &mut impl ProtectedSource,
    endpoint: &Endpoint,
    expected: &Serving,
    journal: &mut Custody,
    deadline: Instant,
) -> Result<()> {
    within(deadline)?;
    let observed = source.serving(endpoint, deadline)?;
    journal.record(Kind::Serving, json!(&observed))?;
    if observed != *expected {
        return refuse("serving generation changed");
    }
    within(deadline)
}

fn pin_effect_stream(
    journal: &mut Custody,
    io: &mut UnixStream,
    op: Operation,
    sender: Option<docker::Sender>,
    deadline: Instant,
) -> Result<()> {
    let Some(sender) = sender else {
        return Ok(());
    };
    docker::authenticate(io, Some(sender))?;
    let cookie = docker::socket_cookie(io)?;
    docker::write(io, docker::STOCK_PROBE, deadline)?;
    let observed = docker::receive(io, deadline, Some(sender));
    journal.record(
        Kind::Serving,
        json!({"before_effect":op, "socket_cookie":cookie,
        "request":docker::STOCK_PROBE,"observed":observed}),
    )?;
    if !observed
        .response
        .as_ref()
        .is_some_and(docker::is_stock_version)
    {
        return refuse("Docker effect stream did not identify the original pinned daemon");
    }
    within(deadline)
}

fn effect(
    journal: &mut Custody,
    mut io: UnixStream,
    op: Operation,
    mark: &str,
    request: &[u8],
    deadline: Instant,
    sender: Option<docker::Sender>,
) -> Result<(bool, Option<docker::Response>)> {
    pin_effect_stream(journal, &mut io, op, sender, deadline)?;
    journal.record(Kind::Armed(op), json!({"mark":mark}))?;
    let written = match docker::write(&mut io, request, deadline) {
        Ok(()) => {
            journal.record(Kind::Written(op), json!(null))?;
            true
        }
        Err(e) => {
            journal.record(
                Kind::Reply(op),
                json!({"write_error":e.to_string(), "response":null}),
            )?;
            return Ok((false, None));
        }
    };
    let observation = docker::receive(&mut io, deadline, sender);
    journal.record(Kind::Reply(op), json!(&observation))?;
    Ok((written, observation.response))
}

fn run_at<A: Anchor>(
    port: &QualificationPort,
    binding: &AttemptBinding,
    root: &Path,
    endpoint: &Endpoint,
    recipe: &Recipe,
    source: &mut impl ProtectedSource,
    pin: impl FnOnce(&TaskKey) -> io::Result<A>,
) -> Result<Verdict> {
    run_mode(
        Admission::Fixture(port, binding),
        root,
        endpoint,
        recipe,
        source,
        #[cfg(target_arch = "x86_64")]
        None,
        pin,
    )
}

enum Admission<'a> {
    Fixture(&'a QualificationPort, &'a AttemptBinding),
    #[cfg(all(target_arch = "x86_64", feature = "private-natural-exit-foreground"))]
    Foreground(&'a ForegroundSelection),
}

impl Admission<'_> {
    fn owner(&self) -> (&QualificationPort, &AttemptBinding) {
        match self {
            Self::Fixture(port, binding) => (port, binding),
            #[cfg(all(target_arch = "x86_64", feature = "private-natural-exit-foreground"))]
            Self::Foreground(selection) => (selection.port(), selection.binding()),
        }
    }

    fn approval_raw(&self) -> Option<&str> {
        match self {
            Self::Fixture(..) => None,
            #[cfg(all(target_arch = "x86_64", feature = "private-natural-exit-foreground"))]
            Self::Foreground(selection) => Some(selection.approval_raw()),
        }
    }

    fn spend(&self, journal: &mut Custody) -> Result<SpentQualification> {
        let (_, binding) = self.owner();
        let mut accept = |spent| {
            journal.record(Kind::Spent, json!(binding.grant_id()))?;
            Ok(spent)
        };
        match self {
            Self::Fixture(port, binding) => accept(
                port.enter(binding)
                    .and_then(|entry| entry.consume())
                    .map_err(invalid)?,
            ),
            #[cfg(all(target_arch = "x86_64", feature = "private-natural-exit-foreground"))]
            Self::Foreground(selection) => selection.handoff(accept).map_err(invalid)?,
        }
    }
}

fn run_mode<A: Anchor>(
    admission: Admission<'_>,
    root: &Path,
    endpoint: &Endpoint,
    recipe: &Recipe,
    source: &mut impl ProtectedSource,
    #[cfg(target_arch = "x86_64")] mut natural: Option<natural::Attempt<'_>>,
    pin: impl FnOnce(&TaskKey) -> io::Result<A>,
) -> Result<Verdict> {
    let (port, binding) = admission.owner();
    let clock = Instant::now();
    let prep = clock + PREPARE;
    let source_identity = source.identity().to_owned();
    let mut history = RecoverySelection::new(binding, endpoint, &source_identity);
    let expected_intent = match binding.purpose() {
        QualificationPurpose::OneStop => {
            #[cfg(target_arch = "x86_64")]
            if natural.is_some() {
                return refuse("one-Stop binding cannot select natural exit");
            }
            intent(root, endpoint, &source_identity)?
        }
        QualificationPurpose::NaturalExit => {
            #[cfg(target_arch = "x86_64")]
            {
                let selected = natural
                    .as_ref()
                    .ok_or_else(|| invalid("natural-exit selection required"))?;
                let identity = selected.identity();
                let digest = identity.intent(root, &history.endpoint, &source_identity)?;
                history.natural = Some(identity);
                digest
            }
            #[cfg(not(target_arch = "x86_64"))]
            return refuse("natural exit is unavailable on this architecture");
        }
    };
    if binding.intent_sha256() != expected_intent
        || binding.recipe_sha256() != recipe.digest().map_err(invalid)?
    {
        return refuse("private wire/source selection differs from Campaign");
    }
    // Preflight has no effect operation. Only Campaign can supply a fresh
    // verified spend; the capture slot is reserved before the spend call.
    endpoint.check_path()?;
    source.preflight(binding, endpoint, prep)?;
    let serving = source.serving(endpoint, prep)?;
    if serving.docker_socket_inode != endpoint.inode
        || binding.serving_sha256() != serving.digest().map_err(invalid)?
    {
        return refuse("selected serving evidence differs from Campaign");
    }
    within(prep)?;
    let mut journal = Custody::begin(root, binding.attempt_id())?;
    let mut selection = json!({
        "schema":"b-private-init-only-v4", "history_key":history.key,
        "recovery_selection":history, "operation":binding.operation_id(), "head":binding.head(),
        "recipe":recipe, "serving":serving,
    });
    if let Some(raw) = admission.approval_raw() {
        selection["foreground_approval_utf8"] = json!(raw);
    }
    journal.record(Kind::Selection, selection)?;
    #[cfg(target_arch = "x86_64")]
    if let Some(attempt) = natural.as_mut() {
        attempt.bind_scope(root, binding, &serving, &mut journal, prep)?;
        attempt.pin_serving(&serving, &mut journal, prep)?;
        attempt.pin_policy(recipe, &mut journal, prep)?;
        attempt.check_image(endpoint, &serving, &mut journal, prep)?;
    }
    #[cfg(target_arch = "x86_64")]
    let sender = natural
        .as_ref()
        .map(natural::Attempt::live_sender)
        .transpose()?;
    #[cfg(not(target_arch = "x86_64"))]
    let sender = None;
    let spent = admission.spend(&mut journal)?;
    let create_digest = binding.create_request_digest().map_err(invalid)?;
    let target = format!(
        "POST /{}/containers/create?platform=linux%2Famd64",
        serving.api
    );
    let body = recipe.create_body().map_err(invalid)?;
    let request = docker::request(&target, &body);
    journal.record(
        Kind::Intended(Operation::Create),
        json!({"request":request, "digest":create_digest}),
    )?;
    serving_now(source, endpoint, &serving, &mut journal, prep)?;
    let marked = spent
        .mark_joined_create(&create_digest, JoinedMarkReadback::Check)
        .map_err(invalid)?;
    #[cfg(target_arch = "x86_64")]
    if let Some(attempt) = natural.as_ref() {
        attempt.live_sender()?;
    }
    let (_, response) = effect(
        &mut journal,
        endpoint.connect()?,
        Operation::Create,
        &marked.mark_digest().map_err(invalid)?,
        &request,
        prep.min(Instant::now() + RPC_LIMIT),
        sender,
    )?;
    let response = response
        .filter(|r| r.status == 201 && r.body.len() <= 4096)
        .ok_or_else(|| invalid("exact Create reply unavailable; no continuation"))?;
    let id = create_reply_id(&response.body).map_err(invalid)?;
    journal.record(Kind::CreateIdentity, json!(&id))?;
    #[cfg(target_arch = "x86_64")]
    if let Some(attempt) = natural.as_ref() {
        attempt.live_sender()?;
    }
    serving_now(source, endpoint, &serving, &mut journal, prep)?;
    let inspect_request =
        docker::request(&format!("GET /{}/containers/{id}/json", serving.api), &[]);
    let mut inspect = endpoint.connect()?;
    docker::authenticate(&inspect, sender)?;
    docker::write(&mut inspect, &inspect_request, prep)?;
    let inspect = docker::receive(&mut inspect, prep, sender);
    journal.record(
        Kind::Inspect,
        json!({"request":inspect_request,"observed":inspect}),
    )?;
    let inspect = inspect
        .response
        .filter(|r| r.status == 200)
        .ok_or_else(|| invalid("exact created-object inspect unavailable"))?;
    serving_now(source, endpoint, &serving, &mut journal, prep)?;
    let trace = CreateTrace {
        serving: serving.clone(),
        campaign_request: create_digest.clone(),
        target,
        body,
        status: response.status,
        reply: response.body,
        inspect: inspect.body,
    };
    recipe
        .verify_create(&trace, &serving, &id, &create_digest)
        .map_err(invalid)?;
    let next = marked
        .confirm_created(
            &create_digest,
            &id,
            FixtureCreateReadback {
                request_digest: create_digest.clone(),
                object: FixtureObject {
                    full_id: id.clone(),
                    serving_generation: binding.serving_sha256().into(),
                },
            },
        )
        .map_err(invalid)?;
    #[cfg(target_arch = "x86_64")]
    if let Some(attempt) = natural.as_ref() {
        attempt.live_sender()?;
    }
    let checked_create = journal.record(Kind::CreateChecked, json!(&trace))?;
    #[cfg(not(target_arch = "x86_64"))]
    let _ = checked_create;
    let start = next.request().clone();
    let mut stdout = Stdout::open(&mut journal, endpoint, &serving, &id, clock, prep, sender)?;
    // The trusted source must still name the same daemon generation after
    // registration. A 101 on its own knows nothing about the task.
    serving_now(source, endpoint, &serving, &mut journal, prep)?;
    let target = format!("POST /{}/containers/{id}/start", serving.api);
    let request = docker::request(&target, &[]);
    journal.record(
        Kind::Intended(Operation::Start),
        json!({"request":request,"start":start}),
    )?;
    within(prep)?;
    let marked = next.mark(JoinedMarkReadback::Check).map_err(invalid)?;
    let start_mark = marked.mark_digest().map_err(invalid)?;
    let mut start_io = endpoint.connect()?;
    pin_effect_stream(
        &mut journal,
        &mut start_io,
        Operation::Start,
        sender,
        prep.min(Instant::now() + RPC_LIMIT),
    )?;
    let rpc = StartRpc {
        serving: serving.clone(),
        full_id: id.clone(),
        exchange: docker::socket_cookie(&start_io)?,
        request_target: target,
        links: OperationLinks {
            selection: binding.binding_digest().into(),
            create_request: create_digest,
            create_mark: start.create_mark_digest.clone(),
            start_request: start.request_digest.clone(),
            start_mark: start_mark.clone(),
        },
    };
    stdout.pending(&mut journal, false)?;
    let mut inherited_fence = || -> Result<_> {
        let (registered, fence) =
            source.fence_start(&rpc, stdout.io.as_fd(), start_io.as_fd(), prep)?;
        Ok((registered, Some(fence)))
    };
    #[cfg(target_arch = "x86_64")]
    let (registered, mut start_fence) = match natural.as_ref() {
        Some(attempt) => {
            // `Stdout::open` received the upgraded stream from the pinned
            // daemon. That path registers this exact object before the 101;
            // admission scope excludes alias, replacement and other producers.
            attempt.live_sender()?;
            (stdout.attach.clone(), None)
        }
        None => inherited_fence()?,
    };
    #[cfg(not(target_arch = "x86_64"))]
    let (registered, mut start_fence) = inherited_fence()?;
    if registered != stdout.attach || rpc.exchange == registered.connection {
        return refuse("prearm resolved a different object or exchange");
    }
    stdout.confirm_open(&mut journal, registered)?;
    #[cfg(target_arch = "x86_64")]
    if let Some(attempt) = natural.as_mut() {
        attempt.arm(&mut journal, checked_create, &rpc, &stdout.attach, prep)?;
    }
    let mut pinned = None;
    let outcome = (|| {
        #[cfg(target_arch = "x86_64")]
        if let Some(attempt) = natural.as_ref() {
            attempt.live_sender()?;
        }
        let attempting_at = Instant::now();
        let start_arm = journal.record(Kind::Armed(Operation::Start), json!({"rpc":rpc}))?;
        #[cfg(target_arch = "x86_64")]
        if let Some(attempt) = natural.as_mut() {
            attempt.start_armed(start_arm, attempting_at)?;
        }
        #[cfg(not(target_arch = "x86_64"))]
        let _ = (start_arm, attempting_at);
        let rpc_deadline = prep.min(Instant::now() + RPC_LIMIT);
        if let Err(e) = docker::write(&mut start_io, &request, rpc_deadline) {
            journal.record(
                Kind::Reply(Operation::Start),
                json!({"write_error":e.to_string()}),
            )?;
            return refuse("Start write unconfirmed");
        }
        // Capture the actual completed local submission, never the previous
        // emptiness check. Only the exclusive producer fence can prove causality
        // across the two sockets, including bytes buffered at this boundary.
        let issued = Instant::now();
        let start_written = journal.record(Kind::Written(Operation::Start), json!(null))?;
        #[cfg(not(target_arch = "x86_64"))]
        let _ = start_written;
        #[cfg(target_arch = "x86_64")]
        if let Some(attempt) = natural.as_mut() {
            attempt.start_written(start_written, issued)?;
        }
        stdout.sample(&mut journal, issued, Event::Issued(rpc.clone()))?;
        let reply = docker::receive_while(
            &mut start_io,
            rpc_deadline,
            sender,
            Some(stdout.io.as_raw_fd()),
            || stdout.pending(&mut journal, true).map_err(io::Error::other),
        );
        let reply_at = Instant::now();
        let start_reply = journal.record(Kind::Reply(Operation::Start), json!(&reply))?;
        #[cfg(not(target_arch = "x86_64"))]
        let _ = start_reply;
        #[cfg(target_arch = "x86_64")]
        if let Some(attempt) = natural.as_mut() {
            attempt.start_accepted(start_reply, reply_at, reply.response.as_ref())?;
        }
        if let Some(r) = reply.response {
            stdout.sample(
                &mut journal,
                reply_at,
                Event::Reply {
                    rpc: rpc.clone(),
                    status: r.status,
                    body: r.body,
                },
            )?;
        }
        let hold_deadline = issued + HOLD_CAPTURE;
        serving_now(source, endpoint, &serving, &mut journal, hold_deadline)?;
        #[cfg(target_arch = "x86_64")]
        let selected = match natural.as_mut() {
            Some(attempt) => {
                attempt.live_sender()?;
                attempt.selected(&mut journal, &rpc, hold_deadline)?
            }
            None => source.selected(&rpc, hold_deadline)?,
        };
        #[cfg(not(target_arch = "x86_64"))]
        let selected = source.selected(&rpc, hold_deadline)?;
        journal.record(Kind::Running, json!(&selected))?;
        let key = selected.task;
        if selected.start != rpc || key.serving != serving {
            return refuse("selected task crossed Start or serving");
        }
        within(hold_deadline)?;
        let running = key.fixture_running(&start).map_err(invalid)?;
        let anchor = pinned.insert(pin(&key)?);
        #[cfg(target_arch = "x86_64")]
        if let Some(attempt) = natural.as_mut() {
            attempt.bind_wait(&mut journal, &rpc, &key, &running, anchor, hold_deadline)?;
        }
        let confirmed = marked.confirm_running(running.clone()).map_err(invalid)?;
        stdout.sample(
            &mut journal,
            Instant::now(),
            Event::Selected(Box::new(key.clone())),
        )?;

        let ready = (|| {
            stdout.until_marker(&mut journal, hold_deadline)?;
            #[cfg(target_arch = "x86_64")]
            let collection_from = Instant::now();
            let archive_applied = |journal: &mut Custody, applied: &AppliedWitness| -> Result<AppliedRecord> {
                let payload = serde_json::to_value(applied)?;
                let sha256 = crate::digest(&serde_json::to_vec(&payload)?);
                let position = journal.record(Kind::Applied, payload)?;
                Ok(AppliedRecord { position, sha256 })
            };
            let mut one_stop = || -> Result<(AppliedWitness, AppliedRecord, serde_json::Value)> {
                let applied = source.applied(&rpc, &stdout.attach, &key, hold_deadline)?;
                // Preserve the one-Stop archive order even if the join fails.
                let record = archive_applied(&mut journal, &applied)?;
                if applied.start != rpc
                    || applied.attach != stdout.attach
                    || applied.launch.task != key
                    || applied.effective.task != key
                {
                    return refuse("applied witness crossed Start, prearmed stdout or execution");
                }
                serving_now(source, endpoint, &serving, &mut journal, hold_deadline)?;
                let kernel = json!({ "kernel_current": anchor.check_current(&applied.effective)? });
                Ok((applied, record, kernel))
            };
            #[cfg(target_arch = "x86_64")]
            let (applied, record, kernel_payload) = if let Some(attempt) = natural.as_ref() {
                serving_now(source, endpoint, &serving, &mut journal, hold_deadline)?;
                let original = anchor
                    .original_kernel_task()
                    .ok_or_else(|| invalid("stock Held requires the retained original task"))?;
                let (applied, observed) = attempt.capture_original_held(
                    original, recipe, &trace, &rpc, &stdout.attach, hold_deadline,
                )?;
                let record = archive_applied(&mut journal, &applied)?;
                let kernel = json!({
                    "original_held":observed, "applied_position":record.position,
                    "collection_from_ns":capture_ns(clock, collection_from)?,
                });
                (applied, record, kernel)
            } else {
                one_stop()?
            };
            #[cfg(not(target_arch = "x86_64"))]
            let (applied, record, kernel_payload) = one_stop()?;
            let kernel_position = journal.record(Kind::Applied, kernel_payload)?;
            stdout.pending(&mut journal, true)?;
            let cut_at = Instant::now();
            within(hold_deadline)?;
            let capture = HeldCapture {
                position: journal.next(),
                applied: record,
                kernel_position,
                at_ns: capture_ns(clock, cut_at)?,
            };
            journal.record(Kind::HeldCapture, json!(&capture))?;
            let mut inherited_coverage = || {
                start_fence.as_mut().ok_or_else(|| invalid("missing one-Stop Start fence"))?
                    .confirm_cut(&capture, cut_at, hold_deadline)
            };
            #[cfg(target_arch = "x86_64")]
            let coverage = if let Some(attempt) = natural.as_ref() {
                attempt.recheck_held(
                    recipe, &applied,
                    anchor.original_kernel_task()
                        .ok_or_else(|| invalid("stock Held requires the retained original task"))?,
                    &capture, collection_from, cut_at, clock, &mut journal, hold_deadline,
                    &source_identity, |journal| stdout.pending(journal, true),
                )?
            } else {
                inherited_coverage()?
            };
            #[cfg(not(target_arch = "x86_64"))]
            let coverage = inherited_coverage()?;
            let received = Instant::now();
            if coverage.source != source_identity
                || coverage.capture_position != capture.position
                || coverage.applied != capture.applied
                || !full_hex(&coverage.evidence_sha256)
                || coverage.from > cut_at
                || coverage.through < cut_at
                || coverage.through > received
            {
                return refuse("protected applied witness does not cover this Held cut");
            }
            journal.record(
                Kind::HeldCoverage,
                json!({
                    "source":coverage.source, "capture_position":coverage.capture_position,
                    "applied":coverage.applied, "evidence_sha256":coverage.evidence_sha256,
                    "covered_from_ns":capture_ns(clock, coverage.from)?,
                    "covered_through_ns":capture_ns(clock, coverage.through)?,
                    "received_ns":capture_ns(clock, received)?,
                }),
            )?;
            within(hold_deadline)?;
            let cut = CurrentCut {
                launch: applied.launch,
                effective: applied.effective,
                anchors: Anchors::SameExecutionLiveAndRetained,
                drain: Drain::UninterruptedAtCut,
            };
            stdout.sample(&mut journal, cut_at, Event::Cut(Box::new(cut)))?;
            within(hold_deadline)?;
            verify_fixture(
                port,
                binding,
                &running,
                confirmed,
                FixtureInput {
                    recipe,
                    serving: &serving,
                    create: &trace,
                    start: &start,
                    samples: &stdout.samples,
                    observed_through_ns: stdout.samples.last().expect("cut just recorded").at_ns,
                },
            )
            .map_err(invalid)
        })();
        let held_position = journal.record(
            Kind::Verdict,
            match &ready {
                Ok(_) => json!({"held":"checked"}),
                Err(e) => json!({"held":"unestablished","cause":e.to_string()}),
            },
        )?;
        #[cfg(not(target_arch = "x86_64"))]
        let _ = held_position;
        drop(start_fence);
        drop(start_io);
        drop(stdout);
        let Ok(held) = ready else {
            // The qualification action authorizes Stop only after confirmed
            // Held. Failure disposition belongs to the designated safety owner.
            return Ok(Verdict {
                held: false,
                terminal: false,
                qualified: false,
            });
        };

        #[cfg(target_arch = "x86_64")]
        if let Some(attempt) = natural.as_mut() {
            return attempt.finish(
                port,
                binding,
                endpoint,
                source,
                &mut journal,
                anchor,
                &held,
                held_position,
            );
        }
        // Held authorizes this sequence, not liveness now or terminality.
        let live = OfflineLiveSubtree {
            running,
            boot: boot_key(&serving.boot),
            runtime_namespace: key.runtime_namespace.clone(),
            cgroup: OfflineCgroupHandle {
                capture: key.subtree_capture.clone(),
                mount_id: key.subtree_mount_id,
                inode: key.subtree_inode,
            },
            capture_position: journal.next(),
            cgroup_events: anchor.live_events()?,
        };
        journal.record(Kind::LiveSubtree, json!(&live))?;
        let cleanup = Instant::now() + SETTLEMENT;
        let target = format!("POST /{}/containers/{id}/stop?{STOP_QUERY}", serving.api);
        let request = docker::request(&target, &[]);
        journal.record(
            Kind::Intended(Operation::Stop),
            json!({"request":request, "live":live}),
        )?;
        serving_now(source, endpoint, &serving, &mut journal, cleanup)?;
        let stop = port
            .mark_offline_scoped_stop(binding, &live, JoinedMarkReadback::Check)
            .map_err(invalid)?;
        // Store the actual selected subject even if the subsequent connection or
        // archive write fails. We never reissue from these bytes or from the mark.
        journal.record(Kind::Settlement, json!({"stop_subject":stop}))?;
        let stop_io = endpoint.connect()?;
        let exchange = StopExchange {
            subject: stop.clone(),
            task: key.clone(),
            request_target: target,
            connection: docker::socket_cookie(&stop_io)?,
        };
        let (protection, mut stop_fence) =
            source.fence_stop(&exchange, stop_io.as_fd(), cleanup)?;
        if protection.source != source_identity
            || protection.exchange != exchange
            || !matches!(&protection.scope, ResolutionScope::ThroughHandlerResolutionOrInert {
            enforcement_sha256,
        } if full_hex(enforcement_sha256))
        {
            return refuse(
                "Stop exchange lacks original-only-or-inert handler resolution protection",
            );
        }
        journal.record(Kind::StopResolution, json!(&protection))?;
        within(cleanup)?;
        let (written, reply) = effect(
            &mut journal,
            stop_io,
            Operation::Stop,
            &stop.close_mark,
            &request,
            cleanup.min(Instant::now() + RPC_LIMIT),
            None,
        )?;
        let position = journal.next();
        let response = match reply {
            Some(r) if r.status == 204 => OfflineStopResponse::ReturnedSuccess,
            Some(_) => OfflineStopResponse::ReturnedFailure,
            None if written => OfflineStopResponse::Lost,
            None => OfflineStopResponse::Unknown,
        };
        let diagnostic = OfflineStopReadback {
            subject: stop.clone(),
            position,
            observed: response,
        };
        journal.record(Kind::Settlement, json!(&diagnostic))?;
        port.record_offline_stop_response(binding, &diagnostic)
            .map_err(invalid)?;
        let checked = terminal(
            source,
            endpoint,
            &mut journal,
            &stop,
            &key,
            anchor,
            &mut stop_fence,
            cleanup,
        )
        .and_then(|evidence| {
            port.record_offline_scoped_terminal(binding, &stop, &evidence)
                .map_err(invalid)?;
            journal.record(Kind::ScopeChecked, json!(&stop))?;
            Ok(evidence)
        });
        let qualified = if let (true, Ok(evidence)) = (written, &checked) {
            qualify_fixture(port, binding, &held, &stop, evidence).map_err(invalid)?;
            true
        } else {
            false
        };
        let verdict = Verdict {
            held: true,
            terminal: checked.is_ok(),
            qualified,
        };
        journal.record(
            Kind::Verdict,
            json!({"result":verdict,"terminal_failure":checked.err().map(|e|e.to_string())}),
        )?;
        Ok(verdict)
    })();
    #[cfg(target_arch = "x86_64")]
    if let Some(attempt) = natural.as_mut() {
        attempt.preserve_unresolved(binding, source, &mut journal, pinned.as_ref(), &outcome)?;
    }
    outcome
}

struct Stdout {
    io: UnixStream,
    sender: Option<docker::Sender>,
    wire: wire::Stream,
    attach: Attach,
    samples: Vec<Sample>,
    clock: Instant,
    opened: Instant,
    bytes: usize,
}

impl Stdout {
    fn open(
        journal: &mut Custody,
        endpoint: &Endpoint,
        serving: &Serving,
        id: &str,
        clock: Instant,
        deadline: Instant,
        sender: Option<docker::Sender>,
    ) -> Result<Self> {
        let io = endpoint.connect()?;
        docker::authenticate(&io, sender)?;
        let attach = Attach {
            serving: serving.clone(),
            connection: docker::socket_cookie(&io)?,
            resolved_full_id: id.into(),
            http_request: attach_request(serving, id),
            registration: Registration::AuthenticatedPinnedSameObject,
        };
        let mut s = Self {
            io,
            sender,
            wire: wire::Stream::default(),
            attach,
            samples: Vec::new(),
            clock,
            opened: Instant::now(),
            bytes: 0,
        };
        journal.record(
            Kind::AttachWire,
            json!({"cookie":s.attach.connection,"request":s.attach.http_request}),
        )?;
        docker::write(&mut s.io, &s.attach.http_request, deadline)?;
        while !s.wire.upgraded() {
            s.read(journal, false, deadline)?;
        }
        s.pending(journal, false)?;
        Ok(s)
    }

    fn confirm_open(&mut self, journal: &mut Custody, registered: Attach) -> Result<()> {
        // The Open sample names the channel at its original local open. The
        // archive separately orders when the registration was corroborated.
        let sample = Sample {
            at_ns: capture_ns(self.clock, self.opened)?,
            event: Event::Open(registered),
        };
        journal.record(Kind::AttachWire, json!({"confirmed_registration":sample}))?;
        self.samples.insert(0, sample);
        Ok(())
    }

    fn sample(&mut self, journal: &mut Custody, time: Instant, event: Event) -> Result<()> {
        let sample = Sample {
            at_ns: capture_ns(self.clock, time)?,
            event,
        };
        journal.record(Kind::Stdout, json!(&sample))?;
        self.samples.push(sample);
        Ok(())
    }

    fn bytes(&mut self, journal: &mut Custody, bytes: &[u8], issued: bool) -> Result<()> {
        if bytes.is_empty() || bytes.len() > super::MAX_STREAM_BYTES - self.bytes {
            self.sample(journal, Instant::now(), Event::EofResetGapOrBackpressure)?;
            return refuse("stdout closed or exceeded bounded capture");
        }
        self.bytes += bytes.len();
        if let Some(sender) = self.sender {
            journal.record(
                Kind::AttachWire,
                json!({"authenticated_stock_sender":sender,"cookie":self.attach.connection,
                    "fragment_end_offset":self.bytes}),
            )?;
        }
        self.sample(
            journal,
            Instant::now(),
            Event::Bytes {
                connection: self.attach.connection,
                chunk: bytes.to_vec(),
            },
        )?;
        self.wire.feed(bytes, issued).map_err(invalid)
    }

    fn read(&mut self, journal: &mut Custody, issued: bool, deadline: Instant) -> Result<()> {
        let mut buf = [0; 512];
        match docker::read(&mut self.io, &mut buf, deadline, self.sender) {
            Ok(n) => self.bytes(journal, &buf[..n], issued),
            Err(e) => {
                self.sample(journal, Instant::now(), Event::EofResetGapOrBackpressure)?;
                Err(e.into())
            }
        }
    }

    fn pending(&mut self, journal: &mut Custody, issued: bool) -> Result<()> {
        let mut buf = [0; 512];
        loop {
            match docker::read_now(&mut self.io, &mut buf, self.sender) {
                Ok(n) => self.bytes(journal, &buf[..n], issued)?,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    self.sample(journal, Instant::now(), Event::EofResetGapOrBackpressure)?;
                    return Err(e.into());
                }
            }
        }
    }

    fn until_marker(&mut self, journal: &mut Custody, deadline: Instant) -> Result<()> {
        while !self.wire.complete() {
            self.read(journal, true, deadline)?;
        }
        Ok(())
    }
}

fn terminal(
    source: &mut impl ProtectedSource,
    endpoint: &Endpoint,
    journal: &mut Custody,
    stop: &OfflineStopSubject,
    key: &TaskKey,
    anchor: &impl Anchor,
    fence: &mut impl StopFence,
    deadline: Instant,
) -> Result<OfflineStopTerminalReadbacks> {
    use Phase::*;
    serving_now(source, endpoint, &key.serving, journal, deadline)?;
    let mut out = OfflineStopTerminalReadbacks::default();
    for phase in [Ingress, OldTask, Accepted, Recovery] {
        within(deadline)?;
        let witness = source.settlement(stop, key, phase, deadline)?;
        let position = journal.record(Kind::Settlement, json!(&witness))?;
        if witness.subject != *stop || witness.task != *key {
            return refuse("crossed terminal observation");
        }
        macro_rules! readback {
            ($fact:expr) => {
                Some(OfflineStopReadback {
                    subject: stop.clone(),
                    position,
                    observed: $fact,
                })
            };
        }
        match (phase, witness.fact) {
            (Ingress, ScopeFact::Ingress(v)) => out.ingress = readback!(v),
            (OldTask, ScopeFact::OldTask(v)) => out.old_task = readback!(v),
            (Accepted, ScopeFact::Accepted(v)) => out.accepted = readback!(v),
            (Recovery, ScopeFact::Recovery(v)) => out.recovery = readback!(v),
            _ => return refuse("unexpected terminal observation kind"),
        }
    }
    serving_now(source, endpoint, &key.serving, journal, deadline)?;
    // Only the original pidfd and cgroup FD supply the end sample. Record
    // it even if containment coverage fails to corroborate that sample.
    let begin = Instant::now();
    let cgroup_events = anchor.end_events()?;
    let read = (begin, Instant::now());
    journal.record(
        Kind::Settlement,
        json!({"original_end_events":cgroup_events}),
    )?;
    within(deadline)?;
    let containment = fence.confirm_end(read, deadline)?;
    within(deadline)?;
    let end = OfflineStopReadback {
        subject: stop.clone(),
        position: journal.next(),
        observed: OfflineSubtreeEnd {
            exited_init_lifetime: Some(stop.execution.live.running.lifetime.clone()),
            cgroup: stop.execution.live.cgroup.clone(),
            containment,
            cgroup_events,
        },
    };
    journal.record(Kind::Settlement, json!(&end))?;
    out.end = Some(end);
    Ok(out)
}
