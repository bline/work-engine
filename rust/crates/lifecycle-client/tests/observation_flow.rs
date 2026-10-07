use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use lifecycle_client::{ClientError, ObservationTransport, ReadClient, SubjectRef, WaitHintV1};
use lifecycle_wire::{
    AdmissionV1, CursorV1, DeliveryV1, EffectSettlementV1, ExecutionOutcomeV1, FieldV1,
    LifecycleSnapshotV1, RuntimeAvailabilityV1, WaitResultV1, WaitTargetV1, WireError,
    WireErrorCode,
};

fn snapshot(outcome: ExecutionOutcomeV1, sequence: &str) -> LifecycleSnapshotV1 {
    LifecycleSnapshotV1 {
        protocol_version: 1,
        subject_id: "subject-1".into(),
        context_generation: "context-1".into(),
        cursor: CursorV1 {
            store_id: "store-1".into(),
            stream_id: "subject-1".into(),
            commit_sequence: sequence.into(),
        },
        transition: FieldV1::NotApplicable,
        current_admission: AdmissionV1::Fenced {
            reason: "unsettled_effect".into(),
        },
        delivery: FieldV1::Known(DeliveryV1 {
            delivery_id: "delivery-1".into(),
            effect_id: "effect-1".into(),
            attempt_id: "attempt-1".into(),
            outcome,
            settlement: EffectSettlementV1::Unresolved,
            provenance: FieldV1::Known(lifecycle_wire::ObservationProvenanceV1 {
                source_ref: "evidence:failure-1".into(),
                observed_wall_ms: "1700000000000".into(),
            }),
        }),
        runtime_availability: RuntimeAvailabilityV1::Unavailable,
        runtime_provenance: FieldV1::Unavailable,
        blocking_reason: FieldV1::Known("unsettled_effect".into()),
        authority_refs: vec![],
        evidence_refs: vec![],
        measurement: FieldV1::Unavailable,
    }
}

struct PassiveTransport {
    snapshots: RefCell<VecDeque<LifecycleSnapshotV1>>,
    reads: Rc<Cell<usize>>,
    hints: Rc<Cell<usize>>,
    hint: WaitHintV1,
}

impl ObservationTransport for PassiveTransport {
    fn snapshot(
        &self,
        _: &SubjectRef,
        _: Option<&WaitTargetV1>,
        budget: Duration,
    ) -> Result<LifecycleSnapshotV1, ClientError> {
        assert!(!budget.is_zero());
        self.reads.set(self.reads.get() + 1);
        self.snapshots
            .borrow_mut()
            .pop_front()
            .ok_or(ClientError::ObservationUnavailable)
    }

    fn wait_hint(
        &self,
        _: &SubjectRef,
        _: &WaitTargetV1,
        _: &CursorV1,
        budget: Duration,
    ) -> Result<WaitHintV1, ClientError> {
        assert!(!budget.is_zero());
        self.hints.set(self.hints.get() + 1);
        Ok(self.hint)
    }
}

#[test]
fn missed_notification_and_explicit_gap_resync_without_settling_failed_effect() {
    let reads = Rc::new(Cell::new(0));
    let hints = Rc::new(Cell::new(0));
    let transport = PassiveTransport {
        snapshots: RefCell::new(VecDeque::from([
            snapshot(ExecutionOutcomeV1::Pending, "5"),
            snapshot(ExecutionOutcomeV1::Failed, "9"),
        ])),
        reads: Rc::clone(&reads),
        hints: Rc::clone(&hints),
        hint: WaitHintV1::CursorGap,
    };
    let client = ReadClient::new(transport);
    let subject = SubjectRef::parse("subject-1").unwrap();
    let target = WaitTargetV1::DeliveryOutcome {
        delivery_id: "delivery-1".into(),
        outcome: ExecutionOutcomeV1::Failed,
    };
    let WaitResultV1::Observed(result) = client.wait(&subject, &target, Duration::from_secs(1))
    else {
        panic!("expected exact failed outcome")
    };
    assert!(matches!(
        result.delivery,
        FieldV1::Known(DeliveryV1 {
            settlement: EffectSettlementV1::Unresolved,
            ..
        })
    ));
    assert!(matches!(
        result.current_admission,
        AdmissionV1::Fenced { .. }
    ));
    assert_eq!(reads.get(), 2);
    assert_eq!(hints.get(), 1);
}

#[test]
fn global_sequence_jump_is_a_valid_changed_hint() {
    let transport = PassiveTransport {
        snapshots: RefCell::new(VecDeque::from([
            snapshot(ExecutionOutcomeV1::Pending, "5"),
            snapshot(ExecutionOutcomeV1::Failed, "9"),
        ])),
        reads: Rc::new(Cell::new(0)),
        hints: Rc::new(Cell::new(0)),
        hint: WaitHintV1::Changed,
    };
    let client = ReadClient::new(transport);
    let target = WaitTargetV1::DeliveryOutcome {
        delivery_id: "delivery-1".into(),
        outcome: ExecutionOutcomeV1::Failed,
    };
    assert!(matches!(
        client.wait(
            &SubjectRef::parse("subject-1").unwrap(),
            &target,
            Duration::from_secs(1)
        ),
        WaitResultV1::Observed(_)
    ));
}

