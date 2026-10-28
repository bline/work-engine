//! B3-AR-01 and B3-AR-02 regressions for the direct realization.
//!
//! Evidence class: synthetic. Ports are scripted doubles and the material owner
//! is a test double; these tests check that the lifecycle's own grant, transition
//! and currentness gates hold for a direct call exactly where the native control
//! entry applies them, not provider behavior.

use lifecycle_core::{
    AuthorityExpiresAt, BuildId, ContextGeneration, GrantId, GrantScope, ProofRunId, Revision,
    TransitionFact, TrustedGrant, WallTimeMs,
};
use lifecycle_store::{SqliteLifecycleStore, StoreError};

use crate::{
    actor::clock_sample,
    semantic_direct_tests::{Harness, ISSUER, ScriptedPort},
};

fn sql(harness: &Harness, statement: &str) {
    rusqlite::Connection::open(harness.root.join("lifecycle.sqlite"))
        .unwrap()
        .execute_batch(statement)
        .unwrap();
}

/// One identity dimension of a grant changed to a value that is valid for some
/// other subject identity: the grant still names this subject.
const WRONG_DIMENSIONS: [(&str, &str); 5] = [
    ("context", "context_id='context:other'"),
    ("build", "build_id='build:other'"),
    ("proof_run", "proof_run_id='proof:other'"),
    ("principal", "principal='principal:other'"),
    ("issuer", "issuer='issuer:other'"),
];

#[tokio::test]
async fn control_grant_with_another_identity_dimension_never_enters_a_call() {
    for (name, assignment) in WRONG_DIMENSIONS {
        let (harness, mut store) = Harness::new();
        sql(
            &harness,
            &format!("UPDATE grants SET {assignment} WHERE grant_id='grant:direct'"),
        );
        let port = ScriptedPort::new(harness.good_replies());
        let runtime = harness.runtime(port.clone());
        let result = harness.establish(&mut store, &runtime).await;
        assert!(matches!(result, Err(StoreError::Rejected)), "{name}: {result:?}");
        assert_eq!(port.calls(), 0, "{name}: no call may be offered");
        assert_eq!(
            harness.count("SELECT COUNT(*) FROM direct_semantic_attempts"),
            0,
            "{name}: no intent may be recorded"
        );
    }
}

#[tokio::test]
async fn a_same_subject_grant_of_another_context_cannot_be_pointed_at() {
    // The review's example: a valid control grant of a successor context on the
    // same subject, referenced from the configuration.
    let (harness, mut store) = Harness::new();
    sql(
        &harness,
        &format!(
            "INSERT INTO grants(grant_id,issuer,principal,subject_id,context_id,build_id,proof_run_id,scope,expires_ms,revision,revoked) VALUES('grant:successor-context','{ISSUER}','principal:direct','{}','context:successor','build:direct','proof:direct','semantic_control',{},1,0)",
            harness.subject.as_str(),
            i64::MAX / 2
        ),
    );
    let port = ScriptedPort::new(harness.good_replies());
    let mut runtime = harness.runtime(port.clone());
    runtime.semantic.control_grant_ref = "grant:successor-context".into();
    let result = harness.establish(&mut store, &runtime).await;
    assert!(matches!(result, Err(StoreError::Rejected)), "{result:?}");
    assert_eq!(port.calls(), 0);
}

#[tokio::test]
async fn acting_principal_must_be_the_grants_principal() {
    // Same grant, correct in every dimension, presented by another principal.
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(harness.good_replies());
    let mut runtime = harness.runtime(port.clone());
    runtime.principal = "principal:impostor".into();
    let result = harness.establish(&mut store, &runtime).await;
    assert!(matches!(result, Err(StoreError::Rejected)), "{result:?}");
    assert_eq!(port.calls(), 0);
}

