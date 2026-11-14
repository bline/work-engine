//! Immutable observations of compiled native input; no execution or research authority.
use crate::{NativeSessionEntry, NativeTurnEntry, SqliteLifecycleStore, StoreError};
use lifecycle_core::ClockSample;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeInputPayload {
    pub inference_call_id: String,
    pub physical_send: u64,
    pub representation: String,
    pub original_sha256: String,
    pub original_codec: String,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeInputRecord {
    pub invocation_id: String,
    pub session_id: String,
    pub attempt_id: String,
    pub context_id: String,
    pub incarnation_id: String,
    pub profile_digest: String,
    pub binary_sha256: String,
    pub protocol_digest: String,
    pub thread_id: String,
    pub turn_id: String,
    pub delivery: Option<crate::NativeDeliveryBinding>,
    pub transport_session: String,
    pub terminal_sequence: u64,
    pub terminal_raw: Vec<u8>,
    pub metadata: Vec<u8>,
    pub payloads: Vec<NativeInputPayload>,
    pub qualification_scope: String,
    pub eligible: bool,
    pub outcome: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeInputApply {
    Committed,
    Replay,
    Conflict,
}

#[cfg(test)]
thread_local! { pub(crate) static AFTER_PUBLICATION: std::cell::RefCell<Option<Box<dyn FnOnce(&rusqlite::Connection)->Result<(),StoreError>>>> = std::cell::RefCell::new(None); }

impl SqliteLifecycleStore {
    /// The service must supply the acquired adapter value through its actor.
    /// Session/turn entries are minted by this store; no capture labels choose
    /// the lifecycle subject, incarnation, attempt or FIFO segment.
    #[allow(clippy::too_many_arguments)]
    pub fn retain_native_input(
        &mut self,
        session: &NativeSessionEntry,
        turn: &NativeTurnEntry,
        transport: &str,
        sequence: u64,
        terminal: &[u8],
        metadata: &[u8],
        payloads: Vec<NativeInputPayload>,
        eligible: bool,
        outcome: &str,
        clock: ClockSample,
    ) -> Result<NativeInputApply, StoreError> {
        self.verify_identity()?;
        if session.profile_digest() != turn.profile_digest() {
            return Err(StoreError::ClaimConflict);
        }
        if (eligible && metadata.is_empty())
            || metadata.len() > 128 * 1024
            || (eligible && payloads.is_empty())
            || payloads.len() > 64
            || payloads
                .iter()
                .try_fold(0usize, |n, p| n.checked_add(p.bytes.len()))
                .is_none_or(|n| n > 8 * 1024 * 1024)
            || terminal.len() > 1_048_576
        {
            return Err(StoreError::Rejected);
        }
        for p in &payloads {
            if p.original_sha256 != format!("{:x}", Sha256::digest(&p.bytes))
                || p.physical_send == 0
                || !matches!(p.representation.as_str(), "wire" | "decoded")
                || !matches!(p.original_codec.as_str(), "identity" | "zstd")
            {
                return Err(StoreError::Rejected);
            }
        }
        let turn_id = turn.turn().ok_or(StoreError::Rejected)?.as_str();
        let row:(String,String,String,String,String,String,String,String,String)=self.connection.query_row(
            "SELECT s.subject_id,s.context_id,s.incarnation_id,s.profile_digest,s.binary_sha256,s.protocol_digest,n.attempt_id,n.thread_id,n.turn_id FROM native_sessions s JOIN native_turns n ON n.session_id=s.session_id WHERE s.session_id=?1 AND n.invocation_id=?2 AND n.send_intent=1 AND n.profile_digest=s.profile_digest AND s.thread_id=n.thread_id",
            params![session.id().as_str(),turn.invocation_id()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?))).map_err(|_|StoreError::Rejected)?;
        if row.0 != session.subject().as_str()
            || row.1 != session.context().as_str()
            || row.3 != session.profile_digest()
            || row.6 != turn.attempt().as_str()
            || row.7 != turn.thread().as_str()
            || row.8 != turn_id
        {
            return Err(StoreError::ClaimConflict);
        }
        let ingress:(Vec<u8>,String,String,String)=self.connection.query_row(
            "SELECT raw,decode_status,thread_id,turn_id FROM native_ingress WHERE transport_session=?1 AND sequence=?2 AND session_id=?3 AND method IN ('turn/completed','error','turn/failed','turn/interrupted')",
            params![transport,sequence,session.id().as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|_|StoreError::Rejected)?;
        if ingress.0 != terminal
            || (eligible && ingress.1 != "qualified")
            || ingress.2 != row.7
            || ingress.3 != row.8
        {
            return Err(StoreError::ClaimConflict);
        }
        let terminal_value: serde_json::Value =
            serde_json::from_slice(terminal).map_err(|_| StoreError::Rejected)?;
        if eligible
            && (terminal_value["method"] != "turn/completed"
                || terminal_value["params"]["turn"]["status"] != "completed"
                || terminal_value["params"]["turn"].get("error") != Some(&serde_json::Value::Null))
        {
            return Err(StoreError::Rejected);
        }
        let mut record = NativeInputRecord {
            invocation_id: turn.invocation_id().into(),
            session_id: session.id().as_str().into(),
            attempt_id: row.6,
            context_id: row.1,
            incarnation_id: row.2,
            profile_digest: row.3,
            binary_sha256: row.4,
            protocol_digest: row.5,
            thread_id: row.7,
            turn_id: row.8,
            delivery: self.native_delivery_binding(turn.invocation_id())?,
            transport_session: transport.into(),
            terminal_sequence: sequence,
            terminal_raw: terminal.to_vec(),
            metadata: metadata.to_vec(),
            payloads,
            qualification_scope:
                "controlled-native-input-observation-v1; historical evidence; no research authority"
                    .into(),
            eligible,
            outcome: outcome.into(),
        };
        if let Some(prior) = self.native_input_observation(turn.invocation_id())? {
            // Replay preserves the original observed segment state; subsequent
            // delivery settlement does not rebase the retained observation.
            if let (Some(now), Some(old)) = (&mut record.delivery, &prior.delivery) {
                now.segment_state = old.segment_state.clone();
            }
            if record == prior {
                return Ok(NativeInputApply::Replay);
            }
        }
        let bytes = serde_json::to_vec(&record).map_err(|_| StoreError::Unavailable)?;
        // All bytes reach the existing synced artifact owner before an observation
        // reference can become visible. A cut here leaves only an orphan artifact.
        let staged = self.stage_artifact(session.subject().clone(), &bytes)?;
        let published = self.publish_artifact(staged, clock)?;
        #[cfg(test)]
        AFTER_PUBLICATION.with(|slot| match slot.borrow_mut().take() {
            Some(hook) => hook(&self.connection),
            None => Ok(()),
        })?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let association:i64=tx.query_row("SELECT COUNT(*) FROM native_turns n JOIN native_sessions s ON s.session_id=n.session_id JOIN native_ingress i ON i.session_id=s.session_id WHERE n.invocation_id=?1 AND n.attempt_id=?2 AND s.session_id=?3 AND n.turn_id=?4 AND i.transport_session=?5 AND i.sequence=?6 AND i.raw=?7 AND i.thread_id=n.thread_id AND i.turn_id=n.turn_id AND (?8=0 OR i.decode_status='qualified')",params![turn.invocation_id(),turn.attempt().as_str(),session.id().as_str(),turn_id,transport,sequence,terminal,eligible],|r|r.get(0)).map_err(|_|StoreError::Unavailable)?;
        if association != 1 {
            return Err(StoreError::ClaimConflict);
        }
        // Re-read every lifecycle association inside the observation transaction;
        // artifact publication and its fsync are deliberately outside it.
        let current:(String,String,String,String,String,String,String,String,String)=tx.query_row(
            "SELECT s.subject_id,s.context_id,s.incarnation_id,s.profile_digest,s.binary_sha256,s.protocol_digest,n.attempt_id,n.thread_id,n.turn_id FROM native_sessions s JOIN native_turns n ON n.session_id=s.session_id WHERE s.session_id=?1 AND n.invocation_id=?2 AND n.send_intent=1 AND n.profile_digest=s.profile_digest AND s.thread_id=n.thread_id",
            params![session.id().as_str(),turn.invocation_id()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?))).map_err(|_|StoreError::ClaimConflict)?;
        if current
            != (
                session.subject().as_str().into(),
                record.context_id.clone(),
                record.incarnation_id.clone(),
                record.profile_digest.clone(),
                record.binary_sha256.clone(),
                record.protocol_digest.clone(),
                record.attempt_id.clone(),
                record.thread_id.clone(),
                record.turn_id.clone(),
            )
        {
            return Err(StoreError::ClaimConflict);
        }
        let mut delivery = crate::native::native_delivery_binding_on(&tx, turn.invocation_id())?;
        if let (Some(now), Some(old)) = (&mut delivery, &record.delivery) {
            now.segment_state = old.segment_state.clone();
        }
        if delivery != record.delivery {
            return Err(StoreError::ClaimConflict);
        }
        let purpose: String = tx
            .query_row(
                "SELECT purpose FROM native_turns WHERE invocation_id=?1",
                [turn.invocation_id()],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        if purpose == "domain_work" && record.delivery.is_none() {
            return Err(StoreError::ClaimConflict);
        }
        let prior: Option<String> = tx
            .query_row(
                "SELECT artifact_sha256 FROM native_input_observations WHERE invocation_id=?1",
                [turn.invocation_id()],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        let result = if let Some(prior) = prior {
            if prior == published.digest_hex {
                NativeInputApply::Replay
            } else {
                tx.execute("INSERT OR IGNORE INTO native_input_conflicts(invocation_id,artifact_sha256,observed_wall_ms) VALUES(?1,?2,?3)",params![turn.invocation_id(),published.digest_hex,clock.wall.get()]).map_err(|_|StoreError::Unavailable)?;
                NativeInputApply::Conflict
            }
        } else {
            tx.execute("INSERT INTO native_input_observations(invocation_id,session_id,attempt_id,transport_session,terminal_sequence,artifact_sha256,observed_wall_ms) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![turn.invocation_id(),session.id().as_str(),turn.attempt().as_str(),transport,sequence,published.digest_hex,clock.wall.get()]).map_err(|_|StoreError::Unavailable)?;
            NativeInputApply::Committed
        };
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(result)
    }

    /// Historical readback never checks fresh eligibility or authorizes a send.
    pub fn native_input_observation(
        &self,
        invocation: &str,
    ) -> Result<Option<NativeInputRecord>, StoreError> {
        self.verify_identity()?;
        let digest: Option<String> = self
            .connection
            .query_row(
                "SELECT artifact_sha256 FROM native_input_observations WHERE invocation_id=?1",
                [invocation],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        digest
            .map(|digest| {
                let bytes = self
                    .read_committed_artifact(&digest)?
                    .ok_or(StoreError::Unavailable)?;
                let record: NativeInputRecord =
                    serde_json::from_slice(&bytes).map_err(|_| StoreError::Unavailable)?;
                if record.invocation_id != invocation {
                    return Err(StoreError::Unavailable);
                }
                Ok(record)
            })
            .transpose()
    }
}
