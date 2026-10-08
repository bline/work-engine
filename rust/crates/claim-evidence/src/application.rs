use crate::authority::{BootstrapFiles, FINDING_PROFILE, OpenFiles, grant_digest};
use crate::codec::{JsValue, digest_unchecked as digest, field};
use crate::contract::{
    ClaimError, ClaimResult, FindingCommand, FindingMutation, FindingReceipt, OperationLocator,
    Publication, Reconciliation, WriteError, ensure,
};
use crate::findings::{
    ClaimId, FindingRevisionId, make_revision, stable_claim_id, text_field,
    validate_revision_payload,
};
use crate::projection::{ClaimProjection, ProjectionRequest, projection_request_sha256};
use crate::reliance::{
    ExactFindingRevisionRef, ExactRelianceRef, RelianceCommand, RelianceId, ReliancePublication,
    RelianceReceipt, RelianceReconciliation,
};
use crate::store::{Store, records, records_mut, state_json, validate_state};
use rusqlite::{OptionalExtension, params};
const MAX_REQUEST_BYTES: usize = 1024 * 1024;

/// Exact campaign-owned request context. The campaign owner checks these
/// values and holds the returned lease through claims commit and readback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmissionBinding {
    pub campaign_root_id: String,
    pub campaign_revision: String,
    pub campaign_operation_id: String,
    pub obligation_id: String,
    pub candidate_digest: String,
    pub selection_digest: String,
    pub profile_digest: String,
    pub prepared_request_sha256: String,
    pub episode_revision: String,
    pub result_sha256: String,
    pub claims_request_sha256: String,
}
impl AdmissionBinding {
    pub(crate) fn validate(&self) -> ClaimResult<()> {
        for (label, value) in [
            ("campaign root", &self.campaign_root_id),
            ("campaign revision", &self.campaign_revision),
            ("campaign operation", &self.campaign_operation_id),
            ("obligation", &self.obligation_id),
            ("episode revision", &self.episode_revision),
        ] {
            ensure(!value.is_empty(), &format!("{label} absent"))?;
        }
        for (label, value) in [
            ("candidate", &self.candidate_digest),
            ("selection", &self.selection_digest),
            ("profile", &self.profile_digest),
            ("prepared request", &self.prepared_request_sha256),
            ("result", &self.result_sha256),
            ("claims request", &self.claims_request_sha256),
        ] {
            ensure(
                value.len() == 64
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
                &format!("{label} digest invalid"),
            )?;
        }
        Ok(())
    }
    pub fn digest(&self) -> String {
        digest(&JsValue::object([
            ("campaign_root_id", JsValue::text(&self.campaign_root_id)),
            ("campaign_revision", JsValue::text(&self.campaign_revision)),
            (
                "campaign_operation_id",
                JsValue::text(&self.campaign_operation_id),
            ),
            ("obligation_id", JsValue::text(&self.obligation_id)),
            ("candidate_digest", JsValue::text(&self.candidate_digest)),
            ("selection_digest", JsValue::text(&self.selection_digest)),
            ("profile_digest", JsValue::text(&self.profile_digest)),
            (
                "prepared_request_sha256",
                JsValue::text(&self.prepared_request_sha256),
            ),
            ("episode_revision", JsValue::text(&self.episode_revision)),
            ("result_sha256", JsValue::text(&self.result_sha256)),
            (
                "claims_request_sha256",
                JsValue::text(&self.claims_request_sha256),
            ),
        ]))
    }
}

pub trait FindingLease {}
pub trait FindingAdmissionPort {
    fn acquire<'a>(&'a self, binding: &AdmissionBinding)
    -> ClaimResult<Box<dyn FindingLease + 'a>>;
}

pub struct ClaimsApplication<P: FindingAdmissionPort> {
    pub(crate) store: Store,
    pub(crate) port: P,
}
impl<P: FindingAdmissionPort> ClaimsApplication<P> {
    pub fn initialize(files: BootstrapFiles, port: P) -> ClaimResult<Self> {
        Ok(Self {
            store: Store::initialize(files)?,
            port,
        })
    }
    pub fn open(files: OpenFiles, port: P) -> ClaimResult<Self> {
        Ok(Self {
            store: Store::open(files)?,
            port,
        })
    }
    pub fn root(&self) -> &crate::contract::RootIdentity {
        &self.store.identity
    }

