use lifecycle_core::{
    AdmissionError, AdmissionOwner, AuthorityExpiresAt, BuildId, ClockSample, Command,
    CommandAdmission, CommandId, CommandOutcome, CommandRequest, ContextGeneration,
    ControlledTextInput, DeliveryId, GrantId, GrantScope, InputId, ProofRunId, RejectionCode,
    Revision, SubjectId, SubjectState, TransitionId, TrustedGrant, WaitBudgetMs, WallTimeMs,
    reduce_command,
};
use serde_json::json;
use work_engine_types::CodecContract;

fn clock(wall: i64) -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(wall),
        wait_budget: WaitBudgetMs::new(7),
    }
}

fn fixture() -> (SubjectState, TrustedGrant, CommandAdmission) {
    let subject = SubjectId::parse("subject-grant").unwrap();
    let context = ContextGeneration::parse("context-a").unwrap();
    let build = BuildId::parse("build-a").unwrap();
    let proof = ProofRunId::parse("proof-a").unwrap();
    let grant_id = GrantId::parse("grant-a").unwrap();
    let input_id = InputId::parse("input-a").unwrap();
    let text = "exact bytes";
    let text_digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap();
    let basis = json!({
        "protocol_version":1,"principal_ref":"trusted-caller","command_id":"command-a",
        "subject_id":subject.as_str(),"context_generation":context.as_str(),
        "build_id":build.as_str(),"proof_run_id":proof.as_str(),"grant_ref":grant_id.as_str(),
        "expected_revision":"3","kind":"enqueue_input",
        "payload":{"input_id":input_id.as_str(),"producer_ref":"fixture:grant",
            "text":text,"text_digest":text_digest.hex()}
    });
    let request = CommandRequest::new(
        CommandId::parse("command-a").unwrap(),
        subject.clone(),
        context.clone(),
        build.clone(),
        proof.clone(),
        grant_id.clone(),
        Revision::new(3),
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap(),
        Command::EnqueueInput {
            input: input_id.clone(),
        },
    )
    .unwrap();
    let admission = CommandAdmission::new(
        request,
        "trusted-caller".into(),
        basis,
        Some(ControlledTextInput::new(input_id, text.into(), text_digest).unwrap()),
    )
    .unwrap();
    let state = SubjectState {
        subject: subject.clone(),
        context: context.clone(),
        build: build.clone(),
        proof_run: proof.clone(),
        revision: Revision::new(3),
        semantic_revision: Revision::new(1),
        queue_revision: Revision::new(2),
        last_wall: WallTimeMs::new(100),
        owner: AdmissionOwner::Domain {
            generation: context.clone(),
        },
    };
    let grant = TrustedGrant {
        id: grant_id,
        issuer: "trusted".into(),
        principal: "trusted-caller".into(),
        subject,
        context,
        build,
        proof_run: proof,
        scope: GrantScope::EnqueueInput,
        expires: AuthorityExpiresAt::new(WallTimeMs::new(200)),
        revision: Revision::new(4),
        revoked: false,
    };
    (state, grant, admission)
}

fn decision(
    state: &SubjectState,
    grant: Option<&TrustedGrant>,
    admission: &CommandAdmission,
    wall: i64,
) -> CommandOutcome {
    reduce_command(
        state,
        admission,
        grant,
        clock(wall),
        DeliveryId::parse("delivery-a").unwrap(),
        TransitionId::parse("transition-a").unwrap(),
        false,
    )
    .result
    .outcome
}

