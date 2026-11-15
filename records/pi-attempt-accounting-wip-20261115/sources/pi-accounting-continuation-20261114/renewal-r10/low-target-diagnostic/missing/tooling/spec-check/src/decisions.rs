//! Fixed declaration agreement only. No raw input parsing or financial evaluation.
use crate::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub id: String,
    pub revision: u32,
    pub stage: String,
    pub dimensions: BTreeMap<String, Vec<String>>,
    pub branches: Vec<Branch>,
    pub refs: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Branch {
    pub id: String,
    pub revision: u32,
    pub guard: BTreeMap<String, Vec<String>>,
    pub exclusion: Option<String>,
    pub result: Option<FinancialResult>,
    pub refs: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinancialResult {
    pub pool: String,
    pub low: Option<String>,
    pub high: Option<String>,
    pub components: Vec<String>,
    pub billing: String,
    pub witnesses: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObligationBinding {
    pub temporal: String,
    pub witness: String,
    pub amount_source: String,
    pub preserve: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub id: String,
    pub revision: u32,
    pub type_id: String,
    pub variant: String,
    pub external_state: String,
    pub wire_tag: String,
    pub wire_value: Value,
    pub fields: BTreeMap<String, ProjectionField>,
    pub refs: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionField {
    #[serde(rename = "type")]
    pub ty: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed: Option<Vec<String>>,
}
pub(crate) fn named_type(ty: &str, s: &Spec) -> Option<String> {
    let mut t = ty;
    while let Some(inner) = t
        .strip_prefix("List<")
        .or_else(|| t.strip_prefix("Option<"))
    {
        t = inner.strip_suffix('>')?;
    }
    s.types.iter().any(|x| x.id == t).then(|| t.to_string())
}
pub(crate) fn validate_cross_fields(type_id: &str, variant: &str, v: &Value) -> Result<()> {
    if type_id == "Obligation" {
        let source = v["amount_source"]["variant"]
            .as_str()
            .ok_or("amount source tag missing")?;
        let allowed = match variant {
            "none" => vec!["none"],
            "outstanding" => vec!["reservation_bound"],
            "closed" => vec![
                "settlement_amount",
                "reservation_confirmed",
                "reservation_bound",
            ],
            _ => return Err("unknown obligation state".into()),
        };
        if !allowed.contains(&source) {
            return Err("illegal obligation state + amount_source".into());
        }
    }
    Ok(())
}
pub fn validate_decisions(s: &Spec) -> Result<()> {
    let expected: BTreeMap<String, Vec<String>> = [
        ("lane", vec!["pi_api", "unbudgeted", "subscription"]),
        ("reservation", vec!["none", "retained"]),
        (
            "settlement",
            vec![
                "usage_eligible",
                "usage_ineligible",
                "reservation",
                "upstream_error",
                "error_status",
                "absent",
                "rejected",
                "conflicting",
                "not_applicable",
            ],
        ),
        (
            "valuation",
            vec![
                "complete",
                "incomplete",
                "unpriceable",
                "not_reported",
                "not_applicable",
            ],
        ),
        ("charge", vec!["none", "present"]),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into_iter().map(String::from).collect()))
    .collect();
    if s.decisions.len() != 1 {
        return Err("one financial decision owner required".into());
    }
    let d = &s.decisions[0];
    if d.id != "X-DECISION" || d.stage != "X" || d.dimensions != expected {
        return Err("changed fixed financial dimensions".into());
    }
    unique(d.branches.iter().map(|b| b.id.as_str()), "financial branch")?;
    for b in &d.branches {
        if b.guard.keys().collect::<BTreeSet<_>>() != expected.keys().collect() {
            return Err("incomplete financial guard".into());
        }
        for (k, vs) in &b.guard {
            if vs.is_empty() || vs.iter().any(|v| !expected[k].contains(v)) {
                return Err("illegal financial guard".into());
            }
        }
        if b.exclusion.is_some() == b.result.is_some() {
            return Err("branch must have one exclusion or result".into());
        }
    }
    for point in product(&expected)? {
        let matched: Vec<_> = d
            .branches
            .iter()
            .filter(|b| point.iter().all(|(k, v)| b.guard[k].contains(v)))
            .collect();
        if matched.len() != 1 {
            return Err(format!("financial branch gap/overlap {point:?}"));
        }
        let b = matched[0];
        let lane = point["lane"].as_str();
        let r = point["reservation"].as_str();
        let st = point["settlement"].as_str();
        let v = point["valuation"].as_str();
        let charge = point["charge"].as_str();
        let valid = matches!(
            st,
            "usage_eligible"
                | "usage_ineligible"
                | "reservation"
                | "upstream_error"
                | "error_status"
        );
        // Fixed witness implications: not a raw settlement or valuation implementation.
        let illegal = (lane == "subscription"
            && st == "not_applicable"
            && (r != "none" || v != "not_applicable" || charge != "none"))
            || (lane == "unbudgeted"
                && (r != "none" || st != "not_applicable" || charge != "none"))
            || (valid && (r != "retained" || charge != "present"))
            || (st == "usage_eligible" && v != "complete")
            || (st == "absent" && charge != "none");
        if illegal {
            if b.exclusion.as_ref().is_none_or(|x| x.trim().is_empty()) {
                return Err("missing witness exclusion justification".into());
            }
            continue;
        }
        let out = b.result.as_ref().ok_or("lawful branch excluded")?;
        if lane == "subscription" && st == "not_applicable" {
            if out.pool != "codex"
                || out.low.is_some()
                || out.high.is_some()
                || !out.components.is_empty()
                || out.billing != "unknown"
            {
                return Err("illegal clean subscription result".into());
            }
            continue;
        }
        if out.pool != "pi_api" {
            return Err("dollar evidence cannot select Codex".into());
        }
        let mut available = BTreeSet::from(["zero"]);
        if r == "retained" {
            available.insert("reservation");
        }
        if charge == "present" {
            available.insert("charge");
        }
        if matches!(v, "complete" | "incomplete") {
            available.extend(["usage_low", "usage_high"]);
        }
        if v == "unpriceable" {
            available.extend(["supported_low", "priced_high"]);
        }
        let low = out.low.as_deref().ok_or("missing low")?;
        let required_low = if matches!(v, "complete" | "incomplete") {
            "usage_low"
        } else if v == "unpriceable" {
            "supported_low"
        } else {
            "zero"
        };
        if low != required_low {
            return Err("supported low erased".into());
        }
        if !available.contains(low)
            || out.components.is_empty()
            || out
                .components
                .iter()
                .any(|x| !available.contains(x.as_str()))
        {
            return Err("unavailable monetary reference".into());
        }
        if out
            .components
            .iter()
            .any(|x| matches!(x.as_str(), "usage_low" | "supported_low"))
        {
            return Err("low cannot stand in for high".into());
        }
        let eligible = st == "usage_eligible";
        let exact = eligible || (lane == "unbudgeted" && v == "complete");
        if out.high.as_deref() != Some(if exact { "exact" } else { "at_least" })
            || out.billing != if exact { "established" } else { "unresolved" }
        {
            return Err("result lacks required financial witness".into());
        }
        if eligible && !out.witnesses.iter().any(|w| w == "EligibleUsageBound") {
            return Err("missing eligible witness".into());
        }
        if !eligible && r == "retained" && !out.components.iter().any(|x| x == "reservation") {
            return Err("ineligible branch erases reservation".into());
        }
        for high in ["usage_high", "priced_high", "charge"] {
            if available.contains(high) && !out.components.iter().any(|x| x == high) {
                return Err("available high component erased".into());
            }
        }
    }
    for temporal in ["none", "outstanding", "closed"] {
        for witness in ["eligible_usage", "valid_keepall", "other"] {
            let rows: Vec<_> = s
                .finalization
                .iter()
                .filter(|b| b.temporal == temporal && b.witness == witness)
                .collect();
            if rows.len() != 1 {
                return Err("finalization gap/overlap".into());
            }
            let b = rows[0];
            let source = if temporal == "none" {
                "none"
            } else if temporal == "outstanding" || witness == "other" {
                "reservation_bound"
            } else if witness == "eligible_usage" {
                "settlement_amount"
            } else {
                "reservation_confirmed"
            };
            let preserve: Vec<String> = if temporal == "none" {
                vec!["cause".into()]
            } else if temporal == "closed" {
                vec!["by".into()]
            } else {
                vec![]
            };
            if b.amount_source != source || b.preserve != preserve {
                return Err("illegal finalization result".into());
            }
        }
    }
    if s.finalization.len() != 9 {
        return Err("unknown finalization tuple".into());
    }
    let expected_boundaries = [
        ("C-primary", vec!["E"]),
        ("C-validity", vec!["C-primary", "SETTLEMENT-WITNESS"]),
        ("C-auxiliary", vec!["C-primary", "C-validity"]),
        ("S-consume", vec!["C-validity", "C-auxiliary"]),
    ];
    if s.boundaries.len() != 4 {
        return Err("causal boundary set".into());
    }
    for (id, reads) in expected_boundaries {
        let b = s
            .boundaries
            .iter()
            .find(|b| b.id == id)
            .ok_or("missing causal boundary")?;
        if b.reads.iter().map(String::as_str).collect::<BTreeSet<_>>()
            != reads.into_iter().collect()
        {
            return Err("reversed or interfering causal boundary".into());
        }
    }
    unique(s.projections.iter().map(|p| p.id.as_str()), "projection")?;
    for t in &s.types {
        for variant in &t.variants {
            let ps: Vec<_> = s
                .projections
                .iter()
                .filter(|p| p.type_id == t.id && p.variant == variant.name)
                .collect();
            if ps.len() != 1 {
                return Err("missing/overlapping total projection".into());
            }
            let p = ps[0];
            if p.external_state != variant.name {
                return Err("unmapped external state".into());
            }
            let expected = external_fields(&t.id, &variant.name, &variant.fields);
            if p.fields.keys().collect::<BTreeSet<_>>() != expected.keys().collect() {
                return Err("unmapped external fields".into());
            }
            for (field, (ty, source)) in expected {
                if p.fields[&field].ty != ty || p.fields[&field].source != source {
                    return Err("projection field type/source".into());
                }
            }
            if t.id == "Obligation" {
                let expected = match variant.name.as_str() {
                    "none" => vec!["none"],
                    "outstanding" => vec!["reservation_bound"],
                    "closed" => vec![
                        "settlement_amount",
                        "reservation_confirmed",
                        "reservation_bound",
                    ],
                    _ => return Err("unknown obligation variant".into()),
                };
                if p.fields["amount_source"]
                    .allowed
                    .as_ref()
                    .map(|a| a.iter().map(String::as_str).collect::<Vec<_>>())
                    != Some(expected)
                {
                    return Err("illegal obligation projection constraints".into());
                }
            }
        }
    }
    for p in &s.projections {
        if !s
            .types
            .iter()
            .any(|t| t.id == p.type_id && t.variants.iter().any(|v| v.name == p.variant))
        {
            return Err("orphan projection".into());
        }
    }
    Ok(())
}

pub(crate) fn validate_wire_interface(s: &Spec, i: &Value) -> Result<()> {
    for (type_id, section, field) in [
        ("Valuation", "valuation", "state"),
        ("SettlementDecision", "settlement", "decision"),
        ("Basis", "settlement", "basis"),
    ] {
        let actual: BTreeSet<_> = i[section][field]
            .as_array()
            .ok_or("missing external state mapping")?
            .iter()
            .filter_map(Value::as_str)
            .collect();
        let t = s
            .types
            .iter()
            .find(|t| t.id == type_id)
            .ok_or("missing mapped type")?;
        if actual != t.variants.iter().map(|v| v.name.as_str()).collect() {
            return Err("unmapped external state".into());
        }
    }
    for (type_id, section) in [
        ("RejectReason", "settlement"),
        ("UnpriceableReason", "valuation"),
    ] {
        let actual: BTreeSet<String> = i[section]["reasons"]
            .as_array()
            .ok_or("missing external reasons")?
            .iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect();
        let t = s
            .types
            .iter()
            .find(|t| t.id == type_id)
            .ok_or("missing reason type")?;
        let expected: BTreeSet<String> = t
            .variants
            .iter()
            .map(|v| {
                if v.fields.contains_key("field") {
                    format!("{}(field)", v.name)
                } else {
                    v.name.clone()
                }
            })
            .collect();
        if actual != expected {
            return Err("unmapped external reason".into());
        }
    }
    let replacement_reasons: BTreeSet<_> = s
        .rules
        .iter()
        .filter(|r| r.partition == "F-REPLACEMENT")
        .filter_map(|r| r.outputs.first())
        .filter_map(|o| o.bindings.get("reason"))
        .filter_map(Value::as_str)
        .collect();
    let wire_reasons: BTreeSet<_> = i["replacement"]["reason"]
        .as_array()
        .ok_or("missing replacement reasons")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    if replacement_reasons != wire_reasons {
        return Err("unmapped external replacement reason".into());
    }
    for p in &s.projections {
        let tag = if p.type_id == "Replacement" {
            "eligible"
        } else if p.type_id == "SettlementDecision" {
            "decision"
        } else if [
            "Valuation",
            "Obligation",
            "TemporalObligation",
            "Field",
            "FieldCount",
            "GateVerification",
        ]
        .contains(&p.type_id.as_str())
        {
            "state"
        } else if p.type_id == "Exposure" {
            "pool"
        } else if p.type_id == "High" {
            "kind"
        } else {
            "variant"
        };
        let value = if p.type_id == "Replacement" {
            Value::Bool(p.variant == "eligible")
        } else {
            Value::String(p.variant.clone())
        };
        if p.wire_tag != tag || p.wire_value != value {
            return Err("conflicting external projection tag/value".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod controls {
    use super::*;
    fn fixture() -> Spec {
        serde_json::from_str(include_str!("../../../spec/ck.json")).unwrap()
    }
    #[test]
    fn branch_gaps_overlap_and_unavailable_money() {
        let mut s = fixture();
        s.decisions[0].branches.remove(0);
        assert!(validate(&s).unwrap_err().contains("gap/overlap"));
        let mut s = fixture();
        let mut b = s.decisions[0].branches[0].clone();
        b.id = "overlap".into();
        s.decisions[0].branches.push(b);
        assert!(validate(&s).unwrap_err().contains("gap/overlap"));
        let mut s = fixture();
        let b = s.decisions[0]
            .branches
            .iter_mut()
            .find(|b| {
                b.result.as_ref().is_some_and(|o| o.pool == "pi_api")
                    && b.guard["charge"].contains(&"none".into())
            })
            .unwrap();
        b.result.as_mut().unwrap().components.push("charge".into());
        assert!(validate(&s).unwrap_err().contains("unavailable"));
    }
    #[test]
    fn retention_and_positive_lawful_branches() {
        let mut s = fixture();
        validate(&s).unwrap();
        let branches = &s.decisions[0].branches;
        assert!(
            branches
                .iter()
                .any(|b| b.guard["lane"].contains(&"subscription".into())
                    && b.guard["reservation"].contains(&"retained".into())
                    && b.guard["settlement"].contains(&"absent".into())
                    && b.result.as_ref().is_some_and(
                        |r| r.pool == "pi_api" && r.components.contains(&"reservation".into())
                    ))
        );
        assert!(branches.iter().any(|b| {
            b.guard["settlement"].contains(&"usage_ineligible".into())
                && b.result.as_ref().is_some_and(|r| {
                    r.components.contains(&"reservation".into())
                        && r.high.as_deref() == Some("at_least")
                })
        }));
        assert!(branches.iter().any(|b| {
            b.guard["lane"].contains(&"unbudgeted".into())
                && b.guard["valuation"].contains(&"complete".into())
                && b.result.as_ref().is_some_and(|r| {
                    r.high.as_deref() == Some("exact") && !r.components.contains(&"charge".into())
                })
        }));
        let b = s.decisions[0]
            .branches
            .iter_mut()
            .find(|b| {
                b.guard["settlement"].contains(&"usage_ineligible".into()) && b.result.is_some()
            })
            .unwrap();
        b.result
            .as_mut()
            .unwrap()
            .components
            .retain(|x| x != "reservation");
        assert!(validate(&s).unwrap_err().contains("erases reservation"));
    }
    #[test]
    fn total_projection_controls() {
        let mut s = fixture();
        s.projections
            .retain(|p| !(p.type_id == "Valuation" && p.variant == "not_applicable"));
        assert!(validate(&s).is_err());
        let mut s = fixture();
        let p = s
            .projections
            .iter_mut()
            .find(|p| p.type_id == "Obligation" && p.variant == "none")
            .unwrap();
        p.fields.get_mut("amount_source").unwrap().allowed = Some(vec!["settlement_amount".into()]);
        assert!(validate(&s).is_err());
        let s = fixture();
        let mut i: Value = serde_json::from_str(
            &s.invariants
                .iter()
                .find(|n| n.id == "SERIALIZATION")
                .unwrap()
                .text,
        )
        .unwrap();
        i["valuation"]["reasons"].as_array_mut().unwrap().pop();
        assert!(
            validate_wire_interface(&s, &i)
                .unwrap_err()
                .contains("reason")
        );
    }
    #[test]
    fn causal_direction_and_named_type_invalidation() {
        let mut s = fixture();
        s.boundaries
            .iter_mut()
            .find(|b| b.id == "C-validity")
            .unwrap()
            .reads
            .push("C-auxiliary".into());
        assert!(validate(&s).is_err());
        let s = fixture();
        let deps: Dependencies =
            serde_json::from_str(include_str!("../../../spec/dependencies.json")).unwrap();
        let before = consumer_digests(&s, &deps).unwrap();
        let mut changed = s.clone();
        changed
            .types
            .iter_mut()
            .find(|t| t.id == "ProviderFamily")
            .unwrap()
            .revision += 1;
        let after = consumer_digests(&changed, &deps).unwrap();
        assert_ne!(before["view-schema"], after["view-schema"]);
    }
    #[test]
    fn financial_provisionality_source_and_finalization() {
        let mut s = fixture();
        let r = s
            .invariants
            .iter()
            .find(|n| n.id == "R-FINANCIAL-PROVISIONAL")
            .unwrap();
        for required in [
            "Outstanding",
            "AtLeast",
            "predecessor_obligations",
            "commitment",
            "uncertain/unadmitted",
            "Temporal closure",
        ] {
            assert!(r.text.contains(required));
        }
        s.finalization
            .iter_mut()
            .find(|b| b.temporal == "none")
            .unwrap()
            .amount_source = "settlement_amount".into();
        assert!(validate(&s).is_err());
    }
}

pub(crate) fn validate_replacement_binding(r: &Rule) -> Result<()> {
    let fact = r
        .guard
        .get("replacement_fact")
        .and_then(|x| x.first())
        .ok_or("missing replacement fact")?;
    let reason = match fact.as_str() {
        "no_reservation" => "NoReservation",
        "not_valid_usage" => "NotUsageSettlement",
        "usage_unpriceable" => "UsageUnpriceable",
        "usage_incomplete" => "UsageIncomplete",
        "usage_bound" => "UsageBound",
        _ => return Err("unknown replacement fact".into()),
    };
    let o = r.outputs.first().ok_or("missing replacement result")?;
    if o.type_id != "Replacement"
        || o.variant
            != if fact == "usage_bound" {
                "eligible"
            } else {
                "ineligible"
            }
        || o.bindings.get("reason").and_then(Value::as_str) != Some(reason)
        || o.bindings.len() != 1
    {
        return Err("replacement result/reason disagreement".into());
    }
    Ok(())
}

/// Fixed serializer field map for the actual external constructors.
fn external_fields(
    type_id: &str,
    variant: &str,
    fields: &BTreeMap<String, String>,
) -> BTreeMap<String, (String, String)> {
    let mut m: BTreeMap<_, _> = fields
        .iter()
        .map(|(k, v)| (k.clone(), (v.clone(), k.clone())))
        .collect();
    if type_id == "High" {
        m = [
            ("nano".into(), ("Nano".into(), "bound.nano".into())),
            ("basis".into(), ("String".into(), "bound.basis".into())),
        ]
        .into();
    }
    if type_id == "Valuation" && variant == "valued" {
        m.insert("low".into(), ("Nano".into(), "low.nano".into()));
        m.insert("high".into(), ("Nano".into(), "high.nano".into()));
        m.remove("splits");
        m.insert("derivations".into(), ("List<Json>".into(), "splits".into()));
    }
    if type_id == "Valuation" && variant == "unpriceable" {
        for name in ["supported_low", "priced_high"] {
            m.insert(name.into(), ("Option<Nano>".into(), format!("{name}.nano")));
        }
    }
    m
}
/// Serialize one already constructed typed value into the fixed external wire.
/// No raw classification, valuation or release decisions occur here.
pub fn serialize_external_constructor(
    s: &Spec,
    type_id: &str,
    variant: &str,
    value: &Value,
) -> Result<Value> {
    validate_constructor(s, type_id, variant, value)?;
    let projection = s
        .projections
        .iter()
        .find(|p| p.type_id == type_id && p.variant == variant)
        .ok_or("missing projection")?;
    let mut out = serde_json::Map::new();
    out.insert(projection.wire_tag.clone(), projection.wire_value.clone());
    for (target, map) in &projection.fields {
        let mut current = value;
        for part in map.source.split('.') {
            if current.get("fields").is_some() {
                current = &current["fields"];
            }
            if current.is_null() {
                break;
            }
            current = current
                .get(part)
                .ok_or_else(|| format!("unmapped projection source {}", map.source))?;
        }
        let mapped = if current.is_null() {
            Value::Null
        } else {
            external_field(s, &map.ty, current)?
        };
        out.insert(target.clone(), mapped);
    }
    Ok(Value::Object(out))
}
fn external_field(s: &Spec, ty: &str, value: &Value) -> Result<Value> {
    if let Some(inner) = ty.strip_prefix("Option<").and_then(|x| x.strip_suffix('>')) {
        return if value.is_null() {
            Ok(Value::Null)
        } else {
            external_field(s, inner, value)
        };
    }
    if let Some(inner) = ty.strip_prefix("List<").and_then(|x| x.strip_suffix('>')) {
        return Ok(Value::Array(
            value
                .as_array()
                .ok_or("projection list")?
                .iter()
                .map(|x| external_field(s, inner, x))
                .collect::<Result<_>>()?,
        ));
    }
    if s.types.iter().any(|t| t.id == ty) {
        let variant = value["variant"]
            .as_str()
            .ok_or("projection nested variant")?;
        if [
            "AmountSource",
            "NoneCause",
            "Basis",
            "ProviderFamily",
            "RejectReason",
            "UnpriceableReason",
        ]
        .contains(&ty)
        {
            if value["fields"].as_object().is_some_and(|m| m.is_empty()) {
                return Ok(Value::String(variant.into()));
            }
            if let Some(field) = value["fields"]["field"].as_str() {
                return Ok(Value::String(format!("{variant}({field})")));
            }
        }
        if ty == "Bound" {
            let mut out = serde_json::Map::new();
            out.insert("nano".into(), value["fields"]["nano"].clone());
            out.insert("basis".into(), value["fields"]["basis"].clone());
            return Ok(Value::Object(out));
        }
        return serialize_external_constructor(s, ty, variant, &value["fields"]);
    }
    Ok(value.clone())
}
