//! Isolated native control turn over the same pinned app-server transport and
//! lifecycle-store entry/ingress/settlement owner as S5 domain turns.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use lifecycle_codex::{
    NATIVE_BINARY_SHA256, NativeEvent, NativeEventKind, NativeHarness, SCHEMA_THREAD_START_SHA256,
};
#[cfg(feature = "native-sim-proof")]
use lifecycle_core::Revision;
use lifecycle_core::{
    AttemptId, ContextGeneration, GrantId, NativeInvocationPurpose, NativeSessionId,
    ProviderThreadId, ProviderTurnId, RuntimeIncarnation, SnapshotId, SubjectId, TransitionId,
};
use lifecycle_semantic::{SemanticInvocationBinding, SemanticPhase};
use lifecycle_store::{
    ControlSessionRequest, NativeFinalMaterial, NativeTurnRequest, SemanticControlRole,
    SqliteLifecycleStore, StoreError,
};
use serde_json::{Value, json};
use tokio::time::timeout;

use crate::{NativeConfig, SemanticNativeConfig, actor::clock_sample, operations};

pub struct ControlTurnSpec<'a> {
    pub subject: SubjectId,
    pub context: ContextGeneration,
    pub transition: TransitionId,
    pub incarnation: RuntimeIncarnation,
    pub snapshot: SnapshotId,
    pub snapshot_manifest_sha256: &'a str,
    pub role: SemanticControlRole,
    pub phase: SemanticPhase,
    pub session_id: &'a str,
    pub invocation_id: &'a str,
    pub attempt_id: &'a str,
    pub prompt: &'a str,
    pub policy_id: &'a str,
    pub exposure_id: String,
}

pub struct ControlTurnResult {
    pub binding: SemanticInvocationBinding,
    pub final_material: NativeFinalMaterial,
    pub observed_provider: String,
    pub observed_model: String,
    pub exposure_ref: String,
}

struct DirectRunner<'a> {
    store: &'a mut SqliteLifecycleStore,
    session: NativeSessionId,
    harness: NativeHarness,
    terminal: Option<lifecycle_codex::NativeTurnTerminal>,
    gate: &'a AtomicBool,
}

/// The simulator-only cut lets an external test operator request a real owner
/// revocation while this process holds the sole store writer. It never changes
/// the result or stage directly and is compiled out of ordinary service builds.
fn optional_proof_revoke(
    store: &mut SqliteLifecycleStore,
    proof_dir: Option<&Path>,
    point: &str,
    grant: &str,
    revision: u64,
) -> Result<(), StoreError> {
    #[cfg(feature = "native-sim-proof")]
    if proof_dir.is_some_and(|root| root.join(format!("{point}.revoke_grant")).exists()) {
        store.revoke_trusted_grant(
            &GrantId::parse(grant.to_owned()).map_err(|_| StoreError::Rejected)?,
            Revision::new(revision),
            clock_sample(),
        )?;
    }
    #[cfg(not(feature = "native-sim-proof"))]
    let _ = (store, proof_dir, point, grant, revision);
    Ok(())
}

