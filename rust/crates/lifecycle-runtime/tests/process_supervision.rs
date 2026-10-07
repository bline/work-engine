use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::time::{Duration, Instant};

use lifecycle_provider::{
    OperationProfile, ProviderError, TextTurnObservation, TextTurnPort, TextTurnRequest,
};
use lifecycle_runtime::{CloseDisposition, ProcessError, ProcessSupervisor};
use lifecycle_runtime::{OwnedExecutor, finish_with_cleanup};
use std::future::{Ready, ready};
use tokio::process::Command;

#[test]
fn controlled_child() {
    let Ok(marker) = std::env::var("S2_CONTROLLED_CHILD_MARKER") else {
        return;
    };
    let mut line = String::new();
    let read = BufReader::new(std::io::stdin())
        .read_line(&mut line)
        .unwrap();
    if read > 0 && line == "GO\n" {
        std::fs::write(marker, b"entered-once").unwrap();
        if std::env::var_os("S2_CONTROLLED_CHILD_FAIL").is_some() {
            std::process::exit(7);
        }
    }
}

struct UnusedPort;
impl TextTurnPort for UnusedPort {
    type Run = Ready<Result<TextTurnObservation, ProviderError>>;
    fn profile(&self) -> OperationProfile {
        OperationProfile::ControlledText
    }
    fn execute(&self, _request: TextTurnRequest) -> Self::Run {
        ready(Err(ProviderError::UnsupportedProfile))
    }
}

fn child_command(marker: &std::path::Path) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg("--exact")
        .arg("controlled_child")
        .arg("--nocapture")
        .env("S2_CONTROLLED_CHILD_MARKER", marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

#[test]
fn missing_runtime_refuses_process_launch_without_side_effect() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("peer-ledger");
    let supervisor = ProcessSupervisor::new(1).unwrap();
    assert!(matches!(
        supervisor.launch(&mut child_command(&marker)),
        Err(ProcessError::NoRuntime)
    ));
    assert_eq!(supervisor.active_count(), 0);
    assert!(!marker.exists());
}

#[tokio::test]
async fn actual_child_is_registered_before_activation_and_reaped_exactly() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("peer-ledger");
    let supervisor = ProcessSupervisor::new(1).unwrap();
    let id = supervisor.launch(&mut child_command(&marker)).unwrap();
    assert_eq!(supervisor.active_count(), 1);
    assert!(!marker.exists());
    supervisor.activate(id).await.unwrap();
    let report = supervisor
        .close_and_drain(
            Instant::now() + Duration::from_secs(2),
            Instant::now() + Duration::from_secs(3),
        )
        .await;
    assert_eq!(report.children.len(), 1);
    assert_eq!(report.children[0].child, id);
    assert_eq!(report.children[0].disposition, CloseDisposition::Resolved);
    assert!(report.children[0].exit.as_ref().unwrap().success);
    assert_eq!(std::fs::read(&marker).unwrap(), b"entered-once");
    assert!(report.unresolved.is_empty());
    assert!(report.unsafe_at_close);
    assert_eq!(supervisor.active_count(), 0);
    assert!(matches!(
        supervisor.launch(&mut child_command(&marker)),
        Err(ProcessError::Closed)
    ));
}

#[tokio::test]
async fn unactivated_child_cannot_escape_closed_launch_gate() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("peer-ledger");
    let supervisor = ProcessSupervisor::new(1).unwrap();
    let id = supervisor.launch(&mut child_command(&marker)).unwrap();
    let report = supervisor
        .close_and_drain(Instant::now(), Instant::now() + Duration::from_secs(2))
        .await;
    assert_eq!(report.children[0].child, id);
    assert!(matches!(
        report.children[0].disposition,
        CloseDisposition::Resolved | CloseDisposition::ResolvedLate
    ));
    assert!(!marker.exists());
    assert!(report.unresolved.is_empty());
    assert!(report.unsafe_at_close);
    assert!(matches!(
        supervisor.activate(id).await,
        Err(ProcessError::Closed)
    ));
}

#[tokio::test]
async fn setup_error_and_failed_child_exit_remain_separate_from_cleanup() {
    let supervisor = ProcessSupervisor::new(1).unwrap();
    let mut missing = Command::new("/definitely/missing/s2-controlled-child");
    assert!(matches!(
        supervisor.launch(&mut missing),
        Err(ProcessError::Io(_))
    ));
    let (executor, _results) = OwnedExecutor::new(UnusedPort, 1).unwrap();
    let setup = finish_with_cleanup(
        &executor,
        &supervisor,
        Err::<(), _>("discovery failed"),
        Instant::now() + Duration::from_secs(1),
        Instant::now() + Duration::from_secs(2),
    )
    .await;
    assert_eq!(setup.primary, Err("discovery failed"));
    assert!(setup.cleanup.processes.children.is_empty());
    assert!(!setup.cleanup.unsafe_at_close);

    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("failed-ledger");
    let supervisor = ProcessSupervisor::new(1).unwrap();
    let mut command = child_command(&marker);
    command.env("S2_CONTROLLED_CHILD_FAIL", "1");
    let id = supervisor.launch(&mut command).unwrap();
    supervisor.activate(id).await.unwrap();
    let (executor, _results) = OwnedExecutor::new(UnusedPort, 1).unwrap();
    let failed = finish_with_cleanup(
        &executor,
        &supervisor,
        Err::<(), _>("receipt validation failed"),
        Instant::now() + Duration::from_secs(2),
        Instant::now() + Duration::from_secs(3),
    )
    .await;
    assert_eq!(failed.primary, Err("receipt validation failed"));
    assert_eq!(
        failed.cleanup.processes.children[0]
            .exit
            .as_ref()
            .unwrap()
            .code,
        Some(7)
    );
    assert!(
        !failed.cleanup.processes.children[0]
            .exit
            .as_ref()
            .unwrap()
            .success
    );
    assert!(failed.cleanup.unsafe_at_close);
    assert_eq!(std::fs::read(&marker).unwrap(), b"entered-once");
}
