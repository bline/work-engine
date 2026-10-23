//! `check --kind comparison [--rederive]` (contract 3.4).
//!
//! Without `--rederive` the checker establishes the finding's internal consistency (pair
//! coverage, outcomes, frontier, limitations) and, when the contract at its stored path can be
//! read and matches its stored sha256, the rules that need the contract (measure coverage, the
//! `conditional_on` block, the limitations that depend on the uncertainty mode). A contract that
//! cannot be resolved is reported and those rules are skipped: nothing about them is established.
//!
//! With `--rederive` the contract and every estimate set are re-read at their stored paths and
//! hashed (a mismatch is a binding refusal, exit 3), the finding is re-derived with its stored
//! `derived_at`, and the whole document except `tool` is compared (contract S9, R1 pattern).
//! A pass then establishes that the finding is the deterministic function of those inputs.

use crate::check::read_regular_file;
use crate::compare::{
    LIM_CONDITIONAL, LIM_POINT_AS_INTERVAL, LIM_SELECTED, LIM_STALE, LoadedEstimateSet,
    current_tool, derive, dropped_prefix, frontier, load_contract, load_estimate_set, pair_outcome,
};
use crate::contract::{ParsedContract, parse_contract, validate_contract};
use crate::diag::{CheckReport, ErrorItem, Failure, codes};
use crate::digest::{is_sha256_hex, sha256_hex};
use crate::model::*;
use crate::scan::scan;
use crate::validate::{check_file_ref, err, is_proposal_id, is_set_id, non_empty, time};
use std::collections::BTreeSet;

/// The `kind` printed in a comparison report.
pub const KIND_COMPARISON: &str = "comparison";

/// Report note: what a pass without `--rederive` leaves open.
pub const NOTE_NOT_REDERIVED: &str = "the finding was checked for internal consistency, against the contract at its stored path, and for the sha256 of every stored estimate-set path; the estimate sets were not read as records and the finding was not re-derived from them (use --rederive)";

/// Check the bytes of a comparison finding (contract 3.4). Not JSON is a [`Failure`] (exit 2); a
/// stored input that cannot be re-read or whose digest differs under `rederive` is a
/// [`Failure::Binding`] (exit 3); every violated rule of a parseable document is listed in the
/// report (exit 1 when any).
pub fn check_comparison_bytes(bytes: &[u8], rederive: bool) -> Result<CheckReport, Failure> {
    let finding = match parse_finding(bytes)? {
        Err(errors) => return Ok(CheckReport::new(KIND_COMPARISON, errors, Vec::new())),
        Ok(f) => f,
    };
    let mut errors = Vec::new();
    structure(&finding, &mut errors);
    internal(&finding, &mut errors);
    let contract_ok = check_file_ref_ok(&finding.contract.path, &finding.contract.sha256);
    let mut notes = Vec::new();
    let mut contract = None;
    if contract_ok {
        if rederive {
            contract = Some(
                load_contract(&finding.contract.path, Some(&finding.contract.sha256))
                    .map_err(|f| resolved(f, "contract"))?,
            );
        } else {
            contract = resolve_contract(&finding, &mut errors);
        }
    }
    if let Some(c) = &contract {
        with_contract(&finding, c, &mut errors);
    }
    if !rederive {
        // Contract S6 / EXT-004: every stored pointer is read at its stored path and hashed.
        // `--rederive` does the same through `load_estimate_set` and refuses with exit 3.
        for (i, set) in finding.estimate_sets.iter().enumerate() {
            if check_file_ref_ok(&set.path, &set.sha256) {
                read_pointer(
                    &set.path,
                    &set.sha256,
                    &format!("estimate_sets.{i}"),
                    &mut errors,
                );
            }
        }
    }
    if !rederive {
        notes.push(NOTE_NOT_REDERIVED.to_string());
    } else if let Some(loaded) = contract {
        rederive_check(&finding, &loaded, &mut errors, &mut notes)?;
    }
    Ok(CheckReport::new(KIND_COMPARISON, errors, notes))
}

