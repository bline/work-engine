//! Provider-disabled coding-path qualification in the existing Pi custody owner.
//! This profile intentionally cannot authorize provider or credential effects.
use super::*;
#[path = "pi_coding_analysis.rs"]
pub mod analysis;
#[path = "pi_coding_claims.rs"]
pub mod claims;
#[path = "pi_coding_export.rs"]
pub mod export;
#[path = "pi_coding_hosted.rs"]
pub mod hosted;
#[path = "pi_coding_https.rs"]
pub mod https;
#[path = "pi_coding_provider.rs"]
pub mod provider;
#[path = "pi_coding_validation.rs"]
pub mod validation;
use std::os::unix::fs::PermissionsExt;
pub const SIGNED_PROFILE: &str = "work-engine.pi-coding-signed-fixture.v1";
pub const PROFILE: &str = "work-engine.pi-coding-fixture.v1";
/// Opt-in composition: Host owns effects; an installed lifecycle gate owns entry.
pub const BRIDGE_PROFILE: &str = "work-engine.pi-coding-lifecycle-fixture.v1";
/// Retired credential-mounted live schemas (the container held provider
/// access). Named only so admission can refuse them explicitly.
pub const RETIRED_LIVE_PROFILES: [&str; 4] = [
    "work-engine.pi-coding-sdk.v1",
    "work-engine.pi-coding-lifecycle-live.v1",
    "work-engine.pi-coding-lifecycle-sol-high.v1",
    "work-engine.pi-coding-signed-subscription.v1",
];
/// Live lifecycle composition with Host-held access and Host dispatch: the
/// inference container has no network and no credential mount.
pub const HOSTED_BRIDGE_PROFILE: &str = "work-engine.pi-coding-hosted.v1";
/// Hosted analysis: the same Host-held access, dispatch and lifecycle gate,
/// no shell, tool container or workspace effect; the Host serves read-only
/// tools over the job's manifest-pinned workspace (`analysis`).
pub const ANALYSIS_PROFILE: &str = "work-engine.pi-analysis-hosted.v1";
/// The exact tool set the approved pi-better-background-tasks 0.18.0 plugin
/// registers (background process management). When allow_background is admitted
/// the ready frame must advertise exactly these names; the receiver maintains
/// the same list. This is a tool-set contract, not per-call execution approval.
pub const BG_TOOLS: [&str; 13] = [
    "bg_task_spawn",
    "bg_task_spawn_batch",
    "bg_task_watch",
    "bg_task_list",
    "bg_task_status",
    "bg_task_log",
    "bg_task_stop",
    "bg_task",
    "bg_status",
    "wait_for_operations",
    "acknowledge_operation_result",
    "bg_task_wait_cancel",
    "bg_task_wait_list",
];
/// The live profiles: Host-held access, Host dispatch, lifecycle gate.
pub fn is_live_bridge(schema: &str) -> bool {
    is_hosted(schema)
}
/// Lifecycle relay (Pi Host realization, `native-realization` `pi_host`):
/// the analysis profile's Host-held access, dispatch and no-effect surface,
/// with the pinned lifecycle tools relayed to the controller instead of
/// served by the Host.
pub const LIFECYCLE_PROFILE: &str = "work-engine.pi-lifecycle-hosted.v1";
/// Finite per-thread bound on relayed tool calls.
pub const MAX_RELAYS: u32 = 256;
/// Upper bound of a relay thread's job deadline (H4: above the 300-second
/// job cap of the other profiles).
pub const RELAY_DEADLINE_SECONDS: u64 = 3600;
pub fn is_hosted(schema: &str) -> bool {
    schema == HOSTED_BRIDGE_PROFILE || is_analysis(schema)
}
/// The no-effect hosted family: Host-served (analysis) or relayed tools.
pub fn is_analysis(schema: &str) -> bool {
    schema == ANALYSIS_PROFILE || is_relay(schema)
}
pub fn is_relay(schema: &str) -> bool {
    schema == LIFECYCLE_PROFILE
}
/// The live coding continuity profile (one shell call, one reset).
fn is_continuity(schema: &str) -> bool {
    schema == HOSTED_BRIDGE_PROFILE
}
/// Opt-in, versioned fixture composition that additionally admits the four
/// read-only claims tools before and after SDK context replacement. Provider-
/// disabled; the existing v1-v4 schemas cannot select it and it cannot select
/// them. Trusted claims configuration is bound only through the installed gate.
pub const CLAIMS_BRIDGE_PROFILE: &str = "work-engine.pi-coding-lifecycle-claims.v1";
/// Finite per-job bound on read-only claims calls (not a workspace effect).
pub const MAX_CLAIMS_CALLS: usize = 12;
pub(super) fn is_bridge(schema: &str) -> bool {
    schema == BRIDGE_PROFILE || is_live_bridge(schema) || schema == CLAIMS_BRIDGE_PROFILE
}
fn is_fixture_bridge(schema: &str) -> bool {
    schema == BRIDGE_PROFILE || schema == CLAIMS_BRIDGE_PROFILE
}
/// The advertised tool set must equal the profile's exact set (order-free for
/// the claims profile, whose SDK registry order is not part of the contract).
fn tools_match(advertised: &Value, schema: &str, allow_shell: bool, allow_background: bool) -> bool {
    let sorted = |v: &Value| -> Option<Vec<String>> {
        let mut names: Vec<String> = v
            .as_array()?
            .iter()
            .map(|n| n.as_str().map(String::from))
            .collect::<Option<_>>()?;
        names.sort();
        Some(names)
    };
    let expected = ready_tools(schema, allow_shell, allow_background);
    if schema == CLAIMS_BRIDGE_PROFILE || is_analysis(schema) {
        sorted(advertised).is_some() && sorted(advertised) == sorted(&expected)
    } else {
        *advertised == expected
    }
}
/// The advertised native tool set for a profile's ready frame. For the live
/// coding profile this is the workspace shell tool only when shell is admitted
/// (`allow_shell`); without it (e.g. the lifecycle relay remap) no execution
/// tool is advertised (Revision 4).
fn ready_tools(schema: &str, allow_shell: bool, allow_background: bool) -> Value {
    if is_relay(schema) {
        json!(native_realization::pi_host::tools::names())
    } else if is_analysis(schema) {
        json!(analysis::TOOLS)
    } else if schema == CLAIMS_BRIDGE_PROFILE {
        json!(
            ["workspace_exec"]
                .into_iter()
                .chain(claims::TOOLS)
                .collect::<Vec<_>>()
        )
    } else if is_live_bridge(schema) && !allow_shell {
        // The hosted coding lane: no execution tool when shell is not
        // admitted (Revision 4). Fixture surfaces (PROFILE, BRIDGE_PROFILE)
        // always advertise the workspace shell tool. When background is
        // admitted, the approved plugin's tool set is expected too.
        if allow_background {
            json!(BG_TOOLS)
        } else {
            json!([])
        }
    } else if is_live_bridge(schema) && allow_background {
        // Shell + the approved background plugin's tool set.
        json!(["workspace_exec"].into_iter().chain(BG_TOOLS.iter().copied()).collect::<Vec<_>>())
    } else {
        json!(["workspace_exec"])
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodingToolIntent {
    pub job_id: String,
    pub input_digest: String,
    pub session_id: String,
    pub call_id: String,
    pub command_sha256: String,
}
/// One upstream attempt under a logical hosted provider request.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProviderAttemptIntent {
    pub request: u64,
    pub attempt: u64,
    pub route: String,
    pub payer: String,
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
    /// Read-only claims access composed by the trusted embedder (see
    /// `claims::ClaimsGate`). Absent by default: no claims capability exists
    /// unless the installed gate provides it.
    fn claims(
        &mut self,
        _tool: &str,
        _arguments: &Value,
        _until: std::time::Instant,
    ) -> Result<Value> {
        refuse("coding claims access unavailable")
    }
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
    /// Hosted profiles only: authorize one upstream provider attempt against
    /// the current grant and its bound failover policy, and run `dispatch`
    /// with the attempt deadline (ms) while that authority is held. Once
    /// admitted against the policy, the attempt is consumed even if currentness
    /// or `dispatch` then fails; it is never replayed.
    fn provider_attempt(
        &mut self,
        _intent: &ProviderAttemptIntent,
        _dispatch: &mut dyn FnMut(i64) -> Result<()>,
    ) -> Result<()> {
        refuse("hosted provider attempts unavailable")
    }
    /// Analysis profile only: record one Host snapshot delivery (already
    /// persisted create-only) before its reply is dispatched.
    fn snapshot_delivered(
        &mut self,
        _id: &str,
        _tool: &str,
        _arguments: &Value,
        _result: &Value,
        _artifact: &Artifact,
    ) -> Result<()> {
        refuse("analysis snapshot deliveries unavailable")
    }
    /// Lifecycle relay profile only: relay one Pi tool call to the
    /// controller and return its answer (success, text) unchanged. The intent
    /// is persisted before the call and the result retained after it.
    fn relay(
        &mut self,
        _id: &str,
        _tool: &str,
        _arguments: &Value,
        _until: std::time::Instant,
    ) -> Result<(bool, String)> {
        refuse("lifecycle tool relay unavailable")
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
    /// Coding profiles only (the tool and validator containers); the analysis
    /// profile has neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolchain: Option<Tree>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<Artifact>,
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
    /// Hosted: nonsecret identity of the bound provider transport (credentials
    /// file digest and each payer's access-source kind), recorded at binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted_transport: Option<Value>,
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub claims_requests: BTreeMap<String, Option<Artifact>>,
    /// Requested tool per claims call id, retained so a native completion must
    /// name the same tool (a different claims tool cannot satisfy a request).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub claims_tools: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub native_claims_ids: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub claims_before_replacement: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub claims_after_replacement: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub replaced: bool,
    /// In-band export collection (signed fixture only); absent for every other profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export: Option<export::ExportState>,
    /// Analysis profile: Host snapshot deliveries per call id (`None` while
    /// the read's outcome is unknown), the tool each call named, the calls
    /// whose native completion matched, and SDK-refused calls (never at the Host).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub snapshot_requests: BTreeMap<String, Option<Artifact>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub snapshot_tools: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub native_snapshot_ids: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub sdk_refused_ids: BTreeSet<String>,
}
fn is_zero(v: &u32) -> bool {
    *v == 0
}
impl Profile {
    pub(super) fn validate(&self, job: &Job) -> Result<()> {
        if RETIRED_LIVE_PROFILES.contains(&self.schema.as_str()) {
            return refuse(
                "credential-mounted live profiles are retired; use work-engine.pi-coding-hosted.v1",
            );
        }
        if self.schema
            != if self.provider.is_some() {
                if is_relay(&self.schema) {
                    LIFECYCLE_PROFILE
                } else if is_analysis(&self.schema) {
                    ANALYSIS_PROFILE
                } else {
                    HOSTED_BRIDGE_PROFILE
                }
            } else if self.schema == BRIDGE_PROFILE {
                BRIDGE_PROFILE
            } else if self.schema == CLAIMS_BRIDGE_PROFILE {
                CLAIMS_BRIDGE_PROFILE
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
            // Analysis: `max_tool_calls` bounds the Host snapshot deliveries.
            || !(1..=if is_relay(&self.schema) {
                MAX_RELAYS
            } else if is_analysis(&self.schema) {
                analysis::MAX_READS
            } else {
                8
            })
                .contains(&self.max_tool_calls)
            || (is_bridge(&self.schema)
                && !is_analysis(&self.schema)
                && self.max_tool_calls
                    != if self.schema == CLAIMS_BRIDGE_PROFILE {
                        2
                    } else {
                        1
                    })
            || (is_continuity(&self.schema)
                && self.workspace_write_boundary != WorkspaceWriteBoundary::SrcLibRs)
            || (is_analysis(&self.schema)
                && (self.workspace_write_boundary != WorkspaceWriteBoundary::LegacyWorkspace
                    || self.toolchain.is_some()
                    || self.acceptance.is_some()))
            || (!is_analysis(&self.schema)
                && (self.toolchain.is_none() || self.acceptance.is_none()))
            || job.idle_seconds != 0
        {
            return refuse("coding fixture profile/pins/limits conflict");
        }
        if let Some(plan) = &self.provider {
            plan.validate(job)?;
            // One tool turn, final text, recovery and one continued turn.
            // Each coding_provider_request frame carries the context twice and must
            // fit the receiver's 64 KiB frame bound.
            if is_continuity(&self.schema)
                && (plan.max_requests != 4 || plan.max_context_bytes > 24_000)
            {
                return refuse(
                    "live continuity profile admits exactly 4 requests and a bounded context",
                );
            }
        }
        if self.workspace_write_boundary == WorkspaceWriteBoundary::SrcLibRs {
            strict_source_file(&job.workspace.root)?;
        }
        let (Some(toolchain), Some(acceptance)) = (&self.toolchain, &self.acceptance) else {
            // Analysis: the workspace is the read-only snapshot the Host serves.
            return analysis::validate_snapshot(job);
        };
        toolchain.verify()?;
        acceptance.verify()?;
        for path in [&toolchain.root, &acceptance.path] {
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
        if !toolchain.root.join("bin/rustc").is_file() {
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
    // Every coding container is networkless; the hosted profile's provider
    // traffic is dispatched by the Host (F5).
    let network = "none";
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
            (
                profile
                    .toolchain
                    .as_ref()
                    .expect("coding toolchain")
                    .root
                    .clone(),
                "/toolchain",
                false,
            ),
            (
                profile
                    .acceptance
                    .as_ref()
                    .expect("coding acceptance")
                    .path
                    .clone(),
                "/acceptance.rs",
                false,
            ),
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
    ];
    // The signed fixture's /export is a size-limited tmpfs collected in-band (no host bind).
    if profile.schema != SIGNED_PROFILE {
        mounts.push((j.job.state_dir.join("exports"), "/export", true));
    }
    if let Some(plan) = &profile.provider
        && is_hosted(&profile.schema)
    {
        mounts.extend([
            (
                plan.stream_adapter.path.clone(),
                "/receiver/pi-native-tools.mjs",
                false,
            ),
            (
                j.job.state_dir.join("coding-public-config"),
                "/config",
                false,
            ),
            (j.job.state_dir.join("coding-public"), "/profile", false),
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
                if !tool && j.job.coding.as_ref().is_some_and(|p| is_bridge(&p.schema)) {
                    args.push("--lifecycle-bridge".into());
                    if j.job
                        .coding
                        .as_ref()
                        .is_some_and(|p| p.schema == CLAIMS_BRIDGE_PROFILE)
                    {
                        args.push("--claims-tools".into());
                    }
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
        || v["HostConfig"]["NetworkMode"] != "none"
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
            } else if !validation
                && j.job
                    .coding
                    .as_ref()
                    .is_some_and(|p| p.schema == SIGNED_PROFILE)
            {
                json!({"/tmp":"rw,noexec,nosuid,nodev,size=64m","/export":export::TMPFS.split_once(':').unwrap().1})
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
/// Protected-surface observation of a relay thread's inference container
/// (H5): a fresh `docker inspect` under the job's pinned Docker client,
/// checked by the same containment verifier the Host applies at launch
/// (identity, image, labels, user, no network, read-only root, capabilities,
/// limits, exact mounts and environment), plus `running`. Read-only; it
/// grants nothing. The journal is the Host's durable `pi-state.json`.
pub fn observe_relay_container(state_dir: &Path) -> Result<Value> {
    let j: Journal = serde_json::from_slice(&fs::read(state_dir.join("pi-state.json"))?)?;
    if j.job.coding.as_ref().is_none_or(|p| !is_relay(&p.schema)) || j.container_id.is_none() {
        return refuse("observation outside a launched relay thread");
    }
    if file_digest(&j.job.docker.path, None)? != j.job.docker.sha256 {
        return refuse("docker client pin conflict");
    }
    let output = std::process::Command::new(&j.job.docker.path)
        .args(["inspect", "--type", "container", &j.container_name])
        .stdin(std::process::Stdio::null())
        .output()?;
    if !output.status.success() || output.stdout.len() > 1 << 20 {
        return refuse("container inspection failed");
    }
    let all: Value = serde_json::from_slice(&output.stdout)?;
    let v = match all.as_array().map(Vec::as_slice) {
        Some([v]) => v.clone(),
        _ => return refuse("container inspection is not one container"),
    };
    verify(&j, &v, false, false)?;
    if v["State"]["Running"] != true {
        return refuse("inference container is not running");
    }
    let mounts: Vec<Value> = v["Mounts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|m| json!({"destination":m["Destination"],"rw":m["RW"]}))
        .collect();
    let auth = mounts.iter().any(|m| {
        m["destination"]
            .as_str()
            .is_some_and(|d| d == "/auth" || d.starts_with("/auth/"))
    });
    if auth {
        return refuse("inference container mounts /auth");
    }
    Ok(
        json!({"schema":"work-engine.pi-host-observation.v1","container_id":v["Id"],
        "container_name":j.container_name,"image":v["Config"]["Image"],"running":true,
        "network_mode":v["HostConfig"]["NetworkMode"],"readonly_rootfs":v["HostConfig"]["ReadonlyRootfs"],
        "user":v["Config"]["User"],"mounts":mounts,"auth_mount":false,
        "containment_verified":true,"inspect_sha256":digest(&output.stdout),
        "observed_unix_ms":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64).unwrap_or(0)}),
    )
}
impl Host {
    /// Gate binding is explicit, fresh and non-recoverable. Existing profiles
    /// cannot acquire lifecycle semantics by merely installing a callback.
    pub fn bind_coding_effect_gate(&mut self, gate: Box<dyn CodingEffectGate>) -> Result<()> {
        if self.journal.phase != Phase::Admitted
            || self.journal.job.coding.as_ref().is_none_or(|p| {
                !((is_fixture_bridge(&p.schema) && p.provider.is_none())
                    || (is_live_bridge(&p.schema) && p.provider.is_some()))
            })
            || self.coding_gate.is_some()
        {
            return refuse("coding lifecycle gate binding conflict");
        }
        self.coding_gate = Some(gate);
        Ok(())
    }
    pub(super) fn coding_bridge_frame(&mut self, frame: &Value) -> Result<Vec<Vec<u8>>> {
        let mut frame = frame.clone();
        if frame["type"] == "coding_provider_request"
            && frame["id"] == 2
            && !self.analysis_profile()
        {
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
                if v["type"] == "bridge_prompt"
                    && is_fixture_bridge(&self.journal.job.coding.as_ref().unwrap().schema)
                {
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
        let hosted = self.coding_hosted_grant(&command)?;
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
            if let (Some(o), Some(Value::Object(fields))) = (command.as_object_mut(), &hosted) {
                o.extend(fields.clone());
            }
            let mut bytes = serde_json::to_vec(&command)?;
            bytes.push(b'\n');
            send(&bytes)
        })
    }
    pub(super) fn signed_pi(&self) -> bool {
        self.journal
            .job
            .coding
            .as_ref()
            .is_some_and(|p| p.schema == SIGNED_PROFILE)
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
        if j.job.coding.as_ref().unwrap().schema == SIGNED_PROFILE {
            args.extend(strings(&["--tmpfs", export::TMPFS]));
        }
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
        if is_bridge(&j.job.coding.as_ref().unwrap().schema) {
            args.push("--lifecycle-bridge".into());
            if j.job.coding.as_ref().unwrap().schema == CLAIMS_BRIDGE_PROFILE {
                args.push("--claims-tools".into());
            }
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
        if self.analysis_profile() {
            // No tool container and no workspace copy: the Host serves the
            // pinned snapshot itself.
            return Ok(());
        }
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
    /// Process the container's frames given its whole output so far: the
    /// first `frames` complete lines were already applied and are skipped.
    pub(super) fn coding_advance(&mut self, raw: &[u8]) -> Result<Vec<Vec<u8>>> {
        let count = self.journal.coding.as_ref().unwrap().frames;
        let replies = self.coding_advance_lines(complete_lines(raw).skip(count).collect())?;
        let done = self.journal.coding.as_ref().unwrap().frames;
        self.coding_out_offset = complete_lines(raw).take(done).map(|l| l.len() as u64).sum();
        Ok(replies)
    }
    /// The Host loop's path: read only the output written after the last
    /// applied frame (`coding_out_offset`) and process its complete lines, so
    /// each tick costs the new output, not the whole output so far.
    pub(super) fn coding_advance_from(&mut self, stdout: &Path) -> Result<Vec<Vec<u8>>> {
        let tail = read_from(stdout, self.coding_out_offset)?;
        let lines: Vec<&[u8]> = complete_lines(&tail).collect();
        let consumed: u64 = lines.iter().map(|l| l.len() as u64).sum();
        let replies = self.coding_advance_lines(lines)?;
        self.coding_out_offset += consumed;
        Ok(replies)
    }
    fn coding_advance_lines(&mut self, lines: Vec<&[u8]>) -> Result<Vec<Vec<u8>>> {
        let mut replies = vec![];
        let frame_limit = if self.analysis_profile() {
            analysis::FRAME_BYTES
        } else {
            65536
        };
        for line in lines {
            if line.len() > frame_limit {
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
                && !(is_bridge(&self.journal.job.coding.as_ref().unwrap().schema)
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
                    let reply = {
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
                    if kind == "provider_request"
                        && is_live_bridge(&self.journal.job.coding.as_ref().unwrap().schema)
                    {
                        // Live continuity entries reserve and are fenced only via
                        // `coding_provider_request`; a bare grant would bypass the gate.
                        return refuse("bare provider request in live continuity profile");
                    }
                    if is_hosted(&self.journal.job.coding.as_ref().unwrap().schema) {
                        // Settlement evidence is the Host's own relay observation.
                        if kind == "provider_observation" {
                            return refuse("container provider observation in hosted profile");
                        }
                        if kind == "provider_complete" {
                            self.coding_hosted_before_complete(frame["id"].as_u64().ok_or_else(
                                || Error::Refused("provider request id invalid".into()),
                            )?)?;
                        }
                    }
                    if let Some(reply) = self.coding_provider_frame(&frame)? {
                        replies.push(serde_json::to_vec(&reply)?);
                    }
                }
                "ready" => {
                    if self.journal.coding.as_ref().unwrap().ready_observed
                        || (!self.signed_pi() && self.journal.session_id.is_some())
                        || (self.signed_pi()
                            && self.journal.session_id.as_deref() != frame["sessionId"].as_str())
                        || !tools_match(
                            &frame["tools"],
                            &self.journal.job.coding.as_ref().unwrap().schema,
                            self.journal
                                .job
                                .coding
                                .as_ref()
                                .unwrap()
                                .provider
                                .as_ref()
                                .map(|p| p.allow_shell)
                                .unwrap_or(false),
                            self.journal
                                .job
                                .coding
                                .as_ref()
                                .unwrap()
                                .provider
                                .as_ref()
                                .map(|p| p.allow_background)
                                .unwrap_or(false),
                        )
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
                    if is_bridge(&self.journal.job.coding.as_ref().unwrap().schema) {
                        if let Some(plan) = &self.journal.job.coding.as_ref().unwrap().provider {
                            // Live: the native catalog model must be the pinned one.
                            if !plan.native_model_matches(&frame["model"])? {
                                return refuse("native selected catalog model differs");
                            }
                        }
                        replies.extend(self.coding_bridge_frame(&frame)?);
                        self.journal.coding.as_mut().unwrap().frames += 1;
                        self.save()?;
                        continue;
                    }
                    let profile = self.journal.job.coding.as_ref().unwrap();
                    let reply = if let Some(plan) = &profile.provider {
                        if !plan.native_model_matches(&frame["model"])? {
                            return refuse("native selected catalog model differs");
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
                    // Policy, not a Docker failure, refuses a shell call
                    // outside the coding profiles.
                    if self.analysis_profile() {
                        return refuse("workspace tool outside a coding profile");
                    }
                    // Revision 4: shell is a capability admitted at admission,
                    // never selected by the model and never implied by the
                    // receiver. Refuse here on Host-held authority before
                    // recording intent or invoking the effect gate, so a
                    // malformed or compromised receiver cannot bypass the gate
                    // by submitting a tool_request when allow_shell is false.
                    if !self
                        .journal
                        .job
                        .coding
                        .as_ref()
                        .unwrap()
                        .provider
                        .as_ref()
                        .map(|p| p.allow_shell)
                        .unwrap_or(false)
                    {
                        return refuse("shell execution not admitted");
                    }
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
                "claims_request" => {
                    if self.journal.job.coding.as_ref().unwrap().schema != CLAIMS_BRIDGE_PROFILE {
                        return refuse("claims request outside the claims profile");
                    }
                    if self.expired() || self.cancelled() {
                        return refuse("claims admission expired/cancelled");
                    }
                    let id = frame["id"]
                        .as_str()
                        .filter(|s| !s.is_empty() && s.len() < 128)
                        .ok_or_else(|| Error::Refused("claims id absent".into()))?
                        .to_string();
                    let tool = frame["tool"]
                        .as_str()
                        .filter(|t| claims::TOOLS.contains(t))
                        .ok_or_else(|| Error::Refused("unknown claims tool".into()))?
                        .to_string();
                    let arguments = frame["arguments"].clone();
                    if !arguments.is_object()
                        || arguments.to_string().len() > claims::MAX_ARGUMENT_BYTES
                    {
                        return refuse("claims arguments bound");
                    }
                    let state = self.journal.coding.as_mut().unwrap();
                    if state.claims_requests.contains_key(&id)
                        || state.claims_requests.len() >= MAX_CLAIMS_CALLS
                    {
                        return refuse("duplicate or excess claims request");
                    }
                    state.claims_requests.insert(id.clone(), None);
                    state.claims_tools.insert(id.clone(), tool.clone());
                    self.save()?; // intent precedes the read; None stays unknown
                    let deadline_ms = self.journal.job.deadline_unix_seconds.saturating_mul(1000);
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|_| Error::Refused("clock".into()))?
                        .as_millis() as u64;
                    if now_ms >= deadline_ms {
                        return refuse("claims deadline expired");
                    }
                    let until = std::time::Instant::now()
                        + std::time::Duration::from_millis(deadline_ms - now_ms);
                    let mut gate = self
                        .coding_gate
                        .take()
                        .ok_or_else(|| Error::Refused("coding lifecycle gate missing".into()))?;
                    let outcome = gate.claims(&tool, &arguments, until);
                    self.coding_gate = Some(gate);
                    let result = match outcome {
                        Ok(v) => json!({"type":"claims_result","id":id,"ok":true,"result":v}),
                        Err(Error::Refused(detail)) => {
                            json!({"type":"claims_result","id":id,"ok":false,"error":detail})
                        }
                        Err(e) => return Err(e),
                    };
                    let bytes = serde_json::to_vec(&result)?;
                    if bytes.len() > 100_000 {
                        return refuse("claims result exceeded frame bound");
                    }
                    let path = self
                        .journal
                        .job
                        .state_dir
                        .join(format!("coding-claims-{}.json", digest(id.as_bytes())));
                    atomic(&path, &result)?;
                    let artifact = Artifact {
                        sha256: file_digest(&path, None)?,
                        path,
                    };
                    let delivered_ok = result["ok"] == true;
                    let state = self.journal.coding.as_mut().unwrap();
                    state.claims_requests.insert(id, Some(artifact));
                    // Only successful owner deliveries count as recovered bytes;
                    // completed refusals are retained but never counted.
                    if delivered_ok {
                        if state.replaced {
                            state.claims_after_replacement += 1;
                        } else {
                            state.claims_before_replacement += 1;
                        }
                    }
                    self.save()?;
                    replies.push(bytes);
                }
                "coding_provider_request" | "coding_history" | "coding_replaced" => {
                    if !is_bridge(&self.journal.job.coding.as_ref().unwrap().schema) {
                        return refuse("coding lifecycle frame in legacy profile");
                    }
                    if kind == "coding_provider_request"
                        && is_live_bridge(&self.journal.job.coding.as_ref().unwrap().schema)
                    {
                        // Existing Host provider path: durable unknown Reservation
                        // before any grant, pinned model, finite budget. The
                        // lifecycle gate (below) then fences the exact grant write.
                        let request = json!({"type":"provider_request","id":frame["id"],
                            "provider":frame["requestModel"]["provider"],
                            "model":frame["requestModel"]["id"],
                            "contextBytes":frame["contextBytes"]});
                        self.coding_provider_frame(&request)?;
                    }
                    replies.extend(self.coding_bridge_frame(&frame)?);
                    if kind == "coding_replaced" {
                        self.journal.coding.as_mut().unwrap().replaced = true;
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
                "snapshot_request" => {
                    let reply = self.coding_snapshot_frame(&frame)?;
                    replies.push(reply);
                }
                "native_event" => {
                    if self.analysis_profile() {
                        analysis::check_answer_bound(&frame["event"])?;
                    }
                    if frame["event"]["type"] == "tool_execution_end" && self.analysis_profile() {
                        self.analysis_native_end(&frame["event"])?;
                    } else if frame["event"]["type"] == "tool_execution_end"
                        && self.journal.job.coding.as_ref().unwrap().schema == CLAIMS_BRIDGE_PROFILE
                        && frame["event"]["toolName"]
                            .as_str()
                            .is_some_and(|t| claims::TOOLS.contains(&t))
                    {
                        let event = &frame["event"];
                        let id = event["toolCallId"]
                            .as_str()
                            .ok_or_else(|| Error::Refused("native claims id absent".into()))?;
                        let state = self.journal.coding.as_mut().unwrap();
                        let delivered: Option<Value> = state
                            .claims_requests
                            .get(id)
                            .and_then(Option::as_ref)
                            .and_then(|a| a.verify().ok())
                            .and_then(|b| serde_json::from_slice(&b).ok());
                        let Some(delivered) = delivered else {
                            return refuse("native claims completion mismatch");
                        };
                        let ok = delivered["ok"] == true;
                        let expected = if ok {
                            delivered["result"].clone()
                        } else {
                            json!({"error":delivered["error"]})
                        };
                        let native_text = event["result"]["content"][0]["text"]
                            .as_str()
                            .and_then(|t| serde_json::from_str::<Value>(t).ok());
                        if event["isError"] != false
                            || state.claims_tools.get(id).map(String::as_str)
                                != event["toolName"].as_str()
                            || event["result"]["content"].as_array().map(Vec::len) != Some(1)
                            || native_text.as_ref() != Some(&expected)
                            || event["result"]["details"]["hostReportedSuccess"] != json!(ok)
                            || !state.native_claims_ids.insert(id.into())
                        {
                            return refuse("native claims completion mismatch");
                        }
                    } else if frame["event"]["type"] == "tool_execution_end" {
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
                    if is_bridge(&self.journal.job.coding.as_ref().unwrap().schema) {
                        replies.extend(self.coding_bridge_frame(&frame)?);
                    }
                }
                "provider_fetch" | "pull" | "cancel" | "ack" => {
                    self.coding_hosted_frame(&frame)?;
                }
                "export_part" | "export_final" => {
                    self.coding_export_frame(&frame, kind)?;
                }
                "complete" if self.analysis_profile() => {
                    self.analysis_complete(&frame)?;
                    self.journal.coding.as_mut().unwrap().completed = true;
                }
                "complete" => {
                    if self.is_signed_fixture() && !self.export_finalized() {
                        return refuse("coding completion before export finalization");
                    }
                    if is_bridge(&self.journal.job.coding.as_ref().unwrap().schema)
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
                            || (is_continuity(&self.journal.job.coding.as_ref().unwrap().schema)
                                && state.provider_requests.len() != 4)
                            || state
                                .provider_requests
                                .values()
                                .any(|r| r.settled_microusd.is_none()))
                    {
                        return refuse("coding provider completion unestablished");
                    }
                    if self.journal.job.coding.as_ref().unwrap().schema == CLAIMS_BRIDGE_PROFILE
                        && (state.claims_requests.values().any(Option::is_none)
                            || state.native_claims_ids.len() != state.claims_requests.len()
                            || state.claims_before_replacement == 0
                            || state.claims_after_replacement == 0
                            || !state.replaced)
                    {
                        return refuse(
                            "claims completion lacks native tool evidence before and after replacement",
                        );
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
    /// Complete container lines not yet applied, from `coding_out_offset`.
    pub(super) fn coding_unprocessed(&self, stdout: &Path) -> Result<Vec<Value>> {
        let tail = read_from(stdout, self.coding_out_offset)?;
        complete_lines(&tail)
            .map(|l| Ok(serde_json::from_slice(l)?))
            .collect()
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
        if let Some(toolchain) = &profile.toolchain {
            toolchain.verify()?;
        }
        if let Some(acceptance) = &profile.acceptance {
            acceptance.verify()?;
        }
        if let Some(plan) = &profile.provider {
            let expected: BTreeMap<String, Entry> =
                serde_json::from_slice(&plan.lane.pi_config.manifest.verify()?)?;
            if inventory(&self.journal.job.state_dir.join("coding-public-config"))? != expected {
                return refuse("public lane config realization changed");
            }
            plan.catalog.verify()?;
            plan.task.verify()?;
            plan.stream_adapter.verify()?;
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
        if self.signed_pi() {
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
                toolchain: Some(tree),
                acceptance: Some(pin),
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
            hosted_transport: None,
            hosted: Default::default(),
            pi_owner: None,
            allocation_lease: None,
            reply_leases: vec![],
            coding_out_offset: 0,
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
    /// The analysis profile refuses a shell call by policy, before any
    /// intent is journaled or Docker is consulted.
    #[test]
    fn analysis_profile_refuses_workspace_tool_requests_by_policy() {
        let (_dir, mut h) = fixture();
        h.journal.job.coding.as_mut().unwrap().schema = ANALYSIS_PROFILE.into();
        h.journal.session_id = Some("native".into());
        let request = frame(
            json!({"schema":ANALYSIS_PROFILE,"sessionId":"native","type":"tool_request","id":"t1","command":"id"}),
        );
        match h.coding_advance(&request) {
            Err(Error::Refused(m)) => assert_eq!(m, "workspace tool outside a coding profile"),
            other => panic!("{other:?}"),
        }
        assert!(h.journal.coding.as_ref().unwrap().tool_requests.is_empty());
        assert!(h.journal.processes.is_empty());
    }
    /// Revision 4: shell is a Host-admitted capability. A tool_request is
    /// refused on Host-held authority (not merely hidden by the receiver) when
    /// the admitted plan does not grant allow_shell, before any intent is
    /// journaled or Docker is consulted.
    #[test]
    fn tool_request_is_refused_when_shell_is_not_admitted() {
        let (_dir, mut h) = fixture();
        h.journal.session_id = Some("native".into());
        let profile = h.journal.job.coding.as_mut().unwrap();
        profile.schema = HOSTED_BRIDGE_PROFILE.into();
        profile.provider = Some(super::provider::ProviderPlan {
            lane: super::provider::LaneInputs {
                pi_config: Tree {
                    root: std::path::PathBuf::from("/nonexistent"),
                    manifest: Artifact {
                        path: std::path::PathBuf::from("/nonexistent"),
                        sha256: "0".repeat(64),
                    },
                },
            },
            task: Artifact {
                path: std::path::PathBuf::from("/nonexistent"),
                sha256: "0".repeat(64),
            },
            catalog: Artifact {
                path: std::path::PathBuf::from("/nonexistent"),
                sha256: "0".repeat(64),
            },
            stream_adapter: Artifact {
                path: std::path::PathBuf::from("/nonexistent"),
                sha256: "0".repeat(64),
            },
            hosted: super::hosted::HostedRoutes {
                routes: vec![super::hosted::HostedRoute {
                    id: "r".into(),
                    payer: "p".into(),
                    endpoint: "loopback:9".into(),
                }],
                max_attempts_per_request: 1,
            },
            provider: "openai-codex".into(),
            model: "gpt-6-luna".into(),
            max_requests: 1,
            budget_microusd: 1_000_000,
            max_context_bytes: 16384,
            allow_shell: false,
            allow_background: false,
        });
        drop(profile);
        let request = frame(
            json!({"schema":HOSTED_BRIDGE_PROFILE,"sessionId":"native","type":"tool_request","id":"t1","command":"id"}),
        );
        match h.coding_advance(&request) {
            Err(Error::Refused(m)) => assert_eq!(m, "shell execution not admitted"),
            other => panic!("{other:?}"),
        }
        assert!(h.journal.coding.as_ref().unwrap().tool_requests.is_empty());
        assert!(h.journal.processes.is_empty());
    }

    /// H4: only a relay thread's inference process outlives 300 s.
    #[test]
    fn relay_inference_process_wall_bound_is_the_relay_thread_bound() {
        let (_dir, mut h) = fixture();
        assert_eq!(h.process_wall_seconds(true), 300);
        assert_eq!(
            h.process_wall_seconds(false),
            h.journal.job.command_timeout_seconds
        );
        h.journal.job.coding.as_mut().unwrap().schema = ANALYSIS_PROFILE.into();
        assert_eq!(h.process_wall_seconds(true), 300);
        h.journal.job.coding.as_mut().unwrap().schema = LIFECYCLE_PROFILE.into();
        assert_eq!(h.process_wall_seconds(true), RELAY_DEADLINE_SECONDS);
        assert_eq!(
            h.process_wall_seconds(false),
            h.journal.job.command_timeout_seconds
        );
    }
    #[test]
    fn analysis_answer_bound_is_checked_when_the_answer_is_observed() {
        let answer = |n: usize| json!({"type":"message_end","message":{"role":"assistant","stopReason":"stop","content":[{"type":"text","text":"x".repeat(n)}]}});
        assert!(analysis::check_answer_bound(&answer(analysis::MAX_ANSWER_BYTES)).is_ok());
        assert!(analysis::check_answer_bound(&answer(analysis::MAX_ANSWER_BYTES + 1)).is_err());
        // Tool-use messages and other events are not answers.
        let mut tool_use = answer(analysis::MAX_ANSWER_BYTES + 1);
        tool_use["message"]["stopReason"] = json!("toolUse");
        assert!(analysis::check_answer_bound(&tool_use).is_ok());
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
    /// The Host loop reads only output after the last applied frame: a partial
    /// line waits, and a later frame is applied exactly once.
    #[test]
    fn incremental_output_applies_each_complete_frame_once() {
        let (dir, mut h) = fixture();
        let out = dir.path().join("rpc.stdout");
        let ready = frame(
            json!({"schema":PROFILE,"sessionId":"native","type":"ready","tools":["workspace_exec"]}),
        );
        let (head, rest) = ready.split_at(ready.len() / 2);
        fs::write(&out, head).unwrap();
        assert!(h.coding_advance_from(&out).unwrap().is_empty());
        assert_eq!(h.coding_out_offset, 0);
        assert!(h.coding_unprocessed(&out).unwrap().is_empty());
        let mut file = OpenOptions::new().append(true).open(&out).unwrap();
        file.write_all(rest).unwrap();
        assert_eq!(h.coding_unprocessed(&out).unwrap().len(), 1);
        assert_eq!(h.coding_advance_from(&out).unwrap().len(), 1);
        assert_eq!(h.coding_out_offset, ready.len() as u64);
        assert_eq!(h.journal.coding.as_ref().unwrap().frames, 1);
        // Nothing new: nothing applied again.
        assert!(h.coding_advance_from(&out).unwrap().is_empty());
        // A second ready frame is new output and is refused as a duplicate.
        file.write_all(&ready).unwrap();
        assert!(h.coding_advance_from(&out).is_err());
        // The whole-output path agrees with the offset it leaves behind.
        let (_dir2, mut g) = fixture();
        g.coding_advance(&ready).unwrap();
        assert_eq!(g.coding_out_offset, ready.len() as u64);
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

    /// Test gate: lifecycle methods are inert; `claims` answers from a closure.
    type ClaimsCalls = std::sync::Arc<std::sync::Mutex<Vec<(String, Value)>>>;
    struct ClaimsStub {
        calls: ClaimsCalls,
        fail: bool,
    }
    impl CodingEffectGate for ClaimsStub {
        fn execute(
            &mut self,
            _: &CodingToolIntent,
            action: &mut dyn FnMut(i64) -> Result<(bool, Vec<u8>)>,
        ) -> Result<(bool, Vec<u8>)> {
            action(1)
        }
        fn frame(&mut self, _: &Value) -> Result<Vec<Value>> {
            Ok(vec![])
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
        fn claims(&mut self, tool: &str, args: &Value, until: std::time::Instant) -> Result<Value> {
            assert!(until > std::time::Instant::now());
            self.calls.lock().unwrap().push((tool.into(), args.clone()));
            if self.fail {
                return refuse("claims evidence_not_listed: stub refusal");
            }
            Ok(json!({"schema":"claims-consumer-page-v1","text":"owner bytes"}))
        }
    }
    fn claims_host(fail: bool) -> (tempfile::TempDir, Host, ClaimsCalls) {
        let (dir, mut h) = fixture();
        let calls = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
        h.journal.job.coding.as_mut().unwrap().schema = CLAIMS_BRIDGE_PROFILE.into();
        h.journal.job.coding.as_mut().unwrap().max_tool_calls = 2;
        h.journal.session_id = Some("native".into());
        h.coding_gate = Some(Box::new(ClaimsStub {
            calls: calls.clone(),
            fail,
        }));
        (dir, h, calls)
    }
    fn claims_frame(id: &str, tool: &str, args: Value) -> Vec<u8> {
        frame(
            json!({"schema":CLAIMS_BRIDGE_PROFILE,"sessionId":"native","type":"claims_request","id":id,"tool":tool,"arguments":args}),
        )
    }
    /// `coding_advance` consumes one cumulative stream and skips frames it
    /// already processed, so tests append to an accumulated buffer; a refused
    /// frame is not retained, matching the Host's unchanged frame count.
    fn adv(h: &mut Host, acc: &mut Vec<u8>, f: Vec<u8>) -> Result<Vec<Vec<u8>>> {
        let mut next = acc.clone();
        next.extend(f);
        let r = h.coding_advance(&next);
        if r.is_ok() {
            *acc = next;
        }
        r
    }
    #[test]
    fn claims_requests_exist_only_in_the_claims_profile_with_exact_tools_and_bounds() {
        // Legacy and v4-style profiles refuse the frame before any gate call.
        for schema in [PROFILE, BRIDGE_PROFILE] {
            let (_d, mut h) = fixture();
            h.journal.session_id = Some("native".into());
            h.journal.job.coding.as_mut().unwrap().schema = schema.into();
            let req = frame(
                json!({"schema":schema,"sessionId":"native","type":"claims_request","id":"c","tool":"claims_discover","arguments":{"query":"x"}}),
            );
            assert!(h.coding_advance(&req).is_err(), "{schema}");
        }
        let (_d, mut h, calls) = claims_host(false);
        let mut acc: Vec<u8> = Vec::new();
        // Unknown tool, workspace tool name, non-object and oversized arguments.
        for (tool, args) in [
            ("workspace_exec", json!({"command":"id"})),
            ("claims_publish", json!({})),
            ("claims_resolve", json!("not an object")),
            ("claims_discover", json!({"query":"x".repeat(3000)})),
        ] {
            assert!(
                adv(&mut h, &mut acc, claims_frame("bad", tool, args)).is_err(),
                "{tool}"
            );
            h.journal.coding.as_mut().unwrap().claims_requests.clear();
        }
        assert!(calls.lock().unwrap().is_empty(), "refused before the gate");
        // Foreign session never reaches the gate.
        let req = frame(
            json!({"schema":CLAIMS_BRIDGE_PROFILE,"sessionId":"foreign","type":"claims_request","id":"c","tool":"claims_discover","arguments":{"query":"x"}}),
        );
        assert!(h.coding_advance(&req).is_err());
        // Valid request: routed once, result persisted and returned verbatim.
        let replies = adv(
            &mut h,
            &mut acc,
            claims_frame("c1", "claims_discover", json!({"query":"assertion"})),
        )
        .unwrap();
        let reply: Value = serde_json::from_slice(&replies[0]).unwrap();
        assert_eq!(reply["type"], "claims_result");
        assert_eq!(reply["ok"], true);
        assert_eq!(reply["result"]["text"], "owner bytes");
        assert_eq!(calls.lock().unwrap().len(), 1);
        let st = h.journal.coding.as_ref().unwrap();
        assert!(st.claims_requests["c1"].is_some());
        assert_eq!(
            (st.claims_before_replacement, st.claims_after_replacement),
            (1, 0)
        );
        // Duplicate ids and the finite budget refuse.
        assert!(
            adv(
                &mut h,
                &mut acc,
                claims_frame("c1", "claims_discover", json!({"query":"a"}))
            )
            .is_err()
        );
        for i in 0..MAX_CLAIMS_CALLS {
            let r = adv(
                &mut h,
                &mut acc,
                claims_frame(&format!("n{i}"), "claims_discover", json!({"query":"a"})),
            );
            assert_eq!(r.is_ok(), i + 1 < MAX_CLAIMS_CALLS, "call {i}");
        }
        assert!(calls.lock().unwrap().len() <= MAX_CLAIMS_CALLS);
    }
    #[test]
    fn claims_gate_refusal_is_a_recorded_tool_failure_and_missing_gate_refuses() {
        let (_d, mut h, _) = claims_host(true);
        let mut acc: Vec<u8> = Vec::new();
        let replies = adv(
            &mut h,
            &mut acc,
            claims_frame("c1", "claims_evidence", json!({"sha256":"0"})),
        )
        .unwrap();
        let reply: Value = serde_json::from_slice(&replies[0]).unwrap();
        assert_eq!(reply["ok"], false);
        assert!(
            reply["error"]
                .as_str()
                .unwrap()
                .contains("evidence_not_listed")
        );
        assert!(h.journal.coding.as_ref().unwrap().claims_requests["c1"].is_some());
        let (_d, mut h, _) = claims_host(false);
        let mut acc: Vec<u8> = Vec::new();
        h.coding_gate = None;
        assert!(
            adv(
                &mut h,
                &mut acc,
                claims_frame("c1", "claims_discover", json!({"query":"a"}))
            )
            .is_err()
        );
        // An installed gate without claims composition refuses by default.
        let (_d, mut h, _) = claims_host(false);
        let mut acc: Vec<u8> = Vec::new();
        h.coding_gate = Some(Box::new(FixtureGate));
        let replies = adv(
            &mut h,
            &mut acc,
            claims_frame("c1", "claims_discover", json!({"query":"a"})),
        )
        .unwrap();
        let reply: Value = serde_json::from_slice(&replies[0]).unwrap();
        assert_eq!(reply["ok"], false);
    }
    #[test]
    fn claims_completion_needs_native_evidence_before_and_after_replacement() {
        let (_d, mut h, _) = claims_host(false);
        let mut acc: Vec<u8> = Vec::new();
        let native_with = |id: &str, name: &str, text: &str, ok: bool| {
            frame(
                json!({"schema":CLAIMS_BRIDGE_PROFILE,"sessionId":"native","type":"native_event","event":{"type":"tool_execution_end","toolCallId":id,"toolName":name,"isError":false,"result":{"content":[{"type":"text","text":text}],"details":{"hostReportedSuccess":ok}}}}),
            )
        };
        let native = |id: &str, name: &str| {
            native_with(
                id,
                name,
                &json!({"schema":"claims-consumer-page-v1","text":"owner bytes"}).to_string(),
                true,
            )
        };
        // Native completion for a never-requested claims id refuses.
        assert!(adv(&mut h, &mut acc, native("ghost", "claims_resolve")).is_err());
        adv(
            &mut h,
            &mut acc,
            claims_frame("c1", "claims_discover", json!({"query":"a"})),
        )
        .unwrap();
        // A completion naming another claims tool, different result bytes or a
        // different outcome cannot satisfy the request.
        assert!(
            adv(&mut h, &mut acc, native("c1", "claims_currency")).is_err(),
            "different tool"
        );
        assert!(
            adv(
                &mut h,
                &mut acc,
                native_with("c1", "claims_discover", "{\"text\":\"other\"}", true)
            )
            .is_err(),
            "different result"
        );
        assert!(
            adv(
                &mut h,
                &mut acc,
                native_with("c1", "claims_discover", "{\"error\":\"x\"}", false)
            )
            .is_err(),
            "wrong outcome"
        );
        adv(&mut h, &mut acc, native("c1", "claims_discover")).unwrap();
        assert!(
            adv(&mut h, &mut acc, native("c1", "claims_discover")).is_err(),
            "replayed native end"
        );
        // Mark a workspace tool as satisfied, then try to complete without any
        // successor claims call: refused.
        let st = h.journal.coding.as_mut().unwrap();
        st.tool_requests.insert(
            "w".into(),
            Some(Artifact {
                path: "/unused".into(),
                sha256: "0".repeat(64),
            }),
        );
        st.native_tool_ends = 1;
        st.native_tool_ids.insert("w".into());
        st.replaced = true;
        let complete = frame(
            json!({"schema":CLAIMS_BRIDGE_PROFILE,"sessionId":"native","type":"complete","counts":{"providerRequests":4,"resets":1}}),
        );
        assert!(
            adv(&mut h, &mut acc, complete.clone()).is_err(),
            "no claims call after replacement"
        );
        assert!(!h.journal.coding.as_ref().unwrap().completed);
        adv(
            &mut h,
            &mut acc,
            claims_frame("c2", "claims_resolve", json!({"claim_id":"a"})),
        )
        .unwrap();
        assert_eq!(
            h.journal.coding.as_ref().unwrap().claims_after_replacement,
            1
        );
        assert!(
            adv(&mut h, &mut acc, complete.clone()).is_err(),
            "second claims call lacks native completion"
        );
        adv(&mut h, &mut acc, native("c2", "claims_resolve")).unwrap();
        assert!(adv(&mut h, &mut acc, native("c2", "bash")).is_err());
        adv(&mut h, &mut acc, complete.clone()).unwrap();
        assert!(h.journal.coding.as_ref().unwrap().completed);
    }
    #[test]
    fn completed_refusals_do_not_count_as_successful_claims_delivery() {
        let (_d, mut h, _) = claims_host(true); // gate refuses every read
        let mut acc: Vec<u8> = Vec::new();
        let replies = adv(
            &mut h,
            &mut acc,
            claims_frame("r1", "claims_resolve", json!({"claim_id":"a"})),
        )
        .unwrap();
        let reply: Value = serde_json::from_slice(&replies[0]).unwrap();
        assert_eq!(reply["ok"], false);
        let text = json!({"error":reply["error"]}).to_string();
        adv(
            &mut h,
            &mut acc,
            frame(
                json!({"schema":CLAIMS_BRIDGE_PROFILE,"sessionId":"native","type":"native_event","event":{"type":"tool_execution_end","toolCallId":"r1","toolName":"claims_resolve","isError":false,"result":{"content":[{"type":"text","text":text}],"details":{"hostReportedSuccess":false}}}}),
            ),
        )
        .unwrap();
        let st = h.journal.coding.as_mut().unwrap();
        assert_eq!(
            (st.claims_before_replacement, st.claims_after_replacement),
            (0, 0),
            "a refusal is retained but is not delivery evidence"
        );
        st.replaced = true;
        st.tool_requests.insert(
            "w".into(),
            Some(Artifact {
                path: "/unused".into(),
                sha256: "0".repeat(64),
            }),
        );
        st.native_tool_ends = 1;
        st.native_tool_ids.insert("w".into());
        let complete = frame(
            json!({"schema":CLAIMS_BRIDGE_PROFILE,"sessionId":"native","type":"complete","counts":{"providerRequests":4,"resets":1}}),
        );
        assert!(adv(&mut h, &mut acc, complete).is_err(), "all-refused");
    }
    #[test]
    fn claims_profile_ready_tool_set_is_exact_and_legacy_sets_are_unchanged() {
        // Provider-disabled fixture profiles always allow shell.
        assert!(tools_match(&json!(["workspace_exec"]), PROFILE, true, false));
        assert!(!tools_match(
            &json!(["workspace_exec", "claims_discover"]),
            BRIDGE_PROFILE,
            true,
            false
        ));
        let all = json!([
            "claims_currency",
            "claims_discover",
            "claims_evidence",
            "claims_resolve",
            "workspace_exec"
        ]);
        assert!(tools_match(&all, CLAIMS_BRIDGE_PROFILE, true, false));
        assert!(!tools_match(
            &json!(["workspace_exec"]),
            CLAIMS_BRIDGE_PROFILE,
            true,
            false
        ));
        assert!(!tools_match(
            &json!([
                "workspace_exec",
                "claims_discover",
                "claims_resolve",
                "claims_evidence",
                "bash"
            ]),
            CLAIMS_BRIDGE_PROFILE,
            true,
            false
        ));
        // Revision 4: on the hosted coding surface, no execution tool is
        // advertised when shell is not admitted.
        assert!(tools_match(&json!([]), HOSTED_BRIDGE_PROFILE, false, false));
        assert!(!tools_match(&json!(["workspace_exec"]), HOSTED_BRIDGE_PROFILE, false, false));
        assert!(tools_match(&json!(["workspace_exec"]), HOSTED_BRIDGE_PROFILE, true, false));
        // Shell + granted background: the shell tool and the approved plugin's
        // tool set are both advertised; without background they are not.
        let shell_and_bg = {
            let mut v = serde_json::json!(["workspace_exec"]);
            let arr = v.as_array_mut().unwrap();
            for t in BG_TOOLS {
                arr.push(serde_json::json!(t));
            }
            v
        };
        assert!(tools_match(&shell_and_bg, HOSTED_BRIDGE_PROFILE, true, true));
        assert!(!tools_match(&shell_and_bg, HOSTED_BRIDGE_PROFILE, true, false));
        assert!(!tools_match(&json!(["workspace_exec"]), HOSTED_BRIDGE_PROFILE, true, true));
        assert!(is_bridge(CLAIMS_BRIDGE_PROFILE) && is_fixture_bridge(CLAIMS_BRIDGE_PROFILE));
        assert!(!is_fixture_bridge(HOSTED_BRIDGE_PROFILE));
    }
}
