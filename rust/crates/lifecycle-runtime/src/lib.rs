//! Owned execution of store-committed lifecycle entries.
//!
//! `AuthorizedEntry` is minted only by the store. Reserving capacity is not
//! admission; the service must claim entry before transferring it here.

mod clock;
mod executor;
mod framing;
mod native_stdio;
mod process;

use std::time::Instant;

use lifecycle_provider::TextTurnPort;

pub use clock::{ClockError, ClockSource, DeadlineStatus, MonotonicDeadline, SystemClock};
pub use executor::{
    ExecutionReservation, ExecutionTicket, LifecycleExecutor, OwnedExecutor, ProviderFailure,
    ResultReceiver, RuntimeError, SubmitError, TaskDrainReport, TaskExit, TaskResult,
    TaskTermination,
};
pub use framing::{
    BoundedFrameReader, BoundedFrameWriter, FrameDiagnosticCode, FrameDiagnostics, FrameError,
};
pub use native_stdio::{NativeChildExit, NativeStdio, NativeStdioError};
pub use process::{
    ActivationState, ChildClose, ChildExit, ChildId, CloseDisposition, ProcessError,
    ProcessShutdownReport, ProcessSupervisor,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShutdownReport {
    pub tasks: TaskDrainReport,
    pub processes: ProcessShutdownReport,
    pub unsafe_at_close: bool,
}

pub struct CompletionWithCleanup<T, E> {
    pub primary: Result<T, E>,
    pub cleanup: ShutdownReport,
}

/// Both launch gates close before either owned set begins awaiting cleanup.
pub async fn finish_with_cleanup<P, T, E>(
    executor: &OwnedExecutor<P>,
    processes: &ProcessSupervisor,
    primary: Result<T, E>,
    proof_deadline: Instant,
    cleanup_deadline: Instant,
) -> CompletionWithCleanup<T, E>
where
    P: TextTurnPort + Send + Sync + 'static,
{
    executor.close_gate();
    processes.close_gate();
    let tasks = executor.close_and_drain(proof_deadline).await;
    let process_report = processes
        .close_and_drain(proof_deadline, cleanup_deadline)
        .await;
    let unsafe_at_close = tasks.unsafe_at_close || process_report.unsafe_at_close;
    CompletionWithCleanup {
        primary,
        cleanup: ShutdownReport {
            tasks,
            processes: process_report,
            unsafe_at_close,
        },
    }
}