/// A stored pointer that cannot be re-read is a binding refusal, not a usage error; `at` names
/// the pointer (`contract` or `estimate_sets.N`).
fn resolved(f: Failure, at: &str) -> Failure {
    match f {
        Failure::Io { path, message } => Failure::Binding {
            message: format!("stored path {path} cannot be read: {message}"),
            errors: vec![ErrorItem::new(codes::POINTER_UNRESOLVED, at, message)],
        },
        other => other,
    }
}

fn check_file_ref_ok(path: &str, sha256: &str) -> bool {
    crate::validate::is_normalised_absolute(path) && is_sha256_hex(sha256)
}

/// Lexical findings, then the `schema` const, then the closed schema (mirrors `parse_contract`).
fn parse_finding(bytes: &[u8]) -> Result<Result<ComparisonFinding, Vec<ErrorItem>>, Failure> {
    let findings = scan(bytes).map_err(|e| Failure::Parse(e.to_string()))?;
    if !findings.is_empty() {
        return Ok(Err(findings
            .into_iter()
            .map(|f| err(codes::SCHEMA, f.at, f.message))
            .collect()));
    }
    let raw: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| Failure::Parse(e.to_string()))?;
    match raw.get("schema") {
        Some(serde_json::Value::String(s)) if s == COMPARISON_SCHEMA => {}
        Some(serde_json::Value::String(s)) => {
            return Ok(Err(vec![err(
                codes::SCHEMA_MISMATCH,
                "schema",
                format!("schema is {s:?}, expected {COMPARISON_SCHEMA:?}"),
            )]));
        }
        _ => {
            return Ok(Err(vec![err(
                codes::SCHEMA,
                "schema",
                "schema must be a string",
            )]));
        }
    }
    match serde_json::from_slice::<ComparisonFinding>(bytes) {
        Ok(f) => Ok(Ok(f)),
        Err(e) if e.classify() == serde_json::error::Category::Data => {
            Ok(Err(vec![err(codes::SCHEMA, "", e.to_string())]))
        }
        Err(e) => Err(Failure::Parse(e.to_string())),
    }
}

fn sha(out: &mut Vec<ErrorItem>, text: &str, at: &str) {
    if !is_sha256_hex(text) {
        out.push(err(
            codes::SHA256_PATTERN,
            at,
            "sha256 must be 64 lowercase hex characters",
        ));
    }
}

fn file_ref(out: &mut Vec<ErrorItem>, path: &str, sha256: &str, at: &str) {
    check_file_ref(
        out,
        &FileRef {
            path: path.to_string(),
            sha256: sha256.to_string(),
        },
        at,
    );
}

