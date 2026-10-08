use claim_evidence::codec::{JsValue, digest, parse_json};
use claim_evidence::{
    AdmissionBinding, AdmissionReadRequest, BootstrapFiles, ClaimError, ClaimResult,
    ClaimsApplication, CustodyEvidence, EstablishmentCommand, FindingAdmissionPort, FindingLease,
    ObservationCommand, OpenFiles, ProductionPathAccess, ProductionPathAdmissionBinding,
    ProductionPathAdmissionPort, ProductionPathLease, ProductionPathPublication,
    ProductionPathReconciliation, ProductionPathRequest, ProductionPathStage,
    ProductionPathWriteError, production_path_request_sha256, sha256_bytes,
};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn parsed(value: Value) -> JsValue {
    parse_json(&value.to_string()).unwrap()
}
fn mutate(value: &mut JsValue, key: &str, entry: JsValue) {
    if let JsValue::Object(fields) = value {
        fields.insert(key.into(), entry);
    } else {
        panic!("not object")
    }
}
fn subject() -> Value {
    json!({"candidate":{"commit":"c1","tree":"t1","patchIdentity":"p1"},"reviewEpisodeId":"episode-1"})
}
fn claim(boundary: &str) -> JsValue {
    let consumer = if boundary == "builder_projection" {
        "slice-builder:campaign-1"
    } else {
        "slice-campaign:campaign-1"
    };
    let mut value = parsed(
        json!({"schemaVersion":1,"claimId":"","revision":"","proposition":"execution path was observed","subject":subject(),"coveredState":"review-result","consumptionBoundary":boundary,"consumer":consumer,"acceptance":{"owner":"operator","source":"campaign","unestablishedRoute":"hold"},"profile":{"id":"production-path-v1","revision":"production-path-profile-v1","allowedMechanisms":["transport-1"],"admissibleObservers":["observer-1"],"integrityRequired":true,"requiredRealization":"model-1","requiredCapabilities":["read"],"continuity":"fresh_initial"}}),
    );
    let id=digest(&parsed(json!({"proposition":"execution path was observed","subject":subject(),"coveredState":"review-result","consumptionBoundary":boundary,"consumer":consumer}))).unwrap();
    mutate(
        &mut value,
        "claimId",
        JsValue::text(&format!("production-path-claim-v1@{id}")),
    );
    let rev = digest(&without(&value, "revision")).unwrap();
    mutate(
        &mut value,
        "revision",
        JsValue::text(&format!("production-path-claim-revision-v1@{rev}")),
    );
    value
}
fn without(value: &JsValue, key: &str) -> JsValue {
    let mut v = value.clone();
    if let JsValue::Object(fields) = &mut v {
        fields.remove(&key.into());
    }
    v
}
fn observation() -> JsValue {
    let mut value = parsed(
        json!({"schema_version":2,"id":"","event_identity":"event-1","kind":"production_path","selection":{"id":"selection-1","revision":"d".repeat(64)},"obligationId":"obligation-1","subject":subject(),"coveredState":"review-result","execution":{"attemptId":"attempt-1","resultDigest":"b".repeat(64)},"realization":{"requested":"model-1","observed":"model-1"},"capabilityEnvelope":{"capabilities":["read"],"mutationAuthorized":false},"continuity":{"mode":"fresh_initial","sessionId":"session-1"},"transport":{"mechanism":"transport-1","digest":"c".repeat(64)},"observer":{"identity":"observer-1","kind":"trusted"},"observedAt":"2026-10-08T01:02:03.004Z","adapterVersion":"v1","artifacts":[{"owner":"owner-1","reference":"artifact:one","digest":"a".repeat(64),"status":"verified"}]}),
    );
    let id = digest(&without(&value, "id")).unwrap();
    mutate(
        &mut value,
        "id",
        JsValue::text(&format!("production-path-observation-v1@{id}")),
    );
    value
}
fn bootstrap(base: &Path) -> BootstrapFiles {
    let custody = json!({"owner":"owner-1","verifier":"verifier-1","profile":"custody-1","artifact_schemes":["artifact:"],"reference_schemes":["evidence:"]});
    let custody_sha = digest(&parsed(custody.clone())).unwrap();
    let source = json!({"schema_version":2,"owner":"host-1","reference":"source-1","revision":"r1","freshness":"exact immutable revision","profile":"native-review-claims-v1","actors":["campaign"],"decision_scopes":["scope-1"],"grant_ids":["grant-production"],"custody_config_sha256":custody_sha});
    let source_bytes = source.to_string();
    let source_sha = sha256_bytes(source_bytes.as_bytes());
    let reference = json!({"owner":"host-1","reference":"source-1","revision":"r1","integrity_sha256":source_sha,"freshness":"exact immutable revision","status":"verified"});
    let config = json!({"schema_version":2,"profile":"native-review-claims-v1","root_id":"claims-root","trusted_custody":custody,"grants":[{"schema_version":1,"grant_id":"grant-production","actor":"campaign","profile":"production-path-v1","permissions":["record_observation","establish_claim","read_admission"],"decision_scope":"scope-1","authority_reference":reference}]});
    let config_bytes = config.to_string();
    let config_path = base.join("config.json");
    let source_path = base.join("source.json");
    fs::write(&config_path, &config_bytes).unwrap();
    fs::write(&source_path, &source_bytes).unwrap();
    BootstrapFiles {
        root: base.join("root"),
        config: config_path,
        source: source_path,
        expected_config_sha256: sha256_bytes(config_bytes.as_bytes()),
        expected_source_sha256: source_sha,
    }
}
fn open(files: &BootstrapFiles) -> OpenFiles {
    OpenFiles {
        root: files.root.clone(),
        config: files.config.clone(),
        source: files.source.clone(),
        expected_config_sha256: files.expected_config_sha256.clone(),
        expected_source_sha256: files.expected_source_sha256.clone(),
    }
}
fn binding(stage: ProductionPathStage, session: Option<&str>) -> ProductionPathAdmissionBinding {
    ProductionPathAdmissionBinding {
        campaign_root_id: "campaign-root".into(),
        campaign_revision: "revision-1".into(),
        campaign_operation_id: "campaign-op-1".into(),
        obligation_id: "obligation-1".into(),
        candidate_digest: "c".repeat(64),
        selection_digest: "d".repeat(64),
        profile_digest: "e".repeat(64),
        prepared_request_sha256: "a".repeat(64),
        child_request_sha256: String::new(),
        review_episode_id: "episode-1".into(),
        attempt_id: "attempt-1".into(),
        native_result_sha256: "b".repeat(64),
        episode_revision: None,
        session_id: session.map(str::to_owned),
        stage,
        access: ProductionPathAccess::Original,
    }
}
fn with_digest(mut request: ProductionPathRequest) -> ProductionPathRequest {
    let sha = production_path_request_sha256(&request).unwrap();
    match &mut request {
        ProductionPathRequest::RecordObservation(c) => c.admission.child_request_sha256 = sha,
        ProductionPathRequest::EstablishClaim(c) => c.admission.child_request_sha256 = sha,
        ProductionPathRequest::ReadAdmission(c) => c.admission.child_request_sha256 = sha,
    };
    request
}
fn observation_request(observation: JsValue) -> ObservationCommand {
    match with_digest(ProductionPathRequest::RecordObservation(
        ObservationCommand {
            grant_id: "grant-production".into(),
            admission: binding(ProductionPathStage::RecordObservation, Some("session-1")),
            observation,
        },
    )) {
        ProductionPathRequest::RecordObservation(c) => c,
        _ => unreachable!(),
    }
}
fn establishment_request(
    claim: JsValue,
    observation_id: Option<String>,
    operation: &str,
) -> EstablishmentCommand {
    match with_digest(ProductionPathRequest::EstablishClaim(
        EstablishmentCommand {
            grant_id: "grant-production".into(),
            admission: binding(
                ProductionPathStage::EstablishClaim,
                observation_id.as_ref().map(|_| "session-1"),
            ),
            operation_id: operation.into(),
            claim,
            observation_id,
        },
    )) {
        ProductionPathRequest::EstablishClaim(c) => c,
        _ => unreachable!(),
    }
}
struct ControlledPort {
    observed: JsValue,
    evidence: CustodyEvidence,
    unavailable: CustodyEvidence,
}
struct ControlledLease<'a>(&'a ControlledPort);
impl FindingLease for ControlledLease<'_> {}
impl FindingAdmissionPort for ControlledPort {
    fn acquire<'a>(&'a self, _: &AdmissionBinding) -> ClaimResult<Box<dyn FindingLease + 'a>> {
        Err(ClaimError::new("finding admission unavailable"))
    }
}
impl ProductionPathLease for ControlledLease<'_> {
    fn read_custody(&self, observation: Option<&JsValue>) -> ClaimResult<CustodyEvidence> {
        match observation {
            Some(value) if value == &self.0.observed => Ok(self.0.evidence.clone()),
            Some(_) => Err(ClaimError::new("controlled owner has no observation")),
            None => Ok(self.0.unavailable.clone()),
        }
    }
}
impl ProductionPathAdmissionPort for ControlledPort {
    fn acquire_production_path<'a>(
        &'a self,
        binding: &ProductionPathAdmissionBinding,
        request: &ProductionPathRequest,
    ) -> ClaimResult<Box<dyn ProductionPathLease + 'a>> {
        if binding.campaign_root_id != "campaign-root"
            || binding.campaign_revision != "revision-1"
            || binding.child_request_sha256 != production_path_request_sha256(request)?
        {
            return Err(ClaimError::new("controlled campaign admission mismatch"));
        }
        Ok(Box::new(ControlledLease(self)))
    }
}
fn port(observed: JsValue) -> ControlledPort {
    let base = CustodyEvidence {
        owner: "owner-1".into(),
        reference: "evidence:one".into(),
        revision: "owner-rev-1".into(),
        sha256: "f".repeat(64),
        verifier: "verifier-1".into(),
        profile: "custody-1".into(),
        attempt_id: "attempt-1".into(),
        native_result_sha256: "b".repeat(64),
        session_id: Some("session-1".into()),
        observation_sha256: Some(digest(&observed).unwrap()),
        artifact_references: vec!["artifact:one".into()],
    };
    let unavailable = CustodyEvidence {
        reference: "evidence:absence".into(),
        session_id: None,
        observation_sha256: None,
        artifact_references: vec![],
        ..base.clone()
    };
    ControlledPort {
        observed,
        evidence: base,
        unavailable,
    }
}
fn scratch() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("claim-evidence-ce3-")
        .tempdir_in(std::env::var("TMPDIR").unwrap())
        .unwrap()
}
fn receipt(publication: ProductionPathPublication) -> claim_evidence::ProductionPathReceipt {
    match publication {
        ProductionPathPublication::Applied(r) | ProductionPathPublication::Replayed(r) => r,
    }
}

