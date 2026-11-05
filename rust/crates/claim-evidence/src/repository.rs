//! Repository facts: separate trusted admission, shared canonical mechanics.
//!
//! Bootstrap hashes bind bytes, not authority. The embedding host must authenticate
//! the launcher and custody of these startup inputs. V1 grants are immutable;
//! changing authority requires quiescing/replacing the deployment.
use crate::authority::{BootstrapFiles, OpenFiles, grant_digest};
use crate::codec::{JsValue, canonical_json, digest, exact_fields, field};
use crate::contract::{ClaimError, ClaimResult, ensure};
use crate::findings::{array, text_field};
use crate::store::{Store, records};
use crate::{
    ClaimId, FindingMutation, FindingRevisionId, OperationId, OperationLocator, Publication,
    ReliancePublication, WriteError,
};
use rusqlite::{OptionalExtension, params};

pub const REPOSITORY_FACT_PROFILE: &str = "repository-fact-v1";

/// Trusted scope, selected out of band. No publication capability is needed.
#[derive(Clone, Debug)]
pub struct RepositoryReadStartup {
    pub files: OpenFiles,
    pub repository_id: String,
    pub decision_scope: String,
}

pub struct RepositoryReader {
    store: Store,
    startup: RepositoryReadStartup,
}
impl RepositoryReader {
    pub fn open(startup: RepositoryReadStartup) -> ClaimResult<Self> {
        ensure(
            !startup.repository_id.is_empty() && !startup.decision_scope.is_empty(),
            "read scope absent",
        )?;
        let store = Store::open_repository(startup.files.clone(), true)?;
        ensure(
            store.bootstrap.repository_id.as_deref() == Some(&startup.repository_id),
            "read repository mismatch",
        )?;
        Ok(Self { store, startup })
    }
    pub fn resolve(
        &self,
        claim_id: &str,
        revision_id: &str,
        revision_sha256: &str,
    ) -> ClaimResult<JsValue> {
        self.snapshot(|| self.resolve_inside(claim_id, revision_id, revision_sha256))
    }
    pub fn discover(
        &self,
        query: &str,
        limit: usize,
        cursor: Option<&JsValue>,
    ) -> ClaimResult<JsValue> {
        self.snapshot(|| self.discover_inside(query, limit, cursor))
    }
    fn snapshot<T>(&self, read: impl FnOnce() -> ClaimResult<T>) -> ClaimResult<T> {
        crate::store::repository_reader_preflight(
            &self
                .store
                .identity
                .absolute_path
                .join(crate::store::DATABASE),
        )?;
        self.store
            .conn
            .execute_batch("BEGIN")
            .map_err(|e| ClaimError::new(e.to_string()))?;
        let result = read();
        self.store
            .conn
            .execute_batch("ROLLBACK")
            .map_err(|e| ClaimError::new(e.to_string()))?;
        result
    }
    /// Exact immutable address; expected digest prevents a reference substitution.
    fn resolve_inside(
        &self,
        claim_id: &str,
        revision_id: &str,
        revision_sha256: &str,
    ) -> ClaimResult<JsValue> {
        let (number, state) = self.store.read_state()?;
        let revision = records(&state, "revisions")?
            .iter()
            .find(|r| text_field(r, "id").ok().as_deref() == Some(revision_id))
            .ok_or_else(|| ClaimError::new("exact revision absent"))?;
        ensure(
            text_field(revision, "claim_id")? == claim_id && digest(revision)? == revision_sha256,
            "exact revision reference mismatch",
        )?;
        ensure(
            text_field(revision, "decision_scope")? == self.startup.decision_scope,
            "read decision scope mismatch",
        )?;
        let input = JsValue::object([
            ("claim_id", JsValue::text(claim_id)),
            ("revision_id", JsValue::text(revision_id)),
            ("revision_sha256", JsValue::text(revision_sha256)),
        ]);
        let item = project_revision(&self.store, &state, revision)?;
        self.projection(&state, number, &input, vec![item], JsValue::Null)
    }
    /// All matching heads are explicit candidates, never a newest-head choice.
    /// Cursor is bound to canonical input; a concurrent change requires restarting.
    fn discover_inside(
        &self,
        query: &str,
        limit: usize,
        cursor: Option<&JsValue>,
    ) -> ClaimResult<JsValue> {
        ensure(
            (1..=100).contains(&limit) && query.len() <= 1024,
            "discovery capacity invalid",
        )?;
        let (number, state) = self.store.read_state()?;
        let state_sha = projection_input_digest(&self.store, &state)?;
        let input = JsValue::object([
            ("query", JsValue::text(query)),
            ("limit", JsValue::Number(limit as f64)),
            ("repository_id", JsValue::text(&self.startup.repository_id)),
            (
                "decision_scope",
                JsValue::text(&self.startup.decision_scope),
            ),
        ]);
        let query_sha = digest(&input)?;
        let offset = if let Some(cursor) = cursor {
            exact_fields(
                cursor,
                &["canonical_input_sha256", "query_sha256", "offset"],
                "discovery cursor",
            )?;
            ensure(
                text_field(cursor, "canonical_input_sha256")? == state_sha
                    && text_field(cursor, "query_sha256")? == query_sha,
                "stale/mismatched discovery cursor",
            )?;
            match field(cursor, "offset")? {
                JsValue::Number(n) if *n >= 0.0 && n.fract() == 0.0 && *n <= 100000.0 => {
                    *n as usize
                }
                _ => return Err(ClaimError::new("invalid cursor offset")),
            }
        } else {
            0
        };
        let mut matching = Vec::new();
        for r in records(&state, "revisions")? {
            if text_field(r, "decision_scope")? != self.startup.decision_scope {
                continue;
            }
            let id = text_field(r, "id")?;
            if records(&state, "revisions")?
                .iter()
                .any(|other| text_field(other, "predecessor_revision").ok().as_deref() == Some(&id))
            {
                continue;
            }
            let claim = claim_for(&state, r)?;
            if query.is_empty()
                || canonical_json(field(claim, "subject")?)?.contains(query)
                || canonical_json(field(r, "proposition")?)?.contains(query)
            {
                matching.push(r);
            }
        }
        matching.sort_by_key(|r| text_field(r, "id").unwrap_or_default());
        ensure(offset <= matching.len(), "cursor offset out of range")?;
        let selected = matching
            .iter()
            .skip(offset)
            .take(limit)
            .map(|r| project_revision(&self.store, &state, r))
            .collect::<ClaimResult<Vec<_>>>()?;
        let next = offset + selected.len();
        let cursor = if next < matching.len() {
            JsValue::object([
                ("canonical_input_sha256", JsValue::text(&state_sha)),
                ("query_sha256", JsValue::text(&query_sha)),
                ("offset", JsValue::Number(next as f64)),
            ])
        } else {
            JsValue::Null
        };
        let mut actual_input = input.as_object()?.clone();
        actual_input.insert("offset".into(), JsValue::Number(offset as f64));
        self.projection(
            &state,
            number,
            &JsValue::Object(actual_input),
            selected,
            cursor,
        )
    }
    fn projection(
        &self,
        state: &JsValue,
        number: i64,
        input: &JsValue,
        items: Vec<JsValue>,
        cursor: JsValue,
    ) -> ClaimResult<JsValue> {
        let result = JsValue::object([
            ("schema_version", JsValue::Number(1.0)),
            ("kind", JsValue::text("repository_fact_projection_v1")),
            ("repository_id", JsValue::text(&self.startup.repository_id)),
            (
                "decision_scope",
                JsValue::text(&self.startup.decision_scope),
            ),
            ("applicability", JsValue::text("not_assessed")),
            ("source_currency", JsValue::text("not_assessed")),
            (
                "projection",
                JsValue::object([
                    (
                        "build_version",
                        JsValue::text("claim-evidence-repository-v1"),
                    ),
                    (
                        "canonical_input_sha256",
                        JsValue::text(&projection_input_digest(&self.store, state)?),
                    ),
                    ("canonical_state_sha256", JsValue::text(&digest(state)?)),
                    ("root_id", JsValue::text(&self.store.identity.root_id)),
                    (
                        "bootstrap_config_sha256",
                        JsValue::text(&self.store.bootstrap.config_sha256),
                    ),
                    (
                        "bootstrap_source_sha256",
                        JsValue::text(&self.store.bootstrap.source_sha256),
                    ),
                    ("store_revision", JsValue::Number(number as f64)),
                    ("request_sha256", JsValue::text(&digest(input)?)),
                    (
                        "freshness",
                        JsValue::text("current_relative_to_canonical_input"),
                    ),
                    ("completeness", JsValue::text("available")),
                ]),
            ),
            ("items", JsValue::Array(items)),
            ("next_cursor", cursor),
            (
                "limitations",
                JsValue::Array(vec![JsValue::text(
                    "Exact historical evidence is attributed; truth, applicability and current repository source currency are not established by this projection.",
                )]),
            ),
        ]);
        canonical_json(&result)?;
        Ok(result)
    }
}
fn claim_for<'a>(state: &'a JsValue, r: &JsValue) -> ClaimResult<&'a JsValue> {
    let id = text_field(r, "claim_id")?;
    records(state, "claims")?
        .iter()
        .find(|c| text_field(c, "id").ok().as_deref() == Some(&id))
        .ok_or_else(|| ClaimError::new("claim absent"))
}
fn project_revision(store: &Store, state: &JsValue, r: &JsValue) -> ClaimResult<JsValue> {
    let claim = claim_for(state, r)?;
    let id = text_field(r, "id")?;
    let mut heads = Vec::new();
    for other in records(state, "revisions")? {
        if field(other, "claim_id")? == field(r, "claim_id")? {
            let oid = text_field(other, "id")?;
            if !records(state, "revisions")?
                .iter()
                .any(|v| text_field(v, "predecessor_revision").ok().as_deref() == Some(&oid))
            {
                heads.push(JsValue::text(&oid));
            }
        }
    }
    heads.sort_by_key(crate::codec::canonical_json_unchecked);
    let grant = records(state, "authorities")?
        .iter()
        .find(|g| field(g, "grant_id").ok() == field(r, "authority_ref").ok())
        .ok_or_else(|| ClaimError::new("authority absent"))?;
    let creation_grant = records(state, "authorities")?
        .iter()
        .find(|g| field(g, "grant_id").ok() == field(claim, "authority_ref").ok())
        .ok_or_else(|| ClaimError::new("creation authority absent"))?;
    let mut unresolved = Vec::new();
    for reference in std::iter::once(field(field(claim, "subject")?, "evidence_baseline")?)
        .chain(array(field(r, "evidence_references")?, "references")?)
        .chain(array(field(r, "sensitivity_references")?, "references")?)
    {
        if text_field(reference, "status")? != "verified" {
            unresolved.push(reference.clone());
        }
    }
    let reliances = records(state, "reliances")?
        .iter()
        .filter(|v| text_field(v, "claim_revision_id").ok().as_deref() == Some(&id))
        .map(|v| {
            let authority = records(state, "authorities")?
                .iter()
                .find(|g| field(g, "grant_id").ok() == field(v, "authority_ref").ok())
                .ok_or_else(|| ClaimError::new("reliance authority absent"))?;
            Ok(JsValue::object([
                (
                    "admitted_operation",
                    admitted_operation(store, state, &text_field(v, "id")?)?,
                ),
                ("record", v.clone()),
                ("sha256", JsValue::text(&digest(v)?)),
                ("attributed_producer", field(authority, "actor")?.clone()),
                ("authority", authority.clone()),
                (
                    "attestation",
                    JsValue::text("producer_attributed_not_direct_consumer_attestation"),
                ),
            ]))
        })
        .collect::<ClaimResult<Vec<_>>>()?;
    Ok(JsValue::object([
        ("admitted_operation", admitted_operation(store, state, &id)?),
        ("claim", claim.clone()),
        ("revision", r.clone()),
        ("revision_sha256", JsValue::text(&digest(r)?)),
        ("authority", grant.clone()),
        ("creation_authority", creation_grant.clone()),
        (
            "current_head",
            JsValue::Bool(heads.contains(&JsValue::text(&id))),
        ),
        ("head_revision_ids", JsValue::Array(heads)),
        ("unresolved_references", JsValue::Array(unresolved)),
        ("reliances", JsValue::Array(reliances)),
        ("applicability", JsValue::text("not_assessed")),
    ]))
}