/// Shape rules: ids, hashes, times, pointers, the constant authority block, empty strings.
fn structure(f: &ComparisonFinding, out: &mut Vec<ErrorItem>) {
    time(out, &f.derived_at, "derived_at");
    non_empty(out, &f.tool.name, "tool.name");
    non_empty(out, &f.tool.version, "tool.version");
    sha(out, &f.tool.binary_sha256, "tool.binary_sha256");
    file_ref(out, &f.contract.path, &f.contract.sha256, "contract");
    if !is_set_id(&f.contract.contract_id) {
        out.push(err(
            codes::ID_PATTERN,
            "contract.contract_id",
            "contract_id must match ^[a-z0-9]+(?:[.-][a-z0-9]+)*$",
        ));
    }
    if f.contract.version < 1 {
        out.push(err(
            codes::VERSION_INVALID,
            "contract.version",
            "version must be >= 1",
        ));
    }
    sha(
        out,
        &f.conditional_on.contract_sha256,
        "conditional_on.contract_sha256",
    );
    time(
        out,
        &f.conditional_on.evidence_cutoff,
        "conditional_on.evidence_cutoff",
    );
    for (i, s) in f.estimate_sets.iter().enumerate() {
        let at = format!("estimate_sets.{i}");
        if !is_proposal_id(&s.proposal_id) {
            out.push(err(
                codes::ID_PATTERN,
                format!("{at}.proposal_id"),
                "proposal_id must match ^[a-z0-9]+(?:[.-][a-z0-9]+)+$",
            ));
        }
        sha(out, &s.packet_sha256, &format!("{at}.packet_sha256"));
        file_ref(out, &s.path, &s.sha256, &at);
    }
    for (i, p) in f.pairs.iter().enumerate() {
        for (side, id) in [("a", &p.a), ("b", &p.b)] {
            if !is_proposal_id(id) {
                out.push(err(
                    codes::ID_PATTERN,
                    format!("pairs.{i}.{side}"),
                    "proposal id must match ^[a-z0-9]+(?:[.-][a-z0-9]+)+$",
                ));
            }
        }
    }
    for (name, value) in [
        ("priority_authorized", f.authority.priority_authorized),
        ("acceptance_authorized", f.authority.acceptance_authorized),
        (
            "implementation_authorized",
            f.authority.implementation_authorized,
        ),
        (
            "readiness_judgment_issued",
            f.authority.readiness_judgment_issued,
        ),
    ] {
        if value {
            out.push(err(
                codes::AUTHORITY_ASSERTED,
                format!("authority.{name}"),
                "a comparison finding carries no authority; this must be false",
            ));
        }
    }
    for (i, l) in f.limitations.iter().enumerate() {
        non_empty(out, l, &format!("limitations.{i}"));
    }
}

/// The unordered pairs of `ids` (already sorted), `a < b`, in sorted order.
fn all_pairs(ids: &[&str]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, a) in ids.iter().enumerate() {
        for b in &ids[i + 1..] {
            out.push((a.to_string(), b.to_string()));
        }
    }
    out
}

