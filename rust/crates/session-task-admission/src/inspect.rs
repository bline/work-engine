//! Deterministic source composition. Successful evidence collection still does
//! not establish exclusive authority or authorize any native or SQLite effect.
use crate::{
    Ref, Row,
    accounting::Observation,
    proof::{self, Assertion, KeyPolicy},
    request::AdmissionRequest,
    source::{self, Pin, Program},
};
use rusqlite::Connection;
use std::path::PathBuf;
use time::OffsetDateTime;

/// All values are protected operator startup configuration, never MCP input.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub(crate) struct Sources {
    pub board: Program,
    pub board_repository: PathBuf,
    pub key_policy: PathBuf,
    pub owner: String,
    pub rp_id: String,
    pub origin: String,
    pub actor: String,
    pub ledger: Program,
    pub budget_record: Pin,
    #[serde(default)]
    pub canonical_budget: Option<CanonicalBudgetConfig>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CanonicalBudgetConfig {
    pub contract: String,
    pub scope: Program,
}
#[allow(dead_code)]
pub(crate) struct Evidence {
    pub task: Ref,
    pub decision: Ref,
    pub board_revision: String,
    pub budget: Observation,
    pub budget_record_sha256: String,
    pub board: crate::board::CompleteBoard,
    pub original: String,
    pub answer: String,
    pub assertion: OwnedAssertion,
    pub verified: proof::VerifiedAnswer,
}
pub(crate) struct OwnedAssertion {
    pub credential_id: String,
    pub authenticator_data: String,
    pub client_data_json: String,
    pub signature: String,
}
impl OwnedAssertion {
    pub fn as_assertion(&self) -> Assertion<'_> {
        Assertion {
            credential_id: &self.credential_id,
            authenticator_data: &self.authenticator_data,
            client_data_json: &self.client_data_json,
            signature: &self.signature,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Held(pub &'static str);
/// Performs real board, pinned key, registry and canonical budget reads. It
/// deliberately returns evidence, not an admission: exclusive-controller
/// qualification, mode freshness and action-bound rechecks remain required.
#[allow(dead_code)]
pub(crate) fn collect(
    config: &Sources,
    db: &Connection,
    raw_request: &str,
    now: OffsetDateTime,
) -> Result<Evidence, Held> {
    collect_inner(config, db, raw_request, now, &config.budget_record)
}
pub(crate) fn collect_current(
    config: &Sources,
    db: &Connection,
    raw_request: &str,
    now: OffsetDateTime,
    source: &crate::canonical_source::CanonicalSource,
) -> Result<Evidence, Held> {
    #[allow(unused_mut)]
    let mut clock = source::AcquisitionTime::new(now);
    let contract = config
        .canonical_budget
        .as_ref()
        .ok_or(Held("budget_source_contract_missing"))?;
    if contract.contract != "canonical-envelope-source/v1"
        || !source.matches(&config.budget_record, &contract.scope, &config.ledger)
    {
        return Err(Held("budget_source_contract_mismatch"));
    }
    source.check_current().map_err(Held)?;
    #[cfg(test)]
    if let Some(elapsed) = source.check_elapsed.take() {
        clock.advance(elapsed);
    }
    collect_inner(config, db, raw_request, clock.now(), source.current())
}
fn collect_inner(
    config: &Sources,
    db: &Connection,
    raw_request: &str,
    now: OffsetDateTime,
    record: &Pin,
) -> Result<Evidence, Held> {
    let clock = source::AcquisitionTime::new(now);
    let request: AdmissionRequest =
        serde_json::from_str(raw_request).map_err(|_| Held("invalid_request"))?;
    let sequence = request
        .decision
        .revision
        .parse::<u64>()
        .map_err(|_| Held("decision_ref"))?;
    if sequence.to_string() != request.decision.revision {
        return Err(Held("decision_ref"));
    }
    let board =
        source::board(&config.board, &config.board_repository).ok_or(Held("board_unavailable"))?;
    let reply = board
        .rows
        .iter()
        .find(|raw| {
            serde_json::from_str::<serde_json::Value>(raw)
                .ok()
                .and_then(|v| v.get("sequence").and_then(|x| x.as_u64()))
                == Some(sequence)
        })
        .ok_or(Held("decision_missing"))?;
    let row: serde_json::Value =
        serde_json::from_str(reply).map_err(|_| Held("decision_invalid"))?;
    let request_sequence = row
        .get("in_reply_to")
        .and_then(|v| v.as_u64())
        .ok_or(Held("decision_invalid"))?;
    let original = board
        .rows
        .iter()
        .find(|raw| {
            serde_json::from_str::<serde_json::Value>(raw)
                .ok()
                .and_then(|v| v.get("sequence").and_then(|x| x.as_u64()))
                == Some(request_sequence)
        })
        .ok_or(Held("request_missing"))?;
    let task = crate::validate_review(original).map_err(|_| Held("unreviewable"))?;
    let decision = Ref {
        id: row
            .get("message_id")
            .and_then(|v| v.as_str())
            .ok_or(Held("decision_invalid"))?
            .to_owned(),
        revision: sequence.to_string(),
        sha256: crate::hash(reply),
    };
    AdmissionRequest::parse(raw_request, &task, &decision).map_err(|_| Held("stale_ref"))?;
    let signed = row
        .pointer("/data/signed_answer")
        .ok_or(Held("proof_missing"))?;
    let field = |name: &str| {
        signed
            .get(name)
            .and_then(|v| v.as_str())
            .ok_or(Held("proof_missing"))
    };
    let answer = field("answer_json")?;
    let assertion = Assertion {
        credential_id: field("credential_id")?,
        authenticator_data: field("authenticator_data")?,
        client_data_json: field("client_data_json")?,
        signature: field("signature")?,
    };
    let key = crate::key_policy::load(&config.key_policy, assertion.credential_id)
        .map_err(|_| Held("key_unavailable"))?;
    let policy = KeyPolicy {
        owner: &config.owner,
        rp_id: &config.rp_id,
        origin: &config.origin,
        credential_id: assertion.credential_id,
        spki_der: &key.spki_der,
        fingerprint: &key.fingerprint,
    };
    let parsed: Row = serde_json::from_str(original).map_err(|_| Held("unreviewable"))?;
    let i = parsed.data.intent;
    if i.submitter != config.actor || i.budget.units != "speed_adjusted_api_usd" {
        return Err(Held("actor_or_units"));
    }
    let live:i64=db.query_row("SELECT count(*) FROM task_head t JOIN proposal_head p ON p.id=t.proposal_id WHERE t.id=?1 AND t.lane_id=?2 AND t.revision=?3 AND t.sha256=?4 AND t.proposal_id=?5 AND t.proposal_revision=?6 AND t.proposal_sha=?7 AND t.state='live' AND p.revision=t.proposal_revision AND p.sha256=t.proposal_sha AND p.state='live'",rusqlite::params![task.id,i.lane_id,task.revision,task.sha256,i.proposal.id,i.proposal.revision,i.proposal.sha256],|r|r.get(0)).map_err(|_|Held("registry_unavailable"))?;
    if live != 1 {
        return Err(Held("stale_task_or_proposal"));
    }
    let budget = source::budget(
        &config.ledger,
        record,
        &i.budget.objective_id,
        &i.budget.maximum,
        &i.budget.productive_maximum,
        &i.budget.closure_reserve,
        clock.now(),
    )
    .ok_or(Held("budget_unavailable"))?;
    let verified = proof::verify(original, answer, &assertion, &policy, clock.now())
        .map_err(|_| Held("proof_invalid"))?;
    crate::board::own_close(&board, original, answer, &assertion, &verified)
        .ok_or(Held("board_not_current"))?;
    let original = original.to_owned();
    let answer = answer.to_owned();
    let assertion = OwnedAssertion {
        credential_id: assertion.credential_id.to_owned(),
        authenticator_data: assertion.authenticator_data.to_owned(),
        client_data_json: assertion.client_data_json.to_owned(),
        signature: assertion.signature.to_owned(),
    };
    Ok(Evidence {
        task,
        decision,
        board_revision: board.revision.clone(),
        budget,
        budget_record_sha256: record.sha256.clone(),
        board,
        original,
        answer,
        assertion,
        verified,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as b64};
    use p256::{
        ecdsa::{Signature, SigningKey, signature::Signer},
        pkcs8::EncodePublicKey,
    };
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    use time::macros::datetime;
    #[test]
    fn composed_sources_check_exact_proof_registry_and_budget() {
        let dir = std::env::temp_dir().join(format!("task-inspect-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        let fixture: Value =
            serde_json::from_str(include_str!("../tests/projection-fixture.json")).unwrap();
        let mut original: Value =
            serde_json::from_str(fixture["tasks"][0]["request_json"].as_str().unwrap()).unwrap();
        original["sequence"] = 1.into();
        let original = original.to_string();
        let task = crate::validate_review(&original).unwrap();
        let answer=json!({"v":1,"type":"work-engine.owner-answer","answer_id":"00000000-0000-4000-8000-000000009003","board_seq":1,"request_digest":crate::hash(&original),"decision":"approve","comment":"","owner":"owner","created_at":"2026-11-02T00:00:00Z","expires_at":"2026-11-03T00:00:00Z","rp_id":"auth.example"}).to_string();
        let key = SigningKey::from_slice(&[7; 32]).unwrap();
        let spki = key.verifying_key().to_public_key_der().unwrap();
        let client=json!({"type":"webauthn.get","challenge":b64.encode(Sha256::digest(answer.as_bytes())),"origin":"https://auth.example","crossOrigin":false}).to_string();
        let mut auth = Sha256::digest(b"auth.example").to_vec();
        auth.extend_from_slice(&[5, 0, 0, 0, 0]);
        let mut message = auth.clone();
        message.extend_from_slice(&Sha256::digest(client.as_bytes()));
        let sig: Signature = key.sign(&message);
        let reply=json!({"kind":"reply","sequence":2,"in_reply_to":1,"topic":"fixture:task","to":["session:fixture-session"],"message_id":"00000000-0000-4000-8000-000000009003","data":{"disposition":"answered","decision":"approve","comment":"","answer_id":"00000000-0000-4000-8000-000000009003","signed_answer":{"format":"work-engine.owner-answer/v1","answer_json":answer,"credential_id":"key","authenticator_data":b64.encode(auth),"client_data_json":b64.encode(client),"signature":b64.encode(sig.to_der().as_bytes())}}}).to_string();
        let decision = Ref {
            id: "00000000-0000-4000-8000-000000009003".into(),
            revision: "2".into(),
            sha256: crate::hash(&reply),
        };
        let transfer = json!({"revision":"a".repeat(40),"next_sequence":3,"rows":[original,reply]});
        let cost = json!({"schema":"objective-budget-control-v1","topic":"control:budget:fixture-objective","closed_at":null,"snapshot":{"snapshot_at":"2026-11-02T01:00:00Z","status":"complete","totals":{"codex":{"quota_usd_low":1,"quota_usd_high":1,"unpriced_calls":0},"claude_all_models":{"quota_usd_low":0,"quota_usd_high":0,"unpriced_calls":0},"claude_fable":{"quota_usd_low":0,"quota_usd_high":0,"unpriced_calls":0}},"cost_controls":[{"pool":"codex","node_id":null,"max_usd":5,"basis":"speed_adjusted_api_usd","remaining_usd":4,"observed":{"quota_usd_low":1,"quota_usd_high":1,"unpriced_calls":0,"cost_basis":"speed_adjusted_api_usd"}}]}});
        let pin = |p: PathBuf| Pin {
            sha256: format!("{:x}", Sha256::digest(std::fs::read(&p).unwrap())),
            path: p,
        };
        let python = std::fs::canonicalize("/usr/bin/python3").unwrap();
        let make_program = |name: &str, value: Value| {
            let path = dir.join(name);
            std::fs::write(&path, format!("print({:?})\n", value.to_string())).unwrap();
            Program {
                python: pin(python.clone()),
                script: pin(path),
                dependencies: vec![],
            }
        };
        let keyfile = dir.join("keys.json");
        std::fs::write(&keyfile,json!({"keys":{"key":{"spki_b64url":b64.encode(spki.as_bytes()),"fingerprint":format!("{:x}",Sha256::digest(spki.as_bytes())),"cose_alg":-7}}}).to_string()).unwrap();
        std::fs::set_permissions(&keyfile, std::fs::Permissions::from_mode(0o600)).unwrap();
        let record = dir.join("record");
        std::fs::write(&record, "fixture").unwrap();
        let mut config = Sources {
            canonical_budget: None,
            board: make_program("board.py", transfer),
            board_repository: dir.clone(),
            key_policy: keyfile,
            owner: "owner".into(),
            rp_id: "auth.example".into(),
            origin: "https://auth.example".into(),
            actor: "fixture-actor".into(),
            ledger: make_program("ledger.py", cost),
            budget_record: pin(record),
        };
        let registry = dir.join("registry.sqlite");
        let mut db = Connection::open(&registry).unwrap();
        std::fs::set_permissions(&registry, std::fs::Permissions::from_mode(0o600)).unwrap();
        crate::store::initialize(&db).unwrap();
        let row: Row = serde_json::from_str(&original).unwrap();
        crate::store::register_proposal(&mut db, &row.data.intent.proposal, "live", None).unwrap();
        crate::store::register_task(&mut db, &original, "live", None).unwrap();
        let request=json!({"schema":"dot.admission-request/v1-proposal","request_id":"request","task":task,"decision":decision,"expected_task_revision":"r1"}).to_string();
        let now = datetime!(2026-11-02 01:00:10 UTC);
        assert!(collect(&config, &db, &request, now).is_ok());
        // This source fixture verifies the typed composition and Held boundary;
        // actual canonical mutation is separately exercised with the real owner.
        let export = json!({"schema":"canonical-envelope-source/v1",
            "path":config.budget_record.path,"before_sha256":config.budget_record.sha256,
            "after_sha256":config.budget_record.sha256,"authority_sha256":"a".repeat(64)});
        let mut scope = make_program("scope.py", export);
        scope.dependencies.push(config.ledger.script.clone());
        config.canonical_budget = Some(CanonicalBudgetConfig {
            contract: "canonical-envelope-source/v1".into(),
            scope: scope.clone(),
        });
        let owned_source = crate::canonical_source::CanonicalSource::new(
            scope,
            config.ledger.clone(),
            config.budget_record.clone(),
        )
        .unwrap();
        assert!(collect_current(&config, &db, &request, now, &owned_source).is_ok());
        let startup = dir.join("native-startup.json");
        std::fs::write(
            &startup,
            json!({"schema":"dot.admission-config/v2-owned-source",
            "database":registry,"sources":&config})
            .to_string(),
        )
        .unwrap();
        std::fs::set_permissions(&startup, std::fs::Permissions::from_mode(0o600)).unwrap();
        let mut producer =
            crate::native_authority::ProtectedRecollector::new(pin(startup), request.clone())
                .unwrap();
        let intent = &row.data.intent;
        let mut native = crate::native_authority::NativeRequest {
            objective: &intent.budget.objective_id,
            actor: &config.actor,
            task: &task.sha256,
            proposal: &intent.proposal.sha256,
            mode: &intent.authority.mode.sha256,
            profile: &intent.workspace.configuration.sha256,
            parent: &intent.prerequisites[0].id,
            parent_sha256: &intent.prerequisites[0].sha256,
            maximum: "0.1",
            reserve: "0.1",
            attempt: &"1".repeat(64),
            request_sha256: &"2".repeat(64),
            session: "00000000-0000-4000-8000-000000000001",
            turn: "00000000-0000-4000-8000-000000000002",
            model: &intent.model.id,
            effort: &intent.model.effort,
            tier: &intent.model.tier,
        };
        assert_eq!(
            producer.recollect(&native, &owned_source, now),
            Err("qualification_unavailable")
        );
        native.actor = "foreign";
        assert_eq!(
            producer.recollect(&native, &owned_source, now),
            Err("signed_native_binding_mismatch")
        );
        native.actor = &config.actor;
        let policy_bytes = std::fs::read(&config.key_policy).unwrap();
        std::fs::remove_file(&config.key_policy).unwrap();
        assert_eq!(
            producer.recollect(&native, &owned_source, now),
            Err("key_unavailable")
        );
        std::fs::write(&config.key_policy, policy_bytes).unwrap();
        std::fs::set_permissions(&config.key_policy, std::fs::Permissions::from_mode(0o600))
            .unwrap();
        db.execute("UPDATE task_head SET state='withdrawn'", [])
            .unwrap();
        assert_eq!(
            producer.recollect(&native, &owned_source, now),
            Err("stale_task_or_proposal")
        );
        db.execute("UPDATE task_head SET state='live'", []).unwrap();
        let wrong_parent = "0".repeat(64);
        native.parent_sha256 = &wrong_parent;
        assert_eq!(
            producer.recollect(&native, &owned_source, now),
            Err("signed_parent_binding_missing")
        );

        // Entire fake path is confined to this test: the production runtime
        // has no constructor for an exclusive/profile qualification.
        let reply_value: Value = serde_json::from_str(&reply).unwrap();
        let signed = &reply_value["data"]["signed_answer"];
        let assertion = Assertion {
            credential_id: signed["credential_id"].as_str().unwrap(),
            authenticator_data: signed["authenticator_data"].as_str().unwrap(),
            client_data_json: signed["client_data_json"].as_str().unwrap(),
            signature: signed["signature"].as_str().unwrap(),
        };
        let pinned = crate::key_policy::load(&config.key_policy, assertion.credential_id).unwrap();
        let policy = KeyPolicy {
            owner: &config.owner,
            rp_id: &config.rp_id,
            origin: &config.origin,
            credential_id: assertion.credential_id,
            spki_der: &pinned.spki_der,
            fingerprint: &pinned.fingerprint,
        };
        let verified = proof::verify(&original, &answer, &assertion, &policy, now).unwrap();
        let qualified =
            crate::admission::synthetic(&original, &config.actor, datetime!(2026-11-02 01:01 UTC));
        let admission =
            crate::admission::admit_qualified(&config, &mut db, &request, &qualified, now).unwrap();
        let retry =
            crate::admission::admit_qualified(&config, &mut db, &request, &qualified, now).unwrap();
        assert_eq!(admission, retry);
        let key_bytes = std::fs::read(&config.key_policy).unwrap();
        std::fs::write(&config.key_policy, br#"{"keys":{}}"#).unwrap();
        assert!(
            crate::admission::prepare_delivery_qualified(
                &config,
                &mut db,
                &request,
                &qualified,
                &admission.operation,
                now
            )
            .is_err()
        );
        std::fs::write(&config.key_policy, key_bytes).unwrap();
        let record_bytes = std::fs::read(&config.budget_record.path).unwrap();
        std::fs::write(&config.budget_record.path, b"changed").unwrap();
        assert!(
            crate::admission::prepare_delivery_qualified(
                &config,
                &mut db,
                &request,
                &qualified,
                &admission.operation,
                now
            )
            .is_err()
        );
        std::fs::write(&config.budget_record.path, record_bytes).unwrap();
        // The fake native peer loses its first ACK. The durable unknown state
        // fences a second invocation after full action-time source rechecks.
        #[derive(Default)]
        struct FakeCustody {
            starts: Vec<String>,
        }
        impl FakeCustody {
            fn start_lost_ack(&mut self, operation: &str) -> Result<(), ()> {
                self.starts.push(operation.to_owned());
                Err(())
            }
            fn observe_exact_turn(&self, operation: &str, packet: &str) -> Option<String> {
                if self.starts.as_slice() != [operation] {
                    return None;
                }
                Some(json!({"operation":{"session_id":"s","native_turn_id":"t","entry_state":"terminal","native_status":"completed"},"completion":{"available":true,"session_id":"s","turn_id":"t","status":"completed","entry_id":"fixture-entry"},"result":{"available":true,"text":packet,"text_offset":0,"text_extent":packet.chars().count(),"redaction_count":0,"next_offset":null,"truncated":false}}).to_string())
            }
        }
        let mut fake = FakeCustody::default();
        if crate::admission::prepare_delivery_qualified(
            &config,
            &mut db,
            &request,
            &qualified,
            &admission.operation,
            now,
        )
        .is_ok()
        {
            assert!(fake.start_lost_ack(&admission.operation).is_err());
        }
        if crate::admission::prepare_delivery_qualified(
            &config,
            &mut db,
            &request,
            &qualified,
            &admission.operation,
            now,
        )
        .is_ok()
        {
            assert!(fake.start_lost_ack(&admission.operation).is_err());
        }
        assert_eq!(fake.starts, vec![admission.operation.clone()]);
        assert_eq!(
            crate::admission::admit_qualified(&config, &mut db, &request, &qualified, now).unwrap(),
            admission
        );
        assert!(
            crate::store::lookup_exact(
                &db,
                "request",
                &task,
                &decision,
                &answer,
                &verified.assertion_digest
            )
            .unwrap()
            .is_some()
        );
        let worker=json!({"schema":"dot.worker-result/v1-proposal","task":task,"admission_id":admission.id,"worker_contract_revision":1,"outcome":"completed","artifacts":[],"evidence":[],"exceptions":[]}).to_string();
        let native_observation = fake
            .observe_exact_turn(&admission.operation, &worker)
            .unwrap();
        let provenance = Ref {
            id: "fake-native-observation".into(),
            revision: "r1".into(),
            sha256: "1".repeat(64),
        };
        let result = crate::result::from_bridge_observation(
            &native_observation,
            "s",
            "t",
            &provenance,
            &task,
            &admission.id,
        )
        .unwrap();
        assert_eq!(result.outcome, "completed");
        let clock = crate::result::Clock {
            first_funded_at: None,
            admitted_at: None,
            observed_start_at: None,
            close_by: "2026-11-03T01:00:00Z",
        };
        let accounting = crate::result::Accounting {
            units: "speed_adjusted_api_usd",
            limit: "5.00",
            reservation: None,
            observed_usage: None,
            coverage: "unknown",
            unsettled: None,
        };
        let custody = crate::result::Custody {
            native: crate::result::Disposition {
                id: Some("s"),
                disposition: "unknown",
            },
            children: vec![crate::result::Disposition {
                id: None,
                disposition: "unknown",
            }],
            processes: vec![crate::result::Disposition {
                id: None,
                disposition: "unknown",
            }],
        };
        let view = crate::result::observed(
            &task,
            &admission.id,
            "s",
            "t",
            &result,
            provenance.clone(),
            clock,
            accounting,
            custody,
            "2026-11-02T01:00:10Z",
            "fake-snapshot",
        )
        .unwrap();
        let projected = serde_json::to_value(view).unwrap();
        assert_eq!(projected["outcome"], "completed");
        assert_eq!(projected["custody"]["native"]["disposition"], "unknown");
        assert!(projected["accounting"]["observed_usage"].is_null());
        assert!(
            crate::result::from_bridge_observation(
                &native_observation,
                "s",
                "wrong-turn",
                &provenance,
                &task,
                &admission.id
            )
            .is_none()
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM outbox", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );

        let database = dir.join("admission.sqlite");
        let mut disk = Connection::open(&database).unwrap();
        crate::store::initialize(&disk).unwrap();
        crate::store::register_proposal(&mut disk, &row.data.intent.proposal, "live", None)
            .unwrap();
        crate::store::register_task(&mut disk, &original, "live", None).unwrap();
        drop(disk);
        std::fs::set_permissions(&database, std::fs::Permissions::from_mode(0o600)).unwrap();
        let cfg = dir.join("config.json");
        std::fs::write(&cfg,json!({"schema":"dot.admission-config/v1-proposal","database":database,"sources":config,"coordinator_token_sha256":crate::hash(&"c".repeat(48))}).to_string()).unwrap();
        std::fs::set_permissions(&cfg, std::fs::Permissions::from_mode(0o600)).unwrap();
        let view = crate::service::call_at(cfg.to_str().unwrap(), &request, now).unwrap();
        assert_eq!(view["status"], "held");
        assert!(view["reservation_ref"].is_null());
        assert!(view["native_operation_id"].is_null());
        assert!(view["authority_revision"].is_null());
        let held_snapshot = crate::projection::snapshot(
            "2026-11-02T01:00:10Z",
            "held-fixture",
            &[crate::projection::TaskProjection {
                task: task.clone(),
                request_json: original.clone(),
                admission: Some(view),
                result: None,
            }],
        )
        .unwrap();
        let mut admitted_db = Connection::open(&database).unwrap();
        let admitted =
            crate::admission::admit_qualified(&config, &mut admitted_db, &request, &qualified, now)
                .unwrap();
        assert_eq!(admitted.id, admission.id);
        crate::admission::prepare_delivery_qualified(
            &config,
            &mut admitted_db,
            &request,
            &qualified,
            &admitted.operation,
            now,
        )
        .unwrap();
        drop(admitted_db);
        let existing = crate::service::call_at(
            cfg.to_str().unwrap(),
            &request,
            datetime!(2026-11-04 00:00 UTC),
        )
        .unwrap();
        assert_eq!(existing["admission_id"], admission.id);
        let observed_snapshot = crate::projection::snapshot(
            "2026-11-02T01:00:10Z",
            "observed-fixture",
            &[crate::projection::TaskProjection {
                task: task.clone(),
                request_json: original.clone(),
                admission: Some(existing),
                result: Some(projected.clone()),
            }],
        )
        .unwrap();
        if let Ok(output) = std::env::var("PHASE1_FIXTURE_DIR") {
            std::fs::write(
                std::path::Path::new(&output).join("actual-service-held.json"),
                held_snapshot,
            )
            .unwrap();
            std::fs::write(
                std::path::Path::new(&output).join("actual-observed-result.json"),
                observed_snapshot,
            )
            .unwrap();
        }
        let original_key = std::fs::read(&config.key_policy).unwrap();
        std::fs::remove_file(&config.key_policy).unwrap();
        let mut retry_db = Connection::open(&database).unwrap();
        assert_eq!(
            crate::admission::admit_qualified(&config, &mut retry_db, &request, &qualified, now)
                .unwrap(),
            admission
        );
        assert!(
            crate::admission::prepare_delivery_qualified(
                &config,
                &mut retry_db,
                &request,
                &qualified,
                &admission.operation,
                now
            )
            .is_err()
        );
        drop(retry_db);
        assert_eq!(
            crate::service::call_at(cfg.to_str().unwrap(), &request, now).unwrap()["admission_id"],
            admission.id
        );
        std::fs::write(&config.key_policy, &original_key).unwrap();
        std::fs::set_permissions(&config.key_policy, std::fs::Permissions::from_mode(0o600))
            .unwrap();
        let record_bytes = std::fs::read(&config.budget_record.path).unwrap();
        std::fs::write(&config.budget_record.path, b"unavailable").unwrap();
        let mut retry_db = Connection::open(&database).unwrap();
        assert_eq!(
            crate::admission::admit_qualified(&config, &mut retry_db, &request, &qualified, now)
                .unwrap(),
            admission
        );
        assert!(
            crate::admission::prepare_delivery_qualified(
                &config,
                &mut retry_db,
                &request,
                &qualified,
                &admission.operation,
                now
            )
            .is_err()
        );
        drop(retry_db);
        assert_eq!(
            crate::service::call_at(cfg.to_str().unwrap(), &request, now).unwrap()["admission_id"],
            admission.id
        );
        std::fs::write(&config.budget_record.path, record_bytes).unwrap();
        let mut retry_db = Connection::open(&database).unwrap();
        assert_eq!(
            crate::admission::admit_qualified(
                &config,
                &mut retry_db,
                &request,
                &qualified,
                datetime!(2026-11-04 00:00 UTC)
            )
            .unwrap(),
            admission
        );
        assert!(
            crate::admission::prepare_delivery_qualified(
                &config,
                &mut retry_db,
                &request,
                &qualified,
                &admission.operation,
                datetime!(2026-11-04 00:00 UTC)
            )
            .is_err()
        );
        drop(retry_db);
        let mut conflict: Value = serde_json::from_str(&request).unwrap();
        conflict["task"]["sha256"] = "f".repeat(64).into();
        assert!(
            crate::service::call_at(cfg.to_str().unwrap(), &conflict.to_string(), now).is_none()
        );
        conflict = serde_json::from_str(&request).unwrap();
        conflict["decision"]["sha256"] = "f".repeat(64).into();
        assert!(
            crate::service::call_at(cfg.to_str().unwrap(), &conflict.to_string(), now).is_none()
        );
        conflict = serde_json::from_str(&request).unwrap();
        conflict["request_id"] = "other".into();
        assert!(
            crate::service::call_at(cfg.to_str().unwrap(), &conflict.to_string(), now).is_none()
        );
        let mut actor_config: Value =
            serde_json::from_slice(&std::fs::read(&cfg).unwrap()).unwrap();
        actor_config["sources"]["actor"] = "untrusted".into();
        std::fs::write(&cfg, actor_config.to_string()).unwrap();
        assert!(crate::service::call_at(cfg.to_str().unwrap(), &request, now).is_none());
        actor_config["sources"]["actor"] = "fixture-actor".into();
        std::fs::write(&cfg, actor_config.to_string()).unwrap();
        let mut command = json!({"schema":"dot.coordinator-command/v1-proposal","operation":"register_task","credential":"wrong","expected_generation":1,"state":"withdrawn","proposal":null,"request_json":original});
        assert!(crate::service::coordinator(cfg.to_str().unwrap(), &command.to_string()).is_none());
        command["credential"] = "c".repeat(48).into();
        assert_eq!(
            crate::service::coordinator(cfg.to_str().unwrap(), &command.to_string()).unwrap()["generation"],
            2
        );
        assert!(crate::service::coordinator(cfg.to_str().unwrap(), &command.to_string()).is_none());
        assert_eq!(
            crate::service::call_at(cfg.to_str().unwrap(), &request, now).unwrap()["admission_id"],
            admission.id
        );
        let mut retry_db = Connection::open(&database).unwrap();
        assert_eq!(
            crate::admission::admit_qualified(&config, &mut retry_db, &request, &qualified, now)
                .unwrap(),
            admission
        );
        assert!(
            crate::admission::prepare_delivery_qualified(
                &config,
                &mut retry_db,
                &request,
                &qualified,
                &admission.operation,
                now
            )
            .is_err()
        );
        drop(retry_db);
        config.actor = "other".into();
        let mut retry_db = Connection::open(&database).unwrap();
        assert!(
            crate::admission::admit_qualified(&config, &mut retry_db, &request, &qualified, now)
                .is_err()
        );
        drop(retry_db);
        assert!(matches!(
            collect(&config, &db, &request, now),
            Err(Held("actor_or_units"))
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
