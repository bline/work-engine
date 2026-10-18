//! S5 native parent/child custody in the sole lifecycle SQLite writer.
//! Exact entered work is never a retry invitation after a crash or lost RPC.

use lifecycle_core::{
    AttemptId, ClockSample, ContextGeneration, GrantId, NativeInvocationPurpose, NativeSessionId,
    NativeToolCallId, OperationAttemptId, OperationContractId, OperationExecutorId,
    OperationImplementationId, ProviderThreadId, ProviderTurnId, RuntimeIncarnation, SnapshotId,
    SubjectId, TransitionId,
};
use lifecycle_semantic::{SemanticInvocationBinding, SemanticPhase};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use work_engine_types::CodecContract;

use crate::{SqliteLifecycleStore, StoreError};

/// A bounded read result, not a delivery outcome or a storage failure.
#[derive(Clone, Debug, PartialEq)]
pub enum NativeDeliveryObservation {
    Complete(serde_json::Value),
    CapacityExceeded { limit: usize },
}

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
type NativeDeliveryRow = (
    String,
    String,
    String,
    String,
    String,
    i64,
    Option<String>,
    Option<String>,
    String,
    Vec<u8>,
    String,
);
type PriorTaskOperationRow = (
    String,
    String,
    String,
    String,
    i64,
    String,
    String,
    String,
    String,
    Option<Vec<u8>>,
);
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
    pub fn profile_digest(&self) -> &str {
        &self.profile_digest
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticControlRole {
    Compile,
    Verify,
    SuccessorVerify,
}

impl SemanticControlRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::Compile => "semantic_compile",
            Self::Verify => "semantic_verify",
            Self::SuccessorVerify => "semantic_successor_verify",
        }
    }
}

#[derive(Debug)]
pub struct ControlSessionRequest {
    pub id: NativeSessionId,
    pub subject: SubjectId,
    pub context: ContextGeneration,
    pub transition: TransitionId,
    pub role: SemanticControlRole,
    pub incarnation: RuntimeIncarnation,
    pub profile_digest: String,
    pub binary_sha256: String,
    pub protocol_digest: String,
    pub snapshot: SnapshotId,
    pub manifest_sha256: String,
    pub clock: ClockSample,
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

/// Exact join from a native domain turn to the lifecycle FIFO owner. A
/// control or rehydration turn has no delivery binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeDeliveryBinding {
    pub input_id: String,
    pub delivery_id: String,
    pub effect_id: String,
    pub attempt_id: String,
    pub native_attempt_id: String,
    pub segment_no: u64,
    pub predecessor_invocation_id: Option<String>,
    pub transition_id: Option<String>,
    pub segment_state: String,
    pub text: String,
    pub context_id: String,
}

/// One settled native turn's observed usage. `units=None` means the provider
/// did not supply a complete, disjoint counter set; it never means zero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeUsageAccounting {
    pub invocation_id: String,
    pub purpose: String,
    pub units: Option<NativeUsageUnits>,
    pub raw_events: u64,
}

/// Disjoint units derived from the last cumulative notification for one turn.
/// Control and rehydration turns remain separate rows, so a domain-work total
/// cannot accidentally charge them again.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeUsageUnits {
    pub uncached_input: u64,
    pub cached_input: u64,
    pub cache_write_input: u64,
    pub nonreasoning_output: u64,
    pub reasoning_output: u64,
}

/// Immutable child entry that may already have committed in the task owner
/// when the lifecycle process died before recording its reply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingNativeTaskOperation {
    pub operation_id: String,
    pub contract_id: String,
    pub payload_sha256: String,
    pub entry_sha256: String,
    pub entry_json: Vec<u8>,
    pub grant_id: String,
    pub grant_revision: u64,
}

