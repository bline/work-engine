use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use lifecycle_core::{
    AdmissionOwner, AttemptFacts, AttemptId, AuthorityExpiresAt, BuildId, ClockSample, Command,
    CommandAdmission, CommandOutcome, CommandResult, ContextGeneration, ControlledTextInput,
    CustodyState, DeliveryId, EffectId, EffectInput, EffectObservation, EffectPlan,
    EffectSettlement, EntryFacts, EntryFailure, ExecutionOutcome, GrantId, GrantScope, InputId,
    ProofRunId, QueueInput, Reduction, RejectionCode, Revision, RuntimeIncarnation, SubjectId,
    SubjectState, TransitionFact, TransitionId, TransitionStage, TransitionState, TrustedGrant,
    WallTimeMs, reduce_command, reduce_entry, reduce_observation, reduce_queue, reduce_transition,
    reduce_transition_custody,
};
use rusqlite::{
    Connection, ErrorCode, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};

use crate::{AuthorizedEntry, EffectProjection, LifecycleStore, ObservationApply, StoreError};

type PriorCommandRow = (String, Vec<u8>, String, Option<String>, Option<String>, i64);
type PendingInputRow = (String, String, Vec<u8>, String, String, i64, String);
type PriorSourceRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
);
type AttemptObservationRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    Option<String>,
    String,
);
type SubjectRow = (
    String,
    String,
    String,
    i64,
    i64,
    i64,
    String,
    Option<String>,
    i64,
);
type GrantRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    i64,
    bool,
);

pub struct SqliteLifecycleStore {
    pub(super) connection: Connection,
    pub(super) root: PathBuf,
    _lock_file: File,
    trusted_issuer: String,
    root_identity: (u64, u64),
    lock_identity: (u64, u64),
    database_identity: (u64, u64),
}

