//! Serialized bridge to the existing coding-effect owner. No second effect
//! ledger: immutable ingress records the Host's intent and custody result,
//! while its native parent remains uncertain until the tool-aware proof settles.
use lifecycle_core::{ClockSample, NativeSessionId};
use lifecycle_store::{NativeTurnEntry, SqliteLifecycleStore};
use serde_json::{Value, json};
use std::{cell::RefCell, path::PathBuf, rc::Rc};
use work_engine_manual_host::{
    Artifact, Error, ProcessRecord, Result, digest,
    pi_docker::coding::{CodingEffectGate, CodingToolIntent, ProviderAttemptIntent},
};

pub struct CodingLifecycle {
    pub store: SqliteLifecycleStore,
    pub parent: NativeTurnEntry,
    session: NativeSessionId,
    transport: String,
    sequence: u64,
    clock: fn() -> ClockSample,
    intent: Option<CodingToolIntent>,
    result: Option<Value>,
}
/// Retained by the deterministic actor, never exported to the SDK.
#[derive(Clone)]
pub struct CodingLifecycleGate(Rc<RefCell<CodingLifecycle>>);
fn refused(message: impl ToString) -> Error {
    Error::Refused(message.to_string())
}
impl CodingLifecycleGate {
    pub fn new(
        store: SqliteLifecycleStore,
        parent: NativeTurnEntry,
        session: NativeSessionId,
        transport: String,
        sequence: u64,
        clock: fn() -> ClockSample,
    ) -> Self {
        Self(Rc::new(RefCell::new(CodingLifecycle {
            store,
            parent,
            session,
            transport,
            sequence,
            clock,
            intent: None,
            result: None,
        })))
    }
    pub fn owner(&self) -> std::cell::Ref<'_, CodingLifecycle> {
        self.0.borrow()
    }
    pub fn owner_mut(&self) -> std::cell::RefMut<'_, CodingLifecycle> {
        self.0.borrow_mut()
    }
    /// Local early refusal complements lifecycle's uncertain-parent transition
    /// fence. A reopened bridge cannot reconstruct this successful observation.
    pub fn replacement_ready(&self) -> Result<()> {
        let owner = self.0.borrow_mut();
        let invocation = owner.parent.invocation_id().to_string();
        let recovery = owner
            .store
            .native_recovery(&invocation)
            .map_err(refused)?
            .ok_or_else(|| refused("missing parent"))?;
        if owner.intent.is_none() || owner.result.is_none() || recovery.class != "CompletedSettled"
        {
            return Err(refused(
                "coding parent/effect unresolved; replacement refused",
            ));
        }
        Ok(())
    }
}
impl CodingLifecycle {
    fn record(&mut self, method: &str, value: &Value) -> Result<()> {
        self.sequence += 1;
        self.store
            .record_native_ingress(
                &self.session,
                &self.transport,
                self.sequence,
                &serde_json::to_vec(value)?,
                "qualified",
                Some(method),
                Some(self.parent.thread().as_str()),
                self.parent.turn().map(|t| t.as_str()),
                None,
                (self.clock)(),
            )
            .map_err(refused)?;
        Ok(())
    }
    pub fn result(&self) -> Option<&Value> {
        self.result.as_ref()
    }
}
impl CodingEffectGate for CodingLifecycleGate {
    fn execute(
        &mut self,
        intent: &CodingToolIntent,
        actuator: &mut dyn FnMut(i64) -> Result<(bool, Vec<u8>)>,
    ) -> Result<(bool, Vec<u8>)> {
        let mut owner = self.0.borrow_mut();
        if owner.intent.is_some() {
            return Err(refused("coding effect already attempted; no replay"));
        }
        // Retain the attempt even when ingress/admission/actuator fails. Host's
        // independent durable intent is the crash-recovery source of uncertainty.
        owner.intent = Some(intent.clone());
        let event = json!({"type":"pi.coding_tool_intent","invocationId":owner.parent.invocation_id(),"intent":intent});
        owner.record("pi.coding_tool_intent", &event)?;
        let CodingLifecycle {
            store,
            parent,
            transport,
            sequence,
            clock,
            ..
        } = &mut *owner;
        store
            .with_pi_coding_effect(
                parent,
                transport,
                *sequence,
                &serde_json::to_value(intent)?,
                clock(),
                actuator,
            )
            .map_err(refused)?
    }
    fn observed(
        &mut self,
        intent: &CodingToolIntent,
        artifact: &Artifact,
        process: &ProcessRecord,
        tool_container: &str,
    ) -> Result<()> {
        let mut owner = self.0.borrow_mut();
        if owner.intent.as_ref() != Some(intent)
            || owner.result.is_some()
            || tool_container.len() != 64
            || !tool_container.bytes().all(|c| c.is_ascii_hexdigit())
            || process.status.is_none()
            || process.timed_out
            || !process.leftovers.is_empty()
            || process.pid.is_none()
            || process.start.is_none()
            || process.argv.len() != 8
            || process.argv[1..4] != ["exec", "--workdir", "/workspace"]
            || process.argv[5..7] != ["/bin/sh", "-c"]
            || digest(process.argv[7].as_bytes()) != intent.command_sha256
        {
            return Err(refused("Host coding custody incomplete"));
        }
        let stdout = std::fs::read(&process.stdout)?;
        let stderr = std::fs::read(&process.stderr)?;
        if process.stdout_sha256.as_deref() != Some(digest(&stdout).as_str())
            || process.stderr_sha256.as_deref() != Some(digest(&stderr).as_str())
        {
            return Err(refused("Host coding process output changed"));
        }
        let artifact_bytes = String::from_utf8(artifact.verify()?).map_err(refused)?;
        let result: Value = serde_json::from_str(&artifact_bytes)?;
        let ok = process.status == Some(0);
        let output = format!(
            "Host command success: {ok}\n{}\n[stderr]\n{}",
            String::from_utf8_lossy(&stdout),
            String::from_utf8_lossy(&stderr)
        );
        if result != json!({"type":"tool_result","id":intent.call_id,"ok":ok,"output":output}) {
            return Err(refused("Host result disagrees with process custody"));
        }
        let evidence = json!({"type":"pi.coding_tool_result","invocationId":owner.parent.invocation_id(),"intent":intent,"result":result,"artifact":artifact,"artifact_bytes":artifact_bytes,"stdout_bytes":stdout,"stderr_bytes":stderr,"process":process,"tool_container":tool_container});
        owner.record("pi.coding_tool_result", &evidence)?;
        owner.result = Some(evidence);
        Ok(())
    }
}

