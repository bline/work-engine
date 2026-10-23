//! Pointer resolution and upstream classes (contract 4.2 steps 2-5, S6, S7; Errata 3, 4, 6).
//!
//! Every stored `{path, sha256}` is resolved from its stored path with a bounded regular-file
//! read and hashed (S6): an unreadable path is `pointer_unresolved`, differing bytes are
//! `digest_mismatch`. A consumed record's class is checked against its owner before any field is
//! trusted (S7). Deep validation of a packet or decision belongs to `proposal_packets.py`; this
//! module classifies by key set and reads only what the portfolio rules need. Nothing is written.

use crate::diag::{ErrorItem, codes};
use crate::model::*;
use crate::read::read_stored;
use crate::structure::{err, parse_portfolio};
use decision_readiness::model::{ASSESSMENT_SCHEMA, Assessment, Purpose};
use proposal_evaluation::contract::{ParsedContract, parse_contract};
use proposal_evaluation::diag::Failure;
use proposal_evaluation::digest::{is_sha256_hex, sha256_hex};
use proposal_evaluation::model::{
    COMPARISON_SCHEMA, ComparisonFinding, ESTIMATE_SET_SCHEMA, EstimateSet,
};
use proposal_evaluation::validate::is_normalised_absolute;
use proposal_evaluation::{check_comparison_bytes, check_estimate_set_bytes};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const PACKET_KEYS: [&str; 11] = [
    "schema_version",
    "proposal_id",
    "family_id",
    "title",
    "lifecycle_state",
    "narrative",
    "origin_refs",
    "placement",
    "uncertainty",
    "authority",
    "relationships",
];
const DECISION_KEYS: [&str; 6] = [
    "schema_version",
    "proposal_id",
    "source",
    "decision",
    "authority",
    "constraints",
];
const DECIDED: &str = "decided";

/// What the cross-proposal rules need from a packet that resolved and classified.
#[derive(Debug, Clone)]
pub(crate) struct PacketView {
    /// `(type, target_id)` of each relationship in the packet bytes (PK:27).
    #[allow(dead_code)] // read by the cross-proposal rules (unit 4)
    pub relationships: Vec<(String, String)>,
}

/// The outcome of resolution: errors in contract order, and the packets that were read.
#[derive(Debug, Default)]
pub(crate) struct Upstream {
    pub errors: Vec<ErrorItem>,
    /// Keyed by the `proposal_set` member's `proposal_id` (the first member with that id).
    pub packets: BTreeMap<String, PacketView>,
}

/// Resolve `path` and hash it. `None` when the pointer's shape is wrong (the structure rules
/// report that), the path cannot be read, or the bytes differ from `sha256`.
pub(crate) fn resolve(
    out: &mut Vec<ErrorItem>,
    path: &str,
    sha256: &str,
    at: &str,
) -> Option<Vec<u8>> {
    if !is_normalised_absolute(path) || !is_sha256_hex(sha256) {
        return None;
    }
    let bytes = match read_stored(path) {
        Ok(b) => b,
        Err(e) => {
            out.push(err(
                codes::POINTER_UNRESOLVED,
                at,
                format!("cannot read {path}: {e}"),
            ));
            return None;
        }
    };
    let actual = sha256_hex(&bytes);
    if actual != sha256 {
        out.push(err(
            codes::DIGEST_MISMATCH,
            at,
            format!("{path} hashes to {actual}, the record states {sha256}"),
        ));
        return None;
    }
    Some(bytes)
}

fn nested(errors: &[ErrorItem]) -> String {
    errors
        .iter()
        .map(|e| format!("{} at {}", e.code, e.at))
        .collect::<Vec<_>>()
        .join("; ")
}

fn object(bytes: &[u8]) -> Option<Map<String, Value>> {
    match serde_json::from_slice::<Value>(bytes) {
        Ok(Value::Object(o)) => Some(o),
        _ => None,
    }
}

fn keys_are(obj: &Map<String, Value>, expected: &[&str], extra: Option<&str>) -> bool {
    let want = expected.len() + usize::from(extra.is_some());
    obj.len() == want
        && expected.iter().all(|k| obj.contains_key(*k))
        && extra.is_none_or(|k| obj.contains_key(k))
}

fn is_packet(obj: &Map<String, Value>) -> bool {
    obj.get("schema_version") == Some(&Value::from(1)) && keys_are(obj, &PACKET_KEYS, None)
}