impl SqliteLifecycleStore {
    /// Open one local lifecycle store. The lock remains held for this value's lifetime.
    pub fn open(root: &Path, trusted_issuer: String) -> Result<Self, StoreError> {
        if trusted_issuer.is_empty() {
            return Err(StoreError::Rejected);
        }
        fs::create_dir_all(root).map_err(|_| StoreError::Unavailable)?;
        let root = fs::canonicalize(root).map_err(|_| StoreError::Unavailable)?;
        let lock_path = root.join("writer.lock");
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|_| StoreError::Unavailable)?;
        lock_file.try_lock().map_err(|_| StoreError::Unavailable)?;
        let database = root.join("lifecycle.sqlite");
        reject_hard_link(&database)?;
        if database.exists() {
            // Incompatible readers refuse before WAL/pragma or schema mutation.
            let inspection = Connection::open_with_flags(
                &database,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
            )
            .map_err(|_| StoreError::Unavailable)?;
            let version: i64 = inspection
                .query_row("PRAGMA user_version", [], |row| row.get(0))
                .map_err(|_| StoreError::Unavailable)?;
            let has_tables: i64 = inspection.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
                [], |row| row.get(0),
            ).map_err(|_| StoreError::Unavailable)?;
            if !(0..=1).contains(&version) || (version == 0 && has_tables != 0) {
                return Err(StoreError::Unavailable);
            }
        }
        let mut connection = Connection::open_with_flags(
            &database,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| StoreError::Unavailable)?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA busy_timeout=250;",
            )
            .map_err(|_| StoreError::Unavailable)?;
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|_| StoreError::Unavailable)?;
        match version {
            0 => {
                let tx = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|_| StoreError::Unavailable)?;
                tx.execute_batch(include_str!("../migrations/0001_initial.sql"))
                    .map_err(|_| StoreError::Unavailable)?;
                tx.pragma_update(None, "user_version", 1)
                    .map_err(|_| StoreError::Unavailable)?;
                tx.commit().map_err(|_| StoreError::Unavailable)?;
            }
            1 => {}
            _ => return Err(StoreError::Unavailable),
        }
        let integrity: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|_| StoreError::Unavailable)?;
        if integrity != "ok" {
            return Err(StoreError::Unavailable);
        }
        crate::recovery::check_coherence(&connection)?;
        reject_hard_link(&database)?;
        let root_identity = file_identity(&root)?;
        let lock_identity = file_identity(&lock_path)?;
        let database_identity = file_identity(&database)?;
        Ok(Self {
            connection,
            root,
            _lock_file: lock_file,
            trusted_issuer,
            root_identity,
            lock_identity,
            database_identity,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub(super) fn verify_identity(&self) -> Result<(), StoreError> {
        if file_identity(&self.root)? != self.root_identity
            || file_identity(&self.root.join("writer.lock"))? != self.lock_identity
            || file_identity(&self.root.join("lifecycle.sqlite"))? != self.database_identity
        {
            return Err(StoreError::Unavailable);
        }
        reject_hard_link(&self.root.join("lifecycle.sqlite"))
    }

    /// Trusted service bootstrap, not a public command or proof of authentication.
    pub fn register_subject(
        &mut self,
        subject: SubjectId,
        context: ContextGeneration,
        build: BuildId,
        proof_run: ProofRunId,
        clock: ClockSample,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        tx.execute(
                "INSERT INTO subjects(subject_id,context_id,build_id,proof_run_id,revision,semantic_revision,queue_revision,owner_kind,last_wall_ms) VALUES (?1,?2,?3,?4,0,0,0,'domain',?5)",
                params![subject.as_str(), context.as_str(), build.as_str(), proof_run.as_str(), clock.wall.get()],
            )
            .map_err(|_| StoreError::Rejected)?;
        tx.execute(
            "INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'subject_registered',?1,?2)",
            params![subject.as_str(),clock.wall.get()],
        ).map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(())
    }

    /// The service must resolve and verify the grant before this restricted call.
    pub fn install_trusted_grant(&mut self, grant: TrustedGrant) -> Result<(), StoreError> {
        self.verify_identity()?;
        if grant.issuer != self.trusted_issuer {
            return Err(StoreError::Rejected);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        tx.execute(
                "INSERT INTO grants(grant_id,issuer,principal,subject_id,context_id,build_id,proof_run_id,scope,expires_ms,revision,revoked) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![grant.id.as_str(), grant.issuer, grant.principal, grant.subject.as_str(), grant.context.as_str(), grant.build.as_str(), grant.proof_run.as_str(), grant.scope.as_str(), grant.expires.wall().get(), i64::try_from(grant.revision.get()).map_err(|_| StoreError::Rejected)?, grant.revoked],
            )
            .map_err(|_| StoreError::Rejected)?;
        tx.execute(
            "INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) SELECT g.subject_id,'grant_installed',g.grant_id,s.last_wall_ms FROM grants g JOIN subjects s ON s.subject_id=g.subject_id WHERE g.grant_id=?1",
            [grant.id.as_str()],
        ).map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(())
    }

    /// Restricted service operation. Revocation and its revision are durable before new entry.
    pub fn revoke_trusted_grant(
        &mut self,
        grant: &GrantId,
        expected_revision: Revision,
        clock: ClockSample,
    ) -> Result<Revision, StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let next = Revision::new(
            expected_revision
                .get()
                .checked_add(1)
                .ok_or(StoreError::Rejected)?,
        );
        let changed = tx.execute(
            "UPDATE grants SET revoked=1,revision=?1 WHERE grant_id=?2 AND revision=?3 AND revoked=0 AND issuer=?4",
            params![to_sql_revision(next)?,grant.as_str(),to_sql_revision(expected_revision)?,self.trusted_issuer],
        ).map_err(|_| StoreError::Unavailable)?;
        if changed != 1 {
            return Err(StoreError::Rejected);
        }
        let subject: String = tx
            .query_row(
                "SELECT subject_id FROM grants WHERE grant_id=?1",
                [grant.as_str()],
                |row| row.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "UPDATE subjects SET last_wall_ms=MAX(last_wall_ms,?1) WHERE subject_id=?2",
            params![clock.wall.get(), subject.as_str()],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'grant_revoked',?2,?3)",
            params![subject,grant.as_str(),clock.wall.get()],
        ).map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(next)
    }

    pub fn apply_checked_command(
        &mut self,
        admission: CommandAdmission,
        clock: ClockSample,
    ) -> Result<CommandResult, StoreError> {
        self.verify_identity()?;
        let request = admission.request();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let prior: Option<PriorCommandRow> = tx
            .query_row(
                "SELECT request_digest,canonical_bytes,outcome_kind,outcome_ref,rejection_code,resulting_revision FROM commands WHERE principal=?1 AND command_id=?2",
                params![admission.principal(), request.command_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        let canonical = admission.canonical_bytes();
        if let Some((digest, bytes, kind, outcome_ref, rejection_code, revision)) = prior {
            if digest != request.request_digest.hex() || bytes != canonical {
                return Err(StoreError::ClaimConflict);
            }
            return decode_result(&tx, &kind, outcome_ref, rejection_code, revision);
        }
        let subject = read_subject(&tx, &request.subject)?;
        let grant = read_grant(&tx, &request.grant_reference)?
            .filter(|grant| grant.issuer == self.trusted_issuer);
        let delivery = DeliveryId::parse(format!("delivery:{}", request.request_digest.hex()))
            .map_err(|_| StoreError::Rejected)?;
        let transition =
            TransitionId::parse(format!("transition:{}", request.request_digest.hex()))
                .map_err(|_| StoreError::Rejected)?;
        let target_attempt_is_entered = match &request.command {
            Command::RequestInterruption { attempt } => tx
                .query_row(
                    "SELECT COUNT(*) FROM attempts a JOIN effects e ON e.effect_id=a.effect_id WHERE a.attempt_id=?1 AND e.subject_id=?2 AND a.outcome='pending' AND a.settlement_kind='unresolved'",
                    params![attempt.as_str(), request.subject.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|_| StoreError::Unavailable)? == 1,
            _ => false,
        };
        let reduction = if let Some(subject) = subject {
            let mut reduction = reduce_command(
                &subject,
                &admission,
                grant.as_ref(),
                clock,
                delivery,
                transition,
                target_attempt_is_entered,
            );
            if let CommandOutcome::Enqueued { input, delivery } = &reduction.result.outcome {
                let existing: i64 = tx
                    .query_row(
                        "SELECT COUNT(*) FROM inputs WHERE input_id=?1 OR delivery_id=?2",
                        params![input.as_str(), delivery.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(|_| StoreError::Unavailable)?;
                if existing != 0 {
                    reduction.result.outcome = CommandOutcome::Rejected(RejectionCode::Conflict);
                    reduction.result.revision = subject.revision;
                    reduction.next_subject = subject;
                    reduction.next_subject.last_wall = clock.wall;
                }
            }
            reduction
        } else {
            Reduction {
                result: CommandResult {
                    outcome: CommandOutcome::Rejected(RejectionCode::UnknownSubject),
                    revision: Revision::new(0),
                },
                next_subject: SubjectState {
                    subject: request.subject.clone(),
                    context: request.context.clone(),
                    build: request.build.clone(),
                    proof_run: request.proof_run.clone(),
                    revision: Revision::new(0),
                    semantic_revision: Revision::new(0),
                    queue_revision: Revision::new(0),
                    last_wall: clock.wall,
                    owner: AdmissionOwner::Fenced,
                },
            }
        };
        match &reduction.result.outcome {
            CommandOutcome::Enqueued { input, delivery } => {
                let controlled = admission.input().ok_or(StoreError::Rejected)?;
                let producer = admission.producer_ref().ok_or(StoreError::Rejected)?;
                let sequence: i64 = tx
                    .query_row(
                        "SELECT COALESCE(MAX(sequence),0)+1 FROM inputs WHERE subject_id=?1",
                        [request.subject.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(|_| StoreError::Unavailable)?;
                tx.execute(
                    "INSERT INTO inputs(input_id,subject_id,sequence,delivery_id,producer_ref,text,text_digest,grant_id,state) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'queued')",
                    params![input.as_str(), request.subject.as_str(), sequence, delivery.as_str(), producer, controlled.text().as_bytes(), controlled.digest().hex(), request.grant_reference.as_str()],
                )
                .map_err(classify_write_error)?;
                tx.execute(
                    "UPDATE subjects SET revision=?1,queue_revision=?2,last_wall_ms=?3 WHERE subject_id=?4 AND revision=?5",
                    params![to_sql_revision(reduction.next_subject.revision)?, to_sql_revision(reduction.next_subject.queue_revision)?, clock.wall.get(), request.subject.as_str(), to_sql_revision(request.expected_revision)?],
                )
                .map_err(|_| StoreError::Unavailable)?;
            }
            CommandOutcome::ReplacementRequested { transition } => {
                tx.execute(
                    "INSERT INTO transitions(transition_id,subject_id,predecessor_context,stage,basis_revision,created_wall_ms) VALUES (?1,?2,?3,'quiescing',?4,?5)",
                    params![transition.as_str(), request.subject.as_str(), request.context.as_str(), to_sql_revision(reduction.next_subject.semantic_revision)?, clock.wall.get()],
                ).map_err(classify_write_error)?;
                tx.execute(
                    "UPDATE subjects SET revision=?1,semantic_revision=?2,owner_kind='transition',owner_ref=?3,last_wall_ms=?4 WHERE subject_id=?5 AND revision=?6",
                    params![to_sql_revision(reduction.next_subject.revision)?, to_sql_revision(reduction.next_subject.semantic_revision)?, transition.as_str(), clock.wall.get(), request.subject.as_str(), to_sql_revision(request.expected_revision)?],
                ).map_err(|_| StoreError::Unavailable)?;
            }
            CommandOutcome::InterruptionRequested { attempt } => {
                tx.execute(
                    "INSERT INTO interruption_requests(principal,command_id,subject_id,attempt_id,requested_wall_ms) VALUES (?1,?2,?3,?4,?5)",
                    params![admission.principal(), request.command_id.as_str(), request.subject.as_str(), attempt.as_str(), clock.wall.get()],
                ).map_err(classify_write_error)?;
                tx.execute(
                    "UPDATE subjects SET revision=?1,last_wall_ms=?2 WHERE subject_id=?3 AND revision=?4",
                    params![to_sql_revision(reduction.next_subject.revision)?, clock.wall.get(), request.subject.as_str(), to_sql_revision(request.expected_revision)?],
                ).map_err(|_| StoreError::Unavailable)?;
            }
            CommandOutcome::Rejected(RejectionCode::UnknownSubject) => {}
            CommandOutcome::Rejected(RejectionCode::ClockRollback) => {
                tx.execute(
                    "UPDATE subjects SET owner_kind='fenced',owner_ref=NULL,revision=?1 WHERE subject_id=?2",
                    params![to_sql_revision(reduction.next_subject.revision)?,request.subject.as_str()],
                ).map_err(|_| StoreError::Unavailable)?;
            }
            CommandOutcome::Rejected(_) => {
                tx.execute(
                    "UPDATE subjects SET last_wall_ms=MAX(last_wall_ms,?1) WHERE subject_id=?2",
                    params![
                        reduction.next_subject.last_wall.get(),
                        request.subject.as_str()
                    ],
                )
                .map_err(|_| StoreError::Unavailable)?;
            }
        }
        let (kind, outcome_ref, rejection_code) = encode_outcome(&reduction.result.outcome);
        tx.execute(
            "INSERT INTO commands(principal,command_id,subject_id,request_digest,canonical_bytes,outcome_kind,outcome_ref,rejection_code,resulting_revision) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![admission.principal(), request.command_id.as_str(), request.subject.as_str(), request.request_digest.hex(), canonical, kind, outcome_ref, rejection_code, to_sql_revision(reduction.result.revision)?],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,?2,?3,?4)",
            params![
                request.subject.as_str(),
                kind,
                request.command_id.as_str(),
                clock.wall.get()
            ],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(reduction.result)
    }

    /// Selects the earliest unresolved input. Enqueue itself never starts delivery.
    pub fn prepare_next_input(&mut self, subject: &SubjectId) -> Result<EffectPlan, StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let subject_state = read_subject(&tx, subject)?.ok_or(StoreError::Rejected)?;
        let mut statement = tx.prepare(
            "SELECT input_id,delivery_id,text,text_digest,state,sequence,grant_id FROM inputs WHERE subject_id=?1 AND state NOT IN ('succeeded','retired')",
        ).map_err(|_| StoreError::Unavailable)?;
        let rows = statement
            .query_map(
                [subject.as_str()],
                |row| -> rusqlite::Result<PendingInputRow> {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .map_err(|_| StoreError::Unavailable)?;
        let mut queue = Vec::new();
        for row in rows {
            let (input_id, delivery_id, text, digest, state, sequence, _) =
                row.map_err(|_| StoreError::Unavailable)?;
            let text = String::from_utf8(text).map_err(|_| StoreError::Unavailable)?;
            let prepared: Option<(String, i64)> = tx.query_row(
                "SELECT effect_id,expected_revision FROM effects WHERE input_id=?1 AND state='prepared'",
                [&input_id],
                |row| Ok((row.get(0)?,row.get(1)?)),
            ).optional().map_err(|_| StoreError::Unavailable)?;
            queue.push(QueueInput {
                input: verified_input(&input_id, text, &digest)?,
                delivery: DeliveryId::parse(delivery_id).map_err(|_| StoreError::Unavailable)?,
                sequence: u64::try_from(sequence).map_err(|_| StoreError::Unavailable)?,
                state: parse_custody(&state)?,
                prepared_effect: prepared
                    .map(|(id, revision)| {
                        Ok((
                            EffectId::parse(id).map_err(|_| StoreError::Unavailable)?,
                            from_sql_revision(revision)?,
                        ))
                    })
                    .transpose()?,
            });
        }
        drop(statement);
        let reduction =
            reduce_queue(&subject_state, &queue).map_err(|_| StoreError::ClaimConflict)?;
        let input_id = match &reduction.plan.input {
            EffectInput::ControlledText(input) => input.input_id().as_str(),
        };
        if reduction.insert_effect {
            tx.execute(
                "INSERT INTO effects(effect_id,subject_id,input_id,expected_revision,state) VALUES (?1,?2,?3,?4,'prepared')",
                params![reduction.plan.effect.as_str(), subject.as_str(), input_id, to_sql_revision(reduction.plan.expected_revision)?],
            )
            .map_err(|_| StoreError::Unavailable)?;
            tx.execute(
                "UPDATE inputs SET state='prepared' WHERE input_id=?1",
                [input_id],
            )
            .map_err(|_| StoreError::Unavailable)?;
            tx.execute(
                "INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) SELECT subject_id,'effect_prepared',?1,last_wall_ms FROM subjects WHERE subject_id=?2",
                params![reduction.plan.effect.as_str(), subject.as_str()],
            )
            .map_err(|_| StoreError::Unavailable)?;
        }
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(reduction.plan)
    }

    /// Re-loads a committed plan, records Entered, then mints the local entry value.
    pub fn claim_prepared_entry(
        &mut self,
        effect: &EffectId,
        incarnation: RuntimeIncarnation,
        clock: ClockSample,
    ) -> Result<AuthorizedEntry, StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let row: (String, String, Vec<u8>, String, String, String, String, i64, i64, String) = tx
            .query_row(
                "SELECT e.subject_id,i.input_id,i.text,i.text_digest,COALESCE(i.entry_grant_id,i.grant_id),e.state,i.state,e.expected_revision,s.semantic_revision,s.owner_kind FROM effects e JOIN inputs i ON i.input_id=e.input_id JOIN subjects s ON s.subject_id=e.subject_id WHERE e.effect_id=?1",
                [effect.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?)),
            )
            .map_err(|_| StoreError::ClaimConflict)?;
        let (
            subject_id,
            input_id,
            text,
            digest,
            grant_id,
            effect_state,
            input_state,
            expected,
            _current,
            _owner,
        ) = row;
        let earlier: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM inputs earlier JOIN inputs selected ON selected.input_id=?1 WHERE earlier.subject_id=selected.subject_id AND earlier.sequence<selected.sequence AND earlier.state NOT IN ('succeeded','retired')",
                [&input_id],
                |row| row.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        let grant_id = GrantId::parse(grant_id).map_err(|_| StoreError::Unavailable)?;
        let grant = read_grant(&tx, &grant_id)?.ok_or(StoreError::Rejected)?;
        let subject = read_subject(
            &tx,
            &SubjectId::parse(subject_id).map_err(|_| StoreError::Unavailable)?,
        )?
        .ok_or(StoreError::Unavailable)?;
        let text = String::from_utf8(text).map_err(|_| StoreError::Unavailable)?;
        let controlled = verified_input(&input_id, text, &digest)?;
        let plan = EffectPlan {
            effect: effect.clone(),
            subject: subject.subject.clone(),
            input: EffectInput::ControlledText(controlled.clone()),
            expected_revision: from_sql_revision(expected)?,
        };
        let entry_decision = reduce_entry(
            &subject,
            &plan,
            EntryFacts {
                effect_state: parse_custody(&effect_state).unwrap_or(CustodyState::Retired),
                input_state: parse_custody(&input_state)?,
                grant: &grant,
                trusted_issuer: &self.trusted_issuer,
                clock,
                earlier_unresolved: earlier != 0,
            },
        );
        if matches!(entry_decision, Err(EntryFailure::ClockRollback)) {
            fence_subject(&tx, subject.subject.as_str(), "clock:rollback", clock)?;
            tx.commit().map_err(|_| StoreError::Unavailable)?;
            return Err(StoreError::Rejected);
        }
        tx.execute(
            "UPDATE subjects SET last_wall_ms=MAX(last_wall_ms,?1) WHERE subject_id=?2",
            params![clock.wall.get(), subject.subject.as_str()],
        )
        .map_err(|_| StoreError::Unavailable)?;
        if let Err(reason) = entry_decision {
            tx.commit().map_err(|_| StoreError::Unavailable)?;
            return Err(match reason {
                EntryFailure::InvalidGrant | EntryFailure::ExpiredGrant => StoreError::Rejected,
                _ => StoreError::ClaimConflict,
            });
        }
        let next: i64 = tx
            .query_row("SELECT COALESCE(MAX(rowid),0)+1 FROM attempts", [], |row| {
                row.get(0)
            })
            .map_err(|_| StoreError::Unavailable)?;
        let attempt =
            AttemptId::parse(format!("attempt:{next}")).map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "INSERT INTO attempts(attempt_id,effect_id,incarnation_id,state,entered_wall_ms) VALUES (?1,?2,?3,'entered',?4)",
            params![attempt.as_str(), effect.as_str(), incarnation.as_str(), clock.wall.get()],
        )
        .map_err(classify_write_error)?;
        tx.execute(
            "UPDATE effects SET state='entered' WHERE effect_id=?1",
            [effect.as_str()],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "UPDATE inputs SET state='entered' WHERE input_id=?1",
            [&input_id],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'effect_entered',?2,?3)",
            params![subject.subject.as_str(), attempt.as_str(), clock.wall.get()],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(AuthorizedEntry::new(
            effect.clone(),
            attempt,
            subject.subject,
            incarnation,
            EffectInput::ControlledText(controlled),
        ))
    }

    pub fn counts(&self) -> Result<(i64, i64, i64), StoreError> {
        let commands = self
            .connection
            .query_row("SELECT COUNT(*) FROM commands", [], |row| row.get(0))
            .map_err(|_| StoreError::Unavailable)?;
        let inputs = self
            .connection
            .query_row("SELECT COUNT(*) FROM inputs", [], |row| row.get(0))
            .map_err(|_| StoreError::Unavailable)?;
        let attempts = self
            .connection
            .query_row("SELECT COUNT(*) FROM attempts", [], |row| row.get(0))
            .map_err(|_| StoreError::Unavailable)?;
        Ok((commands, inputs, attempts))
    }

    pub fn apply_bound_observation(
        &mut self,
        observation: EffectObservation,
        clock: ClockSample,
    ) -> Result<ObservationApply, StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let prior_source: Option<PriorSourceRow> = tx
            .query_row(
                "SELECT effect_id,attempt_id,incarnation_id,provider_thread_id,provider_turn_id,outcome,settlement_kind,settlement_evidence FROM observations WHERE source_id=?1",
                [observation.source.as_str()],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?)),
            ).optional().map_err(|_| StoreError::Unavailable)?;
        let (new_settlement_kind, new_evidence) = encode_settlement(&observation.settlement);
        if let Some(prior) = prior_source {
            if prior.0 == observation.effect.as_str()
                && prior.1 == observation.attempt.as_str()
                && prior.2 == observation.incarnation.as_str()
                && prior.3 == observation.provider_thread.as_str()
                && prior.4 == observation.provider_turn.as_str()
                && prior.5 == outcome_text(&observation.outcome)
                && prior.6 == new_settlement_kind
                && prior.7.as_deref() == new_evidence
            {
                return Ok(ObservationApply::Duplicate);
            }
            let previous_variant: Option<String> = tx.query_row(
                "SELECT attribution_kind FROM observation_conflicts WHERE source_id=?1 AND effect_id=?2 AND attempt_id=?3 AND incarnation_id=?4 AND provider_thread_id=?5 AND provider_turn_id=?6 AND outcome=?7 AND settlement_kind=?8 AND settlement_evidence IS ?9",
                params![observation.source.as_str(),observation.effect.as_str(),observation.attempt.as_str(),observation.incarnation.as_str(),observation.provider_thread.as_str(),observation.provider_turn.as_str(),outcome_text(&observation.outcome),new_settlement_kind,new_evidence],
                |row| row.get(0),
            ).optional().map_err(|_| StoreError::Unavailable)?;
            if let Some(attribution) = previous_variant {
                return if attribution == "exact" {
                    Ok(ObservationApply::ConflictFenced)
                } else {
                    Err(StoreError::ObservationMismatch)
                };
            }
            let subject: String = tx.query_row(
                "SELECT e.subject_id FROM attempts a JOIN effects e ON e.effect_id=a.effect_id WHERE a.attempt_id=?1",
                [&prior.1],
                |row| row.get(0),
            ).map_err(|_| StoreError::ObservationMismatch)?;
            let incoming: Option<(String, String)> = tx.query_row(
                "SELECT e.subject_id,a.incarnation_id FROM attempts a JOIN effects e ON e.effect_id=a.effect_id WHERE a.attempt_id=?1 AND e.effect_id=?2",
                params![observation.attempt.as_str(),observation.effect.as_str()],
                |row| Ok((row.get(0)?,row.get(1)?)),
            ).optional().map_err(|_| StoreError::Unavailable)?;
            let attributed_subject = incoming
                .filter(|(_, incarnation)| incarnation == observation.incarnation.as_str())
                .map(|(subject, _)| subject);
            let attribution = if attributed_subject.is_some() {
                "exact"
            } else {
                "mismatch"
            };
            tx.execute(
                "INSERT INTO observation_conflicts(source_id,effect_id,attempt_id,incarnation_id,provider_thread_id,provider_turn_id,outcome,settlement_kind,settlement_evidence,attribution_kind,observed_wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![observation.source.as_str(),observation.effect.as_str(),observation.attempt.as_str(),observation.incarnation.as_str(),observation.provider_thread.as_str(),observation.provider_turn.as_str(),outcome_text(&observation.outcome),new_settlement_kind,new_evidence,attribution,clock.wall.get()],
            ).map_err(|_| StoreError::Unavailable)?;
            // A mismatched variant is retained as a source-integrity fault,
            // not applied as a settlement fact about the original attempt.
            if attribution == "exact" {
                tx.execute(
                    "UPDATE attempts SET settlement_kind='conflict',settlement_evidence=?1 WHERE attempt_id=?2",
                    params![observation.source.as_str(),prior.1],
                ).map_err(|_| StoreError::Unavailable)?;
            }
            fence_subject(&tx, &subject, observation.source.as_str(), clock)?;
            if let Some(incoming_subject) = attributed_subject
                && incoming_subject != subject
            {
                fence_subject(&tx, &incoming_subject, observation.source.as_str(), clock)?;
            }
            tx.commit().map_err(|_| StoreError::Unavailable)?;
            return if attribution == "exact" {
                Ok(ObservationApply::ConflictFenced)
            } else {
                Err(StoreError::ObservationMismatch)
            };
        }
        let row: AttemptObservationRow = tx
            .query_row(
                "SELECT e.subject_id,e.input_id,a.incarnation_id,a.state,a.provider_thread_id,a.provider_turn_id,a.outcome,a.settlement_kind,a.settlement_evidence,i.state FROM attempts a JOIN effects e ON e.effect_id=a.effect_id JOIN inputs i ON i.input_id=e.input_id WHERE a.attempt_id=?1 AND e.effect_id=?2",
                params![observation.attempt.as_str(), observation.effect.as_str()],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?,row.get(9)?)),
            ).map_err(|_| StoreError::ObservationMismatch)?;
        let (
            subject,
            input,
            incarnation,
            attempt_state,
            thread,
            turn,
            prior_outcome,
            prior_settlement,
            prior_evidence,
            prior_input_state,
        ) = row;
        let prior = AttemptFacts {
            incarnation: RuntimeIncarnation::parse(incarnation)
                .map_err(|_| StoreError::Unavailable)?,
            provider_thread: thread
                .map(|value| {
                    lifecycle_core::ProviderThreadId::parse(value)
                        .map_err(|_| StoreError::Unavailable)
                })
                .transpose()?,
            provider_turn: turn
                .map(|value| {
                    lifecycle_core::ProviderTurnId::parse(value)
                        .map_err(|_| StoreError::Unavailable)
                })
                .transpose()?,
            outcome: parse_outcome(&prior_outcome)?,
            settlement: parse_settlement(&prior_settlement, prior_evidence)?,
            input_state: parse_custody(&prior_input_state)?,
            attempt_entered: attempt_state == "entered",
        };
        let subject_state = read_subject(
            &tx,
            &SubjectId::parse(subject.clone()).map_err(|_| StoreError::Unavailable)?,
        )?
        .ok_or(StoreError::Unavailable)?;
        let decision = reduce_observation(&subject_state, &prior, &observation, clock)
            .map_err(|_| StoreError::ObservationMismatch)?;
        tx.execute(
            "INSERT INTO observations(source_id,effect_id,attempt_id,incarnation_id,provider_thread_id,provider_turn_id,outcome,settlement_kind,settlement_evidence,applied_kind,observed_wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![observation.source.as_str(),observation.effect.as_str(),observation.attempt.as_str(),observation.incarnation.as_str(),observation.provider_thread.as_str(),observation.provider_turn.as_str(),outcome_text(&observation.outcome),new_settlement_kind,new_evidence,if decision.conflict { "conflict_fenced" } else { "applied" },clock.wall.get()],
        ).map_err(|_| StoreError::Unavailable)?;
        let (settlement_kind, settlement_evidence) = encode_settlement(&decision.settlement);
        let still_running = decision.still_running;
        tx.execute(
            "UPDATE attempts SET state=?1,provider_thread_id=?2,provider_turn_id=?3,outcome=?4,settlement_kind=?5,settlement_evidence=?6 WHERE attempt_id=?7",
            params![if still_running { "entered" } else { "observed" },decision.provider_thread.as_ref().map(|id| id.as_str()),decision.provider_turn.as_ref().map(|id| id.as_str()),outcome_text(&decision.outcome),settlement_kind,settlement_evidence,observation.attempt.as_str()],
        ).map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "UPDATE effects SET state=?1 WHERE effect_id=?2",
            params![
                if still_running { "entered" } else { "applied" },
                observation.effect.as_str()
            ],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "UPDATE inputs SET state=?1 WHERE input_id=?2",
            params![custody_text(decision.input_state), input],
        )
        .map_err(|_| StoreError::Unavailable)?;
        let (owner_kind, owner_ref) = match &decision.next_subject.owner {
            AdmissionOwner::Domain { .. } => ("domain", None),
            AdmissionOwner::Transition { id } => ("transition", Some(id.as_str())),
            AdmissionOwner::Fenced => ("fenced", None),
        };
        tx.execute(
            "UPDATE subjects SET revision=?1,last_wall_ms=?2,owner_kind=?3,owner_ref=?4 WHERE subject_id=?5 AND revision=?6",
            params![
                to_sql_revision(decision.next_subject.revision)?,
                decision.next_subject.last_wall.get(),
                owner_kind,
                owner_ref,
                subject,
                to_sql_revision(subject_state.revision)?
            ],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,?2,?3,?4)",
            params![
                subject,
                if decision.conflict {
                    "observation_conflict"
                } else {
                    "observation_applied"
                },
                observation.source.as_str(),
                clock.wall.get()
            ],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(if decision.conflict {
            ObservationApply::ConflictFenced
        } else {
            ObservationApply::Applied
        })
    }

    pub fn effect_projection(&self, effect: &EffectId) -> Result<EffectProjection, StoreError> {
        let row: (String, String, String, Option<String>, String) = self.connection.query_row(
            "SELECT a.attempt_id,a.outcome,a.settlement_kind,a.settlement_evidence,s.owner_kind FROM attempts a JOIN effects e ON e.effect_id=a.effect_id JOIN subjects s ON s.subject_id=e.subject_id WHERE e.effect_id=?1",
            [effect.as_str()],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
        ).map_err(|_| StoreError::Unavailable)?;
        Ok(EffectProjection {
            effect: effect.clone(),
            attempt: AttemptId::parse(row.0).map_err(|_| StoreError::Unavailable)?,
            outcome: parse_outcome(&row.1)?,
            settlement: parse_settlement(&row.2, row.3)?,
            admission_fenced: row.4 == "fenced",
        })
    }

    /// Restricted service fact application. External evidence admission remains S4-owned.
    pub fn apply_transition_fact(
        &mut self,
        transition: &TransitionId,
        expected_revision: Revision,
        fact: TransitionFact,
        successor_grant: Option<&GrantId>,
        clock: ClockSample,
    ) -> Result<TransitionState, StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let current = read_transition(&tx, transition)?;
        let subject = read_subject(&tx, &current.subject)?.ok_or(StoreError::Unavailable)?;
        if subject.revision != expected_revision
            || !matches!(&subject.owner, AdmissionOwner::Transition { id } if id == transition)
            || clock.wall.get() < subject.last_wall.get()
        {
            return Err(StoreError::Rejected);
        }
        let already_seen: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM transition_evidence WHERE source_id=?1",
                [fact.evidence().as_str()],
                |row| row.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        if already_seen != 0 {
            return Err(StoreError::ClaimConflict);
        }
        let next = reduce_transition(&current, fact.clone()).map_err(|_| StoreError::Rejected)?;
        match next.stage {
            TransitionStage::Reconciled => {
                let unresolved: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM attempts a JOIN effects e ON e.effect_id=a.effect_id WHERE e.subject_id=?1 AND (a.settlement_kind!='established' OR a.outcome NOT IN ('completed','failed','interrupted'))",
                    [subject.subject.as_str()], |row| row.get(0),
                ).map_err(|_| StoreError::Unavailable)?;
                let blocked: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM inputs WHERE subject_id=?1 AND state IN ('entered','failed_blocked','unknown')",
                    [subject.subject.as_str()], |row| row.get(0),
                ).map_err(|_| StoreError::Unavailable)?;
                if unresolved != 0 || blocked != 0 {
                    return Err(StoreError::Rejected);
                }
                let pending: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM inputs WHERE subject_id=?1 AND state IN ('queued','prepared')",
                    [subject.subject.as_str()], |row| row.get(0),
                ).map_err(|_| StoreError::Unavailable)?;
                let successor = next.successor.as_ref().ok_or(StoreError::Rejected)?;
                if pending != 0 {
                    let grant = successor_grant
                        .map(|id| read_grant(&tx, id))
                        .transpose()?
                        .flatten();
                    let valid_grant = grant.as_ref().is_some_and(|grant| {
                        grant.issuer == self.trusted_issuer
                            && !grant.revoked
                            && grant.expires.allows_new_entry(clock)
                            && grant.scope == GrantScope::EnqueueInput
                            && grant.subject == subject.subject
                            && grant.context == *successor
                            && grant.build == subject.build
                            && grant.proof_run == subject.proof_run
                    });
                    if !valid_grant {
                        // The fact and custody remain uncommitted, but a
                        // trusted clock sample cannot be forgotten on denial.
                        tx.execute(
                            "UPDATE subjects SET last_wall_ms=MAX(last_wall_ms,?1) WHERE subject_id=?2",
                            params![clock.wall.get(), subject.subject.as_str()],
                        )
                        .map_err(|_| StoreError::Unavailable)?;
                        tx.commit().map_err(|_| StoreError::Unavailable)?;
                        return Err(StoreError::Rejected);
                    }
                    let grant_id = successor_grant.ok_or(StoreError::Unavailable)?;
                    if reduce_transition_custody(next.stage, CustodyState::Prepared)
                        == CustodyState::Queued
                    {
                        tx.execute(
                            "UPDATE effects SET state='cancelled' WHERE subject_id=?1 AND state='prepared'",
                            [subject.subject.as_str()],
                        ).map_err(|_| StoreError::Unavailable)?;
                        tx.execute(
                            "UPDATE inputs SET state='queued',entry_grant_id=?1 WHERE subject_id=?2 AND state IN ('queued','prepared')",
                            params![grant_id.as_str(),subject.subject.as_str()],
                        ).map_err(|_| StoreError::Unavailable)?;
                    }
                }
                tx.execute(
                    "UPDATE subjects SET context_id=?1,owner_kind='domain',owner_ref=NULL,revision=revision+1,semantic_revision=semantic_revision+1,last_wall_ms=?2 WHERE subject_id=?3",
                    params![successor.as_str(),clock.wall.get(),subject.subject.as_str()],
                ).map_err(|_| StoreError::Unavailable)?;
            }
            TransitionStage::AbortedBeforeEntry => {
                // Replacement advanced the semantic revision when it reserved
                // ownership. Unentered plans from the predecessor can no
                // longer be claimed, so return their custody to the queue.
                if reduce_transition_custody(next.stage, CustodyState::Prepared)
                    == CustodyState::Queued
                {
                    tx.execute(
                        "UPDATE effects SET state='cancelled' WHERE subject_id=?1 AND state='prepared'",
                        [subject.subject.as_str()],
                    ).map_err(|_| StoreError::Unavailable)?;
                    tx.execute(
                        "UPDATE inputs SET state='queued' WHERE subject_id=?1 AND state='prepared'",
                        [subject.subject.as_str()],
                    )
                    .map_err(|_| StoreError::Unavailable)?;
                }
                tx.execute(
                    "UPDATE subjects SET owner_kind='domain',owner_ref=NULL,revision=revision+1,last_wall_ms=?1 WHERE subject_id=?2",
                    params![clock.wall.get(),subject.subject.as_str()],
                ).map_err(|_| StoreError::Unavailable)?;
            }
            TransitionStage::RecoveryRequired => {
                tx.execute(
                    "UPDATE subjects SET owner_kind='fenced',owner_ref=NULL,revision=revision+1,last_wall_ms=?1 WHERE subject_id=?2",
                    params![clock.wall.get(),subject.subject.as_str()],
                ).map_err(|_| StoreError::Unavailable)?;
            }
            _ => {
                tx.execute(
                    "UPDATE subjects SET revision=revision+1,last_wall_ms=?1 WHERE subject_id=?2",
                    params![clock.wall.get(), subject.subject.as_str()],
                )
                .map_err(|_| StoreError::Unavailable)?;
            }
        }
        tx.execute(
            "UPDATE transitions SET stage=?1,checkpoint_id=?2,successor_context=?3,successor_thread=?4 WHERE transition_id=?5",
            params![next.stage.as_str(),next.checkpoint.as_ref().map(|id| id.as_str()),next.successor.as_ref().map(|id| id.as_str()),next.successor_thread.as_ref().map(|id| id.as_str()),transition.as_str()],
        ).map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "INSERT INTO transition_evidence(source_id,transition_id,stage_after,observed_wall_ms) VALUES (?1,?2,?3,?4)",
            params![fact.evidence().as_str(),transition.as_str(),next.stage.as_str(),clock.wall.get()],
        ).map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'transition_fact',?2,?3)",
            params![subject.subject.as_str(),fact.evidence().as_str(),clock.wall.get()],
        ).map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(next)
    }
}

