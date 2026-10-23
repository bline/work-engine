//! E2-AC2, E2-AC3, E2-AC4: the derivation core, driven through typed records (no files).

mod support;
use proposal_evaluation::compare::{LoadedEstimateSet, derive};
use proposal_evaluation::model::*;
use serde_json::{Value, json};
use support::*;

/// Minimum case counts, stated here and not read from any fixture (EXT-005).
const MIN_CONDITION_CELLS: usize = 24;
const MIN_RELATION_ROWS: usize = 24;
const MIN_GENERATED_CONTRACTS: usize = 50;

const TOOL_SHA: &str = HEX_A;

fn tool() -> ToolRecord {
    ToolRecord {
        name: "work-engine-evaluation".into(),
        version: "0.1.0".into(),
        binary_sha256: TOOL_SHA.into(),
    }
}

/// The contract measure's comparability key as JSON (what an estimate's `measure` must equal).
fn measure_key(contract: &Value, index: usize) -> Value {
    let mut m = contract["measures"][index].clone();
    let obj = m.as_object_mut().unwrap();
    obj.remove("measure_id");
    obj.remove("minimum_confidence");
    m
}

fn estimated(
    contract: &Value,
    index: usize,
    value: Value,
    interval: Value,
    confidence: &str,
) -> Value {
    json!({
        "status": "estimated",
        "measure_id": contract["measures"][index]["measure_id"],
        "measure": measure_key(contract, index),
        "evidence_cutoff": null, "baseline": null, "basis": [], "limitations": [],
        "value": value, "interval": interval, "confidence": confidence
    })
}

fn with(mut entry: Value, key: &str, v: Value) -> Value {
    entry[key] = v;
    entry
}

fn set(pid: &str, packet_sha: &str, estimates: Vec<Value>) -> Value {
    let mut s = valid_set("/x/packet.json", packet_sha, "/x/projection.json", HEX_B);
    s["proposal"]["proposal_id"] = json!(pid);
    s["evidence_cutoff"] = json!("2026-10-22T09:00:00Z");
    s["estimates"] = json!(estimates);
    s
}

fn load(v: &Value) -> LoadedEstimateSet {
    LoadedEstimateSet {
        path: "/x/set.json".into(),
        sha256: HEX_B.into(),
        set: serde_json::from_value(v.clone()).expect("typed set"),
    }
}

fn run(contract: &Value, sets: &[Value]) -> ComparisonFinding {
    let c: ComparisonContract = serde_json::from_value(contract.clone()).expect("typed contract");
    let loaded: Vec<LoadedEstimateSet> = sets.iter().map(load).collect();
    derive(
        &c,
        "/x/contract.json",
        HEX_A,
        &loaded,
        "2026-10-23T00:00:00Z",
        tool(),
    )
    .expect("binds")
}

fn q(n: i64) -> Value {
    json!({"quantity": n})
}
fn lvl(l: &str) -> Value {
    json!({"level": l})
}

/// The standard two-proposal contract and healthy entries: `alpha.one` is better on `risk`
/// and equal on `cost` unless a test changes something.
fn healthy(contract: &Value, cost: i64, risk: &str) -> Vec<Value> {
    vec![
        estimated(contract, 0, q(cost), Value::Null, "high"),
        estimated(contract, 1, lvl(risk), Value::Null, "high"),
    ]
}

fn bound(contract: &Value, a: Vec<Value>, b: Vec<Value>) -> ComparisonFinding {
    run(
        contract,
        &[set("alpha.one", HEX_A, a), set("alpha.two", HEX_B, b)],
    )
}

