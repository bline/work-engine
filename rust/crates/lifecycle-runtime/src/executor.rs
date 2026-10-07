use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use lifecycle_core::{AttemptId, EffectInput, EffectObservation, WallTimeMs};
use lifecycle_provider::{ProviderError, TextTurnPort, TextTurnRequest};
use lifecycle_store::AuthorizedEntry;
use thiserror::Error;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError, mpsc, oneshot};
use tokio::task::{AbortHandle, JoinHandle};

use crate::clock::{ClockError, ClockSource, SystemClock};

#[cfg(test)]
#[allow(dead_code)]
#[path = "../tests/support/mod.rs"]
mod test_support;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderFailure {
    UnsupportedProfile,
    EntryUncertain,
    ObservationUnavailable,
}

impl From<ProviderError> for ProviderFailure {
    fn from(value: ProviderError) -> Self {
        match value {
            ProviderError::UnsupportedProfile => Self::UnsupportedProfile,
            ProviderError::EntryUncertain => Self::EntryUncertain,
            ProviderError::ObservationUnavailable => Self::ObservationUnavailable,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskTermination {
    Completed,
    ProviderError(ProviderFailure),
    InvalidObservation,
    Panic,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskExit {
    pub attempt: AttemptId,
    pub local_task_ended: bool,
    pub child_exit_code: Option<i32>,
    pub termination: TaskTermination,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskResult {
    pub exit: TaskExit,
    /// Only exact, provider-qualified facts are offered to the S4 store consumer.
    pub observation: Option<EffectObservation>,
    /// Local completion clocks are facts for deadline accounting, not settlement.
    pub finished_wall: Result<WallTimeMs, ClockError>,
    pub finished_mono: Instant,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum RuntimeError {
    #[error("bounded execution or result capacity is unavailable")]
    Capacity,
    #[error("executor is closed")]
    Closed,
    #[error("entry is already owned")]
    Duplicate,
    #[error("reservation belongs to another executor")]
    WrongReservation,
    #[error("a Tokio runtime is required to own the submitted task")]
    NoRuntime,
}

#[derive(Debug)]
pub struct SubmitError {
    pub reason: RuntimeError,
    /// Still owned by the caller; S4 must account for this entered, unsent work.
    pub entry: Box<AuthorizedEntry>,
}

struct OwnedTask {
    join: Arc<tokio::sync::Mutex<JoinState>>,
    abort: AbortHandle,
}

struct JoinState {
    handle: Option<JoinHandle<()>>,
    terminal_success: Option<bool>,
}

impl JoinState {
    /// Dropping a waiter leaves the handle here for the next drain or ack.
    async fn observe_terminal(&mut self) -> bool {
        if let Some(success) = self.terminal_success {
            return success;
        }
        let success = match self.handle.as_mut() {
            Some(handle) => handle.await.is_ok(),
            None => return false,
        };
        self.handle = None;
        self.terminal_success = Some(success);
        success
    }
}

struct Registry {
    open: bool,
    unsafe_history: bool,
    tasks: BTreeMap<AttemptId, OwnedTask>,
}

#[cfg(test)]
type TestHook = Box<dyn FnOnce() + Send>;
#[cfg(test)]
type SharedTestHook = Arc<Mutex<Option<TestHook>>>;
#[cfg(test)]
type TestActivationHook = Box<dyn FnOnce(&mut oneshot::Receiver<()>) + Send>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskDrainReport {
    pub joined: Vec<AttemptId>,
    pub unresolved: Vec<AttemptId>,
    pub pending_results: Vec<AttemptId>,
    pub unsafe_at_close: bool,
}

struct Inner<P> {
    provider: Arc<P>,
    clock: Arc<dyn ClockSource>,
    capacity: Arc<Semaphore>,
    run_slots: Arc<Semaphore>,
    results: Arc<ResultState>,
    notifications: mpsc::Sender<AttemptId>,
    registry: Mutex<Registry>,
    drain_lock: tokio::sync::Mutex<()>,
    #[cfg(test)]
    pre_registration_hook: Mutex<Option<TestHook>>,
    #[cfg(test)]
    pre_activation_hook: Arc<Mutex<Option<TestActivationHook>>>,
    #[cfg(test)]
    post_result_hook: SharedTestHook,
}

struct ResultRecord {
    value: TaskResult,
    _capacity: OwnedSemaphorePermit,
}

struct ResultState {
    capacity: Arc<Semaphore>,
    pending: Mutex<BTreeMap<AttemptId, ResultRecord>>,
}

pub struct OwnedExecutor<P> {
    inner: Arc<Inner<P>>,
}

impl<P> Clone for OwnedExecutor<P> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct ExecutionReservation<P> {
    inner: Arc<Inner<P>>,
    execution_slot: OwnedSemaphorePermit,
    result_slot: OwnedSemaphorePermit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionTicket {
    pub attempt: AttemptId,
}

pub struct ResultReceiver {
    notifications: mpsc::Receiver<AttemptId>,
    results: Arc<ResultState>,
}

impl ResultReceiver {
    pub async fn recv(&mut self) -> Option<TaskResult> {
        loop {
            if let Some(result) = self.results.pending.lock().unwrap().values().next() {
                return Some(result.value.clone());
            }
            self.notifications.recv().await?;
        }
    }

    /// A missed or dropped notification cannot erase an owned result.
    pub fn pending(&self) -> Vec<TaskResult> {
        self.results
            .pending
            .lock()
            .unwrap()
            .values()
            .map(|record| record.value.clone())
            .collect()
    }
}

/// The service reserves before claiming entry, then transfers the nonclone permit.
/// A successful `submit` owns execution before returning its observer ticket.
pub trait LifecycleExecutor {
    type Reservation;
    fn try_reserve(&self) -> Result<Self::Reservation, RuntimeError>;
    fn submit(
        &self,
        reservation: Self::Reservation,
        entry: AuthorizedEntry,
    ) -> Result<ExecutionTicket, SubmitError>;
}

impl<P: TextTurnPort + Send + Sync + 'static> OwnedExecutor<P> {
    pub fn new(provider: P, capacity: usize) -> Result<(Self, ResultReceiver), RuntimeError> {
        Self::with_clock_and_limits(provider, capacity, capacity, Arc::new(SystemClock))
    }

    /// At most `capacity` committed entries can wait or run; `concurrency`
    /// bounds provider calls. Every accepted entry also holds one result slot.
    pub fn with_clock_and_limits(
        provider: P,
        capacity: usize,
        concurrency: usize,
        clock: Arc<dyn ClockSource>,
    ) -> Result<(Self, ResultReceiver), RuntimeError> {
        if capacity == 0 || concurrency == 0 || concurrency > capacity {
            return Err(RuntimeError::Capacity);
        }
        let (notifications, receiver) = mpsc::channel(capacity.min(4));
        let results = Arc::new(ResultState {
            capacity: Arc::new(Semaphore::new(capacity)),
            pending: Mutex::new(BTreeMap::new()),
        });
        Ok((
            Self {
                inner: Arc::new(Inner {
                    provider: Arc::new(provider),
                    clock,
                    capacity: Arc::new(Semaphore::new(capacity)),
                    run_slots: Arc::new(Semaphore::new(concurrency)),
                    results: results.clone(),
                    notifications,
                    registry: Mutex::new(Registry {
                        open: true,
                        unsafe_history: false,
                        tasks: BTreeMap::new(),
                    }),
                    drain_lock: tokio::sync::Mutex::new(()),
                    #[cfg(test)]
                    pre_registration_hook: Mutex::new(None),
                    #[cfg(test)]
                    pre_activation_hook: Arc::new(Mutex::new(None)),
                    #[cfg(test)]
                    post_result_hook: Arc::new(Mutex::new(None)),
                }),
            },
            ResultReceiver {
                notifications: receiver,
                results,
            },
        ))
    }

    pub fn owned_count(&self) -> usize {
        self.inner.registry.lock().unwrap().tasks.len()
    }

    pub fn pending_results(&self) -> Vec<TaskResult> {
        self.inner
            .results
            .pending
            .lock()
            .unwrap()
            .values()
            .map(|record| record.value.clone())
            .collect()
    }

    /// Call only after S4 has committed the result or its unresolved disposition.
    pub async fn acknowledge_result(&self, attempt: &AttemptId) -> bool {
        let _drain = self.inner.drain_lock.lock().await;
        if !self
            .inner
            .results
            .pending
            .lock()
            .unwrap()
            .contains_key(attempt)
        {
            return false;
        }
        let join = self
            .inner
            .registry
            .lock()
            .unwrap()
            .tasks
            .get(attempt)
            .map(|task| task.join.clone());
        if let Some(join) = join {
            let mut state = join.lock().await;
            let _ = state.observe_terminal().await;
        }
        self.inner.registry.lock().unwrap().tasks.remove(attempt);
        self.inner
            .results
            .pending
            .lock()
            .unwrap()
            .remove(attempt)
            .is_some()
    }

    /// Requests cancellation of one local task; remote settlement stays unknown.
    pub fn request_abort(&self, attempt: &AttemptId) -> bool {
        let registry = self.inner.registry.lock().unwrap();
        if let Some(task) = registry.tasks.get(attempt) {
            task.abort.abort();
            true
        } else {
            false
        }
    }

    /// Close admission synchronously before any asynchronous drain begins.
    pub fn close_gate(&self) {
        let mut registry = self.inner.registry.lock().unwrap();
        registry.open = false;
        if !registry.tasks.is_empty() {
            registry.unsafe_history = true;
        }
    }

    pub async fn close_and_drain(&self, deadline: Instant) -> TaskDrainReport {
        self.close_gate();
        let _drain = self.inner.drain_lock.lock().await;
        let attempts = {
            let registry = self.inner.registry.lock().unwrap();
            registry.tasks.keys().cloned().collect::<Vec<_>>()
        };
        let mut joined = Vec::new();
        let mut unresolved = Vec::new();
        for attempt in attempts {
            let join = self
                .inner
                .registry
                .lock()
                .unwrap()
                .tasks
                .get(&attempt)
                .map(|task| task.join.clone());
            if let Some(join) = join {
                let observed =
                    tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), async {
                        let mut state = join.lock().await;
                        state.observe_terminal().await
                    })
                    .await;
                match observed {
                    Ok(true) => joined.push(attempt),
                    Ok(false) | Err(_) => unresolved.push(attempt),
                }
            }
        }
        let pending_results = self
            .inner
            .results
            .pending
            .lock()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        let unsafe_at_close = self.inner.registry.lock().unwrap().unsafe_history;
        TaskDrainReport {
            joined,
            unresolved,
            pending_results,
            unsafe_at_close,
        }
    }
}

impl<P: TextTurnPort + Send + Sync + 'static> LifecycleExecutor for OwnedExecutor<P> {
    type Reservation = ExecutionReservation<P>;

    fn try_reserve(&self) -> Result<Self::Reservation, RuntimeError> {
        if !self.inner.registry.lock().unwrap().open {
            return Err(RuntimeError::Closed);
        }
        let execution_slot = self
            .inner
            .capacity
            .clone()
            .try_acquire_owned()
            .map_err(|error| match error {
                TryAcquireError::Closed => RuntimeError::Closed,
                _ => RuntimeError::Capacity,
            })?;
        let result_slot = self
            .inner
            .results
            .capacity
            .clone()
            .try_acquire_owned()
            .map_err(|error| match error {
                TryAcquireError::Closed => RuntimeError::Closed,
                _ => RuntimeError::Capacity,
            })?;
        Ok(ExecutionReservation {
            inner: self.inner.clone(),
            execution_slot,
            result_slot,
        })
    }

    fn submit(
        &self,
        reservation: Self::Reservation,
        entry: AuthorizedEntry,
    ) -> Result<ExecutionTicket, SubmitError> {
        if tokio::runtime::Handle::try_current().is_err() {
            return Err(SubmitError {
                reason: RuntimeError::NoRuntime,
                entry: Box::new(entry),
            });
        }
        if !Arc::ptr_eq(&self.inner, &reservation.inner) {
            return Err(SubmitError {
                reason: RuntimeError::WrongReservation,
                entry: Box::new(entry),
            });
        }
        let mut registry = self.inner.registry.lock().unwrap();
        if !registry.open {
            return Err(SubmitError {
                reason: RuntimeError::Closed,
                entry: Box::new(entry),
            });
        }
        let attempt = entry.attempt().clone();
        if registry.tasks.contains_key(&attempt) {
            return Err(SubmitError {
                reason: RuntimeError::Duplicate,
                entry: Box::new(entry),
            });
        }
        let request = match entry.input() {
            EffectInput::ControlledText(input) => TextTurnRequest {
                subject: entry.subject().clone(),
                effect: entry.effect().clone(),
                attempt: attempt.clone(),
                incarnation: entry.incarnation().clone(),
                input: input.clone(),
            },
        };
        let ExecutionReservation {
            execution_slot,
            result_slot,
            ..
        } = reservation;
        let provider = self.inner.provider.clone();
        let clock = self.inner.clock.clone();
        let run_slots = self.inner.run_slots.clone();
        let result_attempt = attempt.clone();
        let results = self.inner.results.clone();
        let notifications = self.inner.notifications.clone();
        let (activate_tx, activate_rx) = oneshot::channel();
        #[cfg(test)]
        let pre_activation_hook = self.inner.pre_activation_hook.clone();
        // The registry lock remains held until both handles are installed.
        let inner = tokio::spawn(async move {
            let _execution_slot = execution_slot;
            #[cfg(test)]
            let activate_rx = {
                let mut activate_rx = activate_rx;
                if let Some(hook) = pre_activation_hook.lock().unwrap().take() {
                    hook(&mut activate_rx);
                }
                activate_rx
            };
            if activate_rx.await.is_err() {
                return (TaskTermination::Cancelled, None);
            }
            let _run_slot = match run_slots.acquire_owned().await {
                Ok(slot) => slot,
                Err(_) => return (TaskTermination::Cancelled, None),
            };
            match provider.execute(request.clone()).await {
                Ok(fact)
                    if fact.effect == request.effect
                        && fact.attempt == request.attempt
                        && fact.incarnation == request.incarnation =>
                {
                    (
                        TaskTermination::Completed,
                        Some(EffectObservation {
                            source: fact.source,
                            effect: fact.effect,
                            attempt: fact.attempt,
                            incarnation: fact.incarnation,
                            provider_thread: fact.thread,
                            provider_turn: fact.turn,
                            outcome: fact.outcome,
                            settlement: fact.settlement,
                        }),
                    )
                }
                Ok(_) => (TaskTermination::InvalidObservation, None),
                Err(error) => (TaskTermination::ProviderError(error.into()), None),
            }
        });
        let abort = inner.abort_handle();
        #[cfg(test)]
        let post_result_hook = self.inner.post_result_hook.clone();
        let outer = tokio::spawn(async move {
            let (termination, observation) = match inner.await {
                Ok(value) => value,
                Err(error) if error.is_cancelled() => (TaskTermination::Cancelled, None),
                Err(_) => (TaskTermination::Panic, None),
            };
            let result = TaskResult {
                exit: TaskExit {
                    attempt: result_attempt.clone(),
                    local_task_ended: true,
                    child_exit_code: None,
                    termination,
                },
                observation,
                finished_wall: clock.wall_time(),
                finished_mono: clock.monotonic_now(),
            };
            results.pending.lock().unwrap().insert(
                result_attempt.clone(),
                ResultRecord {
                    value: result,
                    _capacity: result_slot,
                },
            );
            let _ = notifications.try_send(result_attempt);
            #[cfg(test)]
            if let Some(hook) = post_result_hook.lock().unwrap().take() {
                hook();
            }
        });
        #[cfg(test)]
        if let Some(hook) = self.inner.pre_registration_hook.lock().unwrap().take() {
            hook();
        }
        registry.tasks.insert(
            attempt.clone(),
            OwnedTask {
                join: Arc::new(tokio::sync::Mutex::new(JoinState {
                    handle: Some(outer),
                    terminal_success: None,
                })),
                abort,
            },
        );
        let _ = activate_tx.send(());
        Ok(ExecutionTicket { attempt })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc as std_mpsc;
    use std::time::Duration;

    use lifecycle_provider::{OperationProfile, TextTurnObservation};

    struct ControlledErrorPort {
        entries: Arc<AtomicUsize>,
        entered: Option<Arc<Semaphore>>,
        release: Option<Arc<Semaphore>>,
    }

    impl TextTurnPort for ControlledErrorPort {
        type Run = Pin<Box<dyn Future<Output = Result<TextTurnObservation, ProviderError>> + Send>>;

        fn profile(&self) -> OperationProfile {
            OperationProfile::ControlledText
        }

        fn execute(&self, _request: TextTurnRequest) -> Self::Run {
            self.entries.fetch_add(1, Ordering::SeqCst);
            if let Some(entered) = &self.entered {
                entered.add_permits(1);
            }
            let release = self.release.clone();
            Box::pin(async move {
                if let Some(release) = release {
                    let permit = release.acquire().await.unwrap();
                    permit.forget();
                }
                Err(ProviderError::ObservationUnavailable)
            })
        }
    }

    fn immediate_port(entries: Arc<AtomicUsize>) -> ControlledErrorPort {
        ControlledErrorPort {
            entries,
            entered: None,
            release: None,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn provider_cannot_enter_during_pre_registration_pause() {
        let (_root, _store, entry) = test_support::claimed_entry();
        let entries = Arc::new(AtomicUsize::new(0));
        let (executor, mut results) =
            OwnedExecutor::new(immediate_port(entries.clone()), 1).unwrap();
        let (paused_tx, paused_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std_mpsc::channel();
        *executor.inner.pre_registration_hook.lock().unwrap() = Some(Box::new(move || {
            let _ = paused_tx.send(());
            let _ = release_rx.recv();
        }));
        let (activation_tx, activation_rx) = tokio::sync::oneshot::channel();
        *executor.inner.pre_activation_hook.lock().unwrap() = Some(Box::new(move |activate_rx| {
            let empty = matches!(
                activate_rx.try_recv(),
                Err(oneshot::error::TryRecvError::Empty)
            );
            let _ = activation_tx.send(empty);
        }));

        let submitting = executor.clone();
        let submit = tokio::spawn(async move {
            submitting
                .submit(submitting.try_reserve().unwrap(), entry)
                .unwrap()
        });
        paused_rx.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_secs(1), activation_rx)
                .await
                .unwrap()
                .unwrap(),
            "worker reached the activation boundary before registration and the token is unsent"
        );
        assert_eq!(entries.load(Ordering::SeqCst), 0);
        release_tx.send(()).unwrap();
        let ticket = submit.await.unwrap();
        let result = tokio::time::timeout(Duration::from_secs(1), results.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.exit.attempt, ticket.attempt);
        assert_eq!(entries.load(Ordering::SeqCst), 1);
        assert_eq!(executor.owned_count(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cancelled_drain_waiter_retains_exact_join_handle() {
        let (_root, _store, entry) = test_support::claimed_entry();
        let entered = Arc::new(Semaphore::new(0));
        let release = Arc::new(Semaphore::new(0));
        let (executor, mut results) = OwnedExecutor::new(
            ControlledErrorPort {
                entries: Arc::new(AtomicUsize::new(0)),
                entered: Some(entered.clone()),
                release: Some(release.clone()),
            },
            1,
        )
        .unwrap();
        let (paused_tx, paused_rx) = tokio::sync::oneshot::channel();
        let (release_collector_tx, release_collector_rx) = std_mpsc::channel();
        *executor.inner.post_result_hook.lock().unwrap() = Some(Box::new(move || {
            let _ = paused_tx.send(());
            let _ = release_collector_rx.recv();
        }));
        let ticket = executor
            .submit(executor.try_reserve().unwrap(), entry)
            .unwrap();
        entered.acquire().await.unwrap().forget();

        let join = executor.inner.registry.lock().unwrap().tasks[&ticket.attempt]
            .join
            .clone();
        let closing = executor.clone();
        let draining = tokio::spawn(async move {
            closing
                .close_and_drain(Instant::now() + Duration::from_secs(5))
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if join.try_lock().is_err() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        draining.abort();
        assert!(draining.await.unwrap_err().is_cancelled());
        release.add_permits(1);
        paused_rx.await.unwrap();
        let result = tokio::time::timeout(Duration::from_secs(1), results.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.exit.attempt, ticket.attempt);
        let still_running = executor
            .close_and_drain(Instant::now() + Duration::from_millis(20))
            .await;
        assert!(still_running.joined.is_empty());
        assert_eq!(still_running.unresolved, vec![ticket.attempt.clone()]);
        assert_eq!(still_running.pending_results, vec![ticket.attempt.clone()]);
        release_collector_tx.send(()).unwrap();
        let repeated = executor
            .close_and_drain(Instant::now() + Duration::from_secs(1))
            .await;
        assert_eq!(repeated.joined, vec![ticket.attempt.clone()]);
        assert!(repeated.unresolved.is_empty());
        assert_eq!(repeated.pending_results, vec![ticket.attempt]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cancelled_ack_waiter_keeps_result_and_join_owned() {
        let (_root, _store, entry) = test_support::claimed_entry();
        let (executor, _results) =
            OwnedExecutor::new(immediate_port(Arc::new(AtomicUsize::new(0))), 1).unwrap();
        let (paused_tx, paused_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std_mpsc::channel();
        *executor.inner.post_result_hook.lock().unwrap() = Some(Box::new(move || {
            let _ = paused_tx.send(());
            let _ = release_rx.recv();
        }));
        let ticket = executor
            .submit(executor.try_reserve().unwrap(), entry)
            .unwrap();
        paused_rx.await.unwrap();
        assert_eq!(executor.pending_results().len(), 1);

        let mut acknowledging = Box::pin(executor.acknowledge_result(&ticket.attempt));
        tokio::select! {
            biased;
            _ = &mut acknowledging => panic!("collector still paused"),
            _ = tokio::task::yield_now() => {},
        }
        drop(acknowledging);
        assert_eq!(executor.owned_count(), 1);
        assert_eq!(executor.pending_results().len(), 1);
        release_tx.send(()).unwrap();
        assert!(
            tokio::time::timeout(
                Duration::from_secs(1),
                executor.acknowledge_result(&ticket.attempt)
            )
            .await
            .unwrap()
        );
        assert_eq!(executor.owned_count(), 0);
        assert!(executor.pending_results().is_empty());
    }
}
