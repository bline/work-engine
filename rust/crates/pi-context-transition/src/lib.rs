//! Pi actuator composition. Lifecycle's durable owner grants admission and
//! settles evidence. A transport reply never grants a context transition.
use lifecycle_core::{ClockSample, SubjectId, TransitionId};
use lifecycle_runtime::NativeStdio;
use lifecycle_store::{NativeTurnEntry, PiSettlementProof, SqliteLifecycleStore};
use serde_json::{Value, json};
use std::time::Duration;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub struct PiAdapter {
    pipe: NativeStdio,
    transport: String,
    sequence: u64,
    uncertain_replacement: bool,
    initialized: bool,
    multiturn: bool,
}
impl PiAdapter {
    /// Trusted composition supplies the pinned executable, glue and private
    /// environment. Uses existing runtime process custody, never raw spawning.
    pub fn launch(command: &mut tokio::process::Command, transport: String) -> Result<Self, Error> {
        Ok(Self {
            pipe: NativeStdio::spawn(command, 1_048_576, 4096)?,
            transport,
            sequence: 0,
            uncertain_replacement: false,
            initialized: false,
            multiturn: false,
        })
    }
    /// Explicit opt-in to the owner-anchored v2 proof. Default launch stays v1.
    pub fn launch_multiturn(
        command: &mut tokio::process::Command,
        transport: String,
    ) -> Result<Self, Error> {
        let mut adapter = Self::launch(command, transport)?;
        adapter.multiturn = true;
        Ok(adapter)
    }
    async fn request(&mut self, value: Value) -> Result<Value, Error> {
        self.pipe.send(&serde_json::to_vec(&value)?).await?;
        let (_, raw) = self.pipe.receive().await?.ok_or("Pi EOF")?;
        let value: Value = serde_json::from_slice(&raw)?;
        if value["type"] == "pi.error" {
            return Err(value.to_string().into());
        }
        Ok(value)
    }
    pub async fn initialize(
        &mut self,
        intent: lifecycle_store::NativeSessionEntry,
    ) -> Result<(lifecycle_store::NativeSessionEntry, Value), Error> {
        if self.initialized
            || intent.thread().is_some()
            || intent.profile_digest()
                != if self.multiturn {
                    lifecycle_store::pi_multiturn_profile_digest()
                } else {
                    lifecycle_store::pi_profile_digest()
                }
        {
            return Err("not a fresh Pi allocation permit".into());
        }
        self.initialized = true;
        let response = self.request(json!({"op":"init"})).await?;
        Ok((intent, response))
    }
    pub async fn collect_prompt(
        &mut self,
        store: &mut SqliteLifecycleStore,
        session: &lifecycle_core::NativeSessionId,
        turn: &mut NativeTurnEntry,
        prompt: &str,
        fixture_reply: &str,
        _clock: ClockSample,
    ) -> Result<PiSettlementProof, Error> {
        let current = store
            .native_recovery(turn.invocation_id())?
            .ok_or("missing durable turn")?;
        if current.send_intent
            || current.class != "EnteredUncertain"
            || current.session_id != session.as_str()
            || current.turn_id.as_deref() != turn.turn().map(|t| t.as_str())
        {
            return Err("unbound send intent".into());
        }
        let correlation =
            lifecycle_core::ProviderTurnId::parse(format!("pi-local:{}", turn.invocation_id()))?;
        let bytes = serde_json::to_vec(
            &json!({"op":"prompt","text":prompt,"fixture_reply":fixture_reply,"invocationId":turn.invocation_id(),"turnCorrelation":correlation.as_str()}),
        )?;
        // This durable CAS is the sole send opportunity, consumed even when
        // sending or collection is cancelled. A reopened entry cannot resend.
        store.mark_native_send_intent(turn)?;
        store.bind_native_turn(turn, correlation)?;
        let first = self.sequence + 1;
        self.pipe.send(&bytes).await?;
        loop {
            let (_, raw) = self
                .pipe
                .receive()
                .await?
                .ok_or("Pi EOF after admitted input; no resend")?;
            let value: Value = serde_json::from_slice(&raw)?;
            self.sequence += 1;
            store.record_native_ingress(
                session,
                &self.transport,
                self.sequence,
                &raw,
                "qualified",
                value["type"].as_str(),
                Some(turn.thread().as_str()),
                Some(turn.turn().ok_or("missing turn correlation")?.as_str()),
                None,
                lifecycle_runtime::ClockSource::sample(
                    &lifecycle_runtime::SystemClock,
                    lifecycle_core::WaitBudgetMs::new(100),
                )?,
            )?;
            if value["type"] == "pi.error" {
                return Err(value.to_string().into());
            }
            if value["type"] == "pi.session_history" {
                break;
            }
        }
        Ok(PiSettlementProof {
            transport: self.transport.clone(),
            first_sequence: first,
            history_sequence: self.sequence,
        })
    }
    /// A failed or lost actuator reply remains uncertain. The same adapter will
    /// not issue a second reset. Reopening the store does not clear this fence.
    pub fn replacement_ready(
        &self,
        store: &mut SqliteLifecycleStore,
        subject: &SubjectId,
        transition: &TransitionId,
        checkpoint: &str,
    ) -> Result<(), Error> {
        let current = store.subject_projection(subject, None, None)?;
        let owner = current.transition.ok_or("no transition")?;
        let source = store
            .committed_text_for_transition(transition)?
            .ok_or("no checkpoint source")?;
        if self.uncertain_replacement
            || current.owner_kind != "transition"
            || owner.transition_id != transition.as_str()
            || owner.stage != "switching"
            || source.final_text != checkpoint
        {
            return Err("replacement not admitted by current owner/checkpoint".into());
        }
        Ok(())
    }
    pub async fn replace(
        &mut self,
        store: &mut SqliteLifecycleStore,
        subject: &SubjectId,
        transition: &TransitionId,
        checkpoint: &str,
        successor: lifecycle_store::NativeSessionEntry,
    ) -> Result<(lifecycle_store::NativeSessionEntry, Value), Error> {
        if self.multiturn {
            store.validate_pi_multiturn_session_binding(&successor)?;
        } else {
            store.validate_pi_session_binding(&successor)?;
        }
        self.replacement_ready(store, subject, transition, checkpoint)?;
        if successor.subject() != subject || successor.thread().is_some() {
            return Err("not a fresh store-minted successor allocation intent".into());
        }
        let expected: Value = serde_json::from_str(checkpoint)?;
        if expected["successor_context"] != successor.context().as_str() {
            return Err("wrong successor generation".into());
        }
        // Consuming this private store-minted permit prevents repeated entry.
        // A new process cannot reacquire an existing unbound allocation intent.
        self.uncertain_replacement = true;
        let response = self.request(json!({"op":"replace"})).await?;
        Ok((successor, response))
    }
    /// Observation only; no lifecycle admission or context replacement.
    pub async fn observe(&mut self) -> Result<Value, Error> {
        self.request(json!({"op":"observe"})).await
    }
    /// Cleanup retains uncertain effects and observes the owned wrapper exit.
    pub async fn terminate(mut self) -> Result<lifecycle_runtime::NativeChildExit, Error> {
        Ok(self.pipe.terminate_and_reap(Duration::from_secs(3)).await?)
    }
    pub async fn close(mut self) -> Result<lifecycle_runtime::NativeChildExit, Error> {
        let response = self.request(json!({"op":"close"})).await;
        self.pipe.close_stdin();
        if response.is_err() {
            return Ok(self.pipe.terminate_and_reap(Duration::from_secs(3)).await?);
        }
        Ok(self.pipe.wait_for_wrapper(Duration::from_secs(3)).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifecycle_core::*;
    use lifecycle_store::*;
    fn clock() -> ClockSample {
        ClockSample {
            wall: WallTimeMs::new(100),
            wait_budget: WaitBudgetMs::new(100),
        }
    }
    fn setup(
        label: &str,
        profile: &str,
    ) -> (SqliteLifecycleStore, NativeSessionEntry, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "pi-boundary-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut store = SqliteLifecycleStore::open(&root, "fixture".into()).unwrap();
        store
            .register_subject(
                SubjectId::parse("s").unwrap(),
                ContextGeneration::parse("c").unwrap(),
                BuildId::parse("b").unwrap(),
                ProofRunId::parse("p").unwrap(),
                clock(),
            )
            .unwrap();
        store
            .install_trusted_grant(TrustedGrant {
                id: GrantId::parse("g").unwrap(),
                issuer: "fixture".into(),
                principal: "operator".into(),
                subject: SubjectId::parse("s").unwrap(),
                context: ContextGeneration::parse("c").unwrap(),
                build: BuildId::parse("b").unwrap(),
                proof_run: ProofRunId::parse("p").unwrap(),
                scope: GrantScope::NativeTurn,
                expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
        let entry = store
            .reserve_native_session(
                NativeSessionId::parse("n").unwrap(),
                SubjectId::parse("s").unwrap(),
                ContextGeneration::parse("c").unwrap(),
                RuntimeIncarnation::parse("i").unwrap(),
                profile,
                PI_SDK_INDEX_SHA256,
                &pi_protocol_digest(),
                SnapshotId::parse("snap").unwrap(),
                &"a".repeat(64),
                false,
                clock(),
            )
            .unwrap();
        (store, entry, root)
    }
    fn echo(path: &std::path::Path) -> PiAdapter {
        let mut cmd = tokio::process::Command::new("/usr/bin/tee");
        cmd.arg(path).env_clear();
        PiAdapter::launch(&mut cmd, "synthetic:boundary-only".into()).unwrap()
    }
    #[tokio::test]
    async fn cancelled_collection_cannot_send_same_entry_twice() {
        let (mut store, mut session, root) = setup("cancel", &pi_profile_digest());
        store
            .bind_native_thread(
                &mut session,
                ProviderThreadId::parse("physical:test").unwrap(),
            )
            .unwrap();
        let mut turn = store
            .claim_native_turn(
                &session,
                NativeTurnRequest {
                    invocation_id: "inv".into(),
                    attempt: AttemptId::parse("a").unwrap(),
                    grant: GrantId::parse("g").unwrap(),
                    grant_revision: 1,
                    purpose: NativeInvocationPurpose::DomainWork,
                    prompt: b"prompt".to_vec(),
                },
                "operator",
                clock(),
            )
            .unwrap();
        let witness = root.join("writes");
        let mut adapter = echo(&witness);
        assert!(
            tokio::time::timeout(
                Duration::from_millis(50),
                adapter.collect_prompt(
                    &mut store,
                    session.id(),
                    &mut turn,
                    "prompt",
                    "fixture",
                    clock()
                )
            )
            .await
            .is_err()
        );
        assert!(store.native_recovery("inv").unwrap().unwrap().send_intent);
        assert!(
            adapter
                .collect_prompt(
                    &mut store,
                    session.id(),
                    &mut turn,
                    "prompt",
                    "fixture",
                    clock()
                )
                .await
                .is_err()
        );
        adapter.close().await.unwrap();
        let lines = std::fs::read_to_string(witness).unwrap();
        assert_eq!(lines.lines().count(), 2); // one prompt, one close, no repeated prompt
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn wrong_profile_permit_never_reaches_reset_transport() {
        let (mut store, permit, root) = setup("profile", &"b".repeat(64));
        let witness = root.join("writes");
        let mut adapter = echo(&witness);
        let refusal = adapter
            .replace(
                &mut store,
                &SubjectId::parse("s").unwrap(),
                &TransitionId::parse("tr").unwrap(),
                "{}",
                permit,
            )
            .await
            .unwrap_err();
        assert!(matches!(
            refusal.downcast_ref::<StoreError>(),
            Some(StoreError::Rejected)
        ));
        adapter.close().await.unwrap();
        let lines = std::fs::read_to_string(witness).unwrap();
        assert_eq!(lines.lines().count(), 1);
        assert!(lines.contains("close"));
        assert!(!lines.contains("replace"));
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
