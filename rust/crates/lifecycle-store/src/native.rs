//! S5 native parent/child custody in the sole lifecycle SQLite writer.
//! Exact entered work is never a retry invitation after a crash or lost RPC.

use lifecycle_core::{
    AttemptId, ClockSample, ContextGeneration, GrantId, NativeInvocationPurpose, NativeSessionId,
    NativeToolCallId, OperationAttemptId, OperationContractId, OperationExecutorId,
    OperationImplementationId, ProviderThreadId, ProviderTurnId, RuntimeIncarnation, SnapshotId,
    SubjectId,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use work_engine_types::CodecContract;

use crate::{SqliteLifecycleStore, StoreError};

const SNAPSHOT_CONTRACT: &str = "workspace.snapshot.read.v1";
const SNAPSHOT_IMPLEMENTATION: &str = "rust.snapshot.read.v1";
const SNAPSHOT_EXECUTOR: &str = "lifecycle.service.snapshot";
const MAX_RESULT: usize = 65_536;
type PriorNativeSessionRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
);
type NativeIngressRow = (String, Option<String>, Option<String>, Vec<u8>);
type PriorNativeOperationRow = (
    String,
    String,
    Option<Vec<u8>>,
    Option<String>,
    String,
    i64,
    String,
    i64,
    String,
    String,
    String,
    String,
    String,
    i64,
    i64,
);
type NativeGrantRow = (
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

struct GrantExpectation<'a> {
    issuer: &'a str,
    principal: &'a str,
    grant: &'a GrantId,
    revision: u64,
    subject: &'a str,
    context: &'a str,
    scope: &'a str,
    clock: ClockSample,
    successor_rehydrate: bool,
}

fn sha256(bytes: &[u8]) -> Result<String, StoreError> {
    Ok(CodecContract::BinaryArtifactV1
        .digest_binary(bytes)
        .map_err(|_| StoreError::Rejected)?
        .hex())
}

#[must_use = "native allocation intent must be bound or retained as uncertain"]
#[derive(Debug)]
pub struct NativeSessionEntry {
    id: NativeSessionId,
    subject: SubjectId,
    context: ContextGeneration,
    profile_digest: String,
    snapshot: SnapshotId,
    thread: Option<ProviderThreadId>,
}

impl NativeSessionEntry {
    pub fn id(&self) -> &NativeSessionId {
        &self.id
    }
    pub fn thread(&self) -> Option<&ProviderThreadId> {
        self.thread.as_ref()
    }
    pub fn subject(&self) -> &SubjectId {
        &self.subject
    }
    pub fn context(&self) -> &ContextGeneration {
        &self.context
    }
    pub fn snapshot(&self) -> &SnapshotId {
        &self.snapshot
    }
}

#[derive(Debug)]
pub struct NativeTurnRequest {
    pub invocation_id: String,
    pub attempt: AttemptId,
    pub grant: GrantId,
    pub grant_revision: u64,
    pub purpose: NativeInvocationPurpose,
    pub prompt: Vec<u8>,
}

#[must_use = "entered native turn must be settled or retained as uncertain"]
#[derive(Debug)]
pub struct NativeTurnEntry {
    invocation_id: String,
    attempt: AttemptId,
    thread: ProviderThreadId,
    turn: Option<ProviderTurnId>,
    profile_digest: String,
}

