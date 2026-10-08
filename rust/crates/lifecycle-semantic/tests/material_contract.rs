mod common;

use common::*;
use lifecycle_semantic::*;

#[test]
fn corpus_sources_are_exact_and_partial_native_history_is_ineligible() {
    for id in ["C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8"] {
        let basis = basis(id);
        assert_eq!(
            basis.obligations.len(),
            case(id)["obligations"].as_array().unwrap().len()
        );
        assert_eq!(
            basis.sources.len(),
            case(id)["source_ids"].as_array().unwrap().len()
        );
        for source in &basis.sources {
            if source.availability == SourceAvailability::Complete {
                assert_eq!(source.sha256, sha256_hex(&source.bytes));
            }
        }
        if id == "C8" {
            assert!(!basis.is_eligible());
            assert!(basis.issues.iter().any(|issue| matches!(
                issue,
                MaterialIssue::Unavailable {
                    status: SourceAvailability::Truncated,
                    ..
                }
            )));
            assert!(
                compile_request(
                    &basis,
                    binding(&basis, SemanticPhase::Compile),
                    &SemanticPolicy::default()
                )
                .is_err()
            );
        } else {
            assert!(basis.is_eligible(), "{id}: {:?}", basis.issues);
        }
    }
}

#[test]
fn currentness_is_revision_specific_and_unrelated_subject_does_not_invalidate() {
    let old = basis("C1");
    let mut current = old.dependencies.clone();
    let question = current
        .iter_mut()
        .find(|item| item.dependency_key == "question-q1")
        .unwrap();
    question.revision = 8;
    assert_eq!(compare_dependencies(&old.dependencies, &current).len(), 1);

    let c4 = basis("C4");
    assert!(
        !c4.dependencies
            .iter()
            .any(|item| item.owner_namespace == "synthetic-other-subject-owner")
    );
    let mut with_unrelated = c4.dependencies.clone();
    with_unrelated.push(DependencyRevision {
        owner_namespace: "synthetic-other-subject-owner".into(),
        dependency_key: "unrelated-r9".into(),
        owner_ref: "unrelated-r9".into(),
        revision: 10,
        sha256: "a".repeat(64),
    });
    assert!(compare_dependencies(&c4.dependencies, &with_unrelated).is_empty());

    let observed: Vec<_> = old
        .dependencies
        .iter()
        .filter(|item| item.dependency_key != "question-q1")
        .cloned()
        .map(DependencyObservation::Current)
        .collect();
    let changes = revalidate_dependencies(&old.dependencies, &observed).unwrap();
    assert!(
        matches!(changes.as_slice(), [DependencyChange::Unavailable { dependency_key, .. }] if dependency_key == "question-q1")
    );
}

#[test]
fn material_refuses_forgery_bad_provenance_and_silent_overflow() {
    let mut altered = input("C5");
    altered.sources[0].sha256 = "0".repeat(64);
    assert_eq!(project_material(altered), Err(SemanticError::InvalidDigest));

    let mut promoted = input("C5");
    promoted
        .sources
        .iter_mut()
        .find(|source| source.id == "untrusted-note")
        .unwrap()
        .trust = SourceTrust::HostGoverning;
    assert_eq!(
        project_material(promoted),
        Err(SemanticError::InvalidSource)
    );

    let mut oversized = input("C1");
    oversized.policy.max_source_item_bytes = 16;
    let projected = project_material(oversized).unwrap();
    assert!(!projected.is_eligible());
    assert!(
        projected
            .issues
            .iter()
            .any(|issue| matches!(issue, MaterialIssue::ItemTooLarge { .. }))
    );
}

#[test]
fn unknown_schema_or_policy_is_explicitly_ineligible() {
    let mut unknown_schema = input("C1");
    unknown_schema.policy.schema_version = 2;
    assert_eq!(
        project_material(unknown_schema),
        Err(SemanticError::UnsupportedPolicy)
    );

    let mut unknown_policy = input("C1");
    unknown_policy.policy.id = "unknown-policy-v1".into();
    assert_eq!(
        project_material(unknown_policy),
        Err(SemanticError::UnsupportedPolicy)
    );
}