    /// Builds a readback locator before entering the effect boundary.
    pub fn locator(&self, command: &FindingCommand) -> ClaimResult<OperationLocator> {
        self.store.check_root()?;
        command.admission.validate()?;
        let operation = operation(command)?;
        ensure(
            crate::codec::canonical_json_unchecked(&operation).len() <= MAX_REQUEST_BYTES,
            "finding request exceeds capacity",
        )?;
        ensure(
            digest(&operation) == command.admission.claims_request_sha256,
            "claims request admission mismatch",
        )?;
        let grant = self.registered_grant(&command.grant_id)?;
        let action = match &command.mutation {
            FindingMutation::Create { .. } => "create_claim",
            FindingMutation::Revise { .. } => "publish_revision",
        };
        Ok(OperationLocator {
            root: self.store.identity.clone(),
            operation_id: command.operation_id.clone(),
            action,
            payload_sha256: digest(&JsValue::object([
                ("operation", operation),
                ("admitted_grant", grant.clone()),
            ])),
            grant_sha256: grant_digest(&grant),
            admission_sha256: command.admission.digest(),
        })
    }

    pub fn publish(&mut self, command: FindingCommand) -> Result<Publication, WriteError> {
        let operation = match operation(&command) {
            Ok(v) => v,
            Err(e) => return Err(WriteError::NoEffect(e)),
        };
        if let Err(e) = command.admission.validate() {
            return Err(WriteError::NoEffect(e));
        }
        if crate::codec::canonical_json_unchecked(&operation).len() > MAX_REQUEST_BYTES {
            return Err(WriteError::NoEffect(ClaimError::new(
                "finding request exceeds capacity",
            )));
        }
        let request_sha = digest(&operation);
        if request_sha != command.admission.claims_request_sha256 {
            return Err(WriteError::NoEffect(ClaimError::new(
                "claims request admission mismatch",
            )));
        }
        let lease = match self.port.acquire(&command.admission) {
            Ok(v) => v,
            Err(e) => return Err(WriteError::NoEffect(e)),
        };
        let _lease = lease;
        let grant = match self.registered_grant(&command.grant_id) {
            Ok(v) => v,
            Err(e) => return Err(WriteError::NoEffect(e)),
        };
        let action = match &command.mutation {
            FindingMutation::Create { .. } => "create_claim",
            FindingMutation::Revise { .. } => "publish_revision",
        };
        let grant_sha = grant_digest(&grant);
        let payload_sha = digest(&JsValue::object([
            ("operation", operation.clone()),
            ("admitted_grant", grant.clone()),
        ]));
        let locator = OperationLocator {
            root: self.store.identity.clone(),
            operation_id: command.operation_id.clone(),
            action,
            payload_sha256: payload_sha.clone(),
            grant_sha256: grant_sha.clone(),
            admission_sha256: command.admission.digest(),
        };
        if let Err(e) = self.store.check_root() {
            return Err(WriteError::NoEffect(e));
        }
        if let Err(e) = self.store.conn.execute_batch("BEGIN IMMEDIATE") {
            return Err(WriteError::NoEffect(ClaimError::new(format!(
                "claims begin transaction: {e}"
            ))));
        }
        let result = self.publish_inside(&command, &grant, &operation, &locator);
        match result {
            Ok((publication, _changed)) => {
                crate::fault_cut("before_commit");
                if let Err(error) = crate::store::harden_sidecars(
                    &self
                        .store
                        .identity
                        .absolute_path
                        .join("claim-evidence.sqlite3"),
                ) {
                    return if self.store.conn.execute_batch("ROLLBACK").is_ok() {
                        Err(WriteError::NoEffect(error))
                    } else {
                        Err(WriteError::OutcomeUnknown(Box::new(locator)))
                    };
                }
                if let Err(_e) = self.store.conn.execute_batch("COMMIT") {
                    return Err(WriteError::OutcomeUnknown(Box::new(locator)));
                }
                crate::fault_cut("after_commit");
                let expected = match &publication {
                    Publication::Applied(r) | Publication::Replayed(r) => r,
                };
                match self.reconcile(&locator) {
                    Reconciliation::Committed(found) if &*found == expected => Ok(publication),
                    _ => Err(WriteError::OutcomeUnknown(Box::new(locator))),
                }
            }
            Err(error) => {
                if self.store.conn.execute_batch("ROLLBACK").is_err() {
                    Err(WriteError::OutcomeUnknown(Box::new(locator)))
                } else {
                    Err(WriteError::NoEffect(error))
                }
            }
        }
    }

