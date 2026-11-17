//! Finite supplied-record consumer. No raw financial evaluator or grant admission.
use crate::{Result, exact_u64, validate_accounting_bundle};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
fn read(p: &Path) -> Result<Value> {
    serde_json::from_slice(&fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?)
        .map_err(|e| e.to_string())
}
fn arr<'a>(v: &'a Value, k: &str) -> Result<&'a Vec<Value>> {
    v[k].as_array().ok_or_else(|| format!("missing array {k}"))
}
fn txt<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str().ok_or_else(|| format!("missing string {k}"))
}
fn set(v: &Value, k: &str) -> Result<BTreeSet<String>> {
    arr(v, k)?
        .iter()
        .map(|v| v.as_str().map(str::to_owned).ok_or("non-string ref".into()))
        .collect()
}
pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
// Python's established digest convention uses ensure_ascii=True.
pub(crate) fn canonical(v: &Value) -> Result<String> {
    let s = serde_json::to_string(v).map_err(|e| e.to_string())?;
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii() {
            out.push(c)
        } else {
            for n in c.encode_utf16(&mut [0; 2]) {
                out.push_str(&format!("\\u{n:04x}"))
            }
        }
    }
    Ok(out)
}
fn keyed(v: &Value, k: &str, id: &str) -> Result<BTreeMap<String, Value>> {
    let mut m = BTreeMap::new();
    for x in arr(v, k)? {
        if m.insert(txt(x, id)?.into(), x.clone()).is_some() {
            return Err(format!("duplicate {k}/{id}"));
        }
    }
    Ok(m)
}
fn part(cid: &str) -> &str {
    if ["P130-", "P132-", "P133-"]
        .iter()
        .any(|p| cid.starts_with(p))
    {
        "A"
    } else {
        "B"
    }
}
fn claim_key(c: &Value) -> Result<String> {
    Ok(format!(
        "{}/{}/{}/{}",
        txt(c, "case_id")?,
        txt(c, "evaluation")?,
        txt(c, "objective")?,
        txt(c, "label")?
    ))
}
fn claims(root: &Path) -> Result<BTreeMap<String, Value>> {
    let mut m: BTreeMap<String, Value> = BTreeMap::new();
    for path in [
        "oracle/R8-LOW-ASSERTIONS.json",
        "oracle/r8-completion-A/LOW-ASSERTIONS.json",
        "oracle/r8-completion-B/LOW-ASSERTIONS.json",
    ] {
        if !root.join(path).exists() {
            continue;
        }
        for c in arr(&read(&root.join(path))?, "assertions")? {
            let k = claim_key(c)?;
            if let Some(old) = m.get(&k) {
                if old["expected_low"] != c["expected_low"] {
                    return Err(format!("conflicting independent assertion {k}"));
                }
            }
            m.insert(k, c.clone());
        }
    }
    Ok(m)
}
pub fn validate_corpus_case(
    root: &Path,
    c: &Value,
    claims: &BTreeMap<String, Value>,
) -> Result<usize> {
    let cid = txt(c, "case_id")?;
    let rel = format!("oracle/r8-completion-{}/{cid}.bundle.json", part(cid));
    let b = read(&root.join(&rel))?;
    validate_accounting_bundle(&b)?;
    let docs = keyed(&b, "documents", "id")?;
    let input = docs
        .get(&format!("input:{cid}"))
        .ok_or("missing input proof")?;
    let result = docs.get(&format!("result:{cid}")).ok_or("missing result")?;
    if input["kind"] != "accounting_input_proof"
        || result["kind"] != "accounting_result"
        || input["lineage_id"] != cid
        || result["lineage_id"] != cid
        || result["input_proof_ref"] != input["id"]
    {
        return Err("wrong subject/kind/lineage/input reference".into());
    }
    let digest = hash(canonical(&c["inputs"])?.as_bytes());
    let origins = keyed(input, "origins", "origin_id")?;
    let mut raw = BTreeMap::new();
    for session in arr(&c["inputs"], "sessions")? {
        for (turn, log) in session["logs"].as_object().ok_or("missing logs")? {
            for (i, line) in arr(log, "lines")?.iter().enumerate() {
                let source = format!("{}/{turn}", txt(session, "alias")?);
                let line_bytes = if let Some(s) = line.as_str() {
                    s.to_owned()
                } else {
                    canonical(line)?
                };
                raw.insert(
                    format!("{source}:{}", i + 1),
                    (
                        source,
                        u64::try_from(i + 1).map_err(|e| e.to_string())?,
                        hash(line_bytes.as_bytes()),
                    ),
                );
            }
        }
    }
    for (oid, o) in &origins {
        let a = raw.get(oid).ok_or("origin not in actual raw subject")?;
        if txt(o, "source_origin_id")? != a.0
            || exact_u64(&o["original_position"]) != Some(a.1)
            || txt(o, "raw_bytes_sha256")? != a.2
            || txt(o, "source_revision_ref")? != format!("sha256:{digest}")
        {
            return Err(format!("wrong raw origin/input digest binding {oid}"));
        }
    }
    let attempts = keyed(&c["expected"], "attempts", "key")?;
    let units: BTreeMap<_, _> = keyed(result, "units", "unit_id")?
        .into_iter()
        .filter(|(_, u)| u["state"] == "active")
        .collect();
    if attempts.keys().collect::<Vec<_>>() != units.keys().collect::<Vec<_>>() {
        return Err("total active unit/attempt coverage mismatch".into());
    }
    let wire_schema: Value =
        serde_json::from_slice(include_bytes!("../../../spec/wire.schema.json"))
            .map_err(|e| e.to_string())?;
    for (key, a) in &attempts {
        let projection: serde_json::Map<String, Value> = a
            .as_object()
            .ok_or("attempt object")?
            .iter()
            .filter(|(k, _)| wire_schema["properties"].get(k).is_some())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        crate::validate_schema(&wire_schema, &Value::Object(projection))?;
        let u = &units[key];
        if a["accounting_result_ref"] != format!("{rel}#result:{cid}")
            || u["exposure_ref"] != format!("case:{cid}#attempt:{key}")
            || set(u, "origin_refs")? != set(a, "evidence")?
        {
            return Err(format!(
                "missing/dangling/wrong unit/result/exposure reference {key}"
            ));
        }
    }
    let validities = keyed(result, "primary_validities", "obligation_id")?;
    let valuations = if result.get("valuation_records").is_some() {
        keyed(result, "valuation_records", "valuation_id")?
    } else {
        BTreeMap::new()
    };
    for (key, a) in &attempts {
        if a["exposure"]["pool"] != "pi_api" || a["exposure"]["high"]["kind"] != "exact" {
            continue;
        }
        let coverage = arr(result, "coverage_decisions")?
            .iter()
            .find(|v| v["status"] == "accepted" && v["result_unit_ref"] == *key)
            .ok_or_else(|| format!("Exact wire unit lacks complete F witness {key}"))?;
        let valuation = valuations
            .get(txt(coverage, "valuation_ref")?)
            .ok_or("missing F valuation")?;
        if valuation["wire_ref"] != format!("case:{cid}#attempt:{key}/valuation")
            || a["valuation"]["complete"] != true
        {
            return Err(format!("F valuation wire/reference disagreement {key}"));
        }
        if coverage["route"] == "budgeted" {
            let validity = validities
                .get(txt(coverage, "validity_ref")?)
                .ok_or("missing F validity")?;
            if a["settlement"]["decision"] != "valid"
                || a["replacement"]["eligible"] != true
                || set(validity, "distinct_reservation_refs")?.len() != 1
                || set(validity, "distinct_settlement_refs")?.len() != 1
            {
                return Err(format!("F validity wire/reference disagreement {key}"));
            }
        } else if a["settlement"]["decision"] != "not_applicable"
            || a["obligation"]["state"] != "none"
        {
            return Err("unbudgeted F applicability wire/reference disagreement".into());
        }
    }
    let lows = keyed(result, "low_rollups", "scope_ref")?;
    let mut required = BTreeSet::new();
    for e in arr(c, "evaluations")? {
        for o in arr(&e["expected"], "objectives")? {
            for label in ["owned_closed", "owned_outstanding"] {
                let scope = format!("{cid}/{}/{}/{label}", txt(e, "name")?, txt(o, "objective")?);
                required.insert(scope.clone());
                let low = lows
                    .get(&scope)
                    .ok_or_else(|| format!("low scope REQUIRED/UNMET {scope}"))?;
                if o[label]["supported_low_evidence_ref"] != format!("{rel}#{scope}") {
                    return Err(format!("missing/dangling/wrong scope reference {scope}"));
                }
                let claim = claims
                    .get(&scope)
                    .ok_or_else(|| format!("independent low scope REQUIRED/UNMET {scope}"))?;
                if claim["raw_input_sha256"] != digest
                    || claim["expected_low"] != low["selected_low"]
                    || claim["expected_low"] != o[label]["low"]
                {
                    return Err(format!(
                        "independent raw/consistent-wrong low binding {scope}"
                    ));
                }
                let mut owned = BTreeSet::new();
                for at in arr(&e["expected"], "attribution")? {
                    if at["ownership"] == "owned" && at["objective"] == o["objective"] {
                        let key = txt(at, "attempt")?;
                        let a = attempts.get(key).ok_or("attribution unknown unit")?;
                        if a["exposure"]["pool"] == "pi_api"
                            && (a["obligation"]["state"] == "outstanding")
                                == (label == "owned_outstanding")
                        {
                            owned.insert(key.to_owned());
                        }
                    }
                }
                if set(claim, "owned_unit_refs")? != owned {
                    return Err(format!(
                        "independent owned scope membership mismatch {scope}"
                    ));
                }
                for field in ["seed_origin_refs", "alternative_origin_refs"] {
                    if set(claim, field)? != set(low, field)?
                        || !set(low, field)?.is_subset(&origins.keys().cloned().collect())
                    {
                        return Err(format!(
                            "independent low membership mismatch {scope}/{field}"
                        ));
                    }
                }
                if low["basis"]
                    != if claim["expected_low"] == "0" {
                        "none"
                    } else {
                        "usage_valuation"
                    }
                {
                    return Err(format!("wrong low basis {scope}"));
                }
            }
        }
    }
    if lows.keys().cloned().collect::<BTreeSet<_>>() != required {
        return Err("total low scope coverage mismatch".into());
    }
    Ok(required.len())
}
pub fn validate_corpus_documents(root: &Path, partition: Option<&str>) -> Result<(usize, usize)> {
    if partition.is_some_and(|p| p != "A" && p != "B") {
        return Err("partition must be A or B".into());
    }
    let claims = claims(root)?;
    let mut paths = fs::read_dir(root.join("corpus/cases"))
        .map_err(|e| e.to_string())?
        .map(|e| e.map(|e| e.path()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>>>()?;
    paths.sort();
    let (mut count, mut scopes) = (0, 0);
    let mut ids = BTreeSet::new();
    for path in paths {
        if path.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let c = read(&path)?;
        let cid = txt(&c, "case_id")?;
        if partition.is_some_and(|p| p != part(cid)) {
            continue;
        }
        if !ids.insert(cid.to_owned()) {
            return Err("duplicate case id".into());
        }
        scopes += validate_corpus_case(root, &c, &claims).map_err(|e| format!("{cid}: {e}"))?;
        count += 1;
    }
    if partition.is_none() && (count != 175 || scopes != 374) {
        return Err(format!(
            "total corpus coverage REQUIRED175/374 got{count}/{scopes}"
        ));
    }
    Ok((count, scopes))
}