#[test]
fn controlled_custody_observation_and_exact_dual_claim_readback() {
    let dir = scratch();
    let files = bootstrap(dir.path());
    let obs = observation();
    let mut app = ClaimsApplication::initialize(files.clone(), port(obs.clone())).unwrap();
    let command = observation_request(obs.clone());
    let locator = app.observation_locator(&command).unwrap();
    assert_eq!(
        app.reconcile_observation(&locator),
        ProductionPathReconciliation::Absent
    );
    let observed = receipt(app.record_observation(command.clone()).unwrap());
    assert_eq!(receipt(app.record_observation(command).unwrap()), observed);
    assert!(matches!(
        app.reconcile_observation(&observed.locator),
        ProductionPathReconciliation::Committed(_)
    ));
    for (boundary, operation) in [
        ("builder_projection", "est-1"),
        ("campaign_terminalization", "est-2"),
    ] {
        let selected = claim(boundary);
        let command = establishment_request(selected.clone(), Some(observed.id.clone()), operation);
        let established = receipt(app.establish_claim(command.clone()).unwrap());
        assert_eq!(receipt(app.establish_claim(command).unwrap()), established);
        let request = AdmissionReadRequest {
            grant_id: "grant-production".into(),
            admission: binding(ProductionPathStage::ReadAdmission, Some("session-1")),
            claim: selected,
            establishment_id: established.id.clone(),
            establishment_sha256: established.sha256.clone(),
            establishment_request_sha256: established.locator.payload_sha256.clone(),
            establishment_admission_sha256: established.locator.admission_sha256.clone(),
            observation_id: Some(observed.id.clone()),
            observation_sha256: Some(observed.sha256.clone()),
        };
        let request = match with_digest(ProductionPathRequest::ReadAdmission(request)) {
            ProductionPathRequest::ReadAdmission(r) => r,
            _ => unreachable!(),
        };
        let checked = app.read_claim_admission(request).unwrap();
        assert_eq!(checked.status().unwrap(), "established");
        assert_eq!(checked.boundary().unwrap(), boundary);
    }
    drop(app);
    let reopened = ClaimsApplication::open(open(&files), port(obs)).unwrap();
    assert!(matches!(
        reopened.reconcile_observation(&observed.locator),
        ProductionPathReconciliation::Committed(_)
    ));
}