    fn registered_grant(&self, id: &str) -> ClaimResult<JsValue> {
        self.store
            .bootstrap
            .grants
            .iter()
            .find(|g| text_field(g, "grant_id").ok().as_deref() == Some(id))
            .cloned()
            .ok_or_else(|| ClaimError::new("grant is not registered"))
    }

    fn publish_inside(
        &self,
        command: &FindingCommand,
        grant: &JsValue,
        _operation: &JsValue,
        locator: &OperationLocator,
    ) -> ClaimResult<(Publication, bool)> {
        let (revision, mut state) = self.store.read_state()?;
        ensure(
            text_field(grant, "profile")? == FINDING_PROFILE,
            "grant profile mismatch",
        )?;
        ensure(
            crate::findings::array(field(grant, "permissions")?, "permissions")?
                .iter()
                .any(|v| v == &JsValue::text(locator.action)),
            "grant permission mismatch",
        )?;
        let prior = records(&state, "operations")?
            .iter()
            .find(|op| {
                text_field(op, "operation_id").ok().as_deref()
                    == Some(command.operation_id.as_str())
            })
            .cloned();
        if let Some(prior) = prior {
            ensure(
                text_field(&prior, "action")? == locator.action
                    && text_field(&prior, "payload_sha256")? == locator.payload_sha256
                    && text_field(&prior, "authority_ref")? == command.grant_id,
                "operation identity conflict",
            )?;
            let binding:(String,String)=self.store.conn.query_row("SELECT admission_sha256,grant_sha256 FROM operation_admissions WHERE operation_id=?",[command.operation_id.as_str()],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|ClaimError::new(format!("admission record absent: {e}")))?;
            ensure(
                binding
                    == (
                        locator.admission_sha256.clone(),
                        locator.grant_sha256.clone(),
                    ),
                "operation admission identity conflict",
            )?;
            let id = FindingRevisionId::new(text_field(&prior, "result_identity")?)?;
            let record = records(&state, "revisions")?
                .iter()
                .find(|r| text_field(r, "id").ok().as_deref() == Some(id.as_str()))
                .ok_or_else(|| ClaimError::new("receipt result absent"))?;
            let claim_id = ClaimId::new(text_field(record, "claim_id")?)?;
            return Ok((
                Publication::Replayed(FindingReceipt {
                    claim_id,
                    revision_id: id,
                    revision_sha256: digest(record),
                    locator: locator.clone(),
                }),
                false,
            ));
        }
        let scope = text_field(grant, "decision_scope")?;
        let actor = text_field(grant, "actor")?;
        let (claim_id, record) = match &command.mutation {
            FindingMutation::Create {
                subject,
                statement_identity,
                initial_revision,
            } => {
                ensure(!statement_identity.0.is_empty(), "statement identity empty")?;
                let claim_id = stable_claim_id(subject)?;
                ensure(
                    !records(&state, "claims")?
                        .iter()
                        .any(|c| text_field(c, "id").ok().as_deref() == Some(claim_id.as_str())),
                    "claim already exists",
                )?;
                validate_revision_payload(initial_revision, &scope)?;
                let claim = JsValue::object([
                    ("id", JsValue::text(claim_id.as_str())),
                    ("schema_version", JsValue::Number(1.0)),
                    ("profile", JsValue::text(FINDING_PROFILE)),
                    ("subject", subject.clone()),
                    (
                        "statement_identity",
                        JsValue::String(statement_identity.clone()),
                    ),
                    ("created_by", JsValue::text(&actor)),
                    ("authority_ref", JsValue::text(&command.grant_id)),
                ]);
                let record =
                    make_revision(&claim_id, None, initial_revision, &actor, &command.grant_id)?;
                records_mut(&mut state, "claims")?.push(claim);
                (claim_id, record)
            }
            FindingMutation::Revise {
                claim_id,
                predecessor,
                revision,
            } => {
                ensure(
                    records(&state, "claims")?
                        .iter()
                        .any(|c| text_field(c, "id").ok().as_deref() == Some(claim_id.as_str())),
                    "claim absent",
                )?;
                let predecessor_record = records(&state, "revisions")?
                    .iter()
                    .find(|r| text_field(r, "id").ok().as_deref() == Some(predecessor.as_str()));
                ensure(
                    predecessor_record.is_some_and(|r| {
                        text_field(r, "claim_id").ok().as_deref() == Some(claim_id.as_str())
                    }),
                    "predecessor absent",
                )?;
                ensure(
                    !records(&state, "revisions")?.iter().any(|r| {
                        text_field(r, "predecessor_revision").ok().as_deref()
                            == Some(predecessor.as_str())
                    }),
                    "conflicting predecessor",
                )?;
                validate_revision_payload(revision, &scope)?;
                (
                    claim_id.clone(),
                    make_revision(
                        claim_id,
                        Some(predecessor),
                        revision,
                        &actor,
                        &command.grant_id,
                    )?,
                )
            }
        };
        let id = FindingRevisionId::new(text_field(&record, "id")?)?;
        ensure(
            !records(&state, "revisions")?
                .iter()
                .any(|r| text_field(r, "id").ok().as_deref() == Some(id.as_str())),
            "revision identity reused",
        )?;
        let sha = digest(&record);
        records_mut(&mut state, "revisions")?.push(record);
        records_mut(&mut state, "operations")?.push(JsValue::object([
            ("operation_id", JsValue::text(command.operation_id.as_str())),
            ("action", JsValue::text(locator.action)),
            ("payload_sha256", JsValue::text(&locator.payload_sha256)),
            ("result_identity", JsValue::text(id.as_str())),
            ("authority_ref", JsValue::text(&command.grant_id)),
        ]));
        validate_state(&state, &self.store.bootstrap.grants)?;
        let json = state_json(&state)?;
        self.store
            .conn
            .execute(
                "INSERT INTO operation_admissions VALUES(?,?,?)",
                params![
                    command.operation_id.as_str(),
                    locator.admission_sha256,
                    locator.grant_sha256
                ],
            )
            .map_err(|e| ClaimError::new(format!("admission insert: {e}")))?;
        let changed=self.store.conn.execute("UPDATE canonical_claim_state SET store_revision=?,store_sha256=?,store_json=? WHERE singleton=1 AND store_revision=?",params![revision+1,digest(&state),json,revision]).map_err(|e|ClaimError::new(format!("state CAS: {e}")))?;
        ensure(changed == 1, "state revision changed")?;
        Ok((
            Publication::Applied(FindingReceipt {
                claim_id,
                revision_id: id,
                revision_sha256: sha,
                locator: locator.clone(),
            }),
            true,
        ))
    }

