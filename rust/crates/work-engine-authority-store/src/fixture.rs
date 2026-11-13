//! Fixture-only boundary (exists only with the non-default `fixture` feature).
//!
//! Everything here is synthetic: the acceptance reference, identity assignments and grants are
//! test inputs labelled `provenance = 'fixture-only'`. The types below have private fields, no
//! `From`/`Into` to or from anything ordinary, and no constructor reachable from ordinary code.
//! The store serializes them; it decides nothing semantic and claims no independence of the
//! simulated reviewer or executor. Nothing here is a `Gate O`/`Gate R` output.
//!
//! A fixture admission cannot be built by struct literal (private fields):
//!
//! ```compile_fail,E0451
//! use work_engine_authority_store::fixture::FixtureAdmission;
//! let _ = FixtureAdmission {
//!     admission_id: String::new(),
//!     org_revision: 1,
//!     requirements_digest: String::new(),
//!     candidate_digest: String::new(),
//! };
//! ```
//!
//! A fixture admission converts into nothing ordinary (there is no `From`/`Into`):
//!
//! ```compile_fail,E0277
//! use work_engine_authority_store::{fixture::FixtureAdmission, store::PublishRequest};
//! fn convert(a: FixtureAdmission) -> PublishRequest { a.into() }
//! ```
//!
//! The ordinary admission API has no success value, so a fixture admission cannot be passed off
//! as its result:
//!
//! ```compile_fail,E0308
//! use work_engine_authority_store::{fixture::FixtureAdmission, store::Store};
//! fn pass_off(s: &Store, c: &work_engine_organization::OrganizationCandidate, f: FixtureAdmission) {
//!     let r: Result<FixtureAdmission, _> = s.admit(c);
//!     let _ = (r, f);
//! }
//! ```
use crate::adapter::{self, AdapterError, AdapterInputs};
use crate::ident::{digest_fields, sha256_hex};
use crate::receipt::{ADAPTER_VERSION, binding_digest};
use crate::report::{report_json, unestablished_json};
use crate::schema::FIXTURE_SCOPE;
use crate::store::{Domain, Store, Write, log_row};
use rusqlite::params;
use work_engine_manual_host::pi_docker::Job;
use work_engine_organization::{OrganizationCandidate, OrganizationView, Profile, enforceability};

/// A synthetic owner-acceptance reference. Not an acceptance.
#[derive(Debug, Clone)]
pub struct FixtureAcceptance {
    reference: String,
}
impl FixtureAcceptance {
    pub fn synthetic(reference: &str) -> Self {
        Self {
            reference: format!("fixture-only:{reference}"),
        }
    }
    pub fn reference(&self) -> &str {
        &self.reference
    }
}

#[derive(Debug, Clone)]
pub struct FixtureAdmission {
    admission_id: String,
    org_revision: i64,
    requirements_digest: String,
    candidate_digest: String,
}
impl FixtureAdmission {
    pub fn admission_id(&self) -> &str {
        &self.admission_id
    }
    pub fn org_revision(&self) -> i64 {
        self.org_revision
    }
    pub fn requirements_digest(&self) -> &str {
        &self.requirements_digest
    }
    pub fn candidate_digest(&self) -> &str {
        &self.candidate_digest
    }
}

#[derive(Debug, Clone)]
pub struct FixtureGrant {
    grant_id: String,
    admission_id: String,
}
impl FixtureGrant {
    pub fn grant_id(&self) -> &str {
        &self.grant_id
    }
    pub fn admission_id(&self) -> &str {
        &self.admission_id
    }
}

/// Fixture-only provisional scheduling: a Job for one obligation, bound to a fixture admission.
/// It confers no ordinary authority and is not a dispatch authorization.
#[derive(Debug, Clone)]
pub struct FixtureSchedule {
    admission_id: String,
    obligation_id: String,
    execution_id: String,
    job: Job,
}
impl FixtureSchedule {
    pub fn job(&self) -> &Job {
        &self.job
    }
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }
    pub fn obligation_id(&self) -> &str {
        &self.obligation_id
    }
    pub fn admission_id(&self) -> &str {
        &self.admission_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionBinding {
    pub image: String,
    pub runtime_manifest_sha256: String,
}

#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    #[error("not committed or unknown: {0}")]
    Write(String),
    #[error("adapter: {0}")]
    Adapter(#[from] AdapterError),
    #[error(
        "an execution already exists for this obligation under this admission; no re-execution"
    )]
    ExecutionExists,
}

