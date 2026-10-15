use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use crate::{
    ProofError, Result,
    identity::{CloseEvidence, ProcessIdentity, sha256_file},
    provider::ProviderEntry,
};

pub const TO2_RECEIPT_SCHEMA: &str = "s8-to2-regression-evidence-v1";
const TO2_SCOPE: &str = "to2-impacted-regression";
const APPROVAL_SHA: &str = "3eb8ddcb99ccbc9027df55714217169eda5ccfc2cceb1a5ead32c41d50aa2207";
const ORIGINAL_PLAN_SHA: &str = "c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7";

fn contract_error(message: &str) -> ProofError {
    ProofError::Contract(message.into())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct To2Scenario {
    pub case_id: String,
    pub contract: ArtifactBinding,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoricalPlanReference {
    pub expected_sha256: String,
    pub status: String,
}

/// This packet binds supervisor-projected acceptance evidence, not human identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct To2AuthorityPacket {
    pub schema: String,
    pub issuance: String,
    pub scope: String,
    pub profile: String,
    pub approval: ArtifactBinding,
    pub to2_plan: ArtifactBinding,
    pub to2_contract: ArtifactBinding,
    pub amendment: ArtifactBinding,
    pub correction_v1: ArtifactBinding,
    pub correction_v2: ArtifactBinding,
    pub manifest: ArtifactBinding,
    pub validation: ArtifactBinding,
    pub takeover: ArtifactBinding,
    pub admission_certificate: ArtifactBinding,
    pub original_plan: HistoricalPlanReference,
    pub scenarios: Vec<To2Scenario>,
    pub exclusions: Vec<String>,
}

impl To2AuthorityPacket {
    fn scenario(&self, case: &str) -> Result<&To2Scenario> {
        self.scenarios
            .iter()
            .find(|scenario| scenario.case_id == case)
            .ok_or_else(|| contract_error("case is outside scoped TO2 authority"))
    }

    fn validate(&self, case: &str) -> Result<()> {
        if self.schema != "s8-to2-receipt-authority-v1"
            || self.issuance != "supervisor_verified"
            || self.scope != TO2_SCOPE
            || self.profile != "retained-repository-analysis-v1"
            || self.approval.sha256 != APPROVAL_SHA
            || self.original_plan.expected_sha256 != ORIGINAL_PLAN_SHA
            || self.original_plan.status != "exact_bytes_unrecovered"
            || self.takeover.sha256
                != "21d6ce369ff3770a3f97e67814506c1861790bd67cfa59f761ed984358ba9d4d"
            || self.admission_certificate.sha256
                != "47bedaff56c19fff112591d4b1199b2ccf249b37d0f85c7aa46d10017a46bf98"
        {
            return Err(contract_error("invalid scoped authority identity"));
        }
        self.approval.verify()?;
        let approval: Value = serde_json::from_slice(&fs::read(&self.approval.path)?)?;
        let accepted = &approval["s8"];
        if accepted["status"] != "accepted_for_bounded_implementation_and_validation"
            || accepted["full_s8_acceptance"] != false
            || accepted["original_s8_plan_bytes"] != "unrecovered"
        {
            return Err(contract_error("approval does not bind scoped acceptance"));
        }
        for (artifact, key) in [
            (&self.to2_plan, "active_to2_implementation_plan_sha256"),
            (&self.to2_contract, "active_to2_contract_sha256"),
            (&self.amendment, "prior_21path_amendment_sha256"),
            (&self.correction_v1, "incorporated_v1_sha256"),
            (&self.correction_v2, "plan_sha256"),
            (&self.manifest, "manifest_sha256"),
            (&self.validation, "validation_proposal_sha256"),
        ] {
            if accepted.get(key).and_then(Value::as_str) != Some(artifact.sha256.as_str()) {
                return Err(contract_error("artifact is not bound by scoped approval"));
            }
            artifact.verify()?;
        }
        self.takeover.verify()?;
        self.admission_certificate.verify()?;
        let expected = [
            (
                "s8-installed-d0-snapshot-child",
                "2afeb81499867f59e5d3b21ce45ce5d7dc61cf9cf5b79702cee09cd497e491b1",
            ),
            (
                "s8-installed-pending-question-authorized-answer",
                "33e6d653bba08d89dbd3de968140d4d7e4f7c17a8807e627b2a73fe27243b230",
            ),
            (
                "s8-installed-attributed-result-owner-commit",
                "b2bf39dbaff3a1ef0bf850aedd5c16c4a86c467f324a6d758e645dab7f069f91",
            ),
        ];
        if self.scenarios.len() != expected.len() {
            return Err(contract_error("scoped scenario set differs"));
        }
        for (id, digest) in expected {
            let scenario = self.scenario(id)?;
            if scenario.contract.sha256 != digest {
                return Err(contract_error("scenario basis differs"));
            }
            scenario.contract.verify()?;
        }
        self.scenario(case)?;
        let required = [
            "full_s8_qualification",
            "historical_failure_closure",
            "admin_schema5",
            "live_provider",
            "custody",
            "activation",
        ];
        if self.exclusions.len() != required.len()
            || required
                .iter()
                .any(|item| !self.exclusions.iter().any(|x| x == item))
        {
            return Err(contract_error("scoped authority exclusions differ"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct To2ReceiptAuthority {
    pub scope: String,
    pub case_id: String,
    pub packet: ArtifactBinding,
}

impl To2ReceiptAuthority {
    fn validate(&self, case: &str) -> Result<To2AuthorityPacket> {
        if self.scope != TO2_SCOPE || self.case_id != case {
            return Err(contract_error("scoped receipt case mismatch"));
        }
        self.packet.verify()?;
        let packet: To2AuthorityPacket = serde_json::from_slice(&fs::read(&self.packet.path)?)?;
        packet.validate(case)?;
        Ok(packet)
    }

    pub fn plan(&self) -> Result<ArtifactBinding> {
        Ok(self.validate(&self.case_id)?.to2_plan)
    }
}

/// Called by each scoped producer before host launch, provider start, or build.
/// Missing both variables selects the unchanged historical evidence path;
/// a partial/invalid scoped request never falls back to historical authority.
pub fn to2_authority_from_environment(case: &str) -> Result<Option<To2ReceiptAuthority>> {
    let path = std::env::var_os("S8_TO2_RECEIPT_AUTHORITY");
    let digest = std::env::var_os("S8_TO2_RECEIPT_AUTHORITY_SHA256");
    match (path, digest) {
        (None, None) => Ok(None),
        (Some(path), Some(digest)) => {
            for flag in [
                "S8_TWO_CYCLES",
                "S8_TRANSITION_A",
                "S8_DESIGN_INQUIRY",
                "S8_HOLD_Q1",
                "S8_CONTRACT_BEFORE_RESULT",
                "S8_CONTRACT_AT_COMMIT",
                "S8_CONTRACT_AFTER_OWNER_COMMIT",
                "S8_KILL_AFTER_OWNER_COMMIT",
                "S8_KILL_BEFORE_OWNER_COMMIT",
                "S8_DROP_GRANT_REPLY",
                "S8_OWNER_LOCK_BUSY",
            ] {
                if std::env::var_os(flag).is_some() {
                    return Err(contract_error(
                        "full-S8 variant is outside scoped receipt authority",
                    ));
                }
            }
            let evidence = std::env::var_os("S8_EVIDENCE_DIR")
                .ok_or_else(|| contract_error("scoped evidence destination required"))?;
            if !Path::new(&evidence).is_absolute() {
                return Err(contract_error(
                    "scoped evidence destination must be absolute",
                ));
            }
            let binding = ArtifactBinding::read(Path::new(&path))?;
            if digest.to_str() != Some(binding.sha256.as_str()) {
                return Err(contract_error("scoped packet digest differs"));
            }
            let authority = To2ReceiptAuthority {
                scope: TO2_SCOPE.into(),
                case_id: case.into(),
                packet: binding,
            };
            authority.validate(case)?;
            Ok(Some(authority))
        }
        _ => Err(contract_error(
            "scoped packet requires both path and digest",
        )),
    }
}

fn nonempty_string(value: Option<&Value>) -> bool {
    value.and_then(Value::as_str).is_some_and(|s| !s.is_empty())
}

fn component_shape(component: &Value) -> Result<()> {
    let object = component
        .as_object()
        .ok_or_else(|| contract_error("component is not an object"))?;
    if !component.get("scope").is_some_and(Value::is_object) {
        return Err(contract_error("component scope missing"));
    }
    let allowed: &[&str] = match component.get("state").and_then(Value::as_str) {
        Some("present") => {
            if !component.get("value").is_some_and(Value::is_object)
                || !component.get("revision").is_some_and(Value::is_object)
                || !nonempty_string(component.pointer("/revision/kind"))
            {
                return Err(contract_error("present component payload missing"));
            }
            &["state", "scope", "revision", "value", "completeness"]
        }
        Some("absent") if nonempty_string(component.get("record_kind")) => {
            &["state", "scope", "record_kind"]
        }
        Some("not_requested") => &["state", "scope"],
        Some("not_applicable" | "unavailable") if nonempty_string(component.get("reason")) => {
            &["state", "scope", "reason"]
        }
        Some("refused") if nonempty_string(component.get("reason")) => {
            &["state", "scope", "reason", "limit"]
        }
        _ => return Err(contract_error("invalid component state")),
    };
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(contract_error("component has forbidden payload"));
    }
    if let Some(limit) = component.get("limit")
        && limit.as_u64() != Some(64)
    {
        return Err(contract_error("invalid refusal limit"));
    }
    Ok(())
}

/// Derive the exact receipt projection without restoring nullable v1 reads.
pub fn to2_lifecycle_projection(host: &Value) -> Result<Value> {
    if host.get("status").and_then(Value::as_str) != Some("observation") {
        return mutation_lifecycle_projection(host);
    }
    if host.get("protocol_version").and_then(Value::as_u64) != Some(2)
        || host.get("consistency").and_then(Value::as_str) != Some("independent_components")
        || !host.get("host_identity").is_some_and(Value::is_object)
    {
        return Err(contract_error("invalid TO2 observation envelope"));
    }
    let request = host
        .get("request")
        .and_then(Value::as_object)
        .ok_or_else(|| contract_error("observation request missing"))?;
    if !nonempty_string(request.get("task_id"))
        || request
            .keys()
            .any(|key| !["task_id", "delivery_id", "command_id"].contains(&key.as_str()))
        || request.values().any(|value| !nonempty_string(Some(value)))
    {
        return Err(contract_error("observation request identity invalid"));
    }
    let components = host
        .get("components")
        .and_then(Value::as_object)
        .ok_or_else(|| contract_error("observation components missing"))?;
    let names = [
        "owner",
        "lifecycle",
        "native_delivery_status",
        "command_result",
        "control_result",
    ];
    if components.len() != names.len() {
        return Err(contract_error("observation component set differs"));
    }
    for name in names {
        component_shape(
            components
                .get(name)
                .ok_or_else(|| contract_error("component missing"))?,
        )?;
        if name != "native_delivery_status"
            && (components[name].get("completeness").is_some()
                || components[name].get("limit").is_some())
        {
            return Err(contract_error(
                "native capacity metadata on another component",
            ));
        }
    }
    let owner = &components["owner"];
    if owner.pointer("/scope/task_id") != request.get("task_id")
        || !matches!(
            owner["state"].as_str(),
            Some("present" | "absent" | "unavailable" | "refused")
        )
    {
        return Err(contract_error("owner component scope/state invalid"));
    }
    let subject = if owner["state"] == "present" {
        let subject = owner
            .pointer("/scope/subject_id")
            .filter(|v| nonempty_string(Some(v)))
            .ok_or_else(|| contract_error("owner subject missing"))?;
        if owner.pointer("/value/task_id") != request.get("task_id")
            || owner.pointer("/value/subject_id") != Some(subject)
            || owner.pointer("/revision/kind").and_then(Value::as_str) != Some("task")
            || owner
                .pointer("/revision/owner_revision")
                .and_then(Value::as_u64)
                .is_none()
            || owner
                .pointer("/revision/cursor_revision")
                .and_then(Value::as_u64)
                .is_none()
            || owner.pointer("/revision/owner_revision") != owner.pointer("/value/revision")
            || owner.pointer("/revision/cursor_revision") != owner.pointer("/value/cursor_revision")
        {
            return Err(contract_error("owner value/revision binding invalid"));
        }
        Some(subject)
    } else {
        if owner["state"] == "absent" && owner["record_kind"] != "task" {
            return Err(contract_error("owner absence record kind invalid"));
        }
        None
    };
    for name in ["lifecycle", "native_delivery_status"] {
        let component = &components[name];
        if component.pointer("/scope/subject_id") != Some(subject.unwrap_or(&Value::Null)) {
            return Err(contract_error("dependent subject scope differs"));
        }
        if subject.is_none() {
            let (state, reason) = if owner["state"] == "absent" {
                ("not_applicable", "task_absent")
            } else {
                ("unavailable", "task_dependency_unavailable")
            };
            if component["state"] != state || component["reason"] != reason {
                return Err(contract_error(
                    "dependent component invents owner availability",
                ));
            }
        } else if !matches!(
            component["state"].as_str(),
            Some("present" | "unavailable" | "refused")
        ) {
            return Err(contract_error("dependent component state invalid"));
        }
    }
    let lifecycle = &components["lifecycle"];
    if lifecycle.pointer("/scope/delivery_id")
        != Some(request.get("delivery_id").unwrap_or(&Value::Null))
    {
        return Err(contract_error("lifecycle delivery scope differs"));
    }
    if lifecycle["state"] == "present" {
        let value = &lifecycle["value"];
        let expected_subject = subject
            .and_then(Value::as_str)
            .ok_or_else(|| contract_error("lifecycle subject missing"))?;
        lifecycle_wire::parse_snapshot(&serde_json::to_vec(value)?, expected_subject)
            .map_err(|_| contract_error("malformed lifecycle value"))?;
        if lifecycle.pointer("/revision/kind").and_then(Value::as_str) != Some("lifecycle")
            || !value.get("cursor").is_some_and(Value::is_object)
            || lifecycle.pointer("/revision/cursor") != value.get("cursor")
            || value.get("protocol_version").and_then(Value::as_u64) != Some(1)
            || value.get("subject_id") != subject
            || ["store_id", "stream_id", "commit_sequence"]
                .iter()
                .any(|key| !nonempty_string(value.get("cursor").and_then(|cursor| cursor.get(key))))
        {
            return Err(contract_error("lifecycle value/revision differs"));
        }
        if let (Some(expected), Some(actual)) = (
            request.get("delivery_id"),
            value.pointer("/delivery/value/delivery_id"),
        ) && expected != actual
        {
            return Err(contract_error("observed delivery differs"));
        }
    }
    let native = &components["native_delivery_status"];
    if native.pointer("/scope/collection").and_then(Value::as_str)
        != Some("native_delivery_segments")
        || native.pointer("/scope/filter").and_then(Value::as_str) != Some("subject")
    {
        return Err(contract_error("native collection scope differs"));
    }
    if native["state"] == "present" {
        if native["revision"] != serde_json::json!({"kind":"not_provided"})
            || native["completeness"] != serde_json::json!({"kind":"complete_for_scope","limit":64})
            || native["value"]["protocol_version"] != 1
            || native["value"].get("subject_id") != subject
            || !native["value"]
                .get("recovery_only")
                .is_some_and(Value::is_boolean)
            || subject.and_then(Value::as_str).is_none_or(|subject| {
                lifecycle_wire::validate_native_delivery_status(&native["value"], subject).is_err()
            })
        {
            return Err(contract_error("native value/completeness differs"));
        }
    } else if native["state"] == "refused"
        && native["reason"] == "capacity_exceeded"
        && native["limit"] != 64
    {
        return Err(contract_error("native capacity limit differs"));
    }
    for name in ["command_result", "control_result"] {
        let component = &components[name];
        let command = request.get("command_id");
        if component.pointer("/scope/command_id") != Some(command.unwrap_or(&Value::Null))
            || component
                .pointer("/scope/task_binding")
                .and_then(Value::as_str)
                != Some("not_established")
            || !nonempty_string(component.pointer("/scope/subject_id"))
            || (command.is_none() && component["state"] != "not_requested")
            || (command.is_some()
                && !matches!(
                    component["state"].as_str(),
                    Some("present" | "absent" | "unavailable" | "refused")
                ))
        {
            return Err(contract_error("command component scope/state differs"));
        }
        if component["state"] == "absent"
            && component["record_kind"]
                != if name == "command_result" {
                    "admission_command_locator"
                } else {
                    "control_command_locator"
                }
        {
            return Err(contract_error("command absence kind differs"));
        }
        if component["state"] == "present" {
            let kind = if name == "command_result" {
                lifecycle_wire::CommandObservationKind::Admission
            } else {
                lifecycle_wire::CommandObservationKind::Control
            };
            if component["revision"] != serde_json::json!({"kind":"not_provided"})
                || lifecycle_wire::validate_command_observation(
                    &component["value"],
                    &component["scope"],
                    kind,
                )
                .is_err()
            {
                return Err(contract_error("command domain binding differs"));
            }
        }
    }
    Ok(lifecycle.clone())
}

fn mutation_lifecycle_projection(host: &Value) -> Result<Value> {
    if !host.get("host_identity").is_some_and(Value::is_object) {
        return Err(contract_error("mutation reply identity missing"));
    }
    match host.get("status").and_then(Value::as_str) {
        Some("answered")
            if host.get("owner").is_some_and(Value::is_object)
                && host.get("lifecycle").is_none() =>
        {
            Ok(
                serde_json::json!({"state":"not_applicable","scope":{"reply_status":"answered"},"reason":"reply_has_no_lifecycle_projection"}),
            )
        }
        Some(
            "admitted"
            | "replacement_requested"
            | "grant_contracted"
            | "rejected_admission"
            | "replacement_rejected"
            | "grant_rejected"
            | "pending_admission"
            | "pending_replacement"
            | "pending_grant_contraction",
        ) => host
            .get("lifecycle")
            .filter(|v| v.is_object())
            .cloned()
            .ok_or_else(|| contract_error("mutation lifecycle evidence missing")),
        _ => Err(contract_error(
            "unrecognized reply is not lifecycle evidence",
        )),
    }
}

fn validate_to2_join(host: &Value, projection: &Value) -> Result<()> {
    if to2_lifecycle_projection(host)? != *projection {
        return Err(contract_error("TO2 lifecycle projection differs"));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactBinding {
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct SourceHash {
    path: String,
    sha256: String,
}

/// Writes the exact installed source scope as compact, path-sorted JSON.
/// The source bytes themselves are read-only; the manifest belongs to proof.
pub fn write_source_manifest(
    repository_root: &Path,
    relative_paths: &[&str],
    destination: &Path,
) -> Result<(ArtifactBinding, Vec<ArtifactBinding>)> {
    let root = fs::canonicalize(repository_root)?;
    if !root.is_dir() || !destination.is_absolute() || relative_paths.is_empty() {
        return Err(ProofError::Contract("invalid source manifest roots".into()));
    }
    let mut pairs = Vec::with_capacity(relative_paths.len());
    for relative in relative_paths {
        let path = Path::new(relative);
        if path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(ProofError::Contract(
                "source path escapes repository".into(),
            ));
        }
        let absolute = fs::canonicalize(root.join(path))?;
        if !absolute.starts_with(&root) || !absolute.is_file() {
            return Err(ProofError::Contract(
                "source path is not a regular file".into(),
            ));
        }
        let binding = ArtifactBinding::read(&absolute)?;
        pairs.push((
            SourceHash {
                path: (*relative).into(),
                sha256: binding.sha256.clone(),
            },
            binding,
        ));
    }
    pairs.sort_by(|a, b| a.0.path.cmp(&b.0.path));
    let (entries, artifacts): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
    if entries.windows(2).any(|pair| pair[0].path == pair[1].path) || destination.exists() {
        return Err(ProofError::Contract("duplicate source or manifest".into()));
    }
    let bytes = serde_json::to_vec(&entries)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok((ArtifactBinding::read(destination)?, artifacts))
}

impl ArtifactBinding {
    pub fn read(path: &Path) -> Result<Self> {
        if !path.is_absolute() || !path.is_file() {
            return Err(ProofError::Contract(
                "artifact path is not an absolute file".into(),
            ));
        }
        Ok(Self {
            path: path.to_path_buf(),
            sha256: sha256_file(path)?,
        })
    }

    pub fn verify(&self) -> Result<()> {
        if sha256_file(&self.path)? != self.sha256 {
            return Err(ProofError::Contract(format!(
                "artifact changed: {}",
                self.path.display()
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildProvenance {
    pub command: Vec<String>,
    pub started_wall_ms: u128,
    pub finished_wall_ms: u128,
    pub exit_code: i32,
    pub executable_sha256: String,
    pub source_manifest_sha256: String,
    pub lock_sha256: String,
    pub toolchain_sha256: String,
    pub features: Vec<String>,
    pub provenance_limit: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub schema: String,
    pub case_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to2_authority: Option<To2ReceiptAuthority>,
    pub contract: ArtifactBinding,
    pub plan: ArtifactBinding,
    pub host_executable: ArtifactBinding,
    pub source_manifest: ArtifactBinding,
    pub sources: Vec<ArtifactBinding>,
    pub cargo_lock: ArtifactBinding,
    pub toolchain: ArtifactBinding,
    pub startup: ArtifactBinding,
    pub build_provenance: BuildProvenance,
    pub build_id: String,
    pub proof_run_id: String,
    pub principal_ref: String,
    pub features: Vec<String>,
    pub profile: String,
    pub provider_identity: String,
    pub disposable_root: PathBuf,
    pub subject_kind: String,
    pub host_identity_before: ProcessIdentity,
    pub host_identity_after: ProcessIdentity,
    pub host_incarnation: String,
    pub provider_ledger: ArtifactBinding,
    pub provider_entries: Vec<ProviderEntry>,
    #[serde(default)]
    pub provider_reply_artifacts: Vec<ArtifactBinding>,
    #[serde(default)]
    pub fault_events: Vec<Value>,
    pub host_observations: Vec<Value>,
    pub lifecycle_observations: Vec<Value>,
    pub close: CloseEvidence,
}

impl EvidenceBundle {
    /// Structural custody checks only. Scenario assertions must separately
    /// inspect ordinary owner/lifecycle observations before calling this.
    pub fn validate_custody(&self) -> Result<()> {
        let scoped = match (self.schema.as_str(), &self.to2_authority) {
            ("s8-external-proof-evidence-v1", None) => false,
            (TO2_RECEIPT_SCHEMA, Some(authority)) => {
                let packet = authority.validate(&self.case_id)?;
                let scenario = packet.scenario(&self.case_id)?;
                if self.plan != packet.to2_plan || self.contract != scenario.contract {
                    return Err(contract_error("scoped receipt authority bindings differ"));
                }
                true
            }
            _ => return Err(contract_error("receipt schema/authority mismatch")),
        };
        if self.case_id.is_empty()
            || self.profile != "retained-repository-analysis-v1"
            || self.provider_identity != "native-sim-proof-v1"
            || self.build_id.is_empty()
            || self.proof_run_id.is_empty()
            || self.principal_ref.is_empty()
            || self.features.is_empty()
            || self.sources.is_empty()
            || !matches!(
                self.subject_kind.as_str(),
                "repository_analysis" | "design_inquiry"
            )
            || !self.disposable_root.is_absolute()
            || self.host_incarnation.is_empty()
            || self.host_identity_before != self.host_identity_after
            || self.host_identity_before.pid != self.close.pid
            || !self.close.reaped
            || self.host_observations.is_empty()
            || self.lifecycle_observations.is_empty()
            || self.provider_entries.is_empty()
            || self.build_provenance.exit_code != 0
            || self.build_provenance.started_wall_ms > self.build_provenance.finished_wall_ms
            || self.build_provenance.command
                != [
                    "cargo",
                    "build",
                    "--offline",
                    "--locked",
                    "-p",
                    "work-engine-task-host",
                    "--features",
                    "controlled-proof",
                ]
            || self.build_provenance.executable_sha256 != self.host_executable.sha256
            || self.build_provenance.source_manifest_sha256 != self.source_manifest.sha256
            || self.build_provenance.lock_sha256 != self.cargo_lock.sha256
            || self.build_provenance.toolchain_sha256 != self.toolchain.sha256
            || self.build_provenance.features != self.features
            || self.build_provenance.provenance_limit.is_empty()
        {
            return Err(ProofError::Contract("evidence custody incomplete".into()));
        }
        for artifact in [
            &self.contract,
            &self.plan,
            &self.host_executable,
            &self.source_manifest,
            &self.cargo_lock,
            &self.toolchain,
            &self.startup,
            &self.provider_ledger,
        ] {
            artifact.verify()?;
        }
        for artifact in &self.provider_reply_artifacts {
            artifact.verify()?;
        }
        if self.host_executable.sha256 != self.host_identity_before.executable_sha256 {
            return Err(ProofError::Contract(
                "installed executable is not the observed host executable".into(),
            ));
        }
        let startup: Value = serde_json::from_slice(&fs::read(&self.startup.path)?)?;
        if startup["executable_sha256"] != self.host_executable.sha256
            || startup["source_sha256"] != self.source_manifest.sha256
            || startup["lock_sha256"] != self.cargo_lock.sha256
            || startup["toolchain_sha256"] != self.toolchain.sha256
            || startup["features"] != serde_json::to_value(&self.features)?
            || startup["provider_identity"] != self.provider_identity
            || startup["profile"] != self.profile
            || startup["lifecycle"]["build_id"] != self.build_id
            || startup["lifecycle"]["proof_run_id"] != self.proof_run_id
            || startup["lifecycle"]["principal"]["principal_ref"] != self.principal_ref
        {
            return Err(ProofError::Contract(
                "startup identity pins mismatch".into(),
            ));
        }
        if self.host_observations.len() != self.lifecycle_observations.len()
            || self
                .host_observations
                .iter()
                .zip(&self.lifecycle_observations)
                .any(|(host, lifecycle)| {
                    (if scoped {
                        validate_to2_join(host, lifecycle).is_err()
                    } else {
                        host["lifecycle"] != *lifecycle
                    }) || host["host_identity"]["incarnation"] != self.host_incarnation
                        || host["host_identity"]["pid"] != self.host_identity_before.pid
                })
        {
            return Err(ProofError::Contract(
                "host and lifecycle observations do not bind".into(),
            ));
        }
        if scoped {
            for observation in &self.host_observations {
                let reported = crate::host::HostReportedIdentity::from_reply(observation)?;
                reported.bind_process(&self.host_identity_before, &self.build_id)?;
                reported.bind_startup(&startup)?;
            }
        }
        for source in &self.sources {
            source.verify()?;
        }
        let listed: Vec<SourceHash> =
            serde_json::from_slice(&fs::read(&self.source_manifest.path)?)?;
        if listed.len() != self.sources.len() {
            return Err(ProofError::Contract(
                "source manifest scope mismatch".into(),
            ));
        }
        for (entry, binding) in listed.iter().zip(&self.sources) {
            if entry.sha256 != binding.sha256 || !binding.path.ends_with(&entry.path) {
                return Err(ProofError::Contract(
                    "source manifest entry mismatch".into(),
                ));
            }
        }
        let data = fs::read_to_string(&self.provider_ledger.path)?;
        let entries: Vec<ProviderEntry> = data
            .lines()
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()?;
        if serde_json::to_value(&entries)? != serde_json::to_value(&self.provider_entries)? {
            return Err(ProofError::Contract(
                "provider ledger does not match receipt".into(),
            ));
        }
        Ok(())
    }

    /// Create one immutable receipt only after all requested close evidence is
    /// present. Existing evidence is never overwritten.
    pub fn write_once(&self, path: &Path) -> Result<String> {
        self.validate_custody()?;
        if !path.is_absolute() {
            return Err(ProofError::Contract("receipt path must be absolute".into()));
        }
        let bytes = serde_json::to_vec_pretty(self)?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        if let Some(parent) = path.parent() {
            OpenOptions::new().read(true).open(parent)?.sync_all()?;
        }
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const CASE: &str = "s8-installed-d0-snapshot-child";

    fn observation() -> Value {
        json!({
            "protocol_version":2,"status":"observation","consistency":"independent_components",
            "request":{"task_id":"task:test"},"host_identity":{},
            "components":{
                "owner":{"state":"present","scope":{"task_id":"task:test","subject_id":"subject:test"},
                    "revision":{"kind":"task","owner_revision":1,"cursor_revision":1},
                    "value":{"task_id":"task:test","subject_id":"subject:test","revision":1,"cursor_revision":1}},
                "lifecycle":{"state":"present","scope":{"subject_id":"subject:test","delivery_id":null},
                    "revision":{"kind":"lifecycle","cursor":{"store_id":"store:test","stream_id":"stream:test","commit_sequence":"1"}},
                    "value":{"protocol_version":1,"subject_id":"subject:test","context_generation":"g",
                        "cursor":{"store_id":"store:test","stream_id":"stream:test","commit_sequence":"1"},
                        "transition":{"state":"unknown"},"current_admission":{"kind":"unknown"},
                        "delivery":{"state":"unknown"},"runtime_availability":"unknown",
                        "runtime_provenance":{"state":"unknown"},"blocking_reason":{"state":"unknown"},
                        "authority_refs":[],"evidence_refs":[],"measurement":{"state":"unknown"}}},
                "native_delivery_status":{"state":"present",
                    "scope":{"subject_id":"subject:test","collection":"native_delivery_segments","filter":"subject"},
                    "revision":{"kind":"not_provided"},"completeness":{"kind":"complete_for_scope","limit":64},
                    "value":{"protocol_version":1,"subject_id":"subject:test","recovery_only":false,"segments":[]}},
                "command_result":{"state":"not_requested","scope":{"subject_id":"subject:test","command_id":null,"task_binding":"not_established"}},
                "control_result":{"state":"not_requested","scope":{"subject_id":"subject:test","command_id":null,"task_binding":"not_established"}}
            }
        })
    }

    #[test]
    fn to2_components_preserve_empty_capacity_and_failure_states() {
        let mut host = observation();
        let projection = to2_lifecycle_projection(&host).unwrap();
        validate_to2_join(&host, &projection).unwrap();
        for count in [0, 64] {
            host["components"]["native_delivery_status"]["value"]["segments"] =
                json!(vec![native_row(); count]);
            assert!(to2_lifecycle_projection(&host).is_ok());
        }
        host["components"]["native_delivery_status"]["value"]["segments"] =
            json!(vec![native_row(); 65]);
        assert!(to2_lifecycle_projection(&host).is_err());
        let scope = host["components"]["native_delivery_status"]["scope"].clone();
        host["components"]["native_delivery_status"] =
            json!({"state":"refused","scope":scope,"reason":"capacity_exceeded","limit":64});
        assert!(to2_lifecycle_projection(&host).is_ok());
        for state in ["unavailable", "refused"] {
            host["components"]["lifecycle"] = json!({"state":state,"scope":{"subject_id":"subject:test","delivery_id":null},"reason":"read_refused"});
            let projection = to2_lifecycle_projection(&host).unwrap();
            assert_eq!(projection["state"], state);
            validate_to2_join(&host, &projection).unwrap();
            assert!(validate_to2_join(&host, &Value::Null).is_err());
        }
    }

    #[test]
    fn absent_and_owner_failure_preserve_dependency_meaning() {
        for state in ["absent", "unavailable", "refused"] {
            let mut host = observation();
            host["components"]["owner"] = if state == "absent" {
                json!({"state":"absent","scope":{"task_id":"task:test"},"record_kind":"task"})
            } else {
                json!({"state":state,"scope":{"task_id":"task:test"},"reason":"owner_unavailable"})
            };
            let (dependent, reason) = if state == "absent" {
                ("not_applicable", "task_absent")
            } else {
                ("unavailable", "task_dependency_unavailable")
            };
            for name in ["lifecycle", "native_delivery_status"] {
                let mut scope = host["components"][name]["scope"].clone();
                scope["subject_id"] = Value::Null;
                host["components"][name] = json!({"state":dependent,"scope":scope,"reason":reason});
            }
            assert_eq!(to2_lifecycle_projection(&host).unwrap()["state"], dependent);
            host["components"]["lifecycle"]["value"] = Value::Null;
            assert!(to2_lifecycle_projection(&host).is_err());
        }
    }

    #[test]
    fn malformed_or_unbound_observations_cannot_join_by_null_equality() {
        let host = observation();
        for pointer in [
            "/components/lifecycle",
            "/components/lifecycle/scope",
            "/components/lifecycle/revision",
            "/components/owner/value",
            "/request/task_id",
        ] {
            let mut changed = host.clone();
            *changed.pointer_mut(pointer).unwrap() = Value::Null;
            assert!(validate_to2_join(&changed, &Value::Null).is_err());
        }
        for (pointer, bad) in [
            ("/protocol_version", json!(1)),
            ("/components/lifecycle/state", json!("future_state")),
            (
                "/components/lifecycle/scope/subject_id",
                json!("subject:other"),
            ),
            (
                "/components/lifecycle/revision/cursor/commit_sequence",
                json!("2"),
            ),
            (
                "/components/native_delivery_status/revision/kind",
                json!("invented"),
            ),
        ] {
            let mut changed = host.clone();
            *changed.pointer_mut(pointer).unwrap() = bad;
            assert!(to2_lifecycle_projection(&changed).is_err());
        }
        assert!(validate_to2_join(&host, &json!({"state":"absent"})).is_err());
        assert!(to2_lifecycle_projection(&json!({"status":"unknown","host_identity":{}})).is_err());
    }

    #[test]
    fn command_domain_unknown_and_rejected_are_present_not_read_failures() {
        for value in [
            json!({"status":"unknown"}),
            json!({"protocol_version":2,"command_id":"command:test","accepted":false,"outcome_kind":"rejected","outcome_ref":null,"rejection_code":"rejected","resulting_revision":"1"}),
        ] {
            let mut host = observation();
            host["request"]["command_id"] = json!("command:test");
            host["components"]["command_result"] = json!({"state":"present",
                "scope":{"subject_id":"subject:test","command_id":"command:test","task_binding":"not_established","request_digest":"a".repeat(64)},
                "revision":{"kind":"not_provided"},"value":value});
            host["components"]["control_result"] = json!({"state":"absent",
                "scope":{"subject_id":"subject:test","command_id":"command:test","task_binding":"not_established"},
                "record_kind":"control_command_locator"});
            assert!(to2_lifecycle_projection(&host).is_ok());
            host["components"]["command_result"]["scope"]["task_binding"] = json!("inferred");
            assert!(to2_lifecycle_projection(&host).is_err());
        }
    }

    #[test]
    fn answer_reply_has_explicit_non_lifecycle_projection() {
        let answer = json!({"status":"answered","owner":{},"host_identity":{}});
        let projection = to2_lifecycle_projection(&answer).unwrap();
        assert_eq!(projection["state"], "not_applicable");
        assert!(validate_to2_join(&answer, &Value::Null).is_err());
        let mut missing = answer;
        missing.as_object_mut().unwrap().remove("owner");
        assert!(to2_lifecycle_projection(&missing).is_err());
    }

    fn artifact(root: &Path, name: &str, value: &[u8]) -> ArtifactBinding {
        let path = root.join(name);
        fs::write(&path, value).unwrap();
        ArtifactBinding::read(&path).unwrap()
    }

    fn authority() -> To2ReceiptAuthority {
        to2_authority_from_environment(CASE)
            .unwrap()
            .expect("unit gate requires issued packet")
    }

    #[test]
    fn scoped_authority_rejects_wrong_case_digest_scope_and_original_plan_claim() {
        let auth = authority();
        let packet = auth.validate(CASE).unwrap();
        assert!(auth.validate("full-s8").is_err());
        let mut wrong = auth.clone();
        wrong.scope = "full_s8_qualification".into();
        assert!(wrong.validate(CASE).is_err());
        wrong = auth;
        wrong.packet.sha256 = "0".repeat(64);
        assert!(wrong.validate(CASE).is_err());
        let mut wrong = packet.clone();
        wrong.original_plan.status = "verified".into();
        assert!(wrong.validate(CASE).is_err());
        wrong = packet.clone();
        wrong.to2_plan.sha256 = ORIGINAL_PLAN_SHA.into();
        assert!(wrong.validate(CASE).is_err());
        wrong = packet.clone();
        wrong.issuance = "pending_supervisor_verification".into();
        assert!(wrong.validate(CASE).is_err());
        wrong = packet;
        wrong.scenarios[0].contract.path = PathBuf::from("/unavailable-scoped-scenario-basis");
        assert!(wrong.validate(CASE).is_err());
    }

    fn bundle(root: &Path, scoped: bool) -> EvidenceBundle {
        let auth = authority();
        let packet = auth.validate(CASE).unwrap();
        let executable = artifact(root, "host", b"synthetic executable");
        let lock = artifact(root, "lock", b"synthetic lock");
        let toolchain = artifact(root, "toolchain", b"synthetic toolchain");
        let source = artifact(root, "source.rs", b"synthetic source");
        let source_manifest = artifact(
            root,
            "sources.json",
            &serde_json::to_vec(&vec![SourceHash {
                path: "source.rs".into(),
                sha256: source.sha256.clone(),
            }])
            .unwrap(),
        );
        let identity = ProcessIdentity {
            pid: 42,
            linux_starttime_ticks: 7,
            executable: executable.path.clone(),
            executable_sha256: executable.sha256.clone(),
        };
        let startup = artifact(root,"startup.json",&serde_json::to_vec(&json!({
            "executable_sha256":executable.sha256,"source_sha256":source_manifest.sha256,
            "lock_sha256":lock.sha256,"toolchain_sha256":toolchain.sha256,"features":["controlled-proof"],
            "provider_identity":"native-sim-proof-v1","profile":"retained-repository-analysis-v1",
            "lifecycle":{"build_id":"build:test","proof_run_id":"proof:test","principal":{"principal_ref":"principal:test"}}
        })).unwrap());
        let entry = ProviderEntry {
            ordinal: 1,
            request_sha256: "digest".into(),
            request: json!({}),
            reply_kind: "text".into(),
        };
        let ledger = artifact(root, "ledger", &serde_json::to_vec(&entry).unwrap());
        let mut host = observation();
        host["host_identity"] = json!({"pid":42,"linux_starttime_ticks":"7","incarnation":"host:test",
            "build_id":"build:test","profile":"retained-repository-analysis-v1",
            "executable_sha256":executable.sha256,"source_sha256":source_manifest.sha256,
            "lock_sha256":lock.sha256,"toolchain_sha256":toolchain.sha256,
            "features":["controlled-proof"],"provider_identity":"native-sim-proof-v1"});
        let projection = if scoped {
            to2_lifecycle_projection(&host).unwrap()
        } else {
            host = json!({"lifecycle":{"historical":true},"host_identity":{"pid":42,"incarnation":"host:test"}});
            host["lifecycle"].clone()
        };
        EvidenceBundle {
            schema: if scoped {
                TO2_RECEIPT_SCHEMA
            } else {
                "s8-external-proof-evidence-v1"
            }
            .into(),
            case_id: CASE.into(),
            to2_authority: scoped.then_some(auth),
            contract: packet.scenario(CASE).unwrap().contract.clone(),
            plan: packet.to2_plan,
            host_executable: executable.clone(),
            source_manifest: source_manifest.clone(),
            sources: vec![source],
            cargo_lock: lock.clone(),
            toolchain: toolchain.clone(),
            startup,
            build_provenance: BuildProvenance {
                command: [
                    "cargo",
                    "build",
                    "--offline",
                    "--locked",
                    "-p",
                    "work-engine-task-host",
                    "--features",
                    "controlled-proof",
                ]
                .map(str::to_string)
                .to_vec(),
                started_wall_ms: 1,
                finished_wall_ms: 2,
                exit_code: 0,
                executable_sha256: executable.sha256,
                source_manifest_sha256: source_manifest.sha256,
                lock_sha256: lock.sha256,
                toolchain_sha256: toolchain.sha256,
                features: vec!["controlled-proof".into()],
                provenance_limit: "synthetic unit fixture".into(),
            },
            build_id: "build:test".into(),
            proof_run_id: "proof:test".into(),
            principal_ref: "principal:test".into(),
            features: vec!["controlled-proof".into()],
            profile: "retained-repository-analysis-v1".into(),
            provider_identity: "native-sim-proof-v1".into(),
            disposable_root: root.to_path_buf(),
            subject_kind: "repository_analysis".into(),
            host_identity_before: identity.clone(),
            host_identity_after: identity,
            host_incarnation: "host:test".into(),
            provider_ledger: ledger,
            provider_entries: vec![entry],
            provider_reply_artifacts: vec![],
            fault_events: vec![],
            host_observations: vec![host],
            lifecycle_observations: vec![projection],
            close: CloseEvidence {
                pid: 42,
                exit_code: Some(0),
                signal: None,
                reaped: true,
            },
        }
    }

    #[test]
    fn scoped_custody_checks_identity_reap_artifacts_and_write_once() {
        let root = tempfile::tempdir().unwrap();
        let valid = bundle(root.path(), true);
        valid.validate_custody().unwrap();
        valid.write_once(&root.path().join("receipt.json")).unwrap();
        assert!(valid.write_once(&root.path().join("receipt.json")).is_err());
        let mut bad = valid.clone();
        bad.close.reaped = false;
        assert!(bad.validate_custody().is_err());
        bad = valid.clone();
        bad.host_observations[0]["host_identity"]["linux_starttime_ticks"] = json!("8");
        assert!(bad.validate_custody().is_err());
        bad = valid.clone();
        bad.startup.sha256 = "0".repeat(64);
        assert!(bad.validate_custody().is_err());
        bad = valid.clone();
        bad.provider_entries[0].request = json!({"altered":true});
        assert!(bad.validate_custody().is_err());
        fs::write(&valid.sources[0].path, b"changed").unwrap();
        assert!(valid.validate_custody().is_err());
    }

    #[test]
    fn historical_schema_and_validator_remain_distinct() {
        let root = tempfile::tempdir().unwrap();
        let mut old = bundle(root.path(), false);
        old.validate_custody().unwrap();
        assert!(
            serde_json::to_value(&old)
                .unwrap()
                .get("to2_authority")
                .is_none()
        );
        old.schema = TO2_RECEIPT_SCHEMA.into();
        assert!(old.validate_custody().is_err());
        old.schema = "s8-external-proof-evidence-v1".into();
        old.to2_authority = Some(authority());
        assert!(old.validate_custody().is_err());
    }

    fn native_row() -> Value {
        json!({"delivery_id":"d","input_id":"i","effect_id":"e","queue_attempt_id":"q","invocation_id":"v","native_attempt_id":"n","thread_id":"t","context_generation":"g","delivery_state":"active","segment_state":"entered","native_state":"entered_uncertain","history_status":"unavailable","predecessor_invocation_id":null,"transition_id":null,"turn_id":null,"final_sha256":null,"sequence":1,"segment_no":0,"send_intent":true})
    }
    #[test]
    fn equal_malformed_values_are_not_evidence() {
        let host = observation();
        for (path, value) in [
            (
                "/components/lifecycle/value/context_generation",
                Value::Null,
            ),
            (
                "/components/lifecycle/value/transition",
                json!({"state":"future"}),
            ),
            (
                "/components/lifecycle/value/cursor/commit_sequence",
                json!("no"),
            ),
            (
                "/components/native_delivery_status/value/segments",
                json!([{}]),
            ),
            ("/components/native_delivery_status/value/segments", {
                let mut row = native_row();
                row["send_intent"] = json!("yes");
                json!([row])
            }),
            ("/components/native_delivery_status/value/segments", {
                let mut row = native_row();
                row.as_object_mut().unwrap().remove("turn_id");
                json!([row])
            }),
        ] {
            let mut bad = host.clone();
            *bad.pointer_mut(path).unwrap() = value;
            if path.contains("/cursor/") {
                bad["components"]["lifecycle"]["revision"]["cursor"] =
                    bad["components"]["lifecycle"]["value"]["cursor"].clone();
            }
            let identical = bad["components"]["lifecycle"].clone();
            assert!(validate_to2_join(&bad, &identical).is_err());
        }
        let mut bad = host.clone();
        bad["components"]["lifecycle"]["value"]
            .as_object_mut()
            .unwrap()
            .remove("delivery");
        assert!(validate_to2_join(&bad, &bad["components"]["lifecycle"]).is_err());
        for state in ["unknown", "not_applicable", "unavailable"] {
            let mut valid = host.clone();
            valid["components"]["lifecycle"]["value"]["transition"] = json!({"state":state});
            assert!(to2_lifecycle_projection(&valid).is_ok());
        }
        let mut valid = host;
        valid["components"]["lifecycle"]["value"]["transition"] = json!({"state":"known","value":{"transition_id":"transition:t","stage":"quiescing","completed":false}});
        assert!(to2_lifecycle_projection(&valid).is_ok());
    }
    #[test]
    fn malformed_command_values_do_not_qualify_receipt_joins() {
        for value in [
            json!({"status":"unknown","command_id":"other"}),
            json!({"protocol_version":999,"command_id":"command:test","accepted":true}),
            json!({"protocol_version":2,"command_id":"command:test","accepted":"yes"}),
        ] {
            let mut host = observation();
            host["request"]["command_id"] = json!("command:test");
            host["components"]["command_result"] = json!({"state":"present","scope":{"subject_id":"subject:test","command_id":"command:test","task_binding":"not_established","request_digest":"a".repeat(64)},"revision":{"kind":"not_provided"},"value":value});
            host["components"]["control_result"] = json!({"state":"absent","scope":{"subject_id":"subject:test","command_id":"command:test","task_binding":"not_established"},"record_kind":"control_command_locator"});
            assert!(validate_to2_join(&host, &host["components"]["lifecycle"]).is_err());
        }
    }
}
