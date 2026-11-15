//! Schema and supplied-record structural checks. This module never reads raw producer
//! logs, prices usage, chooses partitions, computes money, or admits external grants.
use crate::{Result, Spec};
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
            o["original_position"].as_u64().unwrap(),
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
    for l in rows(input, "exact_links")? {
        let r = BTreeSet::from([
            name(l, "from_origin_ref")?.into(),
            name(l, "to_origin_ref")?.into(),
        ]);
        subset(&r, &os, "link origin")?;
        grant(input, name(l, "grant_ref")?, "exact_row_coreference", &r)?;
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
    for l in rows(result, "definite_links")? {
        let source = name(l, "from_origin_ref")?;
        let target = name(l, "to_origin_ref")?;
        let g = name(l, "admitted_input_link_ref")?;
        if !rows(input, "exact_links")?.iter().any(|x| {
            x["from_origin_ref"] == source && x["to_origin_ref"] == target && x["grant_ref"] == g
        }) {
            return Err("result link not admitted by exact input link".into());
        }
    }
    let _ = keyed(result, "coverage_decisions", "coverage_id")?;
    let mut covered = BTreeSet::new();
    for c in rows(result, "coverage_decisions")? {
        let mut unit_origins = BTreeSet::new();
        for id in refs(c, "unit_refs")? {
            let u = units.get(&id).ok_or("dangling coverage unit")?;
            unit_origins.extend(refs(u, "origin_refs")?);
        }
        let mut origins = refs(c, "monetary_origin_refs")?;
        origins.extend(refs(c, "anchor_origin_refs")?);
        subset(&origins, &os, "coverage origin")?;
        if c["status"] == "accepted" {
            if origins != unit_origins || origins.is_empty() {
                return Err("coverage is not whole-unit complete".into());
            }
            if !covered.is_disjoint(&origins) {
                return Err("overlapping accepted coverage".into());
            }
            covered.extend(origins.clone());
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
                for primary in validities.values() {
                    let mut dollars = refs(primary, "distinct_reservation_refs")?;
                    dollars.extend(refs(primary, "distinct_settlement_refs")?);
                    if !dollars.is_disjoint(&origins) {
                        return Err("unbudgeted completion overlaps dollar primary evidence".into());
                    }
                }
            }
            let valuations = keyed(result, "valuation_records", "valuation_id")?;
            let valuation = valuations
                .get(name(c, "valuation_ref")?)
                .ok_or("dangling/non-valuation coverage reference")?;
            if valuation["complete"] != true {
                return Err("coverage requires complete valuation witness".into());
            }
            subset(
                &refs(valuation, "usage_origin_refs")?,
                &origins,
                "coverage valuation origin",
            )?;
            if c["route"] == "unbudgeted_completion" {
                let completions = keyed(result, "unbudgeted_completions", "completion_id")?;
                let completion = completions[name(c, "completion_ref")?];
                if !refs(valuation, "usage_origin_refs")?
                    .contains(name(completion, "usage_origin_ref")?)
                {
                    return Err("unbudgeted valuation does not cover exact usage origin".into());
                }
            }
            let resolved = units
                .get(name(c, "result_unit_ref")?)
                .ok_or("dangling result unit")?;
            if refs(resolved, "origin_refs")? != origins || resolved["state"] != "active" {
                return Err("replacement origin coverage mismatch".into());
            }
            for id in refs(c, "unit_refs")? {
                if units[&id]["state"] != "inactive" {
                    return Err("covered hold remains active".into());
                }
            }
            for link_ref in refs(c, "definite_link_refs")? {
                if !rows(result, "definite_links")?
                    .iter()
                    .any(|l| l["admitted_input_link_ref"] == link_ref)
                {
                    return Err("dangling coverage definite-link reference".into());
                }
            }
            for a in refs(c, "anchor_origin_refs")? {
                if !rows(result, "definite_links")?
                    .iter()
                    .any(|l| l["from_origin_ref"] == a || l["to_origin_ref"] == a)
                {
                    return Err("unlinked anchor released".into());
                }
            }
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