fn wr<T>(w: Write<T>) -> Result<T, FixtureError> {
    match w {
        Write::Done(v) => Ok(v),
        Write::NotCommitted(m) => Err(FixtureError::Write(format!("NotCommitted: {m}"))),
        Write::Unknown(m) => Err(FixtureError::Write(format!("OutcomeUnknown: {m}"))),
    }
}

impl Store {
    /// Install the fixture organization state (revision 1) from a candidate the real compiler derived.
    pub fn fixture_install(
        &mut self,
        acceptance: &FixtureAcceptance,
        candidate: &OrganizationCandidate,
    ) -> Result<(), FixtureError> {
        let (acc, req, cand) = (
            acceptance.reference.clone(),
            candidate.requirements_digest().to_hex(),
            candidate.content_digest().to_hex(),
        );
        wr(self.write_txn(move |tx| {
            let seq = log_row(tx, "fixture-install", &cand)?;
            let _ = seq;
            tx.execute(
                "INSERT INTO org_state(domain, org_revision, requirements_digest, acceptance_ref, candidate_digest, provenance) VALUES('fixture', 1, ?1, ?2, ?3, 'fixture-only')",
                params![req, acc, cand],
            )?;
            Ok(())
        }))
    }

    /// Requirements supersession: a new requirements digest and candidate at revision + 1.
    pub fn fixture_supersede_requirements(
        &mut self,
        acceptance: &FixtureAcceptance,
        candidate: &OrganizationCandidate,
    ) -> Result<i64, FixtureError> {
        let (acc, req, cand) = (
            acceptance.reference.clone(),
            candidate.requirements_digest().to_hex(),
            candidate.content_digest().to_hex(),
        );
        wr(self.write_txn(move |tx| {
            let seq = log_row(tx, "fixture-supersede", &req)?;
            tx.execute(
                "UPDATE org_state SET org_revision = org_revision + 1, requirements_digest=?1, acceptance_ref=?2, candidate_digest=?3 WHERE domain='fixture'",
                params![req, acc, cand],
            )?;
            Ok(seq)
        }))
    }

    /// Fixture admission. The enforceability report is computed here from the candidate and
    /// recorded verbatim, together with every unestablished requirement. A candidate whose report
    /// has `None` rows is admitted here *as a fixture only*; ordinary [`Store::admit`] refuses it.
    pub fn fixture_admit(
        &mut self,
        candidate: &OrganizationCandidate,
        binding: &AdmissionBinding,
    ) -> Result<FixtureAdmission, FixtureError> {
        let report = enforceability(candidate, Profile::FixedSliceReviewV1);
        let report_text = report_json(&report).to_string();
        let unest = unestablished_json(candidate, &report).to_string();
        let (req, cand) = (
            candidate.requirements_digest().to_hex(),
            candidate.content_digest().to_hex(),
        );
        let bind = binding_digest(&binding.image, &binding.runtime_manifest_sha256);
        wr(self.write_txn(move |tx| {
            let (rev,): (i64,) = tx.query_row("SELECT org_revision FROM org_state WHERE domain='fixture'", [], |r| Ok((r.get(0)?,)))?;
            let id = digest_fields("p1f.fixture-admission.v1", &[req.as_bytes(), cand.as_bytes(), &rev.to_be_bytes(), bind.as_bytes()]);
            let seq = log_row(tx, "fixture-admit", &id)?;
            tx.execute(
                "INSERT INTO admissions(admission_id, domain, provenance, org_revision, requirements_digest, candidate_digest, binding_digest, report_json, unestablished_json, state, seq) VALUES(?1, 'fixture', 'fixture-only', ?2, ?3, ?4, ?5, ?6, ?7, 'admitted', ?8)",
                params![id, rev, req, cand, bind, report_text, unest, seq],
            )?;
            Ok(FixtureAdmission { admission_id: id, org_revision: rev, requirements_digest: req, candidate_digest: cand })
        }))
    }