    pub fn reconcile(&self, locator: &OperationLocator) -> Reconciliation {
        if locator.root != self.store.identity {
            return Reconciliation::Conflicting;
        }
        let ((), state) = match self.store.read_state() {
            Ok((_, v)) => ((), v),
            Err(e) => return Reconciliation::Unresolved(e.message),
        };
        let prior = match records(&state, "operations") {
            Ok(v) => v
                .iter()
                .find(|op| {
                    text_field(op, "operation_id").ok().as_deref()
                        == Some(locator.operation_id.as_str())
                })
                .cloned(),
            Err(e) => return Reconciliation::Unresolved(e.message),
        };
        let Some(prior) = prior else {
            return Reconciliation::Absent;
        };
        let binding:Option<(String,String)>=match self.store.conn.query_row("SELECT admission_sha256,grant_sha256 FROM operation_admissions WHERE operation_id=?",[locator.operation_id.as_str()],|r|Ok((r.get(0)?,r.get(1)?))).optional(){Ok(v)=>v,Err(e)=>return Reconciliation::Unresolved(e.to_string())};
        let Some(binding) = binding else {
            return Reconciliation::Unresolved("operation admission absent".into());
        };
        if binding
            != (
                locator.admission_sha256.clone(),
                locator.grant_sha256.clone(),
            )
            || text_field(&prior, "action").ok().as_deref() != Some(locator.action)
            || text_field(&prior, "payload_sha256").ok().as_deref() != Some(&locator.payload_sha256)
        {
            return Reconciliation::Conflicting;
        }
        let result_id = match text_field(&prior, "result_identity") {
            Ok(v) => v,
            Err(e) => return Reconciliation::Unresolved(e.message),
        };
        let record = match records(&state, "revisions") {
            Ok(v) => v
                .iter()
                .find(|r| text_field(r, "id").ok().as_deref() == Some(&result_id))
                .cloned(),
            Err(e) => return Reconciliation::Unresolved(e.message),
        };
        let Some(record) = record else {
            return Reconciliation::Unresolved("receipt result absent".into());
        };
        let claim_id = match text_field(&record, "claim_id").and_then(ClaimId::new) {
            Ok(v) => v,
            Err(e) => return Reconciliation::Unresolved(e.message),
        };
        Reconciliation::Committed(Box::new(FindingReceipt {
            claim_id,
            revision_id: FindingRevisionId::new(result_id).expect("validated result id"),
            revision_sha256: digest(&record),
            locator: locator.clone(),
        }))
    }
}

