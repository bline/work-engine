mod support;

use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

use lifecycle_core::{
    EffectSettlement, EvidenceId, ExecutionOutcome, ProviderThreadId, ProviderTurnId,
};
use lifecycle_provider::{
    OperationProfile, ProviderError, TextTurnObservation, TextTurnPort, TextTurnRequest,
};
use lifecycle_runtime::{ClockError, ClockSource};
use lifecycle_runtime::{LifecycleExecutor, OwnedExecutor, RuntimeError, TaskTermination};
use tokio::sync::Semaphore;

#[derive(Clone)]
struct ControlledPort {
    entries: Arc<AtomicUsize>,
    fail: bool,
}

impl TextTurnPort for ControlledPort {
    type Run = Ready<Result<TextTurnObservation, ProviderError>>;

    fn profile(&self) -> OperationProfile {
        OperationProfile::ControlledText
    }

    fn execute(&self, request: TextTurnRequest) -> Self::Run {
        self.entries.fetch_add(1, Ordering::SeqCst);
        let result = if self.fail {
            Err(ProviderError::ObservationUnavailable)
        } else {
            let evidence = EvidenceId::parse("controlled-source-1").unwrap();
            Ok(TextTurnObservation {
                source: evidence.clone(),
                effect: request.effect,
                attempt: request.attempt,
                incarnation: request.incarnation,
                thread: ProviderThreadId::parse("thread-1").unwrap(),
                turn: ProviderTurnId::parse("turn-1").unwrap(),
                final_text: Some("reply".into()),
                outcome: ExecutionOutcome::Completed,
                settlement: EffectSettlement::Established(evidence),
            })
        };
        ready(result)
    }
}

fn completed(request: TextTurnRequest) -> TextTurnObservation {
    let evidence = EvidenceId::parse(format!("source-{}", request.attempt.as_str())).unwrap();
    TextTurnObservation {
        source: evidence.clone(),
        effect: request.effect,
        attempt: request.attempt,
        incarnation: request.incarnation,
        thread: ProviderThreadId::parse("thread-1").unwrap(),
        turn: ProviderTurnId::parse("turn-1").unwrap(),
        final_text: Some("reply".into()),
        outcome: ExecutionOutcome::Completed,
        settlement: EffectSettlement::Established(evidence),
    }
}

#[derive(Clone)]
struct PausedPort {
    entered: Arc<Semaphore>,
    release: Arc<Semaphore>,
    entries: Arc<AtomicUsize>,
}

impl TextTurnPort for PausedPort {
    type Run = Pin<Box<dyn Future<Output = Result<TextTurnObservation, ProviderError>> + Send>>;

    fn profile(&self) -> OperationProfile {
        OperationProfile::ControlledText
    }

    fn execute(&self, request: TextTurnRequest) -> Self::Run {
        self.entries.fetch_add(1, Ordering::SeqCst);
        self.entered.add_permits(1);
        let release = self.release.clone();
        Box::pin(async move {
            let permit = release.acquire().await.unwrap();
            permit.forget();
            Ok(completed(request))
        })
    }
}

struct PanicPort;

impl TextTurnPort for PanicPort {
    type Run = Ready<Result<TextTurnObservation, ProviderError>>;
    fn profile(&self) -> OperationProfile {
        OperationProfile::ControlledText
    }
    fn execute(&self, _request: TextTurnRequest) -> Self::Run {
        panic!("controlled port panic")
    }
}

struct MismatchedPort;

impl TextTurnPort for MismatchedPort {
    type Run = Ready<Result<TextTurnObservation, ProviderError>>;
    fn profile(&self) -> OperationProfile {
        OperationProfile::ControlledText
    }
    fn execute(&self, request: TextTurnRequest) -> Self::Run {
        let mut fact = completed(request);
        fact.attempt = lifecycle_core::AttemptId::parse("forged-attempt").unwrap();
        ready(Ok(fact))
    }
}

struct FixedClock {
    wall: lifecycle_core::WallTimeMs,
    now: Instant,
}

impl ClockSource for FixedClock {
    fn wall_time(&self) -> Result<lifecycle_core::WallTimeMs, ClockError> {
        Ok(self.wall)
    }
    fn monotonic_now(&self) -> Instant {
        self.now
    }
}

