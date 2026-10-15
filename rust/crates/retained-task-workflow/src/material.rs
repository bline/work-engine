use lifecycle_core::{SubjectId, TransitionId};
use lifecycle_semantic::{
    DependencyObservation, DependencyRevision, MaterialInput, SemanticPolicy,
};
use lifecycle_service::{CanonicalMaterialPort, MaterialPortError, ServiceError};
use rusqlite::TransactionBehavior;

use crate::{OWNER_NAMESPACE, TaskRecord, TaskStore, obligations};

impl CanonicalMaterialPort for TaskStore {
    fn project(
        &self,
        subject: &SubjectId,
        transition: &TransitionId,
        capture_id: &str,
    ) -> Result<MaterialInput, MaterialPortError> {
        let record = self
            .observe_subject(subject.as_str())
            .map_err(|_| MaterialPortError::Unavailable)?
            .ok_or(MaterialPortError::Unavailable)?;
        if record.sources.iter().any(|s| s.bytes.len() > 16 * 1024)
            || record.sources.iter().map(|s| s.bytes.len()).sum::<usize>() > 64 * 1024
        {
            return Err(MaterialPortError::Invalid);
        }
        let obligations = obligations(&record);
        Ok(MaterialInput {
            subject: subject.clone(),
            transition: transition.clone(),
            capture_id: capture_id.into(),
            sources: record.sources,
            obligations,
            policy: SemanticPolicy::retained_repository_analysis(),
        })
    }

    fn observe_dependencies(
        &self,
        expected: &[DependencyRevision],
    ) -> Result<Vec<DependencyObservation>, MaterialPortError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| MaterialPortError::Unavailable)?;
        Ok(expected
            .iter()
            .map(|item| {
                if item.owner_namespace != OWNER_NAMESPACE {
                    return DependencyObservation::Unavailable {
                        owner_namespace: item.owner_namespace.clone(),
                        dependency_key: item.dependency_key.clone(),
                    };
                }
                let record = super::read_task(&connection, &item.owner_ref);
                match record {
                    Ok(Some(record)) => match current_dependency(&record, item) {
                        Some(revision) => DependencyObservation::Current(revision),
                        None => DependencyObservation::Missing {
                            owner_namespace: item.owner_namespace.clone(),
                            dependency_key: item.dependency_key.clone(),
                        },
                    },
                    _ => DependencyObservation::Unavailable {
                        owner_namespace: item.owner_namespace.clone(),
                        dependency_key: item.dependency_key.clone(),
                    },
                }
            })
            .collect())
    }

    fn with_current_fence(
        &self,
        expected: &[DependencyRevision],
        commit: &mut dyn FnMut() -> Result<(), ServiceError>,
    ) -> Result<(), ServiceError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ServiceError::Material(MaterialPortError::Unavailable))?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ServiceError::Material(MaterialPortError::Unavailable))?;
        for item in expected {
            if item.owner_namespace != OWNER_NAMESPACE {
                return Err(ServiceError::Material(MaterialPortError::Invalid));
            }
            let record = super::read_task(&tx, &item.owner_ref)
                .map_err(|_| ServiceError::Material(MaterialPortError::Unavailable))?
                .ok_or(ServiceError::Material(MaterialPortError::Conflict))?;
            if current_dependency(&record, item).as_ref() != Some(item) {
                return Err(ServiceError::Material(MaterialPortError::Conflict));
            }
        }
        commit()?;
        tx.commit()
            .map_err(|_| ServiceError::Material(MaterialPortError::Unavailable))?;
        Ok(())
    }
}

fn current_dependency(
    record: &TaskRecord,
    expected: &DependencyRevision,
) -> Option<DependencyRevision> {
    record
        .sources
        .iter()
        .find(|source| {
            source.current
                && source.dependency_required
                && source.dependency_key == expected.dependency_key
                && source.owner_ref == expected.owner_ref
        })
        .map(|source| DependencyRevision {
            owner_namespace: source.owner_namespace.clone(),
            dependency_key: source.dependency_key.clone(),
            owner_ref: source.owner_ref.clone(),
            revision: source.revision,
            sha256: source.sha256.clone(),
        })
}
