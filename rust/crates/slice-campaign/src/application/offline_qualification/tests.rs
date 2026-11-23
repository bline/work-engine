use super::*;
use crate::{AdmitRequest, AdvancePhase, Consequence, TrustedConfig, test_support as support};
use std::sync::Barrier;

struct Root {
    _temp: tempfile::TempDir,
    config: Box<dyn Fn() -> TrustedConfig>,
    identity: CampaignIdentity,
    head: CampaignRevision,
    app: Campaign,
}

fn root() -> Root {
    let (temp, repository, workspace, identity, baseline, boundary) = support::setup();
    let root = temp.path().join("campaign");
    let (r, w, i, a, b) = (
        repository.clone(),
        workspace.clone(),
        identity.clone(),
        baseline.clone(),
        boundary.clone(),
    );
    let config = Box::new(move || {
        support::config(
            root.clone(),
            r.clone(),
            w.clone(),
            i.clone(),
            a.clone(),
            b.clone(),
        )
    });
    let mut app = Campaign::initialize(config()).unwrap();
    let head = support::applied(
        app.admit(
            "admit-offline-qualification",
            AdmitRequest {
                identity: identity.clone(),
                repository,
                workspace,
                accepted_boundary: boundary,
                baseline,
                expected_impact: None,
            },
        )
        .unwrap(),
    )
    .revision;
    Root {
        _temp: temp,
        config,
        identity,
        head,
        app,
    }
}

fn pins() -> SelectionPins {
    SelectionPins {
        grant_id: "a".repeat(64),
        intent_sha256: "b".repeat(64),
        recipe_sha256: "c".repeat(64),
        serving_sha256: "d".repeat(64),
        expires_at_unix_ms: 4_102_444_800_000,
    }
}

fn selected(r: &Root) -> AttemptBinding {
    r.app.attach_offline_qualification_fixture().unwrap();
    r.app
        .select_offline_qualification(&r.identity, &r.head, pins())
        .unwrap()
}

fn spend_count(app: &Campaign) -> i64 {
    app.owner()
        .unwrap()
        .store
        .conn
        .query_row(
            "SELECT count(*) FROM offline_qualification_spend",
            [],
            |r| r.get(0),
        )
        .unwrap()
}

#[test]
fn default_off_missing_crossed_or_conflicting_selection_never_consumes() {
    let r = root();
    assert!(
        r.app
            .select_offline_qualification(&r.identity, &r.head, pins())
            .is_err()
    );
    let b = selected(&r);
    let mut alternate = pins();
    alternate.serving_sha256 = "e".repeat(64);
    let different = r
        .app
        .owner()
        .unwrap()
        .qualification_binding(&r.identity, &r.head, alternate.clone())
        .unwrap();
    assert_eq!(different.attempt_id(), b.attempt_id());
    assert_eq!(different.operation_id(), b.operation_id());
    assert_ne!(different.binding_digest(), b.binding_digest());
    assert_ne!(alternate.serving_sha256, b.serving_sha256());
    assert!(
        r.app
            .select_offline_qualification(&r.identity, &r.head, alternate)
            .is_err()
    );
    let mut wrong = r.identity.clone();
    wrong.attempt_id = "wrong-authority".into();
    assert!(
        r.app
            .select_offline_qualification(&wrong, &r.head, pins())
            .is_err()
    );
    let other = root();
    selected(&other);
    assert!(other.app.offline_qualification_port().enter(&b).is_err());
    let port = r.app.offline_qualification_port();
    assert!(port.enter(&different).is_err());
    let mut missing = pins();
    missing.grant_id = "f".repeat(64);
    let missing = r
        .app
        .owner()
        .unwrap()
        .qualification_binding(&r.identity, &r.head, missing)
        .unwrap();
    assert!(port.enter(&missing).is_err());
    assert_eq!(spend_count(&r.app), 0);
    let winner = port.enter(&b).unwrap().consume().unwrap();
    assert_eq!(winner.binding(), &b);
    assert!(port.enter(&b).is_err());
    assert_eq!(spend_count(&r.app), 1);
}

#[test]
fn stale_head_refuses_at_spend_after_entry() {
    let mut r = root();
    let b = selected(&r);
    let port = r.app.offline_qualification_port();
    let mut expired = pins();
    expired.grant_id = "e".repeat(64);
    expired.expires_at_unix_ms = 1;
    assert!(
        r.app
            .select_offline_qualification(&r.identity, &r.head, expired)
            .is_err()
    );
    let pending = port.enter(&b).unwrap();
    support::applied(
        r.app
            .advance(
                &r.identity,
                &r.head,
                "advance-offline-qualification",
                AdvancePhase::Implementing,
                Consequence {
                    owner: "offline-test".into(),
                    reference: "new-head".into(),
                    sha256: "a".repeat(64),
                },
            )
            .unwrap(),
    );
    assert!(pending.consume().is_err());
    assert_eq!(spend_count(&r.app), 0);
}

