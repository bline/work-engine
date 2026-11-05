//! Bounded Pi coding composition. The existing Host journal owns child effects;
//! lifecycle owns the already-committed uncertain parent and its current grant.
//! This module does not create a second tool ledger or settle native events.
use crate::{NativeTurnEntry, SqliteLifecycleStore, StoreError};
use lifecycle_core::{ClockSample, NativeSessionId};
use rusqlite::{TransactionBehavior, params};
use serde_json::Value;
use work_engine_types::CodecContract;

pub fn pi_coding_profile_digest() -> String {
    digest(b"pi-1.5.0-coding-host-child-v3:provider-disabled:one-workspace-exec:host-journal")
}
pub fn pi_coding_protocol_digest() -> String {
    digest(b"pi-1.5.0:raw-agent-events+host-intent-result+session-branch:v3")
}
fn digest(bytes: &[u8]) -> String {
    CodecContract::BinaryArtifactV1
        .digest_binary(bytes)
        .expect("binary codec")
        .hex()
}

impl SqliteLifecycleStore {
    /// Serialize invalidation with actual actuator entry/collection. The Host
    /// must have committed its non-replayable intent before calling this API.
    /// The parent send intent is independently durable before BEGIN IMMEDIATE;
    /// a crash or rollback here cannot erase uncertainty or authorize replay.
    /// Cancellation/revocation serialized after entry affects subsequent entry;
    /// it cannot retrospectively make the already-entered effect unentered.
    /// The closure must directly enter the bounded Host actuator, without await
    /// or a queue. This is not a reusable permit or an effect settlement API.
    #[allow(clippy::too_many_arguments)]
    pub fn with_pi_coding_effect<T, E>(
        &mut self,
        parent: &NativeTurnEntry,
        transport: &str,
        sequence: u64,
        intent: &Value,
        clock: ClockSample,
        actuator: impl FnOnce(i64) -> Result<T, E>,
    ) -> Result<Result<T, E>, StoreError> {
        self.verify_identity()?;
        if sequence == 0 || sequence > i64::MAX as u64 {
            return Err(StoreError::Rejected);
        }
        let turn = parent.turn().ok_or(StoreError::ClaimConflict)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let (session, thread, profile, protocol, binary, subject, last_wall, expires_ms): (String,String,String,String,String,String,i64,i64) = tx.query_row(
            "SELECT s.session_id,t.thread_id,s.profile_digest,s.protocol_digest,s.binary_sha256,s.subject_id,sub.last_wall_ms,g.expires_ms FROM native_turns t JOIN native_sessions s ON s.session_id=t.session_id JOIN subjects sub ON sub.subject_id=s.subject_id JOIN grants g ON g.grant_id=t.grant_id WHERE t.invocation_id=?1 AND t.turn_id=?2 AND t.state='entered_uncertain' AND t.send_intent=1 AND t.purpose='domain_work' AND t.profile_digest=s.profile_digest AND s.allocation_state='bound' AND g.revision=t.grant_revision AND g.revoked=0 AND g.issuer=?3 AND g.scope='native_turn' AND g.subject_id=s.subject_id AND g.context_id=s.context_id AND g.build_id=sub.build_id AND g.proof_run_id=sub.proof_run_id AND sub.owner_kind='domain' AND sub.owner_ref IS NULL AND sub.context_id=s.context_id AND g.expires_ms>?4 AND EXISTS(SELECT 1 FROM admin_ownership WHERE singleton=1 AND status='active')",
            params![parent.invocation_id(),turn.as_str(),&self.trusted_issuer,clock.wall.get()],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?)))
            .map_err(|_| StoreError::ClaimConflict)?;
        if thread != parent.thread().as_str()
            || profile != pi_coding_profile_digest()
            || profile != parent.profile_digest()
            || protocol != pi_coding_protocol_digest()
            || binary != crate::PI_SDK_INDEX_SHA256
            || clock.wall.get() < last_wall
            || !crate::native::native_session_current(
                &tx,
                &NativeSessionId::parse(session).map_err(|_| StoreError::Rejected)?,
            )?
            || intent.as_object().is_none_or(|v| v.len() != 5)
            || intent["session_id"] != thread
            || ["job_id", "input_digest", "call_id", "command_sha256"]
                .iter()
                .any(|k| intent[*k].as_str().is_none_or(str::is_empty))
            || ["input_digest", "command_sha256"].iter().any(|k| {
                intent[*k]
                    .as_str()
                    .is_none_or(|s| s.len() != 64 || !s.bytes().all(|c| c.is_ascii_hexdigit()))
            })
            || intent["call_id"].as_str().is_none_or(|s| s.len() >= 128)
        {
            return Err(StoreError::ClaimConflict);
        }
        let raw: Vec<u8> = tx.query_row(
            "SELECT raw FROM native_ingress WHERE transport_session=?1 AND sequence=?2 AND thread_id=?3 AND turn_id=?4 AND decode_status='qualified' AND method='pi.coding_tool_intent'",
            params![transport,sequence,thread,turn.as_str()],|r| r.get(0)).map_err(|_| StoreError::ClaimConflict)?;
        let observed: Value =
            serde_json::from_slice(&raw).map_err(|_| StoreError::ClaimConflict)?;
        if observed["type"] != "pi.coding_tool_intent"
            || observed["invocationId"] != parent.invocation_id()
            || observed["intent"] != *intent
        {
            return Err(StoreError::ClaimConflict);
        }
        let result = actuator(expires_ms);
        tx.execute(
            "UPDATE subjects SET last_wall_ms=MAX(last_wall_ms,?1) WHERE subject_id=?2",
            params![clock.wall.get(), subject],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(result)
    }
}