impl LifecycleStore for SqliteLifecycleStore {
    type CommandResult = CommandResult;
    type ApplyResult = ObservationApply;

    fn apply_command(
        &mut self,
        admission: CommandAdmission,
        clock: ClockSample,
    ) -> Result<Self::CommandResult, StoreError> {
        self.apply_checked_command(admission, clock)
    }

    fn prepare_next_input(&mut self, subject: &SubjectId) -> Result<EffectPlan, StoreError> {
        SqliteLifecycleStore::prepare_next_input(self, subject)
    }

    fn claim_entry(
        &mut self,
        effect: &EffectId,
        incarnation: RuntimeIncarnation,
        clock: ClockSample,
    ) -> Result<AuthorizedEntry, StoreError> {
        self.claim_prepared_entry(effect, incarnation, clock)
    }

    fn apply_observation(
        &mut self,
        observation: EffectObservation,
        clock: ClockSample,
    ) -> Result<Self::ApplyResult, StoreError> {
        self.apply_bound_observation(observation, clock)
    }
}

fn read_transition(
    tx: &Transaction<'_>,
    transition: &TransitionId,
) -> Result<TransitionState, StoreError> {
    use lifecycle_core::{CheckpointId, EvidenceId, ProviderThreadId};
    let row: (String, String, String, i64, Option<String>, Option<String>, Option<String>) = tx.query_row(
        "SELECT subject_id,predecessor_context,stage,basis_revision,checkpoint_id,successor_context,successor_thread FROM transitions WHERE transition_id=?1",
        [transition.as_str()],
        |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?)),
    ).map_err(|_| StoreError::Rejected)?;
    let mut statement = tx
        .prepare("SELECT source_id FROM transition_evidence WHERE transition_id=?1 ORDER BY rowid")
        .map_err(|_| StoreError::Unavailable)?;
    let evidence = statement
        .query_map([transition.as_str()], |row| row.get::<_, String>(0))
        .map_err(|_| StoreError::Unavailable)?
        .map(|entry| {
            EvidenceId::parse(entry.map_err(|_| StoreError::Unavailable)?)
                .map_err(|_| StoreError::Unavailable)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TransitionState {
        id: transition.clone(),
        subject: SubjectId::parse(row.0).map_err(|_| StoreError::Unavailable)?,
        predecessor: ContextGeneration::parse(row.1).map_err(|_| StoreError::Unavailable)?,
        stage: TransitionStage::parse(&row.2).ok_or(StoreError::Unavailable)?,
        basis_semantic_revision: from_sql_revision(row.3)?,
        checkpoint: row
            .4
            .map(|value| CheckpointId::parse(value).map_err(|_| StoreError::Unavailable))
            .transpose()?,
        successor: row
            .5
            .map(|value| ContextGeneration::parse(value).map_err(|_| StoreError::Unavailable))
            .transpose()?,
        successor_thread: row
            .6
            .map(|value| ProviderThreadId::parse(value).map_err(|_| StoreError::Unavailable))
            .transpose()?,
        evidence,
    })
}

