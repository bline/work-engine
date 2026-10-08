//! Exact, query-only recovery projection over the lifecycle store owner.

use lifecycle_store::{
    DispositionReceipt, ExactAttemptRecovery, ExactAttemptSelector, QualifiedRecoveryFact,
    RecoveryApplyReceipt, SignedDispositionRequest, SqliteLifecycleStore, StoreError,
};

pub fn inspect_exact(
    store: &SqliteLifecycleStore,
    selector: &ExactAttemptSelector,
) -> Result<ExactAttemptRecovery, StoreError> {
    store.read_exact_attempt(selector)
}

/// Only the store can qualify and commit a fact about the original attempt.
pub fn apply_qualified_fact(
    store: &mut SqliteLifecycleStore,
    fact: &QualifiedRecoveryFact,
) -> Result<RecoveryApplyReceipt, StoreError> {
    store.apply_qualified_recovery(fact)
}

/// The store verifies pinned trust and effect coverage before recording a
/// terminal no-retry disposition; this wrapper cannot install a trust root.
pub fn retire_exact_no_retry(
    store: &mut SqliteLifecycleStore,
    request: &SignedDispositionRequest,
) -> Result<DispositionReceipt, StoreError> {
    store.retire_no_retry(request)
}
