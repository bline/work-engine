//! Candidate transaction kernel. No production caller is wired: protected board,
//! current authority and accounting adapters must construct the private gate.
use crate::proof::VerifiedAnswer;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Store,
    Conflict,
    Held,
}
impl From<rusqlite::Error> for Error {
    fn from(_: rusqlite::Error) -> Self {
        Self::Store
    }
}

// Intentionally private to the crate. The eventual trusted adapters must check
// current board/proposal/actor/mode, reservation, deadlines and revocation at
// consumption and delivery. Untrusted JSON cannot construct this gate.
#[allow(dead_code)]
pub(crate) struct CurrentGate {
    pub(crate) task_id: String,
    pub(crate) revision: String,
    pub(crate) request_sha: String,
    pub(crate) lane_id: String,
    pub(crate) proposal: crate::Ref,
    pub(crate) decision: crate::Ref,
    pub(crate) objective_id: String,
    pub(crate) canonical_budget_revision: String,
    pub(crate) observation_ref: String,
    pub(crate) exclusive_binding_ref: String,
    pub(crate) maximum: String,
    pub(crate) productive_maximum: String,
    pub(crate) closure_reserve: String,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Admission {
    pub(crate) id: String,
    pub(crate) operation: String,
}
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}

const TABLES: [(&str, &str); 12] = [
    (
        "admission",
        "CREATE TABLE admission(answer_id TEXT PRIMARY KEY,request_id TEXT NOT NULL UNIQUE,task_id TEXT NOT NULL,revision TEXT NOT NULL,request_sha TEXT NOT NULL,lane_id TEXT NOT NULL,proposal_id TEXT NOT NULL,proposal_revision TEXT NOT NULL,proposal_sha TEXT NOT NULL,decision_revision TEXT NOT NULL,decision_sha TEXT NOT NULL,answer_sha TEXT NOT NULL,answer_json TEXT NOT NULL,assertion_sha TEXT NOT NULL,admission_id TEXT NOT NULL UNIQUE,reservation_ref TEXT NOT NULL,operation_id TEXT NOT NULL UNIQUE,UNIQUE(task_id,revision),FOREIGN KEY(task_id,revision) REFERENCES task_revision(task_id,revision))",
    ),
    (
        "capacity_claim",
        "CREATE TABLE capacity_claim(claim_id TEXT PRIMARY KEY,operation_id TEXT NOT NULL UNIQUE REFERENCES admission(operation_id),objective_id TEXT NOT NULL UNIQUE,envelope_sha TEXT NOT NULL,budget_revision TEXT NOT NULL,observation_ref TEXT NOT NULL,exclusive_binding_ref TEXT NOT NULL,maximum TEXT NOT NULL,productive_maximum TEXT NOT NULL,closure_reserve TEXT NOT NULL,state TEXT NOT NULL CHECK(state IN ('active','unknown')))",
    ),
    (
        "funding_clock",
        "CREATE TABLE funding_clock(task_id TEXT NOT NULL,revision TEXT NOT NULL,first_funded INTEGER NOT NULL,source_ref TEXT NOT NULL,PRIMARY KEY(task_id,revision),FOREIGN KEY(task_id,revision) REFERENCES task_revision(task_id,revision))",
    ),
    (
        "outbox",
        "CREATE TABLE outbox(operation_id TEXT PRIMARY KEY REFERENCES admission(operation_id),state TEXT NOT NULL CHECK(state IN ('pending','unknown','delivered')))",
    ),
    (
        "proposal_head",
        "CREATE TABLE proposal_head(id TEXT PRIMARY KEY,revision TEXT NOT NULL,sha256 TEXT NOT NULL,state TEXT NOT NULL CHECK(state IN ('live','withdrawn','superseded')),generation INTEGER NOT NULL)",
    ),
    (
        "proposal_revision",
        "CREATE TABLE proposal_revision(id TEXT NOT NULL,revision TEXT NOT NULL,sha256 TEXT NOT NULL,PRIMARY KEY(id,revision))",
    ),
    (
        "scoped_claim",
        "CREATE TABLE scoped_claim(profile_sha TEXT PRIMARY KEY,operation_id TEXT NOT NULL UNIQUE REFERENCES outbox(operation_id),activation_sha TEXT NOT NULL,session TEXT NOT NULL,turn TEXT NOT NULL,entry_sha TEXT NOT NULL,attempt TEXT NOT NULL,request_sha TEXT NOT NULL,state TEXT NOT NULL CHECK(state IN ('claimed','effect_unknown','effect_observed')),effect_sha TEXT,result_json TEXT,UNIQUE(session,turn))",
    ),
    (
        "scoped_control",
        "CREATE TABLE scoped_control(profile_sha TEXT NOT NULL,generation INTEGER NOT NULL,request_sha TEXT NOT NULL UNIQUE,predecessor TEXT,state TEXT NOT NULL CHECK(state IN ('active','revoked')),request_json TEXT NOT NULL,PRIMARY KEY(profile_sha,generation))",
    ),
    (
        "scoped_final_claim",
        "CREATE TABLE scoped_final_claim(profile_sha TEXT PRIMARY KEY REFERENCES scoped_claim(profile_sha),first_attempt TEXT NOT NULL,first_request TEXT NOT NULL,attempt TEXT NOT NULL UNIQUE,request_sha TEXT NOT NULL,callback_sha TEXT NOT NULL,call_id TEXT NOT NULL,callback_json TEXT NOT NULL)",
    ),
    (
        "scoped_publication",
        "CREATE TABLE scoped_publication(profile_sha TEXT PRIMARY KEY REFERENCES scoped_claim(profile_sha),revision TEXT NOT NULL,body TEXT NOT NULL)",
    ),
    (
        "task_head",
        "CREATE TABLE task_head(id TEXT PRIMARY KEY,lane_id TEXT NOT NULL,revision TEXT NOT NULL,sha256 TEXT NOT NULL,proposal_id TEXT NOT NULL,proposal_revision TEXT NOT NULL,proposal_sha TEXT NOT NULL,state TEXT NOT NULL CHECK(state IN ('live','withdrawn','superseded')),generation INTEGER NOT NULL)",
    ),
    (
        "task_revision",
        "CREATE TABLE task_revision(task_id TEXT NOT NULL,revision TEXT NOT NULL,request_sha TEXT NOT NULL,request_json TEXT NOT NULL,PRIMARY KEY(task_id,revision))",
    ),
];
#[allow(dead_code)]
pub(crate) fn initialize(db: &Connection) -> Result<(), Error> {
    db.execute_batch("PRAGMA foreign_keys=ON")?;
    let integrity: String = db.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(Error::Store);
    }
    let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if ![0, 1, 2].contains(&version) {
        return Err(Error::Store);
    }
    let count: i64 = db.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE sql IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    if count == 0 {
        db.execute_batch("BEGIN IMMEDIATE")?;
        let created = (|| -> Result<(), Error> {
            // Task table must exist before its foreign-key consumers.
            for n in [11, 4, 5, 10, 0, 1, 2, 3, 6, 7, 8, 9] {
                db.execute_batch(TABLES[n].1)?;
            }
            db.execute_batch("PRAGMA user_version=2; COMMIT")?;
            Ok(())
        })();
        if created.is_err() {
            let _ = db.execute_batch("ROLLBACK");
        }
        created?;
    }
    if count == 8 {
        if version != 0 {
            return Err(Error::Store);
        }
        let legacy = TABLES
            .iter()
            .filter(|(name, _)| !name.starts_with("scoped_"));
        let mut stmt =
            db.prepare("SELECT name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY name")?;
        let actual = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        if actual
            .iter()
            .zip(legacy)
            .any(|((name, sql), (expected, ddl))| name != expected || sql != ddl)
        {
            return Err(Error::Store);
        }
        drop(stmt);
        db.execute_batch("BEGIN IMMEDIATE")?;
        let migrated = (|| -> Result<(), Error> {
            for n in [6, 7, 8, 9] {
                db.execute_batch(TABLES[n].1)?;
            }
            db.execute_batch("PRAGMA user_version=2; COMMIT")?;
            Ok(())
        })();
        if migrated.is_err() {
            let _ = db.execute_batch("ROLLBACK");
        }
        migrated?;
    }
    if count == 11 {
        if version != 1 {
            return Err(Error::Store);
        }
        let mut statement =
            db.prepare("SELECT name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY name")?;
        let actual = statement
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        let expected: Vec<_> = TABLES
            .iter()
            .filter(|(name, _)| *name != "scoped_final_claim")
            .collect();
        if actual.len() != expected.len()
            || actual
                .iter()
                .zip(expected)
                .any(|((name, sql), (n, ddl))| name != n || sql != ddl)
        {
            return Err(Error::Store);
        }
        drop(statement);
        db.execute_batch("BEGIN IMMEDIATE")?;
        let migrated = (|| -> Result<(), Error> {
            db.execute_batch(TABLES[8].1)?;
            db.execute_batch("PRAGMA user_version=2; COMMIT")?;
            Ok(())
        })();
        if migrated.is_err() {
            let _ = db.execute_batch("ROLLBACK");
        }
        migrated?;
    }
    let mut statement =
        db.prepare("SELECT name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY name")?;
    let actual = statement
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    if actual.len() != TABLES.len()
        || actual
            .iter()
            .zip(TABLES)
            .any(|((name, sql), (expected, ddl))| name != expected || sql != ddl)
    {
        return Err(Error::Store);
    }
    if db
        .prepare("PRAGMA foreign_key_check")?
        .query([])?
        .next()?
        .is_some()
    {
        return Err(Error::Store);
    }
    Ok(())
}

