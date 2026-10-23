//! Structural rules of a `portfolio-decision-v1` record (contract 4.1, S4-S5, S10).
//!
//! Everything here is decided from the record alone: no file is read and no clock is consulted.
//! Pointer resolution lives in [`crate::upstream`] and the cross-proposal rules that need packet
//! bytes live in [`crate::cross`]. Every violated rule is reported with its one code.

use crate::diag::{ErrorItem, codes, limitations};
use crate::model::*;
use proposal_evaluation::diag::Failure;
use proposal_evaluation::model::FileRef;
use proposal_evaluation::scan::{locate_data_error, scan};
use proposal_evaluation::validate::{check_file_ref, is_proposal_id, is_set_id, is_utc_timestamp};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn err(code: &str, at: impl Into<String>, message: impl Into<String>) -> ErrorItem {
    ErrorItem::new(code, at, message)
}

pub(crate) fn non_empty(out: &mut Vec<ErrorItem>, text: &str, at: &str) {
    if text.is_empty() {
        out.push(err(codes::EMPTY_STRING, at, "must be a non-empty string"));
    }
}

/// Parse `bytes`. Not JSON at all is a `Failure` (exit 2); JSON that breaks the closed schema is
/// reported as an invalid document (exit 1). Lexical findings (duplicate keys, a number out of
/// range) come first, then the `schema` const, then the closed schema (contract S4).
pub fn parse_portfolio(bytes: &[u8]) -> Result<Result<PortfolioDecision, Vec<ErrorItem>>, Failure> {
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
        Some(serde_json::Value::String(s)) if s == PORTFOLIO_SCHEMA => {}
        Some(serde_json::Value::String(s)) => {
            return Ok(Err(vec![err(
                codes::SCHEMA_MISMATCH,
                "schema",
                format!("schema is {s:?}, expected {PORTFOLIO_SCHEMA:?}"),
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
    match serde_json::from_slice::<PortfolioDecision>(bytes) {
        Ok(p) => Ok(Ok(p)),
        Err(e) if e.classify() == serde_json::error::Category::Data => Ok(Err(vec![err(
            codes::SCHEMA,
            locate_data_error(bytes, &e),
            e.to_string(),
        )])),
        Err(e) => Err(Failure::Parse(e.to_string())),
    }
}

fn file_ref_shape(out: &mut Vec<ErrorItem>, f: &FileRef, at: &str) {
    check_file_ref(out, f, at);
}

/// Every structural rule, in the order of contract 4.1.
pub fn validate(p: &PortfolioDecision) -> Vec<ErrorItem> {
    let mut out = Vec::new();
    identity(p, &mut out);
    let ids: BTreeSet<&str> = p
        .proposal_set
        .iter()
        .map(|m| m.proposal_id.as_str())
        .collect();
    bases(p, &ids, &mut out);
    dispositions(p, &ids, &mut out);
    non_effects(&p.non_effects, &mut out);
    required_limitations(p, &mut out);
    out
}

fn identity(p: &PortfolioDecision, out: &mut Vec<ErrorItem>) {
    if !is_set_id(&p.portfolio_id) {
        out.push(err(
            codes::ID_PATTERN,
            "portfolio_id",
            "portfolio_id must match ^[a-z0-9]+(?:[.-][a-z0-9]+)*$",
        ));
    }
    if p.revision == 0 {
        out.push(err(
            codes::SCHEMA,
            "revision",
            "revision must be at least 1",
        ));
    }
    match (&p.supersedes, p.revision) {
        (Some(_), 1) => out.push(err(
            codes::SUPERSESSION_INVALID,
            "supersedes",
            "revision 1 supersedes nothing: supersedes must be null",
        )),
        (None, r) if r > 1 => out.push(err(
            codes::SUPERSESSION_INVALID,
            "supersedes",
            format!("revision {r} must name the revision it supersedes"),
        )),
        _ => {}
    }
    if let Some(s) = &p.supersedes {
        file_ref_shape(out, s, "supersedes");
    }
    if !is_utc_timestamp(&p.decided_at) {
        out.push(err(
            codes::TIME_FORMAT,
            "decided_at",
            format!("{:?} is not YYYY-MM-DDTHH:MM:SSZ", p.decided_at),
        ));
    }
    non_empty(out, &p.authority.decision_owner, "authority.decision_owner");
    non_empty(out, &p.authority.exercised_by, "authority.exercised_by");
    non_empty(out, &p.authority.evidence, "authority.evidence");
    file_ref_shape(out, &p.strategic_basis.handoff, "strategic_basis.handoff");
    let rev = &p.relationship_graph.repository_revision;
    if rev.len() != 40 || !rev.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        out.push(err(
            codes::SCHEMA,
            "relationship_graph.repository_revision",
            "repository_revision must be 40 lowercase hex characters",
        ));
    }
}

fn bases(p: &PortfolioDecision, ids: &BTreeSet<&str>, out: &mut Vec<ErrorItem>) {
    if p.proposal_set.is_empty() {
        out.push(err(
            codes::SCHEMA,
            "proposal_set",
            "proposal_set needs at least one member",
        ));
    }
    let mut seen = BTreeSet::new();
    for (i, m) in p.proposal_set.iter().enumerate() {
        let at = format!("proposal_set.{i}");
        if !is_proposal_id(&m.proposal_id) {
            out.push(err(
                codes::ID_PATTERN,
                format!("{at}.proposal_id"),
                "proposal_id must match ^[a-z0-9]+(?:[.-][a-z0-9]+)+$",
            ));
        }
        if !seen.insert(m.proposal_id.as_str()) {
            out.push(err(
                codes::DUPLICATE_PROPOSAL_ID,
                format!("{at}.proposal_id"),
                format!("{:?} appears more than once in proposal_set", m.proposal_id),
            ));
        }
        file_ref_shape(out, &m.packet, &format!("{at}.packet"));
        if let Some(d) = &m.decision {
            file_ref_shape(out, d, &format!("{at}.decision"));
        }
    }
    for (i, e) in p.evaluation_basis.iter().enumerate() {
        file_ref_shape(out, &e.file_ref(), &format!("evaluation_basis.{i}"));
    }
    for (i, r) in p.readiness_basis.iter().enumerate() {
        let at = format!("readiness_basis.{i}");
        if !ids.contains(r.proposal_id.as_str()) {
            out.push(err(
                codes::READINESS_PROPOSAL_UNKNOWN,
                format!("{at}.proposal_id"),
                format!("{:?} is not in proposal_set", r.proposal_id),
            ));
        }
        non_empty(out, &r.decision_id, &format!("{at}.decision_id"));
        file_ref_shape(out, &r.assessment, &format!("{at}.assessment"));
    }
}

fn dispositions(p: &PortfolioDecision, ids: &BTreeSet<&str>, out: &mut Vec<ErrorItem>) {
    let mut first: BTreeMap<&str, usize> = BTreeMap::new();
    let mut ranks: BTreeMap<u32, usize> = BTreeMap::new();
    for (i, d) in p.dispositions.iter().enumerate() {
        let at = format!("dispositions.{i}");
        let id = d.proposal_id();
        if !ids.contains(id) {
            out.push(err(
                codes::DISPOSITION_UNKNOWN_PROPOSAL,
                format!("{at}.proposal_id"),
                format!("{id:?} is not in proposal_set"),
            ));
        } else if let Some(prior) = first.insert(id, i) {
            out.push(err(
                codes::DISPOSITION_DUPLICATE,
                format!("{at}.proposal_id"),
                format!("{id:?} already has a disposition at dispositions.{prior}"),
            ));
        }
        non_empty(out, d.rationale(), &format!("{at}.rationale"));
        match d {
            Disposition::Prioritize { rank, .. } => {
                if *rank == 0 {
                    out.push(err(
                        codes::SCHEMA,
                        format!("{at}.rank"),
                        "rank must be at least 1",
                    ));
                } else if let Some(prior) = ranks.insert(*rank, i) {
                    out.push(err(
                        codes::RANK_DUPLICATE,
                        format!("{at}.rank"),
                        format!("rank {rank} is already used at dispositions.{prior}"),
                    ));
                }
            }
            Disposition::Sequence { after, .. } => {
                if after.is_empty() {
                    out.push(err(
                        codes::SEQUENCE_TARGET_INVALID,
                        format!("{at}.after"),
                        "after needs at least one proposal",
                    ));
                }
                for (j, target) in after.iter().enumerate() {
                    if target == id {
                        out.push(err(
                            codes::SEQUENCE_TARGET_INVALID,
                            format!("{at}.after.{j}"),
                            format!("{id:?} cannot be sequenced after itself"),
                        ));
                    } else if !ids.contains(target.as_str()) {
                        out.push(err(
                            codes::SEQUENCE_TARGET_INVALID,
                            format!("{at}.after.{j}"),
                            format!("{target:?} is not in proposal_set"),
                        ));
                    }
                }
            }
            Disposition::Defer { revisit_when, .. } => {
                if revisit_when.is_empty() {
                    out.push(err(
                        codes::SCHEMA,
                        format!("{at}.revisit_when"),
                        "revisit_when needs at least one condition",
                    ));
                }
                for (j, c) in revisit_when.iter().enumerate() {
                    non_empty(out, c, &format!("{at}.revisit_when.{j}"));
                }
            }
            Disposition::ExcludeFromCurrentPortfolio { reason, .. } => {
                non_empty(out, reason, &format!("{at}.reason"));
            }
            Disposition::SelectForCampaign { campaign_ref, .. } => {
                non_empty(out, campaign_ref, &format!("{at}.campaign_ref"));
            }
        }
    }
    for (i, m) in p.proposal_set.iter().enumerate() {
        if !first.contains_key(m.proposal_id.as_str()) {
            out.push(err(
                codes::DISPOSITION_MISSING,
                format!("proposal_set.{i}"),
                format!("{:?} has no disposition", m.proposal_id),
            ));
        }
    }
    sequence_cycles(p, ids, out);
}

/// `sequence_cycle`: report at every `sequence` disposition whose proposal can reach itself
/// through `after` edges. Edges to unknown proposals and self edges are other codes' business.
fn sequence_cycles(p: &PortfolioDecision, ids: &BTreeSet<&str>, out: &mut Vec<ErrorItem>) {
    let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for d in &p.dispositions {
        if let Disposition::Sequence {
            proposal_id, after, ..
        } = d
        {
            let list = edges.entry(proposal_id.as_str()).or_default();
            for t in after {
                if t != proposal_id && ids.contains(t.as_str()) {
                    list.push(t.as_str());
                }
            }
        }
    }
    for (i, d) in p.dispositions.iter().enumerate() {
        let Disposition::Sequence { proposal_id, .. } = d else {
            continue;
        };
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut stack: Vec<&str> = edges.get(proposal_id.as_str()).cloned().unwrap_or_default();
        let mut cyclic = false;
        while let Some(n) = stack.pop() {
            if n == proposal_id {
                cyclic = true;
                break;
            }
            if seen.insert(n) {
                stack.extend(edges.get(n).cloned().unwrap_or_default());
            }
        }
        if cyclic {
            out.push(err(
                codes::SEQUENCE_CYCLE,
                format!("dispositions.{i}"),
                format!("{proposal_id:?} is sequenced (transitively) after itself"),
            ));
        }
    }
}

fn non_effects(n: &NonEffects, out: &mut Vec<ErrorItem>) {
    for (name, value) in n.fields() {
        if value {
            out.push(err(
                codes::AUTHORITY_ASSERTED,
                format!("non_effects.{name}"),
                format!("{name} must be false: a portfolio decision informs and effects nothing"),
            ));
        }
    }
}

fn required_limitations(p: &PortfolioDecision, out: &mut Vec<ErrorItem>) {
    for (i, l) in p.limitations.iter().enumerate() {
        non_empty(out, l, &format!("limitations.{i}"));
    }
    // A limitation names itself before its first ':' (or is only that name); the name must equal
    // the required one, so `strategic_basis_unverified_is_false_here` does not satisfy it.
    let has = |name: &str| {
        p.limitations
            .iter()
            .any(|l| l.split_once(':').map_or(l.as_str(), |(head, _)| head) == name)
    };
    let mut need = vec![
        limitations::STRATEGIC_BASIS_UNVERIFIED,
        limitations::RELATIONSHIP_GRAPH_REVISION_UNVERIFIED,
    ];
    if p.evaluation_basis.is_empty() {
        need.push(limitations::NO_EVALUATION_BASIS);
    }
    if !p.readiness_basis.is_empty() {
        need.push(limitations::READINESS_DECISION_FIT_UNVERIFIED);
    }
    for name in need {
        if !has(name) {
            out.push(err(
                codes::LIMITATION_MISSING,
                "limitations",
                format!("no limitation is named {name:?} (the text before its first ':' must equal the name)"),
            ));
        }
    }
}
