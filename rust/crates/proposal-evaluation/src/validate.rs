//! Structural validation of an estimate set (contract 2.1, 2.2 step 1 and 5).
//!
//! `validate_estimate_set` reports every violated rule with its one stable code instead of
//! stopping at the first. It reads no file and no clock. Pointer resolution lives in
//! [`crate::check`].

use crate::diag::{ErrorItem, Failure, codes};
use crate::digest::is_sha256_hex;
use crate::model::*;
use std::collections::BTreeSet;

/// The outcome of parsing: a typed set, or the errors that kept it from being one.
#[derive(Debug)]
pub enum Parsed {
    Set(Box<EstimateSet>),
    Invalid(Vec<ErrorItem>),
}

fn err(code: &str, at: impl Into<String>, message: impl Into<String>) -> ErrorItem {
    ErrorItem::new(code, at, message)
}

/// Parse `bytes`. Not JSON at all is a `Failure` (exit 2); JSON that breaks the closed schema
/// is reported as an invalid document (exit 1). `schema` is checked first (contract S4), and a
/// float where an integer is required is a `schema` error (contract S3) because the model
/// holds no float type.
pub fn parse_estimate_set(bytes: &[u8]) -> Result<Parsed, Failure> {
    let raw: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| Failure::Parse(e.to_string()))?;
    match raw.get("schema") {
        Some(serde_json::Value::String(s)) if s == ESTIMATE_SET_SCHEMA => {}
        Some(serde_json::Value::String(s)) => {
            return Ok(Parsed::Invalid(vec![err(
                codes::SCHEMA_MISMATCH,
                "schema",
                format!("schema is {s:?}, expected {ESTIMATE_SET_SCHEMA:?}"),
            )]));
        }
        _ => {
            return Ok(Parsed::Invalid(vec![err(
                codes::SCHEMA,
                "schema",
                "schema must be a string",
            )]));
        }
    }
    match serde_json::from_slice::<EstimateSet>(bytes) {
        Ok(set) => Ok(Parsed::Set(Box::new(set))),
        Err(e) if e.classify() == serde_json::error::Category::Data => {
            Ok(Parsed::Invalid(vec![err(codes::SCHEMA, "", e.to_string())]))
        }
        Err(e) => Err(Failure::Parse(e.to_string())),
    }
}

// ---------------------------------------------------------------------------
// Lexical helpers
// ---------------------------------------------------------------------------

/// `[a-z0-9]+` segments joined by one separator from `seps`; at least `min_seps` separators.
fn segmented(text: &str, seps: &[char], min_seps: usize) -> bool {
    let mut count = 0;
    let mut segment = 0;
    for c in text.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            segment += 1;
        } else if seps.contains(&c) && segment > 0 {
            count += 1;
            segment = 0;
        } else {
            return false;
        }
    }
    segment > 0 && count >= min_seps
}

/// `^[a-z0-9]+(?:[.-][a-z0-9]+)*$`
pub fn is_set_id(text: &str) -> bool {
    segmented(text, &['.', '-'], 0)
}

/// Packet pattern PK:10 `^[a-z0-9]+(?:[.-][a-z0-9]+)+$`.
pub fn is_proposal_id(text: &str) -> bool {
    segmented(text, &['.', '-'], 1)
}

/// `^[a-z0-9]+(?:[._-][a-z0-9]+)*$`
pub fn is_measure_id(text: &str) -> bool {
    segmented(text, &['.', '_', '-'], 0)
}

/// Absolute and normalised: begins with `/`, no `.`, `..` or empty component, no trailing `/`.
pub fn is_normalised_absolute(text: &str) -> bool {
    match text.strip_prefix('/') {
        Some(rest) => {
            !rest.is_empty()
                && !rest
                    .split('/')
                    .any(|c| c.is_empty() || c == "." || c == "..")
        }
        None => false,
    }
}

