//! Private source-backed qualification: S5_GRAPH_V2_REPLY is a full admitted frame.
use historical_evidence_wire::{ORIGINAL_REPORTED_REVISION, REPAIR_COMMIT_OID, graph_v2};

fn fixture() -> graph_v2::Observation {
    let path = std::env::var("S5_GRAPH_V2_REPLY").expect("qualified graph v2 frame path");
    let bytes = std::fs::read(path).unwrap();
    match serde_json::from_slice::<graph_v2::Reply>(&bytes).unwrap() {
        graph_v2::Reply::Observed { observation } => observation,
        _ => panic!("fixture must be an admitted graph v2 observation"),
    }
}

#[test]
#[ignore = "requires S5_GRAPH_V2_REPLY from a private owner-qualified read"]
fn exact_source_and_interpreter_are_both_required() {
    let graph = fixture();
    graph.validate().unwrap();
    assert_eq!(graph.repair.commit_oid, REPAIR_COMMIT_OID);
    assert_eq!(graph.nodes.len(), 64);
    assert_eq!(graph.edges.len(), 69);
    let edge = graph
        .edges
        .iter()
        .find(|e| e.kind == "clarifies_reported_continuation_for")
        .unwrap();
    let source = &graph.edge_sources[edge.edge_source_index];
    assert_eq!(source.owner, graph_v2::INTERPRETER);
    assert_eq!(source.source_revision, graph_v2::RULE_REVISION);
    assert!(
        edge.id
            .ends_with(&format!("|rule@{}", graph_v2::RULE_REVISION))
    );
    assert_eq!(
        edge.support.as_ref().unwrap().review_revision,
        ORIGINAL_REPORTED_REVISION
    );
    assert_eq!(graph.repair_source.confidentiality, "private_local");

    let mut wrong = graph.clone();
    wrong.repair.script_after_blob_oid.replace_range(..1, "0");
    assert!(wrong.validate().is_err());
    let mut wrong = graph.clone();
    wrong.interpreter_rule.source_revision = "v2@historical_s5.graph.v2".into();
    assert!(wrong.validate().is_err());
    let mut wrong = graph.clone();
    let index = wrong
        .edges
        .iter()
        .position(|e| e.kind == "clarifies_reported_continuation_for")
        .unwrap();
    wrong.edges[index].support.as_mut().unwrap().review_revision = "0".repeat(40);
    assert!(wrong.validate().is_err());
    let mut wrong = graph.clone();
    wrong.edges[index].support = None;
    assert!(wrong.validate().is_err());
    let mut wrong = graph.clone();
    wrong.edges[index].edge_source_index = usize::MAX;
    assert!(wrong.validate().is_err());
    let mut wrong = graph.clone();
    wrong.edge_sources.push(wrong.edge_sources[0].clone());
    assert!(wrong.validate().is_err());
    let mut wrong = graph.clone();
    wrong.edge_sources[edge.edge_source_index].owner = "independent-review-state".into();
    assert!(wrong.validate().is_err());
    let mut wrong = graph.clone();
    wrong.edges[index].evidence_class = historical_evidence_wire::graph::EvidenceClass::Observed;
    assert!(wrong.validate().is_err());
    let mut wrong = graph.clone();
    wrong.repair.source_class = "unqualified_contract_source".into();
    assert!(wrong.validate().is_err());
    let mut wrong = graph.clone();
    wrong.base.base.reviews[0].revisions.clear();
    assert!(wrong.validate().is_err());
    let mut wrong = graph.clone();
    wrong.base.base.reviews[0]
        .revisions
        .iter_mut()
        .find(|r| r.revision == ORIGINAL_REPORTED_REVISION)
        .unwrap()
        .phase = "remediation".into();
    assert!(wrong.validate().is_err());
    let mut wrong = graph.clone();
    wrong.repair_source.confidentiality = "public".into();
    assert!(wrong.validate().is_err());
    let mut wrong = graph.clone();
    wrong.gaps.clear();
    assert!(wrong.validate().is_err());
}

#[test]
#[ignore = "requires S5_GRAPH_V2_REPLY from a private owner-qualified read"]
fn stable_historical_ids_ignore_fresh_grant_and_read_anchor() {
    let graph = fixture();
    let mut base = graph.base.clone();
    base.base.grant_id = "another-fresh-equivalent-local-grant".into();
    base.base.cutoff = "2026-10-18T05:55:00+00:00".into();
    base.base.read_at = "unix:1792302900".into();
    base.base.source_identity_sha256 = "a".repeat(64);
    base.source_identity_sha256 = "b".repeat(64);
    let other = graph_v2::Observation::from_base(base, graph.repair.clone()).unwrap();
    assert_eq!(
        graph.nodes.iter().map(|n| &n.id).collect::<Vec<_>>(),
        other.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
    );
    assert_eq!(
        graph.edges.iter().map(|e| &e.id).collect::<Vec<_>>(),
        other.edges.iter().map(|e| &e.id).collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "requires S5_GRAPH_V2_REPLY from a private owner-qualified read"]
fn optional_metrics_absence_keeps_repair_and_frame_bound() {
    let graph = fixture();
    let mut base = graph.base.clone();
    base.metrics = None;
    base.metrics_unavailable = Some("Frozen metrics source unavailable under this read".into());
    let source = base.source_reads.last_mut().unwrap();
    source.identity = "frozen metrics row unavailable under this read".into();
    source.integrity = "unavailable".into();
    source.completeness = "no_frozen_metrics_comparison".into();
    source.confidentiality = "no_metric_values_projected".into();
    source.limit = base.metrics_unavailable.clone().unwrap();
    let omitted = graph_v2::Observation::from_base(base, graph.repair.clone()).unwrap();
    omitted.validate().unwrap();
    assert!(!omitted.nodes.iter().any(|n| matches!(
        n.kind.as_str(),
        "frozen_metrics_row" | "measurement_discrepancy"
    )));
    assert!(omitted.nodes.iter().any(|n| n.kind == "contract_revision"));
    let frame = serde_json::to_vec(&graph_v2::Reply::Observed {
        observation: omitted,
    })
    .unwrap();
    assert!(frame.len() <= historical_evidence_wire::MAX_FRAME);
    println!("METRICS_UNAVAILABLE_GRAPH_V2_FRAME_BYTES={}", frame.len());
    if let Ok(path) = std::env::var("S5_GRAPH_V2_METRICS_UNAVAILABLE_OUT") {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .unwrap();
        file.write_all(&frame).unwrap();
    }
}