fn operation(command: &FindingCommand) -> ClaimResult<JsValue> {
    // Check direct Rust values before cloning them into the historical envelope.
    match &command.mutation {
        FindingMutation::Create {
            subject,
            initial_revision,
            ..
        } => {
            crate::codec::validate_transport(subject)?;
            crate::codec::validate_transport(initial_revision)?;
        }
        FindingMutation::Revise { revision, .. } => {
            crate::codec::validate_transport(revision)?;
        }
    }
    let (action, expected, payload) = match &command.mutation {
        FindingMutation::Create {
            subject,
            statement_identity,
            initial_revision,
        } => (
            "create_claim",
            JsValue::Null,
            JsValue::object([
                ("subject", subject.clone()),
                (
                    "statement_identity",
                    JsValue::String(statement_identity.clone()),
                ),
                ("initial_revision", initial_revision.clone()),
            ]),
        ),
        FindingMutation::Revise {
            claim_id,
            predecessor,
            revision,
        } => (
            "publish_revision",
            JsValue::text(predecessor.as_str()),
            JsValue::object([
                ("claim_id", JsValue::text(claim_id.as_str())),
                ("revision", revision.clone()),
            ]),
        ),
    };
    let operation = JsValue::object([
        ("schema_version", JsValue::Number(1.0)),
        ("operation_id", JsValue::text(command.operation_id.as_str())),
        ("action", JsValue::text(action)),
        ("profile", JsValue::text(FINDING_PROFILE)),
        ("expected_state", expected),
        ("payload", payload),
    ]);
    crate::codec::canonical_json(&operation)?;
    Ok(operation)
}

/// Produces the exact claim-codec request hash for a campaign admission.
pub fn claims_request_sha256(command: &FindingCommand) -> ClaimResult<String> {
    crate::codec::digest(&operation(command)?)
}

pub const FINDING_REQUEST_MAX_BYTES: usize = MAX_REQUEST_BYTES;
pub fn claims_request_canonical(command: &FindingCommand) -> ClaimResult<String> {
    crate::codec::canonical_json(&operation(command)?)
}

fn reliance_operation(command: &RelianceCommand) -> ClaimResult<JsValue> {
    for (name, value) in [
        ("consumer", &command.consumer),
        ("consumer revision", &command.consumer_revision),
        ("decision scope", &command.decision_scope),
    ] {
        ensure(!value.is_empty(), &format!("{name} absent"))?;
    }
    let operation = JsValue::object([
        ("schema_version", JsValue::Number(1.0)),
        ("operation_id", JsValue::text(command.operation_id.as_str())),
        ("action", JsValue::text("record_reliance")),
        ("profile", JsValue::text(FINDING_PROFILE)),
        ("expected_state", JsValue::Null),
        (
            "payload",
            JsValue::object([
                ("consumer", JsValue::text(&command.consumer)),
                (
                    "consumer_revision",
                    JsValue::text(&command.consumer_revision),
                ),
                ("decision_scope", JsValue::text(&command.decision_scope)),
                (
                    "claim_revision_id",
                    JsValue::text(command.revision_id.as_str()),
                ),
                ("state", JsValue::text("active")),
                ("predecessor_reliance", JsValue::Null),
            ]),
        ),
    ]);
    crate::codec::canonical_json(&operation)?;
    Ok(operation)
}