/// `YYYY-MM-DDTHH:MM:SSZ`, a real UTC calendar time (contract S5).
pub fn is_utc_timestamp(text: &str) -> bool {
    let b = text.as_bytes();
    if b.len() != 20 {
        return false;
    }
    for (i, byte) in b.iter().enumerate() {
        let ok = match i {
            4 | 7 => *byte == b'-',
            10 => *byte == b'T',
            13 | 16 => *byte == b':',
            19 => *byte == b'Z',
            _ => byte.is_ascii_digit(),
        };
        if !ok {
            return false;
        }
    }
    let num = |from: usize, to: usize| text[from..to].parse::<u32>().unwrap_or(u32::MAX);
    let (year, month, day) = (num(0, 4), num(5, 7), num(8, 10));
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day) && num(11, 13) < 24 && num(14, 16) < 60 && num(17, 19) < 60
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

fn non_empty(out: &mut Vec<ErrorItem>, text: &str, at: &str) {
    if text.is_empty() {
        out.push(err(codes::EMPTY_STRING, at, "must be a non-empty string"));
    }
}

fn time(out: &mut Vec<ErrorItem>, text: &str, at: &str) -> bool {
    let ok = is_utc_timestamp(text);
    if !ok {
        out.push(err(
            codes::TIME_FORMAT,
            at,
            format!("{text:?} is not YYYY-MM-DDTHH:MM:SSZ"),
        ));
    }
    ok
}

/// Shape of a `{path, sha256}` pointer. Returns true when both parts are well formed, which
/// is the precondition for resolving it.
pub fn check_file_ref(out: &mut Vec<ErrorItem>, f: &FileRef, at: &str) -> bool {
    let mut ok = true;
    if !is_normalised_absolute(&f.path) {
        out.push(err(
            codes::PATH_NOT_NORMALISED,
            format!("{at}.path"),
            "path must be absolute and normalised",
        ));
        ok = false;
    }
    if !is_sha256_hex(&f.sha256) {
        out.push(err(
            codes::SHA256_PATTERN,
            format!("{at}.sha256"),
            "sha256 must be 64 lowercase hex characters",
        ));
        ok = false;
    }
    ok
}

fn limitations(out: &mut Vec<ErrorItem>, list: &[String], at: &str) {
    for (i, text) in list.iter().enumerate() {
        non_empty(out, text, &format!("{at}.{i}"));
    }
}

fn level_index(levels: &[String], label: &str) -> Option<usize> {
    levels.iter().position(|l| l == label)
}

/// Checks `value` against `scale`; returns its position in scale order when comparable.
fn check_value(
    out: &mut Vec<ErrorItem>,
    scale: &Scale,
    value: &Value,
    at: &str,
) -> Option<(i64, usize)> {
    match (scale, value) {
        (Scale::Quantity { .. }, Value::Quantity(n)) => Some((*n, 0)),
        (Scale::Ordinal { levels }, Value::Level(label)) => match level_index(levels, label) {
            Some(i) => Some((0, i)),
            None => {
                out.push(err(
                    codes::LEVEL_UNKNOWN,
                    at,
                    format!("level {label:?} is not in the scale's levels"),
                ));
                None
            }
        },
        _ => {
            out.push(err(
                codes::VALUE_SCALE_MISMATCH,
                at,
                "value kind does not match the measure's scale kind",
            ));
            None
        }
    }
}

fn check_scale(out: &mut Vec<ErrorItem>, scale: &Scale, at: &str) {
    match scale {
        Scale::Ordinal { levels } => {
            for (i, l) in levels.iter().enumerate() {
                non_empty(out, l, &format!("{at}.levels.{i}"));
            }
            let unique: BTreeSet<&String> = levels.iter().collect();
            if levels.len() < 2 || unique.len() != levels.len() {
                out.push(err(
                    codes::SCALE_INVALID,
                    format!("{at}.levels"),
                    "an ordinal scale needs at least two distinct levels",
                ));
            }
        }
        Scale::Quantity { unit, exponent } => {
            non_empty(out, unit, &format!("{at}.unit"));
            if !(-18..=18).contains(exponent) {
                out.push(err(
                    codes::SCALE_INVALID,
                    format!("{at}.exponent"),
                    format!("exponent {exponent} is outside -18..=18"),
                ));
            }
        }
    }
}

