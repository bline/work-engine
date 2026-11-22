//! Unselected Stage E inference-entry candidate. The pinned bootstrap does not
//! load the receiver (which reads the staged task at module scope) until the
//! outer Host writes one byte. Its stderr marker schedules observation; it is
//! not evidence of the task's identity, grants, or custody.
use super::{Host, Phase, pin_is_hash};
use crate::{Error, Result, refuse};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs::File,
    io::{Read, Write},
    os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    os::unix::ffi::OsStrExt,
    path::Path,
    time::Instant,
};

const MARKER: &[u8] = b"work-engine: inference-entry held\n";
const RELEASE: &[u8] = b"G";
const BOOTSTRAP: &str = r#"import {readSync,writeSync} from 'node:fs';
writeSync(2,'work-engine: inference-entry held\n');
const release=Buffer.alloc(1);
if(readSync(0,release,0,1,null)!==1 || release[0]!==71) process.exit(2);
await import('file:///receiver/receiver.mjs');"#;

pub(super) fn command() -> Vec<String> {
    [
        "node",
        "--input-type=module",
        "--eval",
        BOOTSTRAP,
        "--",
        "/receiver/receiver.mjs",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

/// Exact candidate subject. PID is the running container's init task in the
/// outer PID namespace, not the process that runs `docker start`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeldInference {
    pub job_id: String,
    pub input_digest: String,
    pub container_id: String,
    pub pid: u32,
    pub start_ticks: String,
}

/// An outside observer's exact-match release decision. The digest refers to
/// its protected evidence, not stdout or a record created by this adapter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceEntryObservation {
    pub subject: HeldInference,
    pub evidence_sha256: String,
}

/// The operator binds trusted observer code in the outer Host, never via the
/// job, child protocol, journal, or a result-supplied locator. The observer
/// must independently use the live task identity, check effective grants and
/// evidence custody for this exact dispatch, and return within `until`. This
/// adapter checks correlation and timeliness, not the observer's OS authority.
pub trait InferenceEntryObserver {
    fn observe(
        &mut self,
        subject: &HeldInference,
        task: BorrowedFd<'_>,
        until: Instant,
    ) -> Result<Option<InferenceEntryObservation>>;
}

struct TaskPin {
    pid: u32,
    start_ticks: String,
    fd: OwnedFd,
}

impl TaskPin {
    fn open(pid: u32, command: &[String]) -> Result<Self> {
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let fd = unsafe { OwnedFd::from_raw_fd(fd as _) };
        let start_ticks = crate::proc_stat(pid)
            .ok_or_else(|| Error::Refused("inference task start identity unavailable".into()))?
            .0;
        let task = Self {
            pid,
            start_ticks,
            fd,
        };
        task.check(command)?;
        Ok(task)
    }

