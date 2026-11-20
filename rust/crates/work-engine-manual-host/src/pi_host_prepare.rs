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
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};

pub const COMPOSITION_SCHEMA: &str = "work-engine.pi-host-composition.v1";
pub const RECORD_SCHEMA: &str = "work-engine.pi-host-composition-record.v1";
/// The relay thread's task text; prompts arrive from the controller.
const TASK: &str = "Pi Host lifecycle relay thread: prompts arrive from the controller.";
/// The runner's thread-start (180 s) and close (120 s) allowances, which a
/// job must hold besides one turn.
const START_AND_CLOSE_SECS: u64 = 300;

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
/// Host's rules), `snapshot-id`, `root` (the root path, its parent or its
/// state is not one the Host can own and mount; see `make_root`),
/// `admission` (the composed job fails the Host's fresh admission) or `io`.
/// From `describe`, `root` and `io` mean its scratch environment failed
/// (`TMPDIR`), not that the profile is invalid.
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
    // An absolute, regular (not symlinked), bounded file.
    let meta = fs::symlink_metadata(profile_path).map_err(class("profile"))?;
    if !profile_path.is_absolute() || !meta.is_file() || meta.len() > 65_536 {
        return refuse(
            "profile",
            "profile must be an absolute regular file of at most 64 KiB",
        );
    }
    let bytes = fs::read(profile_path).map_err(class("profile"))?;
    let profile: CompositionProfile = serde_json::from_slice(&bytes).map_err(class("profile"))?;
    if profile.schema != COMPOSITION_SCHEMA {
        return refuse("profile", "composition profile schema");
    }
    // The job must hold thread start (180 s), one turn and close (120 s).
    if !(1..=crate::pi_docker::coding::RELAY_DEADLINE_SECONDS).contains(&profile.job_secs)
        || profile.turn_deadline_secs == 0
        || profile.turn_deadline_secs > profile.job_secs.saturating_sub(START_AND_CLOSE_SECS)
    {
        return refuse("profile", "composition job or turn duration");
    }
    if crate::pi_docker::coding::provider::catalog_path(&profile.provider).is_none() {
        return refuse("profile", "composition provider has no catalog");
    }
    match &profile.pi_settings {
        Some(settings)
            if !settings.is_object()
                || serde_json::to_vec(settings).map_or(true, |b| b.len() > 65_536) =>
        {
            return refuse(
                "profile",
                "public Pi settings must be an object of at most 64 KiB",
            );
        }
        _ => {}
    }
    profile.hosted.validate().map_err(class("profile"))?;
    let wire = match profile.provider.as_str() {
        "openai-codex" => crate::pi_stream::Wire::Responses,
        "claude-sdk" => crate::pi_stream::Wire::Messages,
        _ => crate::pi_stream::Wire::Completions,
    };
    let mut payers: std::collections::BTreeMap<String, Option<Vec<&str>>> = Default::default();
    let mut loopback_payers = std::collections::BTreeSet::new();
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
        if kinds.is_empty() {
            loopback_payers.insert(r.payer.clone());
            payers.entry(r.payer.clone()).or_insert(None);
        } else {
            // One payer holds one credential: it must serve every route it pays.
            let entry = payers.entry(r.payer.clone()).or_insert(None);
            let next: Vec<&str> = match entry.take() {
                None => kinds.to_vec(),
                Some(prev) => prev.into_iter().filter(|k| kinds.contains(k)).collect(),
            };
            if next.is_empty() {
                return refuse(
                    "profile",
                    "a payer's routes need incompatible credential kinds",
                );
            }
            *entry = Some(next);
        }
        routes.push(json!({"id":r.id,"endpoint":r.endpoint,"payer":r.payer}));
    }
    // A payer is either loopback-only (no credential) or https-only.
    if payers
        .iter()
        .any(|(p, k)| k.is_some() && loopback_payers.contains(p))
    {
        return refuse("profile", "a payer pays both loopback and https routes");
    }
    let payers: std::collections::BTreeMap<String, Vec<&str>> = payers
        .into_iter()
        .map(|(p, k)| (p, k.unwrap_or_default()))
        .collect();
    let sha256 = digest(&bytes);
    let description = json!({"schema":COMPOSITION_SCHEMA,"profile_sha256":sha256,
        "provider":profile.provider,"model":profile.model,"wire":format!("{wire:?}").to_lowercase(),
        "routes":routes,"payers":payers});
    Ok((profile, sha256, description))
}

