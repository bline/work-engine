//! Store tests, fixture-scoped. Every claim here concerns the fixture experiment only.
#![cfg(feature = "fixture")]
mod common;
use common::{harness::*, *};
use rusqlite::params;
use std::time::Duration;
use work_engine_authority_store::{
    fixture::*,
    store::{
        self, FaultAction, FaultPoint, PublishOutcome, Reconciled, RevokeOutcome, Store, Write,
    },
};

fn published(o: Write<PublishOutcome>) -> (String, i64, bool) {
    match o {
        Write::Done(PublishOutcome::Published {
            result_id,
            seq,
            replayed,
        }) => (result_id, seq, replayed),
        other => panic!("expected Published, got {other:?}"),
    }
}
fn refused(o: Write<PublishOutcome>) -> String {
    match o {
        Write::Done(PublishOutcome::Refused { reason, .. }) => reason,
        other => panic!("expected Refused, got {other:?}"),
    }
}

// ---- Preflight 4 and 5 (executed) -------------------------------------------------------------

#[test]
fn pf4_ordinary_admission_refuses_the_candidate_and_names_the_none_rows() {
    let h = harness();
    let err = match h.store.admit(&h.cand) {
        Err(e) => e,
        Ok(never) => match never {},
    };
    assert_eq!(err.none_rows.len(), 2);
    assert!(err.none_rows.iter().all(|r| r.property == "separation"));
    assert!(err.reasons.iter().all(|r| r.starts_with("O5: ")));
    assert!(
        err.reasons[0].contains("cond-builder-projection")
            || err.reasons[1].contains("cond-builder-projection")
    );
    println!("PF4 reasons={:?}", err.reasons);
    // Refusal wrote nothing: no ordinary row exists anywhere.
    for t in ["admissions", "grants", "executions", "org_state"] {
        let n: i64 = h
            .store
            .connection()
            .query_row(
                &format!("SELECT COUNT(*) FROM {t} WHERE domain='ordinary'"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0, "{t}");
    }
}

#[test]
fn pf5_fixture_scheduling_exists_without_ordinary_admission() {
    let h = harness_with(2000, false);
    assert_eq!(h.count("admissions"), 1);
    let dom: String = h
        .store
        .connection()
        .query_row("SELECT domain||'/'||provenance FROM admissions", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(dom, "fixture/fixture-only");
    assert!(h.sched.job().job_id.starts_with("p1f-"));
    assert!(
        h.store.admit(&h.cand).is_err(),
        "ordinary admission still refuses"
    );
}

// ---- Publication (T-positive, readback) ---------------------------------------------------------

#[test]
fn fixture_publication_is_marked_complete_and_readable_from_the_store() {
    let mut h = harness();
    let (rid, seq, replayed) = published(h.store.fixture_publish(&h.request()));
    assert!(!replayed);
    let r = store::read_result(h.store.dir(), &rid).unwrap().unwrap();
    assert_eq!(
        (
            r.domain.as_str(),
            r.provenance.as_str(),
            r.result_class.as_str(),
            r.attestation.as_str()
        ),
        ("fixture", "fixture-only", "fixture_experiment", "none")
    );
    assert!(r.label.starts_with("FIXTURE EXPERIMENT RESULT"));
    assert_eq!(r.seq, seq);
    assert_eq!(
        r.receipt_bytes, h.receipt_bytes,
        "stored bytes are the verified bytes"
    );
    assert_eq!(r.receipt_id, h.receipt_id);
    let u: serde_json::Value = serde_json::from_str(&r.unestablished_json).unwrap();
    assert_eq!(u["none_rows"].as_array().unwrap().len(), 2);
    assert_eq!(u["separation_requirements"].as_array().unwrap().len(), 2);
    assert_eq!(u["authority_requirements"].as_array().unwrap().len(), 2);
    assert_eq!(
        u["component_binding_requirements"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(u["state"], "not established");
    assert_eq!(store::integrity_check(h.store.dir()).unwrap(), "ok");
}

#[test]
fn projection_failure_after_publish_leaves_authority_intact() {
    let mut h = harness();
    let (rid, _, _) = published(h.store.fixture_publish(&h.request()));
    let r = store::read_result(h.store.dir(), &rid).unwrap().unwrap();
    let bad = h.env.root.join("no-such-dir");
    assert!(store::write_projection(&bad, &r, "p1f-test-store").is_err());
    assert!(store::read_result(h.store.dir(), &rid).unwrap().is_some());
    let ok = store::write_projection(&h.env.root, &r, "p1f-test-store").unwrap();
    let text = std::fs::read_to_string(ok).unwrap();
    assert!(text.contains("never authority"));
    // Projection regenerated; the store, not the file, is consulted.
    assert!(
        store::read_result(h.store.dir(), "projection-only-id")
            .unwrap()
            .is_none()
    );
}

// ---- Fixture/ordinary negatives ----------------------------------------------------------------

#[test]
fn fixture_rows_never_satisfy_the_ordinary_path() {
    let mut h = harness();
    // Ordinary publication with fixture ids: refused, and no result is ever written by it.
    let reason = refused(h.store.publish_ordinary(&h.request()));
    assert_eq!(reason, "DomainMismatch");
    assert_eq!(h.count("results"), 0);
    // A request that names nothing at all is refused as missing, not as published.
    let mut empty = h.request();
    empty.admission_id = "nope".into();
    empty.operation_id = "op-empty".into();
    assert_eq!(
        refused(h.store.publish_ordinary(&empty)),
        "AdmissionMissing"
    );
}

#[test]
fn ordinary_rows_never_satisfy_the_fixture_path() {
    let mut h = harness();
    // Raw ordinary-domain rows (what no code path in this slice can create) are refused by the fixture path.
    let c = h.store.connection();
    c.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    c.execute(
        "INSERT INTO org_state VALUES('ordinary', 1, ?1, 'acc', ?2, 'ordinary')",
        params![h.adm.requirements_digest(), h.adm.candidate_digest()],
    )
    .unwrap();
    c.execute("INSERT INTO admissions VALUES('ord-adm','ordinary','ordinary',1,?1,?2,'b','{}','{}','admitted',99)", params![h.adm.requirements_digest(), h.adm.candidate_digest()]).unwrap();
    c.execute(
        "INSERT INTO grants VALUES('ord-grant','ord-adm','ordinary','ordinary',?1,'active',1,NULL)",
        params![work_engine_authority_store::schema::FIXTURE_SCOPE],
    )
    .unwrap();
    let mut req = h.request();
    req.admission_id = "ord-adm".into();
    req.grant_id = "ord-grant".into();
    req.operation_id = "op-ordinary-rows".into();
    assert_eq!(refused(h.store.fixture_publish(&req)), "DomainMismatch");
    assert_eq!(h.count("results"), 0);
}

#[test]
fn storage_constraints_reject_non_fixture_results_and_attestation() {
    let h = harness();
    let c = h.store.connection();
    let base = "INSERT INTO operations VALUES('o1','d','published',NULL,'r1',500)";
    c.execute(base, []).unwrap();
    let ins = |domain: &str, class: &str, att: &str, prov: &str| {
        c.execute(
            "INSERT INTO results VALUES('r1','o1',?1,?2,?3,?4,?5,?6,?7,?8,1,1,?9,?10,'ob',x'00','[1,2]',500)",
            params![domain, prov, class, att, work_engine_authority_store::schema::FIXTURE_LABEL, h.sched.execution_id(), h.adm.admission_id(), h.grant.grant_id(), h.adm.candidate_digest(), h.adm.requirements_digest()],
        )
    };
    assert!(
        ins("ordinary", "fixture_experiment", "none", "fixture-only").is_err(),
        "domain CHECK"
    );
    assert!(
        ins("fixture", "qualified_execution", "none", "fixture-only").is_err(),
        "result_class CHECK"
    );
    assert!(
        ins(
            "fixture",
            "fixture_experiment",
            "independent",
            "fixture-only"
        )
        .is_err(),
        "attestation CHECK"
    );
    assert!(
        ins("fixture", "fixture_experiment", "none", "ordinary").is_err(),
        "provenance CHECK"
    );
    assert!(
        ins("fixture", "fixture_experiment", "none", "fixture-only").is_ok(),
        "the one allowed shape inserts"
    );
    assert!(
        c.execute(
            "UPDATE results SET attestation='none' WHERE result_id='r1'",
            []
        )
        .is_err(),
        "immutable"
    );
    assert!(
        c.execute("DELETE FROM results WHERE result_id='r1'", [])
            .is_err(),
        "immutable"
    );
}

#[test]
fn schema_label_constant_matches_the_check() {
    assert!(
        work_engine_authority_store::schema::DDL
            .contains(work_engine_authority_store::schema::FIXTURE_LABEL)
    );
}

// ---- Idempotency (T2, T3, T4) --------------------------------------------------------------------

#[test]
fn duplicate_request_replays_and_writes_nothing_new() {
    let mut h = harness();
    let req = h.request();
    let (rid, seq, _) = published(h.store.fixture_publish(&req));
    let log_before = h.count("log");
    let (rid2, seq2, replayed) = published(h.store.fixture_publish(&req));
    assert_eq!((rid, seq), (rid2, seq2));
    assert!(replayed);
    assert_eq!(h.count("log"), log_before);
    assert_eq!(h.count("results"), 1);
}

#[test]
fn same_operation_id_with_a_different_request_conflicts_without_writing() {
    let mut h = harness();
    let req = h.request();
    published(h.store.fixture_publish(&req));
    let mut other = req.clone();
    other.body = b"different".to_vec();
    let log_before = h.count("log");
    assert_eq!(
        h.store.fixture_publish(&other),
        Write::Done(PublishOutcome::Conflict)
    );
    assert_eq!(h.count("log"), log_before);
    assert_eq!(h.count("results"), 1);
}

#[test]
fn a_refusal_is_recorded_and_replays_identically() {
    let mut h = harness();
    h.store.revoke_grant(h.grant.grant_id()).done();
    let req = h.request();
    let r1 = refused(h.store.fixture_publish(&req));
    assert_eq!(r1, "GrantRevoked");
    let log_before = h.count("log");
    match h.store.fixture_publish(&req) {
        Write::Done(PublishOutcome::Refused {
            reason, replayed, ..
        }) => assert!(replayed && reason == r1),
        o => panic!("{o:?}"),
    }
    assert_eq!(h.count("log"), log_before);
}

#[test]
fn second_result_for_the_same_execution_is_refused_as_already_published() {
    let mut h = harness();
    published(h.store.fixture_publish(&h.request()));
    let mut again = h.request();
    again.operation_id = "a-different-operation".into();
    assert_eq!(refused(h.store.fixture_publish(&again)), "AlreadyPublished");
    assert_eq!(h.count("results"), 1);
}

// ---- Currentness: revocation, supersession, withdrawal (T6) -------------------------------------

#[test]
fn revocation_before_publication_refuses() {
    let mut h = harness();
    let RevokeOutcome::Effective { seq } = h.store.revoke_grant(h.grant.grant_id()).done() else {
        panic!()
    };
    assert_eq!(
        refused(h.store.fixture_publish(&h.request())),
        "GrantRevoked"
    );
    assert_eq!(h.count("results"), 0);
    assert!(seq > 0);
}

#[test]
fn revocation_after_publication_keeps_the_earlier_result_and_orders_after_it() {
    let mut h = harness();
    let (rid, pseq, _) = published(h.store.fixture_publish(&h.request()));
    let RevokeOutcome::Effective { seq } = h.store.revoke_grant(h.grant.grant_id()).done() else {
        panic!()
    };
    assert!(
        pseq < seq,
        "publication committed before the revocation in log order"
    );
    assert!(
        store::read_result(h.store.dir(), &rid).unwrap().is_some(),
        "not silently retracted"
    );
    assert_eq!(
        h.store.revoke_grant(h.grant.grant_id()).done(),
        RevokeOutcome::AlreadyRevoked { seq: Some(seq) }
    );
}

#[test]
fn requirements_supersession_refuses_the_old_revision() {
    let mut h = harness();
    let newer = variant_candidate();
    h.store
        .fixture_supersede_requirements(&FixtureAcceptance::synthetic("v2"), &newer)
        .unwrap();
    assert_eq!(
        refused(h.store.fixture_publish(&h.request())),
        "RequirementsSuperseded"
    );
    assert_eq!(h.count("results"), 0);
}

#[test]
fn admission_withdrawal_refuses() {
    let mut h = harness();
    h.store.withdraw_admission(h.adm.admission_id()).done();
    assert_eq!(
        refused(h.store.fixture_publish(&h.request())),
        "AdmissionNotAdmitted"
    );
}

#[test]
fn a_wrong_organization_identity_is_refused() {
    let mut h = harness();
    let mut req = h.request();
    req.candidate_digest = "0".repeat(64);
    assert_eq!(
        refused(h.store.fixture_publish(&req)),
        "OrganizationIdentityMismatch"
    );
}

#[test]
fn a_wrong_revision_is_refused() {
    let mut h = harness();
    let mut req = h.request();
    req.org_revision = 7;
    req.operation_id = req.derived_operation_id() + "-x";
    assert_eq!(
        refused(h.store.fixture_publish(&req)),
        "RequirementsSuperseded"
    );
}

#[test]
fn a_grant_for_another_admission_or_another_scope_is_refused() {
    let mut h = harness();
    // Second admission (different binding) with its own grant.
    let binding = AdmissionBinding {
        image: work_engine_manual_host::pi_docker::IMAGE.into(),
        runtime_manifest_sha256: "f".repeat(64),
    };
    let adm2 = h.store.fixture_admit(&h.cand, &binding).unwrap();
    let grant2 = h.store.fixture_grant(&adm2).unwrap();
    let mut req = h.request();
    req.grant_id = grant2.grant_id().to_owned();
    assert_eq!(
        refused(h.store.fixture_publish(&req)),
        "GrantWrongAdmission"
    );
    // Scope: the CHECK forbids a non-fixture scope in a fixture grant, so tamper the scope check
    // with a different fixture-prefixed scope through raw SQL (an owner-side corruption).
    h.store
        .connection()
        .execute(
            "UPDATE grants SET scope='fixture:other' WHERE grant_id=?1",
            [h.grant.grant_id()],
        )
        .unwrap();
    let mut req2 = h.request();
    req2.operation_id = "op-scope".into();
    assert_eq!(
        refused(h.store.fixture_publish(&req2)),
        "GrantScopeMismatch"
    );
}

// ---- Evidence (T7) --------------------------------------------------------------------------------

fn tampered(h: &mut H, f: impl FnOnce(&mut serde_json::Value)) -> String {
    let (id, bytes) = synthetic_receipt(h.sched.job(), f);
    // Make the execution row point at this receipt so only the evidence check can object.
    h.store
        .connection()
        .execute("UPDATE executions SET host_receipt_digest=?1", [&id])
        .unwrap();
    let mut req = h.request();
    req.receipt_id = id;
    req.receipt_bytes = bytes;
    req.operation_id = format!("op-{}", req.receipt_id);
    refused(h.store.fixture_publish(&req))
}

#[test]
fn wrong_evidence_is_refused() {
    let mut h = harness();
    assert!(tampered(&mut h, |v| v["status"] = "not_established".into()).contains("status is not"));
    assert!(tampered(&mut h, |v| v["provider"] = "openai-codex".into()).contains("fake_fixture"));
    assert!(tampered(&mut h, |v| v["artifacts"] = serde_json::json!([])).contains("artifact"));
    assert!(
        tampered(&mut h, |v| v["input_digest"] = "1".repeat(64).into()).contains("input digest")
    );
    assert!(tampered(&mut h, |v| v["job_id"] = "p1f-other".into()).contains("job id"));
    assert!(tampered(&mut h, |v| v["schema"] = "other".into()).contains("schema"));
    assert!(
        tampered(&mut h, |v| {
            v["job"]["image"] = "node@sha256:".to_owned().add_hex().into();
        })
        .contains("job does not hash")
    );
    assert_eq!(h.count("results"), 0);
}
trait AddHex {
    fn add_hex(self) -> String;
}
impl AddHex for String {
    fn add_hex(mut self) -> String {
        self.push_str(&"e".repeat(64));
        self
    }
}

#[test]
fn tampered_receipt_bytes_and_wrong_receipt_id_are_refused() {
    let mut h = harness();
    let mut req = h.request();
    req.receipt_bytes.push(b' ');
    assert!(refused(h.store.fixture_publish(&req)).contains("do not hash"));
    let mut req2 = h.request();
    req2.operation_id = "op2".into();
    req2.receipt_id = "9".repeat(64);
    assert_eq!(
        refused(h.store.fixture_publish(&req2)),
        "ReceiptNotTheRecordedOne"
    );
}

#[test]
fn receipt_from_another_job_is_refused() {
    let mut h = harness();
    let other = {
        let mut inputs = h.env.inputs.clone();
        inputs.command_timeout_seconds = 11;
        let ob = h.view.obligations[0].id.as_str().to_owned();
        work_engine_authority_store::adapter::obligation_job(
            &h.view,
            &h.cand.content_digest(),
            &ob,
            &inputs,
        )
        .unwrap()
    };
    let (id, bytes) = synthetic_receipt(&other, |_| {});
    h.store
        .connection()
        .execute("UPDATE executions SET host_receipt_digest=?1", [&id])
        .unwrap();
    let mut req = h.request();
    req.receipt_id = id;
    req.receipt_bytes = bytes;
    req.operation_id = "op-other-job".into();
    assert!(refused(h.store.fixture_publish(&req)).contains("does not hash to the execution id"));
}

#[test]
fn a_binding_to_another_image_or_runtime_is_refused() {
    let mut h = harness();
    // Receipt (and job) bound to a different runtime tree manifest than the admission's binding.
    let mut inputs = h.env.inputs.clone();
    std::fs::write(h.env.root.join("runtime/extra.txt"), b"x").unwrap();
    inputs.runtime = tree(
        &h.env.root.join("runtime"),
        &h.env.root.join("meta"),
        "runtime-manifest-2.json",
    );
    let ob = h.view.obligations[0].id.as_str().to_owned();
    let job = work_engine_authority_store::adapter::obligation_job(
        &h.view,
        &h.cand.content_digest(),
        &ob,
        &inputs,
    )
    .unwrap();
    let sched2 = h
        .store
        .fixture_schedule(&h.adm, &h.view, &h.cand, &ob, &inputs)
        .unwrap();
    // A second obligation row is not allowed, so rewrite the prepared row to this execution.
    let (id, bytes) = synthetic_receipt(&job, |_| {});
    h.store
        .connection()
        .execute(
            "UPDATE executions SET execution_id=?1, job_id=?2, host_receipt_digest=?3",
            params![sched2.execution_id(), job.job_id, id],
        )
        .unwrap();
    let mut req = h.request();
    req.execution_id = sched2.execution_id().to_owned();
    req.receipt_id = id;
    req.receipt_bytes = bytes;
    req.operation_id = "op-binding".into();
    assert!(refused(h.store.fixture_publish(&req)).contains("binding"));
}

#[test]
fn oversized_body_is_refused() {
    let mut h = harness();
    let mut req = h.request();
    req.body = vec![0u8; 65_537];
    assert_eq!(refused(h.store.fixture_publish(&req)), "BodyTooLarge");
}

// ---- Host Unknown (T8) -----------------------------------------------------------------------------

#[test]
fn host_unknown_is_never_re_executed_and_cannot_publish() {
    let mut h = harness_with(2000, false);
    let mut calls = 0;
    let r = run_once(&mut h.store, &h.sched, |_| {
        calls += 1;
        HostRun::Unknown("CreateUnknown".into())
    })
    .unwrap();
    assert!(r.is_none());
    assert_eq!(calls, 1);
    let again = run_once(&mut h.store, &h.sched, |_| {
        calls += 1;
        HostRun::Receipt(h.receipt_bytes.clone())
    });
    assert!(matches!(again, Err(FixtureError::ExecutionExists)));
    assert_eq!(calls, 1, "the Host runner is not called again");
    let state: String = h
        .store
        .connection()
        .query_row("SELECT state FROM executions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(state, "unknown");
    assert_eq!(
        refused(h.store.fixture_publish(&h.request())),
        "ExecutionNotFinished"
    );
}

#[test]
fn publish_without_a_prepared_execution_is_refused() {
    let mut h = harness_with(2000, false);
    h.store
        .connection()
        .execute("DELETE FROM executions", [])
        .unwrap();
    assert_eq!(
        refused(h.store.fixture_publish(&h.request())),
        "ExecutionMissing"
    );
}

// ---- Storage outcomes (T9) and uncertain commit ---------------------------------------------------------

#[test]
fn failure_before_begin_is_not_committed_and_retryable() {
    let mut h = harness();
    let mut once = true;
    h.store.set_hook(Box::new(move |p, _| {
        if p == FaultPoint::BeforeBegin && std::mem::take(&mut once) {
            FaultAction::Fail("injected before begin".into())
        } else {
            FaultAction::Continue
        }
    }));
    let req = h.request();
    assert!(matches!(
        h.store.fixture_publish(&req),
        Write::NotCommitted(_)
    ));
    assert_eq!(h.count("results"), 0);
    assert!(!h.store.is_failed_closed());
    published(h.store.fixture_publish(&req));
}

#[test]
fn busy_begin_is_not_committed_and_the_same_operation_is_retryable() {
    let mut h = harness_with(50, true);
    let other = rusqlite::Connection::open(h.dir.join(store::DB_FILE)).unwrap();
    other.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let req = h.request();
    match h.store.fixture_publish(&req) {
        Write::NotCommitted(m) => assert!(m.starts_with("begin:"), "{m}"),
        o => panic!("{o:?}"),
    }
    other.execute_batch("ROLLBACK;").unwrap();
    assert_eq!(h.count("results"), 0);
    published(h.store.fixture_publish(&req));
}

#[test]
fn a_commit_error_is_unknown_never_not_committed_and_the_handle_fails_closed() {
    let mut h = harness();
    // A real COMMIT error: a deferred foreign-key violation is checked at COMMIT.
    h.store.set_hook(Box::new(|p, conn| {
        if p == FaultPoint::BeforeCommit {
            conn.execute_batch("PRAGMA defer_foreign_keys=ON; INSERT INTO receipts VALUES('dangling','no-such-result',x'00');").unwrap();
        }
        FaultAction::Continue
    }));
    let req = h.request();
    match h.store.fixture_publish(&req) {
        Write::Unknown(m) => assert!(m.contains("commit error"), "{m}"),
        o => panic!("must be OutcomeUnknown, got {o:?}"),
    }
    assert!(h.store.is_failed_closed());
    // The handle refuses further writes; this is a definite non-write (no transaction began).
    assert!(
        matches!(h.store.fixture_publish(&req), Write::NotCommitted(m) if m.contains("failed closed"))
    );
    // Reconciliation from a fresh read-only connection: the failed commit left nothing.
    assert_eq!(
        store::reconcile(h.store.dir(), &req.operation_id).unwrap(),
        Reconciled::Absent
    );
    assert_eq!(store::integrity_check(h.store.dir()).unwrap(), "ok");
    // No automatic re-publish happened; an owner-driven retry on a fresh handle is possible.
    assert_eq!(h.count("results"), 0);
    let mut fresh = Store::open(h.store.dir(), Duration::from_secs(2)).unwrap();
    published(fresh.fixture_publish(&req));
}

#[test]
fn an_error_after_begin_is_unknown() {
    let mut h = harness();
    h.store.set_hook(Box::new(|p, _| {
        if p == FaultPoint::AfterBegin {
            FaultAction::Fail("injected after begin".into())
        } else {
            FaultAction::Continue
        }
    }));
    assert!(matches!(
        h.store.fixture_publish(&h.request()),
        Write::Unknown(_)
    ));
    assert!(h.store.is_failed_closed());
}

#[test]
fn lost_reply_after_commit_reconciles_as_committed_and_replays_without_a_new_row() {
    let mut h = harness();
    h.store.set_hook(Box::new(|p, _| {
        if p == FaultPoint::AfterCommit {
            FaultAction::Fail("reply dropped".into())
        } else {
            FaultAction::Continue
        }
    }));
    let req = h.request();
    assert!(matches!(h.store.fixture_publish(&req), Write::Unknown(m) if m.contains("reply lost")));
    let Reconciled::Committed {
        outcome,
        durability,
        ..
    } = store::reconcile(h.store.dir(), &req.operation_id).unwrap()
    else {
        panic!("must be committed")
    };
    assert_eq!(outcome, "published");
    assert!(durability.contains("not established"));
    let log_before = h.count("log");
    let mut fresh = Store::open(h.store.dir(), Duration::from_secs(2)).unwrap();
    let (_, _, replayed) = published(fresh.fixture_publish(&req));
    assert!(replayed);
    assert_eq!(h.count("log"), log_before);
}

#[test]
fn read_only_database_never_reports_published() {
    use std::os::unix::fs::PermissionsExt;
    let h = harness();
    let dir = h.dir.clone();
    let req = h.request();
    drop(h.store);
    for suffix in ["", "-wal", "-shm"] {
        let p = dir.join(format!("{}{suffix}", store::DB_FILE));
        if p.exists() {
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o400)).unwrap();
        }
    }
    let mut s = Store::open(&dir, Duration::from_millis(200));
    if let Ok(s) = &mut s {
        assert!(!matches!(
            s.fixture_publish(&req),
            Write::Done(PublishOutcome::Published { .. })
        ));
    }
    for suffix in ["", "-wal", "-shm"] {
        let p = dir.join(format!("{}{suffix}", store::DB_FILE));
        if p.exists() {
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
    assert_eq!(
        store::reconcile(&dir, &req.operation_id).unwrap(),
        Reconciled::Absent
    );
}

#[test]
fn corrupt_database_is_a_hard_error() {
    let h = harness();
    let dir = h.dir.clone();
    drop(h.store);
    std::fs::write(dir.join(store::DB_FILE), vec![0x5a; 4096]).unwrap();
    let _ = std::fs::remove_file(dir.join(format!("{}-wal", store::DB_FILE)));
    let _ = std::fs::remove_file(dir.join(format!("{}-shm", store::DB_FILE)));
    assert!(Store::open(&dir, Duration::from_millis(100)).is_err());
}

// ---- Transaction mode ------------------------------------------------------------------------------------

#[test]
fn write_transactions_take_the_write_lock_at_begin() {
    let mut h = harness();
    let db = h.dir.join(store::DB_FILE);
    let probe = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
    let p2 = probe.clone();
    h.store.set_hook(Box::new(move |p, _| {
        if p == FaultPoint::AfterBegin {
            let c = rusqlite::Connection::open(&db).unwrap();
            c.busy_timeout(Duration::ZERO).unwrap();
            *p2.lock().unwrap() = Some(match c.execute_batch("BEGIN IMMEDIATE; ROLLBACK;") {
                Ok(()) => "acquired".into(),
                Err(e) => format!("blocked: {e}"),
            });
        }
        FaultAction::Continue
    }));
    published(h.store.fixture_publish(&h.request()));
    let seen = probe.lock().unwrap().clone().unwrap();
    assert!(
        seen.starts_with("blocked"),
        "another writer must be excluded from AfterBegin on: {seen}"
    );
}

#[test]
fn feature_ordinary_api_has_no_success_value() {
    // `Store::admit` returns Result<Infallible, _>: this test documents the type, executed by compiling.
    let h = harness();
    let r: Result<std::convert::Infallible, _> = h.store.admit(&h.cand);
    assert!(r.is_err());
}
