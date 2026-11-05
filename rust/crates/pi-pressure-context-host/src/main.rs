use lifecycle_core::*;
use lifecycle_store::*;
use lifecycle_store::{
    pi_multiturn_profile_digest as pi_profile_digest,
    pi_multiturn_protocol_digest as pi_protocol_digest,
};
use pi_context_transition::{Error, PiAdapter};
use pi_pressure_context_host::{FencedPi, pressure_candidate};
use serde_json::{Value, json};
use std::path::Path;
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
fn grant(
    store: &mut SqliteLifecycleStore,
    id: &str,
    context: &str,
    scope: GrantScope,
    expires_ms: i64,
) {
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
            expires: AuthorityExpiresAt::new(WallTimeMs::new(expires_ms)),
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
    store.claim_native_turn(
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
    )
}
async fn lane(
    root: &Path,
    output: &Path,
    container: &str,
    name: &str,
    expires: i64,
) -> Result<Value, Error> {
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
        grant(&mut store, id, context, scope, expires);
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
    let mut command = tokio::process::Command::new("/usr/bin/docker");
    command
        .args([
            "exec",
            "-i",
            "-w",
            &format!("/work/{name}"),
            "-e",
            &format!("PI_FIXTURE_ROOT=/work/{name}"),
            "-e",
            &format!("HOME=/work/{name}/home"),
            container,
            "/usr/local/bin/node",
            "/subject/fixture.mjs",
        ])
        .env_clear()
        .env("PATH", "/usr/bin:/bin");
    let raw = PiAdapter::launch_multiturn(&mut command, format!("transport:pi:{name}"))?;
    let mut adapter = FencedPi::new(raw, expires);
    let outcome: Result<Value,Error> = async {
    let initial_intent = allocate(
        &mut store,
        "session:old",
        "context:old",
        &published.digest_hex,
    );
    let (mut session, ready) = adapter.initialize(&mut store, initial_intent, clock()).await?;
    let old = ready["sessionId"]
        .as_str()
        .ok_or("no Pi session")?
        .to_string();
    store.bind_native_thread(&mut session, ProviderThreadId::parse(old.clone())?)?;
    let actual = adapter.observe().await?;
    assert_eq!(actual["sessionId"], old);
    assert_eq!(actual["idle"], true);
    let pressure = json!({"schemaVersion":1,"observationId":format!("fixture:{name}:1"),"sequence":1,"observedAt":actual["observedAt"],"pressureBasisPoints":9500,"source":"explicit outside injected fixture; NOT SDK usage","sourceRevision":format!("sha256:{}",CodecContract::BinaryArtifactV1.digest_binary(b"outside-injected-pressure-v1")?.hex())});
    let envelope = json!({"fixture":true,"authority":"observation_only","physical_session":old,"generation":"context:old","pressure":pressure,"budget":{"producer":"explicit fixture control","state":"available"}}).to_string();
    std::fs::write(output.join("outside-pressure.json"), &envelope)?;
    std::fs::write(output.join("actual-sdk-observation.json"),serde_json::to_vec_pretty(&actual)?)?;
    let prompt_owned = format!("OLD_CONVERSATION_CANARY_7f39. Outside fixture pressure: {envelope}. Produce fixture checkpoint data only.");
    let prompt = prompt_owned.as_str();
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
    store.settle_pi_multiturn(&turn, &proof, clock())?;
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
    assert!(store.settle_pi_multiturn(&turn, &proof, clock()).is_err());
    if !pressure_candidate(Some(&pressure))? { return Err("no outside pressure candidate".into()); }
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
        .replace(&mut store, &subject, &transition, &checkpoint, intent, clock())
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
    if name != "happy" {
        let trigger=json!({"fixture":true,"origin":"outer-host","control":name,"before":"successor recovery send"});
        std::fs::write(output.join("outside-stop-control.json"),serde_json::to_vec_pretty(&trigger)?)?;
        FencedPi::stop(&mut store,clock())?;
        assert!(adapter.collect_prompt(&mut store,successor.id(),&mut recovery_turn,&checkpoint,"MUST NOT SEND",clock()).await.is_err());
        let uncertain = store.native_recovery("rehydrate")?.ok_or("missing held recovery")?;
        assert_eq!(uncertain.class,"EnteredUncertain");
        assert!(!uncertain.send_intent);
        drop(store);
        let mut reopened=SqliteLifecycleStore::open(root,"fixture-root".into())?;
        assert!(FencedPi::check(&mut reopened,expires,clock()).is_err());
        assert!(!reopened.native_recovery("rehydrate")?.ok_or("missing reopened recovery")?.send_intent);
        return Ok(json!({"lane":name,"status":"stopped_before_recovery","preissued_permit_denied":true,"reopen_stop_retained":true,"recovery_send_intent":false,"provider_requests":1,"resets":1,"old_physical_session":old,"new_physical_session":new}));
    }
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
            .settle_pi_multiturn(&recovery_turn, &missing, clock())
            .is_err()
    );
    store.settle_pi_multiturn(&recovery_turn, &recovery_proof, clock())?;
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
    let continued_prompt="Continue fake-provider domain work after durable recovery: compute 6*7.";
    let mut continued=claim(&mut store,&successor,"continued",continued_prompt,NativeInvocationPurpose::DomainWork,"grant:new")?;
    let continued_proof=adapter.collect_prompt(&mut store,successor.id(),&mut continued,continued_prompt,"42; continued fixture domain work.",clock()).await?;
    store.settle_pi_multiturn(&continued,&continued_proof,clock())?;
    let continued_state=store.native_recovery("continued")?.ok_or("no continued result")?;
    assert_eq!(continued_state.class,"CompletedSettled");
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
    let report = json!({"profile":"pi-1.5.0-runtime-anchored-turns-v2","fixture":true,"semantic_adequacy":"not_claimed","old_physical_session":old,"new_physical_session":new,"logical_subject":subject.as_str(),"context":after.context_id,"owner":after.owner_kind,"checkpoint_sha256":published.digest_hex,"recovery_receipt":receipt,"same_state_reopen_equal":true,"first_queued_input":original,"continued_domain_work":true,"provider_requests":3,"resets":1,"unanswered_human":true});
    Ok(report)
    }.await;
    let exit = if outcome.is_ok() {
        adapter.close().await?
    } else {
        adapter.terminate().await?
    };
    std::fs::write(
        output.join("wrapper-exit.json"),
        serde_json::to_vec_pretty(
            &json!({"observed":format!("{exit:?}"),"success":exit.success,"forced":exit.forced}),
        )?,
    )?;
    if outcome.is_ok() && (!exit.success || exit.forced) {
        return Err("SDK wrapper exit unsuccessful".into());
    }
    outcome
}
#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();
    let root = Path::new(args.get(1).ok_or("output root required")?);
    let container = args.get(2).ok_or("admitted container required")?;
    let expires: i64 = args
        .get(3)
        .ok_or("immutable job expiry required")?
        .parse()?;
    for name in ["happy", "cancel", "exhausted"] {
        let output = root.join("reports").join(name);
        std::fs::create_dir_all(&output)?;
        let owner = root.join("owners").join(name);
        let result = lane(&owner, &output, container, name, expires).await;
        match result {
            Ok(value) => std::fs::write(
                output.join("result.json"),
                serde_json::to_vec_pretty(&value)?,
            )?,
            Err(error) => {
                std::fs::write(output.join("failure.txt"), error.to_string())?;
                return Err(error);
            }
        }
    }
    Ok(())
}
