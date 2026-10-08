use lifecycle_core::{SubjectId, TransitionId};
use lifecycle_semantic::{DependencyObservation, DependencyRevision, MaterialInput};
use thiserror::Error;

use crate::ServiceError;

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum MaterialPortError {
    #[error("canonical material is unavailable")]
    Unavailable,
    #[error("canonical material changed")]
    Conflict,
    #[error("canonical material is invalid")]
    Invalid,
}

/// A canonical owner supplies exact source bytes and holds a revision guard
/// across the short lifecycle commit. Inference must run outside that guard.
pub trait CanonicalMaterialPort: Send + Sync {
    fn project(
        &self,
        subject: &SubjectId,
        transition: &TransitionId,
        capture_id: &str,
    ) -> Result<MaterialInput, MaterialPortError>;

    fn observe_dependencies(
        &self,
        expected: &[DependencyRevision],
    ) -> Result<Vec<DependencyObservation>, MaterialPortError>;

    fn with_current_fence(
        &self,
        expected: &[DependencyRevision],
        commit: &mut dyn FnMut() -> Result<(), ServiceError>,
    ) -> Result<(), ServiceError>;
}
