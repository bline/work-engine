//! Derivation of the comparison finding (contract 3.2, 3.3).
//!
//! `derive` is a pure function of the contract, the loaded estimate sets, the supplied time and
//! the tool record: it reads no clock and no file. Per unordered pair `a < b` and per contract
//! measure it decides side conditions, then a relation, then the pair outcome and the frontier.
//! It computes no score and no weight; a pair that cannot be decided is `blocked` or
//! `incomparable`, never defaulted.

use crate::diag::{ErrorItem, codes};
use crate::model::*;
use std::collections::BTreeSet;

pub const LIM_CONDITIONAL: &str = "conditional_on_contract:";
pub const LIM_STALE: &str = "stale_is_cutoff_age:";
pub const LIM_POINT_AS_INTERVAL: &str = "point_as_degenerate_interval:";
pub const LIM_DROPPED: &str = "dropped_measures";
pub const LIM_SELECTED: &str = "estimate_sets_selected_at_derivation:";

/// An estimate set read from `path`, whose bytes hash to `sha256`.
#[derive(Debug, Clone)]
pub struct LoadedEstimateSet {
    pub path: String,
    pub sha256: String,
    pub set: EstimateSet,
}

/// A binding refusal (exit 3): the inputs do not describe one comparable evidence base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingError {
    pub errors: Vec<ErrorItem>,
}

impl std::fmt::Display for BindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "binding refused: ")?;
        for (i, e) in self.errors.iter().enumerate() {
            if i > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{} at {}", e.code, e.at)?;
        }
        Ok(())
    }
}

impl std::error::Error for BindingError {}

/// Bind sets to the contract's proposals: one set each, same revision, not after the cutoff.
fn bind<'a>(
    c: &'a ComparisonContract,
    sets: &'a [LoadedEstimateSet],
) -> Result<Vec<(&'a ContractProposal, &'a LoadedEstimateSet)>, BindingError> {
    let mut errors = Vec::new();
    let known: BTreeSet<&str> = c.proposals.iter().map(|p| p.proposal_id.as_str()).collect();
    for (i, s) in sets.iter().enumerate() {
        let id = s.set.proposal.proposal_id.as_str();
        let at = format!("estimates.{i}");
        if !known.contains(id) {
            errors.push(ErrorItem::new(
                codes::ESTIMATE_SET_UNEXPECTED,
                at,
                format!("proposal {id:?} is not in the contract"),
            ));
        } else if sets[..i].iter().any(|o| o.set.proposal.proposal_id == id) {
            errors.push(ErrorItem::new(
                codes::ESTIMATE_SET_DUPLICATE,
                at,
                format!("a second estimate set for proposal {id:?}"),
            ));
        }
    }
    let mut bound = Vec::new();
    for (pi, p) in c.proposals.iter().enumerate() {
        let Some(s) = sets
            .iter()
            .find(|s| s.set.proposal.proposal_id == p.proposal_id)
        else {
            errors.push(ErrorItem::new(
                codes::ESTIMATE_SET_MISSING,
                format!("proposals.{pi}"),
                format!("no estimate set for proposal {:?}", p.proposal_id),
            ));
            continue;
        };
        let at = format!(
            "estimates.{}",
            sets.iter().position(|o| std::ptr::eq(o, s)).unwrap_or(0)
        );
        if s.set.proposal.packet.sha256 != p.packet_sha256 {
            errors.push(ErrorItem::new(
                codes::BASIS_REVISION_MISMATCH,
                at.clone(),
                "estimate set is bound to a different packet revision than the contract names",
            ));
        }
        if s.set.evidence_cutoff > c.evidence_cutoff {
            errors.push(ErrorItem::new(
                codes::ESTIMATE_AFTER_CUTOFF,
                at,
                "estimate set evidence_cutoff is after the contract's",
            ));
        }
        bound.push((p, s));
    }
    if errors.is_empty() {
        Ok(bound)
    } else {
        Err(BindingError { errors })
    }
}

/// A side's standing on one measure: the reasons it cannot be compared, split by governing
/// treatment; `value` is set only when nothing prevents comparing it.
struct Side {
    block: Vec<Reason>,
    drop: Vec<Reason>,
    range: Option<(i128, i128)>,
}

/// Position of `v` on `scale` as a comparable integer; `None` when it is not on the scale.
fn key(scale: &Scale, v: &Value) -> Option<i128> {
    match (scale, v) {
        (Scale::Quantity { .. }, Value::Quantity(n)) => Some(i128::from(*n)),
        (Scale::Ordinal { levels }, Value::Level(l)) => {
            levels.iter().position(|x| x == l).map(|i| i as i128)
        }
        _ => None,
    }
}

