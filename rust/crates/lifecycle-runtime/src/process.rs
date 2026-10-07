use std::collections::BTreeMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex as AsyncMutex;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ChildId(u64);

impl ChildId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildExit {
    pub child: ChildId,
    pub pid: Option<u32>,
    pub born_at: Instant,
    pub code: Option<i32>,
    pub success: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloseDisposition {
    Resolved,
    ResolvedLate,
    TimedOut,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActivationState {
    NotAttempted,
    Requested,
    Confirmed,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildClose {
    pub child: ChildId,
    pub activation: ActivationState,
    pub disposition: CloseDisposition,
    pub exit: Option<ChildExit>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessShutdownReport {
    pub children: Vec<ChildClose>,
    pub unresolved: Vec<ChildId>,
    pub unsafe_at_close: bool,
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("a Tokio runtime is required to own the child process")]
    NoRuntime,
    #[error("process launch gate is closed")]
    Closed,
    #[error("process registry capacity is exhausted")]
    Capacity,
    #[error("unknown child handle")]
    UnknownChild,
    #[error("child activation has already been attempted")]
    AlreadyActivated,
    #[error("child activation write may have entered before failure")]
    ActivationUncertain,
    #[error("process operation failed: {0}")]
    Io(#[from] std::io::Error),
}

struct ProcessRecord {
    pid: Option<u32>,
    born_at: Instant,
    activation: ActivationState,
    child: Arc<AsyncMutex<Child>>,
    exit: Option<ChildExit>,
    exit_observed_at: Option<Instant>,
    late_at_close: bool,
    unresolved_disposition: CloseDisposition,
}

struct ProcessRegistry {
    open: bool,
    next_id: u64,
    unsafe_history: bool,
    children: BTreeMap<ChildId, ProcessRecord>,
}

pub struct ProcessSupervisor {
    registry: Mutex<ProcessRegistry>,
    drain_lock: AsyncMutex<()>,
    max_children: usize,
}

struct ActivationGuard<'a> {
    supervisor: &'a ProcessSupervisor,
    child: ChildId,
    finished: bool,
}

impl Drop for ActivationGuard<'_> {
    fn drop(&mut self) {
        if !self.finished
            && let Some(record) = self
                .supervisor
                .registry
                .lock()
                .unwrap()
                .children
                .get_mut(&self.child)
        {
            record.activation = ActivationState::Uncertain;
        }
    }
}

impl ProcessSupervisor {
    pub fn new(max_children: usize) -> Result<Self, ProcessError> {
        if max_children == 0 {
            return Err(ProcessError::Capacity);
        }
        Ok(Self {
            registry: Mutex::new(ProcessRegistry {
                open: true,
                next_id: 1,
                unsafe_history: false,
                children: BTreeMap::new(),
            }),
            drain_lock: AsyncMutex::new(()),
            max_children,
        })
    }

    /// The controlled process must wait for `activate` before effectful work.
    /// Spawn and exact-handle registration are serialized with gate closure.
    pub fn launch(&self, command: &mut Command) -> Result<ChildId, ProcessError> {
        self.launch_with_hooks(command, || {}, || {})
    }

    fn launch_with_hooks<B, A>(
        &self,
        command: &mut Command,
        before_spawn: B,
        after_spawn: A,
    ) -> Result<ChildId, ProcessError>
    where
        B: FnOnce(),
        A: FnOnce(),
    {
        if tokio::runtime::Handle::try_current().is_err() {
            return Err(ProcessError::NoRuntime);
        }
        let mut registry = self.registry.lock().unwrap();
        if !registry.open {
            return Err(ProcessError::Closed);
        }
        if registry.children.len() >= self.max_children {
            return Err(ProcessError::Capacity);
        }
        let id = ChildId(registry.next_id);
        registry.next_id = registry
            .next_id
            .checked_add(1)
            .ok_or(ProcessError::Capacity)?;
        command.stdin(Stdio::piped()).kill_on_drop(true);
        before_spawn();
        let child = command.spawn()?;
        after_spawn();
        let pid = child.id();
        registry.children.insert(
            id,
            ProcessRecord {
                pid,
                born_at: Instant::now(),
                activation: ActivationState::NotAttempted,
                child: Arc::new(AsyncMutex::new(child)),
                exit: None,
                exit_observed_at: None,
                late_at_close: false,
                unresolved_disposition: CloseDisposition::TimedOut,
            },
        );
        Ok(id)
    }

    /// Gives the registered controlled peer its one activation token.
    pub async fn activate(&self, id: ChildId) -> Result<(), ProcessError> {
        let child = {
            let mut registry = self.registry.lock().unwrap();
            if !registry.open {
                return Err(ProcessError::Closed);
            }
            let record = registry
                .children
                .get_mut(&id)
                .ok_or(ProcessError::UnknownChild)?;
            if record.activation != ActivationState::NotAttempted {
                return Err(ProcessError::AlreadyActivated);
            }
            record.activation = ActivationState::Requested;
            record.child.clone()
        };
        let mut guard = ActivationGuard {
            supervisor: self,
            child: id,
            finished: false,
        };
        let mut child = child.lock().await;
        let mut stdin = child
            .stdin
            .take()
            .ok_or(ProcessError::ActivationUncertain)?;
        let outcome = async {
            stdin.write_all(b"GO\n").await?;
            stdin.flush().await
        }
        .await;
        drop(stdin);
        let mut registry = self.registry.lock().unwrap();
        if let Some(record) = registry.children.get_mut(&id) {
            record.activation = if outcome.is_ok() {
                ActivationState::Confirmed
            } else {
                ActivationState::Uncertain
            };
        }
        guard.finished = true;
        outcome.map_err(|_| ProcessError::ActivationUncertain)
    }

    pub fn active_count(&self) -> usize {
        self.registry
            .lock()
            .unwrap()
            .children
            .values()
            .filter(|record| record.exit.is_none())
            .count()
    }

    /// Close launches synchronously before any asynchronous drain begins.
    pub fn close_gate(&self) {
        let mut registry = self.registry.lock().unwrap();
        registry.open = false;
        if registry
            .children
            .values()
            .any(|record| record.exit.is_none())
        {
            registry.unsafe_history = true;
        }
    }

    /// Closes the launch gate before snapshotting exact child handles. A later
    /// reap never clears the latched unsafe-at-close history.
    pub async fn close_and_drain(
        &self,
        proof_deadline: Instant,
        cleanup_deadline: Instant,
    ) -> ProcessShutdownReport {
        self.close_gate();
        // The child and its exit record remain owned across cancellation. A
        // second drain may observe them, but never races a second reap.
        let _drain = match tokio::time::timeout_at(
            tokio::time::Instant::from_std(cleanup_deadline),
            self.drain_lock.lock(),
        )
        .await
        {
            Ok(guard) if Instant::now() < cleanup_deadline => guard,
            _ => return self.report_snapshot(proof_deadline),
        };
        let snapshot = {
            let registry = self.registry.lock().unwrap();
            registry
                .children
                .iter()
                .map(|(id, record)| {
                    (
                        *id,
                        record.child.clone(),
                        record.pid,
                        record.born_at,
                        record.exit.is_some(),
                    )
                })
                .collect::<Vec<_>>()
        };
        for (id, handle, pid, born_at, already_reaped) in snapshot {
            if already_reaped {
                continue;
            }
            let mut child = match tokio::time::timeout_at(
                tokio::time::Instant::from_std(cleanup_deadline),
                handle.lock(),
            )
            .await
            {
                Ok(child) if Instant::now() < cleanup_deadline => child,
                _ => {
                    self.registry
                        .lock()
                        .unwrap()
                        .children
                        .get_mut(&id)
                        .expect("serialized drain retains exact child")
                        .unresolved_disposition = CloseDisposition::TimedOut;
                    continue;
                }
            };
            let waited = tokio::time::timeout_at(
                tokio::time::Instant::from_std(proof_deadline),
                child.wait(),
            )
            .await;
            let (status, failure, cleanup_path) = match waited {
                Ok(Ok(status)) => (Some(status), None, false),
                Ok(Err(_)) => (None, Some(CloseDisposition::Rejected), false),
                Err(_) => {
                    let _ = child.start_kill();
                    match tokio::time::timeout_at(
                        tokio::time::Instant::from_std(cleanup_deadline),
                        child.wait(),
                    )
                    .await
                    {
                        Ok(Ok(status)) => (Some(status), None, true),
                        Ok(Err(_)) => (None, Some(CloseDisposition::Rejected), true),
                        Err(_) => (None, Some(CloseDisposition::TimedOut), true),
                    }
                }
            };
            let observed_at = status.as_ref().map(|_| Instant::now());
            drop(child);
            let mut registry = self.registry.lock().unwrap();
            let record = registry
                .children
                .get_mut(&id)
                .expect("serialized drain retains exact child");
            if let Some(status) = status {
                record.exit = Some(ChildExit {
                    child: id,
                    pid,
                    born_at,
                    code: status.code(),
                    success: status.success(),
                });
                record.exit_observed_at = observed_at;
                record.late_at_close |=
                    cleanup_path || observed_at.is_some_and(|at| at >= proof_deadline);
            } else if let Some(disposition) = failure {
                record.unresolved_disposition = disposition;
            }
        }
        self.report_snapshot(proof_deadline)
    }

    fn report_snapshot(&self, proof_deadline: Instant) -> ProcessShutdownReport {
        let registry = self.registry.lock().unwrap();
        let children = registry
            .children
            .iter()
            .map(|(id, record)| ChildClose {
                child: *id,
                activation: record.activation.clone(),
                disposition: if record.exit.is_some() {
                    if record.late_at_close
                        || record
                            .exit_observed_at
                            .is_some_and(|at| at >= proof_deadline)
                    {
                        CloseDisposition::ResolvedLate
                    } else {
                        CloseDisposition::Resolved
                    }
                } else {
                    record.unresolved_disposition.clone()
                },
                exit: record.exit.clone(),
            })
            .collect::<Vec<_>>();
        let unresolved = children
            .iter()
            .filter(|close| close.exit.is_none())
            .map(|close| close.child)
            .collect();
        ProcessShutdownReport {
            children,
            unresolved,
            unsafe_at_close: registry.unsafe_history,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, BufRead, BufReader, Write};
    use std::path::{Path, PathBuf};
    use std::sync::mpsc as std_mpsc;
    use std::time::Duration;

    struct ChildBarrier {
        _directory: tempfile::TempDir,
        ready: PathBuf,
        release: PathBuf,
    }

    impl ChildBarrier {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            Self {
                ready: directory.path().join("ready"),
                release: directory.path().join("release"),
                _directory: directory,
            }
        }

        fn configure(&self, command: &mut Command, allow_eof: bool) {
            command
                .env("S2_INTERNAL_CHILD_READY", &self.ready)
                .env("S2_INTERNAL_CHILD_RELEASE", &self.release);
            if allow_eof {
                command.env("S2_INTERNAL_CHILD_ALLOW_EOF", "1");
            }
        }

        fn publish_ready(path: &Path, pid: u32) -> io::Result<()> {
            Self::publish_ready_with_hook(path, pid, || {})
        }

        fn publish_ready_with_hook<F: FnOnce()>(
            path: &Path,
            pid: u32,
            before_publish: F,
        ) -> io::Result<()> {
            // The parent must never observe a newly created but incomplete
            // acknowledgment. A hard link publishes the closed staging file
            // atomically and fails if an acknowledgment already exists.
            let staged = path.with_extension("staged");
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&staged)?;
            file.write_all(pid.to_string().as_bytes())?;
            file.flush()?;
            drop(file);
            before_publish();
            std::fs::hard_link(&staged, path)?;
            let _ = std::fs::remove_file(staged);
            Ok(())
        }

        async fn wait_ready(&self, pid: Option<u32>, max_wait: Duration) -> io::Result<()> {
            let expected = pid.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "child has no process identity")
            })?;
            let observed = tokio::time::timeout(max_wait, async {
                loop {
                    match std::fs::read_to_string(&self.ready) {
                        Ok(value) => break Ok(value),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                            tokio::time::sleep(Duration::from_millis(1)).await;
                        }
                        Err(error) => break Err(error),
                    }
                }
            })
            .await
            .map_err(|_| {
                io::Error::new(io::ErrorKind::TimedOut, "child acknowledgment absent")
            })??;
            if observed != expected.to_string() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "malformed or mismatched child acknowledgment",
                ));
            }
            Ok(())
        }

        fn release(&self) {
            std::fs::write(&self.release, b"GO").unwrap();
        }
    }

    #[test]
    fn gated_child() {
        if std::env::var_os("S2_INTERNAL_CHILD_GATE").is_none() {
            return;
        }
        let mut token = String::new();
        let _ = BufReader::new(std::io::stdin()).read_line(&mut token);
        if token == "GO\n"
            || (token.is_empty() && std::env::var_os("S2_INTERNAL_CHILD_ALLOW_EOF").is_some())
        {
            if let Ok(ready) = std::env::var("S2_INTERNAL_CHILD_READY") {
                let release = std::env::var("S2_INTERNAL_CHILD_RELEASE").unwrap();
                ChildBarrier::publish_ready(Path::new(&ready), std::process::id()).unwrap();
                let deadline = Instant::now() + Duration::from_secs(15);
                while !Path::new(&release).exists() {
                    assert!(Instant::now() < deadline, "child release barrier expired");
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            std::process::exit(9);
        }
    }

    #[tokio::test]
    async fn child_acknowledgment_is_complete_exact_and_bounded() {
        let absent = ChildBarrier::new();
        assert_eq!(
            absent
                .wait_ready(Some(7), Duration::from_millis(10))
                .await
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
        let malformed = ChildBarrier::new();
        std::fs::write(&malformed.ready, b"7partial").unwrap();
        assert_eq!(
            malformed
                .wait_ready(Some(7), Duration::from_secs(1))
                .await
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );

        let complete = ChildBarrier::new();
        let (staged_tx, staged_rx) = std_mpsc::channel();
        let (publish_tx, publish_rx) = std_mpsc::channel();
        let path = complete.ready.clone();
        let publisher = std::thread::spawn(move || {
            ChildBarrier::publish_ready_with_hook(&path, 7, || {
                staged_tx.send(()).unwrap();
                let _ = publish_rx.recv();
            })
        });
        staged_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(
            std::fs::read_to_string(complete.ready.with_extension("staged")).unwrap(),
            "7"
        );
        assert!(!complete.ready.exists());
        assert_eq!(
            complete
                .wait_ready(Some(7), Duration::from_millis(10))
                .await
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
        publish_tx.send(()).unwrap();
        publisher.join().unwrap().unwrap();
        complete
            .wait_ready(Some(7), Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(
            complete
                .wait_ready(Some(8), Duration::from_secs(1))
                .await
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            ChildBarrier::publish_ready(&complete.ready, 8)
                .unwrap_err()
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read_to_string(&complete.ready).unwrap(), "7");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn launch_gate_tracks_child_across_both_pause_boundaries() {
        for pause_after_spawn in [false, true] {
            let supervisor = Arc::new(ProcessSupervisor::new(1).unwrap());
            let (paused_tx, paused_rx) = tokio::sync::oneshot::channel();
            let (release_tx, release_rx) = std_mpsc::channel();
            let launching = supervisor.clone();
            let launch = tokio::spawn(async move {
                let mut command = Command::new(std::env::current_exe().unwrap());
                command
                    .arg("--exact")
                    .arg("process::tests::gated_child")
                    .env("S2_INTERNAL_CHILD_GATE", "1")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                let hook = move || {
                    let _ = paused_tx.send(());
                    let _ = release_rx.recv();
                };
                if pause_after_spawn {
                    launching.launch_with_hooks(&mut command, || {}, hook)
                } else {
                    launching.launch_with_hooks(&mut command, hook, || {})
                }
            });
            paused_rx.await.unwrap();
            let closing = supervisor.clone();
            let (closing_tx, closing_rx) = tokio::sync::oneshot::channel();
            let close = tokio::spawn(async move {
                let _ = closing_tx.send(());
                closing.close_gate();
            });
            closing_rx.await.unwrap();
            release_tx.send(()).unwrap();
            let id = launch.await.unwrap().unwrap();
            close.await.unwrap();
            let report = supervisor
                .close_and_drain(Instant::now(), Instant::now() + Duration::from_secs(2))
                .await;
            assert_eq!(report.children.len(), 1);
            assert_eq!(report.children[0].child, id);
            assert!(report.children[0].exit.is_some());
            assert!(report.unresolved.is_empty());
            assert!(report.unsafe_at_close);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn overlapping_drains_keep_one_reaper_and_respect_shorter_cleanup_budget() {
        let supervisor = Arc::new(ProcessSupervisor::new(1).unwrap());
        let barrier = ChildBarrier::new();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .arg("--exact")
            .arg("process::tests::gated_child")
            .env("S2_INTERNAL_CHILD_GATE", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        barrier.configure(&mut command, false);
        let id = supervisor.launch(&mut command).unwrap();
        supervisor.activate(id).await.unwrap();
        let child = supervisor
            .registry
            .lock()
            .unwrap()
            .children
            .get(&id)
            .unwrap()
            .child
            .clone();
        let pid = supervisor.registry.lock().unwrap().children[&id].pid;
        barrier
            .wait_ready(pid, Duration::from_secs(2))
            .await
            .unwrap();

        let first_owner = supervisor.clone();
        let first = tokio::spawn(async move {
            first_owner
                .close_and_drain(
                    Instant::now() + Duration::from_secs(5),
                    Instant::now() + Duration::from_secs(10),
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if child.try_lock().is_err() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let short = supervisor
            .close_and_drain(
                Instant::now() + Duration::from_millis(10),
                Instant::now() + Duration::from_millis(30),
            )
            .await;
        assert!(!first.is_finished());
        assert_eq!(short.unresolved, vec![id]);
        assert_eq!(short.children[0].disposition, CloseDisposition::TimedOut);

        barrier.release();
        let reaped = first.await.unwrap();
        assert_eq!(reaped.children[0].exit.as_ref().unwrap().code, Some(9));
        assert!(reaped.unresolved.is_empty());
        let repeated = supervisor
            .close_and_drain(
                Instant::now() + Duration::from_secs(1),
                Instant::now() + Duration::from_secs(1),
            )
            .await;
        assert_eq!(repeated.children[0].exit, reaped.children[0].exit);
        assert!(repeated.unresolved.is_empty());
        assert_eq!(supervisor.active_count(), 0);
    }

    #[tokio::test]
    async fn ready_exit_after_cutoff_and_later_drain_keep_late_disposition() {
        let supervisor = ProcessSupervisor::new(1).unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .arg("--exact")
            .arg("process::tests::gated_child")
            .env("S2_INTERNAL_CHILD_GATE", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let id = supervisor.launch(&mut command).unwrap();
        supervisor.activate(id).await.unwrap();
        let child = supervisor
            .registry
            .lock()
            .unwrap()
            .children
            .get(&id)
            .unwrap()
            .child
            .clone();
        assert_eq!(child.lock().await.wait().await.unwrap().code(), Some(9));

        let past = Instant::now() - Duration::from_millis(1);
        let first = supervisor
            .close_and_drain(past, Instant::now() + Duration::from_secs(1))
            .await;
        assert_eq!(
            first.children[0].disposition,
            CloseDisposition::ResolvedLate
        );
        assert_eq!(first.children[0].exit.as_ref().unwrap().code, Some(9));
        let resumed = supervisor
            .close_and_drain(
                Instant::now() + Duration::from_secs(1),
                Instant::now() + Duration::from_secs(1),
            )
            .await;
        assert_eq!(
            resumed.children[0].disposition,
            CloseDisposition::ResolvedLate
        );
        assert_eq!(resumed.children[0].exit, first.children[0].exit);
    }

    #[tokio::test]
    async fn cancelled_drain_waiter_leaves_exact_child_for_late_reap() {
        let supervisor = ProcessSupervisor::new(1).unwrap();
        let barrier = ChildBarrier::new();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .arg("--exact")
            .arg("process::tests::gated_child")
            .env("S2_INTERNAL_CHILD_GATE", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        barrier.configure(&mut command, true);
        let id = supervisor.launch(&mut command).unwrap();
        let pid = supervisor.registry.lock().unwrap().children[&id].pid;
        let mut interrupted = Box::pin(supervisor.close_and_drain(
            Instant::now() + Duration::from_secs(5),
            Instant::now() + Duration::from_secs(10),
        ));
        tokio::select! {
            _ = &mut interrupted => panic!("held child cannot be reaped"),
            ready = barrier.wait_ready(pid, Duration::from_secs(2)) => ready.unwrap(),
        }
        // Child::wait has polled and closed stdin; the child acknowledged that
        // fact but remains held by an independent release barrier.
        drop(interrupted);
        barrier.release();
        let resumed = supervisor
            .close_and_drain(Instant::now(), Instant::now() + Duration::from_secs(1))
            .await;
        assert_eq!(resumed.children[0].child, id);
        assert_eq!(
            resumed.children[0].disposition,
            CloseDisposition::ResolvedLate
        );
        assert!(resumed.children[0].exit.is_some());
        assert!(resumed.unresolved.is_empty());
        assert_eq!(supervisor.active_count(), 0);
    }
}