#[test]
fn reordered_artifact_custody_survives_commit_reconciliation_and_readback() {
    let dir = scratch();
    let files = bootstrap(dir.path());
    let mut obs = observation();
    let mut artifacts = obs.get("artifacts").unwrap().as_array().unwrap().to_vec();
    artifacts.push(parsed(json!({
        "owner":"owner-1",
        "reference":"artifact:two",
        "digest":"2".repeat(64),
        "status":"verified"
    })));
    mutate(&mut obs, "artifacts", JsValue::Array(artifacts));
    let id = digest(&without(&obs, "id")).unwrap();
    mutate(
        &mut obs,
        "id",
        JsValue::text(&format!("production-path-observation-v1@{id}")),
    );
    let mut owner = port(obs.clone());
    owner.evidence.artifact_references = vec!["artifact:two".into(), "artifact:one".into()];
    let mut app = ClaimsApplication::initialize(files.clone(), owner).unwrap();
    let observed = receipt(
        app.record_observation(observation_request(obs.clone()))
            .unwrap(),
    );
    assert!(matches!(
        app.reconcile_observation(&observed.locator),
        ProductionPathReconciliation::Committed(_)
    ));
    let selected = claim("builder_projection");
    let established = receipt(
        app.establish_claim(establishment_request(
            selected.clone(),
            Some(observed.id.clone()),
            "est-two-artifacts",
        ))
        .unwrap(),
    );
    assert!(matches!(
        app.reconcile_establishment(&established.locator, &selected),
        ProductionPathReconciliation::Committed(_)
    ));
    let read = AdmissionReadRequest {
        grant_id: "grant-production".into(),
        admission: binding(ProductionPathStage::ReadAdmission, Some("session-1")),
        claim: selected.clone(),
        establishment_id: established.id.clone(),
        establishment_sha256: established.sha256.clone(),
        establishment_request_sha256: established.locator.payload_sha256.clone(),
        establishment_admission_sha256: established.locator.admission_sha256.clone(),
        observation_id: Some(observed.id.clone()),
        observation_sha256: Some(observed.sha256.clone()),
    };
    let read = match with_digest(ProductionPathRequest::ReadAdmission(read)) {
        ProductionPathRequest::ReadAdmission(r) => r,
        _ => unreachable!(),
    };
    assert_eq!(
        app.read_claim_admission(read).unwrap().status().unwrap(),
        "established"
    );
    drop(app);
    let mut reopened_owner = port(obs.clone());
    reopened_owner.evidence.artifact_references =
        vec!["artifact:two".into(), "artifact:one".into()];
    let reopened = ClaimsApplication::open(open(&files), reopened_owner).unwrap();
    assert!(matches!(
        reopened.reconcile_establishment(&established.locator, &selected),
        ProductionPathReconciliation::Committed(_)
    ));
    let bad_dir = scratch();
    let bad_files = bootstrap(bad_dir.path());
    let mut bad_owner = port(obs.clone());
    bad_owner.evidence.artifact_references = vec!["artifact:one".into(), "artifact:one".into()];
    let mut bad_app = ClaimsApplication::initialize(bad_files, bad_owner).unwrap();
    assert!(matches!(
        bad_app.record_observation(observation_request(obs.clone())),
        Err(ProductionPathWriteError::NoEffect(_))
    ));
    let mut duplicate_obs = obs;
    if let JsValue::Object(fields) = &mut duplicate_obs {
        let artifacts = fields.get_mut(&"artifacts".into()).unwrap();
        if let JsValue::Array(artifacts) = artifacts {
            mutate(
                &mut artifacts[1],
                "reference",
                JsValue::text("artifact:one"),
            );
        }
    }
    let id = digest(&without(&duplicate_obs, "id")).unwrap();
    mutate(
        &mut duplicate_obs,
        "id",
        JsValue::text(&format!("production-path-observation-v1@{id}")),
    );
    let duplicate_dir = scratch();
    let duplicate_files = bootstrap(duplicate_dir.path());
    let mut duplicate_owner = port(duplicate_obs.clone());
    duplicate_owner.evidence.artifact_references =
        vec!["artifact:one".into(), "artifact:one".into()];
    let mut duplicate_app =
        ClaimsApplication::initialize(duplicate_files, duplicate_owner).unwrap();
    let duplicate_receipt = receipt(
        duplicate_app
            .record_observation(observation_request(duplicate_obs))
            .unwrap(),
    );
    let duplicate_claim = claim("builder_projection");
    let duplicate_establishment = receipt(
        duplicate_app
            .establish_claim(establishment_request(
                duplicate_claim.clone(),
                Some(duplicate_receipt.id),
                "est-duplicate-artifact-references",
            ))
            .unwrap(),
    );
    assert!(matches!(
        duplicate_app.reconcile_establishment(&duplicate_establishment.locator, &duplicate_claim),
        ProductionPathReconciliation::Committed(_)
    ));
}

