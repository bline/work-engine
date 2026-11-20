//! Production composition of a Pi Host relay job (dev-vertical slice 4).
//!
//! `work-engine-pi-host prepare` turns an installed, digest-pinned
//! composition profile (`work-engine.pi-host-composition.v1`) into one
//! launch's Host job under a fresh root: the inputs, the runtime admission
//! derived from the profile for this job id, `job.json`, `serve-config.json`,
//! `pi-host.json` (read by `lifecycle-service`'s Pi Host runner) and
//! `composition.json` (the profile digest and what was derived). The job is
//! checked by the Host's own fresh-admission rules before anything is
//! reported. Nothing here holds or names a credential: the Host's transport
//! reads the credentials file named by its environment at serve time.
use crate::pi_docker::coding::{
    LIFECYCLE_PROFILE, Profile, WorkspaceWriteBoundary,
    hosted::HostedRoutes,
    provider::{LaneInputs, ProviderPlan},
};
use crate::pi_docker::{IMAGE, Job, Tree, inventory};
use crate::pi_host_serve::{SERVE_SCHEMA, ServeConfig};
use crate::{Artifact, Error, Result, digest, file_digest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub const COMPOSITION_SCHEMA: &str = "work-engine.pi-host-composition.v1";
pub const RECORD_SCHEMA: &str = "work-engine.pi-host-composition-record.v1";
/// The relay thread's task text; prompts arrive from the controller.
const TASK: &str = "Pi Host lifecycle relay thread: prompts arrive from the controller.";

/// The operator's pinned composition profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionProfile {
    pub schema: String,
    /// The pinned runtime tree (root and its manifest).
    pub runtime: Tree,
    /// The Docker executable the Host drives.
    pub docker: PathBuf,
    pub provider: String,
    pub model: String,
    pub hosted: HostedRoutes,
    pub max_requests: u32,
    pub budget_microusd: u64,
    pub max_context_bytes: u64,
    pub max_tool_calls: u32,
    pub turn_deadline_secs: u64,
    /// Job lifetime from preparation; within the relay deadline bound.
    pub job_secs: u64,
    /// Public Pi settings (`settings.json`), when any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pi_settings: Option<Value>,
}

/// A refusal with its named class: `profile` (the profile is invalid by the
/// Host's rules), `snapshot-id`, `root` (not absolute, or not fresh),
/// `admission` (the composed job fails the Host's fresh admission) or `io`.
#[derive(Debug)]
pub struct Refusal {
    pub class: &'static str,
    pub message: String,
}
impl Refusal {
    pub fn to_json(&self) -> Value {
        json!({"status":"refused","class":self.class,"error":self.message})
    }
}
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.class, self.message)
    }
}
fn class<E: std::fmt::Display>(class: &'static str) -> impl Fn(E) -> Refusal {
    move |e| Refusal {
        class,
        message: e.to_string(),
    }
}
fn refuse<T>(class: &'static str, message: &str) -> std::result::Result<T, Refusal> {
    Err(Refusal {
        class,
        message: message.into(),
    })
}

fn write_artifact(path: &Path, bytes: &[u8]) -> Result<Artifact> {
    fs::write(path, bytes)?;
    Ok(Artifact {
        path: path.to_owned(),
        sha256: digest(bytes),
    })
}

/// The snapshot id grammar the lifecycle store admits.
fn snapshot_id_ok(id: &str) -> bool {
    (1..=128).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}

