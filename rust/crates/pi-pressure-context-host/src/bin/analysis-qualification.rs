//! Explicit finite entry for one hosted analysis job
//! (`work-engine.pi-analysis-hosted.v1`). No defaults admit runtime effects:
//! the Host's exact provider admission must name this binary and job.
use lifecycle_core::*;
use lifecycle_store::*;
use pi_pressure_context_host::{
    analysis::{AnalysisConfig, AnalysisSession},
    coding::HostedProviderPolicy,
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
use work_engine_manual_host::{
    digest,
    pi_docker::{
        Host, Job, Journal,
        coding::{ANALYSIS_PROFILE, hosted::LoopbackTransport},
        request_cancel,
    },
};
const PACKET_SCHEMA: &str = "work-engine.pi-analysis-packet.v1";
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
    work_engine_manual_host::install_signals();
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|v| v == "pi-docker") {
        if args.len() != 5 {
            return Err("fixed pi-docker action and one path required".into());
        }
        let output = match (args[2].as_str(), args[3].as_str()) {
            ("inspect", "--state") => {
                serde_json::to_value(Host::open(&PathBuf::from(&args[4]))?.journal())?
            }
            ("reconcile", "--state") => Host::open(&PathBuf::from(&args[4]))?.reconcile()?,
            ("cancel", "--state") => {
                let state = PathBuf::from(&args[4]);
                let journal: Journal =
                    serde_json::from_slice(&fs::read(state.join("pi-state.json"))?)?;
                if journal.binary_sha256 != digest(&fs::read(std::env::current_exe()?)?) {
                    return Err("producer journal binary conflict".into());
                }
                request_cancel(&state)?
            }
            _ => return Err("unsupported fixed producer action".into()),
        };
        println!("{}", output);
        return Ok(());
    }
    if args.len() != 3 || args[1] != "--execute" {
        return Err("explicit --execute <pinned-job.json> required; no runtime entered".into());
    }
    let result = execute(&args[2])?;
    println!(
        "{}",
        json!({"status":"bounded_live_analysis_completed","result":PathBuf::from(result["receipt"]["job"]["state_dir"].as_str().ok_or("state absent")?).join("analysis-result.json"),"receipt_id":result["receipt_id"],"acceptance":"owner_pending","domain_effect_settlement":"not_claimed"})
    );
    Ok(())
}
fn execute(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let job: Job = serde_json::from_slice(&fs::read(path)?)?;
    let profile = job.coding.as_ref().ok_or("analysis profile missing")?;
    let plan = profile
        .provider
        .as_ref()
        .ok_or("analysis provider plan missing")?;
    if profile.schema != ANALYSIS_PROFILE {
        return Err("hosted analysis profile required".into());
    }
    let own_hash = digest(&fs::read(std::env::current_exe()?)?);
    let authority: Value = serde_json::from_slice(&job.authority.verify()?)?;
    // The Host's exact provider admission (binary, job, model, config, task,
    // catalog, requests, budget, route table), checked again in Host::admit.
    if authority["schema"] != "work-engine.pi-coding-runtime-admission.v1"
        || authority["job_id"] != job.job_id
        || authority["binary_sha256"] != own_hash
    {
        return Err("exact analysis provider admission/binary missing".into());
    }
    let sdk = job
        .runtime
        .root
        .join("node_modules/@earendil-works/pi-coding-agent/dist/index.js");
    if digest(&fs::read(sdk)?) != PI_SDK_INDEX_SHA256 {
        return Err("Pi SDK pin conflict".into());
    }
    let packet: Value = serde_json::from_slice(&job.packet.verify()?)?;
    if packet["schema"] != PACKET_SCHEMA
        || packet["max_provider_requests"] != plan.max_requests
        || packet["max_snapshot_reads"] != profile.max_tool_calls
    {
        return Err("analysis packet schema/bounds".into());
    }
    let config = AnalysisConfig {
        task: String::from_utf8(plan.task.verify()?)?,
        expires_ms: job.deadline_unix_seconds.saturating_mul(1000).try_into()?,
        max_requests: plan.max_requests.into(),
        hosted: HostedProviderPolicy {
            routes: plan.hosted.pairs(),
            max_attempts_per_request: plan.hosted.max_attempts_per_request,
        },
    };
    let state_dir = job.state_dir.clone();
    let snapshot_manifest = job.workspace.manifest.verify()?;
    let mut host = Host::admit(job)?;
    let mut store =
        SqliteLifecycleStore::open(&state_dir.join("lifecycle"), "analysis-root".into())?;
    let subject = SubjectId::parse("subject:pi:analysis")?;
    store.register_subject(
        subject.clone(),
        ContextGeneration::parse("context:analysis")?,
        BuildId::parse("build:pi:analysis")?,
        ProofRunId::parse("proof:pi:analysis")?,
        clock(),
    )?;
    store.install_trusted_grant(TrustedGrant {
        id: GrantId::parse(config.grant_id())?,
        issuer: "analysis-root".into(),
        principal: "operator:analysis".into(),
        subject: subject.clone(),
        context: ContextGeneration::parse("context:analysis")?,
        build: BuildId::parse("build:pi:analysis")?,
        proof_run: ProofRunId::parse("proof:pi:analysis")?,
        scope: GrantScope::NativeTurn,
        expires: AuthorityExpiresAt::new(WallTimeMs::new(config.expires_ms)),
        revision: Revision::new(1),
        revoked: false,
    })?;
    let staged = store.stage_artifact(subject.clone(), &snapshot_manifest)?;
    let published = store.publish_artifact(staged, clock())?;
    let session = store.reserve_native_session(
        NativeSessionId::parse("session:analysis")?,
        subject,
        ContextGeneration::parse("context:analysis")?,
        RuntimeIncarnation::parse("incarnation:pi:analysis")?,
        &pi_analysis_profile_digest(),
        PI_SDK_INDEX_SHA256,
        &pi_analysis_protocol_digest(),
        SnapshotId::parse("snapshot:analysis")?,
        &published.digest_hex,
        false,
        clock(),
    )?;
    let analysis = AnalysisSession::new(store, session, config, clock)?;
    host.bind_coding_effect_gate(Box::new(analysis.clone()))?;
    // Live routes: the embedder's credentials file (never part of the job).
    // Without it, offline qualification over synthetic loopback routes.
    match std::env::var_os("WORK_ENGINE_HOSTED_CREDENTIALS") {
        Some(path) => host.bind_hosted_transport(Box::new(
            work_engine_manual_host::pi_docker::coding::https::HttpsTransport::from_file(
                std::path::Path::new(&path),
            )?,
        ))?,
        None => host.bind_hosted_transport(Box::new(LoopbackTransport::new()?))?,
    }
    let result = host.run();
    fs::write(
        state_dir.join("coding-hosted-gate-attempts.json"),
        serde_json::to_vec_pretty(&analysis.hosted_attempts())?,
    )?;
    let result = result?;
    fs::write(
        state_dir.join("analysis-host-result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    let report = analysis.report()?;
    fs::write(
        state_dir.join("analysis-result.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    if result["receipt"]["status"] != "component_passed"
        || report["settlement"] != "CompletedSettled"
    {
        return Err("analysis unestablished; retain Host/lifecycle uncertainty and custody".into());
    }
    let id = result["receipt_id"]
        .as_str()
        .ok_or("receipt identity absent")?;
    host.acknowledge(id)?;
    Ok(result)
}
