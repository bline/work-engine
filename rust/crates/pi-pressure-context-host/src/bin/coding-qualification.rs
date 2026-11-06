//! Explicit finite local qualification entry. No defaults admit runtime effects.
use lifecycle_core::*;
use lifecycle_store::*;
use pi_pressure_context_host::coding::{CodingConfig, CodingContinuity};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
use work_engine_manual_host::{
    digest,
    pi_docker::{
        Host, Job, Journal,
        coding::{BRIDGE_PROFILE, LIVE_BRIDGE_PROFILE, WorkspaceWriteBoundary},
        request_cancel,
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
    work_engine_manual_host::install_signals();
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|v| v == "pi-docker") {
        if args.len() != 5 {
            return Err("fixed pi-docker action and one path required".into());
        }
        let output = match (args[2].as_str(), args[3].as_str()) {
            ("run", "--request") => execute(&args[4])?,
            ("inspect", "--state") => {
                serde_json::to_value(Host::open(&PathBuf::from(&args[4]))?.journal())?
            }
            ("reconcile", "--state") => Host::open(&PathBuf::from(&args[4]))?.reconcile()?,
            ("cancel", "--state") => {
                // Cancellation remains deliverable while run owns the Host lock, but it
                // must still name this producing binary's original journal.
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
        json!({"status":if result["receipt"]["provider"] == "disabled" {"bounded_fixture_passed"} else {"bounded_live_continuity_completed"},"result":PathBuf::from(result["receipt"]["job"]["state_dir"].as_str().ok_or("state absent")?).join("coding-continuity-result.json"),"receipt_id":result["receipt_id"],"acceptance":"owner_pending","domain_effect_settlement":"not_claimed"})
    );
    Ok(())
}
fn execute(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let job: Job = serde_json::from_slice(&fs::read(path)?)?;
    let profile = job.coding.as_ref().ok_or("coding profile missing")?;
    // Two separately versioned profiles; neither can be selected by the other's
    // schema. Live additionally needs the Host's existing exact provider
    // admission (checked again inside Host::admit) and never reuses fixture labels.
    let live = profile.schema == LIVE_BRIDGE_PROFILE;
    if !((profile.schema == BRIDGE_PROFILE && profile.provider.is_none())
        || (live && profile.provider.is_some()))
        || profile.max_tool_calls != 1
        || profile.workspace_write_boundary != WorkspaceWriteBoundary::SrcLibRs
    {
        return Err("bounded source-file coding continuity profile required".into());
    }
    let own_hash = digest(&fs::read(std::env::current_exe()?)?);
    let authority: Value = serde_json::from_slice(&job.authority.verify()?)?;
    let bounds = if live {
        json!({"inference_containers":1,"tool_containers":1,"validator_containers":1,"sdk_sessions":2,"workspace_exec_calls":1,"fake_provider_requests":0,"physical_resets":1,"live_provider_requests":4})
    } else {
        json!({"inference_containers":1,"tool_containers":1,"validator_containers":1,"sdk_sessions":2,"workspace_exec_calls":1,"fake_provider_requests":4,"physical_resets":1,"live_provider_requests":0})
    };
    if live {
        // The Host's existing provider admission document (exact binary, job,
        // model, auth-source path, task, catalog, requests, budget). It is not
        // widened; bounds travel in the pinned packet below.
        if authority["schema"] != "work-engine.pi-coding-runtime-admission.v1"
            || authority["job_id"] != job.job_id
            || authority["binary_sha256"] != own_hash
        {
            return Err("exact live provider admission/binary missing".into());
        }
    } else if authority["schema"] != "work-engine.coding-continuity-runtime-admission.v1"
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
    if packet["schema"]
        != if live {
            "work-engine.coding-live-continuity-packet.v1"
        } else {
            "work-engine.coding-continuity-packet.v1"
        }
        || (live && packet["bounds"] != bounds)
    {
        return Err("coding packet schema/bounds".into());
    }
    let config = CodingConfig {
        task: if live {
            Some(String::from_utf8(
                profile
                    .provider
                    .as_ref()
                    .ok_or("provider plan absent")?
                    .task
                    .verify()?,
            )?)
        } else {
            None
        },
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
        &if live {
            pi_coding_live_profile_digest()
        } else {
            pi_coding_profile_digest()
        },
        PI_SDK_INDEX_SHA256,
        &if live {
            pi_coding_live_protocol_digest()
        } else {
            pi_coding_protocol_digest()
        },
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
    Ok(result)
}