#[test]
fn refusal_absence_false_and_tamper_are_distinct() {
    let dir = scratch();
    let files = bootstrap(dir.path());
    let obs = observation();
    let mut app = ClaimsApplication::initialize(files.clone(), port(obs.clone())).unwrap();
    let selected = claim("builder_projection");
    let absent = establishment_request(selected.clone(), None, "est-absent");
    let absent_receipt = receipt(app.establish_claim(absent).unwrap());
    let read = AdmissionReadRequest {
        grant_id: "grant-production".into(),
        admission: binding(ProductionPathStage::ReadAdmission, None),
        claim: selected.clone(),
        establishment_id: absent_receipt.id.clone(),
        establishment_sha256: absent_receipt.sha256.clone(),
        establishment_request_sha256: absent_receipt.locator.payload_sha256.clone(),
        establishment_admission_sha256: absent_receipt.locator.admission_sha256.clone(),
        observation_id: None,
        observation_sha256: None,
    };
    let read = match with_digest(ProductionPathRequest::ReadAdmission(read)) {
        ProductionPathRequest::ReadAdmission(r) => r,
        _ => unreachable!(),
    };
    assert_eq!(
        app.read_claim_admission(read).unwrap().status().unwrap(),
        "unestablished"
    );
    let mut changed = obs.clone();
    mutate(&mut changed, "event_identity", JsValue::text("forged"));
    assert!(matches!(
        app.record_observation(observation_request(changed)),
        Err(ProductionPathWriteError::NoEffect(_))
    ));
    let observed = receipt(
        app.record_observation(observation_request(obs.clone()))
            .unwrap(),
    );
    let mut false_obs = obs.clone();
    if let JsValue::Object(fields) = &mut false_obs {
        let cap = fields.get_mut(&"capabilityEnvelope".into()).unwrap();
        mutate(cap, "mutationAuthorized", JsValue::Bool(true));
    }
    let id = digest(&without(&false_obs, "id")).unwrap();
    mutate(
        &mut false_obs,
        "id",
        JsValue::text(&format!("production-path-observation-v1@{id}")),
    );
    // A caller's changed flag has no controlled-owner record and is refused before truth evaluation.
    assert!(matches!(
        app.record_observation(observation_request(false_obs)),
        Err(ProductionPathWriteError::NoEffect(_))
    ));
    let mut wrong = establishment_request(selected, Some(observed.id.clone()), "est-wrong");
    wrong.admission.campaign_revision = "stale".into();
    assert!(matches!(
        app.establish_claim(wrong),
        Err(ProductionPathWriteError::NoEffect(_))
    ));
    drop(app);
    let db = files.root.join("claim-evidence.sqlite3");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "UPDATE production_path_observations SET observation_sha256=? WHERE observation_id=?",
        ["0".repeat(64), observed.id.clone()],
    )
    .unwrap();
    drop(conn);
    let reopened = ClaimsApplication::open(open(&files), port(obs)).unwrap();
    assert!(matches!(
        reopened.reconcile_observation(&observed.locator),
        ProductionPathReconciliation::Conflicting | ProductionPathReconciliation::Unresolved(_)
    ));
}

#[test]
fn mutation_authorization_is_false_with_sorted_reasons() {
    let dir = scratch();
    let files = bootstrap(dir.path());
    let mut obs = observation();
    if let JsValue::Object(fields) = &mut obs {
        let cap = fields.get_mut(&"capabilityEnvelope".into()).unwrap();
        mutate(cap, "mutationAuthorized", JsValue::Bool(true));
        let realization = fields.get_mut(&"realization".into()).unwrap();
        mutate(realization, "observed", JsValue::text("other-model"));
    }
    let id = digest(&without(&obs, "id")).unwrap();
    mutate(
        &mut obs,
        "id",
        JsValue::text(&format!("production-path-observation-v1@{id}")),
    );
    let mut app = ClaimsApplication::initialize(files, port(obs.clone())).unwrap();
    let observed = receipt(app.record_observation(observation_request(obs)).unwrap());
    let command =
        establishment_request(claim("builder_projection"), Some(observed.id), "est-false");
    let established = receipt(app.establish_claim(command).unwrap());
    let db = rusqlite::Connection::open(app.root().absolute_path.join("claim-evidence.sqlite3"))
        .unwrap();
    let row:String=db.query_row("SELECT establishment_json FROM production_path_establishments WHERE establishment_id=?",[&established.id],|r|r.get(0)).unwrap();
    let value = parse_json(&row).unwrap();
    assert_eq!(value.get("status"), Some(&JsValue::text("false")));
    assert_eq!(
        value.get("reasons"),
        Some(&JsValue::Array(vec![
            JsValue::text("mutation_authorized"),
            JsValue::text("realization_mismatch")
        ]))
    );
}

