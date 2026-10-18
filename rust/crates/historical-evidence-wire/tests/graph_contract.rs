//! Run with S5_GRAPH_REPLY and S5_V2_REPLY set to private, owner-qualified
//! fixture paths. The repository never embeds historical provider captures.
use historical_evidence_wire::{ReplyV2, graph};
use std::{env, fs};

fn fixtures() -> (graph::Observation, historical_evidence_wire::ObservationV2) {
    let graph_path =
        env::var("S5_GRAPH_REPLY").expect("set S5_GRAPH_REPLY to a qualified private frame");
    let v2_path =
        env::var("S5_V2_REPLY").expect("set S5_V2_REPLY to a distinct admitted grant frame");
    let graph =
        match serde_json::from_slice::<graph::Reply>(&fs::read(graph_path).unwrap()).unwrap() {
            graph::Reply::Observed { observation } => observation,
            _ => panic!("graph fixture must be observed"),
        };
    let v2 = match serde_json::from_slice::<ReplyV2>(&fs::read(v2_path).unwrap()).unwrap() {
        ReplyV2::Observed { observation } => observation,
        _ => panic!("v2 fixture must be observed"),
    };
    (graph, v2)
}

#[test]
#[ignore = "requires two private owner-qualified response fixtures"]
fn exact_graph_and_stable_ids_across_grants() {
    let (graph, prior) = fixtures();
    graph.validate().unwrap();
    assert_ne!(
        graph.base.source_identity_sha256,
        prior.source_identity_sha256
    );
    let other = graph::Observation::from_base(prior).unwrap();
    assert_eq!(
        graph.nodes.iter().map(|n| &n.id).collect::<Vec<_>>(),
        other.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
    );
    assert_eq!(
        graph.edges.iter().map(|e| &e.id).collect::<Vec<_>>(),
        other.edges.iter().map(|e| &e.id).collect::<Vec<_>>()
    );
    assert_eq!(graph.nodes.len(), 62);
    assert_eq!(graph.edges.len(), 67);
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|n| n.kind == "review_revision")
            .count(),
        14
    );
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|n| n.kind == "provider_result")
            .count(),
        8
    );
    let authority = graph
        .edges
        .iter()
        .find(|e| e.kind == "replaces_reported_episode")
        .unwrap();
    let origin = graph.nodes.iter().find(|n| n.id == authority.from).unwrap();
    assert_eq!(
        origin.source_revision,
        "24f3f38e5351b48bcd3e069978b22a597f590e82"
    );
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.kind == "replaces_reported_episode" && e.from.contains("c02c6bf"))
    );
    assert!(
        graph
            .gaps
            .iter()
            .any(|gap| gap.contains("not a UUID-to-schedule causal edge"))
    );
}