fn side(c: &ComparisonContract, m: &ContractMeasure, s: &LoadedEstimateSet) -> Side {
    let mut out = Side {
        block: Vec::new(),
        drop: Vec::new(),
        range: None,
    };
    let mut put = |kind: TreatmentKind, r: Reason| match kind {
        TreatmentKind::Block => out.block.push(r),
        TreatmentKind::DropMeasure => out.drop.push(r),
    };
    let entry = s
        .set
        .estimates
        .iter()
        .find(|e| e.measure_id() == m.measure_id);
    let Some(entry) = entry else {
        put(c.treatment.missing, Reason::Missing);
        return out;
    };
    match entry {
        Estimate::NotEstimated(_) => put(c.treatment.missing, Reason::Missing),
        Estimate::Contested(_) => put(c.treatment.contested, Reason::Contested),
        Estimate::Estimated(e) if e.confidence < m.minimum_confidence => {
            put(c.treatment.contested, Reason::BelowMinimumConfidence)
        }
        Estimate::Estimated(_) => {}
    }
    if entry.effective_cutoff(&s.set.evidence_cutoff) < c.stale_before.as_str() {
        put(c.treatment.stale, Reason::Stale);
    }
    if entry.measure() != &m.measure() {
        out.block.push(Reason::DefinitionMismatch);
    }
    if out.block.is_empty()
        && out.drop.is_empty()
        && let Estimate::Estimated(e) = entry
    {
        let point = key(&m.scale, &e.value);
        let range = match (&c.uncertainty, &e.interval) {
            (Uncertainty::IntervalRobust, Some(i)) => {
                key(&m.scale, &i.low).zip(key(&m.scale, &i.high))
            }
            _ => point.zip(point),
        };
        match range {
            Some(r) => out.range = Some(r),
            // A value off the scale cannot be compared; E1 validation rejects it, so this
            // is reachable only for an unchecked set and is reported, not defaulted.
            None => out.block.push(Reason::DefinitionMismatch),
        }
    }
    out
}

fn merge(a: &[Reason], b: &[Reason]) -> Vec<Reason> {
    let set: BTreeSet<Reason> = a.iter().chain(b).copied().collect();
    set.into_iter().collect()
}

/// Relation of `a` to `b` from their ranges (point mode passes degenerate ranges).
fn relate(dir: Direction, a: (i128, i128), b: (i128, i128)) -> Relation {
    if a == b && a.0 == a.1 {
        return Relation::Equal;
    }
    let (hi_better, lo_better) = match dir {
        Direction::HigherIsBetter => (a.0 > b.1, a.1 < b.0),
        Direction::LowerIsBetter => (a.1 < b.0, a.0 > b.1),
    };
    if hi_better {
        Relation::Better
    } else if lo_better {
        Relation::Worse
    } else {
        Relation::Overlap
    }
}

/// The outcome and `blocked_reasons` implied by a pair's per-measure relations (contract 3.2).
pub fn pair_outcome(measures: &[PairMeasure]) -> (Outcome, Vec<Reason>) {
    if measures.is_empty() {
        return (Outcome::Blocked, vec![Reason::NoComparableMeasures]);
    }
    if measures.iter().any(|m| m.relation == Relation::Blocked) {
        let all: Vec<Reason> = measures
            .iter()
            .filter(|m| m.relation == Relation::Blocked)
            .flat_map(|m| m.reasons.iter().copied())
            .collect();
        return (Outcome::Blocked, merge(&all, &[]));
    }
    let has = |r: Relation| measures.iter().any(|m| m.relation == r);
    let outcome = if has(Relation::Overlap) {
        Outcome::Incomparable
    } else if !has(Relation::Worse) && has(Relation::Better) {
        Outcome::ADominatesB
    } else if !has(Relation::Better) && has(Relation::Worse) {
        Outcome::BDominatesA
    } else {
        Outcome::Incomparable
    };
    (outcome, Vec::new())
}

/// The frontier over `ids` implied by pair outcomes (contract 3.2); arrays are sorted.
pub fn frontier(ids: &[String], pairs: &[Pair]) -> Frontier {
    let mut dominated = BTreeSet::new();
    let mut blocked = BTreeSet::new();
    for p in pairs {
        match p.outcome {
            Outcome::ADominatesB => {
                dominated.insert(p.b.clone());
            }
            Outcome::BDominatesA => {
                dominated.insert(p.a.clone());
            }
            Outcome::Blocked => {
                blocked.insert(p.a.clone());
                blocked.insert(p.b.clone());
            }
            Outcome::Incomparable => {}
        }
    }
    let mut f = Frontier {
        non_dominated: Vec::new(),
        dominated: Vec::new(),
        undetermined: Vec::new(),
    };
    let sorted: BTreeSet<&String> = ids.iter().collect();
    for id in sorted {
        if dominated.contains(id) {
            f.dominated.push(id.clone());
        } else if blocked.contains(id) {
            f.undetermined.push(id.clone());
        } else {
            f.non_dominated.push(id.clone());
        }
    }
    f
}

