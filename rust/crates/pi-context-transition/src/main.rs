use lifecycle_core::*;
use lifecycle_store::*;
use pi_context_transition::{Error, PiAdapter};
use serde_json::{Value, json};
use work_engine_types::CodecContract;
fn clock() -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        ),
        wait_budget: WaitBudgetMs::new(100),
    }
}
fn grant(store: &mut SqliteLifecycleStore, id: &str, context: &str, scope: GrantScope) {
    store
        .install_trusted_grant(TrustedGrant {
            id: GrantId::parse(id).unwrap(),
            issuer: "fixture-root".into(),
            principal: "operator:fixture".into(),
            subject: SubjectId::parse("subject:pi:repair").unwrap(),
            context: ContextGeneration::parse(context).unwrap(),
            build: BuildId::parse("build:pi").unwrap(),
            proof_run: ProofRunId::parse("proof:pi").unwrap(),
            scope,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(clock().wall.get() + 600_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
}
fn admission(id: &str, kind: &str, grant: &str, revision: u64, payload: Value) -> CommandAdmission {
    let basis = json!({"protocol_version":1,"principal_ref":"operator:fixture","command_id":id,"subject_id":"subject:pi:repair","context_generation":"context:old","build_id":"build:pi","proof_run_id":"proof:pi","grant_ref":grant,"expected_revision":revision.to_string(),"kind":kind,"payload":payload});
    let (command, input) = if kind == "enqueue_input" {
        let id = InputId::parse(basis["payload"]["input_id"].as_str().unwrap()).unwrap();
        let text = basis["payload"]["text"].as_str().unwrap();
        let d = CodecContract::LifecycleTextInputV1
            .digest_binary(text.as_bytes())
            .unwrap();
        (
            Command::EnqueueInput { input: id.clone() },
            Some(ControlledTextInput::new(id, text.into(), d).unwrap()),
        )
    } else {
        (Command::RequestReplacement, None)
    };
    let req = CommandRequest::new(
        CommandId::parse(id).unwrap(),
        SubjectId::parse("subject:pi:repair").unwrap(),
        ContextGeneration::parse("context:old").unwrap(),
        BuildId::parse("build:pi").unwrap(),
        ProofRunId::parse("proof:pi").unwrap(),
        GrantId::parse(grant).unwrap(),
        Revision::new(revision),
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap(),
        command,
    )
    .unwrap();
    CommandAdmission::new(req, "operator:fixture".into(), basis, input).unwrap()
}
fn allocate(
    store: &mut SqliteLifecycleStore,
    id: &str,
    context: &str,
    manifest: &str,
) -> NativeSessionEntry {
    store
        .reserve_native_session(
            NativeSessionId::parse(id).unwrap(),
            SubjectId::parse("subject:pi:repair").unwrap(),
            ContextGeneration::parse(context).unwrap(),
            RuntimeIncarnation::parse("incarnation:pi").unwrap(),
            &pi_profile_digest(),
            PI_SDK_INDEX_SHA256,
            &pi_protocol_digest(),
            SnapshotId::parse("snapshot:pi").unwrap(),
            manifest,
            false,
            clock(),
        )
        .unwrap()
}
fn claim(
    store: &mut SqliteLifecycleStore,
    session: &NativeSessionEntry,
    id: &str,
    prompt: &str,
    purpose: NativeInvocationPurpose,
    grant: &str,
) -> Result<NativeTurnEntry, StoreError> {
    let entry = store.claim_native_turn(
        session,
        NativeTurnRequest {
            invocation_id: id.into(),
            attempt: AttemptId::parse(format!("attempt:{id}")).unwrap(),
            grant: GrantId::parse(grant).unwrap(),
            grant_revision: 1,
            purpose,
            prompt: prompt.as_bytes().to_vec(),
        },
        "operator:fixture",
        clock(),
    )?;
    Ok(entry)
}
#[tokio::main]
async fn main() -> Result<(), Error> {
    let root = std::path::Path::new("/work/store");
    let mut store = SqliteLifecycleStore::open(root, "fixture-root".into())?;
    let subject = SubjectId::parse("subject:pi:repair")?;
    store.register_subject(
        subject.clone(),
        ContextGeneration::parse("context:old")?,
        BuildId::parse("build:pi")?,
        ProofRunId::parse("proof:pi")?,
        clock(),
    )?;
    for (id, context, scope) in [
        ("grant:old", "context:old", GrantScope::NativeTurn),
        ("grant:new", "context:new", GrantScope::NativeTurn),
        (
            "grant:replace",
            "context:old",
            GrantScope::RequestReplacement,
        ),
        ("grant:queue", "context:old", GrantScope::EnqueueInput),
        ("grant:successor", "context:new", GrantScope::EnqueueInput),
    ] {
        grant(&mut store, id, context, scope);
    }
    for n in 0..2 {
        let text = format!("queued-human-{n}");
        let d = CodecContract::LifecycleTextInputV1.digest_binary(text.as_bytes())?;
        let result=store.apply_checked_command(admission(&format!("command:q{n}"),"enqueue_input","grant:queue",n,json!({"input_id":format!("input:q{n}"),"producer_ref":"fixture:human-unanswered","text":text,"text_digest":d.hex()})),clock())?;
        assert!(matches!(result.outcome, CommandOutcome::Enqueued { .. }));
    }
    let checkpoint=json!({"version":1,"successor_context":"context:new","unanswered_human":{"question":"Proceed beyond fixture?","authority":"human","resolved":false},"queued_order":["input:q0","input:q1"],"fixture":true}).to_string();
    let staged = store.stage_artifact(subject.clone(), checkpoint.as_bytes())?;
    let published = store.publish_artifact(staged, clock())?;
    let mut command = tokio::process::Command::new("/usr/local/bin/node");
    command
        .arg("/subject/fixture.mjs")
        .current_dir("/work")
        .env_clear()
        .env("HOME", "/work/home")
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("PI_CODING_AGENT_DIR", "/work/agent");
    let mut adapter = PiAdapter::launch(&mut command, "transport:pi-fixture".into())?;
    let initial_intent = allocate(
        &mut store,
        "session:old",
        "context:old",
        &published.digest_hex,
    );
    let (mut session, ready) = adapter.initialize(initial_intent).await?;
    let old = ready["sessionId"]
        .as_str()
        .ok_or("no Pi session")?
        .to_string();
    store.bind_native_thread(&mut session, ProviderThreadId::parse(old.clone())?)?;
    let prompt = "OLD_CONVERSATION_CANARY_7f39. Produce fixture checkpoint data only.";
    let mut turn = claim(
        &mut store,
        &session,
        "domain",
        prompt,
        NativeInvocationPurpose::DomainWork,
        "grant:old",
    )?;
    let proof = adapter
        .collect_prompt(
            &mut store,
            session.id(),
            &mut turn,
            prompt,
            &checkpoint,
            clock(),
        )
        .await?;
    assert!(
        store
            .settle_native_turn(
                &turn,
                &checkpoint,
                "observed_complete",
                &proof.transport,
                proof.history_sequence,
                clock()
            )
            .is_err()
    );
    assert!(
        adapter
            .collect_prompt(
                &mut store,
                session.id(),
                &mut turn,
                prompt,
                &checkpoint,
                clock()
            )
            .await
            .is_err()
    );
    store.settle_pi_turn(&turn, &proof, clock())?;
    assert!(
        adapter
            .collect_prompt(
                &mut store,
                session.id(),
                &mut turn,
                prompt,
                &checkpoint,
                clock()
            )
            .await
            .is_err()
    );
    assert!(store.settle_pi_turn(&turn, &proof, clock()).is_err());
    let result = store.apply_checked_command(
        admission(
            "command:replace",
            "request_replacement",
            "grant:replace",
            2,
            json!({"reason":"operator-approved fixture replacement"}),
        ),
        clock(),
    )?;
    let transition = match result.outcome {
        CommandOutcome::ReplacementRequested { transition } => transition,
        _ => return Err("replacement denied".into()),
    };
    assert!(
        adapter
            .replacement_ready(&mut store, &subject, &transition, &checkpoint)
            .is_err()
    ); // cancellation before actuator entry
    let semantic_revision = store
        .subject_projection(&subject, None, None)?
        .semantic_revision;
    let evidence = |x: &str| EvidenceId::parse(format!("fixture:{x}")).unwrap();
    let facts = [
        TransitionFact::PredecessorSafe {
            evidence: evidence("safe"),
        },
        TransitionFact::SourceFrozen {
            evidence: evidence("source"),
            semantic_revision: Revision::new(semantic_revision),
        },
        TransitionFact::VerificationEstablished {
            evidence: evidence("checkpoint"),
            checkpoint: CheckpointId::parse(format!("checkpoint:{}", published.digest_hex))?,
        },
        TransitionFact::SwitchAuthorized {
            evidence: evidence("switch"),
        },
    ];
    for fact in facts {
        let rev = store.subject_projection(&subject, None, None)?.revision;
        store.apply_transition_fact(&transition, Revision::new(rev), fact, None, clock())?;
    }
    assert!(
        adapter
            .replacement_ready(&mut store, &subject, &transition, "wrong checkpoint")
            .is_err()
    );
    let intent = store.reserve_native_session(
        NativeSessionId::parse("session:new")?,
        subject.clone(),
        ContextGeneration::parse("context:new")?,
        RuntimeIncarnation::parse("incarnation:pi")?,
        &pi_profile_digest(),
        PI_SDK_INDEX_SHA256,
        &pi_protocol_digest(),
        SnapshotId::parse("snapshot:pi")?,
        &published.digest_hex,
        false,
        clock(),
    )?;
    let (mut successor, replaced) = adapter
        .replace(&mut store, &subject, &transition, &checkpoint, intent)
        .await?;
    let new = replaced["sessionId"]
        .as_str()
        .ok_or("no fresh Pi session")?
        .to_string();
    assert_ne!(old, new);
    assert_eq!(replaced["messages"], json!([]));
    assert_eq!(replaced["subscriptions"], 2);
    assert_eq!(replaced["observationBinding"]["sessionId"], new);
    assert_eq!(replaced["observationBinding"]["contextWindow"], 32768);
    assert_eq!(replaced["observationBinding"]["collectionSubscriptions"], 1);
    assert_eq!(replaced["observationBinding"]["pressureSubscriptions"], 1);
    assert_eq!(replaced["result"]["cancelled"], false);
    store.bind_native_thread(&mut successor, ProviderThreadId::parse(new.clone())?)?;
    let rev = store.subject_projection(&subject, None, None)?.revision;
    store.apply_transition_fact(
        &transition,
        Revision::new(rev),
        TransitionFact::SuccessorObserved {
            evidence: evidence("new-session"),
            context: ContextGeneration::parse("context:new")?,
            thread: ProviderThreadId::parse(new.clone())?,
        },
        None,
        clock(),
    )?;
    assert!(
        claim(
            &mut store,
            &successor,
            "wrong-rehydration",
            "wrong checkpoint",
            NativeInvocationPurpose::SuccessorRehydrate,
            "grant:new"
        )
        .is_err()
    );
    let mut recovery_turn = claim(
        &mut store,
        &successor,
        "rehydrate",
        &checkpoint,
        NativeInvocationPurpose::SuccessorRehydrate,
        "grant:new",
    )?;
    let recovery_proof = adapter
        .collect_prompt(
            &mut store,
            successor.id(),
            &mut recovery_turn,
            &checkpoint,
            "Fixture checkpoint received; unanswered human remains unresolved.",
            clock(),
        )
        .await?;
    // No success on missing or shifted immutable coordinates.
    let mut missing = recovery_proof.clone();
    missing.history_sequence -= 1;
    assert!(
        store
            .settle_pi_turn(&recovery_turn, &missing, clock())
            .is_err()
    );
    store.settle_pi_turn(&recovery_turn, &recovery_proof, clock())?;
    let receipt = store
        .native_rehydration_receipt(&transition)?
        .ok_or("owner lacks rehydration receipt")?;
    for fact in [
        TransitionFact::RehydrationVerified {
            evidence: EvidenceId::parse(format!("{receipt}:rehydration"))?,
        },
        TransitionFact::CommitSuccessor {
            evidence: evidence("commit"),
            semantic_revision: Revision::new(semantic_revision),
        },
    ] {
        let rev = store.subject_projection(&subject, None, None)?.revision;
        store.apply_transition_fact(
            &transition,
            Revision::new(rev),
            fact,
            Some(&GrantId::parse("grant:successor")?),
            clock(),
        )?;
    }
    let before = store.subject_projection(&subject, None, None)?;
    assert_eq!(before.context_id, "context:new");
    assert_eq!(before.owner_kind, "domain");
    drop(store);
    let mut store = SqliteLifecycleStore::open(root, "fixture-root".into())?;
    let after = store.subject_projection(&subject, None, None)?;
    assert_eq!(before, after);
    assert!(
        adapter
            .replacement_ready(&mut store, &subject, &transition, &checkpoint)
            .is_err()
    );
    let queued = store.prepare_next_input(&subject)?;
    let original = match &queued.input {
        EffectInput::ControlledText(input) => input.input_id().as_str(),
    };
    assert_eq!(original, "input:q0");
    let exit = adapter.close().await?;
    assert!(exit.success);
    assert!(!exit.forced);
    let report = json!({"profile":"pi-1.5.0-runtime-fresh-session-v1","fixture":true,"semantic_adequacy":"not_claimed","old_physical_session":old,"new_physical_session":new,"logical_subject":subject.as_str(),"context":after.context_id,"owner":after.owner_kind,"checkpoint_sha256":published.digest_hex,"recovery_receipt":receipt,"same_state_reopen_equal":true,"first_queued_input":original,"native_child_exit":format!("{exit:?}"),"unanswered_human":true});
    std::fs::write(
        "/work/qualification-result.json",
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{report}");
    Ok(())
}