fn is_decision(obj: &Map<String, Value>) -> bool {
    match obj.get("schema_version").and_then(Value::as_u64) {
        Some(1) => keys_are(obj, &DECISION_KEYS, None),
        Some(2) => keys_are(obj, &DECISION_KEYS, Some("supersedes")),
        _ => false,
    }
}

/// Resolve every pointer of `p` and apply the class rules (contract 4.2 steps 2-5).
pub(crate) fn resolve_all(p: &PortfolioDecision) -> Upstream {
    let mut up = Upstream::default();
    let h = &p.strategic_basis.handoff;
    // Digest-only (UD4): the handoff is re-hashed, never parsed.
    resolve(
        &mut up.errors,
        &h.path,
        &h.sha256,
        "strategic_basis.handoff",
    );
    supersedes(p, &mut up.errors);
    for (i, m) in p.proposal_set.iter().enumerate() {
        member(i, m, &mut up);
    }
    for (i, e) in p.evaluation_basis.iter().enumerate() {
        evaluation(p, i, e, &mut up.errors);
    }
    for (i, r) in p.readiness_basis.iter().enumerate() {
        readiness(i, r, &mut up.errors);
    }
    up
}

fn supersedes(p: &PortfolioDecision, out: &mut Vec<ErrorItem>) {
    let Some(s) = &p.supersedes else { return };
    let Some(bytes) = resolve(out, &s.path, &s.sha256, "supersedes") else {
        return;
    };
    let bad = |out: &mut Vec<ErrorItem>, message: String| {
        out.push(err(codes::SUPERSESSION_INVALID, "supersedes", message));
    };
    match parse_portfolio(&bytes) {
        Ok(Ok(target)) => {
            if target.portfolio_id != p.portfolio_id {
                bad(
                    out,
                    format!(
                        "the superseded record is portfolio {:?}, not {:?}",
                        target.portfolio_id, p.portfolio_id
                    ),
                );
            } else if p.revision < 2 || target.revision != p.revision - 1 {
                bad(
                    out,
                    format!(
                        "the superseded record is revision {}, expected {}",
                        target.revision,
                        p.revision.saturating_sub(1)
                    ),
                );
            }
        }
        Ok(Err(errors)) => bad(
            out,
            format!(
                "the superseded record is not a valid {PORTFOLIO_SCHEMA} document: {}",
                nested(&errors)
            ),
        ),
        Err(e) => bad(out, format!("the superseded record cannot be parsed: {e}")),
    }
}

fn member(i: usize, m: &SetMember, up: &mut Upstream) {
    let at = format!("proposal_set.{i}");
    let mut state = None;
    if let Some(bytes) = resolve(
        &mut up.errors,
        &m.packet.path,
        &m.packet.sha256,
        &format!("{at}.packet"),
    ) {
        state = packet(i, m, &bytes, up);
    }
    if let Some(state) = state {
        decision_binding(&at, m, &state, &mut up.errors);
    }
    if let Some(d) = &m.decision
        && let Some(bytes) = resolve(
            &mut up.errors,
            &d.path,
            &d.sha256,
            &format!("{at}.decision"),
        )
    {
        decision(&at, m, &bytes, &mut up.errors);
    }
}

/// Class and identity of a bound packet; returns its `lifecycle_state` when the class holds.
fn packet(i: usize, m: &SetMember, bytes: &[u8], up: &mut Upstream) -> Option<String> {
    let at = format!("proposal_set.{i}.packet");
    let Some(obj) = object(bytes).filter(is_packet) else {
        up.errors.push(err(
            codes::UPSTREAM_CLASS_MISMATCH,
            &at,
            "the bound file is not a proposal packet (schema_version 1 with the packet key set)",
        ));
        return None;
    };
    if obj.get("proposal_id").and_then(Value::as_str) != Some(m.proposal_id.as_str()) {
        up.errors.push(err(
            codes::PROPOSAL_ID_MISMATCH,
            &at,
            format!("the packet does not carry proposal_id {:?}", m.proposal_id),
        ));
        return None;
    }
    let state = obj.get("lifecycle_state").and_then(Value::as_str);
    let rels = obj.get("relationships").and_then(Value::as_array).map(|a| {
        a.iter()
            .map(|r| {
                let t = r.get("type").and_then(Value::as_str)?;
                let target = r.get("target_id").and_then(Value::as_str)?;
                Some((t.to_string(), target.to_string()))
            })
            .collect::<Option<Vec<_>>>()
    });
    let (Some(state), Some(Some(relationships))) = (state, rels) else {
        up.errors.push(err(
            codes::SCHEMA,
            &at,
            "the packet's lifecycle_state or relationships cannot be read",
        ));
        return None;
    };
    up.packets
        .entry(m.proposal_id.clone())
        .or_insert(PacketView { relationships });
    Some(state.to_string())
}