fn classify_write_error(error: rusqlite::Error) -> StoreError {
    match error {
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.code == ErrorCode::ConstraintViolation =>
        {
            StoreError::ClaimConflict
        }
        _ => StoreError::Unavailable,
    }
}

fn fence_subject(
    tx: &Transaction<'_>,
    subject: &str,
    source: &str,
    clock: ClockSample,
) -> Result<(), StoreError> {
    tx.execute(
        "UPDATE subjects SET owner_kind='fenced',owner_ref=NULL,revision=revision+1,last_wall_ms=MAX(last_wall_ms,?1) WHERE subject_id=?2",
        params![clock.wall.get(),subject],
    ).map_err(|_| StoreError::Unavailable)?;
    tx.execute(
        "INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'observation_conflict',?2,?3)",
        params![subject,source,clock.wall.get()],
    ).map_err(|_| StoreError::Unavailable)?;
    Ok(())
}

fn outcome_text(value: &ExecutionOutcome) -> &'static str {
    match value {
        ExecutionOutcome::Pending => "pending",
        ExecutionOutcome::Completed => "completed",
        ExecutionOutcome::Failed => "failed",
        ExecutionOutcome::Interrupted => "interrupted",
        ExecutionOutcome::Unknown => "unknown",
    }
}

fn parse_custody(value: &str) -> Result<CustodyState, StoreError> {
    match value {
        "queued" => Ok(CustodyState::Queued),
        "prepared" => Ok(CustodyState::Prepared),
        "entered" => Ok(CustodyState::Entered),
        "succeeded" => Ok(CustodyState::Succeeded),
        "failed_blocked" => Ok(CustodyState::FailedBlocked),
        "unknown" => Ok(CustodyState::Unknown),
        "retired" => Ok(CustodyState::Retired),
        _ => Err(StoreError::Unavailable),
    }
}

