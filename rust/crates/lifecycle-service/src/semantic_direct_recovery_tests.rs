//! B3-AR-03: direct operations are visible to recovery and public status.
//!
//! Evidence class: synthetic. Ports are scripted doubles; the store, recovery
//! readers, snapshot export and status projection are the real owners. These
//! tests check what those readers report for each durable direct state, not
//! provider behavior.

use std::sync::Arc;

use lifecycle_semantic::{
    DirectCapture, DirectEvidenceClass, DirectInferencePort, DirectPreparedCall, DirectRole,
    DirectSubmitError, DirectTerminal, DirectWire,
};
use lifecycle_store::{RecoveryDisposition, RecoveryEntry, SqliteLifecycleStore, StoreError};
use lifecycle_wire::{FieldV1, LifecycleSnapshotV1, RuntimeAvailabilityV1};

use crate::{
    projection::into_snapshot,
    semantic_direct_tests::{Harness, Reply, ScriptedPort},
};

/// A port whose adapter establishes that no byte reached the endpoint.
struct NotSentPort(Arc<ScriptedPort>);

impl DirectInferencePort for NotSentPort {
    fn evidence(&self) -> DirectEvidenceClass {
        self.0.evidence()
    }
    fn wire(&self, role: DirectRole, input: &str) -> Result<DirectWire, DirectSubmitError> {
        self.0.wire(role, input)
    }
    fn submit(&self, _: &DirectPreparedCall) -> Result<DirectCapture, DirectSubmitError> {
        Err(DirectSubmitError::NotSent)
    }
}

fn direct_entries(store: &SqliteLifecycleStore) -> Vec<RecoveryEntry> {
    store
        .load_recovery_entries()
        .unwrap()
        .into_iter()
        .filter(|entry| entry.input_id.starts_with("direct_semantic:"))
        .collect()
}

fn disposition(store: &SqliteLifecycleStore, operation: &str) -> RecoveryDisposition {
    let entries = direct_entries(store);
    let entry = entries
        .iter()
        .find(|entry| entry.effect_id == operation)
        .unwrap_or_else(|| panic!("no recovery entry for {operation}: {entries:?}"));
    // A direct operation has no native session, thread or incarnation.
    assert_eq!(entry.incarnation_id, None, "{operation}");
    assert_eq!(entry.input_id, format!("direct_semantic:{operation}"));
    assert!(entry.attempt_id.is_some(), "{operation}");
    entry.disposition
}

fn status(harness: &Harness, store: &mut SqliteLifecycleStore) -> LifecycleSnapshotV1 {
    into_snapshot(
        store.subject_projection(&harness.subject, None, None).unwrap(),
        false,
    )
}

/// Every reader that reports unresolved attempts: the recovery snapshot, the
/// exported image, and the subject status.
fn unresolved_everywhere(harness: &Harness, store: &mut SqliteLifecycleStore, name: &str) -> [i64; 3] {
    let recovery = store.load_recovery().unwrap().unresolved_attempts;
    let exported = store
        .snapshot_to(&harness.root.parent().unwrap().join(format!("export-{name}")))
        .unwrap()
        .unresolved_attempts;
    let projected = i64::try_from(
        store
            .subject_projection(&harness.subject, None, None)
            .unwrap()
            .unresolved_attempts,
    )
    .unwrap();
    [recovery, exported, projected]
}

fn assert_unresolved(snapshot: &LifecycleSnapshotV1) {
    assert_eq!(snapshot.runtime_availability, RuntimeAvailabilityV1::Unknown);
    assert_eq!(
        snapshot.blocking_reason,
        FieldV1::Known("unresolved_entered_attempt".into())
    );
}

#[tokio::test]
async fn unknown_submission_is_a_possibly_sent_entry_in_every_reader_after_restart() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(vec![Reply {
        unknown: true,
        ..Reply::complete(Vec::new())
    }]);
    let runtime = harness.runtime(port.clone());
    assert!(matches!(
        harness.establish(&mut store, &runtime).await,
        Err(StoreError::ClaimConflict)
    ));
    assert_eq!(port.calls(), 1);
    drop(store);

    // Restart: a fresh handle reads the durable state only.
    let mut store = harness.reopen();
    assert_eq!(
        disposition(&store, "direct-compile-1"),
        RecoveryDisposition::EnteredUncertain
    );
    assert_eq!(direct_entries(&store).len(), 1);
    assert!(
        !store
            .load_recovery_entries()
            .unwrap()
            .iter()
            .any(|entry| entry.input_id.starts_with("native_")),
        "no native session or turn is fabricated for a direct operation"
    );
    assert_eq!(unresolved_everywhere(&harness, &mut store, "unknown"), [1, 1, 1]);
    assert_unresolved(&status(&harness, &mut store));

    // The uncertainty is not cleared by a retry: the operation is not resent.
    let again = ScriptedPort::new(harness.good_replies());
    let runtime = harness.runtime(again.clone());
    assert!(matches!(
        harness.establish(&mut store, &runtime).await,
        Err(StoreError::ClaimConflict)
    ));
    assert_eq!(again.calls(), 0);
    assert_unresolved(&status(&harness, &mut store));
}

