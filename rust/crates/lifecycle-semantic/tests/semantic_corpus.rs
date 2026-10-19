mod common;

use common::*;
use lifecycle_semantic::*;

#[test]
fn c1_through_c7_control_prompts_exclude_private_fixture_oracle() {
    let policy = SemanticPolicy::default();
    for id in ["C1", "C2", "C3", "C4", "C5", "C6", "C7"] {
        let material = basis(id);
        let (compile, candidate) = candidate(&material);
        let verify = verify_candidate(
            &material,
            &candidate,
            binding(&material, SemanticPhase::CandidateVerification),
            &policy,
        )
        .unwrap();
        for obligation in &material.obligations {
            assert!(
                !compile.prompt.contains(&obligation.description),
                "{id} compile oracle leak"
            );
            assert!(
                !verify.prompt.contains(&obligation.description),
                "{id} verifier oracle leak"
            );
        }
        assert!(!compile.prompt.contains("controlled_oracle"));
        assert!(!verify.prompt.contains("forbidden"));
        assert_eq!(compile.source_basis_sha256, material.sha256);
        assert_eq!(verify.candidate_sha256, candidate.sha256);
        let controlled = assess_candidate(
            &material,
            &candidate,
            &verify,
            result(&verify, &report(&material, FindingStatus::Satisfied)),
            exposure(&verify),
            &policy,
        )
        .unwrap();
        assert_eq!(
            controlled.status,
            VerdictStatus::Accepted,
            "{id}: {:?}",
            controlled.reasons
        );
    }
}

#[test]
fn c2_late_scoped_answer_invalidates_old_basis_without_granting_publication() {
    let material = basis("C2");
    let policy = SemanticPolicy::default();
    let (_, candidate) = candidate(&material);
    let verify = verify_candidate(
        &material,
        &candidate,
        binding(&material, SemanticPhase::CandidateVerification),
        &policy,
    )
    .unwrap();
    let result = assess_candidate(
        &material,
        &candidate,
        &verify,
        result(&verify, &report(&material, FindingStatus::Satisfied)),
        exposure(&verify),
        &policy,
    )
    .unwrap();
    let mut current = material.dependencies.clone();
    current
        .iter_mut()
        .find(|dependency| dependency.dependency_key == "question-q1")
        .unwrap()
        .revision = 9;
    assert!(
        matches!(assess_currentness(&result, Some(&current)), BasisEligibility::Stale(changes) if changes.len() == 1)
    );
    assert_eq!(
        assess_currentness(&result, None),
        BasisEligibility::Unavailable
    );
}

#[test]
fn c5_forged_reference_and_untrusted_authority_are_refused() {
    let material = basis("C5");
    let policy = SemanticPolicy::default();
    let compile = compile_request(
        &material,
        binding(&material, SemanticPhase::Compile),
        &policy,
    )
    .unwrap();
    let mut plan = complete_plan(&material);
    plan.next_action.sources[0].id = "invented-R-2".into();
    assert_eq!(
        candidate_from_output(
            &material,
            &compile,
            serde_json::to_vec(&plan).unwrap(),
            &policy
        ),
        Err(SemanticError::InvalidReference)
    );
    let untrusted = material.source("untrusted-note").unwrap();
    assert_eq!(untrusted.trust, SourceTrust::UntrustedData);
    assert!(compile.prompt.contains("untrusted_data"));
}

