//! Cross-proposal rules (contract 4.1 Unlock and Mutual exclusion, errata 5 and 7).
//!
//! An unlock edge is checked against the relationships found in the bound packet bytes of its
//! `source`; a mutual-exclusion entry is checked against the record's own dispositions. Nothing
//! here infers a disposition or reads the relationship graph beyond the bound packets.

use crate::diag::{ErrorItem, codes};
use crate::model::{Disposition, MutualExclusion, PortfolioDecision, Unlock};
use crate::structure::{err, non_empty};
use crate::upstream::PacketView;
use std::collections::{BTreeMap, BTreeSet};

/// The relationship types an unlock may rest on (graph types, contract 4.1).
const UNLOCK_TYPES: [&str; 3] = ["depends_on", "enables", "informs"];

/// Apply the unlock and exclusion rules. `packets` holds the packets that resolved; a member
/// whose packet could not be read is reported by the pointer rules, so its edges are not also
/// reported absent (an unread packet is `not established`, not empty).
pub(crate) fn check(
    p: &PortfolioDecision,
    packets: &BTreeMap<String, PacketView>,
) -> Vec<ErrorItem> {
    let mut out = Vec::new();
    let ids: BTreeSet<&str> = p
        .proposal_set
        .iter()
        .map(|m| m.proposal_id.as_str())
        .collect();
    let analysis = &p.cross_proposal_analysis;
    for (i, unlock) in analysis.unlock.iter().enumerate() {
        check_unlock(&mut out, unlock, i, &ids, packets);
    }
    for (k, exclusion) in analysis.mutually_exclusive_under_basis.iter().enumerate() {
        check_exclusion(&mut out, exclusion, k, &ids, &p.dispositions);
    }
    out
}

fn check_unlock(
    out: &mut Vec<ErrorItem>,
    unlock: &Unlock,
    i: usize,
    ids: &BTreeSet<&str>,
    packets: &BTreeMap<String, PacketView>,
) {
    let base = format!("cross_proposal_analysis.unlock.{i}");
    for (name, id) in [("from", &unlock.from), ("to", &unlock.to)] {
        if !ids.contains(id.as_str()) {
            out.push(err(
                codes::UNLOCK_ENDPOINT_UNKNOWN,
                format!("{base}.{name}"),
                format!("`{id}` is not in proposal_set"),
            ));
        }
    }
    non_empty(out, &unlock.judgment, &format!("{base}.judgment"));
    non_empty(out, &unlock.attributed_to, &format!("{base}.attributed_to"));
    if unlock.relationship_basis.is_empty() {
        out.push(err(
            codes::SCHEMA,
            format!("{base}.relationship_basis"),
            "an unlock needs at least one relationship_basis edge",
        ));
    }
    let ends = unordered(&unlock.from, &unlock.to);
    for (j, edge) in unlock.relationship_basis.iter().enumerate() {
        let at = format!("{base}.relationship_basis.{j}");
        if !UNLOCK_TYPES.contains(&edge.edge_type.as_str()) {
            out.push(err(
                codes::UNLOCK_TYPE_INVALID,
                format!("{at}.type"),
                format!(
                    "`{}` is not one of depends_on, enables, informs",
                    edge.edge_type
                ),
            ));
        }
        if !ids.contains(edge.source.as_str()) {
            out.push(err(
                codes::RELATIONSHIP_BASIS_ABSENT,
                &at,
                format!("source `{}` is not in proposal_set", edge.source),
            ));
        } else if let Some(packet) = packets.get(&edge.source) {
            let found = packet
                .relationships
                .iter()
                .any(|(kind, target)| *kind == edge.edge_type && *target == edge.target);
            if !found {
                out.push(err(
                    codes::RELATIONSHIP_BASIS_ABSENT,
                    &at,
                    format!(
                        "the packet of `{}` has no relationship `{}` to `{}`",
                        edge.source, edge.edge_type, edge.target
                    ),
                ));
            }
        }
        if unordered(&edge.source, &edge.target) != ends {
            out.push(err(
                codes::RELATIONSHIP_BASIS_DISCONNECTED,
                &at,
                format!(
                    "the edge `{}` -> `{}` does not connect from `{}` and to `{}`",
                    edge.source, edge.target, unlock.from, unlock.to
                ),
            ));
        }
    }
}

fn check_exclusion(
    out: &mut Vec<ErrorItem>,
    exclusion: &MutualExclusion,
    k: usize,
    ids: &BTreeSet<&str>,
    dispositions: &[Disposition],
) {
    let base = format!("cross_proposal_analysis.mutually_exclusive_under_basis.{k}");
    non_empty(out, &exclusion.reason, &format!("{base}.reason"));
    non_empty(
        out,
        &exclusion.attributed_to,
        &format!("{base}.attributed_to"),
    );
    if exclusion.proposals.len() < 2 {
        out.push(err(
            codes::EXCLUSION_MEMBER_INVALID,
            format!("{base}.proposals"),
            "a mutual exclusion names at least two proposals",
        ));
    }
    let mut seen = BTreeSet::new();
    for (m, id) in exclusion.proposals.iter().enumerate() {
        let at = format!("{base}.proposals.{m}");
        if !ids.contains(id.as_str()) {
            out.push(err(
                codes::EXCLUSION_MEMBER_INVALID,
                at,
                format!("`{id}` is not in proposal_set"),
            ));
        } else if !seen.insert(id.as_str()) {
            out.push(err(
                codes::EXCLUSION_MEMBER_INVALID,
                at,
                format!("`{id}` is listed more than once"),
            ));
        }
    }
    // Erratum 7: the record is checked against itself; no disposition is inferred.
    let selected: BTreeSet<&str> = dispositions
        .iter()
        .filter(|d| matches!(d, Disposition::SelectForCampaign { .. }))
        .map(Disposition::proposal_id)
        .filter(|id| seen.contains(id))
        .collect();
    if selected.len() >= 2 {
        out.push(err(
            codes::EXCLUSION_CONTRADICTED,
            base,
            format!(
                "{} members carry select_for_campaign but cannot be selected together",
                selected.len()
            ),
        ));
    }
}

fn unordered<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    if a <= b { (a, b) } else { (b, a) }
}
