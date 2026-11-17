//! Finite supplied-payload coherence, distinct from independent raw admission.
use crate::{Result, exact_u64, validate_accounting_bundle};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .ok_or_else(|| format!("payload applicability: missing {k}"))
}
fn list<'a>(v: &'a Value, k: &str) -> Result<&'a Vec<Value>> {
    v[k].as_array()
        .ok_or_else(|| format!("payload applicability: missing {k}"))
}
fn set(v: &Value, k: &str) -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for x in list(v, k)? {
        let x = x
            .as_str()
            .ok_or("payload applicability: non-string origin")?;
        if !out.insert(x.to_owned()) {
            return Err("payload applicability: duplicate origin".into());
        }
    }
    Ok(out)
}
fn require(ok: bool, why: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(format!("payload applicability: {why}"))
    }
}
/// Resolves local references without silently choosing between distinct locations.
/// Keys include the exact fragment; values are entire supplied payload records.
pub fn resolve_accounting_payloads(
    root: &Path,
    bundle_path: &Path,
    bundle: &Value,
) -> Result<BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    for input in list(bundle, "documents")?
        .iter()
        .filter(|d| d["kind"] == "accounting_input_proof")
    {
        for g in list(input, "grants")? {
            let reference = text(g, "exact_payload_ref")?;
            if out.contains_key(reference) {
                continue;
            }
            let file = reference.split('#').next().ok_or("missing payload path")?;
            require(!file.is_empty(), "empty payload path")?;
            let mut paths = BTreeSet::new();
            for p in [
                root.join(file),
                bundle_path.parent().unwrap_or(root).join(file),
            ] {
                if p.exists() {
                    paths.insert(p.canonicalize().map_err(|e| e.to_string())?);
                }
            }
            require(paths.len() == 1, "missing or ambiguous payload resolution")?;
            let p = paths.first().unwrap();
            let value = serde_json::from_slice(&fs::read(p).map_err(|e| e.to_string())?)
                .map_err(|e| format!("payload JSON: {e}"))?;
            out.insert(reference.to_owned(), value);
        }
    }
    Ok(out)
}
/// Checks supplied records against resolved payloads. Coherent supplied records do
/// not establish that raw evidence is true or the external guarantee is admitted.
pub fn validate_accounting_bundle_with_payloads(
    bundle: &Value,
    payloads: &BTreeMap<String, Value>,
) -> Result<()> {
    validate_accounting_bundle(bundle)?;
    for input in list(bundle, "documents")?
        .iter()
        .filter(|d| d["kind"] == "accounting_input_proof")
    {
        for g in list(input, "grants")? {
            let reference = text(g, "exact_payload_ref")?;
            let p = payloads
                .get(reference)
                .ok_or("payload applicability: unresolved exact payload")?;
            match text(g, "guarantee_kind")? {
                "initial_reference_scope" | "exact_copy_equivalence" => {
                    let (_, fragment) = reference
                        .split_once('#')
                        .ok_or("payload applicability: missing source fragment")?;
                    require(
                        !fragment.is_empty() && !fragment.contains('#'),
                        "wrong source fragment",
                    )?;
                    if p.get("cases").is_some() {
                        require(
                            g["guarantee_kind"] == "initial_reference_scope",
                            "unsupported derivation guarantee kind",
                        )?;
                        let selected_cases: Vec<_> = list(p, "cases")?
                            .iter()
                            .filter(|c| c["case_id"] == input["lineage_id"])
                            .collect();
                        require(
                            selected_cases.len() == 1,
                            "missing or ambiguous derivation case",
                        )?;
                        let case = selected_cases[0];
                        let expected = format!(
                            "{}/initial_reference_scope_payloads/{}",
                            text(case, "case_id")?,
                            text(g, "grant_id")?
                        );
                        require(fragment == expected, "wrong derivation payload fragment")?;
                        let ps: Vec<_> = list(case, "initial_reference_scope_payloads")?
                            .iter()
                            .filter(|p| p["grant_id"] == g["grant_id"])
                            .collect();
                        require(ps.len() == 1, "missing or ambiguous derivation payload")?;
                        let selected = ps[0];
                        require(
                            g["admission"] == "conditional_fixture"
                                && g["authority_ref"] == selected["authority"],
                            "derivation admission or authority mismatch",
                        )?;
                        let extent = format!("sha256:{}", text(case, "raw_input_sha256")?);
                        require(g["covered_extent_ref"] == extent, "covered extent mismatch")?;
                        let covered = set(g, "covered_origin_refs")?;
                        require(
                            covered == set(selected, "covered_origin_refs")?,
                            "covered origin inventory mismatch",
                        )?;
                        for id in &covered {
                            let origins: Vec<_> = list(input, "origins")?
                                .iter()
                                .filter(|o| o["origin_id"] == *id)
                                .collect();
                            require(origins.len() == 1, "missing derivation origin")?;
                            let o = origins[0];
                            let raw: Vec<_> = list(case, "raw_origins")?
                                .iter()
                                .filter(|r| r["origin_ref"] == *id)
                                .collect();
                            require(
                                raw.len() == 1
                                    && raw[0]["raw_bytes_sha256"] == o["raw_bytes_sha256"],
                                "derivation origin bytes mismatch",
                            )?;
                            let (source, pos) = id
                                .rsplit_once(':')
                                .ok_or("payload applicability: invalid derivation origin")?;
                            require(
                                o["source_origin_id"] == source
                                    && exact_u64(&o["original_position"])
                                        .is_some_and(|p| pos == p.to_string()),
                                "derivation origin position mismatch",
                            )?;
                            require(
                                o["source_revision_ref"] == extent,
                                "origin source extent mismatch",
                            )?;
                            let cs: Vec<_> = list(input, "initial_contracts")?
                                .iter()
                                .filter(|c| c["contract_id"] == o["initial_contract_ref"])
                                .collect();
                            require(cs.len() == 1, "missing derivation initial contract")?;
                            let c = cs[0];
                            require(
                                c["grant_ref"] == g["grant_id"]
                                    && c["scope_id"] == selected["scope_id"]
                                    && c["reference_namespace"] == selected["reference_namespace"]
                                    && c["duplicate_rule"] == "same_origin_only",
                                "derivation initial contract/scope/namespace mismatch",
                            )?;
                        }
                        continue;
                    }
                    let selected: Vec<_> = list(p, "grant_payloads")?
                        .iter()
                        .filter(|v| v["source_origin_id"] == fragment)
                        .collect();
                    require(selected.len() == 1, "missing or ambiguous source fragment")?;
                    let selected = selected[0];
                    require(
                        p["admission"] == g["admission"]
                            && p["authority_ref"] == g["authority_ref"],
                        "admission or authority mismatch",
                    )?;
                    require(
                        p["case_id"] == input["lineage_id"],
                        "payload subject mismatch",
                    )?;
                    let extent = format!("sha256:{}", text(p, "raw_input_sha256")?);
                    require(g["covered_extent_ref"] == extent, "covered extent mismatch")?;
                    let covered = set(g, "covered_origin_refs")?;
                    require(
                        covered == set(selected, "exact_origin_refs")?,
                        "covered origin inventory mismatch",
                    )?;
                    let origins: Vec<_> = list(input, "origins")?
                        .iter()
                        .filter(|o| o["source_origin_id"] == fragment)
                        .collect();
                    let actual: BTreeSet<_> = origins
                        .iter()
                        .map(|o| text(o, "origin_id").map(str::to_owned))
                        .collect::<Result<_>>()?;
                    require(actual == covered, "payload/source inventory mismatch")?;
                    let contracts = list(input, "initial_contracts")?;
                    for o in origins {
                        require(
                            o["source_revision_ref"] == extent,
                            "origin source extent mismatch",
                        )?;
                        let cs: Vec<_> = contracts
                            .iter()
                            .filter(|c| c["contract_id"] == o["initial_contract_ref"])
                            .collect();
                        require(cs.len() == 1, "missing initial contract")?;
                        let c = cs[0];
                        require(
                            list(p, "initial_scope_contracts")?
                                .iter()
                                .filter(|pc| *pc == c)
                                .count()
                                == 1,
                            "initial contract/scope/namespace mismatch",
                        )?;
                        require(
                            c["scope_id"] == selected["immutable_reference_namespace"],
                            "payload namespace mismatch",
                        )?;
                        if g["guarantee_kind"] == "initial_reference_scope" {
                            require(c["grant_ref"] == g["grant_id"], "initial grant mismatch")?;
                        }
                        require(
                            (c["duplicate_rule"] == "scope_exact_line")
                                == (selected["exact_copy_equivalence"] == true),
                            "copy policy mismatch",
                        )?;
                    }
                    if g["guarantee_kind"] == "exact_copy_equivalence" {
                        require(
                            selected["exact_copy_equivalence"] == true,
                            "copy equivalence not admitted",
                        )?;
                    }
                }
                "exact_row_coreference" => {
                    require(!reference.contains('#'), "unsupported exact-link fragment")?;
                    let a = &p["conditional_harness_assumptions"]["exact_row_coreference"];
                    require(a.is_object(), "missing conditional exact-link premise")?;
                    require(
                        g["admission"] == "conditional_fixture",
                        "conditional link admission mismatch",
                    )?;
                    let from = text(a, "from")?;
                    let to = text(a, "to")?;
                    require(
                        g["admitted_payload"]["from"] == from && g["admitted_payload"]["to"] == to,
                        "exact-link pair mismatch",
                    )?;
                    require(
                        set(g, "covered_origin_refs")?
                            == BTreeSet::from([from.to_owned(), to.to_owned()]),
                        "exact-link extent inventory mismatch",
                    )?;
                    let namespace = text(a, "source_namespace")?;
                    require(
                        g["covered_extent_ref"] == format!("{namespace}:two-origins"),
                        "exact-link namespace/extent mismatch",
                    )?;
                    // Only byte addressing: no event interpretation, pricing or financial inference.
                    for endpoint in [from, to] {
                        let os: Vec<_> = list(input, "origins")?
                            .iter()
                            .filter(|o| o["origin_id"] == endpoint)
                            .collect();
                        require(os.len() == 1, "missing exact-link endpoint")?;
                        let o = os[0];
                        let source = text(o, "source_origin_id")?;
                        let (alias, turn) = source
                            .split_once('/')
                            .ok_or("payload applicability: bad source origin")?;
                        let sessions = list(&p["original_inputs"], "sessions")?;
                        let sessions: Vec<_> =
                            sessions.iter().filter(|s| s["alias"] == alias).collect();
                        require(sessions.len() == 1, "missing or ambiguous endpoint source")?;
                        let lines = list(&sessions[0]["logs"][turn], "lines")?;
                        let pos = exact_u64(&o["original_position"])
                            .and_then(|n| usize::try_from(n).ok())
                            .ok_or("payload applicability: invalid endpoint position")?;
                        require(lines.len() == 2 && pos == if endpoint == from {1} else {2}, "conditional endpoint position/extent mismatch")?;
                        let source_digest = format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(&p["original_inputs"]).map_err(|e|e.to_string())?));
                        require(o["source_revision_ref"] == source_digest, "conditional endpoint source extent mismatch")?;
                        let line = pos
                            .checked_sub(1)
                            .and_then(|n| lines.get(n))
                            .and_then(Value::as_str)
                            .ok_or("payload applicability: missing endpoint bytes")?;
                        require(
                            o["raw_bytes_sha256"]
                                == format!("{:x}", Sha256::digest(line.as_bytes())),
                            "exact-link endpoint bytes mismatch",
                        )?;
                    }
                }
                _ => return Err("payload applicability: unsupported finite guarantee kind".into()),
            }
        }
    }
    Ok(())
}