#[test]
fn every_condition_by_treatment_cell() {
    // (condition, treatment member, expected reason, mutation of the cost entry or None to remove it)
    type Mutation = Box<dyn Fn(&Value) -> Option<Value>>;
    let rows: Vec<(&str, &str, Reason, Mutation)> = vec![
        (
            "missing: no entry",
            "missing",
            Reason::Missing,
            Box::new(|_| None),
        ),
        (
            "missing: not_estimated",
            "missing",
            Reason::Missing,
            Box::new(|e| {
                let mut n = e.clone();
                n["status"] = json!("not_estimated");
                for k in ["value", "interval", "confidence"] {
                    n.as_object_mut().unwrap().remove(k);
                }
                n["reason"] = json!("none");
                Some(n)
            }),
        ),
        (
            "contested: status",
            "contested",
            Reason::Contested,
            Box::new(|e| {
                let mut n = e.clone();
                n["status"] = json!("contested");
                for k in ["value", "interval", "confidence"] {
                    n.as_object_mut().unwrap().remove(k);
                }
                n["competing"] = json!([]);
                Some(n)
            }),
        ),
        (
            "contested: below minimum confidence",
            "contested",
            Reason::BelowMinimumConfidence,
            Box::new(|e| Some(with(e.clone(), "confidence", json!("low")))),
        ),
        (
            "stale: per-estimate cutoff earlier than stale_before",
            "stale",
            Reason::Stale,
            Box::new(|e| {
                Some(with(
                    e.clone(),
                    "evidence_cutoff",
                    json!("2026-09-01T00:00:00Z"),
                ))
            }),
        ),
    ];
    let mut cells = 0;
    for (label, member, reason, mutate) in &rows {
        for treatment in ["block", "drop_measure"] {
            for mutate_a in [true, false] {
                let mut contract = valid_contract();
                contract["treatment"][*member] = json!(treatment);
                let mut a = healthy(&contract, 5, "low");
                let mut b = healthy(&contract, 5, "high");
                let side = if mutate_a { &mut a } else { &mut b };
                match mutate(&side[0]) {
                    Some(e) => side[0] = e,
                    None => {
                        side.remove(0);
                    }
                }
                let f = bound(&contract, a, b);
                let p = &f.pairs[0];
                if treatment == "block" {
                    assert_eq!(p.outcome, Outcome::Blocked, "{label}");
                    assert_eq!(p.blocked_reasons, vec![*reason], "{label}");
                    assert_eq!(p.measures[0].relation, Relation::Blocked, "{label}");
                    assert_eq!(p.measures[0].reasons, vec![*reason], "{label}");
                    assert!(p.dropped_measures.is_empty(), "{label}");
                } else {
                    assert_eq!(p.dropped_measures.len(), 1, "{label}");
                    assert_eq!(p.dropped_measures[0].measure_id, "cost", "{label}");
                    assert_eq!(p.dropped_measures[0].reasons, vec![*reason], "{label}");
                    // The remaining measure decides: alpha.one is lower (better) on risk.
                    assert_eq!(p.measures.len(), 1, "{label}");
                    assert_eq!(p.outcome, Outcome::ADominatesB, "{label}");
                    assert!(p.blocked_reasons.is_empty(), "{label}");
                }
                cells += 1;
            }
        }
    }
    // definition_mismatch is never droppable: both treatments block.
    for treatment in ["block", "drop_measure"] {
        for mutate_a in [true, false] {
            let mut contract = valid_contract();
            for m in ["missing", "contested", "stale"] {
                contract["treatment"][m] = json!(treatment);
            }
            let mut a = healthy(&contract, 5, "low");
            let mut b = healthy(&contract, 5, "high");
            let side = if mutate_a { &mut a } else { &mut b };
            side[0]["measure"]["scope"] = json!("a different scope");
            let p = &bound(&contract, a, b).pairs[0].clone();
            assert_eq!(p.outcome, Outcome::Blocked);
            assert_eq!(p.blocked_reasons, vec![Reason::DefinitionMismatch]);
            cells += 1;
        }
    }
    assert!(cells >= MIN_CONDITION_CELLS, "{cells} cells");
}

#[test]
fn a_block_reason_and_a_drop_reason_on_one_measure_block_with_both() {
    let mut contract = valid_contract();
    contract["treatment"]["stale"] = json!("drop_measure");
    let mut a = healthy(&contract, 5, "low");
    a[0]["evidence_cutoff"] = json!("2026-09-01T00:00:00Z");
    let mut b = healthy(&contract, 5, "high");
    b[0]["confidence"] = json!("low");
    let p = bound(&contract, a, b).pairs[0].clone();
    assert_eq!(p.outcome, Outcome::Blocked);
    assert_eq!(
        p.blocked_reasons,
        vec![Reason::BelowMinimumConfidence, Reason::Stale]
    );
}

