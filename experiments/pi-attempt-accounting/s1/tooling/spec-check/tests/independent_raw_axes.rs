//! Independent raw-axis qualification. No canonical crate/helper imports.
//! This does not calculate C-K reducer outputs or establish producer reachability.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};
fn root() -> PathBuf {
    std::env::var_os("R9_QUALIFICATION_SUBJECT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}
fn read(p: impl AsRef<std::path::Path>) -> Value {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn nat(v: &Value) -> Option<i128> {
    v.as_u64().map(i128::from)
}
// Decimal rationals use a fixed 1e6 scale; no IEEE arithmetic.
fn scaled(s: &str, scale: i128) -> Option<i128> {
    let (a, b) = s.split_once('.').unwrap_or((s, ""));
    if b.len() > 9
        || a.starts_with('-')
        || !a.chars().all(|c| c.is_ascii_digit())
        || !b.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let den = 10i128.pow(b.len() as u32);
    let n = a
        .parse::<i128>()
        .ok()?
        .checked_mul(den)?
        .checked_add(if b.is_empty() { 0 } else { b.parse().ok()? })?;
    let z = n.checked_mul(scale)?;
    if z % den == 0 { Some(z / den) } else { None }
}
fn rate(v: &Value) -> i128 {
    scaled(&v.to_string(), 1_000_000).unwrap()
}
fn dollars(v: &Value) -> Option<i128> {
    if !v.is_number() {
        None
    } else {
        scaled(&v.to_string(), 1_000_000_000)
    }
}
fn field(u: &Value, k: &str) -> &'static str {
    match u.get(k) {
        None => "absent",
        Some(Value::Null) => "null",
        Some(v) if nat(v).is_some() => "present_integer",
        _ => "malformed",
    }
}
fn family(p: &str) -> Option<&str> {
    match p {
        "openai" | "openai-codex" => Some("openai"),
        "anthropic" => Some("anthropic"),
        _ => None,
    }
}
fn identity(s: &Value) -> Option<(String, String)> {
    let (p, m) = s.as_str()?.split_once('/')?;
    Some((family(p)?.into(), m.into()))
}
fn request_family(row: &Value) -> Option<&str> {
    match row.get("provider") {
        None => Some("openai"),
        Some(v) => family(v.as_str()?),
    }
}
fn billing(f: &str) -> Vec<&'static str> {
    if f == "openai" {
        vec![
            "input_tokens",
            "output_tokens",
            "cached_tokens",
            "cache_write_tokens",
        ]
    } else {
        vec![
            "input_tokens",
            "output_tokens",
            "cache_read_input_tokens",
            "cache_creation_input_tokens",
            "ephemeral_5m_input_tokens",
            "ephemeral_1h_input_tokens",
        ]
    }
}
fn schema_mismatch(row: &Value, f: &str) -> bool {
    let exclusive = if f == "openai" {
        vec![
            "cache_read_input_tokens",
            "cache_creation_input_tokens",
            "ephemeral_5m_input_tokens",
            "ephemeral_1h_input_tokens",
        ]
    } else {
        vec!["cached_tokens", "cache_write_tokens"]
    };
    exclusive
        .iter()
        .any(|k| row["usage"].get(*k).is_some_and(|v| !v.is_null()))
}
fn identities(row: &Value, launch: &Value) -> BTreeSet<(String, String)> {
    let mut ids = BTreeSet::new();
    if let Some(i) = identity(&launch["model"]) {
        ids.insert(i);
    }
    if let Some(f) = request_family(row) {
        for k in ["model", "response_model", "authorized_model"] {
            if let Some(s) = row[k].as_str() {
                if !s.is_empty() {
                    ids.insert((f.into(), s.into()));
                }
            }
        }
    }
    ids
}
fn terminal(row: &Value, f: &str) -> bool {
    row["http_status"].as_u64() == Some(200)
        && match f {
            "openai" => matches!(
                row["terminal"].as_str(),
                Some("response.completed" | "response.done")
            ),
            "anthropic" => row["terminal"] == "message_stop",
            _ => false,
        }
}
fn price_range(row: &Value, launch: &Value, rates: &Value) -> Option<(i128, i128, bool)> {
    let f = request_family(row)?;
    if schema_mismatch(row, f) {
        return None;
    }
    let u = row.get("usage")?.as_object()?;
    let u = Value::Object(u.clone());
    let keys = billing(f);
    if !keys.iter().any(|k| u.get(*k).is_some_and(|v| !v.is_null())) {
        return None;
    }
    if keys.iter().any(|k| field(&u, k) == "malformed") {
        return None;
    }
    let i = nat(&u["input_tokens"])?;
    let o = nat(&u["output_tokens"])?;
    let mut vals = vec![];
    let mut counts_complete = true;
    let ids = identities(row, launch);
    if ids.is_empty() || identity(&launch["model"]).is_none() {
        return None;
    }
    for (p, m) in ids {
        if p != f {
            return None;
        }
        let r = &rates[&p]["models"][&m];
        if r.is_null() {
            return None;
        }
        let pr = |k: &str| rate(&r[k]);
        if f == "openai" {
            let tier = row["observed_tier"].as_str()?;
            let t = &rates[f]["tier_multiplier"][tier];
            if t.is_null() {
                return None;
            }
            let t = rate(t);
            let c = u.get("cached_tokens").and_then(nat);
            let w = u.get("cache_write_tokens").and_then(nat);
            if c.is_some_and(|c| c > i)
                || w.is_some_and(|w| w > i)
                || matches!((c,w),(Some(c),Some(w)) if c+w>i)
            {
                return None;
            }
            let pts = match (c, w) {
                (Some(c), Some(w)) => vec![(c, w)],
                (Some(c), None) => vec![(c, 0), (c, i - c)],
                (None, Some(w)) => vec![(0, w), (i - w, w)],
                _ => vec![(0, 0), (i, 0), (0, i)],
            };
            let long = i > nat(&rates[f]["long_input_threshold"])?;
            let mi = if long {
                rate(&rates[f]["long_input_multiplier"])
            } else {
                1_000_000
            };
            let mo = if long {
                rate(&rates[f]["long_output_multiplier"])
            } else {
                1_000_000
            };
            for (c, w) in pts {
                let n =
                    (((i - c - w) * pr("input") + c * pr("cached_input") + w * pr("cache_write"))
                        * mi
                        + o * pr("output") * mo)
                        * t
                        * 1000;
                assert_eq!(n % 1_000_000i128.pow(3), 0);
                vals.push(n / 1_000_000i128.pow(3));
            }
        } else {
            if row["observed_tier"]
                .as_str()
                .is_some_and(|s| s != "standard")
            {
                return None;
            }
            if row
                .get("observed_tier")
                .is_some_and(|v| !v.is_null() && !v.is_string())
            {
                return None;
            }
            let cr = nat(&u["cache_read_input_tokens"])?;
            let cw = nat(&u["cache_creation_input_tokens"])?;
            let w5 = u
                .get("ephemeral_5m_input_tokens")
                .and_then(nat)
                .unwrap_or(0);
            let w1 = u
                .get("ephemeral_1h_input_tokens")
                .and_then(nat)
                .unwrap_or(0);
            counts_complete &= w5 + w1 <= cw;
            let base = i * pr("input") + cr * pr("cache_read") + o * pr("output");
            let a = pr("cache_write_5m");
            let b = pr("cache_write_1h");
            for rr in [a.min(b), a.max(b)] {
                let writes = if w5 + w1 > cw {
                    cw * rr
                } else {
                    w5 * a + w1 * b + (cw - w5 - w1) * rr
                };
                let n = (base + writes) * 1000;
                assert_eq!(n % 1_000_000, 0);
                vals.push(n / 1_000_000);
            }
        }
    }
    Some((
        *vals.iter().min()?,
        *vals.iter().max()?,
        terminal(row, f)
            && counts_complete
            && ["model", "response_model", "authorized_model"]
                .iter()
                .all(|k| {
                    row.get(*k)
                        .is_none_or(|v| v.is_null() || v.as_str().is_some_and(|s| !s.is_empty()))
                }),
    ))
}
fn rows(raw: &Value) -> Vec<(String, &Value, String, bool)> {
    let mut out = vec![];
    for s in raw["inputs"]["sessions"].as_array().unwrap() {
        for (name, log) in s["logs"].as_object().unwrap() {
            for (n, line) in log["lines"].as_array().unwrap().iter().enumerate() {
                out.push((
                    format!("{}/{name}:{}", s["alias"].as_str().unwrap(), n + 1),
                    s,
                    line.as_str().unwrap().into(),
                    log["final_newline"] == false
                        && n + 1 == log["lines"].as_array().unwrap().len(),
                ));
            }
        }
    }
    out
}
#[test]
fn frozen_raw_coverage_and_independent_arithmetic() {
    let r = root();
    let facts_bytes = fs::read(r.join("oracle/FROZEN-FACTS.json")).unwrap();
    assert_eq!(
        hash(&facts_bytes),
        "f097e5a541b279f2081c71f54ca3403151475348b8a30245e6846708590e293e"
    );
    let facts: Value = serde_json::from_slice(&facts_bytes).unwrap();
    let rates = json!({"openai":read(r.join("oracle/rates/openai_api_rates.json")),"anthropic":read(r.join("oracle/rates/anthropic_api_rates.json"))});
    for f in ["openai", "anthropic"] {
        assert_eq!(
            hash(&fs::read(r.join(format!("oracle/rates/{f}_api_rates.json"))).unwrap()),
            facts["rate_sha256"][f].as_str().unwrap()
        );
    }
    let mut selected = 0;
    let mut sources = 0;
    let mut count = 0;
    let mut groups = 0;
    let mut priced = 0;
    let mut report = vec![];
    for c in facts["cases"].as_array().unwrap() {
        let cid = c["case_id"].as_str().unwrap();
        let raw_bytes = fs::read(r.join(format!("oracle/frozen-raw/{cid}.json"))).unwrap();
        assert_eq!(hash(&raw_bytes), c["raw_sha256"].as_str().unwrap());
        let raw: Value = serde_json::from_slice(&raw_bytes).unwrap();
        let rr = rows(&raw);
        let mut axes = vec![];
        let mut selected_sources = BTreeSet::new();
        for sess in raw["inputs"]["sessions"].as_array().unwrap() {
            for (name, log) in sess["logs"].as_object().unwrap() {
                let dollar = log["lines"].as_array().unwrap().iter().any(|l| {
                    serde_json::from_str::<Value>(l.as_str().unwrap())
                        .ok()
                        .is_some_and(|v| v["event"] == "usd_reserved" || v.get("usd").is_some())
                });
                if sess["launch"]["auth_lane"] == "api" && dollar {
                    selected_sources.insert(format!("{}/{name}", sess["alias"].as_str().unwrap()));
                }
            }
        }
        let frozen_sources: BTreeSet<String> = c["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|ss| {
                ss["sources"].as_array().unwrap().iter().map(move |src| {
                    format!(
                        "{}/{}",
                        ss["alias"].as_str().unwrap(),
                        src["source"].as_str().unwrap()
                    )
                })
            })
            .collect();
        assert_eq!(selected_sources, frozen_sources, "{cid}: source coverage");
        assert_eq!(
            !selected_sources.is_empty(),
            c["selected"] == true,
            "{cid}: raw selector"
        );
        if c["selected"] == true {
            selected += 1;
        }
        let current = read(r.join(format!("corpus/cases/{cid}.json")));
        assert_eq!(
            current["inputs"], raw["inputs"],
            "{cid}: historical input bytes changed"
        );
        axes.push(json!({"axis":"raw_selection","standing":"established","selected":c["selected"],"projection":current["profile"]}));
        for s in c["sessions"].as_array().unwrap() {
            for src in s["sources"].as_array().unwrap() {
                sources += 1;
                for g in src["reservation_groups"].as_array().unwrap() {
                    groups += 1;
                    let nums: Vec<i128> = g["distinct_amounts_nano"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter_map(nat)
                        .collect();
                    assert_eq!(
                        Some(nums.iter().max().copied().unwrap_or(0)),
                        nat(&g["maximum_known_reservation_nano"]),
                        "{cid}: maximumR"
                    );
                    assert_eq!(
                        g["conflict"] == true,
                        g["distinct_rows"].as_u64().unwrap() > 1
                    );
                    axes.push(json!({"axis":"reservation_maximum_and_uniqueness","standing":"established","group":g["raw_group"],"R":g["maximum_known_reservation_nano"],"comparison_applicable":g["distinct_rows"].as_u64()==Some(1)}));
                }
                for row in src["rows"].as_array().unwrap() {
                    count += 1;
                    let reference = row["ref"].as_str().unwrap();
                    let (_, session, line, unreadable) =
                        rr.iter().find(|x| x.0 == reference).unwrap();
                    assert_eq!(hash(line.as_bytes()), row["line_sha256"].as_str().unwrap());
                    assert_eq!(*unreadable, row["adapter_final_line_unreadable"] == true);
                    let v: Value = serde_json::from_str(line).unwrap();
                    let matched: Vec<&Value> = current["expected"]["attempts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|a| {
                            a["evidence"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|x| x == reference)
                        })
                        .collect();
                    let represented = !matched.is_empty()
                        || ["non_attempt", "unreadable"].iter().any(|k| {
                            current["expected"][*k]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|x| x["evidence"] == reference)
                        });
                    assert!(represented, "{cid}/{reference}: raw evidence omitted");

                    if let Some(x) = row.get("reserved_nano") {
                        assert_eq!(dollars(&v["reserved_usd"]), nat(x));
                    }
                    if let Some(f) = row.get("usage_facts") {
                        let family = request_family(&v);
                        assert_eq!(family, f["declared_provider_family"].as_str());
                        if let Some(family) = family {
                            assert_eq!(
                                terminal(&v, family),
                                f["status_200"] == true && f["terminal_success_marker"] == true
                            );
                        }
                        if let Some((lo, hi, complete)) =
                            price_range(&v, &session["launch"], &rates)
                        {
                            priced += 1;
                            assert_eq!(
                                lo.to_string(),
                                f["supported_low_nano"].as_str().unwrap(),
                                "{cid}/{reference}: low"
                            );
                            assert_eq!(
                                hi.to_string(),
                                f["priced_high_nano"].as_str().unwrap(),
                                "{cid}/{reference}: high"
                            );
                            for a in &matched {
                                let request_count = a["evidence"]
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .filter(|rf| {
                                        rr.iter()
                                            .find(|q| q.0 == rf.as_str().unwrap())
                                            .and_then(|q| serde_json::from_str::<Value>(&q.2).ok())
                                            .is_some_and(|q| q["event"] == "request")
                                    })
                                    .count();
                                if request_count == 1 {
                                    assert_eq!(
                                        a["valuation"]["state"], "valued",
                                        "{cid}/{reference}: supported usage erased"
                                    );
                                    assert_eq!(
                                        a["valuation"]["low"],
                                        lo.to_string(),
                                        "{cid}/{reference}: expected low"
                                    );
                                    assert_eq!(
                                        a["valuation"]["high"],
                                        hi.to_string(),
                                        "{cid}/{reference}: expected high"
                                    );
                                    assert_eq!(
                                        a["valuation"]["complete"], complete,
                                        "{cid}/{reference}: complete policy"
                                    );
                                }
                                if v["usd"]["charge_basis"] == "error_status"
                                    && a["settlement"]["decision"] == "valid"
                                {
                                    let retained =
                                        dollars(&v["usd"]["reserved_usd"]).unwrap().max(hi);
                                    assert_eq!(a["exposure"]["high"]["kind"], "at_least");
                                    assert!(
                                        a["exposure"]["high"]["nano"]
                                            .as_str()
                                            .unwrap()
                                            .parse::<i128>()
                                            .unwrap()
                                            >= retained
                                    );
                                    assert_eq!(a["replacement"]["eligible"], false);
                                }
                            }
                            axes.push(json!({"axis":"usage_raw_arithmetic_and_completion","standing":"established","ref":reference,"low":lo.to_string(),"high":hi.to_string(),"complete":complete}));
                        } else {
                            axes.push(json!({"axis":"usage_priceability","standing":"unestablished","ref":reference,"facts":f["arithmetic_state"],"scope":"raw axis only; not reducer acceptance"}));
                        }
                    }
                }
            }
        }
        axes.push(json!({"axis":"full_C_K_acceptance","standing":"unestablished","reason":"not implemented by bounded raw-axis assertions"}));
        report.push(json!({"case_id":cid,"axes":axes}));
    }
    assert_eq!((selected, sources, count, groups), (137, 142, 287, 147));
    assert!(priced > 90);
    if let Ok(p) = std::env::var("ORACLE_REPORT") {
        fs::write(p,serde_json::to_vec_pretty(&json!({"schema":"independent-axis-qualification/1","policy":"root P1-P6 approval07303d5c","case_count":173,"selected_cases":selected,"sources":sources,"rows":count,"groups":groups,"priced_observations":priced,"cases":report,"authority":"bounded_axes_only_no_acceptance"})).unwrap()).unwrap();
    }
}
#[test]
fn independent_writer_positive_controls() {
    let witnesses = read(root().join("oracle/writer-controls.json"));
    for w in witnesses["witnesses"].as_array().unwrap() {
        let body = &w["body"];
        let mut rest = body.clone();
        rest.as_object_mut().unwrap().remove("input");
        let rest_bytes = serde_json::to_vec(&rest).unwrap().len();
        let item_bytes = serde_json::to_vec(&body["input"][0]).unwrap().len();
        let bound = rest_bytes + item_bytes + 32 * 2;
        assert_eq!(Some(bound as i128), nat(&w["input_basis"]["input_bound"]));
        assert_eq!(hash(&serde_json::to_vec(body).unwrap()), w["body_sha256"]);
        assert_eq!(w["budget_config"]["strict_tier"], false);
        assert_eq!(w["budget_config"]["multiplier_floor"], 0);
        assert!(w["input_basis"]["prefix"].is_null());
        let c = corpus_case(w["case_id"].as_str().unwrap());
        let req: Value = serde_json::from_str(
            c["inputs"]["sessions"][0]["logs"]["turn-1"]["lines"][1]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            dollars(&req["usd"]["multiplier_floor"]),
            nat(&w["expected_logged_floor"]).map(|n| n * 1_000_000_000)
        );
        assert_eq!(c["reachability"], "not_established");
    }

    let r = root();
    let a = read(r.join("corpus/cases/R6-C01-writer-reservation-floor-control.json"));
    let l = &a["inputs"]["sessions"][0]["logs"]["turn-1"]["lines"];
    let reservation: Value = serde_json::from_str(l[0].as_str().unwrap()).unwrap();
    let request: Value = serde_json::from_str(l[1].as_str().unwrap()).unwrap();
    let input = nat(&reservation["input_bound"]).unwrap();
    assert_eq!(input, 1000);
    assert_eq!((input * 12500 + 150 * 50000) / 2, 10_000_000);
    assert_eq!(dollars(&reservation["reserved_usd"]), Some(10_000_000));
    assert_eq!(request["requested_tier"], "flex");
    assert_eq!(request["observed_tier"], "default");
    assert_eq!(
        a["expected"]["attempts"][0]["replacement"]["eligible"],
        false
    );
    let a = read(r.join("corpus/cases/R6-C02-non200-success-terminal-usage-control.json"));
    let request: Value = serde_json::from_str(
        a["inputs"]["sessions"][0]["logs"]["turn-1"]["lines"][1]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(6000 * 12500 + 14500 * 50000, 800_000_000);
    assert_eq!(request["http_status"], 429);
    assert_eq!(request["terminal"], "response.completed");
    assert_eq!(dollars(&request["usd"]["charged_usd"]), Some(0));
    assert_eq!(a["expected"]["attempts"][0]["valuation"]["complete"], false);
    assert_eq!(
        a["expected"]["attempts"][0]["exposure"]["high"]["nano"],
        "1000000000"
    );
    assert_eq!(a["reachability"], "not_established");
}

#[test]
fn policy_edge_controls() {
    let rates = json!({"openai":read(root().join("oracle/rates/openai_api_rates.json")),"anthropic":read(root().join("oracle/rates/anthropic_api_rates.json"))});
    let launch = json!({"model":"openai/gpt-6-astra"});
    let mut row = json!({"model":"gpt-6-astra","observed_tier":"default","http_status":200,"terminal":"response.completed","usage":{"input_tokens":1000,"output_tokens":100,"cached_tokens":null,"cache_write_tokens":null}});
    assert_eq!(
        price_range(&row, &launch, &rates),
        Some((6_000_000, 17_500_000, true))
    );
    row["http_status"] = json!(429);
    assert_eq!(
        price_range(&row, &launch, &rates),
        Some((6_000_000, 17_500_000, false))
    );
    let retained = 800_000_000i128.max(price_range(&row, &launch, &rates).unwrap().1);
    assert_eq!(retained, 800_000_000);
    row["usage"]["output_tokens"] = json!(20000);
    assert_eq!(
        800_000_000i128.max(price_range(&row, &launch, &rates).unwrap().1),
        1_012_500_000
    );
    row["usage"]["reasoning_tokens"] = json!("bad");
    assert!(price_range(&row, &launch, &rates).is_some());
    row["usage"]["cached_tokens"] = json!(false);
    assert!(price_range(&row, &launch, &rates).is_none());
    let empty = json!({"usage":{"reasoning_tokens":1}});
    assert!(
        !billing("openai")
            .iter()
            .any(|k| empty["usage"].get(*k).is_some_and(|v| !v.is_null()))
    );
    let mut mixed = json!({"model":"gpt-6-astra","usage":{"cached_tokens":null,"cache_read_input_tokens":null}});
    assert!(!schema_mismatch(&mixed, "openai"));
    mixed["usage"]["cache_read_input_tokens"] = json!(0);
    assert!(schema_mismatch(&mixed, "openai"));
    assert_ne!(
        identity(&json!("anthropic/same")),
        identity(&json!("openai/same"))
    );
    assert_eq!(scaled("0.015001", 1_000_000_000), Some(15_001_000));
    assert!(scaled("0.0000000001", 1_000_000_000).is_none());
}

fn corpus_case(id: &str) -> Value {
    read(root().join(format!("corpus/cases/{id}.json")))
}
// R8-GATE-IDENTITY-01: identity equality is meaningful only between present canonical nonempty strings.
fn canonical_id(v: &Value) -> bool {
    v.as_str().is_some_and(|s| !s.is_empty() && s.trim() == s)
}
fn gate_identity_triple(objective: &Value, g: &Value, w: &Value) -> bool {
    let ids = [&objective["admission_authority"]["id"], &g["authority"]["id"], &w["authority_id"]];
    ids.iter().all(|v| canonical_id(v)) && ids[0] == ids[1] && ids[1] == ids[2]
}
fn mock_binding(c: &Value, g: &Value) -> bool {
    let Some(gid) = g["gate_id"].as_str() else {
        return false;
    };
    let w = &c["trusted_fixture_inputs"]["gate_verifications"][gid];
    w["source"] == "test_mock"
        && w["payload_sha256"].as_str() == Some(hash(&serde_json::to_vec(g).unwrap()).as_str())
        && canonical_id(&w["authority_id"])
        && canonical_id(&g["authority"]["id"])
        && w["authority_id"] == g["authority"]["id"]
        && w["session"] == g["session"]
        && w["policy_ref"].as_str().is_some_and(|s| !s.is_empty())
        && w["verification_evidence"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
}
#[test]
fn historical_findings_have_independent_controls() {
    let a = corpus_case("N03-api-row-other-provider");
    let a = &a["expected"]["attempts"][0];
    assert_eq!(a["valuation"]["state"], "unpriceable");
    assert_eq!(a["settlement"]["decision"], "rejected");
    assert_eq!(a["exposure"]["high"]["nano"], "300000000");
    assert!(
        a["settlement"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("ModelMismatch"))
    );
    let c = corpus_case("V01-usage-settlement-other-response-model");
    let a = &c["expected"]["attempts"][0];
    assert_eq!(
        a["valuation"]["identities"],
        json!(["anthropic/claude-fable-5-2", "anthropic/claude-sonnet-5"])
    );
    assert_eq!(a["valuation"]["complete"], true);
    assert_eq!(a["valuation"]["low"], "20000000");
    assert_eq!(a["valuation"]["high"], "80000000");
    assert_eq!(a["exposure"]["high"]["kind"], "exact");
    assert_eq!(a["replacement"]["eligible"], true);
    assert!(
        a["blocking"]
            .as_array()
            .unwrap()
            .contains(&json!("IdentityContradiction"))
    );
    let c = corpus_case("S02-control-error-status-after-reservation");
    assert_eq!(
        c["expected"]["attempts"][0]["valuation"]["state"],
        "not_reported"
    );
    assert_eq!(c["reachability"], "producer_unreachable");
    for (id, expected) in [
        ("VC01-openai-usage-settlement", "settlement_amount"),
        ("U08-missing-observed-tier", "reservation_bound"),
        (
            "A06-reservation-usage-null-status-no-id",
            "reservation_bound",
        ),
    ] {
        let c = corpus_case(id);
        assert_eq!(
            c["expected"]["attempts"][0]["obligation"]["amount_source"],
            expected
        );
    }
    let c = corpus_case("P133-A04-cached-malformed");
    assert!(
        c["expected"]["attempts"][0]["blocking"]
            .as_array()
            .unwrap()
            .contains(&json!("MalformedEvidence(cached_tokens)"))
    );
    let c = corpus_case("P133-A09-conflicting-reservations-with-settlement");
    let a = &c["expected"]["attempts"][0];
    assert_eq!(a["settlement"]["reasons"], json!(["ReservationConflict"]));
    assert_eq!(a["exposure"]["high"]["nano"], "500000000");
    let c = corpus_case("R06-launch-row-missing");
    assert_eq!(
        c["expected"]["attempts"][0]["valuation"]["state"],
        "unpriceable"
    );
    assert_eq!(
        c["expected"]["attempts"][0]["exposure"]["high"]["kind"],
        "at_least"
    );
    let c = corpus_case("P130-R06-reservation-rid-missing");
    assert_eq!(c["expected"]["attempts"][0]["key"], "row:S1/turn-1:line=1");
    assert_eq!(
        c["expected"]["attempts"][0]["exposure"]["high"]["nano"],
        "300000000"
    );
    let mut c = corpus_case("P133-FG-VC14-authenticated-bound");
    let mut g = c["inputs"]["gates"][0].clone();
    assert!(mock_binding(&c, &g));
    let gid = g["gate_id"].as_str().unwrap().to_owned();
    assert!(
        c["evaluations"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|e| e["expected"]["objectives"].as_array().unwrap())
            .flat_map(|o| o["commitments"].as_array().unwrap())
            .all(|cm| cm["gate"]["verification_source"] == "test_mock")
    );
    g["verified"] = json!(true);
    assert!(!mock_binding(&c, &g));
    g.as_object_mut().unwrap().remove("verified");
    c["trusted_fixture_inputs"]["gate_verifications"]
        .as_object_mut()
        .unwrap()
        .remove(&gid);
    assert!(!mock_binding(&c, &g));
    g["verified"] = json!(true);
    g["verification"] = json!({"state":"verified","provenance":"trusted_adapter"});
    assert!(!mock_binding(&c, &g));
    let c = corpus_case("P130-R02-rounded-up-charge-admitted");
    let a = &c["expected"]["attempts"][0];
    assert_eq!(21 * 50 / 2, 525);
    assert_eq!(a["valuation"]["high"], "525");
    assert_eq!(a["settlement"]["charged"], "1000");
    assert_eq!(a["exposure"]["high"]["nano"], "1000");
    assert_eq!(a["exposure"]["high"]["kind"], "exact");
    // Arithmetic undercharge and one-micro-dollar rounding controls use exact raw charges.
    let c = corpus_case("V01-cheaper-response-model-undercharge");
    let a = &c["expected"]["attempts"][0];
    assert!(
        a["settlement"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("Undercharge"))
    );
    assert!(
        a["settlement"]["charged"]
            .as_str()
            .unwrap()
            .parse::<i128>()
            .unwrap()
            < a["valuation"]["high"]
                .as_str()
                .unwrap()
                .parse::<i128>()
                .unwrap()
    );
}

// R7 raw witnesses deliberately do not call any canonical semantic machinery.
#[test]
fn r7_anthropic_completeness_negative_witnesses() {
    let rates = json!({"anthropic":read(root().join("oracle/rates/anthropic_api_rates.json"))});
    let launch = json!({"model":"anthropic/claude-fable-5-2"});
    let row = json!({"provider":"anthropic","model":"claude-fable-5-2","http_status":200,"terminal":"message_stop","usage":{"input_tokens":1000,"output_tokens":100,"cache_read_input_tokens":0,"cache_creation_input_tokens":10,"ephemeral_5m_input_tokens":10,"ephemeral_1h_input_tokens":0}});
    for tier in [json!("standard"), Value::Null] {
        let mut v = row.clone();
        v["observed_tier"] = tier;
        assert!(price_range(&v, &launch, &rates).unwrap().2);
    }
    assert!(price_range(&row, &launch, &rates).unwrap().2); // absent tier
    for tier in [
        json!("default"),
        json!("flex"),
        json!(7),
        json!(false),
        json!({}),
    ] {
        let mut v = row.clone();
        v["observed_tier"] = tier;
        assert!(
            price_range(&v, &launch, &rates).is_none(),
            "unestablished Anthropic tier cannot price"
        );
    }
    let mut inconsistent = row.clone();
    inconsistent["usage"]["ephemeral_1h_input_tokens"] = json!(1);
    let range = price_range(&inconsistent, &launch, &rates).unwrap();
    assert!(range.0 <= range.1);
    assert!(
        !range.2,
        "lifetime total exceeds aggregate; supported range is incomplete"
    );
    for key in [
        "input_tokens",
        "output_tokens",
        "cache_read_input_tokens",
        "cache_creation_input_tokens",
    ] {
        for state in [Value::Null, json!("0"), json!(-1), json!(false)] {
            let mut v = row.clone();
            v["usage"][key] = state;
            assert!(
                price_range(&v, &launch, &rates).is_none(),
                "required count {key}"
            );
        }
        let mut v = row.clone();
        v["usage"].as_object_mut().unwrap().remove(key);
        assert!(price_range(&v, &launch, &rates).is_none());
    }
    for terminal in [Value::Null, json!("response.completed"), json!("unknown")] {
        let mut v = row.clone();
        v["terminal"] = terminal;
        assert!(!price_range(&v, &launch, &rates).unwrap().2);
    }
    for status in [json!(429), json!("200"), Value::Null, json!(true)] {
        let mut v = row.clone();
        v["http_status"] = status;
        assert!(!price_range(&v, &launch, &rates).unwrap().2);
    }
    let mut zero = row.clone();
    zero["usage"] = json!({"input_tokens":0,"output_tokens":0,"cache_read_input_tokens":0,"cache_creation_input_tokens":0});
    assert_eq!(price_range(&zero, &launch, &rates), Some((0, 0, true)));
    for usage in [
        Value::Null,
        json!([]),
        json!({}),
        json!({"reasoning_tokens":10}),
    ] {
        let mut v = row.clone();
        v["usage"] = usage;
        assert!(price_range(&v, &launch, &rates).is_none());
    }
}

fn unresolved_billing(a: &Value) -> bool {
    a["obligation"]["state"] == "outstanding" || a["exposure"]["high"]["kind"] == "at_least"
}
fn provisional(attempts: &[Value], attribution: &[Value], o: &Value) -> bool {
    let find = |t: &Value| attempts.iter().find(|a| a["key"] == t["attempt"]).unwrap();
    attribution.iter().any(|t| {
        t["ownership"] == "owned" && t["objective"] == o["objective"] && unresolved_billing(find(t))
    }) || o["predecessor_obligations"]["high"]["kind"] == "at_least"
        || o["commitments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|cm| cm["state"] != "invalid" && cm["kind"] == "at_least")
        || attribution.iter().any(|t| {
            matches!(t["ownership"].as_str(), Some("uncertain" | "unadmitted"))
                && t["candidates"]
                    .as_array()
                    .is_some_and(|cs| cs.contains(&o["objective"]))
                && find(t)["exposure"]["pool"] == "pi_api"
                && unresolved_billing(find(t))
        })
}
#[test]
fn r7_financial_provisionality_and_semantic_mutants() {
    for path in fs::read_dir(root().join("corpus/cases")).unwrap() {
        let c = read(path.unwrap().path());
        let attempts = c["expected"]["attempts"].as_array().unwrap();
        for e in c["evaluations"].as_array().unwrap() {
            for o in e["expected"]["objectives"].as_array().unwrap() {
                assert_eq!(
                    o["provisional"],
                    json!(provisional(
                        attempts,
                        e["expected"]["attribution"].as_array().unwrap(),
                        o
                    )),
                    "{} {} {}",
                    c["case_id"],
                    e["name"],
                    o["objective"]
                );
            }
        }
    }
    let mut o = json!({"objective":"o","commitments":[{"state":"pending","kind":"exact","committed":"100"}],"predecessor_obligations":{"high":{"kind":"exact"}}});
    assert!(
        !provisional(&[], &[], &o),
        "finite Exact commitment alone does not force provisional"
    );
    o["commitments"][0]["kind"] = json!("at_least");
    assert!(provisional(&[], &[], &o));
    o["commitments"][0]["state"] = json!("invalid");
    assert!(!provisional(&[], &[], &o));
    o["predecessor_obligations"]["high"]["kind"] = json!("at_least");
    assert!(provisional(&[], &[], &o));
    let c = corpus_case("P130-R01-usage-incomplete-terminal");
    assert!(c["evaluations"][0]["expected"]["objectives"][0]["provisional"] == true);
    // Self-consistent finite invented output still contradicts the independently pinned raw amount.
    let raw = json!({"launch":{"auth_lane":"subscription"},"reservation":{"event":"usd_reserved","rid":1,"reserved_usd":0.5}});
    let floor = dollars(&raw["reservation"]["reserved_usd"]).unwrap();
    assert_eq!(floor, 500_000_000);
    let mut invented = json!({"exposure":{"high":{"kind":"exact","nano":"0"}},"billing":"established","blocking":[]});
    assert!(!accepts_retention_axis(&invented, floor));
    invented["exposure"]["high"] = json!({"kind":"at_least","nano":"500000000"});
    assert!(accepts_retention_axis(&invented, floor));
    let c = corpus_case("P130-R01-usage-incomplete-terminal");
    let mut wrong = c["expected"]["attempts"][0].clone();
    wrong["replacement"]["eligible"] = json!(true);
    wrong["valuation"]["complete"] = json!(true);
    wrong["exposure"]["high"]["kind"] = json!("exact");
    wrong["blocking"] = json!([]);
    let req: Value = serde_json::from_str(
        c["inputs"]["sessions"][0]["logs"]["turn-1"]["lines"][1]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert!(!terminal(&req, "openai"));
    assert!(wrong["valuation"]["complete"] == true); // raw semantic check rejects this internally coherent assertion
}

// Bounded raw-to-output checks are independent semantic assertions, not a reducer.
// R8-IDENTITY-01: a present identity field that is not a canonical nonempty string is an unpriceable identity (V-1/V-6).
fn malformed_identity(row: &Value) -> bool {
    ["model", "response_model", "authorized_model"].iter().any(|k| {
        // present null is malformed identity evidence (review-r2 D-3); an absent member keeps V-1 handling
        row.get(*k).is_some_and(|v| !v.as_str().is_some_and(|s| !s.is_empty() && s.trim() == s))
    })
}
fn accepts_usage_axis(row: &Value, launch: &Value, rates: &Value, a: &Value) -> bool {
    if malformed_identity(row) {
        // Malformed evidence cannot disappear into a complete favourable set; the independently priced high survives.
        let v = &a["valuation"];
        return v["state"] == "unpriceable"
            && v.get("supported_low").is_none_or(Value::is_null)
            && v["complete"] != true
            && a["replacement"]["eligible"] != true
            && match price_range(row, launch, rates) {
                Some((_, hi, _)) => v["priced_high"] == hi.to_string(),
                None => v.get("priced_high").is_none_or(Value::is_null),
            };
    }
    match price_range(row, launch, rates) {
        None => a["valuation"]["state"] != "valued" && a["replacement"]["eligible"] != true,
        Some((lo, hi, complete)) => {
            a["valuation"]["state"] == "valued"
                && a["valuation"]["low"] == lo.to_string()
                && a["valuation"]["high"] == hi.to_string()
                && a["valuation"]["complete"] == complete
                && (a["replacement"]["eligible"] != true || complete)
        }
    }
}
fn accepts_retention_axis(a: &Value, floor: i128) -> bool {
    a["exposure"]["high"]["kind"] == "at_least"
        && a["exposure"]["high"]["nano"]
            .as_str()
            .and_then(|s| s.parse::<i128>().ok())
            .is_some_and(|n| n >= floor)
}
#[test]
fn r7_semantically_wrong_consistent_outputs_are_rejected() {
    let rates = json!({"openai":read(root().join("oracle/rates/openai_api_rates.json")),"anthropic":read(root().join("oracle/rates/anthropic_api_rates.json"))});
    let c = corpus_case("P130-R01-usage-incomplete-terminal");
    let sess = &c["inputs"]["sessions"][0];
    let row: Value =
        serde_json::from_str(sess["logs"]["turn-1"]["lines"][1].as_str().unwrap()).unwrap();
    let a = &c["expected"]["attempts"][0];
    assert!(accepts_usage_axis(&row, &sess["launch"], &rates, a));
    let mut wrong = a.clone();
    wrong["valuation"]["complete"] = json!(true);
    wrong["replacement"] = json!({"eligible":true,"reason":"UsageBound"});
    wrong["exposure"]["high"] =
        json!({"kind":"exact","nano":wrong["valuation"]["high"],"basis":"usage_valuation"});
    wrong["billing"] = json!("established");
    wrong["blocking"] = json!([]);
    assert!(!accepts_usage_axis(&row, &sess["launch"], &rates, &wrong));
    let raw = json!({"launch":{"auth_lane":"subscription"},"reservation":{"event":"usd_reserved","rid":1,"reserved_usd":0.5}});
    let floor = dollars(&raw["reservation"]["reserved_usd"]).unwrap();
    let a = json!({"exposure":{"high":{"kind":"at_least","nano":"500000000"}},"billing":"unresolved","blocking":["LaneContradiction","UnresolvedExposure"]});
    assert!(accepts_retention_axis(&a, floor));
    let mut wrong = a.clone();
    wrong["exposure"]["high"] = json!({"kind":"exact","nano":"0","basis":"none"});
    wrong["billing"] = json!("established");
    wrong["blocking"] = json!([]);
    assert!(!accepts_retention_axis(&wrong, floor));
    let c = corpus_case("VC02-anthropic-usage-settlement");
    let sess = &c["inputs"]["sessions"][0];
    let mut row: Value =
        serde_json::from_str(sess["logs"]["turn-1"]["lines"][1].as_str().unwrap()).unwrap();
    let a = &c["expected"]["attempts"][0];
    assert!(accepts_usage_axis(&row, &sess["launch"], &rates, a));
    row["observed_tier"] = json!("default");
    assert!(
        !accepts_usage_axis(&row, &sess["launch"], &rates, a),
        "unchanged internally consistent numeric output cannot repair invalid raw tier"
    );
    let c = corpus_case("P130-R01-usage-incomplete-terminal");
    let e = &c["evaluations"][0]["expected"];
    let mut wrong = e["objectives"][0].clone();
    wrong["provisional"] = json!(false);
    assert_ne!(
        wrong["provisional"],
        json!(provisional(
            c["expected"]["attempts"].as_array().unwrap(),
            e["attribution"].as_array().unwrap(),
            &wrong
        ))
    );
}

// Independently authored finite claim consumer. The selected subject supplies actual
// outputs; literals below are derived in pilot-r9/oracle/DERIVATIONS.md, not a generator.
fn r10_admitted() -> Value {
    serde_json::from_str(include_str!("../../../validator/r10_admitted.json")).unwrap()
}
// Dedicated duplicate-rejecting parser: serde_json::Value alone overwrites members.
struct PremiseJson(Value);
impl<'de> serde::Deserialize<'de> for PremiseJson {
 fn deserialize<D:serde::Deserializer<'de>>(d:D)->Result<Self,D::Error>{
  struct Visitor;
  impl<'de> serde::de::Visitor<'de> for Visitor {
   type Value=PremiseJson;
   fn expecting(&self,f:&mut std::fmt::Formatter)->std::fmt::Result{f.write_str("JSON with unique members")}
   fn visit_bool<E:serde::de::Error>(self,v:bool)->Result<PremiseJson,E>{Ok(PremiseJson(json!(v)))}
   fn visit_i64<E:serde::de::Error>(self,v:i64)->Result<PremiseJson,E>{Ok(PremiseJson(json!(v)))}
   fn visit_u64<E:serde::de::Error>(self,v:u64)->Result<PremiseJson,E>{Ok(PremiseJson(json!(v)))}
   fn visit_f64<E:serde::de::Error>(self,v:f64)->Result<PremiseJson,E>{serde_json::Number::from_f64(v).map(|n|PremiseJson(Value::Number(n))).ok_or_else(||E::custom("nonfinite"))}
   fn visit_str<E:serde::de::Error>(self,v:&str)->Result<PremiseJson,E>{Ok(PremiseJson(json!(v)))}
   fn visit_unit<E:serde::de::Error>(self)->Result<PremiseJson,E>{Ok(PremiseJson(Value::Null))}
   fn visit_seq<A:serde::de::SeqAccess<'de>>(self,mut a:A)->Result<PremiseJson,A::Error>{let mut v=Vec::new();while let Some(PremiseJson(x))=a.next_element()?{v.push(x);}Ok(PremiseJson(Value::Array(v)))}
   fn visit_map<A:serde::de::MapAccess<'de>>(self,mut a:A)->Result<PremiseJson,A::Error>{let mut m=serde_json::Map::new();while let Some((k,PremiseJson(v)))=a.next_entry::<String,PremiseJson>()?{if m.insert(k.clone(),v).is_some(){return Err(serde::de::Error::custom(format!("duplicate member {k}")));}}Ok(PremiseJson(Value::Object(m)))}
  }
  d.deserialize_any(Visitor)
 }
}
fn premise_read(p:impl AsRef<std::path::Path>)->Result<Value,String>{
 let bytes=fs::read(p.as_ref()).map_err(|e|format!("{}: {e}",p.as_ref().display()))?;
 serde_json::from_slice::<PremiseJson>(&bytes).map(|v|v.0).map_err(|e|e.to_string())
}
fn finite_premises(r:&std::path::Path,a:&Value)->Result<(),String>{
 let check=||->Result<(),String>{
  let exclusions=a["premise_metadata_exclusions"].as_array().ok_or("exclusions")?;
  for (path,entry) in a["admissions"].as_object().ok_or("admissions")? {
   let source=entry["case_file"].as_str().or_else(||entry["conditional_raw_file"].as_str()).ok_or("source")?;
   let c=premise_read(r.join(source))?;
   let obj=c.as_object().ok_or("record object")?;
   let inventory:Vec<_>=obj.keys().cloned().collect();
   if json!(inventory)!=entry["premise_inventory"] {return Err(format!("{source} top-level inventory"));}
   let mut projected=serde_json::Map::new();
   for (key,value) in obj {
    if key=="expected" || exclusions.iter().any(|x|x==key){continue;}
    let v=if key=="evaluations" {
     Value::Array(value.as_array().ok_or("evaluation array")?.iter().map(|e|{
      let mut ev=e.as_object().ok_or("evaluation object")?.clone();ev.remove("expected");Ok(Value::Object(ev))
     }).collect::<Result<Vec<_>,String>>()?)
    }else{value.clone()};projected.insert(key.clone(),v);
   }
   if Value::Object(projected)!=entry["premise_record"] {return Err(format!("{source} governing values/types/inventory"));}
   let bundle=premise_read(r.join(path))?;
   if bundle["documents"].as_array().map(Vec::len)!=Some(2) || bundle["documents"][0]["kind"]!="accounting_input_proof" {return Err(format!("{path} input proof inventory"));}
   for field in ["origins","initial_contracts","grants","exact_links"] {if bundle["documents"][0][field]!=entry[field] {return Err(format!("R10 exact raw applicability {field}"));}}
   for payload in entry["payloads"].as_object().ok_or("payloads")?.keys(){premise_read(r.join(payload))?;}
  }
  for (path,want) in a["rate_premises"].as_object().ok_or("rates")?{if premise_read(r.join(path))?!=*want{return Err(format!("{path} valuation rate"));}}
  premise_read(r.join("oracle/r9-pilot/claims.json"))?;
  Ok(())
 };
 check().map_err(|e|format!("R10 finite premise outside independent admission: {e}"))
}
fn r10_applicability(r: &std::path::Path, path: &str, admitted: &Value) -> Result<(),String> {
    let a=&admitted["admissions"][path];
    if a.is_null() { return Err("R10 unadmitted selected subject".into()); }
    let b=read(r.join(path)); let inp=&b["documents"][0];
    let inputs=if let Some(p)=a["case_file"].as_str() { read(r.join(p))["inputs"].clone() }
        else { read(r.join(a["conditional_raw_file"].as_str().ok_or("conditional raw selector")?))["original_inputs"].clone() };
    let digest=hash(&serde_json::to_vec(&inputs).unwrap());
    if digest!=a["raw_input_sha256"] {return Err(format!("{path}: R10 raw subject outside independent admission"));}
    for field in ["origins","initial_contracts","grants","exact_links"] {
        if inp[field]!=a[field] {return Err(format!("{path}: R10 exact raw applicability {field}"));}
    }
    for o in inp["origins"].as_array().ok_or("origins")? {
        let source=o["source_origin_id"].as_str().ok_or("source")?;
        let (alias,turn)=source.split_once('/').ok_or("source coordinate")?;
        let pos=o["original_position"].as_u64().ok_or("position")?;
        let session=inputs["sessions"].as_array().ok_or("sessions")?.iter().find(|x|x["alias"]==alias).ok_or("session origin")?;
        let line=session["logs"][turn]["lines"].get(pos.checked_sub(1).ok_or("position0")? as usize).and_then(Value::as_str).ok_or("original line")?;
        if hash(line.as_bytes())!=o["raw_bytes_sha256"] || o["source_revision_ref"]!=format!("sha256:{digest}") {
            return Err("R10 endpoint bytes/position/extent applicability".into());
        }
    }
    for (path,want) in a["payloads"].as_object().ok_or("payloads")? {
        if read(r.join(path))!=*want {return Err("R10 referenced payload fragment/scope/namespace outside independent admission".into());}
    }
    Ok(())
}
fn r10_reservation_inventory(bundle: &Value) -> Result<(),String> {
    let result=&bundle["documents"][1];
    let want=json!([{"origin_refs":["S1/turn-1:1"],"representative_origin_ref":"S1/turn-1:1"},{"origin_refs":["S1/turn-1:2"],"representative_origin_ref":"S1/turn-1:2"}]);
    if result["primary_memberships"].as_array().map(Vec::len)!=Some(1)
        || result["primary_memberships"][0]["reservation_classes"]!=want
        || result["primary_validities"].as_array().map(Vec::len)!=Some(1)
        || result["primary_validities"][0]["distinct_reservation_refs"]!=json!(["S1/turn-1:1","S1/turn-1:2"]) {
        return Err("R10 raw reservation-class actual membership/validity inventory".into());
    }
    Ok(())
}
fn r9_case_qualification(r: &std::path::Path) -> Result<Vec<String>, String> {
    let admitted=r10_admitted();
    finite_premises(r,&admitted)?;
    let registry = read(r.join("oracle/r9-pilot/claims.json"));
    let claims = registry["claims"].as_array().ok_or("missing R9 registry")?;
    let required = [
        ("P9-RETAINED-CHARGE", "retained_charge"),
        ("PRIMARY/ANCHOR-APPLICATION", "primary_anchor"),
        ("RESPONSE-MODEL7-ACTUAL", "malformed_response_model"),
        ("EQUAL-AMOUNT-DISTINCT-RESERVATIONS", "reservation_classes"),
    ];
    let admitted=r10_admitted();
    let mut seen=BTreeSet::new();
    let mut consumed = BTreeSet::new();
    for claim in claims {
        let id = claim["claim_id"].as_str().ok_or("claim id")?;
        if !required
            .iter()
            .any(|(i, t)| *i == id && claim["relation"] == *t)
            || !seen.insert(id.to_string())
        {
            return Err("missing/duplicate/inapplicable R9 claim".into());
        }
        let cases = claim["cases"].as_array().ok_or("missing cases")?;
        if cases.is_empty() {
            return Err("unconsumed claim".into());
        }
        let expected_cases: Vec<&str> = match id {
            "P9-RETAINED-CHARGE" => vec!["P9-claimed-charge-above-reservation"],
            "PRIMARY/ANCHOR-APPLICATION" => vec![
                "VC11-multi-turn-session",
                "P132-FG-VC11-gated",
                "VC07-read-failure-paired",
            ],
            "RESPONSE-MODEL7-ACTUAL" => vec![
                "VC01-openai-usage-settlement",
                "A04-undated-settlement-between-dated-neighbours",
            ],
            _ => vec!["VC01-openai-usage-settlement"],
        };
        let actual_cases: BTreeSet<_> = cases
            .iter()
            .map(|b| b["case_id"].as_str().unwrap_or(""))
            .collect();
        if cases.len() != expected_cases.len()
            || actual_cases != expected_cases.into_iter().collect()
        {
            return Err("unconsumed/missing/inapplicable fixed case binding".into());
        }
        for b in cases {
            let cid = b["case_id"].as_str().ok_or("case id")?;
            let key=format!("{}:{cid}",claim["relation"].as_str().ok_or("relation")?);
            let selected=&admitted["selected"][&key];
            if selected.is_null() {return Err("R10 inapplicable relation subject".into());}
            for field in ["control_file","bundle_file","raw_file"] {
                if b[field]!=selected[field] && !(field=="bundle_file" && selected["control_file"].is_null() && b[field].is_null()) {
                    return Err(format!("R10 inapplicable relation selector {field}"));
                }
            }
            r10_applicability(r,selected["bundle_file"].as_str().ok_or("bundle selector")?,&admitted)?;
            let c = if let Some(p) = b["control_file"].as_str() {
                read(r.join("oracle/r9-pilot").join(p))
            } else {
                read(r.join(format!("corpus/cases/{cid}.json")))
            };
            let mut raw = std::collections::BTreeMap::new();
            for s in c["inputs"]["sessions"].as_array().ok_or("sessions")? {
                for (turn, log) in s["logs"].as_object().ok_or("logs")? {
                    for (i, line) in log["lines"].as_array().ok_or("lines")?.iter().enumerate() {
                        raw.insert(
                            format!("{}/{turn}:{}", s["alias"].as_str().unwrap(), i + 1),
                            line.as_str().ok_or("raw line")?.to_string(),
                        );
                    }
                }
            }
            let ps = b["origins"].as_array().ok_or("premises")?;
            if raw.len() != ps.len() {
                return Err(format!("{cid}: raw inventory premise"));
            }
            for p in ps {
                let oid = p["origin_ref"].as_str().ok_or("origin")?;
                let l = raw.get(oid).ok_or("missing raw origin")?;
                if hash(l.as_bytes()) != p["raw_bytes_sha256"]
                    || format!(
                        "{}:{}",
                        p["source_origin_id"].as_str().unwrap(),
                        p["original_position"]
                    ) != oid
                {
                    return Err(format!("{cid}: exact raw premise"));
                }
            }
            let attempts = c["expected"]["attempts"].as_array().ok_or("attempts")?;
            if id == "P9-RETAINED-CHARGE" {
                let a = attempts
                    .iter()
                    .find(|a| a["key"] == b["attempt_key"])
                    .ok_or("P9 attempt")?;
                let row: Value = serde_json::from_str(&raw["S1/turn-1:2"]).map_err(|_| "P9 raw")?;
                let reservation: Value = serde_json::from_str(&raw["S1/turn-1:1"]).unwrap();
                if dollars(&row["usd"]["charged_usd"]) != Some(700_000_000)
                    || dollars(&row["usd"]["reserved_usd"]) != Some(500_000_000)
                    || dollars(&reservation["reserved_usd"]) != Some(400_000_000)
                {
                    return Err("P9 independent raw component premises".into());
                }
                let components = b["components"]
                    .as_array()
                    .ok_or("missing component premises")?;
                let cs: BTreeSet<_> = components
                    .iter()
                    .map(|x| {
                        (
                            x["origin_ref"].as_str().unwrap_or(""),
                            x["kind"].as_str().unwrap_or(""),
                            x["nano"].as_str().unwrap_or(""),
                        )
                    })
                    .collect();
                let want: BTreeSet<_> = [
                    ("S1/turn-1:1", "reservation", "400000000"),
                    ("S1/turn-1:2", "claimed_reserved", "500000000"),
                    ("S1/turn-1:2", "charge", "700000000"),
                    ("S1/turn-1:2", "usage", "15000000"),
                ]
                .into_iter()
                .collect();
                if components.len() != 4 || cs != want {
                    return Err("P9 independent component premises disagree with raw".into());
                }
                if a["settlement"]["charged"] != "700000000"
                    || a["settlement"]["reserved"] != "500000000"
                    || a["exposure"]["high"]["nano"] != "700000000"
                    || a["valuation"]["high"] != "15000000"
                {
                    return Err("P9 raw retained charge700M".into());
                }
                let ev = c["evaluations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|e| e["name"] == b["evaluation"])
                    .ok_or("P9 evaluation")?;
                let ob = ev["expected"]["objectives"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|o| o["objective"] == b["objective"])
                    .ok_or("P9 objective")?;
                if ob["owned_outstanding"]["high"]["nano"] != "700000000"
                    || !ob["commitments"].as_array().unwrap().iter().any(|x| {
                        x["admission_id"] == b["admission_id"] && x["committed"] == "300000000"
                    })
                {
                    return Err("P9 raw rollup/commitment700M/300M".into());
                }
                let bundle = read(r.join(
                    "oracle/r8-completion-B/P9-claimed-charge-above-reservation.bundle.json",
                ));
                let result = &bundle["documents"][1];
                let cs = result["retained_components"]
                    .as_array()
                    .ok_or("P9 retained component inventory missing")?;
                let actual: BTreeSet<_> = cs
                    .iter()
                    .map(|x| {
                        (
                            x["origin_ref"].as_str().unwrap_or(""),
                            x["role"].as_str().unwrap_or(""),
                            x["nano_usd"].as_str().unwrap_or(""),
                        )
                    })
                    .collect();
                let want: BTreeSet<_> = [
                    ("S1/turn-1:1", "reservation", "400000000"),
                    ("S1/turn-1:2", "claimed_reserved", "500000000"),
                    ("S1/turn-1:2", "charge", "700000000"),
                    ("S1/turn-1:2", "usage", "15000000"),
                ]
                .into_iter()
                .collect();
                if cs.len() != 4 || actual != want {
                    return Err("P9 raw retained component inventory400/500/700/15M".into());
                }
                let ids: BTreeSet<_> = cs
                    .iter()
                    .map(|x| x["component_id"].clone().to_string())
                    .collect();
                let unit = result["units"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|u| u["unit_id"] == b["attempt_key"] && u["state"] == "active")
                    .ok_or("P9 component owner")?;
                let refs: BTreeSet<_> = unit["retained_component_refs"]
                    .as_array()
                    .ok_or("P9 retention references")?
                    .iter()
                    .map(|x| x.to_string())
                    .collect();
                if ids.len() != 4 || ids != refs {
                    return Err("P9 retained component owner/reference mismatch".into());
                }
            } else if id == "PRIMARY/ANCHOR-APPLICATION" {
                if cid=="P132-FG-VC11-gated" {
                    let gate=&c["inputs"]["gates"][0];
                    let witness=&c["trusted_fixture_inputs"]["gate_verifications"]["g-S1"];
                    let commitments=&c["evaluations"][0]["expected"]["objectives"][0]["commitments"];
                    let want=json!([{"admission_id":"a1","committed":"0","kind":"exact","state":"retired","gate":{"qualified":true,"reasons":[],"verification_source":"test_mock","verification_payload_sha256":witness["payload_sha256"]}}]);
                    if !mock_binding(&c,gate) || *commitments!=want {return Err("R10 finite trusted gate consequence".into());}
                }
                let part = if cid.starts_with("P132") { "A" } else { "B" };
                let bundle = read(r.join(format!("oracle/r8-completion-{part}/{cid}.bundle.json")));
                let result = &bundle["documents"][1];
                if matches!(cid, "VC11-multi-turn-session" | "P132-FG-VC11-gated") {
                    let mut expected = vec![];
                    for selector in b["obligations"].as_array().ok_or("obligation selectors")? {
                        let rr = selector["reservation_origin_ref"].clone();
                        let ss = selector["settlement_origin_ref"].clone();
                        expected.push(json!({"obligation_id":selector["obligation_id"],"reservation_classes":[{"origin_refs":[rr],"representative_origin_ref":rr}],"settlement_classes":[{"origin_refs":[ss],"representative_origin_ref":ss}],"usage_origin_refs":[ss]}));
                    }
                    let raw_primary: BTreeSet<_> = raw
                        .iter()
                        .filter_map(|(o, l)| {
                            let row: Value = serde_json::from_str(l).ok()?;
                            if row["event"] == "usd_reserved" || row["usd"].is_object() {
                                Some(o.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    let claimed: BTreeSet<_> = expected
                        .iter()
                        .flat_map(|m| {
                            ["reservation_classes", "settlement_classes"]
                                .into_iter()
                                .flat_map(move |role| {
                                    m[role].as_array().unwrap().iter().flat_map(|cl| {
                                        cl["origin_refs"]
                                            .as_array()
                                            .unwrap()
                                            .iter()
                                            .map(|o| o.as_str().unwrap().to_string())
                                    })
                                })
                        })
                        .collect();
                    if raw_primary != claimed {
                        return Err("R9 registered primary premises omit raw evidence".into());
                    }
                    if result["primary_memberships"] != json!(expected) {
                        return Err(format!("{cid}: exhaustive independent raw membership"));
                    }
                    let mut wanted_components = BTreeSet::new();
                    for m in &expected {
                        let rr = m["reservation_classes"][0]["representative_origin_ref"]
                            .as_str()
                            .unwrap()
                            .to_string();
                        let sr = m["settlement_classes"][0]["representative_origin_ref"]
                            .as_str()
                            .unwrap()
                            .to_string();
                        wanted_components.insert((
                            rr,
                            "reservation".to_string(),
                            "500000000".to_string(),
                        ));
                        for (role, amount) in [
                            ("claimed_reserved", "500000000"),
                            ("charge", "15000000"),
                            ("usage", "15000000"),
                        ] {
                            wanted_components.insert((
                                sr.clone(),
                                role.to_string(),
                                amount.to_string(),
                            ));
                        }
                    }
                    let cs = result["retained_components"]
                        .as_array()
                        .ok_or("VC11 retained component inventory")?;
                    let actual_components: BTreeSet<_> = cs
                        .iter()
                        .map(|x| {
                            (
                                x["origin_ref"].as_str().unwrap_or("").to_string(),
                                x["role"].as_str().unwrap_or("").to_string(),
                                x["nano_usd"].as_str().unwrap_or("").to_string(),
                            )
                        })
                        .collect();
                    if cs.len() != 8 || actual_components != wanted_components {
                        return Err("VC11 retained facts disagree with raw500/500/15/15M".into());
                    }
                    for m in &expected {
                        let validity = result["primary_validities"]
                            .as_array()
                            .ok_or("primary validities")?
                            .iter()
                            .find(|v| v["obligation_id"] == m["obligation_id"])
                            .ok_or("raw primary witness missing")?;
                        if validity["distinct_reservation_refs"]
                            != json!([m["reservation_classes"][0]["representative_origin_ref"]])
                            || validity["distinct_settlement_refs"]
                                != json!([m["settlement_classes"][0]["representative_origin_ref"]])
                        {
                            return Err("raw primary witness application".into());
                        }
                        let covers: Vec<_> = result["coverage_decisions"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter(|c| {
                                c["status"] == "accepted" && c["validity_ref"] == m["obligation_id"]
                            })
                            .collect();
                        if covers.len() != 1 || covers[0]["obligation_id"] != m["obligation_id"] {
                            return Err("raw primary exact coverage/witness application".into());
                        }
                        let cover = covers[0];
                        let money = cover["monetary_origin_refs"]
                            .as_array()
                            .ok_or("covered monetary")?;
                        for role in ["reservation_classes", "settlement_classes"] {
                            for cl in m[role].as_array().unwrap() {
                                for o in cl["origin_refs"].as_array().unwrap() {
                                    if !money.contains(o) {
                                        return Err("raw primary coverage omitted original".into());
                                    }
                                }
                            }
                        }
                        let valuation = result["valuation_records"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|v| v["valuation_id"] == cover["valuation_ref"])
                            .ok_or("coverage valuation")?;
                        if valuation["usage_origin_refs"] != m["usage_origin_refs"] {
                            return Err("raw primary exact valuation usage application".into());
                        }
                    }
                } else {
                    let cs = result["retained_components"]
                        .as_array()
                        .ok_or("historical VC07 retained usage")?;
                    if cs.len() != 1
                        || cs[0]["origin_ref"] != "S1/turn-1:2"
                        || cs[0]["role"] != "usage"
                        || cs[0]["nano_usd"] != "15000000"
                    {
                        return Err("VC07 retained usage disagrees with raw15M/no-dollar".into());
                    }
                    if result["coverage_decisions"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|x| {
                            x["anchor_origin_refs"]
                                .as_array()
                                .is_some_and(|a| !a.is_empty())
                        })
                    {
                        return Err("VC07 historical unlinked anchor cannot release".into());
                    }
                    if !attempts.iter().any(|a| {
                        a["key"] == "row:S1/turn-1:line=1"
                            && a["exposure"]["high"]["kind"] == "at_least"
                            && a["exposure"]["high"]["nano"] == "0"
                    }) {
                        return Err("VC07 retained unlinked anchor".into());
                    }
                }
            } else if id == "RESPONSE-MODEL7-ACTUAL" {
                let a = attempts
                    .iter()
                    .find(|a| a["key"] == b["attempt_key"])
                    .ok_or("model7 attempt")?;
                let row: Value =
                    serde_json::from_str(raw.get(b["origin_ref"].as_str().unwrap()).unwrap())
                        .unwrap();
                let rates = json!({"openai":read(r.join("oracle/rates/openai_api_rates.json")),"anthropic":read(r.join("oracle/rates/anthropic_api_rates.json"))});
                if b["response_model_state"] != "present_number"
                    || b["response_model_value"] != 7
                    || row.get("response_model") != Some(&json!(7))
                    || price_range(&row, &c["inputs"]["sessions"][0]["launch"], &rates).map(|(_,hi,_)|hi)!=Some(15_000_000)
                    || a["valuation"]!=json!({"state":"unpriceable","reasons":["IdentityWithoutRate"],"supported_low":null,"priced_high":"15000000"})
                    || a["exposure"]["low"]!=json!({"nano":"0","basis":"none"})
                    || a["replacement"]!=json!({"eligible":false,"reason":"UsageUnpriceable"})
                    || a["exposure"]["pool"]!="pi_api"
                    || a["exposure"]["high"]["basis"]!="proxy_reservation"
                    || a["replacement"]["eligible"] == true
                    || a["exposure"]["high"]["kind"] != "at_least"
                    || a["exposure"]["high"]["nano"] != "500000000"
                {
                    return Err("model7 actual malformed identity/conservative hold; R10 model7 unsupported minimum/priced high/conservative hold".into());
                }
            } else {
                let a = attempts
                    .iter()
                    .find(|a| a["key"] == b["attempt_key"])
                    .ok_or("reservation attempt")?;
                let refs = b["reservation_origin_refs"]
                    .as_array()
                    .ok_or("reservations")?;
                let bundle = read(
                    r.join(
                        b["bundle_file"]
                            .as_str()
                            .ok_or("reservation bundle selector")?,
                    ),
                );
                r10_reservation_inventory(&bundle)?;
                r10_reservation_consequence(&c, a, b, &raw, &bundle)?;
                let provenance =
                    read(r.join("oracle/r9-pilot/cases/equal-reservations.provenance.json"));
                let scope = &provenance["grant_payloads"][0];
                let expected_origins: Vec<_> = raw.keys().cloned().collect();
                if scope["source_origin_id"] != "S1/turn-1"
                    || scope["exact_origin_refs"] != json!(expected_origins)
                    || provenance["raw_input_sha256"] != b["raw_input_sha256"]
                    || bundle["documents"][0]["grants"][0]["covered_origin_refs"]
                        != scope["exact_origin_refs"]
                    || bundle["documents"][0]["grants"][0]["exact_payload_ref"]
                        != "oracle/r9-pilot/cases/equal-reservations.provenance.json#S1/turn-1"
                {
                    return Err("equal-reservation exact inherited scope extent".into());
                }
                let classes: BTreeSet<_> = refs
                    .iter()
                    .map(|x| raw[x.as_str().unwrap()].clone())
                    .collect();
                if b["expected_classes"] != 2
                    || b["maximum_reservation_nano"] != "500000000"
                    || b["comparison_applicable"] != false
                    || classes.len() != 2
                    || classes.iter().any(|l| {
                        dollars(&serde_json::from_str::<Value>(l).unwrap()["reserved_usd"])
                            != Some(500_000_000)
                    })
                    || a["replacement"]["eligible"] == true
                    || a["settlement"]["decision"] == "valid"
                {
                    return Err("reservation evidence cardinality2 despite max500M".into());
                }
            }
            if id=="RESPONSE-MODEL7-ACTUAL" || id=="EQUAL-AMOUNT-DISTINCT-RESERVATIONS" {
                let bundle=read(r.join(b["bundle_file"].as_str().ok_or("selected bundle")?));
                r10_selected_low_consequence(&c,b,&bundle)?;
            }
            println!("R10 evaluated predicate={} subject={} bundle={} outcome=pass",claim["relation"],selected["case_file"],selected["bundle_file"]);
        }
        consumed.insert(id.to_string());
    }
    r10_applicability(r,"oracle/r8-controls/VC07-conditional.bundle.json",&admitted)?;
    r10_applicability(r,"oracle/r8-controls/VC07-unlinked.bundle.json",&admitted)?;
    r10_composed_reservations(r,&admitted)?;
    r10_conditional_consequence(r)?;
    let anchor_binding = claims
        .iter()
        .find(|c| c["claim_id"] == "PRIMARY/ANCHOR-APPLICATION")
        .unwrap()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["case_id"] == "VC07-read-failure-paired")
        .unwrap();
    if hash(
        &fs::read(r.join("oracle/r8-controls/VC07-exact.raw.json"))
            .map_err(|_| "conditional raw")?,
    ) != anchor_binding["conditional_raw_sha256"]
    {
        return Err("VC07 conditional raw premise custody changed".into());
    }
    let control_raw = read(r.join("oracle/r8-controls/VC07-exact.raw.json"));
    let conditional = read(r.join("oracle/r8-controls/VC07-conditional.bundle.json"));
    let assumption = &control_raw["conditional_harness_assumptions"]["exact_row_coreference"];
    let grant = conditional["documents"][0]["grants"]
        .as_array()
        .ok_or("conditional grants")?
        .iter()
        .find(|g| g["guarantee_kind"] == "exact_row_coreference")
        .ok_or("conditional grant")?;
    if grant["admitted_payload"] != json!({"from":assumption["from"],"to":assumption["to"]})
        || grant["exact_payload_ref"] != "VC07-exact.raw.json"
    {
        return Err("VC07 conditional exact payload differs from raw assumption".into());
    }
    let cs = conditional["documents"][1]["retained_components"]
        .as_array()
        .ok_or("conditional retained usage")?;
    if cs.len() != 1
        || cs[0]["origin_ref"] != "u"
        || cs[0]["role"] != "usage"
        || cs[0]["nano_usd"] != "15000000"
    {
        return Err("conditional VC07 retained usage disagrees with raw15M/no-dollar".into());
    }
    if consumed.len() != 4 {
        return Err("missing mandatory R9 claims".into());
    }
    Ok(consumed.into_iter().collect())
}
#[test]
fn r9_registered_actual_subject_qualification() {
    let ids = r9_case_qualification(&root())
        .unwrap_or_else(|e| panic!("R9 finite semantic qualification: {e}"));
    println!(
        "R9 consumed{} claims: {:?}; finite pilot only",
        ids.len(),
        ids
    );
}
fn r9_copy_subject(src: &std::path::Path, dst: &std::path::Path) {
    fs::create_dir_all(dst).unwrap();
    for e in fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        if e.file_name() == "target" || e.file_name() == "__pycache__" {
            continue;
        }
        let p = dst.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            r9_copy_subject(&e.path(), &p)
        } else {
            fs::copy(e.path(), p).unwrap();
        }
    }
}
fn r9_edit(r: &std::path::Path, rel: &str, f: impl FnOnce(&mut Value)) {
    let p = r.join(rel);
    let mut d = read(&p);
    f(&mut d);
    fs::write(p, serde_json::to_vec_pretty(&d).unwrap()).unwrap();
}
#[test]
fn r9_consistent_wrong_actual_subjects_rejected() {
    let base = root();
    let dir = std::env::temp_dir().join(format!("r9-rust-{}", std::process::id()));
    for kind in [
        "p9",
        "components",
        "primary",
        "model7",
        "cardinality",
        "missing",
        "duplicate",
        "unconsumed",
    ] {
        let subject = dir.join(kind);
        r9_copy_subject(&base, &subject);
        let reason = match kind {
            "p9" => {
                r9_edit(
                    &subject,
                    "corpus/cases/P9-claimed-charge-above-reservation.json",
                    |c| {
                        let a = &mut c["expected"]["attempts"][0];
                        a["settlement"]["charged"] = json!("500000000");
                        a["exposure"]["high"]["nano"] = json!("500000000");
                        let o = &mut c["evaluations"][0]["expected"]["objectives"][0];
                        o["owned_outstanding"]["high"]["nano"] = json!("500000000");
                        o["commitments"][0]["committed"] = json!("500000000");
                    },
                );
                r9_edit(
                    &subject,
                    "oracle/r8-completion-B/P9-claimed-charge-above-reservation.bundle.json",
                    |b| {
                        b["documents"][1]=serde_json::from_str(&serde_json::to_string(&b["documents"][1]).unwrap().replace("700000000","500000000").replace("300000000","500000000")).unwrap();
                    },
                );
                "P9 raw retained charge700M"
            }
            "components" => {
                r9_edit(
                    &subject,
                    "oracle/r8-completion-B/P9-claimed-charge-above-reservation.bundle.json",
                    |b| {
                        b["documents"][1]["retained_components"][2]["nano_usd"] = json!("500000000")
                    },
                );
                "P9 raw retained component"
            }
            "primary" => {
                r9_edit(
                    &subject,
                    "oracle/r8-completion-B/VC11-multi-turn-session.bundle.json",
                    |b| {
                        let r = &mut b["documents"][1];
                        let omitted = r["primary_memberships"][1]["obligation_id"].clone();
                        r["primary_memberships"]
                            .as_array_mut()
                            .unwrap()
                            .retain(|m| m["obligation_id"] != omitted);
                        r["primary_validities"]
                            .as_array_mut()
                            .unwrap()
                            .retain(|m| m["obligation_id"] != omitted);
                        r["coverage_decisions"]
                            .as_array_mut()
                            .unwrap()
                            .retain(|c| c["validity_ref"] != omitted);
                    },
                );
                "exhaustive independent raw membership"
            }
            "model7" => {
                r9_edit(
                    &subject,
                    "oracle/r9-pilot/cases/model7-VC01-openai-usage-settlement.json",
                    |c| {
                        let a = &mut c["expected"]["attempts"][0];
                        a["valuation"]["complete"] = json!(true);
                        a["replacement"]["eligible"] = json!(true);
                        a["exposure"]["high"] =
                            json!({"kind":"exact","nano":"15000000","basis":"usage_valuation"});
                    },
                );
                "model7 actual malformed identity"
            }
            "cardinality" => {
                r9_edit(
                    &subject,
                    "oracle/r9-pilot/cases/equal-reservations.json",
                    |c| {
                        c["expected"]["attempts"][0]["settlement"]["decision"] = json!("valid");
                        c["expected"]["attempts"][0]["replacement"]["eligible"] = json!(true);
                    },
                );
                "raw reservation outstanding obligation/replacement"
            }
            "missing" => {
                r9_edit(&subject, "oracle/r9-pilot/claims.json", |d| {
                    d["claims"].as_array_mut().unwrap().pop();
                });
                "missing mandatory"
            }
            "duplicate" => {
                r9_edit(&subject, "oracle/r9-pilot/claims.json", |d| {
                    d["claims"][1] = d["claims"][0].clone()
                });
                "duplicate"
            }
            _ => {
                r9_edit(&subject, "oracle/r9-pilot/claims.json", |d| {
                    d["claims"][2]["cases"] = json!([])
                });
                "unconsumed"
            }
        };
        let err = r9_case_qualification(&subject).expect_err(kind);
        assert!(err.contains(reason), "{kind}: expected {reason}, got {err}");
        fs::remove_dir_all(subject).unwrap();
    }
    fs::remove_dir_all(dir).unwrap();
}
#[test]
fn r9_priced_multiidentity_and_reservation_evidence_controls() {
    let rates = json!({"openai":{"models":{"m1":{"input":20,"cached_input":2,"cache_write":25,"output":0},"m2":{"input":80,"cached_input":8,"cache_write":100,"output":0}},"tier_multiplier":{"default":1},"long_input_threshold":272000,"long_input_multiplier":2,"long_output_multiplier":1.5}});
    let launch = json!({"model":"openai/m1"});
    let row = json!({"model":"m1","response_model":"m2","observed_tier":"default","http_status":200,"terminal":"response.completed","usage":{"input_tokens":1000,"output_tokens":0,"cached_tokens":0,"cache_write_tokens":0}});
    assert_eq!(
        price_range(&row, &launch, &rates),
        Some((20_000_000, 80_000_000, true))
    );
    let mut malformed = row.clone();
    malformed["response_model"] = json!(7);
    assert_eq!(
        price_range(&malformed, &launch, &rates),
        Some((20_000_000, 20_000_000, false))
    );
    // Original identity, equality and admitted copy relation are distinct axes.
    fn classes(observed: &[(&str, &str)], admitted: Option<(&str, &str)>) -> usize {
        let origins: std::collections::BTreeMap<_, _> = observed.iter().copied().collect();
        let mut n = origins.len();
        if let Some((a, b)) = admitted {
            assert_ne!(a, b);
            assert_eq!(origins.get(a), origins.get(b));
            assert!(origins.contains_key(a));
            n -= 1;
        }
        n
    }
    let a = "{\"reserved_usd\":0.5,\"at\":1}";
    let b = "{\"reserved_usd\":0.5,\"at\":2}";
    assert_eq!(classes(&[("o1", a), ("o2", b)], None), 2);
    assert_eq!(classes(&[("o1", a), ("o1", a)], None), 1);
    assert_eq!(classes(&[("o1", a), ("o2", a)], None), 2);
    assert_eq!(classes(&[("o1", a), ("o2", a)], Some(("o1", "o2"))), 1);
}

fn r10_conditional_consequence(r:&std::path::Path)->Result<(),String>{
 for (name,linked) in [("conditional",true),("unlinked",false)] {
  let b=read(r.join(format!("oracle/r8-controls/VC07-{name}.bundle.json")));let result=&b["documents"][1];
  let active:std::collections::BTreeMap<_,_>=result["units"].as_array().ok_or("conditional units")?.iter().filter(|u|u["state"]=="active").map(|u|(u["unit_id"].as_str().unwrap(),json!([u["origin_refs"],u["exposure_ref"]]))).collect();
  let want=if linked{json!({"unit:F:a-u":[["a","u"],"covered:Exact15000000"]})}else{json!({"unit:a":[["a"],"anchor:AtLeast0"],"unit:u":[["u"],"usage:Exact15000000"]})};
  let owners:std::collections::BTreeMap<_,_>=result["origins"].as_array().ok_or("conditional origins")?.iter().map(|o|(o["origin_ref"].as_str().unwrap(),o["active_high_unit_id"].as_str().unwrap())).collect();
  let wanted_owners=if linked{json!({"a":"unit:F:a-u","u":"unit:F:a-u"})}else{json!({"a":"unit:a","u":"unit:u"})};
  if json!(active)!=want || json!(owners)!=wanted_owners{return Err("R10 raw conditional anchor/usage active financial inventory".into());}
  let covers=result["coverage_decisions"].as_array().ok_or("coverage")?;let links=result["definite_links"].as_array().ok_or("links")?;
  if linked {
   if covers.len()!=1||covers[0]["status"]!="accepted"||covers[0]["anchor_origin_refs"]!=json!(["a"])||covers[0]["monetary_origin_refs"]!=json!(["u"])||covers[0]["definite_link_refs"]!=json!(["result-link:a-u"])||links.len()!=1||links[0]["from_origin_ref"]!="a"||links[0]["to_origin_ref"]!="u"{return Err("R10 raw exact-link conditional financial consequence".into());}
  }else if !covers.is_empty()||!links.is_empty(){return Err("R10 unlinked raw anchor cannot be covered/released".into());}
  println!("R10 evaluated predicate=primary_anchor_conditional subject=oracle/r8-controls/VC07-{name}.bundle.json outcome=pass");
 }
 Ok(())
}
fn r10_composed_reservations(r: &std::path::Path,a: &Value)->Result<(),String>{
    for control in a["reservation_controls"].as_array().ok_or("reservation controls")? {
        let path=control["bundle_file"].as_str().ok_or("control bundle")?;
        r10_applicability(r,path,a)?;
        let bundle=read(r.join(path));let inp=&bundle["documents"][0];let result=&bundle["documents"][1];
        let observed:BTreeSet<_>=control["observations"].as_array().ok_or("observations")?.iter().map(|x|x.as_str().unwrap()).collect();
        let mut classes:Vec<(String,Vec<&str>)>=vec![];
        for oid in observed {
            let o=inp["origins"].as_array().ok_or("origins")?.iter().find(|o|o["origin_id"]==oid).ok_or("observed origin")?;
            let key=format!("{}/{}",o["source_origin_id"],o["raw_bytes_sha256"]);
            let admitted_copy=inp["grants"].as_array().unwrap().iter().any(|g|g["guarantee_kind"]=="exact_copy_equivalence"&&g["covered_origin_refs"].as_array().unwrap().contains(&json!(oid)));
            if let Some((_,c))=classes.iter_mut().find(|(k,_)|admitted_copy&&*k==key){c.push(oid)}else{classes.push((key,vec![oid]));}
        }
        let want:Vec<_>=classes.iter().map(|(_,c)|json!({"origin_refs":c,"representative_origin_ref":c[0]})).collect();
        let reps:Vec<_>=classes.iter().map(|(_,c)|c[0]).collect();
        if Some(classes.len() as u64)!=control["expected_classes"].as_u64() || result["primary_memberships"][0]["reservation_classes"]!=json!(want) || result["primary_validities"][0]["distinct_reservation_refs"]!=json!(reps){return Err("R10 composed raw reservation class inventory".into());}
        println!("R10 composed reservation {} classes={} outcome=pass",control["label"],classes.len());
    }
    Ok(())
}
#[test]
fn r10_exact_applicability_and_registration_mutants(){
    let base=root();let temp=std::env::temp_dir().join(format!("r10-rust-{}",std::process::id()));
    for kind in ["extent","interchanged","fragment","scope","namespace","endpoint","link","copy","stale-payload","omitted-class","copy-merge","raw-extent","exact-link","unlinked-erasure","route"] {
        let s=temp.join(kind);r9_copy_subject(&base,&s);
        let eq="oracle/r9-pilot/cases/equal-reservations.bundle.json";
        let vc="oracle/r8-completion-B/VC11-multi-turn-session.bundle.json";
        let conditional="oracle/r8-controls/VC07-conditional.bundle.json";
        let reason=match kind {
            "extent"=>{r9_edit(&s,eq,|d|d["documents"][0]["grants"][0]["covered_extent_ref"]=json!(format!("sha256:{}","0".repeat(64))));"applicability grants"},
            "interchanged"=>{r9_edit(&s,vc,|d|d["documents"][0]["grants"][0]["exact_payload_ref"]=json!("oracle/r8-completion-B/P9-claimed-charge-above-reservation.provenance.json#S1/turn-1"));"applicability grants"},
            "fragment"=>{r9_edit(&s,vc,|d|d["documents"][0]["grants"][0]["exact_payload_ref"]=json!("oracle/r8-completion-B/VC11-multi-turn-session.provenance.json#S1/turn-2"));"applicability grants"},
            "scope"=>{r9_edit(&s,eq,|d|d["documents"][0]["initial_contracts"][0]["scope_id"]=json!("different"));"applicability initial_contracts"},
            "namespace"=>{r9_edit(&s,eq,|d|d["documents"][0]["initial_contracts"][0]["reference_namespace"]=json!("different"));"applicability initial_contracts"},
            "endpoint"=>{r9_edit(&s,conditional,|d|d["documents"][0]["origins"][0]["raw_bytes_sha256"]=json!("0".repeat(64)));"applicability origins"},
            "link"=>{r9_edit(&s,conditional,|d|d["documents"][0]["grants"][0]["admitted_payload"]["to"]=json!("a"));"applicability grants"},
            "copy"=>{r9_edit(&s,eq,|d|d["documents"][0]["initial_contracts"][0]["duplicate_rule"]=json!("scope_exact_line"));"applicability initial_contracts"},
            "stale-payload"=>{r9_edit(&s,"oracle/r9-pilot/cases/model7-VC01-openai-usage-settlement.provenance.json",|d|d["raw_input_sha256"]=json!("0".repeat(64)));"referenced payload fragment/scope/namespace"},
            "unlinked-erasure"=>{r9_edit(&s,"oracle/r8-controls/VC07-unlinked.bundle.json",|d|d["documents"][1]["units"][0]["exposure_ref"]=json!("covered:Exact15000000"));"raw conditional anchor/usage active financial inventory"},
            "raw-extent"=>{r9_edit(&s,"oracle/r9-pilot/cases/equal-reservations.json",|d|{let line=d["inputs"]["sessions"][0]["logs"]["turn-1"]["lines"][0].as_str().unwrap().to_string()+" ";d["inputs"]["sessions"][0]["logs"]["turn-1"]["lines"][0]=json!(line);});"finite premise outside independent admission"},
            "exact-link"=>{r9_edit(&s,conditional,|d|d["documents"][0]["exact_links"][0]["to_origin_ref"]=json!("a"));"applicability exact_links"},
            "copy-merge"=>{r9_edit(&s,eq,|d|{d["documents"][1]["primary_memberships"][0]["reservation_classes"]=json!([{"origin_refs":["S1/turn-1:1","S1/turn-1:2"],"representative_origin_ref":"S1/turn-1:1"}]);d["documents"][1]["primary_validities"][0]["distinct_reservation_refs"]=json!(["S1/turn-1:1"]);});"raw reservation-class actual membership/validity"},
            "omitted-class"=>{r9_edit(&s,eq,|d|{d["documents"][1]["primary_memberships"][0]["reservation_classes"].as_array_mut().unwrap().pop();d["documents"][1]["primary_validities"][0]["distinct_reservation_refs"].as_array_mut().unwrap().pop();});"raw reservation-class actual membership/validity"},
            _=>{r9_edit(&s,"oracle/r9-pilot/claims.json",|d|{d["claims"][0]["cases"][0]["control_file"]=json!("../../corpus/cases/P9-claimed-charge-above-reservation.json");d["claims"][0]["cases"][0]["bundle_file"]=json!("oracle/r8-completion-B/P9-claimed-charge-above-reservation.bundle.json");});"inapplicable relation selector"},
        };
        let e=r9_case_qualification(&s).expect_err(kind);assert!(e.contains(reason),"{kind}: expected {reason}, got {e}");println!("R10 negative {kind}: {e}");fs::remove_dir_all(&s).unwrap();
    }
    fs::remove_dir_all(temp).unwrap();
}

fn r10_reservation_consequence(c:&Value,a:&Value,b:&Value,raw:&std::collections::BTreeMap<String,String>,bundle:&Value)->Result<(),String>{
 let maximum=b["reservation_origin_refs"].as_array().ok_or("reservation refs")?.iter().map(|o|{let row:Value=serde_json::from_str(&raw[o.as_str().unwrap()]).unwrap();dollars(&row["reserved_usd"]).ok_or("raw reservation amount")}).collect::<Result<Vec<_>,_>>()?.into_iter().max().ok_or("raw reservation maximum")?.to_string();
 if a["exposure"]["high"]!=json!({"kind":"at_least","basis":"proxy_reservation","nano":maximum}) || a["exposure"]["pool"]!="pi_api" {return Err("R10 raw reservation maximum/conservative high".into());}
 if a["settlement"]["reserved"]!=maximum {return Err("R10 raw reservation fixed settlement reserved amount".into());}
 if a["obligation"]!=json!({"amount_source":"reservation_bound","state":"outstanding"}) || a["replacement"]["eligible"]!=false || a["settlement"]["decision"]!="rejected" {return Err("R10 raw reservation outstanding obligation/replacement".into());}
 let cid=c["case_id"].as_str().ok_or("case id")?;let key=a["key"].as_str().ok_or("attempt key")?;let inp=&bundle["documents"][0];let result=&bundle["documents"][1];let origins:Vec<_>=raw.keys().collect();let obligation=format!("validity:{key}");let exposure=format!("case:{cid}#attempt:{key}");let ms=&result["primary_memberships"];let vs=&result["primary_validities"];
 let owners:Vec<_>=result["origins"].as_array().ok_or("owners")?.iter().map(|o|json!([o["origin_ref"],o["active_high_unit_id"]])).collect();let want_owners:Vec<_>=origins.iter().map(|o|json!([o,key])).collect();
 if inp["kind"]!="accounting_input_proof" || inp["id"]!=format!("input:{cid}") || inp["lineage_id"]!=cid || result["kind"]!="accounting_result" || result["id"]!=format!("result:{cid}") || result["lineage_id"]!=cid || result["input_proof_ref"]!=inp["id"] || a["accounting_result_ref"]!=format!("{}#result:{cid}",b["bundle_file"].as_str().unwrap()) || a["evidence"]!=json!(origins) || result["units"]!=json!([{"unit_id":key,"state":"active","origin_refs":origins,"exposure_ref":exposure}]) || ms.as_array().map(Vec::len)!=Some(1) || ms[0]["obligation_id"]!=obligation || ms[0]["settlement_classes"]!=json!([{"origin_refs":["S1/turn-1:3"],"representative_origin_ref":"S1/turn-1:3"}]) || ms[0]["usage_origin_refs"]!=json!(["S1/turn-1:3"]) || vs.as_array().map(Vec::len)!=Some(1) || vs[0]["obligation_id"]!=obligation || vs[0]["validity_result"]!="rejected" || vs[0]["applicable_check_refs"]!=json!([format!("{exposure}/settlement")]) || vs[0]["distinct_settlement_refs"]!=json!(["S1/turn-1:3"]) || result["coverage_decisions"]!=json!([]) || owners!=want_owners {return Err("R10 reservation selected raw/result/unit/validity join".into());}
 for ev in c["evaluations"].as_array().ok_or("evaluations")?{for ob in ev["expected"]["objectives"].as_array().ok_or("objectives")?{for label in ["owned_closed","owned_outstanding"]{let scope=format!("{cid}/{}/{}/{label}",ev["name"].as_str().unwrap(),ob["objective"].as_str().unwrap());if ob[label]["supported_low_evidence_ref"]!=format!("{}#{scope}",b["bundle_file"].as_str().unwrap()){return Err("R10 reservation selected low reference join".into());}}}}
 Ok(())
}


// Independently authored finite facts from raw/pinned policy; see DERIVATION.md.
// This is not a general financial or attribution reducer.
fn r10_selected_low_consequence(c:&Value,b:&Value,bundle:&Value)->Result<(),String>{
 let cid=c["case_id"].as_str().ok_or("selected case id")?;
 let eq=b["control_file"]=="cases/equal-reservations.json";
 let extra=cid=="A04-undated-settlement-between-dated-neighbours";
 let key="res:S1/turn-1:rid=1";let key2="res:S1/turn-1:rid=2";
 let usage=if eq {"S1/turn-1:3"} else {"S1/turn-1:2"};
 let docs=bundle["documents"].as_array().ok_or("selected documents")?;
 if docs.len()!=2 || docs[0]["kind"]!="accounting_input_proof" || docs[1]["kind"]!="accounting_result" {return Err("R10 selected result/input inventory".into());}
 let inp=&docs[0];let result=&docs[1];let path=b["bundle_file"].as_str().ok_or("selected bundle file")?;
 if inp["id"]!=format!("input:{cid}") || result["id"]!=format!("result:{cid}") || result["input_proof_ref"]!=inp["id"] || inp["lineage_id"]!=cid || result["lineage_id"]!=cid || inp.get("independence_frame").is_some(){return Err("R10 selected result/input join".into());}
 let attempts=c["expected"]["attempts"].as_array().ok_or("selected attempts")?;
 let keys=if extra {vec![key,key2]} else {vec![key]};
 if attempts.len()!=keys.len() || attempts.iter().zip(&keys).any(|(a,k)|a["key"]!=*k){return Err("R10 selected attempt inventory".into());}
 let origins=if eq || extra {vec!["S1/turn-1:1","S1/turn-1:2","S1/turn-1:3"]} else {vec!["S1/turn-1:1","S1/turn-1:2"]};
 let mut units=vec![json!({"unit_id":key,"state":"active","origin_refs":if eq {origins.clone()} else {origins[..2].to_vec()},"exposure_ref":format!("case:{cid}#attempt:{key}")})];
 if extra {units.push(json!({"unit_id":key2,"state":"active","origin_refs":["S1/turn-1:3"],"exposure_ref":format!("case:{cid}#attempt:{key2}")}));}
 let owners:Vec<_>=result["origins"].as_array().ok_or("selected origins")?.iter().map(|o|json!([o["origin_ref"],o["active_high_unit_id"]])).collect();
 let expected_owners:Vec<_>=origins.iter().map(|o|json!([o,if extra && *o=="S1/turn-1:3" {key2} else {key}])).collect();
 if result["units"]!=json!(units) || owners!=expected_owners {return Err("R10 selected unit/origin join".into());}
 for (i,a) in attempts.iter().enumerate(){if a["accounting_result_ref"]!=format!("{path}#result:{cid}") || a["evidence"]!=units[i]["origin_refs"]{return Err("R10 selected result/unit reference".into());}}
 if eq && (attempts[0]["valuation"]["state"]!="valued" || attempts[0]["valuation"]["low"]!="15000000" || attempts[0]["valuation"]["high"]!="15000000" || attempts[0]["valuation"]["complete"]!=true || attempts[0]["exposure"]["low"]!=json!({"basis":"usage_valuation","nano":"15000000"})){return Err("R10 selected priceable usage minimum".into());}
 if !eq {
  if attempts[0]["obligation"]!=json!({"state":"closed","amount_source":"reservation_bound","by":usage}) || attempts[0]["settlement"]["decision"]!="valid" {return Err("R10 selected model7 lawful temporal closure".into());}
  if extra && (attempts[1]["obligation"]!=json!({"state":"outstanding","amount_source":"reservation_bound"}) || attempts[1]["exposure"]!=json!({"pool":"pi_api","low":{"basis":"none","nano":"0"},"high":{"kind":"at_least","nano":"300000000","basis":"proxy_reservation"}})){return Err("R10 selected separate reservation300M".into());}
  let mut ms=vec![];let mut vs=vec![];
  let mut roles=vec![(key,vec!["S1/turn-1:1"],vec![usage])];if extra {roles.push((key2,vec!["S1/turn-1:3"],vec![]));}
  for (k,rs,ss) in roles {
   let classes=|os:&Vec<&str>|os.iter().map(|o|json!({"origin_refs":[o],"representative_origin_ref":o})).collect::<Vec<_>>();
   ms.push(json!({"obligation_id":format!("validity:{k}"),"reservation_classes":classes(&rs),"settlement_classes":classes(&ss),"usage_origin_refs":ss}));
   vs.push(json!({"obligation_id":format!("validity:{k}"),"distinct_reservation_refs":rs,"distinct_settlement_refs":ss,"validity_result":if ss.is_empty(){"absent"}else{"valid"},"applicable_check_refs":[format!("case:{cid}#attempt:{k}/settlement")]}));
  }
  if result["primary_memberships"]!=json!(ms) || result["primary_validities"]!=json!(vs) || result["coverage_decisions"]!=json!([]){return Err("R10 selected model7 membership/validity join".into());}
 }
 if result["valuation_records"]!=json!([{"valuation_id":format!("valuation:{key}"),"usage_origin_refs":[usage],"complete":eq,"wire_ref":format!("case:{cid}#attempt:{key}/valuation")}]){return Err("R10 selected valuation/wire join".into());}
 let evs=c["evaluations"].as_array().ok_or("selected evaluations")?;
 if evs.len()!=1 || evs[0]["name"]!="default" {return Err("R10 selected evaluation inventory".into());}
 let ev=&evs[0];let obs=ev["expected"]["objectives"].as_array().ok_or("selected expected objectives")?;
 if ev["objectives"].as_array().map(Vec::len)!=Some(1) || ev["objectives"][0]["objective"]!="o" || obs.len()!=1 || obs[0]["objective"]!="o" {return Err("R10 selected objective inventory".into());}
 let ats:Vec<_>=keys.iter().map(|k|json!({"attempt":k,"objective":"o","ownership":"owned","placement":"inside"})).collect();
 if ev["expected"]["attribution"]!=json!(ats){return Err("R10 selected raw ownership/placement".into());}
 let mut lows=vec![];let mut assertions=vec![];
 for label in ["owned_closed","owned_outstanding"]{
  let positive=eq && label=="owned_outstanding";
  let amount=if positive {"15000000"}else{"0"};
  let has_usage=if eq {label=="owned_outstanding"} else {label=="owned_closed"};
  let inventory=if has_usage {vec![usage]} else {vec![]};
  let scope=format!("{cid}/default/o/{label}");
  lows.push(json!({"scope_ref":scope,"selected_low":amount,"single_floor":amount,"independent_floor":"0","basis":if positive{"usage_valuation"}else{"none"},"credit_mode":if positive{"single_ambiguous"}else{"none"},"credits":[],"seed_origin_refs":inventory,"alternative_origin_refs":inventory}));
  let high=if (eq && label=="owned_outstanding") || (!eq && label=="owned_closed") {json!({"kind":"at_least","nano":"500000000"})} else if extra && label=="owned_outstanding" {json!({"kind":"at_least","nano":"300000000"})} else {json!({"kind":"exact","nano":"0"})};
  if obs[0][label]["supported_low_evidence_ref"]!=format!("{path}#{scope}"){return Err("R10 selected low reference join".into());}
  if obs[0][label]["low"]!=amount || obs[0][label]["high"]!=high {return Err("R10 selected independent low/result/rollup".into());}
  let owned=if has_usage {vec![key]}else if extra && label=="owned_outstanding"{vec![key2]}else{vec![]};
  assertions.push(json!({"evaluation":"default","objective":"o","label":label,"expected_low":amount,"seed_origin_refs":inventory,"alternative_origin_refs":inventory,"owned_unit_refs":owned}));
 }
 if b["low_assertions"]!=json!(assertions){return Err("R10 selected independent assertion inventory".into());}
 // Full-list comparison preserves uniqueness and total inventory before maps exist.
 if result["low_rollups"]!=json!(lows){return Err("R10 selected complete low witness inventory/fields".into());}
 Ok(())
}

// ---------------------------------------------------------------------------------------------
// Slice 2a (r11): independent finite predicates for the oracle-side historical findings.
// Each predicate reads raw lines and supplied outputs; none computes a C-K result.
fn r11_cases() -> Vec<Value> {
    let mut v: Vec<_> = fs::read_dir(root().join("corpus/cases"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    v.sort();
    v.into_iter().map(read).collect()
}
fn r11_lines(c: &Value) -> std::collections::BTreeMap<String, (String, Value)> {
    let mut m = std::collections::BTreeMap::new();
    for s in c["inputs"]["sessions"].as_array().unwrap() {
        for (t, l) in s["logs"].as_object().unwrap() {
            for (i, x) in l["lines"].as_array().unwrap().iter().enumerate() {
                let raw = x.as_str().unwrap_or("").to_string();
                let row = serde_json::from_str(&raw).unwrap_or(Value::Null);
                m.insert(format!("{}/{}:{}", s["alias"].as_str().unwrap(), t, i + 1), (raw, row));
            }
        }
    }
    m
}
fn r11_bundle(cid: &str) -> Value {
    let part = if ["P130-", "P132-", "P133-"].iter().any(|p| cid.starts_with(p)) { "A" } else { "B" };
    read(root().join(format!("oracle/r8-completion-{part}/{cid}.bundle.json")))
}
fn r11_nano(v: &Value) -> Option<i128> {
    v.as_str()?.parse().ok()
}
/// Exact decimal (including exponent forms such as 1e-6) to nano-USD; None for non-exact/negative/non-numbers (MONEY).
fn r11_dollars(v: &Value) -> Option<i128> {
    if !v.is_number() { return None; }
    let s = v.to_string().to_ascii_lowercase();
    let (m, e) = s.split_once('e').map(|(m, e)| (m, e.parse::<i32>().ok())).unwrap_or((s.as_str(), Some(0)));
    let e = e?;
    let (a, b) = m.split_once('.').unwrap_or((m, ""));
    if a.starts_with('-') || !a.chars().chain(b.chars()).all(|c| c.is_ascii_digit()) { return None; }
    let digits: i128 = format!("{a}{b}").parse().ok()?;
    let shift = 9 + e - b.len() as i32; // nano = digits * 10^(9 + e - frac_len)
    if shift >= 0 { digits.checked_mul(10i128.checked_pow(shift as u32)?) }
    else { let d = 10i128.checked_pow((-shift) as u32)?; (digits % d == 0).then_some(digits / d) }
}
/// R8-FIN-01: the retained raw charge (largest well-formed usd.charged_usd among distinct settlement rows) is the
/// supplied settlement.charged and a lower bound of the unit high, whatever the other supplied fields say.
fn r11_raw_charge_violation(c: &Value) -> Option<String> {
    let lines = r11_lines(c);
    for a in c["expected"]["attempts"].as_array().unwrap() {
        let mut seen = BTreeSet::new();
        let mut best: Option<i128> = None;
        for r in a["evidence"].as_array().unwrap() {
            let Some((raw, row)) = lines.get(r.as_str().unwrap()) else { continue };
            let ev = row["event"].as_str().unwrap_or("");
            if !(ev == "request" || ev == "upstream_error") || !row["usd"].is_object() || !seen.insert(raw.clone()) {
                continue;
            }
            if let Some(n) = r11_dollars(&row["usd"]["charged_usd"]) {
                best = Some(best.map_or(n, |b| b.max(n)));
            }
        }
        if let Some(ch) = best {
            if r11_nano(&a["settlement"]["charged"]) != Some(ch) {
                return Some(format!("{} settlement.charged != raw charge {ch}", a["key"]));
            }
            if a["exposure"]["pool"] == "pi_api" && !r11_nano(&a["exposure"]["high"]["nano"]).is_some_and(|h| h >= ch) {
                return Some(format!("{} high below retained raw charge {ch}", a["key"]));
            }
        }
    }
    None
}
#[test]
fn r11_fin_retained_raw_charge() {
    for c in r11_cases() {
        assert_eq!(r11_raw_charge_violation(&c), None, "{}", c["case_id"]);
    }
    let mut c = corpus_case("P9-claimed-charge-above-reservation");
    let a = &mut c["expected"]["attempts"][0];
    a["settlement"]["charged"] = json!("500000000");
    a["exposure"]["high"]["nano"] = json!("500000000");
    for e in c["evaluations"].as_array_mut().unwrap() {
        let o = &mut e["expected"]["objectives"][0];
        o["owned_outstanding"]["high"]["nano"] = json!("500000000");
        o["commitments"][0]["committed"] = json!("500000000");
    }
    let why = r11_raw_charge_violation(&c).expect("coherent 700->500 mutant must be rejected");
    assert!(why.contains("raw charge 700000000"), "{why}");
    // non-pilot: P130-R02's exponent-form rounded-up raw charge 1e-6 USD = 1000 nano is the retained Exact high
    assert_eq!(r11_dollars(&json!(1e-6)), Some(1000));
    let mut c = corpus_case("P130-R02-rounded-up-charge-admitted");
    assert_eq!(r11_raw_charge_violation(&c), None);
    let a = &mut c["expected"]["attempts"][0];
    a["settlement"]["charged"] = json!("525");
    a["exposure"]["high"] = json!({"basis":"usage_valuation","kind":"exact","nano":"525"});
    let why = r11_raw_charge_violation(&c).expect("coherent 1000->525 non-pilot rewrite must be rejected");
    assert!(why.contains("raw charge 1000"), "{why}");
}
#[test]
fn r11_identity_malformed_cannot_be_favourable() {
    let rates = json!({"openai": read(root().join("oracle/rates/openai_api_rates.json")), "anthropic": read(root().join("oracle/rates/anthropic_api_rates.json"))});
    let check = |c: &Value, a: &Value| {
        let lines = r11_lines(c);
        let launch = &c["inputs"]["sessions"][0]["launch"];
        a["evidence"].as_array().unwrap().iter().filter_map(|r| lines.get(r.as_str().unwrap())).filter(|(_, row)| row["event"] == "request")
            .all(|(_, row)| accepts_usage_axis(row, launch, &rates, a))
    };
    for f in ["model7-VC01-openai-usage-settlement", "model7-A04-undated-settlement-between-dated-neighbours"] {
        let c = read(root().join(format!("oracle/r9-pilot/cases/{f}.json")));
        let a = &c["expected"]["attempts"][0];
        assert!(check(&c, a), "repaired conservative positive {f}");
        // consistent wrong: the old coherent complete valued 15M with unchanged raw numeric response_model 7
        let mut w = a.clone();
        w["valuation"] = json!({"state":"valued","identities":["openai/gpt-6-astra"],"low":"15000000","high":"15000000","complete":true});
        w["replacement"] = json!({"eligible":true,"reason":"UsageBound"});
        assert!(!check(&c, &w), "malformed identity made complete/favourable must be rejected ({f})");
        let mut w2 = a.clone();
        w2["valuation"]["supported_low"] = json!("15000000");
        assert!(!check(&c, &w2), "unsupported minimum invented ({f})");
    }
    // other malformed shapes injected into a valid corpus request row, with its unchanged favourable outputs
    let c = corpus_case("P132-FG-VC15-gated");
    for bad in [json!(""), json!(true), json!({}), json!(" gpt-6-astra"), json!(7), Value::Null] {
        let mut m = c.clone();
        let line = m["inputs"]["sessions"][0]["logs"]["turn-1"]["lines"][1].as_str().unwrap().to_string();
        let mut row: Value = serde_json::from_str(&line).unwrap();
        row["response_model"] = bad.clone();
        m["inputs"]["sessions"][0]["logs"]["turn-1"]["lines"][1] = json!(row.to_string());
        let a = m["expected"]["attempts"][0].clone();
        assert!(!check(&m, &a), "malformed response_model {bad} disappeared into a complete set");
    }
    // priced multi-identity remains finite and supported
    let v = corpus_case("V01-usage-settlement-other-response-model");
    assert!(check(&v, &v["expected"]["attempts"][0]));
}
#[test]
fn r11_gate_identity_triple_nonempty() {
    let c = corpus_case("P132-FG-VC15-gated");
    let o = &c["evaluations"][0]["objectives"][0];
    let g = c["inputs"]["gates"][0].clone();
    let w = &c["trusted_fixture_inputs"]["gate_verifications"]["g-S1"];
    assert!(gate_identity_triple(o, &g, w) && mock_binding(&c, &g));
    for val in [None, Some(json!("")), Some(json!(" ")), Some(json!(7))] {
        let mut m = c.clone();
        let set = |x: &mut Value, k: &str| match &val {
            None => { x.as_object_mut().unwrap().remove(k); }
            Some(v) => { x[k] = v.clone(); }
        };
        set(&mut m["evaluations"][0]["objectives"][0]["admission_authority"], "id");
        set(&mut m["inputs"]["gates"][0]["authority"], "id");
        set(&mut m["trusted_fixture_inputs"]["gate_verifications"]["g-S1"], "authority_id");
        let g2 = m["inputs"]["gates"][0].clone();
        m["trusted_fixture_inputs"]["gate_verifications"]["g-S1"]["payload_sha256"] = json!(hash(&serde_json::to_vec(&g2).unwrap()));
        let (o2, w2) = (&m["evaluations"][0]["objectives"][0], &m["trusted_fixture_inputs"]["gate_verifications"]["g-S1"]);
        assert!(!gate_identity_triple(o2, &g2, w2), "triple {val:?} with recomputed payload must not qualify");
        assert!(!mock_binding(&m, &g2), "witness binding with {val:?} identities must not qualify");
    }
    let mut m = c.clone();
    m["trusted_fixture_inputs"]["gate_verifications"]["g-S1"]["authority_id"] = json!("wev-admission-authority-2");
    assert!(!gate_identity_triple(&m["evaluations"][0]["objectives"][0], &g, &m["trusted_fixture_inputs"]["gate_verifications"]["g-S1"]));
}
/// Raw-namespace rule table oracle/r11-controls/raw-namespace-rules.json (review-r2 D-4): D1 usd_reserved+rid, D2 usd.rid;
/// A1 subscription_proxy_admitted+rid, A2 subscription_proxy.rid; both families -> conflict; neither -> none.
fn r11_raw_namespace(row: &Value) -> Option<&'static str> {
    let has = |v: &Value| v.as_object().is_some_and(|o| o.contains_key("rid"));
    let dollar = (row["event"] == "usd_reserved" && has(row)) || has(&row["usd"]);
    let admission = (row["event"] == "subscription_proxy_admitted" && has(row)) || has(&row["subscription_proxy"]);
    match (dollar, admission) {
        (true, true) => Some("conflict"),
        (true, false) => Some("dollar_ticket"),
        (false, true) => Some("admission"),
        _ => None,
    }
}
#[test]
fn r11_raw_namespace_census_agreement() {
    // Cross-classifier agreement control: the Python actual consumer and this independent classifier must both reproduce
    // the committed census over every corpus raw row plus synthetic rule-coverage rows.
    let census = read(root().join("oracle/r11-controls/raw-namespace-census.json"));
    let rows = census["rows"].as_array().unwrap();
    assert!(rows.len() > 300);
    for r in rows {
        let line = r["raw"].as_str().unwrap();
        assert_eq!(hash(line.as_bytes()), r["raw_utf8_sha256"].as_str().unwrap());
        let got = serde_json::from_str::<Value>(line).ok().filter(|v| v.is_object()).and_then(|v| r11_raw_namespace(&v));
        assert_eq!(got.map(Value::from).unwrap_or(Value::Null), r["class"], "{} {}", r["case_id"], r["origin_id"]);
    }
    for want in ["dollar_ticket", "admission", "conflict"] {
        assert!(rows.iter().any(|r| r["class"] == want), "census lacks {want}");
    }
}
/// R8-NAMESPACE-01: an initial contract's namespace must equal the raw reference namespace of each covered origin.
fn r11_namespace_violation(c: &Value, bundle: &Value) -> Option<String> {
    let lines = r11_lines(c);
    let inp = bundle["documents"].as_array()?.iter().find(|d| d["kind"] == "accounting_input_proof")?;
    for o in inp["origins"].as_array()? {
        let Some(ic) = inp["initial_contracts"].as_array()?.iter().find(|x| x["contract_id"] == o["initial_contract_ref"]) else {
            return Some(format!("{} dangling initial_contract_ref", o["origin_id"])); // review-r2 D-5 observation
        };
        let ns = ic["reference_namespace"].as_str()?;
        let raw = lines.get(o["origin_id"].as_str()?).and_then(|(_, r)| r11_raw_namespace(r));
        if matches!(ns, "dollar_ticket" | "admission") && raw.is_some_and(|r| r != ns) {
            return Some(format!("{} contract {ns} on raw {raw:?}", o["origin_id"]));
        }
    }
    None
}
#[test]
fn r11_namespace_application() {
    for c in r11_cases() {
        let cid = c["case_id"].as_str().unwrap();
        assert_eq!(r11_namespace_violation(&c, &r11_bundle(cid)), None, "{cid}");
    }
    for cid in ["VC08-subscription-measured-request", "U06-subscription-upstream-error-unlabelled", "V03-control-labelled-upstream-error", "V03-subscription-upstream-error-labelled-api"] {
        let c = corpus_case(cid);
        let mut b = r11_bundle(cid);
        // consistent wrong: re-declare the withdrawn dollar_ticket contract; bundle, grant and refs all agree
        b["documents"][0]["initial_contracts"][0]["reference_namespace"] = json!("dollar_ticket");
        assert!(r11_namespace_violation(&c, &b).is_some(), "{cid} dollar_ticket on admission origins must fail");
        // OriginBase truth: one active unit per origin (no admitted admission-namespace scope)
        let units: Vec<_> = r11_bundle(cid)["documents"][1]["units"].as_array().unwrap().iter().filter(|u| u["state"] == "active").map(|u| u["origin_refs"].clone()).collect();
        assert_eq!(units, vec![json!(["S1/turn-1:1"]), json!(["S1/turn-1:2"])], "{cid}");
    }
    let ctl = root().join("oracle/r11-controls/admission-namespace");
    let c = read(ctl.join("case.json"));
    let b = read(ctl.join("bundle.json"));
    assert_eq!(r11_namespace_violation(&c, &b), None, "conditional admission-namespace positive control");
}
/// R8-LOW-INVENTORY-01: U holds every owned/candidate PiApi usage origin on that line; Seed only owned usage.
fn r11_low_violation(c: &Value, bundle: &Value) -> Option<String> {
    let cid = c["case_id"].as_str()?;
    let lines = r11_lines(c);
    let result = bundle["documents"].as_array()?.iter().find(|d| d["kind"] == "accounting_result")?;
    let attempts: std::collections::BTreeMap<_, _> = c["expected"]["attempts"].as_array()?.iter().map(|a| (a["key"].as_str().unwrap(), a)).collect();
    for e in c["evaluations"].as_array()? {
        for o in e["objectives"].as_array()? {
            let name = o["objective"].as_str()?;
            for label in ["owned_closed", "owned_outstanding"] {
                let scope = format!("{cid}/{}/{name}/{label}", e["name"].as_str()?);
                let Some(low) = result["low_rollups"].as_array()?.iter().find(|r| r["scope_ref"].as_str().is_some_and(|s| s.ends_with(&scope))) else { continue };
                let (mut u, mut seed) = (BTreeSet::new(), BTreeSet::new());
                for at in e["expected"]["attribution"].as_array()? {
                    let Some(a) = attempts.get(at["attempt"].as_str()?) else { continue };
                    if a["exposure"]["pool"] != "pi_api" || (a["obligation"]["state"] == "outstanding") != (label == "owned_outstanding") { continue }
                    let owned = at["ownership"] == "owned" && at["objective"] == name;
                    let cand = (at["ownership"] == "uncertain" || at["ownership"] == "unadmitted") && at["candidates"].as_array().is_some_and(|x| x.iter().any(|y| y == name));
                    let usage: BTreeSet<String> = a["evidence"].as_array()?.iter().filter_map(|r| r.as_str()).filter(|r| lines.get(*r).is_some_and(|(_, row)| row["usage"].is_object())).map(String::from).collect();
                    if owned { seed.extend(usage.clone()); }
                    if owned || cand { u.extend(usage); }
                }
                let alt: BTreeSet<String> = low["alternative_origin_refs"].as_array()?.iter().filter_map(|x| x.as_str().map(String::from)).collect();
                let got_seed: BTreeSet<String> = low["seed_origin_refs"].as_array()?.iter().filter_map(|x| x.as_str().map(String::from)).collect();
                if !u.is_subset(&alt) { return Some(format!("{scope} U omits {:?}", u.difference(&alt).collect::<Vec<_>>())); }
                if !got_seed.is_subset(&seed) { return Some(format!("{scope} Seed has non-owned usage")); }
            }
        }
    }
    None
}
#[test]
fn r11_low_inventory_keeps_candidates() {
    for c in r11_cases() {
        let cid = c["case_id"].as_str().unwrap();
        assert_eq!(r11_low_violation(&c, &r11_bundle(cid)), None, "{cid}");
    }
    for (cid, scope, origin) in [
        ("P130-R08-admissions-without-tree-still-block", "default/o/owned_closed", "S1/turn-1:2"),
        ("P130-R08-admissions-without-tree-still-block", "default/p/owned_closed", "S1/turn-1:2"),
        ("P130-R12-untrusted-clock-keeps-candidate", "default/o/owned_closed", "S1/turn-1:1"),
        ("P133-A04-cache-counts-both-absent", "default/o/owned_closed", "S1/turn-1:1"),
        ("P133-A04-cache-write-absent-legacy", "default/o/owned_closed", "S1/turn-1:1"),
    ] {
        let c = corpus_case(cid);
        let mut b = r11_bundle(cid);
        let lows = b["documents"][1]["low_rollups"].as_array_mut().unwrap();
        let low = lows.iter_mut().find(|r| r["scope_ref"].as_str().unwrap().ends_with(scope)).unwrap();
        assert_eq!(low["alternative_origin_refs"], json!([origin]));
        assert_eq!((low["seed_origin_refs"].clone(), low["selected_low"].clone()), (json!([]), json!("0")), "candidate adds no seed/credit");
        low["alternative_origin_refs"] = json!([]); // the historical jointly-omitted U (zero low unchanged)
        assert!(r11_low_violation(&c, &b).is_some(), "{cid} {scope}");
        let low = b["documents"][1]["low_rollups"].as_array_mut().unwrap().iter_mut().find(|r| r["scope_ref"].as_str().unwrap().ends_with(scope)).unwrap();
        low["alternative_origin_refs"] = json!([origin]);
        low["seed_origin_refs"] = json!([origin]); // candidate promoted to owned seed
        assert!(r11_low_violation(&c, &b).is_some(), "{cid} {scope} candidate seed");
    }
}
/// R8-PLACEMENT-01 for fully dated owned attempts on a trusted clock (others are outside this finite predicate).
fn r11_placement_violation(c: &Value) -> Option<String> {
    let lines = r11_lines(c);
    let reference = c["inputs"]["reference_time"].as_str()?;
    for e in c["evaluations"].as_array()? {
        for at in e["expected"]["attribution"].as_array()? {
            if at["ownership"] != "owned" { continue }
            let a = c["expected"]["attempts"].as_array()?.iter().find(|a| a["key"] == at["attempt"])?;
            let alias = a["key"].as_str()?.split(':').nth(1)?.split('/').next()?;
            let sess = c["inputs"]["sessions"].as_array()?.iter().find(|s| s["alias"] == alias)?;
            let stamps: Vec<&str> = a["evidence"].as_array()?.iter().filter_map(|r| lines.get(r.as_str()?)).map(|(_, row)| row["at"].as_str().unwrap_or("")).collect();
            let all_rows: Vec<&str> = sess["logs"].as_object()?.values().flat_map(|l| l["lines"].as_array().unwrap().iter()).filter_map(|x| serde_json::from_str::<Value>(x.as_str()?).ok()).filter_map(|r| r["at"].as_str().map(String::from)).map(|s| Box::leak(s.into_boxed_str()) as &str).collect();
            let life: Vec<&str> = sess["lifecycle"].as_array()?.iter().filter_map(|r| r["at"].as_str()).collect();
            let first_retire = sess["lifecycle"].as_array()?.iter().filter(|r| r["event"] == "retire").filter_map(|r| r["at"].as_str()).min();
            // L-1..L-3/L-6: an untrusted or contradicted clock is outside this finite predicate (P-2/P-3 unplaceable route).
            if stamps.iter().any(|s| s.len() != 20) || all_rows.iter().any(|s| *s > reference) || all_rows.windows(2).any(|w| w[0] > w[1])
                || life.windows(2).any(|w| w[0] > w[1]) || first_retire.is_some_and(|r| all_rows.iter().any(|s| *s > r))
                || !sess["lifecycle"].as_array()?.iter().any(|r| r["event"] == "launch" && r["at"].as_str().is_some_and(|l| all_rows.iter().all(|s| *s >= l)))
                || sess["lifecycle"].as_array()?.iter().any(|r| r["at"].is_null() || r["event"] == "deliver") { continue }
            let lo = *stamps.iter().min()?;
            let mut hi = Some(*stamps.iter().max()?);
            if a["obligation"]["state"] == "outstanding" {
                hi = sess["lifecycle"].as_array()?.iter().filter(|r| r["event"] == "retire").filter_map(|r| r["at"].as_str()).filter(|s| *s >= lo).min();
            }
            let o = e["objectives"].as_array()?.iter().find(|o| o["objective"] == at["objective"])?;
            let ca = o["closed_at"].as_str();
            let closed = match (c["profile"].as_str()?, ca) {
                ("current_producer", Some(ca)) => (o["close"]["at"].as_str() == Some(ca) && o["closure_record"].is_null() && ca >= o["started_at"].as_str()? && ca <= reference
                    && !o["admissions"].as_array()?.iter().flat_map(|x| x["rows"].as_array().unwrap()).any(|r| r["at"].as_str().is_some_and(|s| s > ca))).then_some(ca),
                _ => None, // future-profile closure records are outside this finite predicate (treated as claimed)
            };
            if c["profile"] != "current_producer" && o["closure_record"].is_object() { continue }
            let start = o["started_at"].as_str()?;
            let want = if hi.is_some_and(|h| h < start) { "before" }
                else if closed.is_some_and(|c| lo > c) { "after" }
                else if lo < start { "straddles_start" }
                else if closed.is_some_and(|c| hi.is_none_or(|h| h > c)) { "straddles_closure" }
                else { "inside" };
            if at["placement"] != want { return Some(format!("{} {} placement {} != raw {want}", e["name"], a["key"], at["placement"])); }
        }
    }
    None
}
#[test]
fn r11_placement_from_raw_interval() {
    for c in r11_cases() {
        assert_eq!(r11_placement_violation(&c), None, "{}", c["case_id"]);
    }
    let mut c = corpus_case("P132-CP-closed-predecessor-outstanding");
    for e in c["evaluations"].as_array_mut().unwrap() {
        e["expected"]["attribution"][0]["placement"] = json!("inside");
        let o = &mut e["expected"]["objectives"][0];
        o["reasons"] = json!(["UnenforcedCommitment", "UnresolvedExposure"]); // matching wrong reasons
    }
    assert!(r11_placement_violation(&c).unwrap().contains("raw straddles_closure"));
    let t = corpus_case("P130-T-owned-before-window-summed");
    assert_eq!(r11_placement_violation(&t), None);
    assert!(t["evaluations"].as_array().unwrap().iter().any(|e| e["expected"]["attribution"].as_array().unwrap().iter().any(|a| a["placement"] == "before")));
}