struct ParentBinding {
    session: String,
    thread: String,
    subject: String,
    purpose: String,
    prompt: Vec<u8>,
    prompt_hash: String,
    expires: i64,
}
type CodingParentRow = (String,String,String,String,Vec<u8>,String,String,String,String,i64,i64);
fn parent_binding(
    tx: &rusqlite::Transaction<'_>,
    parent: &NativeTurnEntry,
    issuer: &str,
    clock: ClockSample,
) -> Result<ParentBinding, StoreError> {
    let turn = parent.turn().ok_or(StoreError::ClaimConflict)?;
    let (session,thread,subject,purpose,prompt,prompt_hash,profile,protocol,binary,last_wall,expires):CodingParentRow=tx.query_row(
      "SELECT s.session_id,t.thread_id,s.subject_id,t.purpose,t.prompt,t.prompt_sha256,s.profile_digest,s.protocol_digest,s.binary_sha256,sub.last_wall_ms,g.expires_ms FROM native_turns t JOIN native_sessions s ON s.session_id=t.session_id JOIN subjects sub ON sub.subject_id=s.subject_id JOIN grants g ON g.grant_id=t.grant_id WHERE t.invocation_id=?1 AND t.turn_id=?2 AND t.state='entered_uncertain' AND t.send_intent=1 AND t.profile_digest=s.profile_digest AND s.allocation_state='bound' AND g.revision=t.grant_revision AND g.revoked=0 AND g.issuer=?3 AND g.scope='native_turn' AND g.subject_id=s.subject_id AND g.context_id=s.context_id AND g.build_id=sub.build_id AND g.proof_run_id=sub.proof_run_id AND ((t.purpose='domain_work' AND sub.owner_kind='domain' AND sub.owner_ref IS NULL AND sub.context_id=s.context_id) OR (t.purpose='successor_rehydrate' AND sub.owner_kind='transition' AND EXISTS(SELECT 1 FROM transitions tr WHERE tr.transition_id=sub.owner_ref AND tr.stage='rehydrating' AND tr.successor_context=s.context_id AND tr.successor_thread=t.thread_id))) AND g.expires_ms>?4 AND EXISTS(SELECT 1 FROM admin_ownership WHERE singleton=1 AND status='active')",
      params![parent.invocation_id(),turn.as_str(),issuer,clock.wall.get()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?))).map_err(|_|StoreError::ClaimConflict)?;
    if thread != parent.thread().as_str()
        || profile != pi_coding_profile_digest()
        || parent.profile_digest() != profile
        || protocol != pi_coding_protocol_digest()
        || binary != crate::PI_SDK_INDEX_SHA256
        || clock.wall.get() < last_wall
        || digest(&prompt) != prompt_hash
        || !crate::native::native_session_current(
            tx,
            &NativeSessionId::parse(session.clone()).map_err(|_| StoreError::Rejected)?,
        )?
    {
        return Err(StoreError::ClaimConflict);
    }
    Ok(ParentBinding {
        session,
        thread,
        subject,
        purpose,
        prompt,
        prompt_hash,
        expires,
    })
}
impl SqliteLifecycleStore {
    /// Current turn authority is held through the exact IPC write; the durable
    /// send CAS has already consumed the sole send opportunity.
    pub fn with_pi_coding_turn_send<T, E>(
        &mut self,
        parent: &NativeTurnEntry,
        clock: ClockSample,
        send: impl FnOnce(i64) -> Result<T, E>,
    ) -> Result<Result<T, E>, StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let binding = parent_binding(&tx, parent, &self.trusted_issuer, clock)?;
        let result = send(binding.expires);
        tx.execute(
            "UPDATE subjects SET last_wall_ms=MAX(last_wall_ms,?1) WHERE subject_id=?2",
            params![clock.wall.get(), binding.subject],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(result)
    }
    /// Fresh allocation and current transition/grant are checked while holding
    /// the exact replacement IPC write. The caller consumes its successor entry
    /// once; reopen never reacquires an existing allocation intent.
    #[allow(clippy::too_many_arguments)]
    pub fn with_pi_coding_reset<T, E>(
        &mut self,
        successor: &crate::NativeSessionEntry,
        transition: &lifecycle_core::TransitionId,
        checkpoint: &str,
        grant: &lifecycle_core::GrantId,
        clock: ClockSample,
        reset: impl FnOnce(i64) -> Result<T, E>,
    ) -> Result<Result<T, E>, StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let expected: Value = serde_json::from_str(checkpoint).map_err(|_| StoreError::Rejected)?;
        if expected["successor_context"] != successor.context().as_str() {
            return Err(StoreError::Rejected);
        }
        let expires:i64=tx.query_row("SELECT g.expires_ms FROM subjects sub JOIN transitions tr ON tr.transition_id=sub.owner_ref JOIN grants g ON g.grant_id=?1 JOIN native_turns t ON t.invocation_id=tr.native_basis_source JOIN native_sessions s ON s.session_id=?2 WHERE sub.subject_id=?3 AND sub.owner_kind='transition' AND tr.transition_id=?4 AND tr.stage='switching' AND t.state='completed_settled' AND t.final_text=?5 AND s.subject_id=sub.subject_id AND s.context_id=?11 AND s.allocation_state='intent' AND s.thread_id IS NULL AND s.profile_digest=?6 AND s.protocol_digest=?7 AND s.binary_sha256=?8 AND g.revision=1 AND g.revoked=0 AND g.issuer=?9 AND g.scope='request_replacement' AND g.context_id=sub.context_id AND g.subject_id=sub.subject_id AND g.build_id=sub.build_id AND g.proof_run_id=sub.proof_run_id AND g.expires_ms>?10 AND sub.last_wall_ms<=?10 AND EXISTS(SELECT 1 FROM admin_ownership WHERE singleton=1 AND status='active')",params![grant.as_str(),successor.id().as_str(),successor.subject().as_str(),transition.as_str(),checkpoint.as_bytes(),pi_coding_profile_digest(),pi_coding_protocol_digest(),crate::PI_SDK_INDEX_SHA256,&self.trusted_issuer,clock.wall.get(),successor.context().as_str()],|r|r.get(0)).map_err(|_|StoreError::ClaimConflict)?;
        if successor.thread().is_some() || successor.profile_digest() != pi_coding_profile_digest()
        {
            return Err(StoreError::Rejected);
        }
        let result = reset(expires);
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(result)
    }
    pub fn settle_pi_coding(
        &mut self,
        parent: &NativeTurnEntry,
        proof: &crate::PiSettlementProof,
        clock: ClockSample,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        if proof.first_sequence == 0
            || proof.history_sequence < proof.first_sequence
            || proof.history_sequence - proof.first_sequence > 1024
        {
            return Err(StoreError::Rejected);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let b = parent_binding(&tx, parent, &self.trusted_issuer, clock)?;
        let children:i64=tx.query_row("SELECT (SELECT COUNT(*) FROM native_operations WHERE parent_invocation_id=?1)+(SELECT COUNT(*) FROM native_task_operations WHERE parent_invocation_id=?1)",[parent.invocation_id()],|r|r.get(0)).map_err(|_|StoreError::Unavailable)?;
        if children != 0 {
            return Err(StoreError::ClaimConflict);
        }
        let mut stmt=tx.prepare("SELECT transport_session,sequence,raw,raw_sha256,decode_status,method,observed_wall_ms FROM native_ingress WHERE session_id=?1 AND thread_id=?2 AND turn_id=?3 ORDER BY transport_session,sequence").map_err(|_|StoreError::Unavailable)?;
        let rows = stmt
            .query_map(
                params![b.session, b.thread, parent.turn().unwrap().as_str()],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, u64>(1)?,
                        r.get::<_, Vec<u8>>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, i64>(6)?,
                    ))
                },
            )
            .map_err(|_| StoreError::Unavailable)?;
        let mut values = Vec::new();
        let mut wall = 0;
        for row in rows {
            let (t, s, raw, h, status, method, w) = row.map_err(|_| StoreError::Unavailable)?;
            if t != proof.transport
                || s != proof.first_sequence + values.len() as u64
                || s > proof.history_sequence
                || digest(&raw) != h
                || status != "qualified"
                || w > clock.wall.get()
                || w < wall
            {
                return Err(StoreError::ClaimConflict);
            }
            let v: Value = serde_json::from_slice(&raw).map_err(|_| StoreError::ClaimConflict)?;
            if v["type"] != method {
                return Err(StoreError::ClaimConflict);
            }
            wall = w;
            values.push(v);
        }
        if values.len() as u64 != proof.history_sequence - proof.first_sequence + 1 {
            return Err(StoreError::ClaimConflict);
        }
        let anchor = crate::pi_settlement::settled_anchor(
            &tx,
            parent.invocation_id(),
            &b.session,
            &b.thread,
            &pi_coding_profile_digest(),
            &proof.transport,
            proof.first_sequence,
        )?;
        if b.purpose == "successor_rehydrate"
            && values.iter().any(|v| v["type"] == "pi.coding_tool_intent")
        {
            return Err(StoreError::ClaimConflict);
        }
        let text = crate::pi_coding_proof::validate(
            &values,
            parent.invocation_id(),
            parent.turn().unwrap().as_str(),
            &b.thread,
            &b.prompt,
            &b.prompt_hash,
            anchor.as_ref(),
        )?;
        drop(stmt);
        crate::native::commit_native_settlement(
            &tx,
            parent,
            &text,
            "observed_complete",
            &proof.transport,
            proof.history_sequence,
            wall,
            &b.purpose,
            clock,
        )?;
        tx.commit().map_err(|_| StoreError::Unavailable)
    }
}