/// Rules that need nothing outside the finding.
fn internal(f: &ComparisonFinding, out: &mut Vec<ErrorItem>) {
    // pair_coverage: estimate_sets sorted and unique; pairs are exactly all unordered pairs.
    let ids: Vec<&str> = f
        .estimate_sets
        .iter()
        .map(|s| s.proposal_id.as_str())
        .collect();
    let sorted_unique = ids.windows(2).all(|w| w[0].as_bytes() < w[1].as_bytes());
    if !sorted_unique {
        out.push(err(
            codes::PAIR_COVERAGE,
            "estimate_sets",
            "estimate_sets must be unique and sorted by proposal_id",
        ));
    }
    if ids.len() < 2 {
        out.push(err(
            codes::PAIR_COVERAGE,
            "estimate_sets",
            "a comparison covers at least two proposals",
        ));
    }
    for (i, p) in f.pairs.iter().enumerate() {
        if p.a.as_bytes() >= p.b.as_bytes() {
            out.push(err(
                codes::PAIR_COVERAGE,
                format!("pairs.{i}"),
                "a pair must have a < b",
            ));
        }
    }
    if sorted_unique {
        let expected = all_pairs(&ids);
        let actual: Vec<(String, String)> =
            f.pairs.iter().map(|p| (p.a.clone(), p.b.clone())).collect();
        let seen: BTreeSet<&(String, String)> = actual.iter().collect();
        if seen.len() != actual.len() {
            out.push(err(
                codes::PAIR_COVERAGE,
                "pairs",
                "a pair is listed more than once",
            ));
        }
        for e in &expected {
            if !seen.contains(e) {
                out.push(err(
                    codes::PAIR_COVERAGE,
                    "pairs",
                    format!("pair a={} b={} is missing", e.0, e.1),
                ));
            }
        }
        let wanted: BTreeSet<&(String, String)> = expected.iter().collect();
        for (i, a) in actual.iter().enumerate() {
            if !wanted.contains(a) && a.0.as_bytes() < a.1.as_bytes() {
                out.push(err(
                    codes::PAIR_COVERAGE,
                    format!("pairs.{i}"),
                    "pair names a proposal that is not in estimate_sets",
                ));
            }
        }
        if seen.len() == actual.len()
            && actual != expected
            && wanted.is_subset(&seen)
            && seen.is_subset(&wanted)
        {
            out.push(err(
                codes::PAIR_COVERAGE,
                "pairs",
                "pairs must be listed in sorted order",
            ));
        }
    }
    // outcome_inconsistent: reasons, outcome and blocked_reasons follow from the relations.
    for (i, p) in f.pairs.iter().enumerate() {
        let at = format!("pairs.{i}");
        for (j, m) in p.measures.iter().enumerate() {
            let blocked = m.relation == Relation::Blocked;
            if blocked == m.reasons.is_empty() {
                out.push(err(
                    codes::OUTCOME_INCONSISTENT,
                    format!("{at}.measures.{j}.reasons"),
                    "reasons must be non-empty exactly when the relation is blocked",
                ));
            }
            if !strictly_ascending(&m.reasons) {
                out.push(err(
                    codes::OUTCOME_INCONSISTENT,
                    format!("{at}.measures.{j}.reasons"),
                    "reasons must be distinct and in declaration order",
                ));
            }
        }
        for (j, d) in p.dropped_measures.iter().enumerate() {
            if d.reasons.is_empty() || !strictly_ascending(&d.reasons) {
                out.push(err(
                    codes::OUTCOME_INCONSISTENT,
                    format!("{at}.dropped_measures.{j}.reasons"),
                    "a dropped measure needs distinct reasons in declaration order",
                ));
            }
        }
        let (outcome, reasons) = pair_outcome(&p.measures);
        if p.outcome != outcome {
            out.push(err(
                codes::OUTCOME_INCONSISTENT,
                format!("{at}.outcome"),
                format!(
                    "the relations imply outcome {outcome:?}, not {:?}",
                    p.outcome
                ),
            ));
        }
        if p.blocked_reasons != reasons {
            out.push(err(
                codes::OUTCOME_INCONSISTENT,
                format!("{at}.blocked_reasons"),
                format!("the relations imply blocked_reasons {reasons:?}"),
            ));
        }
    }
    // frontier_inconsistent: the stored frontier is the one the outcomes imply (a partition).
    let owned: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
    if frontier(&owned, &f.pairs) != f.frontier {
        out.push(err(
            codes::FRONTIER_INCONSISTENT,
            "frontier",
            "the frontier is not the partition implied by the pair outcomes over estimate_sets",
        ));
    }
    // limitation_missing: the entries that do not depend on the contract.
    for (prefix, name) in [
        (LIM_CONDITIONAL, "conditional_on_contract"),
        (LIM_STALE, "stale_is_cutoff_age"),
        (LIM_SELECTED, "estimate_sets_selected_at_derivation"),
    ] {
        need_limitation(f, prefix, name, out);
    }
    for p in f.pairs.iter().filter(|p| !p.dropped_measures.is_empty()) {
        let prefix = dropped_prefix(&p.a, &p.b, &p.dropped_measures);
        need_limitation(f, &prefix, &prefix, out);
    }
    // conditional_on agrees with the contract block.
    let (c, k) = (&f.contract, &f.conditional_on);
    for (at, same) in [
        ("conditional_on.contract_id", c.contract_id == k.contract_id),
        (
            "conditional_on.contract_version",
            c.version == k.contract_version,
        ),
        (
            "conditional_on.contract_sha256",
            c.sha256 == k.contract_sha256,
        ),
    ] {
        if !same {
            out.push(err(
                codes::CONDITIONAL_ON_MISMATCH,
                at,
                "conditional_on must equal the contract block",
            ));
        }
    }
}

