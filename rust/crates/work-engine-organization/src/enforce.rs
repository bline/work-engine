//! `EnforceabilityReport` (contract r4 4.3 mechanism table, G29, amendment 4 and OC2).
//!
//! One row per condition and property, and one `resultContract` row per obligation, from an
//! exhaustive match: a property added to `Normative` or a dimension added to `Dimension` without a
//! row fails to compile. The report is data; `executable` is true iff no row is `None`, and it permits nothing.

use crate::derive::OrganizationCandidate;
use crate::ids::Id;
use crate::schema::{
    Continuity, EvidenceCustody, ExecutionConfinement, Integrity, Mutation, Normative, Obligation,
    ObserverClass, Profile, ProviderFamily, RealizationClass, RealizationConstraint,
};

/// A row's status in the v1 mechanism table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforceStatus {
    Supported,
    /// Supported only when the realization admission states the class-to-identity mapping.
    SupportedWithRealizationMapping,
    None,
}

/// Evidence class (DESIGN.md 3.9): C constraint, R recording, O independent observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    Constraint,
    Recording,
    ConstraintAndRecording,
    Observation,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EnforceabilityRow {
    pub property: String,
    /// The condition ID the row belongs to; for the `resultContract` property, the obligation ID.
    pub scope: String,
    pub requirement: String,
    pub status: EnforceStatus,
    pub evidence_class: EvidenceClass,
    pub note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EnforceabilityReport {
    pub profile: Profile,
    pub rows: Vec<EnforceabilityRow>,
}

impl EnforceabilityReport {
    /// True iff no row is `None`. A report field only; it authorizes nothing.
    pub fn executable(&self) -> bool {
        self.rows
            .iter()
            .all(|row| row.status != EnforceStatus::None)
    }
}

fn join(set: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    set.into_iter()
        .map(|item| item.as_ref().to_owned())
        .collect::<Vec<_>>()
        .join(",")
}

fn rows_for_normative(scope: &Id, normative: &Normative) -> Vec<EnforceabilityRow> {
    use EnforceStatus::{None as NoRow, Supported, SupportedWithRealizationMapping};
    let Normative {
        continuity,
        mutation,
        independence,
        capabilities,
        integrity,
        evidence_custody,
        observer_class,
    } = normative;
    let row = |property: &str, requirement: String, status, evidence_class, note: &'static str| {
        EnforceabilityRow {
            property: property.to_owned(),
            scope: scope.as_str().to_owned(),
            requirement,
            status,
            evidence_class,
            note,
        }
    };
    let mut rows = Vec::new();
    rows.push(match continuity {
        Continuity::FreshInitial => row(
            "continuity",
            continuity.as_str().to_owned(),
            Supported,
            EvidenceClass::ConstraintAndRecording,
            "guarded launch starts a fresh session; custody records the session",
        ),
        Continuity::Retained => row(
            "continuity",
            continuity.as_str().to_owned(),
            NoRow,
            EvidenceClass::NotApplicable,
            "no mechanism for retained continuity in v1",
        ),
    });
    rows.push(match mutation {
        Mutation::None => row(
            "mutation",
            mutation.as_str().to_owned(),
            Supported,
            EvidenceClass::ConstraintAndRecording,
            "pinned guard program; custody records mutationAuthorized",
        ),
    });
    rows.push(if capabilities.is_empty() {
        row(
            "capabilities",
            String::new(),
            Supported,
            EvidenceClass::Recording,
            "custody constant: no capabilities",
        )
    } else {
        row(
            "capabilities",
            join(capabilities.iter().map(Id::as_str)),
            NoRow,
            EvidenceClass::NotApplicable,
            "no capability is recognized in v1; validation refuses a non-empty list, so this row is reached only if that check is bypassed",
        )
    });
    rows.push(match integrity {
        Integrity::VerifiedArtifacts => row(
            "integrity",
            integrity.as_str().to_owned(),
            Supported,
            EvidenceClass::Recording,
            "custody artifacts with verified status",
        ),
    });
    rows.push(match evidence_custody {
        EvidenceCustody::ExecutionEvidenceCustody => row(
            "evidenceCustody",
            evidence_custody.as_str().to_owned(),
            Supported,
            EvidenceClass::Observation,
            "custody checked against the trusted bootstrap configuration",
        ),
    });
    rows.push(match observer_class {
        ObserverClass::IndependentExecutionEvidence => row(
            "observerClass",
            observer_class.as_str().to_owned(),
            SupportedWithRealizationMapping,
            EvidenceClass::Recording,
            "the realization admission must state the class-to-identity mapping",
        ),
        // Exists only in cfg(test) builds; fail closed like any unrecognized class.
        #[cfg(test)]
        ObserverClass::TestOnlyAlternate => row(
            "observerClass",
            observer_class.as_str().to_owned(),
            NoRow,
            EvidenceClass::NotApplicable,
            "test-only class: no realization exists",
        ),
    });
    if let Some(from) = independence {
        rows.push(row(
            "independence",
            join(from.iter().map(|source| source.as_str())),
            NoRow,
            EvidenceClass::NotApplicable,
            "any stated independence requirement is non-executable in v1",
        ));
    }
    rows
}

