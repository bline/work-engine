//! The private SQLite authority store. One database, one total commit order (`log.seq`).
//!
//! Outcome taxonomy (owner P1 section 4, retained by P1-F):
//! - `NotCommitted`: only when no transaction began (store failed closed, `BEGIN` refused). No write
//!   was possible.
//! - `Done(..)`: only after a checked `COMMIT` returned, under the honest-storage assumptions in
//!   [`HONEST_STORAGE_ASSUMPTIONS`].
//! - `Unknown`: any error after `BEGIN` succeeded, a `COMMIT` error, or a lost reply. The handle
//!   then fails closed; the caller must reopen and `reconcile`. Never `NotCommitted` after a commit
//!   attempt, never an automatic re-publish.
use crate::ident::{derived_operation_id, digest_fields, sha256_hex};
use crate::receipt::{self, ReceiptRefusal};
use crate::report::{NoneRow, none_rows};
use crate::schema::{DDL, FIXTURE_LABEL, FIXTURE_SCOPE, SCHEMA_VERSION};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::time::Duration;
use work_engine_organization::{OrganizationCandidate, Profile, enforceability};

pub const DB_FILE: &str = "authority.sqlite3";
pub const HONEST_STORAGE_ASSUMPTIONS: &str = "Committed means a checked COMMIT returned with journal_mode=WAL and synchronous=FULL on honest local storage (fsync is honored, no silent corruption). Process crash is exercised; power loss and storage lying are NOT qualified.";