/// Reads and checks a profile by the Host's own rules (schema, durations,
/// provider catalog, route table, endpoint wire family and credential
/// kinds). Returns the profile, its digest and its description.
fn load_profile(
    profile_path: &Path,
) -> std::result::Result<(CompositionProfile, String, Value), Refusal> {
    let bytes = fs::read(profile_path).map_err(class("profile"))?;
    let profile: CompositionProfile = serde_json::from_slice(&bytes).map_err(class("profile"))?;
    if profile.schema != COMPOSITION_SCHEMA {
        return refuse("profile", "composition profile schema");
    }
    if !(1..=crate::pi_docker::coding::RELAY_DEADLINE_SECONDS).contains(&profile.job_secs)
        || !(1..=profile.job_secs).contains(&profile.turn_deadline_secs)
    {
        return refuse("profile", "composition job or turn duration");
    }
    if crate::pi_docker::coding::provider::catalog_path(&profile.provider).is_none() {
        return refuse("profile", "composition provider has no catalog");
    }
    profile.hosted.validate().map_err(class("profile"))?;
    let wire = match profile.provider.as_str() {
        "openai-codex" => crate::pi_stream::Wire::Responses,
        "claude-sdk" => crate::pi_stream::Wire::Messages,
        _ => crate::pi_stream::Wire::Completions,
    };
    let mut payers: std::collections::BTreeMap<String, Vec<&str>> = Default::default();
    let mut routes = vec![];
    for r in &profile.hosted.routes {
        let serves = crate::pi_docker::coding::hosted::Endpoint::parse(&r.endpoint)
            .is_some_and(|e| e.serves(wire));
        let kinds = crate::pi_docker::coding::https::allowed_kinds(&r.endpoint);
        let (true, Some(kinds)) = (serves, kinds) else {
            return refuse(
                "profile",
                "route endpoint does not serve the provider's wire family",
            );
        };
        // One payer holds one credential: it must serve every route it pays.
        let entry = payers
            .entry(r.payer.clone())
            .or_insert_with(|| kinds.to_vec());
        entry.retain(|k| kinds.contains(k));
        if entry.is_empty() && !kinds.is_empty() {
            return refuse(
                "profile",
                "a payer's routes need incompatible credential kinds",
            );
        }
        routes.push(json!({"id":r.id,"endpoint":r.endpoint,"payer":r.payer}));
    }
    let sha256 = digest(&bytes);
    let description = json!({"schema":COMPOSITION_SCHEMA,"profile_sha256":sha256,
        "provider":profile.provider,"model":profile.model,"wire":format!("{wire:?}").to_lowercase(),
        "routes":routes,"payers":payers});
    Ok((profile, sha256, description))
}

/// `profile describe`: the profile checked by the Host's rules and described
/// (payers with the credential kinds that can serve all their routes). It
/// never opens a credentials file.
pub fn describe(profile_path: &Path) -> std::result::Result<Value, Refusal> {
    load_profile(profile_path).map(|(_, _, description)| description)
}

/// Prepares one launch under `root`, which must be absent or empty.
/// `now_unix` is the preparation time; the job deadline is
/// `now_unix + job_secs`. Returns the composition record plus the digests of
/// `pi-host.json` and `composition.json`.
pub fn prepare(
    profile_path: &Path,
    root: &Path,
    snapshot_id: &str,
    now_unix: u64,
) -> std::result::Result<Value, Refusal> {
    let (profile, profile_sha256, _) = load_profile(profile_path)?;
    if !snapshot_id_ok(snapshot_id) {
        return refuse("snapshot-id", "snapshot id outside the admitted grammar");
    }
    if !root.is_absolute() {
        return refuse("root", "composition root must be absolute");
    }
    if root.exists() {
        if fs::read_dir(root).map_err(class("root"))?.next().is_some() {
            return refuse("root", "composition root must be fresh");
        }
    } else {
        fs::create_dir_all(root).map_err(class("root"))?;
    }
    let root = fs::canonicalize(root).map_err(class("root"))?;
    let job = compose(&profile, &root, snapshot_id, now_unix).map_err(class("io"))?;
    // The Host's own fresh-admission rules, before anything is reported.
    crate::pi_docker::validate_fresh(&job).map_err(class("admission"))?;
    finish(
        &profile,
        &profile_sha256,
        profile_path,
        &root,
        &job,
        snapshot_id,
        now_unix,
    )
    .map_err(class("io"))
}