#[test]
#[ignore = "requires two private owner-qualified response fixtures"]
fn altered_graphs_are_refused_and_snapshot_identity_is_scoped() {
    let (graph, _) = fixtures();
    let mut changed = graph.clone();
    changed
        .edges
        .iter_mut()
        .find(|e| e.kind == "replaces_reported_episode")
        .unwrap()
        .from = changed
        .nodes
        .iter()
        .find(|n| n.source_revision.starts_with("c02c6bf"))
        .unwrap()
        .id
        .clone();
    assert!(
        changed.validate().is_err(),
        "candidate-three authority forgery"
    );
    let mut changed = graph.clone();
    changed.edges[0].to = "missing-node".into();
    assert!(changed.validate().is_err(), "dangling endpoint");
    let mut changed = graph.clone();
    changed.nodes.push(changed.nodes[0].clone());
    assert!(changed.validate().is_err(), "duplicate node");
    let mut changed = graph.clone();
    changed.edges[0].owner = "role-scheduler".into();
    assert!(changed.validate().is_err(), "wrong edge owner");
    let mut changed = graph.clone();
    changed.base.base.candidates[4].gate_digest = "0".repeat(64);
    assert!(changed.validate().is_err(), "wrong checkpoint gate digest");
    let mut changed = graph.base.clone();
    changed.base.candidates[4]
        .gate
        .as_mut()
        .unwrap()
        .source_kind = "slice_builder_runner_output".into();
    assert!(
        graph::Observation::from_base(changed).is_err(),
        "wrong capture schema"
    );
    let mut changed = graph.base.clone();
    changed.base.reviews[2].revisions[1].predecessor_revision = Some("0".repeat(40));
    assert!(
        graph::Observation::from_base(changed).is_err(),
        "altered predecessor"
    );
    let mut changed = graph.base.clone();
    changed.base.reviews[2].revisions[1].subjects[0].owner = "other-owner".into();
    assert!(
        graph::Observation::from_base(changed).is_err(),
        "wrong subject owner"
    );
    let mut changed = graph.base.clone();
    changed.base.reviews[2].revisions[1].writer_generation = 2;
    assert!(
        graph::Observation::from_base(changed).is_err(),
        "incompatible writer generation"
    );
    let mut value = serde_json::to_value(&graph::Reply::Observed {
        observation: graph.clone(),
    })
    .unwrap();
    value["observation"]["nodes"][0]["evidence_class"] = "oracle".into();
    assert!(
        serde_json::from_value::<graph::Reply>(value).is_err(),
        "unknown evidence class"
    );

    let mut changed_base = graph.base.clone();
    changed_base.scheduler.row_sha256 = "a".repeat(64);
    changed_base.source_reads[15].identity = format!(
        "{}@{}:{}",
        changed_base.scheduler.item_id,
        changed_base.scheduler.store_revision,
        changed_base.scheduler.row_sha256
    );
    let next = graph::Observation::from_base(changed_base).unwrap();
    for (left, right) in graph.nodes.iter().zip(next.nodes.iter()) {
        if left.kind == "scheduler_current_row" {
            assert_ne!(left.id, right.id);
        } else {
            assert_eq!(left.id, right.id);
        }
    }
    for (left, right) in graph.edges.iter().zip(next.edges.iter()) {
        if left.kind == "refers_to_review_result" {
            assert_ne!(left.id, right.id);
        } else {
            assert_eq!(left.id, right.id);
        }
    }

    let mut no_metrics = graph.base.clone();
    no_metrics.metrics = None;
    no_metrics.metrics_unavailable =
        Some("Frozen metrics source unavailable under this read".into());
    let source = no_metrics.source_reads.last_mut().unwrap();
    source.identity = "frozen metrics row unavailable under this read".into();
    source.integrity = "unavailable".into();
    source.completeness = "no_frozen_metrics_comparison".into();
    source.confidentiality = "no_metric_values_projected".into();
    source.limit = no_metrics.metrics_unavailable.clone().unwrap();
    let omitted = graph::Observation::from_base(no_metrics).unwrap();
    omitted.validate().unwrap();
    assert!(!omitted.nodes.iter().any(|n| matches!(
        n.kind.as_str(),
        "frozen_metrics_row" | "measurement_discrepancy"
    )));
    assert!(
        omitted
            .gaps
            .iter()
            .any(|gap| gap.contains("Optional frozen metrics row unavailable"))
    );
}

#[test]
#[ignore = "requires a private owner-qualified graph response fixture"]
fn coordinated_base_and_projection_weakening_is_refused() {
    let (graph, _) = fixtures();
    let mut missing_authority = graph.clone();
    missing_authority.base.base.replacement_links.clear();
    missing_authority
        .edges
        .retain(|edge| edge.kind != "replaces_reported_episode");
    assert!(missing_authority.validate().is_err());

    let mut public_source = graph.clone();
    public_source.base.source_reads[0].confidentiality = "public".into();
    public_source.base.base.source_reads[0].confidentiality = "public".into();
    assert!(public_source.validate().is_err());

    let mut invalid_tree = graph.clone();
    invalid_tree.base.base.candidates[0].tree_oid = "file:///private/not-an-oid".into();
    assert!(invalid_tree.validate().is_err());

    let mut missing_gate = graph.clone();
    missing_gate.base.base.candidates[0].gate = None;
    missing_gate
        .nodes
        .retain(|node| node.kind != "gate_capture" || node.local_id != "candidate-1");
    missing_gate
        .edges
        .retain(|edge| !edge.id.contains("has_gate_digest|c1@"));
    assert!(missing_gate.validate().is_err());

    let mut altered_ref = graph.clone();
    altered_ref.base.base.candidates[0].checkpoint_ref = "file:///private/checkpoint".into();
    altered_ref.base.source_reads[1].identity = format!(
        "{}@{}",
        altered_ref.base.base.candidates[0].checkpoint_ref,
        altered_ref.base.base.candidates[0].commit_oid
    );
    altered_ref.base.base.source_reads[1].identity =
        altered_ref.base.source_reads[1].identity.clone();
    assert!(altered_ref.validate().is_err());
}

#[test]
#[ignore = "requires a private owner-qualified graph response fixture"]
fn malformed_review_cardinalities_refuse_without_panicking() {
    let (graph, _) = fixtures();
    for episode in 0..3 {
        for variant in ["empty", "truncated", "oversized"] {
            let mut changed = graph.clone();
            let revisions = &mut changed.base.base.reviews[episode].revisions;
            match variant {
                "empty" => revisions.clear(),
                "truncated" => {
                    revisions.pop();
                }
                "oversized" => revisions.push(revisions.last().unwrap().clone()),
                _ => unreachable!(),
            }
            let result = std::panic::catch_unwind(|| changed.validate());
            assert!(
                matches!(result, Ok(Err(_))),
                "episode {episode} with {variant} revisions must refuse without panic"
            );
        }
    }
}
