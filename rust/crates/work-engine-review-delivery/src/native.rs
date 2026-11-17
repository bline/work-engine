//! Fixture-only native runner (feature `fixture`). Starts the provider-disabled Pi SDK adapter as
//! one owned child, binds the SDK-allocated session to one attempt before the prompt, sends one
//! start frame, then collects the COMPLETE attempt under one owner (see [`run_attempt`]): every
//! stdout byte until a real EOF, stderr, the terminal process outcome. Only a complete, clean
//! attempt whose whole stream is the permitted protocol reaches the existing fixture consumer.
//! Everything supplied here is a synthetic test assumption; no authority, admission or blocker
//! change results.
//!
//! The raw received bytes are retained as read, independently of framing, decoding or any
//! terminal report. Anything after the first matching observation (duplicate/malformed/
//! invalid-UTF-8/incomplete frames, delayed or prompt) stays in `raw_stdout` and refuses success.
//! Bytes beyond a configured limit are counted as overflow, never retained or papered over.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::json;
use work_engine_review_preparation::fixture::{ConsumeRefusal, FixtureConsumption};

pub use crate::attempt::*;
use crate::binding::ExperimentBinding;
use crate::{
    BoundAttempt, DeliveryRefusal, NativeObservation, StreamInterpretation, VerifiedDelivery,
    interpret_stream,
};

/// Written under an owned ancestor of the session cwd. Its appearance in any SDK context would
/// show an outside-loader resource was consumed.
pub const CANARY_MARKER: &str = "CANARY-OUTSIDE-LOADER-RESOURCE-5f1c";
const MAX_FRAME_BYTES: usize = 1 << 20;
const MAX_TOTAL_BYTES: u64 = 8 << 20;
const MAX_STDERR_BYTES: u64 = 64 << 10;