#[test]
fn a_set_cutoff_alone_does_not_make_an_entry_stale_when_the_entry_cutoff_is_recent() {
    let contract = valid_contract();
    let mut a = healthy(&contract, 5, "low");
    a[0]["evidence_cutoff"] = json!("2026-10-10T00:00:00Z");
    let mut s = set("alpha.one", HEX_A, a);
    s["evidence_cutoff"] = json!("2026-09-01T00:00:00Z"); // older than stale_before
    let other = set("alpha.two", HEX_B, healthy(&contract, 5, "high"));
    let f = run(&contract, &[s, other]);
    // cost uses its own recent cutoff; risk inherits the old set cutoff and is stale -> blocked.
    let p = &f.pairs[0];
    assert_eq!(p.measures[0].relation, Relation::Equal);
    assert_eq!(p.measures[1].relation, Relation::Blocked);
    assert_eq!(p.measures[1].reasons, vec![Reason::Stale]);
}

struct Row {
    mode: &'static str,
    direction: &'static str,
    ordinal: bool,
    a: (&'static str, Option<(&'static str, &'static str)>),
    b: (&'static str, Option<(&'static str, &'static str)>),
    expect: Relation,
}

fn r(
    mode: &'static str,
    direction: &'static str,
    ordinal: bool,
    a: &'static str,
    b: &'static str,
    expect: Relation,
) -> Row {
    Row {
        mode,
        direction,
        ordinal,
        a: (a, None),
        b: (b, None),
        expect,
    }
}

fn ri(
    direction: &'static str,
    a: (&'static str, (&'static str, &'static str)),
    b: (&'static str, (&'static str, &'static str)),
    expect: Relation,
) -> Row {
    Row {
        mode: "interval_robust",
        direction,
        ordinal: false,
        a: (a.0, Some(a.1)),
        b: (b.0, Some(b.1)),
        expect,
    }
}

#[test]
fn relations_by_mode_direction_and_scale() {
    use Relation::*;
    let rows = vec![
        r("point", "higher_is_better", false, "5", "3", Better),
        r("point", "higher_is_better", false, "3", "5", Worse),
        r("point", "higher_is_better", false, "4", "4", Equal),
        r("point", "lower_is_better", false, "3", "5", Better),
        r("point", "lower_is_better", false, "5", "3", Worse),
        r("point", "lower_is_better", false, "4", "4", Equal),
        r("point", "higher_is_better", true, "high", "low", Better),
        r("point", "higher_is_better", true, "low", "medium", Worse),
        r("point", "higher_is_better", true, "medium", "medium", Equal),
        r("point", "lower_is_better", true, "low", "high", Better),
        r("point", "lower_is_better", true, "high", "medium", Worse),
        r("point", "lower_is_better", true, "low", "low", Equal),
        r(
            "interval_robust",
            "higher_is_better",
            false,
            "5",
            "3",
            Better,
        ),
        r(
            "interval_robust",
            "higher_is_better",
            true,
            "low",
            "high",
            Worse,
        ),
        r(
            "interval_robust",
            "lower_is_better",
            true,
            "low",
            "low",
            Equal,
        ),
        ri(
            "higher_is_better",
            ("8", ("6", "9")),
            ("3", ("1", "4")),
            Better,
        ),
        ri(
            "higher_is_better",
            ("3", ("1", "4")),
            ("8", ("6", "9")),
            Worse,
        ),
        ri(
            "higher_is_better",
            ("5", ("3", "7")),
            ("6", ("5", "8")),
            Overlap,
        ),
        ri(
            "higher_is_better",
            ("5", ("5", "5")),
            ("5", ("5", "5")),
            Equal,
        ),
        ri(
            "higher_is_better",
            ("5", ("2", "6")),
            ("5", ("2", "6")),
            Overlap,
        ),
        ri(
            "higher_is_better",
            ("6", ("5", "6")),
            ("4", ("2", "5")),
            Overlap,
        ),
        ri(
            "lower_is_better",
            ("3", ("1", "4")),
            ("8", ("6", "9")),
            Better,
        ),
        ri(
            "lower_is_better",
            ("8", ("6", "9")),
            ("3", ("1", "4")),
            Worse,
        ),
        ri(
            "lower_is_better",
            ("5", ("3", "7")),
            ("6", ("5", "8")),
            Overlap,
        ),
        ri(
            "lower_is_better",
            ("5", ("5", "5")),
            ("5", ("5", "5")),
            Equal,
        ),
    ];
    assert!(rows.len() >= MIN_RELATION_ROWS, "{} rows", rows.len());
    for row in &rows {
        let mut contract = valid_contract();
        contract["uncertainty"] = json!(row.mode);
        let index = usize::from(row.ordinal);
        let mut measure = contract["measures"][index].clone();
        measure["direction"] = json!(row.direction);
        contract["measures"] = json!([measure]);
        let entry = |side: &(&str, Option<(&str, &str)>)| {
            let v = |t: &str| {
                if row.ordinal {
                    lvl(t)
                } else {
                    q(t.parse().unwrap())
                }
            };
            let interval = side
                .1
                .map_or(Value::Null, |(lo, hi)| json!({"low": v(lo), "high": v(hi)}));
            // The entry builder indexes the contract's measures; there is exactly one now.
            estimated(&contract, 0, v(side.0), interval, "high")
        };
        let f = bound(&contract, vec![entry(&row.a)], vec![entry(&row.b)]);
        let rel = f.pairs[0].measures[0].relation;
        assert_eq!(
            rel, row.expect,
            "{} {} ordinal={} {:?} vs {:?}",
            row.mode, row.direction, row.ordinal, row.a, row.b
        );
    }
}

#[test]
fn equal_on_every_measure_is_incomparable_with_all_relations_equal() {
    let contract = valid_contract();
    let f = bound(
        &contract,
        healthy(&contract, 5, "medium"),
        healthy(&contract, 5, "medium"),
    );
    let p = &f.pairs[0];
    assert_eq!(p.outcome, Outcome::Incomparable);
    assert!(p.measures.iter().all(|m| m.relation == Relation::Equal));
    assert_eq!(p.measures.len(), 2);
    assert_eq!(f.frontier.non_dominated, vec!["alpha.one", "alpha.two"]);
}

#[test]
fn all_measures_dropped_is_blocked_with_no_comparable_measures() {
    let mut contract = valid_contract();
    contract["treatment"]["missing"] = json!("drop_measure");
    let f = bound(&contract, vec![], healthy(&contract, 5, "low"));
    let p = &f.pairs[0];
    assert_eq!(p.outcome, Outcome::Blocked);
    assert_eq!(p.blocked_reasons, vec![Reason::NoComparableMeasures]);
    assert_eq!(p.dropped_measures.len(), 2);
    assert_eq!(f.frontier.undetermined, vec!["alpha.one", "alpha.two"]);
}

/// Exhaustive matches: adding a variant breaks the build here before it can reach a consumer.
fn outcome_names() -> Vec<String> {
    use Outcome::*;
    [ADominatesB, BDominatesA, Incomparable, Blocked]
        .iter()
        .map(|o| {
            match o {
                ADominatesB | BDominatesA | Incomparable | Blocked => {}
            }
            serde_json::to_value(o)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

#[test]
fn closed_enum_variant_lists_are_exact() {
    assert_eq!(
        outcome_names(),
        ["a_dominates_b", "b_dominates_a", "incomparable", "blocked"]
    );
    let relations: Vec<String> = [
        Relation::Better,
        Relation::Worse,
        Relation::Equal,
        Relation::Overlap,
        Relation::Blocked,
    ]
    .iter()
    .map(|r| {
        match r {
            Relation::Better
            | Relation::Worse
            | Relation::Equal
            | Relation::Overlap
            | Relation::Blocked => {}
        }
        serde_json::to_value(r)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    })
    .collect();
    assert_eq!(
        relations,
        ["better", "worse", "equal", "overlap", "blocked"]
    );
    let reasons: Vec<String> = [
        Reason::Missing,
        Reason::Contested,
        Reason::BelowMinimumConfidence,
        Reason::Stale,
        Reason::DefinitionMismatch,
        Reason::NoComparableMeasures,
    ]
    .iter()
    .map(|r| {
        match r {
            Reason::Missing
            | Reason::Contested
            | Reason::BelowMinimumConfidence
            | Reason::Stale
            | Reason::DefinitionMismatch
            | Reason::NoComparableMeasures => {}
        }
        serde_json::to_value(r)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    })
    .collect();
    assert_eq!(
        reasons,
        [
            "missing",
            "contested",
            "below_minimum_confidence",
            "stale",
            "definition_mismatch",
            "no_comparable_measures"
        ]
    );
    // Deserialising refuses any name outside the list, including any that names equality.
    for bad in ["equal", "tie", "dominates", "equivalent", "a_equals_b"] {
        assert!(
            serde_json::from_value::<Outcome>(json!(bad)).is_err(),
            "{bad}"
        );
    }
    for bad in ["unknown", "ties"] {
        assert!(
            serde_json::from_value::<Relation>(json!(bad)).is_err(),
            "{bad}"
        );
    }
}

/// Deterministic generator (a 64-bit LCG); no dependency.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self, bound: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % bound
    }
}

fn pick<'a>(g: &mut Lcg, items: &[&'a str]) -> &'a str {
    items[g.next(items.len() as u64) as usize]
}

/// A random contract and, per proposal, a random estimate set; ids are `p.aa`, `p.ab`, ...
fn generated(g: &mut Lcg, rename: &dyn Fn(usize, usize) -> usize) -> (Value, Vec<Value>) {
    let n = 2 + g.next(4) as usize;
    let mut contract = valid_contract();
    let proposals: Vec<Value> = (0..n)
        .map(|i| json!({"proposal_id": format!("p.a{}", (b'a' + rename(i, n) as u8) as char), "packet_sha256": HEX_A}))
        .collect();
    contract["proposals"] = json!(proposals);
    let count = 1 + g.next(3) as usize;
    let mut measures = Vec::new();
    for k in 0..count {
        let mut m = if g.next(2) == 0 {
            valid_contract()["measures"][0].clone()
        } else {
            valid_contract()["measures"][1].clone()
        };
        m["measure_id"] = json!(format!("m{k}"));
        m["direction"] = json!(pick(g, &["higher_is_better", "lower_is_better"]));
        m["minimum_confidence"] = json!(pick(g, &["low", "medium", "high"]));
        measures.push(m);
    }
    contract["measures"] = json!(measures);
    contract["uncertainty"] = json!(pick(g, &["point", "interval_robust"]));
    for t in ["missing", "contested", "stale"] {
        contract["treatment"][t] = json!(pick(g, &["block", "drop_measure"]));
    }
    let mut sets = Vec::new();
    for i in 0..n {
        let mut estimates = Vec::new();
        for k in 0..count {
            if g.next(8) == 0 {
                continue; // missing
            }
            let ordinal = contract["measures"][k]["scale"]["kind"] == "ordinal";
            let value = |g: &mut Lcg, lo: u64| -> (u64, Value) {
                let x = lo + g.next(4);
                (
                    x,
                    if ordinal {
                        lvl(["low", "medium", "high"][(x % 3) as usize])
                    } else {
                        q(x as i64)
                    },
                )
            };
            let (x, v) = value(g, 0);
            let interval = if g.next(2) == 0 {
                Value::Null
            } else {
                let (lo, hi) = (value(g, 0), value(g, 0));
                let (lo, hi) = if ordinal {
                    let (a, b) = ((lo.0 % 3), (hi.0 % 3));
                    (
                        lvl(["low", "medium", "high"][a.min(b) as usize]),
                        lvl(["low", "medium", "high"][a.max(b) as usize]),
                    )
                } else {
                    (
                        q(lo.0.min(hi.0).min(x) as i64),
                        q(lo.0.max(hi.0).max(x) as i64),
                    )
                };
                json!({"low": lo, "high": hi})
            };
            let mut e = estimated(
                &contract,
                k,
                v,
                interval,
                pick(g, &["low", "medium", "high"]),
            );
            if g.next(6) == 0 {
                e["evidence_cutoff"] = json!("2026-09-01T00:00:00Z");
            }
            if g.next(10) == 0 {
                e["status"] = json!("contested");
                for key in ["value", "interval", "confidence"] {
                    e.as_object_mut().unwrap().remove(key);
                }
                e["competing"] = json!([]);
            }
            estimates.push(e);
        }
        sets.push(set(
            &format!("p.a{}", (b'a' + rename(i, n) as u8) as char),
            HEX_A,
            estimates,
        ));
    }
    (contract, sets)
}

fn mirror(o: Outcome) -> Outcome {
    match o {
        Outcome::ADominatesB => Outcome::BDominatesA,
        Outcome::BDominatesA => Outcome::ADominatesB,
        other => other,
    }
}

fn mirror_relation(r: Relation) -> Relation {
    match r {
        Relation::Better => Relation::Worse,
        Relation::Worse => Relation::Better,
        other => other,
    }
}

#[test]
fn dominance_is_antisymmetric_and_the_frontier_partitions_over_generated_contracts() {
    let mut decided = 0;
    for seed in 0..(MIN_GENERATED_CONTRACTS as u64 + 10) {
        // Same random draws twice; the second run reverses the proposal id order.
        let (c1, s1) = generated(&mut Lcg(seed), &|i, _| i);
        let (c2, s2) = generated(&mut Lcg(seed), &|i, n| n - 1 - i);
        let f1 = run(&c1, &s1);
        let f2 = run(&c2, &s2);
        let n = c1["proposals"].as_array().unwrap().len();
        // Partition of the contract's proposals, each array sorted.
        let mut all: Vec<String> = [
            &f1.frontier.non_dominated,
            &f1.frontier.dominated,
            &f1.frontier.undetermined,
        ]
        .iter()
        .flat_map(|a| a.iter().cloned())
        .collect();
        for a in [
            &f1.frontier.non_dominated,
            &f1.frontier.dominated,
            &f1.frontier.undetermined,
        ] {
            assert!(
                a.windows(2).all(|w| w[0] < w[1]),
                "seed {seed}: unsorted {a:?}"
            );
        }
        all.sort();
        let mut want: Vec<String> = c1["proposals"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["proposal_id"].as_str().unwrap().to_string())
            .collect();
        want.sort();
        assert_eq!(all, want, "seed {seed}: not a partition");
        assert_eq!(f1.pairs.len(), n * (n - 1) / 2, "seed {seed}");
        // No proposal both dominates and is dominated by the same one: a pair has one outcome.
        // Antisymmetry: with ids reversed, every pair's outcome and relations mirror.
        let rename = |id: &str| {
            format!(
                "p.a{}",
                (b'a' + (n as u8 - 1 - (id.as_bytes()[3] - b'a'))) as char
            )
        };
        for p in &f1.pairs {
            let q = f2
                .pairs
                .iter()
                .find(|q| q.a == rename(&p.b) && q.b == rename(&p.a))
                .unwrap_or_else(|| panic!("seed {seed}: mirror pair missing"));
            assert_eq!(q.outcome, mirror(p.outcome), "seed {seed} {}-{}", p.a, p.b);
            let rels: Vec<_> = p
                .measures
                .iter()
                .map(|m| (m.measure_id.clone(), mirror_relation(m.relation)))
                .collect();
            let qrels: Vec<_> = q
                .measures
                .iter()
                .map(|m| (m.measure_id.clone(), m.relation))
                .collect();
            assert_eq!(rels, qrels, "seed {seed}");
            if matches!(p.outcome, Outcome::ADominatesB | Outcome::BDominatesA) {
                decided += 1;
            }
        }
        // A dominated proposal is never non_dominated.
        for d in &f1.frontier.dominated {
            assert!(!f1.frontier.non_dominated.contains(d));
        }
    }
    assert!(decided > 0, "the generator never produced a dominance");
}