    pub fn fixture_grant(
        &mut self,
        admission: &FixtureAdmission,
    ) -> Result<FixtureGrant, FixtureError> {
        let a = admission.admission_id.clone();
        wr(self.write_txn(move |tx| {
            let id = digest_fields("p1f.fixture-grant.v1", &[a.as_bytes(), FIXTURE_SCOPE.as_bytes()]);
            log_row(tx, "fixture-grant", &id)?;
            tx.execute(
                "INSERT INTO grants(grant_id, admission_id, domain, provenance, scope, state, revision, revoked_seq) VALUES(?1, ?2, 'fixture', 'fixture-only', ?3, 'active', 1, NULL)",
                params![id, a, FIXTURE_SCOPE],
            )?;
            Ok(FixtureGrant { grant_id: id, admission_id: a })
        }))
    }

    /// Fixture-only provisional scheduling. Needs a fixture admission and issues no ordinary one.
    pub fn fixture_schedule(
        &self,
        admission: &FixtureAdmission,
        view: &OrganizationView,
        candidate: &OrganizationCandidate,
        obligation: &str,
        inputs: &AdapterInputs,
    ) -> Result<FixtureSchedule, FixtureError> {
        let job = adapter::obligation_job(view, &candidate.content_digest(), obligation, inputs)?;
        let execution_id = adapter::input_digest(&job)?;
        Ok(FixtureSchedule {
            admission_id: admission.admission_id.clone(),
            obligation_id: obligation.to_owned(),
            execution_id,
            job,
        })
    }

    /// Record `prepared` once. A second call for the same obligation fails: no re-execution.
    pub fn fixture_prepare_execution(&mut self, s: &FixtureSchedule) -> Result<(), FixtureError> {
        let (e, a, o, j) = (
            s.execution_id.clone(),
            s.admission_id.clone(),
            s.obligation_id.clone(),
            s.job.job_id.clone(),
        );
        let existed = wr(self.write_txn(move |tx| {
            let n: i64 = tx.query_row("SELECT COUNT(*) FROM executions WHERE execution_id=?1 OR (admission_id=?2 AND obligation_id=?3)", params![e, a, o], |r| r.get(0))?;
            if n > 0 {
                return Ok(true);
            }
            log_row(tx, "execution-prepared", &e)?;
            tx.execute(
                "INSERT INTO executions(execution_id, admission_id, domain, obligation_id, job_id, state, host_receipt_digest) VALUES(?1, ?2, 'fixture', ?3, ?4, 'prepared', NULL)",
                params![e, a, o, j],
            )?;
            Ok(false)
        }))?;
        if existed {
            Err(FixtureError::ExecutionExists)
        } else {
            Ok(())
        }
    }

    pub fn fixture_publish(
        &mut self,
        req: &crate::store::PublishRequest,
    ) -> Write<crate::store::PublishOutcome> {
        self.publish_in_domain(Domain::Fixture, req)
    }
}

/// Host outcome as seen by the driver. `Unknown` never triggers re-execution.
pub enum HostRun {
    Receipt(Vec<u8>),
    Unknown(String),
}

/// Prepare once, run the Host once, record the result. There is no retry path.
pub fn run_once(
    store: &mut Store,
    s: &FixtureSchedule,
    runner: impl FnOnce(&Job) -> HostRun,
) -> Result<Option<(String, Vec<u8>)>, FixtureError> {
    store.fixture_prepare_execution(s)?;
    match runner(&s.job) {
        HostRun::Receipt(bytes) => {
            let id = sha256_hex(&bytes);
            wr(store.mark_execution_finished(&s.execution_id, &id))?;
            Ok(Some((id, bytes)))
        }
        HostRun::Unknown(_) => {
            wr(store.mark_execution_unknown(&s.execution_id))?;
            Ok(None)
        }
    }
}

pub fn binding_for(image: &str, runtime_manifest_sha256: &str) -> AdmissionBinding {
    let _ = ADAPTER_VERSION;
    AdmissionBinding {
        image: image.to_owned(),
        runtime_manifest_sha256: runtime_manifest_sha256.to_owned(),
    }
}