#[test]
fn missing_runtime_returns_committed_entry_without_dispatch() {
    let (_root, store, entry) = support::claimed_entry();
    let entries = Arc::new(AtomicUsize::new(0));
    let (executor, _results) = OwnedExecutor::new(
        ControlledPort {
            entries: entries.clone(),
            fail: false,
        },
        1,
    )
    .unwrap();
    let unsent = executor
        .submit(executor.try_reserve().unwrap(), entry)
        .unwrap_err();
    assert_eq!(unsent.reason, RuntimeError::NoRuntime);
    assert_eq!(store.counts().unwrap(), (1, 1, 1));
    assert_eq!(entries.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn store_claimed_entry_survives_dropped_ticket_and_keeps_exact_result() {
    let (_root, store, entry) = support::claimed_entry();
    assert_eq!(store.counts().unwrap(), (1, 1, 1));
    let entries = Arc::new(AtomicUsize::new(0));
    let (executor, mut results) = OwnedExecutor::new(
        ControlledPort {
            entries: entries.clone(),
            fail: false,
        },
        1,
    )
    .unwrap();
    let reservation = executor.try_reserve().unwrap();
    let ticket = executor.submit(reservation, entry).unwrap();
    drop(ticket);
    let result = results.recv().await.unwrap();
    assert_eq!(result.exit.termination, TaskTermination::Completed);
    assert_eq!(
        result.exit.attempt,
        result.observation.as_ref().unwrap().attempt
    );
    assert_eq!(entries.load(Ordering::SeqCst), 1);
    assert_eq!(executor.try_reserve().err(), Some(RuntimeError::Capacity));
}

#[tokio::test]
async fn port_error_is_owned_data_without_invented_settlement() {
    let (_root, _store, entry) = support::claimed_entry();
    let (executor, mut results) = OwnedExecutor::new(
        ControlledPort {
            entries: Arc::new(AtomicUsize::new(0)),
            fail: true,
        },
        1,
    )
    .unwrap();
    let reservation = executor.try_reserve().unwrap();
    drop(executor.submit(reservation, entry).unwrap());
    let result = results.recv().await.unwrap();
    assert!(matches!(
        result.exit.termination,
        TaskTermination::ProviderError(_)
    ));
    assert!(result.observation.is_none());
}

#[tokio::test]
async fn dropped_result_listener_does_not_drop_accepted_completion() {
    let (_root, _store, entry) = support::claimed_entry();
    let (executor, results) = OwnedExecutor::new(
        ControlledPort {
            entries: Arc::new(AtomicUsize::new(0)),
            fail: false,
        },
        1,
    )
    .unwrap();
    drop(results);
    let ticket = executor
        .submit(executor.try_reserve().unwrap(), entry)
        .unwrap();
    let drained = executor
        .close_and_drain(Instant::now() + Duration::from_secs(1))
        .await;
    assert_eq!(drained.pending_results, vec![ticket.attempt.clone()]);
    assert_eq!(executor.pending_results()[0].exit.attempt, ticket.attempt);
    assert!(executor.acknowledge_result(&ticket.attempt).await);
    assert!(executor.pending_results().is_empty());
}

#[tokio::test]
async fn completion_uses_injected_clock_facts_for_later_deadline_accounting() {
    let (_root, _store, entry) = support::claimed_entry();
    let now = Instant::now();
    let clock = Arc::new(FixedClock {
        wall: lifecycle_core::WallTimeMs::new(777),
        now,
    });
    let (executor, mut results) = OwnedExecutor::with_clock_and_limits(
        ControlledPort {
            entries: Arc::new(AtomicUsize::new(0)),
            fail: false,
        },
        1,
        1,
        clock,
    )
    .unwrap();
    executor
        .submit(executor.try_reserve().unwrap(), entry)
        .unwrap();
    let result = results.recv().await.unwrap();
    assert_eq!(result.finished_wall.unwrap().get(), 777);
    assert_eq!(result.finished_mono, now);
}

#[tokio::test]
async fn reservation_precedes_claim_and_closed_gate_returns_exact_unsent_entry() {
    let entries = Arc::new(AtomicUsize::new(0));
    let (executor, results) = OwnedExecutor::new(
        ControlledPort {
            entries: entries.clone(),
            fail: false,
        },
        1,
    )
    .unwrap();
    let reservation = executor.try_reserve().unwrap();
    assert_eq!(executor.try_reserve().err(), Some(RuntimeError::Capacity));
    let (_root, mut store, entry) = support::claimed_entry();
    let effect = entry.effect().clone();
    let report = executor.close_and_drain(Instant::now()).await;
    assert!(!report.unsafe_at_close);
    let unsent = executor.submit(reservation, entry).unwrap_err();
    assert_eq!(unsent.reason, RuntimeError::Closed);
    assert_eq!(unsent.entry.effect(), &effect);
    assert_eq!(store.counts().unwrap(), (1, 1, 1));
    assert!(
        store
            .claim_prepared_entry(
                &effect,
                lifecycle_core::RuntimeIncarnation::parse("second").unwrap(),
                support::clock(103)
            )
            .is_err()
    );
    assert_eq!(entries.load(Ordering::SeqCst), 0);
    assert!(results.pending().is_empty());
}

#[tokio::test]
async fn bounded_waiting_and_reserved_results_cannot_deadlock_on_saturation() {
    let (_root, _store, first, second) = support::claimed_two_entries();
    let entered = Arc::new(Semaphore::new(0));
    let release = Arc::new(Semaphore::new(0));
    let entries = Arc::new(AtomicUsize::new(0));
    let port = PausedPort {
        entered: entered.clone(),
        release: release.clone(),
        entries: entries.clone(),
    };
    let (executor, mut results) =
        OwnedExecutor::with_clock_and_limits(port, 2, 1, Arc::new(lifecycle_runtime::SystemClock))
            .unwrap();
    let first_ticket = executor
        .submit(executor.try_reserve().unwrap(), first)
        .unwrap();
    let started = entered.acquire().await.unwrap();
    started.forget();
    let second_ticket = executor
        .submit(executor.try_reserve().unwrap(), second)
        .unwrap();
    assert_eq!(entries.load(Ordering::SeqCst), 1);
    assert_eq!(executor.try_reserve().err(), Some(RuntimeError::Capacity));
    release.add_permits(1);
    let first_result = results.recv().await.unwrap();
    assert_eq!(first_result.exit.attempt, first_ticket.attempt);
    assert!(executor.acknowledge_result(&first_ticket.attempt).await);
    let started = entered.acquire().await.unwrap();
    started.forget();
    assert_eq!(entries.load(Ordering::SeqCst), 2);
    release.add_permits(1);
    let second_result = results.recv().await.unwrap();
    assert_eq!(second_result.exit.attempt, second_ticket.attempt);
    assert_eq!(results.pending().len(), 1);
    assert!(executor.try_reserve().is_ok());
}

#[tokio::test]
async fn panic_abort_and_wrong_identity_are_owned_without_settlement() {
    let (_root, _store, entry) = support::claimed_entry();
    let (executor, mut results) = OwnedExecutor::new(PanicPort, 1).unwrap();
    let ticket = executor
        .submit(executor.try_reserve().unwrap(), entry)
        .unwrap();
    let panic_result = results.recv().await.unwrap();
    assert_eq!(panic_result.exit.attempt, ticket.attempt);
    assert_eq!(panic_result.exit.termination, TaskTermination::Panic);
    assert!(panic_result.observation.is_none());

    let (_root, _store, entry) = support::claimed_entry();
    let (executor, mut results) = OwnedExecutor::new(MismatchedPort, 1).unwrap();
    executor
        .submit(executor.try_reserve().unwrap(), entry)
        .unwrap();
    let mismatch = results.recv().await.unwrap();
    assert_eq!(
        mismatch.exit.termination,
        TaskTermination::InvalidObservation
    );
    assert!(mismatch.observation.is_none());

    let (_root, _store, entry) = support::claimed_entry();
    let entered = Arc::new(Semaphore::new(0));
    let release = Arc::new(Semaphore::new(0));
    let port = PausedPort {
        entered: entered.clone(),
        release,
        entries: Arc::new(AtomicUsize::new(0)),
    };
    let (executor, mut results) = OwnedExecutor::new(port, 1).unwrap();
    let ticket = executor
        .submit(executor.try_reserve().unwrap(), entry)
        .unwrap();
    let started = entered.acquire().await.unwrap();
    started.forget();
    assert!(executor.request_abort(&ticket.attempt));
    let cancelled = results.recv().await.unwrap();
    assert_eq!(cancelled.exit.termination, TaskTermination::Cancelled);
    assert!(cancelled.observation.is_none());
}

#[tokio::test]
async fn late_completion_keeps_unsafe_close_history_and_result_owner() {
    let (_root, _store, entry) = support::claimed_entry();
    let entered = Arc::new(Semaphore::new(0));
    let release = Arc::new(Semaphore::new(0));
    let port = PausedPort {
        entered: entered.clone(),
        release: release.clone(),
        entries: Arc::new(AtomicUsize::new(0)),
    };
    let (executor, mut results) = OwnedExecutor::new(port, 1).unwrap();
    let ticket = executor
        .submit(executor.try_reserve().unwrap(), entry)
        .unwrap();
    let started = entered.acquire().await.unwrap();
    started.forget();
    let early = executor.close_and_drain(Instant::now()).await;
    assert_eq!(early.unresolved, vec![ticket.attempt.clone()]);
    assert!(early.unsafe_at_close);
    release.add_permits(1);
    let completed = results.recv().await.unwrap();
    assert_eq!(completed.exit.attempt, ticket.attempt);
    let later = executor
        .close_and_drain(Instant::now() + Duration::from_secs(1))
        .await;
    assert!(later.unresolved.is_empty());
    assert!(later.unsafe_at_close);
    assert_eq!(later.pending_results, vec![ticket.attempt]);
}