/// Writes the inputs and the job (not yet `job.json`).
fn compose(
    profile: &CompositionProfile,
    root: &Path,
    snapshot_id: &str,
    now_unix: u64,
) -> Result<Job> {
    fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
    let inputs = root.join("inputs");
    let state = root.join("pi-host-state");
    let workspace = inputs.join("workspace");
    let pi_config = inputs.join("pi-config");
    for d in [&inputs, &state, &workspace, &pi_config] {
        fs::create_dir(d)?;
    }
    // The workspace binds the identity of the snapshot the controller
    // serves; the relay profile never mounts or serves it.
    fs::write(
        workspace.join("snapshot-identity.json"),
        serde_json::to_vec(&json!({"snapshot_id":snapshot_id}))?,
    )?;
    if let Some(settings) = &profile.pi_settings {
        if !settings.is_object() {
            return Err(Error::Refused(
                "public Pi settings must be an object".into(),
            ));
        }
        fs::write(
            pi_config.join("settings.json"),
            serde_json::to_vec(settings)?,
        )?;
    }
    let tree = |dir: &Path, manifest: &str| -> Result<Tree> {
        let bytes = serde_json::to_vec(&inventory(dir)?)?;
        Ok(Tree {
            root: dir.to_owned(),
            manifest: write_artifact(&inputs.join(manifest), &bytes)?,
        })
    };
    let workspace_tree = tree(&workspace, "workspace-manifest.json")?;
    let pi_config_tree = tree(&pi_config, "pi-config-manifest.json")?;
    let task = write_artifact(&inputs.join("task.txt"), TASK.as_bytes())?;
    let shim = write_artifact(
        &inputs.join("pi-coding.mjs"),
        crate::pi_docker::coding::SHIM,
    )?;
    let adapter = write_artifact(
        &inputs.join("pi-native-tools.mjs"),
        crate::pi_docker::coding::provider::STREAM_ADAPTER,
    )?;
    let catalog_path = profile.runtime.root.join(
        crate::pi_docker::coding::provider::catalog_path(&profile.provider).unwrap_or_default(),
    );
    let catalog = Artifact {
        sha256: file_digest(&catalog_path, None)?,
        path: catalog_path,
    };
    let docker = Artifact {
        sha256: file_digest(&profile.docker, None)?,
        path: profile.docker.clone(),
    };
    let host_sha256 = file_digest(&std::env::current_exe()?, None)?;
    let job_id = format!(
        "pi-host-{}",
        &digest(format!("{}:{now_unix}", root.display()).as_bytes())[..24]
    );
    let authority = write_artifact(
        &inputs.join("authority.json"),
        &serde_json::to_vec(&json!({
            "schema":"work-engine.pi-coding-runtime-admission.v1","job_id":job_id,
            "binary_sha256":host_sha256,"model":profile.model,"provider":profile.provider,
            "pi_config_manifest_sha256":pi_config_tree.manifest.sha256,"task_sha256":task.sha256,
            "catalog_sha256":catalog.sha256,"max_requests":profile.max_requests,
            "budget_microusd":profile.budget_microusd,"allow_provider":true,
            "hosted_routes_sha256":profile.hosted.digest()}))?,
    )?;
    let packet = write_artifact(
        &inputs.join("packet.json"),
        &serde_json::to_vec(&json!({"schema":"work-engine.pi-host-serve-packet.v1"}))?,
    )?;
    Ok(Job {
        schema: crate::pi_docker::SCHEMA.into(),
        job_id,
        state_dir: state,
        authority,
        packet,
        docker,
        image: IMAGE.into(),
        runtime: profile.runtime.clone(),
        workspace: workspace_tree,
        deadline_unix_seconds: now_unix + profile.job_secs,
        command_timeout_seconds: 30,
        idle_seconds: 0,
        receiver: None,
        coding: Some(Profile {
            schema: LIFECYCLE_PROFILE.into(),
            shim,
            toolchain: None,
            acceptance: None,
            fixture_command: String::new(),
            max_tool_calls: profile.max_tool_calls,
            workspace_write_boundary: WorkspaceWriteBoundary::default(),
            provider: Some(ProviderPlan {
                lane: LaneInputs {
                    pi_config: pi_config_tree,
                },
                task,
                catalog,
                stream_adapter: adapter,
                hosted: profile.hosted.clone(),
                provider: profile.provider.clone(),
                model: profile.model.clone(),
                max_requests: profile.max_requests,
                budget_microusd: profile.budget_microusd,
                max_context_bytes: profile.max_context_bytes,
                failover: None,
                classification_table_sha256: None,
                allow_shell: false,
                allow_background: false,
            }),
        }),
    })
}

