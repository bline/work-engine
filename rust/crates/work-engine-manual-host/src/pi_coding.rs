//! Provider-disabled coding-path qualification in the existing Pi custody owner.
//! This profile intentionally cannot authorize provider or credential effects.
use super::*;
#[path = "pi_coding_auth.rs"]
pub mod auth;
#[path = "pi_coding_provider.rs"]
pub mod provider;
#[path = "pi_coding_validation.rs"]
pub mod validation;
use std::os::unix::fs::PermissionsExt;
pub const SIGNED_SUBSCRIPTION_PROFILE: &str = "work-engine.pi-coding-signed-subscription.v1";
pub const SIGNED_PROFILE: &str = "work-engine.pi-coding-signed-fixture.v1";
pub const PROFILE: &str = "work-engine.pi-coding-fixture.v1";
/// Opt-in composition: Host owns effects; an installed lifecycle gate owns entry.
pub const BRIDGE_PROFILE: &str = "work-engine.pi-coding-lifecycle-fixture.v1";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodingToolIntent {
    pub job_id: String,
    pub input_digest: String,
    pub session_id: String,
    pub call_id: String,
    pub command_sha256: String,
}
/// Trusted composition, not an inference capability. Implementations serialize
/// current lifecycle admission with the supplied existing Host actuator.
/// A gate error leaves the already-durable Host intent unresolved. Reopen never
/// reinstalls a gate or reexecutes the effect.
pub trait CodingEffectGate {
    fn execute(
        &mut self,
        intent: &CodingToolIntent,
        actuator: &mut dyn FnMut(i64) -> Result<(bool, Vec<u8>)>,
    ) -> Result<(bool, Vec<u8>)>;
    fn frame(&mut self, _frame: &Value) -> Result<Vec<Value>> {
        refuse("coding lifecycle control unavailable")
    }
    fn dispatch(
        &mut self,
        _command: &Value,
        _send: &mut dyn FnMut(i64) -> Result<()>,
    ) -> Result<()> {
        refuse("coding lifecycle IPC unavailable")
    }
    /// Called only after the exact result artifact is persisted in the journal.
    fn observed(
        &mut self,
        intent: &CodingToolIntent,
        result: &Artifact,
        process: &ProcessRecord,
        tool_container: &str,
    ) -> Result<()>;
}
pub const TOOL_IMAGE: &str = "cbm-doc-claims-lint-tidy@sha256:dda6d32b2c2df1064f056db0db4cff894c833089ccf92862331359c0e6072754";
pub const SHIM: &[u8] = include_bytes!("../receiver/pi-coding.mjs");
/// Private provider-disabled pressure qualification; never a provider/task grant.
pub const PRESSURE_PROFILE: &str = "work-engine.pi-pressure-fixture.v2";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PressurePacket {
    pub schema: String,
    pub source: String,
    pub generation: String,
    pub budget: Artifact,
    pub max_age_ms: u64,
}
impl PressurePacket {
    fn validate(&self, profile: &Profile) -> Result<()> {
        if self.schema != PRESSURE_PROFILE
            || profile.schema != PROFILE
            || profile.provider.is_some()
            || self.source.trim().is_empty()
            || self.generation.trim().is_empty()
            || !(1000..=30000).contains(&self.max_age_ms)
            || self.budget.verify()?.len() > 12000
        {
            return refuse("pressure packet fixture/bounds conflict");
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PressureRelease {
    pub schema: String,
    pub input_digest: String,
    pub container_id: String,
    pub session_id: String,
    pub source: String,
    pub generation: String,
    pub sequence: u64,
    pub observed_ms: u64,
    pub pressure: String,
}
/// One-shot transport validation; the accepted observer owns pressure derivation.
/// No payload changes deadline, task/provider permission or recovery authority.
pub fn validate_pressure_release(
    packet: &PressurePacket,
    release: &PressureRelease,
    barrier: &Value,
    budget: &str,
    now_ms: u64,
    fenced: bool,
) -> Result<()> {
    let snapshot = &barrier["snapshot"];
    if fenced
        || release.schema != PRESSURE_PROFILE
        || release.sequence != 1
        || release.input_digest != barrier["input_digest"]
        || release.container_id != barrier["container_id"]
        || release.session_id != barrier["sessionId"]
        || release.source != packet.source
        || release.generation != packet.generation
        || release.pressure.len() > 24000
        || release.observed_ms != snapshot["observedMs"].as_u64().unwrap_or(0)
        || release.observed_ms > now_ms
        || now_ms - release.observed_ms > packet.max_age_ms
    {
        return refuse("pressure release binding/freshness/fence conflict");
    }
    let payload: Value = serde_json::from_str(&release.pressure)?;
    let input = &payload["context"]["input"];
    if payload["schema"] != PRESSURE_PROFILE
        || payload["authority"] != "observation_only"
        || payload["budget_control"]["raw"] != budget
        || payload["sdk_usage"] != snapshot["usage"]
        || input["identity"]["source"] != packet.source
        || input["identity"]["session"] != release.session_id
        || input["identity"]["generation"] != packet.generation
        || input["identity"]["model"] != snapshot["model"]["id"]
        || input["identity"]["window"] != snapshot["model"]["contextWindow"]
        || input["sequence"] != release.sequence
        || input["observed_ms"] != release.observed_ms
        || input["tokens"] != snapshot["usage"]["value"]["tokens"]
    {
        return refuse("pressure payload observation/budget conflict");
    }
    Ok(())
}
/// Filesystem containment only; this is never signed admission or qualification.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceWriteBoundary {
    #[default]
    LegacyWorkspace,
    SrcLibRs,
}
impl WorkspaceWriteBoundary {
    fn is_legacy(&self) -> bool {
        *self == Self::LegacyWorkspace
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: String,
    pub shim: Artifact,
    pub toolchain: Tree,
    pub acceptance: Artifact,
    /// Pinned fixture input, never represented as provider-generated work.
    #[serde(default)]
    pub fixture_command: String,
    pub max_tool_calls: u32,
    #[serde(default, skip_serializing_if = "WorkspaceWriteBoundary::is_legacy")]
    pub workspace_write_boundary: WorkspaceWriteBoundary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<provider::ProviderPlan>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_workspace_manifest: Option<Artifact>,
    pub tool_create_intent: bool,
    pub tool_id: Option<String>,
    pub tool_removed: bool,
    pub tool_exit: Option<i64>,
    pub frames: usize,
    pub tool_requests: BTreeMap<String, Option<Artifact>>,
    pub native_tool_ends: u32,
    #[serde(default)]
    pub native_tool_ids: BTreeSet<String>,
    pub completed: bool,
    #[serde(default)]
    pub ready_observed: bool,
    #[serde(default)]
    pub provider_requests: BTreeMap<u64, provider::Reservation>,
    #[serde(default)]
    pub provider_failed: bool,
    #[serde(default)]
    pub access_removed: bool,
    #[serde(default)]
    pub validation_intent: bool,
    #[serde(default)]
    pub validation_id: Option<String>,
    #[serde(default)]
    pub validation_removed: bool,
    #[serde(default)]
    pub validation_exit: Option<i64>,
    #[serde(default)]
    pub validation_snapshot: Option<Tree>,
    #[serde(default)]
    pub final_acceptance: Option<validation::Proof>,
}
impl Profile {
    pub(super) fn validate(&self, job: &Job) -> Result<()> {
        if self.schema
            != if self.provider.is_some() {
                if self.schema == SIGNED_SUBSCRIPTION_PROFILE {
                    SIGNED_SUBSCRIPTION_PROFILE
                } else {
                    provider::LIVE_PROFILE
                }
            } else if self.schema == BRIDGE_PROFILE {
                BRIDGE_PROFILE
            } else if self.schema == SIGNED_PROFILE {
                SIGNED_PROFILE
            } else {
                PROFILE
            }
            || self.shim.verify()? != SHIM
            || !self.shim.path.is_absolute()
            || fs::canonicalize(&self.shim.path)? != self.shim.path
            || self.shim.path.to_string_lossy().contains([',', ':'])
            || if self.provider.is_none() {
                self.fixture_command.is_empty() || self.fixture_command.len() > 8192
            } else {
                !self.fixture_command.is_empty()
            }
            || !(1..=8).contains(&self.max_tool_calls)
            || (self.schema == BRIDGE_PROFILE && self.max_tool_calls != 1)
            || job.idle_seconds != 0
        {
            return refuse("coding fixture profile/pins/limits conflict");
        }
        if let Some(plan) = &self.provider {
            plan.validate(job)?;
        }
        if self.workspace_write_boundary == WorkspaceWriteBoundary::SrcLibRs {
            strict_source_file(&job.workspace.root)?;
        }
        self.toolchain.verify()?;
        self.acceptance.verify()?;
        for path in [&self.toolchain.root, &self.acceptance.path] {
            if !path.is_absolute()
                || fs::canonicalize(path)? != *path
                || path.to_string_lossy().contains([',', ':'])
                || path.starts_with(&job.state_dir)
                || job.state_dir.starts_with(path)
                || path.starts_with(&job.runtime.root)
                || path.starts_with(&job.workspace.root)
            {
                return refuse("coding toolchain/acceptance isolation conflict");
            }
        }
        if !self.toolchain.root.join("bin/rustc").is_file() {
            return refuse("offline rustc absent");
        }
        Ok(())
    }
}
fn strict_source_file(workspace: &Path) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    for path in [workspace.to_path_buf(), workspace.join("src")] {
        let m = fs::symlink_metadata(path)?;
        if !m.is_dir() || m.file_type().is_symlink() {
            return refuse("strict coding workspace parent is not a regular directory");
        }
    }
    let m = fs::symlink_metadata(workspace.join("src/lib.rs"))?;
    if !m.is_file() || m.file_type().is_symlink() || m.nlink() != 1 {
        return refuse("strict coding source is not a single regular file");
    }
    Ok(())
}
fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| (*s).into()).collect()
}
fn mount(args: &mut Vec<String>, source: &Path, target: &str, rw: bool) {
    args.extend([
        "--mount".into(),
        format!(
            "type=bind,src={},dst={target}{}",
            source.display(),
            if rw { "" } else { ",readonly" }
        ),
    ]);
}
fn base(j: &Journal, name: &str) -> Vec<String> {
    let tmpfs = if name != j.container_name {
        "/tmp:rw,exec,nosuid,nodev,size=64m"
    } else {
        "/tmp:rw,noexec,nosuid,nodev,size=64m"
    };
    let live = j.job.coding.as_ref().is_some_and(|p| p.provider.is_some());
    let network = if live && name == j.container_name {
        "bridge"
    } else {
        "none"
    };
    let mut args = strings(&[
        "create",
        "--pull",
        "never",
        "--interactive",
        "--name",
        name,
        "--read-only",
        "--network",
        network,
        "--cap-drop",
        "ALL",
        "--security-opt",
        "no-new-privileges",
        "--pids-limit",
        "64",
        "--memory",
        "512m",
        "--cpus",
        "1",
        "--user",
        "1000:1000",
        "--tmpfs",
        tmpfs,
        "--workdir",
        "/workspace",
        "--env",
        "HOME=/tmp/home",
        "--env",
        "PATH=/toolchain/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    ]);
    args.extend([
        "--label".into(),
        format!("work-engine.pi-job={}", j.job.job_id),
        "--label".into(),
        format!("work-engine.pi-input={}", j.input_digest),
    ]);
    args
}
fn tool_name(j: &Journal) -> String {
    format!("{}-tool", j.container_name)
}
fn expected_mounts(
    j: &Journal,
    tool: bool,
    validation: bool,
) -> Vec<(PathBuf, &'static str, bool)> {
    let profile = j.job.coding.as_ref().expect("coding profile");
    if tool {
        let mut mounts = vec![
            (profile.toolchain.root.clone(), "/toolchain", false),
            (profile.acceptance.path.clone(), "/acceptance.rs", false),
            (
                if validation {
                    j.coding
                        .as_ref()
                        .unwrap()
                        .validation_snapshot
                        .as_ref()
                        .expect("snapshot before validator")
                        .root
                        .clone()
                } else {
                    j.job.state_dir.join("coding-workspace")
                },
                "/workspace",
                !validation
                    && profile.workspace_write_boundary == WorkspaceWriteBoundary::LegacyWorkspace,
            ),
        ];
        if !validation && profile.workspace_write_boundary == WorkspaceWriteBoundary::SrcLibRs {
            mounts.push((
                j.job.state_dir.join("coding-workspace/src/lib.rs"),
                "/workspace/src/lib.rs",
                true,
            ));
        }
        return mounts;
    }
    let mut mounts = vec![
        (j.job.runtime.root.clone(), "/pi", false),
        (profile.shim.path.clone(), "/receiver/receiver.mjs", false),
        (j.job.state_dir.join("exports"), "/export", true),
    ];
    if let Some(plan) = &profile.provider {
        mounts.extend([
            (
                plan.access_adapter.path.clone(),
                "/receiver/pi-native-access.mjs",
                false,
            ),
            (
                j.job.state_dir.join("coding-public-config"),
                "/config",
                false,
            ),
            (j.job.state_dir.join("coding-public"), "/profile", false),
            (
                j.job
                    .state_dir
                    .join(if profile.schema == SIGNED_SUBSCRIPTION_PROFILE {
                        "private-access"
                    } else {
                        "private-access/access.json"
                    }),
                if profile.schema == SIGNED_SUBSCRIPTION_PROFILE {
                    "/auth"
                } else {
                    "/auth/access.json"
                },
                false,
            ),
        ]);
    } else {
        mounts.push((j.job.state_dir.join("coding-canary"), "/auth", false));
    }
    mounts
}
fn command(tool: bool) -> Vec<String> {
    if tool {
        strings(&["-c", "exec sleep 300"])
    } else {
        strings(&["node", "/receiver/receiver.mjs"])
    }
}
fn verify(j: &Journal, v: &Value, tool: bool, validation: bool) -> Result<()> {
    let name = if validation {
        validation::name(j)
    } else if tool {
        tool_name(j)
    } else {
        j.container_name.clone()
    };
    let expected_id = if validation {
        j.coding.as_ref().and_then(|c| c.validation_id.as_ref())
    } else if tool {
        j.coding.as_ref().and_then(|c| c.tool_id.as_ref())
    } else {
        j.container_id.as_ref()
    };
    if !pin_is_hash(v["Id"].as_str().unwrap_or(""))
        || expected_id.is_some_and(|id| v["Id"] != *id)
        || v["Name"] != format!("/{name}")
        || v["Config"]["Image"] != if tool { TOOL_IMAGE } else { &j.job.image }
        || v["Config"]["Labels"]["work-engine.pi-input"] != j.input_digest
        || v["Config"]["Labels"]["work-engine.pi-job"] != j.job.job_id
        || v["Config"]["User"] != "1000:1000"
        || v["Config"]["Cmd"]
            != json!(if validation {
                validation::command()
            } else {
                let mut args = command(tool);
                if !tool
                    && j.job
                        .coding
                        .as_ref()
                        .is_some_and(|p| p.schema == BRIDGE_PROFILE)
                {
                    args.push("--lifecycle-bridge".into());
                }
                args
            })
        || v["Config"]["Entrypoint"]
            != if tool {
                json!(["/bin/sh"])
            } else {
                json!(["docker-entrypoint.sh"])
            }
        || v["Config"]["WorkingDir"] != "/workspace"
        || v["HostConfig"]["PidMode"] != ""
        || v["HostConfig"]["IpcMode"] != "private"
        || v["HostConfig"]["NetworkMode"]
            != if !tool && j.job.coding.as_ref().unwrap().provider.is_some() {
                "bridge"
            } else {
                "none"
            }
        || v["HostConfig"]["ReadonlyRootfs"] != true
        || v["HostConfig"]["Privileged"] != false
        || v["HostConfig"]["CapDrop"] != json!(["ALL"])
        || v["HostConfig"]["SecurityOpt"] != json!(["no-new-privileges"])
        || v["HostConfig"]["Memory"] != 536870912_u64
        || v["HostConfig"]["PidsLimit"] != 64
        || v["HostConfig"]["NanoCpus"] != 1_000_000_000_u64
        || v["HostConfig"]["Tmpfs"]
            != if tool {
                json!({"/tmp":"rw,exec,nosuid,nodev,size=64m"})
            } else {
                json!({"/tmp":"rw,noexec,nosuid,nodev,size=64m"})
            }
    {
        return refuse("coding container identity/containment mismatch");
    }
    let mounts = v["Mounts"]
        .as_array()
        .ok_or_else(|| Error::Refused("mount evidence absent".into()))?;
    let expected = expected_mounts(j, tool, validation);
    if mounts.len() != expected.len()
        || expected.iter().any(|(src, dst, rw)| {
            !mounts.iter().any(|m| {
                m["Source"] == src.to_string_lossy().as_ref()
                    && m["Destination"] == *dst
                    && m["RW"] == *rw
                    && m["Type"] == "bind"
            })
        })
    {
        return refuse("coding mount isolation mismatch");
    }
    // Docker env contains fixed image defaults and our fixed public values only.
    let env = v["Config"]["Env"]
        .as_array()
        .ok_or_else(|| Error::Refused("env evidence absent".into()))?;
    if env.iter().any(|e| {
        let s = e.as_str().unwrap_or("");
        ![
            "PATH=",
            "HOME=",
            "NODE_VERSION=",
            "YARN_VERSION=",
            "PI_OFFLINE=",
            "PI_CODING_AGENT_DIR=",
            "DEBIAN_FRONTEND=",
        ]
        .iter()
        .any(|prefix| s.starts_with(prefix))
    }) {
        return refuse("unexpected coding environment");
    }
    Ok(())
}
pub(super) fn verify_inference(j: &Journal, v: &Value) -> Result<()> {
    verify(j, v, false, false)
}
impl Host {
    /// Gate binding is explicit, fresh and non-recoverable. Existing profiles
    /// cannot acquire lifecycle semantics by merely installing a callback.
    pub fn bind_coding_effect_gate(&mut self, gate: Box<dyn CodingEffectGate>) -> Result<()> {
        if self.journal.phase != Phase::Admitted
            || self
                .journal
                .job
                .coding
                .as_ref()
                .is_none_or(|p| p.schema != BRIDGE_PROFILE || p.provider.is_some())
            || self.coding_gate.is_some()
        {
            return refuse("coding lifecycle gate binding conflict");
        }
        self.coding_gate = Some(gate);
        Ok(())
    }
    pub(super) fn coding_bridge_frame(&mut self, frame: &Value) -> Result<Vec<Vec<u8>>> {
        let mut frame = frame.clone();
        if frame["type"] == "coding_provider_request" && frame["id"] == 2 {
            self.coding_collect_tool()?;
            if !self.journal.coding.as_ref().unwrap().tool_removed {
                return refuse("coding checkpoint writers not excluded");
            }
            let path = self
                .journal
                .job
                .state_dir
                .join("coding-bridge-workspace.json");
            atomic(
                &path,
                &inventory(&self.journal.job.state_dir.join("coding-workspace"))?,
            )?;
            self.journal
                .coding
                .as_mut()
                .unwrap()
                .bridge_workspace_manifest = Some(Artifact {
                sha256: file_digest(&path, None)?,
                path,
            });
            self.save()?;
        }
        if let Some(manifest) = &self
            .journal
            .coding
            .as_ref()
            .unwrap()
            .bridge_workspace_manifest
        {
            let bytes = manifest.verify()?;
            if serde_json::from_slice::<Value>(&bytes)?
                != serde_json::to_value(inventory(
                    &self.journal.job.state_dir.join("coding-workspace"),
                )?)?
            {
                return refuse("coding checkpoint workspace changed");
            }
            frame["host_workspace"] = json!({"manifest":manifest,"manifest_bytes":String::from_utf8(bytes).map_err(|_|Error::Refused("manifest UTF8".into()))?});
        }
        let replies = self
            .coding_gate
            .as_mut()
            .ok_or_else(|| Error::Refused("coding lifecycle gate missing".into()))?
            .frame(&frame)?;
        replies
            .into_iter()
            .map(|mut v| {
                if v["type"] == "bridge_prompt" {
                    v["command"] = json!(self.journal.job.coding.as_ref().unwrap().fixture_command);
                }
                Ok(serde_json::to_vec(&v)?)
            })
            .collect()
    }
    pub(super) fn coding_bridge_dispatch(
        &mut self,
        raw: &[u8],
        send: &mut dyn FnMut(&[u8]) -> Result<()>,
    ) -> Result<()> {
        let command: Value = serde_json::from_slice(raw)?;
        let gate = self
            .coding_gate
            .as_mut()
            .ok_or_else(|| Error::Refused("coding lifecycle gate missing".into()))?;
        let deadline: i64 = self
            .journal
            .job
            .deadline_unix_seconds
            .saturating_mul(1000)
            .try_into()
            .map_err(|_| Error::Refused("deadline range".into()))?;
        gate.dispatch(&command, &mut |expires| {
            let until = expires.min(deadline);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| Error::Refused("clock".into()))?
                .as_millis() as i64;
            if now >= until {
                return refuse("coding IPC expired");
            }
            let mut command = command.clone();
            command["consumptionDeadlineMs"] = json!(until);
            let mut bytes = serde_json::to_vec(&command)?;
            bytes.push(b'\n');
            send(&bytes)
        })
    }
    pub(super) fn signed_pi(&self) -> bool {
        self.journal.job.coding.as_ref().is_some_and(|p| {
            matches!(
                p.schema.as_str(),
                SIGNED_PROFILE | SIGNED_SUBSCRIPTION_PROFILE
            )
        })
    }
    pub(super) fn signed_subscription(&self) -> bool {
        self.journal
            .job
            .coding
            .as_ref()
            .is_some_and(|p| p.schema == SIGNED_SUBSCRIPTION_PROFILE)
    }
    fn pi_lease(
        &self,
        effect: session_task_admission::pi_pilot::Effect,
    ) -> Result<Option<session_task_admission::pi_pilot::EffectLease>> {
        self.pi_owner
            .as_ref()
            .map(|o| o.enter(effect).map_err(|e| Error::Refused(e.into())))
            .transpose()
    }
    pub(super) fn coding_inference_args(&self) -> Vec<String> {
        let j = &self.journal;
        let mut args = base(j, &j.container_name);
        if j.job.coding.as_ref().unwrap().provider.is_some() {
            args.extend(strings(&["--env", "PI_CODING_AGENT_DIR=/config"]));
        } else {
            args.extend(strings(&[
                "--env",
                "PI_OFFLINE=1",
                "--env",
                "PI_CODING_AGENT_DIR=/tmp/pi-empty",
            ]));
        }
        for (src, dst, rw) in expected_mounts(j, false, false) {
            mount(&mut args, &src, dst, rw);
        }
        args.push(j.job.image.clone());
        args.extend(command(false));
        if j.job.coding.as_ref().unwrap().schema == BRIDGE_PROFILE {
            args.push("--lifecycle-bridge".into());
        }
        args
    }
    fn coding_inspect_tool(&mut self) -> Result<Value> {
        let (ok, bytes) = self.command(
            "coding-tool-inspect",
            vec!["inspect".into(), tool_name(&self.journal)],
            false,
        )?;
        if !ok {
            return refuse("tool inspect unavailable");
        }
        let rows: Vec<Value> = serde_json::from_slice(&bytes)?;
        if rows.len() != 1 {
            return refuse("tool inspect ambiguous");
        }
        verify(&self.journal, &rows[0], true, false)?;
        self.journal.coding.as_mut().unwrap().tool_id = rows[0]["Id"].as_str().map(String::from);
        self.save()?;
        Ok(rows[0].clone())
    }
    pub(super) fn coding_prepare_tool(&mut self) -> Result<()> {
        self.coding_prepare_provider()?;
        let j = &self.journal;
        let workspace = j.job.state_dir.join("coding-workspace");
        fs::create_dir(&workspace)?;
        for (path, entry) in inventory(&j.job.workspace.root)? {
            if !matches!(entry, Entry::File { .. }) {
                return refuse("coding input symlink refused");
            }
            let dst = workspace.join(&path);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(j.job.workspace.root.join(path), dst)?;
        }
        if j.job.coding.as_ref().unwrap().provider.is_none() {
            let canary = j.job.state_dir.join("coding-canary");
            fs::create_dir(&canary)?;
            if !self.signed_pi() {
                fs::write(canary.join("auth.json"), b"FAKE-CANARY-NOT-A-CREDENTIAL\n")?;
            }
            fs::set_permissions(&canary, fs::Permissions::from_mode(0o755))?;
            if self.signed_pi() {
                fs::write(canary.join("signed-fixture.json"), br#"{"schema":"work-engine.pi-coding-signed-fixture.v1","classification":"public bootstrap marker; no credential"}"#)?;
            }
        }
        if j.job.coding.as_ref().unwrap().workspace_write_boundary
            == WorkspaceWriteBoundary::SrcLibRs
        {
            strict_source_file(&workspace)?;
        }
        let mut args = base(j, &tool_name(j));
        for (src, dst, rw) in expected_mounts(j, true, false) {
            mount(&mut args, &src, dst, rw);
        }
        args.extend(strings(&["--entrypoint", "/bin/sh"]));
        args.push(TOOL_IMAGE.into());
        args.extend(command(true));
        self.journal.coding.as_mut().unwrap().tool_create_intent = true;
        self.save()?;
        let (ok, _) = self.command("coding-tool-create", args, false)?;
        if !ok {
            return refuse("tool create uncertain; no retry");
        }
        self.coding_inspect_tool()?;
        if self.expired() || self.cancelled() {
            return refuse("tool start expired/cancelled");
        }
        let (ok, _) = self.command(
            "coding-tool-start",
            vec!["start".into(), tool_name(&self.journal)],
            false,
        )?;
        if !ok {
            return refuse("tool start uncertain; no retry");
        }
        Ok(())
    }
    pub(super) fn coding_advance(&mut self, raw: &[u8]) -> Result<Vec<Vec<u8>>> {
        let mut replies = vec![];
        let count = self.journal.coding.as_ref().unwrap().frames;
        let lines: Vec<_> = raw
            .split_inclusive(|b| *b == b'\n')
            .filter(|line| line.ends_with(b"\n"))
            .collect();
        for line in lines.into_iter().skip(count) {
            if line.len() > 65536 {
                return refuse("coding frame exceeded bound");
            }
            let frame: Value = serde_json::from_slice(line)?;
            if frame["schema"] != self.journal.job.coding.as_ref().unwrap().schema {
                return refuse("coding schema mismatch");
            }
            let kind = frame["type"].as_str().unwrap_or("");
            if self.journal.coding.as_ref().unwrap().completed {
                return refuse("frames after coding completion");
            }
            if kind != "ready"
                && !(self.signed_pi() && kind == "allocated")
                && !(self.journal.job.coding.as_ref().unwrap().schema == BRIDGE_PROFILE
                    && kind == "coding_replaced")
                && (self.journal.session_id.is_none()
                    || frame["sessionId"].as_str() != self.journal.session_id.as_deref())
            {
                return refuse("coding session mismatch");
            }
            match kind {
                "allocated" if self.signed_pi() => {
                    if self.journal.session_id.is_some() {
                        return refuse("replacement native allocation");
                    }
                    let native = frame["sessionId"]
                        .as_str()
                        .ok_or_else(|| Error::Refused("allocation ID absent".into()))?;
                    let owner = self
                        .pi_owner
                        .as_ref()
                        .ok_or_else(|| Error::Refused("Pi owner absent".into()))?;
                    owner
                        .seal(&self.journal.input_digest, native)
                        .map_err(|e| Error::Refused(e.into()))?;
                    self.journal.session_id = Some(native.into());
                    self.save()?;
                    let lease = self
                        .pi_lease(session_task_admission::pi_pilot::Effect::Initialization)?
                        .ok_or_else(|| Error::Refused("Pi initialization lease absent".into()))?;
                    lease
                        .check_current()
                        .map_err(|e| Error::Refused(e.into()))?;
                    if self.expired() || self.cancelled() {
                        return refuse("fixture initialization expired/cancelled");
                    }
                    if frame["accessPresent"] != false {
                        return refuse("fixture access existed before allocation");
                    }
                    let consumption_deadline = lease
                        .consumption_deadline_millis(self.journal.job.deadline_unix_seconds)
                        .map_err(|e| Error::Refused(e.into()))?;
                    let reply = if self.signed_subscription() {
                        // Access content is first opened only after the exact allocation is
                        // sealed and current signed initialization authority is held.
                        lease
                            .check_current()
                            .map_err(|e| Error::Refused(e.into()))?;
                        self.coding_project_provider_access(Some(&lease))?;
                        json!({"type":"initialize","sessionId":native,"deadline":self.journal.job.deadline_unix_seconds,"consumptionDeadlineMs":consumption_deadline})
                    } else {
                        let access = self.journal.job.state_dir.join("coding-canary/access.json");
                        let mut access_file = OpenOptions::new()
                            .create_new(true)
                            .write(true)
                            .open(&access)?;
                        access_file.write_all(br#"{"classification":"fake_fixture","access":"FAKE-NEVER-A-CREDENTIAL"}"#)?;
                        access_file.sync_all()?;
                        fs::set_permissions(&access, fs::Permissions::from_mode(0o644))?;
                        File::open(access.parent().unwrap())?.sync_all()?;
                        json!({"type":"initialize","sessionId":native,"fixtureAccess":"FAKE-NEVER-A-CREDENTIAL","deadline":self.journal.job.deadline_unix_seconds,"consumptionDeadlineMs":consumption_deadline})
                    };
                    lease
                        .check_current()
                        .map_err(|e| Error::Refused(e.into()))?;
                    self.reply_leases.push(lease);
                    replies.push(serde_json::to_vec(&reply)?);
                }
                "provider_request" | "provider_observation" | "provider_complete" => {
                    if let Some(reply) = self.coding_provider_frame(&frame)? {
                        replies.push(serde_json::to_vec(&reply)?);
                    }
                }
                "ready" => {
                    if self.journal.coding.as_ref().unwrap().ready_observed
                        || (!self.signed_pi() && self.journal.session_id.is_some())
                        || (self.signed_pi()
                            && self.journal.session_id.as_deref() != frame["sessionId"].as_str())
                        || frame["tools"] != json!(["workspace_exec"])
                    {
                        return refuse("coding ready mismatch");
                    }
                    let session = frame["sessionId"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| Error::Refused("native session absent".into()))?;
                    if let Some(owner) = &self.pi_owner {
                        owner
                            .check_native(session)
                            .map_err(|e| Error::Refused(e.into()))?;
                    }
                    self.journal.session_id = Some(session.into());
                    self.journal.coding.as_mut().unwrap().ready_observed = true;
                    self.save()?;
                    if self.journal.job.coding.as_ref().unwrap().schema == BRIDGE_PROFILE {
                        replies.extend(self.coding_bridge_frame(&frame)?);
                        self.journal.coding.as_mut().unwrap().frames += 1;
                        self.save()?;
                        continue;
                    }
                    let profile = self.journal.job.coding.as_ref().unwrap();
                    let reply = if let Some(plan) = &profile.provider {
                        let selected = plan.selected()?;
                        for key in [
                            "id",
                            "provider",
                            "api",
                            "contextWindow",
                            "maxTokens",
                            "cost",
                        ] {
                            if frame["model"][key] != selected[key] {
                                return refuse("native selected catalog model differs");
                            }
                        }
                        json!({"type":"task_start","sessionId":session})
                    } else {
                        json!({"type":"fixture_start","sessionId":session,"command":profile.fixture_command})
                    };
                    let packet: Value = serde_json::from_slice(&self.journal.job.packet.verify()?)
                        .unwrap_or(Value::Null);
                    if packet["schema"] == PRESSURE_PROFILE {
                        let packet: PressurePacket = serde_json::from_value(packet)?;
                        packet.validate(profile)?;
                        replies.push(serde_json::to_vec(
                            &json!({"type":"pressure_context","sessionId":session}),
                        )?);
                    } else {
                        replies.push(serde_json::to_vec(&reply)?);
                    }
                }
                "pressure_context" => {
                    let profile = self.journal.job.coding.as_ref().unwrap();
                    let packet: PressurePacket =
                        serde_json::from_slice(&self.journal.job.packet.verify()?)?;
                    packet.validate(profile)?;
                    let dir = self.journal.job.state_dir.clone();
                    if dir.join("coding-pressure-ready.json").exists()
                        || frame["snapshot"]["sessionId"] != frame["sessionId"]
                        || frame["snapshot"]["idle"] != true
                    {
                        return refuse("pressure readiness conflict");
                    }
                    let barrier = json!({"schema":PRESSURE_PROFILE,"input_digest":self.journal.input_digest,
                        "container_id":self.journal.container_id,"sessionId":frame["sessionId"],"snapshot":frame["snapshot"]});
                    atomic(&dir.join("coding-pressure-ready.json"), &barrier)?;
                    let path = dir.join("coding-pressure-release.json");
                    while !path.exists() {
                        if self.expired() || self.cancelled() {
                            return refuse("pressure release expired/cancelled");
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    if fs::metadata(&path)?.len() > 32768 {
                        return refuse("pressure release size");
                    }
                    let release: PressureRelease = serde_json::from_slice(&fs::read(path)?)?;
                    let budget = String::from_utf8(packet.budget.verify()?)
                        .map_err(|_| Error::Refused("budget UTF8".into()))?;
                    validate_pressure_release(
                        &packet,
                        &release,
                        &barrier,
                        &budget,
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(|_| Error::Refused("host clock".into()))?
                            .as_millis()
                            .try_into()
                            .map_err(|_| Error::Refused("host clock range".into()))?,
                        self.expired() || self.cancelled(),
                    )?;
                    atomic(&dir.join("coding-pressure-admitted.json"), &release)?;
                    replies.push(serde_json::to_vec(
                        &json!({"type":"fixture_start","sessionId":frame["sessionId"],
                        "command":profile.fixture_command,"pressureCheckpoint":release}),
                    )?);
                }
                "pressure_inserted" | "pressure_request" => {
                    let dir = &self.journal.job.state_dir;
                    let packet: PressurePacket =
                        serde_json::from_slice(&self.journal.job.packet.verify()?)?;
                    packet.validate(self.journal.job.coding.as_ref().unwrap())?;
                    let admitted: PressureRelease = serde_json::from_slice(&fs::read(
                        dir.join("coding-pressure-admitted.json"),
                    )?)?;
                    let barrier: Value =
                        serde_json::from_slice(&fs::read(dir.join("coding-pressure-ready.json"))?)?;
                    let destination = dir.join(if kind == "pressure_inserted" {
                        "coding-pressure-message.json"
                    } else {
                        "coding-pressure-request.json"
                    });
                    let messages = if kind == "pressure_inserted" {
                        &frame["messages"]
                    } else {
                        &frame["context"]["messages"]
                    };
                    let present = messages.as_array().is_some_and(|messages| {
                        messages.iter().any(|m| {
                            m["content"].as_array().is_some_and(|content| {
                                content
                                    .iter()
                                    .any(|c| c["type"] == "text" && c["text"] == admitted.pressure)
                            })
                        })
                    });
                    if destination.exists()
                        || !present
                        || frame["generation"] != admitted.generation
                        || frame["input_digest"] != admitted.input_digest
                        || (kind == "pressure_request"
                            && (frame["requestModel"] != barrier["snapshot"]["model"]
                                || !dir.join("coding-pressure-message.json").exists()))
                    {
                        return refuse("pressure native evidence conflict");
                    }
                    atomic(&destination, &frame)?;
                }
                "tool_request" => {
                    if self.expired() || self.cancelled() {
                        return refuse("coding tool admission expired/cancelled");
                    }
                    let id = frame["id"]
                        .as_str()
                        .filter(|s| !s.is_empty() && s.len() < 128)
                        .ok_or_else(|| Error::Refused("tool id absent".into()))?
                        .to_string();
                    let state = self.journal.coding.as_mut().unwrap();
                    if state.tool_requests.contains_key(&id)
                        || state.tool_requests.len()
                            >= self.journal.job.coding.as_ref().unwrap().max_tool_calls as usize
                    {
                        return refuse("duplicate or excess tool request");
                    }
                    let command = frame["command"]
                        .as_str()
                        .filter(|s| s.len() <= 8192)
                        .ok_or_else(|| Error::Refused("tool command bound".into()))?
                        .to_string();
                    state.tool_requests.insert(id.clone(), None);
                    self.save()?; // intent precedes exec; None stays unknown on interruption
                    self.coding_inspect_tool()?;
                    let effect_lease =
                        self.pi_lease(session_task_admission::pi_pilot::Effect::Tool(id.clone()))?;
                    if let Some(lease) = &effect_lease {
                        lease
                            .check_current()
                            .map_err(|e| Error::Refused(e.into()))?;
                    }
                    let intent = CodingToolIntent {
                        job_id: self.journal.job.job_id.clone(),
                        input_digest: self.journal.input_digest.clone(),
                        session_id: self.journal.session_id.clone().unwrap(),
                        call_id: id.clone(),
                        command_sha256: digest(command.as_bytes()),
                    };
                    let mut gate = self.coding_gate.take();
                    let mut entered = false;
                    let executed = {
                        let mut actuator = |deadline_ms| {
                            if std::mem::replace(&mut entered, true) {
                                return refuse("coding actuator already consumed");
                            }
                            self.coding_gate_deadline_ms = Some(deadline_ms);
                            self.command_with_lease(
                                "coding-tool-exec",
                                vec![
                                    "exec".into(),
                                    "--workdir".into(),
                                    "/workspace".into(),
                                    tool_name(&self.journal),
                                    "/bin/sh".into(),
                                    "-c".into(),
                                    command.clone(),
                                ],
                                false,
                                effect_lease.as_ref(),
                            )
                        };
                        if let Some(gate) = &mut gate {
                            gate.execute(&intent, &mut actuator)
                        } else {
                            actuator(i64::MAX)
                        }
                    };
                    self.coding_gate_deadline_ms = None;
                    self.coding_gate = gate;
                    if executed.is_ok() && !entered {
                        return refuse("coding gate returned without actuator evidence");
                    }
                    let (ok, output) = executed?;
                    if let Some(lease) = effect_lease {
                        lease
                            .observed(&digest(&output))
                            .map_err(|e| Error::Refused(e.into()))?;
                    }
                    let process = self
                        .journal
                        .processes
                        .last()
                        .ok_or_else(|| Error::Refused("tool process absent".into()))?;
                    let stderr = fs::read(&process.stderr)?;
                    if output.len().saturating_add(stderr.len()) > 60000 {
                        return refuse("tool output exceeded frame bound");
                    }
                    let combined = format!(
                        "Host command success: {ok}\n{}\n[stderr]\n{}",
                        String::from_utf8_lossy(&output),
                        String::from_utf8_lossy(&stderr)
                    );
                    let result = json!({"type":"tool_result","id":id,"ok":ok,"output":combined});
                    let path = self
                        .journal
                        .job
                        .state_dir
                        .join(format!("coding-result-{}.json", digest(id.as_bytes())));
                    atomic(&path, &result)?;
                    let artifact = Artifact {
                        sha256: file_digest(&path, None)?,
                        path,
                    };
                    self.journal
                        .coding
                        .as_mut()
                        .unwrap()
                        .tool_requests
                        .insert(id, Some(artifact.clone()));
                    self.save()?;
                    if let Some(gate) = &mut self.coding_gate {
                        let state = self.journal.coding.as_ref().unwrap();
                        gate.observed(
                            &intent,
                            &artifact,
                            self.journal.processes.last().unwrap(),
                            state.tool_id.as_deref().ok_or_else(|| {
                                Error::Refused("tool container identity absent".into())
                            })?,
                        )?;
                    }
                    replies.push(serde_json::to_vec(&result)?);
                }
                "coding_provider_request" | "coding_history" | "coding_replaced" => {
                    if self.journal.job.coding.as_ref().unwrap().schema != BRIDGE_PROFILE {
                        return refuse("coding lifecycle frame in legacy profile");
                    }
                    replies.extend(self.coding_bridge_frame(&frame)?);
                    if kind == "coding_replaced" {
                        self.journal.session_id = Some(
                            frame["sessionId"]
                                .as_str()
                                .ok_or_else(|| {
                                    Error::Refused("replacement native identity".into())
                                })?
                                .into(),
                        );
                    }
                }
                "native_event" => {
                    if frame["event"]["type"] == "tool_execution_end" {
                        let event = &frame["event"];
                        let id = event["toolCallId"]
                            .as_str()
                            .ok_or_else(|| Error::Refused("native tool id absent".into()))?;
                        let state = self.journal.coding.as_mut().unwrap();
                        if event["toolName"] != "workspace_exec"
                            || event["isError"] != false
                            || !state.tool_requests.get(id).is_some_and(Option::is_some)
                            || !state.native_tool_ids.insert(id.into())
                        {
                            return refuse("native tool completion mismatch");
                        }
                        state.native_tool_ends += 1;
                    }
                    if self.journal.job.coding.as_ref().unwrap().schema == BRIDGE_PROFILE {
                        replies.extend(self.coding_bridge_frame(&frame)?);
                    }
                }
                "complete" => {
                    if self.journal.job.coding.as_ref().unwrap().schema == BRIDGE_PROFILE
                        && frame["counts"] != json!({"providerRequests":4,"resets":1})
                    {
                        return refuse("coding bridge counts mismatch");
                    }
                    let state = self.journal.coding.as_mut().unwrap();
                    if state.completed
                        || state.tool_requests.is_empty()
                        || state.tool_requests.values().any(Option::is_none)
                        || state.native_tool_ends != state.tool_requests.len() as u32
                    {
                        return refuse("coding completion lacks native tool evidence");
                    }
                    if (self.journal.job.coding.as_ref().unwrap().provider.is_some()
                        || self.journal.job.coding.as_ref().unwrap().schema == SIGNED_PROFILE)
                        && (state.provider_failed
                            || state.provider_requests.is_empty()
                            || state
                                .provider_requests
                                .values()
                                .any(|r| r.settled_microusd.is_none()))
                    {
                        return refuse("coding provider completion unestablished");
                    }
                    state.completed = true;
                }
                _ => return refuse("unknown coding frame"),
            }
            self.journal.coding.as_mut().unwrap().frames += 1;
            self.save()?;
        }
        for reply in &mut replies {
            if self.signed_pi() {
                let mut value: Value = serde_json::from_slice(reply)?;
                value["sessionId"] = json!(self.journal.session_id);
                *reply = serde_json::to_vec(&value)?;
            }
            reply.push(b'\n');
        }
        Ok(replies)
    }
    pub(super) fn coding_complete(&mut self, raw: &[u8]) -> Result<()> {
        if !raw.ends_with(b"\n")
            || !self.coding_advance(raw)?.is_empty()
            || !self.journal.coding.as_ref().unwrap().completed
        {
            return refuse("coding incomplete tail/state");
        }
        let path = self
            .journal
            .job
            .state_dir
            .join("exports/coding-native-history.json");
        let history: Value = serde_json::from_slice(&fs::read(path)?)?;
        if history["sessionId"].as_str() != self.journal.session_id.as_deref()
            || history["messages"].as_array().is_none_or(Vec::is_empty)
        {
            return refuse("native coding history absent");
        }
        Ok(())
    }
    pub(super) fn coding_collect_tool(&mut self) -> Result<()> {
        if self.journal.coding.as_ref().unwrap().tool_removed
            || !self.journal.coding.as_ref().unwrap().tool_create_intent
        {
            return Ok(());
        }
        let mut v = self.coding_inspect_tool()?;
        let name = tool_name(&self.journal);
        if v["State"]["Running"] == true {
            let (ok, _) = self.command(
                "coding-tool-stop",
                vec!["stop".into(), "--time".into(), "1".into(), name.clone()],
                false,
            )?;
            if !ok {
                return refuse("tool stop uncertain");
            }
            v = self.coding_inspect_tool()?;
        }
        if v["State"]["Running"] != false || v["State"]["OOMKilled"] != false {
            return refuse("tool terminal state unknown");
        }
        if v["State"]["Status"] == "exited" {
            let (ok, bytes) =
                self.command("coding-tool-wait", vec!["wait".into(), name.clone()], false)?;
            let exit = String::from_utf8_lossy(&bytes)
                .trim()
                .parse::<i64>()
                .map_err(|_| Error::Refused("tool wait malformed".into()))?;
            if !ok || v["State"]["ExitCode"] != exit {
                return refuse("tool exit mismatch");
            }
            self.journal.coding.as_mut().unwrap().tool_exit = Some(exit);
            self.save()?;
        } else if v["State"]["Status"] != "created" {
            return refuse("tool terminal state invalid");
        }
        let (ok, _) = self.command("coding-tool-remove", vec!["rm".into(), name.clone()], false)?;
        if !ok {
            return refuse("tool remove uncertain");
        }
        let (ok, output) = self.command(
            "coding-tool-absence",
            vec![
                "ps".into(),
                "--all".into(),
                "--no-trunc".into(),
                "--filter".into(),
                format!("name=^/{name}$"),
                "--format".into(),
                "{{.ID}}".into(),
            ],
            false,
        )?;
        if !ok || !output.iter().all(u8::is_ascii_whitespace) {
            return refuse("tool absence unestablished");
        }
        self.journal.coding.as_mut().unwrap().tool_removed = true;
        self.save()?;
        let profile = self.journal.job.coding.as_ref().unwrap();
        profile.toolchain.verify()?;
        profile.acceptance.verify()?;
        if let Some(plan) = &profile.provider {
            let expected: BTreeMap<String, Entry> =
                serde_json::from_slice(&plan.lane.pi_config.manifest.verify()?)?;
            if inventory(&self.journal.job.state_dir.join("coding-public-config"))? != expected {
                return refuse("public lane config realization changed");
            }
            plan.catalog.verify()?;
            plan.task.verify()?;
            plan.access_adapter.verify()?;
        }
        self.save()
    }
    pub(super) fn coding_confirm_inference_absence(&mut self) -> Result<bool> {
        let name = self.journal.container_name.clone();
        let (ok, output) = self.command(
            "coding-inference-absence",
            vec![
                "ps".into(),
                "--all".into(),
                "--no-trunc".into(),
                "--filter".into(),
                format!("name=^/{name}$"),
                "--format".into(),
                "{{.ID}}".into(),
            ],
            false,
        )?;
        Ok(ok && output.iter().all(u8::is_ascii_whitespace))
    }
    pub(super) fn coding_cleanup_secret(&mut self) -> Result<()> {
        if !self.journal.removed {
            return Ok(());
        }
        if self.signed_pi() && !self.signed_subscription() {
            let private = self.journal.job.state_dir.join("coding-canary");
            for name in ["access.json", "signed-fixture.json"] {
                let p = private.join(name);
                if p.try_exists()? {
                    fs::remove_file(p)?;
                }
            }
            if fs::read_dir(&private)?.next().is_some() {
                return refuse("fixture access directory not empty");
            }
            File::open(&private)?.sync_all()?;
            self.journal.coding.as_mut().unwrap().access_removed = true;
            return self.save();
        }
        if self.journal.job.coding.as_ref().unwrap().provider.is_none() {
            return Ok(());
        }
        let private = self.journal.job.state_dir.join("private-access");
        let secret = private.join("access.json");
        if secret.try_exists()? {
            fs::remove_file(secret)?;
            File::open(&private)?.sync_all()?;
        }
        self.journal.coding.as_mut().unwrap().access_removed = true;
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FixtureGate;
    impl CodingEffectGate for FixtureGate {
        fn execute(
            &mut self,
            _: &CodingToolIntent,
            action: &mut dyn FnMut(i64) -> Result<(bool, Vec<u8>)>,
        ) -> Result<(bool, Vec<u8>)> {
            action(1)
        }
        fn observed(
            &mut self,
            _: &CodingToolIntent,
            _: &Artifact,
            _: &ProcessRecord,
            _: &str,
        ) -> Result<()> {
            Ok(())
        }
    }
    #[test]
    fn lifecycle_gate_is_opt_in_and_expired_admission_cannot_spawn() {
        let (dir, mut h) = fixture();
        assert!(h.bind_coding_effect_gate(Box::new(FixtureGate)).is_err());
        h.journal.job.coding.as_mut().unwrap().schema = BRIDGE_PROFILE.into();
        assert!(h.run().is_err());
        assert!(h.journal.processes.is_empty());
        h.bind_coding_effect_gate(Box::new(FixtureGate)).unwrap();
        assert!(h.bind_coding_effect_gate(Box::new(FixtureGate)).is_err());
        h.journal.job.docker = Artifact {
            path: "/bin/sh".into(),
            sha256: digest(&fs::read("/bin/sh").unwrap()),
        };
        h.coding_gate_deadline_ms = Some(1);
        let marker = dir.path().join("must-not-enter");
        let result = h.command(
            "coding-tool-exec",
            vec!["-c".into(), format!("touch {}", marker.display())],
            false,
        );
        assert!(
            matches!(result, Err(Error::Refused(ref s)) if s.contains("dispatch expired/cancelled/unadmitted"))
        );
        assert!(!marker.exists());
        assert!(h.journal.processes.iter().all(|p| p.pid.is_none()));
    }
    pub(super) fn fixture() -> (tempfile::TempDir, Host) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let pin = Artifact {
            path: root.join("unused"),
            sha256: "0".repeat(64),
        };
        let tree = Tree {
            root: root.join("unused-tree"),
            manifest: pin.clone(),
        };
        // Ready dispatch now reads the admitted packet to recognize an explicit
        // pressure fixture; this fake Host must supply a genuine no-pressure pin.
        let packet_path = root.join("packet.json");
        fs::write(&packet_path, b"{}").unwrap();
        let packet = Artifact {
            path: packet_path,
            sha256: digest(b"{}"),
        };
        let job = Job {
            schema: SCHEMA.into(),
            job_id: "test".into(),
            state_dir: root.into(),
            authority: pin.clone(),
            packet,
            docker: pin.clone(),
            image: IMAGE.into(),
            runtime: tree.clone(),
            workspace: tree.clone(),
            deadline_unix_seconds: now() + 60,
            command_timeout_seconds: 1,
            idle_seconds: 0,
            receiver: None,
            coding: Some(Profile {
                schema: PROFILE.into(),
                shim: pin.clone(),
                toolchain: tree,
                acceptance: pin,
                fixture_command: "unused".into(),
                max_tool_calls: 1,
                workspace_write_boundary: WorkspaceWriteBoundary::LegacyWorkspace,
                provider: None,
            }),
        };
        let journal = Journal {
            schema: SCHEMA.into(),
            binary_sha256: "0".repeat(64),
            input_digest: "1".repeat(64),
            container_name: "test".into(),
            job,
            container_id: None,
            session_id: None,
            phase: Phase::Admitted,
            processes: vec![],
            failures: vec![],
            rpc_verified: false,
            receiver: None,
            coding: Some(State::default()),
            rpc_completion: None,
            daemon_exit: None,
            removed: false,
            artifacts: vec![],
            exported_files: BTreeMap::new(),
            receipt_id: None,
            acknowledged: BTreeSet::new(),
        };
        let host = Host {
            journal,
            coding_gate: None,
            coding_gate_deadline_ms: None,
            pi_owner: None,
            allocation_lease: None,
            reply_leases: vec![],
            _lock: File::create(root.join("lock")).unwrap(),
        };
        (dir, host)
    }
    #[test]
    fn strict_workspace_mounts_allow_only_regular_source_and_keep_legacy_wire() {
        let (dir, mut h) = fixture();
        let profile = h.journal.job.coding.as_mut().unwrap();
        let legacy = serde_json::to_value(&*profile).unwrap();
        assert!(legacy.get("workspace_write_boundary").is_none());
        let decoded: Profile = serde_json::from_value(legacy).unwrap();
        assert_eq!(
            decoded.workspace_write_boundary,
            WorkspaceWriteBoundary::LegacyWorkspace
        );
        let mounts = expected_mounts(&h.journal, true, false);
        assert!(
            mounts
                .iter()
                .any(|(_, dst, rw)| *dst == "/workspace" && *rw)
        );
        h.journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .workspace_write_boundary = WorkspaceWriteBoundary::SrcLibRs;
        let mounts = expected_mounts(&h.journal, true, false);
        assert_eq!(mounts.iter().filter(|(_, _, rw)| *rw).count(), 1);
        assert!(
            mounts
                .iter()
                .any(|(_, dst, rw)| *dst == "/workspace" && !*rw)
        );
        assert!(
            mounts
                .iter()
                .any(|(src, dst, rw)| *dst == "/workspace/src/lib.rs"
                    && *rw
                    && *src == dir.path().join("coding-workspace/src/lib.rs"))
        );
        let workspace = dir.path().join("coding-workspace");
        fs::create_dir_all(workspace.join("src")).unwrap();
        fs::write(workspace.join("src/lib.rs"), "original").unwrap();
        strict_source_file(&workspace).unwrap();
        std::fs::hard_link(workspace.join("src/lib.rs"), workspace.join("alias")).unwrap();
        assert!(strict_source_file(&workspace).is_err());
        fs::remove_file(workspace.join("alias")).unwrap();
        fs::remove_file(workspace.join("src/lib.rs")).unwrap();
        std::os::unix::fs::symlink("/tmp/foreign", workspace.join("src/lib.rs")).unwrap();
        assert!(strict_source_file(&workspace).is_err());
        fs::remove_file(workspace.join("src/lib.rs")).unwrap();
        fs::remove_dir(workspace.join("src")).unwrap();
        std::os::unix::fs::symlink(dir.path(), workspace.join("src")).unwrap();
        assert!(strict_source_file(&workspace).is_err());
    }
    #[test]
    fn strict_validator_has_no_writable_workspace_or_source_mount() {
        let (_dir, mut h) = fixture();
        h.journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .workspace_write_boundary = WorkspaceWriteBoundary::SrcLibRs;
        h.journal.coding.as_mut().unwrap().validation_snapshot =
            Some(h.journal.job.workspace.clone());
        let mounts = expected_mounts(&h.journal, true, true);
        assert!(mounts.iter().all(|(_, _, rw)| !*rw));
        assert!(
            !mounts
                .iter()
                .any(|(_, dst, _)| *dst == "/workspace/src/lib.rs")
        );
    }
    fn frame(value: Value) -> Vec<u8> {
        let mut raw = serde_json::to_vec(&value).unwrap();
        raw.push(b'\n');
        raw
    }
    #[test]
    fn tool_requests_require_native_session_and_never_replay_unknown_intent() {
        let (_dir, mut h) = fixture();
        let request = frame(
            json!({"schema":PROFILE,"type":"tool_request","id":"one","command":"must never run"}),
        );
        assert!(h.coding_advance(&request).is_err());
        h.journal.session_id = Some("native".into());
        let request = frame(
            json!({"schema":PROFILE,"sessionId":"foreign","type":"tool_request","id":"one","command":"must never run"}),
        );
        assert!(h.coding_advance(&request).is_err());
        h.journal
            .coding
            .as_mut()
            .unwrap()
            .tool_requests
            .insert("one".into(), None);
        let request = frame(
            json!({"schema":PROFILE,"sessionId":"native","type":"tool_request","id":"one","command":"must never run"}),
        );
        assert!(h.coding_advance(&request).is_err());
        assert!(h.journal.processes.is_empty());
        assert!(h.journal.coding.as_ref().unwrap().tool_requests["one"].is_none());
    }
    #[test]
    fn native_completion_cannot_replace_missing_or_foreign_tool_evidence() {
        let (_dir, mut h) = fixture();
        h.journal.session_id = Some("native".into());
        for event in [
            json!({"type":"tool_execution_end","toolCallId":"foreign","toolName":"workspace_exec","isError":false}),
            json!({"type":"tool_execution_end","toolCallId":"one","toolName":"bash","isError":false}),
        ] {
            assert!(h.coding_advance(&frame(json!({"schema":PROFILE,"sessionId":"native","type":"native_event","event":event}))).is_err());
        }
        assert!(
            h.coding_advance(&frame(
                json!({"schema":PROFILE,"sessionId":"native","type":"complete"})
            ))
            .is_err()
        );
        assert!(h.journal.processes.is_empty());
    }
    #[test]
    fn readiness_refuses_extra_builtin_tools_and_duplicate_session() {
        let (_dir, mut h) = fixture();
        assert!(h.coding_advance(&frame(json!({"schema":PROFILE,"sessionId":"native","type":"ready","tools":["workspace_exec","bash"]}))).is_err());
        let ready = frame(
            json!({"schema":PROFILE,"sessionId":"native","type":"ready","tools":["workspace_exec"]}),
        );
        assert_eq!(h.coding_advance(&ready).unwrap().len(), 1);
        let mut duplicate = ready.clone();
        duplicate.extend(&ready);
        assert!(h.coding_advance(&duplicate).is_err());
        assert!(h.journal.processes.is_empty());
    }
    #[test]
    fn signed_fixture_dispatch_and_replacement_fail_before_effects() {
        let (_dir, mut h) = fixture();
        h.journal.job.coding.as_mut().unwrap().schema = SIGNED_PROFILE.into();
        h.journal.job.deadline_unix_seconds = now().saturating_sub(1);
        assert!(
            h.command("coding-tool-exec", vec!["must-never-execute".into()], false)
                .is_err()
        );
        assert!(h.journal.processes.iter().all(|p| p.pid.is_none()));
        h.journal.session_id = Some("00000000-0000-4000-8000-000000000001".into());
        assert!(h.coding_advance(&frame(json!({"schema":SIGNED_PROFILE,"type":"allocated","sessionId":"00000000-0000-4000-8000-000000000002","accessPresent":false}))).is_err());
        assert!(h.coding_advance(&frame(json!({"schema":SIGNED_PROFILE,"type":"ready","sessionId":"00000000-0000-4000-8000-000000000002","tools":["workspace_exec"]}))).is_err());
        assert!(h.journal.processes.iter().all(|p| p.pid.is_none()));
    }
    #[test]
    fn signed_subscription_cannot_allocate_or_spawn_without_owner_or_after_deadline() {
        let (_dir, mut h) = fixture();
        h.journal.job.coding.as_mut().unwrap().schema = SIGNED_SUBSCRIPTION_PROFILE.into();
        assert!(Host::admit(h.journal.job.clone()).is_err());
        assert!(h.coding_advance(&frame(json!({"schema":SIGNED_SUBSCRIPTION_PROFILE,"type":"allocated","sessionId":"00000000-0000-4000-8000-000000000001","accessPresent":false}))).is_err());
        assert!(h.journal.session_id.is_none());
        h.journal.job.deadline_unix_seconds = now().saturating_sub(1);
        assert!(
            h.command("coding-tool-exec", vec!["must-never-execute".into()], false)
                .is_err()
        );
        assert!(h.journal.processes.iter().all(|p| p.pid.is_none()));
        assert!(
            !h.journal
                .job
                .state_dir
                .join("private-access/access.json")
                .exists()
        );
    }
    #[test]
    #[ignore = "explicit signed authority generator required; no Docker or provider"]
    fn signed_tool_authority_expires_before_job_and_prevents_spawn() {
        let (dir, mut h) = fixture();
        h.journal.job.deadline_unix_seconds = now() + 180;
        let generator = std::env::var_os("PI_CODING_GENERATOR").expect("pinned generator");
        let authority = dir.path().join("signed-authority");
        let job_sha = digest(&serde_json::to_vec(&h.journal.job).unwrap());
        let exe_sha = digest(&fs::read(std::env::current_exe().unwrap()).unwrap());
        let status = std::process::Command::new(generator)
            .args([
                "--ignored",
                "--exact",
                "pi_pilot::tests::export_signed_pi_fixture",
            ])
            .env("PI_FIXTURE_EXPORT", &authority)
            .env("PI_FIXTURE_JOB_SHA", &job_sha)
            .env("PI_FIXTURE_EXECUTOR_SHA", &exe_sha)
            .env("PI_FIXTURE_LIFETIME_SECONDS", "45")
            .status()
            .unwrap();
        assert!(status.success());
        let pin = serde_json::from_slice(&fs::read(authority.join("descriptor-pin.json")).unwrap())
            .unwrap();
        let owner = session_task_admission::pi_pilot::PiOwner::open(pin).unwrap();
        owner.admit(&job_sha, &exe_sha).unwrap();
        drop(
            owner
                .enter(session_task_admission::pi_pilot::Effect::Allocation)
                .unwrap(),
        );
        owner
            .seal(&job_sha, "00000000-0000-4000-8000-000000000001")
            .unwrap();
        let lease = owner
            .enter(session_task_admission::pi_pilot::Effect::Tool(
                "expiry".into(),
            ))
            .unwrap();
        lease.check_current().unwrap();
        let clock: Value =
            serde_json::from_slice(&fs::read(authority.join("export-clock.json")).unwrap())
                .unwrap();
        // Cross the signed deadline after acquiring valid authority, while the
        // independently pinned job deadline remains well in the future.
        while now() <= clock["profile_expires_unix"].as_u64().unwrap() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            !h.expired(),
            "job remains valid after signed profile expiry"
        );
        let err = h
            .command_with_lease(
                "coding-tool-exec",
                vec!["must-never-execute".into()],
                false,
                Some(&lease),
            )
            .unwrap_err();
        assert!(err.to_string().contains("pi_lease_expired"), "{err}");
        assert_eq!(h.journal.processes.len(), 1, "journal preparation occurred");
        assert!(h.journal.processes[0].stdout.exists());
        assert!(h.journal.processes[0].stderr.exists());
        assert!(h.journal.processes[0].pid.is_none(), "no child started");
    }
}
