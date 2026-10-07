use lifecycle_core::{
    AdmissionOwner, AttemptFacts, AttemptId, AuthorityExpiresAt, BuildId, CheckpointId,
    ClockSample, Command, CommandAdmission, CommandId, CommandOutcome, CommandRequest,
    ContextGeneration, ControlledTextInput, CustodyState, DeliveryId, EffectInput,
    EffectObservation, EffectPlan, EffectSettlement, EntryFacts, EvidenceId, ExecutionOutcome,
    GrantId, GrantScope, InputId, ProofRunId, ProviderThreadId, ProviderTurnId, QueueInput,
    Revision, RuntimeIncarnation, SubjectId, SubjectState, TransitionFact, TransitionFailure,
    TransitionId, TransitionStage, TransitionState, TrustedGrant, WaitBudgetMs, WallTimeMs,
    reduce_command, reduce_entry, reduce_observation, reduce_queue, reduce_transition,
    reduce_transition_custody,
};
use proptest::prelude::*;
use serde_json::json;
use work_engine_types::CodecContract;

fn subject() -> SubjectState {
    let context = ContextGeneration::parse("context-a").unwrap();
    SubjectState {
        subject: SubjectId::parse("subject-a").unwrap(),
        context: context.clone(),
        build: BuildId::parse("build-a").unwrap(),
        proof_run: ProofRunId::parse("proof-a").unwrap(),
        revision: Revision::new(0),
        semantic_revision: Revision::new(0),
        queue_revision: Revision::new(0),
        last_wall: WallTimeMs::new(100),
        owner: AdmissionOwner::Domain {
            generation: context,
        },
    }
}

fn command(index: usize, replacement: bool, expected: Revision) -> CommandAdmission {
    let command_id = CommandId::parse(format!("command-{index}")).unwrap();
    let input_id = InputId::parse(format!("input-{index}")).unwrap();
    let text = format!("work-{index}");
    let text_digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap();
    let (kind, payload, input, command) = if replacement {
        (
            "request_replacement",
            json!({"reason":"model"}),
            None,
            Command::RequestReplacement,
        )
    } else {
        (
            "enqueue_input",
            json!({"input_id":input_id.as_str(),"producer_ref":"model","text":text,"text_digest":text_digest.hex()}),
            Some(ControlledTextInput::new(input_id.clone(), text, text_digest).unwrap()),
            Command::EnqueueInput { input: input_id },
        )
    };
    let basis = json!({
        "protocol_version":1,"principal_ref":"model-principal","command_id":command_id.as_str(),
        "subject_id":"subject-a","context_generation":"context-a","build_id":"build-a",
        "proof_run_id":"proof-a","grant_ref":"model-grant","expected_revision":expected.get().to_string(),
        "kind":kind,"payload":payload
    });
    let request = CommandRequest::new(
        command_id,
        SubjectId::parse("subject-a").unwrap(),
        ContextGeneration::parse("context-a").unwrap(),
        BuildId::parse("build-a").unwrap(),
        ProofRunId::parse("proof-a").unwrap(),
        GrantId::parse("model-grant").unwrap(),
        expected,
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap(),
        command,
    )
    .unwrap();
    CommandAdmission::new(request, "model-principal".into(), basis, input).unwrap()
}

fn grant(replacement: bool) -> TrustedGrant {
    TrustedGrant {
        id: GrantId::parse("model-grant").unwrap(),
        issuer: "trusted".into(),
        principal: "model-principal".into(),
        subject: SubjectId::parse("subject-a").unwrap(),
        context: ContextGeneration::parse("context-a").unwrap(),
        build: BuildId::parse("build-a").unwrap(),
        proof_run: ProofRunId::parse("proof-a").unwrap(),
        scope: if replacement {
            GrantScope::RequestReplacement
        } else {
            GrantScope::EnqueueInput
        },
        expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
        revision: Revision::new(1),
        revoked: false,
    }
}

