//! Durable custody for direct-origin semantic control.
//!
//! A direct operation has no native session or thread, so it is not a
//! `semantic_attempts` row. It keeps its own operation identity and joins the
//! same capture, grant, artifact and checkpoint owners. Send intent commits
//! before the call is offered, the terminal observation is persisted before it
//! is interpreted, and a possibly sent operation is never offered again.

use lifecycle_core::{ClockSample, SubjectId};
use lifecycle_semantic::{
    CheckedSemanticResult, DirectCapture, DirectEvidenceClass, DirectPreparedCall, DirectRole,
    DirectTerminal, SemanticPhase, sha256_hex,
};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

use crate::{SqliteLifecycleStore, StoreError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectAttemptStart {
    /// Intent was just committed. The caller may submit exactly once.
    Fresh,
    /// A terminal observation is already durable. Interpret it; never resend.
    Terminal(Box<DirectCapture>),
    /// Intent was committed earlier and no terminal exists. The remote effect
    /// is unknown: never resend.
    PossiblySent,
    /// The terminal was recorded but its content was unusable.
    Refused(String),
    /// The adapter established that nothing was sent. A new operation is needed.
    NotSent,
}

/// The trusted acting identity that enters a direct control call. The store
/// compares it with the grant, the subject's current context, build and proof
/// run, and the transition that owns the subject; none of it is read from the
/// call or from model output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectControlEntry<'a> {
    pub principal: &'a str,
    pub context: &'a str,
    pub grant_id: &'a str,
    pub grant_revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectCheckpointOrigin {
    Native,
    /// Direct-origin and owner-admitted at publication: authoritative for
    /// checkpoint consumers (not for native successor consumers).
    DirectAdmitted {
        evidence_class: String,
        evidence_ref: String,
        admission_grant_id: String,
        admission_grant_revision: i64,
    },
    /// Direct-origin without admission: proof only.
    Direct {
        evidence_class: String,
        evidence_ref: String,
    },
}

struct AttemptRow {
    capture_id: String,
    role: String,
    prepared_binding_sha256: String,
    prepared_wire_sha256: String,
    evidence_class: String,
    evidence_ref: String,
    grant_id: String,
    grant_revision: i64,
    principal: String,
    state: String,
    terminal_artifact_sha256: Option<String>,
    output_sha256: Option<String>,
    result_artifact_sha256: Option<String>,
    refusal_code: Option<String>,
    input_sha256: String,
}

/// Public status reason for a refused direct compile. The stored code is the
/// recorded semantic error; an unrecognized code stays a generic refusal
/// rather than being given a more specific meaning.
pub(crate) fn direct_compile_refusal_reason(code: Option<&str>) -> &'static str {
    match code {
        Some("InvalidOutput") => "semantic_compile_invalid_output",
        Some("InvalidReference") => "semantic_compile_invalid_reference",
        Some("OutputTooLarge") => "semantic_compile_output_too_large",
        Some("WrongBinding") => "semantic_compile_wrong_binding",
        _ => "semantic_compile_refused",
    }
}

const ATTEMPT_COLUMNS: &str = "capture_id,role,prepared_binding_sha256,prepared_wire_sha256,evidence_class,evidence_ref,grant_id,grant_revision,principal,state,terminal_artifact_sha256,output_sha256,result_artifact_sha256,refusal_code,input_sha256";

fn unavailable<E>(_: E) -> StoreError {
    StoreError::Unavailable
}

fn attempt_row(tx: &Transaction<'_>, operation_id: &str) -> Result<Option<AttemptRow>, StoreError> {
    tx.query_row(
        &format!("SELECT {ATTEMPT_COLUMNS} FROM direct_semantic_attempts WHERE operation_id=?1"),
        [operation_id],
        |r| {
            Ok(AttemptRow {
                capture_id: r.get(0)?,
                role: r.get(1)?,
                prepared_binding_sha256: r.get(2)?,
                prepared_wire_sha256: r.get(3)?,
                evidence_class: r.get(4)?,
                evidence_ref: r.get(5)?,
                grant_id: r.get(6)?,
                grant_revision: r.get(7)?,
                principal: r.get(8)?,
                state: r.get(9)?,
                terminal_artifact_sha256: r.get(10)?,
                output_sha256: r.get(11)?,
                result_artifact_sha256: r.get(12)?,
                refusal_code: r.get(13)?,
                input_sha256: r.get(14)?,
            })
        },
    )
    .optional()
    .map_err(unavailable)
}