/// Writes `job.json`, `serve-config.json`, `pi-host.json` and
/// `composition.json`; returns the record with the written digests.
fn finish(
    profile: &CompositionProfile,
    profile_sha256: &str,
    profile_path: &Path,
    root: &Path,
    job: &Job,
    snapshot_id: &str,
    now_unix: u64,
) -> Result<Value> {
    let host_sha256 = &job_authority_binary(job)?;
    let job_artifact = write_artifact(&root.join("job.json"), &serde_json::to_vec_pretty(job)?)?;
    let serve_path = root.join("serve-config.json");
    let serve = write_artifact(
        &serve_path,
        &serde_json::to_vec(&ServeConfig {
            schema: SERVE_SCHEMA.into(),
            job: job_artifact.clone(),
            turn_deadline_secs: profile.turn_deadline_secs,
        })?,
    )?;
    ServeConfig::load(&serve_path)?;
    let pi_host = write_artifact(
        &root.join("pi-host.json"),
        &serde_json::to_vec(&json!({
            "host_sha256":host_sha256,"serve_config":serve.path,"serve_config_sha256":serve.sha256,
            "job_sha256":job_artifact.sha256,"state_dir":job.state_dir,"provider":profile.provider,
            "model":profile.model,"turn_deadline_secs":profile.turn_deadline_secs}))?,
    )?;
    let mut record = json!({"schema":RECORD_SCHEMA,
        "profile":{"path":profile_path,"sha256":profile_sha256},
        "root":root,"job_id":job.job_id,"job_sha256":job_artifact.sha256,
        "serve_config_sha256":serve.sha256,"pi_host_json_sha256":pi_host.sha256,
        "host_sha256":host_sha256,"snapshot_id":snapshot_id,"prepared_unix_seconds":now_unix,
        "deadline_unix_seconds":job.deadline_unix_seconds,"routes":profile.hosted.routes});
    let composition = write_artifact(
        &root.join("composition.json"),
        &serde_json::to_vec_pretty(&record)?,
    )?;
    record["composition_sha256"] = json!(composition.sha256);
    Ok(record)
}