/// Exact historical operation bytes; the separately checked claim ID is a
/// domain precondition, while the revision ID is the stored operation target.
pub fn reliance_request_canonical(command: &RelianceCommand) -> ClaimResult<String> {
    crate::codec::canonical_json(&reliance_operation(command)?)
}
pub fn reliance_request_sha256(command: &RelianceCommand) -> ClaimResult<String> {
    crate::codec::digest(&reliance_operation(command)?)
}

impl<P: FindingAdmissionPort> ClaimsApplication<P> {
    pub fn project_exact_revisions(
        &self,
        request: &ProjectionRequest,
    ) -> ClaimResult<ClaimProjection> {
        self.store.check_root()?;
        request.admission.validate()?;
        ensure(
            projection_request_sha256(request)? == request.admission.claims_request_sha256,
            "projection request admission mismatch",
        )?;
        let _lease = self.port.acquire(&request.admission)?;
        crate::projection::build(&self.store, request)
    }

    pub fn verify_projection_fresh(&self, projection: &ClaimProjection) -> ClaimResult<()> {
        crate::projection::verify_fresh(&self.store, projection)
    }

    pub fn reliance_locator(&self, command: &RelianceCommand) -> ClaimResult<OperationLocator> {
        self.store.check_root()?;
        command.admission.validate()?;
        let operation = reliance_operation(command)?;
        ensure(
            crate::codec::canonical_json_unchecked(&operation).len() <= MAX_REQUEST_BYTES,
            "reliance request exceeds capacity",
        )?;
        ensure(
            digest(&operation) == command.admission.claims_request_sha256,
            "claims request admission mismatch",
        )?;
        let grant = self.registered_grant(&command.grant_id)?;
        Ok(OperationLocator {
            root: self.store.identity.clone(),
            operation_id: command.operation_id.clone(),
            action: "record_reliance",
            payload_sha256: digest(&JsValue::object([
                ("operation", operation),
                ("admitted_grant", grant.clone()),
            ])),
            grant_sha256: grant_digest(&grant),
            admission_sha256: command.admission.digest(),
        })
    }