impl NativeTurnEntry {
    pub fn invocation_id(&self) -> &str {
        &self.invocation_id
    }
    pub fn attempt(&self) -> &AttemptId {
        &self.attempt
    }
    pub fn thread(&self) -> &ProviderThreadId {
        &self.thread
    }
    pub fn turn(&self) -> Option<&ProviderTurnId> {
        self.turn.as_ref()
    }
    pub fn profile_digest(&self) -> &str {
        &self.profile_digest
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeIngressApply {
    Inserted,
    Duplicate,
    ConflictFenced,
}

#[derive(Debug)]
pub struct SnapshotReadRequest {
    pub attempt: OperationAttemptId,
    pub call: NativeToolCallId,
    pub transport_session: String,
    pub sequence: u64,
    pub grant: GrantId,
    pub grant_revision: u64,
    pub contract: OperationContractId,
    pub implementation: OperationImplementationId,
    pub executor: OperationExecutorId,
    pub snapshot: SnapshotId,
    pub member_id: String,
    pub range_start: u64,
    pub range_length: u64,
}

#[must_use = "entered child operation must be settled or retained as uncertain"]
#[derive(Debug)]
pub struct NativeOperationEntry {
    attempt: OperationAttemptId,
    parent_invocation_id: String,
    call: NativeToolCallId,
    snapshot: SnapshotId,
    member_id: String,
    range_start: u64,
    range_length: u64,
}

impl NativeOperationEntry {
    pub fn attempt(&self) -> &OperationAttemptId {
        &self.attempt
    }
    pub fn call(&self) -> &NativeToolCallId {
        &self.call
    }
    pub fn snapshot(&self) -> &SnapshotId {
        &self.snapshot
    }
    pub fn member_id(&self) -> &str {
        &self.member_id
    }
    pub fn range(&self) -> (u64, u64) {
        (self.range_start, self.range_length)
    }
    pub fn parent_invocation_id(&self) -> &str {
        &self.parent_invocation_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeOperationResult {
    pub attempt: OperationAttemptId,
    pub call: NativeToolCallId,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

#[derive(Debug)]
pub enum NativeOperationClaim {
    Entered(NativeOperationEntry),
    Committed(NativeOperationResult),
    EnteredUncertain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeRecovery {
    pub class: String,
    pub session_id: String,
    pub attempt_id: String,
    pub purpose: String,
    pub grant_id: String,
    pub grant_revision: u64,
    pub send_intent: bool,
    pub allocation_state: String,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub profile_digest: String,
    pub binary_sha256: String,
    pub protocol_digest: String,
    pub snapshot_id: String,
    pub snapshot_manifest_sha256: String,
    pub admitted_children: u64,
    pub unsettled_children: u64,
    pub children: Vec<NativeOperationRecovery>,
    pub final_sha256: Option<String>,
    pub history_status: String,
    pub history_transport_session: Option<String>,
    pub history_sequence: Option<u64>,
    pub process_outcome: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeOperationRecovery {
    pub attempt_id: String,
    pub call_id: String,
    pub class: String,
    pub request_transport_session: String,
    pub request_sequence: u64,
    pub contract_id: String,
    pub implementation_id: String,
    pub executor_id: String,
    pub snapshot_id: String,
    pub member_id: String,
    pub range_start: u64,
    pub range_length: u64,
    pub result_sha256: Option<String>,
}

/// Bounded raw transport evidence. A page is an observation, never proof that
/// an absent emission or a missing turn did not occur.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeIngressPage {
    pub version: u16,
    pub transport_session: String,
    pub after_sequence: u64,
    pub events: Vec<NativeIngressRecord>,
    pub next_after_sequence: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeIngressRecord {
    pub sequence: u64,
    pub raw_sha256: String,
    pub raw: Vec<u8>,
    pub decode_status: String,
    pub method: Option<String>,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub observed_wall_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeFinalMaterial {
    pub version: u16,
    pub invocation_id: String,
    pub attempt_id: String,
    pub purpose: String,
    pub thread_id: String,
    pub turn_id: String,
    pub final_text: Vec<u8>,
    pub final_sha256: String,
    pub history_transport_session: String,
    pub history_sequence: u64,
    pub history_observed_wall_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeSessionRecovery {
    pub version: u16,
    pub session_id: String,
    pub subject_id: String,
    pub context_id: String,
    pub incarnation_id: String,
    pub allocation_state: String,
    pub thread_id: Option<String>,
    pub process_outcome: Option<String>,
}

impl SqliteLifecycleStore {
    pub fn native_session_recovery(
        &self,
        session_id: &NativeSessionId,
    ) -> Result<Option<NativeSessionRecovery>, StoreError> {
        self.verify_identity()?;
        self.connection.query_row(
            "SELECT s.subject_id,s.context_id,s.incarnation_id,s.allocation_state,s.thread_id,x.outcome_kind FROM native_sessions s LEFT JOIN native_process_exits x ON x.session_id=s.session_id WHERE s.session_id=?1",
            [session_id.as_str()], |r| Ok(NativeSessionRecovery {
                version: 1, session_id: session_id.as_str().into(), subject_id: r.get(0)?, context_id: r.get(1)?,
                incarnation_id: r.get(2)?, allocation_state: r.get(3)?, thread_id: r.get(4)?, process_outcome: r.get(5)?,
            })
        ).optional().map_err(|_| StoreError::Unavailable)
    }

    pub fn native_ingress_page(
        &self,
        transport_session: &str,
        after_sequence: u64,
        limit: u16,
    ) -> Result<NativeIngressPage, StoreError> {
        self.verify_identity()?;
        if transport_session.is_empty()
            || limit == 0
            || limit > 256
            || after_sequence > i64::MAX as u64
        {
            return Err(StoreError::Rejected);
        }
        let mut statement = self.connection.prepare(
            "SELECT sequence,raw_sha256,raw,decode_status,method,thread_id,turn_id,observed_wall_ms FROM native_ingress WHERE transport_session=?1 AND sequence>?2 ORDER BY sequence LIMIT ?3"
        ).map_err(|_| StoreError::Unavailable)?;
        let events = statement
            .query_map(
                params![
                    transport_session,
                    after_sequence as i64,
                    i64::from(limit) + 1
                ],
                |r| {
                    Ok(NativeIngressRecord {
                        sequence: r.get::<_, i64>(0)? as u64,
                        raw_sha256: r.get(1)?,
                        raw: r.get(2)?,
                        decode_status: r.get(3)?,
                        method: r.get(4)?,
                        thread_id: r.get(5)?,
                        turn_id: r.get(6)?,
                        observed_wall_ms: r.get(7)?,
                    })
                },
            )
            .map_err(|_| StoreError::Unavailable)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StoreError::Unavailable)?;
        let mut events = events;
        let more = events.len() > usize::from(limit);
        if more {
            events.pop();
        }
        let next_after_sequence = if more {
            events.last().map(|event| event.sequence)
        } else {
            None
        };
        Ok(NativeIngressPage {
            version: 1,
            transport_session: transport_session.into(),
            after_sequence,
            events,
            next_after_sequence,
        })
    }

    pub fn native_final_material(
        &self,
        invocation_id: &str,
    ) -> Result<Option<NativeFinalMaterial>, StoreError> {
        self.verify_identity()?;
        self.connection.query_row(
            "SELECT t.attempt_id,t.purpose,t.thread_id,t.turn_id,t.final_text,t.final_sha256,t.history_transport_session,t.history_sequence,i.observed_wall_ms \
             FROM native_turns t JOIN native_ingress i ON i.transport_session=t.history_transport_session AND i.sequence=t.history_sequence \
             WHERE t.invocation_id=?1 AND t.state='completed_settled' AND t.final_text IS NOT NULL",
            [invocation_id], |r| Ok(NativeFinalMaterial {
                version: 1, invocation_id: invocation_id.into(), attempt_id: r.get(0)?, purpose: r.get(1)?,
                thread_id: r.get(2)?, turn_id: r.get(3)?, final_text: r.get(4)?, final_sha256: r.get(5)?,
                history_transport_session: r.get(6)?, history_sequence: r.get::<_,i64>(7)? as u64,
                history_observed_wall_ms: r.get(8)?,
            })
        ).optional().map_err(|_| StoreError::Unavailable)
    }

    /// Only an actually bound successor thread can satisfy S4 observation.
    pub fn native_successor_thread(
        &self,
        subject: &SubjectId,
        context: &ContextGeneration,
    ) -> Result<Option<ProviderThreadId>, StoreError> {
        self.verify_identity()?;
        let row: Option<String> = self.connection.query_row(
            "SELECT thread_id FROM native_sessions WHERE subject_id=?1 AND context_id=?2 AND allocation_state='bound' AND thread_id IS NOT NULL",
            params![subject.as_str(),context.as_str()], |r| r.get(0),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        row.map(|value| ProviderThreadId::parse(value).map_err(|_| StoreError::Unavailable))
            .transpose()
    }

    /// Settlement already verified the exact prompt in the successor's full
    /// observed user-message history. This is a mechanical source receipt.
    pub fn native_rehydration_receipt(
        &self,
        transition: &lifecycle_core::TransitionId,
    ) -> Result<Option<String>, StoreError> {
        self.verify_identity()?;
        self.connection.query_row(
            "SELECT 'native:'||r.invocation_id FROM transitions tr \
             JOIN native_turns p ON tr.native_basis_source=p.invocation_id \
             JOIN native_sessions s ON s.subject_id=tr.subject_id AND s.context_id=tr.successor_context \
             JOIN native_turns r ON r.session_id=s.session_id AND r.thread_id=tr.successor_thread \
             WHERE tr.transition_id=?1 AND tr.stage='rehydrating' AND r.purpose='successor_rehydrate' \
             AND r.state='completed_settled' AND r.prompt=p.final_text AND r.history_status='observed_complete' \
             ORDER BY r.settled_wall_ms DESC,r.rowid DESC LIMIT 1",
            [transition.as_str()], |r| r.get(0),
        ).optional().map_err(|_|StoreError::Unavailable)
    }
    /// A unique generation allocation intent is durable before thread/start.
    /// An existing unbound intent is uncertainty, never permission to allocate again.
    #[allow(clippy::too_many_arguments)]
    pub fn reserve_native_session(
        &mut self,
        id: NativeSessionId,
        subject: SubjectId,
        context: ContextGeneration,
        incarnation: RuntimeIncarnation,
        profile_digest: &str,
        binary_sha256: &str,
        protocol_digest: &str,
        snapshot: SnapshotId,
        manifest_sha256: &str,
        same_session_required: bool,
        clock: ClockSample,
    ) -> Result<NativeSessionEntry, StoreError> {
        self.verify_identity()?;
        if same_session_required
            || [
                profile_digest,
                binary_sha256,
                protocol_digest,
                manifest_sha256,
            ]
            .iter()
            .any(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err(StoreError::Rejected);
        }
        let registered: Option<(String, String, Option<String>)> = self
            .connection
            .query_row(
                "SELECT context_id,owner_kind,owner_ref FROM subjects WHERE subject_id=?1",
                [subject.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        let admitted_domain = registered
            .as_ref()
            .is_some_and(|(c, o, owner)| c == context.as_str() && o == "domain" && owner.is_none());
        let admitted_successor = if let Some((_, owner, Some(transition))) = registered.as_ref() {
            if owner != "transition" {
                false
            } else {
                let basis: Option<(String,Option<String>)> = self.connection.query_row(
                    "SELECT stage,native_basis_source FROM transitions WHERE transition_id=?1 AND subject_id=?2",
                    params![transition,subject.as_str()], |r| Ok((r.get(0)?,r.get(1)?)),
                ).optional().map_err(|_|StoreError::Unavailable)?;
                if let Some((stage, Some(source))) = basis {
                    if stage != "switching" {
                        false
                    } else {
                        let final_text: Option<Vec<u8>> = self.connection.query_row(
                            "SELECT final_text FROM native_turns WHERE invocation_id=?1 AND state='completed_settled' AND purpose='domain_work'",
                            [source], |r| r.get(0),
                        ).optional().map_err(|_|StoreError::Unavailable)?;
                        final_text
                            .and_then(|bytes| {
                                serde_json::from_slice::<serde_json::Value>(&bytes).ok()
                            })
                            .is_some_and(|value| value["successor_context"] == context.as_str())
                    }
                } else {
                    false
                }
            }
        } else {
            false
        };
        if !admitted_domain && !admitted_successor {
            return Err(StoreError::Rejected);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let prior: Option<PriorNativeSessionRow> = tx.query_row(
            "SELECT session_id,incarnation_id,profile_digest,binary_sha256,protocol_digest,snapshot_id,snapshot_manifest_sha256,thread_id,allocation_state FROM native_sessions WHERE subject_id=?1 AND context_id=?2",
            params![subject.as_str(),context.as_str()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?)))
            .optional().map_err(|_| StoreError::Unavailable)?;
        if let Some((old_id, old_inc, p, b, proto, snap, manifest, thread, state)) = prior {
            if old_id != id.as_str()
                || old_inc != incarnation.as_str()
                || p != profile_digest
                || b != binary_sha256
                || proto != protocol_digest
                || snap != snapshot.as_str()
                || manifest != manifest_sha256
                || state == "conflicted"
            {
                return Err(StoreError::ClaimConflict);
            }
            let Some(thread) = thread else {
                return Err(StoreError::ClaimConflict);
            };
            return Ok(NativeSessionEntry {
                id,
                subject,
                context,
                profile_digest: p,
                snapshot,
                thread: Some(ProviderThreadId::parse(thread).map_err(|_| StoreError::Unavailable)?),
            });
        }
        let last_wall: i64 = tx
            .query_row(
                "SELECT last_wall_ms FROM subjects WHERE subject_id=?1",
                [subject.as_str()],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Rejected)?;
        if clock.wall.get() < last_wall {
            crate::sqlite::fence_subject(&tx, subject.as_str(), "clock:rollback", clock)?;
            tx.commit().map_err(|_| StoreError::Unavailable)?;
            return Err(StoreError::Rejected);
        }
        tx.execute("INSERT INTO native_sessions(session_id,subject_id,context_id,incarnation_id,profile_digest,binary_sha256,protocol_digest,snapshot_id,snapshot_manifest_sha256,allocation_state,created_wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,'intent',?10)",
            params![id.as_str(),subject.as_str(),context.as_str(),incarnation.as_str(),profile_digest,binary_sha256,protocol_digest,snapshot.as_str(),manifest_sha256,clock.wall.get()])
            .map_err(|_| StoreError::Unavailable)?;
        observe_native_subject_clock(&tx, subject.as_str(), clock.wall.get())?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(NativeSessionEntry {
            id,
            subject,
            context,
            profile_digest: profile_digest.into(),
            snapshot,
            thread: None,
        })
    }

    pub fn bind_native_thread(
        &mut self,
        entry: &mut NativeSessionEntry,
        thread: ProviderThreadId,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let changed=self.connection.execute("UPDATE native_sessions SET thread_id=?2,allocation_state='bound' WHERE session_id=?1 AND allocation_state='intent' AND thread_id IS NULL",
            params![entry.id.as_str(),thread.as_str()]).map_err(|_| StoreError::Unavailable)?;
        if changed != 1 {
            return Err(StoreError::ClaimConflict);
        }
        entry.thread = Some(thread);
        Ok(())
    }

    pub fn claim_native_turn(
        &mut self,
        session: &NativeSessionEntry,
        request: NativeTurnRequest,
        principal: &str,
        clock: ClockSample,
    ) -> Result<NativeTurnEntry, StoreError> {
        self.verify_identity()?;
        let thread = session.thread.as_ref().ok_or(StoreError::ClaimConflict)?;
        if request.prompt.is_empty()
            || request.prompt.len() > 2048
            || std::str::from_utf8(&request.prompt).is_err()
        {
            return Err(StoreError::Rejected);
        }
        let mut tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        if request.purpose == NativeInvocationPurpose::SuccessorRehydrate {
            let source_text: Vec<u8> = tx.query_row(
                "SELECT n.final_text FROM subjects sub JOIN transitions tr ON tr.transition_id=sub.owner_ref \
                 JOIN native_turns n ON tr.native_basis_source=n.invocation_id \
                 WHERE sub.subject_id=?1 AND sub.owner_kind='transition' AND tr.stage='rehydrating' \
                 AND tr.successor_context=?2 AND tr.successor_thread=?3 AND n.state='completed_settled'",
                params![session.subject.as_str(),session.context.as_str(),thread.as_str()], |r| r.get(0),
            ).map_err(|_|StoreError::Rejected)?;
            if request.prompt != source_text {
                return Err(StoreError::Rejected);
            }
        }
        let grant_check = check_grant(
            &tx,
            GrantExpectation {
                issuer: &self.trusted_issuer,
                principal,
                grant: &request.grant,
                revision: request.grant_revision,
                subject: session.subject.as_str(),
                context: session.context.as_str(),
                scope: "native_turn",
                clock,
                successor_rehydrate: request.purpose == NativeInvocationPurpose::SuccessorRehydrate,
            },
        );
        if let Err(reason) = grant_check {
            if matches!(reason, StoreError::Rejected) {
                tx.commit().map_err(|_| StoreError::Unavailable)?;
            }
            return Err(reason);
        }
        // Once a bound grant has observed the wall clock, a later claim conflict
        // must not discard that observation. Keep the entry attempt in a
        // savepoint so every failed exit commits only the subject clock.
        let mut entry = tx.savepoint().map_err(|_| StoreError::Unavailable)?;
        let claim = (|| {
            let active:i64=entry.query_row("SELECT COUNT(*) FROM native_turns WHERE session_id=?1 AND state='entered_uncertain'",
                [session.id.as_str()],|r|r.get(0)).map_err(|_| StoreError::Unavailable)?;
            if active != 0 {
                return Err(StoreError::ClaimConflict);
            }
            let prompt_sha256 = sha256(&request.prompt)?;
            entry.execute("INSERT INTO native_turns(invocation_id,session_id,attempt_id,grant_id,grant_revision,purpose,prompt_sha256,prompt,profile_digest,state,thread_id,entered_wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,'entered_uncertain',?10,?11)",
                params![request.invocation_id,session.id.as_str(),request.attempt.as_str(),request.grant.as_str(),request.grant_revision,request.purpose.as_str(),prompt_sha256,request.prompt,session.profile_digest,thread.as_str(),clock.wall.get()])
                .map_err(|_| StoreError::ClaimConflict)?;
            Ok(NativeTurnEntry {
                invocation_id: request.invocation_id,
                attempt: request.attempt,
                thread: thread.clone(),
                turn: None,
                profile_digest: session.profile_digest.clone(),
            })
        })();
        if claim.is_err() {
            entry.rollback().map_err(|_| StoreError::Unavailable)?;
        }
        entry.commit().map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        claim
    }

    /// Persist send intent before turn/start bytes; a lost response remains no-resend.
    pub fn mark_native_send_intent(&mut self, entry: &NativeTurnEntry) -> Result<(), StoreError> {
        self.verify_identity()?;
        let n=self.connection.execute("UPDATE native_turns SET send_intent=1 WHERE invocation_id=?1 AND send_intent=0 AND state='entered_uncertain'",
            [entry.invocation_id.as_str()]).map_err(|_| StoreError::Unavailable)?;
        if n != 1 {
            return Err(StoreError::ClaimConflict);
        }
        Ok(())
    }

    pub fn bind_native_turn(
        &mut self,
        entry: &mut NativeTurnEntry,
        turn: ProviderTurnId,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let n=self.connection.execute("UPDATE native_turns SET turn_id=?2 WHERE invocation_id=?1 AND send_intent=1 AND turn_id IS NULL AND state='entered_uncertain'",
            params![entry.invocation_id.as_str(),turn.as_str()]).map_err(|_| StoreError::Unavailable)?;
        if n != 1 {
            return Err(StoreError::ClaimConflict);
        }
        entry.turn = Some(turn);
        Ok(())
    }

    /// One ingress owner assigns transport-session + emission sequence before
    /// fan-out. Replaying that coordinate is idempotent; equal bytes at a new
    /// sequence are separate provider emissions.
    #[allow(clippy::too_many_arguments)]
    pub fn record_native_ingress(
        &mut self,
        session: &NativeSessionId,
        transport: &str,
        sequence: u64,
        raw: &[u8],
        decode_status: &str,
        method: Option<&str>,
        thread: Option<&str>,
        turn: Option<&str>,
        item: Option<&str>,
        clock: ClockSample,
    ) -> Result<NativeIngressApply, StoreError> {
        self.verify_identity()?;
        if transport.is_empty()
            || sequence == 0
            || raw.is_empty()
            || raw.len() > 1_048_576
            || !matches!(
                decode_status,
                "qualified" | "unavailable" | "unsupported" | "malformed" | "denied_before_entry"
            )
        {
            return Err(StoreError::Rejected);
        }
        let digest = sha256(raw)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let subject: String = tx
            .query_row(
                "SELECT subject_id FROM native_sessions WHERE session_id=?1",
                [session.as_str()],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::ClaimConflict)?;
        let prior:Option<(String,Vec<u8>)>=tx.query_row("SELECT raw_sha256,raw FROM native_ingress WHERE transport_session=?1 AND sequence=?2",
            params![transport,sequence],|r|Ok((r.get(0)?,r.get(1)?)))
            .optional().map_err(|_| StoreError::Unavailable)?;
        let outcome = if let Some((old_digest, old_raw)) = prior {
            if old_digest == digest && old_raw == raw {
                NativeIngressApply::Duplicate
            } else {
                tx.execute("INSERT OR IGNORE INTO native_ingress_conflicts(transport_session,sequence,raw_sha256,raw,observed_wall_ms) VALUES (?1,?2,?3,?4,?5)",
                    params![transport,sequence,digest,raw,clock.wall.get()]).map_err(|_| StoreError::Unavailable)?;
                if let (Some(thread), Some(turn)) = (thread, turn) {
                    tx.execute("UPDATE native_turns SET state='blocked_conflicted' WHERE thread_id=?1 AND turn_id=?2",
                        params![thread,turn]).map_err(|_| StoreError::Unavailable)?;
                }
                NativeIngressApply::ConflictFenced
            }
        } else {
            tx.execute("INSERT INTO native_ingress(transport_session,sequence,raw_sha256,raw,decode_status,method,thread_id,turn_id,item_id,observed_wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![transport,sequence,digest,raw,decode_status,method,thread,turn,item,clock.wall.get()])
                .map_err(|_| StoreError::Unavailable)?;
            NativeIngressApply::Inserted
        };
        // The ingress owner supplies its store-minted native session even for
        // startup notices and history responses without a thread/turn field.
        observe_native_subject_clock(&tx, &subject, clock.wall.get())?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(outcome)
    }

    pub fn claim_snapshot_read(
        &mut self,
        parent: &NativeTurnEntry,
        request: SnapshotReadRequest,
        principal: &str,
        clock: ClockSample,
    ) -> Result<NativeOperationClaim, StoreError> {
        self.verify_identity()?;
        let turn = parent.turn.as_ref().ok_or(StoreError::ClaimConflict)?;
        if request.contract.as_str() != SNAPSHOT_CONTRACT
            || request.implementation.as_str() != SNAPSHOT_IMPLEMENTATION
            || request.executor.as_str() != SNAPSHOT_EXECUTOR
            || request.snapshot.as_str().is_empty()
            || request.member_id.is_empty()
            || request.member_id.len() > 128
            || request.range_length == 0
            || request.range_length > MAX_RESULT as u64
            || request
                .range_start
                .checked_add(request.range_length)
                .is_none()
            || [
                request.sequence,
                request.grant_revision,
                request.range_start,
                request.range_length,
            ]
            .iter()
            .any(|value| *value > i64::MAX as u64)
        {
            return Err(StoreError::Rejected);
        }
        let manifest_digest:String=self.connection.query_row(
            "SELECT s.snapshot_manifest_sha256 FROM native_turns t JOIN native_sessions s ON t.session_id=s.session_id WHERE t.invocation_id=?1 AND t.thread_id=?2 AND t.turn_id=?3",
            params![parent.invocation_id,parent.thread.as_str(),turn.as_str()],|r|r.get(0))
            .map_err(|_|StoreError::ClaimConflict)?;
        let manifest_bytes = self
            .read_committed_artifact(&manifest_digest)?
            .ok_or(StoreError::Unavailable)?;
        let manifest: serde_json::Value =
            serde_json::from_slice(&manifest_bytes).map_err(|_| StoreError::Unavailable)?;
        let member = manifest["members"]
            .as_array()
            .and_then(|members| members.iter().find(|m| m["id"] == request.member_id))
            .ok_or(StoreError::Rejected)?;
        let size = member["length"].as_u64().ok_or(StoreError::Unavailable)?;
        if manifest["codec"] != "workspace-snapshot-manifest-v1"
            || manifest["id"] != request.snapshot.as_str()
            || member["sha256"].as_str().is_none_or(|s| s.len() != 64)
            || request
                .range_start
                .checked_add(request.range_length)
                .is_none_or(|end| end > size)
        {
            return Err(StoreError::Rejected);
        }
        let mut tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let (session_snapshot,subject,context):(String,String,String)=tx.query_row("SELECT s.snapshot_id,s.subject_id,s.context_id FROM native_turns t JOIN native_sessions s ON t.session_id=s.session_id WHERE t.invocation_id=?1 AND t.thread_id=?2 AND t.turn_id=?3 AND t.state='entered_uncertain'",
            params![parent.invocation_id, parent.thread.as_str(),turn.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))
            .map_err(|_| StoreError::ClaimConflict)?;
        if session_snapshot != request.snapshot.as_str() {
            return Err(StoreError::ClaimConflict);
        }
        let ingress:Option<NativeIngressRow>=tx.query_row(
            "SELECT decode_status,thread_id,turn_id,raw FROM native_ingress WHERE transport_session=?1 AND sequence=?2 AND method='item/tool/call'",
            params![request.transport_session,request.sequence],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))
            .optional().map_err(|_| StoreError::Unavailable)?;
        let Some((status, Some(th), Some(tu), raw)) = ingress else {
            return Err(StoreError::ClaimConflict);
        };
        if status != "qualified" || th != parent.thread.as_str() || tu != turn.as_str() {
            return Err(StoreError::ClaimConflict);
        }
        let observed: serde_json::Value =
            serde_json::from_slice(&raw).map_err(|_| StoreError::ClaimConflict)?;
        let args = &observed["params"]["arguments"];
        if observed["method"] != "item/tool/call"
            || observed["params"]["callId"] != request.call.as_str()
            || observed["params"]["namespace"] != "workspace"
            || observed["params"]["tool"] != "snapshot_read"
            || args.as_object().is_none_or(|o| o.len() != 4)
            || args["snapshot_id"] != request.snapshot.as_str()
            || args["member_id"] != request.member_id
            || args["offset"].as_u64() != Some(request.range_start)
            || args["length"].as_u64() != Some(request.range_length)
        {
            return Err(StoreError::ClaimConflict);
        }
        let prior:Option<PriorNativeOperationRow>=tx.query_row(
            "SELECT operation_attempt_id,state,result,result_sha256,request_transport_session,request_sequence,grant_id,grant_revision,contract_id,implementation_id,executor_id,snapshot_id,member_id,range_start,range_length FROM native_operations WHERE parent_invocation_id=?1 AND call_id=?2",
            params![parent.invocation_id,request.call.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?,r.get(12)?,r.get(13)?,r.get(14)?)))
            .optional().map_err(|_| StoreError::Unavailable)?;
        if let Some((
            old_attempt,
            state,
            bytes,
            digest,
            old_transport,
            old_sequence,
            old_grant,
            old_revision,
            old_contract,
            old_implementation,
            old_executor,
            old_snapshot,
            old_member,
            old_start,
            old_length,
        )) = prior
        {
            if old_attempt != request.attempt.as_str()
                || old_transport != request.transport_session
                || old_sequence != request.sequence as i64
                || old_grant != request.grant.as_str()
                || old_revision != request.grant_revision as i64
                || old_contract != request.contract.as_str()
                || old_implementation != request.implementation.as_str()
                || old_executor != request.executor.as_str()
                || old_snapshot != request.snapshot.as_str()
                || old_member != request.member_id
                || old_start != request.range_start as i64
                || old_length != request.range_length as i64
            {
                return Err(StoreError::ClaimConflict);
            }
            return if state == "completed_settled" {
                Ok(NativeOperationClaim::Committed(NativeOperationResult {
                    attempt: request.attempt,
                    call: request.call,
                    bytes: bytes.ok_or(StoreError::Unavailable)?,
                    sha256: digest.ok_or(StoreError::Unavailable)?,
                }))
            } else {
                Ok(NativeOperationClaim::EnteredUncertain)
            };
        }
        let grant_check = check_grant(
            &tx,
            GrantExpectation {
                issuer: &self.trusted_issuer,
                principal,
                grant: &request.grant,
                revision: request.grant_revision,
                subject: &subject,
                context: &context,
                scope: "snapshot_read",
                clock,
                successor_rehydrate: false,
            },
        );
        if let Err(reason) = grant_check {
            if matches!(reason, StoreError::Rejected) {
                tx.commit().map_err(|_| StoreError::Unavailable)?;
            }
            return Err(reason);
        }
        // Authority has bound this request to the subject and sampled the
        // clock. Roll back only entry work on any subsequent failure.
        let mut entry = tx.savepoint().map_err(|_| StoreError::Unavailable)?;
        let claim = (|| {
            let active:i64=entry.query_row("SELECT COUNT(*) FROM native_operations WHERE parent_invocation_id=?1 AND state='entered_uncertain'",
                [parent.invocation_id.as_str()],|r|r.get(0)).map_err(|_| StoreError::Unavailable)?;
            let count: i64 = entry
                .query_row(
                    "SELECT COUNT(*) FROM native_operations WHERE parent_invocation_id=?1",
                    [parent.invocation_id.as_str()],
                    |r| r.get(0),
                )
                .map_err(|_| StoreError::Unavailable)?;
            if active != 0 || count >= 16 {
                return Err(StoreError::ClaimConflict);
            }
            entry.execute("INSERT INTO native_operations(operation_attempt_id,parent_invocation_id,call_id,request_transport_session,request_sequence,grant_id,grant_revision,contract_id,implementation_id,executor_id,snapshot_id,member_id,range_start,range_length,state,entered_wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,'entered_uncertain',?15)",
                params![request.attempt.as_str(),parent.invocation_id,request.call.as_str(),request.transport_session,request.sequence,request.grant.as_str(),request.grant_revision,request.contract.as_str(),request.implementation.as_str(),request.executor.as_str(),request.snapshot.as_str(),request.member_id,request.range_start,request.range_length,clock.wall.get()])
                .map_err(|_| StoreError::ClaimConflict)?;
            Ok(NativeOperationClaim::Entered(NativeOperationEntry {
                attempt: request.attempt,
                parent_invocation_id: parent.invocation_id.clone(),
                call: request.call,
                snapshot: request.snapshot,
                member_id: request.member_id,
                range_start: request.range_start,
                range_length: request.range_length,
            }))
        })();
        if claim.is_err() {
            entry.rollback().map_err(|_| StoreError::Unavailable)?;
        }
        entry.commit().map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        claim
    }

    pub fn commit_snapshot_result(
        &mut self,
        entry: &NativeOperationEntry,
        bytes: &[u8],
        clock: ClockSample,
    ) -> Result<NativeOperationResult, StoreError> {
        self.verify_identity()?;
        if bytes.len() > MAX_RESULT {
            return Err(StoreError::Rejected);
        }
        let digest = sha256(bytes)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let changed=tx.execute("UPDATE native_operations SET state='completed_settled',result=?2,result_sha256=?3,settled_wall_ms=?4 WHERE operation_attempt_id=?1 AND parent_invocation_id=?5 AND call_id=?6 AND state='entered_uncertain'",
            params![entry.attempt.as_str(),bytes,digest,clock.wall.get(),entry.parent_invocation_id,entry.call.as_str()])
            .map_err(|_| StoreError::Unavailable)?;
        if changed != 1 {
            return Err(StoreError::ClaimConflict);
        }
        let subject: String = tx.query_row(
            "SELECT s.subject_id FROM native_operations o JOIN native_turns t ON t.invocation_id=o.parent_invocation_id JOIN native_sessions s ON s.session_id=t.session_id WHERE o.operation_attempt_id=?1",
            [entry.attempt.as_str()], |r| r.get(0),
        ).map_err(|_| StoreError::Unavailable)?;
        observe_native_subject_clock(&tx, &subject, clock.wall.get())?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(NativeOperationResult {
            attempt: entry.attempt.clone(),
            call: entry.call.clone(),
            bytes: bytes.to_vec(),
            sha256: digest,
        })
    }

    pub fn settle_native_turn(
        &mut self,
        entry: &NativeTurnEntry,
        final_text: &str,
        history_status: &str,
        history_transport: &str,
        history_sequence: u64,
        clock: ClockSample,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let turn = entry.turn.as_ref().ok_or(StoreError::ClaimConflict)?;
        if !matches!(
            history_status,
            "observed_complete" | "observed_tool_omitted"
        ) || final_text.is_empty()
            || final_text.len() > 65_536
        {
            return Err(StoreError::Rejected);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let incomplete:i64=tx.query_row("SELECT COUNT(*) FROM native_operations WHERE parent_invocation_id=?1 AND state!='completed_settled'",
            [entry.invocation_id.as_str()],|r|r.get(0)).map_err(|_| StoreError::Unavailable)?;
        let calls:i64=tx.query_row("SELECT COUNT(*) FROM native_ingress WHERE method='item/tool/call' AND thread_id=?1 AND turn_id=?2",
            params![entry.thread.as_str(),turn.as_str()],|r|r.get(0)).map_err(|_| StoreError::Unavailable)?;
        let children: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM native_operations WHERE parent_invocation_id=?1",
                [entry.invocation_id.as_str()],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        let terminal:i64=tx.query_row("SELECT COUNT(*) FROM native_ingress WHERE method='turn/completed' AND thread_id=?1 AND turn_id=?2 AND decode_status='qualified'",
            params![entry.thread.as_str(),turn.as_str()],|r|r.get(0)).map_err(|_| StoreError::Unavailable)?;
        let adverse_terminal:i64=tx.query_row("SELECT COUNT(*) FROM native_ingress WHERE method IN ('turn/failed','turn/interrupted') AND thread_id=?1 AND turn_id=?2",
            params![entry.thread.as_str(),turn.as_str()],|r|r.get(0)).map_err(|_| StoreError::Unavailable)?;
        let unknown:i64=tx.query_row("SELECT COUNT(*) FROM native_ingress WHERE thread_id=?1 AND turn_id=?2 AND decode_status NOT IN ('qualified','denied_before_entry')",
            params![entry.thread.as_str(),turn.as_str()],|r|r.get(0)).map_err(|_| StoreError::Unavailable)?;
        if incomplete != 0
            || calls != children
            || terminal != 1
            || adverse_terminal != 0
            || unknown != 0
        {
            return Err(StoreError::ClaimConflict);
        }
        let started_items = {
            let mut statement=tx.prepare("SELECT raw FROM native_ingress WHERE method='item/started' AND thread_id=?1 AND turn_id=?2 ORDER BY sequence")
                .map_err(|_|StoreError::Unavailable)?;
            statement
                .query_map(params![entry.thread.as_str(), turn.as_str()], |r| {
                    r.get::<_, Vec<u8>>(0)
                })
                .map_err(|_| StoreError::Unavailable)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| StoreError::Unavailable)?
        };
        let mut started_calls = std::collections::BTreeSet::<String>::new();
        for raw in started_items {
            let value: serde_json::Value =
                serde_json::from_slice(&raw).map_err(|_| StoreError::ClaimConflict)?;
            let item = &value["params"]["item"];
            match item["type"].as_str() {
                Some("dynamicToolCall") => {
                    let call = item["id"].as_str().ok_or(StoreError::ClaimConflict)?;
                    if item["namespace"] != "workspace"
                        || item["tool"] != "snapshot_read"
                        || !started_calls.insert(call.into())
                    {
                        return Err(StoreError::ClaimConflict);
                    }
                }
                Some("agentMessage")
                    if item["questions"].is_null() && item["delivery"].is_null() => {}
                Some("userMessage" | "reasoning") => {}
                _ => return Err(StoreError::ClaimConflict),
            }
        }
        let completed_items = {
            let mut statement=tx.prepare("SELECT raw FROM native_ingress WHERE method='item/completed' AND thread_id=?1 AND turn_id=?2 ORDER BY sequence")
                .map_err(|_|StoreError::Unavailable)?;
            statement
                .query_map(params![entry.thread.as_str(), turn.as_str()], |r| {
                    r.get::<_, Vec<u8>>(0)
                })
                .map_err(|_| StoreError::Unavailable)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| StoreError::Unavailable)?
        };
        let mut final_seen = 0u64;
        let mut tool_completions = std::collections::BTreeMap::<String, serde_json::Value>::new();
        for raw in completed_items {
            let value: serde_json::Value =
                serde_json::from_slice(&raw).map_err(|_| StoreError::ClaimConflict)?;
            let item = &value["params"]["item"];
            match item["type"].as_str() {
                Some("agentMessage")
                    if item["text"] == final_text
                        && item["questions"].is_null()
                        && item["delivery"].is_null() =>
                {
                    final_seen += 1
                }
                Some("dynamicToolCall") => {
                    let call = item["id"].as_str().ok_or(StoreError::ClaimConflict)?;
                    if item["namespace"] != "workspace"
                        || item["tool"] != "snapshot_read"
                        || item["status"] != "completed"
                        || item["success"] != true
                        || tool_completions.insert(call.into(), item.clone()).is_some()
                    {
                        return Err(StoreError::ClaimConflict);
                    }
                }
                Some("userMessage" | "reasoning") => {}
                _ => return Err(StoreError::ClaimConflict),
            }
        }
        if final_seen != 1
            || tool_completions.len() != children as usize
            || started_calls
                .iter()
                .any(|call| !tool_completions.contains_key(call))
        {
            return Err(StoreError::ClaimConflict);
        }
        let operations = {
            let mut statement=tx.prepare("SELECT call_id,snapshot_id,member_id,range_start,range_length,result_sha256,result FROM native_operations WHERE parent_invocation_id=?1")
                .map_err(|_|StoreError::Unavailable)?;
            statement
                .query_map([entry.invocation_id.as_str()], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, u64>(3)?,
                        r.get::<_, u64>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, Vec<u8>>(6)?,
                    ))
                })
                .map_err(|_| StoreError::Unavailable)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| StoreError::Unavailable)?
        };
        for (call, snapshot, member, start, length, digest, bytes) in operations {
            let item = tool_completions
                .get(&call)
                .ok_or(StoreError::ClaimConflict)?;
            let arguments = &item["arguments"];
            if arguments["snapshot_id"] != snapshot
                || arguments["member_id"] != member
                || arguments["offset"].as_u64() != Some(start)
                || arguments["length"].as_u64() != Some(length)
            {
                return Err(StoreError::ClaimConflict);
            }
            let content = item["contentItems"]
                .as_array()
                .ok_or(StoreError::ClaimConflict)?;
            if content.len() != 1 || content[0]["type"] != "inputText" {
                return Err(StoreError::ClaimConflict);
            }
            let text = content[0]["text"]
                .as_str()
                .ok_or(StoreError::ClaimConflict)?;
            let result: serde_json::Value =
                serde_json::from_str(text).map_err(|_| StoreError::ClaimConflict)?;
            let valid_bytes = if result["encoding"] == "utf8" {
                result["text"]
                    .as_str()
                    .is_some_and(|s| s.as_bytes() == bytes)
            } else if result["encoding"] == "hex" {
                result["bytes_hex"].as_str().is_some_and(|s| {
                    s == bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
                })
            } else {
                false
            };
            if !valid_bytes
                || result["sha256"] != digest
                || result["snapshot_id"] != snapshot
                || result["member_id"] != member
                || result["offset"].as_u64() != Some(start)
                || result["length"].as_u64() != Some(length)
            {
                return Err(StoreError::ClaimConflict);
            }
        }
        // Usage is attributable to each raw native emission. These totals are
        // cumulative native counters, never additive charges per notification.
        let usage_events = {
            let mut statement=tx.prepare("SELECT transport_session,sequence,raw FROM native_ingress WHERE method='thread/tokenUsage/updated' AND thread_id=?1 AND turn_id=?2 ORDER BY sequence")
                .map_err(|_|StoreError::Unavailable)?;
            statement
                .query_map(params![entry.thread.as_str(), turn.as_str()], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, Vec<u8>>(2)?,
                    ))
                })
                .map_err(|_| StoreError::Unavailable)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| StoreError::Unavailable)?
        };
        let mut previous: Option<(u64, u64, u64)> = None;
        for (transport, sequence, raw) in usage_events {
            let event: serde_json::Value =
                serde_json::from_slice(&raw).map_err(|_| StoreError::ClaimConflict)?;
            let total = &event["params"]["tokenUsage"]["total"];
            let keys = [
                "totalTokens",
                "inputTokens",
                "cachedInputTokens",
                "cacheWriteInputTokens",
                "outputTokens",
                "reasoningOutputTokens",
            ];
            let values = keys.map(|key| total.get(key).and_then(serde_json::Value::as_u64));
            let scope = if let [
                Some(all),
                Some(input),
                Some(cached),
                Some(writes),
                Some(output),
                Some(reasoning),
            ] = values
            {
                if [all, input, cached, writes, output, reasoning]
                    .iter()
                    .any(|v| *v > i64::MAX as u64)
                    || cached > input
                    || reasoning > output
                    || input.checked_add(output) != Some(all)
                    || previous.is_some_and(|(a, i, o)| all < a || input < i || output < o)
                {
                    return Err(StoreError::ClaimConflict);
                }
                previous = Some((all, input, output));
                "native_cumulative"
            } else {
                "unavailable"
            };
            tx.execute("INSERT OR IGNORE INTO native_usage(transport_session,sequence,invocation_id,scope,raw_json) VALUES (?1,?2,?3,?4,?5)",
                params![transport,sequence,entry.invocation_id,scope,raw])
                .map_err(|_|StoreError::Unavailable)?;
        }
        let (history_raw, history_wall): (Vec<u8>, i64)=tx.query_row("SELECT raw,observed_wall_ms FROM native_ingress WHERE transport_session=?1 AND sequence=?2 AND decode_status='qualified' AND method IS NULL",
            params![history_transport,history_sequence],|r|Ok((r.get(0)?,r.get(1)?)))
            .map_err(|_|StoreError::ClaimConflict)?;
        let history: serde_json::Value =
            serde_json::from_slice(&history_raw).map_err(|_| StoreError::ClaimConflict)?;
        let thread = &history["result"]["thread"];
        if thread["id"] != entry.thread.as_str() || thread["historyMode"] != "legacy" {
            return Err(StoreError::ClaimConflict);
        }
        let turns = thread["turns"]
            .as_array()
            .ok_or(StoreError::ClaimConflict)?;
        let matching = turns
            .iter()
            .filter(|v| v["id"] == turn.as_str())
            .collect::<Vec<_>>();
        if matching.len() != 1
            || matching[0]["status"] != "completed"
            || matching[0]["itemsView"] != "full"
        {
            return Err(StoreError::ClaimConflict);
        }
        let items = matching[0]["items"]
            .as_array()
            .ok_or(StoreError::ClaimConflict)?;
        if items.len() > 1024 {
            return Err(StoreError::ClaimConflict);
        }
        let (purpose, submitted_prompt): (String,Vec<u8>) = tx.query_row(
            "SELECT purpose,prompt FROM native_turns WHERE invocation_id=?1 AND thread_id=?2 AND turn_id=?3",
            params![entry.invocation_id,entry.thread.as_str(),turn.as_str()], |r| Ok((r.get(0)?,r.get(1)?)),
        ).map_err(|_|StoreError::ClaimConflict)?;
        if purpose == NativeInvocationPurpose::SuccessorRehydrate.as_str() {
            let prompt =
                std::str::from_utf8(&submitted_prompt).map_err(|_| StoreError::ClaimConflict)?;
            let input_matches = items
                .iter()
                .filter(|item| item["type"] == "userMessage")
                .filter(|item| native_user_message_matches(item, prompt))
                .count();
            if input_matches != 1 {
                return Err(StoreError::ClaimConflict);
            }
        }
        let mut history_tools = std::collections::BTreeSet::<String>::new();
        for item in items {
            match item["type"].as_str() {
                Some("dynamicToolCall") => {
                    let call = item["id"].as_str().ok_or(StoreError::ClaimConflict)?;
                    if item["namespace"] != "workspace"
                        || item["tool"] != "snapshot_read"
                        || item["status"] != "completed"
                        || item["success"] != true
                        || tool_completions.get(call) != Some(item)
                        || !history_tools.insert(call.into())
                    {
                        return Err(StoreError::ClaimConflict);
                    }
                }
                Some("agentMessage")
                    if item["questions"].is_null() && item["delivery"].is_null() => {}
                Some("userMessage" | "reasoning") => {}
                _ => return Err(StoreError::ClaimConflict),
            }
        }
        let history_final = items
            .iter()
            .rev()
            .find(|v| v["type"] == "agentMessage")
            .and_then(|v| v["text"].as_str());
        let visible_tools = history_tools.len();
        if history_final != Some(final_text)
            || (history_status == "observed_complete" && visible_tools != children as usize)
            || (history_status == "observed_tool_omitted" && (children == 0 || visible_tools != 0))
        {
            return Err(StoreError::ClaimConflict);
        }
        let digest = sha256(final_text.as_bytes())?;
        let changed=tx.execute("UPDATE native_turns SET state='completed_settled',terminal_status='completed',final_text=?2,final_sha256=?3,history_status=?4,history_transport_session=?8,history_sequence=?9,settled_wall_ms=?5 WHERE invocation_id=?1 AND thread_id=?6 AND turn_id=?7 AND state='entered_uncertain'",
            params![entry.invocation_id,final_text.as_bytes(),digest,history_status,clock.wall.get(),entry.thread.as_str(),turn.as_str(),history_transport,history_sequence])
            .map_err(|_| StoreError::Unavailable)?;
        if changed != 1 {
            return Err(StoreError::ClaimConflict);
        }
        let subject: String = tx.query_row(
            "SELECT s.subject_id FROM native_turns t JOIN native_sessions s ON s.session_id=t.session_id WHERE t.invocation_id=?1",
            [entry.invocation_id.as_str()], |r| r.get(0),
        ).map_err(|_| StoreError::Unavailable)?;
        observe_native_subject_clock(&tx, &subject, clock.wall.get().max(history_wall))?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(())
    }

    /// Query-only recovery; absence of native history never means non-entry.
    pub fn native_recovery(
        &self,
        invocation_id: &str,
    ) -> Result<Option<NativeRecovery>, StoreError> {
        self.verify_identity()?;
        let row=self.connection.query_row(
            "SELECT t.state,t.session_id,t.attempt_id,t.purpose,t.grant_id,t.grant_revision,t.send_intent,s.allocation_state,t.thread_id,t.turn_id,t.profile_digest,s.binary_sha256,s.protocol_digest,s.snapshot_id,s.snapshot_manifest_sha256,t.final_sha256,t.history_status,t.history_transport_session,t.history_sequence,x.outcome_kind FROM native_turns t JOIN native_sessions s ON s.session_id=t.session_id LEFT JOIN native_process_exits x ON x.session_id=s.session_id WHERE t.invocation_id=?1",
            [invocation_id],|r|Ok((r.get::<_,String>(0)?,NativeRecovery{
                class:String::new(),session_id:r.get(1)?,attempt_id:r.get(2)?,purpose:r.get(3)?,
                grant_id:r.get(4)?,grant_revision:r.get::<_,i64>(5)? as u64,
                send_intent:r.get::<_,i64>(6)?!=0,allocation_state:r.get(7)?,
                thread_id:r.get(8)?,turn_id:r.get(9)?,profile_digest:r.get(10)?,
                binary_sha256:r.get(11)?,protocol_digest:r.get(12)?,snapshot_id:r.get(13)?,
                snapshot_manifest_sha256:r.get(14)?,final_sha256:r.get(15)?,history_status:r.get(16)?,
                history_transport_session:r.get(17)?,history_sequence:r.get::<_,Option<i64>>(18)?.map(|n|n as u64),
                process_outcome:r.get(19)?,admitted_children:0,unsettled_children:0,children:Vec::new(),
            })))
            .optional().map_err(|_| StoreError::Unavailable)?;
        let Some((state, mut recovery)) = row else {
            return Ok(None);
        };
        let (children,unsettled):(i64,i64)=self.connection.query_row(
            "SELECT COUNT(*),COALESCE(SUM(CASE WHEN state!='completed_settled' THEN 1 ELSE 0 END),0) FROM native_operations WHERE parent_invocation_id=?1",
            [invocation_id],|r|Ok((r.get(0)?,r.get(1)?)))
            .map_err(|_| StoreError::Unavailable)?;
        let class = match state.as_str() {
            "entered_uncertain" => "EnteredUncertain",
            "completed_settled" => "CompletedSettled",
            _ => "BlockedOrConflicted",
        };
        let operations = {
            let mut statement=self.connection.prepare("SELECT operation_attempt_id,call_id,state,request_transport_session,request_sequence,contract_id,implementation_id,executor_id,snapshot_id,member_id,range_start,range_length,result_sha256 FROM native_operations WHERE parent_invocation_id=?1 ORDER BY entered_wall_ms,operation_attempt_id")
                .map_err(|_|StoreError::Unavailable)?;
            statement
                .query_map([invocation_id], |r| {
                    Ok(NativeOperationRecovery {
                        attempt_id: r.get(0)?,
                        call_id: r.get(1)?,
                        class: r.get(2)?,
                        request_transport_session: r.get(3)?,
                        request_sequence: r.get::<_, i64>(4)? as u64,
                        contract_id: r.get(5)?,
                        implementation_id: r.get(6)?,
                        executor_id: r.get(7)?,
                        snapshot_id: r.get(8)?,
                        member_id: r.get(9)?,
                        range_start: r.get::<_, i64>(10)? as u64,
                        range_length: r.get::<_, i64>(11)? as u64,
                        result_sha256: r.get(12)?,
                    })
                })
                .map_err(|_| StoreError::Unavailable)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| StoreError::Unavailable)?
        };
        recovery.class = class.into();
        recovery.admitted_children = children as u64;
        recovery.unsettled_children = unsettled as u64;
        recovery.children = operations;
        Ok(Some(recovery))
    }

