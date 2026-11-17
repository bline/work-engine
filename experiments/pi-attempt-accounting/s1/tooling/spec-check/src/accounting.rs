//! Schema and supplied-record structural checks. This module never reads raw producer
//! logs, prices usage, chooses partitions, computes money, or admits external grants.
use crate::{Result, Spec, canonicalize_integers, exact_u64};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub fn validate_schema(schema: &Value, value: &Value) -> Result<()> {
    let validator =
        jsonschema::draft202012::new(schema).map_err(|e| format!("schema compilation: {e}"))?;
    validator
        .validate(value)
        .map_err(|e| format!("Draft2020-12: {e}"))
}
pub fn validate_accounting_document(value: &Value) -> Result<()> {
    let schema = serde_json::from_slice(include_bytes!("../../../spec/accounting.schema.json"))
        .map_err(|e| e.to_string())?;
    validate_schema(&schema, value)
}
pub fn validate_wire_document(_s: &Spec, value: &Value) -> Result<()> {
    if let Some(values) = value.as_array() {
        for v in values {
            validate_wire_document(_s, v)?;
        }
        return Ok(());
    }
    let schema = serde_json::from_slice(include_bytes!("../../../spec/wire.schema.json"))
        .map_err(|e| e.to_string())?;
    validate_schema(&schema, value)
}
fn rows<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    v[key]
        .as_array()
        .ok_or_else(|| format!("missing array {key}"))
}
fn name<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .ok_or_else(|| format!("missing string {key}"))
}
fn refs(v: &Value, key: &str) -> Result<BTreeSet<String>> {
    let mut set = BTreeSet::new();
    for item in rows(v, key)? {
        let s = item.as_str().ok_or("non-string reference")?;
        if !set.insert(s.into()) {
            return Err(format!("duplicate ref {s}"));
        }
    }
    Ok(set)
}
fn keyed<'a>(v: &'a Value, key: &str, id: &str) -> Result<BTreeMap<String, &'a Value>> {
    let mut out = BTreeMap::new();
    for r in rows(v, key)? {
        let n = name(r, id)?;
        if out.insert(n.into(), r).is_some() {
            return Err(format!("duplicate identity {n}"));
        }
    }
    Ok(out)
}
fn subset(a: &BTreeSet<String>, b: &BTreeSet<String>, label: &str) -> Result<()> {
    if !a.is_subset(b) {
        return Err(format!("dangling/interchanged {label}"));
    }
    Ok(())
}
fn grant(input: &Value, id: &str, kind: &str, origins: &BTreeSet<String>) -> Result<()> {
    let grants = keyed(input, "grants", "grant_id")?;
    let g = grants
        .get(id)
        .ok_or_else(|| format!("dangling grant {id}"))?;
    if name(g, "guarantee_kind")? != kind {
        return Err(format!("interchanged grant kind {id}, expected {kind}"));
    }
    subset(origins, &refs(g, "covered_origin_refs")?, "grant extent")?;
    if g["admission"] == "established"
        && (g.get("producer_revision_ref").is_none() || g.get("incarnation_ref").is_none())
    {
        return Err("established producer grant lacks incarnation/revision".into());
    }
    Ok(())
}
fn validate_input(input: &Value) -> Result<()> {
    let origins = keyed(input, "origins", "origin_id")?;
    let os = origins.keys().cloned().collect();
    let contracts = keyed(input, "initial_contracts", "contract_id")?;
    let _ = keyed(input, "grants", "grant_id")?;
    for g in rows(input, "grants")? {
        subset(&refs(g, "covered_origin_refs")?, &os, "grant origin")?;
    }
    let mut identities = BTreeSet::new();
    for (id, o) in &origins {
        let identity = (
            name(o, "source_origin_id")?,
            exact_u64(&o["original_position"]).ok_or("origin position outside u64 domain")?,
            name(o, "raw_bytes_sha256")?,
        );
        if !identities.insert(identity) {
            return Err("same origin inventoried twice".into());
        }
        let c = contracts
            .get(name(o, "initial_contract_ref")?)
            .ok_or("dangling initial contract")?;
        if c["reference_namespace"] != "unestablished" {
            grant(
                input,
                name(c, "grant_ref")?,
                "initial_reference_scope",
                &BTreeSet::from([id.clone()]),
            )?;
            if c.get("scope_id").is_none() {
                return Err("admitted reference scope missing".into());
            }
        }
        if c["duplicate_rule"] == "scope_exact_line" {
            // Scope grant does not imply exact-copy equivalence. Require a separately
            // stated copy guarantee covering this origin and its immutable scope extent.
            if !rows(input, "grants")?.iter().any(|g| {
                g["guarantee_kind"] == "exact_copy_equivalence"
                    && refs(g, "covered_origin_refs").is_ok_and(|r| r.contains(id))
            }) {
                return Err("copy rule lacks exact-copy guarantee".into());
            }
        }
    }
    for b in rows(input, "identity_bindings")? {
        let origin = name(b, "origin_ref")?;
        if !origins.contains_key(origin) {
            return Err("dangling identity origin".into());
        }
        grant(
            input,
            name(b, "grant_ref")?,
            "producer_attempt_binding",
            &BTreeSet::from([origin.into()]),
        )?;
    }
    let links = keyed(input, "exact_links", "link_id")?;
    let grants = keyed(input, "grants", "grant_id")?;
    for l in links.values() {
        let source = name(l, "from_origin_ref")?;
        let target = name(l, "to_origin_ref")?;
        if source == target {
            return Err("self-link cannot establish co-reference".into());
        }
        let r = BTreeSet::from([source.into(), target.into()]);
        subset(&r, &os, "link origin")?;
        let id = name(l, "grant_ref")?;
        grant(input, id, "exact_row_coreference", &r)?;
        let payload = &grants[id]["admitted_payload"];
        let pairs: Vec<&Value> = if payload.get("from").is_some() {
            vec![payload]
        } else {
            rows(payload, "pairs")?.iter().collect()
        };
        if !pairs
            .iter()
            .any(|p| p["from"] == source && p["to"] == target)
        {
            return Err("input link pair not admitted by exact grant payload".into());
        }
    }
    if let Some(f) = input.get("independence_frame") {
        grant(
            input,
            name(f, "grant_ref")?,
            "physical_bill_independence",
            &BTreeSet::new(),
        )?;
        let slots = keyed(f, "slots", "slot_id")?;
        let mut occurrences = BTreeSet::new();
        for s in slots.values() {
            if !occurrences.insert(name(s, "bill_occurrence_ref")?) {
                return Err("slot IDs duplicate physical bill".into());
            }
            grant(
                input,
                name(s, "grant_ref")?,
                "physical_bill_independence",
                &BTreeSet::new(),
            )?;
        }
        for (list, kind) in [
            ("origin_slots", "origin_slot_binding"),
            ("distinct_origin_slot", "origin_slot_distinctness"),
        ] {
            for b in rows(f, list)? {
                let origin = name(b, "origin_ref")?;
                if !origins.contains_key(origin) || !slots.contains_key(name(b, "slot_id")?) {
                    return Err("dangling slot binding".into());
                }
                grant(
                    input,
                    name(b, "grant_ref")?,
                    kind,
                    &BTreeSet::from([origin.into()]),
                )?;
            }
        }
        let mut binding = BTreeMap::new();
        for b in rows(f, "origin_slots")? {
            let o = name(b, "origin_ref")?;
            if binding.insert(o, name(b, "slot_id")?).is_some() {
                return Err("origin bound to multiple physical slots".into());
            }
        }
        for b in rows(f, "distinct_origin_slot")? {
            if binding.get(name(b, "origin_ref")?).copied() == Some(name(b, "slot_id")?) {
                return Err("contradictory physical frame".into());
            }
        }
    }
    Ok(())
}
fn validate_result(input: &Value, result: &Value) -> Result<()> {
    if result["lineage_id"] != input["lineage_id"] {
        return Err("lineage mismatch".into());
    }
    let inv = keyed(input, "origins", "origin_id")?;
    let os: BTreeSet<_> = inv.keys().cloned().collect();
    let owners = keyed(result, "origins", "origin_ref")?;
    if owners.keys().cloned().collect::<BTreeSet<_>>() != os {
        return Err("inconsistent total origin coverage".into());
    }
    let bases = keyed(result, "bases", "base_id")?;
    let units = keyed(result, "units", "unit_id")?;
    let validities = keyed(result, "primary_validities", "obligation_id")?;
    let components = if result.get("retained_components").is_some() {
        keyed(result, "retained_components", "component_id")?
    } else {
        BTreeMap::new()
    };
    for (id, component) in &components {
        let origin = name(component, "origin_ref")?;
        let owner = owners
            .get(origin)
            .ok_or("dangling retained component origin")?;
        let unit = units
            .get(name(owner, "active_high_unit_id")?)
            .ok_or("dangling retained component unit")?;
        if unit.get("retained_component_refs").is_none()
            || !refs(unit, "retained_component_refs")?.contains(id)
        {
            return Err("retained component omitted by active origin owner".into());
        }
    }
    for unit in units.values() {
        if unit.get("retained_component_refs").is_none() {
            continue;
        }
        let ids = refs(unit, "retained_component_refs")?;
        let ordered: Vec<_> = rows(unit, "retained_component_refs")?
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        if ordered != ids.iter().map(String::as_str).collect::<Vec<_>>() {
            return Err("noncanonical retained component refs".into());
        }
        let origins = refs(unit, "origin_refs")?;
        for id in ids {
            let component = components
                .get(&id)
                .ok_or("dangling unit retained component")?;
            if !origins.contains(name(component, "origin_ref")?) {
                return Err("retained component has wrong unit owner".into());
            }
        }
    }

    let admitted_fact_ids: BTreeSet<String> = rows(input, "grants")?
        .iter()
        .map(|g| g["grant_id"].as_str().unwrap().into())
        .chain(refs(result, "failure_refs")?)
        .collect();
    let mut base_owned = BTreeSet::new();
    for (bid, b) in &bases {
        let members = refs(b, "origin_refs")?;
        subset(&members, &os, "base origin")?;
        if b["break_copies"] == true && b["atomize"] != true {
            return Err("BreakCopies requires Atomize".into());
        }
        for (flag, facts) in [
            ("atomize", "atomize_fact_refs"),
            ("break_copies", "break_copies_fact_refs"),
        ] {
            subset(&refs(b, facts)?, &admitted_fact_ids, "refinement fact")?;
            if b[flag].as_bool().unwrap() != !refs(b, facts)?.is_empty() {
                return Err("refinement flag lacks exact final facts".into());
            }
        }
        for identity in rows(input, "identity_bindings")? {
            if members.contains(name(identity, "origin_ref")?)
                && (b["atomize"] != true
                    || !refs(b, "atomize_fact_refs")?.contains(name(identity, "grant_ref")?))
            {
                return Err("identity refinement omitted from final-fact atomization".into());
            }
        }
        for o in members {
            if !base_owned.insert(o.clone()) || owners[&o]["base_id"] != *bid {
                return Err("inconsistent base ownership".into());
            }
        }
    }
    if base_owned != os {
        return Err("base inventory is incomplete".into());
    }
    // BASE §3 OriginBase: without an admitted scope each origin is its own base. A base grouping
    // several origins needs every member under an established (scope-granted) contract, or its
    // members joined into one component by admitted exact links or shared identity bindings.
    {
        let contracts = keyed(input, "initial_contracts", "contract_id")?;
        let bindings = rows(input, "identity_bindings")?;
        let links = rows(input, "exact_links")?;
        for (bid, b) in &bases {
            let members: Vec<String> = refs(b, "origin_refs")?.into_iter().collect();
            if members.len() < 2 {
                continue;
            }
            let mut scoped = true;
            for o in &members {
                let contract = contracts
                    .get(name(inv[o], "initial_contract_ref")?)
                    .ok_or("dangling initial contract")?;
                scoped &= contract["reference_namespace"] != "unestablished";
            }
            if scoped {
                continue;
            }
            let mut component: BTreeMap<&str, usize> =
                members.iter().enumerate().map(|(i, o)| (o.as_str(), i)).collect();
            let mut join = |a: &str, b: &str| {
                if let (Some(&x), Some(&y)) = (component.get(a), component.get(b)) {
                    for v in component.values_mut() {
                        if *v == y {
                            *v = x;
                        }
                    }
                }
            };
            for l in links {
                join(name(l, "from_origin_ref")?, name(l, "to_origin_ref")?);
            }
            for x in bindings {
                for y in bindings {
                    if x["producer_attempt_id"] == y["producer_attempt_id"] {
                        join(name(x, "origin_ref")?, name(y, "origin_ref")?);
                    }
                }
            }
            if component.values().collect::<BTreeSet<_>>().len() != 1 {
                return Err(format!(
                    "base {bid} groups {} origins under an unestablished contract (or without an admitted scope) with no admitted exact link or identity binding joining them",
                    members.len()
                ));
            }
        }
    }
    let mut active_owned = BTreeSet::new();
    for (uid, u) in &units {
        let members = refs(u, "origin_refs")?;
        subset(&members, &os, "unit origin")?;
        if u["state"] == "active" {
            for o in members {
                if !active_owned.insert(o.clone()) || owners[&o]["active_high_unit_id"] != *uid {
                    return Err("origin lacks unique active high owner".into());
                }
            }
        }
        if let Some(s) = u.get("successor_ref") {
            if !units.contains_key(s.as_str().unwrap()) || s == uid {
                return Err("dangling/cyclic successor".into());
            }
        }
    }
    if active_owned != os {
        return Err("active ownership inventory incomplete".into());
    }
    let memberships = if result.get("primary_memberships").is_some() {
        keyed(result, "primary_memberships", "obligation_id")?
    } else {
        BTreeMap::new()
    };
    if memberships.keys().collect::<Vec<_>>() != validities.keys().collect::<Vec<_>>() {
        return Err("primary validity/membership inventory mismatch".into());
    }
    let mut primary_owner = BTreeMap::new();
    let mut primary_sets = BTreeMap::new();
    let mut usage_sets = BTreeMap::new();
    for (obligation, m) in &memberships {
        let mut all = BTreeSet::new();
        for (classes, distinct) in [
            ("reservation_classes", "distinct_reservation_refs"),
            ("settlement_classes", "distinct_settlement_refs"),
        ] {
            let mut representatives = BTreeSet::new();
            for class in rows(m, classes)? {
                let members = refs(class, "origin_refs")?;
                subset(&members, &os, "primary membership origin")?;
                let ordered: Vec<_> = rows(class, "origin_refs")?
                    .iter()
                    .map(|v| v.as_str().unwrap())
                    .collect();
                if ordered != members.iter().map(String::as_str).collect::<Vec<_>>()
                    || members.first().map(String::as_str)
                        != Some(name(class, "representative_origin_ref")?)
                {
                    return Err(
                        "primary class is not canonical or representative is not minimum member"
                            .into(),
                    );
                }
                if members.len() > 1 {
                    let first = inv[members.first().unwrap()];
                    let contracts = keyed(input, "initial_contracts", "contract_id")?;
                    let contract = contracts[name(first, "initial_contract_ref")?];
                    if contract["duplicate_rule"] != "scope_exact_line"
                        || contract.get("scope_id").is_none()
                        || !members.iter().all(|o| {
                            let c = contracts.get(inv[o]["initial_contract_ref"].as_str().unwrap());
                            inv[o]["raw_bytes_sha256"] == first["raw_bytes_sha256"]
                                && c.is_some_and(|c| {
                                    c["duplicate_rule"] == "scope_exact_line"
                                        && c["scope_id"] == contract["scope_id"]
                                })
                        })
                        || !rows(input, "grants")?.iter().any(|g| {
                            g["guarantee_kind"] == "exact_copy_equivalence"
                                && refs(g, "covered_origin_refs")
                                    .is_ok_and(|r| members.is_subset(&r))
                        })
                    {
                        return Err("primary class lacks admitted exact-copy equivalence".into());
                    }
                }
                representatives.insert(name(class, "representative_origin_ref")?.to_owned());
                for o in members {
                    if primary_owner
                        .insert(o.clone(), obligation.clone())
                        .is_some()
                    {
                        return Err(
                            "primary origin belongs to multiple classes/roles/obligations".into(),
                        );
                    }
                    all.insert(o);
                }
            }
            if representatives != refs(validities[obligation], distinct)? {
                return Err("primary validity representatives differ from membership".into());
            }
        }
        let usage = refs(m, "usage_origin_refs")?;
        subset(&usage, &os, "primary usage membership")?;
        primary_sets.insert(obligation.clone(), all);
        usage_sets.insert(obligation.clone(), usage);
    }
    for v in validities.values() {
        if v["validity_result"] == "valid"
            && (refs(v, "distinct_reservation_refs")?.len() != 1
                || refs(v, "distinct_settlement_refs")?.len() != 1)
        {
            return Err("valid witness lacks unique primary evidence".into());
        }
        subset(
            &refs(v, "distinct_reservation_refs")?,
            &os,
            "reservation origin",
        )?;
        subset(
            &refs(v, "distinct_settlement_refs")?,
            &os,
            "settlement origin",
        )?;
    }
    for c in rows(result, "financial_candidates")? {
        for k in ["usage_origin_ref", "context_origin_ref"] {
            if !inv.contains_key(name(c, k)?) {
                return Err("dangling candidate origin".into());
            }
        }
    }
    let links = keyed(result, "definite_links", "link_id")?;
    let input_links = keyed(input, "exact_links", "link_id")?;
    for l in links.values() {
        let admitted = input_links
            .get(name(l, "admitted_input_link_ref")?)
            .ok_or("result link not admitted by exact input link identity")?;
        if l["from_origin_ref"] != admitted["from_origin_ref"]
            || l["to_origin_ref"] != admitted["to_origin_ref"]
        {
            return Err("result link endpoints differ from exact admitted link".into());
        }
    }
    let _ = keyed(result, "coverage_decisions", "coverage_id")?;
    let mut covered = BTreeSet::new();
    let mut retired_by = BTreeMap::new();
    for c in rows(result, "coverage_decisions")? {
        let mut unit_origins = BTreeSet::new();
        for id in refs(c, "unit_refs")? {
            let u = units.get(&id).ok_or("dangling coverage unit")?;
            unit_origins.extend(refs(u, "origin_refs")?);
        }
        let monetary = refs(c, "monetary_origin_refs")?;
        let anchors = refs(c, "anchor_origin_refs")?;
        if !monetary.is_disjoint(&anchors) {
            return Err("monetary and anchor coverage overlap".into());
        }
        let mut origins = monetary.clone();
        origins.extend(anchors.clone());
        subset(&origins, &os, "coverage origin")?;
        let mut component_origins = origins.clone();
        for candidate in rows(result, "financial_candidates")? {
            if origins.contains(name(candidate, "usage_origin_ref")?) {
                component_origins.insert(name(candidate, "context_origin_ref")?.to_owned());
            }
        }
        for id in refs(c, "retained_component_refs")? {
            let component = components
                .get(&id)
                .ok_or("dangling coverage retained component")?;
            if !component_origins.contains(name(component, "origin_ref")?) {
                return Err("retained component has wrong coverage owner".into());
            }
        }

        if c["status"] == "accepted" {
            if origins != unit_origins || origins.is_empty() {
                return Err("coverage is not whole-unit complete".into());
            }
            if !covered.is_disjoint(&origins) {
                return Err("overlapping accepted coverage".into());
            }
            covered.extend(origins.clone());
            let expected_usage;
            let mut request_origins;
            if c["route"] == "budgeted" {
                let validity = validities
                    .get(name(c, "validity_ref")?)
                    .ok_or("dangling/non-validity coverage ref")?;
                if validity["validity_result"] != "valid" {
                    return Err("coverage uses invalid primary witness".into());
                }
                if c["obligation_id"] != c["validity_ref"] {
                    return Err("coverage obligation/validity reference mismatch".into());
                }
                let obligation = name(c, "obligation_id")?;
                subset(
                    &primary_sets[obligation],
                    &monetary,
                    "coverage missing selected primary inventory",
                )?;
                if origins
                    .iter()
                    .any(|o| primary_owner.get(o).is_some_and(|id| id != obligation))
                {
                    return Err("coverage includes another obligation primary origin".into());
                }
                expected_usage = usage_sets[obligation].clone();
                subset(
                    &expected_usage,
                    &origins,
                    "coverage missing selected usage inventory",
                )?;
                request_origins = primary_sets[obligation].clone();
                request_origins.extend(expected_usage.clone());
            } else {
                if c.get("validity_ref").is_some() || c.get("obligation_id").is_some() {
                    return Err(
                        "unbudgeted completion cannot relabel dollar primary validity".into(),
                    );
                }
                let completions = keyed(result, "unbudgeted_completions", "completion_id")?;
                let completion = completions
                    .get(name(c, "completion_ref")?)
                    .ok_or("dangling unbudgeted completion")?;
                let applicability =
                    keyed(result, "unbudgeted_applicabilities", "applicability_id")?;
                let applicability = applicability
                    .get(name(completion, "applicability_ref")?)
                    .ok_or("dangling computed unbudgeted applicability")?;
                let usage = name(completion, "usage_origin_ref")?;
                if !origins.contains(usage)
                    || applicability["usage_origin_ref"] != usage
                    || completion["valuation_ref"] != c["valuation_ref"]
                {
                    return Err(
                        "unbudgeted applicability/completion origin or valuation mismatch".into(),
                    );
                }
                expected_usage = refs(completion, "usage_origin_refs")?;
                if !expected_usage.contains(usage) {
                    return Err("completion usage inventory omits its usage origin".into());
                }
                subset(
                    &expected_usage,
                    &origins,
                    "completion usage outside coverage",
                )?;
                request_origins = expected_usage.clone();
                for dollars in primary_sets.values() {
                    if !dollars.is_disjoint(&origins) {
                        return Err("unbudgeted completion overlaps dollar primary evidence".into());
                    }
                }
            }
            let selected_inventory: BTreeSet<String> = if c["route"] == "budgeted" {
                primary_sets[name(c, "obligation_id")?]
                    .union(&expected_usage)
                    .cloned()
                    .collect()
            } else {
                expected_usage.clone()
            };
            if monetary != selected_inventory {
                return Err(
                    "coverage monetary origins differ from the selected primary/usage inventory"
                        .into(),
                );
            }
            let valuations = keyed(result, "valuation_records", "valuation_id")?;
            let valuation = valuations
                .get(name(c, "valuation_ref")?)
                .ok_or("dangling/non-valuation coverage reference")?;
            if valuation["complete"] != true {
                return Err("coverage requires complete valuation witness".into());
            }
            if refs(valuation, "usage_origin_refs")? != expected_usage {
                return Err(
                    "coverage valuation usage differs from selected membership inventory".into(),
                );
            }
            let resolved = units
                .get(name(c, "result_unit_ref")?)
                .ok_or("dangling result unit")?;
            for id in refs(c, "unit_refs")? {
                retired_by.insert(id, name(c, "result_unit_ref")?.to_owned());
            }
            if refs(resolved, "origin_refs")? != origins || resolved["state"] != "active" {
                return Err("replacement origin coverage mismatch".into());
            }
            for id in refs(c, "unit_refs")? {
                if units[&id]["state"] != "inactive" {
                    return Err("covered hold remains active".into());
                }
            }
            let cited = refs(c, "definite_link_refs")?;
            for link_ref in &cited {
                let l = links
                    .get(link_ref)
                    .ok_or("dangling coverage definite-link identity")?;
                let source = name(l, "from_origin_ref")?;
                let target = name(l, "to_origin_ref")?;
                let joins = |anchor: &str, request: &str| {
                    anchors.contains(anchor)
                        && request != anchor
                        && request_origins.contains(request)
                        && origins.contains(request)
                };
                if !(joins(source, target) || joins(target, source)) {
                    return Err(format!(
                        "cited definite link {link_ref} does not join a covered anchor to the covered request"
                    ));
                }
            }
            for a in &anchors {
                if !cited.iter().any(|id| {
                    let l = links[id];
                    let source = l["from_origin_ref"].as_str().unwrap();
                    let target = l["to_origin_ref"].as_str().unwrap();
                    (source == a.as_str()
                        && target != a.as_str()
                        && request_origins.contains(target)
                        && origins.contains(target))
                        || (target == a.as_str()
                            && source != a.as_str()
                            && request_origins.contains(source)
                            && origins.contains(source))
                }) {
                    return Err(
                        "released anchor lacks cited exact link to selected covered request".into(),
                    );
                }
            }
        }
    }
    for (id, u) in &units {
        if u["state"] == "inactive"
            && !retired_by
                .get(id)
                .is_some_and(|s| u.get("successor_ref").and_then(Value::as_str) == Some(s))
        {
            return Err(format!(
                "inactive unit {id} is not retired into its successor by an accepted coverage"
            ));
        }
    }
    for l in rows(result, "low_rollups")? {
        let all = refs(l, "alternative_origin_refs")?;
        subset(&all, &os, "low alternative")?;
        subset(&refs(l, "seed_origin_refs")?, &all, "usage seed")?;
        let mut slots = BTreeSet::new();
        for c in rows(l, "credits")? {
            let frame = input
                .get("independence_frame")
                .ok_or("credit lacks physical frame")?;
            let slot = name(c, "slot_ref")?;
            if !slots.insert(slot) || !rows(frame, "slots")?.iter().any(|s| s["slot_id"] == slot) {
                return Err("duplicate/dangling physical credit slot".into());
            }
            let excluded: BTreeSet<_> = rows(frame, "distinct_origin_slot")?
                .iter()
                .filter(|b| b["slot_id"] == slot)
                .map(|b| b["origin_ref"].as_str().unwrap().to_owned())
                .collect();
            let complete = all.difference(&excluded).cloned().collect();
            if refs(c, "alternative_origin_refs")? != complete {
                return Err("incomplete physical-credit alternatives".into());
            }
            if !rows(frame, "origin_slots")?.iter().any(|b| {
                b["slot_id"] == slot
                    && refs(l, "seed_origin_refs")
                        .is_ok_and(|s| s.contains(b["origin_ref"].as_str().unwrap()))
            }) {
                return Err("slot has no positively bound usage seed".into());
            }
        }
    }
    Ok(())
}
/// Checks a closed supplied document set. Opaque external raw/valuation locators
/// remain references only; this establishes neither their truth nor admission.
pub fn validate_accounting_bundle(bundle: &Value) -> Result<()> {
    let bundle = &canonicalize_integers(bundle);
    let docs = rows(bundle, "documents")?;
    let mut by_id = BTreeMap::new();
    for d in docs {
        validate_accounting_document(d)?;
        if by_id.insert(name(d, "id")?, d).is_some() {
            return Err("duplicate document identity".into());
        }
    }
    for d in docs {
        match name(d, "kind")? {
            "accounting_input_proof" => validate_input(d)?,
            "accounting_result" => {
                let input = by_id
                    .get(name(d, "input_proof_ref")?)
                    .ok_or("dangling input proof")?;
                if input["kind"] != "accounting_input_proof" {
                    return Err("interchanged evidence kind".into());
                }
                validate_result(input, d)?;
                for predecessor in refs(&input["continuity"], "predecessor_subject_refs")? {
                    let Some(previous) = by_id.get(predecessor.as_str()) else {
                        if d["continuity_result"] != "unestablished" {
                            return Err(
                                "continuity claimed without exact predecessor evidence".into()
                            );
                        }
                        continue;
                    };
                    if previous["kind"] != "accounting_input_proof" {
                        return Err("continuity predecessor evidence kind".into());
                    }
                    let now = keyed(input, "origins", "origin_id")?;
                    for (id, origin) in keyed(previous, "origins", "origin_id")? {
                        if now.get(&id).copied() != Some(origin) {
                            return Err("continuity drops or rewrites origin".into());
                        }
                    }
                    let now = keyed(input, "initial_contracts", "contract_id")?;
                    for (id, contract) in keyed(previous, "initial_contracts", "contract_id")? {
                        if now.get(&id).copied() != Some(contract) {
                            return Err("continuity rewrites initial reference contract".into());
                        }
                    }
                    let now = keyed(input, "grants", "grant_id")?;
                    for (id, fact) in keyed(previous, "grants", "grant_id")? {
                        if now.get(&id).copied() != Some(fact) {
                            return Err("continuity drops or rewrites admitted fact".into());
                        }
                    }
                }
            }
            "accounting_expected_assertion" => {
                for id in refs(d, "exact_subject_refs")? {
                    if !by_id.contains_key(id.as_str()) {
                        return Err("dangling expected exact subject".into());
                    }
                }
            }
            _ => return Err("unknown evidence kind".into()),
        }
    }
    Ok(())
}