/// Collection limits. Tests lower them to exercise a limit without producing megabytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub frame_bytes: usize,
    pub total_bytes: u64,
    pub stderr_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            frame_bytes: MAX_FRAME_BYTES,
            total_bytes: MAX_TOTAL_BYTES,
            stderr_bytes: MAX_STDERR_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Positive,
    /// Fixture-only raw duplicate member injection before the consumption boundary.
    DuplicateMember(WireObject, bool, bool),
    /// Omit the fake runtime key: the genuine SDK auth preflight must refuse before any stream.
    MissingKey,
    /// Leave the ordinary SDK stream selected: the provider-entry trap must refuse.
    OrdinaryStream,
    /// The adapter exits while the request is in flight, with no observation.
    DieDuringPrompt,
    /// Start frame names a session the adapter did not allocate.
    WrongSession,
    /// Send the start frame twice, both frames queued together (coalesced writes).
    DuplicateStart,
    /// Send the start frame twice, each frame in its own write (`WriteShape::PerFrame`).
    DuplicateStartPerFrame,
    /// Send the start frame twice in small writes that split frames (`WriteShape::Chunked`).
    DuplicateStartChunked,
    /// Exit zero after the start frame without any observation (early EOF).
    CleanExitNoObservation,
    /// Exit before reading the start frame (send failure or no observation).
    DieAfterAllocation,
    /// Valid observation, then trailing data. `delayed` waits until after Rust closed stdin.
    Trail(Trailing, bool),
    /// Valid observation, then exit with a nonzero status.
    ExitNonzeroAfterObservation,
    /// Valid observation, then never exit nor close stdout (timeout).
    HangAfterObservation,
    /// Valid observation, then one frame longer than any permitted bound, no newline.
    OversizeFrameAfterObservation,
    /// Valid observation, then many small valid frames past the total limit.
    FloodAfterObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireObject {
    Allocation,
    Observation,
    Context,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trailing {
    DuplicateObservation,
    Malformed,
    ErrorFrame,
    InvalidUtf8,
    IncompleteFrame,
}

impl Control {
    fn duplicate_start(self) -> bool {
        matches!(
            self,
            Control::DuplicateStart
                | Control::DuplicateStartPerFrame
                | Control::DuplicateStartChunked
        )
    }
    fn write_shape(self) -> WriteShape {
        match self {
            Control::DuplicateStartPerFrame => WriteShape::PerFrame,
            Control::DuplicateStartChunked => WriteShape::Chunked(4093),
            _ => WriteShape::Coalesced,
        }
    }
    fn adapter_name(self) -> String {
        match self {
            Control::DuplicateMember(object, conflict, reversed) => format!(
                "duplicate-{}-{}-{}",
                match object {
                    WireObject::Allocation => "allocation",
                    WireObject::Observation => "observation",
                    WireObject::Context => "context",
                    WireObject::History => "history",
                },
                if conflict { "conflict" } else { "equal" },
                if reversed { "last" } else { "first" }
            ),
            Control::MissingKey => "missing-key".into(),
            Control::OrdinaryStream => "ordinary-stream".into(),
            Control::DieDuringPrompt => "die-during-prompt".into(),
            Control::CleanExitNoObservation => "clean-exit-no-observation".into(),
            Control::DieAfterAllocation => "die-after-allocation".into(),
            Control::ExitNonzeroAfterObservation => "exit-nonzero-after-observation".into(),
            Control::HangAfterObservation => "hang-after-observation".into(),
            Control::OversizeFrameAfterObservation => "oversize-after-observation".into(),
            Control::FloodAfterObservation => "flood-after-observation".into(),
            Control::Trail(kind, delayed) => format!(
                "trail-{}-{}",
                match kind {
                    Trailing::DuplicateObservation => "duplicate",
                    Trailing::Malformed => "malformed",
                    Trailing::ErrorFrame => "error",
                    Trailing::InvalidUtf8 => "invalid-utf8",
                    Trailing::IncompleteFrame => "incomplete",
                },
                if delayed { "delayed" } else { "prompt" }
            ),
            _ => "positive".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NativeConfig {
    pub node: PathBuf,
    pub adapter: PathBuf,
    /// Pi SDK artifact root (contains `node_modules`), read-only to the child. Product code does
    /// not verify a pin; a caller that needs a pinned SDK checks that itself.
    pub sdk_root: PathBuf,
    /// Owned fixture directory; HOME/XDG/agent/canary/tmp are created beneath it.
    pub fixture_root: PathBuf,
    /// The attempt deadline: sends, reads and the child's own completion.
    pub timeout: Duration,
    /// Cleanup bound after the deadline (or after an earlier stop): kill, reap and draining the
    /// pipes for evidence. Both bounds are checked by the owner loop, not preemptive; they are not
    /// a wall-clock guarantee for the whole attempt (see [`run_attempt`]).
    pub cleanup: Duration,
    pub control: Control,
    pub limits: Limits,
}

impl NativeConfig {
    pub fn default_adapter() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("adapter/review-delivery-adapter.mjs")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NativeError {
    #[error("fixture setup failed: {0}")]
    Setup(String),
    #[error("could not start the adapter: {0}")]
    Spawn(String),
}

/// Budget and bounds of one owned attempt.
#[derive(Debug, Clone, Copy)]
pub struct AttemptPolicy {
    pub timeout: Duration,
    pub cleanup: Duration,
    pub limits: Limits,
    pub write_shape: WriteShape,
}

/// How queued frame bytes are grouped into write(2) calls. A labelled deterministic I/O seam:
/// transport grouping never changes what a send record means (bytes accepted by the pipe), and
/// controls use it to show that peer-side adjudication does not depend on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WriteShape {
    /// Every queued byte offered in one write per readiness.
    #[default]
    Coalesced,
    /// One frame per write; the next frame waits for the next readiness.
    PerFrame,
    /// At most this many bytes per write, one write per readiness (splits frames).
    Chunked(usize),
}

/// The owner's non-blocking wait observation. A labelled deterministic seam: the lifecycle asks
/// this for every wait, so a control can inject wait failures into the real decision path.
/// [`run_attempt`] uses [`OsWait`].
pub trait WaitObserver {
    fn try_wait(&mut self, child: &mut Child) -> std::io::Result<Option<std::process::ExitStatus>>;
}

/// The operating system's wait.
pub struct OsWait;
impl WaitObserver for OsWait {
    fn try_wait(&mut self, child: &mut Child) -> std::io::Result<Option<std::process::ExitStatus>> {
        child.try_wait()
    }
}

/// What the protocol driver asks of the owner after new stdout bytes were retained.
#[derive(Debug, Default)]
pub struct DriverStep {
    /// Frames to send, in order (text without the newline).
    pub send: Vec<String>,
    /// No further frames will be sent: close stdin once queued frames are written.
    pub close_stdin: bool,
}

/// Protocol progression over the retained raw stdout. The driver never reads a pipe or owns I/O.
pub trait AttemptDriver {
    fn on_stdout(&mut self, raw: &[u8]) -> DriverStep;
}

/// Everything one owned attempt produced: retained bytes exactly as read/written and the facts.
#[derive(Debug, Clone)]
pub struct AttemptRecord {
    pub raw_stdout: Vec<u8>,
    pub raw_stderr: Vec<u8>,
    /// Exact bytes of every frame whose send began, in order (text plus newline each).
    pub sent_bytes: Vec<u8>,
    pub facts: AttemptFacts,
}

/// Deterministic accounting seam for one owned output stream: every byte offered is either
/// retained (in order) or counted as known overflow at a bound. Public so limit/boundary semantics
/// can be exercised directly; the attempt owner uses the same value on the real pipe.
#[derive(Debug, Clone)]
pub struct StreamCollector {
    raw: Vec<u8>,
    limit: u64,
    frame_limit: Option<u64>,
    open_frame: u64,
    overflow: u64,
    end: Option<StreamEnd>,
}

impl StreamCollector {
    pub fn new(limit: u64, frame_limit: Option<u64>) -> Self {
        Self {
            raw: Vec::new(),
            limit,
            frame_limit,
            open_frame: 0,
            overflow: 0,
            end: None,
        }
    }
    /// How many bytes the next read may request: the room left plus one, so that a bound reached
    /// exactly is distinguished from more bytes being available.
    pub fn want(&self) -> usize {
        (self.limit.saturating_sub(self.raw.len() as u64))
            .saturating_add(1)
            .min(64 << 10) as usize
    }
    /// Offers bytes just read. Returns the end when a bound was reached.
    pub fn absorb(&mut self, chunk: &[u8]) -> Option<StreamEnd> {
        if self.end.is_some() || chunk.is_empty() {
            return None;
        }
        let mut take = chunk.len();
        let mut end = None;
        if let Some(f) = self.frame_limit {
            let mut open = self.open_frame;
            for (i, b) in chunk.iter().enumerate() {
                if *b == b'\n' {
                    open = 0;
                } else {
                    open += 1;
                    if open > f {
                        take = i + 1;
                        end = Some(StreamEnd::FrameLimit);
                        break;
                    }
                }
            }
        }
        let room = self.limit.saturating_sub(self.raw.len() as u64) as usize;
        if take > room {
            take = room;
            end = Some(StreamEnd::TotalLimit);
        }
        for b in &chunk[..take] {
            if *b == b'\n' {
                self.open_frame = 0;
            } else {
                self.open_frame += 1;
            }
        }
        self.raw.extend_from_slice(&chunk[..take]);
        self.overflow += (chunk.len() - take) as u64;
        if let Some(e) = &end {
            self.end = Some(e.clone());
        }
        end
    }
    /// Records a non-limit end (EOF, read error, cancellation, not started).
    pub fn finish(&mut self, end: StreamEnd) {
        if self.end.is_none() {
            self.end = Some(end);
        }
    }
    pub fn ended(&self) -> bool {
        self.end.is_some()
    }
    pub fn bytes(&self) -> &[u8] {
        &self.raw
    }
    pub fn into_parts(self) -> (Vec<u8>, StreamRecord) {
        let record = StreamRecord {
            end: self.end.unwrap_or(StreamEnd::Cancelled),
            retained_bytes: self.raw.len() as u64,
            overflow_bytes_observed: self.overflow,
            limit_bytes: self.limit,
            frame_limit_bytes: self.frame_limit,
        };
        (self.raw, record)
    }
}

/// Kills on unwind only, without ever blocking: a normal return has already recorded the
/// child's kill/reap facts and disarmed this.
struct UnwindKill(Option<Child>);
impl Drop for UnwindKill {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.try_wait();
        }
    }
}

fn nonblocking(file: &File) -> std::io::Result<()> {
    let fd = file.as_raw_fd();
    // SAFETY: fcntl on a file descriptor this process owns, with integer arguments only.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: as above.
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn owned_pipe<T: Into<std::os::fd::OwnedFd>>(pipe: Option<T>) -> Result<File, String> {
    let file = File::from(pipe.ok_or("pipe unavailable")?.into());
    nonblocking(&file).map_err(|e| format!("could not make the pipe non-blocking: {e}"))?;
    Ok(file)
}

struct Output {
    pipe: Option<File>,
    collector: StreamCollector,
}

impl Output {
    fn new(pipe: Result<File, String>, limit: u64, frame: Option<u64>) -> Self {
        let mut collector = StreamCollector::new(limit, frame);
        let pipe = match pipe {
            Ok(f) => Some(f),
            Err(reason) => {
                collector.finish(StreamEnd::NotStarted { reason });
                None
            }
        };
        Self { pipe, collector }
    }
    /// Reads what is available now (bounded number of reads). Returns true if bytes arrived.
    /// A terminal read closes the pipe; the end is recorded in the collector.
    fn drain(&mut self) -> bool {
        let Some(pipe) = self.pipe.as_mut() else {
            return false;
        };
        let mut got = false;
        let mut buf = vec![0u8; 64 << 10];
        for _ in 0..16 {
            let want = self.collector.want();
            match pipe.read(&mut buf[..want]) {
                Ok(0) => {
                    self.collector.finish(StreamEnd::Eof);
                    break;
                }
                Ok(n) => {
                    got = true;
                    if self.collector.absorb(&buf[..n]).is_some() {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    self.collector.finish(StreamEnd::ReadError {
                        error: e.to_string(),
                    });
                    break;
                }
            }
        }
        if self.collector.ended() {
            self.pipe = None;
        }
        got
    }
    fn stopped_early(&self) -> bool {
        matches!(
            self.collector.end,
            Some(StreamEnd::FrameLimit | StreamEnd::TotalLimit | StreamEnd::ReadError { .. })
        )
    }
}

struct Pending {
    bytes: Vec<u8>,
    written: usize,
    record: Option<usize>,
}

fn reap_fact(status: std::process::ExitStatus) -> ReapFact {
    use std::os::unix::process::ExitStatusExt;
    match (status.code(), status.signal()) {
        (Some(code), _) => ReapFact::Exited { code },
        (None, Some(signal)) => ReapFact::Signalled { signal },
        (None, None) => ReapFact::UnknownStatus {
            raw: status.into_raw(),
        },
    }
}

const POLL_QUANTUM: Duration = Duration::from_millis(20);

/// Runs one owned attempt of `command` to its end. ONE owner on the calling thread multiplexes the
/// child's stdin, stdout and stderr with poll(2) on non-blocking pipes, so the deadline is
/// checked between every send, read and cleanup step and no I/O activity outlives the call. The
/// deadline is checked, not preemptive: driver callbacks, `/proc` sampling, per-frame copies and the
/// post-loop digests and finalization run to completion between checks, so the recorded
/// spawn-to-return `elapsed_ms` can exceed `timeout + cleanup` by that work.
///
/// Lifecycle: spawn (failure: `Err`) → pipes made non-blocking (failure: that stream is
/// `NotStarted`, the child is killed) → frames queued by the driver are written as the pipe
/// accepts them, each recorded from its first write on → stdout/stderr bytes retained as read →
/// stdin closed when the driver needs nothing more → clean completion once both streams reached
/// EOF, stdin is closed and the child is reaped. A limit or read error stops that stream and kills
/// the child; the deadline kills it. Either way a cleanup phase of at most `policy.cleanup` keeps
/// draining open pipes and reaping; when it ends, open streams are `Cancelled` and an unreaped
/// child is `NotReaped`. Facts are recorded as they happen, never inferred afterwards.
pub fn run_attempt(
    command: Command,
    driver: &mut dyn AttemptDriver,
    policy: &AttemptPolicy,
) -> Result<AttemptRecord, NativeError> {
    run_attempt_observed(command, driver, policy, &mut OsWait)
}

/// [`run_attempt`] with an explicit wait observer (see [`WaitObserver`]). A failed wait is never
/// treated as termination: it is recorded, the child is killed within the cleanup bound, waiting
/// continues until that bound, and only a successful wait establishes the reap.
pub fn run_attempt_observed(
    mut command: Command,
    driver: &mut dyn AttemptDriver,
    policy: &AttemptPolicy,
    waiter: &mut dyn WaitObserver,
) -> Result<AttemptRecord, NativeError> {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let started = Instant::now();
    let child = command
        .spawn()
        .map_err(|e| NativeError::Spawn(e.to_string()))?;
    let deadline = started + policy.timeout;
    let mut owned = UnwindKill(Some(child));
    let child = owned.0.as_mut().expect("child present");
    let pid = child.id();
    let mut stdin: Option<File> = None;
    let mut stdin_problem: Option<String> = None;
    match owned_pipe(child.stdin.take()) {
        Ok(f) => stdin = Some(f),
        Err(e) => stdin_problem = Some(e),
    }
    let limits = policy.limits;
    let mut out = Output::new(
        owned_pipe(child.stdout.take()),
        limits.total_bytes,
        Some(limits.frame_bytes as u64),
    );
    let mut err = Output::new(owned_pipe(child.stderr.take()), limits.stderr_bytes, None);

    let mut sent: Vec<SendRecord> = Vec::new();
    let mut sent_bytes: Vec<u8> = Vec::new();
    let mut queue: std::collections::VecDeque<Pending> = Default::default();
    let mut close_wanted = false;
    let mut kill = KillFact::NotSent;
    // Established termination only (exit, signal or unknown status from a successful wait).
    let mut reap: Option<ReapFact> = None;
    let mut wait_errors: Vec<String> = Vec::new();
    let mut timed_out = false;
    let mut cleanup_until: Option<Instant> = None;
    let mut descendants: Vec<u32> = Vec::new();
    let mut sampled = false;

    let enter_cleanup = |reason: KillReason,
                         child: &mut Child,
                         reap: &Option<ReapFact>,
                         queue: &mut std::collections::VecDeque<Pending>,
                         sent: &mut Vec<SendRecord>,
                         stdin: &mut Option<File>,
                         kill: &mut KillFact,
                         cleanup_until: &mut Option<Instant>| {
        if cleanup_until.is_some() {
            return;
        }
        *cleanup_until = Some(Instant::now() + policy.cleanup);
        let cancel = if reason == KillReason::Deadline {
            SendCancel::Deadline
        } else {
            SendCancel::AttemptStopped
        };
        for p in queue.drain(..) {
            if let Some(i) = p.record {
                sent[i].written = p.written as u64;
                sent[i].outcome = SendOutcome::Cancelled { reason: cancel };
            }
        }
        *stdin = None;
        if reap.is_none() {
            *kill = match child.kill() {
                Ok(()) => KillFact::Sent { reason },
                Err(e) => KillFact::Failed {
                    reason,
                    error: e.to_string(),
                },
            };
        }
    };
    if stdin_problem.is_some() || out.pipe.is_none() || err.pipe.is_none() {
        enter_cleanup(
            KillReason::OwnerIoFailure,
            child,
            &reap,
            &mut queue,
            &mut sent,
            &mut stdin,
            &mut kill,
            &mut cleanup_until,
        );
    }

    loop {
        if reap.is_none() {
            match waiter.try_wait(child) {
                Ok(Some(status)) => reap = Some(reap_fact(status)),
                Ok(None) => {}
                Err(e) => {
                    // Unknown termination is not termination: keep owning the child, stop it
                    // within the cleanup bound and keep trying to establish its end.
                    let error = e.to_string();
                    if wait_errors.last() != Some(&error) {
                        wait_errors.push(error);
                    }
                    enter_cleanup(
                        KillReason::WaitFailed,
                        child,
                        &reap,
                        &mut queue,
                        &mut sent,
                        &mut stdin,
                        &mut kill,
                        &mut cleanup_until,
                    );
                }
            }
        }
        let now = Instant::now();
        if cleanup_until.is_none() && now >= deadline {
            timed_out = true;
            if reap.is_none() {
                for c in children_of(pid) {
                    if !descendants.contains(&c) {
                        descendants.push(c);
                    }
                }
            }
            enter_cleanup(
                KillReason::Deadline,
                child,
                &reap,
                &mut queue,
                &mut sent,
                &mut stdin,
                &mut kill,
                &mut cleanup_until,
            );
        }
        if cleanup_until.is_some_and(|u| now >= u) {
            break;
        }
        if stdin.is_some() && queue.is_empty() && (close_wanted || out.collector.ended()) {
            stdin = None;
        }
        if stdin.is_none()
            && queue.is_empty()
            && out.collector.ended()
            && err.collector.ended()
            && reap.is_some()
        {
            break;
        }

        let mut fds: Vec<libc::pollfd> = Vec::with_capacity(3);
        let mut slots: [Option<usize>; 3] = [None; 3];
        if let (Some(f), false) = (stdin.as_ref(), queue.is_empty()) {
            slots[0] = Some(fds.len());
            fds.push(libc::pollfd {
                fd: f.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            });
        }
        for (k, o) in [(1usize, &out), (2, &err)] {
            if let Some(f) = o.pipe.as_ref() {
                slots[k] = Some(fds.len());
                fds.push(libc::pollfd {
                    fd: f.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                });
            }
        }
        let until = cleanup_until.unwrap_or(deadline);
        let mut wait = until.saturating_duration_since(now);
        if reap.is_none() {
            wait = wait.min(POLL_QUANTUM);
        }
        // SAFETY: `fds` is a valid, exclusively borrowed array of `fds.len()` pollfd values whose
        // descriptors are owned by this function for the duration of the call.
        let ready = unsafe {
            libc::poll(
                fds.as_mut_ptr(),
                fds.len() as libc::nfds_t,
                wait.as_millis().min(i32::MAX as u128) as i32,
            )
        };
        if ready < 0 {
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            let error = format!("poll failed: {e}");
            for o in [&mut out, &mut err] {
                if o.pipe.take().is_some() {
                    o.collector.finish(StreamEnd::ReadError {
                        error: error.clone(),
                    });
                }
            }
            for p in queue.drain(..) {
                if let Some(i) = p.record {
                    sent[i].written = p.written as u64;
                    sent[i].outcome = SendOutcome::Failed {
                        error: error.clone(),
                    };
                }
            }
            enter_cleanup(
                KillReason::OwnerIoFailure,
                child,
                &reap,
                &mut queue,
                &mut sent,
                &mut stdin,
                &mut kill,
                &mut cleanup_until,
            );
            continue;
        }
        let revents = |slot: Option<usize>| slot.map_or(0, |i| fds[i].revents);

        // Writes: as much of the queue as the pipe accepts now.
        if revents(slots[0]) != 0
            && let Some(pipe) = stdin.as_mut()
        {
            {
                // Queued bytes are offered per `write_shape` (a transport grouping only: it says
                // nothing about how the peer reads them). Each frame's record exists from the
                // first write that carries its bytes, so only the last recorded frame can be
                // incomplete.
                while let Some(head) = queue.front_mut() {
                    if head.record.is_none() {
                        sent_bytes.extend_from_slice(&head.bytes);
                        sent.push(SendRecord {
                            // Digest filled in after the owned lifecycle (no O(frame) work
                            // inside the deadline-enforced loop).
                            frame_sha256: String::new(),
                            frame_len: head.bytes.len() as u64,
                            written: 0,
                            outcome: SendOutcome::Cancelled {
                                reason: SendCancel::AttemptStopped,
                            },
                        });
                        head.record = Some(sent.len() - 1);
                    }
                    // The queued bytes are offered in place (writev), grouped per `write_shape`.
                    let (frames, mut room) = match policy.write_shape {
                        WriteShape::Coalesced => (queue.len(), usize::MAX),
                        WriteShape::PerFrame => (1, usize::MAX),
                        WriteShape::Chunked(n) => (queue.len(), n.max(1)),
                    };
                    let mut slices: Vec<std::io::IoSlice<'_>> = Vec::with_capacity(frames);
                    for p in queue.iter().take(frames) {
                        let rest = &p.bytes[p.written..];
                        let take = rest.len().min(room);
                        if take == 0 {
                            break;
                        }
                        slices.push(std::io::IoSlice::new(&rest[..take]));
                        room -= take;
                    }
                    let result = pipe.write_vectored(&slices);
                    drop(slices);
                    match result {
                        Ok(0) => {
                            let i = queue[0].record.expect("head recorded");
                            sent[i].outcome = SendOutcome::Failed {
                                error: "write accepted zero bytes".into(),
                            };
                            queue.clear();
                            stdin = None;
                            break;
                        }
                        Ok(mut n) => {
                            while n > 0 {
                                let p = queue.front_mut().expect("bytes belong to a queued frame");
                                if p.record.is_none() {
                                    sent_bytes.extend_from_slice(&p.bytes);
                                    sent.push(SendRecord {
                                        frame_sha256: String::new(),
                                        frame_len: p.bytes.len() as u64,
                                        written: 0,
                                        outcome: SendOutcome::Cancelled {
                                            reason: SendCancel::AttemptStopped,
                                        },
                                    });
                                    p.record = Some(sent.len() - 1);
                                }
                                let i = p.record.expect("recorded");
                                let take = n.min(p.bytes.len() - p.written);
                                p.written += take;
                                n -= take;
                                sent[i].written = p.written as u64;
                                if p.written == p.bytes.len() {
                                    sent[i].outcome = SendOutcome::Completed;
                                    queue.pop_front();
                                }
                            }
                            if policy.write_shape != WriteShape::Coalesced {
                                // One write per readiness: the next bytes go in a later write.
                                break;
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(e) => {
                            let i = queue[0].record.expect("head recorded");
                            sent[i].outcome = SendOutcome::Failed {
                                error: e.to_string(),
                            };
                            queue.clear();
                            stdin = None;
                            break;
                        }
                    }
                }
            }
        }
        // Descendants are sampled once, after the queued frames were written back to back (a
        // /proc scan between frames would change the send timing the child observes).
        if !sampled && queue.is_empty() && sent.iter().any(|r| r.outcome == SendOutcome::Completed)
        {
            sampled = true;
            descendants = children_of(pid);
        }
        // Reads: retain everything available now; a terminal read closes that pipe.
        if revents(slots[1]) != 0 && out.drain() && cleanup_until.is_none() {
            let step = driver.on_stdout(out.collector.bytes());
            close_wanted |= step.close_stdin;
            // A frame is queued only while stdin is open; one never attempted gets no record.
            for text in step.send.into_iter().filter(|_| stdin.is_some()) {
                queue.push_back(Pending {
                    bytes: format!("{text}\n").into_bytes(),
                    written: 0,
                    record: None,
                });
            }
        }
        if revents(slots[2]) != 0 {
            err.drain();
        }
        if cleanup_until.is_none() && (out.stopped_early() || err.stopped_early()) {
            enter_cleanup(
                KillReason::CollectionStopped,
                child,
                &reap,
                &mut queue,
                &mut sent,
                &mut stdin,
                &mut kill,
                &mut cleanup_until,
            );
        }
    }
    // End of the deadline-checked loop. Frame digests, release of every still-open pipe and
    // evidence finalization follow; `elapsed_ms` is taken after all of them, just before return.
    let mut offset = 0usize;
    for r in sent.iter_mut() {
        let end = offset + r.frame_len as usize;
        r.frame_sha256 = sha256_hex(&sent_bytes[offset..end]);
        offset = end;
    }
    drop(stdin);
    for o in [&mut out, &mut err] {
        if o.pipe.take().is_some() {
            o.collector.finish(StreamEnd::Cancelled);
        }
    }
    // Termination is established only by a successful wait; otherwise the last wait error (or
    // the elapsed cleanup bound) is the truthful disposition.
    let reap = reap.unwrap_or_else(|| match wait_errors.first() {
        Some(error) => ReapFact::WaitFailed {
            error: error.clone(),
        },
        None => ReapFact::NotReaped,
    });
    let pid_present_after_reap = reap
        .reaped()
        .then(|| Path::new(&format!("/proc/{pid}")).exists());
    // Disarm only after the facts are recorded. Every path that reaches here without an
    // established reap has entered cleanup, which attempted the kill (recorded in `kill`) and
    // kept waiting for its whole bound; the unestablished end is recorded in `reap`.
    owned.0.take();
    let (raw_stdout, stdout) = out.collector.into_parts();
    let (raw_stderr, stderr) = err.collector.into_parts();
    // Complete caller-visible operation: spawn to return, including the post-loop hashing, pipe
    // release and finalization above. Nothing owned by this call runs after this point.
    let elapsed_ms = started.elapsed().as_millis() as u64;
    Ok(AttemptRecord {
        raw_stdout,
        raw_stderr,
        sent_bytes,
        facts: AttemptFacts {
            sent,
            stdout,
            stderr,
            process: ProcessFacts {
                pid,
                kill,
                reap,
                timed_out,
                descendants_seen: descendants,
                pid_present_after_reap,
                wait_errors,
            },
            timing: TimingFacts {
                attempt_ms: policy.timeout.as_millis() as u64,
                cleanup_ms: policy.cleanup.as_millis() as u64,
                elapsed_ms,
            },
        },
    })
}

#[derive(Debug)]
pub struct NativeRun {
    /// First-allocation candidate (narrow claim). Complete success uses `incompleteness`.
    pub bound: Option<BoundAttempt>,
    /// First-observation candidate (narrow claim).
    pub observation: Option<NativeObservation>,
    /// First diagnostic of the attempt; never the only record of a defect.
    pub error_frame: Option<String>,
    /// Every stdout byte retained, exactly as received (framing included, nothing decoded).
    pub raw_stdout: Vec<u8>,
    /// Every stderr byte retained, up to the stderr bound.
    pub raw_stderr: Vec<u8>,
    /// Exact bytes of every frame whose send began.
    pub sent_bytes: Vec<u8>,
    pub facts: AttemptFacts,
}

impl NativeRun {
    /// Delivery-owned interpretation of the whole received stream.
    pub fn interpretation(&self) -> StreamInterpretation {
        interpret_stream(&self.raw_stdout)
    }

    pub fn stderr_lossy(&self) -> String {
        String::from_utf8_lossy(&self.raw_stderr).into_owned()
    }

    /// The delivery-owned verdict on the retained evidence (see [`check_attempt_evidence`]).
    pub fn evidence_verdict(&self) -> Result<Vec<String>, Vec<String>> {
        check_attempt_evidence(
            &self.facts,
            &self.raw_stdout,
            &self.raw_stderr,
            &self.sent_bytes,
        )
    }

    /// Every reason this run is not a complete native success; empty only for a complete clean
    /// attempt whose candidates equal the interpretation of the whole stream.
    pub fn incompleteness(&self) -> Vec<String> {
        let mut why = match self.evidence_verdict() {
            Ok(why) => why,
            Err(bad) => vec![format!("attempt evidence is contradictory: {bad:?}")],
        };
        let interp = self.interpretation();
        if interp.permitted_success()
            && (self.bound.as_ref().map(|b| b.allocation()) != interp.allocation.as_ref()
                || self.observation != interp.observation)
        {
            why.push("first-frame candidates differ from the interpreted stream".to_owned());
        }
        why
    }
}

/// The delivery protocol over the real SDK child: bind the first allocation, send the start
/// frame bound to it and close stdin after it (the adapter observes only after host input ended).
struct NativeDriver<'a> {
    binding: &'a ExperimentBinding,
    control: Control,
    bound: Option<BoundAttempt>,
    diagnostics: Vec<String>,
    decided: bool,
}

impl AttemptDriver for NativeDriver<'_> {
    fn on_stdout(&mut self, raw: &[u8]) -> DriverStep {
        let mut step = DriverStep::default();
        // Only complete frames: an open frame may still change.
        let Some(last) = raw.iter().rposition(|b| *b == b'\n') else {
            return step;
        };
        let interp = interpret_stream(&raw[..=last]);
        if !self.decided {
            self.decided = true;
            let first_allocation = interp.kinds.first().map(String::as_str) == Some("allocated");
            match (first_allocation, interp.allocation.clone()) {
                (true, Some(a)) => match BoundAttempt::bind(self.binding.frozen().clone(), a) {
                    Ok(b) => {
                        let frozen = self.binding.frozen();
                        let session = if self.control == Control::WrongSession {
                            format!("{}-not-allocated", b.session_id())
                        } else {
                            b.session_id().to_owned()
                        };
                        let start = json!({
                            "type": "start",
                            "session_id": session,
                            "attempt_id": frozen.transport().attempt_id,
                            "generation": frozen.transport().generation,
                            "contract": frozen.contract(),
                            "contract_sha256": frozen.contract_sha256(),
                            "result_text": self.binding.synthetic_result(),
                        })
                        .to_string();
                        let sends = if self.control.duplicate_start() { 2 } else { 1 };
                        step.send = vec![start; sends];
                        // No further host frame follows: the adapter adjudicates every host frame
                        // and emits an observation only after the host input ended.
                        step.close_stdin = true;
                        self.bound = Some(b);
                    }
                    Err(e) => {
                        self.diagnostics.push(format!("allocation refused: {e}"));
                        step.close_stdin = true;
                    }
                },
                _ => step.close_stdin = true,
            }
        }
        if interp.observation.is_some() {
            step.close_stdin = true;
        }
        step
    }
}

/// Runs one attempt of the provider-disabled SDK adapter under the owned-attempt lifecycle of
/// [`run_attempt`]. Setup and spawn failures are `Err` (no child exists); every later problem is
/// a returned run carrying its truthful facts.
pub fn run_native(
    cfg: &NativeConfig,
    binding: &ExperimentBinding,
) -> Result<NativeRun, NativeError> {
    let cwd = setup_fixture_root(&cfg.fixture_root)?;
    let root = cfg
        .fixture_root
        .to_str()
        .ok_or_else(|| NativeError::Setup("non-utf8 root".into()))?;
    let mut command = Command::new(&cfg.node);
    command
        .arg(&cfg.adapter)
        .args(["--sdk", cfg.sdk_root.to_str().unwrap_or_default()])
        .args(["--fixture", root])
        .args(["--cwd", cwd.to_str().unwrap_or_default()])
        .args(["--control", &cfg.control.adapter_name()])
        .env_clear()
        .env("HOME", cfg.fixture_root.join("home"))
        .env("XDG_CONFIG_HOME", cfg.fixture_root.join("xdg/config"))
        .env("XDG_DATA_HOME", cfg.fixture_root.join("xdg/data"))
        .env("XDG_CACHE_HOME", cfg.fixture_root.join("xdg/cache"))
        .env("XDG_STATE_HOME", cfg.fixture_root.join("xdg/state"))
        .env("TMPDIR", cfg.fixture_root.join("tmp"))
        .current_dir(&cwd);
    let mut driver = NativeDriver {
        binding,
        control: cfg.control,
        bound: None,
        diagnostics: Vec::new(),
        decided: false,
    };
    let record = run_attempt(
        command,
        &mut driver,
        &AttemptPolicy {
            timeout: cfg.timeout,
            cleanup: cfg.cleanup,
            limits: cfg.limits,
            write_shape: cfg.control.write_shape(),
        },
    )?;
    let interp = interpret_stream(&record.raw_stdout);
    let bound = driver.bound;
    let error_frame = driver
        .diagnostics
        .into_iter()
        .next()
        .or_else(|| interp.defects.first().cloned())
        .or_else(|| interp.error_message.clone())
        .or_else(|| {
            (bound.is_some() && interp.observation.is_none())
                .then(|| "no native observation arrived".to_owned())
        });
    Ok(NativeRun {
        bound,
        observation: interp.observation,
        error_frame,
        raw_stdout: record.raw_stdout,
        raw_stderr: record.raw_stderr,
        sent_bytes: record.sent_bytes,
        facts: record.facts,
    })
}

fn setup_fixture_root(root: &Path) -> Result<PathBuf, NativeError> {
    let err = |e: std::io::Error| NativeError::Setup(e.to_string());
    for dir in [
        "home",
        "xdg/config",
        "xdg/data",
        "xdg/cache",
        "xdg/state",
        "agent",
        "tmp",
        "canary/work",
        "canary/.pi/skills/canary",
    ] {
        fs::create_dir_all(root.join(dir)).map_err(err)?;
    }
    for (path, body) in [
        (
            "canary/AGENTS.md",
            format!("# canary context file {CANARY_MARKER}\n"),
        ),
        (
            "canary/CLAUDE.md",
            format!("# canary context file {CANARY_MARKER}\n"),
        ),
        (
            "canary/.pi/skills/canary/SKILL.md",
            format!("---\nname: canary\ndescription: {CANARY_MARKER}\n---\n"),
        ),
        (
            "agent/AGENTS.md",
            format!("# canary agent-dir file {CANARY_MARKER}\n"),
        ),
    ] {
        fs::write(root.join(path), body).map_err(err)?;
    }
    Ok(root.join("canary/work"))
}

fn children_of(pid: u32) -> Vec<u32> {
    let mut found = Vec::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let Some(other) = entry
                .file_name()
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
            else {
                continue;
            };
            if let Ok(stat) = fs::read_to_string(format!("/proc/{other}/stat")) {
                // field 4 (after the parenthesised comm) is the parent pid
                let parent = stat
                    .rsplit_once(')')
                    .and_then(|(_, rest)| rest.trim().split(' ').nth(1)?.parse::<u32>().ok());
                if parent == Some(pid) {
                    found.push(other);
                }
            }
        }
    }
    found
}

#[derive(Debug)]
pub struct Delivered {
    pub verified: VerifiedDelivery,
    pub consumption: FixtureConsumption,
    pub run: NativeRun,
}

#[derive(Debug, thiserror::Error)]
pub enum DeliverError {
    #[error(transparent)]
    Native(#[from] NativeError),
    /// Correspondence refused or uncertain, or the attempt was not a complete clean run; the
    /// consumer was not called. The run carries all evidence and the actual disposition.
    #[error("delivery refused: {refusal}")]
    Delivery {
        refusal: DeliveryRefusal,
        run: Box<NativeRun>,
    },
    /// Native correspondence held; the existing consumer refused. Its error is unchanged.
    #[error("consumer refused: {refusal}")]
    Consumer {
        refusal: ConsumeRefusal,
        verified: Box<VerifiedDelivery>,
        run: Box<NativeRun>,
    },
}

/// Proof that an attempt was complete, constructible only here, only from a run that passed
/// `NativeRun::incompleteness`. The binding is consumed against this value alone.
struct CompletedRun<'a> {
    bound: &'a BoundAttempt,
    observation: &'a NativeObservation,
}

fn completed(run: &NativeRun) -> Result<CompletedRun<'_>, DeliveryRefusal> {
    let why = run.incompleteness();
    if !why.is_empty() {
        return Err(DeliveryRefusal::Incomplete(why.join("; ")));
    }
    match (&run.bound, &run.observation) {
        (Some(bound), Some(observation)) => Ok(CompletedRun { bound, observation }),
        _ => Err(DeliveryRefusal::Incomplete(
            "complete protocol without a bound allocation and observation".into(),
        )),
    }
}

/// The vertical: run the native SDK from the checked binding to its complete end, require a
/// complete clean attempt over the whole stream, verify exact observation correspondence, then
/// call the existing `consume_fixture_result` with the text found in the native history and the
/// binding's own candidate, declaration and episode inputs. There is no independent candidate,
/// preparation or declaration argument and no other route to a consumption in this crate.
#[allow(clippy::result_large_err)] // the error carries the retained native run so failure evidence survives
pub fn deliver_and_consume(
    cfg: &NativeConfig,
    binding: &ExperimentBinding,
) -> Result<Delivered, DeliverError> {
    let run = run_native(cfg, binding)?;
    let verdict = completed(&run).and_then(|done| {
        done.bound
            .verify(done.observation, binding.synthetic_result())
    });
    let verified = match verdict {
        Ok(v) => v,
        Err(refusal) => {
            return Err(DeliverError::Delivery {
                refusal,
                run: Box::new(run),
            });
        }
    };
    match binding.consume(&verified.result_text) {
        Ok(consumption) => Ok(Delivered {
            verified,
            consumption,
            run,
        }),
        Err(refusal) => Err(DeliverError::Consumer {
            refusal,
            verified: Box::new(verified),
            run: Box::new(run),
        }),
    }
}
