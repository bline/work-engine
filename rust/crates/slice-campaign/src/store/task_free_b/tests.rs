use std::path::{Path, PathBuf};

use super::*;

const CONFIG: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const WRITER: &str = "fixture-owner";
const PROFILE: &str = crate::contract::QUALIFIED_INITIAL_PROFILE;

fn digest(digit: char) -> Digest {
    Digest::new(digit.to_string().repeat(64)).unwrap()
}

struct Fixture {
    writer: Store,
    binding: Binding,
    root: PathBuf,
    _dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let _dir = tempfile::tempdir().unwrap();
        let root = _dir.path().canonicalize().unwrap().join("campaign");
        let mut writer = Store::initialize(&root, CONFIG, WRITER, 4, PROFILE).unwrap();
        let binding = Binding {
            purpose: QualificationPurpose::OneStop,
            root: writer.root_id.clone(),
            profile: PROFILE.into(),
            opening_epoch: writer.epoch.clone(),
            issuer: "owner".into(),
            grant: "grant".into(),
            attempt: "attempt".into(),
            spend_receipt: "fixture-committed-receipt".into(),
            head: digest('1'),
            canonical_selection: digest('a'),
            recipe: digest('2'),
            server_operation: "a-selected-server-op".into(),
            serving_generation: digest('3'),
            evidence_ingress: digest('4'),
        };
        // Inert fixture, not A's issuer/schema or a production permission. The
        // underlying table intentionally allows malformed/duplicate A rows so
        // the consumer's exact check is tested rather than presumed.
        let attach = writer.conn.transaction().unwrap();
        attach.execute_batch(MIGRATION_SQL).unwrap();
        attach
            .execute_batch(
                "CREATE TABLE fixture_spend (
                 issuer_id TEXT, grant_id TEXT, root_id TEXT, store_profile TEXT,
                 opening_epoch TEXT, attempt_id TEXT, spend_receipt TEXT, purpose TEXT,
                 head_digest TEXT, selection_digest TEXT, recipe_digest TEXT, server_operation TEXT,
                 serving_generation TEXT, evidence_ingress TEXT
             ) STRICT;
             CREATE VIEW task_free_b_committed_spend_v1 AS SELECT * FROM fixture_spend;",
            )
            .unwrap();
        attach
            .execute(
                "INSERT INTO fixture_spend VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![
                    binding.issuer,
                    binding.grant,
                    binding.root,
                    binding.profile,
                    binding.opening_epoch,
                    binding.attempt,
                    binding.spend_receipt,
                    PURPOSE,
                    binding.head.0,
                    binding.canonical_selection.0,
                    binding.recipe.0,
                    binding.server_operation,
                    binding.serving_generation.0,
                    binding.evidence_ingress.0
                ],
            )
            .unwrap();
        attach.commit().unwrap();
        Self {
            _dir,
            root,
            writer,
            binding,
        }
    }

    fn mark(&mut self, expected: Option<&Mark>, kind: Kind, target: Option<Target>) -> Result<()> {
        self.writer
            .mark_b_possible_issue(&self.binding, expected, kind, digest('5'), target)
    }

    fn read(&mut self) -> History {
        self.writer.read_b_history(&self.binding).unwrap()
    }

    fn record(&mut self, fact: AuthenticatedFact) -> Result<Recorded> {
        self.writer.record_b_fact(&self.binding, fact)
    }

    fn create(&mut self) -> (Mark, ObjectTarget) {
        self.mark(None, Kind::Create, None).unwrap();
        let create = self.read().tip().unwrap().clone();
        let object = ObjectTarget {
            full_id: digest('6'),
            serving_generation: self.binding.serving_generation.clone(),
        };
        self.record(attest(
            &self.binding,
            Slot::Operation(Kind::Create),
            &create,
            None,
            Observation::Created(object.clone()),
        ))
        .unwrap();
        (create, object)
    }

    fn start(&mut self) -> (Mark, TaskIdentity) {
        let (create, object) = self.create();
        self.mark(Some(&create), Kind::Start, Some(Target::Start(object)))
            .unwrap();
        let task = TaskIdentity {
            task: "opaque-task".into(),
            lifetime: digest('a'),
            exec_generation: digest('b'),
        };
        (self.read().tip().unwrap().clone(), task)
    }

    fn confirm_start(&mut self, start: &Mark, task: &TaskIdentity) {
        self.record(attest(
            &self.binding,
            Slot::Operation(Kind::Start),
            start,
            Some(task.clone()),
            Observation::Started,
        ))
        .unwrap();
    }
}