    fn check(&self, command: &[String]) -> Result<()> {
        let mut poll = libc::pollfd {
            fd: self.fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        if unsafe { libc::poll(&mut poll, 1, 0) } != 0
            || crate::proc_stat(self.pid).is_none_or(|(ticks, _, state)| {
                ticks != self.start_ticks || matches!(state.as_str(), "Z" | "X")
            })
        {
            return refuse("held inference task exited or changed identity");
        }
        let mut raw = Vec::new();
        File::open(format!("/proc/{}/cmdline", self.pid))?
            .take(4097)
            .read_to_end(&mut raw)?;
        let Some(argv) = raw.strip_suffix(b"\0") else {
            return refuse("held inference task argv unavailable");
        };
        let mut argv = argv.split(|b| *b == 0);
        if raw.len() > 4096
            || argv
                .next()
                .and_then(|s| Path::new(OsStr::from_bytes(s)).file_name())
                != Some(OsStr::new("node"))
            || !argv.eq(command.iter().skip(1).map(|s| s.as_bytes()))
        {
            return refuse("running task is not the exact inference hold bootstrap");
        }
        Ok(())
    }
}

/// Returns true until the complete scheduling marker is available. Nothing
/// else may be emitted before release. Never read an unbounded child log.
pub(super) fn waiting_for_marker(stdout_bytes: u64, stderr: &Path) -> Result<bool> {
    let mut marker = Vec::new();
    File::open(stderr)?
        .take((MARKER.len() + 1) as u64)
        .read_to_end(&mut marker)?;
    waiting_for_marker_bytes(stdout_bytes, &marker)
}

fn waiting_for_marker_bytes(stdout_bytes: u64, stderr: &[u8]) -> Result<bool> {
    if stdout_bytes != 0 || !MARKER.starts_with(stderr) {
        return refuse("inference output before entry release");
    }
    Ok(stderr.len() < MARKER.len())
}

fn release_after_observation(
    expected: &HeldInference,
    observe: impl FnOnce() -> Result<Option<InferenceEntryObservation>>,
    mut check_current: impl FnMut() -> Result<()>,
    retain: impl FnOnce(InferenceEntryObservation) -> Result<()>,
    stdin: &mut impl Write,
) -> Result<()> {
    check_current()?;
    let observation =
        observe()?.ok_or_else(|| Error::Refused("outside inference observation absent".into()))?;
    if &observation.subject != expected || !pin_is_hash(&observation.evidence_sha256) {
        return refuse("outside inference observation identity/evidence mismatch");
    }
    check_current()?;
    retain(observation)?;
    check_current()?;
    stdin.write_all(RELEASE)?;
    stdin.flush()?;
    Ok(())
}

impl Host {
    /// Explicit opt-in for one freshly admitted credential-free battery
    /// candidate. No existing CLI or profile selector enables this path.
    pub fn bind_battery_inference_hold(
        &mut self,
        observer: Box<dyn InferenceEntryObserver>,
    ) -> Result<()> {
        let offline = self.journal.job.coding.as_ref().is_some_and(|profile| {
            super::coding::is_battery(&profile.schema)
                && profile.provider.as_ref().is_some_and(|plan| {
                    !plan.hosted.routes.is_empty()
                        && plan.hosted.routes.iter().all(|route| {
                            route
                                .endpoint
                                .strip_prefix("loopback:")
                                .and_then(|port| port.parse::<u16>().ok())
                                .is_some_and(|port| port > 0)
                        })
                })
        });
        if !offline || self.journal.phase != Phase::Admitted || self.journal.inference_entry_hold {
            return refuse("inference entry hold requires a fresh offline battery");
        }
        // Preserve the exact command shape for reconciliation; a reopened
        // Host cannot reconstruct or replace the live observer binding.
        self.journal.inference_entry_hold = true;
        self.save()?;
        self.inference_entry_observer = Some(observer);
        Ok(())
    }

    pub(super) fn check_inference_hold_deadline(&self, until: Instant) -> Result<()> {
        if Instant::now() >= until || self.expired() || self.cancelled() {
            return refuse("inference entry hold deadline elapsed or cancelled");
        }
        Ok(())
    }