#[test]
fn compiler_contract_exposes_citation_categories_and_rejects_live2_shape() {
    let material = basis("C5");
    let policy = SemanticPolicy::default();
    let compile = compile_request(
        &material,
        binding(&material, SemanticPhase::Compile),
        &policy,
    )
    .unwrap();
    let prompt: serde_json::Value = serde_json::from_str(&compile.prompt).unwrap();
    let rules = &prompt["model_output_contract"]["field_citation_rules"];
    for field in [
        "next_action",
        "governing",
        "canonical_refs",
        "decisions",
        "interactions",
    ] {
        assert!(rules[field].as_str().is_some_and(|rule| !rule.is_empty()));
    }
    let mut plan = complete_plan(&material);
    let canonical = material
        .sources
        .iter()
        .find(|source| source.kind == SourceKind::CanonicalRecord && source.current)
        .unwrap();
    plan.governing.push(AttributedStatement {
        text: "A canonical question incorrectly presented as governing".into(),
        sources: vec![canonical.reference()],
    });
    assert_eq!(
        candidate_from_output(
            &material,
            &compile,
            serde_json::to_vec(&plan).unwrap(),
            &policy
        ),
        Err(SemanticError::InvalidReference)
    );
    assert_eq!(
        candidate_from_output(&material, &compile, b"{".to_vec(), &policy),
        Err(SemanticError::InvalidOutput)
    );
    assert_eq!(
        candidate_from_output(
            &material,
            &compile,
            vec![b'x'; policy.max_control_output_bytes + 1],
            &policy
        ),
        Err(SemanticError::OutputTooLarge)
    );
}

#[test]
fn persisted_public_dto_swaps_cannot_change_the_assessed_subject() {
    let material = basis("C1");
    let policy = SemanticPolicy::default();
    let (_, candidate) = candidate(&material);
    let request = verify_candidate(
        &material,
        &candidate,
        binding(&material, SemanticPhase::CandidateVerification),
        &policy,
    )
    .unwrap();

    let mut forged_basis = material.clone();
    forged_basis.sources[0].bytes.push(b'!');
    assert_eq!(
        forged_basis.validate(&policy),
        Err(SemanticError::InvalidDigest)
    );

    let mut forged_candidate = candidate.clone();
    forged_candidate.plan.commitments.clear();
    assert_eq!(
        verify_candidate(
            &material,
            &forged_candidate,
            binding(&material, SemanticPhase::CandidateVerification),
            &policy,
        ),
        Err(SemanticError::WrongBinding)
    );

    let mut forged_request = request.clone();
    forged_request.compile_invocation_id = "other-compiler".into();
    assert_eq!(
        assess_candidate(
            &material,
            &candidate,
            &forged_request,
            result(
                &forged_request,
                &report(&material, FindingStatus::Satisfied)
            ),
            exposure(&forged_request),
            &policy,
        ),
        Err(SemanticError::WrongBinding)
    );
}

#[test]
fn c7_completed_effect_replay_is_critical_even_with_valid_hashes() {
    let material = basis("C7");
    let policy = SemanticPolicy::default();
    let (_, candidate) = candidate(&material);
    let verify = verify_candidate(
        &material,
        &candidate,
        binding(&material, SemanticPhase::CandidateVerification),
        &policy,
    )
    .unwrap();
    let mut report = report(&material, FindingStatus::Satisfied);
    report.critical_errors.push(CriticalFinding {
        kind: ErrorKind::CompletedEffectReplay,
        sources: material.obligations[0].sources.clone(),
        explanation: "E-a was already settled and must not be replayed".into(),
    });
    let result = assess_candidate(
        &material,
        &candidate,
        &verify,
        result(&verify, &report),
        exposure(&verify),
        &policy,
    )
    .unwrap();
    assert_eq!(result.status, VerdictStatus::Rejected);
}

#[test]
fn c8_partial_history_cannot_be_relabelled_complete_by_a_verifier() {
    let material = basis("C8");
    assert!(!material.is_eligible());
    assert!(material.issues.iter().any(|issue| matches!(issue, MaterialIssue::RequiredSourceUnavailable { source, .. } if source == "native-history-partial")));
    assert_eq!(
        compile_request(
            &material,
            binding(&material, SemanticPhase::Compile),
            &SemanticPolicy::default()
        ),
        Err(SemanticError::IneligibleMaterial)
    );
}
