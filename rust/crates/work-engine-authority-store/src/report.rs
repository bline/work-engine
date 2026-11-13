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

/// The `provider_family` classes the frozen requirements declare, read from the requirements JSON
/// itself (every `realizationConstraints` entry whose `dimension` is `provider_family`). Sorted,
/// deduplicated. The pure compiler is not consulted and not changed.
pub fn declared_provider_families(requirements_json: &[u8]) -> Vec<String> {
    fn walk(v: &Value, out: &mut std::collections::BTreeSet<String>) {
        match v {
            Value::Object(m) => {
                if let Some(Value::Array(cs)) = m.get("realizationConstraints") {
                    for c in cs {
                        if c["dimension"] == "provider_family"
                            && let Some(allowed) = c["allowed"].as_array()
                        {
                            out.extend(
                                allowed.iter().filter_map(|a| a.as_str().map(str::to_owned)),
                            );
                        }
                    }
                }
                m.values().for_each(|x| walk(x, out));
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = std::collections::BTreeSet::new();
    if let Ok(v) = serde_json::from_slice::<Value>(requirements_json) {
        walk(&v, &mut out);
    }
    out.into_iter().collect()
}

/// What was declared against what the real Host receipt shows was realized. `state` is
/// `not qualified`, which is distinct from a negative result: the declared class was neither
/// realized nor shown violated by a provider-reported model check, because none ran.
pub fn realization_observed(declared: &[String], receipt: &Value) -> Value {
    json!({
        "dimension": "provider_family",
        "declared": declared,
        "realized": receipt["provider"],
        "model": receipt["model"],
        "state": REALIZATION_STATE,
        "note": "declared provider family not realized by this run; transport validation of a provider-reported model did not run; compiler row Supported/Recording for provider_family not exercised",
    })
}
pub const REALIZATION_STATE: &str = "not qualified";
