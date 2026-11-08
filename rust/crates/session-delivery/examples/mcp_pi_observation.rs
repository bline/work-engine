//! Fixed, operator-configured MCP observation composition. No caller command surface.
#[allow(dead_code)]
#[path = "pressure_release.rs"]
mod proof;
use proof::{Startup, VerifiedRun, now, publish, release, source};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use session_delivery::digest;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::fd::FromRawFd,
    path::{Path, PathBuf},
};
use work_engine_manual_host::{Artifact, pi_docker::Job};
type R<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
const SCHEMA: &str = "work-engine.mcp-pi-observation.v1";
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema: String,
    principal: String,
    subject: String,
    target: String,
    handle: String,
    revision: String,
    idempotency_key: String,
    expires_at: u64,
    revoked_path: PathBuf,
    operation_dir: PathBuf,
    job: Job,
    startup: Startup,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    method: String,
    principal: String,
    handle: String,
    revision: String,
    idempotency_key: String,
}
fn err(s: &str) -> Box<dyn std::error::Error + Send + Sync> {
    s.to_string().into()
}
fn convert<T>(r: Result<T, Box<dyn std::error::Error>>) -> R<T> {
    r.map_err(|e| err(&e.to_string()))
}
fn validate(c: &Config, r: &Request) -> R<()> {
    if c.schema != SCHEMA
        || r.schema != SCHEMA
        || !["observe", "status", "result"].contains(&r.method.as_str())
        || c.principal.is_empty()
        || c.subject.is_empty()
        || c.target.is_empty()
        || r.principal != c.principal
        || r.handle != c.handle
        || r.revision != c.revision
        || r.idempotency_key != c.idempotency_key
        || c.handle.is_empty()
        || c.revision.is_empty()
        || c.idempotency_key.is_empty()
    {
        return Err(err("authority_or_request_conflict"));
    }
    for p in [
        &c.operation_dir,
        &c.revoked_path,
        &c.job.state_dir,
        &c.startup.event_path,
        &c.startup.ready_path,
        &c.startup.release_path,
        &c.startup.envelope_path,
    ] {
        if !p.is_absolute() {
            return Err(err("absolute_trusted_paths_required"));
        }
    }
    if c.revoked_path != c.job.state_dir.join("cancel.json")
        || c.startup.recipient.principal != c.principal
        || c.startup.recipient.recipient != c.target
        || c.startup.job != c.job.job_id
        || c.startup.deadline_unix_seconds != c.job.deadline_unix_seconds
        || c.expires_at < c.job.deadline_unix_seconds
        || c.job.command_timeout_seconds > 3
        || c.job.idle_seconds != 0
        || c.startup.ready_path != c.job.state_dir.join("receiver-ready.json")
        || c.startup.release_path != c.job.state_dir.join("receiver-release.json")
        || c.job.receiver.as_ref().is_none_or(|p| {
            p.event.path != c.startup.event_path
                || p.recipient_without_session
                    != serde_json::to_value(&c.startup.recipient).unwrap()
        })
    {
        return Err(err("trusted_target_profile_conflict"));
    }
    Ok(())
}
fn pin(p: &Path) -> R<Artifact> {
    Ok(Artifact {
        path: p.into(),
        sha256: digest(&fs::read(p)?),
    })
}
fn observation_ack(e: &session_delivery::Envelope) -> session_delivery::Ack {
    session_delivery::Ack {
        schema: "work-engine.delivery-observation/v1".into(),
        attempt_id: e.attempt_id.clone(),
        delivery_id: e.delivery_id.clone(),
        event_id: e.event.event_id.clone(),
        binding_sha256: e.binding_sha256.clone(),
        envelope_sha256: e.sha(),
        payload_sha256: e.event.payload_sha256.clone(),
        generation: e.binding.generation,
        stage: "target_observed".into(),
    }
}
// Read-only attribution checks. Never reconstruct native proof, ACK or custody.
fn validate_success(c: &Config, v: &Value, refs: &[Artifact]) -> R<()> {
    let refuse = || err("result_evidence_conflict");
    let expected = [
        c.operation_dir.join("native-receipt.json"),
        c.startup.event_path.clone(),
        c.startup.envelope_path.clone(),
        c.operation_dir.join("job.json"),
        c.operation_dir.join("startup.json"),
    ];
    if refs.len() != 7
        || refs.iter().take(5).zip(expected).any(|(a, p)| a.path != p)
        || refs[6].path != c.startup.ready_path
        || v["fixture_only"] != true
        || v["transport_accepted"] != true
        || v["target_observed"] != true
    {
        return Err(refuse());
    }
    let load = |n: usize| -> R<Vec<u8>> { refs[n].verify().map_err(|e| err(&e.to_string())) };
    let native: Value = serde_json::from_slice(&load(0)?)?;
    let event: session_delivery::Event = serde_json::from_slice(&load(1)?)?;
    let envelope: session_delivery::Envelope = serde_json::from_slice(&load(2)?)?;
    let job: Job = serde_json::from_slice(&load(3)?)?;
    let startup: Startup = serde_json::from_slice(&load(4)?)?;
    let history: Value = serde_json::from_slice(&load(5)?)?;
    let ready: proof::Ready = serde_json::from_slice(&load(6)?)?;
    let mut expected_job = c.job.clone();
    expected_job.receiver.as_mut().ok_or_else(refuse)?.event = refs[1].clone();
    let mut expected_startup = c.startup.clone();
    expected_startup.input_digest = digest(&serde_json::to_vec(&job)?);
    expected_startup.container_id = ready.container_id.clone();
    expected_startup.ready_sha256 = refs[6].sha256.clone();
    let receipt = &native["receipt"];
    let receipt_id = digest(&serde_json::to_vec(receipt)?);
    let session = &ready.native_session_id;
    let mut expected_recipient = serde_json::to_value(&c.startup.recipient)?;
    expected_recipient["session"] = json!(session);
    if serde_json::to_value(&job)? != serde_json::to_value(expected_job)?
        || serde_json::to_value(&startup)? != serde_json::to_value(expected_startup.clone())?
        || load(1)? != serde_json::to_vec(&event)?
        || load(2)? != serde_json::to_vec(&envelope)?
        || envelope.event != event
        || event.payload_sha256 != digest(event.raw_payload.as_bytes())
        || serde_json::to_value(&envelope.binding)? != expected_recipient
        || envelope.binding_sha256 != envelope.binding.sha()
        || ready.schema != work_engine_manual_host::pi_docker::receiver::PROFILE
        || ready.input_digest != expected_startup.input_digest
        || ready.deadline_unix_seconds != job.deadline_unix_seconds
        || session.is_empty()
        || v["native_session_id"] != *session
        || history["sessionId"] != *session
        || native["receipt_id"] != receipt_id
        || v["native_receipt_id"] != receipt_id
        || receipt["job"] != serde_json::to_value(&job)?
        || receipt["job_id"] != job.job_id
        || receipt["input_digest"] != ready.input_digest
        || receipt["container_id"] != ready.container_id
        || receipt["session_id"] != *session
        || receipt["status"] != "component_passed"
        || receipt["receiver"]["native_session_id"] != *session
        || receipt["receiver"]["native_history"] != serde_json::to_value(&refs[5])?
        || receipt["receiver"]["release_intent"]["sha256"] != refs[2].sha256
        || receipt["receiver"]["target_observed"] != true
        || receipt["receiver"]["transport_accepted"] != true
        || v["observation_ack"] != serde_json::to_value(observation_ack(&envelope))?
        || v["store_ack"]
            != json!({"schema":"session-delivery-status-v1","stage":"target_observed","duplicate":false,
            "native_receipt_delivery":{"receipt_id":receipt_id,"delivery":"acknowledged","acceptance":"owner_pending"}})
    {
        return Err(refuse());
    }
    Ok(())
}
fn read_result(c: &Config, binding: &str) -> R<Value> {
    let reserve = c.operation_dir.join("intent.json");
    if !reserve.exists() {
        return Ok(json!({"state":"not_started","accepted":false}));
    }
    let intent: Value = serde_json::from_slice(&fs::read(reserve)?)?;
    let expected_request = json!({"schema":SCHEMA,"method":"observe","principal":c.principal,
        "handle":c.handle,"revision":c.revision,"idempotency_key":c.idempotency_key});
    if intent["binding"] != binding
        || intent["request"] != expected_request
        || intent["state"] != "unknown"
    {
        return Err(err("immutable_input_conflict"));
    }
    let result = c.operation_dir.join("result.json");
    if !result.exists() {
        return Ok(
            json!({"state":"unknown","accepted":true,"transport_accepted":null,"target_observed":null,"application_result":null,"completion":null,"retry_permitted":false}),
        );
    }
    let v: Value = serde_json::from_slice(&fs::read(&result)?)?;
    if v["input_binding_sha256"] != binding
        || v["accepted"] != true
        || !v.get("application_result").is_some_and(Value::is_null)
        || !v.get("completion").is_some_and(Value::is_null)
        || v["retry_permitted"] != false
    {
        return Err(err("result_binding_conflict"));
    }
    let refs: Vec<Artifact> = serde_json::from_value(v["evidence"].clone())?;
    for a in &refs {
        a.verify().map_err(|e| err(&e.to_string()))?;
    }
    match v["state"].as_str() {
        Some("target_observed") => validate_success(c, &v, &refs)?,
        Some("unknown")
            if refs.is_empty()
                && v["transport_accepted"].is_null()
                && v["target_observed"].is_null()
                && v.get("native_session_id").is_none()
                && v.get("native_receipt_id").is_none()
                && v.get("store_ack").is_none()
                && v.get("observation_ack").is_none() =>
        {
            ()
        }
        _ => return Err(err("invalid_result_state")),
    }
    let mut v = v;
    v["result_reference"] = serde_json::to_value(pin(&result)?)?;
    v["intent_reference"] = serde_json::to_value(pin(&c.operation_dir.join("intent.json"))?)?;
    Ok(v)
}
/// What the Host worker returns in place of the Host itself (see `execute`).
struct Observed {
    ack: Value,
    receipt: Value,
    session_id: Option<String>,
    native_history: Option<Artifact>,
}
fn execute(c: &Config) -> R<Value> {
    let mut s = c.startup.clone();
    convert(source(&s))?; // Source event exists before the native session can start.
    let mut job = c.job.clone();
    job.receiver
        .as_mut()
        .ok_or_else(|| err("profile_missing"))?
        .event = pin(&s.event_path)?;
    // The exact final Job is durable before Host admission (which reserves its own effects).
    convert(publish(
        &c.operation_dir.join("job.json"),
        &serde_json::to_vec(&job)?,
    ))?;
    if c.revoked_path.exists() || convert(now())? >= c.expires_at {
        return Err(err("grant_expired_or_revoked"));
    }
    let expected = digest(&serde_json::to_vec(&job)?);
    // The Host stays on its worker thread: its optional coding gate is not Send, so
    // the Host cannot come back through join(). The worker finishes the run, waits
    // for the released startup, acknowledges, and returns plain results. Host::admit
    // installs no Pi owner, so dropping the Host there releases only its state lock.
    let (released, released_rx) = std::sync::mpsc::channel::<Option<Startup>>();
    let worker = std::thread::spawn(move || -> Result<Observed, String> {
        let mut run = VerifiedRun::run(job).map_err(|e| e.to_string())?;
        let s = released_rx
            .recv()
            .ok()
            .flatten()
            .ok_or("event_not_released")?;
        let ack = run
            .ack_with_clock(&s, proof::now)
            .map_err(|e| e.to_string())?;
        let journal = run.journal();
        Ok(Observed {
            ack,
            receipt: run.receipt_value().clone(),
            session_id: journal.session_id.clone(),
            native_history: journal
                .receiver
                .as_ref()
                .and_then(|r| r.native_history.clone()),
        })
    });
    let release_result = (|| -> R<()> {
        while !s.ready_path.exists() {
            if worker.is_finished() || convert(now())? >= s.deadline_unix_seconds {
                return Err(err("native_readiness_unknown"));
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let bytes = fs::read(&s.ready_path)?;
        let r: proof::Ready = serde_json::from_slice(&bytes)?;
        if r.input_digest != expected {
            return Err(err("native_job_mismatch"));
        }
        s.input_digest = r.input_digest;
        s.container_id = r.container_id;
        s.ready_sha256 = digest(&bytes);
        convert(publish(
            &c.operation_dir.join("startup.json"),
            &serde_json::to_vec(&s)?,
        ))?;
        // Recheck revocation and expiry immediately before event release.
        if c.revoked_path.exists() || convert(now())? >= c.expires_at {
            return Err(err("grant_expired_or_revoked"));
        }
        convert(release(&s, convert(now())?))
    })();
    // Tell the worker whether the event was released; it exits either way.
    let _ = released.send(release_result.is_ok().then(|| s.clone()));
    // Always join, including release errors and HTTP disconnect. Host owns cleanup.
    let run = worker.join().map_err(|_| err("host_panicked"))?;
    release_result?;
    let run = run.map_err(|e| err(&e))?;
    let ack = run.ack;
    let receipt_path = c.operation_dir.join("native-receipt.json");
    convert(publish(&receipt_path, &serde_json::to_vec(&run.receipt)?))?;
    let envelope: session_delivery::Envelope =
        serde_json::from_slice(&fs::read(&s.envelope_path)?)?;
    let observation_ack = observation_ack(&envelope);
    let evidence = [
        pin(&receipt_path)?,
        pin(&s.event_path)?,
        pin(&s.envelope_path)?,
        pin(&c.operation_dir.join("job.json"))?,
        pin(&c.operation_dir.join("startup.json"))?,
        run.native_history
            .clone()
            .ok_or_else(|| err("history_missing"))?,
        pin(&s.ready_path)?,
    ];
    Ok(
        json!({"state":"target_observed","accepted":true,"transport_accepted":true,"target_observed":true,
        "application_result":null,"completion":null,"fixture_only":true,"provenance":"provider-disabled SDK observation; source reader/event may be synthetic; native JSONL not established",
        "native_session_id":run.session_id,"store_ack":ack,"observation_ack":observation_ack,"native_receipt_id":run.receipt["receipt_id"],"evidence":evidence,"retry_permitted":false}),
    )
}
fn call(c: &Config, config_bytes: &[u8], r: &Request) -> R<Value> {
    validate(c, r)?;
    let binding = digest(&[config_bytes, c.idempotency_key.as_bytes()].concat());
    if r.method != "observe" {
        return read_result(c, &binding);
    }
    if c.revoked_path.exists() || convert(now())? >= c.expires_at {
        return Err(err("grant_expired_or_revoked"));
    }
    if c.operation_dir.join("intent.json").exists() {
        return read_result(c, &binding);
    }
    // Exclusive reservation before any source/store/shared job writes.
    let path = c.operation_dir.join("intent.json");
    let mut f = match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return read_result(c, &binding),
        Err(e) => return Err(e.into()),
    };
    f.write_all(&serde_json::to_vec(
        &json!({"binding":binding,"request":r,"state":"unknown"}),
    )?)?;
    f.sync_all()?;
    fs::File::open(&c.operation_dir)?.sync_all()?;
    let mut result = match execute(c) {
        Ok(v) => v,
        Err(e) => {
            json!({"state":"unknown","accepted":true,"transport_accepted":null,"target_observed":null,"application_result":null,"completion":null,"error":e.to_string(),"evidence":[],"retry_permitted":false})
        }
    };
    result["input_binding_sha256"] = json!(binding);
    convert(publish(
        &c.operation_dir.join("result.json"),
        &serde_json::to_vec(&result)?,
    ))?;
    read_result(c, &binding)
}
fn main() -> R<()> {
    let args: Vec<_> = std::env::args().collect();
    // Operator-only custody recovery must retain this executable's journal pin.
    // It never enters source collection, delivery release or Store ACK.
    if args.len() == 3 && args[1] == "custody-reconcile" {
        let state = Path::new(&args[2]);
        if !state.is_absolute() {
            return Err(err("absolute_trusted_state_required"));
        }
        let mut host = work_engine_manual_host::pi_docker::Host::open(state)?;
        println!("{}", host.reconcile()?);
        return Ok(());
    }
    if args.len() != 4 || args[1] != "--config-fd" || args[3] != "rpc" {
        return Err(err("usage: mcp_pi_observation --config-fd FD rpc"));
    }
    let fd: i32 = args[2].parse()?;
    if fd < 3 {
        return Err(err("invalid_config_fd"));
    }
    let mut bytes = vec![];
    unsafe { fs::File::from_raw_fd(fd) }
        .take(16385)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 16384 {
        return Err(err("oversized_config"));
    }
    let c: Config = serde_json::from_slice(&bytes)?;
    let mut input = vec![];
    std::io::stdin().take(4097).read_to_end(&mut input)?;
    let result = (|| -> R<Value> {
        if input.len() > 4096 {
            return Err(err("oversized_request"));
        }
        call(&c, &bytes, &serde_json::from_slice(&input)?)
    })();
    let output = match result {
        Ok(v) => {
            json!({"schema_version":1,"protocol":SCHEMA,"ok":true,"result":{ "schema_version":1,"observation":v}})
        }
        Err(e) => json!({"schema_version":1,"protocol":SCHEMA,"ok":false,"error":e.to_string()}),
    };
    println!("{output}");
    Ok(())
}