#[test]
fn expiry_between_entry_and_spend_blocks_consume_at_exact_deadline() {
    use crate::store::offline_qualification::FIXTURE_NOW_MS;
    let r = root();
    let b = selected(&r);
    let port = r.app.offline_qualification_port();
    let entered = port.enter(&b).unwrap();
    FIXTURE_NOW_MS.set(Some(b.expires_at()));
    let result = entered.consume();
    FIXTURE_NOW_MS.set(None);
    assert!(result.is_err());
    assert_eq!(spend_count(&r.app), 0);
    assert!(port.enter(&b).is_err());
}

#[test]
fn revoke_first_and_spend_first_prevent_further_entry_or_send() {
    for revoke_first in [true, false] {
        let r = root();
        let b = selected(&r);
        let entered = r.app.offline_qualification_port().enter(&b).unwrap();
        let barrier = Barrier::new(2);
        let spent = std::thread::scope(|scope| {
            let spender = scope.spawn(|| {
                if revoke_first {
                    barrier.wait();
                }
                let spent = entered.consume();
                if !revoke_first {
                    barrier.wait();
                }
                spent
            });
            if !revoke_first {
                barrier.wait();
            }
            r.app.revoke_offline_qualification(&b).unwrap();
            if revoke_first {
                barrier.wait();
            }
            spender.join().unwrap()
        });
        if revoke_first {
            assert!(spent.is_err());
        } else {
            let mut spent = spent.unwrap();
            assert!(
                spent
                    .mark_inert_send::<()>(OperationKind::Create, || panic!(
                        "revoked marker must not run"
                    ))
                    .is_err()
            );
        }
        assert!(r.app.offline_qualification_port().enter(&b).is_err());
        assert_eq!(spend_count(&r.app), i64::from(!revoke_first));
    }
}

#[test]
fn simultaneous_collectors_do_not_mint_two_winners() {
    let r = root();
    let b = selected(&r);
    let start = Barrier::new(2);
    let count = std::thread::scope(|scope| {
        let compete = || {
            let port = r.app.offline_qualification_port();
            start.wait();
            usize::from(
                port.enter(&b)
                    .and_then(EnteredQualification::consume)
                    .is_ok(),
            )
        };
        let other = scope.spawn(compete);
        compete() + other.join().unwrap()
    });
    assert_eq!(count, 1);
    assert_eq!(spend_count(&r.app), 1);
}

#[test]
fn reopened_owner_cannot_resume_after_enter_or_regenerate_spent_winner() {
    for cut in [0, 1, 2] {
        let r = root();
        let b = selected(&r);
        let entered = r.app.offline_qualification_port().enter(&b).unwrap();
        match cut {
            0 => drop(entered),
            1 => assert!(entered.consume_with_confirmation(false).is_err()),
            2 => drop(entered.consume().unwrap()),
            _ => unreachable!(),
        }
        assert_eq!(spend_count(&r.app), i64::from(cut != 0));
        drop(r.app);
        let reopened = Campaign::open((r.config)()).unwrap();
        assert_eq!(spend_count(&reopened), i64::from(cut != 0));
        assert!(reopened.offline_qualification_port().enter(&b).is_err());
        assert!(
            reopened
                .select_offline_qualification(&r.identity, &r.head, pins())
                .is_err()
        );
    }
}

fn joined(r: &Root) -> AttemptBinding {
    r.app.attach_joined_offline_qualification_fixture().unwrap();
    r.app
        .select_offline_qualification(&r.identity, &r.head, pins())
        .unwrap()
}

fn spend(r: &Root, binding: &AttemptBinding) -> SpentQualification {
    r.app
        .offline_qualification_port()
        .enter(binding)
        .unwrap()
        .consume()
        .unwrap()
}

