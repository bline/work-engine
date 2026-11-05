//! Tool-free Pi 1.5.0 proof qualification. Raw Pi bytes remain raw Pi bytes.
//! This profile has one prompt, one provider request and one successful assistant
//! response per invocation. More capable profiles require their own owner contract.
use crate::{NativeTurnEntry, SqliteLifecycleStore, StoreError};
use lifecycle_core::ClockSample;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde_json::Value;
use work_engine_types::CodecContract;

pub const PI_SDK_INDEX_SHA256: &str =
    "4569157e6038be9840fa514db4b00a27fd0be9e3622cc7b49a281de6c027d7ff";
pub fn pi_protocol_digest() -> String {
    digest(b"pi-1.5.0:raw-agent-events+provider-request+session-branch:v1:single-tool-free-turn")
}
pub fn pi_profile_digest() -> String {
    digest(b"pi-1.5.0-runtime-fresh-session-v1:provider-disabled:tools-none:owner-settlement-v1")
}
/// Opt-in protocol: immutable settled history is owned, never caller supplied.
pub fn pi_multiturn_protocol_digest() -> String {
    digest(b"pi-1.5.0:raw-agent-events+provider-request+session-branch:v2:anchored-tool-free-turns")
}
pub fn pi_multiturn_profile_digest() -> String {
    digest(b"pi-1.5.0-runtime-anchored-turns-v2:provider-disabled:tools-none:owner-settlement-v2")
}
fn digest(bytes: &[u8]) -> String {
    CodecContract::BinaryArtifactV1
        .digest_binary(bytes)
        .expect("binary codec")
        .hex()
}
/// Coordinates, not a verdict. All proof bytes are read from immutable ingress.
#[derive(Clone, Debug)]
pub struct PiSettlementProof {
    pub transport: String,
    pub first_sequence: u64,
    pub history_sequence: u64,
}
fn reject<T>() -> Result<T, StoreError> {
    Err(StoreError::ClaimConflict)
}
fn text(message: &Value) -> Option<String> {
    if let Some(s) = message["content"].as_str() {
        return Some(s.into());
    }
    let blocks = message["content"].as_array()?;
    let mut result = String::new();
    for block in blocks {
        if block["type"] != "text" {
            return None;
        }
        result.push_str(block["text"].as_str()?);
    }
    Some(result)
}

