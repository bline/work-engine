//! Owned execution signatures. S2 supplies supervision and result ownership.

use std::future::Future;

use lifecycle_core::{AttemptId, EffectObservation};
use lifecycle_store::AuthorizedEntry;
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskExit {
    pub attempt: AttemptId,
    pub local_task_ended: bool,
    pub child_exit_code: Option<i32>,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("bounded execution capacity is unavailable")]
    Capacity,
    #[error("task ownership could not be established")]
    Ownership,
}

/// Entry must already be committed. A local task exit is not remote settlement.
pub trait LifecycleExecutor {
    type Run: Future<Output = (TaskExit, Option<EffectObservation>)> + Send;

    fn submit(&self, entry: AuthorizedEntry) -> Result<Self::Run, RuntimeError>;
}