#[test]
fn joined_existing_create_cannot_supply_a_fresh_decision() {
    use crate::store::task_free_b::Kind;
    let r = root();
    let b = joined(&r);
    let spent = spend(&r, &b);
    let request = b.create_request_digest().unwrap();
    // Inert injection through B's typed transaction on the real A spend.
    // Even the fresh, still-held A value must refuse this pre-existing mark.
    let mut owner = r.app.owner().unwrap();
    owner
        .store
        .mark_b_possible_issue(
            b.lifecycle(),
            None,
            Kind::Create,
            Digest::new(request.clone()).unwrap(),
            None,
        )
        .unwrap();
    drop(owner);
    assert!(
        spent
            .mark_joined_create(&request, JoinedMarkReadback::Check)
            .is_err()
    );
    assert!(r.app.offline_qualification_port().enter(&b).is_err());
    assert!(
        r.app
            .owner()
            .unwrap()
            .store
            .read_b_history(b.lifecycle())
            .unwrap()
            .mark(Kind::Start)
            .is_none()
    );
}

#[test]
fn joined_wrong_predecessor_or_version_forfeits_without_a_mark() {
    let r = root();
    let b = joined(&r);
    let spent = spend(&r, &b);
    let stale = campaign_digest(&json!([
        "b-offline-operation",
        0,
        2,
        "create",
        b.root_id(),
        b.grant_id(),
        b.attempt_id(),
        b.operation_id(),
        b.binding_digest(),
        "wrong-predecessor",
        null
    ]))
    .unwrap();
    assert!(
        spent
            .mark_joined_create(&stale, JoinedMarkReadback::Check)
            .is_err()
    );
    assert!(
        r.app
            .owner()
            .unwrap()
            .store
            .read_b_history(b.lifecycle())
            .unwrap()
            .tip()
            .is_none()
    );
    assert!(r.app.offline_qualification_port().enter(&b).is_err());
}

#[test]
fn joined_lost_handoff_and_mark_confirmation_are_historical_only_on_reopen() {
    use crate::store::task_free_b::{Evidence, Kind, Slot};
    for cut in 0..3 {
        let r = root();
        let b = joined(&r);
        let entered = r.app.offline_qualification_port().enter(&b).unwrap();
        // Different losses: unconfirmed committed spend, discarded fresh
        // handoff, and durable mark with lost post-commit confirmation.
        match cut {
            0 => assert!(entered.consume_with_confirmation(false).is_err()),
            1 => drop(entered.consume().unwrap()),
            2 => assert!(
                entered
                    .consume()
                    .unwrap()
                    .mark_joined_create(
                        &b.create_request_digest().unwrap(),
                        JoinedMarkReadback::LostAfterCommit
                    )
                    .is_err()
            ),
            _ => unreachable!(),
        }
        assert_eq!(spend_count(&r.app), 1);
        drop(r.app);
        let reopened = Campaign::open((r.config)()).unwrap();
        assert!(reopened.offline_qualification_port().enter(&b).is_err());
        assert!(
            reopened
                .select_offline_qualification(&r.identity, &r.head, pins())
                .is_err()
        );
        let mut owner = reopened.owner().unwrap();
        let original = b.lifecycle();
        assert_ne!(owner.store.qualification_scope().1, original.opening_epoch);
        let history = owner.store.read_b_history(original).unwrap();
        assert_eq!(history.mark(Kind::Create).is_some(), cut == 2);
        assert!(history.mark(Kind::Start).is_none());
        assert_eq!(history.evidence(Slot::Terminal), Evidence::Absent);
        assert!(
            owner
                .store
                .mark_qualification_fixture(
                    &b,
                    Digest::new(b.create_request_digest().unwrap()).unwrap(),
                    JoinedMarkReadback::Check
                )
                .is_err()
        );
    }
}

#[test]
fn joined_store_revoke_winner_or_projection_mismatch_refuses_the_marker() {
    for revoke in [true, false] {
        let r = root();
        let b = joined(&r);
        let spent = spend(&r, &b);
        if revoke {
            r.app.revoke_offline_qualification(&b).unwrap();
        } else {
            // Only B's exact projection comparison can spot this mismatch:
            // Campaign grant/attempt/receipt/selection digest still agree.
            r.app
                .owner()
                .unwrap()
                .store
                .conn
                .execute(
                    "UPDATE offline_qualification_selection SET serving_generation=?1",
                    ["9".repeat(64)],
                )
                .unwrap();
        }
        assert!(
            spent
                .mark_joined_create(
                    &b.create_request_digest().unwrap(),
                    JoinedMarkReadback::Check
                )
                .is_err()
        );
        let count: i64 = r
            .app
            .owner()
            .unwrap()
            .store
            .conn
            .query_row("SELECT count(*) FROM b_lifecycle_mark", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
        assert_eq!(spend_count(&r.app), 1);
    }
}