#[derive(Clone, Debug)]
pub struct RepositoryProducerStartup {
    pub read: RepositoryReadStartup,
    pub actor: String,
    pub grant_id: String,
    pub allowed_actions: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryAdmissionBinding {
    pub root_id: String,
    pub repository_id: String,
    pub operation_id: String,
    pub producer: String,
    pub decision_scope: String,
    pub request_sha256: String,
    pub authority_ref_sha256: String,
}
impl RepositoryAdmissionBinding {
    pub fn to_js_value(&self) -> JsValue {
        JsValue::object([
            ("root_id", JsValue::text(&self.root_id)),
            ("repository_id", JsValue::text(&self.repository_id)),
            ("operation_id", JsValue::text(&self.operation_id)),
            ("producer", JsValue::text(&self.producer)),
            ("decision_scope", JsValue::text(&self.decision_scope)),
            ("request_sha256", JsValue::text(&self.request_sha256)),
            (
                "authority_ref_sha256",
                JsValue::text(&self.authority_ref_sha256),
            ),
        ])
    }
}
pub trait RepositoryLease {
    fn validate(&self) -> ClaimResult<()>;
}
pub trait RepositoryAdmissionPort {
    fn acquire<'a>(
        &'a self,
        binding: &RepositoryAdmissionBinding,
    ) -> ClaimResult<Box<dyn RepositoryLease + 'a>>;
}
/// The host retains its admission lease until commit AND exact readback finish.
pub struct RepositoryApplication<P: RepositoryAdmissionPort> {
    store: Store,
    startup: RepositoryProducerStartup,
    port: P,
}
impl<P: RepositoryAdmissionPort> RepositoryApplication<P> {
    pub fn initialize(startup: RepositoryProducerStartup, port: P) -> ClaimResult<Self> {
        let f = &startup.read.files;
        validate_producer_startup(&startup)?;
        let store = Store::initialize_repository(BootstrapFiles {
            root: f.root.clone(),
            config: f.config.clone(),
            source: f.source.clone(),
            expected_config_sha256: f.expected_config_sha256.clone(),
            expected_source_sha256: f.expected_source_sha256.clone(),
        })?;
        let app = Self {
            store,
            startup,
            port,
        };
        app.selected_grant()?;
        Ok(app)
    }
    pub fn open(startup: RepositoryProducerStartup, port: P) -> ClaimResult<Self> {
        validate_producer_startup(&startup)?;
        let store = Store::open_repository(startup.read.files.clone(), false)?;
        let app = Self {
            store,
            startup,
            port,
        };
        app.selected_grant()?;
        Ok(app)
    }
    fn selected_grant(&self) -> ClaimResult<JsValue> {
        self.store.check_root()?;
        ensure(
            self.store.bootstrap.repository_id.as_deref() == Some(&self.startup.read.repository_id),
            "producer repository mismatch",
        )?;
        let grant = self
            .store
            .bootstrap
            .grants
            .iter()
            .find(|g| text_field(g, "grant_id").ok().as_deref() == Some(&self.startup.grant_id))
            .ok_or_else(|| ClaimError::new("selected grant absent"))?;
        ensure(
            text_field(grant, "profile")? == REPOSITORY_FACT_PROFILE
                && text_field(grant, "actor")? == self.startup.actor
                && text_field(grant, "decision_scope")? == self.startup.read.decision_scope,
            "selected authority mismatch",
        )?;
        ensure(
            !self.startup.allowed_actions.is_empty()
                && self.startup.allowed_actions.iter().all(|v| {
                    ["create_claim", "publish_revision", "record_reliance"].contains(&v.as_str())
                }),
            "invalid selected actions",
        )?;
        Ok(grant.clone())
    }
    fn prepare(
        &self,
        request: &JsValue,
    ) -> ClaimResult<(OperationLocator, RepositoryAdmissionBinding)> {
        // Reserve transport depth for state/admission/projection envelopes before
        // entering a write, so an accepted fact remains serializable on read.
        let mut framed = request.clone();
        for _ in 0..8 {
            framed = JsValue::Array(vec![framed]);
        }
        canonical_json(&framed)?;
        ensure(
            canonical_json(request)?.len() <= 1024 * 1024,
            "repository request exceeds capacity",
        )?;
        exact_fields(
            request,
            &[
                "schema_version",
                "operation_id",
                "action",
                "profile",
                "expected_state",
                "payload",
            ],
            "repository operation",
        )?;
        ensure(
            field(request, "schema_version")? == &JsValue::Number(1.0)
                && text_field(request, "profile")? == REPOSITORY_FACT_PROFILE,
            "repository operation version/profile mismatch",
        )?;
        let grant = self.selected_grant()?;
        let action = text_field(request, "action")?;
        ensure(
            self.startup.allowed_actions.contains(&action)
                && array(field(&grant, "permissions")?, "permissions")?
                    .contains(&JsValue::text(&action)),
            "action not selected",
        )?;
        let action = match action.as_str() {
            "create_claim" => "create_claim",
            "publish_revision" => "publish_revision",
            "record_reliance" => "record_reliance",
            _ => return Err(ClaimError::new("unsupported action")),
        };
        let binding = RepositoryAdmissionBinding {
            root_id: self.store.identity.root_id.clone(),
            repository_id: self.startup.read.repository_id.clone(),
            operation_id: text_field(request, "operation_id")?,
            producer: self.startup.actor.clone(),
            decision_scope: self.startup.read.decision_scope.clone(),
            request_sha256: digest(request)?,
            authority_ref_sha256: grant_digest(&grant),
        };
        let locator = OperationLocator {
            root: self.store.identity.clone(),
            operation_id: OperationId::new(&binding.operation_id)?,
            action,
            payload_sha256: digest(&JsValue::object([
                ("operation", request.clone()),
                ("admitted_grant", grant),
            ]))?,
            grant_sha256: binding.authority_ref_sha256.clone(),
            admission_sha256: digest(&binding.to_js_value())?,
        };
        Ok((locator, binding))
    }
    pub fn locator(&self, request: &JsValue) -> ClaimResult<OperationLocator> {
        Ok(self.prepare(request)?.0)
    }
    /// Closed owner envelope; authority and root cannot be selected by this input.
    pub fn execute(&mut self, request: &JsValue) -> Result<JsValue, WriteError> {
        let (locator, binding) = self.prepare(request).map_err(WriteError::NoEffect)?;
        let _lease = self.port.acquire(&binding).map_err(WriteError::NoEffect)?;
        self.selected_grant().map_err(WriteError::NoEffect)?;
        _lease.validate().map_err(WriteError::NoEffect)?;
        let locator_value = locator_value(&locator);
        let locator_json = canonical_json(&locator_value).map_err(WriteError::NoEffect)?;
        // A persisted entered/unknown operation is reconciled before another write.
        let prior: Option<(String, String)> = self
            .store
            .conn
            .query_row(
                "SELECT locator_json,status FROM repository_attempts WHERE operation_id=?",
                [locator.operation_id.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| WriteError::NoEffect(ClaimError::new(e.to_string())))?;
        if let Some((old, status)) = &prior {
            if old != &locator_json {
                self.store.conn.execute("INSERT INTO repository_attempt_events(operation_id,locator_json,status,detail) VALUES(?,?,'conflicting','operation locator conflict')",params![locator.operation_id.as_str(),locator_json]).map_err(|_|WriteError::OutcomeUnknown(Box::new(locator.clone())))?;
                return Err(WriteError::NoEffect(ClaimError::new(
                    "operation locator conflict",
                )));
            }
            if status == "entered" || status == "unknown" {
                return self
                    .reconcile_inside(&locator)
                    .and_then(|found| {
                        _lease.validate()?;
                        self.persist_attempt(&locator, "committed", "")?;
                        Ok(found)
                    })
                    .map_err(|_| WriteError::OutcomeUnknown(Box::new(locator)));
            }
        }
        self.persist_attempt(&locator, "entered", "")
            .map_err(WriteError::NoEffect)?;
        if let Err(e) = self.store.conn.execute_batch("BEGIN IMMEDIATE") {
            if self
                .persist_attempt(&locator, "no_effect", &e.to_string())
                .is_err()
            {
                return Err(WriteError::OutcomeUnknown(Box::new(locator)));
            }
            return Err(WriteError::NoEffect(ClaimError::new(e.to_string())));
        }
        let result = self.execute_inside(request, &locator, &binding);
        match result {
            Ok((replayed, result)) => {
                crate::fault_cut("repository_before_commit");
                if let Err(e) = self.selected_grant().and_then(|_| _lease.validate()) {
                    if self.store.conn.execute_batch("ROLLBACK").is_ok() {
                        if self
                            .persist_attempt(&locator, "no_effect", &e.message)
                            .is_err()
                        {
                            return Err(WriteError::OutcomeUnknown(Box::new(locator)));
                        }
                        return Err(WriteError::NoEffect(e));
                    }
                    self.finish_attempt(&locator, "unknown", "startup changed before commit");
                    return Err(WriteError::OutcomeUnknown(Box::new(locator)));
                }
                if self.store.conn.execute_batch("COMMIT").is_err() {
                    self.finish_attempt(&locator, "unknown", "commit failed");
                    return Err(WriteError::OutcomeUnknown(Box::new(locator)));
                }
                crate::fault_cut("repository_after_commit");
                let readback = _lease
                    .validate()
                    .and_then(|_| self.reconcile_inside(&locator));
                match readback {
                    Ok(found) if field(&found, "result").ok() == Some(&result) => {
                        if self
                            .persist_attempt(
                                &locator,
                                if replayed { "replayed" } else { "committed" },
                                "",
                            )
                            .is_err()
                        {
                            return Err(WriteError::OutcomeUnknown(Box::new(locator)));
                        }
                        Ok(JsValue::object([
                            (
                                "status",
                                JsValue::text(if replayed { "replayed" } else { "applied" }),
                            ),
                            ("locator", locator_value),
                            ("result", result),
                        ]))
                    }
                    _ => {
                        self.finish_attempt(&locator, "unknown", "readback failed");
                        Err(WriteError::OutcomeUnknown(Box::new(locator)))
                    }
                }
            }
            Err(e) => {
                if self.store.conn.execute_batch("ROLLBACK").is_err() {
                    self.finish_attempt(&locator, "unknown", &e.message);
                    Err(WriteError::OutcomeUnknown(Box::new(locator)))
                } else {
                    if self
                        .persist_attempt(&locator, "no_effect", &e.message)
                        .is_err()
                    {
                        return Err(WriteError::OutcomeUnknown(Box::new(locator)));
                    }
                    Err(WriteError::NoEffect(e))
                }
            }
        }
    }
    fn finish_attempt(&self, locator: &OperationLocator, status: &str, detail: &str) {
        // The durable entered locator remains available if final reporting fails.
        let _ = self.persist_attempt(locator, status, detail);
    }
    fn persist_attempt(
        &self,
        locator: &OperationLocator,
        status: &str,
        detail: &str,
    ) -> ClaimResult<()> {
        ensure(
            self.store.conn.is_autocommit(),
            "attempt transaction still unresolved",
        )?;
        let json = canonical_json(&locator_value(locator))?;
        self.store
            .conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| ClaimError::new(e.to_string()))?;
        let result = (|| {
            let changed=self.store.conn.execute("INSERT INTO repository_attempts VALUES(?,?,?,?) ON CONFLICT(operation_id) DO UPDATE SET status=excluded.status,detail=excluded.detail WHERE repository_attempts.locator_json=excluded.locator_json",params![locator.operation_id.as_str(),json,status,detail]).map_err(|e|ClaimError::new(e.to_string()))?;
            ensure(changed == 1, "operation locator conflict")?;
            self.store.conn.execute("INSERT INTO repository_attempt_events(operation_id,locator_json,status,detail) VALUES(?,?,?,?)",params![locator.operation_id.as_str(),json,status,detail]).map_err(|e|ClaimError::new(e.to_string()))?;
            Ok(())
        })();
        if result.is_ok() {
            self.store
                .conn
                .execute_batch("COMMIT")
                .map_err(|e| ClaimError::new(e.to_string()))?;
        } else {
            self.store
                .conn
                .execute_batch("ROLLBACK")
                .map_err(|e| ClaimError::new(e.to_string()))?;
        }
        result
    }
    fn execute_inside(
        &self,
        request: &JsValue,
        locator: &OperationLocator,
        binding: &RepositoryAdmissionBinding,
    ) -> ClaimResult<(bool, JsValue)> {
        let grant = self.selected_grant()?;
        let payload = field(request, "payload")?;
        let mutation = match locator.action {
            "create_claim" => {
                exact_fields(
                    payload,
                    &["subject", "statement_identity", "initial_revision"],
                    "create payload",
                )?;
                ensure(
                    field(request, "expected_state")? == &JsValue::Null,
                    "create predecessor must be null",
                )?;
                FindingMutation::Create {
                    subject: field(payload, "subject")?.clone(),
                    statement_identity: field(payload, "statement_identity")?.as_text()?.clone(),
                    initial_revision: field(payload, "initial_revision")?.clone(),
                }
            }
            "publish_revision" => {
                exact_fields(payload, &["claim_id", "revision"], "revise payload")?;
                FindingMutation::Revise {
                    claim_id: ClaimId::new(text_field(payload, "claim_id")?)?,
                    predecessor: FindingRevisionId::new(text_field(request, "expected_state")?)?,
                    revision: field(payload, "revision")?.clone(),
                }
            }
            "record_reliance" => {
                exact_fields(
                    payload,
                    &[
                        "consumer",
                        "consumer_revision",
                        "decision_scope",
                        "claim_revision_id",
                        "claim_revision_sha256",
                        "state",
                        "predecessor_reliance",
                    ],
                    "reliance payload",
                )?;
                ensure(
                    field(request, "expected_state")? == &JsValue::Null
                        && field(payload, "state")? == &JsValue::text("active")
                        && field(payload, "predecessor_reliance")? == &JsValue::Null,
                    "unsupported reliance state",
                )?;
                let (_, state) = self.store.read_state()?;
                let revision_id =
                    FindingRevisionId::new(text_field(payload, "claim_revision_id")?)?;
                let rev = records(&state, "revisions")?
                    .iter()
                    .find(|r| text_field(r, "id").ok().as_deref() == Some(revision_id.as_str()))
                    .ok_or_else(|| ClaimError::new("reliance revision absent"))?;
                ensure(
                    digest(rev)? == text_field(payload, "claim_revision_sha256")?,
                    "reliance revision digest mismatch",
                )?;
                let claim_id = ClaimId::new(text_field(rev, "claim_id")?)?;
                let consumer = text_field(payload, "consumer")?;
                let consumer_revision = text_field(payload, "consumer_revision")?;
                let scope = text_field(payload, "decision_scope")?;
                let input = crate::application::RelianceInput {
                    operation_id: &locator.operation_id,
                    grant_id: &self.startup.grant_id,
                    claim_id: &claim_id,
                    revision_id: &revision_id,
                    consumer: &consumer,
                    consumer_revision: &consumer_revision,
                    decision_scope: &scope,
                };
                let result = self.store.record_reliance_inside(&input, &grant, locator)?;
                let replayed = matches!(result, ReliancePublication::Replayed(_));
                self.store_repository_admission(request, locator, binding)?;
                return Ok((replayed, self.result_for(locator)?));
            }
            _ => return Err(ClaimError::new("unsupported action")),
        };
        let (result, _) = self.store.publish_inside(
            &mutation,
            &locator.operation_id,
            &self.startup.grant_id,
            &grant,
            locator,
        )?;
        self.store_repository_admission(request, locator, binding)?;
        Ok((
            matches!(result, Publication::Replayed(_)),
            self.result_for(locator)?,
        ))
    }
    fn store_repository_admission(
        &self,
        request: &JsValue,
        locator: &OperationLocator,
        binding: &RepositoryAdmissionBinding,
    ) -> ClaimResult<()> {
        let binding_json = canonical_json(&binding.to_js_value())?;
        let request_json = canonical_json(request)?;
        self.store
            .conn
            .execute(
                "INSERT OR IGNORE INTO repository_admission_bindings VALUES(?,?,?)",
                params![locator.operation_id.as_str(), binding_json, request_json],
            )
            .map_err(|e| ClaimError::new(e.to_string()))?;
        let stored:(String,String)=self.store.conn.query_row("SELECT binding_json,request_json FROM repository_admission_bindings WHERE operation_id=?",[locator.operation_id.as_str()],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|ClaimError::new(e.to_string()))?;
        ensure(
            stored == (binding_json, request_json),
            "immutable repository admission mismatch",
        )
    }
    fn result_for(&self, locator: &OperationLocator) -> ClaimResult<JsValue> {
        let (_, state) = self.store.read_state()?;
        let op = records(&state, "operations")?
            .iter()
            .find(|r| {
                text_field(r, "operation_id").ok().as_deref() == Some(locator.operation_id.as_str())
            })
            .ok_or_else(|| ClaimError::new("operation absent"))?;
        ensure(
            text_field(op, "payload_sha256")? == locator.payload_sha256
                && text_field(op, "action")? == locator.action,
            "operation conflict",
        )?;
        let (admission,grant):(String,String)=self.store.conn.query_row("SELECT admission_sha256,grant_sha256 FROM operation_admissions WHERE operation_id=?",[locator.operation_id.as_str()],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|ClaimError::new(e.to_string()))?;
        ensure(
            admission == locator.admission_sha256 && grant == locator.grant_sha256,
            "operation admission conflict",
        )?;
        let id = text_field(op, "result_identity")?;
        let collection = if locator.action == "record_reliance" {
            "reliances"
        } else {
            "revisions"
        };
        let record = records(&state, collection)?
            .iter()
            .find(|r| text_field(r, "id").ok().as_deref() == Some(&id))
            .ok_or_else(|| ClaimError::new("operation result absent"))?;
        Ok(JsValue::object([
            ("record", record.clone()),
            ("sha256", JsValue::text(&digest(record)?)),
        ]))
    }
    fn reconcile_inside(&self, locator: &OperationLocator) -> ClaimResult<JsValue> {
        // Same coherent snapshot as normal read; never a blind publication retry.
        self.store
            .conn
            .execute_batch("BEGIN")
            .map_err(|e| ClaimError::new(e.to_string()))?;
        let result = self.result_for(locator).and_then(|result| {
            self.selected_grant()?;
            Ok(JsValue::object([
                ("status", JsValue::text("committed")),
                ("locator", locator_value(locator)),
                ("result", result),
            ]))
        });
        self.store
            .conn
            .execute_batch("ROLLBACK")
            .map_err(|e| ClaimError::new(e.to_string()))?;
        result
    }
    /// A request-bound inspection reports persisted entered/no-effect states and
    /// reconciles a committed unknown; it never re-enters the write automatically.
    pub fn inspect(&self, request: &JsValue) -> ClaimResult<JsValue> {
        let (locator, binding) = self.prepare(request)?;
        let _lease = self.port.acquire(&binding)?;
        let row: Option<(String, String, String)> = self
            .store
            .conn
            .query_row(
                "SELECT locator_json,status,detail FROM repository_attempts WHERE operation_id=?",
                [locator.operation_id.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| ClaimError::new(e.to_string()))?;
        if let Some((stored, status, detail)) = row {
            if stored != canonical_json(&locator_value(&locator))? {
                return Ok(JsValue::object([
                    ("status", JsValue::text("conflicting")),
                    ("locator", locator_value(&locator)),
                ]));
            }
            match self.reconcile_inside(&locator) {
                Ok(found) => {
                    _lease.validate()?;
                    self.persist_attempt(&locator, "committed", "")?;
                    return Ok(found);
                }
                Err(e)
                    if e.message == "operation absent"
                        && (status == "entered" || status == "unknown") =>
                {
                    _lease.validate()?;
                    self.persist_attempt(
                        &locator,
                        "no_effect",
                        "reconciled canonical absence while holding producer admission",
                    )?;
                    return Ok(JsValue::object([
                        ("status", JsValue::text("no_effect")),
                        ("detail", JsValue::text("reconciled canonical absence")),
                        ("locator", locator_value(&locator)),
                    ]));
                }
                Err(_) => {}
            }
            return Ok(JsValue::object([
                ("status", JsValue::text(&status)),
                ("detail", JsValue::text(&detail)),
                ("locator", locator_value(&locator)),
            ]));
        }
        Ok(JsValue::object([
            ("status", JsValue::text("absent")),
            ("locator", locator_value(&locator)),
        ]))
    }
}
pub fn repository_locator_value(locator: &OperationLocator) -> JsValue {
    locator_value(locator)
}
fn locator_value(l: &OperationLocator) -> JsValue {
    JsValue::object([
        ("root_id", JsValue::text(&l.root.root_id)),
        ("profile", JsValue::text(&l.root.profile)),
        ("operation_id", JsValue::text(l.operation_id.as_str())),
        ("action", JsValue::text(l.action)),
        ("payload_sha256", JsValue::text(&l.payload_sha256)),
        ("grant_sha256", JsValue::text(&l.grant_sha256)),
        ("admission_sha256", JsValue::text(&l.admission_sha256)),
    ])
}

fn validate_producer_startup(startup: &RepositoryProducerStartup) -> ClaimResult<()> {
    let b = crate::authority::load(&startup.read.files)?;
    ensure(
        b.repository_id.as_deref() == Some(&startup.read.repository_id),
        "producer repository mismatch",
    )?;
    let grant = b
        .grants
        .iter()
        .find(|g| text_field(g, "grant_id").ok().as_deref() == Some(&startup.grant_id))
        .ok_or_else(|| ClaimError::new("selected grant absent"))?;
    ensure(
        text_field(grant, "profile")? == REPOSITORY_FACT_PROFILE
            && text_field(grant, "actor")? == startup.actor
            && text_field(grant, "decision_scope")? == startup.read.decision_scope,
        "selected authority mismatch",
    )?;
    ensure(
        !startup.allowed_actions.is_empty()
            && startup.allowed_actions.iter().all(|a| {
                ["create_claim", "publish_revision", "record_reliance"].contains(&a.as_str())
                    && array(
                        field(grant, "permissions").unwrap_or(&JsValue::Null),
                        "permissions",
                    )
                    .is_ok_and(|p| p.contains(&JsValue::text(a)))
            }),
        "invalid selected actions",
    )
}

fn admitted_operation(store: &Store, state: &JsValue, result_id: &str) -> ClaimResult<JsValue> {
    let operation = records(state, "operations")?
        .iter()
        .find(|op| text_field(op, "result_identity").ok().as_deref() == Some(result_id))
        .ok_or_else(|| ClaimError::new("result admission absent"))?;
    let id = text_field(operation, "operation_id")?;
    let (binding_json,request_json):(String,String)=store.conn.query_row("SELECT binding_json,request_json FROM repository_admission_bindings WHERE operation_id=?",[id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|ClaimError::new(e.to_string()))?;
    Ok(JsValue::object([
        ("operation", operation.clone()),
        ("binding", crate::codec::parse_json(&binding_json)?),
        ("request", crate::codec::parse_json(&request_json)?),
    ]))
}

fn projection_input_digest(store: &Store, state: &JsValue) -> ClaimResult<String> {
    let mut statement=store.conn.prepare("SELECT operation_id,binding_json,request_json FROM repository_admission_bindings ORDER BY operation_id").map_err(|e|ClaimError::new(e.to_string()))?;
    let rows = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| ClaimError::new(e.to_string()))?;
    let mut admissions = Vec::new();
    for row in rows {
        let (id, binding, request) = row.map_err(|e| ClaimError::new(e.to_string()))?;
        admissions.push(JsValue::object([
            ("operation_id", JsValue::text(&id)),
            ("binding", crate::codec::parse_json(&binding)?),
            ("request", crate::codec::parse_json(&request)?),
        ]));
    }
    digest(&JsValue::object([
        ("root_id", JsValue::text(&store.identity.root_id)),
        ("profile", JsValue::text(&store.identity.profile)),
        (
            "bootstrap_config_sha256",
            JsValue::text(&store.bootstrap.config_sha256),
        ),
        (
            "bootstrap_source_sha256",
            JsValue::text(&store.bootstrap.source_sha256),
        ),
        ("canonical_state", state.clone()),
        ("admissions", JsValue::Array(admissions)),
    ]))
}
