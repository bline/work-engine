//! Reference-namespace application against independently readable raw rows.
//!
//! A supplied initial contract declares `dollar_ticket`, `admission` or `unestablished`. Bundle,
//! contract, grant and payload can all agree with one another and still apply the wrong
//! namespace, so the declaration is compared with the raw row the origin is bound to by digest.
//! Raw rows come from the lineage's own `corpus/cases/<lineage>.json` subject or from a supplied
//! payload carrying `original_inputs`. This is applicability, not raw admission: it never prices,
//! chooses partitions or establishes a guarantee.
use crate::{Result, corpus, exact_u64};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

pub(crate) const RAW_SUBJECT_PREFIX: &str = "raw-subject:";

fn fail<T>(why: String) -> Result<T> {
    Err(format!("namespace application: {why}"))
}

/// Adds the lineage's corpus subject (when one exists under ROOT) as an independent raw source.
pub(crate) fn resolve_raw_subjects(
    root: &Path,
    bundle: &Value,
    out: &mut BTreeMap<String, Value>,
) -> Result<()> {
    for doc in bundle["documents"].as_array().into_iter().flatten() {
        if doc["kind"] != "accounting_input_proof" {
            continue;
        }
        let Some(lineage) = doc["lineage_id"].as_str() else {
            continue;
        };
        if lineage.is_empty()
            || !lineage
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        {
            continue;
        }
        let file = root.join("corpus/cases").join(format!("{lineage}.json"));
        if !file.is_file() {
            continue;
        }
        let case = serde_json::from_slice::<Value>(
            &fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?,
        )
        .map_err(|e| format!("raw subject JSON: {e}"))?;
        let inputs = case
            .get("inputs")
            .cloned()
            .ok_or("namespace application: corpus subject lacks inputs")?;
        out.insert(
            format!("{RAW_SUBJECT_PREFIX}{lineage}"),
            serde_json::json!({ "original_inputs": inputs }),
        );
    }
    Ok(())
}

fn digests(inputs: &Value) -> Result<[String; 2]> {
    Ok([
        format!("sha256:{}", corpus::hash(corpus::canonical(inputs)?.as_bytes())),
        format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(inputs).map_err(|e| e.to_string())?)
        ),
    ])
}

fn raw_row<'a>(inputs: &'a Value, source: &str, position: u64) -> Option<&'a Value> {
    let (alias, turn) = source.split_once('/')?;
    let session = inputs["sessions"]
        .as_array()?
        .iter()
        .find(|s| s["alias"] == alias)?;
    let index = usize::try_from(position.checked_sub(1)?).ok()?;
    session["logs"][turn]["lines"].as_array()?.get(index)
}

/// Namespace a raw row's own reference belongs to, by the single shared rule table
/// `oracle/r11-controls/raw-namespace-rules.json` (D1 `usd_reserved`+`rid`, D2 `usd.rid`, A1
/// `subscription_proxy_admitted`+`rid`, A2 `subscription_proxy.rid`; a row satisfying both a D and
/// an A rule is `conflict`; rid-less admission rows, non-objects and unreadable lines carry none).
pub(crate) fn classify(row: &Value) -> Option<&'static str> {
    let parsed;
    let object = match row {
        Value::String(line) => {
            parsed = serde_json::from_str::<Value>(line).ok()?;
            &parsed
        }
        other => other,
    };
    let object = object.as_object()?;
    let has_rid = |v: Option<&Value>| v.and_then(Value::as_object).is_some_and(|o| o.contains_key("rid"));
    let event = object.get("event").and_then(Value::as_str);
    let dollar = (event == Some("usd_reserved") && object.contains_key("rid")) || has_rid(object.get("usd"));
    let admission = (event == Some("subscription_proxy_admitted") && object.contains_key("rid"))
        || has_rid(object.get("subscription_proxy"));
    match (dollar, admission) {
        (true, true) => Some("conflict"),
        (true, false) => Some("dollar_ticket"),
        (false, true) => Some("admission"),
        (false, false) => None,
    }
}

