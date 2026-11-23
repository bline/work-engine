use super::*;
use slice_campaign::offline_qualification::{OperationKind, SelectionPins};
use slice_campaign::{
    AcceptedBoundary, AdmitRequest, Baseline, Campaign, CampaignIdentity, Effect, TrustedConfig,
};
use work_engine_manual_host::pi_docker::offline_qualification::MarkerCase;

struct Root {
    _temp: tempfile::TempDir,
    app: Campaign,
    request: Admission,
}

fn root() -> Root {
    root_with_mode(false)
}

fn root_with_mode(joined: bool) -> Root {
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join("repo");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir(&repository).unwrap();
    std::fs::create_dir(&workspace).unwrap();
    let identity = CampaignIdentity {
        run_id: "sc0-oracle-c2".into(),
        slice_number: 1,
        attempt_id: "offline-authority-fixture".into(),
        plan_version: "sc0-controlled-v1".into(),
    };
    let boundary = AcceptedBoundary {
        reference: "offline exact Campaign fixture".into(),
        sha256: "f45b94d3ef985a446002e21a795a51004a5c732e5b150208be09f964152662ef".into(),
    };
    let baseline = Baseline {
        accepted_commit: "a8e074d765d94a7e816b7ab49db428fd4a760a9f".into(),
        accepted_tree: "17868afbf46ea53715981fce83171001c632b957".into(),
        inter_slice_commit: "a8e074d765d94a7e816b7ab49db428fd4a760a9f".into(),
    };
    macro_rules! fixture {
        ($name:literal) => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../slice-campaign/tests/fixtures/vectors/",
                $name,
                ".json"
            ))
            .to_vec()
        };
    }
    let config = TrustedConfig::controlled_sc0(
        temp.path().join("authority"),
        repository.clone(),
        workspace.clone(),
        identity.clone(),
        boundary.clone(),
        baseline.clone(),
        "offline-fixture-owner".into(),
        fixture!("f52792e22d2a1b73e007bc7078607d6657989248a35de1371a1180d23101d206"),
        fixture!("d155cc2ac1cf1ab26b0e186d6a6ac86f547b1a0b9c3ce74f0ba69eecd37a2f34"),
        fixture!("c927b51b887e37f6fdc02e1fabfadddd211d0ac30712cbc6c5028993ad3889c2"),
    )
    .unwrap();
    let mut app = Campaign::initialize(config).unwrap();
    let Effect::Applied { value, .. } = app
        .admit(
            "offline-qualification-campaign",
            AdmitRequest {
                identity: identity.clone(),
                repository,
                workspace,
                accepted_boundary: boundary,
                baseline,
                expected_impact: None,
            },
        )
        .unwrap()
    else {
        panic!("fixture campaign not admitted")
    };
    if joined {
        app.attach_joined_offline_qualification_fixture().unwrap();
    } else {
        app.attach_offline_qualification_fixture().unwrap();
    }
    let recipe = "c".repeat(64);
    let serving = "d".repeat(64);
    let binding = app
        .select_offline_qualification(
            &identity,
            &value.revision,
            SelectionPins {
                grant_id: "a".repeat(64),
                intent_sha256: "b".repeat(64),
                recipe_sha256: recipe.clone(),
                serving_sha256: serving.clone(),
                expires_at_unix_ms: 4_102_444_800_000,
            },
        )
        .unwrap();
    let request = Admission {
        operation_id: binding.operation_id().into(),
        binding,
        recipe_sha256: recipe,
        serving_sha256: serving,
    };
    Root {
        _temp: temp,
        app,
        request,
    }
}

impl Root {
    fn collector(&self) -> Collector {
        Collector::stage(
            self.app.offline_qualification_port(),
            Some(self.request.binding.clone()),
        )
        .unwrap()
    }

