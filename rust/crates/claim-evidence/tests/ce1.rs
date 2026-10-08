use claim_evidence::codec::{JsValue, canonical_json, digest, parse_json, validate_transport};
use claim_evidence::{
    AdmissionBinding, BootstrapFiles, ClaimsApplication, FINDING_REQUEST_MAX_BYTES,
    FindingAdmissionPort, FindingCommand, FindingLease, FindingMutation, OpenFiles, OperationId,
    Publication, Reconciliation, WriteError, claims_request_canonical, claims_request_sha256,
};
use serde_json::json;
use std::{fs, path::Path};
#[cfg(feature = "test-faults")]
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    thread,
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

fn hex(bytes: &str) -> Vec<u8> {
    bytes
        .as_bytes()
        .chunks_exact(2)
        .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
        .collect()
}
fn parsed(value: serde_json::Value) -> JsValue {
    parse_json(&value.to_string()).unwrap()
}
fn scratch() -> tempfile::TempDir {
    let path = std::env::var("TMPDIR").expect("TMPDIR is explicitly configured");
    tempfile::Builder::new()
        .prefix("claim-evidence-ce1-")
        .tempdir_in(path)
        .unwrap()
}

fn files(base: &Path) -> BootstrapFiles {
    let source = base.join("source.json");
    let config = base.join("config.json");
    let source_bytes=json!({"schema_version":1,"owner":"trusted-host","reference":"source-1","revision":"r1","freshness":"exact immutable revision","profile":"native-review-claims-v1","actors":["reviewer"],"decision_scopes":["scope-1"],"grant_ids":["grant-1"]}).to_string();
    fs::write(&source, &source_bytes).unwrap();
    let source_sha = claim_evidence::sha256_bytes(source_bytes.as_bytes());
    let config_bytes=json!({"schema_version":1,"profile":"native-review-claims-v1","root_id":"root-1","grants":[{"schema_version":1,"grant_id":"grant-1","actor":"reviewer","profile":"revision-bound-review-finding-v1","permissions":["create_claim","publish_revision"],"decision_scope":"scope-1","authority_reference":{"owner":"trusted-host","reference":"source-1","revision":"r1","integrity_sha256":source_sha,"freshness":"exact immutable revision","status":"verified"}}]}).to_string();
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
fn subject() -> JsValue {
    parsed(
        json!({"namespace":"implementation-review","subject_kind":"revision-bound-finding","stable_subject_id":"review-episode@test:f-1","evidence_baseline":{"owner":"review-episode","reference":"episode-1","revision":"revision-1","integrity_sha256":"0000000000000000000000000000000000000000000000000000000000000000","freshness":"exact immutable revision","status":"verified"},"content_set":["episode-1"]}),
    )
}
fn revision(proposition: &str) -> JsValue {
    parsed(
        json!({"proposition":proposition,"support_qualification":"attributed_review_finding","assumptions":[],"limitations":[],"confidence":{"label":"high"},"evidence_references":[{"owner":"review-episode","reference":"episode-1","revision":"revision-1","integrity_sha256":"0000000000000000000000000000000000000000000000000000000000000000","freshness":"exact immutable revision","status":"verified"}],"sensitivity_references":[],"evidence_mode":"review_episode_result","judgment_kind":"review_finding","decision_scope":"scope-1","profile_payload":{"finding_id":"f-1","severity":"high","episode":"review-episode@test","outcome":"open"},"reopening_conditions":[],"tombstone":false}),
    )
}
fn command(operation: &str, mutation: FindingMutation) -> FindingCommand {
    let mut c = FindingCommand {
        operation_id: OperationId::new(operation).unwrap(),
        grant_id: "grant-1".into(),
        admission: AdmissionBinding {
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
        },
        mutation,
    };
    c.admission.claims_request_sha256 = claims_request_sha256(&c).unwrap();
    c
}
fn create() -> FindingCommand {
    command(
        "finding-op-1",
        FindingMutation::Create {
            subject: subject(),
            statement_identity: "title".into(),
            initial_revision: revision("first"),
        },
    )
}

#[test]
fn legacy_codec_vectors() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/legacy-v1/codec-vectors.json")).unwrap();
    assert_eq!(fixture["codec"], "claim-evidence-legacy-json-v1");
    for vector in fixture["vectors"].as_array().unwrap() {
        let input = String::from_utf8(hex(vector["input_hex"].as_str().unwrap())).unwrap();
        let value = parse_json(&input).unwrap();
        validate_transport(&value).unwrap();
        assert_eq!(
            canonical_json(&value).unwrap().as_bytes(),
            hex(vector["canonical_hex"].as_str().unwrap()),
            "{}",
            vector["name"]
        );
        assert_eq!(digest(&value).unwrap(), vector["sha256"].as_str().unwrap());
    }
    let subject =
        parse_json(&String::from_utf8(hex(fixture["subject_hex"].as_str().unwrap())).unwrap())
            .unwrap();
    assert_eq!(
        claim_evidence::stable_claim_id(&subject).unwrap().as_str(),
        fixture["claim_id"].as_str().unwrap()
    );
    let revision =
        parse_json(&String::from_utf8(hex(fixture["revision_hex"].as_str().unwrap())).unwrap())
            .unwrap();
    let mut map = revision.as_object().unwrap().clone();
    map.remove(&claim_evidence::codec::JsString::new("id"));
    assert_eq!(
        format!(
            "{}@{}",
            fixture["claim_id"].as_str().unwrap(),
            digest(&JsValue::Object(map)).unwrap()
        ),
        fixture["revision_id"].as_str().unwrap()
    );
    assert!(validate_transport(&parse_json("9007199254740992").unwrap()).is_err());
    assert!(parse_json("1e999").is_err());
}