impl DirectRunner<'_> {
    async fn receive(&mut self) -> Result<NativeEvent, StoreError> {
        if self.gate.load(Ordering::SeqCst) {
            return Err(StoreError::Rejected);
        }
        timeout(Duration::from_secs(30), self.harness.receive())
            .await
            .map_err(|_| StoreError::Unavailable)?
            .map_err(|_| StoreError::Unavailable)
    }

    fn record(&mut self, event: &NativeEvent) -> Result<(), StoreError> {
        let (method, params) = match &event.kind {
            NativeEventKind::Malformed | NativeEventKind::Response { .. } => (None, None),
            NativeEventKind::Notification { method }
            | NativeEventKind::ServerRequest { method, .. } => {
                (Some(method.as_str()), event.value.get("params"))
            }
        };
        let thread = params
            .and_then(|p| p.get("threadId"))
            .and_then(Value::as_str);
        let turn = params
            .and_then(|p| p.get("turnId").or_else(|| p.get("turn")?.get("id")))
            .and_then(Value::as_str);
        let item = params
            .and_then(|p| p.get("item"))
            .and_then(|i| i.get("id"))
            .and_then(Value::as_str);
        let status = match &event.kind {
            NativeEventKind::Malformed => "malformed",
            NativeEventKind::Response { .. } => "qualified",
            NativeEventKind::ServerRequest { .. } => "unsupported",
            NativeEventKind::Notification { method }
                if method == "item/started" || method == "item/completed" =>
            {
                let value = params.and_then(|p| p.get("item"));
                match value.and_then(|i| i.get("type")).and_then(Value::as_str) {
                    Some("userMessage" | "reasoning") => "qualified",
                    Some("agentMessage")
                        if value.is_some_and(|i| {
                            i["questions"].is_null() && i["delivery"].is_null()
                        }) =>
                    {
                        "qualified"
                    }
                    _ => "unsupported",
                }
            }
            NativeEventKind::Notification { method } if method == "turn/completed" => {
                if event
                    .terminal()
                    .ok()
                    .flatten()
                    .is_some_and(|t| t.status == "completed")
                {
                    "qualified"
                } else {
                    "unsupported"
                }
            }
            NativeEventKind::Notification { method }
                if method == "thread/readState/changed"
                    && lifecycle_codex::read_state_changed_matches(&event.value) =>
            {
                "qualified"
            }
            NativeEventKind::Notification { method }
                if matches!(
                    method.as_str(),
                    "configWarning"
                        | "remoteControl/status/changed"
                        | "warning"
                        | "thread/status/changed"
                        | "thread/tokenUsage/updated"
                        | "account/rateLimits/updated"
                        | "thread/started"
                        | "turn/started"
                        | "turn/failed"
                        | "turn/interrupted"
                ) =>
            {
                "qualified"
            }
            _ => "unsupported",
        };
        let applied = self.store.record_native_ingress(
            &self.session,
            &event.transport_session,
            event.sequence,
            &event.raw,
            status,
            method,
            thread,
            turn,
            item,
            clock_sample(),
        )?;
        if applied == lifecycle_store::NativeIngressApply::ConflictFenced || status != "qualified" {
            return Err(StoreError::ClaimConflict);
        }
        Ok(())
    }

    async fn handle(&mut self, event: NativeEvent) -> Result<(), StoreError> {
        self.record(&event)?;
        if let Some(terminal) = event.terminal().map_err(|_| StoreError::ClaimConflict)?
            && self.terminal.replace(terminal).is_some()
        {
            return Err(StoreError::ClaimConflict);
        }
        if let NativeEventKind::ServerRequest { id, .. } = event.kind {
            self.harness
                .deny_request(id)
                .await
                .map_err(|_| StoreError::Unavailable)?;
            return Err(StoreError::ClaimConflict);
        }
        Ok(())
    }

    async fn rpc(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<(Value, (String, u64)), StoreError> {
        let id = self
            .harness
            .send_request(method, params)
            .await
            .map_err(|_| StoreError::Unavailable)?;
        loop {
            let event = self.receive().await?;
            let response = matches!(event.kind,NativeEventKind::Response{id:seen} if seen==id);
            if matches!(event.kind, NativeEventKind::Response { .. }) && !response {
                self.record(&event)?;
                return Err(StoreError::ClaimConflict);
            }
            let result = if response {
                event.value.get("result").cloned()
            } else {
                None
            };
            let error = response && event.value.get("error").is_some();
            let coord = (event.transport_session.clone(), event.sequence);
            self.handle(event).await?;
            if error || (response && result.is_none()) {
                return Err(StoreError::Unavailable);
            }
            if let Some(result) = result {
                return Ok((result, coord));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn run_control_turn(
    store: &mut SqliteLifecycleStore,
    native: &NativeConfig,
    semantic: &SemanticNativeConfig,
    spec: ControlTurnSpec<'_>,
    principal: &str,
    uid: u32,
    proof_dir: Option<&Path>,
    gate: &AtomicBool,
) -> Result<ControlTurnResult, StoreError> {
    let role_name = match spec.role {
        SemanticControlRole::Compile => "compile",
        SemanticControlRole::Verify => "verify",
        SemanticControlRole::SuccessorVerify => "successor_verify",
    };
    let (control_grant_ref, control_grant_revision) = semantic.grant_for_phase(spec.phase);
    let mut isolated = native.clone();
    // Each control session owns fresh app-server state. Reusing the role path
    // after transition A would make transition B fail before native entry.
    isolated.root = native.root.join(format!(
        "{}-{}",
        role_name,
        lifecycle_semantic::sha256_hex(spec.session_id.as_bytes())
    ));
    fs::create_dir(&isolated.root).map_err(|_| StoreError::Unavailable)?;
    fs::set_permissions(&isolated.root, fs::Permissions::from_mode(0o700))
        .map_err(|_| StoreError::Unavailable)?;
    let launch = operations::prepare_native_launch(&isolated, uid)?;
    let mut session = store.claim_control_session(ControlSessionRequest {
        id: NativeSessionId::parse(spec.session_id.to_owned()).map_err(|_| StoreError::Rejected)?,
        subject: spec.subject.clone(),
        context: spec.context.clone(),
        transition: spec.transition.clone(),
        role: spec.role,
        incarnation: spec.incarnation,
        profile_digest: launch.profile_digest.clone(),
        binary_sha256: NATIVE_BINARY_SHA256.into(),
        protocol_digest: SCHEMA_THREAD_START_SHA256.into(),
        snapshot: spec.snapshot,
        manifest_sha256: spec.snapshot_manifest_sha256.into(),
        clock: clock_sample(),
    })?;
    if session.thread().is_some() {
        return Err(StoreError::ClaimConflict);
    }
    let transport = format!("transport:{}:{}", spec.session_id, spec.invocation_id);
    let harness = NativeHarness::spawn(
        &native.executable,
        &launch.work,
        &launch.home,
        &launch.codex_home,
        transport,
    )
    .map_err(|_| StoreError::Unavailable)?;
    let pid = harness.pid();
    let mut runner = DirectRunner {
        store,
        session: session.id().clone(),
        harness,
        terminal: None,
        gate,
    };
    let execution=async {
        let (init,_)=runner.rpc("initialize",json!({"clientInfo":{"name":"work_engine_s6","version":"0.1.0"},"capabilities":{"experimentalApi":true}})).await?;
        if !init.get("userAgent").and_then(Value::as_str).is_some_and(|s|lifecycle_codex::user_agent_matches(s,"work_engine_s6"))
            || init.get("codexHome").and_then(Value::as_str)!=launch.codex_home.to_str() {return Err(StoreError::Rejected);}
        runner.harness.send_notification("initialized",json!({})).await.map_err(|_|StoreError::Unavailable)?;
        let (effective,_)=runner.rpc("config/read",json!({"includeLayers":false})).await?;
        let c=&effective["config"];
        if c["model_provider"]!="local_probe" || c["model"]!="gpt-6-sol"
            || c["sandbox_mode"]!="read-only" || c["approval_policy"]!="never"
            || c["web_search"]!="disabled" || c["features"]["shell_tool"]!=false
            || c["features"]["multi_agent"]!=false || c["features"]["code_mode_host"]!=false
            || c["agents"]["enabled"]!=false || c["apps"]["_default"]["enabled"]!=false
        {return Err(StoreError::Rejected);}
        let (started,_)=runner.rpc("thread/start",json!({"cwd":launch.work,"model":"gpt-6-sol",
            "modelProvider":"local_probe","approvalPolicy":"never","sandbox":"read-only",
            "environments":[],"dynamicTools":[],"ephemeral":false,"historyMode":"legacy"})).await?;
        let thread=started["thread"]["id"].as_str().ok_or(StoreError::Unavailable)?.to_owned();
        if started["thread"]["ephemeral"]!=false {return Err(StoreError::Rejected);}
        runner.store.bind_native_thread(&mut session,ProviderThreadId::parse(thread.clone()).map_err(|_|StoreError::Rejected)?)?;
        let binding=SemanticInvocationBinding{subject:spec.subject,transition:spec.transition,
            capture_id:semantic.capture_id.clone(),phase:spec.phase,
            purpose:if spec.phase==SemanticPhase::Compile {NativeInvocationPurpose::SemanticCompile} else {NativeInvocationPurpose::SemanticVerify},
            invocation_id:spec.invocation_id.into(),attempt_id:spec.attempt_id.into(),
            session_id:spec.session_id.into(),thread_id:thread.clone(),policy_id:spec.policy_id.into(),
            exposure_id:spec.exposure_id.clone(),grant_ref:control_grant_ref.into(),
            grant_revision:control_grant_revision};
        operations::optional_native_barrier(proof_dir,&format!("semantic_{role_name}_before_entry"))?;
        optional_proof_revoke(runner.store,proof_dir,&format!("semantic_{role_name}_before_entry"),
            control_grant_ref,control_grant_revision)?;
        let mut turn=runner.store.claim_control_turn(&session,NativeTurnRequest{
            invocation_id:spec.invocation_id.into(),attempt:AttemptId::parse(spec.attempt_id.to_owned()).map_err(|_|StoreError::Rejected)?,
            grant:GrantId::parse(control_grant_ref).map_err(|_|StoreError::Rejected)?,
            grant_revision:control_grant_revision,purpose:binding.purpose,prompt:spec.prompt.as_bytes().to_vec(),
        },&binding,principal,clock_sample())?;
        runner.store.mark_native_send_intent(&turn)?;
        operations::optional_native_barrier(proof_dir,&format!("semantic_{role_name}_after_entry"))?;
        optional_proof_revoke(runner.store,proof_dir,&format!("semantic_{role_name}_after_entry"),
            control_grant_ref,control_grant_revision)?;
        let (response,_)=runner.rpc("turn/start",json!({"threadId":thread,"input":[{"type":"text","text":spec.prompt}],"environments":[]})).await?;
        let turn_id=response["turn"]["id"].as_str().ok_or(StoreError::Unavailable)?.to_owned();
        runner.store.bind_native_turn(&mut turn,ProviderTurnId::parse(turn_id.clone()).map_err(|_|StoreError::Rejected)?)?;
        let terminal=loop {
            if let Some(terminal)=runner.terminal.take() {
                if terminal.thread_id!=thread || terminal.turn_id!=turn_id {return Err(StoreError::ClaimConflict);}
                break terminal;
            }
            let event=runner.receive().await?;
            runner.handle(event).await?;
        };
        if terminal.status!="completed" {return Err(StoreError::ClaimConflict);}
        let (read,(history_transport,history_sequence))=runner.rpc("thread/read",json!({"threadId":thread,"includeTurns":true})).await?;
        let exact=read["thread"]["turns"].as_array().ok_or(StoreError::Unavailable)?
            .iter().find(|t|t["id"]==turn_id).ok_or(StoreError::Unavailable)?;
        if exact["status"]!="completed" {return Err(StoreError::ClaimConflict);}
        let final_text=exact["items"].as_array().ok_or(StoreError::Unavailable)?
            .iter().rev().find(|i|i["type"]=="agentMessage")
            .and_then(|i|i["text"].as_str()).ok_or(StoreError::Unavailable)?.to_owned();
        runner.store.settle_native_turn(&turn,&final_text,"observed_complete",&history_transport,history_sequence,clock_sample())?;
        operations::optional_native_barrier(proof_dir,&format!("semantic_{role_name}_after_result"))?;
        let final_material=runner.store.native_final_material(spec.invocation_id)?.ok_or(StoreError::Unavailable)?;
        Ok(ControlTurnResult{binding,final_material,observed_provider:"local_probe".into(),
            observed_model:"gpt-6-sol".into(),exposure_ref:format!("native:{}:{}",spec.session_id,thread)})
    }.await;
    let exit = runner.harness.terminate_and_reap().await;
    let outcome = if execution.is_ok() {
        "completed"
    } else {
        "transport_error"
    };
    match exit {
        Ok(exit) => runner.store.record_native_process_exit(
            session.id(),
            pid,
            exit.code,
            exit.forced,
            outcome,
            clock_sample(),
        )?,
        Err(_) => runner.store.record_native_process_exit(
            session.id(),
            pid,
            None,
            true,
            "cleanup_unobserved",
            clock_sample(),
        )?,
    }
    execution
}
