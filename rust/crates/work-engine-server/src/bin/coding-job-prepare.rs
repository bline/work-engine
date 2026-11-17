//! Local operator preparation only. No Host admission, process launch or inference.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use work_engine_manual_host::{
    Artifact, digest,
    pi_docker::{
        Job, Tree,
        coding::{
            self, WorkspaceWriteBoundary,
            hosted::HostedRoutes,
            provider::{LaneInputs, ProviderPlan},
        },
        inventory,
    },
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    schema: String,
    job_id: String,
    output_dir: PathBuf,
    state_dir: PathBuf,
    operation_root: PathBuf,
    host_binary: PathBuf,
    task: PathBuf,
    workspace: PathBuf,
    validator: PathBuf,
    runtime: PathBuf,
    toolchain: PathBuf,
    docker: PathBuf,
    image: String,
    deadline_unix_seconds: u64,
    command_timeout_seconds: u64,
    collector_deadline_seconds: u64,
    cancel_grace_seconds: u64,
    fixture_command: String,
    #[serde(default)]
    hosted: Option<HostedInputs>,
}
/// Live hosted inputs. Provider access is never an input: the Host's embedder
/// loads it from the credentials file, which names access sources only.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HostedInputs {
    pi_config: PathBuf,
    routes: HostedRoutes,
    /// The Host embedder's credentials file (owner-only), passed to the Host
    /// as `WORK_ENGINE_HOSTED_CREDENTIALS` through the job profile.
    credentials: PathBuf,
    runtime_admission: Artifact,
    budget_control: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authority {
    schema: String,
    spec_sha256: String,
    inputs_sha256: String,
    job_id: String,
    authorization_id: String,
    caller: String,
    credential_sha256: String,
    allow_prepare: bool,
    allow_provider: bool,
}
#[derive(Serialize)]
struct Inputs {
    task: Artifact,
    workspace: Value,
    validator: Artifact,
    runtime: Value,
    toolchain: Value,
    docker: Artifact,
    host_binary: Artifact,
    pi_config: Option<Value>,
}
fn read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let m = fs::symlink_metadata(path)?;
    if !path.is_absolute() || fs::canonicalize(path)? != path || !m.is_file() || m.len() > limit {
        return Err("regular canonical bounded input required".into());
    }
    Ok(fs::read(path)?)
}
fn pin(path: &Path) -> Result<Artifact> {
    Ok(Artifact {
        path: path.into(),
        sha256: digest(&read(path, 512 * 1024 * 1024)?),
    })
}
fn tree(path: &Path) -> Result<Value> {
    if !path.is_absolute() || fs::canonicalize(path)? != path {
        return Err("canonical absolute tree required".into());
    }
    Ok(serde_json::to_value(inventory(path)?)?)
}
fn normal(path: &Path) -> bool {
    path.is_absolute()
        && path.components().all(|c| {
            matches!(
                c,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
}
fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn inputs(s: &Spec) -> Result<Inputs> {
    if s.schema != "work-engine.coding-preparation.v1"
        || s.job_id.is_empty()
        || s.job_id.len() > 64
        || !s
            .job_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        || !normal(&s.output_dir)
        || !normal(&s.state_dir)
        || !normal(&s.operation_root)
        || s.fixture_command.len() > 8192
        || !(1..=30).contains(&s.command_timeout_seconds)
        || !(1..=900).contains(&s.collector_deadline_seconds)
        || !(1..=30).contains(&s.cancel_grace_seconds)
        || !s.image.contains("@sha256:")
        || (s.hosted.is_some() != s.fixture_command.is_empty())
    {
        return Err("preparation schema/mode/limits conflict".into());
    }
    if let Some(live) = &s.hosted {
        // Mirror the current server admission window and continuity owner's
        // observation contract. This checks preparation, never grants entry.
        let now_ms: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis()
            .try_into()?;
        let deadline_ms: i64 = s
            .deadline_unix_seconds
            .checked_mul(1000)
            .ok_or("live deadline overflow")?
            .try_into()?;
        let b = &live.budget_control;
        let observed = b["observed_ms"]
            .as_i64()
            .ok_or("live budget observation missing")?;
        let age = b["max_age_ms"]
            .as_i64()
            .filter(|n| *n > 0 && *n <= 300000)
            .ok_or("live budget age invalid")?;
        if s.deadline_unix_seconds <= (now_ms / 1000) as u64 + 5
            || s.deadline_unix_seconds > (now_ms / 1000) as u64 + 240
            || b["schema"] != "work-engine.coding-budget-control.v1"
            || b["authority"] != "observation_only"
            || b["classification"] != "live_provider_observation"
            || b["sequence"] != 1
            || b["source"].as_str().is_none_or(str::is_empty)
            || b["generation"].as_str().is_none_or(str::is_empty)
            || b["expires_ms"] != deadline_ms
            || b["available"] != true
            || b["max_provider_requests"] != 4
            || b["max_tool_calls"] != 1
            || observed > now_ms
            || now_ms.saturating_sub(observed) > age
            || now_ms >= deadline_ms
        {
            return Err("live deadline/canonical budget not current".into());
        }
        live.routes.validate()?;
        use work_engine_manual_host::pi_docker::coding::hosted::Endpoint;
        if live.routes.routes.iter().any(|r| {
            !matches!(
                Endpoint::parse(&r.endpoint),
                Some(Endpoint::ChatGptCodex | Endpoint::OpenAiResponses)
            )
        }) {
            return Err("live routes are the fixed HTTPS endpoints only".into());
        }
        let m = fs::symlink_metadata(&live.credentials)?;
        if !live.credentials.is_absolute()
            || fs::canonicalize(&live.credentials)? != live.credentials
            || !m.is_file()
            || m.permissions().mode() & 0o077 != 0
            || m.uid() != unsafe { libc::geteuid() }
            || live.credentials.to_string_lossy().contains(['\0', '\n'])
        {
            return Err("owner-only canonical credentials file required".into());
        }
    }
    read(&s.task, 8192)?;
    Ok(Inputs {
        task: pin(&s.task)?,
        workspace: tree(&s.workspace)?,
        validator: pin(&s.validator)?,
        runtime: tree(&s.runtime)?,
        toolchain: tree(&s.toolchain)?,
        docker: pin(&s.docker)?,
        host_binary: pin(&s.host_binary)?,
        pi_config: s.hosted.as_ref().map(|p| tree(&p.pi_config)).transpose()?,
    })
}
fn write(path: &Path, bytes: &[u8]) -> Result<Artifact> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(Artifact {
        path: path.into(),
        sha256: digest(bytes),
    })
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<Artifact> {
    write(path, &serde_json::to_vec(value)?)
}
fn prepared_tree(root: &Path, out: &Path, name: &str, value: &Value) -> Result<Tree> {
    Ok(Tree {
        root: root.into(),
        manifest: write_json(&out.join(name), value)?,
    })
}
fn run() -> Result<Value> {
    let args: Vec<_> = std::env::args().collect();
    let inspect = match (args.get(1).map(String::as_str), args.len()) {
        (Some("inspect"), 3) => true,
        (Some("prepare"), 4) => false,
        _ => return Err("coding-job-prepare inspect SPEC | prepare SPEC AUTHORITY".into()),
    };
    let raw = read(Path::new(&args[2]), 65536)?;
    let spec: Spec = serde_json::from_slice(&raw)?;
    let inputs = inputs(&spec)?;
    let spec_sha = digest(&raw);
    let input_sha = digest(&serde_json::to_vec(&inputs)?);
    if inspect {
        return Ok(
            json!({"schema":"work-engine.coding-preparation-inspection.v1","spec_sha256":spec_sha,"inputs_sha256":input_sha,"job_id":spec.job_id,"provider_execution":false,"authority":"observation_only"}),
        );
    }
    let authority_path = Path::new(&args[3]);
    let auth_raw = read(authority_path, 16384)?;
    let authority: Authority = serde_json::from_slice(&auth_raw)?;
    if authority.schema != "work-engine.coding-preparation-authority.v1"
        || authority.spec_sha256 != spec_sha
        || authority.inputs_sha256 != input_sha
        || authority.job_id != spec.job_id
        || !authority.allow_prepare
        || authority.allow_provider
        || !identifier(&authority.authorization_id)
        || !identifier(&authority.caller)
        || authority.credential_sha256.len() != 64
        || !authority
            .credential_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("operator preparation authority does not bind exact inputs; provider permission cannot be minted".into());
    }
    // Refuse reuse before writing any output. Fresh state is separately owned by this job.
    if spec.output_dir.exists() || spec.state_dir.exists() {
        return Err("fresh output and state directories required".into());
    }
    if spec.output_dir.starts_with(&spec.state_dir)
        || spec.state_dir.starts_with(&spec.output_dir)
        || [&spec.workspace, &spec.runtime, &spec.toolchain]
            .iter()
            .any(|p| {
                spec.output_dir.starts_with(p)
                    || spec.state_dir.starts_with(p)
                    || p.starts_with(&spec.output_dir)
                    || p.starts_with(&spec.state_dir)
            })
    {
        return Err("preparation outputs overlap immutable inputs or state".into());
    }
    if spec.operation_root.starts_with(&spec.state_dir)
        || spec.state_dir.starts_with(&spec.operation_root)
        || spec.operation_root.starts_with(&spec.output_dir)
        || spec.output_dir.starts_with(&spec.operation_root)
    {
        return Err("operation ownership overlaps state or bundle".into());
    }
    if spec.operation_root.exists() {
        let m = fs::symlink_metadata(&spec.operation_root)?;
        if !m.is_dir()
            || fs::canonicalize(&spec.operation_root)? != spec.operation_root
            || m.permissions().mode() & 0o077 != 0
            || m.uid() != unsafe { libc::geteuid() }
        {
            return Err("private canonical operation root required".into());
        }
    }
    // Validate the supplied runtime admission against the exact live inputs.
    if let Some(live) = &spec.hosted {
        let admission: Value = serde_json::from_slice(&live.runtime_admission.verify()?)?;
        let config_sha = digest(&serde_json::to_vec(inputs.pi_config.as_ref().unwrap())?);
        let catalog = pin(&spec
            .runtime
            .join("node_modules/@earendil-works/pi-ai/dist/providers/data/openai-codex.json"))?;
        if admission.as_object().map_or(0, |o| o.len()) != 11
            || admission["schema"] != "work-engine.pi-coding-runtime-admission.v1"
            || admission["job_id"] != spec.job_id
            || admission["binary_sha256"] != inputs.host_binary.sha256
            || admission["model"] != "gpt-6-luna"
            || admission["task_sha256"] != inputs.task.sha256
            || admission["catalog_sha256"] != catalog.sha256
            || admission["pi_config_manifest_sha256"] != config_sha
            || admission["max_requests"] != 4
            || admission["budget_microusd"]
                .as_u64()
                .is_none_or(|b| !(1..=700_000).contains(&b))
            || admission["allow_provider"] != true
            || admission["hosted_routes_sha256"] != live.routes.digest()
        {
            return Err("supplied live runtime admission mismatch".into());
        }
    }
    if !spec.operation_root.exists() {
        fs::create_dir(&spec.operation_root)?;
        fs::set_permissions(&spec.operation_root, fs::Permissions::from_mode(0o700))?;
    }
    fs::create_dir(&spec.output_dir)?;
    fs::set_permissions(&spec.output_dir, fs::Permissions::from_mode(0o700))?;
    fs::create_dir(&spec.state_dir)?;
    fs::set_permissions(&spec.state_dir, fs::Permissions::from_mode(0o700))?;
    let out = &spec.output_dir;
    let shim = write(&out.join("pi-coding.mjs"), coding::SHIM)?;
    let provider = if let Some(live) = &spec.hosted {
        let admission: Value = serde_json::from_slice(&live.runtime_admission.verify()?)?;
        Some(ProviderPlan {
            lane: LaneInputs {
                pi_config: prepared_tree(
                    &live.pi_config,
                    out,
                    "config-manifest.json",
                    inputs.pi_config.as_ref().unwrap(),
                )?,
            },
            task: inputs.task.clone(),
            catalog: pin(&spec
                .runtime
                .join("node_modules/@earendil-works/pi-ai/dist/providers/data/openai-codex.json"))?,
            stream_adapter: write(
                &out.join("pi-native-tools.mjs"),
                coding::provider::STREAM_ADAPTER,
            )?,
            hosted: live.routes.clone(),
            model: "gpt-6-luna".into(),
            max_requests: 4,
            budget_microusd: admission["budget_microusd"].as_u64().unwrap_or(0),
            max_context_bytes: 24_000,
        })
    } else {
        None
    };
    if let Some(plan) = &provider
        && plan.reservation_microusd()? > plan.budget_microusd
    {
        return Err("conservative reservation exceeds the admitted budget".into());
    }
    let bounds = json!({"inference_containers":1,"tool_containers":1,"validator_containers":1,"sdk_sessions":2,"workspace_exec_calls":1,"fake_provider_requests":0,"physical_resets":1,"live_provider_requests":4});
    let packet = if let Some(live) = &spec.hosted {
        json!({"schema":"work-engine.coding-live-continuity-packet.v1","bounds":bounds,"budget":live.budget_control,"pressure_basis_points":9500})
    } else {
        json!({"schema":"work-engine.coding-prepared-task.v1","task_sha256":inputs.task.sha256,"task":String::from_utf8(read(&spec.task,8192)?)?,"classification":"provider_disabled_fixture"})
    };
    let job = Job {
        schema: "work-engine.pi-docker-custody.v1".into(),
        job_id: spec.job_id.clone(),
        state_dir: spec.state_dir.clone(),
        authority: spec
            .hosted
            .as_ref()
            .map(|p| p.runtime_admission.clone())
            .unwrap_or(Artifact {
                path: authority_path.into(),
                sha256: digest(&auth_raw),
            }),
        packet: write_json(&out.join("packet.json"), &packet)?,
        docker: inputs.docker,
        image: spec.image,
        runtime: prepared_tree(&spec.runtime, out, "runtime-manifest.json", &inputs.runtime)?,
        workspace: prepared_tree(
            &spec.workspace,
            out,
            "workspace-manifest.json",
            &inputs.workspace,
        )?,
        deadline_unix_seconds: spec.deadline_unix_seconds,
        command_timeout_seconds: spec.command_timeout_seconds,
        idle_seconds: 0,
        receiver: None,
        coding: Some(coding::Profile {
            schema: if provider.is_some() {
                coding::HOSTED_BRIDGE_PROFILE
            } else {
                coding::PROFILE
            }
            .into(),
            shim,
            toolchain: prepared_tree(
                &spec.toolchain,
                out,
                "toolchain-manifest.json",
                &inputs.toolchain,
            )?,
            acceptance: inputs.validator,
            fixture_command: spec.fixture_command,
            max_tool_calls: 1,
            workspace_write_boundary: WorkspaceWriteBoundary::SrcLibRs,
            provider,
        }),
    };
    let input_digest = digest(&serde_json::to_vec(&job)?);
    let job_artifact = write_json(&out.join("job.json"), &job)?;
    let live = spec.hosted.is_some();
    let env = match &spec.hosted {
        Some(live) => json!({"WORK_ENGINE_HOSTED_CREDENTIALS": live.credentials}),
        None => json!({}),
    };
    let auth = json!({"authorization_id":authority.authorization_id,"caller":authority.caller,"credential_sha256":authority.credential_sha256,"job_file":job_artifact.path,"job_sha256":job_artifact.sha256,"packet_sha256":job.packet.sha256,"job_id":job.job_id,"state_dir":job.state_dir,"host_input_digest":input_digest});
    let profile = json!({"version":if live{2}else{1},"kind":"coding-job-profile","capability":if live{"coding-job-local-live-v1"}else{"coding-job-offline-fixture-v1"},"class":if live{"local-live"}else{"offline-fixture"},"host_binary":inputs.host_binary.path,"host_binary_sha256":inputs.host_binary.sha256,"operation_root":spec.operation_root,"env":env,"collector_deadline_seconds":spec.collector_deadline_seconds,"cancel_grace_seconds":spec.cancel_grace_seconds,"authorizations":[auth]});
    let profile = write_json(&out.join("job-profile.json"), &profile)?;
    Ok(
        json!({"schema":"work-engine.coding-preparation-result.v1","job":job_artifact,"job_profile":profile,"host_input_digest":input_digest,"packet_sha256":job.packet.sha256,"provider_execution":false,"runtime_admission":"separate_existing_authority_required","acceptance":"owner_pending"}),
    )
}
fn main() {
    match run() {
        Ok(v) => println!("{v}"),
        Err(e) => {
            eprintln!("preparation refused: {e}");
            std::process::exit(2)
        }
    }
}
