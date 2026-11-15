//! Selection of one obligation and its owner-referenced rows from one canonical candidate.
//!
//! Both the production preparation and the fixture consumer start here, so they cannot disagree
//! about which obligation, conditions or enforceability rows are relevant. The view and the
//! enforceability report are derived from the candidate in this function; no caller supplies
//! either one, and nothing here accepts a flag that declares a mapping verified.

use work_engine_organization::{
    CandidateCondition, EnforceStatus, EnforceabilityReport, EnforceabilityRow, Obligation,
    OrganizationCandidate, OrganizationView, ViewError, ViewObligation, enforceability, view,
};

/// The one enforceability property the owner scopes by obligation id; all others are scoped by
/// condition id.
const RESULT_CONTRACT_PROPERTY: &str = "resultContract";

/// A mechanism-table row that has no mechanism, cited by the refusal it causes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedRow {
    pub property: String,
    pub scope: String,
    pub requirement: String,
    pub note: String,
}

/// A request that preparation refuses. No preparation exists for a refused request: there is no
/// partial result, and refusal precedes any effect because this crate has none to perform.
///
/// `View` carries the organization owner's own refusal of the candidate and is distinct from
/// every other variant, which are this crate's refusals.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PreparationRefusal {
    #[error("the organization view refused the candidate: {0}")]
    View(#[from] ViewError),
    #[error("obligation `{0}` is not carried by the candidate")]
    UnknownObligation(String),
    #[error("obligation `{0}` is carried more than once by the candidate")]
    AmbiguousObligation(String),
    #[error("obligation `{0}` has no acceptance condition to derive a context from")]
    NoAcceptanceConditions(String),
    #[error("obligation `{obligation}` does not have exactly one serving vantage in the view")]
    ServingVantage { obligation: String },
    #[error(
        "condition `{condition}` names vantage `{condition_vantage}`, but the view's serving vantage for obligation `{obligation}` is `{served_by}`"
    )]
    ConditionVantageMismatch {
        obligation: String,
        condition: String,
        condition_vantage: String,
        served_by: String,
    },
    #[error(
        "result contract `{requirement}` of obligation `{obligation}` has no mechanism in the mechanism table"
    )]
    UnsupportedResultContract {
        obligation: String,
        requirement: String,
    },
    #[error("mandatory semantics have no mechanism and cannot be conveyed: {rows:?}")]
    UnsupportedMandatorySemantics { rows: Vec<UnsupportedRow> },
    #[error("the exact subject is invalid: {0}")]
    InvalidSubject(String),
}

/// The relevant part of one candidate, selected only through references the owners provide:
/// the obligation id, each condition's `obligation` reference, the view's `serves` result and
/// the report's row scopes. Ambiguity is refused rather than reduced to one reading.
pub(crate) struct Selection<'a> {
    pub view: OrganizationView,
    pub report: EnforceabilityReport,
    pub obligation: &'a Obligation,
    pub conditions: Vec<&'a CandidateCondition>,
    pub served_by: String,
}

impl<'a> Selection<'a> {
    pub(crate) fn new(
        candidate: &'a OrganizationCandidate,
        obligation_id: &str,
    ) -> Result<Self, PreparationRefusal> {
        let view = view(candidate)?;
        let report = enforceability(candidate, candidate.profile());

        let mut matching = candidate
            .obligations()
            .iter()
            .filter(|o| o.id.as_str() == obligation_id);
        let obligation = matching
            .next()
            .ok_or_else(|| PreparationRefusal::UnknownObligation(obligation_id.to_owned()))?;
        if matching.next().is_some() {
            return Err(PreparationRefusal::AmbiguousObligation(
                obligation_id.to_owned(),
            ));
        }

        let conditions: Vec<&CandidateCondition> = candidate
            .conditions()
            .iter()
            .filter(|c| c.obligation.as_str() == obligation_id)
            .collect();
        if conditions.is_empty() {
            return Err(PreparationRefusal::NoAcceptanceConditions(
                obligation_id.to_owned(),
            ));
        }

        let mut view_rows = view
            .obligations
            .iter()
            .filter(|o| o.id.as_str() == obligation_id);
        let served_by = match (view_rows.next(), view_rows.next()) {
            (
                Some(ViewObligation {
                    served_by: Some(vantage),
                    ..
                }),
                None,
            ) => vantage.clone(),
            _ => {
                return Err(PreparationRefusal::ServingVantage {
                    obligation: obligation_id.to_owned(),
                });
            }
        };
        for condition in &conditions {
            if condition.vantage != served_by {
                return Err(PreparationRefusal::ConditionVantageMismatch {
                    obligation: obligation_id.to_owned(),
                    condition: condition.id.as_str().to_owned(),
                    condition_vantage: condition.vantage.clone(),
                    served_by,
                });
            }
        }

        let selection = Selection {
            view,
            report,
            obligation,
            conditions,
            served_by,
        };
        selection.refuse_unsupported()?;
        Ok(selection)
    }

    /// The report rows that belong to the selected obligation (its `resultContract` row) or to
    /// one of its conditions, in the report's own order.
    ///
    /// A row's `scope` is an obligation id for `resultContract` and a condition id for every
    /// other property, and the compiler keeps the two id namespaces apart: an obligation and a
    /// condition (of the same or another obligation) may carry the same id. The owner therefore
    /// has to be decided by the property, never by the scope string alone.
    pub(crate) fn rows(&self) -> impl Iterator<Item = &EnforceabilityRow> {
        self.report.rows.iter().filter(|row| {
            if row.property == RESULT_CONTRACT_PROPERTY {
                row.scope == self.obligation.id.as_str()
            } else {
                self.conditions.iter().any(|c| c.id.as_str() == row.scope)
            }
        })
    }

    /// Refuses when the result contract has no mechanism, or when a row other than `separation`
    /// has none. A `separation` row without a mechanism (OC2) is the known current state and is
    /// carried as a blocker, because the projection can state it faithfully; the other `None`
    /// rows name semantics that the context could not convey, so a context built over them would
    /// silently omit a mandatory requirement.
    fn refuse_unsupported(&self) -> Result<(), PreparationRefusal> {
        if let Some(row) = self.rows().find(|row| {
            row.property == RESULT_CONTRACT_PROPERTY && row.status == EnforceStatus::None
        }) {
            return Err(PreparationRefusal::UnsupportedResultContract {
                obligation: row.scope.clone(),
                requirement: row.requirement.clone(),
            });
        }
        let rows: Vec<UnsupportedRow> = self
            .rows()
            .filter(|row| row.status == EnforceStatus::None && row.property != "separation")
            .map(|row| UnsupportedRow {
                property: row.property.clone(),
                scope: row.scope.clone(),
                requirement: row.requirement.clone(),
                note: row.note.to_owned(),
            })
            .collect();
        if rows.is_empty() {
            Ok(())
        } else {
            Err(PreparationRefusal::UnsupportedMandatorySemantics { rows })
        }
    }
}
