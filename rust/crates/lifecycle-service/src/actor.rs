use std::{
    fs,
    io::Write,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use lifecycle_core::{
    AuthorityExpiresAt, BuildId, ClockSample, Command, CommandOutcome, ContextGeneration, GrantId,
    GrantScope, ProofRunId, Revision, RuntimeIncarnation, SubjectId, TrustedGrant, WaitBudgetMs,
    WallTimeMs,
};
use lifecycle_runtime::{
    ExecutionReservation, LifecycleExecutor, OwnedExecutor, ProviderFailure, ResultReceiver,
    TaskResult, TaskTermination,
};
use lifecycle_store::{RecoveryDisposition, SqliteLifecycleStore, StoreError, SubjectProjection};
use lifecycle_wire::{CommandResultV2, WireErrorCode};
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use crate::{
    ServiceConfig, ServiceError,
    admission::{AdmitError, parse_authenticated},
    controlled::ControlledPort,
    engine,
};

pub enum Request {
    Command {
        uid: u32,
        bytes: Vec<u8>,
        reply: oneshot::Sender<Value>,
    },
    Snapshot {
        subject: String,
        delivery: Option<String>,
        transition: Option<String>,
        reply: oneshot::Sender<Result<SubjectProjection, StoreError>>,
    },
    Shutdown {
        reply: oneshot::Sender<Result<(), StoreError>>,
    },
}

pub fn clock_sample() -> ClockSample {
    let wall = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    ClockSample {
        wall: WallTimeMs::new(i64::try_from(wall.as_millis()).unwrap_or(i64::MAX)),
        wait_budget: WaitBudgetMs::new(1_000),
    }
}

pub struct Actor {
    config: ServiceConfig,
    store: SqliteLifecycleStore,
    executor: OwnedExecutor<ControlledPort>,
    results: ResultReceiver,
    incarnation: RuntimeIncarnation,
    entered_uncertain_on_open: bool,
    verifier_gate: Arc<engine::VerifierGate>,
}

impl Actor {
    pub fn new(config: ServiceConfig) -> Result<Self, ServiceError> {
        let mut store =
            SqliteLifecycleStore::open(&config.store_root, config.trusted_issuer.clone())?;
        let recovery = store.load_recovery()?;
        let entered_uncertain_on_open = store
            .load_recovery_entries()?
            .iter()
            .any(|entry| entry.disposition == RecoveryDisposition::EnteredUncertain);
        if recovery.subjects == 0 {
            store.register_subject(
                SubjectId::parse(config.subject_id.clone())
                    .map_err(|_| ServiceError::Configuration)?,
                ContextGeneration::parse(config.context_generation.clone())
                    .map_err(|_| ServiceError::Configuration)?,
                BuildId::parse(config.build_id.clone()).map_err(|_| ServiceError::Configuration)?,
                ProofRunId::parse(config.proof_run_id.clone())
                    .map_err(|_| ServiceError::Configuration)?,
                clock_sample(),
            )?;
            for grant in &config.grants {
                store.install_trusted_grant(TrustedGrant {
                    id: GrantId::parse(grant.grant_ref.clone())
                        .map_err(|_| ServiceError::Configuration)?,
                    issuer: config.trusted_issuer.clone(),
                    principal: config.principal.principal_ref.clone(),
                    subject: SubjectId::parse(config.subject_id.clone())
                        .map_err(|_| ServiceError::Configuration)?,
                    context: ContextGeneration::parse(grant.context_generation.clone())
                        .map_err(|_| ServiceError::Configuration)?,
                    build: BuildId::parse(config.build_id.clone())
                        .map_err(|_| ServiceError::Configuration)?,
                    proof_run: ProofRunId::parse(config.proof_run_id.clone())
                        .map_err(|_| ServiceError::Configuration)?,
                    scope: GrantScope::parse(&grant.scope).ok_or(ServiceError::Configuration)?,
                    expires: AuthorityExpiresAt::new(WallTimeMs::new(grant.expires_wall_ms)),
                    revision: Revision::new(grant.revision),
                    revoked: false,
                })?;
            }
        } else if recovery.subjects != 1 {
            return Err(ServiceError::Configuration);
        }
        let subject =
            SubjectId::parse(config.subject_id.clone()).map_err(|_| ServiceError::Configuration)?;
        store.subject_projection(&subject, None, None)?;
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| ServiceError::Configuration)?;
        let incarnation = RuntimeIncarnation::parse(format!(
            "incarnation:{}",
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ))
        .map_err(|_| ServiceError::Configuration)?;
        let (executor, results) = OwnedExecutor::new(
            ControlledPort {
                socket: config.peer_socket_path.clone(),
            },
            config.max_executions,
        )?;
        Ok(Self {
            config,
            store,
            executor,
            results,
            incarnation,
            entered_uncertain_on_open,
            verifier_gate: Arc::new(engine::VerifierGate::new()),
        })
    }

    pub async fn serve(mut self, mut receiver: mpsc::Receiver<Request>) {
        if self.config.profile == "controlled" {
            let _ =
                engine::drive_transition(&mut self.store, &self.config, &self.verifier_gate).await;
        }
        if self.config.profile == "controlled" && !self.entered_uncertain_on_open {
            let _ = self.schedule_once(None).await;
        }
        loop {
            tokio::select! {
                maybe = receiver.recv() => {
                    match maybe {
                        Some(Request::Command { uid,bytes,reply }) => {
                            let result = self.handle_command(uid,&bytes).await;
                            let _ = reply.send(result);
                        }
                        Some(Request::Snapshot { subject,delivery,transition,reply }) => {
                            let result = self.snapshot(&subject,delivery.as_deref(),transition.as_deref());
                            let _ = reply.send(result);
                        }
                        Some(Request::Shutdown {reply}) => {
                            let result=self.shutdown().await;
                            let _=reply.send(result);
                            return;
                        }
                        None => {let _=self.shutdown().await;return;},
                    }
                }
                maybe = self.results.recv() => {
                    let Some(result) = maybe else { break; };
                    if self.handle_result(result,false).await.is_err() { break; }
                }
            }
        }
        let _ = self.shutdown().await;
    }

    async fn shutdown(&mut self) -> Result<(), StoreError> {
        self.executor.close_gate();
        self.verifier_gate.close_gate();
        let report = self
            .executor
            .close_and_drain(Instant::now() + Duration::from_secs(5))
            .await;
        let subject = SubjectId::parse(self.config.subject_id.clone())
            .map_err(|_| StoreError::Unavailable)?;
        self.store.record_service_close(
            &self.incarnation,
            &subject,
            &report.joined,
            &report.unresolved,
            &report.pending_results,
            report.unsafe_at_close,
            clock_sample().wall,
        )?;
        for result in self.executor.pending_results() {
            self.handle_result(result, true).await?;
        }
        Ok(())
    }

    async fn handle_command(&mut self, uid: u32, bytes: &[u8]) -> Value {
        let parsed = match parse_authenticated(&self.config, uid, bytes) {
            Ok(value) => value,
            Err(AdmitError::Wire(error)) => return error_reply(error.code()),
            Err(_) => return error_reply(WireErrorCode::SemanticInvalidity),
        };
        let prior = match self.store.lookup_checked_command(&parsed.admission) {
            Ok(value) => value,
            Err(StoreError::ClaimConflict) => return error_reply(WireErrorCode::Conflict),
            Err(_) => return error_reply(WireErrorCode::ObservationUnavailable),
        };
        let replayed = prior.is_some();
        let reservation = if !replayed
            && matches!(
                parsed.admission.request().command,
                Command::EnqueueInput { .. }
            ) {
            match self.executor.try_reserve() {
                Ok(value) => Some(value),
                Err(_) => return error_reply(WireErrorCode::ObservationUnavailable),
            }
        } else {
            None
        };
        let result = match prior.map(Ok).unwrap_or_else(|| {
            self.store
                .apply_checked_command(parsed.admission, clock_sample())
        }) {
            Ok(value) => value,
            Err(StoreError::ClaimConflict) => return error_reply(WireErrorCode::Conflict),
            Err(StoreError::Rejected) => return error_reply(WireErrorCode::SemanticInvalidity),
            Err(_) => return error_reply(WireErrorCode::ObservationUnavailable),
        };
        let (accepted, outcome_kind, outcome_ref, rejection_code) = match &result.outcome {
            CommandOutcome::Enqueued { delivery, .. } => {
                (true, "enqueued", Some(delivery.as_str().to_owned()), None)
            }
            CommandOutcome::ReplacementRequested { transition } => (
                true,
                "replacement_requested",
                Some(transition.as_str().to_owned()),
                None,
            ),
            CommandOutcome::InterruptionRequested { attempt } => (
                true,
                "interruption_requested",
                Some(attempt.as_str().to_owned()),
                None,
            ),
            CommandOutcome::Rejected(code) => {
                (false, "rejected", None, Some(code.as_str().to_owned()))
            }
        };
        if accepted && !replayed {
            if outcome_kind == "enqueued" {
                let _ = self.schedule_once(reservation).await;
            }
            if outcome_kind == "replacement_requested" {
                let _ =
                    engine::drive_transition(&mut self.store, &self.config, &self.verifier_gate)
                        .await;
            }
        }
        serde_json::to_value(CommandResultV2 {
            protocol_version: 2,
            command_id: parsed.command_id,
            accepted,
            outcome_kind: outcome_kind.into(),
            outcome_ref,
            rejection_code,
            resulting_revision: result.revision.get().to_string(),
        })
        .expect("result serializes")
    }

    async fn schedule_once(
        &mut self,
        reserved: Option<ExecutionReservation<ControlledPort>>,
    ) -> Result<(), StoreError> {
        let reservation = match reserved {
            Some(value) => value,
            None => match self.executor.try_reserve() {
                Ok(value) => value,
                Err(_) => return Ok(()),
            },
        };
        let subject = SubjectId::parse(self.config.subject_id.clone())
            .map_err(|_| StoreError::Unavailable)?;
        let plan = match self.store.prepare_next_input(&subject) {
            Ok(value) => value,
            Err(StoreError::ClaimConflict) => return Ok(()),
            Err(error) => return Err(error),
        };
        proof_barrier(&self.config, "before_entry_commit").await?;
        let entry = match self.store.claim_prepared_entry(
            &plan.effect,
            self.incarnation.clone(),
            clock_sample(),
        ) {
            Ok(value) => value,
            Err(StoreError::ClaimConflict | StoreError::Rejected) => return Ok(()),
            Err(error) => return Err(error),
        };
        proof_barrier(&self.config, "after_entry_commit").await?;
        self.executor
            .submit(reservation, entry)
            .map_err(|_| StoreError::Unavailable)?;
        Ok(())
    }

    async fn handle_result(&mut self, result: TaskResult, closing: bool) -> Result<(), StoreError> {
        let termination = match &result.exit.termination {
            TaskTermination::Completed => "completed",
            TaskTermination::ProviderError(ProviderFailure::UnsupportedProfile) => {
                "unsupported_profile"
            }
            TaskTermination::ProviderError(ProviderFailure::EntryUncertain) => "entry_uncertain",
            TaskTermination::ProviderError(ProviderFailure::ObservationUnavailable) => {
                "observation_unavailable"
            }
            TaskTermination::InvalidObservation => "invalid_observation",
            TaskTermination::Panic => "panic",
            TaskTermination::Cancelled => "cancelled",
        };
        self.store.record_owned_task_result(
            &result.exit.attempt,
            &self.incarnation,
            termination,
            result.exit.local_task_ended,
            result.exit.child_exit_code,
            result.finished_wall.as_ref().ok().copied(),
            result.observation.as_ref().map(|value| &value.source),
            result.final_text.as_deref(),
        )?;
        if let Some(observation) = result.observation {
            self.store.apply_bound_result(
                observation,
                result.final_text.as_deref(),
                clock_sample(),
            )?;
            if !closing {
                proof_barrier(&self.config, "after_result_commit").await?;
                engine::drive_transition(&mut self.store, &self.config, &self.verifier_gate)
                    .await?;
            }
        }
        if !self.executor.acknowledge_result(&result.exit.attempt).await {
            return Err(StoreError::Unavailable);
        }
        // An entered attempt with no qualified observation remains blocked in S1.
        if !closing {
            self.schedule_once(None).await?;
        }
        Ok(())
    }

    fn snapshot(
        &mut self,
        subject: &str,
        delivery: Option<&str>,
        transition: Option<&str>,
    ) -> Result<SubjectProjection, StoreError> {
        if subject != self.config.subject_id {
            return Err(StoreError::Rejected);
        }
        let subject = SubjectId::parse(subject.to_owned()).map_err(|_| StoreError::Rejected)?;
        let delivery = delivery
            .map(|value| {
                lifecycle_core::DeliveryId::parse(value.to_owned())
                    .map_err(|_| StoreError::Rejected)
            })
            .transpose()?;
        let transition = transition
            .map(|value| {
                lifecycle_core::TransitionId::parse(value.to_owned())
                    .map_err(|_| StoreError::Rejected)
            })
            .transpose()?;
        self.store
            .subject_projection(&subject, delivery.as_ref(), transition.as_ref())
    }
}