fn attest(
    binding: &Binding,
    slot: Slot,
    mark: &Mark,
    task: Option<TaskIdentity>,
    observation: Observation,
) -> AuthenticatedFact {
    AuthenticatedFact {
        binding: binding.clone(),
        fact: Fact {
            ingress: binding.evidence_ingress.clone(),
            source: digest('7'),
            slot,
            mark: mark.clone(),
            task,
            observation,
        },
    }
}

fn open(root: &Path) -> Result<Store> {
    Store::open(root, CONFIG, WRITER, 4, PROFILE)
}

fn terminal() -> Observation {
    Observation::Terminal {
        workload_and_subtree_exit: digest('8'),
        accepted_and_recovered_work_fenced: digest('9'),
    }
}

#[test]
fn one_writer_and_one_create_marker_with_exact_cas_order() {
    let mut f = Fixture::new();
    std::thread::scope(|s| {
        let root = &f.root;
        let rival = s.spawn(|| assert!(matches!(open(root), Err(CampaignError::Root(_)))));
        f.writer
            .mark_b_possible_issue(&f.binding, None, Kind::Create, digest('5'), None)
            .unwrap();
        rival.join().unwrap();
    });
    let create = f.read().tip().unwrap().clone();
    assert!(f.mark(None, Kind::Create, None).is_err());
    assert!(f.mark(Some(&create), Kind::Create, None).is_err());
    assert!(f.mark(Some(&create), Kind::Start, None).is_err());
    assert_eq!(f.read().marks, [create]);
}

#[test]
fn identical_request_from_another_attempt_is_not_a_predecessor_or_anchor() {
    let mut f = Fixture::new();
    let (create, object) = f.create();
    f.writer
        .conn
        .execute_batch(
            "INSERT INTO fixture_spend SELECT issuer_id, 'other-grant', root_id, store_profile,
             opening_epoch, 'other-attempt', spend_receipt, purpose, head_digest, selection_digest,
             recipe_digest, server_operation, serving_generation, evidence_ingress FROM fixture_spend",
        )
        .unwrap();
    let other = Binding {
        grant: "other-grant".into(),
        attempt: "other-attempt".into(),
        ..f.binding.clone()
    };
    f.writer
        .mark_b_possible_issue(&other, None, Kind::Create, create.request.clone(), None)
        .unwrap();
    let foreign_mark = f
        .writer
        .read_b_history(&other)
        .unwrap()
        .tip()
        .unwrap()
        .clone();
    assert_eq!(create.request, foreign_mark.request);
    assert_ne!(create, foreign_mark);
    assert!(
        f.mark(
            Some(&foreign_mark),
            Kind::Start,
            Some(Target::Start(object.clone()))
        )
        .is_err()
    );
    assert!(
        f.record(attest(
            &f.binding,
            Slot::Operation(Kind::Create),
            &foreign_mark,
            None,
            Observation::Created(object.clone())
        ))
        .is_err()
    );
    f.mark(Some(&create), Kind::Start, Some(Target::Start(object)))
        .unwrap();
}