fn check_basis(out: &mut Vec<ErrorItem>, basis: &[BasisRef], at: &str) {
    for (i, b) in basis.iter().enumerate() {
        let at = format!("{at}.{i}");
        non_empty(out, &b.claim_id, &format!("{at}.claim_id"));
        non_empty(
            out,
            &b.claim_revision_id,
            &format!("{at}.claim_revision_id"),
        );
        if !is_sha256_hex(&b.claim_revision_sha256) {
            out.push(err(
                codes::SHA256_PATTERN,
                format!("{at}.claim_revision_sha256"),
                "sha256 must be 64 lowercase hex characters",
            ));
        }
    }
}

fn check_estimate(out: &mut Vec<ErrorItem>, e: &Estimate, set_cutoff: &str, at: &str) {
    if !is_measure_id(e.measure_id()) {
        out.push(err(
            codes::ID_PATTERN,
            format!("{at}.measure_id"),
            "measure_id must match ^[a-z0-9]+(?:[._-][a-z0-9]+)*$",
        ));
    }
    let measure = e.measure();
    non_empty(
        out,
        &measure.definition,
        &format!("{at}.measure.definition"),
    );
    non_empty(out, &measure.scope, &format!("{at}.measure.scope"));
    check_scale(out, &measure.scale, &format!("{at}.measure.scale"));
    if let Some(cutoff) = e.own_cutoff()
        && time(out, cutoff, &format!("{at}.evidence_cutoff"))
        && is_utc_timestamp(set_cutoff)
        && cutoff > set_cutoff
    {
        out.push(err(
            codes::ESTIMATE_CUTOFF_AFTER_SET_CUTOFF,
            format!("{at}.evidence_cutoff"),
            "an estimate's cutoff is later than the set's cutoff",
        ));
    }
    if let Some(baseline) = e.baseline() {
        check_value(out, &measure.scale, baseline, &format!("{at}.baseline"));
    }
    limitations(out, e.limitations(), &format!("{at}.limitations"));
    check_basis(out, e.basis(), &format!("{at}.basis"));
    match e {
        Estimate::Estimated(x) => {
            let v = check_value(out, &measure.scale, &x.value, &format!("{at}.value"));
            if let Some(iv) = &x.interval {
                let lo = check_value(out, &measure.scale, &iv.low, &format!("{at}.interval.low"));
                let hi = check_value(
                    out,
                    &measure.scale,
                    &iv.high,
                    &format!("{at}.interval.high"),
                );
                if let (Some(lo), Some(hi)) = (lo, hi) {
                    let within = v.is_none_or(|v| lo <= v && v <= hi);
                    if lo > hi || !within {
                        out.push(err(
                            codes::INTERVAL_OUT_OF_ORDER,
                            format!("{at}.interval"),
                            "interval needs low <= value <= high in scale order",
                        ));
                    }
                }
            }
            if x.basis.is_empty() {
                out.push(err(
                    codes::BASIS_REQUIRED,
                    format!("{at}.basis"),
                    "an estimated measure needs at least one basis entry",
                ));
            }
        }
        Estimate::Contested(x) => {
            if x.competing.len() < 2 {
                out.push(err(
                    codes::COMPETING_INSUFFICIENT,
                    format!("{at}.competing"),
                    "a contested measure needs at least two competing readings",
                ));
            }
            for (i, c) in x.competing.iter().enumerate() {
                let cat = format!("{at}.competing.{i}");
                check_value(out, &measure.scale, &c.value, &format!("{cat}.value"));
                check_basis(out, &c.basis, &format!("{cat}.basis"));
                if c.basis.is_empty() {
                    out.push(err(
                        codes::COMPETING_INSUFFICIENT,
                        format!("{cat}.basis"),
                        "each competing reading needs at least one basis entry",
                    ));
                }
            }
        }
        Estimate::NotEstimated(x) => non_empty(out, &x.reason, &format!("{at}.reason")),
    }
}