#[test]
fn conflicting_terminal_observation_preserves_prior_fact_and_custody() {
    let current = subject();
    let prior = AttemptFacts {
        incarnation: RuntimeIncarnation::parse("incarnation-model").unwrap(),
        provider_thread: Some(ProviderThreadId::parse("thread-model").unwrap()),
        provider_turn: Some(ProviderTurnId::parse("turn-model").unwrap()),
        outcome: ExecutionOutcome::Failed,
        settlement: EffectSettlement::Unresolved,
        input_state: CustodyState::FailedBlocked,
        attempt_entered: false,
    };
    let source = EvidenceId::parse("source-contradiction").unwrap();
    let observation = EffectObservation {
        source: source.clone(),
        effect: lifecycle_core::EffectId::parse("effect-model").unwrap(),
        attempt: AttemptId::parse("attempt-model").unwrap(),
        incarnation: prior.incarnation.clone(),
        provider_thread: prior.provider_thread.clone().unwrap(),
        provider_turn: prior.provider_turn.clone().unwrap(),
        outcome: ExecutionOutcome::Completed,
        settlement: EffectSettlement::Established(EvidenceId::parse("settlement-new").unwrap()),
    };
    let reduction = reduce_observation(
        &current,
        &prior,
        &observation,
        ClockSample {
            wall: WallTimeMs::new(101),
            wait_budget: WaitBudgetMs::new(10),
        },
    )
    .unwrap();
    assert!(reduction.conflict);
    assert_eq!(reduction.outcome, ExecutionOutcome::Failed);
    assert_eq!(reduction.settlement, EffectSettlement::Conflict(source));
    assert_eq!(reduction.input_state, CustodyState::FailedBlocked);
    assert_eq!(reduction.provider_thread, prior.provider_thread);
    assert_eq!(reduction.provider_turn, prior.provider_turn);
    assert_eq!(reduction.next_subject.owner, AdmissionOwner::Fenced);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]
    #[test]
    fn generated_commands_preserve_single_owner_and_monotonic_revisions(
        actions in prop::collection::vec((any::<bool>(), any::<bool>()), 1..60)
    ) {
        let mut current = subject();
        for (index, (replacement, stale)) in actions.into_iter().enumerate() {
            let expected = Revision::new(current.revision.get() + u64::from(stale));
            let admission = command(index, replacement, expected);
            let reduction = reduce_command(
                &current, &admission, Some(&grant(replacement)),
                ClockSample { wall: WallTimeMs::new(101 + index as i64), wait_budget: WaitBudgetMs::new(10) },
                DeliveryId::parse(format!("delivery-{index}")).unwrap(),
                TransitionId::parse(format!("transition-{index}")).unwrap(), false,
            );
            prop_assert!(reduction.next_subject.revision >= current.revision);
            prop_assert!(reduction.next_subject.queue_revision >= current.queue_revision);
            prop_assert!(reduction.next_subject.semantic_revision >= current.semantic_revision);
            prop_assert!(reduction.next_subject.queue_revision <= reduction.next_subject.revision);
            prop_assert!(reduction.next_subject.semantic_revision <= reduction.next_subject.revision);
            if stale {
                prop_assert!(matches!(reduction.result.outcome, CommandOutcome::Rejected(_)));
            }
            if matches!(current.owner, AdmissionOwner::Transition { .. }) && !replacement {
                prop_assert!(matches!(reduction.result.outcome, CommandOutcome::Rejected(_)));
            }
            current = reduction.next_subject;
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(192))]
    #[test]
    fn generated_lifecycle_events_keep_fifo_no_resend_and_monotonic_fact_state(
        actions in prop::collection::vec(0u8..9, 1..90)
    ) {
        let mut current = subject();
        let mut queue: Vec<QueueInput> = Vec::new();
        let mut plan: Option<EffectPlan> = None;
        let mut active: Option<(EffectPlan, AttemptId, AttemptFacts, bool)> = None;
        let mut entered_inputs: Vec<InputId> = Vec::new();
        for (index, action) in actions.into_iter().enumerate() {
            let previous_revision = current.revision;
            let previous_semantic = current.semantic_revision;
            let previous_queue = current.queue_revision;
            let wall = ClockSample { wall:WallTimeMs::new(101 + index as i64), wait_budget:WaitBudgetMs::new(10) };
            match action {
                0 | 1 => {
                    let admission = command(index,false,current.revision);
                    let delivery = DeliveryId::parse(format!("delivery-{index}")).unwrap();
                    let reduction = reduce_command(&current,&admission,Some(&grant(false)),wall,
                        delivery.clone(),TransitionId::parse(format!("transition-{index}")).unwrap(),false);
                    if let CommandOutcome::Enqueued { .. } = reduction.result.outcome {
                        queue.push(QueueInput { input:admission.input().unwrap().clone(),delivery,
                            sequence:queue.len() as u64 + 1,state:CustodyState::Queued,prepared_effect:None });
                    }
                    current = reduction.next_subject;
                }
                2 => {
                    if let Ok(reduction) = reduce_queue(&current,&queue) {
                        let input = match &reduction.plan.input { EffectInput::ControlledText(input) => input.input_id() };
                        let selected = queue.iter_mut().find(|item| item.input.input_id()==input).unwrap();
                        if reduction.insert_effect {
                            selected.state = CustodyState::Prepared;
                            selected.prepared_effect = Some((reduction.plan.effect.clone(),reduction.plan.expected_revision));
                        }
                        plan = Some(reduction.plan);
                    }
                }
                3 => {
                    if let Some(selected) = plan.as_ref() {
                        let input = match &selected.input { EffectInput::ControlledText(input) => input.input_id() };
                        let item = queue.iter().find(|item| item.input.input_id()==input).unwrap();
                        let earlier = queue.iter().any(|other| other.sequence<item.sequence
                            && !matches!(other.state,CustodyState::Succeeded | CustodyState::Retired));
                        if reduce_entry(&current,selected,EntryFacts {
                            effect_state:CustodyState::Prepared,input_state:item.state,
                            grant:&grant(false),trusted_issuer:"trusted",clock:wall,
                            earlier_unresolved:earlier,
                        }).is_ok() {
                            prop_assert!(!entered_inputs.contains(input));
                            entered_inputs.push(input.clone());
                            queue.iter_mut().find(|item| item.input.input_id()==input).unwrap().state=CustodyState::Entered;
                            let facts = AttemptFacts { incarnation:RuntimeIncarnation::parse(format!("incarnation-{index}")).unwrap(),
                                provider_thread:None,provider_turn:None,outcome:ExecutionOutcome::Pending,
                                settlement:EffectSettlement::Unresolved,input_state:CustodyState::Entered,attempt_entered:true };
                            active = Some((selected.clone(),AttemptId::parse(format!("attempt-{index}")).unwrap(),facts,true));
                            plan=None;
                        }
                    }
                }
                4..=7 => {
                    if let Some((selected,attempt,facts,running)) = active.as_mut() {
                        let outcome = match action {
                            4 => ExecutionOutcome::Pending,
                            5 => ExecutionOutcome::Completed,
                            6 => ExecutionOutcome::Failed,
                            _ if !*running && facts.outcome==ExecutionOutcome::Completed => ExecutionOutcome::Failed,
                            _ if !*running && facts.outcome==ExecutionOutcome::Failed => ExecutionOutcome::Completed,
                            _ => ExecutionOutcome::Pending,
                        };
                        if *running || action==7 && outcome!=ExecutionOutcome::Pending {
                            let settlement = if outcome==ExecutionOutcome::Completed {
                                EffectSettlement::Established(EvidenceId::parse(format!("settlement-{index}")).unwrap())
                            } else { EffectSettlement::Unresolved };
                            let observation = EffectObservation { source:EvidenceId::parse(format!("source-{index}")).unwrap(),
                                effect:selected.effect.clone(),attempt:attempt.clone(),incarnation:facts.incarnation.clone(),
                                provider_thread:ProviderThreadId::parse("thread-model").unwrap(),
                                provider_turn:ProviderTurnId::parse("turn-model").unwrap(),outcome,settlement };
                            let reduction=reduce_observation(&current,facts,&observation,wall).unwrap();
                            if action==7 && !*running { prop_assert!(reduction.conflict); }
                            if reduction.conflict {
                                let input=match &selected.input { EffectInput::ControlledText(input)=>input.input_id() };
                                let old_custody=queue.iter().find(|item|item.input.input_id()==input).unwrap().state;
                                prop_assert_eq!(&reduction.outcome,&facts.outcome);
                                prop_assert!(matches!(reduction.settlement,EffectSettlement::Conflict(_)));
                                prop_assert_eq!(reduction.input_state,old_custody);
                            }
                            current=reduction.next_subject;
                            facts.provider_thread=reduction.provider_thread;
                            facts.provider_turn=reduction.provider_turn;
                            facts.outcome=reduction.outcome;
                            facts.settlement=reduction.settlement;
                            facts.input_state=reduction.input_state;
                            facts.attempt_entered=reduction.still_running;
                            *running=reduction.still_running;
                            let input=match &selected.input { EffectInput::ControlledText(input)=>input.input_id() };
                            queue.iter_mut().find(|item|item.input.input_id()==input).unwrap().state=reduction.input_state;
                        }
                    }
                }
                8 => {
                    if plan.is_some() && !active.as_ref().is_some_and(|(_,_,_,running)| *running) {
                        let admission=command(index,true,current.revision);
                        let reduction=reduce_command(&current,&admission,Some(&grant(true)),wall,
                            DeliveryId::parse(format!("unused-{index}")).unwrap(),
                            TransitionId::parse(format!("transition-{index}")).unwrap(),false);
                        if let CommandOutcome::ReplacementRequested { transition } = reduction.result.outcome {
                            current=reduction.next_subject;
                            let transition_state=TransitionState { id:transition,subject:current.subject.clone(),
                                predecessor:current.context.clone(),stage:TransitionStage::Quiescing,
                                basis_semantic_revision:current.semantic_revision,checkpoint:None,successor:None,
                                successor_thread:None,evidence:Vec::new() };
                            let aborted=reduce_transition(&transition_state,TransitionFact::AbortNoEntry {
                                evidence:EvidenceId::parse(format!("abort-{index}")).unwrap() }).unwrap();
                            prop_assert_eq!(aborted.stage,TransitionStage::AbortedBeforeEntry);
                            for item in &mut queue {
                                let next=reduce_transition_custody(aborted.stage,item.state);
                                if next!=item.state { item.state=next; item.prepared_effect=None; }
                            }
                            current.owner=AdmissionOwner::Domain { generation:current.context.clone() };
                            current.revision=Revision::new(current.revision.get()+1);
                            plan=None;
                        }
                    }
                }
                _ => unreachable!(),
            }
            prop_assert!(current.revision>=previous_revision);
            prop_assert!(current.semantic_revision>=previous_semantic);
            prop_assert!(current.queue_revision>=previous_queue);
            prop_assert!(queue.windows(2).all(|pair|pair[0].sequence<pair[1].sequence));
            prop_assert!(queue.iter().filter(|item|item.state==CustodyState::Entered).count()<=1);
            prop_assert!(entered_inputs.len()<=queue.len());
            if matches!(current.owner,AdmissionOwner::Fenced) {
                prop_assert!(reduce_queue(&current,&queue).is_err());
            }
            if let Some(first) = queue.iter().find(|item| !matches!(item.state,CustodyState::Succeeded|CustodyState::Retired))
                && !matches!(first.state,CustodyState::Queued|CustodyState::Prepared) {
                prop_assert!(reduce_queue(&current,&queue).is_err());
            }
        }
    }
}

