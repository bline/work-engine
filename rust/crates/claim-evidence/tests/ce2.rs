use claim_evidence::codec::{JsString, JsValue, parse_json};
use claim_evidence::{
    AdmissionBinding, BootstrapFiles, ClaimsApplication, FindingAdmissionPort, FindingCommand,
    FindingLease, FindingMutation, OpenFiles, OperationId, ProjectionConsumer, ProjectionRequest,
    ProjectionSelection, Publication, RelianceCommand, ReliancePublication, RelianceReconciliation,
    WriteError, claims_request_sha256, projection_request_sha256, reliance_request_sha256,
};
use serde_json::json;
use std::{fs, path::Path};
#[cfg(feature = "test-faults")]
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};

struct Lease;
impl FindingLease for Lease {}
struct Port;
impl FindingAdmissionPort for Port {
    fn acquire<'a>(
        &'a self,
        binding: &AdmissionBinding,
    ) -> claim_evidence::ClaimResult<Box<dyn FindingLease + 'a>> {
        if binding.campaign_root_id != "campaign-root"
            || binding.campaign_revision != "campaign-rev"
            || binding.campaign_operation_id != "campaign-op-1"
            || binding.obligation_id != "obligation-1"
            || binding.candidate_digest != "c".repeat(64)
            || binding.selection_digest != "d".repeat(64)
            || binding.profile_digest != "e".repeat(64)
            || binding.prepared_request_sha256 != "a".repeat(64)
            || binding.episode_revision != "episode-rev-1"
            || binding.result_sha256 != "b".repeat(64)
        {
            return Err(claim_evidence::ClaimError::new(
                "controlled admission mismatch",
            ));
        }
        Ok(Box::new(Lease))
    }
}
fn scratch() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("claim-evidence-ce2-")
        .tempdir_in(std::env::var("TMPDIR").unwrap())
        .unwrap()
}
fn hex(bytes: &str) -> Vec<u8> {
    bytes
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn bootstrap_files(base: &Path, permissions: &[&str]) -> BootstrapFiles {
    let source = base.join("source.json");
    let config = base.join("config.json");
    let source_bytes=json!({"schema_version":1,"owner":"trusted-host","reference":"source-1","revision":"r1","freshness":"exact immutable revision","profile":"native-review-claims-v1","actors":["reviewer"],"decision_scopes":["scope-1"],"grant_ids":["grant-1"]}).to_string();
    fs::write(&source, &source_bytes).unwrap();
    let source_sha = claim_evidence::sha256_bytes(source_bytes.as_bytes());
    let config_bytes=json!({"schema_version":1,"profile":"native-review-claims-v1","root_id":"root-1","grants":[{"schema_version":1,"grant_id":"grant-1","actor":"reviewer","profile":"revision-bound-review-finding-v1","permissions":permissions,"decision_scope":"scope-1","authority_reference":{"owner":"trusted-host","reference":"source-1","revision":"r1","integrity_sha256":source_sha,"freshness":"exact immutable revision","status":"verified"}}]}).to_string();
    fs::write(&config, &config_bytes).unwrap();
    BootstrapFiles {
        root: base.join("root"),
        config,
        source,
        expected_config_sha256: claim_evidence::sha256_bytes(config_bytes.as_bytes()),
        expected_source_sha256: source_sha,
    }
}
fn open(f: &BootstrapFiles) -> OpenFiles {
    OpenFiles {
        root: f.root.clone(),
        config: f.config.clone(),
        source: f.source.clone(),
        expected_config_sha256: f.expected_config_sha256.clone(),
        expected_source_sha256: f.expected_source_sha256.clone(),
    }
}
fn parsed(value: serde_json::Value) -> JsValue {
    parse_json(&value.to_string()).unwrap()
}
fn subject(id: &str) -> JsValue {
    parsed(
        json!({"namespace":"implementation-review","subject_kind":"revision-bound-finding","stable_subject_id":id,"evidence_baseline":{"owner":"review-episode","reference":"episode-1","revision":"revision-1","integrity_sha256":"0000000000000000000000000000000000000000000000000000000000000000","freshness":"exact immutable revision","status":"verified"},"content_set":["episode-1"]}),
    )
}
fn revision(proposition: &str) -> JsValue {
    parsed(
        json!({"proposition":proposition,"support_qualification":"attributed_review_finding","assumptions":[],"limitations":["bounded"],"confidence":{"label":"high"},"evidence_references":[{"owner":"review-episode","reference":"episode-1","revision":"revision-1","integrity_sha256":"0000000000000000000000000000000000000000000000000000000000000000","freshness":"exact immutable revision","status":"verified"}],"sensitivity_references":[],"evidence_mode":"review_episode_result","judgment_kind":"review_finding","decision_scope":"scope-1","profile_payload":{"finding_id":"f-1","severity":"high","episode":"review-episode@test","outcome":"open"},"reopening_conditions":[],"tombstone":false}),
    )
}
fn admission() -> AdmissionBinding {
    AdmissionBinding {
        campaign_root_id: "campaign-root".into(),
        campaign_revision: "campaign-rev".into(),
        campaign_operation_id: "campaign-op-1".into(),
        obligation_id: "obligation-1".into(),
        candidate_digest: "c".repeat(64),
        selection_digest: "d".repeat(64),
        profile_digest: "e".repeat(64),
        prepared_request_sha256: "a".repeat(64),
        episode_revision: "episode-rev-1".into(),
        result_sha256: "b".repeat(64),
        claims_request_sha256: String::new(),
    }
}
fn finding(operation: &str, mutation: FindingMutation) -> FindingCommand {
    let mut command = FindingCommand {
        operation_id: OperationId::new(operation).unwrap(),
        grant_id: "grant-1".into(),
        admission: admission(),
        mutation,
    };
    command.admission.claims_request_sha256 = claims_request_sha256(&command).unwrap();
    command
}
fn reliance(
    operation: &str,
    claim_id: claim_evidence::ClaimId,
    revision_id: claim_evidence::FindingRevisionId,
) -> RelianceCommand {
    let mut command = RelianceCommand {
        operation_id: OperationId::new(operation).unwrap(),
        grant_id: "grant-1".into(),
        admission: admission(),
        claim_id,
        revision_id,
        consumer: "slice-builder:s12".into(),
        consumer_revision: "builder-eval-1".into(),
        decision_scope: "scope-1".into(),
    };
    command.admission.claims_request_sha256 = reliance_request_sha256(&command).unwrap();
    command
}
fn projection(revision_id: claim_evidence::FindingRevisionId) -> ProjectionRequest {
    let mut request = ProjectionRequest {
        request_id: "projection-1".into(),
        admission: admission(),
        consumer: ProjectionConsumer {
            identity: "slice-builder:s12".into(),
            revision: "builder-eval-1".into(),
            decision_scope: "scope-1".into(),
        },
        selections: vec![ProjectionSelection {
            revision_id,
            selection_reason: JsString::new("reported finding"),
        }],
        limitations: vec![JsString::new("not an acceptance decision")],
    };
    request.admission.claims_request_sha256 = projection_request_sha256(&request).unwrap();
    request
}
fn applied_finding(value: Publication) -> claim_evidence::FindingReceipt {
    match value {
        Publication::Applied(r) => r,
        Publication::Replayed(_) => panic!("unexpected replay"),
    }
}
fn applied_reliance(value: ReliancePublication) -> claim_evidence::RelianceReceipt {
    match value {
        ReliancePublication::Applied(r) => r,
        ReliancePublication::Replayed(_) => panic!("unexpected replay"),
    }
}

#[test]
fn exact_reliance_projection_revision_change_and_restart() {
    let temp = scratch();
    let files = bootstrap_files(
        temp.path(),
        &["create_claim", "publish_revision", "record_reliance"],
    );
    let mut app = ClaimsApplication::initialize(files.clone(), Port).unwrap();
    let first = applied_finding(
        app.publish(finding(
            "create-1",
            FindingMutation::Create {
                subject: subject("episode:f1"),
                statement_identity: JsString::new("title"),
                initial_revision: revision("first"),
            },
        ))
        .unwrap(),
    );
    let relied = applied_reliance(
        app.record_reliance(reliance(
            "rely-1",
            first.claim_id.clone(),
            first.revision_id.clone(),
        ))
        .unwrap(),
    );
    assert_eq!(relied.exact.finding.revision_sha256, first.revision_sha256);
    let mut wrong_locator = relied.locator.clone();
    wrong_locator.action = "publish_revision";
    assert_eq!(
        app.reconcile_reliance(&wrong_locator),
        RelianceReconciliation::Conflicting
    );
    let mut conflicting = reliance("rely-1", first.claim_id.clone(), first.revision_id.clone());
    conflicting.consumer_revision = "builder-eval-2".into();
    conflicting.admission.claims_request_sha256 = reliance_request_sha256(&conflicting).unwrap();
    assert!(matches!(
        app.record_reliance(conflicting),
        Err(WriteError::NoEffect(_))
    ));
    let wrong_claim = reliance(
        "rely-1",
        claim_evidence::ClaimId::new("other-claim").unwrap(),
        first.revision_id.clone(),
    );
    assert!(matches!(
        app.record_reliance(wrong_claim),
        Err(WriteError::NoEffect(_))
    ));
    let request = projection(first.revision_id.clone());
    let context = app.project_exact_revisions(&request).unwrap();
    let projection_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schemas/exact-revision-projection-v1.schema.json"
    ))
    .unwrap();
    let reliance_schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/exact-reliance-ref-v1.schema.json")).unwrap();
    let projected_wire: serde_json::Value = serde_json::from_str(
        &claim_evidence::codec::canonical_json(&context.to_js_value()).unwrap(),
    )
    .unwrap();
    let reliance_wire: serde_json::Value = serde_json::from_str(
        &claim_evidence::codec::canonical_json(&relied.exact.to_js_value()).unwrap(),
    )
    .unwrap();
    assert!(jsonschema::is_valid(&projection_schema, &projected_wire));
    assert!(jsonschema::is_valid(&reliance_schema, &reliance_wire));
    assert!(projected_wire.get("operations").is_none());
    assert!(projected_wire.get("permissions").is_none());
    assert_eq!(projected_wire["authority"]["mutationAuthorized"], false);
    let mut forged = projected_wire.clone();
    forged["authority"]["mutationAuthorized"] = json!(true);
    assert!(!jsonschema::is_valid(&projection_schema, &forged));
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/legacy-v1/reliance-projection.json")).unwrap();
    assert_eq!(oracle["codec"], "claim-evidence-legacy-json-v1");
    assert_eq!(
        first.claim_id.as_str(),
        oracle["claim_id"].as_str().unwrap()
    );
    assert_eq!(
        first.revision_id.as_str(),
        oracle["revision_id"].as_str().unwrap()
    );
    assert_eq!(
        first.revision_sha256,
        oracle["revision_sha256"].as_str().unwrap()
    );
    let same_reliance = reliance("rely-1", first.claim_id.clone(), first.revision_id.clone());
    assert_eq!(
        reliance_request_sha256(&same_reliance).unwrap(),
        oracle["reliance_request_sha256"].as_str().unwrap()
    );
    assert_eq!(
        claim_evidence::reliance_request_canonical(&same_reliance)
            .unwrap()
            .as_bytes(),
        hex(oracle["reliance_request_hex"].as_str().unwrap())
    );
    assert_eq!(
        relied.exact.id.as_str(),
        oracle["reliance_id"].as_str().unwrap()
    );
    assert_eq!(
        relied.exact.sha256,
        oracle["reliance_sha256"].as_str().unwrap()
    );
    assert_eq!(
        context.provenance().canonical_input_sha256,
        oracle["canonical_state_sha256"].as_str().unwrap()
    );
    assert_eq!(
        context.relevant_exact_revisions()[0]
            .exact
            .revision_id
            .as_str(),
        oracle["selected_revision_id"].as_str().unwrap()
    );
    assert_eq!(
        context.relevant_exact_revisions()[0].authority_ref,
        oracle["selected_authority_ref"].as_str().unwrap()
    );
    assert_eq!(context.provenance().build_version, "claim-evidence-rust-v1");
    assert_eq!(
        context.provenance().canonical_input_path,
        "canonical/store.json"
    );
    assert_eq!(
        context.provenance().completeness,
        JsString::new("available")
    );
    assert_eq!(
        context.relevant_exact_revisions()[0].current_reliances[0].id,
        relied.exact.id
    );
    assert_eq!(
        context.relevant_exact_revisions()[0]
            .authority_reference
            .get("owner"),
        Some(&JsValue::text("trusted-host"))
    );
    assert_eq!(
        context.limitations(),
        [JsString::new("not an acceptance decision")]
    );
    app.verify_projection_fresh(&context).unwrap();
    let mut surrogate_request = projection(first.revision_id.clone());
    surrogate_request.selections[0].selection_reason = JsString(vec![0xd800]);
    surrogate_request.limitations = vec![JsString(vec![0xdc00])];
    surrogate_request.admission.claims_request_sha256 =
        projection_request_sha256(&surrogate_request).unwrap();
    let surrogate_context = app.project_exact_revisions(&surrogate_request).unwrap();
    let surrogate_wire = surrogate_context.to_js_value();
    let surrogate_bytes = claim_evidence::codec::canonical_json(&surrogate_wire).unwrap();
    assert!(surrogate_bytes.contains("\\uD800") || surrogate_bytes.contains("\\ud800"));
    assert_eq!(parse_json(&surrogate_bytes).unwrap(), surrogate_wire);
    drop(app);
    let mut app = ClaimsApplication::open(open(&files), Port).unwrap();
    app.verify_projection_fresh(&context).unwrap();
    let replay = app
        .record_reliance(reliance(
            "rely-1",
            first.claim_id.clone(),
            first.revision_id.clone(),
        ))
        .unwrap();
    assert!(matches!(replay,ReliancePublication::Replayed(r) if r==relied));
    let second = applied_finding(
        app.publish(finding(
            "revise-1",
            FindingMutation::Revise {
                claim_id: first.claim_id.clone(),
                predecessor: first.revision_id.clone(),
                revision: revision("second"),
            },
        ))
        .unwrap(),
    );
    assert!(app.verify_projection_fresh(&context).is_err());
    let old = app.project_exact_revisions(&request).unwrap();
    assert!(
        old.relevant_exact_revisions()[0]
            .current_reliances
            .is_empty()
    );
    assert!(matches!(
        app.record_reliance(reliance(
            "rely-1",
            first.claim_id.clone(),
            first.revision_id.clone()
        )),
        Ok(ReliancePublication::Replayed(_))
    ));
    assert_eq!(
        app.reconcile_reliance(&relied.locator),
        RelianceReconciliation::Committed(Box::new(relied.clone()))
    );
    let new_request = projection(second.revision_id.clone());
    assert!(
        app.project_exact_revisions(&new_request)
            .unwrap()
            .relevant_exact_revisions()[0]
            .current_reliances
            .is_empty()
    );
    let stale = reliance(
        "rely-stale",
        first.claim_id.clone(),
        first.revision_id.clone(),
    );
    assert!(matches!(
        app.record_reliance(stale),
        Err(WriteError::NoEffect(_))
    ));
    let new_reliance = applied_reliance(
        app.record_reliance(reliance(
            "rely-2",
            first.claim_id,
            second.revision_id.clone(),
        ))
        .unwrap(),
    );
    assert_eq!(
        app.project_exact_revisions(&new_request)
            .unwrap()
            .relevant_exact_revisions()[0]
            .current_reliances[0]
            .id,
        new_reliance.exact.id
    );
}