#[test]
fn wrong_custody_owner_is_not_admissible() {
    let dir = scratch();
    let files = bootstrap(dir.path());
    let obs = observation();
    let mut controlled = port(obs.clone());
    controlled.evidence.owner = "request-claims-verified".into();
    let mut app = ClaimsApplication::initialize(files, controlled).unwrap();
    assert!(matches!(
        app.record_observation(observation_request(obs)),
        Err(ProductionPathWriteError::NoEffect(_))
    ));
    let db = rusqlite::Connection::open(app.root().absolute_path.join("claim-evidence.sqlite3"))
        .unwrap();
    let count: i64 = db
        .query_row(
            "SELECT count(*) FROM production_path_observations",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn generated_production_schemas_validate_real_documents() {
    for kind in claim_evidence::schema::DTO_SCHEMA_KINDS {
        let schema = claim_evidence::schema::dto_schema(kind).unwrap();
        let checked = fs::read_to_string(format!(
            "{}/schemas/{kind}.schema.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert_eq!(
            claim_evidence::codec::canonical_json(&schema).unwrap(),
            checked
        );
    }
    for (kind, value) in [
        ("production-path-claim-v1", claim("builder_projection")),
        ("production-path-observation-v2", observation()),
    ] {
        let schema_json: Value = serde_json::from_str(
            &fs::read_to_string(format!(
                "{}/schemas/{kind}.schema.json",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap(),
        )
        .unwrap();
        let validator = jsonschema::validator_for(&schema_json).unwrap();
        let document: Value =
            serde_json::from_str(&claim_evidence::codec::canonical_json(&value).unwrap()).unwrap();
        assert!(validator.is_valid(&document), "{kind}");
    }
    let dir = scratch();
    let files = bootstrap(dir.path());
    for (kind, path) in [
        ("bootstrap-config-v2", &files.config),
        ("bootstrap-source-v2", &files.source),
    ] {
        let schema: Value = serde_json::from_str(
            &fs::read_to_string(format!(
                "{}/schemas/{kind}.schema.json",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap(),
        )
        .unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let document: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert!(validator.is_valid(&document), "{kind}");
        if kind == "bootstrap-config-v2" {
            let mut unsupported = document;
            unsupported["grants"][0]["permissions"] = json!(["create_claim"]);
            assert!(!validator.is_valid(&unsupported));
        }
    }
}

#[test]
fn status_tamper_even_with_consistent_record_hash_refuses_readback() {
    let dir = scratch();
    let files = bootstrap(dir.path());
    let obs = observation();
    let mut app = ClaimsApplication::initialize(files, port(obs)).unwrap();
    let selected = claim("builder_projection");
    let command = establishment_request(selected.clone(), None, "est-status");
    let receipt = receipt(app.establish_claim(command).unwrap());
    let db = rusqlite::Connection::open(app.root().absolute_path.join("claim-evidence.sqlite3"))
        .unwrap();
    let row:String=db.query_row("SELECT establishment_json FROM production_path_establishments WHERE operation_id='est-status'",[],|r|r.get(0)).unwrap();
    let mut value = parse_json(&row).unwrap();
    mutate(&mut value, "status", JsValue::text("established"));
    let changed_id = format!(
        "production-path-establishment-v1@{}",
        digest(&without(&value, "id")).unwrap()
    );
    mutate(&mut value, "id", JsValue::text(&changed_id));
    let changed_sha = digest(&value).unwrap();
    db.execute("UPDATE production_path_establishments SET establishment_id=?,establishment_sha256=?,establishment_json=? WHERE operation_id='est-status'",rusqlite::params![changed_id,changed_sha,claim_evidence::codec::canonical_json(&value).unwrap()]).unwrap();
    drop(db);
    let request = AdmissionReadRequest {
        grant_id: "grant-production".into(),
        admission: binding(ProductionPathStage::ReadAdmission, None),
        claim: selected,
        establishment_id: changed_id,
        establishment_sha256: changed_sha,
        establishment_request_sha256: receipt.locator.payload_sha256,
        establishment_admission_sha256: receipt.locator.admission_sha256,
        observation_id: None,
        observation_sha256: None,
    };
    let request = match with_digest(ProductionPathRequest::ReadAdmission(request)) {
        ProductionPathRequest::ReadAdmission(r) => r,
        _ => unreachable!(),
    };
    assert!(
        app.read_claim_admission(request)
            .unwrap_err()
            .message
            .contains("evaluation")
    );
}

#[cfg(feature = "test-faults")]
#[test]
fn ce3_fault_child() {
    let Ok(base) = std::env::var("CE3_CHILD_BASE") else {
        return;
    };
    let files = bootstrap(Path::new(&base));
    let obs = observation();
    let mut app = ClaimsApplication::open(open(&files), port(obs.clone())).unwrap();
    match std::env::var("CE3_CHILD_STAGE").unwrap().as_str() {
        "observation" => {
            let _ = app.record_observation(observation_request(obs));
        }
        "establishment" => {
            let id = obs
                .get("id")
                .unwrap()
                .as_text()
                .unwrap()
                .to_string_checked()
                .unwrap();
            let _ = app.establish_claim(establishment_request(
                claim("builder_projection"),
                Some(id),
                "est-fault",
            ));
        }
        "terminal" => {
            let id = obs
                .get("id")
                .unwrap()
                .as_text()
                .unwrap()
                .to_string_checked()
                .unwrap();
            let _ = app.establish_claim(establishment_request(
                claim("campaign_terminalization"),
                Some(id),
                "est-terminal-fault",
            ));
        }
        _ => panic!("invalid stage"),
    }
}
#[cfg(feature = "test-faults")]
fn wait_fault(path: &Path) {
    let start = std::time::Instant::now();
    while !path.exists() {
        assert!(
            start.elapsed() < std::time::Duration::from_secs(10),
            "fault cut never reached"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
#[cfg(feature = "test-faults")]
#[test]
fn actual_process_observation_and_each_establishment_cut() {
    use std::process::{Command, Stdio};
    for (stage, cut, committed) in [
        ("observation", "before_observation_commit", false),
        ("observation", "after_observation_commit", true),
        ("establishment", "before_establishment_commit", false),
        ("establishment", "after_establishment_commit", true),
        ("terminal", "before_establishment_commit", false),
        ("terminal", "after_establishment_commit", true),
    ] {
        let dir = scratch();
        let files = bootstrap(dir.path());
        let obs = observation();
        let mut app = ClaimsApplication::initialize(files.clone(), port(obs.clone())).unwrap();
        if stage != "observation" {
            app.record_observation(observation_request(obs.clone()))
                .unwrap();
        }
        if stage == "terminal" {
            let id = obs
                .get("id")
                .unwrap()
                .as_text()
                .unwrap()
                .to_string_checked()
                .unwrap();
            app.establish_claim(establishment_request(
                claim("builder_projection"),
                Some(id),
                "est-1",
            ))
            .unwrap();
        }
        let locator = if stage == "observation" {
            app.observation_locator(&observation_request(obs.clone()))
                .unwrap()
        } else {
            let id = obs
                .get("id")
                .unwrap()
                .as_text()
                .unwrap()
                .to_string_checked()
                .unwrap();
            let c = establishment_request(
                claim(if stage == "terminal" {
                    "campaign_terminalization"
                } else {
                    "builder_projection"
                }),
                Some(id),
                if stage == "terminal" {
                    "est-terminal-fault"
                } else {
                    "est-fault"
                },
            );
            app.establishment_locator(&c).unwrap()
        };
        drop(app);
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("ce3_fault_child")
            .arg("--nocapture")
            .env("CE3_CHILD_BASE", dir.path())
            .env("CE3_CHILD_STAGE", stage)
            .env("CLAIM_EVIDENCE_FAULT_CUT", cut)
            .env("CLAIM_EVIDENCE_FAULT_DIR", dir.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        wait_fault(&dir.path().join(format!("{cut}.ready")));
        child.kill().unwrap();
        child.wait().unwrap();
        let reopened = ClaimsApplication::open(open(&files), port(obs)).unwrap();
        let actual = if stage == "observation" {
            reopened.reconcile_observation(&locator)
        } else {
            reopened.reconcile_establishment(
                &locator,
                &claim(if stage == "terminal" {
                    "campaign_terminalization"
                } else {
                    "builder_projection"
                }),
            )
        };
        assert_eq!(
            matches!(actual, ProductionPathReconciliation::Committed(_)),
            committed,
            "{stage}/{cut}: {actual:?}"
        );
        if !committed {
            assert_eq!(actual, ProductionPathReconciliation::Absent);
        }
        if stage == "terminal" {
            let db = rusqlite::Connection::open(
                reopened.root().absolute_path.join("claim-evidence.sqlite3"),
            )
            .unwrap();
            let count: i64 = db
                .query_row(
                    "SELECT count(*) FROM production_path_establishments",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, if committed { 2 } else { 1 });
        }
    }
}

#[test]
fn frozen_legacy_production_vectors_match_rust_codec() {
    let fixture: Value = serde_json::from_str(include_str!(
        "fixtures/production-path-v1/legacy-vectors.json"
    ))
    .unwrap();
    assert_eq!(fixture["schema_version"], 1);
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    for (field, path) in [
        (
            "contract_sha256",
            "app-server/src/services/claim-evidence/production-path-contract.mjs",
        ),
        (
            "service_sha256",
            "app-server/src/services/claim-evidence/production-path-service.mjs",
        ),
        (
            "identity_sha256",
            "app-server/src/services/claim-evidence/identity.mjs",
        ),
    ] {
        assert_eq!(
            fixture["sources"][field],
            sha256_bytes(&fs::read(repo.join(path)).unwrap())
        );
    }
    for (i, vector) in fixture["vectors"].as_array().unwrap().iter().enumerate() {
        let value = parsed(vector["value"].clone());
        assert_eq!(
            claim_evidence::codec::canonical_json(&value).unwrap(),
            vector["canonical"],
            "vector {i}"
        );
        assert_eq!(digest(&value).unwrap(), vector["sha256"], "vector {i}");
        match i {
            0 | 1 => claim_evidence::validate_production_path_claim(&value).unwrap(),
            2 | 3 => claim_evidence::validate_production_path_observation(&value).unwrap(),
            _ => claim_evidence::validate_production_path_establishment(&value).unwrap(),
        }
    }
    assert_eq!(fixture["vectors"][4]["value"]["status"], "established");
    assert_eq!(
        fixture["vectors"][6]["value"]["reasons"],
        json!(["observation_unavailable"])
    );
    assert_eq!(fixture["vectors"][7]["value"]["status"], "false");
}

#[test]
fn production_command_capacity_refuses_before_effect_and_v1_open_refuses() {
    let dir = scratch();
    let files = bootstrap(dir.path());
    let obs = observation();
    let mut app = ClaimsApplication::initialize(files.clone(), port(obs.clone())).unwrap();
    let mut huge = obs;
    mutate(
        &mut huge,
        "adapterVersion",
        JsValue::text(&"x".repeat(1024 * 1024)),
    );
    let id = digest(&without(&huge, "id")).unwrap();
    mutate(
        &mut huge,
        "id",
        JsValue::text(&format!("production-path-observation-v1@{id}")),
    );
    assert!(matches!(
        app.record_observation(observation_request(huge)),
        Err(ProductionPathWriteError::NoEffect(_))
    ));
    let db = rusqlite::Connection::open(app.root().absolute_path.join("claim-evidence.sqlite3"))
        .unwrap();
    let count: i64 = db
        .query_row(
            "SELECT count(*) FROM production_path_observations",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    db.execute_batch("PRAGMA user_version=1").unwrap();
    drop(db);
    drop(app);
    assert!(ClaimsApplication::open(open(&files), port(observation())).is_err());
}

#[test]
fn root_and_bootstrap_binding_refuse_cross_root_or_changed_source() {
    let first = scratch();
    let first_files = bootstrap(first.path());
    let obs = observation();
    let mut app = ClaimsApplication::initialize(first_files.clone(), port(obs.clone())).unwrap();
    let locator = app
        .observation_locator(&observation_request(obs.clone()))
        .unwrap();
    let second = scratch();
    let second_files = bootstrap(second.path());
    let second_app = ClaimsApplication::initialize(second_files, port(obs.clone())).unwrap();
    assert!(matches!(
        second_app.reconcile_observation(&locator),
        ProductionPathReconciliation::Conflicting
    ));
    fs::write(&first_files.source, b"changed source\n").unwrap();
    assert!(matches!(
        app.record_observation(observation_request(obs)),
        Err(ProductionPathWriteError::NoEffect(_))
    ));
    assert!(ClaimsApplication::open(open(&first_files), port(observation())).is_err());
}

#[cfg(feature = "test-faults")]
#[test]
fn production_busy_writer_is_no_effect() {
    let dir = scratch();
    let files = bootstrap(dir.path());
    let obs = observation();
    let mut app = ClaimsApplication::initialize(files, port(obs.clone())).unwrap();
    let blocker =
        rusqlite::Connection::open(app.root().absolute_path.join("claim-evidence.sqlite3"))
            .unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(matches!(
        app.record_observation(observation_request(obs.clone())),
        Err(ProductionPathWriteError::NoEffect(_))
    ));
    blocker.execute_batch("ROLLBACK").unwrap();
    assert!(matches!(
        app.record_observation(observation_request(obs)),
        Ok(ProductionPathPublication::Applied(_))
    ));
}

#[test]
fn recorded_legacy_delta_vectors_refuse_at_owner_boundary() {
    let deltas: Value = serde_json::from_str(include_str!(
        "fixtures/production-path-v1/expected-deltas.json"
    ))
    .unwrap();
    assert_eq!(deltas["schema_version"], 1);
    assert_eq!(deltas["legacy_observations"].as_array().unwrap().len(), 3);
    let mut empty_event = observation();
    mutate(&mut empty_event, "event_identity", JsValue::text(""));
    let id = digest(&without(&empty_event, "id")).unwrap();
    mutate(
        &mut empty_event,
        "id",
        JsValue::text(&format!("production-path-observation-v1@{id}")),
    );
    assert!(claim_evidence::validate_production_path_observation(&empty_event).is_err());
    let fixture: Value = serde_json::from_str(include_str!(
        "fixtures/production-path-v1/legacy-vectors.json"
    ))
    .unwrap();
    let mut prior = parsed(fixture["vectors"][4]["value"].clone());
    mutate(&mut prior, "predecessor", JsValue::text("prior"));
    let id = digest(&without(&prior, "id")).unwrap();
    mutate(
        &mut prior,
        "id",
        JsValue::text(&format!("production-path-establishment-v1@{id}")),
    );
    assert!(claim_evidence::validate_production_path_establishment(&prior).is_err());
}

#[test]
fn independently_persisted_observation_bindings_reject_tamper() {
    for (column, replacement) in [
        ("event_identity", "changed-event"),
        ("observation_id", "changed-observation"),
        ("observation_sha256", "0"),
        ("observation_json", "{}"),
        ("request_sha256", "0"),
        ("admission_sha256", "0"),
        ("grant_id", "changed-grant"),
        ("grant_sha256", "0"),
        ("custody_json", "{}"),
        ("custody_sha256", "0"),
    ] {
        let dir = scratch();
        let files = bootstrap(dir.path());
        let obs = observation();
        let mut app = ClaimsApplication::initialize(files.clone(), port(obs.clone())).unwrap();
        let receipt = receipt(
            app.record_observation(observation_request(obs.clone()))
                .unwrap(),
        );
        drop(app);
        let conn = rusqlite::Connection::open(files.root.join("claim-evidence.sqlite3")).unwrap();
        let sql = format!(
            "UPDATE production_path_observations SET {column}=? WHERE event_identity='event-1'"
        );
        conn.execute(&sql, [replacement]).unwrap();
        drop(conn);
        let reopened = ClaimsApplication::open(open(&files), port(obs)).unwrap();
        assert!(
            !matches!(
                reopened.reconcile_observation(&receipt.locator),
                ProductionPathReconciliation::Committed(_)
            ),
            "{column}"
        );
    }
}

#[test]
fn missing_referenced_observation_never_becomes_unavailable_sentinel() {
    let dir = scratch();
    let files = bootstrap(dir.path());
    let obs = observation();
    let mut app = ClaimsApplication::initialize(files.clone(), port(obs.clone())).unwrap();
    let observed = receipt(
        app.record_observation(observation_request(obs.clone()))
            .unwrap(),
    );
    let selected = claim("builder_projection");
    let established = receipt(
        app.establish_claim(establishment_request(
            selected.clone(),
            Some(observed.id.clone()),
            "est-missing",
        ))
        .unwrap(),
    );
    drop(app);
    let conn = rusqlite::Connection::open(files.root.join("claim-evidence.sqlite3")).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
    conn.execute(
        "DELETE FROM production_path_observations WHERE observation_id=?",
        [&observed.id],
    )
    .unwrap();
    drop(conn);
    let reopened = ClaimsApplication::open(open(&files), port(obs)).unwrap();
    assert!(matches!(
        reopened.reconcile_establishment(&established.locator, &selected),
        ProductionPathReconciliation::Unresolved(_)
    ));
    let read = AdmissionReadRequest {
        grant_id: "grant-production".into(),
        admission: binding(ProductionPathStage::ReadAdmission, Some("session-1")),
        claim: selected,
        establishment_id: established.id,
        establishment_sha256: established.sha256,
        establishment_request_sha256: established.locator.payload_sha256,
        establishment_admission_sha256: established.locator.admission_sha256,
        observation_id: Some(observed.id),
        observation_sha256: Some(observed.sha256),
    };
    let read = match with_digest(ProductionPathRequest::ReadAdmission(read)) {
        ProductionPathRequest::ReadAdmission(r) => r,
        _ => unreachable!(),
    };
    assert!(reopened.read_claim_admission(read).is_err());
}

#[test]
fn root_total_capacity_refuses_without_partial_row() {
    let dir = scratch();
    let files = bootstrap(dir.path());
    let template = observation();
    let mut app = ClaimsApplication::initialize(files.clone(), port(template)).unwrap();
    drop(app);
    let mut admitted = 0;
    for index in 0..22 {
        let mut obs = observation();
        mutate(
            &mut obs,
            "event_identity",
            JsValue::text(&format!("large-event-{index}")),
        );
        mutate(
            &mut obs,
            "adapterVersion",
            JsValue::text(&format!("{}-{index}", "x".repeat(900_000))),
        );
        let id = digest(&without(&obs, "id")).unwrap();
        mutate(
            &mut obs,
            "id",
            JsValue::text(&format!("production-path-observation-v1@{id}")),
        );
        app = ClaimsApplication::open(open(&files), port(obs.clone())).unwrap();
        match app.record_observation(observation_request(obs)) {
            Ok(ProductionPathPublication::Applied(_)) => admitted += 1,
            Err(ProductionPathWriteError::NoEffect(error)) => {
                assert!(error.message.contains("root capacity"), "{error:?}");
                break;
            }
            result => panic!("unexpected capacity result: {result:?}"),
        }
        drop(app);
    }
    assert!((16..=19).contains(&admitted), "admitted {admitted}");
    let conn = rusqlite::Connection::open(files.root.join("claim-evidence.sqlite3")).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM production_path_observations",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, admitted);
}

#[test]
fn oversized_binding_regression() {
    let dir = scratch();
    let files = bootstrap(dir.path());
    let obs = observation();
    let mut app = ClaimsApplication::initialize(files, port(obs.clone())).unwrap();
    let mut command = observation_request(obs.clone());
    command.admission.campaign_operation_id = "x".repeat(1024 * 1024);
    assert!(
        matches!(
            app.record_observation(command),
            Err(ProductionPathWriteError::NoEffect(_))
        ),
        "complete admission envelope must be bounded"
    );
}

#[test]
fn empty_artifact_profile_regression() {
    let mut empty = observation();
    mutate(&mut empty, "artifacts", JsValue::Array(vec![]));
    let id = digest(&without(&empty, "id")).unwrap();
    mutate(
        &mut empty,
        "id",
        JsValue::text(&format!("production-path-observation-v1@{id}")),
    );
    let mut owner = port(empty.clone());
    owner.evidence.artifact_references.clear();
    let dir = scratch();
    let files = bootstrap(dir.path());
    let mut app = ClaimsApplication::initialize(files, owner).unwrap();
    let observed = receipt(app.record_observation(observation_request(empty)).unwrap());
    let established = receipt(
        app.establish_claim(establishment_request(
            claim("builder_projection"),
            Some(observed.id),
            "est-empty-artifacts",
        ))
        .unwrap(),
    );
    assert!(matches!(
        app.reconcile_establishment(&established.locator, &claim("builder_projection")),
        ProductionPathReconciliation::Committed(_)
    ));
    let db = rusqlite::Connection::open(app.root().absolute_path.join("claim-evidence.sqlite3"))
        .unwrap();
    let json:String=db.query_row("SELECT establishment_json FROM production_path_establishments WHERE operation_id='est-empty-artifacts'",[],|r|r.get(0)).unwrap();
    assert_eq!(
        parse_json(&json).unwrap().get("status"),
        Some(&JsValue::text("established"))
    );
}

#[test]
fn establishment_custody_corruption_refuses_pre_effect_and_returned_locators() {
    for present in [true, false] {
        let dir = scratch();
        let files = bootstrap(dir.path());
        let obs = observation();
        let mut app = ClaimsApplication::initialize(files.clone(), port(obs.clone())).unwrap();
        let observed = if present {
            Some(receipt(
                app.record_observation(observation_request(obs.clone()))
                    .unwrap(),
            ))
        } else {
            None
        };
        let selected = claim("builder_projection");
        let command = establishment_request(
            selected.clone(),
            observed.as_ref().map(|r| r.id.clone()),
            if present {
                "est-present-custody"
            } else {
                "est-absent-custody"
            },
        );
        let before = app.establishment_locator(&command).unwrap();
        let after = receipt(app.establish_claim(command).unwrap());
        assert!(matches!(
            app.reconcile_establishment(&before, &selected),
            ProductionPathReconciliation::Committed(_)
        ));
        drop(app);
        let conn = rusqlite::Connection::open(files.root.join("claim-evidence.sqlite3")).unwrap();
        conn.execute(
            "UPDATE production_path_establishments SET custody_sha256=? WHERE establishment_id=?",
            ["0".repeat(64), after.id.clone()],
        )
        .unwrap();
        drop(conn);
        let reopened = ClaimsApplication::open(open(&files), port(obs)).unwrap();
        assert!(
            !matches!(
                reopened.reconcile_establishment(&before, &selected),
                ProductionPathReconciliation::Committed(_)
            ),
            "pre-effect locator, present={present}"
        );
        assert!(
            !matches!(
                reopened.reconcile_establishment(&after.locator, &selected),
                ProductionPathReconciliation::Committed(_)
            ),
            "returned locator, present={present}"
        );
    }
}

#[test]
fn historical_javascript_timestamp_domain_is_preserved() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/production-path-v1/timestamps.json")).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../app-server/src/services/claim-evidence/production-path-contract.mjs");
    assert_eq!(
        fixture["source_sha256"],
        sha256_bytes(&fs::read(source).unwrap())
    );
    for vector in fixture["vectors"].as_array().unwrap() {
        let timestamp = vector["observedAt"].as_str().unwrap();
        let mut obs = observation();
        mutate(&mut obs, "observedAt", JsValue::text(timestamp));
        let id = digest(&without(&obs, "id")).unwrap();
        mutate(
            &mut obs,
            "id",
            JsValue::text(&format!("production-path-observation-v1@{id}")),
        );
        assert_eq!(
            claim_evidence::validate_production_path_observation(&obs).is_ok(),
            vector["accepted"].as_bool().unwrap(),
            "{timestamp}"
        );
    }
}

#[test]
fn stored_oversize_is_preflighted_before_json_readback() {
    for column in ["custody_json", "observation_json", "admission_sha256"] {
        let dir = scratch();
        let files = bootstrap(dir.path());
        let obs = observation();
        let mut app = ClaimsApplication::initialize(files.clone(), port(obs.clone())).unwrap();
        let recorded = receipt(
            app.record_observation(observation_request(obs.clone()))
                .unwrap(),
        );
        drop(app);
        let conn = rusqlite::Connection::open(files.root.join("claim-evidence.sqlite3")).unwrap();
        conn.execute(
            &format!(
                "UPDATE production_path_observations SET {column}=? WHERE event_identity='event-1'"
            ),
            ["x".repeat(2 * 1024 * 1024)],
        )
        .unwrap();
        drop(conn);
        let reopened = ClaimsApplication::open(open(&files), port(obs)).unwrap();
        match reopened.reconcile_observation(&recorded.locator) {
            ProductionPathReconciliation::Unresolved(reason) => {
                assert!(reason.contains("capacity"), "{column}: {reason}")
            }
            other => panic!("{column}: {other:?}"),
        }
    }
    let dir = scratch();
    let files = bootstrap(dir.path());
    let obs = observation();
    let mut app = ClaimsApplication::initialize(files.clone(), port(obs.clone())).unwrap();
    let selected = claim("builder_projection");
    let recorded = receipt(
        app.establish_claim(establishment_request(
            selected.clone(),
            None,
            "est-oversize",
        ))
        .unwrap(),
    );
    drop(app);
    let conn = rusqlite::Connection::open(files.root.join("claim-evidence.sqlite3")).unwrap();
    let original_json:String=conn.query_row("SELECT establishment_json FROM production_path_establishments WHERE operation_id='est-oversize'",[],|r|r.get(0)).unwrap();
    conn.execute("UPDATE production_path_establishments SET establishment_json=? WHERE operation_id='est-oversize'",["x".repeat(2*1024*1024)]).unwrap();
    drop(conn);
    let reopened = ClaimsApplication::open(open(&files), port(obs)).unwrap();
    match reopened.reconcile_establishment(&recorded.locator, &selected) {
        ProductionPathReconciliation::Unresolved(reason) => {
            assert!(reason.contains("capacity"), "{reason}")
        }
        other => panic!("{other:?}"),
    }
    let conn = rusqlite::Connection::open(files.root.join("claim-evidence.sqlite3")).unwrap();
    conn.execute("UPDATE production_path_establishments SET establishment_json=?,custody_json=? WHERE operation_id='est-oversize'",rusqlite::params![original_json,"x".repeat(2*1024*1024)]).unwrap();
    drop(conn);
    match reopened.reconcile_establishment(&recorded.locator, &selected) {
        ProductionPathReconciliation::Unresolved(reason) => {
            assert!(reason.contains("capacity"), "{reason}")
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn complete_row_limit_counts_custody_and_metadata() {
    let dir = scratch();
    let files = bootstrap(dir.path());
    let obs = observation();
    let mut owner = port(obs.clone());
    owner.evidence.reference = format!("evidence:{}", "x".repeat(1_047_900));
    let mut app = ClaimsApplication::initialize(files, owner).unwrap();
    match app.record_observation(observation_request(obs)) {
        Err(ProductionPathWriteError::NoEffect(error)) => {
            assert!(error.message.contains("row exceeds capacity"), "{error:?}")
        }
        other => panic!("oversized complete row accepted: {other:?}"),
    }
}
