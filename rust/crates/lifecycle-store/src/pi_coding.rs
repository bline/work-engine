//! Bounded Pi coding composition. The existing Host journal owns child effects;
//! lifecycle owns the already-committed uncertain parent and its current grant.
//! This module does not create a second tool ledger or settle native events.
use crate::{NativeTurnEntry, SqliteLifecycleStore, StoreError};
use lifecycle_core::{ClockSample, NativeSessionId};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde_json::Value;
use work_engine_types::CodecContract;

pub fn pi_coding_profile_digest() -> String {
    digest(b"pi-1.5.0-coding-host-child-v3:provider-disabled:one-workspace-exec:host-journal")
}
pub fn pi_coding_protocol_digest() -> String {
    digest(b"pi-1.5.0:raw-agent-events+host-intent-result+session-branch:v3")
}
/// Separately versioned profile for a real provider. Real model text is
/// provenance only; the owner derives and settles the checkpoint (see
/// `pi_coding_proof::validate`). It never aliases the provider-disabled v3 IDs.
pub fn pi_coding_live_profile_digest() -> String {
    digest(b"pi-1.5.0-coding-host-live-continuity-v4:real-provider:one-workspace-exec:host-journal:owner-derived-checkpoint")
}
pub fn pi_coding_live_protocol_digest() -> String {
    digest(
        b"pi-1.5.0:raw-agent-events+host-intent-result+session-branch+owner-derived-checkpoint:v4",
    )
}
/// Profile and protocol must come from the same version; mixed bindings are
/// refused rather than interpreted.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PiCodingKind {
    Fixture,
    Live,
}
pub(crate) fn pi_coding_kind(profile: &str, protocol: &str) -> Option<PiCodingKind> {
    if profile == pi_coding_profile_digest() && protocol == pi_coding_protocol_digest() {
        Some(PiCodingKind::Fixture)
    } else if profile == pi_coding_live_profile_digest()
        && protocol == pi_coding_live_protocol_digest()
    {
        Some(PiCodingKind::Live)
    } else {
        None
    }
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
            || pi_coding_kind(&profile, &protocol).is_none()
            || profile != parent.profile_digest()
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
    kind: PiCodingKind,
}
type CodingParentRow = (
    String,
    String,
    String,
    String,
    Vec<u8>,
    String,
    String,
    String,
    String,
    i64,
    i64,
);
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
    let kind = pi_coding_kind(&profile, &protocol).ok_or(StoreError::ClaimConflict)?;
    if thread != parent.thread().as_str()
        || parent.profile_digest() != profile
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
        kind,
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
        // The successor's own version selects the profile/protocol pair. The
        // basis turn must carry the same profile (below), so a v3 predecessor
        // cannot be recovered into a v4 successor or the reverse.
        let (profile, protocol) = if successor.profile_digest() == pi_coding_profile_digest() {
            (pi_coding_profile_digest(), pi_coding_protocol_digest())
        } else if successor.profile_digest() == pi_coding_live_profile_digest() {
            (
                pi_coding_live_profile_digest(),
                pi_coding_live_protocol_digest(),
            )
        } else {
            return Err(StoreError::Rejected);
        };
        // The caller's checkpoint must equal the committed basis (owner-derived
        // for v4, the committed final text otherwise); anything else is refused.
        let basis: Option<String> = tx
            .query_row(
                "SELECT native_basis_source FROM transitions WHERE transition_id=?1",
                [transition.as_str()],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::ClaimConflict)?;
        let committed = settled_basis_text(&tx, &basis.ok_or(StoreError::ClaimConflict)?)?
            .ok_or(StoreError::ClaimConflict)?;
        if committed != checkpoint.as_bytes() {
            return Err(StoreError::ClaimConflict);
        }
        let expires:i64=tx.query_row("SELECT g.expires_ms FROM subjects sub JOIN transitions tr ON tr.transition_id=sub.owner_ref JOIN grants g ON g.grant_id=?1 JOIN native_turns t ON t.invocation_id=tr.native_basis_source JOIN native_sessions s ON s.session_id=?2 WHERE sub.subject_id=?3 AND sub.owner_kind='transition' AND tr.transition_id=?4 AND tr.stage='switching' AND t.state='completed_settled' AND t.final_text IS NOT NULL AND length(?5)>0 AND s.subject_id=sub.subject_id AND s.context_id=?11 AND s.allocation_state='intent' AND s.thread_id IS NULL AND t.profile_digest=?6 AND s.profile_digest=?6 AND s.protocol_digest=?7 AND s.binary_sha256=?8 AND g.revision=1 AND g.revoked=0 AND g.issuer=?9 AND g.scope='request_replacement' AND g.context_id=sub.context_id AND g.subject_id=sub.subject_id AND g.build_id=sub.build_id AND g.proof_run_id=sub.proof_run_id AND g.expires_ms>?10 AND sub.last_wall_ms<=?10 AND EXISTS(SELECT 1 FROM admin_ownership WHERE singleton=1 AND status='active')",params![grant.as_str(),successor.id().as_str(),successor.subject().as_str(),transition.as_str(),checkpoint.as_bytes(),&profile,&protocol,crate::PI_SDK_INDEX_SHA256,&self.trusted_issuer,clock.wall.get(),successor.context().as_str()],|r|r.get(0)).map_err(|_|StoreError::ClaimConflict)?;
        if successor.thread().is_some() {
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
        let profile_digest = if b.kind == PiCodingKind::Live {
            pi_coding_live_profile_digest()
        } else {
            pi_coding_profile_digest()
        };
        let anchor = crate::pi_settlement::settled_anchor(
            &tx,
            parent.invocation_id(),
            &b.session,
            &b.thread,
            &profile_digest,
            &proof.transport,
            proof.first_sequence,
        )?;
        if b.purpose == "successor_rehydrate"
            && values.iter().any(|v| v["type"] == "pi.coding_tool_intent")
        {
            return Err(StoreError::ClaimConflict);
        }
        let validated = crate::pi_coding_proof::validate(
            &values,
            parent.invocation_id(),
            parent.turn().unwrap().as_str(),
            &b.thread,
            &b.prompt,
            &b.prompt_hash,
            anchor.as_ref(),
            b.kind == PiCodingKind::Live,
        )?;
        // This bounded live profile's sole tool-bearing turn is the initial
        // emission prefix. Together with the committed end coordinate, this
        // fixes both slice boundaries without adding another material store.
        if b.kind == PiCodingKind::Live
            && validated.checkpoint.is_some()
            && proof.first_sequence != 1
        {
            return Err(StoreError::ClaimConflict);
        }
        drop(stmt);
        // The native final fields always carry the actual assistant text. A v4
        // checkpoint is never written there; it is derived from the same
        // immutable evidence (see `live_checkpoint_of`).
        crate::native::commit_native_settlement(
            &tx,
            parent,
            &validated.final_text,
            "observed_complete",
            &proof.transport,
            proof.history_sequence,
            wall,
            &b.purpose,
            clock,
        )?;
        if b.kind == PiCodingKind::Live {
            // Atomic self-check: what any later reader derives from the
            // committed rows must equal what this settlement validated.
            let committed = live_checkpoint_of(&tx, parent.invocation_id())?;
            if committed != validated.checkpoint {
                return Err(StoreError::ClaimConflict);
            }
        }
        tx.commit().map_err(|_| StoreError::Unavailable)
    }
    /// Owner-settled checkpoint of a live v4 tool-bearing turn, read back from
    /// the lifecycle store. It is never caller-supplied: only `settle_pi_coding`
    /// writes it, and only after recomputing it from the exact Host result,
    /// workspace manifest and settled history.
    pub fn pi_coding_live_checkpoint(
        &mut self,
        parent: &NativeTurnEntry,
    ) -> Result<String, StoreError> {
        self.verify_identity()?;
        let turn = parent.turn().ok_or(StoreError::ClaimConflict)?;
        let bound: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM native_turns WHERE invocation_id=?1 AND turn_id=?2 AND thread_id=?3 AND state='completed_settled' AND purpose='domain_work' AND history_status='observed_complete')",
                params![parent.invocation_id(), turn.as_str(), parent.thread().as_str()],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::ClaimConflict)?;
        if !bound {
            return Err(StoreError::ClaimConflict);
        }
        live_checkpoint_of(&self.connection, parent.invocation_id())?.ok_or(StoreError::ClaimConflict)
    }
}