use lifecycle_core::*;
use lifecycle_store::{
    NativeSessionEntry, NativeTurnRequest, PI_SDK_INDEX_SHA256, PiSettlementProof,
    pi_coding_live_profile_digest, pi_coding_live_protocol_digest, pi_coding_profile_digest,
    pi_coding_protocol_digest, pi_coding_sol_high_profile_digest,
};
/// Pinned outside controls. These fixture controls never authorize live access.
#[derive(Clone)]
pub struct CodingConfig {
    pub budget: Value,
    /// `Some` selects the separately versioned live-provider continuity mode: the
    /// exact task bytes admitted by the Host plan. `None` is the provider-disabled
    /// fixture. The two never share budget classification or checkpoint source.
    pub task: Option<String>,
    pub expires_ms: i64,
    pub pressure_basis_points: u64,
    /// `Some` selects hosted provider transport for live mode. The policy is
    /// bound into the turn grant ids, so the issuer's grant is what admits it.
    pub hosted: Option<HostedProviderPolicy>,
    /// Battery mode: one bounded long-form builder run (domain-work turn loop
    /// with parametrized replacement cycles) instead of the fixed 4-request
    /// continuity choreography. Battery knobs travel in the packet budget:
    /// `max_resets`, `replacement_basis_points`, `max_context_bytes` (the
    /// choreography keys `max_provider_requests`/`max_tool_calls` carry the
    /// battery job's admission-pinned values instead of 4/1).
    pub battery: bool,
    /// Battery only: the Host's staged working workspace
    /// (`state_dir/coding-workspace`), where the gate reads the builder's
    /// durable completion signal. `None` for every other mode.
    pub battery_workspace: Option<std::path::PathBuf>,
}
/// Failover policy a hosted grant covers: the only `(route, payer)` pairs a
/// provider attempt may use, and the attempt limit per logical request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedProviderPolicy {
    pub routes: Vec<(String, String)>,
    pub max_attempts_per_request: u32,
}
impl HostedProviderPolicy {
    pub(crate) fn validate(&self) -> Result<()> {
        let bounded = |v: &str| {
            !v.is_empty()
                && v.len() <= 128
                && v.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        };
        let mut pairs = self.routes.clone();
        pairs.sort();
        pairs.dedup();
        if self.routes.is_empty()
            || self.routes.len() > 8
            || pairs.len() != self.routes.len()
            || !self.routes.iter().all(|(r, p)| bounded(r) && bounded(p))
            || !(1..=8).contains(&self.max_attempts_per_request)
        {
            return Err(refused("hosted provider policy invalid"));
        }
        Ok(())
    }
    pub fn digest(&self) -> String {
        let routes: Vec<Value> = self
            .routes
            .iter()
            .map(|(route, payer)| json!({"route":route,"payer":payer}))
            .collect();
        digest(
            json!({"schema":"work-engine.pi-hosted-provider-policy.v1","routes":routes,"max_attempts_per_request":self.max_attempts_per_request})
                .to_string()
                .as_bytes(),
        )
    }
    /// Turn grant id the trusted issuer must install for this policy.
    pub fn grant_id(&self, base: &str) -> String {
        format!("{base}:pi-hosted:{}", self.digest())
    }
}
struct ContinuityState {
    store: Option<SqliteLifecycleStore>,
    initial: Option<NativeSessionEntry>,
    session: Option<NativeSessionEntry>,
    gate: Option<CodingLifecycleGate>,
    config: CodingConfig,
    clock: fn() -> ClockSample,
    pending: Option<Value>,
    first_sequence: u64,
    phase: u8,
    provider_calls: u64,
    transition: Option<TransitionId>,
    successor: Option<NativeSessionEntry>,
    checkpoint: Option<String>,
    recovery_receipt: Option<String>,
    workspace: Option<Value>,
    original_result: Option<Value>,
    observations: Vec<Value>,
    fresh_successor: Option<Value>,
    report: Option<Value>,
    reset_entered: bool,
    tool_result_sent: bool,
    battery: bool,
    battery_max_requests: u64,
    battery_max_resets: u64,
    replacement_basis_points: u64,
    battery_max_context_bytes: u64,
    last_context_bytes: u64,
    resets_done: u64,
    cycle_request_count: u64,
    turn_counter: u64,
    tool_executions: u64,
    last_continued: Option<String>,
    /// Hosted attempts consumed for the current logical request.
    hosted_attempts: (u64, u64),
    /// Every admitted hosted attempt, reported with the continuity result.
    /// Not recorded in the turn's ingress: the Pi event grammar and the Live
    /// lifecycle digests stay unchanged. The Host ledger durably records each
    /// attempt before its dispatch.
    hosted_log: Vec<Value>,
}
impl ContinuityState {
    fn live(&self) -> bool {
        self.config.task.is_some()
    }
    fn turn_grant(&self, base: &str) -> String {
        match &self.config.hosted {
            Some(policy) => policy.grant_id(base),
            None => base.into(),
        }
    }
    fn provider_attempt(
        &mut self,
        intent: &ProviderAttemptIntent,
        dispatch: &mut dyn FnMut(i64) -> Result<()>,
    ) -> Result<()> {
        self.check()?;
        let policy = self
            .config
            .hosted
            .clone()
            .ok_or_else(|| refused("hosted provider policy absent"))?;
        if !self.live()
            || self.pending.is_some()
            || intent.request == 0
            || intent.request != self.provider_calls
        {
            return Err(refused("provider attempt outside an admitted request"));
        }
        let previous = if self.hosted_attempts.0 == intent.request {
            self.hosted_attempts.1
        } else {
            0
        };
        if intent.attempt != previous + 1
            || intent.attempt > u64::from(policy.max_attempts_per_request)
        {
            return Err(refused("provider attempt order/limit"));
        }
        if !policy
            .routes
            .iter()
            .any(|(r, p)| *r == intent.route && *p == intent.payer)
        {
            return Err(refused("provider attempt route/payer not granted"));
        }
        // Consumed before entry: a refused or failed attempt is never replayed.
        self.hosted_attempts = (intent.request, intent.attempt);
        let gate = self
            .gate
            .as_ref()
            .ok_or_else(|| refused("coding parent absent"))?
            .clone();
        let mut o = gate.0.borrow_mut();
        let entry = json!({"invocationId":o.parent.invocation_id(),"intent":intent,"policy":policy.digest()});
        let clock = (self.clock)();
        let configured = self.config.expires_ms;
        let log = &mut self.hosted_log;
        let CodingLifecycle { store, parent, .. } = &mut *o;
        store
            .with_pi_coding_turn_send(parent, clock, |granted| {
                // Logged only once currentness of the parent grant has been
                // established for this dispatch.
                let deadline = granted.min(configured);
                let mut entry = entry;
                entry["deadlineMs"] = json!(deadline);
                log.push(entry);
                dispatch(deadline)
            })
            .map_err(refused)?
    }
}
#[derive(Clone)]
pub struct CodingContinuity(Rc<RefCell<ContinuityState>>);
impl CodingContinuity {
    pub fn new(
        store: SqliteLifecycleStore,
        initial: NativeSessionEntry,
        config: CodingConfig,
        clock: fn() -> ClockSample,
    ) -> Result<Self> {
        // The retired credential-mounted Sol-high profile cannot be resumed,
        // and live mode exists only with a hosted policy (the retired
        // lifecycle-live composition had none, F4).
        if initial.profile_digest() == pi_coding_sol_high_profile_digest()
            || config.task.is_some() != config.hosted.is_some()
            || config.budget["schema"] != "work-engine.coding-budget-control.v1"
            || config.budget["classification"]
                != if config.task.is_some() {
                    "live_provider_observation"
                } else {
                    "provider_disabled_fixture"
                }
            || config
                .task
                .as_ref()
                .is_some_and(|t| t.is_empty() || t.len() > 8192)
            || config.budget["available"] != true
            || if config.battery {
                // Battery: admission-pinned finite bounds inside the profile
                // caps (the producer re-checks against the plan's exact pins;
                // the gate enforces the packet's values at runtime).
                !config.budget["max_provider_requests"]
                    .as_u64()
                    .is_some_and(|v| (1..=512).contains(&v))
                    || !config.budget["max_tool_calls"]
                        .as_u64()
                        .is_some_and(|v| (1..=512).contains(&v))
                    || !config.budget["max_resets"]
                        .as_u64()
                        .is_some_and(|v| (0..=16).contains(&v))
                    || !config.budget["replacement_basis_points"]
                        .as_u64()
                        .is_some_and(|v| (1..=10000).contains(&v))
                    || !config.budget["max_context_bytes"]
                        .as_u64()
                        .is_some_and(|v| (1..=2_000_000).contains(&v))
            } else {
                config.budget["max_provider_requests"] != 4 || config.budget["max_tool_calls"] != 1
            }
            || config.expires_ms <= clock().wall.get()
            || config.pressure_basis_points > 10000
            || config
                .hosted
                .as_ref()
                .is_some_and(|p| config.task.is_none() || p.validate().is_err())
        {
            return Err(refused("canonical coding budget/control refused"));
        }
        budget_current(&config, clock())?;
        let battery = config.battery;
        let battery_max_requests = if config.battery {
            config.budget["max_provider_requests"].as_u64().unwrap_or(0)
        } else {
            4
        };
        let battery_max_resets = if config.battery {
            config.budget["max_resets"].as_u64().unwrap_or(0)
        } else {
            1
        };
        let replacement_basis_points = if config.battery {
            config.budget["replacement_basis_points"]
                .as_u64()
                .unwrap_or(0)
        } else {
            config.pressure_basis_points
        };
        let battery_max_context_bytes = config.budget["max_context_bytes"].as_u64().unwrap_or(0);
        Ok(Self(Rc::new(RefCell::new(ContinuityState {
            store: Some(store),
            initial: Some(initial),
            session: None,
            gate: None,
            config,
            clock,
            pending: None,
            first_sequence: 1,
            phase: 0,
            provider_calls: 0,
            transition: None,
            successor: None,
            checkpoint: None,
            recovery_receipt: None,
            workspace: None,
            original_result: None,
            observations: vec![],
            fresh_successor: None,
            report: None,
            reset_entered: false,
            tool_result_sent: false,
            battery,
            battery_max_requests,
            battery_max_resets,
            replacement_basis_points,
            battery_max_context_bytes,
            last_context_bytes: 0,
            resets_done: 0,
            cycle_request_count: 0,
            turn_counter: 0,
            tool_executions: 0,
            last_continued: None,
            hosted_attempts: (0, 0),
            hosted_log: vec![],
        }))))
    }
    /// Hosted attempts admitted under a current grant so far (empty otherwise).
    /// Available on every exit, unlike the success-only report.
    pub fn hosted_attempts(&self) -> Vec<Value> {
        self.0.borrow().hosted_log.clone()
    }
    pub fn report(&self) -> Result<Value> {
        self.0
            .borrow()
            .report
            .clone()
            .ok_or_else(|| refused("coding continuity unfinished"))
    }
}
fn budget_current(config: &CodingConfig, clock: ClockSample) -> Result<()> {
    let b = &config.budget;
    let now = clock.wall.get();
    let observed = b["observed_ms"]
        .as_i64()
        .ok_or_else(|| refused("canonical budget observation unavailable"))?;
    let age = b["max_age_ms"]
        .as_i64()
        .filter(|n| *n > 0 && *n <= 300000)
        .ok_or_else(|| refused("budget age bound"))?;
    if b["authority"] != "observation_only"
        || b["sequence"] != 1
        || b["expires_ms"] != config.expires_ms
        || b["source"].as_str().is_none_or(str::is_empty)
        || b["generation"].as_str().is_none_or(str::is_empty)
        || observed > now
        || now - observed > age
        || b["available"] != true
        || now >= config.expires_ms
    {
        return Err(refused("canonical budget stale/exhausted/misbound"));
    }
    Ok(())
}
impl ContinuityState {
    fn snapshot(&self, frame: &Value) -> Result<()> {
        let v = &frame["snapshot"];
        let now = (self.clock)().wall.get();
        let observed = v["observedMs"]
            .as_i64()
            .ok_or_else(|| refused("SDK observation time absent"))?;
        if observed > now
            || now - observed > 30000
            || v["sessionId"] != frame["sessionId"]
            || v["idle"] != true
            || !v["observedAt"].is_string()
            || v["model"]["contextWindow"].as_u64().is_none_or(|n| n == 0)
            || self
                .observations
                .first()
                .is_some_and(|first| first["model"] != v["model"])
        {
            return Err(refused(
                "actual SDK observation freshness/identity/model conflict",
            ));
        }
        Ok(())
    }
    fn check(&self) -> Result<()> {
        budget_current(&self.config, (self.clock)())?;
        if (self.clock)().wall.get() >= self.config.expires_ms
            || self.config.budget["available"] != true
        {
            return Err(refused("coding control exhausted"));
        }
        Ok(())
    }
    fn queue(&mut self, command: Value) -> Result<Vec<Value>> {
        if self.pending.is_some() {
            return Err(refused("coding command already pending"));
        }
        self.pending = Some(command.clone());
        Ok(vec![command])
    }
    fn claim(
        store: &mut SqliteLifecycleStore,
        session: &NativeSessionEntry,
        id: &str,
        prompt: &str,
        purpose: NativeInvocationPurpose,
        grant: &str,
        clock: ClockSample,
    ) -> Result<NativeTurnEntry> {
        let mut p = store
            .claim_native_turn(
                session,
                NativeTurnRequest {
                    invocation_id: id.into(),
                    attempt: AttemptId::parse(format!("attempt:{id}")).map_err(refused)?,
                    grant: GrantId::parse(grant).map_err(refused)?,
                    grant_revision: 1,
                    purpose,
                    prompt: prompt.as_bytes().to_vec(),
                },
                "operator:fixture",
                clock,
            )
            .map_err(refused)?;
        store.mark_native_send_intent(&p).map_err(refused)?;
        store
            .bind_native_turn(
                &mut p,
                ProviderTurnId::parse(format!("turn:{id}")).map_err(refused)?,
            )
            .map_err(refused)?;
        Ok(p)
    }
    fn prompt(
        &mut self,
        id: &str,
        prompt: &str,
        purpose: NativeInvocationPurpose,
        grant: &str,
    ) -> Result<Vec<Value>> {
        let gate = self
            .gate
            .as_ref()
            .ok_or_else(|| refused("missing owner"))?
            .clone();
        let mut o = gate.0.borrow_mut();
        let p = Self::claim(
            &mut o.store,
            self.session.as_ref().unwrap(),
            id,
            prompt,
            purpose,
            grant,
            (self.clock)(),
        )?;
        o.parent = p;
        self.first_sequence = o.sequence + 1;
        // The tool-result guard is per-turn: each new turn may consume exactly
        // one observed Host result.
        self.tool_result_sent = false;
        self.cycle_request_count = 0;
        if purpose == NativeInvocationPurpose::DomainWork {
            self.last_continued = Some(id.to_string());
        }
        let command = json!({"type":"bridge_prompt","sessionId":o.parent.thread().as_str(),"invocationId":id,"turnCorrelation":format!("turn:{id}"),"prompt":prompt});
        drop(o);
        self.queue(command)
    }
    fn ready(&mut self, frame: &Value) -> Result<Vec<Value>> {
        if self.gate.is_some()
            || frame["snapshot"]["idle"] != true
            || frame["snapshot"]["sessionId"] != frame["sessionId"]
        {
            return Err(refused("coding initial observation conflict"));
        }
        self.check()?;
        self.snapshot(frame)?;
        let snapshot = frame["snapshot"].clone();
        self.observations.push(snapshot.clone());
        let mut store = self.store.take().unwrap();
        let mut session = self.initial.take().unwrap();
        store
            .bind_native_thread(
                &mut session,
                ProviderThreadId::parse(
                    frame["sessionId"]
                        .as_str()
                        .ok_or_else(|| refused("native session absent"))?,
                )
                .map_err(refused)?,
            )
            .map_err(refused)?;
        let observation = json!({"authority":"observation_only","budget":self.config.budget,"sdk_context":snapshot});
        let prompt = match &self.config.task {
            Some(task) if self.battery => format!(
                "{task}\n\nHosted battery builder job: the workspace at /workspace is the whole repository clone you are to work in. Use workspace_exec across as many turns as the task needs; builds and tests run offline (the full toolchain including cargo is at /toolchain, mounted read-only; invoke cargo with CARGO_HOME=/toolchain/cargo-home and --offline). Commit on the task's branch as the task instructs. When the task's obligations are met, write its durable completion artifact (evidence/completion.json) exactly as the task specifies, then reply with a brief final summary for this turn. Outside canonical controls and actual SDK observation: {observation}"
            ),
            Some(task) => format!(
                "{task}\n\nBounded live-provider continuity job: the single workspace_exec command may be used exactly once, so make the whole edit and run the acceptance in that one command, then reply with a brief final summary. Outside canonical controls and actual SDK observation: {observation}"
            ),
            None => format!(
                "Bounded provider-disabled coding fixture. Preserve the Host tool result. Outside canonical controls and actual SDK observation: {observation}"
            ),
        };
        let parent = Self::claim(
            &mut store,
            &session,
            "coding",
            &prompt,
            NativeInvocationPurpose::DomainWork,
            &self.turn_grant("grant:old"),
            (self.clock)(),
        )?;
        let gate = CodingLifecycleGate::new(
            store,
            parent,
            session.id().clone(),
            "coding-wire".into(),
            0,
            self.clock,
        );
        self.session = Some(session);
        self.gate = Some(gate);
        self.queue(json!({"type":"bridge_prompt","sessionId":frame["sessionId"],"invocationId":"coding","turnCorrelation":"turn:coding","prompt":prompt}))
    }
    fn provider(&mut self, frame: &Value) -> Result<Vec<Value>> {
        self.check()?;
        if self.battery {
            if !matches!(self.phase, 0 | 2 | 3) {
                return Err(refused("provider request outside an admitted turn"));
            }
            self.provider_calls += 1;
            if frame["id"] != self.provider_calls || self.provider_calls > self.battery_max_requests
            {
                return Err(refused("coding provider count conflict"));
            }
            self.last_context_bytes = frame["contextBytes"].as_u64().unwrap_or(0);
            self.cycle_request_count += 1;
            let gate = self.gate.as_ref().unwrap().clone();
            let mut o = gate.0.borrow_mut();
            if frame["sessionId"] != o.parent.thread().as_str()
                || frame["invocationId"] != o.parent.invocation_id()
                || frame["turnCorrelation"] != o.parent.turn().unwrap().as_str()
            {
                return Err(refused("coding provider owner correlation conflict"));
            }
            let mut request = frame.clone();
            request["type"] = json!("pi.provider_request");
            if let Some(m) = request.as_object_mut() {
                m.remove("host_workspace");
                m.remove("schema");
            }
            o.record("pi.provider_request", &request)?;
            drop(o);
            return self.queue(json!({"type":"bridge_provider","sessionId":frame["sessionId"],"id":self.provider_calls,"mode":"live"}));
        }
        if !matches!(
            (self.phase, self.provider_calls + 1),
            (0, 1 | 2) | (2, 3) | (3, 4)
        ) {
            return Err(refused("provider request outside admitted phase"));
        }
        self.provider_calls += 1;
        if self.provider_calls > 4 || frame["id"] != self.provider_calls {
            return Err(refused("coding provider count conflict"));
        }
        let gate = self.gate.as_ref().unwrap().clone();
        let mut o = gate.0.borrow_mut();
        if frame["sessionId"] != o.parent.thread().as_str()
            || frame["invocationId"] != o.parent.invocation_id()
            || frame["turnCorrelation"] != o.parent.turn().unwrap().as_str()
        {
            return Err(refused("coding provider owner correlation conflict"));
        }
        if self.provider_calls == 2 {
            let workspace = frame["host_workspace"].clone();
            let raw = workspace["manifest_bytes"]
                .as_str()
                .ok_or_else(|| refused("Host stable workspace absent"))?;
            if digest(raw.as_bytes()) != workspace["manifest"]["sha256"] {
                return Err(refused("Host workspace manifest binding"));
            }
            let result = o
                .result
                .clone()
                .ok_or_else(|| refused("Host tool result unresolved"))?;
            self.original_result = Some(result.clone());
            self.workspace = Some(workspace.clone());
            let e = json!({"type":"pi.coding_workspace","invocationId":o.parent.invocation_id(),"manifest":workspace["manifest"],"manifest_bytes":raw});
            o.record("pi.coding_workspace", &e)?;
        }
        if self.provider_calls == 2 && !self.live() {
            let workspace = self.workspace.clone().unwrap();
            let result = self.original_result.clone().unwrap();
            let checkpoint = json!({"version":1,"successor_context":"context:new","fixture":true,"coding_result":result["result"],"coding_result_sha256":result["artifact"]["sha256"],"workspace_manifest_sha256":workspace["manifest"]["sha256"],"tool_executions":1,"unanswered_human":{"authority":"human","resolved":false},"canonical_budget":self.config.budget});
            let text = checkpoint.to_string();
            let staged = o
                .store
                .stage_artifact(
                    SubjectId::parse("subject:pi:coding").map_err(refused)?,
                    text.as_bytes(),
                )
                .map_err(refused)?;
            o.store
                .publish_artifact(staged, (self.clock)())
                .map_err(refused)?;
            self.checkpoint = Some(text);
        }
        let mut request = frame.clone();
        request["type"] = json!("pi.provider_request");
        if let Some(m) = request.as_object_mut() {
            m.remove("host_workspace");
            m.remove("schema");
        }
        o.record("pi.provider_request", &request)?;
        if self.live() {
            // Real inference: no manufactured reply. This is only a current-grant
            // entry to the Host-reserved provider request (consumption deadline
            // is added when the exact IPC write is dispatched).
            drop(o);
            return self.queue(json!({"type":"bridge_provider","sessionId":frame["sessionId"],"id":self.provider_calls,"mode":"live"}));
        }
        let reply = match self.provider_calls {
            1 => Value::Null,
            2 => json!(self.checkpoint.as_ref().unwrap()),
            3 => json!(
                "Exact coding result and workspace recovered; unanswered human remains unresolved."
            ),
            4 => json!("42; continued text-only coding fixture work."),
            _ => unreachable!(),
        };
        drop(o);
        self.queue(json!({"type":"bridge_provider","sessionId":frame["sessionId"],"id":self.provider_calls,"mode":if self.provider_calls==1{"tool"}else{"text"},"reply":reply}))
    }
    fn record_native(&mut self, frame: &Value) -> Result<Vec<Value>> {
        if !matches!(self.phase, 0 | 2 | 3) {
            return Err(refused("native event outside an admitted turn"));
        }
        let gate = self.gate.as_ref().unwrap().clone();
        let mut o = gate.0.borrow_mut();
        if frame["sessionId"] != o.parent.thread().as_str() {
            return Err(refused("foreign coding native event"));
        }
        let event = &frame["event"];
        let kind = event["type"]
            .as_str()
            .ok_or_else(|| refused("native event kind"))?;
        o.record(kind, event)?;
        Ok(vec![])
    }
    fn history(&mut self, frame: &Value) -> Result<Vec<Value>> {
        let gate = self.gate.as_ref().unwrap().clone();
        let mut o = gate.0.borrow_mut();
        if frame["sessionId"] != o.parent.thread().as_str()
            || frame["invocationId"] != o.parent.invocation_id()
            || frame["snapshot"]["idle"] != true
        {
            return Err(refused("coding history mismatch"));
        }
        self.snapshot(frame)?;
        self.observations.push(frame["snapshot"].clone());
        let mut history = frame.clone();
        history["type"] = json!("pi.session_history");
        if let Some(m) = history.as_object_mut() {
            m.remove("schema");
            m.remove("host_workspace");
        }
        o.record("pi.session_history", &history)?;
        let proof = PiSettlementProof {
            transport: o.transport.clone(),
            first_sequence: self.first_sequence,
            history_sequence: o.sequence,
        };
        let CodingLifecycle { store, parent, .. } = &mut *o;
        store
            .settle_pi_coding(parent, &proof, (self.clock)())
            .map_err(refused)?;
        if self.config.task.is_some() && self.phase == 0 && !self.battery {
            // Owner-derived v4 checkpoint, read back from the settled turn. The
            // model's final text is provenance only (digest inside it).
            if self.original_result.is_none() || self.workspace.is_none() {
                return Err(refused(
                    "live settlement without Host tool result/workspace",
                ));
            }
            let text = store.pi_coding_live_checkpoint(parent).map_err(refused)?;
            let staged = store
                .stage_artifact(
                    SubjectId::parse("subject:pi:coding").map_err(refused)?,
                    text.as_bytes(),
                )
                .map_err(refused)?;
            store
                .publish_artifact(staged, (self.clock)())
                .map_err(refused)?;
            self.checkpoint = Some(text);
        }
        drop(o);
        if self.battery {
            match self.phase {
                0 | 3 => self.battery_after_turn(frame),
                2 => {
                    self.battery_finish_rehydration()?;
                    self.phase = 3;
                    self.battery_continue()
                }
                _ => Err(refused("unexpected coding history phase")),
            }
        } else if self.phase == 0 {
            gate.replacement_ready()?;
            self.prepare_reset(frame)
        } else if self.phase == 2 {
            self.finish_rehydration()?;
            let prompt = "Continue text-only work after exact coding recovery: compute 6*7.";
            self.phase = 3;
            let grant = self.turn_grant("grant:new");
            self.prompt(
                "continued",
                prompt,
                NativeInvocationPurpose::DomainWork,
                &grant,
            )
        } else if self.phase == 3 {
            let mut o = gate.0.borrow_mut();
            let subject = o
                .store
                .subject_projection(
                    &SubjectId::parse("subject:pi:coding").map_err(refused)?,
                    None,
                    None,
                )
                .map_err(refused)?;
            self.report = Some(
                json!({"schema":"work-engine.pi-coding-continuity-result.v1","standing":if self.live() {"live-provider disposable continuity pilot; real inference; no production/domain acceptance"} else {"provider-disabled disposable qualification; no production/domain acceptance"},"mode":if self.live() {"live"} else {"fixture"},"context":subject.context_id,"owner":subject.owner_kind,"checkpoint":self.checkpoint,"original_host_result":self.original_result,"workspace":self.workspace,"observations":self.observations,"fresh_successor":self.fresh_successor,"predecessor_history_external_only":true,"provider_requests":self.provider_calls,"tool_executions":1,"physical_resets":1,"recovery_receipt":self.recovery_receipt,"continued_parent":o.store.native_recovery("continued").map_err(refused)?.unwrap().class}),
            );
            if self.config.hosted.is_some() {
                self.report.as_mut().unwrap()["hosted_attempts"] = json!(self.hosted_log);
            }
            self.phase = 4;
            self.queue(json!({"type":"bridge_close","sessionId":frame["sessionId"]}))
        } else {
            Err(refused("unexpected coding history phase"))
        }
    }
    fn prepare_reset(&mut self, frame: &Value) -> Result<Vec<Value>> {
        self.check()?;
        let pressure = json!({"schemaVersion":1,"observationId":"coding-fixture:1","sequence":1,"observedAt":frame["snapshot"]["observedAt"],"pressureBasisPoints":self.config.pressure_basis_points,"source":if self.live() {"explicit operator-requested continuity replacement; not measured quota or context exhaustion; actual SDK observation retained separately"} else {"explicit outside fixture pressure; actual SDK observation retained separately"},"sourceRevision":format!("sha256:{}",digest(b"coding-pressure-fixture-v1"))});
        if !crate::pressure_candidate(Some(&pressure)).map_err(refused)? {
            return Err(refused("no pressure candidate"));
        }
        let gate = self.gate.as_ref().unwrap().clone();
        let mut o = gate.0.borrow_mut();
        let subject = SubjectId::parse("subject:pi:coding").map_err(refused)?;
        let rev = o
            .store
            .subject_projection(&subject, None, None)
            .map_err(refused)?
            .revision;
        let result = o
            .store
            .apply_checked_command(replacement_admission(rev)?, (self.clock)())
            .map_err(refused)?;
        let transition = match result.outcome {
            CommandOutcome::ReplacementRequested { transition } => transition,
            _ => return Err(refused("replacement not admitted")),
        };
        let semantic = o
            .store
            .subject_projection(&subject, None, None)
            .map_err(refused)?
            .semantic_revision;
        let checkpoint = self.checkpoint.as_ref().unwrap();
        let hash = digest(checkpoint.as_bytes());
        for fact in [
            TransitionFact::PredecessorSafe {
                evidence: EvidenceId::parse("coding:host-writers-absent").map_err(refused)?,
            },
            TransitionFact::SourceFrozen {
                evidence: EvidenceId::parse("coding:exact-result-workspace").map_err(refused)?,
                semantic_revision: Revision::new(semantic),
            },
            TransitionFact::VerificationEstablished {
                evidence: EvidenceId::parse(if self.live() {
                    "coding:owner-derived-live-checkpoint"
                } else {
                    "coding:fixture-checkpoint"
                })
                .map_err(refused)?,
                checkpoint: CheckpointId::parse(format!("checkpoint:{hash}")).map_err(refused)?,
            },
            TransitionFact::SwitchAuthorized {
                evidence: EvidenceId::parse(if self.live() {
                    "coding:operator-live-grant"
                } else {
                    "coding:operator-fixture-grant"
                })
                .map_err(refused)?,
            },
        ] {
            let rev = o
                .store
                .subject_projection(&subject, None, None)
                .map_err(refused)?
                .revision;
            o.store
                .apply_transition_fact(&transition, Revision::new(rev), fact, None, (self.clock)())
                .map_err(refused)?;
        }
        let successor = o
            .store
            .reserve_native_session(
                NativeSessionId::parse("session:new").map_err(refused)?,
                subject,
                ContextGeneration::parse("context:new").map_err(refused)?,
                RuntimeIncarnation::parse("incarnation:pi:coding").map_err(refused)?,
                &if self.live() {
                    pi_coding_live_profile_digest()
                } else {
                    pi_coding_profile_digest()
                },
                PI_SDK_INDEX_SHA256,
                &if self.live() {
                    pi_coding_live_protocol_digest()
                } else {
                    pi_coding_protocol_digest()
                },
                SnapshotId::parse("snapshot:coding").map_err(refused)?,
                &hash,
                false,
                (self.clock)(),
            )
            .map_err(refused)?;
        self.transition = Some(transition);
        self.successor = Some(successor);
        self.phase = 1;
        drop(o);
        self.queue(json!({"type":"bridge_replace","sessionId":frame["sessionId"]}))
    }
    fn replaced(&mut self, frame: &Value) -> Result<Vec<Value>> {
        fresh_successor(frame)?;
        self.fresh_successor = Some(frame.clone());
        self.check()?;
        self.snapshot(frame)?;
        if self.phase != 1
            || !self.reset_entered
            || frame["result"]["cancelled"] != false
            || frame["messages"] != json!([])
            || frame["tools"] != json!([])
            || frame["snapshot"]["idle"] != true
            || frame["oldSessionId"] != self.session.as_ref().unwrap().thread().unwrap().as_str()
            || frame["oldSessionId"] == frame["sessionId"]
            || frame["snapshot"]["sessionId"] != frame["sessionId"]
        {
            return Err(refused("physical fresh replacement not established"));
        }
        let gate = self.gate.as_ref().unwrap().clone();
        let mut o = gate.0.borrow_mut();
        let mut successor = self
            .successor
            .take()
            .ok_or_else(|| refused("successor permit consumed"))?;
        o.store
            .bind_native_thread(
                &mut successor,
                ProviderThreadId::parse(
                    frame["sessionId"]
                        .as_str()
                        .ok_or_else(|| refused("successor UUID"))?,
                )
                .map_err(refused)?,
            )
            .map_err(refused)?;
        let subject = SubjectId::parse("subject:pi:coding").map_err(refused)?;
        let rev = o
            .store
            .subject_projection(&subject, None, None)
            .map_err(refused)?
            .revision;
        o.store
            .apply_transition_fact(
                self.transition.as_ref().unwrap(),
                Revision::new(rev),
                TransitionFact::SuccessorObserved {
                    evidence: EvidenceId::parse("coding:actual-newSession").map_err(refused)?,
                    context: ContextGeneration::parse("context:new").map_err(refused)?,
                    thread: ProviderThreadId::parse(frame["sessionId"].as_str().unwrap())
                        .map_err(refused)?,
                },
                None,
                (self.clock)(),
            )
            .map_err(refused)?;
        o.session = successor.id().clone();
        self.session = Some(successor);
        self.phase = 2;
        self.observations.push(frame["snapshot"].clone());
        drop(o);
        let checkpoint = self.checkpoint.clone().unwrap();
        let grant = self.turn_grant("grant:new");
        self.prompt(
            "rehydrate",
            &checkpoint,
            NativeInvocationPurpose::SuccessorRehydrate,
            &grant,
        )
    }
    fn finish_rehydration(&mut self) -> Result<()> {
        let gate = self.gate.as_ref().unwrap().clone();
        let mut o = gate.0.borrow_mut();
        let subject = SubjectId::parse("subject:pi:coding").map_err(refused)?;
        let semantic = o
            .store
            .subject_projection(&subject, None, None)
            .map_err(refused)?
            .semantic_revision;
        let transition = self.transition.as_ref().unwrap();
        let receipt = o
            .store
            .native_rehydration_receipt(transition)
            .map_err(refused)?
            .ok_or_else(|| refused("rehydration receipt absent"))?;
        self.recovery_receipt = Some(receipt.clone());
        for fact in [
            TransitionFact::RehydrationVerified {
                evidence: EvidenceId::parse(format!("{receipt}:rehydration")).map_err(refused)?,
            },
            TransitionFact::CommitSuccessor {
                evidence: EvidenceId::parse("coding:commit").map_err(refused)?,
                semantic_revision: Revision::new(semantic),
            },
        ] {
            let rev = o
                .store
                .subject_projection(&subject, None, None)
                .map_err(refused)?
                .revision;
            o.store
                .apply_transition_fact(
                    transition,
                    Revision::new(rev),
                    fact,
                    Some(&GrantId::parse("grant:successor").map_err(refused)?),
                    (self.clock)(),
                )
                .map_err(refused)?;
        }
        Ok(())
    }
    // ---- battery mode: one bounded long-form builder run -------------------
    // The choreography's replacement machinery, parametrized by cycle index
    // (planning/openrouter-lane-e1a-battery-20261120.md): cycle c >= 1 names
    // successor generation context:new{c} with its own grants, command,
    // session and evidence ids. The issuer (the producer) installs every
    // cycle's grants up front, bounded by the packet's max_resets.
    fn battery_successor_context(cycle: u64) -> Result<String> {
        ContextGeneration::parse(format!("context:new{cycle}"))
            .map(|g| g.as_str().to_string())
            .map_err(refused)
    }
    fn battery_after_turn(&mut self, frame: &Value) -> Result<Vec<Value>> {
        self.check()?;
        // Completion signal: the durable workspace artifact the task told the
        // builder to write. The model's final text is provenance only.
        let completed = self
            .config
            .battery_workspace
            .as_ref()
            .and_then(|root| std::fs::read(root.join("evidence/completion.json")).ok())
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .is_some_and(|v| v["status"] == "complete" || v["status"] == "handoff_ready");
        if completed {
            self.battery_report()?;
            self.phase = 4;
            return self.queue(json!({"type":"bridge_close","sessionId":frame["sessionId"]}));
        }
        let pressure = self.battery_max_context_bytes > 0
            && self.last_context_bytes.saturating_mul(10000)
                >= self
                    .battery_max_context_bytes
                    .saturating_mul(self.replacement_basis_points);
        if pressure {
            if self.resets_done >= self.battery_max_resets {
                return Err(refused("battery reset bound exhausted under pressure"));
            }
            return self.battery_prepare_reset(frame, self.resets_done + 1);
        }
        self.phase = 3;
        self.battery_continue()
    }
    fn battery_continue(&mut self) -> Result<Vec<Value>> {
        self.turn_counter += 1;
        let id = format!("cont-{}", self.turn_counter);
        let grant = if self.resets_done == 0 {
            self.turn_grant("grant:old")
        } else {
            self.turn_grant(&format!("grant:new{}", self.resets_done))
        };
        self.prompt(
            &id,
            "Continue the assigned builder work exactly where it stopped; obey the task's original instructions and its completion artifact obligation.",
            NativeInvocationPurpose::DomainWork,
            &grant,
        )
    }
    fn battery_prepare_reset(&mut self, frame: &Value, cycle: u64) -> Result<Vec<Value>> {
        self.check()?;
        let pressure = json!({"schemaVersion":1,"observationId":format!("coding-battery:{cycle}"),"sequence":cycle,"observedAt":frame["snapshot"]["observedAt"],"pressureBasisPoints":self.config.pressure_basis_points,"source":format!("battery context-pressure replacement cycle {cycle}; pinned replacement policy, not measured quota; actual SDK observation retained separately"),"sourceRevision":format!("sha256:{}",digest(b"coding-pressure-fixture-v1"))});
        if !crate::pressure_candidate(Some(&pressure)).map_err(refused)? {
            return Err(refused("no pressure candidate"));
        }
        let gate = self.gate.as_ref().unwrap().clone();
        let mut o = gate.0.borrow_mut();
        let successor_context = Self::battery_successor_context(cycle)?;
        // Owner-derived v4 checkpoint of the just-settled domain turn, labeled
        // with this cycle's successor generation (never stored; re-derived).
        let CodingLifecycle { store, parent, .. } = &mut *o;
        let checkpoint = store
            .pi_coding_live_checkpoint_with_successor(parent, &successor_context)
            .map_err(refused)?;
        self.checkpoint = Some(checkpoint.clone());
        let subject = SubjectId::parse("subject:pi:coding").map_err(refused)?;
        let rev = o
            .store
            .subject_projection(&subject, None, None)
            .map_err(refused)?
            .revision;
        let result = o
            .store
            .apply_checked_command(battery_replacement_admission(rev, cycle)?, (self.clock)())
            .map_err(refused)?;
        let transition = match result.outcome {
            CommandOutcome::ReplacementRequested { transition } => transition,
            _ => return Err(refused("replacement not admitted")),
        };
        let semantic = o
            .store
            .subject_projection(&subject, None, None)
            .map_err(refused)?
            .semantic_revision;
        let hash = digest(checkpoint.as_bytes());
        for fact in [
            TransitionFact::PredecessorSafe {
                evidence: EvidenceId::parse(format!("coding:host-writers-absent-{cycle}"))
                    .map_err(refused)?,
            },
            TransitionFact::SourceFrozen {
                evidence: EvidenceId::parse(format!("coding:exact-result-workspace-{cycle}"))
                    .map_err(refused)?,
                semantic_revision: Revision::new(semantic),
            },
            TransitionFact::VerificationEstablished {
                evidence: EvidenceId::parse(format!(
                    "coding:owner-derived-live-checkpoint-{cycle}"
                ))
                .map_err(refused)?,
                checkpoint: CheckpointId::parse(format!("checkpoint:{hash}")).map_err(refused)?,
            },
            TransitionFact::SwitchAuthorized {
                evidence: EvidenceId::parse(format!("coding:operator-live-grant-{cycle}"))
                    .map_err(refused)?,
            },
        ] {
            let rev = o
                .store
                .subject_projection(&subject, None, None)
                .map_err(refused)?
                .revision;
            o.store
                .apply_transition_fact(&transition, Revision::new(rev), fact, None, (self.clock)())
                .map_err(refused)?;
        }
        let successor = o
            .store
            .reserve_native_session(
                NativeSessionId::parse(format!("session:new{cycle}")).map_err(refused)?,
                subject,
                ContextGeneration::parse(&successor_context).map_err(refused)?,
                RuntimeIncarnation::parse("incarnation:pi:coding").map_err(refused)?,
                &pi_coding_live_profile_digest(),
                PI_SDK_INDEX_SHA256,
                &pi_coding_live_protocol_digest(),
                SnapshotId::parse("snapshot:coding").map_err(refused)?,
                &hash,
                false,
                (self.clock)(),
            )
            .map_err(refused)?;
        self.transition = Some(transition);
        self.successor = Some(successor);
        self.phase = 1;
        drop(o);
        self.queue(json!({"type":"bridge_replace","sessionId":frame["sessionId"]}))
    }
    fn battery_replaced(&mut self, frame: &Value) -> Result<Vec<Value>> {
        fresh_successor(frame)?;
        self.fresh_successor = Some(frame.clone());
        self.check()?;
        self.snapshot(frame)?;
        if self.phase != 1
            || !self.reset_entered
            || frame["result"]["cancelled"] != false
            || frame["messages"] != json!([])
            || frame["tools"] != json!([])
            || frame["snapshot"]["idle"] != true
            || frame["oldSessionId"] != self.session.as_ref().unwrap().thread().unwrap().as_str()
            || frame["oldSessionId"] == frame["sessionId"]
            || frame["snapshot"]["sessionId"] != frame["sessionId"]
        {
            return Err(refused("physical fresh replacement not established"));
        }
        let gate = self.gate.as_ref().unwrap().clone();
        let mut o = gate.0.borrow_mut();
        let mut successor = self
            .successor
            .take()
            .ok_or_else(|| refused("successor permit consumed"))?;
        o.store
            .bind_native_thread(
                &mut successor,
                ProviderThreadId::parse(
                    frame["sessionId"]
                        .as_str()
                        .ok_or_else(|| refused("successor UUID"))?,
                )
                .map_err(refused)?,
            )
            .map_err(refused)?;
        let subject = SubjectId::parse("subject:pi:coding").map_err(refused)?;
        let rev = o
            .store
            .subject_projection(&subject, None, None)
            .map_err(refused)?
            .revision;
        o.store
            .apply_transition_fact(
                self.transition.as_ref().unwrap(),
                Revision::new(rev),
                TransitionFact::SuccessorObserved {
                    evidence: EvidenceId::parse("coding:actual-newSession").map_err(refused)?,
                    context: ContextGeneration::parse(Self::battery_successor_context(
                        self.resets_done + 1,
                    )?)
                    .map_err(refused)?,
                    thread: ProviderThreadId::parse(frame["sessionId"].as_str().unwrap())
                        .map_err(refused)?,
                },
                None,
                (self.clock)(),
            )
            .map_err(refused)?;
        o.session = successor.id().clone();
        self.session = Some(successor);
        self.resets_done += 1;
        self.reset_entered = false;
        self.phase = 2;
        self.observations.push(frame["snapshot"].clone());
        drop(o);
        let checkpoint = self.checkpoint.clone().unwrap();
        let grant = self.turn_grant(&format!("grant:new{}", self.resets_done));
        self.prompt(
            "rehydrate",
            &checkpoint,
            NativeInvocationPurpose::SuccessorRehydrate,
            &grant,
        )
    }
    fn battery_finish_rehydration(&mut self) -> Result<()> {
        let gate = self.gate.as_ref().unwrap().clone();
        let mut o = gate.0.borrow_mut();
        let subject = SubjectId::parse("subject:pi:coding").map_err(refused)?;
        let semantic = o
            .store
            .subject_projection(&subject, None, None)
            .map_err(refused)?
            .semantic_revision;
        let transition = self.transition.as_ref().unwrap();
        let receipt = o
            .store
            .native_rehydration_receipt(transition)
            .map_err(refused)?
            .ok_or_else(|| refused("rehydration receipt absent"))?;
        self.recovery_receipt = Some(receipt.clone());
        for fact in [
            TransitionFact::RehydrationVerified {
                evidence: EvidenceId::parse(format!("{receipt}:rehydration")).map_err(refused)?,
            },
            TransitionFact::CommitSuccessor {
                evidence: EvidenceId::parse("coding:commit").map_err(refused)?,
                semantic_revision: Revision::new(semantic),
            },
        ] {
            let rev = o
                .store
                .subject_projection(&subject, None, None)
                .map_err(refused)?
                .revision;
            o.store
                .apply_transition_fact(
                    transition,
                    Revision::new(rev),
                    fact,
                    Some(
                        &GrantId::parse(format!("grant:successor{}", self.resets_done))
                            .map_err(refused)?,
                    ),
                    (self.clock)(),
                )
                .map_err(refused)?;
        }
        Ok(())
    }
    fn battery_report(&mut self) -> Result<()> {
        let gate = self.gate.as_ref().unwrap().clone();
        let mut o = gate.0.borrow_mut();
        let subject = o
            .store
            .subject_projection(
                &SubjectId::parse("subject:pi:coding").map_err(refused)?,
                None,
                None,
            )
            .map_err(refused)?;
        let continued = self
            .last_continued
            .clone()
            .ok_or_else(|| refused("battery continuation turn absent"))?;
        self.report = Some(
            json!({"schema":"work-engine.pi-coding-continuity-result.v1","standing":"live-provider battery builder run; real inference; acceptance is the coordinator's F3.1 scoring of the produced commit, not a production/domain claim","mode":"battery","context":subject.context_id,"owner":subject.owner_kind,"checkpoint":self.checkpoint,"original_host_result":self.original_result,"workspace":self.workspace,"observations":self.observations,"fresh_successor":self.fresh_successor,"predecessor_history_external_only":true,"provider_requests":self.provider_calls,"tool_executions":self.tool_executions,"physical_resets":self.resets_done,"completion_signal":true,"recovery_receipt":self.recovery_receipt,"continued_parent":o.store.native_recovery(&continued).map_err(refused)?.map(|r| r.class).ok_or_else(|| refused("battery continuation unsettled"))?}),
        );
        if self.config.hosted.is_some() {
            self.report.as_mut().unwrap()["hosted_attempts"] = json!(self.hosted_log);
        }
        Ok(())
    }
}
fn fresh_successor(frame: &Value) -> Result<()> {
    if frame["messages"] != json!([])
        || frame["autoCompactionEnabled"] != false
        || frame["header"]["type"] != "session"
        || frame["header"]["id"] != frame["sessionId"]
    {
        return Err(refused("fresh successor inherited context/compaction"));
    }
    let branch = frame["branch"]
        .as_array()
        .ok_or_else(|| refused("fresh branch absent"))?;
    let mut parent = Value::Null;
    let mut ids = std::collections::BTreeSet::new();
    for row in branch {
        let id = row["id"]
            .as_str()
            .ok_or_else(|| refused("fresh branch identity"))?;
        let kind = row["type"]
            .as_str()
            .ok_or_else(|| refused("fresh metadata kind"))?;
        if !matches!(kind, "model_change" | "thinking_level_change")
            || !ids.insert(id)
            || row["parentId"] != parent
        {
            return Err(refused(
                "fresh branch contains inherited conversation/summary",
            ));
        }
        parent = json!(id);
    }
    if frame["leafId"] != parent {
        return Err(refused("fresh leaf identity"));
    }
    Ok(())
}
fn replacement_admission(revision: u64) -> Result<CommandAdmission> {
    use work_engine_types::CodecContract;
    let basis = json!({"protocol_version":1,"principal_ref":"operator:fixture","command_id":"command:replace","subject_id":"subject:pi:coding","context_generation":"context:old","build_id":"build:pi:coding","proof_run_id":"proof:pi:coding","grant_ref":"grant:replace","expected_revision":revision.to_string(),"kind":"request_replacement","payload":{"reason":"operator-approved coding fixture replacement"}});
    let req = CommandRequest::new(
        CommandId::parse("command:replace").map_err(refused)?,
        SubjectId::parse("subject:pi:coding").map_err(refused)?,
        ContextGeneration::parse("context:old").map_err(refused)?,
        BuildId::parse("build:pi:coding").map_err(refused)?,
        ProofRunId::parse("proof:pi:coding").map_err(refused)?,
        GrantId::parse("grant:replace").map_err(refused)?,
        Revision::new(revision),
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .map_err(refused)?,
        Command::RequestReplacement,
    )
    .map_err(refused)?;
    CommandAdmission::new(req, "operator:fixture".into(), basis, None)
        .map_err(|e| refused(format!("{e:?}")))
}
/// The battery profile's per-cycle replacement command: cycle c replaces the
/// cycle c-1 successor generation (cycle 1 replaces context:old) and names
/// its own successor context:new{c} with its own command and grant ids.
fn battery_replacement_admission(revision: u64, cycle: u64) -> Result<CommandAdmission> {
    use work_engine_types::CodecContract;
    let predecessor = if cycle == 1 {
        "context:old".to_string()
    } else {
        format!("context:new{}", cycle - 1)
    };
    let basis = json!({"protocol_version":1,"principal_ref":"operator:fixture","command_id":format!("command:replace{cycle}"),"subject_id":"subject:pi:coding","context_generation":predecessor,"build_id":"build:pi:coding","proof_run_id":"proof:pi:coding","grant_ref":format!("grant:replace{cycle}"),"expected_revision":revision.to_string(),"kind":"request_replacement","payload":{"reason":format!("battery context-pressure replacement cycle {cycle}")}});
    let req = CommandRequest::new(
        CommandId::parse(format!("command:replace{cycle}")).map_err(refused)?,
        SubjectId::parse("subject:pi:coding").map_err(refused)?,
        ContextGeneration::parse(&predecessor).map_err(refused)?,
        BuildId::parse("build:pi:coding").map_err(refused)?,
        ProofRunId::parse("proof:pi:coding").map_err(refused)?,
        GrantId::parse(format!("grant:replace{cycle}")).map_err(refused)?,
        Revision::new(revision),
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .map_err(refused)?,
        Command::RequestReplacement,
    )
    .map_err(refused)?;
    CommandAdmission::new(req, "operator:fixture".into(), basis, None)
        .map_err(|e| refused(format!("{e:?}")))
}
impl CodingEffectGate for CodingContinuity {
    fn execute(
        &mut self,
        intent: &CodingToolIntent,
        actuator: &mut dyn FnMut(i64) -> Result<(bool, Vec<u8>)>,
    ) -> Result<(bool, Vec<u8>)> {
        let state = self.0.borrow();
        state.check()?;
        if state.phase != 0 && !(state.battery && state.phase == 3) {
            return Err(refused("recovery/continuation tools forbidden"));
        }
        state
            .gate
            .as_ref()
            .unwrap()
            .clone()
            .execute(intent, actuator)
    }
    fn observed(
        &mut self,
        intent: &CodingToolIntent,
        result: &Artifact,
        process: &ProcessRecord,
        container: &str,
    ) -> Result<()> {
        self.0.borrow_mut().tool_executions += 1;
        self.0
            .borrow()
            .gate
            .as_ref()
            .unwrap()
            .clone()
            .observed(intent, result, process, container)
    }
    fn frame(&mut self, frame: &Value) -> Result<Vec<Value>> {
        let mut s = self.0.borrow_mut();
        s.check()?;
        match frame["type"].as_str() {
            Some("ready") => s.ready(frame),
            Some("native_event") => s.record_native(frame),
            Some("coding_provider_request") => s.provider(frame),
            Some("coding_history") => s.history(frame),
            Some("coding_replaced") => {
                if s.battery {
                    s.battery_replaced(frame)
                } else {
                    s.replaced(frame)
                }
            }
            _ => Err(refused("unsupported coding frame")),
        }
    }
    fn provider_attempt(
        &mut self,
        intent: &ProviderAttemptIntent,
        dispatch: &mut dyn FnMut(i64) -> Result<()>,
    ) -> Result<()> {
        self.0.borrow_mut().provider_attempt(intent, dispatch)
    }
    fn dispatch(&mut self, command: &Value, send: &mut dyn FnMut(i64) -> Result<()>) -> Result<()> {
        let mut s = self.0.borrow_mut();
        s.check()?;
        // Tool results originate in the Host journal, not the lifecycle control
        // queue. Bind the exact observed result and consume its sole IPC attempt
        // before entering the current parent/grant transaction. A failed write
        // stays uncertain and must never be replayed.
        if command["type"] == "tool_result" {
            if s.tool_result_sent || !(s.phase == 0 || (s.battery && s.phase == 3)) {
                return Err(refused("coding tool result already consumed/forbidden"));
            }
            let gate = s
                .gate
                .as_ref()
                .ok_or_else(|| refused("coding parent absent"))?
                .clone();
            let mut o = gate.0.borrow_mut();
            let evidence = o
                .result
                .as_ref()
                .ok_or_else(|| refused("Host tool result absent"))?;
            let artifact: Artifact = serde_json::from_value(evidence["artifact"].clone())?;
            let recorded: Value = serde_json::from_slice(&artifact.verify()?)?;
            if command != &evidence["result"] || command != &recorded {
                return Err(refused("coding tool result differs from Host custody"));
            }
            s.tool_result_sent = true;
            let clock = (s.clock)();
            let CodingLifecycle { store, parent, .. } = &mut *o;
            return store
                .with_pi_coding_turn_send(parent, clock, send)
                .map_err(refused)?;
        }
        let mut command = command.clone();
        command
            .as_object_mut()
            .ok_or_else(|| refused("coding control object"))?
            .remove("command");
        let expected = s
            .pending
            .take()
            .ok_or_else(|| refused("coding command consumed"))?;
        if command != expected {
            return Err(refused("coding command differs from admitted bytes"));
        }
        let gate = s.gate.as_ref().unwrap().clone();
        let mut o = gate.0.borrow_mut();
        let clock = (s.clock)();
        if command["type"] == "bridge_replace" {
            if s.reset_entered {
                return Err(refused("reset already attempted"));
            }
            s.reset_entered = true;
            let CodingLifecycle { store, .. } = &mut *o;
            store
                .with_pi_coding_reset(
                    s.successor.as_ref().unwrap(),
                    s.transition.as_ref().unwrap(),
                    s.checkpoint.as_ref().unwrap(),
                    &if s.battery {
                        GrantId::parse(format!("grant:replace{}", s.resets_done + 1))
                            .map_err(refused)?
                    } else {
                        GrantId::parse("grant:replace").map_err(refused)?
                    },
                    clock,
                    send,
                )
                .map_err(refused)?
        } else if command["type"] == "bridge_close" {
            if s.phase != 4 {
                return Err(refused("close before continuation"));
            }
            send(s.config.expires_ms)
        } else {
            let CodingLifecycle { store, parent, .. } = &mut *o;
            store
                .with_pi_coding_turn_send(parent, clock, send)
                .map_err(refused)?
        }
    }
}