    pub fn record_reliance(
        &mut self,
        command: RelianceCommand,
    ) -> Result<ReliancePublication, WriteError> {
        let locator = self
            .reliance_locator(&command)
            .map_err(WriteError::NoEffect)?;
        let lease = self
            .port
            .acquire(&command.admission)
            .map_err(WriteError::NoEffect)?;
        let _lease = lease;
        let grant = self
            .registered_grant(&command.grant_id)
            .map_err(WriteError::NoEffect)?;
        self.store.check_root().map_err(WriteError::NoEffect)?;
        self.store
            .conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| {
                WriteError::NoEffect(ClaimError::new(format!("claims begin transaction: {e}")))
            })?;
        let result = self.record_reliance_inside(&command, &grant, &locator);
        match result {
            Ok(publication) => {
                crate::fault_cut("before_reliance_commit");
                if let Err(error) = crate::store::harden_sidecars(
                    &self
                        .store
                        .identity
                        .absolute_path
                        .join("claim-evidence.sqlite3"),
                ) {
                    return if self.store.conn.execute_batch("ROLLBACK").is_ok() {
                        Err(WriteError::NoEffect(error))
                    } else {
                        Err(WriteError::OutcomeUnknown(Box::new(locator)))
                    };
                }
                if self.store.conn.execute_batch("COMMIT").is_err() {
                    return Err(WriteError::OutcomeUnknown(Box::new(locator)));
                }
                crate::fault_cut("after_reliance_commit");
                let expected = match &publication {
                    ReliancePublication::Applied(r) | ReliancePublication::Replayed(r) => r,
                };
                match self.reconcile_reliance(&locator) {
                    RelianceReconciliation::Committed(found) if &*found == expected => {
                        Ok(publication)
                    }
                    _ => Err(WriteError::OutcomeUnknown(Box::new(locator))),
                }
            }
            Err(error) => {
                if self.store.conn.execute_batch("ROLLBACK").is_ok() {
                    Err(WriteError::NoEffect(error))
                } else {
                    Err(WriteError::OutcomeUnknown(Box::new(locator)))
                }
            }
        }
    }

    fn record_reliance_inside(
        &self,
        command: &RelianceCommand,
        grant: &JsValue,
        locator: &OperationLocator,
    ) -> ClaimResult<ReliancePublication> {
        let (state_revision, mut state) = self.store.read_state()?;
        ensure(
            text_field(grant, "profile")? == FINDING_PROFILE
                && text_field(grant, "decision_scope")? == command.decision_scope
                && crate::findings::array(field(grant, "permissions")?, "permissions")?
                    .contains(&JsValue::text("record_reliance")),
            "reliance grant profile, scope or permission mismatch",
        )?;
        let claim = records(&state, "claims")?
            .iter()
            .find(|c| text_field(c, "id").ok().as_deref() == Some(command.claim_id.as_str()))
            .ok_or_else(|| ClaimError::new("reliance claim absent"))?;
        ensure(
            text_field(claim, "profile")? == FINDING_PROFILE,
            "reliance claim profile mismatch",
        )?;
        let revision = records(&state, "revisions")?
            .iter()
            .find(|r| text_field(r, "id").ok().as_deref() == Some(command.revision_id.as_str()))
            .ok_or_else(|| ClaimError::new("reliance revision absent"))?;
        ensure(
            text_field(revision, "claim_id")? == command.claim_id.as_str()
                && text_field(revision, "decision_scope")? == command.decision_scope,
            "reliance revision claim or scope mismatch",
        )?;
        if let Some(prior) = records(&state, "operations")?.iter().find(|op| {
            text_field(op, "operation_id").ok().as_deref() == Some(command.operation_id.as_str())
        }) {
            ensure(
                text_field(prior, "action")? == "record_reliance"
                    && text_field(prior, "payload_sha256")? == locator.payload_sha256
                    && text_field(prior, "authority_ref")? == command.grant_id,
                "operation identity conflict",
            )?;
            let binding:(String,String)=self.store.conn.query_row("SELECT admission_sha256,grant_sha256 FROM operation_admissions WHERE operation_id=?",[command.operation_id.as_str()],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|ClaimError::new(format!("admission record absent: {e}")))?;
            ensure(
                binding
                    == (
                        locator.admission_sha256.clone(),
                        locator.grant_sha256.clone(),
                    ),
                "operation admission identity conflict",
            )?;
            return Ok(ReliancePublication::Replayed(reliance_receipt(
                &state, prior, locator,
            )?));
        }
        ensure(
            !records(&state, "revisions")?.iter().any(|r| {
                text_field(r, "predecessor_revision").ok().as_deref()
                    == Some(command.revision_id.as_str())
            }),
            "reliance revision is not current",
        )?;
        let without_id = JsValue::object([
            ("schema_version", JsValue::Number(1.0)),
            ("consumer", JsValue::text(&command.consumer)),
            (
                "consumer_revision",
                JsValue::text(&command.consumer_revision),
            ),
            ("decision_scope", JsValue::text(&command.decision_scope)),
            (
                "claim_revision_id",
                JsValue::text(command.revision_id.as_str()),
            ),
            ("state", JsValue::text("active")),
            ("predecessor_reliance", JsValue::Null),
            ("authority_ref", JsValue::text(&command.grant_id)),
            ("operation_id", JsValue::text(command.operation_id.as_str())),
        ]);
        let id = RelianceId::new(format!("reliance@{}", digest(&without_id)))?;
        ensure(
            !records(&state, "reliances")?
                .iter()
                .any(|r| text_field(r, "id").ok().as_deref() == Some(id.as_str())),
            "reliance identity reused",
        )?;
        let mut fields = without_id.as_object()?.clone();
        fields.insert("id".into(), JsValue::text(id.as_str()));
        records_mut(&mut state, "reliances")?.push(JsValue::Object(fields));
        records_mut(&mut state, "operations")?.push(JsValue::object([
            ("operation_id", JsValue::text(command.operation_id.as_str())),
            ("action", JsValue::text("record_reliance")),
            ("payload_sha256", JsValue::text(&locator.payload_sha256)),
            ("result_identity", JsValue::text(id.as_str())),
            ("authority_ref", JsValue::text(&command.grant_id)),
        ]));
        validate_state(&state, &self.store.bootstrap.grants)?;
        let json = state_json(&state)?;
        self.store
            .conn
            .execute(
                "INSERT INTO operation_admissions VALUES(?,?,?)",
                params![
                    command.operation_id.as_str(),
                    locator.admission_sha256,
                    locator.grant_sha256
                ],
            )
            .map_err(|e| ClaimError::new(format!("admission insert: {e}")))?;
        let changed=self.store.conn.execute("UPDATE canonical_claim_state SET store_revision=?,store_sha256=?,store_json=? WHERE singleton=1 AND store_revision=?",params![state_revision+1,digest(&state),json,state_revision]).map_err(|e|ClaimError::new(format!("state CAS: {e}")))?;
        ensure(changed == 1, "state revision changed")?;
        let prior = records(&state, "operations")?
            .last()
            .expect("just appended");
        Ok(ReliancePublication::Applied(reliance_receipt(
            &state, prior, locator,
        )?))
    }

    pub fn reconcile_reliance(&self, locator: &OperationLocator) -> RelianceReconciliation {
        if locator.root != self.store.identity || locator.action != "record_reliance" {
            return RelianceReconciliation::Conflicting;
        }
        let (_, state) = match self.store.read_state() {
            Ok(v) => v,
            Err(e) => return RelianceReconciliation::Unresolved(e.message),
        };
        let prior = match records(&state, "operations") {
            Ok(v) => v.iter().find(|op| {
                text_field(op, "operation_id").ok().as_deref()
                    == Some(locator.operation_id.as_str())
            }),
            Err(e) => return RelianceReconciliation::Unresolved(e.message),
        };
        let Some(prior) = prior else {
            return RelianceReconciliation::Absent;
        };
        let binding:Option<(String,String)>=match self.store.conn.query_row("SELECT admission_sha256,grant_sha256 FROM operation_admissions WHERE operation_id=?",[locator.operation_id.as_str()],|r|Ok((r.get(0)?,r.get(1)?))).optional(){Ok(v)=>v,Err(e)=>return RelianceReconciliation::Unresolved(e.to_string())};
        let Some(binding) = binding else {
            return RelianceReconciliation::Unresolved("operation admission absent".into());
        };
        if binding
            != (
                locator.admission_sha256.clone(),
                locator.grant_sha256.clone(),
            )
            || text_field(prior, "action").ok().as_deref() != Some("record_reliance")
            || text_field(prior, "payload_sha256").ok().as_deref() != Some(&locator.payload_sha256)
        {
            return RelianceReconciliation::Conflicting;
        }
        match reliance_receipt(&state, prior, locator) {
            Ok(v) => RelianceReconciliation::Committed(Box::new(v)),
            Err(e) => RelianceReconciliation::Unresolved(e.message),
        }
    }
}