    pub fn record_native_process_exit(
        &mut self,
        session: &NativeSessionId,
        pid: Option<u32>,
        exit_code: Option<i32>,
        forced: bool,
        outcome_kind: &str,
        clock: ClockSample,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        if !matches!(
            outcome_kind,
            "completed" | "protocol_error" | "transport_error" | "shutdown" | "cleanup_unobserved"
        ) {
            return Err(StoreError::Rejected);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        tx.execute("INSERT INTO native_process_exits(session_id,pid,exit_code,forced,outcome_kind,observed_wall_ms) VALUES (?1,?2,?3,?4,?5,?6)",
            params![session.as_str(),pid.map(i64::from),exit_code,forced,outcome_kind,clock.wall.get()])
            .map_err(|_|StoreError::ClaimConflict)?;
        let subject: String = tx
            .query_row(
                "SELECT subject_id FROM native_sessions WHERE session_id=?1",
                [session.as_str()],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        observe_native_subject_clock(&tx, &subject, clock.wall.get())?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(())
    }
}

fn native_user_message_matches(item: &serde_json::Value, prompt: &str) -> bool {
    if item["text"] == prompt {
        return true;
    }
    let Some(content) = item["content"].as_array() else {
        return false;
    };
    content.len() == 1
        && content[0]["text"] == prompt
        && matches!(content[0]["type"].as_str(), Some("text" | "inputText"))
}

fn observe_native_subject_clock(
    tx: &rusqlite::Transaction<'_>,
    subject: &str,
    observed_wall_ms: i64,
) -> Result<(), StoreError> {
    let changed = tx
        .execute(
            "UPDATE subjects SET last_wall_ms=MAX(last_wall_ms,?1) WHERE subject_id=?2",
            params![observed_wall_ms, subject],
        )
        .map_err(|_| StoreError::Unavailable)?;
    if changed != 1 {
        return Err(StoreError::ClaimConflict);
    }
    Ok(())
}

fn check_grant(
    tx: &rusqlite::Transaction<'_>,
    expected: GrantExpectation<'_>,
) -> Result<(), StoreError> {
    let current: Option<(String, String, String, String, Option<String>, i64)> = tx
        .query_row(
            "SELECT context_id,build_id,proof_run_id,owner_kind,owner_ref,last_wall_ms FROM subjects WHERE subject_id=?1",
            [expected.subject],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()
        .map_err(|_| StoreError::Unavailable)?;
    let Some((current_context, current_build, current_proof, owner, owner_ref, last_wall_ms)) =
        current
    else {
        return Err(StoreError::Rejected);
    };
    if expected.clock.wall.get() < last_wall_ms {
        crate::sqlite::fence_subject(tx, expected.subject, "clock:rollback", expected.clock)?;
        return Err(StoreError::Rejected);
    }
    tx.execute(
        "UPDATE subjects SET last_wall_ms=MAX(last_wall_ms,?1) WHERE subject_id=?2",
        params![expected.clock.wall.get(), expected.subject],
    )
    .map_err(|_| StoreError::Unavailable)?;
    if expected.successor_rehydrate {
        if owner != "transition" || owner_ref.is_none() || current_context == expected.context {
            return Err(StoreError::Rejected);
        }
        let valid: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM transitions WHERE transition_id=?1 AND subject_id=?2 AND stage='rehydrating' AND successor_context=?3)",
            params![owner_ref,expected.subject,expected.context], |r| r.get(0),
        ).map_err(|_| StoreError::Unavailable)?;
        if !valid {
            return Err(StoreError::Rejected);
        }
    } else {
        if current_context != expected.context || owner != "domain" || owner_ref.is_some() {
            return Err(StoreError::Rejected);
        }
        let open_transition: i64 = tx.query_row(
            "SELECT COUNT(*) FROM transitions WHERE subject_id=?1 AND stage NOT IN ('reconciled','aborted_before_entry')",
            [expected.subject],
            |r| r.get(0),
        ).map_err(|_| StoreError::Unavailable)?;
        if open_transition != 0 {
            return Err(StoreError::Rejected);
        }
    }
    let row:Option<NativeGrantRow>=tx.query_row(
        "SELECT issuer,principal,subject_id,context_id,build_id,proof_run_id,scope,expires_ms,revision,revoked FROM grants WHERE grant_id=?1",
        [expected.grant.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?)))
        .optional().map_err(|_| StoreError::Unavailable)?;
    let Some((i, p, s, c, b, proof, sc, expires, rev, revoked)) = row else {
        return Err(StoreError::Rejected);
    };
    if i != expected.issuer
        || p != expected.principal
        || s != expected.subject
        || c != expected.context
        || b != current_build
        || proof != current_proof
        || sc != expected.scope
        || expires <= expected.clock.wall.get()
        || revoked
        || rev < 0
        || rev as u64 != expected.revision
    {
        return Err(StoreError::Rejected);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifecycle_core::{
        AuthorityExpiresAt, BuildId, GrantScope, ProofRunId, Revision, TrustedGrant, WaitBudgetMs,
        WallTimeMs,
    };
    use tempfile::tempdir;

    fn clock() -> ClockSample {
        ClockSample {
            wall: WallTimeMs::new(10),
            wait_budget: WaitBudgetMs::new(1),
        }
    }

    #[test]
    fn raw_ingress_replay_uses_source_coordinate_not_value_hash() {
        let root = tempdir().unwrap();
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        let subject = SubjectId::parse("subject:ingress").unwrap();
        let context = ContextGeneration::parse("context:ingress").unwrap();
        store
            .register_subject(
                subject.clone(),
                context.clone(),
                BuildId::parse("build:ingress").unwrap(),
                ProofRunId::parse("proof:ingress").unwrap(),
                clock(),
            )
            .unwrap();
        let hash = "a".repeat(64);
        let _allocation = store
            .reserve_native_session(
                NativeSessionId::parse("session:test").unwrap(),
                subject,
                context,
                RuntimeIncarnation::parse("incarnation:ingress").unwrap(),
                &hash,
                &hash,
                &hash,
                SnapshotId::parse("snapshot:ingress").unwrap(),
                &hash,
                false,
                clock(),
            )
            .unwrap();
        let first = br#"{"method":"warning","params":{"message":"one"}}"#;
        let conflict = br#"{"method":"warning","params":{"message":"two"}}"#;
        assert_eq!(
            store
                .record_native_ingress(
                    &NativeSessionId::parse("session:test").unwrap(),
                    "transport:test",
                    1,
                    first,
                    "qualified",
                    Some("warning"),
                    None,
                    None,
                    None,
                    clock()
                )
                .unwrap(),
            NativeIngressApply::Inserted
        );
        assert_eq!(
            store
                .record_native_ingress(
                    &NativeSessionId::parse("session:test").unwrap(),
                    "transport:test",
                    1,
                    first,
                    "qualified",
                    Some("warning"),
                    None,
                    None,
                    None,
                    clock()
                )
                .unwrap(),
            NativeIngressApply::Duplicate
        );
        assert_eq!(
            store
                .record_native_ingress(
                    &NativeSessionId::parse("session:test").unwrap(),
                    "transport:test",
                    2,
                    first,
                    "qualified",
                    Some("warning"),
                    None,
                    None,
                    None,
                    clock()
                )
                .unwrap(),
            NativeIngressApply::Inserted
        );
        assert_eq!(
            store
                .record_native_ingress(
                    &NativeSessionId::parse("session:test").unwrap(),
                    "transport:test",
                    1,
                    conflict,
                    "qualified",
                    Some("warning"),
                    None,
                    None,
                    None,
                    clock()
                )
                .unwrap(),
            NativeIngressApply::ConflictFenced
        );
        let count: i64 = store
            .connection
            .query_row("SELECT COUNT(*) FROM native_ingress", [], |r| r.get(0))
            .unwrap();
        let conflicts: i64 = store
            .connection
            .query_row("SELECT COUNT(*) FROM native_ingress_conflicts", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!((count, conflicts), (2, 1));
        let first_page = store.native_ingress_page("transport:test", 0, 1).unwrap();
        assert_eq!(first_page.version, 1);
        assert_eq!(first_page.events.len(), 1);
        assert_eq!(first_page.events[0].sequence, 1);
        assert_eq!(first_page.events[0].raw, first);
        assert_eq!(first_page.events[0].observed_wall_ms, clock().wall.get());
        assert_eq!(first_page.next_after_sequence, Some(1));
        let second_page = store.native_ingress_page("transport:test", 1, 1).unwrap();
        assert_eq!(second_page.events[0].sequence, 2);
        assert_eq!(second_page.next_after_sequence, None);
    }

    #[test]
    fn unbound_allocation_intent_cannot_be_recreated_after_restart() {
        let root = tempdir().unwrap();
        let subject = SubjectId::parse("subject:test").unwrap();
        let context = ContextGeneration::parse("context:test").unwrap();
        let incarnation = RuntimeIncarnation::parse("incarnation:test").unwrap();
        let session = NativeSessionId::parse("session:test").unwrap();
        let snapshot = SnapshotId::parse("snapshot:test").unwrap();
        let hash = "a".repeat(64);
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        store
            .register_subject(
                subject.clone(),
                context.clone(),
                BuildId::parse("build:test").unwrap(),
                ProofRunId::parse("proof:test").unwrap(),
                clock(),
            )
            .unwrap();
        let allocation = store
            .reserve_native_session(
                session.clone(),
                subject.clone(),
                context.clone(),
                incarnation.clone(),
                &hash,
                &hash,
                &hash,
                snapshot.clone(),
                &hash,
                false,
                clock(),
            )
            .unwrap();
        let allocation_recovery = store.native_session_recovery(&session).unwrap().unwrap();
        assert_eq!(allocation_recovery.version, 1);
        assert_eq!(allocation_recovery.allocation_state, "intent");
        assert_eq!(allocation_recovery.thread_id, None);
        assert!(matches!(
            store.record_service_close(&incarnation, &subject, &[], &[], &[], false, clock().wall),
            Err(StoreError::Rejected)
        ));
        store
            .record_service_close(&incarnation, &subject, &[], &[], &[], true, clock().wall)
            .unwrap();
        let close: String = store
            .connection
            .query_row(
                "SELECT native_close_json FROM service_closes WHERE incarnation_id=?1",
                [incarnation.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        assert!(close.contains("\"allocation_state\":\"intent\""));
        drop(allocation);
        drop(store);
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        assert!(matches!(
            store.reserve_native_session(
                session,
                subject,
                context,
                incarnation,
                &hash,
                &hash,
                &hash,
                snapshot,
                &hash,
                false,
                clock()
            ),
            Err(StoreError::ClaimConflict)
        ));
    }

    #[test]
    fn native_thread_allocation_follows_committed_context_and_refuses_same_session_policy() {
        let root = tempdir().unwrap();
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        let subject = SubjectId::parse("subject:generation").unwrap();
        let first = ContextGeneration::parse("context:first").unwrap();
        let second = ContextGeneration::parse("context:second").unwrap();
        let snapshot = SnapshotId::parse("snapshot:generation").unwrap();
        let hash = "a".repeat(64);
        store
            .register_subject(
                subject.clone(),
                first.clone(),
                BuildId::parse("build:generation").unwrap(),
                ProofRunId::parse("proof:generation").unwrap(),
                clock(),
            )
            .unwrap();
        let reserve = |store: &mut SqliteLifecycleStore,
                       session: &str,
                       context: ContextGeneration,
                       same_session_required| {
            store.reserve_native_session(
                NativeSessionId::parse(session).unwrap(),
                subject.clone(),
                context,
                RuntimeIncarnation::parse(format!("incarnation:{session}")).unwrap(),
                &hash,
                &hash,
                &hash,
                snapshot.clone(),
                &hash,
                same_session_required,
                clock(),
            )
        };
        assert!(matches!(
            reserve(&mut store, "session:refused", first.clone(), true),
            Err(StoreError::Rejected)
        ));
        assert!(matches!(
            reserve(&mut store, "session:premature", second.clone(), false),
            Err(StoreError::Rejected)
        ));
        let allocated: i64 = store
            .connection
            .query_row("SELECT COUNT(*) FROM native_sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(allocated, 0);
        let mut first_session = reserve(&mut store, "session:first", first.clone(), false).unwrap();
        let first_thread = ProviderThreadId::parse("thread:first").unwrap();
        store
            .bind_native_thread(&mut first_session, first_thread.clone())
            .unwrap();

        // S4's ordered transition tests establish the producer that commits this
        // subject projection. Here the native owner consumes that resulting state.
        store
            .connection
            .execute(
                "UPDATE subjects SET context_id=?1 WHERE subject_id=?2",
                params![second.as_str(), subject.as_str()],
            )
            .unwrap();
        assert!(matches!(
            reserve(&mut store, "session:stale", first, false),
            Err(StoreError::Rejected)
        ));
        let mut second_session = reserve(&mut store, "session:second", second, false).unwrap();
        assert!(
            store
                .bind_native_thread(&mut second_session, first_thread)
                .is_err()
        );
        store
            .bind_native_thread(
                &mut second_session,
                ProviderThreadId::parse("thread:second").unwrap(),
            )
            .unwrap();
        assert_ne!(first_session.thread(), second_session.thread());
    }

    #[test]
    fn rejected_native_admission_advances_clock_and_rollback_fences_subject() {
        let root = tempdir().unwrap();
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        let subject = SubjectId::parse("subject:clock").unwrap();
        let context = ContextGeneration::parse("context:clock").unwrap();
        let build = BuildId::parse("build:clock").unwrap();
        let proof = ProofRunId::parse("proof:clock").unwrap();
        store
            .register_subject(
                subject.clone(),
                context.clone(),
                build.clone(),
                proof.clone(),
                clock(),
            )
            .unwrap();
        let grant = GrantId::parse("grant:clock").unwrap();
        store
            .install_trusted_grant(TrustedGrant {
                id: grant.clone(),
                issuer: "trusted".into(),
                principal: "principal:clock".into(),
                subject: subject.clone(),
                context: context.clone(),
                build,
                proof_run: proof,
                scope: GrantScope::parse("native_turn").unwrap(),
                expires: AuthorityExpiresAt::new(WallTimeMs::new(100)),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
        let hash = "a".repeat(64);
        let mut session = store
            .reserve_native_session(
                NativeSessionId::parse("session:clock").unwrap(),
                subject,
                context,
                RuntimeIncarnation::parse("incarnation:clock").unwrap(),
                &hash,
                &hash,
                &hash,
                SnapshotId::parse("snapshot:clock").unwrap(),
                &hash,
                false,
                clock(),
            )
            .unwrap();
        store
            .bind_native_thread(
                &mut session,
                ProviderThreadId::parse("thread:clock").unwrap(),
            )
            .unwrap();
        let request = || NativeTurnRequest {
            invocation_id: "invocation:clock".into(),
            attempt: AttemptId::parse("attempt:clock").unwrap(),
            grant: grant.clone(),
            grant_revision: 1,
            purpose: NativeInvocationPurpose::DomainWork,
            prompt: b"clock".to_vec(),
        };
        let later = ClockSample {
            wall: WallTimeMs::new(101),
            wait_budget: clock().wait_budget,
        };
        assert!(matches!(
            store.claim_native_turn(&session, request(), "principal:clock", later),
            Err(StoreError::Rejected)
        ));
        assert!(matches!(
            store.claim_native_turn(&session, request(), "principal:clock", clock()),
            Err(StoreError::Rejected)
        ));
        let (owner, last_wall): (String, i64) = store
            .connection
            .query_row(
                "SELECT owner_kind,last_wall_ms FROM subjects WHERE subject_id='subject:clock'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((owner.as_str(), last_wall), ("fenced", 101));
        let entered: i64 = store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM native_turns WHERE invocation_id='invocation:clock'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(entered, 0);
        let rollback_journal: i64 = store.connection.query_row(
            "SELECT COUNT(*) FROM journal WHERE event_kind='observation_conflict' AND event_ref='clock:rollback' AND subject_id='subject:clock'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(rollback_journal, 1);
    }

    #[test]
    fn settled_native_observations_fence_rolled_back_followup() {
        let root = tempdir().unwrap();
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        let at = |wall| ClockSample {
            wall: WallTimeMs::new(wall),
            wait_budget: clock().wait_budget,
        };
        let subject = SubjectId::parse("subject:sequential").unwrap();
        let context = ContextGeneration::parse("context:sequential").unwrap();
        let build = BuildId::parse("build:sequential").unwrap();
        let proof = ProofRunId::parse("proof:sequential").unwrap();
        store
            .register_subject(
                subject.clone(),
                context.clone(),
                build.clone(),
                proof.clone(),
                clock(),
            )
            .unwrap();
        let grant = GrantId::parse("grant:sequential").unwrap();
        store
            .install_trusted_grant(TrustedGrant {
                id: grant.clone(),
                issuer: "trusted".into(),
                principal: "principal:sequential".into(),
                subject: subject.clone(),
                context: context.clone(),
                build: build.clone(),
                proof_run: proof.clone(),
                scope: GrantScope::parse("native_turn").unwrap(),
                expires: AuthorityExpiresAt::new(WallTimeMs::new(250)),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
        let long_grant = GrantId::parse("grant:sequential-long").unwrap();
        store
            .install_trusted_grant(TrustedGrant {
                id: long_grant.clone(),
                issuer: "trusted".into(),
                principal: "principal:sequential".into(),
                subject: subject.clone(),
                context: context.clone(),
                build,
                proof_run: proof,
                scope: GrantScope::parse("native_turn").unwrap(),
                expires: AuthorityExpiresAt::new(WallTimeMs::new(500)),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
        let hash = "a".repeat(64);
        let session_id = NativeSessionId::parse("session:sequential").unwrap();
        let mut session = store
            .reserve_native_session(
                session_id.clone(),
                subject.clone(),
                context,
                RuntimeIncarnation::parse("incarnation:sequential").unwrap(),
                &hash,
                &hash,
                &hash,
                SnapshotId::parse("snapshot:sequential").unwrap(),
                &hash,
                false,
                at(100),
            )
            .unwrap();
        store
            .bind_native_thread(
                &mut session,
                ProviderThreadId::parse("thread:sequential").unwrap(),
            )
            .unwrap();
        let request = |name: &str| NativeTurnRequest {
            invocation_id: format!("invocation:{name}"),
            attempt: AttemptId::parse(format!("attempt:{name}")).unwrap(),
            grant: grant.clone(),
            grant_revision: 1,
            purpose: NativeInvocationPurpose::DomainWork,
            prompt: b"test".to_vec(),
        };
        let mut first = store
            .claim_native_turn(&session, request("first"), "principal:sequential", at(100))
            .unwrap();
        store.mark_native_send_intent(&first).unwrap();
        store
            .bind_native_turn(
                &mut first,
                ProviderTurnId::parse("turn:sequential").unwrap(),
            )
            .unwrap();
        let final_item =
            serde_json::json!({"type":"agentMessage","id":"final:sequential","text":"done"});
        for (sequence, method, value) in [
            (
                1,
                "turn/completed",
                serde_json::json!({"method":"turn/completed","params":{"threadId":"thread:sequential","turn":{"id":"turn:sequential","status":"completed"}}}),
            ),
            (
                2,
                "item/completed",
                serde_json::json!({"method":"item/completed","params":{"threadId":"thread:sequential","turnId":"turn:sequential","item":final_item.clone()}}),
            ),
        ] {
            store
                .record_native_ingress(
                    &session_id,
                    "transport:sequential",
                    sequence,
                    &serde_json::to_vec(&value).unwrap(),
                    "qualified",
                    Some(method),
                    Some("thread:sequential"),
                    Some("turn:sequential"),
                    None,
                    at(200),
                )
                .unwrap();
        }
        let last_wall: i64 = store
            .connection
            .query_row(
                "SELECT last_wall_ms FROM subjects WHERE subject_id=?1",
                [subject.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(last_wall, 200);
        let history = serde_json::json!({"id":3,"result":{"thread":{"id":"thread:sequential","historyMode":"legacy","turns":[{"id":"turn:sequential","status":"completed","itemsView":"full","items":[final_item]}]}}});
        store
            .record_native_ingress(
                &session_id,
                "transport:sequential",
                3,
                &serde_json::to_vec(&history).unwrap(),
                "qualified",
                None,
                None,
                None,
                None,
                at(220),
            )
            .unwrap();
        let last_wall: i64 = store
            .connection
            .query_row(
                "SELECT last_wall_ms FROM subjects WHERE subject_id=?1",
                [subject.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(last_wall, 220);
        // A valid later grant may lose the active-turn race. Its trusted wall
        // sample still fences a subsequent rolled-back admission.
        let long_request = |name: &str| NativeTurnRequest {
            invocation_id: format!("invocation:{name}"),
            attempt: AttemptId::parse(format!("attempt:{name}")).unwrap(),
            grant: long_grant.clone(),
            grant_revision: 1,
            purpose: NativeInvocationPurpose::DomainWork,
            prompt: b"test".to_vec(),
        };
        assert!(matches!(
            store.claim_native_turn(
                &session,
                long_request("active-conflict"),
                "principal:sequential",
                at(400)
            ),
            Err(StoreError::ClaimConflict)
        ));
        let last_wall: i64 = store
            .connection
            .query_row(
                "SELECT last_wall_ms FROM subjects WHERE subject_id=?1",
                [subject.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(last_wall, 400);
        store
            .settle_native_turn(
                &first,
                "done",
                "observed_complete",
                "transport:sequential",
                3,
                at(300),
            )
            .unwrap();
        assert_eq!(
            store
                .native_recovery("invocation:first")
                .unwrap()
                .unwrap()
                .class,
            "CompletedSettled"
        );
        assert_eq!(
            store
                .native_final_material("invocation:first")
                .unwrap()
                .unwrap()
                .history_observed_wall_ms,
            220
        );
        assert!(matches!(
            store.claim_native_turn(
                &session,
                long_request("rollback-after-conflict"),
                "principal:sequential",
                at(350)
            ),
            Err(StoreError::Rejected)
        ));
        assert!(matches!(
            store.claim_native_turn(&session, request("second"), "principal:sequential", at(150)),
            Err(StoreError::Rejected)
        ));
        let (owner, last_wall): (String, i64) = store
            .connection
            .query_row(
                "SELECT owner_kind,last_wall_ms FROM subjects WHERE subject_id=?1",
                [subject.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((owner.as_str(), last_wall), ("fenced", 400));
        let turns: i64 = store
            .connection
            .query_row("SELECT COUNT(*) FROM native_turns", [], |r| r.get(0))
            .unwrap();
        assert_eq!(turns, 1);
        assert_eq!(
            store
                .native_final_material("invocation:first")
                .unwrap()
                .unwrap()
                .final_text,
            b"done"
        );
    }

    #[test]
    fn completed_native_turn_cannot_settle_async_question_or_unsupported_descendant() {
        for (item, ingress_method, history_only) in [
            (
                serde_json::json!({"type":"agentMessage","text":"Need answer","questions":[{"id":"q1"}]}),
                "item/completed",
                false,
            ),
            (
                serde_json::json!({"type":"dynamicToolCall","id":"call:running","namespace":"workspace","tool":"snapshot_read","status":"inProgress"}),
                "item/started",
                false,
            ),
            (
                serde_json::json!({"type":"remoteToolCall","id":"remote:one","status":"completed"}),
                "item/completed",
                true,
            ),
        ] {
            let root = tempdir().unwrap();
            let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
            let subject = SubjectId::parse("subject:adverse").unwrap();
            let context = ContextGeneration::parse("context:adverse").unwrap();
            let build = BuildId::parse("build:adverse").unwrap();
            let proof = ProofRunId::parse("proof:adverse").unwrap();
            store
                .register_subject(
                    subject.clone(),
                    context.clone(),
                    build.clone(),
                    proof.clone(),
                    clock(),
                )
                .unwrap();
            let grant = GrantId::parse("grant:turn").unwrap();
            store
                .install_trusted_grant(TrustedGrant {
                    id: grant.clone(),
                    issuer: "trusted".into(),
                    principal: "principal:adverse".into(),
                    subject: subject.clone(),
                    context: context.clone(),
                    build,
                    proof_run: proof,
                    scope: GrantScope::parse("native_turn").unwrap(),
                    expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
                    revision: Revision::new(1),
                    revoked: false,
                })
                .unwrap();
            let hash = "a".repeat(64);
            let mut session = store
                .reserve_native_session(
                    NativeSessionId::parse("session:adverse").unwrap(),
                    subject,
                    context,
                    RuntimeIncarnation::parse("incarnation:adverse").unwrap(),
                    &hash,
                    &hash,
                    &hash,
                    SnapshotId::parse("snapshot:adverse").unwrap(),
                    &hash,
                    false,
                    clock(),
                )
                .unwrap();
            store
                .bind_native_thread(
                    &mut session,
                    ProviderThreadId::parse("thread:adverse").unwrap(),
                )
                .unwrap();
            let mut turn = store
                .claim_native_turn(
                    &session,
                    NativeTurnRequest {
                        invocation_id: "invocation:adverse".into(),
                        attempt: AttemptId::parse("attempt:adverse").unwrap(),
                        grant,
                        grant_revision: 1,
                        purpose: NativeInvocationPurpose::DomainWork,
                        prompt: b"test".to_vec(),
                    },
                    "principal:adverse",
                    clock(),
                )
                .unwrap();
            store.mark_native_send_intent(&turn).unwrap();
            store
                .bind_native_turn(&mut turn, ProviderTurnId::parse("turn:adverse").unwrap())
                .unwrap();
            let mut ingress = vec![(
                1,
                "turn/completed",
                serde_json::json!({"method":"turn/completed","params":{"threadId":"thread:adverse","turn":{"id":"turn:adverse","status":"completed"}}}),
            )];
            if !history_only {
                ingress.push((
                    2,
                    ingress_method,
                    serde_json::json!({"method":ingress_method,"params":{"threadId":"thread:adverse","turnId":"turn:adverse","item":item.clone()}}),
                ));
            }
            let final_item =
                serde_json::json!({"type":"agentMessage","id":"final:one","text":"Need answer"});
            ingress.push((
                3,
                "item/completed",
                serde_json::json!({"method":"item/completed","params":{"threadId":"thread:adverse","turnId":"turn:adverse","item":final_item.clone()}}),
            ));
            for (sequence, method, value) in ingress {
                store
                    .record_native_ingress(
                        &NativeSessionId::parse("session:adverse").unwrap(),
                        "transport:adverse",
                        sequence,
                        &serde_json::to_vec(&value).unwrap(),
                        "qualified",
                        Some(method),
                        Some("thread:adverse"),
                        Some("turn:adverse"),
                        None,
                        clock(),
                    )
                    .unwrap();
            }
            let history_items = if history_only {
                vec![item, final_item]
            } else {
                vec![final_item]
            };
            let history = serde_json::json!({"id":4,"result":{"thread":{"id":"thread:adverse","historyMode":"legacy","turns":[{"id":"turn:adverse","status":"completed","itemsView":"full","items":history_items}]}}});
            store
                .record_native_ingress(
                    &NativeSessionId::parse("session:adverse").unwrap(),
                    "transport:adverse",
                    4,
                    &serde_json::to_vec(&history).unwrap(),
                    "qualified",
                    None,
                    None,
                    None,
                    None,
                    clock(),
                )
                .unwrap();
            assert!(matches!(
                store.settle_native_turn(
                    &turn,
                    "Need answer",
                    "observed_complete",
                    "transport:adverse",
                    4,
                    clock()
                ),
                Err(StoreError::ClaimConflict)
            ));
            assert_eq!(
                store
                    .native_recovery("invocation:adverse")
                    .unwrap()
                    .unwrap()
                    .class,
                "EnteredUncertain"
            );
        }
    }

    #[test]
    fn child_claim_checks_exact_raw_call_and_entry_bound_revocation() {
        let root = tempdir().unwrap();
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        let subject = SubjectId::parse("subject:test").unwrap();
        let context = ContextGeneration::parse("context:test").unwrap();
        let build = BuildId::parse("build:test").unwrap();
        let proof = ProofRunId::parse("proof:test").unwrap();
        store
            .register_subject(
                subject.clone(),
                context.clone(),
                build.clone(),
                proof.clone(),
                clock(),
            )
            .unwrap();
        for (id, scope) in [
            ("grant:turn", "native_turn"),
            ("grant:read", "snapshot_read"),
            ("grant:read-long", "snapshot_read"),
        ] {
            store
                .install_trusted_grant(TrustedGrant {
                    id: GrantId::parse(id).unwrap(),
                    issuer: "trusted".into(),
                    principal: "principal:test".into(),
                    subject: subject.clone(),
                    context: context.clone(),
                    build: build.clone(),
                    proof_run: proof.clone(),
                    scope: GrantScope::parse(scope).unwrap(),
                    expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
                    revision: Revision::new(1),
                    revoked: false,
                })
                .unwrap();
        }
        let hash = "a".repeat(64);
        let manifest=serde_json::to_vec(&serde_json::json!({
            "codec":"workspace-snapshot-manifest-v1","id":"snapshot:test",
            "source_revision":"source:test","members":[{"id":"m1","sha256":sha256(b"abc").unwrap(),"length":3}]
        })).unwrap();
        let staged = store.stage_artifact(subject.clone(), &manifest).unwrap();
        let manifest_digest = store.publish_artifact(staged, clock()).unwrap().digest_hex;
        let mut session = store
            .reserve_native_session(
                NativeSessionId::parse("session:test").unwrap(),
                subject,
                context,
                RuntimeIncarnation::parse("incarnation:test").unwrap(),
                &hash,
                &hash,
                &hash,
                SnapshotId::parse("snapshot:test").unwrap(),
                &manifest_digest,
                false,
                clock(),
            )
            .unwrap();
        store
            .bind_native_thread(
                &mut session,
                ProviderThreadId::parse("thread:test").unwrap(),
            )
            .unwrap();
        let make_turn = || NativeTurnRequest {
            invocation_id: "invocation:test".into(),
            attempt: AttemptId::parse("attempt:test").unwrap(),
            grant: GrantId::parse("grant:turn").unwrap(),
            grant_revision: 1,
            purpose: NativeInvocationPurpose::DomainWork,
            prompt: b"test".to_vec(),
        };
        assert!(matches!(
            store.claim_native_turn(&session, make_turn(), "principal:other", clock()),
            Err(StoreError::Rejected)
        ));
        store
            .connection
            .execute(
                "UPDATE subjects SET owner_kind='fenced' WHERE subject_id='subject:test'",
                [],
            )
            .unwrap();
        assert!(matches!(
            store.claim_native_turn(&session, make_turn(), "principal:test", clock()),
            Err(StoreError::Rejected)
        ));
        store
            .connection
            .execute(
                "UPDATE subjects SET owner_kind='domain' WHERE subject_id='subject:test'",
                [],
            )
            .unwrap();
        let mut turn = store
            .claim_native_turn(&session, make_turn(), "principal:test", clock())
            .unwrap();
        store.mark_native_send_intent(&turn).unwrap();
        store
            .bind_native_turn(&mut turn, ProviderTurnId::parse("turn:test").unwrap())
            .unwrap();
        let raw=br#"{"method":"item/tool/call","id":0,"params":{"threadId":"thread:test","turnId":"turn:test","callId":"call:test","namespace":"workspace","tool":"snapshot_read","arguments":{"snapshot_id":"snapshot:test","member_id":"m1","offset":0,"length":3}}}"#;
        store
            .record_native_ingress(
                &NativeSessionId::parse("session:test").unwrap(),
                "transport:test",
                1,
                raw,
                "qualified",
                Some("item/tool/call"),
                Some("thread:test"),
                Some("turn:test"),
                None,
                clock(),
            )
            .unwrap();
        let request = |member: &str, call: &str, sequence| SnapshotReadRequest {
            attempt: OperationAttemptId::parse(format!("opattempt:{call}")).unwrap(),
            call: NativeToolCallId::parse(call).unwrap(),
            transport_session: "transport:test".into(),
            sequence,
            grant: GrantId::parse("grant:read").unwrap(),
            grant_revision: 1,
            contract: OperationContractId::parse(SNAPSHOT_CONTRACT).unwrap(),
            implementation: OperationImplementationId::parse(SNAPSHOT_IMPLEMENTATION).unwrap(),
            executor: OperationExecutorId::parse(SNAPSHOT_EXECUTOR).unwrap(),
            snapshot: SnapshotId::parse("snapshot:test").unwrap(),
            member_id: member.into(),
            range_start: 0,
            range_length: 3,
        };
        assert!(matches!(
            store.claim_snapshot_read(
                &turn,
                request("m2", "call:test", 1),
                "principal:test",
                clock()
            ),
            Err(StoreError::Rejected)
        ));
        let NativeOperationClaim::Entered(child) = store
            .claim_snapshot_read(
                &turn,
                request("m1", "call:test", 1),
                "principal:test",
                clock(),
            )
            .unwrap()
        else {
            panic!("expected entered child")
        };
        let raw_conflict=br#"{"method":"item/tool/call","id":9,"params":{"threadId":"thread:test","turnId":"turn:test","callId":"call:conflict","namespace":"workspace","tool":"snapshot_read","arguments":{"snapshot_id":"snapshot:test","member_id":"m1","offset":0,"length":3}}}"#;
        store
            .record_native_ingress(
                &NativeSessionId::parse("session:test").unwrap(),
                "transport:test",
                9,
                raw_conflict,
                "qualified",
                Some("item/tool/call"),
                Some("thread:test"),
                Some("turn:test"),
                None,
                clock(),
            )
            .unwrap();
        assert!(matches!(
            store.claim_snapshot_read(
                &turn,
                request("m1", "call:conflict", 9),
                "principal:test",
                ClockSample {
                    wall: WallTimeMs::new(300),
                    wait_budget: clock().wait_budget,
                },
            ),
            Err(StoreError::ClaimConflict)
        ));
        let last_wall: i64 = store
            .connection
            .query_row(
                "SELECT last_wall_ms FROM subjects WHERE subject_id='subject:test'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(last_wall, 300);
        store
            .revoke_trusted_grant(
                &GrantId::parse("grant:read").unwrap(),
                Revision::new(1),
                clock(),
            )
            .unwrap();
        let result = store
            .commit_snapshot_result(&child, b"abc", clock())
            .unwrap();
        assert_eq!(result.bytes, b"abc");
        let raw_rollback=br#"{"method":"item/tool/call","id":10,"params":{"threadId":"thread:test","turnId":"turn:test","callId":"call:rollback","namespace":"workspace","tool":"snapshot_read","arguments":{"snapshot_id":"snapshot:test","member_id":"m1","offset":0,"length":3}}}"#;
        store
            .record_native_ingress(
                &NativeSessionId::parse("session:test").unwrap(),
                "transport:test",
                10,
                raw_rollback,
                "qualified",
                Some("item/tool/call"),
                Some("thread:test"),
                Some("turn:test"),
                None,
                ClockSample {
                    wall: WallTimeMs::new(150),
                    wait_budget: clock().wait_budget,
                },
            )
            .unwrap();
        let mut rollback_request = request("m1", "call:rollback", 10);
        rollback_request.grant = GrantId::parse("grant:read-long").unwrap();
        assert!(matches!(
            store.claim_snapshot_read(
                &turn,
                rollback_request,
                "principal:test",
                ClockSample {
                    wall: WallTimeMs::new(150),
                    wait_budget: clock().wait_budget,
                },
            ),
            Err(StoreError::Rejected)
        ));
        let (owner, last_wall): (String, i64) = store
            .connection
            .query_row(
                "SELECT owner_kind,last_wall_ms FROM subjects WHERE subject_id='subject:test'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((owner.as_str(), last_wall), ("fenced", 300));
        let rollback_entries: i64 = store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM native_operations WHERE call_id='call:rollback'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rollback_entries, 0);
        let NativeOperationClaim::Committed(replay) = store
            .claim_snapshot_read(
                &turn,
                request("m1", "call:test", 1),
                "principal:test",
                clock(),
            )
            .unwrap()
        else {
            panic!("expected exact committed replay")
        };
        assert_eq!(replay.bytes, b"abc");
        let tool_result_text = serde_json::json!({
            "encoding":"utf8", "text":"abc", "sha256":result.sha256.clone(),
            "snapshot_id":"snapshot:test", "member_id":"m1", "offset":0, "length":3
        })
        .to_string();
        let completed_tool = serde_json::json!({
            "type":"dynamicToolCall", "id":"call:test", "namespace":"workspace",
            "tool":"snapshot_read", "status":"completed", "success":true,
            "arguments":{"snapshot_id":"snapshot:test","member_id":"m1","offset":0,"length":3},
            "contentItems":[{"type":"inputText","text":tool_result_text}]
        });
        let final_item = serde_json::json!({"type":"agentMessage","id":"final:test","text":"done"});
        for (sequence, method, value) in [
            (
                4,
                "turn/completed",
                serde_json::json!({"method":"turn/completed","params":{"threadId":"thread:test","turn":{"id":"turn:test","status":"completed"}}}),
            ),
            (
                5,
                "item/completed",
                serde_json::json!({"method":"item/completed","params":{"threadId":"thread:test","turnId":"turn:test","item":completed_tool.clone()}}),
            ),
            (
                6,
                "item/completed",
                serde_json::json!({"method":"item/completed","params":{"threadId":"thread:test","turnId":"turn:test","item":final_item.clone()}}),
            ),
        ] {
            store
                .record_native_ingress(
                    &NativeSessionId::parse("session:test").unwrap(),
                    "transport:test",
                    sequence,
                    &serde_json::to_vec(&value).unwrap(),
                    "qualified",
                    Some(method),
                    Some("thread:test"),
                    Some("turn:test"),
                    None,
                    clock(),
                )
                .unwrap();
        }
        let mut conflicting_history_tool = completed_tool.clone();
        conflicting_history_tool["arguments"]["offset"] = serde_json::json!(1);
        let history = serde_json::json!({"id":7,"result":{"thread":{"id":"thread:test","historyMode":"legacy",
            "turns":[{"id":"turn:test","status":"completed","itemsView":"full",
            "items":[conflicting_history_tool,final_item]}]}}});
        store
            .record_native_ingress(
                &NativeSessionId::parse("session:test").unwrap(),
                "transport:test",
                7,
                &serde_json::to_vec(&history).unwrap(),
                "qualified",
                None,
                None,
                None,
                None,
                clock(),
            )
            .unwrap();
        assert!(matches!(
            store.settle_native_turn(
                &turn,
                "done",
                "observed_complete",
                "transport:test",
                7,
                clock()
            ),
            Err(StoreError::ClaimConflict)
        ));
        let mut conflicting_result_tool = completed_tool;
        conflicting_result_tool["contentItems"][0]["text"] = serde_json::json!("corrupt result");
        let conflicting_result_history = serde_json::json!({"id":8,"result":{"thread":{"id":"thread:test","historyMode":"legacy",
            "turns":[{"id":"turn:test","status":"completed","itemsView":"full",
            "items":[conflicting_result_tool,{"type":"agentMessage","id":"final:test","text":"done"}]}]}}});
        store
            .record_native_ingress(
                &NativeSessionId::parse("session:test").unwrap(),
                "transport:test",
                8,
                &serde_json::to_vec(&conflicting_result_history).unwrap(),
                "qualified",
                None,
                None,
                None,
                None,
                clock(),
            )
            .unwrap();
        assert!(matches!(
            store.settle_native_turn(
                &turn,
                "done",
                "observed_complete",
                "transport:test",
                8,
                clock()
            ),
            Err(StoreError::ClaimConflict)
        ));
        assert_eq!(
            store
                .native_recovery("invocation:test")
                .unwrap()
                .unwrap()
                .class,
            "EnteredUncertain"
        );
        store
            .connection
            .execute(
                "UPDATE subjects SET owner_kind='fenced' WHERE subject_id='subject:test'",
                [],
            )
            .unwrap();
        assert!(matches!(
            store.claim_snapshot_read(
                &turn,
                request("m1", "call:test", 1),
                "principal:test",
                clock()
            ),
            Ok(NativeOperationClaim::Committed(_))
        ));
        store
            .record_native_ingress(
                &NativeSessionId::parse("session:test").unwrap(),
                "transport:test",
                3,
                raw,
                "qualified",
                Some("item/tool/call"),
                Some("thread:test"),
                Some("turn:test"),
                None,
                clock(),
            )
            .unwrap();
        assert!(matches!(
            store.claim_snapshot_read(
                &turn,
                request("m1", "call:test", 3),
                "principal:test",
                clock()
            ),
            Err(StoreError::ClaimConflict)
        ));
        let raw2=br#"{"method":"item/tool/call","id":1,"params":{"threadId":"thread:test","turnId":"turn:test","callId":"call:next","namespace":"workspace","tool":"snapshot_read","arguments":{"snapshot_id":"snapshot:test","member_id":"m1","offset":0,"length":3}}}"#;
        store
            .record_native_ingress(
                &NativeSessionId::parse("session:test").unwrap(),
                "transport:test",
                2,
                raw2,
                "qualified",
                Some("item/tool/call"),
                Some("thread:test"),
                Some("turn:test"),
                None,
                clock(),
            )
            .unwrap();
        assert!(matches!(
            store.claim_snapshot_read(
                &turn,
                request("m1", "call:next", 2),
                "principal:test",
                clock()
            ),
            Err(StoreError::Rejected)
        ));
        assert_eq!(
            store
                .native_recovery("invocation:test")
                .unwrap()
                .unwrap()
                .class,
            "EnteredUncertain"
        );
        assert!(matches!(
            store.settle_native_turn(
                &turn,
                "unsupported final",
                "observed_complete",
                "transport:test",
                99,
                clock()
            ),
            Err(StoreError::ClaimConflict)
        ));
        drop(store);
        let store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        let recovery = store.native_recovery("invocation:test").unwrap().unwrap();
        assert_eq!(recovery.class, "EnteredUncertain");
        assert!(recovery.send_intent);
        assert_eq!(recovery.admitted_children, 1);
        assert_eq!(recovery.unsettled_children, 0);
        assert_eq!(recovery.children[0].class, "completed_settled");
        assert_eq!(recovery.children[0].result_sha256, Some(result.sha256));
        assert_eq!(recovery.history_status, "unavailable");
    }
}
