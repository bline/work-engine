mod common;

use common::*;
use lifecycle_semantic::*;
use serde_json::Value;

#[test]
fn isolated_control_prompts_project_consumed_schemas_and_private_obligation_scope() {
    let material = basis("C1");
    let policy = SemanticPolicy::default();
    let (compile, candidate) = candidate(&material);
    let compiled: Value = serde_json::from_str(&compile.prompt).unwrap();
    let continuation = &compiled["model_output_contract"];
    assert_eq!(continuation["schema_version"], material.schema_version);
    assert_eq!(
        continuation["source_reference"]["owner_namespace"],
        "string"
    );
    assert!(
        continuation["required_fields"]["interactions"]
            .as_str()
            .unwrap()
            .contains("answer_id")
    );
    assert!(
        continuation["required_fields"]["decisions"]
            .as_str()
            .unwrap()
            .contains("supersedes")
    );
    assert_eq!(
        continuation["interaction_status"],
        serde_json::json!(["pending", "scoped_answer", "resolved", "uncertain"])
    );

    let verify = verify_candidate(
        &material,
        &candidate,
        binding(&material, SemanticPhase::CandidateVerification),
        &policy,
    )
    .unwrap();
    let candidate_prompt: Value = serde_json::from_str(&verify.prompt).unwrap();
    assert_eq!(
        candidate_prompt["model_output_contract"]["finding_status"],
        serde_json::json!(["satisfied", "violated", "indeterminate"])
    );
    assert_eq!(
        candidate_prompt["model_output_contract"]["required_fields"]["whole_response_assessed"],
        "boolean"
    );
    assert_eq!(
        candidate_prompt["required_obligations"]
            .as_array()
            .unwrap()
            .len(),
        material.obligations.len()
    );
    for (rendered, obligation) in candidate_prompt["required_obligations"]
        .as_array()
        .unwrap()
        .iter()
        .zip(&material.obligations)
    {
        assert_eq!(rendered["id"], obligation.id);
        assert_eq!(rendered["verifier_scope"], obligation.verifier_scope);
        assert_eq!(
            rendered["sources"],
            serde_json::to_value(&obligation.sources).unwrap()
        );
        assert!(
            rendered.get("description").is_none(),
            "private expected answer entered verifier projection"
        );
    }
    let scopes = candidate_prompt["required_obligations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["verifier_scope"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        scopes.len(),
        material.obligations.len(),
        "C1 obligations require distinct scopes"
    );
    assert!(!verify.prompt.contains("supported, contradicted"));

    let challenge = challenge_successor(
        &material,
        &candidate,
        "challenge-schema".into(),
        case("C1")["next_domain_prompt"].as_str().unwrap().into(),
        &policy,
    )
    .unwrap();
    let successor = successor_response(
        &challenge,
        "rehydrate-schema".into(),
        b"Controlled response".to_vec(),
        &policy,
    )
    .unwrap();
    let successor_verify = verify_successor(
        &material,
        &candidate,
        &challenge,
        &successor,
        binding(&material, SemanticPhase::SuccessorAdequacy),
        &policy,
    )
    .unwrap();
    let successor_prompt: Value = serde_json::from_str(&successor_verify.prompt).unwrap();
    assert_eq!(
        successor_prompt["required_obligations"],
        candidate_prompt["required_obligations"]
    );
    assert_eq!(
        successor_prompt["model_output_contract"],
        candidate_prompt["model_output_contract"]
    );
}

#[test]
fn controlled_c1_candidate_and_successor_require_separate_complete_assessments() {
    let material = basis("C1");
    let policy = SemanticPolicy::default();
    let (_, candidate) = candidate(&material);
    let verify = verify_candidate(
        &material,
        &candidate,
        binding(&material, SemanticPhase::CandidateVerification),
        &policy,
    )
    .unwrap();
    let accepted = assess_candidate(
        &material,
        &candidate,
        &verify,
        result(&verify, &report(&material, FindingStatus::Satisfied)),
        exposure(&verify),
        &policy,
    )
    .unwrap();
    assert_eq!(accepted.status, VerdictStatus::Accepted);
    assert_eq!(
        assess_currentness(&accepted, Some(&material.dependencies)),
        BasisEligibility::Current
    );

    let challenge = challenge_successor(
        &material,
        &candidate,
        "challenge-C1".into(),
        case("C1")["next_domain_prompt"].as_str().unwrap().into(),
        &policy,
    )
    .unwrap();
    assert!(!challenge.next_domain_prompt.contains("K-1"));
    let successor = successor_response(
        &challenge,
        "rehydrate-C1".into(),
        b"D-2 governs. K-1 is unfinished. Q-1 remains pending; do not publish. I-2 stays queued."
            .to_vec(),
        &policy,
    )
    .unwrap();
    let successor_verification = verify_successor(
        &material,
        &candidate,
        &challenge,
        &successor,
        binding(&material, SemanticPhase::SuccessorAdequacy),
        &policy,
    )
    .unwrap();
    let ready = assess_successor(
        &material,
        &candidate,
        &challenge,
        &successor,
        &successor_verification,
        result(
            &successor_verification,
            &report(&material, FindingStatus::Satisfied),
        ),
        exposure(&successor_verification),
        &policy,
    )
    .unwrap();
    assert_eq!(ready.status, VerdictStatus::Accepted);

    let echo = successor_response(
        &challenge,
        "rehydrate-echo".into(),
        candidate.sha256.as_bytes().to_vec(),
        &policy,
    )
    .unwrap();
    let echo_request = verify_successor(
        &material,
        &candidate,
        &challenge,
        &echo,
        binding(&material, SemanticPhase::SuccessorAdequacy),
        &policy,
    )
    .unwrap();
    let echo_result = assess_successor(
        &material,
        &candidate,
        &challenge,
        &echo,
        &echo_request,
        result(
            &echo_request,
            &report(&material, FindingStatus::Indeterminate),
        ),
        exposure(&echo_request),
        &policy,
    )
    .unwrap();
    assert_eq!(echo_result.status, VerdictStatus::Unresolved);
}