fn custody_text(value: CustodyState) -> &'static str {
    match value {
        CustodyState::Queued => "queued",
        CustodyState::Prepared => "prepared",
        CustodyState::Entered => "entered",
        CustodyState::Succeeded => "succeeded",
        CustodyState::FailedBlocked => "failed_blocked",
        CustodyState::Unknown => "unknown",
        CustodyState::Retired => "retired",
    }
}

fn parse_outcome(value: &str) -> Result<ExecutionOutcome, StoreError> {
    match value {
        "pending" => Ok(ExecutionOutcome::Pending),
        "completed" => Ok(ExecutionOutcome::Completed),
        "failed" => Ok(ExecutionOutcome::Failed),
        "interrupted" => Ok(ExecutionOutcome::Interrupted),
        "unknown" => Ok(ExecutionOutcome::Unknown),
        _ => Err(StoreError::Unavailable),
    }
}

fn encode_settlement(value: &EffectSettlement) -> (&'static str, Option<&str>) {
    match value {
        EffectSettlement::Unresolved => ("unresolved", None),
        EffectSettlement::Established(evidence) => ("established", Some(evidence.as_str())),
        EffectSettlement::Conflict(evidence) => ("conflict", Some(evidence.as_str())),
    }
}

fn parse_settlement(kind: &str, evidence: Option<String>) -> Result<EffectSettlement, StoreError> {
    use lifecycle_core::EvidenceId;
    match kind {
        "unresolved" if evidence.is_none() => Ok(EffectSettlement::Unresolved),
        "established" => Ok(EffectSettlement::Established(
            EvidenceId::parse(evidence.ok_or(StoreError::Unavailable)?)
                .map_err(|_| StoreError::Unavailable)?,
        )),
        "conflict" => Ok(EffectSettlement::Conflict(
            EvidenceId::parse(evidence.ok_or(StoreError::Unavailable)?)
                .map_err(|_| StoreError::Unavailable)?,
        )),
        _ => Err(StoreError::Unavailable),
    }
}