#[test]
fn generated_dto_schemas_match_checked_in_contract() {
    for (kind, expected) in [
        (
            "exact-reliance-ref-v1",
            include_str!("../schemas/exact-reliance-ref-v1.schema.json"),
        ),
        (
            "exact-revision-projection-v1",
            include_str!("../schemas/exact-revision-projection-v1.schema.json"),
        ),
    ] {
        assert_eq!(
            claim_evidence::codec::canonical_json(
                &claim_evidence::schema::dto_schema(kind).unwrap()
            )
            .unwrap(),
            expected
        );
    }
    assert!(claim_evidence::schema::dto_schema("unknown").is_err());
}

#[test]
fn exactly_one_hundred_distinct_revisions_are_projectable() {
    let temp = scratch();
    let files = bootstrap_files(temp.path(), &["create_claim", "record_reliance"]);
    let mut app = ClaimsApplication::initialize(files, Port).unwrap();
    let mut selections = Vec::new();
    for index in 0..100 {
        let receipt = applied_finding(
            app.publish(finding(
                &format!("create-{index}"),
                FindingMutation::Create {
                    subject: subject(&format!("episode:f-{index}")),
                    statement_identity: JsString::new("title"),
                    initial_revision: revision("first"),
                },
            ))
            .unwrap(),
        );
        selections.push(ProjectionSelection {
            revision_id: receipt.revision_id,
            selection_reason: JsString::new("selected"),
        });
    }
    let mut request = projection(selections[0].revision_id.clone());
    request.selections = selections;
    request.admission.claims_request_sha256 = projection_request_sha256(&request).unwrap();
    assert_eq!(
        app.project_exact_revisions(&request)
            .unwrap()
            .relevant_exact_revisions()
            .len(),
        100
    );
    request.selections.push(request.selections[0].clone());
    assert!(projection_request_sha256(&request).is_err());
    assert!(app.project_exact_revisions(&request).is_err());
}

