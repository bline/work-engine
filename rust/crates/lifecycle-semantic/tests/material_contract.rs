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

#[test]
fn retained_capacity_boundaries_are_typed_and_exact_prompt_overflow_is_distinct() {
    let mut item = input("C1");
    item.policy = SemanticPolicy::retained_repository_analysis();
    item.sources[0].bytes = vec![b'a'; 16 * 1024];
    item.sources[0].sha256 = sha256_hex(&item.sources[0].bytes);
    for obligation in &mut item.obligations {
        for reference in &mut obligation.sources {
            if reference.id == item.sources[0].id {
                reference.sha256 = item.sources[0].sha256.clone();
            }
        }
    }
    assert!(
        !project_material(item.clone())
            .unwrap()
            .issues
            .iter()
            .any(|issue| matches!(issue, MaterialIssue::ItemTooLarge { .. }))
    );
    item.sources[0].bytes.push(b'a');
    item.sources[0].sha256 = sha256_hex(&item.sources[0].bytes);
    for obligation in &mut item.obligations {
        for reference in &mut obligation.sources {
            if reference.id == item.sources[0].id {
                reference.sha256 = item.sources[0].sha256.clone();
            }
        }
    }
    assert!(
        project_material(item)
            .unwrap()
            .issues
            .iter()
            .any(|issue| matches!(issue, MaterialIssue::ItemTooLarge { .. }))
    );

    let mut count = input("C1");
    count.policy = SemanticPolicy::retained_repository_analysis();
    let template = count.sources[0].clone();
    while count.sources.len() < 64 {
        let mut next = template.clone();
        next.id = format!("extra-{}", count.sources.len());
        next.dependency_key = next.id.clone();
        count.sources.push(next);
    }
    assert!(
        !project_material(count.clone())
            .unwrap()
            .issues
            .iter()
            .any(|issue| matches!(issue, MaterialIssue::TooManyItems))
    );
    let mut next = template.clone();
    next.id = "extra-64".into();
    next.dependency_key = next.id.clone();
    count.sources.push(next);
    assert!(
        project_material(count)
            .unwrap()
            .issues
            .iter()
            .any(|issue| matches!(issue, MaterialIssue::TooManyItems))
    );

    let mut total = input("C1");
    total.policy = SemanticPolicy::retained_repository_analysis();
    let mut raw: usize = total.sources.iter().map(|s| s.bytes.len()).sum();
    while raw < 64 * 1024 {
        let length = (64 * 1024 - raw).min(16 * 1024);
        let mut next = template.clone();
        next.id = format!("total-{}", total.sources.len());
        next.dependency_key = next.id.clone();
        next.bytes = vec![b'x'; length];
        next.sha256 = sha256_hex(&next.bytes);
        total.sources.push(next);
        raw += length;
    }
    assert!(
        !project_material(total.clone())
            .unwrap()
            .issues
            .iter()
            .any(|issue| matches!(issue, MaterialIssue::TotalTooLarge))
    );
    let mut next = template.clone();
    next.id = "total-extra".into();
    next.dependency_key = next.id.clone();
    next.bytes = vec![b'x'];
    next.sha256 = sha256_hex(&next.bytes);
    total.sources.push(next);
    assert!(
        project_material(total)
            .unwrap()
            .issues
            .iter()
            .any(|issue| matches!(issue, MaterialIssue::TotalTooLarge))
    );

    let mut prompt = input("C1");
    prompt.policy = SemanticPolicy::retained_repository_analysis();
    let basis = project_material(prompt.clone()).unwrap();
    let exact = compile_request(
        &basis,
        binding(&basis, SemanticPhase::Compile),
        &prompt.policy,
    )
    .unwrap();
    let size = exact.prompt.len();
    prompt.policy.max_control_input_bytes = size;
    let at_limit = project_material(prompt.clone()).unwrap();
    assert!(
        compile_request(
            &at_limit,
            binding(&at_limit, SemanticPhase::Compile),
            &prompt.policy
        )
        .is_ok()
    );
    prompt.policy.max_control_input_bytes = size - 1;
    let over = project_material(prompt.clone()).unwrap();
    assert_eq!(
        compile_request(
            &over,
            binding(&over, SemanticPhase::Compile),
            &prompt.policy
        ),
        Err(SemanticError::PromptTooLarge)
    );
}

#[test]
fn configured_smaller_capacity_reports_typed_issues_without_losing_material() {
    let mut material = input("C1");
    material.policy = SemanticPolicy::retained_repository_analysis();
    let first_size = material.sources[0].bytes.len();
    let other_size: usize = material
        .sources
        .iter()
        .skip(1)
        .map(|source| source.bytes.len())
        .sum();
    material.policy.max_source_items = material.sources.len() - 1;
    material.policy.max_source_item_bytes = first_size - 1;
    material.policy.max_source_total_bytes = first_size + other_size - 1;
    let original_sources = material.sources.clone();
    let basis = project_material(material).unwrap();
    assert_eq!(basis.sources, original_sources);
    assert!(basis.issues.contains(&MaterialIssue::TooManyItems));
    assert!(basis.issues.contains(&MaterialIssue::ItemTooLarge {
        id: original_sources[0].id.clone(),
    }));
    assert!(basis.issues.contains(&MaterialIssue::TotalTooLarge));
    assert!(!basis.is_eligible());
}
