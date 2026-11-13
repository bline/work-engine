//! Verbatim serialization of the compiler's enforceability report and the unestablished
//! requirement rows. The store records these; it never changes or satisfies them.
use serde_json::{Value, json};
use work_engine_organization::{EnforceStatus, EnforceabilityReport, OrganizationCandidate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoneRow {
    pub property: String,
    pub scope: String,
    pub requirement: String,
    pub note: String,
}

pub fn none_rows(report: &EnforceabilityReport) -> Vec<NoneRow> {
    report
        .rows
        .iter()
        .filter(|r| r.status == EnforceStatus::None)
        .map(|r| NoneRow {
            property: r.property.clone(),
            scope: r.scope.clone(),
            requirement: r.requirement.clone(),
            note: r.note.to_owned(),
        })
        .collect()
}

/// Every row: property, scope, requirement, status, evidence class, note.
pub fn report_json(report: &EnforceabilityReport) -> Value {
    json!({
        "profile": format!("{:?}", report.profile),
        "executable": report.executable(),
        "rows": report.rows.iter().map(|r| json!({
            "property": r.property, "scope": r.scope, "requirement": r.requirement,
            "status": format!("{:?}", r.status), "evidence_class": format!("{:?}", r.evidence_class),
            "note": r.note,
        })).collect::<Vec<_>>(),
    })
}

/// The unestablished set carried with every fixture result: `None` report rows plus the
/// candidate's separation, authority and component-binding requirements, verbatim. No row of
/// these has a status field that this crate could mark satisfied.
pub fn unestablished_json(
    candidate: &OrganizationCandidate,
    report: &EnforceabilityReport,
) -> Value {
    json!({
        "state": "not established",
        "none_rows": none_rows(report).iter().map(|r| json!({
            "property": r.property, "scope": r.scope, "requirement": r.requirement, "note": r.note,
        })).collect::<Vec<_>>(),
        "separation_requirements": candidate.separation_requirements().iter().map(|s| json!({
            "condition": s.condition.as_str(), "acceptor": s.acceptor, "vantages": s.vantages,
        })).collect::<Vec<_>>(),
        "authority_requirements": candidate.authority_requirements().iter().map(|a| json!({
            "id": a.id.as_str(), "principal": a.principal.as_str(), "permission": a.permission.as_str(),
            "boundaries": a.boundaries.as_ref().map(|b| b.iter().map(|x| x.as_str().to_owned()).collect::<Vec<_>>()),
            "state": "not established",
        })).collect::<Vec<_>>(),
        "component_binding_requirements": candidate.component_binding_requirements().iter().map(|c| json!({
            "component": c.component, "permissions": c.permissions, "state": "not established",
        })).collect::<Vec<_>>(),
    })
}