pub(crate) async fn proof_barrier(config: &ServiceConfig, point: &str) -> Result<(), StoreError> {
    if !cfg!(feature = "controlled-proof") {
        return Ok(());
    }
    let Some(root) = config.proof_barrier_dir.clone() else {
        return Ok(());
    };
    let point = point.to_owned();
    // Keep the signal loop schedulable even while a controlled launch cut is
    // held by the external test process.
    tokio::task::spawn_blocking(move || {
        if !root.exists() {
            return Err(StoreError::Unavailable);
        }
        let reached = root.join(format!("{point}.reached"));
        let release = root.join(format!("{point}.release"));
        let mut file = fs::File::create(reached).map_err(|_| StoreError::Unavailable)?;
        file.write_all(b"reached\n")
            .map_err(|_| StoreError::Unavailable)?;
        file.sync_all().map_err(|_| StoreError::Unavailable)?;
        let started = Instant::now();
        while !release.exists() {
            if started.elapsed() > Duration::from_secs(15) {
                return Err(StoreError::Unavailable);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    })
    .await
    .map_err(|_| StoreError::Unavailable)?
}

fn error_reply(code: WireErrorCode) -> Value {
    json!({"protocol_version":1,"code":code,"detail":"request unavailable"})
}

pub type ActorSender = mpsc::Sender<Request>;

pub struct ShutdownGate {
    executor: OwnedExecutor<ControlledPort>,
    verifier: Arc<engine::VerifierGate>,
}

impl ShutdownGate {
    pub fn close_gate(&self) {
        self.executor.close_gate();
        self.verifier.close_gate();
    }
}

pub fn start(config: ServiceConfig) -> Result<(ActorSender, ShutdownGate), ServiceError> {
    let actor = Actor::new(config)?;
    let gate = ShutdownGate {
        executor: actor.executor.clone(),
        verifier: actor.verifier_gate.clone(),
    };
    let (sender, receiver) = mpsc::channel(64);
    tokio::spawn(actor.serve(receiver));
    Ok((sender, gate))
}