#[test]
fn unsafe_direct_scalars_never_receive_identity_bytes() {
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        9_007_199_254_740_992.0,
    ] {
        let scalar = JsValue::Number(value);
        assert!(canonical_json(&scalar).is_err());
        assert!(digest(&scalar).is_err());
        let mut invalid = revision("first");
        if let JsValue::Object(map) = &mut invalid {
            map.insert("confidence".into(), scalar);
        }
        let c = command_unchecked(
            "unsafe-request",
            FindingMutation::Create {
                subject: subject(),
                statement_identity: "title".into(),
                initial_revision: invalid,
            },
        );
        assert!(claims_request_canonical(&c).is_err());
        assert!(claims_request_sha256(&c).is_err());
    }
    assert_eq!(canonical_json(&JsValue::Number(-0.0)).unwrap(), "0\n");
    assert_ne!(
        digest(&JsValue::Null).unwrap(),
        digest(&JsValue::Number(-0.0)).unwrap()
    );
}

#[test]
fn legacy_unpaired_surrogate_finding_round_trips() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/legacy-v1/surrogate-finding.json")).unwrap();
    let subject =
        parse_json(&String::from_utf8(hex(fixture["subject_hex"].as_str().unwrap())).unwrap())
            .unwrap();
    let mut direct_subject = subject.clone();
    if let JsValue::Object(map) = &mut direct_subject {
        let mut units = "review-episode@test:".encode_utf16().collect::<Vec<_>>();
        units.push(0xd800);
        map.insert(
            "stable_subject_id".into(),
            JsValue::String(claim_evidence::codec::JsString(units)),
        );
    }
    assert_eq!(direct_subject, subject);
    let payload =
        parse_json(&String::from_utf8(hex(fixture["payload_hex"].as_str().unwrap())).unwrap())
            .unwrap();
    let statement =
        parse_json(&String::from_utf8(hex(fixture["statement_hex"].as_str().unwrap())).unwrap())
            .unwrap()
            .as_text()
            .unwrap()
            .clone();
    assert_eq!(
        claim_evidence::stable_claim_id(&subject).unwrap().as_str(),
        fixture["claim_id"].as_str().unwrap()
    );
    let tmp = scratch();
    let f = files(tmp.path());
    let mut app = ClaimsApplication::initialize(f.clone(), Port).unwrap();
    let c = command(
        "surrogate-finding-op",
        FindingMutation::Create {
            subject: subject.clone(),
            statement_identity: statement.clone(),
            initial_revision: payload,
        },
    );
    let receipt = match app.publish(c.clone()).unwrap() {
        Publication::Applied(r) => r,
        _ => panic!(),
    };
    assert_eq!(
        receipt.claim_id.as_str(),
        fixture["claim_id"].as_str().unwrap()
    );
    assert_eq!(
        receipt.revision_id.as_str(),
        fixture["revision_id"].as_str().unwrap()
    );
    assert_eq!(
        receipt.revision_sha256,
        fixture["revision_sha256"].as_str().unwrap()
    );
    let conn = rusqlite::Connection::open(f.root.join("claim-evidence.sqlite3")).unwrap();
    let state_text: String = conn
        .query_row(
            "SELECT store_json FROM canonical_claim_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let state = parse_json(&state_text).unwrap();
    let claim = &state.get("claims").unwrap().as_array().unwrap()[0];
    assert_eq!(claim.get("subject"), Some(&subject));
    assert_eq!(
        claim.get("statement_identity"),
        Some(&JsValue::String(statement))
    );
    let revision = &state.get("revisions").unwrap().as_array().unwrap()[0];
    assert_eq!(
        canonical_json(revision).unwrap().as_bytes(),
        hex(fixture["canonical_revision_hex"].as_str().unwrap())
    );
    drop(app);
    drop(conn);
    let mut reopened = ClaimsApplication::open(open(&f), Port).unwrap();
    assert!(matches!(reopened.publish(c), Ok(Publication::Replayed(_))));
}