/// Erratum 4: `decision` is null iff the packet is `decided`; when bound it is the adjacent
/// `decision.json`.
fn decision_binding(at: &str, m: &SetMember, state: &str, out: &mut Vec<ErrorItem>) {
    let at = format!("{at}.decision");
    let decided = state == DECIDED;
    match (&m.decision, decided) {
        (None, true) => out.push(err(
            codes::DECISION_BINDING,
            &at,
            "the packet is decided, so its adjacent decision.json must be bound",
        )),
        (Some(_), false) => out.push(err(
            codes::DECISION_BINDING,
            &at,
            format!(
                "the packet's lifecycle_state is {state:?}; only a decided packet binds a decision"
            ),
        )),
        (Some(d), true) => {
            let adjacent = std::path::Path::new(&m.packet.path)
                .parent()
                .map(|dir| dir.join("decision.json"));
            if adjacent.as_deref().and_then(|a| a.to_str()) != Some(d.path.as_str()) {
                out.push(err(
                    codes::DECISION_BINDING,
                    &at,
                    "the decision path is not the decision.json adjacent to the packet",
                ));
            }
        }
        (None, false) => {}
    }
}

fn decision(at: &str, m: &SetMember, bytes: &[u8], out: &mut Vec<ErrorItem>) {
    let at = format!("{at}.decision");
    let Some(obj) = object(bytes).filter(is_decision) else {
        out.push(err(
            codes::UPSTREAM_CLASS_MISMATCH,
            &at,
            "the bound file is not a proposal decision (v1 or v2)",
        ));
        return;
    };
    if obj.get("proposal_id").and_then(Value::as_str) != Some(m.proposal_id.as_str()) {
        out.push(err(
            codes::PROPOSAL_ID_MISMATCH,
            &at,
            format!(
                "the decision does not carry proposal_id {:?}",
                m.proposal_id
            ),
        ));
    }
}

fn class_of(bytes: &[u8]) -> Option<String> {
    object(bytes)?.get("schema")?.as_str().map(str::to_string)
}

fn check_bound(
    out: &mut Vec<ErrorItem>,
    p: &PortfolioDecision,
    at: &str,
    proposal_id: &str,
    packet_sha256: &str,
) {
    match p.proposal_set.iter().find(|m| m.proposal_id == proposal_id) {
        Some(m) if m.packet.sha256 == packet_sha256 => {}
        Some(m) => out.push(err(
            codes::BASIS_REVISION_MISMATCH,
            at,
            format!(
                "{proposal_id:?} is bound to packet {packet_sha256}, the portfolio binds {}",
                m.packet.sha256
            ),
        )),
        None => out.push(err(
            codes::BASIS_REVISION_MISMATCH,
            at,
            format!("{proposal_id:?} is not in proposal_set"),
        )),
    }
}

fn failure_item(at: &str, failure: &Failure) -> ErrorItem {
    match failure {
        Failure::Binding { message, errors } => err(
            codes::DIGEST_MISMATCH,
            at,
            format!("{message}: {}", nested(errors)),
        ),
        other => err(codes::EVALUATION_BASIS_INVALID, at, other.to_string()),
    }
}

fn evaluation(p: &PortfolioDecision, i: usize, e: &EvaluationRef, out: &mut Vec<ErrorItem>) {
    let at = format!("evaluation_basis.{i}");
    let Some(bytes) = resolve(out, &e.path, &e.sha256, &at) else {
        return;
    };
    let (want, owner) = match e.kind {
        EvaluationKind::Comparison => (COMPARISON_SCHEMA, "comparison finding"),
        EvaluationKind::EstimateSet => (ESTIMATE_SET_SCHEMA, "estimate set"),
    };
    if class_of(&bytes).as_deref() != Some(want) {
        out.push(err(
            codes::UPSTREAM_CLASS_MISMATCH,
            &at,
            format!("the entry is tagged as a {owner} but the file's schema is not {want:?}"),
        ));
        return;
    }
    match e.kind {
        EvaluationKind::Comparison => comparison(p, &at, &bytes, out),
        EvaluationKind::EstimateSet => estimate_set(p, &at, &bytes, out),
    }
}

