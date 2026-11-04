//! Current authenticated owner-answer composition for an owned native request.
//! The missing qualified mode/controller capability remains a hard Held result.
use crate::{
    canonical_source::{CanonicalSource, Pin},
    inspect, service, source,
};
use rusqlite::{Connection, OpenFlags};
use time::OffsetDateTime;
type Result<T> = std::result::Result<T, &'static str>;

pub struct NativeRequest<'a> {
    pub objective: &'a str,
    pub actor: &'a str,
    pub task: &'a str,
    pub proposal: &'a str,
    pub mode: &'a str,
    pub profile: &'a str,
    pub parent: &'a str,
    pub parent_sha256: &'a str,
    pub maximum: &'a str,
    pub reserve: &'a str,
    pub attempt: &'a str,
    pub request_sha256: &'a str,
    pub session: &'a str,
    pub turn: &'a str,
    pub model: &'a str,
    pub effort: &'a str,
    pub tier: &'a str,
}
/// Startup configuration selects sources, never grants user or mode authority.
/// The only human proof is recollected by inspect::collect_current from the
/// existing signed answer/key/revocation/registry boundary on every call.
pub struct ProtectedRecollector {
    config: Pin,
    request: String,
    qualified: Option<crate::admission::QualifiedContext>,
}
impl ProtectedRecollector {
    pub fn new(config: Pin, admission_request: String) -> Result<Self> {
        let value = Self {
            config,
            request: admission_request,
            qualified: None,
        };
        value.config()?;
        serde_json::from_str::<crate::request::AdmissionRequest>(&value.request)
            .map_err(|_| "invalid_request")?;
        Ok(value)
    }
    fn config(&self) -> Result<service::Config> {
        if !source::pinned(&self.config) {
            return Err("authority_config_pin");
        }
        let raw = service::protected(&self.config.path).ok_or("authority_config_unprotected")?;
        let config: service::Config =
            serde_json::from_slice(&raw).map_err(|_| "authority_config_invalid")?;
        if config.schema != "dot.admission-config/v2-owned-source"
            || !std::path::Path::new(&config.database).is_absolute()
            || !source::pinned(&self.config)
        {
            return Err("authority_config_invalid");
        }
        Ok(config)
    }
    pub fn budget_source(&self) -> Result<CanonicalSource> {
        let c = self.config()?;
        let contract = c
            .sources
            .canonical_budget
            .as_ref()
            .ok_or("budget_source_contract_missing")?;
        if contract.contract != "canonical-envelope-source/v1" {
            return Err("budget_source_contract_mismatch");
        }
        CanonicalSource::new(
            contract.scope.clone(),
            c.sources.ledger.clone(),
            c.sources.budget_record.clone(),
        )
    }
    pub fn recollect(
        &mut self,
        b: &NativeRequest<'_>,
        budget: &CanonicalSource,
        now: OffsetDateTime,
    ) -> Result<()> {
        let clock = source::AcquisitionTime::new(now);
        let c = self.config()?;
        // Reuse the registry owner's protected read boundary; no coordinator
        // token or registry mutation can manufacture signed user approval.
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::symlink_metadata(&c.database).map_err(|_| "registry_unavailable")?;
        if !meta.is_file()
            || meta.file_type().is_symlink()
            || meta.mode() & 0o077 != 0
            || meta.uid() != unsafe { libc::geteuid() }
        {
            return Err("registry_unprotected");
        }
        let db = Connection::open_with_flags(&c.database, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| "registry_unavailable")?;
        crate::store::initialize(&db).map_err(|_| "registry_unavailable")?;
        let evidence =
            inspect::collect_current(&c.sources, &db, &self.request, clock.now(), budget)
                .map_err(|e| e.0)?;
        let row: crate::Row =
            serde_json::from_str(&evidence.original).map_err(|_| "unreviewable")?;
        let i = row.data.intent;
        if b.actor != c.sources.actor
            || b.actor != i.submitter
            || b.objective != i.budget.objective_id
            || b.task != evidence.task.sha256
            || b.proposal != i.proposal.sha256
            || b.mode != i.authority.mode.sha256
            || b.profile != i.workspace.configuration.sha256
            || b.model != i.model.id
            || b.effort != i.model.effort
            || b.tier != i.model.tier
            || !crate::budget_within(&i.budget.maximum, b.maximum, "0")
            || !crate::budget_within(&i.budget.closure_reserve, b.reserve, "0")
            || uuid::Uuid::parse_str(b.session).is_err()
            || uuid::Uuid::parse_str(b.turn).is_err()
            || [b.attempt, b.request_sha256].iter().any(|s| {
                s.len() != 64
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            })
        {
            return Err("signed_native_binding_mismatch");
        }
        // The exact parent must be present in the authenticated task packet.
        // Storage possession and objective-wide capacity are not authority.
        if !i
            .prerequisites
            .iter()
            .any(|r| r.id == b.parent && r.sha256 == b.parent_sha256)
        {
            return Err("signed_parent_binding_missing");
        }
        budget.check_current()?;
        self.config()?;
        let q = self.qualified.as_ref().ok_or("qualification_unavailable")?;
        crate::admission::gate(&c.sources, &evidence, q, clock.now())
            .ok_or("qualification_unavailable")?;
        Ok(())
    }
}