#[test]
fn grant_scope_principal_revision_and_expiry_are_pure_entry_inputs() {
    let (state, grant, admission) = fixture();
    assert!(matches!(
        decision(&state, Some(&grant), &admission, 199),
        CommandOutcome::Enqueued { .. }
    ));
    assert_eq!(
        decision(&state, Some(&grant), &admission, 200),
        CommandOutcome::Rejected(RejectionCode::ExpiredGrant)
    );
    assert_eq!(
        decision(&state, Some(&grant), &admission, 201),
        CommandOutcome::Rejected(RejectionCode::ExpiredGrant)
    );
    assert_eq!(
        decision(&state, Some(&grant), &admission, 99),
        CommandOutcome::Rejected(RejectionCode::ClockRollback)
    );
    assert_eq!(
        decision(&state, None, &admission, 101),
        CommandOutcome::Rejected(RejectionCode::InvalidGrant)
    );
    let mut bad = grant.clone();
    bad.revoked = true;
    assert_eq!(
        decision(&state, Some(&bad), &admission, 101),
        CommandOutcome::Rejected(RejectionCode::InvalidGrant)
    );
    bad = grant.clone();
    bad.scope = GrantScope::RequestReplacement;
    assert_eq!(
        decision(&state, Some(&bad), &admission, 101),
        CommandOutcome::Rejected(RejectionCode::InvalidGrant)
    );
    bad = grant.clone();
    bad.principal = "other".into();
    assert_eq!(
        decision(&state, Some(&bad), &admission, 101),
        CommandOutcome::Rejected(RejectionCode::InvalidGrant)
    );
    let mut stale = state.clone();
    stale.revision = Revision::new(4);
    assert_eq!(
        decision(&stale, Some(&grant), &admission, 101),
        CommandOutcome::Rejected(RejectionCode::StaleRevision)
    );
}

#[test]
fn canonical_basis_rejects_swapped_identity_input_and_untrusted_principal() {
    let (_, _, admission) = fixture();
    let mut basis = admission.digest_basis().clone();
    basis["context_generation"] = json!("context-other");
    let mut request = admission.request().clone();
    request.request_digest = CodecContract::LifecycleCommandV1
        .digest_json(&basis)
        .unwrap();
    assert!(matches!(
        CommandAdmission::new(
            request,
            "trusted-caller".into(),
            basis,
            admission.input().cloned()
        ),
        Err(AdmissionError::MalformedBasis)
    ));

    for (field, replacement) in [
        ("build_id", "build-other"),
        ("proof_run_id", "proof-other"),
        ("principal_ref", "forged-principal"),
        ("grant_ref", "grant-other"),
        ("expected_revision", "99"),
    ] {
        let mut basis = admission.digest_basis().clone();
        basis[field] = json!(replacement);
        let mut request = admission.request().clone();
        request.request_digest = CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap();
        assert!(
            matches!(
                CommandAdmission::new(
                    request,
                    "trusted-caller".into(),
                    basis,
                    admission.input().cloned()
                ),
                Err(AdmissionError::MalformedBasis)
            ),
            "{field}"
        );
    }
    let mut basis = admission.digest_basis().clone();
    basis["payload"]["input_id"] = json!("input-other");
    let mut request = admission.request().clone();
    request.request_digest = CodecContract::LifecycleCommandV1
        .digest_json(&basis)
        .unwrap();
    assert!(matches!(
        CommandAdmission::new(
            request,
            "trusted-caller".into(),
            basis,
            admission.input().cloned()
        ),
        Err(AdmissionError::MalformedBasis)
    ));
    let mut basis = admission.digest_basis().clone();
    basis["payload"]["text_digest"] = json!("caller-chosen-digest");
    let mut request = admission.request().clone();
    request.request_digest = CodecContract::LifecycleCommandV1
        .digest_json(&basis)
        .unwrap();
    assert!(matches!(
        CommandAdmission::new(
            request,
            "trusted-caller".into(),
            basis,
            admission.input().cloned()
        ),
        Err(AdmissionError::MalformedBasis)
    ));
    assert!(matches!(
        CommandAdmission::new(
            admission.request().clone(),
            "other".into(),
            admission.digest_basis().clone(),
            admission.input().cloned()
        ),
        Err(AdmissionError::MalformedBasis)
    ));
    let mut basis = admission.digest_basis().clone();
    basis["request_supplied_verification_key"] = json!("untrusted");
    let mut request = admission.request().clone();
    request.request_digest = CodecContract::LifecycleCommandV1
        .digest_json(&basis)
        .unwrap();
    assert!(matches!(
        CommandAdmission::new(
            request,
            "trusted-caller".into(),
            basis,
            admission.input().cloned()
        ),
        Err(AdmissionError::MalformedBasis)
    ));
}
