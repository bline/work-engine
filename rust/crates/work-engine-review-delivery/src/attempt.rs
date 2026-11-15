//! Delivery-owned meaning of one native attempt's retained evidence. Compiled in every build (no
//! process code): the fixture runner produces these facts, the capture writes them, and the
//! campaign qualifier strictly decodes them and asks this module, not a second definition,
//! whether they are consistent with the retained bytes and what they establish.
//!
//! Two results are deliberately distinct. [`check_attempt_evidence`] returns `Err` when the
//! evidence is malformed or contradicts itself or the retained bytes: such evidence establishes
//! nothing, in particular not a valid failed attempt. It returns `Ok(reasons)` for consistent
//! evidence; an empty list means a complete clean attempt, a non-empty list names every truthful
//! reason the attempt was not one.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::interpret_stream;

/// How one owned output stream ended. Bytes before the end are retained exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StreamEnd {
    /// The pipe reported end of file.
    Eof,
    /// A read failed; the bytes before it are retained.
    ReadError { error: String },
    /// One frame exceeded the frame bound: its first `frame_limit_bytes + 1` bytes are retained,
    /// bytes after them in the same read are counted as overflow, nothing later is read.
    FrameLimit,
    /// The total bound was reached while more bytes were available; those bytes are counted as
    /// overflow and nothing later is read.
    TotalLimit,
    /// The attempt owner closed the still-open stream when the cleanup bound elapsed. Coverage
    /// after the retained prefix is unknown.
    Cancelled,
    /// The stream never started (pipe unavailable or could not be made non-blocking).
    NotStarted { reason: String },
}

/// One owned output stream. A started stream is read only by the attempt owner on the calling
/// thread and is closed by it before the attempt returns: there is no helper thread, so stream
/// termination and collector join are the same owned event and nothing outlives the attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamRecord {
    pub end: StreamEnd,
    /// Bytes retained (equals the retained artifact length).
    pub retained_bytes: u64,
    /// Bytes read from the pipe but not retained because a bound was reached (known overflow).
    pub overflow_bytes_observed: u64,
    pub limit_bytes: u64,
    /// Frame bound for newline-framed streams (stdout); `None` for unframed stderr.
    pub frame_limit_bytes: Option<u64>,
}

/// Why a send stopped before the frame was fully accepted by the pipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SendCancel {
    /// The attempt deadline passed during the send.
    Deadline,
    /// The attempt stopped for another recorded reason (collection stopped, owner I/O failure).
    AttemptStopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SendOutcome {
    /// Every frame byte was accepted by the pipe. Whether the child read them is not observable.
    Completed,
    /// A write failed (for example a closed pipe) after `written` bytes were accepted.
    Failed {
        error: String,
    },
    Cancelled {
        reason: SendCancel,
    },
}

