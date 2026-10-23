//! Parsing and structural validation of a comparison contract (contract 3.1, 3.4).
//!
//! The contract is authored by the decision owner. It names the proposal revisions compared
//! (by `packet_sha256`, UD1), the measures with their comparability definition, the treatment
//! of missing, contested and stale evidence, and the cutoffs. It has no weights field.

use crate::diag::{ErrorItem, Failure, codes};
use crate::digest::is_sha256_hex;
use crate::model::*;
use crate::validate::{
    check_scale, err, is_measure_id, is_proposal_id, is_set_id, non_empty, time,
};
use std::collections::BTreeSet;

/// The outcome of parsing: a typed contract, or the errors that kept it from being one.
#[derive(Debug)]
pub enum ParsedContract {
    Contract(Box<ComparisonContract>),
    Invalid(Vec<ErrorItem>),
}

/// Parse `bytes` as a contract: lexical findings first, then the `schema` const, then the
/// closed schema. Not JSON at all is a `Failure` (exit 2).
pub fn parse_contract(bytes: &[u8]) -> Result<ParsedContract, Failure> {
    let findings = crate::scan::scan(bytes).map_err(|e| Failure::Parse(e.to_string()))?;
    if !findings.is_empty() {
        return Ok(ParsedContract::Invalid(
            findings
                .into_iter()
                .map(|f| err(codes::SCHEMA, f.at, f.message))
                .collect(),
        ));
    }
    let raw: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| Failure::Parse(e.to_string()))?;
    match raw.get("schema") {
        Some(serde_json::Value::String(s)) if s == CONTRACT_SCHEMA => {}
        Some(serde_json::Value::String(s)) => {
            return Ok(ParsedContract::Invalid(vec![err(
                codes::SCHEMA_MISMATCH,
                "schema",
                format!("schema is {s:?}, expected {CONTRACT_SCHEMA:?}"),
            )]));
        }
        _ => {
            return Ok(ParsedContract::Invalid(vec![err(
                codes::SCHEMA,
                "schema",
                "schema must be a string",
            )]));
        }
    }
    match serde_json::from_slice::<ComparisonContract>(bytes) {
        Ok(c) => Ok(ParsedContract::Contract(Box::new(c))),
        Err(e) if e.classify() == serde_json::error::Category::Data => Ok(ParsedContract::Invalid(
            vec![err(codes::SCHEMA, "", e.to_string())],
        )),
        Err(e) => Err(Failure::Parse(e.to_string())),
    }
}

/// Every structural rule of contract 3.1 and the authority block (S10).
pub fn validate_contract(c: &ComparisonContract) -> Vec<ErrorItem> {
    let mut out = Vec::new();
    if c.schema != CONTRACT_SCHEMA {
        out.push(err(
            codes::SCHEMA_MISMATCH,
            "schema",
            format!("schema must be {CONTRACT_SCHEMA:?}"),
        ));
    }
    if !is_set_id(&c.contract_id) {
        out.push(err(
            codes::ID_PATTERN,
            "contract_id",
            "contract_id must match ^[a-z0-9]+(?:[.-][a-z0-9]+)*$",
        ));
    }
    if c.version < 1 {
        out.push(err(
            codes::VERSION_INVALID,
            "version",
            "version must be >= 1",
        ));
    }
    let a = &c.selecting_authority;
    non_empty(
        &mut out,
        &a.decision_owner,
        "selecting_authority.decision_owner",
    );
    non_empty(
        &mut out,
        &a.exercised_by,
        "selecting_authority.exercised_by",
    );
    non_empty(&mut out, &a.evidence, "selecting_authority.evidence");
    non_empty(&mut out, &c.decision_scope, "decision_scope");
    if c.proposals.len() < 2 {
        out.push(err(
            codes::CONTRACT_PROPOSALS_INSUFFICIENT,
            "proposals",
            "a contract compares at least two proposals",
        ));
    }
    let mut seen = BTreeSet::new();
    for (i, p) in c.proposals.iter().enumerate() {
        let at = format!("proposals.{i}");
        if !is_proposal_id(&p.proposal_id) {
            out.push(err(
                codes::ID_PATTERN,
                format!("{at}.proposal_id"),
                "proposal_id must match ^[a-z0-9]+(?:[.-][a-z0-9]+)+$",
            ));
        }
        if !seen.insert(p.proposal_id.as_str()) {
            out.push(err(
                codes::DUPLICATE_PROPOSAL_ID,
                format!("{at}.proposal_id"),
                format!("proposal_id {:?} appears more than once", p.proposal_id),
            ));
        }
        if !is_sha256_hex(&p.packet_sha256) {
            out.push(err(
                codes::SHA256_PATTERN,
                format!("{at}.packet_sha256"),
                "sha256 must be 64 lowercase hex characters",
            ));
        }
    }
    if c.measures.is_empty() {
        out.push(err(
            codes::MEASURES_EMPTY,
            "measures",
            "a contract includes at least one measure",
        ));
    }
    let mut seen = BTreeSet::new();
    for (i, m) in c.measures.iter().enumerate() {
        let at = format!("measures.{i}");
        if !is_measure_id(&m.measure_id) {
            out.push(err(
                codes::ID_PATTERN,
                format!("{at}.measure_id"),
                "measure_id must match ^[a-z0-9]+(?:[._-][a-z0-9]+)*$",
            ));
        }
        if !seen.insert(m.measure_id.as_str()) {
            out.push(err(
                codes::DUPLICATE_MEASURE_ID,
                format!("{at}.measure_id"),
                format!("measure_id {:?} appears more than once", m.measure_id),
            ));
        }
        non_empty(&mut out, &m.definition, &format!("{at}.definition"));
        non_empty(&mut out, &m.scope, &format!("{at}.scope"));
        check_scale(&mut out, &m.scale, &format!("{at}.scale"));
    }
    for (i, x) in c.excluded_concerns.iter().enumerate() {
        non_empty(
            &mut out,
            &x.concern,
            &format!("excluded_concerns.{i}.concern"),
        );
        non_empty(
            &mut out,
            &x.reason,
            &format!("excluded_concerns.{i}.reason"),
        );
    }
    let cutoff_ok = time(&mut out, &c.evidence_cutoff, "evidence_cutoff");
    let stale_ok = time(&mut out, &c.stale_before, "stale_before");
    if cutoff_ok && stale_ok && c.stale_before > c.evidence_cutoff {
        out.push(err(
            codes::STALE_BEFORE_AFTER_CUTOFF,
            "stale_before",
            "stale_before is later than evidence_cutoff",
        ));
    }
    let au = &c.authority;
    for (name, set) in [
        ("priority_authorized", au.priority_authorized),
        ("acceptance_authorized", au.acceptance_authorized),
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
