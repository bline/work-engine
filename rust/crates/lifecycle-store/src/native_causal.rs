//! Immutable causal companion to accepted native input observations.
use crate::{
    NativeInputApply, NativeSessionEntry, NativeTurnEntry, SqliteLifecycleStore, StoreError,
};
use lifecycle_core::ClockSample;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeCausalRecord {
    pub version: u32,
    pub operation_id: String,
    pub invocation_id: String,
    pub input_artifact_sha256: String,
    pub session_id: String,
    pub call_id: String,
    pub transport_session: String,
    pub transport_sequence: u64,
    pub ingress: Vec<u8>,
    pub entry: Vec<u8>,
    pub owner_commit: Vec<u8>,
    pub bundle: Vec<u8>,
    pub scope: String,
}
#[cfg(test)]
thread_local! {
    pub(crate) static AFTER_PUBLICATION: std::cell::RefCell<bool> = const { std::cell::RefCell::new(false) };
    pub(crate) static BEFORE_TRANSACTION: std::cell::RefCell<Option<Box<dyn FnOnce(&rusqlite::Connection)>>> = std::cell::RefCell::new(None);
    pub(crate) static AFTER_COMMIT: std::cell::RefCell<bool> = const { std::cell::RefCell::new(false) };
}
impl SqliteLifecycleStore {
    /// The actor passes only its adapter-owned acquired bundle. Operation IDs,
    /// ingress, settlement and input references are resolved here from owner rows.
    pub fn retain_native_causal(
        &mut self,
        session: &NativeSessionEntry,
        turn: &NativeTurnEntry,
        bundle: &[u8],
        clock: ClockSample,
    ) -> Result<Vec<NativeInputApply>, StoreError> {
        self.verify_identity()?;
        if bundle.len() > 8 * 1024 * 1024 {
            return Err(StoreError::Rejected);
        }
        let value: Value = serde_json::from_slice(bundle).map_err(|_| StoreError::Rejected)?;
        let calls = value["calls"].as_array().ok_or(StoreError::Rejected)?;
        let input = self
            .native_input_observation(turn.invocation_id())?
            .ok_or(StoreError::Rejected)?;
        if self.native_input_conflicted(turn.invocation_id())?
            || !input.eligible
            || input.session_id != session.id().as_str()
            || input.turn_id != turn.turn().ok_or(StoreError::Rejected)?.as_str()
        {
            return Err(StoreError::ClaimConflict);
        }
        let input_sha: String = self
            .connection
            .query_row(
                "SELECT artifact_sha256 FROM native_input_observations WHERE invocation_id=?1",
                [turn.invocation_id()],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Rejected)?;
        let mut records = Vec::new();
        let mut statement=self.connection.prepare("SELECT operation_id,call_id,transport_session,transport_sequence,entry_json,owner_commit_json FROM native_task_operations WHERE parent_invocation_id=?1 AND contract_id='task.result.publish.v1' AND state='completed_settled'").map_err(|_|StoreError::Unavailable)?;
        let rows = statement
            .query_map([turn.invocation_id()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, u64>(3)?,
                    r.get::<_, Vec<u8>>(4)?,
                    r.get::<_, Vec<u8>>(5)?,
                ))
            })
            .map_err(|_| StoreError::Unavailable)?;
        for row in rows {
            let (operation, call, transport, sequence, entry, commit) =
                row.map_err(|_| StoreError::Unavailable)?;
            let matching = calls
                .iter()
                .filter(|c| c["call_id"] == call)
                .collect::<Vec<_>>();
            if matching.len() != 1 {
                return Err(StoreError::Rejected);
            }
            let ingress:Vec<u8>=self.connection.query_row("SELECT raw FROM native_ingress WHERE transport_session=?1 AND sequence=?2 AND session_id=?3 AND thread_id=?4 AND turn_id=?5 AND method='item/tool/call' AND decode_status='qualified'",params![transport,sequence,session.id().as_str(),input.thread_id,input.turn_id],|r|r.get(0)).map_err(|_|StoreError::ClaimConflict)?;
            let observed: Value =
                serde_json::from_slice(&ingress).map_err(|_| StoreError::Rejected)?;
            let c = matching[0];
            if observed["params"]["callId"] != call
                || observed["params"]["namespace"] != c["namespace"]
                || observed["params"]["tool"] != c["tool"]
                || observed["params"]["arguments"] != c["arguments"]
            {
                return Err(StoreError::ClaimConflict);
            }
            records.push(NativeCausalRecord{version:1,operation_id:operation,invocation_id:turn.invocation_id().into(),input_artifact_sha256:input_sha.clone(),session_id:session.id().as_str().into(),call_id:call,transport_session:transport,transport_sequence:sequence,ingress,entry,owner_commit:commit,bundle:bundle.to_vec(),scope:"controlled-native-causal-evidence-v1; explicit direct/same-turn-code-mode; historical non-authority evidence".into()});
        }
        drop(statement);
        let mut results = Vec::new();
        for record in records {
            let bytes = serde_json::to_vec(&record).map_err(|_| StoreError::Unavailable)?;
            let staged = self.stage_artifact(session.subject().clone(), &bytes)?;
            let published = self.publish_artifact(staged, clock)?;
            #[cfg(test)]
            if AFTER_PUBLICATION.with(|v| v.replace(false)) {
                return Err(StoreError::Unavailable);
            }
            #[cfg(test)]
            BEFORE_TRANSACTION.with(|slot| {
                if let Some(hook) = slot.borrow_mut().take() {
                    hook(&self.connection);
                }
            });
            let tx = self
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|_| StoreError::Unavailable)?;
            let association:i64=tx.query_row("SELECT count(*) FROM native_task_operations o JOIN native_input_observations b ON b.invocation_id=o.parent_invocation_id JOIN native_turns n ON n.invocation_id=b.invocation_id JOIN native_sessions s ON s.session_id=n.session_id JOIN native_ingress i ON i.transport_session=o.transport_session AND i.sequence=o.transport_sequence WHERE o.operation_id=?1 AND o.parent_invocation_id=?2 AND o.call_id=?3 AND o.transport_session=?4 AND o.transport_sequence=?5 AND o.entry_json=?6 AND o.owner_commit_json=?7 AND o.state='completed_settled' AND o.contract_id='task.result.publish.v1' AND b.artifact_sha256=?8 AND b.session_id=s.session_id AND s.session_id=?9 AND s.subject_id=?10 AND s.context_id=?11 AND n.attempt_id=?12 AND i.raw=?13 AND i.decode_status='qualified' AND i.thread_id=n.thread_id AND i.turn_id=n.turn_id",params![record.operation_id,record.invocation_id,record.call_id,record.transport_session,record.transport_sequence,record.entry,record.owner_commit,record.input_artifact_sha256,record.session_id,session.subject().as_str(),session.context().as_str(),turn.attempt().as_str(),record.ingress],|r|r.get(0)).map_err(|_|StoreError::Unavailable)?;
            let conflicted: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM native_input_conflicts WHERE invocation_id=?1)",
                    [&record.invocation_id],
                    |r| r.get(0),
                )
                .map_err(|_| StoreError::Unavailable)?;
            if association != 1 || conflicted {
                return Err(StoreError::ClaimConflict);
            }
            let old: Option<String> = tx
                .query_row(
                    "SELECT artifact_sha256 FROM native_causal_evidence WHERE operation_id=?1",
                    [&record.operation_id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|_| StoreError::Unavailable)?;
            let outcome = if let Some(old) = old {
                if old == published.digest_hex {
                    NativeInputApply::Replay
                } else {
                    tx.execute(
                        "INSERT OR IGNORE INTO native_causal_conflicts VALUES(?1,?2,?3)",
                        params![record.operation_id, published.digest_hex, clock.wall.get()],
                    )
                    .map_err(|_| StoreError::Unavailable)?;
                    NativeInputApply::Conflict
                }
            } else {
                tx.execute(
                    "INSERT INTO native_causal_evidence VALUES(?1,?2,?3,?4,?5)",
                    params![
                        record.operation_id,
                        record.invocation_id,
                        record.input_artifact_sha256,
                        published.digest_hex,
                        clock.wall.get()
                    ],
                )
                .map_err(|_| StoreError::Unavailable)?;
                NativeInputApply::Committed
            };
            tx.commit().map_err(|_| StoreError::Unavailable)?;
            #[cfg(test)]
            if AFTER_COMMIT.with(|v| v.replace(false)) {
                return Err(StoreError::Unavailable);
            }
            results.push(outcome);
        }
        Ok(results)
    }
    /// Read qualification and historical bytes from one SQLite snapshot. A
    /// conflict committed before this read cannot hide behind eligible history.
    pub fn qualified_native_causal_evidence(
        &self,
        operation: &str,
    ) -> Result<Option<(NativeCausalRecord, crate::NativeInputRecord)>, StoreError> {
        let tx = self
            .connection
            .unchecked_transaction()
            .map_err(|_| StoreError::Unavailable)?;
        if self.native_causal_conflicted(operation)? {
            return Err(StoreError::ClaimConflict);
        }
        let Some(causal) = self.native_causal_evidence(operation)? else {
            return Ok(None);
        };
        if self.native_input_conflicted(&causal.invocation_id)? {
            return Err(StoreError::ClaimConflict);
        }
        let input = self
            .native_input_observation(&causal.invocation_id)?
            .ok_or(StoreError::Unavailable)?;
        if !input.eligible {
            return Err(StoreError::Rejected);
        }
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(Some((causal, input)))
    }

    /// Historical bytes remain readable even if a conflicting replay is recorded.
    pub fn native_causal_evidence(
        &self,
        operation: &str,
    ) -> Result<Option<NativeCausalRecord>, StoreError> {
        self.verify_identity()?;
        let row:Option<(String,String,String)>=self.connection.query_row("SELECT invocation_id,input_artifact_sha256,artifact_sha256 FROM native_causal_evidence WHERE operation_id=?1",[operation],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|_|StoreError::Unavailable)?;
        row.map(|(invocation, input, digest)| {
            let bytes = self
                .read_committed_artifact(&digest)?
                .ok_or(StoreError::Unavailable)?;
            if sha(&bytes) != digest {
                return Err(StoreError::Unavailable);
            }
            let record: NativeCausalRecord =
                serde_json::from_slice(&bytes).map_err(|_| StoreError::Unavailable)?;
            if record.operation_id != operation
                || record.invocation_id != invocation
                || record.input_artifact_sha256 != input
            {
                return Err(StoreError::Unavailable);
            }
            Ok(record)
        })
        .transpose()
    }
    pub fn native_causal_conflicted(&self, operation: &str) -> Result<bool, StoreError> {
        self.verify_identity()?;
        self.connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM native_causal_conflicts WHERE operation_id=?1)",
                [operation],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)
    }
}