impl SqliteLifecycleStore {
    /// Verify the immutable selected producer binding before actuator entry.
    pub fn validate_pi_session_binding(
        &self,
        entry: &crate::NativeSessionEntry,
    ) -> Result<(), StoreError> {
        self.validate_pi_binding_for(entry, false)
    }
    pub fn validate_pi_multiturn_session_binding(
        &self,
        entry: &crate::NativeSessionEntry,
    ) -> Result<(), StoreError> {
        self.validate_pi_binding_for(entry, true)
    }
    fn validate_pi_binding_for(
        &self,
        entry: &crate::NativeSessionEntry,
        multiturn: bool,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let profile = if multiturn {
            pi_multiturn_profile_digest()
        } else {
            pi_profile_digest()
        };
        let protocol = if multiturn {
            pi_multiturn_protocol_digest()
        } else {
            pi_protocol_digest()
        };
        let valid:bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM native_sessions WHERE session_id=?1 AND subject_id=?2 AND context_id=?3 AND profile_digest=?4 AND binary_sha256=?5 AND protocol_digest=?6 AND snapshot_id=?7 AND allocation_state='intent' AND thread_id IS NULL)",params![entry.id().as_str(),entry.subject().as_str(),entry.context().as_str(),profile,PI_SDK_INDEX_SHA256,protocol,entry.snapshot().as_str()],|r|r.get(0)).map_err(|_|StoreError::Unavailable)?;
        if !valid
            || entry.profile_digest() != profile
            || !crate::native::native_session_current(&self.connection, entry.id())?
        {
            return Err(StoreError::Rejected);
        }
        Ok(())
    }

    pub fn settle_pi_turn(
        &mut self,
        entry: &NativeTurnEntry,
        proof: &PiSettlementProof,
        clock: ClockSample,
    ) -> Result<(), StoreError> {
        self.settle_pi_turn_for(entry, proof, clock, false)
    }
    pub fn settle_pi_multiturn(
        &mut self,
        entry: &NativeTurnEntry,
        proof: &PiSettlementProof,
        clock: ClockSample,
    ) -> Result<(), StoreError> {
        self.settle_pi_turn_for(entry, proof, clock, true)
    }
    fn settle_pi_turn_for(
        &mut self,
        entry: &NativeTurnEntry,
        proof: &PiSettlementProof,
        clock: ClockSample,
        multiturn: bool,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let turn = entry.turn().ok_or(StoreError::ClaimConflict)?;
        if proof.first_sequence == 0
            || proof.history_sequence < proof.first_sequence
            || proof.history_sequence - proof.first_sequence > 1024
        {
            return reject();
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let (session, thread, profile, binary, protocol, purpose, prompt, prompt_hash, subject, last_wall): (String,String,String,String,String,String,Vec<u8>,String,String,i64) = tx.query_row(
            "SELECT s.session_id,t.thread_id,s.profile_digest,s.binary_sha256,s.protocol_digest,t.purpose,t.prompt,t.prompt_sha256,s.subject_id,sub.last_wall_ms FROM native_turns t JOIN native_sessions s ON s.session_id=t.session_id JOIN subjects sub ON sub.subject_id=s.subject_id JOIN grants g ON g.grant_id=t.grant_id WHERE t.invocation_id=?1 AND t.turn_id=?2 AND t.state='entered_uncertain' AND t.send_intent=1 AND t.profile_digest=s.profile_digest AND s.allocation_state='bound' AND g.revision=t.grant_revision AND g.revoked=0 AND g.issuer=?3 AND g.scope='native_turn' AND g.subject_id=s.subject_id AND g.context_id=s.context_id AND g.build_id=sub.build_id AND g.proof_run_id=sub.proof_run_id AND ((t.purpose='domain_work' AND sub.owner_kind='domain' AND sub.owner_ref IS NULL AND sub.context_id=s.context_id) OR (t.purpose='successor_rehydrate' AND sub.owner_kind='transition' AND EXISTS(SELECT 1 FROM transitions tr WHERE tr.transition_id=sub.owner_ref AND tr.stage='rehydrating' AND tr.successor_context=s.context_id AND tr.successor_thread=t.thread_id))) AND g.expires_ms>?4 AND EXISTS(SELECT 1 FROM admin_ownership WHERE singleton=1 AND status='active')",
            params![entry.invocation_id(),turn.as_str(),&self.trusted_issuer,clock.wall.get()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?))).map_err(|_| StoreError::ClaimConflict)?;
        let expected_profile = if multiturn {
            pi_multiturn_profile_digest()
        } else {
            pi_profile_digest()
        };
        let expected_protocol = if multiturn {
            pi_multiturn_protocol_digest()
        } else {
            pi_protocol_digest()
        };
        if thread != entry.thread().as_str()
            || profile != expected_profile
            || profile != entry.profile_digest()
            || binary != PI_SDK_INDEX_SHA256
            || protocol != expected_protocol
            || digest(&prompt) != prompt_hash
            || clock.wall.get() < last_wall
        {
            return reject();
        }
        if !crate::native::native_session_current(
            &tx,
            &lifecycle_core::NativeSessionId::parse(session.clone())
                .map_err(|_| StoreError::Rejected)?,
        )? {
            return reject();
        }
        // This selected profile disables tools entirely: even a settled child is
        // evidence of a different profile, while unresolved children always fence.
        let children: i64 = tx.query_row("SELECT (SELECT COUNT(*) FROM native_operations WHERE parent_invocation_id=?1)+(SELECT COUNT(*) FROM native_task_operations WHERE parent_invocation_id=?1)", [entry.invocation_id()], |r|r.get(0)).map_err(|_| StoreError::Unavailable)?;
        if children != 0 {
            return reject();
        }
        let mut stmt=tx.prepare("SELECT transport_session,sequence,raw,raw_sha256,decode_status,method,observed_wall_ms FROM native_ingress WHERE session_id=?1 AND thread_id=?2 AND turn_id=?3 ORDER BY transport_session,sequence").map_err(|_| StoreError::Unavailable)?;
        let rows = stmt
            .query_map(params![session, thread, turn.as_str()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, u64>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, i64>(6)?,
                ))
            })
            .map_err(|_| StoreError::Unavailable)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StoreError::Unavailable)?;
        if rows.len() as u64 != proof.history_sequence - proof.first_sequence + 1 {
            return reject();
        }
        let mut values = Vec::new();
        let mut history_wall = 0;
        for (i, (transport, sequence, raw, hash, status, method, wall)) in rows.iter().enumerate() {
            let v: Value = serde_json::from_slice(raw).map_err(|_| StoreError::ClaimConflict)?;
            if transport != &proof.transport
                || *sequence != proof.first_sequence + i as u64
                || digest(raw) != *hash
                || status != "qualified"
                || method.as_deref() != v["type"].as_str()
                || *wall > clock.wall.get()
            {
                return reject();
            }
            history_wall = history_wall.max(*wall);
            values.push(v);
        }
        let anchor = if multiturn {
            settled_anchor(
                &tx,
                entry.invocation_id(),
                &session,
                &thread,
                &profile,
                &proof.transport,
                proof.first_sequence,
            )?
        } else {
            None
        };
        let final_text = validate_with_prefix(
            &values,
            entry.invocation_id(),
            turn.as_str(),
            &thread,
            &prompt,
            &prompt_hash,
            anchor.as_ref(),
        )?;
        if final_text.is_empty() || final_text.len() > 65_536 {
            return reject();
        }
        drop(stmt);
        crate::native::commit_native_settlement(
            &tx,
            entry,
            &final_text,
            "observed_complete",
            &proof.transport,
            proof.history_sequence,
            history_wall,
            &purpose,
            clock,
        )?;
        // Subject identity was obtained from the admitted turn, never supplied by
        // the adapter. Commit helper preserves its durable clock observation.
        let _ = subject;
        tx.commit().map_err(|_| StoreError::Unavailable)
    }
}