// These registry transitions are intentionally crate-private. A qualified,
// authenticated coordinator must own the caller; MCP/worker input cannot invoke
// them. Exact upstream Ref bytes remain unchanged and generation is local CAS.
fn next_generation(old: Option<i64>, expected: Option<i64>) -> Result<i64, Error> {
    if old != expected {
        return Err(Error::Conflict);
    }
    old.unwrap_or(0).checked_add(1).ok_or(Error::Store)
}
fn supported_state(state: &str) -> bool {
    matches!(state, "live" | "withdrawn" | "superseded")
}
#[allow(dead_code)]
pub(crate) fn register_proposal(
    db: &mut Connection,
    reference: &crate::Ref,
    state: &str,
    expected: Option<i64>,
) -> Result<i64, Error> {
    if !crate::valid_ref(reference) || !supported_state(state) {
        return Err(Error::Held);
    }
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let prior: Option<(i64, String, String)> = tx
        .query_row(
            "SELECT generation,revision,state FROM proposal_head WHERE id=?1",
            [&reference.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let generation = next_generation(prior.as_ref().map(|v| v.0), expected)?;
    if prior.as_ref().is_some_and(|(_, revision, previous)| {
        previous != "live" && state == "live" && revision == &reference.revision
    }) {
        return Err(Error::Held);
    }
    let old_sha: Option<String> = tx
        .query_row(
            "SELECT sha256 FROM proposal_revision WHERE id=?1 AND revision=?2",
            params![reference.id, reference.revision],
            |r| r.get(0),
        )
        .optional()?;
    if old_sha.as_deref().is_some_and(|v| v != reference.sha256) {
        return Err(Error::Conflict);
    }
    if old_sha.is_some()
        && prior
            .as_ref()
            .is_some_and(|(_, revision, _)| revision != &reference.revision)
    {
        return Err(Error::Held);
    }
    if old_sha.is_none() {
        tx.execute(
            "INSERT INTO proposal_revision VALUES (?1,?2,?3)",
            params![reference.id, reference.revision, reference.sha256],
        )?;
    }
    tx.execute("INSERT INTO proposal_head VALUES (?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,sha256=excluded.sha256,state=excluded.state,generation=excluded.generation",params![reference.id,reference.revision,reference.sha256,state,generation])?;
    tx.commit()?;
    Ok(generation)
}
#[allow(dead_code)]
pub(crate) fn register_task(
    db: &mut Connection,
    raw_request: &str,
    state: &str,
    expected: Option<i64>,
) -> Result<i64, Error> {
    if !supported_state(state) {
        return Err(Error::Held);
    }
    let task = crate::validate_review(raw_request).map_err(|_| Error::Held)?;
    let row: crate::Row = serde_json::from_str(raw_request).map_err(|_| Error::Held)?;
    let intent = row.data.intent;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let prior: Option<(i64, String, String)> = tx
        .query_row(
            "SELECT generation,revision,state FROM task_head WHERE id=?1",
            [&task.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let generation = next_generation(prior.as_ref().map(|v| v.0), expected)?;
    if prior.as_ref().is_some_and(|(_, revision, previous)| {
        previous != "live" && state == "live" && revision == &task.revision
    }) {
        return Err(Error::Held);
    }
    if state == "live" {
        let live:i64=tx.query_row("SELECT count(*) FROM proposal_head WHERE id=?1 AND revision=?2 AND sha256=?3 AND state='live'",params![intent.proposal.id,intent.proposal.revision,intent.proposal.sha256],|r|r.get(0))?;
        if live != 1 {
            return Err(Error::Held);
        }
    }
    let old: Option<(String, String)> = tx
        .query_row(
            "SELECT request_sha,request_json FROM task_revision WHERE task_id=?1 AND revision=?2",
            params![task.id, task.revision],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if old.is_some()
        && prior
            .as_ref()
            .is_some_and(|(_, revision, _)| revision != &task.revision)
    {
        return Err(Error::Held);
    }
    match old {
        Some((sha, raw)) if sha != task.sha256 || raw != raw_request => {
            return Err(Error::Conflict);
        }
        Some(_) => {}
        None => {
            tx.execute(
                "INSERT INTO task_revision VALUES (?1,?2,?3,?4)",
                params![task.id, task.revision, task.sha256, raw_request],
            )?;
        }
    }
    tx.execute("INSERT INTO task_head VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET lane_id=excluded.lane_id,revision=excluded.revision,sha256=excluded.sha256,proposal_id=excluded.proposal_id,proposal_revision=excluded.proposal_revision,proposal_sha=excluded.proposal_sha,state=excluded.state,generation=excluded.generation",params![task.id,intent.lane_id,task.revision,task.sha256,intent.proposal.id,intent.proposal.revision,intent.proposal.sha256,state,generation])?;
    tx.commit()?;
    Ok(generation)
}

// Record a protected canonical observation, not a new accounting ledger. An
// earlier later-discovered observation corrects the clock; a later one never
// resets it. No production source adapter currently invokes this kernel.
#[allow(dead_code)]
pub(crate) fn observe_funding(
    db: &mut Connection,
    task_id: &str,
    revision: &str,
    request: &str,
    at: i64,
    source_ref: &str,
) -> Result<(), Error> {
    if task_id.is_empty() || revision.is_empty() || at <= 0 || source_ref.is_empty() {
        return Err(Error::Held);
    }
    let sha = hash(request);
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let old: Option<(String, String)> = tx
        .query_row(
            "SELECT request_sha,request_json FROM task_revision WHERE task_id=?1 AND revision=?2",
            params![task_id, revision],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match old {
        Some((old_sha, old_request)) if old_sha != sha || old_request != request => {
            return Err(Error::Conflict);
        }
        Some(_) => {}
        None => {
            tx.execute(
                "INSERT INTO task_revision VALUES (?1,?2,?3,?4)",
                params![task_id, revision, sha, request],
            )?;
        }
    }
    tx.execute("INSERT INTO funding_clock VALUES (?1,?2,?3,?4) ON CONFLICT(task_id,revision) DO UPDATE SET first_funded=excluded.first_funded,source_ref=excluded.source_ref WHERE excluded.first_funded<funding_clock.first_funded",params![task_id,revision,at,source_ref])?;
    tx.commit()?;
    Ok(())
}

// Exact retry returns the same identity; a conflicting answer, request id or
// immutable task revision is rejected. This transaction does not reserve funds
// and does not confer authority on caller-provided state.
#[allow(dead_code)]
pub(crate) fn commit(
    db: &mut Connection,
    request_id: &str,
    request: &str,
    answer: &str,
    verified: &VerifiedAnswer,
    gate: &CurrentGate,
) -> Result<Admission, Error> {
    if request_id.is_empty()
        || verified.decision != "approve"
        || !verified.comment.is_empty()
        || gate.objective_id.is_empty()
        || gate.canonical_budget_revision.is_empty()
        || gate.observation_ref.is_empty()
        || gate.exclusive_binding_ref.is_empty()
        || !crate::budget_within(
            &gate.maximum,
            &gate.productive_maximum,
            &gate.closure_reserve,
        )
        || verified.request_digest != hash(request)
        || gate.request_sha != verified.request_digest
        || gate.decision.id != verified.answer_id
        || !crate::valid_ref(&gate.decision)
    {
        return Err(Error::Held);
    }
    let answer_sha = hash(answer);
    if verified.answer_digest != answer_sha {
        return Err(Error::Held);
    }
    let admission_id = hash(&format!(
        "dot-admission/v1\0{}\0{}",
        verified.answer_id, gate.request_sha
    ));
    let operation = hash(&format!("dot-operation/v1\0{admission_id}"));
    let claim_id = hash(&format!("dot-capacity-claim/v1\0{operation}"));
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    type Existing = (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    );
    let existing: Option<Existing> = tx.query_row("SELECT request_id,task_id,revision,request_sha,answer_sha,answer_json,assertion_sha,admission_id,operation_id FROM admission WHERE answer_id=?1", [&verified.answer_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?))).optional()?;
    if let Some((
        old_request,
        task,
        rev,
        sha,
        answer_digest,
        original_answer,
        assertion_digest,
        id,
        op,
    )) = existing
    {
        let stored_decision: (String, String) = tx.query_row(
            "SELECT decision_revision,decision_sha FROM admission WHERE answer_id=?1",
            [&verified.answer_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if stored_decision.0 != gate.decision.revision || stored_decision.1 != gate.decision.sha256
        {
            return Err(Error::Conflict);
        }
        if old_request != request_id
            || task != gate.task_id
            || rev != gate.revision
            || sha != gate.request_sha
            || answer_digest != answer_sha
            || original_answer != answer
            || assertion_digest != verified.assertion_digest
        {
            return Err(Error::Conflict);
        }
        tx.commit()?;
        return Ok(Admission { id, operation: op });
    }
    let live: i64 = tx.query_row("SELECT count(*) FROM task_head t JOIN proposal_head p ON p.id=t.proposal_id WHERE t.id=?1 AND t.lane_id=?2 AND t.revision=?3 AND t.sha256=?4 AND t.proposal_id=?5 AND t.proposal_revision=?6 AND t.proposal_sha=?7 AND t.state='live' AND p.revision=t.proposal_revision AND p.sha256=t.proposal_sha AND p.state='live'",params![gate.task_id,gate.lane_id,gate.revision,gate.request_sha,gate.proposal.id,gate.proposal.revision,gate.proposal.sha256],|r|r.get(0))?;
    if live != 1 || gate.decision.id != verified.answer_id || !crate::valid_ref(&gate.decision) {
        return Err(Error::Held);
    }
    let existing_task: Option<(String, String)> = tx
        .query_row(
            "SELECT request_sha,request_json FROM task_revision WHERE task_id=?1 AND revision=?2",
            params![gate.task_id, gate.revision],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((sha, raw)) = existing_task {
        if sha != gate.request_sha || raw != request {
            return Err(Error::Conflict);
        }
    } else {
        tx.execute(
            "INSERT INTO task_revision VALUES (?1,?2,?3,?4)",
            params![gate.task_id, gate.revision, gate.request_sha, request],
        )?;
    }
    let inserted = tx.execute(
        "INSERT INTO admission VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
        params![
            verified.answer_id,
            request_id,
            gate.task_id,
            gate.revision,
            gate.request_sha,
            gate.lane_id,
            gate.proposal.id,
            gate.proposal.revision,
            gate.proposal.sha256,
            gate.decision.revision,
            gate.decision.sha256,
            answer_sha,
            answer,
            verified.assertion_digest,
            admission_id,
            claim_id,
            operation
        ],
    );
    if let Err(error) = inserted {
        return match error {
            rusqlite::Error::SqliteFailure(ref code, _)
                if code.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(Error::Conflict)
            }
            _ => Err(Error::Store),
        };
    }
    let claim = tx.execute(
        "INSERT INTO capacity_claim VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'active')",
        params![
            claim_id,
            operation,
            gate.objective_id,
            gate.request_sha,
            gate.canonical_budget_revision,
            gate.observation_ref,
            gate.exclusive_binding_ref,
            gate.maximum,
            gate.productive_maximum,
            gate.closure_reserve
        ],
    );
    if let Err(error) = claim {
        return match error {
            rusqlite::Error::SqliteFailure(ref code, _)
                if code.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(Error::Held)
            }
            _ => Err(Error::Store),
        };
    }
    tx.execute("INSERT INTO outbox VALUES (?1,'pending')", [&operation])?;
    tx.commit()?;
    Ok(Admission {
        id: admission_id,
        operation,
    })
}

/// Authenticated read of immutable accepted state. No current source, proof,
/// revocation or budget check is made and no outbox transition is performed.
#[derive(Debug)]
pub(crate) struct ExistingView {
    pub admission: Admission,
    pub status: String,
}
pub(crate) fn retrieve_existing(
    db: &Connection,
    request: &crate::request::AdmissionRequest,
    actor: &str,
) -> Result<Option<ExistingView>, Error> {
    crate::request::AdmissionRequest::parse(&serde_json::to_string(&serde_json::json!({"schema":request.schema,"request_id":request.request_id,"task":request.task,"decision":request.decision,"expected_task_revision":request.expected_task_revision})).map_err(|_|Error::Store)?,&request.task,&request.decision).map_err(|_|Error::Conflict)?;
    type Stored = (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    );
    let mut statement=db.prepare("SELECT a.request_id,a.task_id,a.revision,a.request_sha,a.answer_id,a.decision_revision,a.decision_sha,a.answer_json,a.answer_sha,a.admission_id,a.operation_id,o.state,t.request_json FROM admission a LEFT JOIN outbox o ON o.operation_id=a.operation_id LEFT JOIN task_revision t ON t.task_id=a.task_id AND t.revision=a.revision WHERE a.request_id=?1 OR a.answer_id=?2")?;
    let rows = statement
        .query_map(params![request.request_id, request.decision.id], |r| {
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
                r.get(10)?,
                r.get(11)?,
                r.get(12)?,
            ))
        })?
        .collect::<Result<Vec<Stored>, _>>()?;
    if rows.is_empty() {
        return Ok(None);
    }
    if rows.len() != 1 {
        return Err(Error::Conflict);
    }
    let (
        rid,
        id,
        rev,
        sha,
        answer_id,
        decision_revision,
        decision_sha,
        answer_json,
        answer_sha,
        admission_id,
        operation,
        state,
        raw,
    ) = rows.into_iter().next().ok_or(Error::Store)?;
    let row: crate::Row = serde_json::from_str(&raw).map_err(|_| Error::Store)?;
    let answer: serde_json::Value = serde_json::from_str(&answer_json).map_err(|_| Error::Store)?;
    if rid != request.request_id
        || id != request.task.id
        || rev != request.task.revision
        || sha != request.task.sha256
        || hash(&raw) != sha
        || row.data.intent.task_id != id
        || row.data.intent.revision != rev
        || row.data.intent.submitter != actor
        || actor.is_empty()
        || answer_id != request.decision.id
        || decision_revision != request.decision.revision
        || decision_sha != request.decision.sha256
        || hash(&answer_json) != answer_sha
        || answer.get("answer_id").and_then(serde_json::Value::as_str) != Some(answer_id.as_str())
        || answer
            .get("request_digest")
            .and_then(serde_json::Value::as_str)
            != Some(sha.as_str())
    {
        return Err(Error::Conflict);
    }
    let status = match state.as_str() {
        "pending" => "delivery_pending",
        "unknown" | "delivered" => "start_unknown",
        _ => return Err(Error::Store),
    };
    Ok(Some(ExistingView {
        admission: Admission {
            id: admission_id,
            operation,
        },
        status: status.into(),
    }))
}

// Read an existing exact admission independently of action-time eligibility.
// A caller must still be authenticated; this function never starts delivery.
#[allow(dead_code)]
pub(crate) fn lookup_exact(
    db: &Connection,
    request_id: &str,
    task: &crate::Ref,
    decision: &crate::Ref,
    answer_json: &str,
    assertion_sha: &str,
) -> Result<Option<Admission>, Error> {
    type Existing = (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    );
    let existing:Option<Existing>=db.query_row("SELECT request_id,task_id,revision,request_sha,answer_json,assertion_sha,answer_id,admission_id,operation_id FROM admission WHERE request_id=?1 OR answer_id=?2",params![request_id,decision.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?))).optional()?;
    match existing {
        None => Ok(None),
        Some((req, id, rev, sha, answer, proof, aid, admission_id, operation)) => {
            let stored_decision: (String, String) = db.query_row(
                "SELECT decision_revision,decision_sha FROM admission WHERE answer_id=?1",
                [&decision.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if !crate::valid_ref(decision)
                || stored_decision.0 != decision.revision
                || stored_decision.1 != decision.sha256
            {
                return Err(Error::Conflict);
            }
            if req != request_id
                || id != task.id
                || rev != task.revision
                || sha != task.sha256
                || answer != answer_json
                || proof != assertion_sha
                || aid != decision.id
            {
                return Err(Error::Conflict);
            }
            Ok(Some(Admission {
                id: admission_id,
                operation,
            }))
        }
    }
}

// Shared exact-admission liveness invariant. Callers that grant an effect lease
// must retain the transaction that owns this observation through the effect.
pub(crate) fn admitted_heads_live(db: &Connection, operation: &str) -> Result<(), Error> {
    let live:i64=db.query_row("SELECT count(*) FROM admission a JOIN task_head t ON t.id=a.task_id JOIN proposal_head p ON p.id=a.proposal_id WHERE a.operation_id=?1 AND t.state='live' AND t.lane_id=a.lane_id AND t.revision=a.revision AND t.sha256=a.request_sha AND t.proposal_id=a.proposal_id AND t.proposal_revision=a.proposal_revision AND t.proposal_sha=a.proposal_sha AND p.state='live' AND p.revision=a.proposal_revision AND p.sha256=a.proposal_sha",[operation],|r|r.get(0))?;
    if live != 1 {
        return Err(Error::Held);
    }
    Ok(())
}

// Persist uncertainty *before* a native entry; restart never silently redelivers.
#[allow(dead_code)]
pub(crate) fn fence_delivery(db: &Connection, operation: &str) -> Result<(), Error> {
    let tx = db.unchecked_transaction()?;
    admitted_heads_live(&tx, operation)?;
    if tx.execute(
        "UPDATE outbox SET state='unknown' WHERE operation_id=?1 AND state='pending'",
        [operation],
    )? != 1
        || tx.execute(
            "UPDATE capacity_claim SET state='unknown' WHERE operation_id=?1 AND state='active'",
            [operation],
        )? != 1
    {
        return Err(Error::Held);
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fake_proposal() -> crate::Ref {
        crate::Ref {
            id: "proposal".into(),
            revision: "p1".into(),
            sha256: "1".repeat(64),
        }
    }
    fn fake_decision(id: &str) -> crate::Ref {
        crate::Ref {
            id: id.into(),
            revision: "2".into(),
            sha256: "2".repeat(64),
        }
    }
    fn seed(db: &Connection, task: &str, raw: &str) {
        db.execute(
            "INSERT OR IGNORE INTO proposal_head VALUES ('proposal','p1',?1,'live',1)",
            ["1".repeat(64)],
        )
        .unwrap();
        db.execute(
            "INSERT INTO task_head VALUES (?1,'lane','r1',?2,'proposal','p1',?3,'live',1)",
            params![task, hash(raw), "1".repeat(64)],
        )
        .unwrap();
    }
    #[test]
    fn transaction_replay_conflict_and_uncertain_delivery() {
        let mut db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        let raw = "request";
        let mut v = VerifiedAnswer {
            decision: "approve".into(),
            comment: String::new(),
            answer_id: "answer".into(),
            answer_digest: hash("answer"),
            assertion_digest: "synthetic".into(),
            board_seq: 1,
            request_digest: hash(raw),
            expires_at: time::OffsetDateTime::UNIX_EPOCH,
        };
        let gate = CurrentGate {
            task_id: "task".into(),
            revision: "r1".into(),
            request_sha: hash(raw),
            lane_id: "lane".into(),
            proposal: fake_proposal(),
            decision: fake_decision("answer"),
            objective_id: "objective".into(),
            canonical_budget_revision: "budget-r1".into(),
            observation_ref: "observation-r1".into(),
            exclusive_binding_ref: "exclusive-r1".into(),
            maximum: "5".into(),
            productive_maximum: "4".into(),
            closure_reserve: "1".into(),
        };
        seed(&db, "task", raw);
        for decision in ["reject", "defer", "ask"] {
            v.decision = decision.into();
            assert_eq!(
                commit(&mut db, "request-id", raw, "answer", &v, &gate),
                Err(Error::Held)
            );
        }
        v.decision = "approve".into();
        v.comment = "not empty".into();
        assert_eq!(
            commit(&mut db, "request-id", raw, "answer", &v, &gate),
            Err(Error::Held)
        );
        v.comment.clear();

        let first = commit(&mut db, "request-id", raw, "answer", &v, &gate).unwrap();
        assert_eq!(
            first,
            commit(&mut db, "request-id", raw, "answer", &v, &gate).unwrap()
        );
        assert_eq!(
            commit(&mut db, "request-id", raw, "changed", &v, &gate),
            Err(Error::Held)
        );
        let changed = VerifiedAnswer {
            decision: "approve".into(),
            comment: String::new(),
            answer_id: "answer".into(),
            answer_digest: hash("changed"),
            assertion_digest: "synthetic".into(),
            board_seq: 1,
            request_digest: hash(raw),
            expires_at: time::OffsetDateTime::UNIX_EPOCH,
        };
        assert_eq!(
            commit(&mut db, "request-id", raw, "changed", &changed, &gate),
            Err(Error::Conflict)
        );
        let other = VerifiedAnswer {
            decision: "approve".into(),
            comment: String::new(),
            answer_id: "other".into(),
            answer_digest: hash("answer"),
            assertion_digest: "synthetic".into(),
            board_seq: 1,
            request_digest: hash(raw),
            expires_at: time::OffsetDateTime::UNIX_EPOCH,
        };
        assert_eq!(
            commit(&mut db, "other-request", raw, "answer", &other, &gate),
            Err(Error::Held)
        );
        let task = crate::Ref {
            id: "task".into(),
            revision: "r1".into(),
            sha256: hash(raw),
        };
        assert_eq!(
            lookup_exact(
                &db,
                "request-id",
                &task,
                &fake_decision("answer"),
                "answer",
                "synthetic"
            ),
            Ok(Some(Admission {
                id: first.id.clone(),
                operation: first.operation.clone()
            }))
        );
        assert_eq!(
            lookup_exact(
                &db,
                "request-id",
                &task,
                &fake_decision("answer"),
                "answer",
                "different"
            ),
            Err(Error::Conflict)
        );
        fence_delivery(&db, &first.operation).unwrap();
        assert_eq!(fence_delivery(&db, &first.operation), Err(Error::Held));
        assert_eq!(
            db.query_row("SELECT state FROM capacity_claim", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "unknown"
        );
        let other_gate = CurrentGate {
            task_id: "another-task".into(),
            revision: "r1".into(),
            request_sha: hash("another-request"),
            lane_id: "lane".into(),
            proposal: fake_proposal(),
            decision: fake_decision("other-answer"),
            objective_id: "objective".into(),
            canonical_budget_revision: "budget-r1".into(),
            observation_ref: "observation-r1".into(),
            exclusive_binding_ref: "exclusive-r1".into(),
            maximum: "5".into(),
            productive_maximum: "4".into(),
            closure_reserve: "1".into(),
        };
        let other_proof = VerifiedAnswer {
            decision: "approve".into(),
            comment: String::new(),
            answer_id: "other-answer".into(),
            answer_digest: hash("other-answer"),
            assertion_digest: "synthetic".into(),
            board_seq: 2,
            request_digest: hash("another-request"),
            expires_at: time::OffsetDateTime::UNIX_EPOCH,
        };
        seed(&db, "another-task", "another-request");
        assert_eq!(
            commit(
                &mut db,
                "another-id",
                "another-request",
                "other-answer",
                &other_proof,
                &other_gate
            ),
            Err(Error::Held)
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM admission", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM outbox", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn outbox_failure_rolls_back_admission_and_task() {
        let mut db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        db.execute_batch("CREATE TRIGGER fail_outbox BEFORE INSERT ON outbox BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
        let v = VerifiedAnswer {
            decision: "approve".into(),
            comment: String::new(),
            answer_id: "answer".into(),
            answer_digest: hash("answer"),
            assertion_digest: "synthetic".into(),
            board_seq: 1,
            request_digest: hash("request"),
            expires_at: time::OffsetDateTime::UNIX_EPOCH,
        };
        let gate = CurrentGate {
            task_id: "task".into(),
            revision: "r1".into(),
            request_sha: hash("request"),
            lane_id: "lane".into(),
            proposal: fake_proposal(),
            decision: fake_decision("answer"),
            objective_id: "objective".into(),
            canonical_budget_revision: "budget-r1".into(),
            observation_ref: "observation-r1".into(),
            exclusive_binding_ref: "exclusive-r1".into(),
            maximum: "5".into(),
            productive_maximum: "4".into(),
            closure_reserve: "1".into(),
        };
        seed(&db, "task", "request");
        assert_eq!(
            commit(&mut db, "id", "request", "answer", &v, &gate),
            Err(Error::Store)
        );
        for table in ["task_revision", "admission", "capacity_claim", "outbox"] {
            assert_eq!(
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, u32>(0))
                    .unwrap(),
                0
            );
        }
    }
    #[test]
    fn unknown_delivery_survives_restart() {
        let path =
            std::env::temp_dir().join(format!("task-admission-test-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut db = Connection::open(&path).unwrap();
        initialize(&db).unwrap();
        let v = VerifiedAnswer {
            decision: "approve".into(),
            comment: String::new(),
            answer_id: "answer".into(),
            answer_digest: hash("answer"),
            assertion_digest: "synthetic".into(),
            board_seq: 1,
            request_digest: hash("request"),
            expires_at: time::OffsetDateTime::UNIX_EPOCH,
        };
        let gate = CurrentGate {
            task_id: "task".into(),
            revision: "r1".into(),
            request_sha: hash("request"),
            lane_id: "lane".into(),
            proposal: fake_proposal(),
            decision: fake_decision("answer"),
            objective_id: "objective".into(),
            canonical_budget_revision: "budget-r1".into(),
            observation_ref: "observation-r1".into(),
            exclusive_binding_ref: "exclusive-r1".into(),
            maximum: "5".into(),
            productive_maximum: "4".into(),
            closure_reserve: "1".into(),
        };
        seed(&db, "task", "request");
        observe_funding(&mut db, "task", "r1", "request", 100, "first-receipt").unwrap();
        let entry = commit(&mut db, "id", "request", "answer", &v, &gate).unwrap();
        fence_delivery(&db, &entry.operation).unwrap();
        drop(db);
        let mut db = Connection::open(&path).unwrap();
        initialize(&db).unwrap();
        observe_funding(&mut db, "task", "r1", "request", 200, "later-receipt").unwrap();
        assert_eq!(
            db.query_row("SELECT first_funded FROM funding_clock", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            100
        );
        assert_eq!(fence_delivery(&db, &entry.operation), Err(Error::Held));
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn exact_v1_stage_migration_preserves_claim_and_rejects_tampering() {
        for altered in [false, true] {
            let db = Connection::open_in_memory().unwrap();
            for (name, ddl) in TABLES
                .iter()
                .filter(|(name, _)| *name != "scoped_final_claim")
            {
                let _ = name;
                db.execute_batch(ddl).unwrap();
            }
            db.execute_batch("PRAGMA user_version=1").unwrap();
            if altered {
                db.execute_batch("ALTER TABLE scoped_claim ADD COLUMN unauthorized TEXT")
                    .unwrap();
                assert!(initialize(&db).is_err());
            } else {
                initialize(&db).unwrap();
                initialize(&db).unwrap();
                assert_eq!(
                    db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                        .unwrap(),
                    2
                );
            }
        }
    }
    #[test]
    fn refuses_partial_or_altered_store_schema() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE task_revision(task_id TEXT)")
            .unwrap();
        assert_eq!(initialize(&db), Err(Error::Store));
        let db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        db.execute_batch("CREATE TABLE extra(id TEXT)").unwrap();
        assert_eq!(initialize(&db), Err(Error::Store));
    }
    #[test]
    fn first_funded_clock_does_not_reset() {
        let mut db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        observe_funding(&mut db, "task", "r1", "request", 100, "receipt1").unwrap();
        observe_funding(&mut db, "task", "r1", "request", 200, "receipt2").unwrap();
        let observed: (i64, String) = db
            .query_row(
                "SELECT first_funded,source_ref FROM funding_clock",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(observed, (100, "receipt1".into()));
        assert_eq!(
            observe_funding(&mut db, "task", "r1", "changed", 50, "receipt3"),
            Err(Error::Conflict)
        );
    }
    #[test]
    fn registry_cas_and_immutable_refs_hold_stale_updates() {
        let mut db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/projection-fixture.json")).unwrap();
        let raw = fixture["tasks"][0]["request_json"].as_str().unwrap();
        let row: crate::Row = serde_json::from_str(raw).unwrap();
        let proposal = row.data.intent.proposal;
        assert_eq!(register_task(&mut db, raw, "live", None), Err(Error::Held));
        assert_eq!(register_proposal(&mut db, &proposal, "live", None), Ok(1));
        assert_eq!(register_task(&mut db, raw, "live", None), Ok(1));
        assert_eq!(
            register_task(&mut db, raw, "withdrawn", None),
            Err(Error::Conflict)
        );
        assert_eq!(register_task(&mut db, raw, "withdrawn", Some(1)), Ok(2));
        assert_eq!(
            register_task(&mut db, raw, "live", Some(2)),
            Err(Error::Held)
        );
        let mut changed = proposal.clone();
        changed.sha256 = "f".repeat(64);
        assert_eq!(
            register_proposal(&mut db, &changed, "live", Some(1)),
            Err(Error::Conflict)
        );
    }
    #[test]
    fn withdrawn_head_fences_delivery_without_releasing_claim() {
        let mut db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        seed(&db, "task", "request");
        let proof = VerifiedAnswer {
            decision: "approve".into(),
            comment: String::new(),
            answer_id: "answer".into(),
            answer_digest: hash("answer"),
            assertion_digest: "synthetic".into(),
            board_seq: 1,
            request_digest: hash("request"),
            expires_at: time::OffsetDateTime::UNIX_EPOCH,
        };
        let gate = CurrentGate {
            task_id: "task".into(),
            revision: "r1".into(),
            request_sha: hash("request"),
            lane_id: "lane".into(),
            proposal: fake_proposal(),
            decision: fake_decision("answer"),
            objective_id: "objective".into(),
            canonical_budget_revision: "r1".into(),
            observation_ref: "obs".into(),
            exclusive_binding_ref: "exclusive".into(),
            maximum: "5".into(),
            productive_maximum: "4".into(),
            closure_reserve: "1".into(),
        };
        let admission = commit(&mut db, "request-id", "request", "answer", &proof, &gate).unwrap();
        db.execute("UPDATE task_head SET state='withdrawn'", [])
            .unwrap();
        assert_eq!(fence_delivery(&db, &admission.operation), Err(Error::Held));
        assert_eq!(
            db.query_row("SELECT state FROM capacity_claim", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "active"
        );
    }
}