fn strictly_ascending(reasons: &[Reason]) -> bool {
    reasons.windows(2).all(|w| w[0] < w[1])
}

fn need_limitation(f: &ComparisonFinding, prefix: &str, name: &str, out: &mut Vec<ErrorItem>) {
    if !f.limitations.iter().any(|l| l.starts_with(prefix)) {
        out.push(err(
            codes::LIMITATION_MISSING,
            "limitations",
            format!("no limitation starts with {name:?}"),
        ));
    }
}

/// Read the regular file at a stored path and hash it. `None` after reporting
/// `pointer_unresolved` (unreadable, not a regular file, too large) or `digest_mismatch` at `at`.
fn read_pointer(
    path: &str,
    stored_sha256: &str,
    at: &str,
    out: &mut Vec<ErrorItem>,
) -> Option<(Vec<u8>, String)> {
    let bytes = match read_regular_file(path) {
        Ok(b) => b,
        Err(message) => {
            out.push(err(codes::POINTER_UNRESOLVED, at, message));
            return None;
        }
    };
    let found = sha256_hex(&bytes);
    if found != stored_sha256 {
        out.push(err(
            codes::DIGEST_MISMATCH,
            at,
            format!("stored {stored_sha256}, found {found}"),
        ));
        return None;
    }
    Some((bytes, found))
}

/// Non-rederive resolution of the contract at its stored path. Reports and returns `None` when
/// it cannot be read, its digest differs, or it is not a valid contract.
fn resolve_contract(f: &ComparisonFinding, out: &mut Vec<ErrorItem>) -> Option<LoadedContract> {
    let (bytes, found) = read_pointer(&f.contract.path, &f.contract.sha256, "contract", out)?;
    match parse_contract(&bytes) {
        Ok(ParsedContract::Contract(c)) if validate_contract(&c).is_empty() => {
            Some(LoadedContract {
                path: f.contract.path.clone(),
                sha256: found,
                contract: *c,
            })
        }
        _ => {
            out.push(err(
                codes::UPSTREAM_CLASS_MISMATCH,
                "contract",
                "the file at the stored path is not a valid proposal-comparison-contract-v1",
            ));
            None
        }
    }
}

use crate::compare::LoadedContract;

/// Rules that need the contract: `conditional_on` against the file, proposals, measure coverage
/// and the interval limitation.
fn with_contract(f: &ComparisonFinding, loaded: &LoadedContract, out: &mut Vec<ErrorItem>) {
    let c = &loaded.contract;
    if f.contract.contract_id != c.contract_id {
        out.push(err(
            codes::CONDITIONAL_ON_MISMATCH,
            "contract.contract_id",
            "contract_id differs from the contract at the stored path",
        ));
    }
    if f.contract.version != c.version {
        out.push(err(
            codes::CONDITIONAL_ON_MISMATCH,
            "contract.version",
            "version differs from the contract at the stored path",
        ));
    }
    if f.conditional_on.evidence_cutoff != c.evidence_cutoff {
        out.push(err(
            codes::CONDITIONAL_ON_MISMATCH,
            "conditional_on.evidence_cutoff",
            "evidence_cutoff differs from the contract at the stored path",
        ));
    }
    // The compared proposal revisions are exactly the contract's.
    let mut want: Vec<(&str, &str)> = c
        .proposals
        .iter()
        .map(|p| (p.proposal_id.as_str(), p.packet_sha256.as_str()))
        .collect();
    want.sort();
    let have: Vec<(&str, &str)> = f
        .estimate_sets
        .iter()
        .map(|s| (s.proposal_id.as_str(), s.packet_sha256.as_str()))
        .collect();
    if want != have {
        out.push(err(
            codes::PAIR_COVERAGE,
            "estimate_sets",
            "estimate_sets must name exactly the contract's proposals and packet revisions",
        ));
    }
    // measure_coverage: measures and dropped_measures partition the contract's measures.
    let order: Vec<&str> = c.measures.iter().map(|m| m.measure_id.as_str()).collect();
    for (i, p) in f.pairs.iter().enumerate() {
        let listed: Vec<&str> = p.measures.iter().map(|m| m.measure_id.as_str()).collect();
        let dropped: Vec<&str> = p
            .dropped_measures
            .iter()
            .map(|m| m.measure_id.as_str())
            .collect();
        let in_contract_order = |ids: &[&str]| {
            let mut last = None;
            ids.iter()
                .all(|id| match order.iter().position(|o| o == id) {
                    Some(n) if last.is_none_or(|l| n > l) => {
                        last = Some(n);
                        true
                    }
                    _ => false,
                })
        };
        let mut union: Vec<&str> = listed.iter().chain(dropped.iter()).copied().collect();
        union.sort_by_key(|id| order.iter().position(|o| o == id));
        if !(in_contract_order(&listed) && in_contract_order(&dropped) && union == order) {
            out.push(err(
                codes::MEASURE_COVERAGE,
                format!("pairs.{i}"),
                "measures and dropped_measures must be disjoint, each in contract order, and together be the contract's measures",
            ));
        }
    }
    // limitation_missing: the interval limitation exists exactly when the contract needs it.
    if c.uncertainty == Uncertainty::IntervalRobust {
        need_limitation(
            f,
            LIM_POINT_AS_INTERVAL,
            "point_as_degenerate_interval",
            out,
        );
    }
}

