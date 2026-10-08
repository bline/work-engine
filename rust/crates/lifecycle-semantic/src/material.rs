use crate::{SemanticError, SemanticPolicy, json_digest, sha256_hex, valid_digest, valid_id};
use lifecycle_core::{SubjectId, TransitionId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Governing,
    CanonicalRecord,
    HumanInteraction,
    Decision,
    GrantReference,
    NativeHistory,
    DeliveryConsequence,
    ExternalData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTrust {
    HostGoverning,
    HostCanonical,
    HistoricalAuthorityReference,
    UntrustedData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAvailability {
    Complete,
    Missing,
    Unavailable,
    Truncated,
    Conflicted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceMaterial {
    pub id: String,
    pub owner_namespace: String,
    pub owner_ref: String,
    pub revision: u64,
    pub dependency_key: String,
    pub current: bool,
    /// Only relevant owner keys enter the activation currentness fence.
    pub dependency_required: bool,
    pub kind: SourceKind,
    /// Host loader assigns trust; source text cannot change this field.
    pub trust: SourceTrust,
    pub availability: SourceAvailability,
    /// Exact admitted bytes. Non-complete items must carry no bytes.
    pub bytes: Vec<u8>,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SourceReference {
    pub id: String,
    pub owner_namespace: String,
    pub owner_ref: String,
    pub revision: u64,
    pub sha256: String,
}

impl SourceMaterial {
    pub fn reference(&self) -> SourceReference {
        SourceReference {
            id: self.id.clone(),
            owner_namespace: self.owner_namespace.clone(),
            owner_ref: self.owner_ref.clone(),
            revision: self.revision,
            sha256: self.sha256.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObligationKind {
    Objective,
    CompletedConsequence,
    Commitment,
    Decision,
    Uncertainty,
    HumanInteraction,
    Governing,
    CanonicalReference,
    DeliveryCustody,
    NextAction,
}

/// Host-owned quality target. `verifier_scope` identifies the question and
/// admitted citation scope without supplying the expected answer. The private
/// `description` remains only in adjudication material, never in a prompt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RequiredObligation {
    pub id: String,
    pub kind: ObligationKind,
    pub verifier_scope: String,
    pub description: String,
    pub sources: Vec<SourceReference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MaterialInput {
    pub subject: SubjectId,
    pub transition: TransitionId,
    pub capture_id: String,
    pub sources: Vec<SourceMaterial>,
    pub obligations: Vec<RequiredObligation>,
    pub policy: SemanticPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DependencyRevision {
    pub owner_namespace: String,
    pub dependency_key: String,
    pub owner_ref: String,
    pub revision: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialIssue {
    TooManyItems,
    ItemTooLarge {
        id: String,
    },
    TotalTooLarge,
    Unavailable {
        id: String,
        status: SourceAvailability,
    },
    RequiredSourceUnavailable {
        obligation: String,
        source: String,
    },
    NoCurrentRevision {
        dependency_key: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceBasis {
    pub subject: SubjectId,
    pub transition: TransitionId,
    pub capture_id: String,
    pub schema_version: u32,
    pub policy_id: String,
    pub sources: Vec<SourceMaterial>,
    pub obligations: Vec<RequiredObligation>,
    pub dependencies: Vec<DependencyRevision>,
    pub issues: Vec<MaterialIssue>,
    pub sha256: String,
}

impl SourceBasis {
    pub fn is_eligible(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn resolve(&self, reference: &SourceReference) -> Option<&SourceMaterial> {
        self.sources.iter().find(|source| {
            source.availability == SourceAvailability::Complete && source.reference() == *reference
        })
    }

    pub fn source(&self, id: &str) -> Option<&SourceMaterial> {
        self.sources.iter().find(|source| source.id == id)
    }

    /// Recheck a persisted or reconstructed basis before using its public DTO
    /// fields to build a control prompt or semantic verdict.
    pub fn validate(&self, policy: &SemanticPolicy) -> Result<(), SemanticError> {
        let rebuilt = project_material(MaterialInput {
            subject: self.subject.clone(),
            transition: self.transition.clone(),
            capture_id: self.capture_id.clone(),
            sources: self.sources.clone(),
            obligations: self.obligations.clone(),
            policy: policy.clone(),
        })?;
        if rebuilt != *self {
            return Err(SemanticError::WrongBinding);
        }
        Ok(())
    }
}

/// Check host-projected material without changing source availability. The
/// caller still must establish owner currentness at commit under its fence.
pub fn project_material(input: MaterialInput) -> Result<SourceBasis, SemanticError> {
    if input.policy.schema_version != 1 || input.policy.id != "s6-strict-synthetic-v1" {
        return Err(SemanticError::UnsupportedPolicy);
    }
    if !input.policy.within_frozen_limits() || !valid_id(&input.capture_id) {
        return Err(SemanticError::InvalidIdentifier);
    }
    let mut ids = BTreeSet::new();
    let mut dependency_current = BTreeMap::new();
    let mut dependency_seen = BTreeSet::new();
    let mut issues = Vec::new();
    if input.sources.len() > input.policy.max_source_items {
        issues.push(MaterialIssue::TooManyItems);
    }
    let mut total_bytes = 0usize;
    for source in &input.sources {
        if !valid_id(&source.id)
            || !valid_id(&source.owner_namespace)
            || !valid_id(&source.owner_ref)
            || !valid_id(&source.dependency_key)
            || !valid_digest(&source.sha256)
        {
            return Err(SemanticError::InvalidIdentifier);
        }
        if !ids.insert(source.id.clone()) {
            return Err(SemanticError::DuplicateIdentifier);
        }
        let valid_trust = match source.kind {
            SourceKind::Governing => source.trust == SourceTrust::HostGoverning,
            SourceKind::GrantReference => source.trust == SourceTrust::HistoricalAuthorityReference,
            SourceKind::ExternalData => source.trust == SourceTrust::UntrustedData,
            _ => source.trust == SourceTrust::HostCanonical,
        };
        if !valid_trust {
            return Err(SemanticError::InvalidSource);
        }
        if source.availability == SourceAvailability::Complete {
            if sha256_hex(&source.bytes) != source.sha256 {
                return Err(SemanticError::InvalidDigest);
            }
            if source.bytes.len() > input.policy.max_source_item_bytes {
                issues.push(MaterialIssue::ItemTooLarge {
                    id: source.id.clone(),
                });
            }
            total_bytes = total_bytes.saturating_add(source.bytes.len());
        } else {
            if !source.bytes.is_empty() {
                return Err(SemanticError::InvalidSource);
            }
            issues.push(MaterialIssue::Unavailable {
                id: source.id.clone(),
                status: source.availability,
            });
        }
        let key = (
            source.owner_namespace.clone(),
            source.dependency_key.clone(),
        );
        if source.dependency_required {
            dependency_seen.insert(key.clone());
        }
        if source.current
            && source.dependency_required
            && dependency_current
                .insert(
                    key,
                    DependencyRevision {
                        owner_namespace: source.owner_namespace.clone(),
                        dependency_key: source.dependency_key.clone(),
                        owner_ref: source.owner_ref.clone(),
                        revision: source.revision,
                        sha256: source.sha256.clone(),
                    },
                )
                .is_some()
        {
            return Err(SemanticError::InvalidSource);
        }
    }
    if total_bytes > input.policy.max_source_total_bytes {
        issues.push(MaterialIssue::TotalTooLarge);
    }
    for (namespace, key) in dependency_seen {
        if !dependency_current.contains_key(&(namespace, key.clone())) {
            issues.push(MaterialIssue::NoCurrentRevision {
                dependency_key: key,
            });
        }
    }
    let mut obligation_ids = BTreeSet::new();
    for obligation in &input.obligations {
        if !valid_id(&obligation.id)
            || obligation.description.trim().is_empty()
            || obligation.verifier_scope.trim().is_empty()
            || obligation.verifier_scope.len() > 256
            || obligation.verifier_scope == obligation.description
        {
            return Err(SemanticError::InvalidIdentifier);
        }
        if !obligation_ids.insert(obligation.id.clone()) {
            return Err(SemanticError::DuplicateIdentifier);
        }
        if obligation.sources.is_empty() {
            return Err(SemanticError::InvalidReference);
        }
        for reference in &obligation.sources {
            if !input
                .sources
                .iter()
                .any(|source| source.reference() == *reference)
            {
                return Err(SemanticError::InvalidReference);
            }
            if input.sources.iter().any(|source| {
                source.reference() == *reference
                    && source.availability != SourceAvailability::Complete
            }) {
                issues.push(MaterialIssue::RequiredSourceUnavailable {
                    obligation: obligation.id.clone(),
                    source: reference.id.clone(),
                });
            }
        }
    }
    let dependencies: Vec<_> = dependency_current.into_values().collect();
    let sha256 = json_digest(&(
        &input.subject,
        &input.transition,
        &input.capture_id,
        input.policy.schema_version,
        &input.policy.id,
        &input.sources,
        &input.obligations,
        &dependencies,
        &issues,
    ))?;
    Ok(SourceBasis {
        subject: input.subject,
        transition: input.transition,
        capture_id: input.capture_id,
        schema_version: input.policy.schema_version,
        policy_id: input.policy.id,
        sources: input.sources,
        obligations: input.obligations,
        dependencies,
        issues,
        sha256,
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DependencyChange {
    Changed(DependencyRevision),
    Missing {
        owner_namespace: String,
        dependency_key: String,
    },
    Unavailable {
        owner_namespace: String,
        dependency_key: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DependencyObservation {
    Current(DependencyRevision),
    Missing {
        owner_namespace: String,
        dependency_key: String,
    },
    Unavailable {
        owner_namespace: String,
        dependency_key: String,
    },
}

/// Preserve owner-reported missing and unavailable as distinct outcomes.
/// An omitted observation is unavailable, never evidence that a source vanished.
pub fn revalidate_dependencies(
    expected: &[DependencyRevision],
    observed: &[DependencyObservation],
) -> Result<Vec<DependencyChange>, SemanticError> {
    let mut by_key = BTreeMap::new();
    for observation in observed {
        let key = match observation {
            DependencyObservation::Current(value) => {
                if !valid_digest(&value.sha256) {
                    return Err(SemanticError::InvalidDigest);
                }
                (value.owner_namespace.clone(), value.dependency_key.clone())
            }
            DependencyObservation::Missing {
                owner_namespace,
                dependency_key,
            }
            | DependencyObservation::Unavailable {
                owner_namespace,
                dependency_key,
            } => (owner_namespace.clone(), dependency_key.clone()),
        };
        if !valid_id(&key.0) || !valid_id(&key.1) || by_key.insert(key, observation).is_some() {
            return Err(SemanticError::InvalidSource);
        }
    }
    let mut changes = Vec::new();
    for old in expected {
        match by_key.get(&(old.owner_namespace.clone(), old.dependency_key.clone())) {
            Some(DependencyObservation::Current(value)) if *value == *old => {}
            Some(DependencyObservation::Current(value)) => {
                changes.push(DependencyChange::Changed(value.clone()));
            }
            Some(DependencyObservation::Missing { .. }) => {
                changes.push(DependencyChange::Missing {
                    owner_namespace: old.owner_namespace.clone(),
                    dependency_key: old.dependency_key.clone(),
                });
            }
            Some(DependencyObservation::Unavailable { .. }) | None => {
                changes.push(DependencyChange::Unavailable {
                    owner_namespace: old.owner_namespace.clone(),
                    dependency_key: old.dependency_key.clone(),
                });
            }
        }
    }
    Ok(changes)
}

/// Compare only captured dependencies. An unrelated owner revision is irrelevant.
pub fn compare_dependencies(
    expected: &[DependencyRevision],
    current: &[DependencyRevision],
) -> Vec<DependencyChange> {
    expected
        .iter()
        .filter_map(|old| {
            let found = current.iter().find(|item| {
                item.owner_namespace == old.owner_namespace
                    && item.dependency_key == old.dependency_key
            });
            match found {
                Some(item) if item == old => None,
                Some(item) => Some(DependencyChange::Changed(item.clone())),
                None => Some(DependencyChange::Missing {
                    owner_namespace: old.owner_namespace.clone(),
                    dependency_key: old.dependency_key.clone(),
                }),
            }
        })
        .collect()
}