#[tokio::test]
async fn ownership_or_stage_lost_while_compile_is_in_flight_blocks_the_verify_entry() {
    for (name, statement) in [
        (
            "stage",
            "UPDATE transitions SET stage='capturing'",
        ),
        (
            "owner",
            "PRAGMA foreign_keys=OFF; UPDATE subjects SET owner_ref='transition:other'",
        ),
    ] {
        let (harness, mut store) = Harness::new();
        let port = ScriptedPort::new(harness.good_replies());
        let runtime = harness.runtime(port.clone());
        let root = harness.root.clone();
        *port.on_submit.lock().unwrap() = Some(Box::new(move |index| {
            if index == 0 {
                rusqlite::Connection::open(root.join("lifecycle.sqlite"))
                    .unwrap()
                    .execute_batch(statement)
                    .unwrap();
            }
        }));
        let result = harness.establish(&mut store, &runtime).await;
        assert!(matches!(result, Err(StoreError::Rejected)), "{name}: {result:?}");
        assert_eq!(port.calls(), 1, "{name}: the verifier must not be called");
        assert_eq!(
            harness.count("SELECT COUNT(*) FROM direct_semantic_attempts WHERE role='verify'"),
            0,
            "{name}"
        );
    }
}

fn install_admission_for(
    harness: &Harness,
    store: &mut SqliteLifecycleStore,
    id: &str,
    principal: &str,
    context: &str,
    build: &str,
    proof: &str,
) {
    store
        .install_trusted_grant(TrustedGrant {
            id: GrantId::parse(id).unwrap(),
            issuer: ISSUER.into(),
            principal: principal.into(),
            subject: harness.subject.clone(),
            context: ContextGeneration::parse(context).unwrap(),
            build: BuildId::parse(build).unwrap(),
            proof_run: ProofRunId::parse(proof).unwrap(),
            scope: GrantScope::SemanticDirectCheckpoint,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(i64::MAX / 2)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
}

#[tokio::test]
async fn an_admission_grant_of_another_identity_does_not_admit() {
    for (name, principal, context, build, proof) in [
        ("principal", "principal:other", "context:direct", "build:direct", "proof:direct"),
        ("context", "principal:direct", "context:other", "build:direct", "proof:direct"),
        ("build", "principal:direct", "context:direct", "build:other", "proof:direct"),
        ("proof_run", "principal:direct", "context:direct", "build:direct", "proof:other"),
    ] {
        let (harness, mut store) = Harness::new();
        install_admission_for(&harness, &mut store, "grant:wrong-admission", principal, context, build, proof);
        let port = ScriptedPort::public_class(harness.good_replies());
        let runtime = harness.runtime(port.clone());
        let result = harness.establish_production(&mut store, &runtime).await;
        assert!(matches!(result, Ok(None)), "{name}: {result:?}");
        assert!(store.admitted_direct_checkpoint(&harness.transition).unwrap().is_none(), "{name}");
        assert!(
            store.direct_proof_checkpoint(&harness.transition).unwrap().is_some(),
            "{name}: the result stays proof-scoped"
        );
    }
}

fn verification_fact(checkpoint: lifecycle_core::CheckpointId) -> TransitionFact {
    TransitionFact::VerificationEstablished {
        evidence: lifecycle_core::EvidenceId::parse("semantic:transition-C1:checkpoint").unwrap(),
        checkpoint,
    }
}

#[tokio::test]
async fn consumption_rechecks_the_full_tuple_of_the_admission_and_control_grants() {
    // After an admitted publication, each grant is changed in one identity
    // dimension. Each remains a current grant for the same subject. The
    // lifecycle must still refuse to consume the checkpoint.
    for grant in ["grant:direct-admission", "grant:direct"] {
        for (name, assignment) in WRONG_DIMENSIONS {
            let (harness, mut store) = Harness::new();
            harness.install_admission(&mut store, ISSUER).unwrap();
            let port = ScriptedPort::public_class(harness.good_replies());
            let runtime = harness.runtime(port);
            let checkpoint = harness
                .establish_production(&mut store, &runtime)
                .await
                .unwrap()
                .unwrap();
            harness.mark_native_basis("verifying");
            sql(
                &harness,
                &format!("UPDATE grants SET {assignment} WHERE grant_id='{grant}'"),
            );
            let refused = store.apply_transition_fact(
                &harness.transition,
                Revision::new(harness.subject_revision()),
                verification_fact(checkpoint),
                None,
                clock_sample(),
            );
            assert!(
                matches!(refused, Err(StoreError::Rejected)),
                "{grant} {name}: {refused:?}"
            );
            assert_eq!(
                harness.count("SELECT COUNT(*) FROM transitions WHERE stage='verifying'"),
                1,
                "{grant} {name}"
            );
        }
    }
}

// ---- B3-AR-02: the material owner's current fence ----------------------------

use std::sync::atomic::Ordering;

#[tokio::test]
async fn publication_holds_the_material_owners_fence() {
    let (harness, mut store) = Harness::new();
    harness.install_admission(&mut store, ISSUER).unwrap();
    let port = ScriptedPort::public_class(harness.good_replies());
    let runtime = harness.runtime(port);
    harness
        .establish_production(&mut store, &runtime)
        .await
        .unwrap()
        .expect("admitted");
    assert_eq!(
        harness.material.fenced_commits.load(Ordering::SeqCst),
        1,
        "the publish transaction must run inside the owner's fence"
    );
}

#[tokio::test]
async fn an_owner_update_between_assessment_and_publication_publishes_nothing() {
    for (name, admitted_owner) in [("proof_scoped", false), ("owner_admitted", true)] {
        let (harness, mut store) = Harness::new();
        if admitted_owner {
            harness.install_admission(&mut store, ISSUER).unwrap();
        }
        let port = ScriptedPort::public_class(harness.good_replies());
        let runtime = harness.runtime(port.clone());
        // The verifier answers; the owner then changes the source. Observation
        // happens afterwards and still sees the old revision (it released its
        // lock); only the fence reads the owner at commit time.
        let material = harness.material.clone();
        *port.on_submit.lock().unwrap() = Some(Box::new(move |index| {
            if index == 1 {
                material.changed_after_observation.store(true, Ordering::SeqCst);
            }
        }));
        let result = harness.establish_production(&mut store, &runtime).await;
        assert!(matches!(result, Ok(None)), "{name}: {result:?}");
        assert_eq!(port.calls(), 2, "{name}");
        assert!(store.direct_proof_checkpoint(&harness.transition).unwrap().is_none(), "{name}");
        assert!(store.admitted_direct_checkpoint(&harness.transition).unwrap().is_none(), "{name}");
        assert_eq!(
            harness.count("SELECT COUNT(*) FROM direct_semantic_results WHERE candidate_record_artifact_sha256 IS NOT NULL"),
            0,
            "{name}: nothing may be published"
        );
        assert_eq!(
            harness.count("SELECT COUNT(*) FROM direct_semantic_results WHERE admission_grant_id IS NOT NULL"),
            0,
            "{name}: nothing may be admitted"
        );
        assert_eq!(
            harness.count("SELECT COUNT(*) FROM semantic_captures WHERE eligibility='stale'"),
            1,
            "{name}: the capture is retired"
        );
        assert_eq!(harness.material.fenced_commits.load(Ordering::SeqCst), 0, "{name}");
    }
}

#[tokio::test]
async fn a_cached_checkpoint_is_not_consumed_after_an_owner_update() {
    let (harness, mut store) = Harness::new();
    harness.install_admission(&mut store, ISSUER).unwrap();
    let port = ScriptedPort::public_class(harness.good_replies());
    let runtime = harness.runtime(port.clone());
    let checkpoint = harness
        .establish_production(&mut store, &runtime)
        .await
        .unwrap()
        .unwrap();
    harness.mark_native_basis("verifying");

    // Re-offered from the cache: observation still says current.
    let again = harness.establish_production(&mut store, &runtime).await.unwrap();
    assert_eq!(again.as_ref(), Some(&checkpoint));
    assert_eq!(port.calls(), 2);

    // The owner changes after that observation, before the lifecycle consumes
    // the checkpoint. The engine's consumption step applies the fact under the
    // fence and must find the conflict.
    harness.material.changed_after_observation.store(true, Ordering::SeqCst);
    let admitted = store
        .admitted_direct_checkpoint(&harness.transition)
        .unwrap()
        .unwrap();
    let applied = crate::engine::apply_fact_under_current_fence(
        &mut store,
        &runtime,
        &admitted.package.basis,
        &harness.transition,
        harness.subject_revision(),
        &verification_fact(checkpoint),
        None,
    )
    .unwrap();
    assert!(!applied, "a conflicting owner must fence consumption");
    assert_eq!(
        harness.count("SELECT COUNT(*) FROM transitions WHERE stage='verifying'"),
        1
    );
    assert_eq!(
        harness.count("SELECT COUNT(*) FROM semantic_captures WHERE eligibility='stale'"),
        1
    );
    assert!(store.admitted_direct_checkpoint(&harness.transition).unwrap().is_none());
}