#[test]
fn failed_transaction_leaves_no_partial_attempt_or_mark() {
    let mut f = Fixture::new();
    f.writer
        .conn
        .execute_batch(
            "CREATE TEMP TRIGGER reject_mark BEFORE INSERT ON b_lifecycle_mark
           BEGIN SELECT RAISE(ABORT, 'fixture rejects marker'); END;",
        )
        .unwrap();
    assert!(f.mark(None, Kind::Create, None).is_err());
    assert!(f.read().tip().is_none());
    let count: i64 = f
        .writer
        .conn
        .query_row("SELECT COUNT(*) FROM b_lifecycle_attempt", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn lost_reply_or_spend_only_reopen_never_accepts_a_new_mark() {
    for commit_mark in [true, false] {
        let Fixture {
            _dir,
            root,
            mut writer,
            binding,
        } = Fixture::new();
        if commit_mark {
            // Suppress successful completion at the fixture boundary: the
            // receipt and marker committed, but no caller gets that result.
            let _: () = writer
                .mark_b_possible_issue(&binding, None, Kind::Create, digest('5'), None)
                .unwrap();
        }
        drop(writer);
        assert!(Store::open(&root, CONFIG, "wrong writer", 4, PROFILE).is_err());
        let mut reopened = open(&root).unwrap();
        assert_ne!(reopened.epoch, binding.opening_epoch);
        let h = reopened.read_b_history(&binding).unwrap();
        assert_eq!(h.mark(Kind::Create).is_some(), commit_mark);
        assert_eq!(h.evidence(Slot::Terminal), Evidence::Absent);
        assert!(
            reopened
                .mark_b_possible_issue(&binding, None, Kind::Create, digest('5'), None)
                .is_err()
        );
        let mut relabeled = binding.clone();
        relabeled.opening_epoch = reopened.epoch.clone();
        assert!(
            reopened
                .mark_b_possible_issue(&relabeled, None, Kind::Create, digest('5'), None)
                .is_err()
        );
    }
}

#[test]
fn missing_wrong_or_duplicate_spend_cannot_mark_an_operation() {
    let mut f = Fixture::new();
    let base = f.binding.clone();
    for binding in [
        Binding {
            spend_receipt: "wrong".into(),
            ..base.clone()
        },
        Binding {
            attempt: "crossed-attempt".into(),
            ..base.clone()
        },
        Binding {
            issuer: "another-owner".into(),
            ..base.clone()
        },
        Binding {
            grant: "another-grant".into(),
            ..base.clone()
        },
        Binding {
            root: "another-root".into(),
            ..base.clone()
        },
        Binding {
            profile: "another-profile".into(),
            ..base.clone()
        },
        Binding {
            serving_generation: digest('f'),
            ..base.clone()
        },
        Binding {
            recipe: digest('f'),
            ..base.clone()
        },
        Binding {
            canonical_selection: digest('f'),
            ..base.clone()
        },
        Binding {
            server_operation: "another-operation".into(),
            ..base.clone()
        },
    ] {
        assert!(
            f.writer
                .mark_b_possible_issue(&binding, None, Kind::Create, digest('5'), None)
                .is_err()
        );
    }
    f.writer
        .conn
        .execute(
            "UPDATE fixture_spend SET purpose='default-implementation'",
            [],
        )
        .unwrap();
    assert!(f.mark(None, Kind::Create, None).is_err());
    f.writer
        .conn
        .execute("UPDATE fixture_spend SET purpose=?1", [PURPOSE])
        .unwrap();
    assert!(f.read().tip().is_none());
    f.writer
        .conn
        .execute("INSERT INTO fixture_spend SELECT * FROM fixture_spend", [])
        .unwrap();
    assert!(matches!(
        f.mark(None, Kind::Create, None),
        Err(CampaignError::Integrity(_))
    ));
}

#[test]
fn ingress_task_generation_stale_anchor_and_fact_kind_checks() {
    let mut f = Fixture::new();
    let (start, task) = f.start();
    f.confirm_start(&start, &task);
    let create = f.read().mark(Kind::Create).unwrap().clone();
    let wrong_task = TaskIdentity {
        lifetime: digest('f'),
        ..task.clone()
    };
    assert!(
        f.mark(
            Some(&start),
            Kind::Close,
            Some(Target::Close(wrong_task.clone()))
        )
        .is_err()
    );
    let mut wrong_ingress = attest(
        &f.binding,
        Slot::Terminal,
        &start,
        Some(task.clone()),
        terminal(),
    );
    wrong_ingress.fact.ingress = digest('f');
    let mut wrong_spend = attest(
        &f.binding,
        Slot::Terminal,
        &start,
        Some(task.clone()),
        terminal(),
    );
    wrong_spend.binding.spend_receipt = "elsewhere".into();
    for fact in [
        wrong_ingress,
        wrong_spend,
        attest(
            &f.binding,
            Slot::Terminal,
            &create,
            Some(task.clone()),
            terminal(),
        ),
        attest(
            &f.binding,
            Slot::Terminal,
            &start,
            Some(wrong_task),
            terminal(),
        ),
        attest(
            &f.binding,
            Slot::Hold,
            &start,
            Some(task.clone()),
            terminal(),
        ),
    ] {
        assert!(f.record(fact).is_err());
    }
    assert_eq!(f.read().evidence(Slot::Terminal), Evidence::Absent);
}

#[test]
fn terminal_and_hold_separate_with_no_close_or_delete_dependency() {
    let mut f = Fixture::new();
    let (start, task) = f.start();
    let held = |b: &Binding| attest(b, Slot::Hold, &start, Some(task.clone()), Observation::Held);
    assert_eq!(f.record(held(&f.binding)).unwrap(), Recorded::Stored);
    assert!(matches!(
        f.read().evidence(Slot::Hold),
        Evidence::UnresolvedHold(_)
    ));
    f.record(attest(
        &f.binding,
        Slot::Operation(Kind::Start),
        &start,
        Some(task.clone()),
        Observation::Started,
    ))
    .unwrap();
    assert_eq!(f.record(held(&f.binding)).unwrap(), Recorded::Duplicate);
    let h = f.read();
    assert!(matches!(
        h.evidence(Slot::Hold),
        Evidence::Observed {
            observation: Observation::Held,
            ..
        }
    ));
    assert_eq!(h.evidence(Slot::Terminal), Evidence::Absent);
    f.record(attest(
        &f.binding,
        Slot::Terminal,
        &start,
        Some(task.clone()),
        terminal(),
    ))
    .unwrap();
    assert!(
        f.mark(Some(&start), Kind::Close, Some(Target::Close(task.clone())))
            .is_err()
    );
    let Fixture {
        _dir,
        root,
        writer,
        binding,
    } = f;
    drop(writer);
    let mut reopened = open(&root).unwrap();
    let h = reopened.read_b_history(&binding).unwrap();
    assert!(h.mark(Kind::Close).is_none());
    assert!(matches!(
        h.evidence(Slot::Terminal),
        Evidence::Observed {
            observation: Observation::Terminal { .. },
            ..
        }
    ));
    assert!(matches!(
        h.evidence(Slot::Hold),
        Evidence::Observed {
            observation: Observation::Held,
            ..
        }
    ));
    assert!(reopened.record_b_fact(&binding, held(&binding)).is_err());
}

#[test]
fn failure_loss_and_unknown_stay_unresolved_and_close_is_not_terminal() {
    for observation in [Observation::Failed, Observation::Lost, Observation::Unknown] {
        let mut f = Fixture::new();
        let (start, task) = f.start();
        let slot = Slot::Operation(Kind::Start);
        f.record(attest(
            &f.binding,
            slot,
            &start,
            Some(task.clone()),
            observation.clone(),
        ))
        .unwrap();
        let h = f.read();
        let source = digest('7');
        assert_eq!(
            h.evidence(slot),
            match observation {
                Observation::Failed => Evidence::Failed(&source),
                Observation::Lost => Evidence::Lost(&source),
                Observation::Unknown => Evidence::Unknown(&source),
                _ => unreachable!(),
            }
        );
        assert!(
            f.mark(Some(&start), Kind::Start, start.target.clone())
                .is_err()
        );
        // Distinct, exact close marker; A must still independently allow a
        // scoped cleanup in the live epoch. This test records no effects.
        f.mark(Some(&start), Kind::Close, Some(Target::Close(task.clone())))
            .unwrap();
        let close = f.read().tip().unwrap().clone();
        assert!(
            f.mark(Some(&close), Kind::Close, Some(Target::Close(task.clone())))
                .is_err()
        );
        f.record(attest(
            &f.binding,
            Slot::Operation(Kind::Close),
            &close,
            Some(task.clone()),
            Observation::Closed,
        ))
        .unwrap();
        assert_eq!(f.read().evidence(Slot::Terminal), Evidence::Absent);
        f.record(attest(
            &f.binding,
            Slot::Terminal,
            &close,
            Some(task.clone()),
            Observation::Lost,
        ))
        .unwrap();
        assert!(matches!(
            f.read().evidence(Slot::Terminal),
            Evidence::Lost(_)
        ));
        assert_eq!(
            f.record(attest(
                &f.binding,
                Slot::Terminal,
                &close,
                Some(task),
                terminal()
            ))
            .unwrap(),
            Recorded::Contested
        );
        assert_eq!(f.read().evidence(Slot::Terminal), Evidence::Contested);
    }
}

#[test]
fn exact_later_frontier_can_close_without_relabeling_a_lost_cleanup() {
    let mut f = Fixture::new();
    let (start, task) = f.start();
    f.confirm_start(&start, &task);
    // Lower-level fixture history only: no composed cleanup sender.
    f.mark(Some(&start), Kind::Close, Some(Target::Close(task.clone())))
        .unwrap();
    let close = f.read().tip().unwrap().clone();
    assert!(
        f.record(attest(
            &f.binding,
            Slot::Terminal,
            &start,
            Some(task.clone()),
            terminal()
        ))
        .is_err()
    );
    let cleanup = Slot::Operation(Kind::Close);
    f.record(attest(
        &f.binding,
        cleanup,
        &close,
        Some(task.clone()),
        Observation::Lost,
    ))
    .unwrap();
    assert_eq!(f.read().evidence(Slot::Terminal), Evidence::Absent);
    // An independent terminal attestation about the later exact frontier
    // cannot turn the lost operation response into a successful response.
    f.record(attest(
        &f.binding,
        Slot::Terminal,
        &close,
        Some(task),
        terminal(),
    ))
    .unwrap();
    let literal: String = f
        .writer
        .conn
        .query_row(
            "SELECT json_extract(fact_json, '$.observation') FROM b_lifecycle_fact WHERE slot=5",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        literal,
        format!(
            r#"{{"Terminal":{{"workload_and_subtree_exit":"{}","accepted_and_recovered_work_fenced":"{}"}}}}"#,
            "8".repeat(64),
            "9".repeat(64),
        )
    );
    let h = f.read();
    assert_eq!(h.evidence(cleanup), Evidence::Lost(&digest('7')));
    assert!(matches!(
        h.evidence(Slot::Terminal),
        Evidence::Observed {
            observation: Observation::Terminal { .. },
            ..
        }
    ));
}

#[test]
fn contradictory_fact_invalidates_previous_positive_without_erasure() {
    let mut f = Fixture::new();
    let (start, task) = f.start();
    f.confirm_start(&start, &task);
    let end = |b: &Binding| attest(b, Slot::Terminal, &start, Some(task.clone()), terminal());
    f.record(end(&f.binding)).unwrap();
    let mut disagreement = end(&f.binding);
    disagreement.fact.observation = Observation::Failed;
    disagreement.fact.source = digest('a');
    assert_eq!(f.record(disagreement).unwrap(), Recorded::Contested);
    let mut third = end(&f.binding);
    third.fact.observation = Observation::Lost;
    assert_eq!(f.record(third).unwrap(), Recorded::Contested);
    assert_eq!(f.record(end(&f.binding)).unwrap(), Recorded::Duplicate);
    let h = f.read();
    assert_eq!(h.evidence(Slot::Terminal), Evidence::Contested);
    assert_eq!(h.facts[Slot::Terminal.index()].len(), 2);
    assert_eq!(h.facts[Slot::Terminal.index()][0].observation, terminal());
    assert_eq!(
        h.facts[Slot::Terminal.index()][1].observation,
        Observation::Failed
    );
}