fn validate_with_prefix(
    v: &[Value],
    invocation: &str,
    correlation: &str,
    thread: &str,
    prompt: &[u8],
    prompt_hash: &str,
    anchor: Option<&Value>,
) -> Result<String, StoreError> {
    let kinds: Vec<_> = v.iter().map(|v| v["type"].as_str().unwrap_or("")).collect();
    let system = v.get(2).is_some_and(|v| v["message"]["role"] == "system");
    let user_index = if system { 4 } else { 2 };
    let request_index = user_index + 2;
    let mut expected = vec!["agent_start", "turn_start"];
    if system {
        expected.extend(["message_start", "message_end"]);
    }
    expected.extend([
        "message_start",
        "message_end",
        "pi.provider_request",
        "message_start",
    ]);
    let mut i = request_index + 2;
    while kinds.get(i) == Some(&"message_update") {
        expected.push("message_update");
        i += 1;
    }
    expected.extend([
        "message_end",
        "turn_end",
        "agent_end",
        "agent_settled",
        "pi.session_history",
    ]);
    if kinds != expected {
        return reject();
    }
    if anchor.is_some() && system {
        return reject();
    }
    if system
        && (v[2]["message"] != v[3]["message"]
            || v[3]["message"]["toolsAdded"]
                .as_array()
                .is_some_and(|a| !a.is_empty()))
    {
        return reject();
    }
    let user = &v[user_index + 1]["message"];
    let assistant = &v[i]["message"];
    let request = &v[request_index];
    let history = &v[i + 4];
    if user["role"] != "user"
        || v[user_index]["message"] != *user
        || text(user).as_deref().map(str::as_bytes) != Some(prompt)
        || request["producer"] != "pi-1.5.0"
        || request["invocationId"] != invocation
        || request["turnCorrelation"] != correlation
        || request["sessionId"] != thread
        || request["prompt_sha256"] != prompt_hash
        || history["producer"] != "pi-1.5.0"
        || history["sessionId"] != thread
        || assistant["role"] != "assistant"
        || assistant["stopReason"] != "stop"
        || v[request_index + 1]["message"]["role"] != "assistant"
        || v[i + 1]["message"] != *assistant
        || v[i + 1]["toolResults"] != serde_json::json!([])
        || v[i + 2]["willRetry"] != false
        || v[i + 3]["aborted"] != false
        || v[i + 2]["messages"]
            != if system {
                serde_json::json!([v[3]["message"], user, assistant])
            } else {
                serde_json::json!([user, assistant])
            }
    {
        return reject();
    }
    let provider = request["messages"]
        .as_array()
        .ok_or(StoreError::ClaimConflict)?;
    let input = request["agentMessages"]
        .as_array()
        .ok_or(StoreError::ClaimConflict)?;
    if input.last() != Some(user)
        || input
            .iter()
            .any(|m| !matches!(m["role"].as_str(), Some("system" | "user" | "assistant")))
    {
        return reject();
    }
    let normalized: Vec<_> = provider.iter().filter(|m| m["role"] != "system").collect();
    let input_visible: Vec<_> = input.iter().filter(|m| m["role"] != "system").collect();
    if let Some(anchor) = anchor {
        let mut expected = anchor["messages"]
            .as_array()
            .ok_or(StoreError::ClaimConflict)?
            .clone();
        expected.push(user.clone());
        if *input != expected {
            return reject();
        }
    } else if input_visible.len() != 1 || input_visible[0] != user {
        return reject();
    }
    if normalized.len() != input_visible.len()
        || normalized
            .iter()
            .zip(input_visible)
            .any(|(a, b)| a["role"] != b["role"] || text(a) != text(b))
    {
        return reject();
    }
    let mut full = input.clone();
    full.push(assistant.clone());
    let branch = history["branch"]
        .as_array()
        .ok_or(StoreError::ClaimConflict)?;
    if let Some(anchor) = anchor {
        let prior = anchor["branch"]
            .as_array()
            .ok_or(StoreError::ClaimConflict)?;
        if history["header"] != anchor["header"]
            || branch.len() != prior.len() + 2
            || branch[..prior.len()] != prior[..]
            || branch[prior.len()]["type"] != "message"
            || branch[prior.len() + 1]["type"] != "message"
        {
            return reject();
        }
    }
    if history["header"]["id"] != thread || history["header"]["type"] != "session" {
        return reject();
    }
    let mut parent = Value::Null;
    let mut ids = std::collections::BTreeSet::new();
    for row in branch {
        let id = row["id"].as_str().ok_or(StoreError::ClaimConflict)?;
        if row["parentId"] != parent || !ids.insert(id) {
            return reject();
        }
        parent = Value::String(id.into());
    }
    if history["leafId"] != parent {
        return reject();
    }
    let branch_messages: Vec<_> = branch
        .iter()
        .filter(|x| x["type"] == "message")
        .map(|x| x["message"].clone())
        .collect();
    if history["messages"] != Value::Array(full.clone()) || branch_messages != full {
        return reject();
    }
    text(assistant).ok_or(StoreError::ClaimConflict)
}

