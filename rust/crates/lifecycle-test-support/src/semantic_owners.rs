//! Disposable synthetic canonical owner for controlled S6 proof only.
//!
//! Its mutex covers revision validation and the caller's short commit callback.
//! A test can update a dependent source concurrently, but the update cannot win
//! between the final owner check and lifecycle commit. No inference belongs in
//! that callback.

use std::sync::{Arc, Mutex};

use lifecycle_semantic::{
    DependencyChange, DependencyObservation, DependencyRevision, MaterialInput, SemanticError,
    SourceAvailability, SourceBasis, SourceMaterial, project_material, revalidate_dependencies,
};

#[derive(Clone)]
pub struct SyntheticOwnerJournal {
    state: Arc<Mutex<MaterialInput>>,
}

#[derive(Debug)]
pub enum FenceError<E> {
    OwnerUnavailable,
    InvalidMaterial(SemanticError),
    Stale(Vec<DependencyChange>),
    Commit(E),
}

impl SyntheticOwnerJournal {
    pub fn new(input: MaterialInput) -> Result<Self, SemanticError> {
        project_material(input.clone())?;
        Ok(Self {
            state: Arc::new(Mutex::new(input)),
        })
    }

    pub fn project(&self) -> Result<SourceBasis, SemanticError> {
        project_material(self.snapshot_input()?)
    }

    pub fn snapshot_input(&self) -> Result<MaterialInput, SemanticError> {
        self.state
            .lock()
            .map_err(|_| SemanticError::InvalidSource)
            .map(|input| input.clone())
    }

    pub fn observe_dependencies(&self) -> Result<Vec<DependencyObservation>, SemanticError> {
        Ok(self
            .project()?
            .dependencies
            .into_iter()
            .map(DependencyObservation::Current)
            .collect())
    }

    pub fn revalidate(
        &self,
        expected: &[DependencyRevision],
    ) -> Result<Vec<DependencyChange>, SemanticError> {
        revalidate_dependencies(expected, &self.observe_dependencies()?)
    }

    /// Publish one exact owner revision. Previously current content remains
    /// historical; a scoped answer does not become a broader approval.
    pub fn update_source(&self, update: SourceMaterial) -> Result<(), SemanticError> {
        if update.availability != SourceAvailability::Complete || !update.current {
            return Err(SemanticError::InvalidSource);
        }
        let mut guard = self
            .state
            .lock()
            .map_err(|_| SemanticError::InvalidSource)?;
        let current = guard.sources.iter().find(|source| {
            source.owner_namespace == update.owner_namespace
                && source.dependency_key == update.dependency_key
                && source.current
        });
        if guard.sources.iter().any(|source| source.id == update.id)
            || !current.is_some_and(|source| {
                update.revision > source.revision
                    && update.kind == source.kind
                    && update.trust == source.trust
                    && update.dependency_required == source.dependency_required
            })
        {
            return Err(SemanticError::InvalidSource);
        }
        let mut next = guard.clone();
        for source in &mut next.sources {
            if source.owner_namespace == update.owner_namespace
                && source.dependency_key == update.dependency_key
            {
                source.current = false;
            }
        }
        next.sources.push(update);
        project_material(next.clone())?;
        *guard = next;
        Ok(())
    }

    /// Add an owner record outside the admitted transition dependency set.
    /// The record remains available for separate synthetic subjects, but its
    /// later revisions cannot invalidate this transition's captured basis.
    pub fn add_unrelated_source(&self, source: SourceMaterial) -> Result<(), SemanticError> {
        if source.dependency_required
            || !source.current
            || source.availability != SourceAvailability::Complete
        {
            return Err(SemanticError::InvalidSource);
        }
        let mut guard = self
            .state
            .lock()
            .map_err(|_| SemanticError::InvalidSource)?;
        if guard.sources.iter().any(|existing| {
            existing.id == source.id
                || (existing.owner_namespace == source.owner_namespace
                    && existing.dependency_key == source.dependency_key)
        }) {
            return Err(SemanticError::DuplicateIdentifier);
        }
        let mut next = guard.clone();
        next.sources.push(source);
        project_material(next.clone())?;
        *guard = next;
        Ok(())
    }

    /// The callback is invoked only while exact dependent owner revisions
    /// remain current, with the journal lock held until the commit returns.
    pub fn with_current_fence<T, E>(
        &self,
        expected: &[DependencyRevision],
        commit: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, FenceError<E>> {
        let guard = self
            .state
            .lock()
            .map_err(|_| FenceError::OwnerUnavailable)?;
        let current = project_material(guard.clone()).map_err(FenceError::InvalidMaterial)?;
        if !current.is_eligible() {
            return Err(FenceError::InvalidMaterial(
                SemanticError::IneligibleMaterial,
            ));
        }
        let observations = current
            .dependencies
            .into_iter()
            .map(DependencyObservation::Current)
            .collect::<Vec<_>>();
        let changes = revalidate_dependencies(expected, &observations)
            .map_err(FenceError::InvalidMaterial)?;
        if !changes.is_empty() {
            return Err(FenceError::Stale(changes));
        }
        let result = commit().map_err(FenceError::Commit);
        drop(guard);
        result
    }
}