/// The grant an operation claims to act under: what a caller entry or a stored
/// attempt names. The issuer it must come from is the store's trusted issuer,
/// never part of the claim.
pub(crate) struct GrantClaim<'a> {
    pub(crate) grant_id: &'a str,
    pub(crate) revision: i64,
    pub(crate) scope: &'a str,
    pub(crate) principal: &'a str,
    pub(crate) subject: &'a str,
}

/// The full grant tuple the native control entry compares (`claim_control_turn`
/// in `native.rs`): trusted issuer, acting principal, subject, and the subject's
/// own context, build and proof run, plus scope, revision, revocation and expiry.
/// A grant that is current for the subject but issued for another principal,
/// context, build or proof run does not authorize.
pub(crate) fn grant_matches_identity(
    tx: &Transaction<'_>,
    trusted_issuer: &str,
    claim: &GrantClaim<'_>,
    now_ms: i64,
) -> Result<bool, StoreError> {
    tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM grants g JOIN subjects s ON s.subject_id=g.subject_id WHERE g.grant_id=?1 AND g.revision=?2 AND g.scope=?3 AND g.issuer=?4 AND g.principal=?5 AND g.subject_id=?6 AND g.context_id=s.context_id AND g.build_id=s.build_id AND g.proof_run_id=s.proof_run_id AND g.revoked=0 AND g.expires_ms>?7)",
        params![
            claim.grant_id,
            claim.revision,
            claim.scope,
            trusted_issuer,
            claim.principal,
            claim.subject,
            now_ms
        ],
        |r| r.get(0),
    )
    .map_err(unavailable)
}

/// The transition owns the subject and is at the stage in which control calls
/// are made, as the native control entry requires. `context`, when supplied by
/// the caller, must be the subject's current context.
pub(crate) fn transition_controls_verification(
    tx: &Transaction<'_>,
    subject: &str,
    transition: &str,
    context: Option<&str>,
    now_ms: i64,
) -> Result<bool, StoreError> {
    tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM subjects s JOIN transitions t ON t.subject_id=s.subject_id WHERE s.subject_id=?1 AND t.transition_id=?2 AND s.owner_kind='transition' AND s.owner_ref=?2 AND t.stage='verifying' AND s.last_wall_ms<=?3 AND (?4 IS NULL OR s.context_id=?4))",
        params![subject, transition, now_ms, context],
        |r| r.get(0),
    )
    .map_err(unavailable)
}