/// The latest invocation, not a caller-selected checkpoint, determines the
/// prefix. Any unresolved earlier invocation makes continuation ineligible.
fn settled_anchor(
    tx: &rusqlite::Transaction<'_>,
    invocation: &str,
    session: &str,
    thread: &str,
    profile: &str,
    transport: &str,
    first: u64,
) -> Result<Option<Value>, StoreError> {
    let unresolved:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM native_turns WHERE session_id=?1 AND rowid<(SELECT rowid FROM native_turns WHERE invocation_id=?2) AND state!='completed_settled')",params![session,invocation],|r|r.get(0)).map_err(|_|StoreError::Unavailable)?;
    if unresolved {
        return reject();
    }
    let prior:Option<(String,String,u64,String)>=tx.query_row("SELECT thread_id,history_transport_session,history_sequence,profile_digest FROM native_turns WHERE session_id=?1 AND rowid<(SELECT rowid FROM native_turns WHERE invocation_id=?2) AND state='completed_settled' AND history_status='observed_complete' ORDER BY rowid DESC LIMIT 1",params![session,invocation],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|_|StoreError::Unavailable)?;
    let Some((prior_thread, prior_transport, sequence, prior_profile)) = prior else {
        return Ok(None);
    };
    if prior_thread != thread
        || prior_profile != profile
        || (prior_transport == transport && sequence >= first)
    {
        return reject();
    }
    let (raw,hash,method,status):(Vec<u8>,String,String,String)=tx.query_row("SELECT raw,raw_sha256,method,decode_status FROM native_ingress WHERE session_id=?1 AND thread_id=?2 AND transport_session=?3 AND sequence=?4",params![session,thread,prior_transport,sequence],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|_|StoreError::ClaimConflict)?;
    if digest(&raw) != hash || method != "pi.session_history" || status != "qualified" {
        return reject();
    }
    let value: Value = serde_json::from_slice(&raw).map_err(|_| StoreError::ClaimConflict)?;
    if value["type"] != "pi.session_history" || value["sessionId"] != thread {
        return reject();
    }
    Ok(Some(value))
}