#[test]
fn orphaned_reliance_refuses_even_with_rehashed_canonical_state() {
    let temp = scratch();
    let files = bootstrap_files(temp.path(), &["create_claim", "record_reliance"]);
    let mut app = ClaimsApplication::initialize(files.clone(), Port).unwrap();
    let first = applied_finding(
        app.publish(finding(
            "create-1",
            FindingMutation::Create {
                subject: subject("episode:f1"),
                statement_identity: JsString::new("title"),
                initial_revision: revision("first"),
            },
        ))
        .unwrap(),
    );
    app.record_reliance(reliance("rely-1", first.claim_id, first.revision_id))
        .unwrap();
    drop(app);
    let conn = rusqlite::Connection::open(files.root.join("claim-evidence.sqlite3")).unwrap();
    let json: String = conn
        .query_row(
            "SELECT store_json FROM canonical_claim_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut state = parse_json(&json).unwrap();
    let JsValue::Object(ref mut map) = state else {
        panic!("state object");
    };
    let JsValue::Array(reliances) = map.get_mut(&JsString::new("reliances")).unwrap() else {
        panic!("reliance array");
    };
    let mut orphan = reliances[0].as_object().unwrap().clone();
    orphan.remove(&JsString::new("id"));
    orphan.insert(
        JsString::new("operation_id"),
        JsValue::text("orphan-operation"),
    );
    let id = format!(
        "reliance@{}",
        claim_evidence::codec::digest(&JsValue::Object(orphan.clone())).unwrap()
    );
    orphan.insert(JsString::new("id"), JsValue::text(&id));
    reliances.push(JsValue::Object(orphan));
    let canonical = claim_evidence::codec::canonical_json(&state).unwrap();
    let sha = claim_evidence::codec::digest(&state).unwrap();
    conn.execute(
        "UPDATE canonical_claim_state SET store_json=?,store_sha256=? WHERE singleton=1",
        rusqlite::params![canonical, sha],
    )
    .unwrap();
    drop(conn);
    assert!(ClaimsApplication::open(open(&files), Port).is_err());
}

#[test]
fn accepted_ce1_maximum_depth_finding_projects_as_checked_public_dto() {
    let temp = scratch();
    let files = bootstrap_files(temp.path(), &["create_claim", "record_reliance"]);
    let mut app = ClaimsApplication::initialize(files, Port).unwrap();
    let mut payload = revision("near-boundary");
    let mut confidence = JsValue::text("deep");
    for _ in 0..253 {
        confidence = JsValue::Array(vec![confidence]);
    }
    if let JsValue::Object(map) = &mut payload {
        map.insert(JsString::new("confidence"), confidence);
    }
    let finding = applied_finding(
        app.publish(finding(
            "deep-create",
            FindingMutation::Create {
                subject: subject("episode:deep"),
                statement_identity: JsString::new("title"),
                initial_revision: payload,
            },
        ))
        .unwrap(),
    );
    let context = app
        .project_exact_revisions(&projection(finding.revision_id.clone()))
        .unwrap();
    let wire = context.to_js_value();
    let encoded = claim_evidence::codec::canonical_json(&wire).unwrap();
    assert_eq!(parse_json(&encoded).unwrap(), wire);
    assert_eq!(
        wire.get("relevant_exact_revisions")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        wire.get("selection_metadata").unwrap().as_array().unwrap()[0].get("revision_id"),
        Some(&JsValue::text(finding.revision_id.as_str()))
    );
}

#[test]
fn rehashed_reliance_with_rebound_consumer_revision_refuses_checked_readback() {
    let temp = scratch();
    let files = bootstrap_files(temp.path(), &["create_claim", "record_reliance"]);
    let mut app = ClaimsApplication::initialize(files.clone(), Port).unwrap();
    let finding = applied_finding(
        app.publish(finding(
            "create-1",
            FindingMutation::Create {
                subject: subject("episode:f1"),
                statement_identity: JsString::new("title"),
                initial_revision: revision("first"),
            },
        ))
        .unwrap(),
    );
    let receipt = applied_reliance(
        app.record_reliance(reliance("rely-1", finding.claim_id, finding.revision_id))
            .unwrap(),
    );
    let conn = rusqlite::Connection::open(files.root.join("claim-evidence.sqlite3")).unwrap();
    let json: String = conn
        .query_row(
            "SELECT store_json FROM canonical_claim_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut state = parse_json(&json).unwrap();
    let JsValue::Object(ref mut map) = state else {
        panic!("state object");
    };
    let JsValue::Array(reliances) = map.get_mut(&JsString::new("reliances")).unwrap() else {
        panic!("reliance array");
    };
    let JsValue::Object(ref mut record) = reliances[0] else {
        panic!("reliance object");
    };
    record.insert(
        JsString::new("consumer_revision"),
        JsValue::text("different-evaluation"),
    );
    record.remove(&JsString::new("id"));
    let id = format!(
        "reliance@{}",
        claim_evidence::codec::digest(&JsValue::Object(record.clone())).unwrap()
    );
    record.insert(JsString::new("id"), JsValue::text(&id));
    let JsValue::Array(operations) = map.get_mut(&JsString::new("operations")).unwrap() else {
        panic!("operations array");
    };
    let JsValue::Object(ref mut operation) = operations[1] else {
        panic!("operation object");
    };
    operation.insert(JsString::new("result_identity"), JsValue::text(&id));
    let canonical = claim_evidence::codec::canonical_json(&state).unwrap();
    let sha = claim_evidence::codec::digest(&state).unwrap();
    conn.execute(
        "UPDATE canonical_claim_state SET store_json=?,store_sha256=? WHERE singleton=1",
        rusqlite::params![canonical, sha],
    )
    .unwrap();
    drop(conn);
    assert!(matches!(
        app.reconcile_reliance(&receipt.locator),
        RelianceReconciliation::Unresolved(_)
    ));
    drop(app);
    assert!(ClaimsApplication::open(open(&files), Port).is_err());
}

#[test]
fn published_bootstrap_schema_admits_exact_ce2_permission_set() {
    let temp = scratch();
    let files = bootstrap_files(temp.path(), &["create_claim", "record_reliance"]);
    let config: serde_json::Value =
        serde_json::from_slice(&fs::read(&files.config).unwrap()).unwrap();
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/bootstrap-config-v1.schema.json")).unwrap();
    assert!(jsonschema::is_valid(&schema, &config));
    ClaimsApplication::initialize(files, Port).unwrap();
    let mut unsupported = config;
    unsupported["grants"][0]["permissions"] = json!(["create_claim", "retire_reliance"]);
    assert!(!jsonschema::is_valid(&schema, &unsupported));
}

#[cfg(feature = "test-faults")]
#[test]
fn ce2_fault_child() {
    let Ok(base) = std::env::var("CLAIM_EVIDENCE_CE2_CHILD_BASE") else {
        return;
    };
    let base = Path::new(&base);
    let config = base.join("config.json");
    let source = base.join("source.json");
    let files = OpenFiles {
        root: base.join("root"),
        config: config.clone(),
        source: source.clone(),
        expected_config_sha256: claim_evidence::sha256_bytes(&fs::read(config).unwrap()),
        expected_source_sha256: claim_evidence::sha256_bytes(&fs::read(source).unwrap()),
    };
    let mut app = ClaimsApplication::open(files, Port).unwrap();
    let claim_id = claim_evidence::stable_claim_id(&subject("episode:f1")).unwrap();
    let first = app.project_exact_revisions(&projection_from_env()).unwrap();
    let revision_id = first.relevant_exact_revisions()[0]
        .exact
        .revision_id
        .clone();
    let _ = app
        .record_reliance(reliance("rely-crash", claim_id, revision_id))
        .unwrap();
}

#[cfg(feature = "test-faults")]
fn projection_from_env() -> ProjectionRequest {
    projection(
        claim_evidence::FindingRevisionId::new(
            std::env::var("CLAIM_EVIDENCE_CE2_REVISION").unwrap(),
        )
        .unwrap(),
    )
}

#[cfg(feature = "test-faults")]
#[test]
fn committed_reliance_survives_actual_process_kill_before_delivery() {
    let temp = scratch();
    let files = bootstrap_files(
        temp.path(),
        &["create_claim", "publish_revision", "record_reliance"],
    );
    let mut app = ClaimsApplication::initialize(files.clone(), Port).unwrap();
    let first = applied_finding(
        app.publish(finding(
            "create-1",
            FindingMutation::Create {
                subject: subject("episode:f1"),
                statement_identity: JsString::new("title"),
                initial_revision: revision("first"),
            },
        ))
        .unwrap(),
    );
    let command = reliance(
        "rely-crash",
        first.claim_id.clone(),
        first.revision_id.clone(),
    );
    let locator = app.reliance_locator(&command).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "ce2_fault_child", "--nocapture"])
        .env("CLAIM_EVIDENCE_CE2_CHILD_BASE", temp.path())
        .env("CLAIM_EVIDENCE_CE2_REVISION", first.revision_id.as_str())
        .env("CLAIM_EVIDENCE_FAULT_CUT", "after_reliance_commit")
        .env("CLAIM_EVIDENCE_FAULT_DIR", temp.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let ready = temp.path().join("after_reliance_commit.ready");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() && Instant::now() < deadline {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("child exited before fault cut: {status}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ready.exists(), "child never reached committed cut");
    child.kill().unwrap();
    child.wait().unwrap();
    drop(app);
    let mut recovered = ClaimsApplication::open(open(&files), Port).unwrap();
    let exact = match recovered.reconcile_reliance(&locator) {
        RelianceReconciliation::Committed(v) => v,
        other => panic!("unexpected reconciliation: {other:?}"),
    };
    assert_eq!(exact.exact.finding.revision_id, first.revision_id);
    assert!(matches!(
        recovered.record_reliance(command),
        Ok(ReliancePublication::Replayed(_))
    ));
}

#[test]
fn reliance_authority_conflicts_and_projection_selection_bounds() {
    let temp = scratch();
    let files = bootstrap_files(temp.path(), &["create_claim", "publish_revision"]);
    let mut app = ClaimsApplication::initialize(files.clone(), Port).unwrap();
    let first = applied_finding(
        app.publish(finding(
            "create-1",
            FindingMutation::Create {
                subject: subject("episode:f1"),
                statement_identity: JsString::new("title"),
                initial_revision: revision("first"),
            },
        ))
        .unwrap(),
    );
    assert!(matches!(
        app.record_reliance(reliance(
            "denied",
            first.claim_id.clone(),
            first.revision_id.clone()
        )),
        Err(WriteError::NoEffect(_))
    ));
    drop(app);
    // A registered grant cannot be changed on reopen to add reliance permission.
    let original_config = fs::read(&files.config).unwrap();
    let more = bootstrap_files(
        temp.path(),
        &["create_claim", "publish_revision", "record_reliance"],
    );
    assert!(ClaimsApplication::open(open(&more), Port).is_err());
    fs::write(&files.config, original_config).unwrap();
    let app = ClaimsApplication::open(open(&files), Port).unwrap();
    let request = projection(first.revision_id.clone());
    assert!(app.project_exact_revisions(&request).is_ok());
    let mut duplicate = projection(first.revision_id.clone());
    duplicate.selections.push(duplicate.selections[0].clone());
    assert!(projection_request_sha256(&duplicate).is_err());
    assert!(app.project_exact_revisions(&duplicate).is_err());
    let mut absent = projection(first.revision_id.clone());
    absent.selections.clear();
    assert!(app.project_exact_revisions(&absent).is_err());
    let mut overflow = projection(first.revision_id.clone());
    overflow.selections = (0..101)
        .map(|i| ProjectionSelection {
            revision_id: claim_evidence::FindingRevisionId::new(format!("unknown-{i}")).unwrap(),
            selection_reason: JsString::new("reason"),
        })
        .collect();
    assert!(app.project_exact_revisions(&overflow).is_err());
    let mut unknown = projection(claim_evidence::FindingRevisionId::new("unknown").unwrap());
    unknown.admission.claims_request_sha256 = projection_request_sha256(&unknown).unwrap();
    assert!(app.project_exact_revisions(&unknown).is_err());
    let mut wrong_scope = projection(first.revision_id);
    wrong_scope.consumer.decision_scope = "other".into();
    wrong_scope.admission.claims_request_sha256 = projection_request_sha256(&wrong_scope).unwrap();
    assert!(app.project_exact_revisions(&wrong_scope).is_err());
}