/// The deterministic limitation strings (contract 3.3 and Erratum 2), in order.
pub fn limitations(c: &ComparisonContract, sha256: &str, pairs: &[Pair]) -> Vec<String> {
    let mut out = vec![
        format!(
            "{LIM_CONDITIONAL} every relation holds only for contract {} version {} (sha256 {sha256}), its measures, treatment and evidence cutoff {}; it is not a statement about any other contract or later evidence",
            c.contract_id, c.version, c.evidence_cutoff
        ),
        format!(
            "{LIM_STALE} stale means an effective evidence cutoff earlier than the contract's stale_before ({}); it is not claim-evidence freshness, which this tool does not evaluate",
            c.stale_before
        ),
    ];
    if c.uncertainty == Uncertainty::IntervalRobust {
        out.push(format!(
            "{LIM_POINT_AS_INTERVAL} an estimate without an interval is compared as [value, value]"
        ));
    }
    for p in pairs.iter().filter(|p| !p.dropped_measures.is_empty()) {
        out.push(format!(
            "{} the relation and outcome of this pair are over the remaining measures only, not every measure the contract includes",
            dropped_prefix(&p.a, &p.b, &p.dropped_measures)
        ));
    }
    out.push(format!("{LIM_SELECTED} the estimate sets compared were chosen by the compare invoker, not named by the contract; estimate_sets records which sets were used"));
    out
}

/// `dropped_measures a=<id> b=<id> measures=<m1,m2,..>:`
pub fn dropped_prefix(a: &str, b: &str, dropped: &[DroppedMeasure]) -> String {
    let names: Vec<&str> = dropped.iter().map(|d| d.measure_id.as_str()).collect();
    format!("{LIM_DROPPED} a={a} b={b} measures={}:", names.join(","))
}

/// Derive the comparison finding. `contract_path` and `contract_sha256` describe the file the
/// contract was read from.
pub fn derive(
    c: &ComparisonContract,
    contract_path: &str,
    contract_sha256: &str,
    sets: &[LoadedEstimateSet],
    derived_at: &str,
    tool: ToolRecord,
) -> Result<ComparisonFinding, BindingError> {
    let mut bound = bind(c, sets)?;
    bound.sort_by(|x, y| x.0.proposal_id.as_bytes().cmp(y.0.proposal_id.as_bytes()));
    let estimate_sets = bound
        .iter()
        .map(|(p, s)| EstimateSetRef {
            proposal_id: p.proposal_id.clone(),
            packet_sha256: p.packet_sha256.clone(),
            path: s.path.clone(),
            sha256: s.sha256.clone(),
        })
        .collect();
    let mut pairs = Vec::new();
    for (i, (pa, sa)) in bound.iter().enumerate() {
        for (pb, sb) in &bound[i + 1..] {
            let mut measures = Vec::new();
            let mut dropped_measures = Vec::new();
            for m in &c.measures {
                let (x, y) = (side(c, m, sa), side(c, m, sb));
                let block = merge(&x.block, &y.block);
                if !block.is_empty() {
                    let reasons = merge(&block, &merge(&x.drop, &y.drop));
                    measures.push(PairMeasure {
                        measure_id: m.measure_id.clone(),
                        relation: Relation::Blocked,
                        reasons,
                    });
                } else if !x.drop.is_empty() || !y.drop.is_empty() {
                    dropped_measures.push(DroppedMeasure {
                        measure_id: m.measure_id.clone(),
                        reasons: merge(&x.drop, &y.drop),
                    });
                } else if let (Some(ra), Some(rb)) = (x.range, y.range) {
                    measures.push(PairMeasure {
                        measure_id: m.measure_id.clone(),
                        relation: relate(m.direction, ra, rb),
                        reasons: Vec::new(),
                    });
                }
            }
            let (outcome, blocked_reasons) = pair_outcome(&measures);
            pairs.push(Pair {
                a: pa.proposal_id.clone(),
                b: pb.proposal_id.clone(),
                measures,
                dropped_measures,
                outcome,
                blocked_reasons,
            });
        }
    }
    let ids: Vec<String> = bound.iter().map(|(p, _)| p.proposal_id.clone()).collect();
    let frontier = frontier(&ids, &pairs);
    let limitations = limitations(c, contract_sha256, &pairs);
    Ok(ComparisonFinding {
        schema: COMPARISON_SCHEMA.to_string(),
        tool,
        derived_at: derived_at.to_string(),
        contract: ContractRef {
            path: contract_path.to_string(),
            sha256: contract_sha256.to_string(),
            contract_id: c.contract_id.clone(),
            version: c.version,
        },
        conditional_on: ConditionalOn {
            contract_id: c.contract_id.clone(),
            contract_version: c.version,
            contract_sha256: contract_sha256.to_string(),
            evidence_cutoff: c.evidence_cutoff.clone(),
        },
        estimate_sets,
        pairs,
        frontier,
        authority: ComparisonAuthority {
            priority_authorized: false,
            acceptance_authorized: false,
            implementation_authorized: false,
            readiness_judgment_issued: false,
        },
        limitations,
    })
}
