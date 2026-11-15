//! Independent raw-axis qualification. No canonical crate/helper imports.
//! This does not calculate C-K reducer outputs or establish producer reachability.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
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
        terminal(row, f) && counts_complete,
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
                    axes.push(json!({"axis":"reservation_maximum_and_uniqueness","standing":"established","group":g["raw_group"],"R":g["maximum_known_reservation_nano"],"comparison_applicable":nums.len()==1}));
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
fn mock_binding(c: &Value, g: &Value) -> bool {
    let Some(gid) = g["gate_id"].as_str() else {
        return false;
    };
    let w = &c["trusted_fixture_inputs"]["gate_verifications"][gid];
    w["source"] == "test_mock"
        && w["payload_sha256"].as_str() == Some(hash(&serde_json::to_vec(g).unwrap()).as_str())
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
fn accepts_usage_axis(row: &Value, launch: &Value, rates: &Value, a: &Value) -> bool {
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