#[test]
fn expired_wait_preserves_last_seen_without_polling_or_claiming_failure() {
    let hints = Rc::new(Cell::new(0));
    let reads = Rc::new(Cell::new(0));
    let transport = PassiveTransport {
        snapshots: RefCell::new(VecDeque::from([snapshot(ExecutionOutcomeV1::Pending, "5")])),
        reads: Rc::clone(&reads),
        hints: Rc::clone(&hints),
        hint: WaitHintV1::Changed,
    };
    let client = ReadClient::new(transport);
    let target = WaitTargetV1::DeliveryOutcome {
        delivery_id: "delivery-1".into(),
        outcome: ExecutionOutcomeV1::Failed,
    };
    assert!(matches!(
        client.wait(
            &SubjectRef::parse("subject-1").unwrap(),
            &target,
            Duration::ZERO
        ),
        WaitResultV1::DeadlineExpired { last_seen: None }
    ));
    assert_eq!(reads.get(), 0);
    assert_eq!(hints.get(), 0);
}

#[test]
fn reconnect_checks_store_identity_and_unavailable_is_observation_only() {
    let prior = snapshot(ExecutionOutcomeV1::Pending, "5").cursor;
    let changed_store = LifecycleSnapshotV1 {
        cursor: CursorV1 {
            store_id: "store-2".into(),
            ..prior.clone()
        },
        ..snapshot(ExecutionOutcomeV1::Pending, "6")
    };
    let client = ReadClient::new(PassiveTransport {
        snapshots: RefCell::new(VecDeque::from([changed_store])),
        reads: Rc::new(Cell::new(0)),
        hints: Rc::new(Cell::new(0)),
        hint: WaitHintV1::Changed,
    });
    assert_eq!(
        client.reconnect(
            &SubjectRef::parse("subject-1").unwrap(),
            &prior,
            Duration::from_secs(1)
        ),
        Err(ClientError::IdentityMismatch)
    );
    let target = WaitTargetV1::DeliveryOutcome {
        delivery_id: "delivery-1".into(),
        outcome: ExecutionOutcomeV1::Failed,
    };
    assert!(matches!(
        client.wait(
            &SubjectRef::parse("subject-1").unwrap(),
            &target,
            Duration::from_secs(1)
        ),
        WaitResultV1::ObservationUnavailable { .. }
    ));
}

#[test]
fn reconnect_rejects_same_stream_sequence_rollback() {
    let prior = snapshot(ExecutionOutcomeV1::Pending, "5").cursor;
    let client = ReadClient::new(PassiveTransport {
        snapshots: RefCell::new(VecDeque::from([snapshot(ExecutionOutcomeV1::Pending, "4")])),
        reads: Rc::new(Cell::new(0)),
        hints: Rc::new(Cell::new(0)),
        hint: WaitHintV1::Changed,
    });
    assert_eq!(
        client.reconnect(
            &SubjectRef::parse("subject-1").unwrap(),
            &prior,
            Duration::from_secs(1)
        ),
        Err(ClientError::InvalidObservation)
    );
}

struct SlowHintTransport {
    reads: Rc<Cell<usize>>,
}

impl ObservationTransport for SlowHintTransport {
    fn snapshot(
        &self,
        _: &SubjectRef,
        _: Option<&WaitTargetV1>,
        budget: Duration,
    ) -> Result<LifecycleSnapshotV1, ClientError> {
        assert!(!budget.is_zero());
        self.reads.set(self.reads.get() + 1);
        Ok(snapshot(ExecutionOutcomeV1::Pending, "5"))
    }

    fn wait_hint(
        &self,
        _: &SubjectRef,
        _: &WaitTargetV1,
        _: &CursorV1,
        budget: Duration,
    ) -> Result<WaitHintV1, ClientError> {
        assert!(!budget.is_zero());
        thread::sleep(Duration::from_millis(40));
        Ok(WaitHintV1::Changed)
    }
}

#[test]
fn consumed_hint_budget_prevents_resync_read() {
    let reads = Rc::new(Cell::new(0));
    let client = ReadClient::new(SlowHintTransport {
        reads: Rc::clone(&reads),
    });
    let target = WaitTargetV1::DeliveryOutcome {
        delivery_id: "delivery-1".into(),
        outcome: ExecutionOutcomeV1::Failed,
    };
    assert!(matches!(
        client.wait(
            &SubjectRef::parse("subject-1").unwrap(),
            &target,
            Duration::from_millis(20)
        ),
        WaitResultV1::DeadlineExpired { last_seen: Some(_) }
    ));
    assert_eq!(reads.get(), 1);
}

struct BudgetTraceTransport {
    reads: Rc<RefCell<Vec<Duration>>>,
}