/// The v1 mechanism row of one recognized realization class (exhaustive: a class added to the
/// vocabulary does not compile until it is given a row here). Classes are recognized only if the
/// profile's vocabulary lists them, so none is `None` today.
fn mechanism_of(class: RealizationClass) -> (EnforceStatus, EvidenceClass, &'static str) {
    match class {
        RealizationClass::ProviderFamily(ProviderFamily::Anthropic) => (
            EnforceStatus::Supported,
            EvidenceClass::Recording,
            "provider-reported model checked against the pinned profile by transport validation",
        ),
        RealizationClass::ExecutionConfinement(ExecutionConfinement::GuardedSandbox) => (
            EnforceStatus::Supported,
            EvidenceClass::ConstraintAndRecording,
            "pinned guard prevents the violation; custody records the realization from guard readiness",
        ),
    }
}

/// One row per constraint. `allowed` is a non-empty set of recognized classes of one dimension;
/// the row is `Supported` when some allowed class has a mechanism, and takes the status and
/// evidence class of the first such class (with one class per dimension in v1 this is that class).
fn row_for_constraint(scope: &Id, constraint: &RealizationConstraint) -> EnforceabilityRow {
    let chosen = constraint
        .allowed
        .iter()
        .map(|class| mechanism_of(*class))
        .find(|(status, _, _)| *status != EnforceStatus::None);
    let (status, evidence_class, note) = chosen.unwrap_or((
        EnforceStatus::None,
        EvidenceClass::NotApplicable,
        "no allowed class has a mechanism",
    ));
    EnforceabilityRow {
        property: format!("realization:{}", constraint.dimension.as_str()),
        scope: scope.as_str().to_owned(),
        requirement: join(constraint.allowed.iter().map(|class| class.as_str())),
        status,
        evidence_class,
        note,
    }
}

/// The result contract id and version the v1 mechanism table declares a mechanism for.
const SUPPORTED_RESULT_CONTRACT: (&str, u64) = ("implementation-review", 1);