    pub(super) fn release_inference_entry(
        &mut self,
        until: Instant,
        docker_client_pid: u32,
        stdout_path: &Path,
        stdin: &mut impl Write,
    ) -> Result<()> {
        self.check_inference_hold_deadline(until)?;
        if self.journal.phase != Phase::StartUnknown
            || self.journal.inference_entry_observation.is_some()
        {
            return refuse("inference entry release is not replayable");
        }
        // This inspection correlates Docker's container ID and kernel PID;
        // it is not a grant attestation. Only the outside callback supplies it.
        let inspect = self.inspect_container()?;
        let state = &inspect["State"];
        let pid = state["Pid"]
            .as_u64()
            .and_then(|pid| u32::try_from(pid).ok())
            .filter(|pid| *pid > 1 && *pid != docker_client_pid)
            .ok_or_else(|| {
                Error::Refused("running inference kernel PID absent or crossed".into())
            })?;
        if state["Running"] != true || state["Paused"] != false || state["Restarting"] != false {
            return refuse("inference task not running at its entry hold");
        }
        let command = super::coding::inference_command(&self.journal);
        let task = TaskPin::open(pid, &command)?;
        let expected = HeldInference {
            job_id: self.journal.job.job_id.clone(),
            input_digest: self.journal.input_digest.clone(),
            container_id: self
                .journal
                .container_id
                .clone()
                .ok_or_else(|| Error::Refused("inference container ID absent".into()))?,
            pid,
            start_ticks: task.start_ticks.clone(),
        };
        let mut observer = self
            .inference_entry_observer
            .take()
            .ok_or_else(|| Error::Refused("outside inference entry observer not bound".into()))?;
        let deadline = self.journal.job.deadline_unix_seconds;
        let cancel_path = self.journal.job.state_dir.join("cancel.json");
        release_after_observation(
            &expected,
            || observer.observe(&expected, std::os::fd::AsFd::as_fd(&task.fd), until),
            || {
                if Instant::now() >= until
                    || crate::now() >= deadline
                    || crate::STOP.load(std::sync::atomic::Ordering::SeqCst)
                    || cancel_path.try_exists()?
                {
                    return refuse("inference entry hold deadline elapsed or cancelled");
                }
                if std::fs::metadata(stdout_path)?.len() != 0 {
                    return refuse("inference output before entry release");
                }
                task.check(&command)
            },
            |observation| {
                self.journal.inference_entry_observation = Some(observation);
                self.save()
            },
            stdin,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn subject() -> HeldInference {
        HeldInference {
            job_id: "stage-e".into(),
            input_digest: "a".repeat(64),
            container_id: "b".repeat(64),
            pid: 1101,
            start_ticks: "2000".into(),
        }
    }

    #[test]
    fn inference_entry_refuses_absent_crossed_invalid_and_late_observation() {
        let expected = subject();
        for fault in 0..9 {
            let checks = Cell::new(0);
            let mut evidence = None;
            let mut bytes = Vec::new();
            let mut obs = InferenceEntryObservation {
                subject: expected.clone(),
                evidence_sha256: "c".repeat(64),
            };
            match fault {
                1 => obs.subject.job_id.push('x'),
                2 => obs.subject.input_digest = "d".repeat(64),
                3 => obs.subject.container_id = "e".repeat(64),
                4 => obs.subject.pid += 1,
                5 => obs.subject.start_ticks.push('1'),
                6 => obs.evidence_sha256.clear(),
                _ => (),
            }
            let result = release_after_observation(
                &expected,
                || Ok((fault != 0).then_some(obs)),
                || {
                    checks.set(checks.get() + 1);
                    if fault == 7 && checks.get() == 2 || fault == 8 && checks.get() == 3 {
                        return refuse("inference entry hold deadline elapsed");
                    }
                    Ok(())
                },
                |value| {
                    evidence = Some(value);
                    Ok(())
                },
                &mut bytes,
            );
            assert!(result.is_err(), "fault {fault}");
            assert!(bytes.is_empty(), "fault {fault}");
            assert_eq!(evidence.is_some(), fault == 8);
        }
    }

    #[test]
    fn inference_entry_persists_exact_outside_decision_before_single_release_byte() {
        let expected = subject();
        let obs = InferenceEntryObservation {
            subject: expected.clone(),
            evidence_sha256: "c".repeat(64),
        };
        let mut bytes = Vec::new();
        let mut retained = None;
        release_after_observation(
            &expected,
            || Ok(Some(obs.clone())),
            || Ok(()),
            |value| {
                retained = Some(value);
                Ok(())
            },
            &mut bytes,
        )
        .unwrap();
        assert_eq!(retained, Some(obs.clone()));
        assert_eq!(bytes, RELEASE);
        bytes.clear();
        assert!(
            release_after_observation(
                &expected,
                || Ok(Some(obs)),
                || Ok(()),
                |_| refuse("no custody write"),
                &mut bytes
            )
            .is_err()
        );
        assert!(bytes.is_empty());
    }

    #[test]
    fn inference_entry_child_marker_is_only_a_schedule_signal() {
        assert!(waiting_for_marker_bytes(0, &[]).unwrap());
        assert!(waiting_for_marker_bytes(0, &MARKER[..5]).unwrap());
        assert!(!waiting_for_marker_bytes(0, MARKER).unwrap());
        assert!(waiting_for_marker_bytes(1, MARKER).is_err());
        assert!(waiting_for_marker_bytes(0, b"different output").is_err());
    }
}
