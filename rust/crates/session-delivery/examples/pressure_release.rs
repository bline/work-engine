//! Bounded operator-owned pilot. No ACK, receiver process, or network activity.
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Startup {
    config: Config,
    event_path: PathBuf,
    ready_path: PathBuf,
    envelope_path: PathBuf,
    release_path: PathBuf,
    input_digest: String,
    container_id: String,
    ready_sha256: String,
    job: String,
    deadline_unix_seconds: u64,
    recipient: Recipient,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipient {
    recipient: String,
    principal: String,
    job: String,
    lane: String,
    workspace: String,
    generation: u64,
    address: String,
    fixture_mode: String,
    expires_at: u64,
    active: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ready {
    schema: String,
    input_digest: String,
    container_id: String,
    native_session_id: String,
    deadline_unix_seconds: u64,
}
fn now() -> R<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
fn publish(path: &Path, bytes: &[u8]) -> R<()> {
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
fn source(s: &Startup) -> R<()> {
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
fn release(s: &Startup, time: u64) -> R<()> {
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
fn main() -> R<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return bad("usage: pressure_release source|release /absolute/trusted-startup.json");
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