/// One `resultContract` row for an obligation, identified by the obligation ID. `Supported` means
/// only that the profile declares a mechanism for this contract id and version; Gate R must still
/// bind the concrete validator. Any other id or version is `None`; a row is never dropped.
fn row_for_result_contract(obligation: &Obligation) -> EnforceabilityRow {
    let contract = &obligation.result_contract;
    let supported = (contract.id.as_str(), contract.version) == SUPPORTED_RESULT_CONTRACT;
    EnforceabilityRow {
        property: "resultContract".to_owned(),
        scope: obligation.id.as_str().to_owned(),
        requirement: format!("{}@{}", contract.id, contract.version),
        status: if supported {
            EnforceStatus::Supported
        } else {
            EnforceStatus::None
        },
        evidence_class: if supported {
            // Revision 4 4.3: the result contract is a constraint (validation in transport and
            // the episode reducer), not a recording.
            EvidenceClass::Constraint
        } else {
            EvidenceClass::NotApplicable
        },
        note: if supported {
            "a mechanism is declared for this contract id and version; Gate R must still bind the concrete validator"
        } else {
            "no mechanism is declared for this result contract id or version"
        },
    }
}

/// The enforceability report of a candidate under a profile.
pub fn enforceability(candidate: &OrganizationCandidate, profile: Profile) -> EnforceabilityReport {
    let Profile::FixedSliceReviewV1 = profile;
    let mut rows: Vec<EnforceabilityRow> = candidate
        .obligations()
        .iter()
        .map(row_for_result_contract)
        .collect();
    for condition in candidate.conditions() {
        rows.extend(rows_for_normative(&condition.id, &condition.normative));
        for constraint in &condition.realization_constraints {
            rows.push(row_for_constraint(&condition.id, constraint));
        }
    }
    // Amendment 4 and OC2: separation has no mechanism in v1.
    for separation in candidate.separation_requirements() {
        rows.push(EnforceabilityRow {
            property: "separation".to_owned(),
            scope: separation.condition.as_str().to_owned(),
            requirement: format!(
                "{} is separate from {}",
                separation.acceptor,
                separation.vantages.join(",")
            ),
            status: EnforceStatus::None,
            evidence_class: EvidenceClass::NotApplicable,
            note: "no qualified identity-mapping mechanism exists (OC2)",
        });
    }
    rows.sort_by(|a, b| (&a.scope, &a.property).cmp(&(&b.scope, &b.property)));
    EnforceabilityReport { profile, rows }
}

/// Crate-private checks for rows validation makes unreachable (it refuses a non-empty capability
/// list). Not part of the public surface.
#[cfg(test)]
mod injection_tests {
    use super::*;
    use crate::schema::IndependenceSource;

    fn normative(capabilities: &[&str]) -> Normative {
        Normative {
            continuity: Continuity::FreshInitial,
            mutation: Mutation::None,
            independence: None,
            capabilities: capabilities.iter().map(|c| Id::parse(c).unwrap()).collect(),
            integrity: Integrity::VerifiedArtifacts,
            evidence_custody: EvidenceCustody::ExecutionEvidenceCustody,
            observer_class: ObserverClass::IndependentExecutionEvidence,
        }
    }

    fn capability_row(capabilities: &[&str]) -> EnforceabilityRow {
        let scope = Id::parse("cond-1").unwrap();
        rows_for_normative(&scope, &normative(capabilities))
            .into_iter()
            .find(|row| row.property == "capabilities")
            .unwrap()
    }

    #[test]
    fn a_non_empty_capability_list_that_bypassed_validation_is_none_with_no_evidence_class() {
        let empty = capability_row(&[]);
        assert_eq!(empty.status, EnforceStatus::Supported);
        assert_eq!(empty.evidence_class, EvidenceClass::Recording);
        let stated = capability_row(&["read-only"]);
        assert_eq!(stated.status, EnforceStatus::None);
        assert_eq!(stated.evidence_class, EvidenceClass::NotApplicable);
    }

    #[test]
    fn independence_is_none_even_for_the_only_recognized_source() {
        let mut stated = normative(&[]);
        stated.independence = Some([IndependenceSource::BuilderContext].into_iter().collect());
        let scope = Id::parse("cond-1").unwrap();
        let row = rows_for_normative(&scope, &stated)
            .into_iter()
            .find(|row| row.property == "independence")
            .unwrap();
        assert_eq!(row.status, EnforceStatus::None);
    }
}