/// Owner admission for a direct result at the publication boundary: the
/// profile covers the stored evidence and a trusted-issuer grant of
/// `semantic_direct_checkpoint_v1` is current for this subject and for the
/// principal, context, build and proof run under which the calls were made.
/// Returns the grant identity and revision to fix on the result; `None` leaves
/// the result proof-scoped. Nothing here reads a caller argument.
pub(crate) fn direct_admission_in(
    tx: &Transaction<'_>,
    trusted_issuer: &str,
    result_id: &str,
    now_ms: i64,
) -> Result<Option<(String, i64)>, StoreError> {
    let (label, reference, subject, principal): (String, String, String, String) = tx
        .query_row(
            "SELECT r.evidence_class,r.evidence_ref,c.subject_id,a.principal FROM direct_semantic_results r JOIN semantic_captures c ON c.capture_id=r.capture_id JOIN direct_semantic_attempts a ON a.operation_id=r.compile_operation_id WHERE r.result_id=?1",
            [result_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(unavailable)?;
    if !lifecycle_semantic::DirectProductionProfile::covers_stored(&label, &reference) {
        return Ok(None);
    }
    tx.query_row(
        "SELECT g.grant_id,g.revision FROM grants g JOIN subjects s ON s.subject_id=g.subject_id WHERE g.scope='semantic_direct_checkpoint_v1' AND g.subject_id=?1 AND g.issuer=?2 AND g.principal=?3 AND g.context_id=s.context_id AND g.build_id=s.build_id AND g.proof_run_id=s.proof_run_id AND g.revoked=0 AND g.expires_ms>?4 ORDER BY g.revision DESC,g.grant_id LIMIT 1",
        params![subject, trusted_issuer, principal, now_ms],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
    .map_err(unavailable)
}

/// Both attempts of a direct result still hold a current semantic-control
/// grant, with the full tuple, at the revision and principal they were sent
/// under.
pub(crate) fn direct_result_grants_current(
    tx: &Transaction<'_>,
    trusted_issuer: &str,
    result_id: &str,
    now_ms: i64,
) -> Result<bool, StoreError> {
    let mut statement = tx
        .prepare("SELECT a.grant_id,a.grant_revision,a.principal,c.subject_id FROM direct_semantic_results r JOIN direct_semantic_attempts a ON a.operation_id IN (r.compile_operation_id,r.verifier_operation_id) JOIN semantic_captures c ON c.capture_id=r.capture_id WHERE r.result_id=?1")
        .map_err(unavailable)?;
    let rows = statement
        .query_map([result_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(unavailable)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(unavailable)?;
    if rows.len() != 2 {
        return Ok(false);
    }
    for (grant, revision, principal, subject) in rows {
        let claim = GrantClaim {
            grant_id: &grant,
            revision,
            scope: "semantic_control",
            principal: &principal,
            subject: &subject,
        };
        if !grant_matches_identity(tx, trusted_issuer, &claim, now_ms)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The owner's admission, fixed on the result at publication, is still a
/// current grant for the same acting principal and subject identity, and the
/// control grants the calls were made under are too. Used where the lifecycle
/// consumes the checkpoint.
pub(crate) fn direct_admission_still_current(
    tx: &Transaction<'_>,
    trusted_issuer: &str,
    result_id: &str,
    admission_grant_id: &str,
    admission_grant_revision: i64,
    now_ms: i64,
) -> Result<bool, StoreError> {
    let identity: Option<(String, String)> = tx
        .query_row(
            "SELECT a.principal,c.subject_id FROM direct_semantic_results r JOIN semantic_captures c ON c.capture_id=r.capture_id JOIN direct_semantic_attempts a ON a.operation_id=r.compile_operation_id WHERE r.result_id=?1",
            [result_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(unavailable)?;
    let Some((principal, subject)) = identity else {
        return Ok(false);
    };
    let admission = GrantClaim {
        grant_id: admission_grant_id,
        revision: admission_grant_revision,
        scope: "semantic_direct_checkpoint_v1",
        principal: &principal,
        subject: &subject,
    };
    Ok(
        grant_matches_identity(tx, trusted_issuer, &admission, now_ms)?
            && direct_result_grants_current(tx, trusted_issuer, result_id, now_ms)?,
    )
}

impl SqliteLifecycleStore {
    fn publish_direct_bytes(
        &mut self,
        subject: &str,
        bytes: &[u8],
        clock: ClockSample,
    ) -> Result<String, StoreError> {
        let subject = SubjectId::parse(subject.to_owned()).map_err(|_| StoreError::Rejected)?;
        let staged = self.stage_artifact(subject, bytes)?;
        Ok(self.publish_artifact(staged, clock)?.digest_hex)
    }

    /// Commit possible-send intent for one prepared direct call.
    pub fn begin_direct_attempt(
        &mut self,
        call: &DirectPreparedCall,
        entry: &DirectControlEntry<'_>,
        evidence: &DirectEvidenceClass,
        clock: ClockSample,
    ) -> Result<DirectAttemptStart, StoreError> {
        let (grant_id, grant_revision, principal) =
            (entry.grant_id, entry.grant_revision, entry.principal);
        self.verify_identity()?;
        if !crate::admin::admission_open(&self.connection)? {
            return Err(StoreError::Rejected);
        }
        call.validate().map_err(|_| StoreError::Rejected)?;
        let binding = &call.binding;
        let role = match binding.role {
            DirectRole::Compile => "compile",
            DirectRole::Verify => "verify",
        };
        let revision = i64::try_from(grant_revision).map_err(|_| StoreError::Rejected)?;
        let (subject, basis_sha, policy, eligibility): (String, String, String, String) = self
            .connection
            .query_row(
                "SELECT subject_id,basis_sha256,policy_id,eligibility FROM semantic_captures WHERE capture_id=?1 AND transition_id=?2",
                params![&binding.capture_id, binding.transition.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|_| StoreError::Rejected)?;
        let prepared = serde_json::to_vec(call).map_err(|_| StoreError::Rejected)?;
        let prepared_artifact = self.publish_direct_bytes(&subject, &prepared, clock)?;
        let claim = GrantClaim {
            grant_id,
            revision,
            scope: "semantic_control",
            principal,
            subject: &subject,
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        if let Some(row) = attempt_row(&tx, &binding.operation_id)? {
            if row.prepared_binding_sha256 != call.binding_sha256
                || row.capture_id != binding.capture_id
                || row.role != role
                || row.evidence_class != evidence.label()
                || row.evidence_ref != evidence.reference()
                || row.grant_id != grant_id
                || row.grant_revision != revision
                || row.principal != principal
            {
                return Err(StoreError::ClaimConflict);
            }
            // Only a replay that lets the caller continue interprets recorded
            // material; it needs the same authority a fresh entry does. A
            // possibly sent, refused or unsent operation is reported as it is.
            let continues = matches!(row.state.as_str(), "terminal_recorded" | "completed_settled");
            if continues
                && !(transition_controls_verification(
                    &tx,
                    &subject,
                    binding.transition.as_str(),
                    Some(entry.context),
                    clock.wall.get(),
                )? && grant_matches_identity(
                    &tx,
                    &self.trusted_issuer,
                    &claim,
                    clock.wall.get(),
                )?)
            {
                return Err(StoreError::Rejected);
            }
            drop(tx);
            return self.start_from_row(row);
        }
        if eligibility != "current"
            || basis_sha != call.source_basis_sha256
            || policy != binding.policy_id
            || subject != binding.subject.as_str()
        {
            return Err(StoreError::Rejected);
        }
        if !transition_controls_verification(
            &tx,
            &subject,
            binding.transition.as_str(),
            Some(entry.context),
            clock.wall.get(),
        )? || !grant_matches_identity(&tx, &self.trusted_issuer, &claim, clock.wall.get())?
        {
            return Err(StoreError::Rejected);
        }
        if binding.role == DirectRole::Verify {
            let compile_binding = call
                .compile_binding_sha256
                .as_deref()
                .ok_or(StoreError::Rejected)?;
            let candidate = call.candidate_sha256.as_deref().ok_or(StoreError::Rejected)?;
            let linked: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM direct_semantic_attempts WHERE prepared_binding_sha256=?1 AND role='compile' AND capture_id=?2 AND state='completed_settled' AND output_sha256=?3 AND result_artifact_sha256 IS NOT NULL AND evidence_class=?4 AND evidence_ref=?5)",
                    params![compile_binding, &binding.capture_id, candidate, evidence.label(), evidence.reference()],
                    |r| r.get(0),
                )
                .map_err(unavailable)?;
            if !linked {
                return Err(StoreError::Rejected);
            }
        } else if call.candidate_sha256.is_some() || call.compile_binding_sha256.is_some() {
            return Err(StoreError::Rejected);
        }
        tx.execute(
            "INSERT INTO direct_semantic_attempts(operation_id,capture_id,role,attempt_id,exposure_id,policy_id,grant_id,grant_revision,principal,evidence_class,evidence_ref,requested_provider,requested_model,input_sha256,prepared_binding_sha256,prepared_wire_sha256,prepared_artifact_sha256,state,created_wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?18,?9,?10,?11,?12,?13,?14,?15,?16,'intent_recorded',?17)",
            params![
                &binding.operation_id,
                &binding.capture_id,
                role,
                &binding.attempt_id,
                &binding.exposure_id,
                &binding.policy_id,
                grant_id,
                revision,
                evidence.label(),
                evidence.reference(),
                &call.wire.requested_provider,
                &call.wire.requested_model,
                &call.input_sha256,
                &call.binding_sha256,
                &call.wire_sha256,
                &prepared_artifact,
                clock.wall.get(),
                principal
            ],
        )
        .map_err(|_| StoreError::ClaimConflict)?;
        tx.commit().map_err(unavailable)?;
        Ok(DirectAttemptStart::Fresh)
    }

    fn start_from_row(&self, row: AttemptRow) -> Result<DirectAttemptStart, StoreError> {
        match row.state.as_str() {
            "intent_recorded" => Ok(DirectAttemptStart::PossiblySent),
            "terminal_recorded" | "completed_settled" => {
                let sha = row.terminal_artifact_sha256.ok_or(StoreError::Unavailable)?;
                let bytes = self
                    .read_committed_artifact(&sha)?
                    .ok_or(StoreError::Unavailable)?;
                let capture: DirectCapture =
                    serde_json::from_slice(&bytes).map_err(unavailable)?;
                Ok(DirectAttemptStart::Terminal(Box::new(capture)))
            }
            "refused" => Ok(DirectAttemptStart::Refused(
                row.refusal_code.unwrap_or_default(),
            )),
            "not_sent" => Ok(DirectAttemptStart::NotSent),
            _ => Err(StoreError::Unavailable),
        }
    }

    /// Persist what the adapter observed, before anything interprets it.
    pub fn record_direct_terminal(
        &mut self,
        operation_id: &str,
        capture: &DirectCapture,
        clock: ClockSample,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let subject: String = self
            .connection
            .query_row(
                "SELECT c.subject_id FROM direct_semantic_attempts a JOIN semantic_captures c ON c.capture_id=a.capture_id WHERE a.operation_id=?1",
                [operation_id],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Rejected)?;
        let bytes = serde_json::to_vec(capture).map_err(|_| StoreError::Rejected)?;
        let artifact = self.publish_direct_bytes(&subject, &bytes, clock)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let row = attempt_row(&tx, operation_id)?.ok_or(StoreError::Rejected)?;
        if capture.operation_id != operation_id
            || capture.prepared_wire_sha256 != row.prepared_wire_sha256
            || capture.prepared_binding_sha256 != row.prepared_binding_sha256
            || capture.evidence.label() != row.evidence_class
            || capture.evidence.reference() != row.evidence_ref
            || capture.output_sha256 != sha256_hex(&capture.output)
        {
            return Err(StoreError::Rejected);
        }
        match row.state.as_str() {
            "intent_recorded" => {
                tx.execute(
                    "UPDATE direct_semantic_attempts SET state='terminal_recorded',terminal_artifact_sha256=?2,output_sha256=?3 WHERE operation_id=?1 AND state='intent_recorded'",
                    params![operation_id, &artifact, &capture.output_sha256],
                )
                .map_err(unavailable)?;
                tx.commit().map_err(unavailable)
            }
            _ if row.terminal_artifact_sha256.as_deref() == Some(artifact.as_str()) => Ok(()),
            _ => Err(StoreError::ClaimConflict),
        }
    }

    /// The adapter established that it consumed no release, custody or action
    /// identity and sent nothing, so the intent is withdrawn and the operation
    /// can be entered again with every entry check repeated. Only an
    /// `intent_recorded` row for the same prepared call is withdrawn; any other
    /// recorded state is reported as a conflict and kept.
    pub fn withdraw_direct_intent(
        &mut self,
        operation_id: &str,
        prepared_binding_sha256: &str,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let Some(row) = attempt_row(&tx, operation_id)? else {
            return Ok(());
        };
        if row.state != "intent_recorded" || row.prepared_binding_sha256 != prepared_binding_sha256 {
            return Err(StoreError::ClaimConflict);
        }
        tx.execute(
            "DELETE FROM direct_semantic_attempts WHERE operation_id=?1 AND state='intent_recorded' AND prepared_binding_sha256=?2",
            params![operation_id, prepared_binding_sha256],
        )
        .map_err(unavailable)?;
        tx.commit().map_err(unavailable)
    }

    /// The adapter established that no byte reached the endpoint.
    pub fn record_direct_not_sent(
        &mut self,
        operation_id: &str,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let changed = self
            .connection
            .execute(
                "UPDATE direct_semantic_attempts SET state='not_sent' WHERE operation_id=?1 AND state='intent_recorded'",
                [operation_id],
            )
            .map_err(unavailable)?;
        if changed == 1 {
            return Ok(());
        }
        let state: Option<String> = self
            .connection
            .query_row(
                "SELECT state FROM direct_semantic_attempts WHERE operation_id=?1",
                [operation_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(unavailable)?;
        if state.as_deref() == Some("not_sent") {
            Ok(())
        } else {
            Err(StoreError::ClaimConflict)
        }
    }

    /// The terminal content cannot be used (for example invalid compile
    /// output). Keep the observation and never resend this operation.
    pub fn record_direct_refusal(
        &mut self,
        operation_id: &str,
        code: &str,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let changed = self
            .connection
            .execute(
                "UPDATE direct_semantic_attempts SET state='refused',refusal_code=?2 WHERE operation_id=?1 AND state='terminal_recorded'",
                params![operation_id, code],
            )
            .map_err(unavailable)?;
        if changed == 1 {
            return Ok(());
        }
        let prior: Option<(String, Option<String>)> = self
            .connection
            .query_row(
                "SELECT state,refusal_code FROM direct_semantic_attempts WHERE operation_id=?1",
                [operation_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(unavailable)?;
        match prior {
            Some((state, Some(existing))) if state == "refused" && existing == code => Ok(()),
            _ => Err(StoreError::ClaimConflict),
        }
    }

    /// Accept a complete terminal's output as this operation's settled result.
    /// Returns the output artifact digest.
    pub fn settle_direct_attempt(
        &mut self,
        operation_id: &str,
        clock: ClockSample,
    ) -> Result<String, StoreError> {
        self.verify_identity()?;
        let (state, terminal, output_sha, existing, subject): (
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        ) = self
            .connection
            .query_row(
                "SELECT a.state,a.terminal_artifact_sha256,a.output_sha256,a.result_artifact_sha256,c.subject_id FROM direct_semantic_attempts a JOIN semantic_captures c ON c.capture_id=a.capture_id WHERE a.operation_id=?1",
                [operation_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .map_err(|_| StoreError::Rejected)?;
        if state == "completed_settled" {
            return existing.ok_or(StoreError::Unavailable);
        }
        if state != "terminal_recorded" {
            return Err(StoreError::Rejected);
        }
        let terminal = terminal.ok_or(StoreError::Unavailable)?;
        let capture: DirectCapture = serde_json::from_slice(
            &self
                .read_committed_artifact(&terminal)?
                .ok_or(StoreError::Unavailable)?,
        )
        .map_err(unavailable)?;
        if capture.terminal != DirectTerminal::Complete
            || Some(capture.output_sha256.as_str()) != output_sha.as_deref()
        {
            return Err(StoreError::Rejected);
        }
        let artifact = self.publish_direct_bytes(&subject, &capture.output, clock)?;
        let changed = self
            .connection
            .execute(
                "UPDATE direct_semantic_attempts SET state='completed_settled',result_artifact_sha256=?2 WHERE operation_id=?1 AND state='terminal_recorded'",
                params![operation_id, &artifact],
            )
            .map_err(unavailable)?;
        if changed != 1 {
            return Err(StoreError::ClaimConflict);
        }
        Ok(artifact)
    }

    /// Record the host-derived verdict. A grant revoked, expired or revised
    /// after the call fences the result: nothing is recorded or published.
    pub fn record_direct_result(
        &mut self,
        result_id: &str,
        compile_operation_id: &str,
        verifier_operation_id: &str,
        checked: &CheckedSemanticResult,
        clock: ClockSample,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        if !crate::admin::admission_open(&self.connection)? {
            return Err(StoreError::Rejected);
        }
        if checked.phase != SemanticPhase::CandidateVerification
            || checked.verifier_invocation_id != verifier_operation_id
        {
            return Err(StoreError::Rejected);
        }
        let (subject, transition, basis_sha, policy, capture_id): (String, String, String, String, String) = self
            .connection
            .query_row(
                "SELECT c.subject_id,c.transition_id,c.basis_sha256,c.policy_id,c.capture_id FROM direct_semantic_attempts a JOIN semantic_captures c ON c.capture_id=a.capture_id WHERE a.operation_id=?1 AND c.eligibility='current'",
                [compile_operation_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .map_err(|_| StoreError::Rejected)?;
        if checked.source_basis_sha256 != basis_sha || checked.policy_id != policy {
            return Err(StoreError::Rejected);
        }
        let bytes = serde_json::to_vec(checked).map_err(|_| StoreError::Rejected)?;
        let findings = self.publish_direct_bytes(&subject, &bytes, clock)?;
        let verdict = match checked.status {
            lifecycle_semantic::VerdictStatus::Accepted => "accepted",
            lifecycle_semantic::VerdictStatus::Rejected => "rejected",
            lifecycle_semantic::VerdictStatus::Unresolved => "unresolved",
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let compile = attempt_row(&tx, compile_operation_id)?.ok_or(StoreError::Rejected)?;
        let verify = attempt_row(&tx, verifier_operation_id)?.ok_or(StoreError::Rejected)?;
        if compile.role != "compile"
            || verify.role != "verify"
            || compile.capture_id != capture_id
            || verify.capture_id != capture_id
            || compile.state != "completed_settled"
            || verify.state != "completed_settled"
            || compile.output_sha256.as_deref() != Some(checked.candidate_sha256.as_str())
            || verify.output_sha256.as_deref() != Some(checked.verifier_result_sha256.as_str())
            || compile.evidence_class != verify.evidence_class
            || compile.evidence_ref != verify.evidence_ref
            || compile.result_artifact_sha256.as_deref() != Some(checked.candidate_sha256.as_str())
            || compile.input_sha256 == verify.input_sha256
        {
            return Err(StoreError::ClaimConflict);
        }
        if compile.principal != verify.principal {
            return Err(StoreError::ClaimConflict);
        }
        for attempt in [&compile, &verify] {
            let claim = GrantClaim {
                grant_id: &attempt.grant_id,
                revision: attempt.grant_revision,
                scope: "semantic_control",
                principal: &attempt.principal,
                subject: &subject,
            };
            if !grant_matches_identity(&tx, &self.trusted_issuer, &claim, clock.wall.get())? {
                return Err(StoreError::ClaimConflict);
            }
        }
        if !transition_controls_verification(&tx, &subject, &transition, None, clock.wall.get())? {
            return Err(StoreError::Rejected);
        }
        let prior: Option<(String, String, String, String)> = tx
            .query_row(
                "SELECT capture_id,verifier_operation_id,candidate_artifact_sha256,findings_artifact_sha256 FROM direct_semantic_results WHERE result_id=?1",
                [result_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()
            .map_err(unavailable)?;
        if let Some((c, v, a, f)) = prior {
            return if c == capture_id
                && v == verifier_operation_id
                && a == checked.candidate_sha256
                && f == findings
            {
                Ok(())
            } else {
                Err(StoreError::ClaimConflict)
            };
        }
        tx.execute(
            "INSERT INTO direct_semantic_results(result_id,transition_id,capture_id,compile_operation_id,verifier_operation_id,candidate_artifact_sha256,findings_artifact_sha256,verdict,eligibility,basis_sha256,policy_id,evidence_class,evidence_ref,created_wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'current',?9,?10,?11,?12,?13)",
            params![
                result_id,
                &transition,
                &capture_id,
                compile_operation_id,
                verifier_operation_id,
                &checked.candidate_sha256,
                &findings,
                verdict,
                &basis_sha,
                &policy,
                &compile.evidence_class,
                &compile.evidence_ref,
                clock.wall.get()
            ],
        )
        .map_err(|_| StoreError::ClaimConflict)?;
        tx.execute(
            "INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'semantic_result',?2,?3)",
            params![&subject, result_id, clock.wall.get()],
        )
        .map_err(unavailable)?;
        tx.commit().map_err(unavailable)
    }

    /// Which owner realized the current checkpoint for this transition.
    pub fn semantic_checkpoint_origin(
        &self,
        transition: &lifecycle_core::TransitionId,
    ) -> Result<Option<DirectCheckpointOrigin>, StoreError> {
        self.verify_identity()?;
        if self.semantic_checkpoint(transition)?.is_some() {
            return Ok(Some(DirectCheckpointOrigin::Native));
        }
        if let Some(admitted) = self.admitted_direct_checkpoint(transition)? {
            return Ok(Some(DirectCheckpointOrigin::DirectAdmitted {
                evidence_class: admitted.evidence_class,
                evidence_ref: admitted.evidence_ref,
                admission_grant_id: admitted.admission_grant_id,
                admission_grant_revision: admitted.admission_grant_revision,
            }));
        }
        Ok(self
            .direct_proof_checkpoint(transition)?
            .map(|proof| DirectCheckpointOrigin::Direct {
                evidence_class: proof.evidence_class,
                evidence_ref: proof.evidence_ref,
            }))
    }
}