/// Every structural rule of contract 2.1 and the authority block (S10).
pub fn validate_estimate_set(s: &EstimateSet) -> Vec<ErrorItem> {
    let mut out = Vec::new();
    if s.schema != ESTIMATE_SET_SCHEMA {
        out.push(err(
            codes::SCHEMA_MISMATCH,
            "schema",
            format!("schema must be {ESTIMATE_SET_SCHEMA:?}"),
        ));
    }
    if !is_set_id(&s.estimate_set_id) {
        out.push(err(
            codes::ID_PATTERN,
            "estimate_set_id",
            "estimate_set_id must match ^[a-z0-9]+(?:[.-][a-z0-9]+)*$",
        ));
    }
    if !is_proposal_id(&s.proposal.proposal_id) {
        out.push(err(
            codes::ID_PATTERN,
            "proposal.proposal_id",
            "proposal_id must match ^[a-z0-9]+(?:[.-][a-z0-9]+)+$",
        ));
    }
    check_file_ref(&mut out, &s.proposal.packet, "proposal.packet");
    non_empty(&mut out, &s.estimator.role, "estimator.role");
    non_empty(&mut out, &s.estimator.identity, "estimator.identity");
    let issued_ok = time(&mut out, &s.issued_at, "issued_at");
    let cutoff_ok = time(&mut out, &s.evidence_cutoff, "evidence_cutoff");
    if issued_ok && cutoff_ok && s.evidence_cutoff > s.issued_at {
        out.push(err(
            codes::CUTOFF_AFTER_ISSUE,
            "evidence_cutoff",
            "evidence_cutoff is later than issued_at",
        ));
    }
    if let Some(p) = &s.claim_projection {
        check_file_ref(&mut out, p, "claim_projection");
    }
    if s.estimates.is_empty() {
        out.push(err(
            codes::ESTIMATES_EMPTY,
            "estimates",
            "a set holds at least one estimate",
        ));
    }
    let mut seen = BTreeSet::new();
    for (i, e) in s.estimates.iter().enumerate() {
        let at = format!("estimates.{i}");
        if !seen.insert(e.measure_id()) {
            out.push(err(
                codes::DUPLICATE_MEASURE_ID,
                format!("{at}.measure_id"),
                format!("measure_id {:?} appears more than once", e.measure_id()),
            ));
        }
        check_estimate(&mut out, e, &s.evidence_cutoff, &at);
    }
    let any_basis = s.estimates.iter().any(|e| !e.all_basis().is_empty());
    if any_basis != s.claim_projection.is_some() {
        out.push(err(
            codes::CLAIM_PROJECTION_PRESENCE,
            "claim_projection",
            "claim_projection must be non-null exactly when some estimate has a basis entry",
        ));
    }
    limitations(&mut out, &s.limitations, "limitations");
    let a = &s.authority;
    for (name, set) in [
        ("readiness_judgment_issued", a.readiness_judgment_issued),
        ("priority_authorized", a.priority_authorized),
        ("acceptance_authorized", a.acceptance_authorized),
        ("implementation_authorized", a.implementation_authorized),
    ] {
        if set {
            out.push(err(
                codes::AUTHORITY_ASSERTED,
                format!("authority.{name}"),
                "authority members must all be false",
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert!(is_set_id("a") && is_set_id("a-b.c1") && !is_set_id("a--b") && !is_set_id("A"));
        assert!(is_proposal_id("a.b") && !is_proposal_id("ab"));
        assert!(is_measure_id("a_b-c.d") && !is_measure_id("_a") && !is_measure_id("a_"));
    }

    #[test]
    fn timestamps() {
        assert!(is_utc_timestamp("2026-10-22T12:00:00Z"));
        assert!(is_utc_timestamp("2028-02-29T00:00:00Z"));
        assert!(!is_utc_timestamp("2027-02-29T00:00:00Z"));
        assert!(!is_utc_timestamp("2026-10-22T12:00:00+00:00"));
        assert!(!is_utc_timestamp("2026-13-01T00:00:00Z"));
        assert!(!is_utc_timestamp("2026-10-22T24:00:00Z"));
    }

    #[test]
    fn absolute_paths() {
        assert!(is_normalised_absolute("/a/b"));
        for bad in ["a/b", "/", "/a/", "/a/./b", "/a/../b", "//a"] {
            assert!(!is_normalised_absolute(bad), "{bad}");
        }
    }
}