/// One frame the owner began to send. A record exists from the first write call on; its exact
/// intended bytes (text plus newline) are retained in order in the sent-bytes artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendRecord {
    pub frame_sha256: String,
    pub frame_len: u64,
    /// Bytes accepted by the pipe before the outcome.
    pub written: u64,
    pub outcome: SendOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KillReason {
    Deadline,
    /// The owner stopped reading a stream (limit or read error); a child blocked on that pipe
    /// could never finish.
    CollectionStopped,
    /// A pipe was unavailable or the owner's own I/O multiplexing failed.
    OwnerIoFailure,
    /// A wait observation failed: whether the child ended is unknown, so the owner may no longer
    /// rely on observing its exit and stops it within the cleanup bound.
    WaitFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum KillFact {
    NotSent,
    Sent { reason: KillReason },
    Failed { reason: KillReason, error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReapFact {
    Exited {
        code: i32,
    },
    Signalled {
        signal: i32,
    },
    /// A wait status with neither exit code nor signal.
    UnknownStatus {
        raw: i32,
    },
    /// Waiting failed; whether the child ended is unknown.
    WaitFailed {
        error: String,
    },
    /// The cleanup bound elapsed before the child could be reaped; its end is unknown.
    NotReaped,
}

impl ReapFact {
    pub fn reaped(&self) -> bool {
        matches!(
            self,
            ReapFact::Exited { .. } | ReapFact::Signalled { .. } | ReapFact::UnknownStatus { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessFacts {
    pub pid: u32,
    pub kill: KillFact,
    pub reap: ReapFact,
    pub timed_out: bool,
    /// Direct children of the child seen in /proc while it was alive (none expected).
    pub descendants_seen: Vec<u32>,
    /// Whether `/proc/<pid>` existed after a reap; `None` when no reap happened.
    pub pid_present_after_reap: Option<bool>,
    /// Every failed wait observation, in order (consecutive repeats once). A failed wait never
    /// establishes termination; a later successful wait may.
    pub wait_errors: Vec<String>,
}

/// The attempt's time budget and its measured duration. `elapsed_ms` covers the complete
/// caller-visible operation, from spawn to return, including post-loop frame digests, release of
/// still-open pipes and evidence finalization. The owner enters cleanup at the deadline at the
/// latest and leaves its loop at the cleanup bound whatever the child or pipes do, but these are
/// checked bounds, not preemption: driver callbacks, `/proc` sampling, per-frame copies and the
/// post-loop work run to completion between checks. No universal wall-clock ceiling is claimed;
/// `elapsed_ms - (attempt_ms + cleanup_ms)` is the observed excess, which consumers may bound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimingFacts {
    pub attempt_ms: u64,
    pub cleanup_ms: u64,
    pub elapsed_ms: u64,
}

/// Every non-byte fact of one native attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptFacts {
    pub sent: Vec<SendRecord>,
    pub stdout: StreamRecord,
    pub stderr: StreamRecord,
    pub process: ProcessFacts,
    pub timing: TimingFacts,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn stream_contradictions(name: &str, s: &StreamRecord, raw: &[u8], why: &mut Vec<String>) {
    let len = raw.len() as u64;
    if s.retained_bytes != len {
        why.push(format!(
            "{name}: retained_bytes {} but {len} bytes retained",
            s.retained_bytes
        ));
    }
    if len > s.limit_bytes {
        why.push(format!(
            "{name}: {len} bytes exceed the limit {}",
            s.limit_bytes
        ));
    }
    let tail = raw.rsplit(|b| *b == b'\n').next().unwrap_or(&[]);
    if let Some(f) = s.frame_limit_bytes {
        let complete = &raw[..raw.len() - tail.len()];
        let long = complete
            .split(|b| *b == b'\n')
            .any(|line| line.len() as u64 > f);
        if long {
            why.push(format!(
                "{name}: a complete frame exceeds the frame limit {f}"
            ));
        }
        let tail_len = tail.len() as u64;
        if s.end == StreamEnd::FrameLimit {
            if tail_len != f + 1 {
                why.push(format!(
                    "{name}: frame limit reported but the open frame holds {tail_len} bytes, not {}",
                    f + 1
                ));
            }
        } else if tail_len > f {
            why.push(format!(
                "{name}: open frame exceeds the frame limit without a frame-limit end"
            ));
        }
    } else if s.end == StreamEnd::FrameLimit {
        why.push(format!(
            "{name}: frame limit reported for an unframed stream"
        ));
    }
    match &s.end {
        StreamEnd::TotalLimit => {
            if len != s.limit_bytes || s.overflow_bytes_observed == 0 {
                why.push(format!(
                    "{name}: total limit needs exactly the bound retained and observed overflow"
                ));
            }
        }
        StreamEnd::FrameLimit => {}
        StreamEnd::NotStarted { .. } => {
            if len != 0 || s.overflow_bytes_observed != 0 {
                why.push(format!("{name}: a stream that never started has bytes"));
            }
        }
        StreamEnd::Eof | StreamEnd::ReadError { .. } | StreamEnd::Cancelled => {
            if s.overflow_bytes_observed != 0 {
                why.push(format!("{name}: overflow without a limit end"));
            }
        }
    }
}

/// Consistency of the facts with themselves and with the retained bytes; then every truthful
/// reason the attempt is not a complete clean run. See the module documentation.
pub fn check_attempt_evidence(
    facts: &AttemptFacts,
    raw_stdout: &[u8],
    raw_stderr: &[u8],
    sent_bytes: &[u8],
) -> Result<Vec<String>, Vec<String>> {
    let mut bad = Vec::new();
    stream_contradictions("stdout", &facts.stdout, raw_stdout, &mut bad);
    stream_contradictions("stderr", &facts.stderr, raw_stderr, &mut bad);
    if facts.stdout.frame_limit_bytes.is_none() {
        bad.push("stdout: newline-framed stream without a frame limit".into());
    }
    if facts.stderr.frame_limit_bytes.is_some() {
        bad.push("stderr: unframed stream with a frame limit".into());
    }

    // Sends: each record joins its exact retained frame bytes, in order.
    let mut offset = 0usize;
    for (n, s) in facts.sent.iter().enumerate() {
        if !is_hex64(&s.frame_sha256) {
            bad.push(format!("sent[{n}]: frame_sha256 is not a sha256 digest"));
        }
        let end = offset.saturating_add(s.frame_len as usize);
        match sent_bytes.get(offset..end) {
            Some(frame) => {
                if sha256_hex(frame) != s.frame_sha256 {
                    bad.push(format!(
                        "sent[{n}]: retained frame bytes do not hash to frame_sha256"
                    ));
                }
                if frame.last() != Some(&b'\n') || frame[..frame.len() - 1].contains(&b'\n') {
                    bad.push(format!(
                        "sent[{n}]: retained frame is not one newline-terminated frame"
                    ));
                }
            }
            None => bad.push(format!(
                "sent[{n}]: frame bytes beyond the retained sent bytes"
            )),
        }
        offset = end;
        let complete = s.written == s.frame_len;
        match (&s.outcome, complete) {
            (SendOutcome::Completed, true) => {}
            (SendOutcome::Completed, false) => bad.push(format!(
                "sent[{n}]: completed with {} of {} bytes",
                s.written, s.frame_len
            )),
            (_, _) if s.written >= s.frame_len => bad.push(format!(
                "sent[{n}]: incomplete outcome with every byte written"
            )),
            _ => {}
        }
        if !matches!(s.outcome, SendOutcome::Completed) && n + 1 != facts.sent.len() {
            bad.push(format!(
                "sent[{n}]: a later frame follows an incomplete send"
            ));
        }
    }
    if offset != sent_bytes.len() {
        bad.push(format!(
            "sent bytes: {} retained, {offset} accounted for by send records",
            sent_bytes.len()
        ));
    }

    let p = &facts.process;
    if p.pid == 0 {
        bad.push("process: pid 0".into());
    }
    if p.reap.reaped() != p.pid_present_after_reap.is_some() {
        bad.push(
            "process: pid_present_after_reap must be known exactly when the child was reaped"
                .into(),
        );
    }
    if let ReapFact::WaitFailed { error } = &p.reap
        && !p.wait_errors.contains(error)
    {
        bad.push("process: wait_failed reap names an error not among wait_errors".into());
    }
    if !p.wait_errors.is_empty()
        && !p.reap.reaped()
        && !matches!(p.reap, ReapFact::WaitFailed { .. })
    {
        bad.push(
            "process: termination unknown after wait errors but not recorded as wait_failed".into(),
        );
    }
    let deadline_kill = matches!(
        p.kill,
        KillFact::Sent {
            reason: KillReason::Deadline
        } | KillFact::Failed {
            reason: KillReason::Deadline,
            ..
        }
    );
    if deadline_kill && !p.timed_out {
        bad.push("process: a deadline kill without a timeout".into());
    }
    if p.timed_out
        && matches!(p.kill, KillFact::Sent { .. } | KillFact::Failed { .. })
        && !deadline_kill
    {
        // A kill for another reason precedes the deadline; the attempt then stops in cleanup.
        bad.push(
            "process: timed out after a non-deadline kill (cleanup never waits for the deadline)"
                .into(),
        );
    }
    if !bad.is_empty() {
        return Err(bad);
    }

    let mut why = Vec::new();
    if p.timed_out {
        why.push("the attempt timed out".to_owned());
    }
    if p.kill != KillFact::NotSent {
        why.push(format!("the runner had to kill the child: {:?}", p.kill));
    }
    match &p.reap {
        ReapFact::Exited { code: 0 } => {}
        other => why.push(format!("child did not reap as a clean exit: {other:?}")),
    }
    if !p.wait_errors.is_empty() {
        why.push(format!("wait observations failed: {:?}", p.wait_errors));
    }
    if p.pid_present_after_reap != Some(false) {
        why.push(format!(
            "child pid presence after reap: {:?}",
            p.pid_present_after_reap
        ));
    }
    if !p.descendants_seen.is_empty() {
        why.push(format!("unexpected descendants {:?}", p.descendants_seen));
    }
    for (name, s) in [("stdout", &facts.stdout), ("stderr", &facts.stderr)] {
        if s.end != StreamEnd::Eof {
            why.push(format!("{name} did not end in EOF: {:?}", s.end));
        }
    }
    let completed = facts
        .sent
        .iter()
        .filter(|s| s.outcome == SendOutcome::Completed)
        .count();
    if facts.sent.len() != 1 || completed != 1 {
        why.push(format!(
            "required start write not exactly one completed write ({} frames, {completed} completed)",
            facts.sent.len()
        ));
    }
    if let Some(s) = facts
        .sent
        .iter()
        .find(|s| s.outcome != SendOutcome::Completed)
    {
        why.push(format!(
            "send did not complete: {:?} after {} of {} bytes",
            s.outcome, s.written, s.frame_len
        ));
    }
    let interp = interpret_stream(raw_stdout);
    if !interp.permitted_success() {
        why.push(format!(
            "stream is not the permitted allocation+observation protocol: frames {:?}, defects {:?}",
            interp.kinds, interp.defects
        ));
    }
    Ok(why)
}