/// The mode of every write transaction. Shared by publish, revoke, supersede and withdraw.
const BEGIN_MODE: TransactionBehavior = TransactionBehavior::Immediate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    Ordinary,
    Fixture,
}
impl Domain {
    pub fn as_str(self) -> &'static str {
        match self {
            Domain::Ordinary => "ordinary",
            Domain::Fixture => "fixture",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Write<T> {
    Done(T),
    NotCommitted(String),
    Unknown(String),
}
impl<T> Write<T> {
    pub fn done(self) -> T {
        match self {
            Write::Done(v) => v,
            Write::NotCommitted(m) => panic!("NotCommitted: {m}"),
            Write::Unknown(m) => panic!("Unknown: {m}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultPoint {
    BeforeBegin,
    AfterBegin,
    BeforeCommit,
    AfterCommit,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultAction {
    Continue,
    Fail(String),
}
pub type Hook = Box<dyn FnMut(FaultPoint, &Connection) -> FaultAction + Send>;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(String),
    #[error("invalid store: {0}")]
    Invalid(String),
}

pub struct Store {
    conn: Connection,
    dir: PathBuf,
    failed_closed: bool,
    hook: Option<Hook>,
}

fn configure(conn: &Connection, busy: Duration) -> rusqlite::Result<()> {
    conn.busy_timeout(busy)?;
    conn.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;")
}

impl Store {
    /// Create a store in a private directory (mode 0700, file 0600). Fails if one exists.
    pub fn create(dir: &Path, store_id: &str, busy: Duration) -> Result<Self, StoreError> {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(dir)
            .map_err(|e| StoreError::Io(e.to_string()))?;
        let path = dir.join(DB_FILE);
        let conn = Connection::open(&path)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| StoreError::Io(e.to_string()))?;
        configure(&conn, busy)?;
        conn.execute_batch(DDL)?;
        conn.execute(
            "INSERT INTO meta(store_id, schema_version) VALUES(?1, ?2)",
            params![store_id, SCHEMA_VERSION],
        )?;
        Ok(Self {
            conn,
            dir: dir.to_owned(),
            failed_closed: false,
            hook: None,
        })
    }

    pub fn open(dir: &Path, busy: Duration) -> Result<Self, StoreError> {
        let conn = Connection::open_with_flags(
            dir.join(DB_FILE),
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        configure(&conn, busy)?;
        let v: i64 = conn.query_row("SELECT schema_version FROM meta", [], |r| r.get(0))?;
        if v != SCHEMA_VERSION {
            return Err(StoreError::Invalid(format!("schema version {v}")));
        }
        Ok(Self {
            conn,
            dir: dir.to_owned(),
            failed_closed: false,
            hook: None,
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
    pub fn is_failed_closed(&self) -> bool {
        self.failed_closed
    }
    /// Test hook for fault injection. Present only with the `fixture` feature.
    #[cfg(feature = "fixture")]
    pub fn set_hook(&mut self, hook: Hook) {
        self.hook = Some(hook);
    }
    /// Raw connection for read-only inspection by tests and the fixture driver.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    fn fire(hook: &mut Option<Hook>, p: FaultPoint, conn: &Connection) -> FaultAction {
        match hook {
            Some(h) => h(p, conn),
            None => FaultAction::Continue,
        }
    }

    /// The single write path. Every authority write, including publication, goes through here.
    pub(crate) fn write_txn<T>(
        &mut self,
        f: impl FnOnce(&Transaction) -> rusqlite::Result<T>,
    ) -> Write<T> {
        if self.failed_closed {
            return Write::NotCommitted(
                "store handle failed closed after an uncertain outcome; reopen and reconcile"
                    .into(),
            );
        }
        let Self {
            conn,
            hook,
            failed_closed,
            ..
        } = self;
        if let FaultAction::Fail(m) = Self::fire(hook, FaultPoint::BeforeBegin, conn) {
            return Write::NotCommitted(m);
        }
        let tx = match Transaction::new_unchecked(conn, BEGIN_MODE) {
            Ok(tx) => tx,
            Err(e) => {
                let busy = matches!(
                    e.sqlite_error_code(),
                    Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
                );
                if !busy {
                    *failed_closed = true;
                }
                return Write::NotCommitted(format!("begin: {e}"));
            }
        };
        // From here a transaction is open: nothing below may report NotCommitted.
        if let FaultAction::Fail(m) = Self::fire(hook, FaultPoint::AfterBegin, conn) {
            *failed_closed = true;
            return Write::Unknown(m);
        }
        let value = match f(&tx) {
            Ok(v) => v,
            Err(e) => {
                *failed_closed = true;
                return Write::Unknown(format!("error after BEGIN: {e}"));
            }
        };
        if let FaultAction::Fail(m) = Self::fire(hook, FaultPoint::BeforeCommit, conn) {
            *failed_closed = true;
            return Write::Unknown(m);
        }
        if let Err(e) = tx.commit() {
            *failed_closed = true;
            return Write::Unknown(format!("commit error: {e}"));
        }
        if let FaultAction::Fail(m) = Self::fire(hook, FaultPoint::AfterCommit, conn) {
            *failed_closed = true;
            return Write::Unknown(format!("reply lost after commit: {m}"));
        }
        Write::Done(value)
    }
}

pub(crate) fn log_row(tx: &Transaction, kind: &str, r: &str) -> rusqlite::Result<i64> {
    tx.execute(
        "INSERT INTO log(kind, ref) VALUES(?1, ?2)",
        params![kind, r],
    )?;
    Ok(tx.last_insert_rowid())
}

// ---------------------------------------------------------------------------------------------
// Ordinary admission API. In this slice it can only refuse (Gate O is a design, not built here).
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRefusal {
    pub reasons: Vec<String>,
    pub none_rows: Vec<NoneRow>,
}

impl Store {
    /// Ordinary (non-fixture) admission. Applies the O5 rule: any `None` row in the enforceability
    /// report refuses. The report is computed here from the candidate, never accepted from the
    /// caller. The return type has no success value: this slice issues no ordinary admission.
    pub fn admit(&self, candidate: &OrganizationCandidate) -> Result<Infallible, AdmissionRefusal> {
        let report = enforceability(candidate, Profile::FixedSliceReviewV1);
        let rows = none_rows(&report);
        if !rows.is_empty() {
            return Err(AdmissionRefusal {
                reasons: rows
                    .iter()
                    .map(|r| {
                        format!(
                            "O5: report row `{}` scope `{}` requirement `{}` is None ({})",
                            r.property, r.scope, r.requirement, r.note
                        )
                    })
                    .collect(),
                none_rows: rows,
            });
        }
        Err(AdmissionRefusal {
            reasons: vec![
                "ordinary admission issuance is not implemented in this slice (Gate O is a design)"
                    .into(),
            ],
            none_rows: vec![],
        })
    }

    /// Ordinary publication. No ordinary admission can exist, so this always refuses; it also
    /// never reads fixture rows (the domain filter below is the only reader).
    pub fn publish_ordinary(&mut self, req: &PublishRequest) -> Write<PublishOutcome> {
        self.publish_in_domain(Domain::Ordinary, req)
    }
}

// ---------------------------------------------------------------------------------------------
// Publication.
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishRequest {
    pub operation_id: String,
    pub admission_id: String,
    pub grant_id: String,
    pub obligation_id: String,
    pub execution_id: String,
    pub org_revision: i64,
    pub requirements_digest: String,
    pub candidate_digest: String,
    pub receipt_id: String,
    pub receipt_bytes: Vec<u8>,
    pub body: Vec<u8>,
}
impl PublishRequest {
    pub fn request_digest(&self) -> String {
        digest_fields(
            "p1f.publish-request.v1",
            &[
                self.operation_id.as_bytes(),
                self.admission_id.as_bytes(),
                self.grant_id.as_bytes(),
                self.obligation_id.as_bytes(),
                self.execution_id.as_bytes(),
                &self.org_revision.to_be_bytes(),
                self.requirements_digest.as_bytes(),
                self.candidate_digest.as_bytes(),
                self.receipt_id.as_bytes(),
                sha256_hex(&self.receipt_bytes).as_bytes(),
                sha256_hex(&self.body).as_bytes(),
            ],
        )
    }
    pub fn derived_operation_id(&self) -> String {
        derived_operation_id(&self.admission_id, &self.obligation_id, &self.execution_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishOutcome {
    Published {
        result_id: String,
        seq: i64,
        replayed: bool,
    },
    Refused {
        reason: String,
        seq: i64,
        replayed: bool,
    },
    /// The operation id is bound to a different request digest. Nothing was written.
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    DomainMismatch,
    RequirementsSuperseded,
    OrganizationIdentityMismatch,
    AdmissionMissing,
    AdmissionNotAdmitted,
    AdmissionStale,
    GrantMissing,
    GrantRevoked,
    GrantScopeMismatch,
    GrantWrongAdmission,
    ExecutionMissing,
    ExecutionNotFinished,
    ExecutionMismatch,
    ReceiptNotTheRecordedOne,
    Evidence(ReceiptRefusal),
    AlreadyPublished,
    BodyTooLarge,
    OrdinaryResultsNotImplemented,
}
impl Refusal {
    pub fn reason(&self) -> String {
        match self {
            Refusal::Evidence(e) => format!("evidence: {e}"),
            other => format!("{other:?}"),
        }
    }
}

struct Row3 {
    a: String,
    b: String,
    c: String,
}

fn opt<T>(r: rusqlite::Result<T>) -> rusqlite::Result<Option<T>> {
    r.optional()
}

macro_rules! refuse_if {
    ($tx:expr, $req:expr, $seq:expr, $check:expr) => {
        if let Some(r) = $check {
            return finish_refusal($tx, $req, $seq, r);
        }
    };
}

fn finish_refusal(
    tx: &Transaction,
    req: &PublishRequest,
    seq: i64,
    r: Refusal,
) -> rusqlite::Result<PublishOutcome> {
    let reason = r.reason();
    tx.execute(
        "INSERT INTO operations(operation_id, request_digest, outcome, reason, result_id, seq) VALUES(?1, ?2, 'refused', ?3, NULL, ?4)",
        params![req.operation_id, req.request_digest(), reason, seq],
    )?;
    Ok(PublishOutcome::Refused {
        reason,
        seq,
        replayed: false,
    })
}

impl Store {
    pub(crate) fn publish_in_domain(
        &mut self,
        domain: Domain,
        req: &PublishRequest,
    ) -> Write<PublishOutcome> {
        let req = req.clone();
        self.write_txn(move |tx| publish_txn(tx, domain, &req))
    }
}

fn publish_txn(
    tx: &Transaction,
    domain: Domain,
    req: &PublishRequest,
) -> rusqlite::Result<PublishOutcome> {
    // Idempotency: replay returns the stored outcome with no new log row.
    let prior = opt(tx.query_row(
        "SELECT request_digest, outcome, reason, result_id, seq FROM operations WHERE operation_id=?1",
        [&req.operation_id],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, Option<String>>(3)?, r.get::<_, i64>(4)?)),
    ))?;
    if let Some((digest, outcome, reason, result_id, seq)) = prior {
        if digest != req.request_digest() {
            return Ok(PublishOutcome::Conflict);
        }
        return Ok(if outcome == "published" {
            PublishOutcome::Published {
                result_id: result_id.unwrap_or_default(),
                seq,
                replayed: true,
            }
        } else {
            PublishOutcome::Refused {
                reason: reason.unwrap_or_default(),
                seq,
                replayed: true,
            }
        });
    }
    let seq = log_row(tx, "publish", &req.operation_id)?;
    // M:body
    refuse_if!(tx, req, seq, check_body(req));
    // M:domain
    refuse_if!(tx, req, seq, check_domain(tx, domain, req)?);
    // M:requirements
    refuse_if!(tx, req, seq, check_requirements(tx, domain, req)?);
    // M:organization
    refuse_if!(tx, req, seq, check_organization(tx, domain, req)?);
    // M:admission
    refuse_if!(tx, req, seq, check_admission(tx, req)?);
    // M:permission
    refuse_if!(tx, req, seq, check_permission(tx, req)?);
    // M:execution
    refuse_if!(tx, req, seq, check_execution(tx, req)?);
    // M:evidence
    refuse_if!(tx, req, seq, check_evidence(tx, req)?);
    // M:supersession
    refuse_if!(tx, req, seq, check_unpublished(tx, req)?);
    if domain != Domain::Fixture {
        return finish_refusal(tx, req, seq, Refusal::OrdinaryResultsNotImplemented);
    }
    let (grant_revision,): (i64,) = tx.query_row(
        "SELECT revision FROM grants WHERE grant_id=?1",
        [&req.grant_id],
        |r| Ok((r.get(0)?,)),
    )?;
    let unestablished: String = tx.query_row(
        "SELECT unestablished_json FROM admissions WHERE admission_id=?1",
        [&req.admission_id],
        |r| r.get(0),
    )?;
    let result_id = digest_fields("p1f.result.v1", &[req.operation_id.as_bytes()]);
    tx.execute(
        "INSERT INTO operations(operation_id, request_digest, outcome, reason, result_id, seq) VALUES(?1, ?2, 'published', NULL, ?3, ?4)",
        params![req.operation_id, req.request_digest(), result_id, seq],
    )?;
    tx.execute(
        "INSERT INTO results(result_id, operation_id, domain, provenance, result_class, attestation, label, execution_id, admission_id, grant_id, grant_revision, org_revision, candidate_digest, requirements_digest, obligation_id, body, unestablished_json, seq) VALUES(?1, ?2, 'fixture', 'fixture-only', 'fixture_experiment', 'none', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![result_id, req.operation_id, FIXTURE_LABEL, req.execution_id, req.admission_id, req.grant_id, grant_revision, req.org_revision, req.candidate_digest, req.requirements_digest, req.obligation_id, req.body, unestablished, seq],
    )?;
    tx.execute(
        "INSERT INTO receipts(receipt_id, result_id, bytes) VALUES(?1, ?2, ?3)",
        params![req.receipt_id, result_id, req.receipt_bytes],
    )?;
    Ok(PublishOutcome::Published {
        result_id,
        seq,
        replayed: false,
    })
}

fn check_body(req: &PublishRequest) -> Option<Refusal> {
    (req.body.len() > 65_536).then_some(Refusal::BodyTooLarge)
}

/// Admission, grant and organization state must all belong to the requested domain, so a fixture
/// row can never satisfy the ordinary path and an ordinary row can never satisfy the fixture path.
fn check_domain(
    tx: &Transaction,
    domain: Domain,
    req: &PublishRequest,
) -> rusqlite::Result<Option<Refusal>> {
    let adm = opt(tx.query_row(
        "SELECT domain FROM admissions WHERE admission_id=?1",
        [&req.admission_id],
        |r| r.get::<_, String>(0),
    ))?;
    let gr = opt(tx.query_row(
        "SELECT domain FROM grants WHERE grant_id=?1",
        [&req.grant_id],
        |r| r.get::<_, String>(0),
    ))?;
    let org = opt(tx.query_row(
        "SELECT domain FROM org_state WHERE domain=?1",
        [domain.as_str()],
        |r| r.get::<_, String>(0),
    ))?;
    Ok(match (adm, gr, org) {
        (Some(a), Some(g), Some(o))
            if a == domain.as_str() && g == domain.as_str() && o == domain.as_str() =>
        {
            None
        }
        (None, _, _) => Some(Refusal::AdmissionMissing),
        (_, None, _) => Some(Refusal::GrantMissing),
        _ => Some(Refusal::DomainMismatch),
    })
}

/// Request quotes the current organization revision and requirements digest.
fn check_requirements(
    tx: &Transaction,
    domain: Domain,
    req: &PublishRequest,
) -> rusqlite::Result<Option<Refusal>> {
    let cur = opt(tx.query_row(
        "SELECT org_revision, requirements_digest FROM org_state WHERE domain=?1",
        [domain.as_str()],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
    ))?;
    Ok(match cur {
        Some((rev, dig)) if rev == req.org_revision && dig == req.requirements_digest => None,
        _ => Some(Refusal::RequirementsSuperseded),
    })
}

/// Request quotes the compiled organization identity currently installed.
fn check_organization(
    tx: &Transaction,
    domain: Domain,
    req: &PublishRequest,
) -> rusqlite::Result<Option<Refusal>> {
    let cur = opt(tx.query_row(
        "SELECT candidate_digest FROM org_state WHERE domain=?1",
        [domain.as_str()],
        |r| r.get::<_, String>(0),
    ))?;
    Ok(match cur {
        Some(c) if c == req.candidate_digest => None,
        _ => Some(Refusal::OrganizationIdentityMismatch),
    })
}

/// Admission is admitted (not withdrawn) and was made for exactly the quoted revision and
/// requirements, and for the installed candidate.
fn check_admission(tx: &Transaction, req: &PublishRequest) -> rusqlite::Result<Option<Refusal>> {
    let row = opt(tx.query_row(
        "SELECT a.state, a.org_revision, a.requirements_digest, a.candidate_digest, o.candidate_digest FROM admissions a JOIN org_state o ON o.domain = a.domain WHERE a.admission_id=?1",
        [&req.admission_id],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?)),
    ))?;
    Ok(match row {
        None => Some(Refusal::AdmissionMissing),
        Some((state, ..)) if state != "admitted" => Some(Refusal::AdmissionNotAdmitted),
        Some((_, rev, reqd, cand, cur_cand))
            if rev == req.org_revision && reqd == req.requirements_digest && cand == cur_cand =>
        {
            None
        }
        Some(_) => Some(Refusal::AdmissionStale),
    })
}

/// Fixture-scoped permission: active, scope matches, same admission.
fn check_permission(tx: &Transaction, req: &PublishRequest) -> rusqlite::Result<Option<Refusal>> {
    let g = opt(tx.query_row(
        "SELECT state, scope, admission_id FROM grants WHERE grant_id=?1",
        [&req.grant_id],
        |r| {
            Ok(Row3 {
                a: r.get(0)?,
                b: r.get(1)?,
                c: r.get(2)?,
            })
        },
    ))?;
    Ok(match g {
        None => Some(Refusal::GrantMissing),
        Some(g) if g.a != "active" => Some(Refusal::GrantRevoked),
        Some(g) if g.b != FIXTURE_SCOPE => Some(Refusal::GrantScopeMismatch),
        Some(g) if g.c != req.admission_id => Some(Refusal::GrantWrongAdmission),
        Some(_) => None,
    })
}

/// Execution identity: a finished execution of this obligation under this admission, whose
/// recorded receipt digest is the one offered.
fn check_execution(tx: &Transaction, req: &PublishRequest) -> rusqlite::Result<Option<Refusal>> {
    let e = opt(tx.query_row(
        "SELECT admission_id, obligation_id, state, host_receipt_digest FROM executions WHERE execution_id=?1",
        [&req.execution_id],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?)),
    ))?;
    Ok(match e {
        None => Some(Refusal::ExecutionMissing),
        Some((adm, obl, ..)) if adm != req.admission_id || obl != req.obligation_id => {
            Some(Refusal::ExecutionMismatch)
        }
        Some((_, _, state, _)) if state != "finished" => Some(Refusal::ExecutionNotFinished),
        Some((_, _, _, digest)) if digest.as_deref() != Some(req.receipt_id.as_str()) => {
            Some(Refusal::ReceiptNotTheRecordedOne)
        }
        Some(_) => None,
    })
}

/// Mechanical verification on the bytes this request carries (and which are then stored).
fn check_evidence(tx: &Transaction, req: &PublishRequest) -> rusqlite::Result<Option<Refusal>> {
    let (binding, job_id): (String, String) = tx.query_row(
        "SELECT a.binding_digest, e.job_id FROM admissions a, executions e WHERE a.admission_id=?1 AND e.execution_id=?2",
        params![req.admission_id, req.execution_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(receipt::verify(
        &req.receipt_bytes,
        &req.receipt_id,
        &req.execution_id,
        &job_id,
        &binding,
    )
    .err()
    .map(Refusal::Evidence))
}

/// No result exists for this execution or for this (admission, obligation).
fn check_unpublished(tx: &Transaction, req: &PublishRequest) -> rusqlite::Result<Option<Refusal>> {
    let n: i64 = tx.query_row(
        "SELECT COUNT(*) FROM results WHERE execution_id=?1 OR (admission_id=?2 AND obligation_id=?3)",
        params![req.execution_id, req.admission_id, req.obligation_id],
        |r| r.get(0),
    )?;
    Ok((n > 0).then_some(Refusal::AlreadyPublished))
}

// ---------------------------------------------------------------------------------------------
// Shared state transitions (same transaction path, same log order).
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevokeOutcome {
    /// Effective when the commit returned. `seq` is the log position of the revocation.
    Effective {
        seq: i64,
    },
    AlreadyRevoked {
        seq: Option<i64>,
    },
    UnknownGrant,
}

impl Store {
    pub fn revoke_grant(&mut self, grant_id: &str) -> Write<RevokeOutcome> {
        let g = grant_id.to_owned();
        self.write_txn(move |tx| {
            let cur = opt(tx.query_row("SELECT state, revoked_seq FROM grants WHERE grant_id=?1", [&g], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<i64>>(1)?))))?;
            match cur {
                None => Ok(RevokeOutcome::UnknownGrant),
                Some((s, seq)) if s == "revoked" => Ok(RevokeOutcome::AlreadyRevoked { seq }),
                Some(_) => {
                    let seq = log_row(tx, "revoke", &g)?;
                    tx.execute("UPDATE grants SET state='revoked', revision=revision+1, revoked_seq=?2 WHERE grant_id=?1", params![g, seq])?;
                    Ok(RevokeOutcome::Effective { seq })
                }
            }
        })
    }

    pub fn withdraw_admission(&mut self, admission_id: &str) -> Write<i64> {
        let a = admission_id.to_owned();
        self.write_txn(move |tx| {
            let seq = log_row(tx, "withdraw", &a)?;
            tx.execute(
                "UPDATE admissions SET state='withdrawn' WHERE admission_id=?1",
                [&a],
            )?;
            Ok(seq)
        })
    }

    pub fn mark_execution_unknown(&mut self, execution_id: &str) -> Write<i64> {
        let e = execution_id.to_owned();
        self.write_txn(move |tx| {
            let seq = log_row(tx, "execution-unknown", &e)?;
            tx.execute(
                "UPDATE executions SET state='unknown' WHERE execution_id=?1 AND state='prepared'",
                [&e],
            )?;
            Ok(seq)
        })
    }

    pub fn mark_execution_finished(&mut self, execution_id: &str, receipt_id: &str) -> Write<i64> {
        let (e, r) = (execution_id.to_owned(), receipt_id.to_owned());
        self.write_txn(move |tx| {
            let seq = log_row(tx, "execution-finished", &e)?;
            tx.execute("UPDATE executions SET state='finished', host_receipt_digest=?2 WHERE execution_id=?1 AND state='prepared'", params![e, r])?;
            Ok(seq)
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Reads and reconciliation (read-only, fresh connection).
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reconciled {
    Absent,
    Committed {
        outcome: String,
        reason: Option<String>,
        result_id: Option<String>,
        seq: i64,
        request_digest: String,
        /// Visibility is not durability: this does not say the original caller saw the commit.
        durability: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredResult {
    pub result_id: String,
    pub domain: String,
    pub provenance: String,
    pub result_class: String,
    pub attestation: String,
    pub label: String,
    pub execution_id: String,
    pub grant_id: String,
    pub grant_revision: i64,
    pub org_revision: i64,
    pub candidate_digest: String,
    pub obligation_id: String,
    pub body: Vec<u8>,
    pub unestablished_json: String,
    pub seq: i64,
    pub receipt_id: String,
    pub receipt_bytes: Vec<u8>,
}

pub fn open_read_only(dir: &Path) -> rusqlite::Result<Connection> {
    let c = Connection::open_with_flags(
        dir.join(DB_FILE),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    c.busy_timeout(Duration::from_secs(30))?;
    Ok(c)
}

pub fn reconcile(dir: &Path, operation_id: &str) -> rusqlite::Result<Reconciled> {
    let c = open_read_only(dir)?;
    let r = c
        .query_row(
            "SELECT outcome, reason, result_id, seq, request_digest FROM operations WHERE operation_id=?1",
            [operation_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, i64>(3)?, r.get::<_, String>(4)?)),
        )
        .optional()?;
    Ok(match r {
        None => Reconciled::Absent,
        Some((outcome, reason, result_id, seq, request_digest)) => Reconciled::Committed {
            outcome,
            reason,
            result_id,
            seq,
            request_digest,
            durability: "visible in the store; durability and delivery to the original caller are not established",
        },
    })
}

pub fn read_result(dir: &Path, result_id: &str) -> rusqlite::Result<Option<StoredResult>> {
    let c = open_read_only(dir)?;
    c.query_row(
        "SELECT r.result_id, r.domain, r.provenance, r.result_class, r.attestation, r.label, r.execution_id, r.grant_id, r.grant_revision, r.org_revision, r.candidate_digest, r.obligation_id, r.body, r.unestablished_json, r.seq, c.receipt_id, c.bytes FROM results r JOIN receipts c ON c.result_id = r.result_id WHERE r.result_id=?1",
        [result_id],
        |r| {
            Ok(StoredResult {
                result_id: r.get(0)?, domain: r.get(1)?, provenance: r.get(2)?, result_class: r.get(3)?,
                attestation: r.get(4)?, label: r.get(5)?, execution_id: r.get(6)?, grant_id: r.get(7)?,
                grant_revision: r.get(8)?, org_revision: r.get(9)?, candidate_digest: r.get(10)?,
                obligation_id: r.get(11)?, body: r.get(12)?, unestablished_json: r.get(13)?, seq: r.get(14)?,
                receipt_id: r.get(15)?, receipt_bytes: r.get(16)?,
            })
        },
    )
    .optional()
}

pub fn integrity_check(dir: &Path) -> rusqlite::Result<String> {
    let c = open_read_only(dir)?;
    c.query_row("PRAGMA integrity_check", [], |r| r.get(0))
}

/// Readers of the authoritative state go through the store; an exported projection is never one.
pub fn write_projection(
    dir: &Path,
    result: &StoredResult,
    store_id: &str,
) -> std::io::Result<PathBuf> {
    let p = dir.join(format!("{}.projection.json", result.result_id));
    let v = serde_json::json!({
        "projection_of": "authoritative store row", "store_id": store_id, "seq": result.seq,
        "result_id": result.result_id, "receipt_id": result.receipt_id,
        "result_class": result.result_class, "label": result.label,
        "note": "a projection is never authority; consult the store",
    });
    std::fs::write(&p, serde_json::to_vec_pretty(&v)?)?;
    Ok(p)
}