#[cfg(test)]
mod control_tests {
    use super::*;
    #[test]
    fn canonical_budget_refuses_future_stale_unavailable_and_wrong_binding() {
        let sample = ClockSample {
            wall: WallTimeMs::new(500000),
            wait_budget: WaitBudgetMs::new(100),
        };
        for case in 0..7 {
            let mut config = CodingConfig {
                task: None,
                budget: json!({"authority":"observation_only","sequence":1,"expires_ms":1000000,"source":"root-canonical-fixture","generation":"bound:1","observed_ms":500000,"max_age_ms":300000,"available":true}),
                expires_ms: 1000000,
                pressure_basis_points: 9500,
                hosted: None,
                battery: false,
                battery_workspace: None,
            };
            match case {
                1 => config.budget["observed_ms"] = json!(500001),
                2 => config.budget["observed_ms"] = json!(199999),
                3 => config.budget["available"] = json!(false),
                4 => config.budget["expires_ms"] = json!(2000000),
                5 => config.budget["generation"] = json!(""),
                6 => config.budget["authority"] = json!("execution_grant"),
                _ => {}
            }
            assert_eq!(
                budget_current(&config, sample).is_ok(),
                case == 0,
                "case {case}"
            );
        }
    }
}

#[cfg(test)]
mod fresh_context_tests {
    use super::*;
    #[test]
    fn successor_cannot_carry_conversation_summary_or_compaction() {
        for case in 0..6 {
            let mut frame = json!({"sessionId":"new","messages":[],"autoCompactionEnabled":false,"header":{"type":"session","id":"new"},"branch":[],"leafId":null});
            match case {
                1 => frame["messages"] = json!([{"role":"assistant","content":"old summary"}]),
                2 => frame["autoCompactionEnabled"] = json!(true),
                3 => {
                    frame["branch"] = json!([{"id":"old","parentId":null,"type":"model_change"}]);
                    frame["leafId"] = json!("old");
                }
                4 => {
                    frame["branch"] = json!([{"id":"summary","parentId":null,"type":"compaction","summary":"old meaning"}]);
                    frame["leafId"] = json!("summary");
                }
                5 => {
                    frame["header"]["parentSession"] = json!("external-predecessor-file");
                }
                _ => {}
            }
            assert_eq!(
                fresh_successor(&frame).is_ok(),
                matches!(case, 0 | 3 | 5),
                "case {case}"
            );
        }
    }
}
