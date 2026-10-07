use lifecycle_core::{
    AuthorityExpiresAt, BuildId, ClockSample, Command, CommandId, CommandRequest,
    ContextGeneration, ControlledTextInput, EffectId, EffectInput, EffectKind, EffectPlan, GrantId,
    InputId, ProofRunId, Revision, SubjectId, WaitBudgetMs, WallTimeMs,
};
use serde_json::json;
use work_engine_types::CodecContract;

#[test]
fn wrong_digest_contract_and_expiry_boundary_reject() {
    let digest = CodecContract::LifecycleSnapshotV1
        .digest_json(&json!({"test": true}))
        .unwrap();
    let request = CommandRequest::new(
        CommandId::parse("command-1").unwrap(),
        SubjectId::parse("subject-1").unwrap(),
        ContextGeneration::parse("context-1").unwrap(),
        BuildId::parse("build-1").unwrap(),
        ProofRunId::parse("proof-1").unwrap(),
        GrantId::parse("grant-1").unwrap(),
        Revision::new(1),
        digest,
        Command::RequestReplacement,
    );
    assert!(request.is_err());

    let expiry = AuthorityExpiresAt::new(WallTimeMs::new(1000));
    assert!(expiry.allows_new_entry(ClockSample {
        wall: WallTimeMs::new(999),
        wait_budget: WaitBudgetMs::new(10),
    }));
    assert!(!expiry.allows_new_entry(ClockSample {
        wall: WallTimeMs::new(1000),
        wait_budget: WaitBudgetMs::new(10),
    }));
}

#[test]
fn distinct_identifiers_serialize_as_values_but_retain_rust_types() {
    let context = ContextGeneration::parse("same-text").unwrap();
    let build = BuildId::parse("same-text").unwrap();
    assert_eq!(context.as_str(), build.as_str());
    assert_eq!(serde_json::to_string(&context).unwrap(), "\"same-text\"");
}

#[test]
fn controlled_text_plan_binds_exact_input_before_entry() {
    let input_id = InputId::parse("input-1").unwrap();
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(b"hello\n")
        .unwrap();
    let input = ControlledTextInput::new(input_id.clone(), "hello\n".into(), digest.clone())
        .expect("exact admitted bytes");
    assert_eq!(input.input_id(), &input_id);
    assert_eq!(input.text(), "hello\n");
    assert_eq!(input.digest(), &digest);
    assert!(ControlledTextInput::new(input_id.clone(), "other".into(), digest.clone()).is_err());
    assert!(
        ControlledTextInput::new(
            input_id,
            "hello\n".into(),
            CodecContract::BinaryArtifactV1
                .digest_binary(b"hello\n")
                .unwrap(),
        )
        .is_err()
    );

    let plan = EffectPlan {
        effect: EffectId::parse("effect-1").unwrap(),
        subject: SubjectId::parse("subject-1").unwrap(),
        input: EffectInput::ControlledText(input),
        expected_revision: Revision::new(3),
    };
    assert_eq!(plan.kind(), EffectKind::ControlledTextTurn);
}