fn reject_hard_link(path: &Path) -> Result<(), StoreError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        // A second root may otherwise lock its own writer.lock while opening
        // the same database through a file symlink.
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            return Err(StoreError::Unavailable);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.nlink() != 1 {
                return Err(StoreError::Unavailable);
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn file_identity(path: &Path) -> Result<(u64, u64), StoreError> {
    use std::os::unix::fs::MetadataExt;
    let metadata = fs::metadata(path).map_err(|_| StoreError::Unavailable)?;
    Ok((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn file_identity(_path: &Path) -> Result<(u64, u64), StoreError> {
    Err(StoreError::Unavailable)
}

fn read_subject(
    tx: &Transaction<'_>,
    subject: &SubjectId,
) -> Result<Option<SubjectState>, StoreError> {
    let row: Option<SubjectRow> = tx
        .query_row(
            "SELECT context_id,build_id,proof_run_id,revision,semantic_revision,queue_revision,owner_kind,owner_ref,last_wall_ms FROM subjects WHERE subject_id=?1",
            [subject.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?)),
        )
        .optional()
        .map_err(|_| StoreError::Unavailable)?;
    row.map(
        |(
            context,
            build,
            proof_run,
            revision,
            semantic_revision,
            queue_revision,
            owner,
            owner_ref,
            last_wall,
        )| {
            let context = ContextGeneration::parse(context).map_err(|_| StoreError::Unavailable)?;
            Ok(SubjectState {
                subject: subject.clone(),
                context: context.clone(),
                build: BuildId::parse(build).map_err(|_| StoreError::Unavailable)?,
                proof_run: ProofRunId::parse(proof_run).map_err(|_| StoreError::Unavailable)?,
                revision: from_sql_revision(revision)?,
                semantic_revision: from_sql_revision(semantic_revision)?,
                queue_revision: from_sql_revision(queue_revision)?,
                last_wall: WallTimeMs::new(last_wall),
                owner: match owner.as_str() {
                    "domain" => AdmissionOwner::Domain {
                        generation: context,
                    },
                    "transition" => AdmissionOwner::Transition {
                        id: TransitionId::parse(owner_ref.ok_or(StoreError::Unavailable)?)
                            .map_err(|_| StoreError::Unavailable)?,
                    },
                    "fenced" => AdmissionOwner::Fenced,
                    _ => return Err(StoreError::Unavailable),
                },
            })
        },
    )
    .transpose()
}

fn read_grant(tx: &Transaction<'_>, grant: &GrantId) -> Result<Option<TrustedGrant>, StoreError> {
    let row: Option<GrantRow> = tx
        .query_row(
            "SELECT issuer,principal,subject_id,context_id,build_id,proof_run_id,scope,expires_ms,revision,revoked FROM grants WHERE grant_id=?1",
            [grant.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?)),
        )
        .optional()
        .map_err(|_| StoreError::Unavailable)?;
    row.map(
        |(issuer, principal, subject, context, build, proof, scope, expires, revision, revoked)| {
            Ok(TrustedGrant {
                id: grant.clone(),
                issuer,
                principal,
                subject: SubjectId::parse(subject).map_err(|_| StoreError::Unavailable)?,
                context: ContextGeneration::parse(context).map_err(|_| StoreError::Unavailable)?,
                build: BuildId::parse(build).map_err(|_| StoreError::Unavailable)?,
                proof_run: ProofRunId::parse(proof).map_err(|_| StoreError::Unavailable)?,
                scope: GrantScope::parse(&scope).ok_or(StoreError::Unavailable)?,
                expires: AuthorityExpiresAt::new(WallTimeMs::new(expires)),
                revision: from_sql_revision(revision)?,
                revoked,
            })
        },
    )
    .transpose()
}

fn verified_input(
    input: &str,
    text: String,
    expected_hex: &str,
) -> Result<ControlledTextInput, StoreError> {
    use work_engine_types::CodecContract;
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .map_err(|_| StoreError::Unavailable)?;
    if digest.hex() != expected_hex {
        return Err(StoreError::Unavailable);
    }
    ControlledTextInput::new(
        InputId::parse(input).map_err(|_| StoreError::Unavailable)?,
        text,
        digest,
    )
    .map_err(|_| StoreError::Unavailable)
}

fn to_sql_revision(value: Revision) -> Result<i64, StoreError> {
    i64::try_from(value.get()).map_err(|_| StoreError::Rejected)
}

fn from_sql_revision(value: i64) -> Result<Revision, StoreError> {
    Ok(Revision::new(
        u64::try_from(value).map_err(|_| StoreError::Unavailable)?,
    ))
}

fn encode_outcome(outcome: &CommandOutcome) -> (&'static str, Option<&str>, Option<&'static str>) {
    match outcome {
        CommandOutcome::Enqueued { delivery, .. } => ("enqueued", Some(delivery.as_str()), None),
        CommandOutcome::ReplacementRequested { transition } => {
            ("replacement_requested", Some(transition.as_str()), None)
        }
        CommandOutcome::InterruptionRequested { attempt } => {
            ("interruption_requested", Some(attempt.as_str()), None)
        }
        CommandOutcome::Rejected(code) => ("rejected", None, Some(code.as_str())),
    }
}

fn decode_result(
    tx: &Transaction<'_>,
    kind: &str,
    outcome_ref: Option<String>,
    rejection: Option<String>,
    revision: i64,
) -> Result<CommandResult, StoreError> {
    let outcome = match kind {
        "enqueued" => {
            let delivery = DeliveryId::parse(outcome_ref.ok_or(StoreError::Unavailable)?)
                .map_err(|_| StoreError::Unavailable)?;
            let input: String = tx
                .query_row(
                    "SELECT input_id FROM inputs WHERE delivery_id=?1",
                    [delivery.as_str()],
                    |row| row.get(0),
                )
                .map_err(|_| StoreError::Unavailable)?;
            CommandOutcome::Enqueued {
                input: InputId::parse(input).map_err(|_| StoreError::Unavailable)?,
                delivery,
            }
        }
        "rejected" => CommandOutcome::Rejected(
            RejectionCode::parse(&rejection.ok_or(StoreError::Unavailable)?)
                .ok_or(StoreError::Unavailable)?,
        ),
        "replacement_requested" => CommandOutcome::ReplacementRequested {
            transition: TransitionId::parse(outcome_ref.ok_or(StoreError::Unavailable)?)
                .map_err(|_| StoreError::Unavailable)?,
        },
        "interruption_requested" => CommandOutcome::InterruptionRequested {
            attempt: AttemptId::parse(outcome_ref.ok_or(StoreError::Unavailable)?)
                .map_err(|_| StoreError::Unavailable)?,
        },
        _ => return Err(StoreError::Unavailable),
    };
    Ok(CommandResult {
        outcome,
        revision: from_sql_revision(revision)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifecycle_core::{CommandId, CommandRequest, WaitBudgetMs};
    use serde_json::json;
    use tempfile::tempdir;
    use work_engine_types::CodecContract;

    #[test]
    fn sqlite_full_rolls_back_command_result_custody_and_journal_together() {
        let root = tempdir().unwrap();
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        let clock = ClockSample {
            wall: WallTimeMs::new(101),
            wait_budget: WaitBudgetMs::new(5),
        };
        let subject = SubjectId::parse("subject-full").unwrap();
        let context = ContextGeneration::parse("context-full").unwrap();
        let build = BuildId::parse("build-full").unwrap();
        let proof = ProofRunId::parse("proof-full").unwrap();
        let grant = GrantId::parse("grant-full").unwrap();
        store
            .register_subject(
                subject.clone(),
                context.clone(),
                build.clone(),
                proof.clone(),
                clock,
            )
            .unwrap();
        store
            .install_trusted_grant(TrustedGrant {
                id: grant.clone(),
                issuer: "trusted".into(),
                principal: "caller".into(),
                subject: subject.clone(),
                context: context.clone(),
                build: build.clone(),
                proof_run: proof.clone(),
                scope: GrantScope::EnqueueInput,
                expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
        let input = InputId::parse("input-full").unwrap();
        let text = "x".repeat(512 * 1024);
        let text_digest = CodecContract::LifecycleTextInputV1
            .digest_binary(text.as_bytes())
            .unwrap();
        let basis = json!({
            "protocol_version":1,"principal_ref":"caller","command_id":"command-full",
            "subject_id":subject.as_str(),"context_generation":context.as_str(),
            "build_id":build.as_str(),"proof_run_id":proof.as_str(),"grant_ref":grant.as_str(),
            "expected_revision":"0","kind":"enqueue_input",
            "payload":{"input_id":input.as_str(),"producer_ref":"fixture:full",
                "text":text,"text_digest":text_digest.hex()}
        });
        let request = CommandRequest::new(
            CommandId::parse("command-full").unwrap(),
            subject,
            context,
            build,
            proof,
            grant,
            Revision::new(0),
            CodecContract::LifecycleCommandV1
                .digest_json(&basis)
                .unwrap(),
            Command::EnqueueInput {
                input: input.clone(),
            },
        )
        .unwrap();
        let admission = CommandAdmission::new(
            request,
            "caller".into(),
            basis,
            Some(ControlledTextInput::new(input, text, text_digest).unwrap()),
        )
        .unwrap();
        let journal_before = store.load_recovery().unwrap().journal_cursor;
        let current_pages: i64 = store
            .connection
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .unwrap();
        store
            .connection
            .pragma_update(None, "max_page_count", current_pages)
            .unwrap();
        assert!(matches!(
            store.apply_checked_command(admission.clone(), clock),
            Err(StoreError::Unavailable)
        ));
        assert_eq!(store.counts().unwrap(), (0, 0, 0));
        assert_eq!(
            store.load_recovery().unwrap().journal_cursor,
            journal_before
        );
        store
            .connection
            .pragma_update(None, "max_page_count", 4_294_967_294_i64)
            .unwrap();
        assert!(matches!(
            store
                .apply_checked_command(admission, clock)
                .unwrap()
                .outcome,
            CommandOutcome::Enqueued { .. }
        ));
        assert_eq!(store.counts().unwrap(), (1, 1, 0));
    }
}