impl ObservationTransport for BudgetTraceTransport {
    fn snapshot(
        &self,
        _: &SubjectRef,
        target: Option<&WaitTargetV1>,
        budget: Duration,
    ) -> Result<LifecycleSnapshotV1, ClientError> {
        assert!(
            matches!(target, Some(WaitTargetV1::DeliveryOutcome { delivery_id, .. }) if delivery_id == "delivery-1")
        );
        let read_number = self.reads.borrow().len();
        self.reads.borrow_mut().push(budget);
        if read_number == 0 {
            thread::sleep(Duration::from_millis(20));
            Ok(snapshot(ExecutionOutcomeV1::Pending, "5"))
        } else {
            Ok(snapshot(ExecutionOutcomeV1::Failed, "9"))
        }
    }

    fn wait_hint(
        &self,
        _: &SubjectRef,
        target: &WaitTargetV1,
        _: &CursorV1,
        budget: Duration,
    ) -> Result<WaitHintV1, ClientError> {
        assert!(
            matches!(target, WaitTargetV1::DeliveryOutcome { delivery_id, .. } if delivery_id == "delivery-1")
        );
        assert!(!budget.is_zero());
        Ok(WaitHintV1::CursorGap)
    }
}

#[test]
fn initial_and_gap_reads_receive_decreasing_remaining_budget_and_exact_target() {
    let reads = Rc::new(RefCell::new(Vec::new()));
    let client = ReadClient::new(BudgetTraceTransport {
        reads: Rc::clone(&reads),
    });
    let target = WaitTargetV1::DeliveryOutcome {
        delivery_id: "delivery-1".into(),
        outcome: ExecutionOutcomeV1::Failed,
    };
    assert!(matches!(
        client.wait(
            &SubjectRef::parse("subject-1").unwrap(),
            &target,
            Duration::from_millis(200)
        ),
        WaitResultV1::Observed(_)
    ));
    let budgets = reads.borrow();
    assert_eq!(budgets.len(), 2);
    assert!(budgets[1] < budgets[0]);
}

#[derive(Clone)]
struct TargetHistoryTransport {
    requested: Arc<Mutex<Vec<String>>>,
}

impl ObservationTransport for TargetHistoryTransport {
    fn snapshot(
        &self,
        _: &SubjectRef,
        target: Option<&WaitTargetV1>,
        budget: Duration,
    ) -> Result<LifecycleSnapshotV1, ClientError> {
        assert!(!budget.is_zero());
        let Some(WaitTargetV1::DeliveryOutcome { delivery_id, .. }) = target else {
            return Err(ClientError::InvalidObservation);
        };
        self.requested.lock().unwrap().push(delivery_id.clone());
        let mut result = snapshot(ExecutionOutcomeV1::Failed, "12");
        if let FieldV1::Known(delivery) = &mut result.delivery {
            delivery.delivery_id = delivery_id.clone();
        }
        Ok(result)
    }

    fn wait_hint(
        &self,
        _: &SubjectRef,
        _: &WaitTargetV1,
        _: &CursorV1,
        _: Duration,
    ) -> Result<WaitHintV1, ClientError> {
        panic!("durable target should be found by the targeted read")
    }
}

#[test]
fn distinct_concurrent_targets_recover_historical_results_after_later_record() {
    let requested = Arc::new(Mutex::new(Vec::new()));
    let transport = TargetHistoryTransport {
        requested: Arc::clone(&requested),
    };
    let handles: Vec<_> = ["delivery-early", "delivery-later"]
        .into_iter()
        .map(|delivery_id| {
            let client = ReadClient::new(transport.clone());
            thread::spawn(move || {
                let target = WaitTargetV1::DeliveryOutcome {
                    delivery_id: delivery_id.into(),
                    outcome: ExecutionOutcomeV1::Failed,
                };
                let result = client.wait(
                    &SubjectRef::parse("subject-1").unwrap(),
                    &target,
                    Duration::from_secs(1),
                );
                assert!(matches!(result, WaitResultV1::Observed(ref snapshot) if target.is_observed(snapshot)));
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let mut actual = requested.lock().unwrap().clone();
    actual.sort();
    assert_eq!(actual, ["delivery-early", "delivery-later"]);
}

struct UnsupportedVariantTransport;

impl ObservationTransport for UnsupportedVariantTransport {
    fn snapshot(
        &self,
        _: &SubjectRef,
        _: Option<&WaitTargetV1>,
        _: Duration,
    ) -> Result<LifecycleSnapshotV1, ClientError> {
        Err(ClientError::from(WireError::UnsupportedObservationVariant))
    }

    fn wait_hint(
        &self,
        _: &SubjectRef,
        _: &WaitTargetV1,
        _: &CursorV1,
        _: Duration,
    ) -> Result<WaitHintV1, ClientError> {
        unreachable!()
    }
}

#[test]
fn unsupported_observation_code_survives_client_wait() {
    let target = WaitTargetV1::DeliveryOutcome {
        delivery_id: "delivery-1".into(),
        outcome: ExecutionOutcomeV1::Failed,
    };
    assert!(matches!(
        ReadClient::new(UnsupportedVariantTransport).wait(
            &SubjectRef::parse("subject-1").unwrap(),
            &target,
            Duration::from_secs(1)
        ),
        WaitResultV1::ObservationUnavailable {
            code: WireErrorCode::UnsupportedObservationVariant
        }
    ));
}
