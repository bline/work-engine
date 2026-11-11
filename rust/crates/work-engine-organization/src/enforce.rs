//! `EnforceabilityReport` (contract r4 4.3 mechanism table, G29, amendment 4 and OC2).
//!
//! One row per condition and property, from an exhaustive match: a property added to
//! `Normative` or a dimension added to `Dimension` without a row fails to compile. The report is
//! data; `executable` is true iff no row is `None`, and it permits nothing.

use crate::derive::OrganizationCandidate;
use crate::ids::Id;
use crate::schema::{
    Continuity, Dimension, EvidenceCustody, Integrity, Mutation, Normative, Profile,
    RealizationConstraint,
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
    /// The condition ID the row belongs to.
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
            "no mechanism evidences a non-empty capability list in v1",
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
    rows.push(
        if observer_class.as_str() == "independent_execution_evidence" {
            row(
                "observerClass",
                observer_class.as_str().to_owned(),
                SupportedWithRealizationMapping,
                EvidenceClass::Recording,
                "the realization admission must state the class-to-identity mapping",
            )
        } else {
            row(
                "observerClass",
                observer_class.as_str().to_owned(),
                NoRow,
                EvidenceClass::NotApplicable,
                "no mapping for this observer class",
            )
        },
    );
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

fn row_for_constraint(scope: &Id, constraint: &RealizationConstraint) -> EnforceabilityRow {
    let (supported, note) = match constraint.dimension {
        Dimension::ProviderFamily => (
            true,
            "any class: checked against the pinned profile by transport validation",
        ),
        Dimension::ExecutionConfinement => (
            constraint
                .allowed
                .iter()
                .any(|class| class.as_str() == "guarded_sandbox"),
            "only guarded_sandbox has a mechanism (pinned guard)",
        ),
    };
    EnforceabilityRow {
        property: format!("realization:{}", constraint.dimension.as_str()),
        scope: scope.as_str().to_owned(),
        requirement: join(constraint.allowed.iter().map(Id::as_str)),
        status: if supported {
            EnforceStatus::Supported
        } else {
            EnforceStatus::None
        },
        evidence_class: if supported {
            EvidenceClass::Recording
        } else {
            EvidenceClass::NotApplicable
        },
        note,
    }
}

/// The enforceability report of a candidate under a profile.
pub fn enforceability(candidate: &OrganizationCandidate, profile: Profile) -> EnforceabilityReport {
    let Profile::FixedSliceReviewV1 = profile;
    let mut rows = Vec::new();
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