fn nested_confidence(depth: usize) -> JsValue {
    let mut value = JsValue::text("deep");
    for _ in 0..depth {
        value = JsValue::Array(vec![value]);
    }
    value
}
fn command_unchecked(operation: &str, mutation: FindingMutation) -> FindingCommand {
    FindingCommand {
        operation_id: OperationId::new(operation).unwrap(),
        grant_id: "grant-1".into(),
        admission: AdmissionBinding {
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
            claims_request_sha256: "f".repeat(64),
        },
        mutation,
    }
}
#[test]
fn nested_confidence_refuses_before_commit_and_root_remains_readable() {
    let tmp = scratch();
    let f = files(tmp.path());
    let mut app = ClaimsApplication::initialize(f.clone(), Port).unwrap();
    for (name, value) in [
        ("direct", nested_confidence(254)),
        (
            "parsed",
            parse_json(&format!("{}\"deep\"{}", "[".repeat(254), "]".repeat(254))).unwrap(),
        ),
    ] {
        let mut payload = revision("first");
        if let JsValue::Object(map) = &mut payload {
            map.insert("confidence".into(), value);
        }
        assert!(canonical_json(&payload).is_ok());
        let c = command_unchecked(
            name,
            FindingMutation::Create {
                subject: subject(),
                statement_identity: "title".into(),
                initial_revision: payload,
            },
        );
        assert!(claims_request_sha256(&c).is_err());
        assert!(matches!(app.publish(c), Err(WriteError::NoEffect(_))));
    }
    let conn = rusqlite::Connection::open(f.root.join("claim-evidence.sqlite3")).unwrap();
    let store_revision: i64 = conn
        .query_row(
            "SELECT store_revision FROM canonical_claim_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(store_revision, 1);
    drop(conn);
    let mut near_payload = revision("near-boundary");
    if let JsValue::Object(map) = &mut near_payload {
        map.insert("confidence".into(), nested_confidence(253));
    }
    let near = command(
        "near-boundary",
        FindingMutation::Create {
            subject: subject(),
            statement_identity: "title".into(),
            initial_revision: near_payload,
        },
    );
    assert!(matches!(
        app.publish(near.clone()),
        Ok(Publication::Applied(_))
    ));
    drop(app);
    let mut reopened = ClaimsApplication::open(open(&f), Port).unwrap();
    assert!(matches!(
        reopened.publish(near),
        Ok(Publication::Replayed(_))
    ));
}

#[test]
fn root_grants_replay_cas_reopen_and_fencing() {
    let tmp = scratch();
    let f = files(tmp.path());
    let mut app = ClaimsApplication::initialize(f.clone(), Port).unwrap();
    assert!(ClaimsApplication::initialize(f.clone(), Port).is_err());
    let first = create();
    let locator = app.locator(&first).unwrap();
    assert_eq!(app.reconcile(&locator), Reconciliation::Absent);
    let receipt = match app.publish(first.clone()).unwrap() {
        Publication::Applied(r) => r,
        _ => panic!(),
    };
    assert_eq!(
        app.reconcile(&locator),
        Reconciliation::Committed(Box::new(receipt.clone()))
    );
    assert!(matches!(
        app.publish(first.clone()),
        Ok(Publication::Replayed(_))
    ));
    let mut conflict = first.clone();
    conflict.mutation = FindingMutation::Create {
        subject: subject(),
        statement_identity: "changed".into(),
        initial_revision: revision("first"),
    };
    conflict.admission.claims_request_sha256 = claims_request_sha256(&conflict).unwrap();
    assert!(matches!(
        app.publish(conflict),
        Err(WriteError::NoEffect(_))
    ));
    let mut bad_grant = first.clone();
    bad_grant.grant_id = "unregistered".into();
    assert!(matches!(
        app.publish(bad_grant),
        Err(WriteError::NoEffect(_))
    ));
    let mut bad_admission = first.clone();
    bad_admission.admission.selection_digest = "f".repeat(64);
    assert!(matches!(
        app.publish(bad_admission),
        Err(WriteError::NoEffect(_))
    ));
    let mut bad_scope = command(
        "scope-op",
        FindingMutation::Revise {
            claim_id: receipt.claim_id.clone(),
            predecessor: receipt.revision_id.clone(),
            revision: revision("changed"),
        },
    );
    if let FindingMutation::Revise {
        revision: JsValue::Object(map),
        ..
    } = &mut bad_scope.mutation
    {
        map.insert("decision_scope".into(), JsValue::text("other"));
    }
    bad_scope.admission.claims_request_sha256 = claims_request_sha256(&bad_scope).unwrap();
    assert!(matches!(
        app.publish(bad_scope),
        Err(WriteError::NoEffect(_))
    ));
    let revise = command(
        "revise-op",
        FindingMutation::Revise {
            claim_id: receipt.claim_id.clone(),
            predecessor: receipt.revision_id.clone(),
            revision: revision("changed"),
        },
    );
    let revised = match app.publish(revise.clone()).unwrap() {
        Publication::Applied(r) => r,
        _ => panic!(),
    };
    assert_ne!(receipt.revision_id, revised.revision_id);
    let stale = command(
        "stale-op",
        FindingMutation::Revise {
            claim_id: receipt.claim_id.clone(),
            predecessor: receipt.revision_id.clone(),
            revision: revision("stale"),
        },
    );
    assert!(matches!(app.publish(stale), Err(WriteError::NoEffect(_))));
    let mut competitor = ClaimsApplication::open(open(&f), Port).unwrap();
    let competing = command(
        "competing-op",
        FindingMutation::Revise {
            claim_id: receipt.claim_id.clone(),
            predecessor: revised.revision_id.clone(),
            revision: revision("next"),
        },
    );
    let competing_receipt = competitor.publish(competing).unwrap();
    assert!(matches!(competing_receipt, Publication::Applied(_)));
    let other = command(
        "other-op",
        FindingMutation::Revise {
            claim_id: receipt.claim_id.clone(),
            predecessor: revised.revision_id.clone(),
            revision: revision("other"),
        },
    );
    assert!(matches!(app.publish(other), Err(WriteError::NoEffect(_))));
    drop(competitor);
    drop(app);
    let mut reopened = ClaimsApplication::open(open(&f), Port).unwrap();
    assert!(matches!(
        reopened.publish(revise),
        Ok(Publication::Replayed(_))
    ));
    assert_eq!(
        reopened.reconcile(&locator),
        Reconciliation::Committed(Box::new(receipt))
    );
    let wrong = OpenFiles {
        expected_source_sha256: "0".repeat(64),
        ..open(&f)
    };
    assert!(ClaimsApplication::open(wrong, Port).is_err());
    fs::write(f.root.join("claim-evidence-root.json"), "{}").unwrap();
    assert!(matches!(
        reopened.reconcile(&locator),
        Reconciliation::Unresolved(_)
    ));
}

#[test]
fn exact_request_capacity_and_corrupt_state() {
    let tmp = scratch();
    let f = files(tmp.path());
    let mut app = ClaimsApplication::initialize(f.clone(), Port).unwrap();
    let make = |operation: &str, count: usize| {
        command(
            operation,
            FindingMutation::Create {
                subject: subject(),
                statement_identity: "title".into(),
                initial_revision: revision(&"x".repeat(count)),
            },
        )
    };
    let base = claims_request_canonical(&make("capacity", 0))
        .unwrap()
        .len();
    let at_limit = make("capacity", FINDING_REQUEST_MAX_BYTES - base);
    assert_eq!(
        claims_request_canonical(&at_limit).unwrap().len(),
        FINDING_REQUEST_MAX_BYTES
    );
    assert!(matches!(app.publish(at_limit), Ok(Publication::Applied(_))));
    let base_over = claims_request_canonical(&make("over-capacity", 0))
        .unwrap()
        .len();
    let over = make("over-capacity", FINDING_REQUEST_MAX_BYTES - base_over + 1);
    assert_eq!(
        claims_request_canonical(&over).unwrap().len(),
        FINDING_REQUEST_MAX_BYTES + 1
    );
    assert!(matches!(app.publish(over), Err(WriteError::NoEffect(_))));
    let locator = app.locator(&create()).unwrap();
    let conn = rusqlite::Connection::open(f.root.join("claim-evidence.sqlite3")).unwrap();
    conn.execute(
        "UPDATE canonical_claim_state SET store_sha256=? WHERE singleton=1",
        ["f".repeat(64)],
    )
    .unwrap();
    assert!(matches!(
        app.reconcile(&locator),
        Reconciliation::Unresolved(_)
    ));
}

#[test]
fn bootstrap_profile_and_semantic_integrity_refuse() {
    let tmp = scratch();
    let mut f = files(tmp.path());
    let original = fs::read_to_string(&f.config).unwrap();
    let invalid = original.replace("native-review-claims-v1", "other-profile");
    fs::write(&f.config, &invalid).unwrap();
    f.expected_config_sha256 = claim_evidence::sha256_bytes(invalid.as_bytes());
    assert!(ClaimsApplication::initialize(f.clone(), Port).is_err());
    assert!(!f.root.exists());
    fs::write(&f.config, &original).unwrap();
    f.expected_config_sha256 = claim_evidence::sha256_bytes(original.as_bytes());
    let mut app = ClaimsApplication::initialize(f.clone(), Port).unwrap();
    let first = create();
    let receipt = match app.publish(first.clone()).unwrap() {
        Publication::Applied(r) => r,
        _ => panic!(),
    };
    let source = fs::read(&f.source).unwrap();
    fs::write(&f.source, b"{}\n").unwrap();
    assert!(matches!(
        app.publish(first.clone()),
        Err(WriteError::NoEffect(_))
    ));
    fs::write(&f.source, source).unwrap();
    let conn = rusqlite::Connection::open(f.root.join("claim-evidence.sqlite3")).unwrap();
    let current: String = conn
        .query_row(
            "SELECT store_json FROM canonical_claim_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut state = parse_json(&current).unwrap();
    if let JsValue::Object(map) = &mut state
        && let Some(JsValue::Array(revisions)) = map.get_mut(&"revisions".into())
        && let JsValue::Object(revision) = &mut revisions[0]
    {
        revision.insert("authority_ref".into(), JsValue::text("missing-grant"));
    }
    let altered = canonical_json(&state).unwrap();
    conn.execute(
        "UPDATE canonical_claim_state SET store_json=?,store_sha256=? WHERE singleton=1",
        [altered, digest(&state).unwrap()],
    )
    .unwrap();
    let locator = app.locator(&first).unwrap();
    assert!(matches!(
        app.reconcile(&locator),
        Reconciliation::Unresolved(_)
    ));
    drop(app);
    assert!(ClaimsApplication::open(open(&f), Port).is_err());
    assert!(!receipt.revision_id.as_str().is_empty());
}

#[test]
fn unexpected_sqlite_schema_refuses_on_reopen() {
    let tmp = scratch();
    let f = files(tmp.path());
    let app = ClaimsApplication::initialize(f.clone(), Port).unwrap();
    drop(app);
    let conn = rusqlite::Connection::open(f.root.join("claim-evidence.sqlite3")).unwrap();
    conn.execute_batch("CREATE TABLE rogue(value TEXT) STRICT")
        .unwrap();
    drop(conn);
    assert!(ClaimsApplication::open(open(&f), Port).is_err());
}

#[cfg(feature = "test-faults")]
#[test]
fn crash_child() {
    let Ok(base) = std::env::var("CLAIM_EVIDENCE_CHILD_BASE") else {
        return;
    };
    let base = PathBuf::from(base);
    let f = files(&base);
    let mut app = ClaimsApplication::open(open(&f), Port).unwrap();
    let _ = app.publish(create());
}

#[cfg(feature = "test-faults")]
#[test]
fn init_child() {
    let Ok(base) = std::env::var("CLAIM_EVIDENCE_INIT_BASE") else {
        return;
    };
    let f = files(Path::new(&base));
    let _ = ClaimsApplication::initialize(f, Port);
}

#[cfg(feature = "test-faults")]
#[test]
fn lock_child() {
    let Ok(base) = std::env::var("CLAIM_EVIDENCE_LOCK_BASE") else {
        return;
    };
    let base = PathBuf::from(base);
    let conn = rusqlite::Connection::open(base.join("root/claim-evidence.sqlite3")).unwrap();
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    fs::write(base.join("lock.ready"), "ready").unwrap();
    while !base.join("lock.release").exists() {
        thread::sleep(Duration::from_millis(10));
    }
    conn.execute_batch("ROLLBACK").unwrap();
}

#[cfg(feature = "test-faults")]
fn wait_ready(path: &Path) {
    let start = Instant::now();
    while !path.exists() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "child did not reach cut"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(feature = "test-faults")]
#[test]
fn actual_process_commit_cuts() {
    for (cut, expected) in [("before_commit", false), ("after_commit", true)] {
        let tmp = scratch();
        let f = files(tmp.path());
        let app = ClaimsApplication::initialize(f.clone(), Port).unwrap();
        let locator = app.locator(&create()).unwrap();
        drop(app);
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("crash_child")
            .arg("--nocapture")
            .env("CLAIM_EVIDENCE_CHILD_BASE", tmp.path())
            .env("CLAIM_EVIDENCE_FAULT_CUT", cut)
            .env("CLAIM_EVIDENCE_FAULT_DIR", tmp.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let ready = tmp.path().join(format!("{cut}.ready"));
        wait_ready(&ready);
        child.kill().unwrap();
        child.wait().unwrap();
        let reopened = ClaimsApplication::open(open(&f), Port).unwrap();
        assert_eq!(
            matches!(reopened.reconcile(&locator), Reconciliation::Committed(_)),
            expected
        );
        if !expected {
            assert_eq!(reopened.reconcile(&locator), Reconciliation::Absent);
        }
    }
}

#[cfg(feature = "test-faults")]
#[test]
fn actual_process_root_init_cuts_and_busy_writer() {
    for (cut, committed) in [("before_root_commit", false), ("after_root_commit", true)] {
        let tmp = scratch();
        let f = files(tmp.path());
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("init_child")
            .arg("--nocapture")
            .env("CLAIM_EVIDENCE_INIT_BASE", tmp.path())
            .env("CLAIM_EVIDENCE_FAULT_CUT", cut)
            .env("CLAIM_EVIDENCE_FAULT_DIR", tmp.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        wait_ready(&tmp.path().join(format!("{cut}.ready")));
        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(ClaimsApplication::open(open(&f), Port).is_ok(), committed);
    }
    let tmp = scratch();
    let f = files(tmp.path());
    let mut app = ClaimsApplication::initialize(f.clone(), Port).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("lock_child")
        .arg("--nocapture")
        .env("CLAIM_EVIDENCE_LOCK_BASE", tmp.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_ready(&tmp.path().join("lock.ready"));
    assert!(matches!(
        app.publish(create()),
        Err(WriteError::NoEffect(_))
    ));
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(matches!(app.publish(create()), Ok(Publication::Applied(_))));
}