/// `--rederive`: load every stored estimate set, derive again with the stored `derived_at`, and
/// compare everything except `tool`.
fn rederive_check(
    f: &ComparisonFinding,
    contract: &LoadedContract,
    out: &mut Vec<ErrorItem>,
    notes: &mut Vec<String>,
) -> Result<(), Failure> {
    if !crate::validate::is_utc_timestamp(&f.derived_at)
        || f.estimate_sets
            .iter()
            .any(|s| !check_file_ref_ok(&s.path, &s.sha256))
    {
        return Ok(()); // already reported by the shape rules
    }
    let sets: Vec<LoadedEstimateSet> = f
        .estimate_sets
        .iter()
        .enumerate()
        .map(|(i, s)| {
            load_estimate_set(&s.path, Some(&s.sha256), &format!("estimate_sets.{i}"))
                .map_err(|f| resolved(f, &format!("estimate_sets.{i}")))
        })
        .collect::<Result<_, _>>()?;
    let fresh = derive(
        &contract.contract,
        &contract.path,
        &contract.sha256,
        &sets,
        &f.derived_at,
        current_tool()?,
    )
    .map_err(|e| Failure::Binding {
        message: e.to_string(),
        errors: e.errors,
    })?;
    let to_value = |x: &ComparisonFinding| {
        serde_json::to_value(x).map_err(|e| Failure::Usage(format!("cannot serialise: {e}")))
    };
    let (mut stored, mut derived) = (to_value(f)?, to_value(&fresh)?);
    let tool_differs = f.tool != fresh.tool;
    for v in [&mut stored, &mut derived] {
        if let Some(o) = v.as_object_mut() {
            o.remove("tool");
        }
    }
    if stored != derived {
        let (s, d) = (stored.as_object(), derived.as_object());
        let key = match (s, d) {
            (Some(s), Some(d)) => s
                .keys()
                .chain(d.keys())
                .filter(|k| s.get(*k) != d.get(*k))
                .min()
                .cloned()
                .unwrap_or_default(),
            _ => String::new(),
        };
        out.push(err(
            codes::REDERIVE_MISMATCH,
            key.clone(),
            format!("re-deriving from the stored inputs gives a different {key:?}"),
        ));
    } else if tool_differs {
        notes.push(codes::NOTE_TOOL_DIFFERS.to_string());
    }
    Ok(())
}
