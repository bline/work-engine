//! Explicit finite local qualification entry. No defaults admit runtime effects.
use lifecycle_core::*;
use lifecycle_store::*;
use pi_pressure_context_host::coding::{CodingConfig, CodingContinuity};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
use work_engine_manual_host::{
    digest,
    pi_docker::{
        Host, Job,
        coding::{BRIDGE_PROFILE, WorkspaceWriteBoundary},
    },
};
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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 || args[1] != "--execute" {
        return Err("explicit --execute <pinned-job.json> required; no runtime entered".into());
    }
    let job: Job = serde_json::from_slice(&fs::read(&args[2])?)?;
    let profile = job.coding.as_ref().ok_or("coding profile missing")?;
    if profile.schema != BRIDGE_PROFILE
        || profile.provider.is_some()
        || profile.max_tool_calls != 1
        || profile.workspace_write_boundary != WorkspaceWriteBoundary::SrcLibRs
    {
        return Err("bounded provider-disabled source-file profile required".into());
    }
    let own_hash = digest(&fs::read(std::env::current_exe()?)?);
    let authority: Value = serde_json::from_slice(&job.authority.verify()?)?;
    let bounds = json!({"inference_containers":1,"tool_containers":1,"validator_containers":1,"sdk_sessions":2,"workspace_exec_calls":1,"fake_provider_requests":4,"physical_resets":1,"live_provider_requests":0});
    if authority["schema"] != "work-engine.coding-continuity-runtime-admission.v1"
        || authority["allow_fixture_runtime"] != true
        || authority["job_id"] != job.job_id
        || authority["binary_sha256"] != own_hash
        || authority["deadline_unix_seconds"] != job.deadline_unix_seconds
        || authority["bounds"] != bounds
    {
        return Err("exact concrete runtime authority/binary/bounds missing".into());
    }
    let sdk = job
        .runtime
        .root
        .join("node_modules/@earendil-works/pi-coding-agent/dist/index.js");
    if digest(&fs::read(sdk)?) != PI_SDK_INDEX_SHA256 {
        return Err("Pi SDK pin conflict".into());
    }
    let packet: Value = serde_json::from_slice(&job.packet.verify()?)?;
    if packet["schema"] != "work-engine.coding-continuity-packet.v1" {
        return Err("coding packet schema".into());
    }
    let config = CodingConfig {
        budget: packet["budget"].clone(),
        expires_ms: job.deadline_unix_seconds.saturating_mul(1000).try_into()?,
        pressure_basis_points: packet["pressure_basis_points"]
            .as_u64()
            .ok_or("pressure fixture required")?,
    };
    let state_dir = job.state_dir.clone();
    let initial_manifest = job.workspace.manifest.verify()?;
    let mut host = Host::admit(job)?;
    let mut store =
        SqliteLifecycleStore::open(&state_dir.join("lifecycle"), "fixture-root".into())?;
    let subject = SubjectId::parse("subject:pi:coding")?;
    store.register_subject(
        subject.clone(),
        ContextGeneration::parse("context:old")?,
        BuildId::parse("build:pi:coding")?,
        ProofRunId::parse("proof:pi:coding")?,
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
        ("grant:successor", "context:new", GrantScope::EnqueueInput),
    ] {
        store.install_trusted_grant(TrustedGrant {
            id: GrantId::parse(id)?,
            issuer: "fixture-root".into(),
            principal: "operator:fixture".into(),
            subject: subject.clone(),
            context: ContextGeneration::parse(context)?,
            build: BuildId::parse("build:pi:coding")?,
            proof_run: ProofRunId::parse("proof:pi:coding")?,
            scope,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(config.expires_ms)),
            revision: Revision::new(1),
            revoked: false,
        })?;
    }
    let initial = store.stage_artifact(subject.clone(), &initial_manifest)?;
    let published = store.publish_artifact(initial, clock())?;
    let session = store.reserve_native_session(
        NativeSessionId::parse("session:old")?,
        subject,
        ContextGeneration::parse("context:old")?,
        RuntimeIncarnation::parse("incarnation:pi:coding")?,
        &pi_coding_profile_digest(),
        PI_SDK_INDEX_SHA256,
        &pi_coding_protocol_digest(),
        SnapshotId::parse("snapshot:coding")?,
        &published.digest_hex,
        false,
        clock(),
    )?;
    let continuity = CodingContinuity::new(store, session, config, clock)?;
    host.bind_coding_effect_gate(Box::new(continuity.clone()))?;
    let result = host.run()?;
    fs::write(
        state_dir.join("coding-host-result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    let report = continuity.report()?;
    fs::write(
        state_dir.join("coding-continuity-result.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    if result["receipt"]["status"] != "component_passed"
        || report["continued_parent"] != "CompletedSettled"
    {
        return Err(
            "coding qualification unestablished; retain Host/lifecycle uncertainty and custody"
                .into(),
        );
    }
    let id = result["receipt_id"]
        .as_str()
        .ok_or("receipt identity absent")?;
    host.acknowledge(id)?;
    println!(
        "{}",
        json!({"status":"bounded_fixture_passed","result":PathBuf::from(&state_dir).join("coding-continuity-result.json"),"receipt_id":id,"acceptance":"owner_pending","domain_effect_settlement":"not_claimed"})
    );
    Ok(())
}
