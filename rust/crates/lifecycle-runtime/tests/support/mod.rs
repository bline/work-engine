use lifecycle_core::{
    AuthorityExpiresAt, BuildId, ClockSample, Command, CommandAdmission, CommandId, CommandRequest,
    ContextGeneration, ControlledTextInput, GrantId, GrantScope, InputId, ProofRunId, Revision,
    RuntimeIncarnation, SubjectId, TrustedGrant, WaitBudgetMs, WallTimeMs,
};
use lifecycle_store::{AuthorizedEntry, SqliteLifecycleStore};
use serde_json::json;
use tempfile::TempDir;
use work_engine_types::CodecContract;

pub fn clock(wall: i64) -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(wall),
        wait_budget: WaitBudgetMs::new(100),
    }
}

pub fn claimed_entry() -> (TempDir, SqliteLifecycleStore, AuthorizedEntry) {
    let root = tempfile::tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let entry = claim_named(&mut store, "1");
    (root, store, entry)
}

pub fn claimed_two_entries() -> (
    TempDir,
    SqliteLifecycleStore,
    AuthorizedEntry,
    AuthorizedEntry,
) {
    let root = tempfile::tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let first = claim_named(&mut store, "1");
    let second = claim_named(&mut store, "2");
    (root, store, first, second)
}

fn claim_named(store: &mut SqliteLifecycleStore, suffix: &str) -> AuthorizedEntry {
    let subject_name = format!("subject-{suffix}");
    let context_name = format!("context-{suffix}");
    let build_name = format!("build-{suffix}");
    let proof_name = format!("proof-{suffix}");
    let grant_name = format!("grant-{suffix}");
    let command_name = format!("command-{suffix}");
    let input_name = format!("input-{suffix}");
    let incarnation_name = format!("incarnation-{suffix}");
    let subject = SubjectId::parse(subject_name.clone()).unwrap();
    let context = ContextGeneration::parse(context_name.clone()).unwrap();
    let build = BuildId::parse(build_name.clone()).unwrap();
    let proof_run = ProofRunId::parse(proof_name.clone()).unwrap();
    let grant_id = GrantId::parse(grant_name.clone()).unwrap();
    store
        .register_subject(
            subject.clone(),
            context.clone(),
            build.clone(),
            proof_run.clone(),
            clock(100),
        )
        .unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: grant_id.clone(),
            issuer: "test-issuer".into(),
            principal: "operator:controlled".into(),
            subject: subject.clone(),
            context: context.clone(),
            build: build.clone(),
            proof_run: proof_run.clone(),
            scope: GrantScope::EnqueueInput,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    let input_id = InputId::parse(input_name.clone()).unwrap();
    let text = "owned text\n";
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap();
    let controlled =
        ControlledTextInput::new(input_id.clone(), text.into(), digest.clone()).unwrap();
    let basis = json!({
        "protocol_version": 1,
        "principal_ref": "operator:controlled",
        "command_id": command_name,
        "subject_id": subject_name,
        "context_generation": context_name,
        "build_id": build_name,
        "proof_run_id": proof_name,
        "grant_ref": grant_name,
        "expected_revision": "0",
        "kind": "enqueue_input",
        "payload": {
            "input_id": input_name, "producer_ref": "fixture:vertical",
            "text": text, "text_digest": digest.hex(),
        },
    });
    let request = CommandRequest::new(
        CommandId::parse(command_name).unwrap(),
        subject.clone(),
        context,
        build,
        proof_run,
        grant_id,
        Revision::new(0),
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap(),
        Command::EnqueueInput { input: input_id },
    )
    .unwrap();
    let admission = CommandAdmission::new(
        request,
        "operator:controlled".into(),
        basis,
        Some(controlled),
    )
    .unwrap();
    store.apply_checked_command(admission, clock(101)).unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    store
        .claim_prepared_entry(
            &plan.effect,
            RuntimeIncarnation::parse(incarnation_name).unwrap(),
            clock(102),
        )
        .unwrap()
}