/// `profile describe`: the profile checked by exactly the rules a launch
/// applies (a full `prepare` into a private scratch root under the system
/// temporary directory, removed afterwards) and described (payers with the
/// credential kinds that can serve all their routes). It never opens a
/// credentials file. Its only environment input is `TMPDIR` (the scratch
/// location). The agreement covers the profile, not a launch root: the
/// root rules of `make_root` apply to the caller's own root at `prepare`.
pub fn describe(profile_path: &Path) -> std::result::Result<Value, Refusal> {
    let (_, profile_sha256, description) = load_profile(profile_path)?;
    let scratch = std::env::temp_dir().join(format!(
        "pi-host-describe-{}-{}",
        std::process::id(),
        &digest(format!("{:?}", std::time::SystemTime::now()).as_bytes())[..12]
    ));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&scratch)
        .map_err(class("io"))?;
    let outcome = prepare(profile_path, &scratch.join("root"), "describe-dry-run", now);
    let _ = fs::remove_dir_all(&scratch);
    // The description and the dry run must be of the same profile bytes.
    if outcome?["profile"]["sha256"] != profile_sha256.as_str() {
        return refuse("profile", "profile changed while it was described");
    }
    Ok(description)
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
    let root = make_root(root)?;
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

/// The fresh root: an absolute path whose every component (ancestors
/// included) is plain (`[A-Za-z0-9._-]`, no `.` or `..` segment) so the Host
/// can mount it, under an existing parent directory that is not a symlink;
/// either absent (created here with mode 0700) or an empty, owner-only
/// directory of the caller that is not a symlink, re-checked after it is
/// resolved. Its
/// `inputs` directory is created here, so a concurrent `prepare` on the same
/// root refuses as `root`. A root is single use: any later refusal leaves it
/// populated.
fn make_root(root: &Path) -> std::result::Result<PathBuf, Refusal> {
    use std::path::Component;
    // `Path::components` drops interior `.` segments; refuse them first.
    let dotted = root
        .as_os_str()
        .as_encoded_bytes()
        .split(|b| *b == b'/')
        .any(|s| s == b".");
    let plain = !dotted
        && root.components().skip(1).all(|c| match c {
            Component::Normal(n) => n.to_str().is_some_and(|n| {
                n.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                    && n != "."
                    && n != ".."
            }),
            _ => false,
        });
    if !root.is_absolute() || !plain || root.parent().is_none() {
        return refuse("root", "root must be an absolute path of plain components");
    }
    let parent = root.parent().unwrap_or(root);
    if !fs::symlink_metadata(parent)
        .map_err(class("root"))?
        .is_dir()
    {
        return refuse(
            "root",
            "root parent must be an existing directory, not a symlink",
        );
    }
    let owner_only = |meta: &fs::Metadata| {
        meta.is_dir() && meta.uid() == unsafe { libc::geteuid() } && meta.mode() & 0o077 == 0
    };
    match fs::symlink_metadata(root) {
        Ok(meta) => {
            if !owner_only(&meta) || fs::read_dir(root).map_err(class("root"))?.next().is_some() {
                return refuse(
                    "root",
                    "root must be an empty owner-only directory, not a symlink",
                );
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(root)
                .map_err(class("root"))?;
        }
        Err(e) => return refuse("root", &e.to_string()),
    }
    let root = fs::canonicalize(root).map_err(class("root"))?;
    if !owner_only(&fs::symlink_metadata(&root).map_err(class("root"))?) {
        return refuse("root", "root changed while it was prepared");
    }
    fs::DirBuilder::new()
        .mode(0o700)
        .create(root.join("inputs"))
        .map_err(class("root"))?;
    Ok(root)
}

/// Writes the inputs and the job (not yet `job.json`).
fn compose(
    profile: &CompositionProfile,
    root: &Path,
    snapshot_id: &str,
    now_unix: u64,
) -> Result<Job> {
    let inputs = root.join("inputs");
    let state = root.join("pi-host-state");
    let workspace = inputs.join("workspace");
    let pi_config = inputs.join("pi-config");
    for d in [&state, &workspace, &pi_config] {
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

    fn native(base: &Path) -> PathBuf {
        let n = base.join("native");
        fs::create_dir_all(&n).unwrap();
        n
    }

    #[test]
    fn prepare_writes_one_admissible_relay_job() {
        let (_dir, base, profile) = fixture();
        let path = write_profile(&base, &profile);
        let root = native(&base).join("attempt-1");
        let t = now();
        let record = prepare(&path, &root, "snapshot:h2-treatment-r3", t).unwrap();
        let read = |name: &str| -> Value {
            serde_json::from_slice(&fs::read(root.join(name)).unwrap()).unwrap()
        };
        let host = read("pi-host.json");
        let job: Job = serde_json::from_value(read("job.json")).unwrap();
        crate::pi_docker::validate_fresh(&job).unwrap();
        let (serve, loaded) = ServeConfig::load(&root.join("serve-config.json")).unwrap();
        assert_eq!(loaded.job_id, job.job_id);
        assert_eq!(serve.turn_deadline_secs, 900);
        assert_eq!(job.deadline_unix_seconds, t + 1800);
        assert_eq!(job.state_dir, root.join("pi-host-state"));
        let plan = job.coding.as_ref().unwrap().provider.as_ref().unwrap();
        assert_eq!(plan.hosted.routes, profile.hosted.routes);
        assert_eq!(
            host["job_sha256"],
            digest(&fs::read(root.join("job.json")).unwrap())
        );
        assert_eq!(host["state_dir"], json!(root.join("pi-host-state")));
        assert_eq!(record["schema"], RECORD_SCHEMA);
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
        assert_eq!(
            fs::read(root.join("inputs/pi-coding.mjs")).unwrap(),
            crate::pi_docker::coding::SHIM
        );
        assert_eq!(fs::metadata(&root).unwrap().mode() & 0o777, 0o700);
        // A second launch into the same root is refused: one job per root.
        let again = prepare(&path, &root, "snapshot:h2-treatment-r3", t).unwrap_err();
        assert_eq!(again.class, "root");
        // An empty owner-only directory is accepted as a fresh root.
        let empty = native(&base).join("attempt-2");
        fs::DirBuilder::new().mode(0o700).create(&empty).unwrap();
        prepare(&path, &empty, "s", t).unwrap();
    }

    #[test]
    fn prepare_refusals_carry_their_class() {
        let (_dir, base, profile) = fixture();
        let n = native(&base);
        let attempt = |p: &CompositionProfile, snapshot: &str, leaf: &str| {
            let path = write_profile(&base, p);
            prepare(&path, &n.join(leaf), snapshot, now())
                .unwrap_err()
                .class
        };
        assert_eq!(attempt(&profile, "snapshot/with-slash", "a"), "snapshot-id");
        let mut p = profile.clone();
        p.job_secs = crate::pi_docker::coding::RELAY_DEADLINE_SECONDS + 1;
        assert_eq!(attempt(&p, "s", "b"), "profile", "job duration");
        let mut p = profile.clone();
        p.turn_deadline_secs = p.job_secs - START_AND_CLOSE_SECS + 1;
        assert_eq!(attempt(&p, "s", "c"), "profile", "start and close margin");
        let mut p = profile.clone();
        p.turn_deadline_secs = 0;
        assert_eq!(attempt(&p, "s", "c0"), "profile", "zero turn");
        // A huge turn refuses instead of overflowing the margin sum.
        let mut p = profile.clone();
        p.turn_deadline_secs = u64::MAX;
        assert_eq!(attempt(&p, "s", "c1"), "profile", "turn overflow");
        p.turn_deadline_secs = u64::MAX - 215;
        assert_eq!(attempt(&p, "s", "c2"), "profile", "turn overflow");
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
        let mut p = profile.clone();
        p.pi_settings = Some(json!([1]));
        assert_eq!(attempt(&p, "s", "i"), "profile", "pi settings");
        let mut p = profile.clone();
        p.docker = base.join("absent-docker");
        assert_eq!(attempt(&p, "s", "j"), "io", "unreadable docker");
        // An unknown profile field.
        let mut v = serde_json::to_value(&profile).unwrap();
        v["extra"] = json!(1);
        let path = base.join("extra.json");
        fs::write(&path, v.to_string()).unwrap();
        assert_eq!(
            prepare(&path, &n.join("k"), "s", now()).unwrap_err().class,
            "profile"
        );
        // Profile path: absolute, regular, not a symlink.
        let link = base.join("link.json");
        std::os::unix::fs::symlink(write_profile(&base, &profile), &link).unwrap();
        assert_eq!(
            prepare(&link, &n.join("l"), "s", now()).unwrap_err().class,
            "profile"
        );
        assert_eq!(
            prepare(Path::new("relative.json"), &n.join("m"), "s", now())
                .unwrap_err()
                .class,
            "profile"
        );
        // A changed runtime file breaks the pinned manifest.
        fs::write(profile.runtime.root.join("extra"), "drift").unwrap();
        assert_eq!(attempt(&profile, "s", "g"), "admission", "runtime pin");
    }

    #[test]
    fn prepare_refuses_roots_it_cannot_own_or_mount() {
        let (_dir, base, profile) = fixture();
        let path = write_profile(&base, &profile);
        let n = native(&base);
        let class = |root: &Path| prepare(&path, root, "s", now()).unwrap_err().class;
        assert_eq!(class(Path::new("relative/root")), "root");
        assert_eq!(class(&n.join("attempt:1")), "root", "':' cannot be mounted");
        assert_eq!(class(&n.join("a,b")), "root", "',' cannot be mounted");
        assert!(!n.join("attempt:1").exists(), "refused before creation");
        let dotted = PathBuf::from(format!("{}/./dotted", n.display()));
        assert_eq!(class(&dotted), "root", "interior '.' segment");
        assert!(!n.join("dotted").exists());
        let real_parent = base.join("real-parent");
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&real_parent)
            .unwrap();
        std::os::unix::fs::symlink(&real_parent, n.join("parent-link")).unwrap();
        assert_eq!(
            class(&n.join("parent-link/attempt")),
            "root",
            "symlinked parent"
        );
        assert!(!real_parent.join("attempt").exists());
        assert_eq!(
            class(&n.join("missing/attempt")),
            "root",
            "parent must exist"
        );
        let target = base.join("elsewhere");
        fs::DirBuilder::new().mode(0o700).create(&target).unwrap();
        let link = n.join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(class(&link), "root", "symlink");
        let file = n.join("file");
        fs::write(&file, "x").unwrap();
        assert_eq!(class(&file), "root", "a file");
        let full = n.join("full");
        fs::DirBuilder::new().mode(0o700).create(&full).unwrap();
        fs::write(full.join("x"), "x").unwrap();
        assert_eq!(class(&full), "root", "not empty");
        let open = n.join("open");
        fs::DirBuilder::new().mode(0o755).create(&open).unwrap();
        assert_eq!(class(&open), "root", "not owner-only");
    }

    #[test]
    fn describe_agrees_with_prepare_and_names_credential_kinds() {
        let (_dir, base, mut profile) = fixture();
        profile.hosted = HostedRoutes {
            routes: vec![
                HostedRoute {
                    id: "subscription".into(),
                    payer: "chatgpt".into(),
                    endpoint: "https:chatgpt-codex".into(),
                },
                HostedRoute {
                    id: "api".into(),
                    payer: "openai".into(),
                    endpoint: "https:openai-responses".into(),
                },
            ],
            max_attempts_per_request: 2,
        };
        let path = write_profile(&base, &profile);
        let d = describe(&path).unwrap();
        assert_eq!(d["wire"], "responses");
        assert_eq!(d["profile_sha256"], digest(&fs::read(&path).unwrap()));
        assert_eq!(
            d["payers"],
            json!({"chatgpt":["chatgpt-pi-auth"],"openai":["api-key-env","api-key-file"]})
        );
        assert_eq!(
            d["routes"][0],
            json!({"id":"subscription","endpoint":"https:chatgpt-codex","payer":"chatgpt"})
        );
        // describe refuses exactly what a launch refuses, with the same class.
        let n = native(&base);
        for (i, change) in [
            (|p: &mut CompositionProfile| p.model = "not-a-model".into())
                as fn(&mut CompositionProfile),
            |p| p.max_tool_calls = 100_000,
            |p| p.pi_settings = Some(json!([1])),
            |p| p.max_requests = 0,
        ]
        .iter()
        .enumerate()
        {
            let mut p = profile.clone();
            change(&mut p);
            let path = write_profile(&base, &p);
            let d = describe(&path).unwrap_err();
            let l = prepare(&path, &n.join(format!("agree-{i}")), "s", now()).unwrap_err();
            assert_eq!(d.class, l.class, "{i}: {d} / {l}");
        }
        // One payer on routes that need different credentials is refused.
        let mut p = profile.clone();
        p.hosted.routes[1].payer = "chatgpt".into();
        assert_eq!(
            describe(&write_profile(&base, &p)).unwrap_err().class,
            "profile"
        );
        // A payer on loopback and https routes is refused in either order.
        for order in [[0, 1], [1, 0]] {
            let mut p = profile.clone();
            let mut routes = vec![
                p.hosted.routes[0].clone(),
                HostedRoute {
                    id: "loop".into(),
                    payer: "chatgpt".into(),
                    endpoint: "loopback:1".into(),
                },
            ];
            if order == [1, 0] {
                routes.reverse();
            }
            p.hosted.routes = routes;
            assert_eq!(
                describe(&write_profile(&base, &p)).unwrap_err().class,
                "profile"
            );
        }
        // A loopback route takes no credential.
        let (_d2, base2, loopback) = fixture();
        assert_eq!(
            describe(&write_profile(&base2, &loopback)).unwrap()["payers"],
            json!({"synthetic":[]})
        );
    }
}