/// The Host binary digest the job's admission binds.
fn job_authority_binary(job: &Job) -> Result<String> {
    let admission: Value = serde_json::from_slice(&job.authority.verify()?)?;
    admission["binary_sha256"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::Refused("admission binary".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pi_docker::coding::hosted::HostedRoute;

    /// A pinned Pi 1.5.0 runtime with the Codex catalog, a fake Docker file
    /// and a profile over one loopback route.
    fn fixture() -> (tempfile::TempDir, PathBuf, CompositionProfile) {
        let dir = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(dir.path()).unwrap();
        let runtime = base.join("runtime");
        let pi = runtime.join("node_modules/@earendil-works/pi-coding-agent");
        fs::create_dir_all(pi.join("dist/bundle")).unwrap();
        fs::write(pi.join("dist/bundle/cli.js"), "fixture only").unwrap();
        fs::write(
            pi.join("package.json"),
            r#"{"name":"@earendil-works/pi-coding-agent","version":"1.5.0"}"#,
        )
        .unwrap();
        let catalog =
            runtime.join(crate::pi_docker::coding::provider::catalog_path("openai-codex").unwrap());
        fs::create_dir_all(catalog.parent().unwrap()).unwrap();
        fs::write(
            &catalog,
            serde_json::to_vec(&json!({"openai-codex-responses":{"chat:gpt-6-luna":{
            "id":"gpt-6-luna","provider":"openai-codex","api":"openai-codex-responses",
            "baseUrl":"https://chatgpt.com/backend-api","contextWindow":272000,"maxTokens":128000,
            "cost":{"input":0.1,"output":0.5,"cacheRead":0.01,"cacheWrite":0.125,
                "tiers":[{"input":0.2,"output":0.75,"cacheRead":0.02,"cacheWrite":0.25}]}}}}))
            .unwrap(),
        )
        .unwrap();
        let manifest = write_artifact(
            &base.join("runtime-manifest.json"),
            &serde_json::to_vec(&inventory(&runtime).unwrap()).unwrap(),
        )
        .unwrap();
        let docker = base.join("docker");
        fs::write(&docker, "fake docker").unwrap();
        let profile = CompositionProfile {
            schema: COMPOSITION_SCHEMA.into(),
            runtime: Tree {
                root: runtime,
                manifest,
            },
            docker,
            provider: "openai-codex".into(),
            model: "gpt-6-luna".into(),
            hosted: HostedRoutes {
                routes: vec![HostedRoute {
                    id: "primary".into(),
                    payer: "synthetic".into(),
                    endpoint: "loopback:1".into(),
                }],
                max_attempts_per_request: 1,
            },
            max_requests: 8,
            budget_microusd: 2_000_000,
            max_context_bytes: 500_000,
            max_tool_calls: 32,
            turn_deadline_secs: 900,
            job_secs: 1800,
            pi_settings: None,
        };
        (dir, base, profile)
    }

    fn write_profile(base: &Path, profile: &CompositionProfile) -> PathBuf {
        let path = base.join("composition.json");
        fs::write(&path, serde_json::to_vec(profile).unwrap()).unwrap();
        path
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[test]
    fn prepare_writes_one_admissible_relay_job() {
        let (_dir, base, profile) = fixture();
        let path = write_profile(&base, &profile);
        let root = base.join("native/attempt-1");
        let t = now();
        let record = prepare(&path, &root, "snapshot:h2-treatment-r3", t).unwrap();
        let read = |name: &str| -> Value {
            serde_json::from_slice(&fs::read(root.join(name)).unwrap()).unwrap()
        };
        let host = read("pi-host.json");
        let job: Job = serde_json::from_value(read("job.json")).unwrap();
        // The written job passes the Host's own fresh admission, and the
        // serve configuration loads it by digest.
        crate::pi_docker::validate_fresh(&job).unwrap();
        let (serve, loaded) = ServeConfig::load(&root.join("serve-config.json")).unwrap();
        assert_eq!(loaded.job_id, job.job_id);
        assert_eq!(serve.turn_deadline_secs, 900);
        assert_eq!(job.deadline_unix_seconds, t + 1800);
        assert_eq!(job.state_dir, root.join("pi-host-state"));
        let plan = job.coding.as_ref().unwrap().provider.as_ref().unwrap();
        assert_eq!(plan.hosted.routes, profile.hosted.routes);
        assert_eq!(plan.max_context_bytes, 500_000);
        assert_eq!(
            host["job_sha256"],
            digest(&fs::read(root.join("job.json")).unwrap())
        );
        assert_eq!(
            host["serve_config_sha256"],
            digest(&fs::read(root.join("serve-config.json")).unwrap())
        );
        assert_eq!(host["state_dir"], json!(root.join("pi-host-state")));
        assert_eq!(host["provider"], "openai-codex");
        assert_eq!(
            record["profile"]["sha256"],
            digest(&fs::read(&path).unwrap())
        );
        let mut written = read("composition.json");
        written["composition_sha256"] =
            json!(digest(&fs::read(root.join("composition.json")).unwrap()));
        assert_eq!(record, written);
        assert_eq!(
            record["pi_host_json_sha256"],
            digest(&fs::read(root.join("pi-host.json")).unwrap())
        );
        assert_eq!(
            read("inputs/workspace/snapshot-identity.json"),
            json!({"snapshot_id":"snapshot:h2-treatment-r3"})
        );
        // The shim and stream adapter are the Host's constants.
        assert_eq!(
            fs::read(root.join("inputs/pi-coding.mjs")).unwrap(),
            crate::pi_docker::coding::SHIM
        );
        assert_eq!(
            fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700,
            "the root is owner-private, as the runner requires of its native root"
        );
        // A second launch into the same root is refused: one job per root.
        let again = prepare(&path, &root, "snapshot:h2-treatment-r3", t).unwrap_err();
        assert_eq!(again.class, "root");
    }

    #[test]
    fn prepare_refuses_profiles_and_inputs_outside_its_bounds() {
        let (_dir, base, profile) = fixture();
        let attempt = |p: &CompositionProfile, snapshot: &str, n: &str| {
            let path = write_profile(&base, p);
            prepare(&path, &base.join("native").join(n), snapshot, now())
                .unwrap_err()
                .class
        };
        assert_eq!(attempt(&profile, "snapshot/with-slash", "a"), "snapshot-id");
        let mut p = profile.clone();
        p.job_secs = crate::pi_docker::coding::RELAY_DEADLINE_SECONDS + 1;
        assert_eq!(attempt(&p, "s", "b"), "profile", "job duration");
        let mut p = profile.clone();
        p.turn_deadline_secs = p.job_secs + 1;
        assert_eq!(attempt(&p, "s", "c"), "profile", "turn within the job");
        let mut p = profile.clone();
        p.provider = "unknown".into();
        assert_eq!(attempt(&p, "s", "d"), "profile", "provider catalog");
        let mut p = profile.clone();
        p.max_context_bytes = 500_001;
        assert_eq!(attempt(&p, "s", "e"), "admission", "relay context bound");
        let mut p = profile.clone();
        p.schema = "other".into();
        assert_eq!(attempt(&p, "s", "f"), "profile", "schema");
        let mut p = profile.clone();
        p.hosted.routes[0].endpoint = "https:anthropic-oauth".into();
        assert_eq!(attempt(&p, "s", "h"), "profile", "endpoint wire family");
        // A changed runtime file breaks the pinned manifest.
        fs::write(profile.runtime.root.join("extra"), "drift").unwrap();
        assert_eq!(attempt(&profile, "s", "g"), "admission", "runtime pin");
        let relative = Path::new("relative/root");
        let refusal = prepare(&write_profile(&base, &profile), relative, "s", now()).unwrap_err();
        assert_eq!(refusal.class, "root");
    }

    #[test]
    fn describe_names_each_payers_credential_kinds() {
        let (_dir, base, mut profile) = fixture();
        profile.provider = "claude-sdk".into();
        profile.model = "claude-haiku-5-5".into();
        profile.hosted = HostedRoutes {
            routes: vec![
                HostedRoute {
                    id: "subscription".into(),
                    payer: "claude".into(),
                    endpoint: "https:anthropic-oauth".into(),
                },
                HostedRoute {
                    id: "api".into(),
                    payer: "anthropic".into(),
                    endpoint: "https:anthropic-api".into(),
                },
            ],
            max_attempts_per_request: 2,
        };
        let path = write_profile(&base, &profile);
        let d = describe(&path).unwrap();
        assert_eq!(d["wire"], "messages");
        assert_eq!(d["profile_sha256"], digest(&fs::read(&path).unwrap()));
        assert_eq!(
            d["payers"],
            json!({"anthropic":["anthropic-api-key-env","anthropic-api-key-file"],
                "claude":["claude-code-auth"]})
        );
        assert_eq!(
            d["routes"][0],
            json!({"id":"subscription","endpoint":"https:anthropic-oauth","payer":"claude"})
        );
        // One payer on routes that need different credentials is refused.
        profile.hosted.routes[1].payer = "claude".into();
        let refusal = describe(&write_profile(&base, &profile)).unwrap_err();
        assert_eq!(refusal.class, "profile");
        // A loopback route takes no credential.
        let (_d2, base2, loopback) = fixture();
        assert_eq!(
            describe(&write_profile(&base2, &loopback)).unwrap()["payers"],
            json!({"synthetic":[]})
        );
    }
}