    fn spent_on_disk(&self) -> bool {
        let conn = rusqlite::Connection::open_with_flags(
            self.app.anchored_root().join("slice-campaign.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        conn.query_row("SELECT EXISTS(SELECT 1 FROM offline_qualification_spend WHERE grant_id=?1 AND attempt_id=?2 AND binding_digest=?3)", rusqlite::params![self.request.binding.grant_id(), self.request.binding.attempt_id(), self.request.binding.binding_digest()], |r| r.get(0)).unwrap()
    }
}

fn fresh_journal() -> MarkerFixture {
    MarkerFixture::new([MarkerCase::Fresh; 3])
}

fn unexpected(_: &EffectRequest) -> EffectObservation {
    panic!("effect must not run")
}

fn observed(op: &EffectRequest) -> EffectObservation {
    if op.kind == OperationKind::Create {
        EffectObservation::Created {
            request_digest: op.request_digest.clone(),
            full_id: "f".repeat(64),
        }
    } else {
        EffectObservation::Acknowledged {
            request_digest: op.request_digest.clone(),
        }
    }
}

#[test]
fn exact_one_winner_and_scoped_order_no_duplicate_or_reentrant_entry() {
    let root = root();
    let collector = root.collector();
    let mut journal = fresh_journal();
    let mut seen: Vec<EffectRequest> = vec![];
    let result = collector.collect(&root.request, &mut journal, |op| {
        assert!(
            root.spent_on_disk(),
            "committed exact spend before every call"
        );
        assert_eq!(
            collector.collect(&root.request, &mut fresh_journal(), unexpected),
            Collected::ClosedOrAlreadyAdmitted
        );
        assert_eq!(op.binding, root.request.binding);
        assert_eq!(op.version, 1);
        assert_eq!(usize::from(op.sequence), seen.len() + 1);
        assert_eq!(
            op.predecessor_digest.as_deref(),
            seen.last().map(|last| last.request_digest.as_str())
        );
        if op.kind == OperationKind::Create {
            assert_eq!(op.target_id, None);
        } else {
            assert_eq!(op.target_id.as_deref(), Some("f".repeat(64).as_str()));
        }
        seen.push(op.clone());
        observed(op)
    });
    assert_eq!(result, Collected::Host(Outcome::InertSequenceAcknowledged));
    assert_eq!(
        seen.iter().map(|op| op.kind).collect::<Vec<_>>(),
        [
            OperationKind::Create,
            OperationKind::Start,
            OperationKind::Close
        ]
    );
    assert_eq!(journal.marked(), seen);
    assert_eq!(
        collector.collect(&root.request, &mut journal, unexpected),
        Collected::ClosedOrAlreadyAdmitted
    );
    assert_eq!(
        root.collector()
            .collect(&root.request, &mut fresh_journal(), unexpected),
        Collected::EntryNotConfirmed
    );
}

#[test]
fn absent_crossed_or_preclosed_never_spend() {
    let root = root();
    assert!(Collector::stage(root.app.offline_qualification_port(), None).is_none());
    let collector = root.collector();
    let mut journal = fresh_journal();
    let mut wrong = root.request.clone();
    wrong.operation_id = "e".repeat(64);
    assert_eq!(
        collector.collect(&wrong, &mut journal, unexpected),
        Collected::ScopeRefused
    );
    wrong = root.request.clone();
    wrong.recipe_sha256 = "e".repeat(64);
    assert_eq!(
        collector.collect(&wrong, &mut journal, unexpected),
        Collected::ScopeRefused
    );
    wrong = root.request.clone();
    wrong.serving_sha256 = "e".repeat(64);
    assert_eq!(
        collector.collect(&wrong, &mut journal, unexpected),
        Collected::ScopeRefused
    );
    collector.close();
    assert_eq!(
        collector.collect(&root.request, &mut journal, unexpected),
        Collected::ClosedOrAlreadyAdmitted
    );
    assert!(!root.spent_on_disk());
    assert!(journal.marked().is_empty());
}

#[test]
fn missing_nonfresh_crossed_or_unknown_marker_never_dispatches() {
    for case in [
        MarkerCase::Missing,
        MarkerCase::Preexisting,
        MarkerCase::WrongBinding,
        MarkerCase::Unknown,
    ] {
        let root = root();
        let mut journal = MarkerFixture::new([case, MarkerCase::Fresh, MarkerCase::Fresh]);
        assert_eq!(
            root.collector()
                .collect(&root.request, &mut journal, unexpected),
            Collected::Host(Outcome::NotSubmitted(OperationKind::Create))
        );
        assert!(root.spent_on_disk());
        assert_eq!(
            root.collector()
                .collect(&root.request, &mut fresh_journal(), unexpected),
            Collected::EntryNotConfirmed
        );
    }
}

#[test]
fn uncertain_start_marker_and_confirmed_refusal_are_not_retry_permissions() {
    let r = root();
    let mut journal =
        MarkerFixture::new([MarkerCase::Fresh, MarkerCase::Unknown, MarkerCase::Fresh]);
    assert_eq!(
        r.collector().collect(&r.request, &mut journal, |op| {
            assert_eq!(op.kind, OperationKind::Create);
            observed(op)
        }),
        Collected::Host(Outcome::NotSubmitted(OperationKind::Start))
    );
    assert_eq!(journal.marked().last().unwrap().kind, OperationKind::Start);
    assert!(r.spent_on_disk());
    assert_eq!(
        r.collector()
            .collect(&r.request, &mut fresh_journal(), unexpected),
        Collected::EntryNotConfirmed
    );

    let r = root();
    let mut journal = fresh_journal();
    assert_eq!(
        r.collector().collect(&r.request, &mut journal, |op| {
            assert_eq!(op.kind, OperationKind::Create);
            EffectObservation::Refused {
                request_digest: op.request_digest.clone(),
            }
        }),
        Collected::Host(Outcome::ConfirmedRefusal(OperationKind::Create))
    );
    assert!(r.spent_on_disk());
    assert_eq!(
        r.collector()
            .collect(&r.request, &mut fresh_journal(), unexpected),
        Collected::EntryNotConfirmed
    );
}

#[test]
fn reentrant_close_or_revoke_during_create_prevents_subsequent_start() {
    for revoke in [false, true] {
        let root = root();
        let collector = root.collector();
        let mut journal = fresh_journal();
        assert_eq!(
            collector.collect(&root.request, &mut journal, |op| {
                assert_eq!(op.kind, OperationKind::Create);
                // The fake runs outside both locks: its scoped close/revoke can
                // complete here without deadlocking or recalling the admitted send.
                if revoke {
                    root.app
                        .revoke_offline_qualification(&root.request.binding)
                        .unwrap();
                } else {
                    collector.close();
                }
                observed(op)
            }),
            Collected::Host(Outcome::NotSubmitted(OperationKind::Start))
        );
        assert_eq!(journal.marked().len(), 1);
        assert!(root.spent_on_disk());
    }
}

#[test]
fn unknown_or_crossed_create_or_start_never_enables_guessed_close() {
    for (cut, wrong_scope) in [
        (OperationKind::Create, false),
        (OperationKind::Create, true),
        (OperationKind::Start, false),
    ] {
        let root = root();
        let mut journal = fresh_journal();
        assert_eq!(
            root.collector().collect(&root.request, &mut journal, |op| {
                assert_ne!(op.kind, OperationKind::Close);
                if op.kind != cut {
                    return observed(op);
                }
                if wrong_scope {
                    return EffectObservation::Created {
                        request_digest: "0".repeat(64),
                        full_id: "f".repeat(64),
                    };
                }
                EffectObservation::Unknown
            }),
            Collected::Host(Outcome::Unresolved(cut))
        );
        assert_eq!(journal.marked().last().unwrap().kind, cut);
        assert!(root.spent_on_disk());
        assert_eq!(
            root.collector()
                .collect(&root.request, &mut fresh_journal(), unexpected),
            Collected::EntryNotConfirmed
        );
    }
}

const CHECK: JoinedMarkReadback = JoinedMarkReadback::Check;

impl Root {
    fn conn(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.app.anchored_root().join("slice-campaign.sqlite")).unwrap()
    }

    fn marks(&self) -> i64 {
        self.conn()
            .query_row("SELECT count(*) FROM b_lifecycle_mark", [], |row| {
                row.get(0)
            })
            .unwrap()
    }
}

#[test]
fn joined_exact_durable_create_precedes_the_only_sender() {
    let r = root_with_mode(true);
    let collector = r.collector();
    let result = collector.collect_joined(&r.request, CHECK, |op| {
        assert!(r.spent_on_disk());
        let mark: String = r
            .conn()
            .query_row(
                "SELECT m.mark_json FROM b_lifecycle_mark m
             JOIN task_free_b_committed_spend_v1 s USING(attempt_id)
             WHERE s.grant_id=?1 AND s.selection_digest=?2 AND m.sequence=1 AND m.kind=1",
                rusqlite::params![op.binding.grant_id(), op.binding.binding_digest()],
                |row| row.get(0),
            )
            .unwrap();
        let mark: serde_json::Value = serde_json::from_str(&mark).unwrap();
        assert_eq!(op.kind, OperationKind::Create);
        assert_eq!((op.version, op.sequence), (1, 1));
        assert!(op.predecessor_digest.is_none() && op.target_id.is_none());
        assert_eq!(mark["request"].as_str(), Some(op.request_digest.as_str()));
        assert!(mark["task"].is_null());
        assert_eq!(
            collector.collect_joined(&r.request, CHECK, unexpected),
            Collected::ClosedOrAlreadyAdmitted
        );
        // These acquire each of the two gates; fake is below and outside both.
        collector.close();
        r.app
            .revoke_offline_qualification(&r.request.binding)
            .unwrap();
        observed(op)
    });
    assert_eq!(result, Collected::Host(Outcome::InertObjectCreated));
    assert_eq!(r.marks(), 1);
    let facts: i64 = r
        .conn()
        .query_row("SELECT count(*) FROM b_lifecycle_fact", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        facts, 0,
        "object reply is no authenticated task or terminal evidence"
    );
    assert_eq!(
        r.collector().collect_joined(&r.request, CHECK, unexpected),
        Collected::EntryNotConfirmed
    );
}

#[test]
fn joined_cross_scope_close_and_competing_collectors_cannot_win_twice() {
    use std::sync::Barrier;
    let r = root_with_mode(true);
    let mut crossed = r.request.clone();
    crossed.serving_sha256 = "e".repeat(64);
    assert_eq!(
        r.collector().collect_joined(&crossed, CHECK, unexpected),
        Collected::ScopeRefused
    );
    let before = r.collector();
    before.close();
    assert_eq!(
        before.collect_joined(&r.request, CHECK, unexpected),
        Collected::ClosedOrAlreadyAdmitted
    );
    assert!(!r.spent_on_disk());
    assert_eq!(r.marks(), 0);

    let after_spend = r.collector();
    assert_eq!(
        after_spend.collect_once(&r.request, |spent| {
            after_spend.close();
            run_joined_fixture(spent, &r.request, &after_spend.fence, CHECK, unexpected)
        }),
        Collected::Host(Outcome::NotSubmitted(OperationKind::Create))
    );
    assert!(r.spent_on_disk());
    assert_eq!(r.marks(), 0);
    assert_eq!(
        r.collector().collect_joined(&r.request, CHECK, unexpected),
        Collected::EntryNotConfirmed
    );

    let r = root_with_mode(true);
    let barrier = Barrier::new(2);
    let outcomes = std::thread::scope(|scope| {
        let compete = || {
            let collector = r.collector();
            barrier.wait();
            collector.collect_joined(&r.request, CHECK, observed)
        };
        let other = scope.spawn(compete);
        [compete(), other.join().unwrap()]
    });
    assert_eq!(
        outcomes
            .iter()
            .filter(|c| **c == Collected::Host(Outcome::InertObjectCreated))
            .count(),
        1
    );
    assert!(outcomes.contains(&Collected::EntryNotConfirmed));
    assert_eq!(r.marks(), 1);
}

#[test]
fn joined_ambiguous_or_inexact_mark_does_not_delegate() {
    for corrupt in [false, true] {
        let r = root_with_mode(true);
        let readback = if corrupt {
            // Inject a well-shaped but wrong request as part of the commit.
            // The exact post-commit comparison, not just row presence, must fail.
            r.conn().execute_batch(
                "CREATE TRIGGER offline_inexact_mark AFTER INSERT ON b_lifecycle_mark
                 BEGIN UPDATE b_lifecycle_mark SET mark_json=json_set(mark_json,'$.request',printf('%064d',0))
                 WHERE attempt_id=NEW.attempt_id AND sequence=NEW.sequence; END;"
            ).unwrap();
            CHECK
        } else {
            JoinedMarkReadback::LostAfterCommit
        };
        assert_eq!(
            r.collector()
                .collect_joined(&r.request, readback, unexpected),
            Collected::Host(Outcome::NotSubmitted(OperationKind::Create))
        );
        assert!(r.spent_on_disk());
        assert_eq!(r.marks(), 1);
        assert_eq!(
            r.collector().collect_joined(&r.request, CHECK, unexpected),
            Collected::EntryNotConfirmed
        );
    }
}

#[test]
fn joined_lost_or_crossed_effect_never_runs_a_successor_or_recovers_a_send() {
    for crossed_reply in [false, true] {
        let r = root_with_mode(true);
        let mut calls = 0;
        let result = r.collector().collect_joined(&r.request, CHECK, |op| {
            calls += 1;
            assert_eq!(op.kind, OperationKind::Create);
            if crossed_reply {
                EffectObservation::Created {
                    request_digest: "e".repeat(64),
                    full_id: "f".repeat(64),
                }
            } else {
                EffectObservation::Unknown
            }
        });
        assert_eq!(
            result,
            Collected::Host(Outcome::Unresolved(OperationKind::Create))
        );
        assert_eq!(calls, 1);
        assert_eq!(r.marks(), 1);
        assert_eq!(
            r.collector().collect_joined(&r.request, CHECK, unexpected),
            Collected::EntryNotConfirmed
        );
    }
}

#[test]
fn joined_selection_never_falls_back_to_a_simulated_mark() {
    let r = root_with_mode(true);
    assert_eq!(
        r.collector()
            .collect(&r.request, &mut fresh_journal(), unexpected),
        Collected::Host(Outcome::NotSubmitted(OperationKind::Create))
    );
    assert_eq!(r.marks(), 0);
    let r = root();
    assert_eq!(
        r.collector().collect_joined(&r.request, CHECK, unexpected),
        Collected::Host(Outcome::NotSubmitted(OperationKind::Create))
    );
}