#[tokio::test]
async fn terminal_observed_but_unsettled_is_distinct_from_possibly_sent() {
    let (harness, mut store) = Harness::new();
    // The verifier's terminal is durable but incomplete, so it is never settled.
    let port = ScriptedPort::new(vec![
        Reply::complete(harness.candidate_json()),
        Reply {
            terminal: DirectTerminal::Incomplete,
            ..Reply::complete(harness.report_json(lifecycle_semantic::FindingStatus::Satisfied))
        },
    ]);
    let runtime = harness.runtime(port.clone());
    assert!(matches!(
        harness.establish(&mut store, &runtime).await,
        Err(StoreError::Rejected)
    ));
    assert_eq!(port.calls(), 2);
    drop(store);

    let mut store = harness.reopen();
    assert_eq!(
        disposition(&store, "direct-compile-1"),
        RecoveryDisposition::CompletedSettled
    );
    assert_eq!(
        disposition(&store, "direct-verify-1"),
        RecoveryDisposition::TerminalObservedUnsettled
    );
    assert_eq!(unresolved_everywhere(&harness, &mut store, "terminal"), [1, 1, 1]);
    assert_unresolved(&status(&harness, &mut store));
}

#[tokio::test]
async fn refused_compile_output_is_kept_decided_and_reported_by_status() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(vec![Reply::complete(b"{\"not\":\"a plan\"}".to_vec())]);
    let runtime = harness.runtime(port.clone());
    assert!(matches!(
        harness.establish(&mut store, &runtime).await,
        Err(StoreError::Rejected)
    ));
    drop(store);

    let mut store = harness.reopen();
    assert_eq!(
        disposition(&store, "direct-compile-1"),
        RecoveryDisposition::Refused
    );
    // Refusal is a decided observation, not an outstanding send.
    assert_eq!(unresolved_everywhere(&harness, &mut store, "refused"), [0, 0, 0]);
    let snapshot = status(&harness, &mut store);
    assert_eq!(snapshot.runtime_availability, RuntimeAvailabilityV1::Unknown);
    assert_eq!(
        snapshot.blocking_reason,
        FieldV1::Known("semantic_compile_invalid_output".into())
    );
    assert_eq!(port.calls(), 1);
}

#[tokio::test]
async fn not_sent_is_decided_and_does_not_block_status() {
    let (harness, mut store) = Harness::new();
    let port = Arc::new(NotSentPort(ScriptedPort::new(Vec::new())));
    let runtime = harness.runtime_with(port);
    assert!(matches!(
        harness.establish(&mut store, &runtime).await,
        Err(StoreError::Unavailable)
    ));
    drop(store);

    let mut store = harness.reopen();
    assert_eq!(
        disposition(&store, "direct-compile-1"),
        RecoveryDisposition::NotSent
    );
    assert_eq!(unresolved_everywhere(&harness, &mut store, "notsent"), [0, 0, 0]);
    let snapshot = status(&harness, &mut store);
    assert_eq!(snapshot.runtime_availability, RuntimeAvailabilityV1::Available);
    assert_eq!(snapshot.blocking_reason, FieldV1::NotApplicable);
}

#[tokio::test]
async fn settled_operations_are_complete_and_leave_nothing_unresolved() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(harness.good_replies());
    let runtime = harness.runtime(port.clone());
    assert!(harness.establish(&mut store, &runtime).await.unwrap().is_some());
    drop(store);

    let mut store = harness.reopen();
    assert_eq!(
        disposition(&store, "direct-compile-1"),
        RecoveryDisposition::CompletedSettled
    );
    assert_eq!(
        disposition(&store, "direct-verify-1"),
        RecoveryDisposition::CompletedSettled
    );
    assert_eq!(unresolved_everywhere(&harness, &mut store, "settled"), [0, 0, 0]);
    let snapshot = status(&harness, &mut store);
    assert_eq!(snapshot.runtime_availability, RuntimeAvailabilityV1::Available);
}

#[tokio::test]
async fn a_store_migrated_from_an_older_physical_schema_has_no_direct_entries() {
    let (harness, store) = Harness::new();
    drop(store);
    {
        // Physical schema 8 has neither direct table (10 adds them to 8).
        let connection = rusqlite::Connection::open(harness.root.join("lifecycle.sqlite")).unwrap();
        connection
            .execute_batch(
                "DROP TABLE direct_semantic_results; DROP TABLE direct_semantic_attempts; PRAGMA user_version=8;",
            )
            .unwrap();
    }
    let mut store = harness.reopen();
    assert!(direct_entries(&store).is_empty());
    assert_eq!(unresolved_everywhere(&harness, &mut store, "older"), [0, 0, 0]);
    assert_eq!(
        status(&harness, &mut store).runtime_availability,
        RuntimeAvailabilityV1::Available
    );
}