fn reliance_receipt(
    state: &JsValue,
    operation: &JsValue,
    locator: &OperationLocator,
) -> ClaimResult<RelianceReceipt> {
    let id = text_field(operation, "result_identity")?;
    let reliance = records(state, "reliances")?
        .iter()
        .find(|r| text_field(r, "id").ok().as_deref() == Some(&id))
        .ok_or_else(|| ClaimError::new("reliance receipt result absent"))?;
    let revision_id = FindingRevisionId::new(text_field(reliance, "claim_revision_id")?)?;
    let revision = records(state, "revisions")?
        .iter()
        .find(|r| text_field(r, "id").ok().as_deref() == Some(revision_id.as_str()))
        .ok_or_else(|| ClaimError::new("reliance target absent"))?;
    Ok(RelianceReceipt {
        exact: ExactRelianceRef {
            id: RelianceId::new(id)?,
            sha256: digest(reliance),
            finding: ExactFindingRevisionRef {
                claim_id: ClaimId::new(text_field(revision, "claim_id")?)?,
                revision_id,
                revision_sha256: digest(revision),
            },
            consumer: text_field(reliance, "consumer")?,
            consumer_revision: text_field(reliance, "consumer_revision")?,
            decision_scope: text_field(reliance, "decision_scope")?,
        },
        locator: locator.clone(),
    })
}
