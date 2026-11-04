//! One bounded subscription-provider lane. Rust owns admission and reservations.
use super::*;
use std::os::unix::fs::OpenOptionsExt;
pub const LIVE_PROFILE: &str = "work-engine.pi-coding-sdk.v1";
pub const ACCESS_ADAPTER: &[u8] = include_bytes!("../receiver/pi-native-access.mjs");
const CATALOG: &str = "node_modules/@earendil-works/pi-ai/dist/providers/data/openai-codex.json";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderPlan {
    pub lane: auth::LaneInputs,
    pub task: Artifact,
    pub catalog: Artifact,
    pub access_adapter: Artifact,
    pub model: String,
    pub max_requests: u32,
    pub budget_microusd: u64,
    pub max_context_bytes: u64,
}
impl std::fmt::Debug for ProviderPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderPlan")
            .field("model", &self.model)
            .field("max_requests", &self.max_requests)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Reservation {
    pub reserved_microusd: u64,
    pub settled_microusd: Option<u64>,
    pub backend_model: Option<String>,
    pub http_status: Option<u64>,
    pub usage: Option<Value>,
    #[serde(default)]
    pub raw_terminal: Option<Value>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Admission {
    schema: String,
    job_id: String,
    binary_sha256: String,
    model: String,
    native_auth_source: PathBuf,
    pi_config_manifest_sha256: String,
    task_sha256: String,
    catalog_sha256: String,
    max_requests: u32,
    budget_microusd: u64,
    allow_native_access_projection: bool,
    allow_provider: bool,
}
impl ProviderPlan {
    pub(super) fn selected(&self) -> Result<Value> {
        let data: Value = serde_json::from_slice(&self.catalog.verify()?)?;
        let selected = data["openai-codex-responses"][format!("chat:{}", self.model)].clone();
        if selected["id"] != self.model
            || selected["provider"] != "openai-codex"
            || selected["api"] != "openai-codex-responses"
            || selected["baseUrl"] != "https://chatgpt.com/backend-api"
        {
            return refuse("exact subscription catalog model absent");
        }
        Ok(selected)
    }
    pub(super) fn validate(&self, job: &Job) -> Result<()> {
        if self.model != "gpt-6-luna"
            || !(1..=6).contains(&self.max_requests)
            || !(1..=1_000_000).contains(&self.budget_microusd)
            || !(1024..=65536).contains(&self.max_context_bytes)
            || self.catalog.path != job.runtime.root.join(CATALOG)
            || self.access_adapter.verify()? != ACCESS_ADAPTER
            || !self.access_adapter.path.is_absolute()
            || fs::canonicalize(&self.access_adapter.path)? != self.access_adapter.path
            || self
                .access_adapter
                .path
                .to_string_lossy()
                .contains([',', ':', '\n', '\r'])
        {
            return refuse("provider lane/catalog/limits conflict");
        }
        self.lane.validate_references()?;
        if self.lane.native_auth_source.starts_with(&job.state_dir)
            || self.lane.native_auth_source.starts_with(&job.runtime.root)
            || self
                .lane
                .native_auth_source
                .starts_with(&job.workspace.root)
            || job
                .coding
                .as_ref()
                .is_some_and(|p| self.lane.native_auth_source.starts_with(&p.toolchain.root))
            || self.lane.pi_config.root.starts_with(&job.state_dir)
            || job.state_dir.starts_with(&self.lane.pi_config.root)
        {
            return refuse("provider private/public mount references overlap");
        }
        // This bounded lane admits public Pi settings, not custom providers or
        // executable resources. Never silently load a caller-supplied endpoint/key.
        for (path, entry) in inventory(&self.lane.pi_config.root)? {
            if !matches!(entry, Entry::File { .. })
                || !matches!(path.as_str(), "settings.json" | "models.json")
            {
                return refuse("unsupported lane configuration resource");
            }
            if fs::metadata(self.lane.pi_config.root.join(&path))?.len() > 65536 {
                return refuse("public config file exceeded bound");
            }
            let value: Value =
                serde_json::from_slice(&fs::read(self.lane.pi_config.root.join(&path))?)?;
            if !value.is_object() || (path == "models.json" && value != json!({})) {
                return refuse(
                    "custom model/provider configuration unsupported in subscription lane",
                );
            }
        }
        if self.task.verify()?.len() > 32768 {
            return refuse("provider task input exceeded bound");
        }
        let selected = self.selected()?;
        let reserve = full_reservation(&selected)?;
        if reserve
            .checked_mul(self.max_requests.into())
            .is_none_or(|v| v > self.budget_microusd)
        {
            return refuse("full catalog request reservations exceed runtime proxy budget");
        }
        let admission: Admission = serde_json::from_slice(&job.authority.verify()?)
            .map_err(|_| Error::Refused("exact provider runtime admission required".into()))?;
        if admission.schema != "work-engine.pi-coding-runtime-admission.v1"
            || admission.job_id != job.job_id
            || admission.binary_sha256 != file_digest(&std::env::current_exe()?, None)?
            || admission.model != self.model
            || admission.native_auth_source != self.lane.native_auth_source
            || admission.pi_config_manifest_sha256 != self.lane.pi_config.manifest.sha256
            || admission.task_sha256 != self.task.sha256
            || admission.catalog_sha256 != self.catalog.sha256
            || admission.max_requests != self.max_requests
            || admission.budget_microusd != self.budget_microusd
            || !admission.allow_native_access_projection
            || !admission.allow_provider
        {
            return refuse("provider admission does not bind this exact realization");
        }
        Ok(())
    }
}
fn rates(model: &Value) -> Result<(f64, f64)> {
    let mut input = 0.0_f64;
    let mut output = 0.0_f64;
    let mut entries = vec![&model["cost"]];
    if let Some(tiers) = model["cost"]["tiers"].as_array() {
        entries.extend(tiers);
    }
    for cost in entries {
        for key in ["input", "cacheRead", "cacheWrite"] {
            let value = cost[key]
                .as_f64()
                .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 1000.0)
                .ok_or_else(|| Error::Refused("catalog input rate absent/invalid".into()))?;
            input = input.max(value);
        }
        output = output.max(
            cost["output"]
                .as_f64()
                .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 1000.0)
                .ok_or_else(|| Error::Refused("catalog output rate absent/invalid".into()))?,
        );
    }
    Ok((input, output))
}
fn cost(model: &Value, input: u64, output: u64) -> Result<u64> {
    let (input_rate, output_rate) = rates(model)?;
    // USD/million * tokens == microUSD. One additional microUSD rounds upward
    // conservatively even if floating catalog representation lands just below.
    let value = (input as f64 * input_rate + output as f64 * output_rate).ceil();
    if !value.is_finite() || !(0.0..=1_000_000_000.0).contains(&value) {
        return refuse("catalog cost overflow");
    }
    Ok(value as u64 + 1)
}
pub(super) fn full_reservation(model: &Value) -> Result<u64> {
    let input = model["contextWindow"]
        .as_u64()
        .filter(|v| *v > 0 && *v <= 2_000_000)
        .ok_or_else(|| Error::Refused("catalog context bound absent".into()))?;
    let output = model["maxTokens"]
        .as_u64()
        .filter(|v| *v > 0 && *v <= input)
        .ok_or_else(|| Error::Refused("catalog output bound absent".into()))?;
    cost(model, input, output)
}
impl Host {
    pub(super) fn coding_prepare_provider(&mut self) -> Result<()> {
        let profile = self.journal.job.coding.as_ref().unwrap();
        let Some(plan) = &profile.provider else {
            return Ok(());
        };
        plan.validate(&self.journal.job)?;
        let private = self.journal.job.state_dir.join("private-access");
        fs::create_dir(&private)?;
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700))?;
        if !self.signed_subscription() {
            self.coding_project_provider_access(None)?;
        }
        let config = self.journal.job.state_dir.join("coding-public-config");
        fs::create_dir(&config)?;
        for (path, entry) in inventory(&plan.lane.pi_config.root)? {
            let Entry::File { sha256 } = entry else {
                return refuse("public config symlink refused");
            };
            let bytes = fs::read(plan.lane.pi_config.root.join(&path))?;
            if digest(&bytes) != sha256 {
                return refuse("public config changed while staging");
            }
            fs::write(config.join(path), bytes)?;
        }
        let public = self.journal.job.state_dir.join("coding-public");
        fs::create_dir(&public)?;
        let task = String::from_utf8(plan.task.verify()?)
            .map_err(|_| Error::Refused("task is not UTF8".into()))?;
        atomic(
            &public.join("job.json"),
            &json!({"schema":if self.signed_subscription() {super::SIGNED_SUBSCRIPTION_PROFILE} else {LIVE_PROFILE},"provider":"openai-codex","model":plan.model,"selected":plan.selected()?,"task":task,"deadline":self.journal.job.deadline_unix_seconds,"maxRequests":plan.max_requests,"maxContextBytes":plan.max_context_bytes}),
        )?;
        Ok(())
    }
    // Caller must hold current Initialization lease for signed jobs. This
    // private operation is never exposed as a request-selected callback.
    pub(super) fn coding_project_provider_access(
        &self,
        lease: Option<&session_task_admission::pi_pilot::EffectLease>,
    ) -> Result<()> {
        if self.signed_subscription() && lease.is_none() {
            return refuse("signed access requires initialization lease");
        }
        let plan = self
            .journal
            .job
            .coding
            .as_ref()
            .and_then(|p| p.provider.as_ref())
            .ok_or_else(|| Error::Refused("provider plan absent".into()))?;
        plan.validate(&self.journal.job)?;
        let private = self.journal.job.state_dir.join("private-access");
        // Only this explicit runtime-admitted branch reads the original native
        // source. It never writes it or copies refresh/id tokens. No digest/log.
        if let Some(lease) = lease {
            lease
                .check_current()
                .map_err(|e| Error::Refused(e.into()))?;
        }
        let metadata = fs::symlink_metadata(&plan.lane.native_auth_source)?;
        if !metadata.is_file()
            || metadata.len() > 65536
            || metadata.permissions().mode() & 0o077 != 0
        {
            return refuse("native credential source metadata invalid");
        }
        use std::io::Read;
        use std::os::unix::fs::MetadataExt;
        if let Some(lease) = lease {
            lease
                .check_current()
                .map_err(|e| Error::Refused(e.into()))?;
        }
        let source = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&plan.lane.native_auth_source)?;
        let before = source.metadata()?;
        let mut raw = Vec::new();
        (&source).take(65537).read_to_end(&mut raw)?;
        let after = source.metadata()?;
        if raw.len() > 65536
            || before.len() != after.len()
            || before.mtime() != after.mtime()
            || before.mtime_nsec() != after.mtime_nsec()
            || before.ino() != metadata.ino()
        {
            return refuse("native source changed during bounded read");
        }
        let grant =
            auth::project_native_access(&raw, now(), self.journal.job.deadline_unix_seconds)?;
        if let Some(lease) = lease {
            lease
                .check_current()
                .map_err(|e| Error::Refused(e.into()))?;
        }
        grant.stage(&private)?;
        Ok(())
    }
    pub(super) fn coding_provider_frame(&mut self, frame: &Value) -> Result<Option<Value>> {
        if self.signed_pi() && !self.signed_subscription() {
            return self.coding_fake_provider_frame(frame);
        }
        let plan = self
            .journal
            .job
            .coding
            .as_ref()
            .unwrap()
            .provider
            .as_ref()
            .ok_or_else(|| Error::Refused("provider frame in disabled fixture".into()))?
            .clone();
        let model = plan.selected()?;
        let id = frame["id"]
            .as_u64()
            .filter(|v| *v >= 1 && *v <= u64::from(plan.max_requests))
            .ok_or_else(|| Error::Refused("provider request id invalid".into()))?;
        match frame["type"].as_str().unwrap_or("") {
            "provider_request" => {
                if self.expired() || self.cancelled() {
                    return refuse("provider request expired/cancelled");
                }
                let state = self.journal.coding.as_mut().unwrap();
                if state.provider_failed
                    || state.provider_requests.contains_key(&id)
                    || id != state.provider_requests.len() as u64 + 1
                    || state
                        .provider_requests
                        .values()
                        .any(|r| r.settled_microusd.is_none())
                    || frame["model"] != plan.model
                    || frame["provider"] != "openai-codex"
                    || frame["contextBytes"]
                        .as_u64()
                        .is_none_or(|v| v > plan.max_context_bytes)
                {
                    return refuse("provider identity/context/unknown reservation conflict");
                }
                let reserve = full_reservation(&model)?;
                let spent: u64 = state
                    .provider_requests
                    .values()
                    .map(|r| r.settled_microusd.unwrap_or(r.reserved_microusd))
                    .sum();
                if spent
                    .checked_add(reserve)
                    .is_none_or(|v| v > plan.budget_microusd)
                {
                    return refuse("provider budget exhausted");
                }
                state.provider_requests.insert(
                    id,
                    Reservation {
                        reserved_microusd: reserve,
                        ..Reservation::default()
                    },
                );
                self.save()?; // Durable unknown reservation BEFORE granting possible network entry.
                if self.signed_subscription() {
                    let owner = self
                        .pi_owner
                        .as_ref()
                        .ok_or_else(|| Error::Refused("Pi owner absent".into()))?;
                    let lease = owner
                        .enter(session_task_admission::pi_pilot::Effect::Provider(id))
                        .map_err(|e| Error::Refused(e.into()))?;
                    lease
                        .check_current()
                        .map_err(|e| Error::Refused(e.into()))?;
                    self.reply_leases.push(lease);
                }
                Ok(Some(
                    json!({"type":"provider_grant","id":id,"reservedMicrousd":reserve}),
                ))
            }
            "provider_observation" => {
                let state = self.journal.coding.as_mut().unwrap();
                let r = state
                    .provider_requests
                    .get_mut(&id)
                    .ok_or_else(|| Error::Refused("provider observation before intent".into()))?;
                if r.settled_microusd.is_some() {
                    return refuse("provider observation after settlement");
                }
                if let Some(status) = frame["httpStatus"].as_u64()
                    && r.http_status.replace(status).is_some()
                {
                    return refuse("duplicate provider HTTP response");
                }
                if let Some(name) = frame["backendModel"].as_str() {
                    if name != plan.model
                        || r.backend_model.as_ref().is_some_and(|prior| prior != name)
                    {
                        return refuse("backend model identity conflict");
                    }
                    r.backend_model = Some(name.into());
                }
                if let Some(raw) = frame.get("rawTerminal")
                    && r.raw_terminal.replace(raw.clone()).is_some()
                {
                    return refuse("duplicate raw terminal evidence");
                }
                self.save()?;
                Ok(None)
            }
            "provider_complete" => {
                let state = self.journal.coding.as_mut().unwrap();
                let r = state
                    .provider_requests
                    .get_mut(&id)
                    .ok_or_else(|| Error::Refused("provider completion before intent".into()))?;
                if r.settled_microusd.is_some() {
                    return refuse("duplicate provider completion");
                }
                let usage = &frame["usage"];
                r.usage = Some(usage.clone());
                if !matches!(frame["stopReason"].as_str(), Some("stop" | "toolUse"))
                    || r.http_status != Some(200)
                    || r.backend_model.as_deref() != Some(&plan.model)
                {
                    state.provider_failed = true;
                    self.save()?;
                    return refuse("provider result unknown/error; reservation retained");
                }
                let raw = r.raw_terminal.as_ref().filter(|raw| {
                    raw["eventType"] == "response.completed" && raw["status"] == "completed"
                });
                let measured = raw.and_then(|raw| {
                    Some((
                        raw["usage"]["input_tokens"].as_u64()?,
                        raw["usage"]["output_tokens"].as_u64()?,
                        raw["usage"]["total_tokens"].as_u64()?,
                    ))
                });
                let Some((input, output, total)) = measured else {
                    state.provider_failed = true;
                    self.save()?;
                    return refuse(
                        "raw completed status/usage unavailable; full reservation retained",
                    );
                };
                if input.checked_add(output) != Some(total) {
                    state.provider_failed = true;
                    self.save()?;
                    return refuse("raw usage inconsistent; full reservation retained");
                }
                let actual = cost(&model, input, output)?;
                if input > model["contextWindow"].as_u64().unwrap_or(0)
                    || output > model["maxTokens"].as_u64().unwrap_or(0)
                    || actual > r.reserved_microusd
                {
                    state.provider_failed = true;
                    self.save()?;
                    return refuse("provider usage exceeds admitted catalog bound");
                }
                r.settled_microusd = Some(actual);
                self.save()?;
                Ok(None)
            }
            _ => refuse("unknown provider protocol frame"),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_catalog_reservation_covers_cache_and_higher_tier_rates() {
        let model = json!({"contextWindow":272000,"maxTokens":128000,"cost":{"input":0.1,"output":0.5,"cacheRead":0.01,"cacheWrite":0.125,"tiers":[{"input":0.2,"output":0.75,"cacheRead":0.02,"cacheWrite":0.25}]}});
        assert_eq!(full_reservation(&model).unwrap(), 164001);
        assert_eq!(full_reservation(&model).unwrap() * 6, 984006);
        assert!(full_reservation(&json!({"contextWindow":272000,"maxTokens":128000})).is_err());
    }
}

#[cfg(test)]
mod ledger_tests {
    use super::*;
    pub(super) fn fixture() -> (tempfile::TempDir, Host) {
        let (dir, mut h) = super::super::tests::fixture();
        let path = dir.path().join("catalog.json");
        let bytes=serde_json::to_vec(&json!({"openai-codex-responses":{"chat:gpt-6-luna":{"id":"gpt-6-luna","provider":"openai-codex","api":"openai-codex-responses","baseUrl":"https://chatgpt.com/backend-api","contextWindow":272000,"maxTokens":128000,"cost":{"input":0.1,"output":0.5,"cacheRead":0.01,"cacheWrite":0.125,"tiers":[{"input":0.2,"output":0.75,"cacheRead":0.02,"cacheWrite":0.25}]}}}})).unwrap();
        fs::write(&path, &bytes).unwrap();
        let pin = Artifact {
            path,
            sha256: digest(&bytes),
        };
        let tree = Tree {
            root: dir.path().join("unused"),
            manifest: pin.clone(),
        };
        h.journal.job.coding.as_mut().unwrap().provider = Some(ProviderPlan {
            lane: auth::LaneInputs {
                pi_config: tree,
                native_auth_source: dir.path().join("not-read"),
            },
            task: pin.clone(),
            catalog: pin.clone(),
            access_adapter: pin,
            model: "gpt-6-luna".into(),
            max_requests: 6,
            budget_microusd: 1_000_000,
            max_context_bytes: 65536,
        });
        (dir, h)
    }
    fn request(id: u64) -> Value {
        json!({"type":"provider_request","id":id,"provider":"openai-codex","model":"gpt-6-luna","contextBytes":1000})
    }
    #[test]
    fn reservation_is_durable_before_grant_and_unknown_cannot_replay() {
        let (dir, mut h) = fixture();
        let grant = h.coding_provider_frame(&request(1)).unwrap().unwrap();
        assert_eq!(grant["reservedMicrousd"], 164001);
        let record: Value =
            serde_json::from_slice(&fs::read(dir.path().join("pi-state.json")).unwrap()).unwrap();
        assert_eq!(
            record["coding"]["provider_requests"]["1"]["reserved_microusd"],
            164001
        );
        assert!(record["coding"]["provider_requests"]["1"]["settled_microusd"].is_null());
        assert!(h.coding_provider_frame(&request(1)).is_err());
        assert!(h.coding_provider_frame(&request(2)).is_err());
        assert!(h.journal.processes.is_empty());
    }
    #[test]
    fn source_bound_usage_settles_and_admits_next_finite_request() {
        let (_dir, mut h) = fixture();
        h.coding_provider_frame(&request(1)).unwrap();
        h.coding_provider_frame(&json!({"type":"provider_observation","id":1,"httpStatus":200,"backendModel":"gpt-6-luna","rawTerminal":{"eventType":"response.completed","status":"completed","usage":{"input_tokens":1000,"output_tokens":100,"total_tokens":1100}}})).unwrap();
        h.coding_provider_frame(&json!({"type":"provider_complete","id":1,"stopReason":"toolUse","usage":{"input":1000,"output":100,"cacheRead":0,"cacheWrite":0}})).unwrap();
        assert_eq!(
            h.journal.coding.as_ref().unwrap().provider_requests[&1].settled_microusd,
            Some(326)
        );
        assert!(h.coding_provider_frame(&request(2)).unwrap().is_some());
        assert!(h.coding_provider_frame(&request(7)).is_err());
    }
    #[test]
    fn error_unknown_identity_and_over_budget_never_regrant() {
        let (_dir, mut h) = fixture();
        h.coding_provider_frame(&request(1)).unwrap();
        assert!(
            h.coding_provider_frame(
                &json!({"type":"provider_complete","id":1,"stopReason":"error","usage":{}})
            )
            .is_err()
        );
        assert!(
            h.journal.coding.as_ref().unwrap().provider_requests[&1]
                .settled_microusd
                .is_none()
        );
        assert!(h.coding_provider_frame(&request(2)).is_err());
        let (_dir, mut h) = fixture();
        h.journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .provider
            .as_mut()
            .unwrap()
            .budget_microusd = 100;
        assert!(h.coding_provider_frame(&request(1)).is_err());
        assert!(
            h.journal
                .coding
                .as_ref()
                .unwrap()
                .provider_requests
                .is_empty()
        );
        let (_dir, mut h) = fixture();
        let mut wrong = request(1);
        wrong["model"] = json!("an-alias");
        assert!(h.coding_provider_frame(&wrong).is_err());
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    #[test]
    fn missing_runtime_admission_refuses_before_loading_even_fake_native_cache() {
        let (dir, mut h) = super::super::tests::fixture();
        let root = dir.path();
        let runtime = root.join("runtime");
        let catalog_path = runtime.join(CATALOG);
        fs::create_dir_all(catalog_path.parent().unwrap()).unwrap();
        let catalog=serde_json::to_vec(&json!({"openai-codex-responses":{"chat:gpt-6-luna":{"id":"gpt-6-luna","provider":"openai-codex","api":"openai-codex-responses","baseUrl":"https://chatgpt.com/backend-api","contextWindow":272000,"maxTokens":128000,"cost":{"input":0.1,"output":0.5,"cacheRead":0.01,"cacheWrite":0.125,"tiers":[{"input":0.2,"output":0.75,"cacheRead":0.02,"cacheWrite":0.25}]}}}})).unwrap();
        fs::write(&catalog_path, &catalog).unwrap();
        let config = root.join("config");
        fs::create_dir(&config).unwrap();
        let manifest = root.join("config.json");
        let bytes = serde_json::to_vec(&inventory(&config).unwrap()).unwrap();
        fs::write(&manifest, &bytes).unwrap();
        let native = root.join("native.json");
        fs::write(&native, b"NOT_A_NATIVE_CACHE_MUST_NOT_LOAD").unwrap();
        fs::set_permissions(&native, fs::Permissions::from_mode(0o600)).unwrap();
        let adapter = root.join("adapter.mjs");
        fs::write(&adapter, ACCESS_ADAPTER).unwrap();
        let task = root.join("task");
        fs::write(&task, b"fixture task").unwrap();
        let authority = root.join("authority");
        fs::write(&authority, b"no provider admission").unwrap();
        h.journal.job.runtime.root = runtime;
        h.journal.job.state_dir = root.join("state");
        fs::create_dir(&h.journal.job.state_dir).unwrap();
        h.journal.job.authority = Artifact {
            path: authority,
            sha256: digest(b"no provider admission"),
        };
        h.journal.job.coding.as_mut().unwrap().provider = Some(ProviderPlan {
            lane: auth::LaneInputs {
                pi_config: Tree {
                    root: config,
                    manifest: Artifact {
                        path: manifest,
                        sha256: digest(&bytes),
                    },
                },
                native_auth_source: native.clone(),
            },
            task: Artifact {
                path: task,
                sha256: digest(b"fixture task"),
            },
            catalog: Artifact {
                path: catalog_path,
                sha256: digest(&catalog),
            },
            access_adapter: Artifact {
                path: adapter,
                sha256: digest(ACCESS_ADAPTER),
            },
            model: "gpt-6-luna".into(),
            max_requests: 6,
            budget_microusd: 1000000,
            max_context_bytes: 65536,
        });
        let error = h.coding_prepare_provider().unwrap_err();
        assert!(error.to_string().contains("runtime admission"));
        assert!(!h.journal.job.state_dir.join("private-access").exists());
        assert!(h.journal.processes.is_empty());
        assert_eq!(
            fs::read(native).unwrap(),
            b"NOT_A_NATIVE_CACHE_MUST_NOT_LOAD"
        );
    }
}

#[cfg(test)]
mod raw_evidence_tests {
    use super::*;
    #[test]
    fn sdk_zero_defaults_cannot_settle_missing_incomplete_or_nonterminal_raw_evidence() {
        for raw in [
            Value::Null,
            json!({"eventType":"response.completed","status":"completed"}),
            json!({"eventType":"response.completed","usage":{"input_tokens":0,"output_tokens":0,"total_tokens":0}}),
            json!({"eventType":"response.completed","status":"in_progress","usage":{"input_tokens":0,"output_tokens":0,"total_tokens":0}}),
            json!({"eventType":"response.completed","status":"completed","usage":{"input_tokens":0,"total_tokens":0}}),
            json!({"eventType":"response.completed","status":"completed","usage":{"input_tokens":"0","output_tokens":0,"total_tokens":0}}),
        ] {
            let (_dir, mut h) = super::ledger_tests::fixture();
            h.coding_provider_frame(&json!({"type":"provider_request","id":1,"provider":"openai-codex","model":"gpt-6-luna","contextBytes":1000})).unwrap();
            h.coding_provider_frame(&json!({"type":"provider_observation","id":1,"httpStatus":200,"backendModel":"gpt-6-luna","rawTerminal":raw})).unwrap();
            assert!(h.coding_provider_frame(&json!({"type":"provider_complete","id":1,"stopReason":"stop","usage":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0}})).is_err());
            assert_eq!(
                h.journal.coding.as_ref().unwrap().provider_requests[&1].reserved_microusd,
                164001
            );
            assert!(
                h.journal.coding.as_ref().unwrap().provider_requests[&1]
                    .settled_microusd
                    .is_none()
            );
            assert!(h.coding_provider_frame(&json!({"type":"provider_request","id":2,"provider":"openai-codex","model":"gpt-6-luna","contextBytes":1000})).is_err());
        }
    }
}

impl Host {
    fn coding_fake_provider_frame(&mut self, frame: &Value) -> Result<Option<Value>> {
        let id = frame["id"]
            .as_u64()
            .filter(|id| (1..=2).contains(id))
            .ok_or_else(|| Error::Refused("fake provider ID".into()))?;
        match frame["type"].as_str().unwrap_or("") {
            "provider_request" => {
                if self.expired() || self.cancelled() {
                    return refuse("fake provider deadline/cancel");
                }
                let state = self.journal.coding.as_mut().unwrap();
                if state.provider_failed
                    || id != state.provider_requests.len() as u64 + 1
                    || state
                        .provider_requests
                        .values()
                        .any(|r| r.settled_microusd.is_none())
                    || frame["provider"] != "fake_fixture"
                    || frame["model"] != "fake-pi-model"
                    || frame["contextBytes"].as_u64().is_none_or(|n| n > 65536)
                {
                    return refuse("fake provider identity/context/unknown conflict");
                }
                state.provider_requests.insert(
                    id,
                    Reservation {
                        reserved_microusd: 0,
                        ..Reservation::default()
                    },
                );
                self.save()?;
                let owner = self
                    .pi_owner
                    .as_ref()
                    .ok_or_else(|| Error::Refused("fake provider owner absent".into()))?;
                let lease = owner
                    .enter(session_task_admission::pi_pilot::Effect::Provider(id))
                    .map_err(|e| Error::Refused(e.into()))?;
                lease
                    .check_current()
                    .map_err(|e| Error::Refused(e.into()))?;
                self.reply_leases.push(lease);
                Ok(Some(
                    json!({"type":"provider_grant","id":id,"classification":"fake_fixture","reservedMicrousd":0}),
                ))
            }
            "provider_observation" => {
                let r = self
                    .journal
                    .coding
                    .as_mut()
                    .unwrap()
                    .provider_requests
                    .get_mut(&id)
                    .ok_or_else(|| Error::Refused("fake observation before intent".into()))?;
                if r.raw_terminal.is_some()
                    || r.settled_microusd.is_some()
                    || frame["backendModel"] != "fake-pi-model"
                    || frame["httpStatus"] != 200
                    || frame["rawTerminal"]
                        != json!({"classification":"fake_fixture","eventType":"response.completed","status":"completed","usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}})
                {
                    return refuse("fake raw terminal conflict");
                }
                r.backend_model = Some("fake-pi-model".into());
                r.http_status = Some(200);
                r.raw_terminal = Some(frame["rawTerminal"].clone());
                self.save()?;
                Ok(None)
            }
            "provider_complete" => {
                let r = self
                    .journal
                    .coding
                    .as_mut()
                    .unwrap()
                    .provider_requests
                    .get_mut(&id)
                    .ok_or_else(|| Error::Refused("fake completion before intent".into()))?;
                if r.settled_microusd.is_some()
                    || r.raw_terminal.is_none()
                    || frame["usage"]
                        != json!({"input":1,"output":1,"cacheRead":0,"cacheWrite":0,"totalTokens":2,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}})
                    || frame["stopReason"] != if id == 1 { "toolUse" } else { "stop" }
                {
                    return refuse("fake usage/completion conflict");
                }
                r.usage = Some(frame["usage"].clone());
                r.settled_microusd = Some(0);
                self.save()?;
                Ok(None)
            }
            _ => refuse("unknown fake provider frame"),
        }
    }
}

#[cfg(test)]
mod signed_fake_tests {
    use super::*;
    #[test]
    fn fake_provider_unknown_entry_and_missing_raw_usage_never_replay() {
        let (_dir, mut h) = super::super::tests::fixture();
        h.journal.job.coding.as_mut().unwrap().schema = super::super::SIGNED_PROFILE.into();
        h.journal.job.deadline_unix_seconds = now() + 120;
        let request = json!({"type":"provider_request","id":1,"provider":"fake_fixture","model":"fake-pi-model","contextBytes":1});
        // The journal reserves first, but an absent authority owner cannot
        // release a provider grant. This durable unknown cannot be replayed.
        assert!(h.coding_provider_frame(&request).is_err());
        assert!(h.coding_provider_frame(&request).is_err());
        assert_eq!(
            h.journal.coding.as_ref().unwrap().provider_requests.len(),
            1
        );
        assert!(
            h.coding_provider_frame(
                &json!({"type":"provider_complete","id":1,"stopReason":"toolUse","usage":{}})
            )
            .is_err()
        );
        assert!(h.coding_provider_frame(&json!({"type":"provider_observation","id":1,"httpStatus":200,"backendModel":"fake-pi-model","rawTerminal":{"classification":"fake_fixture","status":"completed"}})).is_err());
        assert!(
            h.journal.coding.as_ref().unwrap().provider_requests[&1]
                .settled_microusd
                .is_none()
        );
        assert!(h.journal.processes.is_empty());
    }
}

#[cfg(test)]
mod signed_subscription_tests {
    use super::*;
    #[test]
    fn signed_subscription_uses_full_reservation_and_requires_owner_before_grant() {
        let (_dir, mut h) = super::ledger_tests::fixture();
        h.journal.job.coding.as_mut().unwrap().schema =
            super::super::SIGNED_SUBSCRIPTION_PROFILE.into();
        let frame = json!({"type":"provider_request","id":1,"provider":"openai-codex","model":"gpt-6-luna","contextBytes":12});
        assert!(h.coding_provider_frame(&frame).is_err());
        let r = &h.journal.coding.as_ref().unwrap().provider_requests[&1];
        assert_eq!(r.reserved_microusd, 164001);
        assert!(r.settled_microusd.is_none());
        assert!(h.reply_leases.is_empty());
        assert!(h.coding_provider_frame(&frame).is_err());
    }
    #[test]
    fn signed_subscription_unknown_completion_cannot_settle_or_regrant() {
        let (_dir, mut h) = super::ledger_tests::fixture();
        h.journal.job.coding.as_mut().unwrap().schema =
            super::super::SIGNED_SUBSCRIPTION_PROFILE.into();
        let request = json!({"type":"provider_request","id":1,"provider":"openai-codex","model":"gpt-6-luna","contextBytes":12});
        assert!(h.coding_provider_frame(&request).is_err());
        // Even SDK normalized zeros plus a success-looking stop are not raw evidence.
        assert!(h.coding_provider_frame(&json!({"type":"provider_complete","id":1,"stopReason":"stop","usage":{"input":0,"output":0}})).is_err());
        assert!(
            h.journal.coding.as_ref().unwrap().provider_requests[&1]
                .settled_microusd
                .is_none()
        );
        assert!(h.coding_provider_frame(&request).is_err());
    }
}