#[test]
fn transition_facts_require_stage_and_semantic_basis() {
    let mut state = TransitionState {
        id: TransitionId::parse("transition-a").unwrap(),
        subject: SubjectId::parse("subject-a").unwrap(),
        predecessor: ContextGeneration::parse("context-a").unwrap(),
        stage: TransitionStage::Quiescing,
        basis_semantic_revision: Revision::new(7),
        checkpoint: None,
        successor: None,
        successor_thread: None,
        evidence: Vec::new(),
    };
    let evidence = |value: &str| EvidenceId::parse(value).unwrap();
    assert_eq!(
        reduce_transition(
            &state,
            TransitionFact::CommitSuccessor {
                evidence: evidence("too-early"),
                semantic_revision: Revision::new(7)
            }
        ),
        Err(TransitionFailure::WrongStage)
    );
    state = reduce_transition(
        &state,
        TransitionFact::PredecessorSafe {
            evidence: evidence("safe"),
        },
    )
    .unwrap();
    assert_eq!(
        reduce_transition(
            &state,
            TransitionFact::SourceFrozen {
                evidence: evidence("stale"),
                semantic_revision: Revision::new(8)
            }
        ),
        Err(TransitionFailure::StaleSemanticBasis)
    );
    state = reduce_transition(
        &state,
        TransitionFact::SourceFrozen {
            evidence: evidence("source"),
            semantic_revision: Revision::new(7),
        },
    )
    .unwrap();
    state = reduce_transition(
        &state,
        TransitionFact::VerificationEstablished {
            evidence: evidence("verified"),
            checkpoint: CheckpointId::parse("checkpoint-a").unwrap(),
        },
    )
    .unwrap();
    state = reduce_transition(
        &state,
        TransitionFact::SwitchAuthorized {
            evidence: evidence("switch"),
        },
    )
    .unwrap();
    state = reduce_transition(
        &state,
        TransitionFact::SuccessorObserved {
            evidence: evidence("successor"),
            context: ContextGeneration::parse("context-b").unwrap(),
            thread: ProviderThreadId::parse("thread-b").unwrap(),
        },
    )
    .unwrap();
    state = reduce_transition(
        &state,
        TransitionFact::RehydrationVerified {
            evidence: evidence("rehydrated"),
        },
    )
    .unwrap();
    state = reduce_transition(
        &state,
        TransitionFact::CommitSuccessor {
            evidence: evidence("commit"),
            semantic_revision: Revision::new(7),
        },
    )
    .unwrap();
    assert_eq!(state.stage, TransitionStage::Reconciled);
    assert_eq!(state.evidence.len(), 7);
}