#[test]
fn omitted_commitment_and_revived_action_are_critical_failures() {
    for (id, kind) in [
        ("C6", ErrorKind::OmittedMaterialCommitment),
        ("C3", ErrorKind::RevivedSupersededAction),
    ] {
        let material = basis(id);
        let policy = SemanticPolicy::default();
        let (_, candidate) = candidate(&material);
        let request = verify_candidate(
            &material,
            &candidate,
            binding(&material, SemanticPhase::CandidateVerification),
            &policy,
        )
        .unwrap();
        let mut annotated = report(&material, FindingStatus::Satisfied);
        annotated.critical_errors.push(CriticalFinding {
            kind,
            sources: material.obligations[0].sources.clone(),
            explanation: "Controlled variant omits K-1 or revives E-1".into(),
        });
        let verdict = assess_candidate(
            &material,
            &candidate,
            &request,
            result(&request, &annotated),
            exposure(&request),
            &policy,
        )
        .unwrap();
        assert_eq!(verdict.status, VerdictStatus::Rejected);
    }
}

#[test]
fn pending_approval_can_be_preserved_without_becoming_permission() {
    let material = basis("C4");
    let policy = SemanticPolicy::default();
    let (_, candidate) = candidate(&material);
    let request = verify_candidate(
        &material,
        &candidate,
        binding(&material, SemanticPhase::CandidateVerification),
        &policy,
    )
    .unwrap();
    let carried = assess_candidate(
        &material,
        &candidate,
        &request,
        result(&request, &report(&material, FindingStatus::Satisfied)),
        exposure(&request),
        &policy,
    )
    .unwrap();
    assert_eq!(carried.status, VerdictStatus::Accepted);
    let mut widened = report(&material, FindingStatus::Satisfied);
    widened.critical_errors.push(CriticalFinding {
        kind: ErrorKind::InventedOrWidenedApproval,
        sources: material.obligations[0].sources.clone(),
        explanation: "Historical G-1 revision 3 was promoted to publication authority".into(),
    });
    let rejected = assess_candidate(
        &material,
        &candidate,
        &request,
        result(&request, &widened),
        exposure(&request),
        &policy,
    )
    .unwrap();
    assert_eq!(rejected.status, VerdictStatus::Rejected);
}

#[test]
fn missing_findings_or_unobserved_isolation_never_accept() {
    let material = basis("C7");
    let policy = SemanticPolicy::default();
    let (_, candidate) = candidate(&material);
    let request = verify_candidate(
        &material,
        &candidate,
        binding(&material, SemanticPhase::CandidateVerification),
        &policy,
    )
    .unwrap();
    let mut incomplete = report(&material, FindingStatus::Satisfied);
    incomplete.findings.pop();
    let verdict = assess_candidate(
        &material,
        &candidate,
        &request,
        result(&request, &incomplete),
        exposure(&request),
        &policy,
    )
    .unwrap();
    assert_eq!(verdict.status, VerdictStatus::Unresolved);

    let mut same_session = exposure(&request);
    same_session.compiler_session_id = same_session.verifier_session_id.clone();
    let verdict = assess_candidate(
        &material,
        &candidate,
        &request,
        result(&request, &report(&material, FindingStatus::Satisfied)),
        same_session,
        &policy,
    )
    .unwrap();
    assert_eq!(verdict.status, VerdictStatus::Unresolved);
    let mut wrong_model = exposure(&request);
    wrong_model.observed_model = "unqualified-model".into();
    let verdict = assess_candidate(
        &material,
        &candidate,
        &request,
        result(&request, &report(&material, FindingStatus::Satisfied)),
        wrong_model,
        &policy,
    )
    .unwrap();
    assert_eq!(verdict.status, VerdictStatus::Unresolved);
}

#[test]
fn swapped_invocation_result_is_refused_before_semantic_use() {
    let material = basis("C5");
    let policy = SemanticPolicy::default();
    let (_, candidate) = candidate(&material);
    let request = verify_candidate(
        &material,
        &candidate,
        binding(&material, SemanticPhase::CandidateVerification),
        &policy,
    )
    .unwrap();
    let mut wrong = result(&request, &report(&material, FindingStatus::Satisfied));
    wrong.invocation_id = "other-invocation".into();
    assert_eq!(
        assess_candidate(
            &material,
            &candidate,
            &request,
            wrong,
            exposure(&request),
            &policy
        ),
        Err(SemanticError::WrongBinding)
    );
}