pub(crate) fn check_namespace_application(
    bundle: &Value,
    payloads: &BTreeMap<String, Value>,
) -> Result<()> {
    let documents = bundle["documents"].as_array().ok_or("missing documents")?;
    let mut sources = Vec::new();
    for payload in payloads.values() {
        if let Some(inputs) = payload.get("original_inputs") {
            sources.push((inputs, digests(inputs)?));
        }
    }
    for input in documents
        .iter()
        .filter(|d| d["kind"] == "accounting_input_proof")
    {
        let contracts: BTreeMap<&str, &Value> = input["initial_contracts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| c["contract_id"].as_str().map(|id| (id, c)))
            .collect();
        let mut namespace_of = BTreeMap::new();
        for origin in input["origins"].as_array().into_iter().flatten() {
            let id = origin["origin_id"].as_str().ok_or("origin id")?;
            let revision = origin["source_revision_ref"].as_str().unwrap_or_default();
            // Only a raw source bound to the origin by its source-revision digest can speak for
            // it; a bundle describing a different (e.g. mutated) raw subject is not judged here.
            let Some((inputs, _)) = sources
                .iter()
                .find(|(_, d)| d.iter().any(|x| x == revision))
            else {
                continue;
            };
            let source = origin["source_origin_id"].as_str().unwrap_or_default();
            let row = exact_u64(&origin["original_position"])
                .and_then(|p| raw_row(inputs, source, p))
                .ok_or_else(|| format!("namespace application: origin {id} has no bound raw row"))?;
            let bytes = match row.as_str() {
                Some(line) => line.to_owned(),
                None => corpus::canonical(row)?,
            };
            if origin["raw_bytes_sha256"] != corpus::hash(bytes.as_bytes()) {
                return fail(format!("origin {id} raw bytes differ from its bound raw row"));
            }
            let Some(actual) = classify(row) else {
                continue;
            };
            let contract_id = origin["initial_contract_ref"].as_str().unwrap_or_default();
            let declared = contracts
                .get(contract_id)
                .and_then(|c| c["reference_namespace"].as_str())
                .ok_or_else(|| format!("namespace application: origin {id} has no initial contract"))?;
            if actual == "conflict" && declared != "unestablished" {
                return fail(format!(
                    "origin {id} raw row carries both dollar and admission references, so initial contract {contract_id} ({declared}) cannot apply to it"
                ));
            }
            if declared != actual && declared != "unestablished" {
                return fail(format!(
                    "origin {id} raw row is {actual}-namespace but its initial contract {contract_id} declares {declared}"
                ));
            }
            namespace_of.insert(id.to_owned(), actual);
        }
        for result in documents
            .iter()
            .filter(|d| d["kind"] == "accounting_result" && d["input_proof_ref"] == input["id"])
        {
            for base in result["bases"].as_array().into_iter().flatten() {
                let kinds: BTreeSet<_> = base["origin_refs"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|o| o.as_str().and_then(|o| namespace_of.get(o)))
                    .collect();
                if kinds.len() > 1 {
                    return fail(format!(
                        "base {} groups origins from different reference namespaces",
                        base["base_id"].as_str().unwrap_or_default()
                    ));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ADMITTED: &str = r#"{"event": "subscription_proxy_admitted", "n": 1, "rid": 1}"#;
    const REQUEST_WITH_RID: &str = r#"{"event": "request", "subscription_proxy": {"rid": 1}}"#;
    const RESERVED: &str = r#"{"event": "usd_reserved", "n": 1, "rid": 1}"#;
    const PLAIN: &str = r#"{"event": "request", "n": 2}"#;

    fn inputs(lines: &[&str]) -> Value {
        json!({"sessions":[{"alias":"S1","logs":{"turn-1":{"lines":lines}}}]})
    }
    fn payloads(inputs: &Value) -> BTreeMap<String, Value> {
        BTreeMap::from([(
            format!("{RAW_SUBJECT_PREFIX}L"),
            json!({ "original_inputs": inputs }),
        )])
    }
    /// One contract per origin: `namespaces[i]` declared for row i; all origins in one base.
    fn bundle(inputs: &Value, namespaces: &[&str], one_base: bool) -> Value {
        let revision = digests(inputs).unwrap()[0].clone();
        let lines = inputs["sessions"][0]["logs"]["turn-1"]["lines"].as_array().unwrap();
        let origins: Vec<Value> = lines.iter().enumerate().map(|(i, l)| json!({
            "origin_id": format!("o{i}"), "source_origin_id": "S1/turn-1", "original_position": i + 1,
            "raw_bytes_sha256": corpus::hash(l.as_str().unwrap().as_bytes()),
            "source_revision_ref": revision, "initial_contract_ref": format!("c{i}")})).collect();
        let contracts: Vec<Value> = namespaces.iter().enumerate().map(|(i, n)| json!({
            "contract_id": format!("c{i}"), "reference_namespace": n, "duplicate_rule": "same_origin_only"})).collect();
        let ids: Vec<String> = (0..lines.len()).map(|i| format!("o{i}")).collect();
        let bases: Vec<Value> = if one_base {
            vec![json!({"base_id": "b", "origin_refs": ids})]
        } else {
            ids.iter().map(|i| json!({"base_id": format!("b-{i}"), "origin_refs": [i]})).collect()
        };
        json!({"documents":[
            {"kind":"accounting_input_proof","id":"in","lineage_id":"L","origins":origins,"initial_contracts":contracts},
            {"kind":"accounting_result","id":"res","input_proof_ref":"in","bases":bases}]})
    }
    fn verdict(lines: &[&str], namespaces: &[&str], one_base: bool) -> Result<()> {
        let raw = inputs(lines);
        check_namespace_application(&bundle(&raw, namespaces, one_base), &payloads(&raw))
    }

    #[test]
    fn raw_rows_classify_by_their_own_reference() {
        for (line, want) in [
            (ADMITTED, Some("admission")),
            (REQUEST_WITH_RID, Some("admission")),
            (RESERVED, Some("dollar_ticket")),
            (r#"{"event": "request", "usd": {"rid": 1}}"#, Some("dollar_ticket")),
            (r#"{"event": "subscription_proxy_admitted", "n": 1}"#, None),
            (r#"{"event": "request", "subscription_proxy": {"status": "valued"}}"#, None),
            (r#"{"event": "request", "usd": {"rid": 1}, "subscription_proxy": {"rid": 1}}"#, Some("conflict")),
            (r#"{"event": "usd_reserved", "rid": 1, "subscription_proxy": {"rid": 1}}"#, Some("conflict")),
            (PLAIN, None),
            (r#"{"event": "usd_reserved"}"#, None),
            ("not json", None),
        ] {
            assert_eq!(classify(&Value::String(line.into())), want, "{line}");
        }
    }

    #[test]
    fn dollar_declaration_over_admission_rows_is_rejected_even_when_everything_agrees() {
        // VC08 shape: admission row + measured request both declared dollar_ticket, one base.
        let error = verdict(&[ADMITTED, REQUEST_WITH_RID], &["dollar_ticket", "dollar_ticket"], true).unwrap_err();
        assert!(error.contains("raw row is admission-namespace") && error.contains("declares dollar_ticket"), "{error}");
        let error = verdict(&[RESERVED], &["admission"], false).unwrap_err();
        assert!(error.contains("raw row is dollar_ticket-namespace") && error.contains("declares admission"), "{error}");
    }

    #[test]
    fn separately_declared_admission_namespace_and_unreferenced_rows_are_accepted() {
        verdict(&[ADMITTED, REQUEST_WITH_RID], &["admission", "admission"], true).unwrap();
        verdict(&[RESERVED, PLAIN], &["dollar_ticket", "dollar_ticket"], true).unwrap();
        // no namespace claimed: nothing to misapply (grouping policy is separate)
        verdict(&[ADMITTED, REQUEST_WITH_RID], &["unestablished", "unestablished"], false).unwrap();
        // a dollar declaration over a row that carries no reference is not a mismatch
        verdict(&[PLAIN], &["dollar_ticket"], false).unwrap();
        // mixed declarations over distinct bases are each judged on their own row
        verdict(&[RESERVED, ADMITTED], &["dollar_ticket", "admission"], false).unwrap();
    }

    #[test]
    fn a_base_cannot_group_rows_of_two_reference_namespaces() {
        let error = verdict(&[RESERVED, ADMITTED], &["dollar_ticket", "admission"], true).unwrap_err();
        assert!(error.contains("groups origins from different reference namespaces"), "{error}");
        let error = verdict(&[RESERVED, ADMITTED], &["unestablished", "unestablished"], true).unwrap_err();
        assert!(error.contains("groups origins from different reference namespaces"), "{error}");
    }

    #[test]
    fn origin_bytes_must_match_the_bound_raw_row_and_unbound_sources_are_not_judged() {
        let raw = inputs(&[ADMITTED]);
        let mut b = bundle(&raw, &["dollar_ticket"], false);
        b["documents"][0]["origins"][0]["raw_bytes_sha256"] = json!("0".repeat(64));
        let error = check_namespace_application(&b, &payloads(&raw)).unwrap_err();
        assert!(error.contains("raw bytes differ"), "{error}");
        let mut b = bundle(&raw, &["dollar_ticket"], false);
        b["documents"][0]["origins"][0]["original_position"] = json!(2);
        assert!(check_namespace_application(&b, &payloads(&raw)).unwrap_err().contains("no bound raw row"));
        let mut b = bundle(&raw, &["dollar_ticket"], false);
        b["documents"][0]["origins"][0]["source_revision_ref"] = json!("sha256:other");
        check_namespace_application(&b, &payloads(&raw)).unwrap();
    }

    fn repo_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn classifier_reproduces_the_shared_census_over_every_corpus_row() {
        // (1) the oracle's census (Python actual-path classes with raw digests)
        let census: Value = serde_json::from_slice(
            &fs::read(repo_root().join("oracle/r11-controls/raw-namespace-census.json")).unwrap(),
        )
        .unwrap();
        let rows = census["rows"].as_array().unwrap();
        assert!(rows.len() > 300, "census rows {}", rows.len());
        for r in rows {
            let raw = r["raw"].as_str().unwrap();
            assert_eq!(r["raw_utf8_sha256"], corpus::hash(raw.as_bytes()), "{}", r["origin_id"]);
            let want = r["class"].as_str();
            assert_eq!(classify(&Value::String(raw.into())), want, "{} {}", r["case_id"], r["origin_id"]);
        }
        // (2) every raw line of every corpus case against the rule table written out literally
        let mut seen = BTreeMap::new();
        for entry in fs::read_dir(repo_root().join("corpus/cases")).unwrap() {
            let case: Value = serde_json::from_slice(&fs::read(entry.unwrap().path()).unwrap()).unwrap();
            for session in case["inputs"]["sessions"].as_array().into_iter().flatten() {
                for log in session["logs"].as_object().into_iter().flatten().map(|(_, v)| v) {
                    for line in log["lines"].as_array().into_iter().flatten() {
                        let row = line.clone();
                        let table = match line.as_str().and_then(|l| serde_json::from_str::<Value>(l).ok()).as_ref().and_then(Value::as_object) {
                            None => None,
                            Some(o) => {
                                let rid = |k: &str| o.get(k).and_then(Value::as_object).is_some_and(|x| x.contains_key("rid"));
                                let e = o.get("event").and_then(Value::as_str);
                                let d = (e == Some("usd_reserved") && o.contains_key("rid")) || rid("usd");
                                let a = (e == Some("subscription_proxy_admitted") && o.contains_key("rid")) || rid("subscription_proxy");
                                if d && a { Some("conflict") } else if d { Some("dollar_ticket") } else if a { Some("admission") } else { None }
                            }
                        };
                        assert_eq!(classify(&row), table, "{row}");
                        *seen.entry(table).or_insert(0usize) += 1;
                    }
                }
            }
        }
        assert!(seen.get(&Some("dollar_ticket")).copied().unwrap_or(0) > 100 && seen.get(&Some("admission")).copied().unwrap_or(0) >= 6, "{seen:?}");
    }

    #[test]
    fn a_row_with_both_reference_kinds_accepts_no_namespace_declaration() {
        let both = r#"{"event": "request", "usd": {"rid": 1}, "subscription_proxy": {"rid": 1}}"#;
        for declared in ["dollar_ticket", "admission"] {
            let error = verdict(&[both], &[declared], false).unwrap_err();
            assert!(error.contains("carries both dollar and admission references"), "{error}");
        }
        verdict(&[both], &["unestablished"], false).unwrap();
        // settlement-only dollar source (D2) is dollar, rid-less admission rows carry no namespace
        verdict(&[r#"{"event": "request", "usd": {"rid": 1}}"#], &["admission"], false).unwrap_err();
        verdict(&[r#"{"event": "subscription_proxy_admitted", "n": 1}"#], &["dollar_ticket"], false).unwrap();
    }
}
