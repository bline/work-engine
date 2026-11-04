//! Bounded operator-owned pilot; observe-ack requires separate runtime admission.
use serde::{Deserialize, Serialize};
use serde_json::json;
use session_delivery::{Binding, Config, Event, Stage, Store, digest};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
const PROFILE: &str = "work-engine.pi-observation-sdk.v1";
type R<T> = Result<T, Box<dyn std::error::Error>>;
fn bad<T>(s: &str) -> R<T> {
    Err(s.to_owned().into())
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Startup {
    pub(crate) config: Config,
    pub(crate) event_path: PathBuf,
    pub(crate) ready_path: PathBuf,
    pub(crate) envelope_path: PathBuf,
    pub(crate) release_path: PathBuf,
    pub(crate) input_digest: String,
    pub(crate) container_id: String,
    pub(crate) ready_sha256: String,
    pub(crate) job: String,
    pub(crate) deadline_unix_seconds: u64,
    pub(crate) recipient: Recipient,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Recipient {
    pub(crate) recipient: String,
    pub(crate) principal: String,
    pub(crate) job: String,
    pub(crate) lane: String,
    pub(crate) workspace: String,
    pub(crate) generation: u64,
    pub(crate) address: String,
    pub(crate) fixture_mode: String,
    pub(crate) expires_at: u64,
    pub(crate) active: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Ready {
    pub(crate) schema: String,
    pub(crate) input_digest: String,
    pub(crate) container_id: String,
    pub(crate) native_session_id: String,
    pub(crate) deadline_unix_seconds: u64,
}
pub(crate) fn now() -> R<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
pub(crate) fn publish(path: &Path, bytes: &[u8]) -> R<()> {
    if !path.is_absolute() {
        return bad("artifact path must be absolute");
    }
    let parent = path.parent().ok_or("missing parent")?;
    let tmp = parent.join(format!(
        ".pressure-release-{}-{}",
        std::process::id(),
        path.file_name()
            .ok_or("missing filename")?
            .to_string_lossy()
    ));
    let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    let result = (|| -> R<()> {
        f.write_all(bytes)?;
        f.set_permissions(fs::Permissions::from_mode(0o400))?;
        f.sync_all()?;
        fs::hard_link(&tmp, path)?;
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    let _ = fs::remove_file(tmp);
    result
}
pub(crate) fn source(s: &Startup) -> R<()> {
    if !s.config.targets.is_empty() || s.config.pressure.is_none() {
        return bad("source-only config required");
    }
    let mut store = Store::open(s.config.clone())?;
    if !store.state()?.events.is_empty() {
        return bad("fresh source store required");
    }
    store.observe_pressure()?;
    let state = store.state()?;
    if state.events.len() != 1 || !state.deliveries.is_empty() || !state.bindings.is_empty() {
        return bad("exactly one retained event required");
    }
    let e = state.events.values().next().unwrap();
    publish(&s.event_path, &serde_json::to_vec(e)?)
}
pub(crate) fn release(s: &Startup, time: u64) -> R<()> {
    if !s.config.targets.is_empty() || s.config.pressure.is_none() {
        return bad("source-only config required");
    }
    if s.ready_path
        .parent()
        .map(|p| p.join("receiver-release.json"))
        != Some(s.release_path.clone())
    {
        return bad("release path must be beside readiness");
    }
    if s.release_path.exists() || s.envelope_path.exists() {
        return bad("release already exists");
    }
    if !s.ready_path.is_absolute()
        || !s.event_path.is_absolute()
        || fs::symlink_metadata(&s.ready_path)?
            .file_type()
            .is_symlink()
    {
        return bad("unsafe readiness path");
    }
    let ready_bytes = fs::read(&s.ready_path)?;
    if digest(&ready_bytes) != s.ready_sha256 {
        return bad("ready digest mismatch");
    }
    let r: Ready = serde_json::from_slice(&ready_bytes)?;
    if r.schema != PROFILE
        || r.input_digest != s.input_digest
        || r.container_id != s.container_id
        || r.native_session_id.is_empty()
        || r.deadline_unix_seconds != s.deadline_unix_seconds
        || time >= r.deadline_unix_seconds
        || s.recipient.job != s.job
        || s.recipient.expires_at < r.deadline_unix_seconds
        || !s.recipient.active
        || s.recipient.fixture_mode != "observe"
    {
        return bad("ready identity or deadline mismatch");
    }
    let p = s.config.pressure.as_ref().unwrap();
    if !p.recipients.contains(&s.recipient.recipient) {
        return bad("recipient not whitelisted");
    }
    let event_bytes = fs::read(&s.event_path)?;
    let event: Event = serde_json::from_slice(&event_bytes)?;
    let x = &s.recipient;
    let binding = Binding {
        recipient: x.recipient.clone(),
        principal: x.principal.clone(),
        job: x.job.clone(),
        lane: x.lane.clone(),
        session: r.native_session_id.clone(),
        workspace: x.workspace.clone(),
        generation: x.generation,
        address: x.address.clone(),
        fixture_mode: x.fixture_mode.clone(),
        expires_at: x.expires_at,
        active: x.active,
    };
    let mut config = s.config.clone();
    config.targets = vec![binding.clone()];
    config.validate()?;
    let mut store = Store::open(config)?;
    let state = store.state()?;
    if state.events.len() != 1
        || state.events.get(&event.event_id) != Some(&event)
        || serde_json::to_vec(&event)? != event_bytes
        || !state.bindings.is_empty()
        || !state.deliveries.is_empty()
    {
        return bad("event or store state mismatch");
    }
    store.subscribe(&binding.recipient, &binding.principal)?;
    let state = store.state()?;
    let ids: Vec<_> = state
        .deliveries
        .values()
        .filter(|d| {
            d.event_id == event.event_id
                && d.binding == binding
                && d.stage == Stage::Persisted
                && d.attempts == 0
        })
        .map(|d| d.delivery_id.clone())
        .collect();
    if ids.len() != 1 || state.deliveries.len() != 1 {
        return bad("exact first delivery missing");
    }
    let envelope = store.prepare(&ids[0], time)?.ok_or("prepare held")?;
    let bytes = serde_json::to_vec(&envelope)?;
    publish(&s.envelope_path, &bytes)?;
    // The receiver may act as soon as the release appears. Persist uncertainty first.
    store.possible_send(&ids[0])?;
    let release = json!({"schema":PROFILE,"input_digest":r.input_digest,"container_id":r.container_id,"native_session_id":r.native_session_id,"envelope":{"path":s.envelope_path,"sha256":digest(&bytes)}});
    publish(&s.release_path, &serde_json::to_vec(&release)?)?;
    Ok(())
}
// Only this function can create VerifiedRun, immediately after the owning Host
// has parsed final native stdout/history and reconciled all owned processes.
pub(crate) struct VerifiedRun {
    host: work_engine_manual_host::pi_docker::Host,
    receipt: serde_json::Value,
}
impl VerifiedRun {
    pub(crate) fn journal(&self) -> &work_engine_manual_host::pi_docker::Journal {
        self.host.journal()
    }
    pub(crate) fn receipt_value(&self) -> &serde_json::Value {
        &self.receipt
    }
    pub(crate) fn run(job: work_engine_manual_host::pi_docker::Job) -> R<Self> {
        if job.receiver.is_none() {
            return bad("observation profile required");
        }
        let mut host = work_engine_manual_host::pi_docker::Host::admit(job)?;
        let receipt = host.run()?;
        Ok(Self { host, receipt })
    }
    #[cfg(test)]
    fn ack(&mut self, s: &Startup, time: u64) -> R<serde_json::Value> {
        self.ack_with_clock(s, || Ok(time))
    }
    pub(crate) fn ack_with_clock(
        &mut self,
        s: &Startup,
        clock: impl FnOnce() -> R<u64>,
    ) -> R<serde_json::Value> {
        let j = self.host.journal();
        let profile = j.job.receiver.as_ref().ok_or("receiver profile missing")?;
        let o = j.receiver.as_ref().ok_or("receiver proof missing")?;
        let proof = j.rpc_completion.as_ref().ok_or("completed proof missing")?;
        if self.receipt["receipt"]["status"] != "component_passed"
            || j.job.job_id != s.job
            || j.input_digest != s.input_digest
            || j.container_id.as_deref() != Some(s.container_id.as_str())
            || j.job.deadline_unix_seconds != s.deadline_unix_seconds
            || s.ready_path != j.job.state_dir.join("receiver-ready.json")
            || s.release_path != j.job.state_dir.join("receiver-release.json")
            || !o.target_observed
            || !o.transport_accepted
            || o.process_index != Some(proof.process_index)
            || o.completed_stdout_sha256.as_deref() != Some(proof.stdout_sha256.as_str())
            || o.native_session_id.as_deref() != Some(proof.session_id.as_str())
            || j.session_id.as_deref() != Some(proof.session_id.as_str())
            || profile.recipient_without_session != serde_json::to_value(&s.recipient)?
        {
            return bad("verified native/job/startup identity mismatch");
        }
        // Recheck the original immutable inputs, including complete runtime trees.
        for tree in [&j.job.runtime, &j.job.workspace] {
            let expected = serde_json::from_slice(&tree.manifest.verify()?)?;
            if work_engine_manual_host::pi_docker::inventory(&tree.root)? != expected {
                return bad("immutable tree changed");
            }
        }
        for a in [
            &j.job.authority,
            &j.job.packet,
            &profile.shim,
            &profile.event,
        ] {
            a.verify()?;
        }
        // Executables use the Host executable pin contract, not Artifact's
        // bounded JSON input reader (debug fixture ELFs can exceed that bound).
        if digest(&fs::read(&j.job.docker.path)?) != j.job.docker.sha256 {
            return bad("pinned executable changed");
        }
        let ready_bytes = fs::read(&s.ready_path)?;
        let ready: Ready = serde_json::from_slice(&ready_bytes)?;
        if digest(&ready_bytes) != s.ready_sha256
            || ready.schema != PROFILE
            || ready.input_digest != j.input_digest
            || ready.container_id != s.container_id
            || ready.native_session_id != proof.session_id
            || ready.deadline_unix_seconds != s.deadline_unix_seconds
        {
            return bad("trusted readiness changed");
        }
        let release: work_engine_manual_host::pi_docker::receiver::Release =
            serde_json::from_slice(&fs::read(&s.release_path)?)?;
        let pin = o
            .release_intent
            .as_ref()
            .ok_or("release intent missing")?
            .clone();
        let raw = pin.verify()?;
        if release.schema != PROFILE
            || release.input_digest != j.input_digest
            || release.container_id != s.container_id
            || release.native_session_id != proof.session_id
            || release.envelope.path != s.envelope_path
            || release.envelope.sha256 != pin.sha256
            || release.envelope.verify()? != raw
            || fs::read(&s.event_path)? != profile.event.verify()?
        {
            return bad("release/envelope/source changed");
        }
        o.native_history
            .as_ref()
            .ok_or("native history missing")?
            .verify()?;
        let process = j
            .processes
            .get(proof.process_index)
            .ok_or("process missing")?;
        if digest(&fs::read(&process.stdout)?) != proof.stdout_sha256 {
            return bad("final stdout changed");
        }
        // Revalidate artifact hashes and closure before changing the service owner.
        let current = self.host.receipt()?;
        if current != self.receipt {
            return bad("receipt changed");
        }
        let e: session_delivery::Envelope = serde_json::from_slice(&raw)?;
        if serde_json::to_vec(&e)? != raw || e.binding.session != ready.native_session_id {
            return bad("nonexact service envelope");
        }
        let mut config = s.config.clone();
        if !config.targets.is_empty() || config.pressure.is_none() {
            return bad("source-only config required");
        }
        config.targets = vec![e.binding.clone()];
        let mut store = Store::open(config)?;
        let state = store.state()?;
        let d = state
            .deliveries
            .get(&e.delivery_id)
            .ok_or("prepared delivery missing")?;
        if state.events.len() != 1
            || state.deliveries.len() != 1
            || d.attempts != 1
            || state.events.get(&e.event.event_id) != Some(&e.event)
            || d.binding != e.binding
            || d.envelope_sha256 != pin.sha256
        {
            return bad("exact source/prepared delivery mismatch");
        }
        let ack = session_delivery::Ack {
            schema: "work-engine.delivery-observation/v1".into(),
            delivery_id: e.delivery_id,
            attempt_id: e.attempt_id,
            event_id: e.event.event_id,
            binding_sha256: e.binding_sha256,
            envelope_sha256: pin.sha256.clone(),
            payload_sha256: e.event.payload_sha256,
            generation: e.binding.generation,
            stage: "target_observed".into(),
        };
        // Expiry is checked at the action boundary, after potentially slow proof
        // and full-tree verification. Never reuse admission's earlier timestamp.
        let mut result = store.ack(&e.binding.recipient, &e.binding.principal, ack, clock()?)?;
        let receipt_id = self.receipt["receipt_id"]
            .as_str()
            .ok_or("native receipt identity missing")?;
        result["native_receipt_delivery"] = self.host.acknowledge(receipt_id)?;
        Ok(result)
    }
}
fn observe_ack(job_path: &Path, startup_path: &Path) -> R<()> {
    // Trusted operator paths only. Startup is finalized by the release owner after
    // readiness; no asserted receipt or caller-selected success flag is accepted.
    let job = serde_json::from_slice(&fs::read(job_path)?)?;
    let mut verified = VerifiedRun::run(job)?;
    let s = serde_json::from_slice(&fs::read(startup_path)?)?;
    println!("{}", verified.ack_with_clock(&s, now)?);
    Ok(())
}
fn main() -> R<()> {
    let args: Vec<_> = std::env::args().collect();
    // Recovery is custody-only. It can never import proof or advance Store ACK.
    if args.len() == 3 && args[1] == "custody-reconcile" {
        let path = Path::new(&args[2]);
        if !path.is_absolute() {
            return bad("absolute trusted state required");
        }
        let mut host = work_engine_manual_host::pi_docker::Host::open(path)?;
        println!("{}", host.reconcile()?);
        return Ok(());
    }
    if args.len() == 4 && args[1] == "observe-ack" {
        if !Path::new(&args[2]).is_absolute() || !Path::new(&args[3]).is_absolute() {
            return bad("absolute trusted paths required");
        }
        return observe_ack(Path::new(&args[2]), Path::new(&args[3]));
    }
    if args.len() != 3 {
        return bad("usage: pressure_release source|release STARTUP | observe-ack JOB STARTUP");
    }
    let path = Path::new(&args[2]);
    if !path.is_absolute() {
        return bad("startup path must be absolute");
    }
    let s: Startup = serde_json::from_slice(&fs::read(path)?)?;
    match args[1].as_str() {
        "source" => source(&s),
        "release" => release(&s, now()?),
        _ => bad("unknown phase"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> (tempfile::TempDir, Startup) {
        let t = tempfile::tempdir().unwrap();
        let p = t.path();
        let reader = std::env::var("PRESSURE_TEST_READER").expect("pinned reader");
        let hash = std::env::var("PRESSURE_TEST_READER_SHA256").expect("pinned hash");
        let rollout = p.join("synthetic.jsonl");
        let session = "11111111-1111-4111-8111-111111111111";
        fs::write(&rollout,format!("{}\n{}\n",json!({"type":"session_meta","payload":{"id":session}}),json!({"type":"event_msg","timestamp":"2026-11-04T00:20:00Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":700,"cached_input_tokens":0},"model_context_window":1000}}}))).unwrap();
        let cfg:Config=serde_json::from_value(json!({"schema":"session-delivery-fixture-config-v1","fixture_only":true,"database":p.join("store.sqlite"),"repository":p,"repository_identity":"synthetic","source_id":"unused-board","board_reader":"/unused/board","source_adapter":"/unused/adapter","targets":[],"pressure":{"reader":reader,"reader_sha256":hash,"incarnation":"synthetic","session":session,"host":"codex","agent":null,"rollout":rollout,"levels":{"window_percent":{"trigger":58.0,"band_floor":66.0}},"step_percent":5.0,"lead_calls":3,"recipients":["target"]}})).unwrap();
        let ready = Ready {
            schema: PROFILE.into(),
            input_digest: "input".into(),
            container_id: "container".into(),
            native_session_id: "native-session".into(),
            deadline_unix_seconds: 500,
        };
        let raw = serde_json::to_vec(&ready).unwrap();
        fs::write(p.join("ready.json"), &raw).unwrap();
        let s = Startup {
            config: cfg,
            event_path: p.join("event.json"),
            ready_path: p.join("ready.json"),
            envelope_path: p.join("envelope.json"),
            release_path: p.join("receiver-release.json"),
            input_digest: "input".into(),
            container_id: "container".into(),
            ready_sha256: digest(&raw),
            job: "job".into(),
            deadline_unix_seconds: 500,
            recipient: Recipient {
                recipient: "target".into(),
                principal: "principal".into(),
                job: "job".into(),
                lane: "lane".into(),
                workspace: p.to_str().unwrap().into(),
                generation: 1,
                address: "role:target".into(),
                fixture_mode: "observe".into(),
                expires_at: 600,
                active: true,
            },
        };
        (t, s)
    }
    fn artifact(path: &Path, bytes: &[u8]) -> work_engine_manual_host::Artifact {
        fs::write(path, bytes).unwrap();
        work_engine_manual_host::Artifact {
            path: path.into(),
            sha256: digest(bytes),
        }
    }
    fn native_run(mode: &str) -> (tempfile::TempDir, Startup, VerifiedRun) {
        use work_engine_manual_host::pi_docker::{Job, Tree, inventory, receiver};
        let (dir, mut s) = fixture();
        let p = dir.path();
        let runtime = p.join("runtime");
        let workspace = p.join("workspace");
        let state = p.join("state");
        for path in [&runtime, &workspace, &state] {
            fs::create_dir(path).unwrap();
        }
        let sdk = runtime.join("node_modules/@earendil-works/pi-coding-agent");
        fs::create_dir_all(sdk.join("dist/bundle")).unwrap();
        fs::write(sdk.join("dist/bundle/cli.js"), "fake never executed").unwrap();
        fs::write(sdk.join("dist/index.js"), "fake never executed").unwrap();
        fs::write(
            sdk.join("package.json"),
            r#"{"name":"@earendil-works/pi-coding-agent","version":"1.5.0"}"#,
        )
        .unwrap();
        let exe = p.join("fake-docker");
        fs::copy(std::env::var("NATIVE_ACK_FIXTURE").unwrap(), &exe).unwrap();
        fs::copy(
            std::env::var("NATIVE_ACK_BASE_FIXTURE").unwrap(),
            exe.with_extension("base"),
        )
        .unwrap();
        fs::write(exe.with_extension("mode"), mode).unwrap();
        s.deadline_unix_seconds = now().unwrap() + 25;
        s.recipient.expires_at = s.deadline_unix_seconds + 20;
        s.ready_path = state.join("receiver-ready.json");
        s.release_path = state.join("receiver-release.json");
        source(&s).unwrap();
        let job = Job {
            schema: work_engine_manual_host::pi_docker::SCHEMA.into(),
            job_id: s.job.clone(),
            state_dir: state,
            authority: artifact(&p.join("authority"), b"fake-only authority"),
            packet: artifact(&p.join("packet"), b"fake-only packet"),
            docker: work_engine_manual_host::Artifact {
                path: exe.clone(),
                sha256: digest(&fs::read(&exe).unwrap()),
            },
            image: work_engine_manual_host::pi_docker::IMAGE.into(),
            runtime: Tree {
                root: runtime.clone(),
                manifest: artifact(
                    &p.join("runtime.json"),
                    &serde_json::to_vec(&inventory(&runtime).unwrap()).unwrap(),
                ),
            },
            workspace: Tree {
                root: workspace.clone(),
                manifest: artifact(
                    &p.join("workspace.json"),
                    &serde_json::to_vec(&inventory(&workspace).unwrap()).unwrap(),
                ),
            },
            deadline_unix_seconds: s.deadline_unix_seconds,
            command_timeout_seconds: 3,
            idle_seconds: 0,
            receiver: Some(receiver::Profile {
                schema: PROFILE.into(),
                shim: artifact(&p.join("shim.mjs"), receiver::SHIM),
                event: work_engine_manual_host::Artifact {
                    path: s.event_path.clone(),
                    sha256: digest(&fs::read(&s.event_path).unwrap()),
                },
                attempt_number: 1,
                recipient_without_session: serde_json::to_value(&s.recipient).unwrap(),
            }),
        };
        // Deterministic operator-owned release producer; never a model/runtime service.
        let producer = std::thread::spawn(move || {
            let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while !s.ready_path.exists() {
                assert!(std::time::Instant::now() < end, "fake readiness timeout");
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            let bytes = fs::read(&s.ready_path).unwrap();
            let r: Ready = serde_json::from_slice(&bytes).unwrap();
            s.input_digest = r.input_digest;
            s.container_id = r.container_id;
            s.ready_sha256 = digest(&bytes);
            release(&s, now().unwrap()).unwrap();
            s
        });
        let verified = VerifiedRun::run(job).unwrap();
        (dir, producer.join().unwrap(), verified)
    }
    #[test]
    fn verified_fake_host_ack_and_duplicate_never_resend() {
        let (_dir, s, mut run) = native_run("observe");
        let calls = run.host.journal().job.docker.path.with_extension("calls");
        let before = fs::read(&calls).unwrap();
        assert_eq!(run.ack(&s, now().unwrap()).unwrap()["duplicate"], false);
        assert!(
            run.host
                .journal()
                .acknowledged
                .contains(run.receipt["receipt_id"].as_str().unwrap())
        );
        assert_eq!(
            run.ack(&s, s.recipient.expires_at + 1).unwrap()["duplicate"],
            true
        );
        let state = Store::open(s.config.clone()).unwrap().state().unwrap();
        let d = state.deliveries.values().next().unwrap();
        assert_eq!(d.stage, Stage::TargetObserved);
        assert_eq!(d.attempts, 1);
        assert!(d.application_result.is_none());
        assert_eq!(fs::read(calls).unwrap(), before);
        assert!(VerifiedRun::run(run.host.journal().job.clone()).is_err());
    }
    #[test]
    fn expiry_crossed_during_proof_never_uses_earlier_time() {
        let (_dir, s, mut run) = native_run("observe");
        let before_verification = now().unwrap();
        assert!(before_verification < s.recipient.expires_at);
        // The clock has advanced to expiry by the action boundary. This is the
        // same late-clock path used by production, without a timing-dependent sleep.
        let mut sampled = false;
        assert!(
            run.ack_with_clock(&s, || {
                sampled = true;
                Ok(s.recipient.expires_at)
            })
            .is_err()
        );
        assert!(sampled);
        let state = Store::open(s.config.clone()).unwrap().state().unwrap();
        let d = state.deliveries.values().next().unwrap();
        assert_eq!(d.stage, Stage::Unknown);
        assert!(!d.target_observed);
        assert!(run.host.journal().acknowledged.is_empty());
    }
    #[test]
    fn failed_or_missing_native_proof_stays_unknown() {
        for mode in [
            "missing-end",
            "missing-messages",
            "missing-history",
            "partial-tail",
            "wrong-session",
        ] {
            let (_dir, s, mut run) = native_run(mode);
            assert!(run.ack(&s, now().unwrap()).is_err(), "{mode}");
            let state = Store::open(s.config.clone()).unwrap().state().unwrap();
            let d = state.deliveries.values().next().unwrap();
            assert_eq!(d.stage, Stage::Unknown, "{mode}");
            assert!(!d.target_observed);
        }
    }
    #[test]
    fn tampered_artifacts_and_mismatched_startup_stay_unknown() {
        for kind in [
            "principal",
            "generation",
            "input",
            "container",
            "job",
            "ready",
            "envelope",
            "event",
            "stdout",
            "history",
            "tree",
            "expired",
        ] {
            let (_dir, mut s, mut run) = native_run("observe");
            match kind {
                "principal" => s.recipient.principal = "foreign".into(),
                "generation" => s.recipient.generation += 1,
                "input" => s.input_digest = "foreign".into(),
                "container" => s.container_id = "foreign".into(),
                "job" => s.job = "foreign".into(),
                "ready" => {
                    fs::remove_file(&s.ready_path).unwrap();
                }
                "envelope" => {
                    fs::set_permissions(&s.envelope_path, fs::Permissions::from_mode(0o600))
                        .unwrap();
                    fs::write(&s.envelope_path, "tampered").unwrap();
                }
                "event" => {
                    fs::set_permissions(&s.event_path, fs::Permissions::from_mode(0o600)).unwrap();
                    fs::write(&s.event_path, "tampered").unwrap();
                }
                "stdout" => {
                    let j = run.host.journal();
                    fs::write(
                        &j.processes[j.rpc_completion.as_ref().unwrap().process_index].stdout,
                        "tampered",
                    )
                    .unwrap();
                }
                "history" => {
                    fs::remove_file(
                        &run.host
                            .journal()
                            .receiver
                            .as_ref()
                            .unwrap()
                            .native_history
                            .as_ref()
                            .unwrap()
                            .path,
                    )
                    .unwrap();
                }
                "tree" => {
                    fs::write(run.host.journal().job.workspace.root.join("new"), "changed")
                        .unwrap();
                }
                _ => {}
            }
            let time = if kind == "expired" {
                s.recipient.expires_at
            } else {
                now().unwrap()
            };
            assert!(run.ack(&s, time).is_err(), "{kind}");
            let state = Store::open(s.config.clone()).unwrap().state().unwrap();
            assert_eq!(
                state.deliveries.values().next().unwrap().stage,
                Stage::Unknown,
                "{kind}"
            );
        }
    }
    #[test]
    fn fake_ready_positive_and_no_replay() {
        let (_t, s) = fixture();
        source(&s).unwrap();
        let before = Store::open(s.config.clone()).unwrap().state().unwrap();
        assert_eq!(before.events.len(), 1);
        assert!(before.bindings.is_empty() && before.deliveries.is_empty());
        release(&s, 100).unwrap();
        let e: session_delivery::Envelope =
            serde_json::from_slice(&fs::read(&s.envelope_path).unwrap()).unwrap();
        assert_eq!(e.binding.session, "native-session");
        assert_eq!(e.binding.job, "job");
        let r: serde_json::Value =
            serde_json::from_slice(&fs::read(&s.release_path).unwrap()).unwrap();
        assert_eq!(
            r["envelope"]["sha256"],
            digest(&fs::read(&s.envelope_path).unwrap())
        );
        assert!(release(&s, 100).is_err());
        fs::remove_file(&s.release_path).unwrap();
        fs::remove_file(&s.envelope_path).unwrap();
        assert!(release(&s, 100).is_err());
        let state = Store::open(s.config.clone()).unwrap().state().unwrap();
        let d = state.deliveries.values().next().unwrap();
        assert_eq!(d.stage, Stage::Unknown);
        assert_eq!(d.attempts, 1);
        assert!(!d.target_observed);
        let id = d.delivery_id.clone();
        let mut configured = s.config.clone();
        configured.targets = vec![e.binding.clone()];
        let mut store = Store::open(configured).unwrap();
        assert!(store.prepare(&id, 100).unwrap().is_none());
        assert!(store.prepare(&id, 100).unwrap().is_none());
        assert!(release(&s, 100).is_err());
    }
    #[test]
    fn fake_wrong_or_stale_ready_never_prepares() {
        for kind in [
            "input",
            "container",
            "session",
            "job",
            "deadline",
            "expired",
            "digest",
        ] {
            let (_t, mut s) = fixture();
            source(&s).unwrap();
            match kind {
                "input" => s.input_digest = "wrong".into(),
                "container" => s.container_id = "wrong".into(),
                "session" => {
                    let mut r: Ready =
                        serde_json::from_slice(&fs::read(&s.ready_path).unwrap()).unwrap();
                    r.native_session_id = "wrong".into();
                    fs::write(&s.ready_path, serde_json::to_vec(&r).unwrap()).unwrap();
                }
                "job" => s.job = "wrong".into(),
                "deadline" => s.deadline_unix_seconds = 501,
                "digest" => s.ready_sha256 = "wrong".into(),
                _ => {}
            }
            assert!(
                release(&s, if kind == "expired" { 500 } else { 100 }).is_err(),
                "{kind}"
            );
            let state = Store::open(s.config.clone()).unwrap().state().unwrap();
            assert!(state.deliveries.is_empty() && state.bindings.is_empty());
        }
    }
    #[test]
    fn source_only_validation_is_narrow() {
        let (_t, mut s) = fixture();
        assert!(s.config.validate().is_ok());
        s.config.pressure = None;
        assert!(s.config.validate().is_err());
    }
}
