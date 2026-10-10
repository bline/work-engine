use crate::process::rpc;
use crate::provider::{ProviderCommand, ProviderReply, ProviderRequestMessage};
use crate::storage::Store;
use crate::types::*;
use anyhow::{Result, ensure};
use serde_json::json;
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub trait Clock: Send + Sync {
    fn now(&self) -> u64;
}
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_millis() as u64
    }
}

enum Work {
    Enter(EffectId),
    Send(EffectId),
    Apply(EffectId, Option<Box<ProviderReply>>),
}

/// Sole writer and executor. A transport actor owns this independently of observers.
pub struct Coordinator {
    store: Store,
    state: State,
    provider: SocketAddr,
    clock: Arc<dyn Clock>,
    pause_at: Option<Stage>,
    barrier: Option<Milestone>,
    work: Option<Work>,
    dispatch_done: BTreeSet<EffectId>,
}
impl Coordinator {
    pub fn open(
        path: &Path,
        provider: SocketAddr,
        clock: Arc<dyn Clock>,
        pause_at: Option<Stage>,
    ) -> Result<Self> {
        let mut store = Store::open(path)?;
        let mut state: State = store.load()?;
        let mut changed = false;
        for effect in state.effects.values_mut() {
            if effect.state == EffectState::Entered {
                effect.state = EffectState::Unknown;
                state
                    .deliveries
                    .get_mut(&effect.delivery)
                    .expect("effect delivery")
                    .outcome = DeliveryOutcome::Unknown;
                changed = true;
            }
        }
        if changed {
            state.revision += 1;
            store.save(&state)?;
        }
        Ok(Self {
            store,
            state,
            provider,
            clock,
            pause_at,
            barrier: None,
            work: None,
            dispatch_done: BTreeSet::new(),
        })
    }
    fn commit(&mut self, mut next: State) -> Result<()> {
        next.revision += 1;
        self.store.save(&next)?;
        self.state = next;
        Ok(())
    }
    fn ordinary(state: &State, owner: &Stamp) -> Result<()> {
        let context = state
            .contexts
            .get(&owner.context)
            .ok_or_else(|| anyhow::anyhow!("unknown context"))?;
        ensure!(
            context.epoch == owner.epoch && context.active,
            "stale or revoked owner"
        );
        ensure!(!context.fenced, "context fenced by conflicting evidence");
        Ok(())
    }
    fn permit<'a>(state: &'a State, permit: &CapturePermit) -> Result<&'a Transition> {
        let transition = state
            .transitions
            .get(&permit.transition)
            .ok_or_else(|| anyhow::anyhow!("unknown transition"))?;
        let context = state
            .contexts
            .get(&permit.context)
            .ok_or_else(|| anyhow::anyhow!("unknown context"))?;
        ensure!(
            transition.permit == *permit
                && !transition.completed
                && !context.active
                && context.epoch == permit.epoch
                && context.transition.as_ref() == Some(&permit.transition),
            "invalid capture permit"
        );
        Ok(transition)
    }
    pub fn execute(&mut self, command: Command) -> Result<Reply> {
        if let Some(prior) = self.state.commands.get(&command.key) {
            ensure!(prior.command == command, "command key conflict");
            return prior.reply.clone().map_err(anyhow::Error::msg);
        }
        ensure!(!command.key.0.is_empty(), "empty command identity");
        let mut next = self.state.clone();
        let now = self.clock.now();
        let reply: Result<Reply> = (|| {
            let reply = match &command.action {
                Action::Register { context } => {
                    ensure!(
                        !context.0.is_empty() && !next.contexts.contains_key(context),
                        "context already exists or is empty"
                    );
                    let stamp = Stamp {
                        context: context.clone(),
                        epoch: Epoch(1),
                    };
                    next.contexts.insert(
                        context.clone(),
                        Context {
                            id: context.clone(),
                            epoch: stamp.epoch,
                            active: true,
                            fenced: false,
                            queue: vec![],
                            transition: None,
                        },
                    );
                    Reply::Owner(stamp)
                }
                Action::Enqueue {
                    owner,
                    input,
                    payload,
                    lineage,
                } => {
                    Self::ordinary(&next, owner)?;
                    ensure!(
                        !input.0.is_empty() && !lineage.is_empty(),
                        "input identity and lineage required"
                    );
                    ensure!(
                        !next
                            .contexts
                            .values()
                            .any(|c| c.queue.iter().any(|i| i.id == *input)),
                        "input identity reused"
                    );
                    next.contexts
                        .get_mut(&owner.context)
                        .expect("checked context")
                        .queue
                        .push(Input {
                            id: input.clone(),
                            payload: payload.clone(),
                            digest: digest(payload),
                            lineage: lineage.clone(),
                            delivery: None,
                        });
                    Reply::Recorded
                }
                Action::Admit {
                    owner,
                    input,
                    delivery,
                    effect,
                    authority,
                } => {
                    Self::ordinary(&next, owner)?;
                    ensure!(
                        now < authority.deadline && !authority.approval.is_empty(),
                        "synthetic authority missing or expired"
                    );
                    ensure!(
                        !next
                            .effects
                            .values()
                            .any(|e| e.context == owner.context && e.state.unresolved()),
                        "unresolved delivery blocks FIFO"
                    );
                    ensure!(
                        !delivery.0.is_empty()
                            && !effect.0.is_empty()
                            && !next.deliveries.contains_key(delivery)
                            && !next.effects.contains_key(effect),
                        "delivery/effect identity reused or empty"
                    );
                    let context = next
                        .contexts
                        .get_mut(&owner.context)
                        .expect("checked context");
                    let queued = context
                        .queue
                        .iter_mut()
                        .find(|i| i.delivery.is_none())
                        .ok_or_else(|| anyhow::anyhow!("queue empty"))?;
                    ensure!(queued.id == *input, "FIFO input overtaking rejected");
                    ensure!(
                        queued.digest == digest(&queued.payload),
                        "input integrity failure"
                    );
                    queued.delivery = Some(delivery.clone());
                    let request = ProviderRequest {
                        effect: effect.clone(),
                        context: owner.context.clone(),
                        epoch: owner.epoch,
                        delivery: delivery.clone(),
                        input: input.clone(),
                        payload: queued.payload.clone(),
                        input_digest: queued.digest.clone(),
                        lineage: queued.lineage.clone(),
                        authority: authority.clone(),
                        purpose: EffectPurpose::Input,
                        profile_digest: crate::codex::profile_digest_for(&authority.mode)
                            .unwrap_or_default(),
                    };
                    let domain = match authority.mode {
                        Mode::ObserveCodex => EvidenceDomain::CodexObservation,
                        Mode::CodexText | Mode::CodexTextLive => {
                            EvidenceDomain::CodexTextTurnCompletion
                        }
                        _ => EvidenceDomain::FakeSeal,
                    };
                    next.effects.insert(
                        effect.clone(),
                        Effect {
                            id: effect.clone(),
                            context: owner.context.clone(),
                            delivery: delivery.clone(),
                            request_digest: digest(&request),
                            request,
                            domain,
                            state: EffectState::Prepared,
                            evidence: vec![],
                            codex: None,
                        },
                    );
                    next.deliveries.insert(
                        delivery.clone(),
                        Delivery {
                            id: delivery.clone(),
                            input: input.clone(),
                            effect: effect.clone(),
                            admitted_epoch: owner.epoch,
                            outcome: DeliveryOutcome::Pending,
                        },
                    );
                    Reply::Ticket(effect.clone())
                }
                Action::Replace {
                    owner,
                    transition,
                    deadline,
                } => {
                    Self::ordinary(&next, owner)?;
                    if next.effects.values().any(|effect| {
                        effect.context == owner.context
                            && effect.domain == EvidenceDomain::CodexTextTurnCompletion
                    }) {
                        ensure!(
                            !next.effects.values().any(|effect| {
                                effect.context == owner.context && effect.state.unresolved()
                            }),
                            "unresolved work blocks native text replacement"
                        );
                    }
                    ensure!(
                        now < *deadline
                            && !transition.0.is_empty()
                            && !next.transitions.contains_key(transition),
                        "transition identity reused, empty, or deadline expired"
                    );
                    let permit = CapturePermit {
                        context: owner.context.clone(),
                        epoch: owner.epoch,
                        transition: transition.clone(),
                        deadline: *deadline,
                    };
                    let context = next
                        .contexts
                        .get_mut(&owner.context)
                        .expect("checked context");
                    context.active = false;
                    context.transition = Some(transition.clone());
                    next.transitions.insert(
                        transition.clone(),
                        Transition {
                            permit: permit.clone(),
                            capture: CaptureState::Pending,
                            continuation: None,
                            attestation: None,
                            completed: false,
                            readiness_package: None,
                            readiness_effect: None,
                            limited_adequacy: None,
                        },
                    );
                    Reply::Capture(permit)
                }
                Action::Capture {
                    permit,
                    continuation,
                } => {
                    let transition = Self::permit(&next, permit)?;
                    ensure!(
                        now < permit.deadline && transition.capture == CaptureState::Pending,
                        "capture closed or expired"
                    );
                    ensure!(
                        !continuation.objective.trim().is_empty()
                            && !continuation.authority.trim().is_empty()
                            && !continuation.references.is_empty()
                            && continuation.references.iter().all(|r| !r.trim().is_empty()),
                        "continuation meaning fields required"
                    );
                    let transition = next
                        .transitions
                        .get_mut(&permit.transition)
                        .expect("checked transition");
                    transition.continuation = Some(continuation.clone());
                    transition.capture = CaptureState::Ready;
                    Reply::Recorded
                }
                Action::BeginReadiness {
                    permit,
                    effect,
                    delivery,
                    authority,
                } => {
                    let transition = Self::permit(&next, permit)?;
                    ensure!(
                        transition.capture == CaptureState::Ready
                            && transition.readiness_effect.is_none(),
                        "readiness unavailable or already entered"
                    );
                    ensure!(
                        matches!(authority.mode, Mode::CodexText | Mode::CodexTextLive)
                            && now < authority.deadline
                            && !authority.approval.is_empty(),
                        "native text readiness authority required"
                    );
                    ensure!(
                        !next.effects.contains_key(effect)
                            && !next.deliveries.contains_key(delivery)
                            && !effect.0.is_empty()
                            && !delivery.0.is_empty(),
                        "readiness identities reused"
                    );
                    ensure!(
                        !next
                            .effects
                            .values()
                            .any(|candidate| candidate.context == permit.context
                                && candidate.state.unresolved()),
                        "unresolved work blocks readiness"
                    );
                    let package = Self::package(&next, &permit.transition)?;
                    let package_digest = digest(&package);
                    let fact = package
                        .continuation
                        .references
                        .first()
                        .expect("capture checked references");
                    let expected_ack = format!("READY:{}:{}", package_digest, fact);
                    let payload = serde_json::to_vec(
                        &json!({"kind":"native-readiness","package":package,"expected_ack":expected_ack}),
                    )?;
                    let input = InputId::new(format!("readiness:{}", permit.transition.0));
                    let request = ProviderRequest {
                        effect: effect.clone(),
                        context: permit.context.clone(),
                        epoch: package.successor,
                        delivery: delivery.clone(),
                        input: input.clone(),
                        input_digest: digest(&payload),
                        payload,
                        lineage: format!("transition:{}", permit.transition.0),
                        authority: authority.clone(),
                        purpose: EffectPurpose::Readiness {
                            transition: permit.transition.clone(),
                            package_revision: package.revision,
                            package_digest,
                            expected_ack,
                        },
                        profile_digest: crate::codex::profile_digest_for(&authority.mode)
                            .expect("native text readiness mode"),
                    };
                    next.effects.insert(
                        effect.clone(),
                        Effect {
                            id: effect.clone(),
                            context: permit.context.clone(),
                            delivery: delivery.clone(),
                            request_digest: digest(&request),
                            request,
                            domain: EvidenceDomain::CodexTextTurnCompletion,
                            state: EffectState::Prepared,
                            evidence: vec![],
                            codex: None,
                        },
                    );
                    next.deliveries.insert(
                        delivery.clone(),
                        Delivery {
                            id: delivery.clone(),
                            input,
                            effect: effect.clone(),
                            admitted_epoch: package.successor,
                            outcome: DeliveryOutcome::Pending,
                        },
                    );
                    let transition = next
                        .transitions
                        .get_mut(&permit.transition)
                        .expect("transition");
                    transition.readiness_package = Some(package);
                    transition.readiness_effect = Some(effect.clone());
                    Reply::Ticket(effect.clone())
                }
                Action::EndCapture { permit, outcome } => {
                    ensure!(
                        Self::permit(&next, permit)?.capture == CaptureState::Pending,
                        "capture already closed"
                    );
                    next.transitions
                        .get_mut(&permit.transition)
                        .expect("checked transition")
                        .capture = match outcome {
                        CaptureEnd::Failed(reason) => CaptureState::Failed(reason.clone()),
                        CaptureEnd::Unknown(reason) => CaptureState::Unknown(reason.clone()),
                    };
                    Reply::Recorded
                }
                Action::Activate { attestation } => {
                    let package = Self::package(&next, &attestation.transition)?;
                    let transition = next
                        .transitions
                        .get(&attestation.transition)
                        .expect("package transition");
                    ensure!(
                        transition.readiness_effect.is_none(),
                        "native readiness requires ActivateText"
                    );
                    ensure!(
                        now < transition.permit.deadline && !transition.completed,
                        "activation closed or expired"
                    );
                    ensure!(
                        package.context == attestation.context
                            && package.successor == attestation.successor
                            && package.revision == attestation.revision
                            && digest(&package) == attestation.digest
                            && !attestation.runtime.0.is_empty()
                            && !attestation.model_context.is_empty(),
                        "readiness identity/revision/digest mismatch"
                    );
                    let context = next
                        .contexts
                        .get_mut(&package.context)
                        .expect("package context");
                    ensure!(!context.fenced, "context fenced");
                    context.active = true;
                    context.epoch = package.successor;
                    let transition = next
                        .transitions
                        .get_mut(&attestation.transition)
                        .expect("package transition");
                    transition.completed = true;
                    transition.attestation = Some(attestation.clone());
                    Reply::Owner(Stamp {
                        context: package.context,
                        epoch: package.successor,
                    })
                }
                Action::ActivateText {
                    attestation,
                    limited_adequacy,
                } => {
                    let transition = next
                        .transitions
                        .get(&attestation.transition)
                        .ok_or_else(|| anyhow::anyhow!("unknown transition"))?;
                    let package = transition
                        .readiness_package
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("native readiness package missing"))?
                        .clone();
                    let readiness_id = transition
                        .readiness_effect
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("native readiness effect missing"))?
                        .clone();
                    let prior_epoch = transition.permit.epoch;
                    let readiness = next.effects.get(&readiness_id).expect("readiness effect");
                    let receipt = readiness
                        .codex
                        .as_ref()
                        .and_then(|observation| observation.text_receipt.as_ref())
                        .cloned();
                    ensure!(
                        readiness.state == EffectState::Success && receipt.is_some(),
                        "readiness turn has no accepted completion"
                    );
                    let receipt = receipt.expect("checked receipt");
                    ensure!(
                        !transition.completed && now < transition.permit.deadline,
                        "activation closed or expired"
                    );
                    ensure!(
                        !limited_adequacy.trim().is_empty(),
                        "explicit limited package-adequacy attestation required"
                    );
                    ensure!(
                        attestation.context == package.context
                            && attestation.successor == package.successor
                            && attestation.revision == package.revision
                            && attestation.digest == digest(&package)
                            && attestation.model_context == receipt.thread_id
                            && !attestation.runtime.0.is_empty(),
                        "native readiness identity/package/thread mismatch"
                    );
                    ensure!(
                        !next
                            .effects
                            .values()
                            .any(|effect| effect.context == package.context
                                && effect.state.unresolved()),
                        "unresolved work blocks activation"
                    );
                    let context = next.contexts.get_mut(&package.context).expect("context");
                    ensure!(
                        !context.fenced && !context.active && context.epoch == prior_epoch,
                        "context not replaceable"
                    );
                    context.active = true;
                    context.epoch = package.successor;
                    let transition = next
                        .transitions
                        .get_mut(&attestation.transition)
                        .expect("transition");
                    transition.completed = true;
                    transition.attestation = Some(attestation.clone());
                    transition.limited_adequacy = Some(limited_adequacy.clone());
                    Reply::Owner(Stamp {
                        context: package.context.clone(),
                        epoch: package.successor,
                    })
                }
                Action::ObserveLocalExit { owner, pid, status } => {
                    let context = next
                        .contexts
                        .get(&owner.context)
                        .ok_or_else(|| anyhow::anyhow!("unknown context"))?;
                    ensure!(
                        owner.epoch.0 > 0 && owner.epoch <= context.epoch && !status.is_empty(),
                        "invalid local exit identity"
                    );
                    let unresolved: Vec<_> = next
                        .effects
                        .values()
                        .filter(|e| e.context == owner.context && e.state.unresolved())
                        .map(|e| e.id.clone())
                        .collect();
                    next.exits.push(LocalExit {
                        context: owner.context.clone(),
                        epoch: owner.epoch,
                        pid: *pid,
                        status: status.clone(),
                        observed_at: now,
                        unsafe_at_time: !unresolved.is_empty(),
                        unresolved,
                    });
                    Reply::Recorded
                }
            };
            Ok(reply)
        })();
        let durable_reply = reply.as_ref().cloned().map_err(ToString::to_string);
        if reply.is_err() {
            // Rejected command identities stay bound without changing semantic revision.
            next = self.state.clone();
        }
        next.commands.insert(
            command.key.clone(),
            JournalEntry {
                command,
                reply: durable_reply,
            },
        );
        if reply.is_ok() {
            self.commit(next)?;
        } else {
            self.store.save(&next)?;
            self.state = next;
        }
        reply
    }
    fn package(state: &State, id: &TransitionId) -> Result<Bootstrap> {
        let transition = state
            .transitions
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("unknown transition"))?;
        Self::permit(state, &transition.permit)?;
        ensure!(
            transition.capture == CaptureState::Ready,
            "continuation not ready"
        );
        let context = state
            .contexts
            .get(&transition.permit.context)
            .expect("permit context");
        Ok(Bootstrap {
            context: context.id.clone(),
            transition: id.clone(),
            successor: Epoch(context.epoch.0 + 1),
            revision: Revision(state.revision),
            continuation: transition.continuation.clone().expect("ready continuation"),
            queue: context.queue.clone(),
            unresolved: state
                .effects
                .values()
                .filter(|e| e.context == context.id && e.state.unresolved())
                .cloned()
                .collect(),
        })
    }
    pub fn bootstrap(&self, id: &TransitionId) -> Result<Bootstrap> {
        Self::package(&self.state, id)
    }
    pub fn result(&self, id: &EffectId) -> Result<Effect> {
        self.state
            .effects
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("unknown effect identity"))
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            revision: Revision(self.state.revision),
            contexts: self.state.contexts.values().cloned().collect(),
            transitions: self.state.transitions.values().cloned().collect(),
            effects: self.state.effects.values().cloned().collect(),
            deliveries: self.state.deliveries.values().cloned().collect(),
            exits: self.state.exits.clone(),
            barrier: self.barrier.clone(),
        }
    }
    pub fn matches(&self, condition: &Condition) -> bool {
        match condition {
            Condition::Barrier => self.barrier.is_some(),
            Condition::Effect(id, state) => self
                .state
                .effects
                .get(id)
                .is_some_and(|e| e.state == *state),
            Condition::RevisionAtLeast(revision) => self.state.revision >= *revision,
            Condition::Native(id, state) => self
                .state
                .effects
                .get(id)
                .and_then(|effect| effect.codex.as_ref())
                .is_some_and(|observation| observation.state == *state),
        }
    }
    pub fn release_barrier(&mut self) {
        self.barrier = None;
    }
    fn pause(&mut self, stage: Stage, effect: &EffectId) {
        if self.pause_at == Some(stage) {
            self.pause_at = None;
            self.barrier = Some(Milestone {
                stage,
                effect: effect.clone(),
            });
        }
    }
    pub(crate) fn ack_ready(&self, id: &EffectId) -> bool {
        self.barrier.is_none()
            && (self.dispatch_done.contains(id)
                || self.state.effects.get(id).is_some_and(|e| {
                    !matches!(e.state, EffectState::Prepared | EffectState::Entered)
                }))
    }
    fn change_outcome(
        &mut self,
        id: &EffectId,
        state: EffectState,
        outcome: DeliveryOutcome,
    ) -> Result<()> {
        if self.state.effects[id].state == state {
            return Ok(());
        }
        let mut next = self.state.clone();
        let effect = next.effects.get_mut(id).expect("effect exists");
        effect.state = state;
        next.deliveries
            .get_mut(&effect.delivery)
            .expect("delivery exists")
            .outcome = outcome;
        self.commit(next)
    }
    fn account(&mut self, id: &EffectId, evidence: Vec<Evidence>) -> Result<()> {
        let mut next = self.state.clone();
        let effect = next.effects.get_mut(id).expect("effect exists");
        let mut changed = false;
        for record in evidence {
            if !effect.evidence.contains(&record) {
                effect.evidence.push(record);
                changed = true;
            }
        }
        if !changed {
            return Ok(());
        }
        let sealed: Vec<_> = effect.evidence.iter().filter(|e| e.sealed).collect();
        let conflict = effect
            .evidence
            .iter()
            .any(|e| e.effect != effect.id || e.request_digest != effect.request_digest)
            || sealed
                .first()
                .is_some_and(|first| sealed.iter().any(|e| e.outcome != first.outcome));
        let outcome = if conflict || effect.state == EffectState::Conflict {
            effect.state = EffectState::Conflict;
            next.contexts
                .get_mut(&effect.context)
                .expect("effect context")
                .fenced = true;
            DeliveryOutcome::Conflict
        } else if let Some(record) = sealed.first() {
            match &record.outcome {
                Outcome::Success(_) => {
                    effect.state = EffectState::Success;
                    DeliveryOutcome::Success
                }
                Outcome::Failure(_) => {
                    effect.state = EffectState::Failure;
                    DeliveryOutcome::Failure
                }
            }
        } else {
            effect.state = EffectState::Unknown;
            DeliveryOutcome::Unknown
        };
        next.deliveries
            .get_mut(&effect.delivery)
            .expect("effect delivery")
            .outcome = outcome;
        self.commit(next)
    }
    fn account_provider(&mut self, id: &EffectId, reply: ProviderReply) -> Result<()> {
        match reply {
            ProviderReply::Record(record) => self.account(id, record.evidence),
            ProviderReply::Codex(observation) => {
                let effect = &self.state.effects[id];
                ensure!(
                    observation.effect == *id
                        && observation.request_digest == effect.request_digest
                        && observation.input_digest == effect.request.input_digest
                        && observation.context == effect.request.context
                        && observation.epoch == effect.request.epoch
                        && observation.delivery == effect.delivery
                        && observation.purpose == effect.request.purpose
                        && observation.profile_digest == effect.request.profile_digest,
                    "native observation request binding mismatch"
                );
                if effect.codex.as_ref() == Some(&observation) {
                    return Ok(());
                }
                let mut next = self.state.clone();
                let effect = next.effects.get_mut(id).expect("effect");
                let text_state = if effect.domain == EvidenceDomain::CodexTextTurnCompletion {
                    if observation.state == crate::codex::ObservationState::Conflicted
                        || observation
                            .text_violation
                            .as_deref()
                            .is_some_and(|reason| reason.starts_with("conflict:"))
                        || (effect.state == EffectState::Success
                            && !Self::valid_text_receipt(effect, &observation))
                    {
                        EffectState::Conflict
                    } else if Self::valid_text_receipt(effect, &observation) {
                        EffectState::Success
                    } else {
                        EffectState::Unknown
                    }
                } else {
                    EffectState::Unknown
                };
                effect.codex = Some(observation);
                effect.state = text_state.clone();
                if text_state == EffectState::Conflict {
                    next.contexts
                        .get_mut(&effect.context)
                        .expect("context")
                        .fenced = true;
                }
                next.deliveries
                    .get_mut(&effect.delivery)
                    .expect("delivery")
                    .outcome = match text_state {
                    EffectState::Success => DeliveryOutcome::Success,
                    EffectState::Conflict => DeliveryOutcome::Conflict,
                    _ => DeliveryOutcome::Unknown,
                };
                self.commit(next)
            }
            ProviderReply::Snapshot { .. } => Ok(()),
        }
    }
    fn valid_text_receipt(effect: &Effect, observation: &crate::codex::Observation) -> bool {
        if effect.domain != EvidenceDomain::CodexTextTurnCompletion
            || observation.state != crate::codex::ObservationState::Completed
            || observation.text_violation.is_some()
            || !observation.notes.is_empty()
            || observation.interrupt.is_some()
            || crate::codex::profile_digest_for(&effect.request.authority.mode).as_deref()
                != Some(observation.profile_digest.as_str())
        {
            return false;
        }
        let (Some(receipt), Some(full), Some(thread), Some(turn)) = (
            observation.text_receipt.as_ref(),
            observation.full_turn.as_ref(),
            observation.thread_id.as_ref(),
            observation.turn_id.as_ref(),
        ) else {
            return false;
        };
        if full["itemsView"] != "full"
            || full["id"] != *turn
            || full["status"] != "completed"
            || !full["error"].is_null()
            || digest(full) != receipt.full_turn_digest
            || receipt.effect != effect.id
            || receipt.delivery != effect.delivery
            || receipt.context != effect.context
            || receipt.epoch != effect.request.epoch
            || receipt.request_digest != effect.request_digest
            || receipt.profile_digest != effect.request.profile_digest
            || receipt.thread_id != *thread
            || receipt.turn_id != *turn
            || receipt.output_digest != digest(&receipt.output)
        {
            return false;
        }
        if crate::codex::text_receipt(observation, full).ok().as_ref() != Some(receipt) {
            return false;
        }
        match &effect.request.purpose {
            EffectPurpose::Input => {
                receipt.package_digest.is_none() && receipt.package_revision.is_none()
            }
            EffectPurpose::Readiness {
                package_digest,
                package_revision,
                expected_ack,
                ..
            } => {
                receipt.package_digest.as_ref() == Some(package_digest)
                    && receipt.package_revision == Some(*package_revision)
                    && receipt.output == *expected_ack
            }
        }
    }
    /// One bounded executor step. Only the owning actor calls this, never an observer.
    pub(crate) fn tick(&mut self) -> Result<()> {
        if self.barrier.is_some() {
            return Ok(());
        }
        if let Some(work) = self.work.take() {
            match work {
                Work::Enter(id) => {
                    if self.clock.now() >= self.state.effects[&id].request.authority.deadline {
                        self.change_outcome(
                            &id,
                            EffectState::ExpiredBeforeEntry,
                            DeliveryOutcome::ExpiredBeforeEntry,
                        )?;
                        self.dispatch_done.insert(id);
                    } else if self.state.contexts[&self.state.effects[&id].context].fenced {
                        // Keep prepared responsibility visible, never dispatch through a fence.
                    } else {
                        self.change_outcome(&id, EffectState::Entered, DeliveryOutcome::Pending)?;
                        self.work = Some(Work::Send(id.clone()));
                        self.pause(Stage::AfterEntry, &id);
                    }
                }
                Work::Send(id) => {
                    let request = self.state.effects[&id].request.clone();
                    let response: Result<ProviderReply> = rpc(
                        self.provider,
                        &ProviderRequestMessage::Command(ProviderCommand::submit(request)),
                    );
                    self.work = Some(Work::Apply(id.clone(), response.ok().map(Box::new)));
                    self.pause(Stage::AfterSend, &id);
                }
                Work::Apply(id, evidence) => {
                    match evidence {
                        Some(evidence) => self.account_provider(&id, *evidence)?,
                        None => self.change_outcome(
                            &id,
                            EffectState::Unknown,
                            DeliveryOutcome::Unknown,
                        )?,
                    }
                    self.dispatch_done.insert(id.clone());
                    self.pause(Stage::AfterResult, &id);
                }
            }
            return Ok(());
        }
        // Reconcile even settled IDs: a later provider contradiction must fence dependents.
        let effects: Vec<_> = self
            .state
            .effects
            .values()
            .filter(|e| {
                !matches!(
                    e.state,
                    EffectState::Prepared | EffectState::ExpiredBeforeEntry
                )
            })
            .map(|e| e.id.clone())
            .collect();
        for id in effects {
            let response: Result<ProviderReply> =
                rpc(self.provider, &ProviderRequestMessage::Lookup(id.clone()));
            match response {
                Ok(reply) => self.account_provider(&id, reply)?,
                _ if matches!(
                    self.state.effects[&id].state,
                    EffectState::Entered | EffectState::Unknown
                ) =>
                {
                    self.change_outcome(&id, EffectState::Unknown, DeliveryOutcome::Unknown)?
                }
                _ => {}
            }
        }
        if let Some(id) = self
            .state
            .effects
            .values()
            .find(|e| e.state == EffectState::Prepared && !self.state.contexts[&e.context].fenced)
            .map(|e| e.id.clone())
        {
            self.work = Some(Work::Enter(id.clone()));
            self.pause(Stage::BeforeEntry, &id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::bail;
    use std::sync::atomic::{AtomicU64, Ordering};
    struct ManualClock(AtomicU64);
    impl Clock for ManualClock {
        fn now(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }
    fn command(key: &str, action: Action) -> Command {
        Command {
            key: CommandId::new(key),
            action,
        }
    }
    #[test]
    fn deadline_is_checked_at_entry_and_capture_boundary() -> Result<()> {
        let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/target"))?;
        let clock = Arc::new(ManualClock(AtomicU64::new(9)));
        let mut coordinator = Coordinator::open(
            &dir.path().join("state.db"),
            "127.0.0.1:1".parse()?,
            clock.clone(),
            Some(Stage::BeforeEntry),
        )?;
        let context = ContextId::new("context");
        let Reply::Owner(owner) =
            coordinator.execute(command("register", Action::Register { context }))?
        else {
            bail!("owner")
        };
        coordinator.execute(command(
            "queue",
            Action::Enqueue {
                owner: owner.clone(),
                input: InputId::new("in"),
                payload: vec![1, 0, 255],
                lineage: "synthetic:test".into(),
            },
        ))?;
        coordinator.execute(command(
            "admit",
            Action::Admit {
                owner: owner.clone(),
                input: InputId::new("in"),
                delivery: DeliveryId::new("d"),
                effect: EffectId::new("e"),
                authority: SyntheticAuthority {
                    approval: "synthetic:yes".into(),
                    deadline: 10,
                    mode: Mode::Hold,
                },
            },
        ))?;
        coordinator.tick()?;
        clock.0.store(10, Ordering::SeqCst);
        coordinator.release_barrier();
        coordinator.tick()?;
        assert_eq!(
            coordinator.result(&EffectId::new("e"))?.state,
            EffectState::ExpiredBeforeEntry
        );
        assert!(
            coordinator
                .execute(command(
                    "expired",
                    Action::Replace {
                        owner: owner.clone(),
                        transition: TransitionId::new("expired"),
                        deadline: 10
                    }
                ))
                .is_err()
        );
        let Reply::Capture(permit) = coordinator.execute(command(
            "replace",
            Action::Replace {
                owner,
                transition: TransitionId::new("t"),
                deadline: 11,
            },
        ))?
        else {
            bail!("capture")
        };
        clock.0.store(11, Ordering::SeqCst);
        assert!(
            coordinator
                .execute(command(
                    "capture",
                    Action::Capture {
                        permit,
                        continuation: Continuation {
                            objective: "goal".into(),
                            authority: "synthetic".into(),
                            commitments: vec![],
                            references: vec!["test".into()]
                        }
                    }
                ))
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn authoritative_result_after_deadline_is_accounted_without_new_entry() -> Result<()> {
        use crate::provider::FakeProvider;
        use std::net::TcpListener;
        use std::time::{Duration, Instant};
        let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/target"))?;
        let clock = Arc::new(ManualClock(AtomicU64::new(9)));
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let mut coordinator = Coordinator::open(
            &dir.path().join("coordinator.db"),
            listener.local_addr()?,
            clock.clone(),
            None,
        )?;
        let mut provider = FakeProvider::open(&dir.path().join("provider.db"))?;
        std::thread::scope(|scope| -> Result<()> {
            let provider_clock = clock.clone();
            let exchange = scope.spawn(move || -> Result<()> {
                let until = Instant::now() + Duration::from_secs(5);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error)
                            if error.kind() == std::io::ErrorKind::WouldBlock
                                && Instant::now() < until =>
                        {
                            std::thread::yield_now()
                        }
                        Err(error) => return Err(error.into()),
                    }
                };
                stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                stream.set_write_timeout(Some(Duration::from_secs(2)))?;
                let request: ProviderRequestMessage = crate::process::read_message(&stream)?;
                // Receipt is acknowledged after the entry deadline, without renewing it.
                provider_clock.0.store(10, Ordering::SeqCst);
                let response = provider.handle(request).map_err(|error| error.to_string());
                crate::process::write_message(&mut stream, &response)?;
                Ok(())
            });
            let result = (|| -> Result<()> {
                let Reply::Owner(owner) = coordinator.execute(command(
                    "register",
                    Action::Register {
                        context: ContextId::new("c"),
                    },
                ))?
                else {
                    bail!("owner")
                };
                coordinator.execute(command(
                    "queue",
                    Action::Enqueue {
                        owner: owner.clone(),
                        input: InputId::new("i"),
                        payload: vec![0, 255],
                        lineage: "synthetic:source".into(),
                    },
                ))?;
                coordinator.execute(command(
                    "admit",
                    Action::Admit {
                        owner,
                        input: InputId::new("i"),
                        delivery: DeliveryId::new("d"),
                        effect: EffectId::new("e"),
                        authority: SyntheticAuthority {
                            approval: "synthetic:explicit".into(),
                            deadline: 10,
                            mode: Mode::Succeed,
                        },
                    },
                ))?;
                for _ in 0..4 {
                    coordinator.tick()?;
                }
                assert_eq!(clock.now(), 10);
                let effect = coordinator.result(&EffectId::new("e"))?;
                assert_eq!(effect.state, EffectState::Success);
                assert_eq!(effect.request.authority.deadline, 10);
                assert_eq!(effect.evidence.len(), 1);
                assert_eq!(
                    coordinator.snapshot().deliveries[0].outcome,
                    DeliveryOutcome::Success
                );
                Ok(())
            })();
            exchange
                .join()
                .map_err(|_| anyhow::anyhow!("provider exchange panicked"))??;
            result
        })
    }
}