fn comparison(p: &PortfolioDecision, at: &str, bytes: &[u8], out: &mut Vec<ErrorItem>) {
    // Re-derivation runs evaluation's own library; no comparison rule is added here (Erratum 9).
    let report = match check_comparison_bytes(bytes, true) {
        Ok(r) => r,
        Err(f) => {
            out.push(failure_item(at, &f));
            return;
        }
    };
    if !report.valid {
        let code = if report
            .errors
            .iter()
            .any(|e| e.code == proposal_evaluation::codes::REDERIVE_MISMATCH)
        {
            codes::EVALUATION_BASIS_REDERIVE_MISMATCH
        } else {
            codes::EVALUATION_BASIS_INVALID
        };
        out.push(err(
            code,
            at,
            format!("comparison check reported: {}", nested(&report.errors)),
        ));
        return;
    }
    // The contract at the finding's stored path (digest already checked by the re-derivation),
    // not `estimate_sets[]`, supplies the proposals and their packet revisions.
    let Ok(finding) = serde_json::from_slice::<ComparisonFinding>(bytes) else {
        out.push(err(
            codes::EVALUATION_BASIS_INVALID,
            at,
            "the finding does not parse",
        ));
        return;
    };
    let Some(contract_bytes) = resolve(
        out,
        &finding.contract.path,
        &finding.contract.sha256,
        &format!("{at}.contract"),
    ) else {
        return;
    };
    match parse_contract(&contract_bytes) {
        Ok(ParsedContract::Contract(c)) => {
            for cp in &c.proposals {
                check_bound(out, p, at, &cp.proposal_id, &cp.packet_sha256);
            }
        }
        Ok(ParsedContract::Invalid(errors)) => out.push(err(
            codes::EVALUATION_BASIS_INVALID,
            at,
            format!("the finding's contract is invalid: {}", nested(&errors)),
        )),
        Err(f) => out.push(failure_item(at, &f)),
    }
}

fn estimate_set(p: &PortfolioDecision, at: &str, bytes: &[u8], out: &mut Vec<ErrorItem>) {
    let report = match check_estimate_set_bytes(bytes) {
        Ok(r) => r,
        Err(f) => {
            out.push(failure_item(at, &f));
            return;
        }
    };
    if !report.valid {
        out.push(err(
            codes::EVALUATION_BASIS_INVALID,
            at,
            format!("estimate-set check reported: {}", nested(&report.errors)),
        ));
        return;
    }
    match serde_json::from_slice::<EstimateSet>(bytes) {
        Ok(set) => check_bound(
            out,
            p,
            at,
            &set.proposal.proposal_id,
            &set.proposal.packet.sha256,
        ),
        Err(e) => out.push(err(codes::EVALUATION_BASIS_INVALID, at, e.to_string())),
    }
}

fn readiness(i: usize, r: &ReadinessRef, out: &mut Vec<ErrorItem>) {
    let at = format!("readiness_basis.{i}");
    let a = &r.assessment;
    let Some(bytes) = resolve(out, &a.path, &a.sha256, &format!("{at}.assessment")) else {
        return;
    };
    if class_of(&bytes).as_deref() != Some(ASSESSMENT_SCHEMA) {
        out.push(err(
            codes::UPSTREAM_CLASS_MISMATCH,
            format!("{at}.assessment"),
            format!("the file's schema is not {ASSESSMENT_SCHEMA:?}"),
        ));
        return;
    }
    // `check_assessment` is not re-run: it needs the profile file the assessment does not locate.
    let assessment = match serde_json::from_slice::<Assessment>(&bytes) {
        Ok(a) => a,
        Err(e) => {
            out.push(err(
                codes::SCHEMA,
                format!("{at}.assessment"),
                e.to_string(),
            ));
            return;
        }
    };
    if assessment.purpose != Purpose::Live {
        out.push(err(
            codes::ASSESSMENT_NOT_LIVE,
            format!("{at}.assessment"),
            format!(
                "the assessment's purpose is {:?}, not live",
                assessment.purpose
            ),
        ));
    }
    if assessment.decision.id != r.decision_id {
        out.push(err(
            codes::READINESS_DECISION_MISMATCH,
            format!("{at}.decision_id"),
            format!(
                "the assessment serves decision {:?}, the entry names {:?}",
                assessment.decision.id, r.decision_id
            ),
        ));
    }
    let ev = &assessment.evaluation;
    resolve(
        out,
        &ev.path,
        &ev.sha256,
        &format!("{at}.assessment.evaluation"),
    );
}