/// Owner-derived checkpoint of a settled live (v4) tool-bearing turn, or
/// `None` for any other turn. No stored copy exists to diverge: the checkpoint
/// is recomputed by the same validator from the immutable ingress rows that the
/// settled invocation references (`history_transport_session`/`history_sequence`
/// and its own session/thread/turn), and the validator's actual final text must
/// equal the committed `native_turns.final_text`.
pub(crate) fn live_checkpoint_of(
    conn: &rusqlite::Connection,
    invocation: &str,
) -> Result<Option<String>, StoreError> {
    type Row = (String, String, String, String, Vec<u8>, String, String, String, Vec<u8>, String, u64);
    let row: Option<Row> = conn
        .query_row(
            "SELECT t.session_id,t.thread_id,t.turn_id,t.purpose,t.prompt,t.prompt_sha256,t.profile_digest,s.protocol_digest,t.final_text,t.history_transport_session,t.history_sequence FROM native_turns t JOIN native_sessions s ON s.session_id=t.session_id WHERE t.invocation_id=?1 AND t.state='completed_settled' AND t.history_status='observed_complete' AND t.final_text IS NOT NULL AND t.turn_id IS NOT NULL AND t.history_transport_session IS NOT NULL AND t.history_sequence IS NOT NULL",
            [invocation],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(6)?, r.get(7)?, r.get(5)?, r.get(8)?, r.get(9)?, r.get(10)?)),
        )
        .optional()
        .map_err(|_| StoreError::Unavailable)?;
    let Some((session, thread, turn, purpose, prompt, profile, protocol, prompt_hash, final_text, transport, last)) = row else {
        return Ok(None);
    };
    if pi_coding_kind(&profile, &protocol) != Some(PiCodingKind::Live) || purpose != "domain_work" {
        return Ok(None);
    }
    let mut stmt = conn
        .prepare("SELECT sequence,raw,raw_sha256,decode_status,method FROM native_ingress WHERE session_id=?1 AND thread_id=?2 AND turn_id=?3 AND transport_session=?4 AND sequence<=?5 ORDER BY sequence")
        .map_err(|_| StoreError::Unavailable)?;
    let rows = stmt
        .query_map(params![session, thread, turn, transport, last], |r| {
            Ok((r.get::<_, u64>(0)?, r.get::<_, Vec<u8>>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?))
        })
        .map_err(|_| StoreError::Unavailable)?;
    let mut values: Vec<Value> = Vec::new();
    let mut first = 0;
    for row in rows {
        let (sequence, raw, hash, status, method) = row.map_err(|_| StoreError::Unavailable)?;
        if values.is_empty() {
            first = sequence;
        }
        if sequence != first + values.len() as u64 || digest(&raw) != hash || status != "qualified" {
            return Err(StoreError::ClaimConflict);
        }
        let v: Value = serde_json::from_slice(&raw).map_err(|_| StoreError::ClaimConflict)?;
        if v["type"] != method {
            return Err(StoreError::ClaimConflict);
        }
        values.push(v);
    }
    if values.is_empty() || first + values.len() as u64 - 1 != last {
        return Err(StoreError::ClaimConflict);
    }
    drop(stmt);
    let anchor = crate::pi_settlement::settled_anchor(conn, invocation, &session, &thread, &profile, &transport, first)?;
    let validated = crate::pi_coding_proof::validate(&values, invocation, &turn, &thread, &prompt, &prompt_hash, anchor.as_ref(), true)?;
    if validated.final_text.as_bytes() != final_text.as_slice()
        || (validated.checkpoint.is_some() && first != 1)
    {
        return Err(StoreError::ClaimConflict);
    }
    Ok(validated.checkpoint)
}

/// Text a transition, rehydration or reset consumes as the settled basis of a
/// domain turn: the owner-derived checkpoint for a v4 tool-bearing turn, the
/// committed final text for every other turn and profile. `None` when the turn
/// is not a settled domain turn.
pub(crate) fn settled_basis_text(
    conn: &rusqlite::Connection,
    invocation: &str,
) -> Result<Option<Vec<u8>>, StoreError> {
    let text: Option<Vec<u8>> = conn
        .query_row(
            "SELECT final_text FROM native_turns WHERE invocation_id=?1 AND state='completed_settled' AND purpose='domain_work'",
            [invocation],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| StoreError::Unavailable)?
        .flatten();
    let Some(text) = text else { return Ok(None) };
    Ok(Some(match live_checkpoint_of(conn, invocation)? {
        Some(checkpoint) => checkpoint.into_bytes(),
        None => text,
    }))
}