fn normalized_usage(raw: &[u8]) -> Option<NativeUsageUnits> {
    let event: serde_json::Value = serde_json::from_slice(raw).ok()?;
    let total = &event["params"]["tokenUsage"]["total"];
    let input = total["inputTokens"].as_u64()?;
    let cached_input = total["cachedInputTokens"].as_u64()?;
    let cache_write_input = total["cacheWriteInputTokens"].as_u64()?;
    let output = total["outputTokens"].as_u64()?;
    let reasoning_output = total["reasoningOutputTokens"].as_u64()?;
    let all = total["totalTokens"].as_u64()?;
    if input.checked_add(output)? != all || reasoning_output > output {
        return None;
    }
    let uncached_input = input.checked_sub(cached_input.checked_add(cache_write_input)?)?;
    Some(NativeUsageUnits {
        uncached_input,
        cached_input,
        cache_write_input,
        nonreasoning_output: output - reasoning_output,
        reasoning_output,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeResumeCandidate {
    pub queue_attempt_id: String,
    pub effect_id: String,
    pub text: String,
    pub next_segment_no: u64,
    pub predecessor_invocation_id: String,
    pub transition_id: String,
}

#[derive(Debug)]
pub struct NativeTaskOperationRequest {
    pub operation_id: String,
    pub call: NativeToolCallId,
    pub transport_session: String,
    pub sequence: u64,
    pub contract_id: String,
    pub implementation_id: String,
    pub executor_id: String,
    pub grant: GrantId,
    pub grant_revision: u64,
    pub payload_json: Vec<u8>,
    pub entry_json: Vec<u8>,
}

#[derive(Debug)]
pub enum NativeTaskOperationClaim {
    Entered,
    Committed(Vec<u8>),
    EnteredUncertain,
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
    pub fn pending_native_task_operations(
        &self,
        subject: &SubjectId,
        session: &NativeSessionId,
    ) -> Result<Vec<PendingNativeTaskOperation>, StoreError> {
        self.verify_identity()?;
        let mut statement = self.connection.prepare(
            "SELECT o.operation_id,o.contract_id,o.payload_sha256,o.entry_sha256,o.entry_json,o.grant_id,o.grant_revision \
             FROM native_task_operations o JOIN native_turns n ON n.invocation_id=o.parent_invocation_id \
             JOIN native_sessions s ON s.session_id=n.session_id \
             WHERE s.subject_id=?1 AND s.session_id=?2 AND o.state='entered_uncertain' \
             ORDER BY o.entered_wall_ms,o.operation_id LIMIT 65",
        ).map_err(|_| StoreError::Unavailable)?;
        let rows = statement
            .query_map([subject.as_str(), session.as_str()], |r| {
                Ok(PendingNativeTaskOperation {
                    operation_id: r.get(0)?,
                    contract_id: r.get(1)?,
                    payload_sha256: r.get(2)?,
                    entry_sha256: r.get(3)?,
                    entry_json: r.get(4)?,
                    grant_id: r.get(5)?,
                    grant_revision: r.get(6)?,
                })
            })
            .map_err(|_| StoreError::Unavailable)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StoreError::Unavailable)?;
        if rows.len() > 64 {
            return Err(StoreError::Unavailable);
        }
        Ok(rows)
    }

    pub fn native_delivery_status(
        &self,
        subject: &SubjectId,
    ) -> Result<NativeDeliveryObservation, StoreError> {
        self.verify_identity()?;
        let mut statement=self.connection.prepare(
            "SELECT i.delivery_id,i.input_id,i.sequence,i.state,e.effect_id,a.attempt_id, \
             d.segment_no,d.invocation_id,n.attempt_id,d.predecessor_invocation_id, \
             d.transition_id,d.state,n.state,n.send_intent,n.thread_id,n.turn_id, \
             s.context_id,n.final_sha256,n.history_status \
             FROM native_delivery_segments d JOIN native_turns n ON n.invocation_id=d.invocation_id \
             JOIN native_sessions s ON s.session_id=n.session_id \
             JOIN attempts a ON a.attempt_id=d.queue_attempt_id \
             JOIN effects e ON e.effect_id=a.effect_id JOIN inputs i ON i.input_id=e.input_id \
             WHERE e.subject_id=?1 ORDER BY i.sequence,d.segment_no LIMIT 65",
        ).map_err(|_|StoreError::Unavailable)?;
        let rows = statement
            .query_map([subject.as_str()], |r| {
                Ok(serde_json::json!({
                    "delivery_id":r.get::<_,String>(0)?,"input_id":r.get::<_,String>(1)?,
                    "sequence":r.get::<_,i64>(2)?,"delivery_state":r.get::<_,String>(3)?,
                    "effect_id":r.get::<_,String>(4)?,"queue_attempt_id":r.get::<_,String>(5)?,
                    "segment_no":r.get::<_,i64>(6)?,"invocation_id":r.get::<_,String>(7)?,
                    "native_attempt_id":r.get::<_,String>(8)?,
                    "predecessor_invocation_id":r.get::<_,Option<String>>(9)?,
                    "transition_id":r.get::<_,Option<String>>(10)?,
                    "segment_state":r.get::<_,String>(11)?,"native_state":r.get::<_,String>(12)?,
                    "send_intent":r.get::<_,i64>(13)?==1,
                    "thread_id":r.get::<_,String>(14)?,"turn_id":r.get::<_,Option<String>>(15)?,
                    "context_generation":r.get::<_,String>(16)?,
                    "final_sha256":r.get::<_,Option<String>>(17)?,
                    "history_status":r.get::<_,String>(18)?,
                }))
            })
            .map_err(|_| StoreError::Unavailable)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StoreError::Unavailable)?;
        if rows.len() > 64 {
            return Ok(NativeDeliveryObservation::CapacityExceeded { limit: 64 });
        }
        Ok(NativeDeliveryObservation::Complete(
            serde_json::json!({"protocol_version":1,"subject_id":subject.as_str(),"segments":rows}),
        ))
    }

    pub fn task_successor_intent(
        &self,
        transition: &TransitionId,
    ) -> Result<Option<String>, StoreError> {
        self.verify_identity()?;
        self.connection
            .query_row(
                "SELECT successor_intent_context FROM transitions WHERE transition_id=?1",
                [transition.as_str()],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)
    }

    /// Bind trusted config's next generation to the exact settled predecessor
    /// before successor allocation. A provider text field cannot select it.
    pub fn bind_task_successor_intent(
        &mut self,
        transition: &TransitionId,
        context: &ContextGeneration,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let prior: Option<(Option<String>, String, String)> = tx
            .query_row(
                "SELECT tr.successor_intent_context,tr.native_basis_source,d.state \
             FROM transitions tr JOIN native_delivery_segments d \
             ON d.invocation_id=tr.native_basis_source WHERE tr.transition_id=?1 \
             AND tr.stage NOT IN ('aborted_before_entry','recovery_required')",
                [transition.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        let Some((old, _, state)) = prior else {
            return Err(StoreError::Rejected);
        };
        if state != "yielded" {
            return Err(StoreError::Rejected);
        }
        if let Some(old) = old {
            return if old == context.as_str() {
                Ok(())
            } else {
                Err(StoreError::ClaimConflict)
            };
        }
        tx.execute(
            "UPDATE transitions SET successor_intent_context=?2 \
                    WHERE transition_id=?1 AND successor_intent_context IS NULL",
            params![transition.as_str(), context.as_str()],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)
    }

    /// S8 may use the complete, single owned raw ingress/child chain when the
    /// selected app-server's legacy thread/read omits dynamicToolCall items.
    /// `completed_settled` is minted only by settle_native_turn after checking
    /// every raw started/completed call, exact child result, terminal and final
    /// history text. The omission label remains truthful; this does not infer
    /// invisible provider-internal effects or qualify an untested live provider.
    pub fn task_transition_source_ready(
        &self,
        transition: &TransitionId,
    ) -> Result<bool, StoreError> {
        self.verify_identity()?;
        self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM transitions tr JOIN native_delivery_segments d \
             ON d.invocation_id=tr.native_basis_source JOIN native_turns n ON n.invocation_id=d.invocation_id \
             WHERE tr.transition_id=?1 AND d.state='yielded' AND n.state='completed_settled' \
             AND n.send_intent=1 AND n.terminal_status='completed' \
             AND n.final_text IS NOT NULL AND n.final_sha256 IS NOT NULL \
             AND n.turn_id IS NOT NULL AND n.history_transport_session IS NOT NULL \
             AND n.history_sequence IS NOT NULL \
             AND n.history_status IN ('observed_complete','observed_tool_omitted') \
             AND EXISTS(SELECT 1 FROM native_ingress h WHERE h.transport_session=n.history_transport_session \
                        AND h.sequence=n.history_sequence AND h.decode_status='qualified' AND h.method IS NULL) \
             AND (SELECT COUNT(*) FROM native_ingress c WHERE c.thread_id=n.thread_id \
                  AND c.turn_id=n.turn_id AND c.method='turn/completed' AND c.decode_status='qualified')=1 \
             AND NOT EXISTS(SELECT 1 FROM native_ingress bad WHERE bad.thread_id=n.thread_id \
                            AND bad.turn_id=n.turn_id AND (bad.method IN ('turn/failed','turn/interrupted') \
                            OR bad.decode_status NOT IN ('qualified','denied_before_entry'))) \
             AND (SELECT COUNT(*) FROM native_ingress c WHERE c.thread_id=n.thread_id \
                  AND c.turn_id=n.turn_id AND c.method='item/tool/call') = \
                 (SELECT COUNT(*) FROM native_operations o WHERE o.parent_invocation_id=n.invocation_id) + \
                 (SELECT COUNT(*) FROM native_task_operations o WHERE o.parent_invocation_id=n.invocation_id) \
             AND NOT EXISTS(SELECT 1 FROM native_operations o WHERE o.parent_invocation_id=n.invocation_id \
                            AND o.state!='completed_settled') \
             AND NOT EXISTS(SELECT 1 FROM native_task_operations o WHERE o.parent_invocation_id=n.invocation_id \
                            AND o.state!='completed_settled'))",
            [transition.as_str()],|r|r.get(0),
        ).map_err(|_|StoreError::Unavailable)
    }

    pub fn native_task_operation_entry(
        &self,
        operation_id: &str,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.verify_identity()?;
        self.connection
            .query_row(
                "SELECT entry_json FROM native_task_operations WHERE operation_id=?1",
                [operation_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)
    }

    /// Admit one exact task child after qualified native ingress and a current
    /// grant. The owner callback is invoked only after this row commits.
    pub fn claim_native_task_operation(
        &mut self,
        parent: &NativeTurnEntry,
        request: &NativeTaskOperationRequest,
        principal: &str,
        clock: ClockSample,
    ) -> Result<NativeTaskOperationClaim, StoreError> {
        self.verify_identity()?;
        let scope = match request.contract_id.as_str() {
            "task.question.open.v1" => "task_question_open",
            "task.result.publish.v1" => "task_result_publish",
            _ => return Err(StoreError::Rejected),
        };
        if request.operation_id.is_empty()
            || request.operation_id.len() > 192
            || request.sequence == 0
            || request.sequence > i64::MAX as u64
            || request.payload_json.is_empty()
            || request.payload_json.len() > 8192
            || request.entry_json.is_empty()
            || request.entry_json.len() > 8192
            || request.implementation_id
                != format!(
                    "rust.{}.v1",
                    if scope == "task_question_open" {
                        "question.open"
                    } else {
                        "result.publish"
                    }
                )
            || request.executor_id != "lifecycle.service.task"
        {
            return Err(StoreError::Rejected);
        }
        let turn = parent.turn.as_ref().ok_or(StoreError::ClaimConflict)?;
        let payload: serde_json::Value =
            serde_json::from_slice(&request.payload_json).map_err(|_| StoreError::Rejected)?;
        let (subject, context, native_state, segment_state): (String, String, String, String) =
            self.connection
                .query_row(
                    "SELECT s.subject_id,s.context_id,n.state,d.state FROM native_turns n \
             JOIN native_sessions s ON s.session_id=n.session_id \
             JOIN native_delivery_segments d ON d.invocation_id=n.invocation_id \
             WHERE n.invocation_id=?1 AND n.thread_id=?2 AND n.turn_id=?3",
                    params![parent.invocation_id, parent.thread.as_str(), turn.as_str()],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .map_err(|_| StoreError::ClaimConflict)?;
        if native_state != "entered_uncertain" || segment_state != "entered" {
            return Err(StoreError::ClaimConflict);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let raw: Vec<u8> = tx.query_row(
            "SELECT raw FROM native_ingress WHERE transport_session=?1 AND sequence=?2 \
             AND decode_status='qualified' AND method='item/tool/call' AND thread_id=?3 AND turn_id=?4",
            params![request.transport_session,i64::try_from(request.sequence).map_err(|_|StoreError::Rejected)?,parent.thread.as_str(),turn.as_str()],
            |r|r.get(0),
        ).map_err(|_| StoreError::ClaimConflict)?;
        let observed: serde_json::Value =
            serde_json::from_slice(&raw).map_err(|_| StoreError::ClaimConflict)?;
        let expected_tool = if scope == "task_question_open" {
            "question_open"
        } else {
            "result_publish"
        };
        if observed["params"]["callId"] != request.call.as_str()
            || observed["params"]["namespace"] != "task"
            || observed["params"]["tool"] != expected_tool
            || observed["params"]["arguments"] != payload
        {
            return Err(StoreError::ClaimConflict);
        }
        let payload_sha256 = sha256(&request.payload_json)?;
        let entry_sha256 = sha256(&request.entry_json)?;
        let prior: Option<PriorTaskOperationRow> = tx
            .query_row(
                "SELECT parent_invocation_id,call_id,contract_id,grant_id,grant_revision, \
             payload_sha256,entry_sha256,implementation_id,executor_id,owner_commit_json \
             FROM native_task_operations WHERE operation_id=?1",
                [&request.operation_id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                        r.get(9)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        if let Some(row) = prior {
            if row.0 != parent.invocation_id
                || row.1 != request.call.as_str()
                || row.2 != request.contract_id
                || row.3 != request.grant.as_str()
                || row.4
                    != i64::try_from(request.grant_revision).map_err(|_| StoreError::Rejected)?
                || row.5 != payload_sha256
                || row.6 != entry_sha256
                || row.7 != request.implementation_id
                || row.8 != request.executor_id
            {
                return Err(StoreError::ClaimConflict);
            }
            return Ok(match row.9 {
                Some(bytes) => NativeTaskOperationClaim::Committed(bytes),
                None => NativeTaskOperationClaim::EnteredUncertain,
            });
        }
        check_grant(
            &tx,
            GrantExpectation {
                issuer: &self.trusted_issuer,
                principal,
                grant: &request.grant,
                revision: request.grant_revision,
                subject: &subject,
                context: &context,
                scope,
                clock,
                successor_rehydrate: false,
            },
        )?;
        tx.execute(
            "INSERT INTO native_task_operations(operation_id,parent_invocation_id,call_id,transport_session,transport_sequence,contract_id,implementation_id,executor_id,grant_id,grant_revision,payload_sha256,entry_sha256,entry_json,state,entered_wall_ms) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'entered_uncertain',?14)",
            params![request.operation_id,parent.invocation_id,request.call.as_str(),request.transport_session,
                i64::try_from(request.sequence).map_err(|_|StoreError::Rejected)?,request.contract_id,
                request.implementation_id,request.executor_id,request.grant.as_str(),
                i64::try_from(request.grant_revision).map_err(|_|StoreError::Rejected)?,
                payload_sha256,entry_sha256,request.entry_json,clock.wall.get()],
        ).map_err(|_|StoreError::ClaimConflict)?;
        tx.execute("INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'native_task_operation_entered',?2,?3)",
            params![subject,request.operation_id,clock.wall.get()]).map_err(|_|StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(NativeTaskOperationClaim::Entered)
    }

    pub fn commit_native_task_operation(
        &mut self,
        operation_id: &str,
        owner_commit_json: &[u8],
        clock: ClockSample,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        if owner_commit_json.is_empty() || owner_commit_json.len() > 8192 {
            return Err(StoreError::Rejected);
        }
        let parsed: serde_json::Value =
            serde_json::from_slice(owner_commit_json).map_err(|_| StoreError::Rejected)?;
        if parsed["operation_id"] != operation_id {
            return Err(StoreError::Rejected);
        }
        let digest = sha256(owner_commit_json)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let row: (String, Option<Vec<u8>>, String) = tx
            .query_row(
                "SELECT o.state,o.owner_commit_json,s.subject_id FROM native_task_operations o \
             JOIN native_turns n ON n.invocation_id=o.parent_invocation_id \
             JOIN native_sessions s ON s.session_id=n.session_id WHERE o.operation_id=?1",
                [operation_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|_| StoreError::Rejected)?;
        if row.0 == "completed_settled" {
            return if row.1.as_deref() == Some(owner_commit_json) {
                Ok(())
            } else {
                Err(StoreError::ClaimConflict)
            };
        }
        if row.0 != "entered_uncertain" {
            return Err(StoreError::ClaimConflict);
        }
        tx.execute("UPDATE native_task_operations SET state='completed_settled',owner_commit_json=?2,owner_commit_sha256=?3,settled_wall_ms=?4 WHERE operation_id=?1 AND state='entered_uncertain'",
            params![operation_id,owner_commit_json,digest,clock.wall.get()]).map_err(|_|StoreError::Unavailable)?;
        tx.execute("INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'native_task_operation_settled',?2,?3)",
            params![row.2,operation_id,clock.wall.get()]).map_err(|_|StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)
    }

    /// A reconciled transition can resume the still-entered logical delivery
    /// only after an explicit settled yield and before any later queue input.
    pub fn native_resume_candidate(
        &self,
        subject: &SubjectId,
    ) -> Result<Option<NativeResumeCandidate>, StoreError> {
        self.verify_identity()?;
        let row: Option<(String,String,Vec<u8>,i64,String,String)> = self.connection.query_row(
            "SELECT a.attempt_id,e.effect_id,i.text,d.segment_no,d.invocation_id,d.transition_id \
             FROM native_delivery_segments d JOIN attempts a ON a.attempt_id=d.queue_attempt_id \
             JOIN effects e ON e.effect_id=a.effect_id JOIN inputs i ON i.input_id=e.input_id \
             JOIN transitions tr ON tr.transition_id=d.transition_id \
             JOIN subjects sub ON sub.subject_id=e.subject_id \
             WHERE e.subject_id=?1 AND d.state='yielded' AND tr.stage='reconciled' \
             AND sub.owner_kind='domain' AND sub.context_id=tr.successor_context \
             AND i.state='entered' AND a.state='entered' \
             AND NOT EXISTS(SELECT 1 FROM native_delivery_segments later \
                            WHERE later.queue_attempt_id=a.attempt_id AND later.segment_no>d.segment_no) \
             AND NOT EXISTS(SELECT 1 FROM inputs earlier WHERE earlier.subject_id=i.subject_id \
                            AND earlier.sequence<i.sequence AND earlier.state NOT IN ('succeeded','retired')) \
             ORDER BY i.sequence,d.segment_no DESC LIMIT 1",
            [subject.as_str()],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
        ).optional().map_err(|_| StoreError::Unavailable)?;
        row.map(
            |(
                queue_attempt_id,
                effect_id,
                text,
                segment_no,
                predecessor_invocation_id,
                transition_id,
            )| {
                Ok(NativeResumeCandidate {
                    queue_attempt_id,
                    effect_id,
                    text: String::from_utf8(text).map_err(|_| StoreError::Unavailable)?,
                    next_segment_no: u64::try_from(segment_no + 1)
                        .map_err(|_| StoreError::Unavailable)?,
                    predecessor_invocation_id,
                    transition_id,
                })
            },
        )
        .transpose()
    }

    pub fn native_delivery_binding(
        &self,
        invocation_id: &str,
    ) -> Result<Option<NativeDeliveryBinding>, StoreError> {
        self.verify_identity()?;
        let row: Option<NativeDeliveryRow> = self.connection
            .query_row(
                "SELECT i.input_id,i.delivery_id,e.effect_id,a.attempt_id,n.attempt_id,d.segment_no, \
                 d.predecessor_invocation_id,d.transition_id,d.state,i.text,s.context_id \
                 FROM native_turns n JOIN native_delivery_segments d ON d.invocation_id=n.invocation_id \
                 JOIN native_sessions s ON s.session_id=n.session_id \
                 JOIN attempts a ON a.attempt_id=d.queue_attempt_id \
                 JOIN effects e ON e.effect_id=a.effect_id \
                 JOIN inputs i ON i.input_id=e.input_id \
                 WHERE n.invocation_id=?1 AND n.purpose='domain_work' \
                 AND e.subject_id=s.subject_id",
                [invocation_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?)),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        row.map(
            |(
                input_id,
                delivery_id,
                effect_id,
                attempt_id,
                native_attempt_id,
                segment_no,
                predecessor_invocation_id,
                transition_id,
                segment_state,
                text,
                context_id,
            )| {
                Ok(NativeDeliveryBinding {
                    input_id,
                    delivery_id,
                    effect_id,
                    attempt_id,
                    native_attempt_id,
                    segment_no: u64::try_from(segment_no).map_err(|_| StoreError::Unavailable)?,
                    predecessor_invocation_id,
                    transition_id,
                    segment_state,
                    text: String::from_utf8(text).map_err(|_| StoreError::Unavailable)?,
                    context_id,
                })
            },
        )
        .transpose()
    }

    /// Bind a physical native attempt to one entered FIFO delivery before
    /// send-intent. A missing map never grants permission to replay the turn.
    pub fn bind_native_delivery_segment(
        &mut self,
        invocation: &str,
        queue_attempt: &AttemptId,
        segment_no: u64,
        predecessor: Option<&str>,
        transition: Option<&TransitionId>,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let current: Option<(String, String, String, String, String, String)> = tx
            .query_row(
                "SELECT n.state,n.send_intent,n.purpose,s.subject_id,s.context_id,i.state \
             FROM native_turns n JOIN native_sessions s ON s.session_id=n.session_id \
             JOIN attempts a ON a.attempt_id=?2 JOIN effects e ON e.effect_id=a.effect_id \
             JOIN inputs i ON i.input_id=e.input_id WHERE n.invocation_id=?1 AND n.prompt=i.text \
             AND e.subject_id=s.subject_id AND a.state='entered'",
                params![invocation, queue_attempt.as_str()],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get::<_, i64>(1)?.to_string(),
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        let Some((state, send_intent, purpose, subject, context, input_state)) = current else {
            return Err(StoreError::Rejected);
        };
        if state != "entered_uncertain"
            || send_intent != "0"
            || purpose != "domain_work"
            || input_state != "entered"
        {
            return Err(StoreError::Rejected);
        }
        if segment_no == 0 {
            if predecessor.is_some() || transition.is_some() {
                return Err(StoreError::Rejected);
            }
            let initial: String = tx.query_row(
                "SELECT sub.context_id FROM attempts a JOIN effects e ON e.effect_id=a.effect_id \
                 JOIN subjects sub ON sub.subject_id=e.subject_id WHERE a.attempt_id=?1",
                [queue_attempt.as_str()], |r| r.get(0),
            ).map_err(|_| StoreError::Unavailable)?;
            if initial != context {
                return Err(StoreError::Rejected);
            }
        } else {
            let prior: Option<(String, i64, Option<String>)> = tx
                .query_row(
                    "SELECT d.state,d.segment_no,d.transition_id FROM native_delivery_segments d \
                 WHERE d.invocation_id=?1 AND d.queue_attempt_id=?2",
                    params![
                        predecessor.ok_or(StoreError::Rejected)?,
                        queue_attempt.as_str()
                    ],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
                .map_err(|_| StoreError::Unavailable)?;
            let (prior_state, prior_no, prior_transition) = prior.ok_or(StoreError::Rejected)?;
            let transition = transition.ok_or(StoreError::Rejected)?;
            if prior_state != "yielded"
                || prior_no + 1 != i64::try_from(segment_no).map_err(|_| StoreError::Rejected)?
                || prior_transition.as_deref() != Some(transition.as_str())
            {
                return Err(StoreError::Rejected);
            }
            let ready: i64 = tx.query_row(
                "SELECT COUNT(*) FROM transitions tr JOIN subjects sub ON sub.subject_id=tr.subject_id \
                 WHERE tr.transition_id=?1 AND tr.subject_id=?2 AND tr.stage='reconciled' \
                 AND tr.successor_context=?3 AND sub.owner_kind='domain' AND sub.context_id=?3",
                params![transition.as_str(),subject,context], |r| r.get(0),
            ).map_err(|_| StoreError::Unavailable)?;
            if ready != 1 {
                return Err(StoreError::Rejected);
            }
        }
        tx.execute(
            "INSERT INTO native_delivery_segments(invocation_id,queue_attempt_id,segment_no,predecessor_invocation_id,transition_id,state) \
             VALUES (?1,?2,?3,?4,?5,'entered')",
            params![invocation,queue_attempt.as_str(),i64::try_from(segment_no).map_err(|_| StoreError::Rejected)?,predecessor,transition.map(TransitionId::as_str)],
        ).map_err(|_| StoreError::ClaimConflict)?;
        tx.commit().map_err(|_| StoreError::Unavailable)
    }

    /// Settle a native segment only after its own terminal/history commit.
    pub fn settle_native_delivery_segment(
        &mut self,
        invocation: &str,
        clock: ClockSample,
    ) -> Result<bool, StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let row: (String, String) = tx
            .query_row(
                "SELECT d.state,n.state FROM native_delivery_segments d JOIN native_turns n \
             ON n.invocation_id=d.invocation_id WHERE d.invocation_id=?1",
                [invocation],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|_| StoreError::Rejected)?;
        if row.0 != "entered" || row.1 != "completed_settled" {
            return Err(StoreError::ClaimConflict);
        }
        let transition: Option<String> = tx
            .query_row(
                "SELECT transition_id FROM transitions WHERE native_basis_source=?1 \
             AND stage NOT IN ('aborted_before_entry','recovery_required')",
                [invocation],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "UPDATE native_delivery_segments SET state=?2,transition_id=COALESCE(?3,transition_id),settled_wall_ms=?4 \
             WHERE invocation_id=?1 AND state='entered'",
            params![invocation,if transition.is_some(){"yielded"}else{"completed"},transition,clock.wall.get()],
        ).map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(transition.is_some())
    }

    pub fn native_turn_has_reserved_transition(
        &self,
        invocation_id: &str,
    ) -> Result<bool, StoreError> {
        self.verify_identity()?;
        self.connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM transitions WHERE native_basis_source=?1 \
                 AND stage NOT IN ('aborted_before_entry','recovery_required'))",
                [invocation_id],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)
    }

    /// Hold the lifecycle writer fence while the canonical task owner commits
    /// one already-entered operation. The caller must hold its task-owner fence
    /// first; this is the same lock order as semantic material publication.
    /// A committed owner operation with a lost reply is recovered by its exact
    /// operation ID, not by replaying this callback.
    #[allow(clippy::too_many_arguments)]
    pub fn with_task_grant_fence(
        &mut self,
        subject: &SubjectId,
        context: &ContextGeneration,
        grant: &GrantId,
        revision: u64,
        principal: &str,
        scope: &str,
        clock: ClockSample,
        commit_owner: &mut dyn FnMut() -> Result<(), StoreError>,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        if !matches!(scope, "task_question_open" | "task_result_publish") {
            return Err(StoreError::Rejected);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        check_grant(
            &tx,
            GrantExpectation {
                issuer: &self.trusted_issuer,
                principal,
                grant,
                revision,
                subject: subject.as_str(),
                context: context.as_str(),
                scope,
                clock,
                successor_rehydrate: false,
            },
        )?;
        // No fallible lifecycle mutation follows this owner commit. If our
        // connection fails afterward, the owner receipt is still discoverable.
        commit_owner()?;
        tx.commit().map_err(|_| StoreError::Unavailable)
    }

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
            "SELECT thread_id FROM native_sessions WHERE subject_id=?1 AND context_id=?2 AND role='successor' AND allocation_state='bound' AND thread_id IS NOT NULL",
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
        let semantic_required = self.semantic_transition_required(transition)?;
        if let Some(package) = self.semantic_checkpoint(transition)? {
            return self.connection.query_row(
                "SELECT 'native:'||r.invocation_id FROM transitions tr JOIN native_sessions s ON s.subject_id=tr.subject_id AND s.context_id=tr.successor_context AND s.role='successor' JOIN native_turns r ON r.session_id=s.session_id AND r.thread_id=tr.successor_thread WHERE tr.transition_id=?1 AND tr.stage='rehydrating' AND r.purpose='successor_rehydrate' AND r.state='completed_settled' AND r.prompt=?2 AND r.history_status='observed_complete' ORDER BY r.settled_wall_ms DESC,r.rowid DESC LIMIT 1",
                params![transition.as_str(),package.successor_prompt.as_bytes()],|r|r.get(0),
            ).optional().map_err(|_|StoreError::Unavailable);
        }
        if semantic_required {
            return Ok(None);
        }
        self.connection.query_row(
            "SELECT 'native:'||r.invocation_id FROM transitions tr \
             JOIN native_turns p ON tr.native_basis_source=p.invocation_id \
             JOIN native_sessions s ON s.subject_id=tr.subject_id AND s.context_id=tr.successor_context AND s.role='successor' \
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
        if !crate::admin::admission_open(&self.connection)? {
            return Err(StoreError::Rejected);
        }
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
                let basis: Option<(String,Option<String>,bool)> = self.connection.query_row(
                    "SELECT stage,native_basis_source,semantic_required FROM transitions WHERE transition_id=?1 AND subject_id=?2",
                    params![transition,subject.as_str()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
                ).optional().map_err(|_|StoreError::Unavailable)?;
                if let Some((stage, Some(source), semantic_required)) = basis {
                    if stage != "switching"
                        || (semantic_required
                            && self
                                .semantic_checkpoint(
                                    &TransitionId::parse(transition.clone())
                                        .map_err(|_| StoreError::Rejected)?,
                                )?
                                .is_none())
                    {
                        false
                    } else {
                        let trusted_intent: Option<String> = self.connection.query_row(
                            "SELECT tr.successor_intent_context FROM transitions tr \
                             JOIN native_delivery_segments d ON d.invocation_id=tr.native_basis_source \
                             WHERE tr.transition_id=?1 AND d.state='yielded'",
                            [transition],|r|r.get(0),
                        ).optional().map_err(|_|StoreError::Unavailable)?.flatten();
                        let final_text: Option<Vec<u8>> = self.connection.query_row(
                            "SELECT final_text FROM native_turns WHERE invocation_id=?1 AND state='completed_settled' AND purpose='domain_work'",
                            [source], |r| r.get(0),
                        ).optional().map_err(|_|StoreError::Unavailable)?;
                        final_text.is_some()
                            && if let Some(intent) = trusted_intent {
                                intent == context.as_str()
                            } else {
                                final_text
                                    .and_then(|bytes| {
                                        serde_json::from_slice::<serde_json::Value>(&bytes).ok()
                                    })
                                    .is_some_and(|value| {
                                        value["successor_context"] == context.as_str()
                                    })
                            }
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
            "SELECT session_id,incarnation_id,profile_digest,binary_sha256,protocol_digest,snapshot_id,snapshot_manifest_sha256,thread_id,allocation_state FROM native_sessions WHERE subject_id=?1 AND context_id=?2 AND role IN ('domain','successor')",
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
        tx.execute("INSERT INTO native_sessions(session_id,subject_id,context_id,incarnation_id,profile_digest,binary_sha256,protocol_digest,snapshot_id,snapshot_manifest_sha256,allocation_state,created_wall_ms,role) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,'intent',?10,?11)",
            params![id.as_str(),subject.as_str(),context.as_str(),incarnation.as_str(),profile_digest,binary_sha256,protocol_digest,snapshot.as_str(),manifest_sha256,clock.wall.get(),if admitted_domain {"domain"} else {"successor"}])
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

    /// Transition-owned control contexts are isolated from the one domain or
    /// successor session of a real context. An unbound intent is never retried.
    pub fn claim_control_session(
        &mut self,
        request: ControlSessionRequest,
    ) -> Result<NativeSessionEntry, StoreError> {
        self.verify_identity()?;
        if !crate::admin::admission_open(&self.connection)?
            || [
                &request.profile_digest,
                &request.binary_sha256,
                &request.protocol_digest,
                &request.manifest_sha256,
            ]
            .iter()
            .any(|value| value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err(StoreError::Rejected);
        }
        let stage:Option<String>=self.connection.query_row(
            "SELECT tr.stage FROM transitions tr JOIN subjects sub ON sub.subject_id=tr.subject_id AND sub.owner_kind='transition' AND sub.owner_ref=tr.transition_id WHERE tr.transition_id=?1 AND tr.subject_id=?2 AND ((?4='semantic_successor_verify' AND tr.successor_context=?3 AND tr.stage='rehydrating') OR (?4!='semantic_successor_verify' AND sub.context_id=?3))",
            params![request.transition.as_str(),request.subject.as_str(),request.context.as_str(),request.role.as_str()],|r|r.get(0),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        if !matches!(
            stage.as_deref(),
            Some("verifying" | "checkpointed" | "rehydrating" | "ready_to_commit")
        ) {
            return Err(StoreError::Rejected);
        }
        let prior:Option<PriorNativeSessionRow>=self.connection.query_row(
            "SELECT session_id,incarnation_id,profile_digest,binary_sha256,protocol_digest,snapshot_id,snapshot_manifest_sha256,thread_id,allocation_state FROM native_sessions WHERE control_transition_id=?1 AND role=?2",
            params![request.transition.as_str(),request.role.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?)),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        if let Some((id, inc, p, b, proto, snapshot, manifest, thread, state)) = prior {
            if id != request.id.as_str()
                || inc != request.incarnation.as_str()
                || p != request.profile_digest
                || b != request.binary_sha256
                || proto != request.protocol_digest
                || snapshot != request.snapshot.as_str()
                || manifest != request.manifest_sha256
                || state != "bound"
            {
                return Err(StoreError::ClaimConflict);
            }
            return Ok(NativeSessionEntry {
                id: request.id,
                subject: request.subject,
                context: request.context,
                profile_digest: p,
                snapshot: request.snapshot,
                thread: Some(
                    ProviderThreadId::parse(thread.ok_or(StoreError::ClaimConflict)?)
                        .map_err(|_| StoreError::Unavailable)?,
                ),
            });
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let last_wall: i64 = tx
            .query_row(
                "SELECT last_wall_ms FROM subjects WHERE subject_id=?1",
                [request.subject.as_str()],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Rejected)?;
        if request.clock.wall.get() < last_wall {
            return Err(StoreError::Rejected);
        }
        tx.execute("INSERT INTO native_sessions(session_id,subject_id,context_id,incarnation_id,profile_digest,binary_sha256,protocol_digest,snapshot_id,snapshot_manifest_sha256,allocation_state,created_wall_ms,role,control_transition_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,'intent',?10,?11,?12)",
            params![request.id.as_str(),request.subject.as_str(),request.context.as_str(),request.incarnation.as_str(),
                &request.profile_digest,&request.binary_sha256,&request.protocol_digest,request.snapshot.as_str(),
                &request.manifest_sha256,request.clock.wall.get(),request.role.as_str(),request.transition.as_str()])
            .map_err(|_|StoreError::ClaimConflict)?;
        observe_native_subject_clock(&tx, request.subject.as_str(), request.clock.wall.get())?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(NativeSessionEntry {
            id: request.id,
            subject: request.subject,
            context: request.context,
            profile_digest: request.profile_digest,
            snapshot: request.snapshot,
            thread: None,
        })
    }

    /// Enter one bounded compiler/verifier effect only under the scoped
    /// transition grant. The host binding, not model output, owns identity.
    pub fn claim_control_turn(
        &mut self,
        session: &NativeSessionEntry,
        request: NativeTurnRequest,
        binding: &SemanticInvocationBinding,
        principal: &str,
        clock: ClockSample,
    ) -> Result<NativeTurnEntry, StoreError> {
        self.verify_identity()?;
        if !crate::admin::admission_open(&self.connection)? {
            return Err(StoreError::Rejected);
        }
        let thread = session.thread.as_ref().ok_or(StoreError::ClaimConflict)?;
        let role = match binding.phase {
            SemanticPhase::Compile => "semantic_compile",
            SemanticPhase::CandidateVerification => "semantic_verify",
            SemanticPhase::SuccessorAdequacy => "semantic_successor_verify",
        };
        let expected_purpose = if binding.phase == SemanticPhase::Compile {
            "semantic_compile"
        } else {
            "semantic_verify"
        };
        if request.purpose.as_str() != expected_purpose
            || request.prompt.is_empty()
            || request.prompt.len() > 65_536
            || std::str::from_utf8(&request.prompt).is_err()
            || binding.subject != session.subject
            || binding.session_id != session.id.as_str()
            || binding.thread_id != thread.as_str()
            || binding.invocation_id != request.invocation_id
            || binding.attempt_id != request.attempt.as_str()
            || binding.purpose != request.purpose
            || binding.grant_ref != request.grant.as_str()
            || binding.grant_revision != request.grant_revision
        {
            return Err(StoreError::Rejected);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let current:Option<(String,String,String,String,String,i64)>=tx.query_row(
            "SELECT sub.context_id,sub.build_id,sub.proof_run_id,sub.owner_kind,sub.owner_ref,sub.last_wall_ms FROM subjects sub WHERE sub.subject_id=?1",
            [session.subject.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        let Some((context, build, proof, owner, transition, last_wall)) = current else {
            return Err(StoreError::Rejected);
        };
        if (binding.phase != SemanticPhase::SuccessorAdequacy
            && context != session.context.as_str())
            || owner != "transition"
            || transition != binding.transition.as_str()
            || clock.wall.get() < last_wall
        {
            return Err(StoreError::Rejected);
        }
        let state:Option<(String,String,String)>=tx.query_row(
            "SELECT ns.role,ns.control_transition_id,tr.stage FROM native_sessions ns JOIN transitions tr ON tr.transition_id=ns.control_transition_id WHERE ns.session_id=?1 AND ns.thread_id=?2 AND ns.allocation_state='bound'",
            params![session.id.as_str(),thread.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        let Some((actual_role, actual_transition, stage)) = state else {
            return Err(StoreError::Rejected);
        };
        if actual_role != role
            || actual_transition != binding.transition.as_str()
            || match binding.phase {
                SemanticPhase::Compile | SemanticPhase::CandidateVerification => {
                    stage != "verifying"
                }
                SemanticPhase::SuccessorAdequacy => stage != "rehydrating",
            }
        {
            return Err(StoreError::Rejected);
        }
        if binding.phase == SemanticPhase::SuccessorAdequacy {
            let successor_context:Option<String>=tx.query_row(
                "SELECT successor_context FROM transitions WHERE transition_id=?1 AND subject_id=?2 AND stage='rehydrating'",
                params![binding.transition.as_str(),session.subject.as_str()],|r|r.get(0),
            ).optional().map_err(|_|StoreError::Unavailable)?;
            if successor_context.as_deref() != Some(session.context.as_str()) {
                return Err(StoreError::Rejected);
            }
        }
        let grant:Option<NativeGrantRow>=tx.query_row(
            "SELECT issuer,principal,subject_id,context_id,build_id,proof_run_id,scope,expires_ms,revision,revoked FROM grants WHERE grant_id=?1",
            [request.grant.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?)),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        let Some((issuer, p, s, c, b, pr, scope, expiry, rev, revoked)) = grant else {
            return Err(StoreError::Rejected);
        };
        if issuer != self.trusted_issuer
            || p != principal
            || s != session.subject.as_str()
            || c != session.context.as_str()
            || b != build
            || pr != proof
            || scope != "semantic_control"
            || revoked
            || expiry <= clock.wall.get()
            || rev < 0
            || rev as u64 != request.grant_revision
        {
            return Err(StoreError::Rejected);
        }
        let capture_exists:bool=tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM semantic_captures WHERE capture_id=?1 AND transition_id=?2 AND subject_id=?3 AND policy_id=?4 AND eligibility='current')",
            params![&binding.capture_id,binding.transition.as_str(),session.subject.as_str(),&binding.policy_id],|r|r.get(0),
        ).map_err(|_|StoreError::Unavailable)?;
        if !capture_exists {
            return Err(StoreError::Rejected);
        }
        let active:i64=tx.query_row("SELECT COUNT(*) FROM native_turns WHERE session_id=?1 AND state='entered_uncertain'",
            [session.id.as_str()],|r|r.get(0)).map_err(|_|StoreError::Unavailable)?;
        if active != 0 {
            return Err(StoreError::ClaimConflict);
        }
        let prompt_sha256 = sha256(&request.prompt)?;
        tx.execute("INSERT INTO native_turns(invocation_id,session_id,attempt_id,grant_id,grant_revision,purpose,prompt_sha256,prompt,profile_digest,state,thread_id,entered_wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,'entered_uncertain',?10,?11)",
            params![&request.invocation_id,session.id.as_str(),request.attempt.as_str(),request.grant.as_str(),
                request.grant_revision,request.purpose.as_str(),prompt_sha256,&request.prompt,
                &session.profile_digest,thread.as_str(),clock.wall.get()])
            .map_err(|_|StoreError::ClaimConflict)?;
        tx.execute("INSERT INTO semantic_attempts(invocation_id,capture_id,phase,purpose,attempt_id,session_id,input_sha256,schema_id,policy_id,exposure_sha256,grant_id,grant_revision,state) VALUES (?1,?2,?3,?4,?5,?6,?7,'continuation.v1',?8,?9,?10,?11,'entered_uncertain')",
            params![&request.invocation_id,&binding.capture_id,match binding.phase {SemanticPhase::Compile=>"compile",SemanticPhase::CandidateVerification=>"candidate_verify",SemanticPhase::SuccessorAdequacy=>"successor_adequacy"},
                expected_purpose,request.attempt.as_str(),session.id.as_str(),sha256(&request.prompt)?,&binding.policy_id,
                sha256(binding.exposure_id.as_bytes())?,request.grant.as_str(),request.grant_revision])
            .map_err(|_|StoreError::ClaimConflict)?;
        observe_native_subject_clock(&tx, session.subject.as_str(), clock.wall.get())?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(NativeTurnEntry {
            invocation_id: request.invocation_id,
            attempt: request.attempt,
            thread: thread.clone(),
            turn: None,
            profile_digest: session.profile_digest.clone(),
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
        if !crate::admin::admission_open(&self.connection)? {
            return Err(StoreError::Rejected);
        }
        let thread = session.thread.as_ref().ok_or(StoreError::ClaimConflict)?;
        let semantic_prompt = if request.purpose == NativeInvocationPurpose::SuccessorRehydrate {
            let owner:Option<String>=self.connection.query_row(
                "SELECT owner_ref FROM subjects WHERE subject_id=?1 AND owner_kind='transition'",
                [session.subject.as_str()],|r|r.get(0),
            ).optional().map_err(|_|StoreError::Unavailable)?;
            if let Some(owner) = owner {
                let transition = TransitionId::parse(owner).map_err(|_| StoreError::Rejected)?;
                let prompt = self
                    .semantic_checkpoint(&transition)?
                    .map(|package| package.successor_prompt.into_bytes());
                if self.semantic_transition_required(&transition)? && prompt.is_none() {
                    return Err(StoreError::Rejected);
                }
                prompt
            } else {
                None
            }
        } else {
            None
        };
        if request.prompt.is_empty()
            || request.prompt.len()
                > if semantic_prompt.is_some() {
                    65_536
                } else if request.purpose == NativeInvocationPurpose::DomainWork
                    && request
                        .attempt
                        .as_str()
                        .starts_with("native-attempt:delivery:")
                {
                    8192
                } else {
                    2048
                }
            || std::str::from_utf8(&request.prompt).is_err()
        {
            return Err(StoreError::Rejected);
        }
        let mut tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        if request.purpose == NativeInvocationPurpose::SuccessorRehydrate {
            let source_text: Vec<u8> = if let Some(prompt) = semantic_prompt {
                prompt
            } else {
                tx.query_row(
                "SELECT n.final_text FROM subjects sub JOIN transitions tr ON tr.transition_id=sub.owner_ref \
                 JOIN native_turns n ON tr.native_basis_source=n.invocation_id \
                 WHERE sub.subject_id=?1 AND sub.owner_kind='transition' AND tr.stage='rehydrating' \
                 AND tr.successor_context=?2 AND tr.successor_thread=?3 AND n.state='completed_settled'",
                params![session.subject.as_str(),session.context.as_str(),thread.as_str()], |r| r.get(0),
            ).map_err(|_|StoreError::Rejected)?
            };
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
        let n=self.connection.execute("UPDATE native_turns SET send_intent=1 WHERE invocation_id=?1 AND send_intent=0 AND state='entered_uncertain' AND EXISTS(SELECT 1 FROM admin_ownership WHERE singleton=1 AND status='active')",
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
        let (subject,bound_thread,allocation_state): (String,Option<String>,String) = tx
            .query_row(
                "SELECT subject_id,thread_id,allocation_state FROM native_sessions WHERE session_id=?1",
                [session.as_str()],
                |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
            )
            .map_err(|_| StoreError::ClaimConflict)?;
        if let Some(thread) = thread {
            if bound_thread.as_deref().is_some_and(|bound| bound != thread)
                || (bound_thread.is_none() && allocation_state != "intent")
            {
                return Err(StoreError::Rejected);
            }
            let other: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM native_sessions WHERE thread_id=?1 AND session_id!=?2",
                    params![thread, session.as_str()],
                    |r| r.get(0),
                )
                .map_err(|_| StoreError::Unavailable)?;
            if other != 0 {
                return Err(StoreError::Rejected);
            }
        }
        if let Some(turn) = turn {
            let wrong:i64=tx.query_row(
                "SELECT COUNT(*) FROM native_turns WHERE turn_id=?1 AND (session_id!=?2 OR thread_id!=?3)",
                params![turn,session.as_str(),thread],|r|r.get(0),
            ).map_err(|_| StoreError::Unavailable)?;
            if wrong != 0 {
                return Err(StoreError::Rejected);
            }
        }
        let prior:Option<(String,Vec<u8>,Option<String>)>=tx.query_row("SELECT raw_sha256,raw,session_id FROM native_ingress WHERE transport_session=?1 AND sequence=?2",
            params![transport,sequence],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))
            .optional().map_err(|_| StoreError::Unavailable)?;
        let mut new_incident = false;
        let mut outcome = if let Some((old_digest, old_raw, old_session)) = prior {
            if old_session
                .as_deref()
                .is_some_and(|id| id != session.as_str())
            {
                return Err(StoreError::ClaimConflict);
            }
            if old_digest == digest && old_raw == raw {
                NativeIngressApply::Duplicate
            } else {
                let inserted=tx.execute("INSERT OR IGNORE INTO native_ingress_conflicts(transport_session,sequence,raw_sha256,raw,observed_wall_ms) VALUES (?1,?2,?3,?4,?5)",
                    params![transport,sequence,digest,raw,clock.wall.get()]).map_err(|_| StoreError::Unavailable)?;
                new_incident = inserted == 1;
                NativeIngressApply::ConflictFenced
            }
        } else {
            tx.execute("INSERT INTO native_ingress(transport_session,sequence,raw_sha256,raw,decode_status,method,thread_id,turn_id,item_id,observed_wall_ms,session_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![transport,sequence,digest,raw,decode_status,method,thread,turn,item,clock.wall.get(),session.as_str()])
                .map_err(|_| StoreError::Unavailable)?;
            NativeIngressApply::Inserted
        };
        // A new transport coordinate can still contradict a turn whose final
        // outcome was already settled. Keep both raw observations and the
        // original result, but fence every dependent future admission.
        if outcome == NativeIngressApply::Inserted
            && decode_status == "qualified"
            && let (Some(thread), Some(turn)) = (thread, turn)
        {
            let settled: Option<(String, Option<String>)> = tx
                .query_row(
                    "SELECT t.state,old.raw_sha256 FROM native_turns t LEFT JOIN native_ingress old ON old.transport_session=t.history_transport_session AND old.sequence=t.history_sequence WHERE t.session_id=?1 AND t.thread_id=?2 AND t.turn_id=?3",
                    params![session.as_str(), thread, turn],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(|_| StoreError::Unavailable)?;
            if let Some((state, settled_digest)) = settled {
                let contrary = state == "completed_settled"
                    && (matches!(
                        method,
                        Some("turn/failed" | "turn/interrupted" | "item/tool/call")
                    ) || (method == Some("turn/completed")
                        && settled_digest.as_deref() != Some(digest.as_str())));
                if contrary {
                    tx.execute("INSERT OR IGNORE INTO native_ingress_conflicts(transport_session,sequence,raw_sha256,raw,observed_wall_ms) VALUES (?1,?2,?3,?4,?5)",
                        params![transport,sequence,digest,raw,clock.wall.get()]).map_err(|_| StoreError::Unavailable)?;
                    new_incident = true;
                    outcome = NativeIngressApply::ConflictFenced;
                }
            }
        }
        if new_incident {
            if let (Some(thread), Some(turn)) = (thread, turn) {
                tx.execute("UPDATE native_turns SET state='blocked_conflicted',history_status='conflicted' WHERE thread_id=?1 AND turn_id=?2",
                    params![thread,turn]).map_err(|_| StoreError::Unavailable)?;
            }
            crate::sqlite::fence_subject(
                &tx,
                &subject,
                &format!("native:ingress:{transport}:{sequence}"),
                clock,
            )?;
        }
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
        let incomplete_task:i64=tx.query_row("SELECT COUNT(*) FROM native_task_operations WHERE parent_invocation_id=?1 AND state!='completed_settled'",
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
        let task_children: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM native_task_operations WHERE parent_invocation_id=?1",
                [entry.invocation_id.as_str()],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        let task_profile: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM native_delivery_segments WHERE invocation_id=?1)",
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
            || incomplete_task != 0
            || calls != children + task_children
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
                    if !((item["namespace"] == "workspace" && item["tool"] == "snapshot_read")
                        || (task_profile
                            && item["namespace"] == "task"
                            && matches!(
                                item["tool"].as_str(),
                                Some("question_open" | "result_publish")
                            )))
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
        let mut completed_messages = Vec::<serde_json::Value>::new();
        let mut tool_completions = std::collections::BTreeMap::<String, serde_json::Value>::new();
        for raw in completed_items {
            let value: serde_json::Value =
                serde_json::from_slice(&raw).map_err(|_| StoreError::ClaimConflict)?;
            let item = &value["params"]["item"];
            match item["type"].as_str() {
                Some("agentMessage")
                    if item["text"].as_str().is_some_and(|text| !text.is_empty())
                        && item["questions"].is_null()
                        && item["delivery"].is_null() =>
                {
                    if item["text"] == final_text {
                        final_seen += 1;
                    }
                    // Native 0.163 can emit a progress message before its final
                    // message. Retain every completed message for the full-history
                    // correspondence check below; only the final is settlement.
                    completed_messages.push(item.clone());
                }
                Some("dynamicToolCall") => {
                    let call = item["id"].as_str().ok_or(StoreError::ClaimConflict)?;
                    if !((item["namespace"] == "workspace" && item["tool"] == "snapshot_read")
                        || (task_profile
                            && item["namespace"] == "task"
                            && matches!(
                                item["tool"].as_str(),
                                Some("question_open" | "result_publish")
                            )))
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
            || tool_completions.len() != (children + task_children) as usize
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
        let task_operations = {
            let mut statement=tx.prepare(
                "SELECT o.call_id,o.contract_id,o.owner_commit_json,o.owner_commit_sha256,i.raw \
                 FROM native_task_operations o JOIN native_ingress i \
                 ON i.transport_session=o.transport_session AND i.sequence=o.transport_sequence \
                 WHERE o.parent_invocation_id=?1",
            ).map_err(|_|StoreError::Unavailable)?;
            statement
                .query_map([entry.invocation_id.as_str()], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Vec<u8>>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, Vec<u8>>(4)?,
                    ))
                })
                .map_err(|_| StoreError::Unavailable)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| StoreError::Unavailable)?
        };
        for (call, contract, commit_bytes, digest, raw) in task_operations {
            let item = tool_completions
                .get(&call)
                .ok_or(StoreError::ClaimConflict)?;
            let request: serde_json::Value =
                serde_json::from_slice(&raw).map_err(|_| StoreError::ClaimConflict)?;
            let commit: serde_json::Value =
                serde_json::from_slice(&commit_bytes).map_err(|_| StoreError::ClaimConflict)?;
            let tool = if contract == "task.question.open.v1" {
                "question_open"
            } else {
                "result_publish"
            };
            let content = item["contentItems"]
                .as_array()
                .ok_or(StoreError::ClaimConflict)?;
            if item["namespace"] != "task"
                || item["tool"] != tool
                || item["arguments"] != request["params"]["arguments"]
                || content.len() != 1
                || content[0]["type"] != "inputText"
                || content[0]["text"]
                    .as_str()
                    .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
                    != Some(commit)
                || sha256(&commit_bytes)? != digest
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
        if matches!(
            purpose.as_str(),
            "successor_rehydrate" | "semantic_compile" | "semantic_verify"
        ) {
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
        let mut history_messages = Vec::<serde_json::Value>::new();
        for item in items {
            match item["type"].as_str() {
                Some("dynamicToolCall") => {
                    let call = item["id"].as_str().ok_or(StoreError::ClaimConflict)?;
                    if !((item["namespace"] == "workspace" && item["tool"] == "snapshot_read")
                        || (task_profile
                            && item["namespace"] == "task"
                            && matches!(
                                item["tool"].as_str(),
                                Some("question_open" | "result_publish")
                            )))
                        || item["status"] != "completed"
                        || item["success"] != true
                        || tool_completions.get(call) != Some(item)
                        || !history_tools.insert(call.into())
                    {
                        return Err(StoreError::ClaimConflict);
                    }
                }
                Some("agentMessage")
                    if item["text"].as_str().is_some_and(|text| !text.is_empty())
                        && item["questions"].is_null()
                        && item["delivery"].is_null() =>
                {
                    history_messages.push(item.clone());
                }
                Some("userMessage" | "reasoning") => {}
                _ => return Err(StoreError::ClaimConflict),
            }
        }
        let history_final = items
            .iter()
            .rev()
            .find(|v| v["type"] == "agentMessage")
            .and_then(|v| v["text"].as_str());
        // The persisted full-history read can assign new item IDs. Compare the
        // complete ordered messages, including their content and other fields,
        // while excluding only that transport-local ID.
        let same_message_except_id = |left: &serde_json::Value, right: &serde_json::Value| {
            let (Some(left), Some(right)) = (left.as_object(), right.as_object()) else {
                return false;
            };
            let mut left = left.clone();
            let mut right = right.clone();
            left.remove("id");
            right.remove("id");
            left == right
        };
        let messages_match = completed_messages.len() == history_messages.len()
            && completed_messages
                .iter()
                .zip(&history_messages)
                .all(|(completed, history)| same_message_except_id(completed, history));
        let visible_tools = history_tools.len();
        if !messages_match
            || history_final != Some(final_text)
            || (history_status == "observed_complete"
                && visible_tools != (children + task_children) as usize)
            || (history_status == "observed_tool_omitted"
                && (children + task_children == 0 || visible_tools != 0))
            || (matches!(purpose.as_str(), "semantic_compile" | "semantic_verify")
                && (children != 0 || visible_tools != 0 || history_status != "observed_complete"))
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
        if matches!(purpose.as_str(), "semantic_compile" | "semantic_verify") {
            let updated=tx.execute("UPDATE semantic_attempts SET state='completed_settled',result_sha256=?2 WHERE invocation_id=?1 AND state='entered_uncertain' AND purpose=?3",
                params![entry.invocation_id,&digest,&purpose]).map_err(|_|StoreError::Unavailable)?;
            if updated != 1 {
                return Err(StoreError::ClaimConflict);
            }
        }
        let subject: String = tx.query_row(
            "SELECT s.subject_id FROM native_turns t JOIN native_sessions s ON s.session_id=t.session_id WHERE t.invocation_id=?1",
            [entry.invocation_id.as_str()], |r| r.get(0),
        ).map_err(|_| StoreError::Unavailable)?;
        observe_native_subject_clock(&tx, &subject, clock.wall.get().max(history_wall))?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(())
    }

    /// Query-only accounting. Each invocation contributes at most its last
    /// cumulative notification, never the sum of its notifications. Purpose
    /// stays attached to the row so domain, control and rehydration costs are
    /// disjoint; absent, malformed or cross-transport ambiguous counters
    /// remain unavailable.
    pub fn native_usage_accounting(
        &self,
        subject: &SubjectId,
    ) -> Result<Vec<NativeUsageAccounting>, StoreError> {
        self.verify_identity()?;
        let mut statement = self
            .connection
            .prepare(
            "SELECT t.invocation_id,t.purpose,u.raw_json, \
                (SELECT COUNT(DISTINCT ux.transport_session) FROM native_usage ux WHERE ux.invocation_id=t.invocation_id) \
             FROM native_turns t JOIN native_sessions s ON s.session_id=t.session_id \
             LEFT JOIN native_usage u ON u.invocation_id=t.invocation_id \
             WHERE s.subject_id=?1 \
             ORDER BY t.entered_wall_ms,t.invocation_id,u.transport_session,u.sequence",
            )
            .map_err(|_| StoreError::Unavailable)?;
        let rows = statement
            .query_map([subject.as_str()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<Vec<u8>>>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .map_err(|_| StoreError::Unavailable)?;
        let mut result: Vec<NativeUsageAccounting> = Vec::new();
        for row in rows {
            let (invocation_id, purpose, raw, transport_count) =
                row.map_err(|_| StoreError::Unavailable)?;
            if result
                .last()
                .is_none_or(|last| last.invocation_id != invocation_id)
            {
                result.push(NativeUsageAccounting {
                    invocation_id,
                    purpose,
                    units: None,
                    raw_events: 0,
                });
            }
            if let Some(raw) = raw {
                let last = result.last_mut().ok_or(StoreError::Unavailable)?;
                last.raw_events += 1;
                last.units = if transport_count == 1 {
                    normalized_usage(&raw)
                } else {
                    None
                };
            }
        }
        Ok(result)
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
    if crate::admin::held_quarantine(tx)? {
        return Err(StoreError::ClaimConflict);
    }
    let active: bool = tx
        .query_row(
            "SELECT status='active' FROM admin_ownership WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(|_| StoreError::Unavailable)?;
    if !active {
        return Err(StoreError::ClaimConflict);
    }
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

    #[test]
    fn held_mode_blocks_an_otherwise_valid_new_native_grant() {
        let root = tempdir().unwrap();
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        let subject = SubjectId::parse("new-unrelated-subject").unwrap();
        let context = ContextGeneration::parse("new-unrelated-context").unwrap();
        let build = BuildId::parse("new-unrelated-build").unwrap();
        let proof = ProofRunId::parse("new-unrelated-proof").unwrap();
        store
            .register_subject(
                subject.clone(),
                context.clone(),
                build.clone(),
                proof.clone(),
                clock(),
            )
            .unwrap();
        let grant = GrantId::parse("new-unrelated-valid-grant").unwrap();
        store
            .install_trusted_grant(TrustedGrant {
                id: grant.clone(),
                issuer: "trusted".into(),
                principal: "operator:synthetic".into(),
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
        let expected = GrantExpectation {
            issuer: "trusted",
            principal: "operator:synthetic",
            grant: &grant,
            revision: 1,
            subject: subject.as_str(),
            context: context.as_str(),
            scope: "native_turn",
            clock: clock(),
            successor_rehydrate: false,
        };
        let tx = store.connection.transaction().unwrap();
        assert!(check_grant(&tx, expected).is_ok());
        tx.commit().unwrap();
        // This deliberately simulates an already-valid grant surviving a later
        // held marker. Full held readback would reject the ordinary rows; the
        // entry guard must still deny before any provider effect.
        store.connection.execute("UPDATE legacy_held_state SET mode='held_quarantine',handoff_id='synthetic',source_capture_sha256='capture',capsule_sha256='capsule',inventory_sha256='inventory',row_count=1,unclassified_count=1 WHERE singleton=1",[]).unwrap();
        let tx = store.connection.transaction().unwrap();
        let expected = GrantExpectation {
            issuer: "trusted",
            principal: "operator:synthetic",
            grant: &grant,
            revision: 1,
            subject: subject.as_str(),
            context: context.as_str(),
            scope: "native_turn",
            clock: clock(),
            successor_rehydrate: false,
        };
        assert!(matches!(
            check_grant(&tx, expected),
            Err(StoreError::ClaimConflict)
        ));
    }

    #[test]
    fn native_observation_distinguishes_empty_capacity_and_storage_failure() {
        let root = tempdir().unwrap();
        let store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        let subject = SubjectId::parse("subject:bounded-read").unwrap();
        // Owner-local query fixture: temp relations isolate the exact production
        // SELECT and its row bound, not admission or persisted schema coherence.
        store.connection.execute_batch(
            "CREATE TEMP TABLE native_delivery_segments(queue_attempt_id TEXT,invocation_id TEXT,segment_no INTEGER,predecessor_invocation_id TEXT,transition_id TEXT,state TEXT);
             CREATE TEMP TABLE native_turns(invocation_id TEXT,session_id TEXT,attempt_id TEXT,state TEXT,send_intent INTEGER,thread_id TEXT,turn_id TEXT,final_sha256 TEXT,history_status TEXT);
             CREATE TEMP TABLE native_sessions(session_id TEXT,context_id TEXT);
             CREATE TEMP TABLE attempts(attempt_id TEXT,effect_id TEXT);
             CREATE TEMP TABLE effects(effect_id TEXT,input_id TEXT,subject_id TEXT);
             CREATE TEMP TABLE inputs(input_id TEXT,delivery_id TEXT,sequence INTEGER,state TEXT);
             INSERT INTO native_sessions VALUES ('session','context');
             INSERT INTO attempts VALUES ('queue-attempt','effect');
             INSERT INTO effects VALUES ('effect','input','subject:bounded-read');
             INSERT INTO inputs VALUES ('input','delivery',1,'active');"
        ).unwrap();
        let NativeDeliveryObservation::Complete(empty) =
            store.native_delivery_status(&subject).unwrap()
        else {
            panic!("empty query refused")
        };
        assert_eq!(empty["segments"], serde_json::json!([]));
        for ordinal in 0..65 {
            let invocation = format!("invocation-{ordinal}");
            store.connection.execute("INSERT INTO native_turns VALUES (?1,'session','native-attempt','completed',1,'thread','turn',NULL,'complete')", [&invocation]).unwrap();
            store.connection.execute("INSERT INTO native_delivery_segments VALUES ('queue-attempt',?1,?2,NULL,NULL,'completed')", params![invocation,ordinal]).unwrap();
            if ordinal == 63 {
                let NativeDeliveryObservation::Complete(full) =
                    store.native_delivery_status(&subject).unwrap()
                else {
                    panic!("64 rows refused")
                };
                assert_eq!(full["segments"].as_array().unwrap().len(), 64);
            }
        }
        assert_eq!(
            store.native_delivery_status(&subject).unwrap(),
            NativeDeliveryObservation::CapacityExceeded { limit: 64 }
        );
        store
            .connection
            .execute_batch(
                "ALTER TABLE temp.native_turns RENAME COLUMN thread_id TO missing_thread;",
            )
            .unwrap();
        assert!(matches!(
            store.native_delivery_status(&subject),
            Err(StoreError::Unavailable)
        ));
    }

    #[test]
    fn native_usage_keeps_missing_distinct_from_zero_and_partitions_counters() {
        assert_eq!(normalized_usage(b"{}"), None);
        let raw = |all, input, cached, writes, output, reasoning| {
            serde_json::to_vec(&serde_json::json!({"params":{"tokenUsage":{"total":{
                "totalTokens":all,"inputTokens":input,"cachedInputTokens":cached,
                "cacheWriteInputTokens":writes,"outputTokens":output,
                "reasoningOutputTokens":reasoning
            }}}}))
            .unwrap()
        };
        assert_eq!(
            normalized_usage(&raw(0, 0, 0, 0, 0, 0)),
            Some(NativeUsageUnits {
                uncached_input: 0,
                cached_input: 0,
                cache_write_input: 0,
                nonreasoning_output: 0,
                reasoning_output: 0,
            })
        );
        assert_eq!(
            normalized_usage(&raw(20, 15, 3, 2, 5, 1)),
            Some(NativeUsageUnits {
                uncached_input: 10,
                cached_input: 3,
                cache_write_input: 2,
                nonreasoning_output: 4,
                reasoning_output: 1,
            })
        );
        assert_eq!(normalized_usage(&raw(20, 15, 12, 4, 5, 1)), None);
        assert_eq!(normalized_usage(&raw(20, 15, 3, 2, 5, 6)), None);
        assert_eq!(normalized_usage(&raw(19, 15, 3, 2, 5, 1)), None);
    }

    #[test]
    fn native_usage_accounting_uses_last_cumulative_event_once_per_purpose() {
        let root = tempdir().unwrap();
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        let subject = SubjectId::parse("subject:usage").unwrap();
        store
            .register_subject(
                subject.clone(),
                ContextGeneration::parse("context:usage").unwrap(),
                BuildId::parse("build:usage").unwrap(),
                ProofRunId::parse("proof:usage").unwrap(),
                clock(),
            )
            .unwrap();
        store.connection.execute_batch("\
            INSERT INTO grants VALUES('grant:usage','trusted','principal:usage','subject:usage','context:usage','build:usage','proof:usage','native_turn',100,1,0); \
            INSERT INTO native_sessions(session_id,subject_id,context_id,incarnation_id,profile_digest,binary_sha256,protocol_digest,snapshot_id,snapshot_manifest_sha256,allocation_state,thread_id,created_wall_ms,role) \
              VALUES('session:usage','subject:usage','context:usage','incarnation:usage','profile','binary','protocol','snapshot:usage','manifest','bound','thread:usage',10,'domain'); \
            INSERT INTO native_turns(invocation_id,session_id,attempt_id,grant_id,grant_revision,purpose,prompt_sha256,prompt,profile_digest,state,thread_id,entered_wall_ms) \
              VALUES('invocation:work','session:usage','attempt:work','grant:usage',1,'domain_work','prompt',x'00','profile','completed_settled','thread:usage',11), \
                    ('invocation:control','session:usage','attempt:control','grant:usage',1,'semantic_compile','prompt',x'00','profile','completed_settled','thread:usage',12), \
                    ('invocation:rehydrate','session:usage','attempt:rehydrate','grant:usage',1,'successor_rehydrate','prompt',x'00','profile','completed_settled','thread:usage',13);
        ").unwrap();
        let event = |all, input, output| {
            serde_json::to_vec(&serde_json::json!({"params":{"tokenUsage":{"total":{
                "totalTokens":all,"inputTokens":input,"cachedInputTokens":0,
                "cacheWriteInputTokens":0,"outputTokens":output,"reasoningOutputTokens":0
            }}}}))
            .unwrap()
        };
        for (sequence, invocation, raw) in [
            (1, "invocation:work", event(10, 8, 2)),
            (2, "invocation:work", event(20, 15, 5)),
            (3, "invocation:control", event(0, 0, 0)),
        ] {
            store.connection.execute("INSERT INTO native_ingress(transport_session,sequence,raw_sha256,raw,decode_status,method,thread_id,turn_id,item_id,observed_wall_ms) VALUES('transport:usage',?1,'raw',?2,'qualified','thread/tokenUsage/updated','thread:usage','turn:usage',NULL,?1)",
                params![sequence, &raw]).unwrap();
            store.connection.execute("INSERT INTO native_usage VALUES('transport:usage',?1,?2,'native_cumulative',?3)",
                params![sequence, invocation, raw]).unwrap();
        }
        let rows = store.native_usage_accounting(&subject).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].purpose, "domain_work");
        assert_eq!(rows[0].raw_events, 2);
        assert_eq!(rows[0].units.unwrap().uncached_input, 15);
        assert_eq!(rows[0].units.unwrap().nonreasoning_output, 5);
        assert_eq!(rows[1].purpose, "semantic_compile");
        assert_eq!(rows[1].units.unwrap().uncached_input, 0);
        assert_eq!(rows[2].purpose, "successor_rehydrate");
        assert_eq!(rows[2].units, None);
        assert_eq!(rows[2].raw_events, 0);
        let ambiguous = event(30, 22, 8);
        store.connection.execute("INSERT INTO native_ingress(transport_session,sequence,raw_sha256,raw,decode_status,method,thread_id,turn_id,item_id,observed_wall_ms) VALUES('transport:reconnected',1,'raw',?1,'qualified','thread/tokenUsage/updated','thread:usage','turn:usage',NULL,14)",
            params![&ambiguous]).unwrap();
        store.connection.execute("INSERT INTO native_usage VALUES('transport:reconnected',1,'invocation:work','native_cumulative',?1)",
            params![ambiguous]).unwrap();
        let after_reconnect = store.native_usage_accounting(&subject).unwrap();
        assert_eq!(after_reconnect[0].raw_events, 3);
        assert_eq!(after_reconnect[0].units, None);
    }

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
        let progress_item = serde_json::json!({
            "type":"agentMessage","id":"progress:sequential","text":"Checking source"
        });
        for (sequence, method, value) in [
            (
                1,
                "turn/completed",
                serde_json::json!({"method":"turn/completed","params":{"threadId":"thread:sequential","turn":{"id":"turn:sequential","status":"completed"}}}),
            ),
            (
                2,
                "item/completed",
                serde_json::json!({"method":"item/completed","params":{"threadId":"thread:sequential","turnId":"turn:sequential","item":progress_item.clone()}}),
            ),
            (
                3,
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
        let mut history_progress = progress_item.clone();
        history_progress["id"] = serde_json::json!("persisted:progress");
        let mut history_final = final_item.clone();
        history_final["id"] = serde_json::json!("persisted:final");
        let history = serde_json::json!({"id":4,"result":{"thread":{"id":"thread:sequential","historyMode":"legacy","turns":[{"id":"turn:sequential","status":"completed","itemsView":"full","items":[history_progress,history_final]}]}}});
        store
            .record_native_ingress(
                &session_id,
                "transport:sequential",
                4,
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
                4,
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
        for (item, ingress_method, history_only, ingress_status) in [
            (
                serde_json::json!({"type":"agentMessage","text":"Need answer","questions":[{"id":"q1"}]}),
                "item/completed",
                false,
                "qualified",
            ),
            (
                serde_json::json!({"type":"agentMessage","id":"progress:one","text":"Missing from history"}),
                "item/completed",
                false,
                "qualified",
            ),
            (
                serde_json::json!({"type":"dynamicToolCall","id":"call:running","namespace":"workspace","tool":"snapshot_read","status":"inProgress"}),
                "item/started",
                false,
                "qualified",
            ),
            (
                serde_json::json!({"type":"remoteToolCall","id":"remote:one","status":"completed"}),
                "item/completed",
                true,
                "qualified",
            ),
            (
                serde_json::json!({"type":"compaction","id":"compaction:unexpected"}),
                "thread/compacted",
                false,
                "unsupported",
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
                "qualified",
                serde_json::json!({"method":"turn/completed","params":{"threadId":"thread:adverse","turn":{"id":"turn:adverse","status":"completed"}}}),
            )];
            if !history_only {
                ingress.push((
                    2,
                    ingress_method,
                    ingress_status,
                    serde_json::json!({"method":ingress_method,"params":{"threadId":"thread:adverse","turnId":"turn:adverse","item":item.clone()}}),
                ));
            }
            let final_item =
                serde_json::json!({"type":"agentMessage","id":"final:one","text":"Need answer"});
            ingress.push((
                3,
                "item/completed",
                "qualified",
                serde_json::json!({"method":"item/completed","params":{"threadId":"thread:adverse","turnId":"turn:adverse","item":final_item.clone()}}),
            ));
            for (sequence, method, status, value) in ingress {
                store
                    .record_native_ingress(
                        &NativeSessionId::parse("session:adverse").unwrap(),
                        "transport:adverse",
                        sequence,
                        &serde_json::to_vec(&value).unwrap(),
                        status,
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
