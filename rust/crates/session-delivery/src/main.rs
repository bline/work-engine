use serde_json::{Value, json};
use session_delivery::*;
use std::os::fd::FromRawFd;
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs()
}
fn read_bounded(r: impl Read, max: usize) -> Result<Vec<u8>> {
    let mut b = Vec::new();
    r.take((max + 1) as u64).read_to_end(&mut b)?;
    if b.len() > max {
        return fail("input_oversized");
    }
    Ok(b)
}
fn private_file(path: &Path, max: u64) -> Result<Vec<u8>> {
    if !path.is_absolute() {
        return fail("absolute_configuration_required");
    }
    let m = fs::symlink_metadata(path)?;
    let parent = fs::symlink_metadata(
        path.parent()
            .ok_or_else(|| Error::Contract("parent".into()))?,
    )?;
    if !m.is_file()
        || m.file_type().is_symlink()
        || m.mode() & 0o077 != 0
        || m.uid() != parent.uid()
        || parent.mode() & 0o077 != 0
        || !parent.is_dir()
        || parent.file_type().is_symlink()
        || m.len() > max
    {
        return fail("private_configuration_required");
    }
    Ok(fs::read(path)?)
}
fn source(cfg: &Config, cursor: u64, previous_revision: Option<&str>) -> Result<Observation> {
    let mut child = Command::new("python3")
        .arg(&cfg.source_adapter)
        .arg(&cfg.board_reader)
        .arg(&cfg.repository)
        .arg(cursor.to_string())
        .arg(previous_revision.unwrap_or("-"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = child.stdout.take().expect("stdout");
    let reader = std::thread::spawn(move || read_bounded(stdout, 8 * 1024 * 1024));
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            let _ = reader.join();
            return fail("source_timeout");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let bytes = reader
        .join()
        .map_err(|_| Error::Contract("source_reader_failed".into()))??;
    if status.code() == Some(17) {
        return fail("source_history_unavailable_or_conflict");
    }
    if !status.success() {
        return fail("source_unavailable_or_gap_or_oversized");
    }
    Ok(serde_json::from_slice(&bytes)?)
}
fn dispatch(store: &mut Store, config_path: &str) -> Result<()> {
    for id in store.pending_ids()? {
        store.refresh_config(serde_json::from_slice(&private_file(
            Path::new(config_path),
            16384,
        )?)?)?;
        let Some(e) = store.prepare(&id, now())? else {
            continue;
        };
        crash("after_prepare");
        if e.binding.fixture_mode == "offline" {
            store.pre_send_failure(&id, "offline_proven_pre_send")?;
            continue;
        }
        let child = Command::new(std::env::current_exe()?)
            .args([
                "--config",
                config_path,
                "fixture-receiver",
                &e.binding.recipient,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let mut child = match child {
            Ok(v) => v,
            Err(_) => {
                store.pre_send_failure(&id, "spawn_failed_pre_send")?;
                continue;
            }
        };
        // All paths close and wait this owned child, including deliberate sender crashes.
        // Crash injection before wait uses a receiver that exits promptly on EOF.
        store.possible_send(&id)?;
        if std::env::var("SESSION_DELIVERY_FIXTURE_CRASH")
            .ok()
            .as_deref()
            == Some("before_send")
        {
            drop(child.stdin.take());
            let _ = child.wait();
            crash("before_send");
        }
        let mut bytes = serde_json::to_vec(&e)?;
        bytes.push(b'\n');
        let write = child.stdin.take().expect("stdin").write_all(&bytes);
        let stdout = child.stdout.take().expect("stdout");
        let reader = std::thread::spawn(move || read_bounded(stdout, 128 * 1024));
        let deadline = Instant::now() + Duration::from_secs(3);
        let status = loop {
            if let Some(s) = child.try_wait()? {
                break s;
            }
            if Instant::now() >= deadline {
                child.kill()?;
                break child.wait()?;
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        let output = reader
            .join()
            .map_err(|_| Error::Contract("receiver_read_failed".into()))?;
        crash("after_send");
        if write.is_err() || !status.success() {
            continue;
        }
        if let Ok(ref bytes) = output
            && let Ok(v) = serde_json::from_slice::<Value>(bytes)
            && v["outcome"] == "busy"
            && v["attempt_id"] == e.attempt_id
            && v["binding_sha256"] == e.binding_sha256
        {
            store.unknown_hold(&id, "held_busy_no_observation")?;
            continue;
        }
        let receipt: Receipt = match output.and_then(|v| Ok(serde_json::from_slice(&v)?)) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if receipt.schema != "isolated-session-receipt-v1"
            || !receipt.fixture_only
            || !receipt.transport_accepted
        {
            continue;
        }
        let mut transport = receipt.ack.clone();
        transport.stage = "transport_accepted".into();
        if store
            .ack(&e.binding.recipient, &e.binding.principal, transport, now())
            .is_err()
        {
            continue;
        }
        crash("after_transport_ack");
        if receipt.ack.stage == "target_observed" {
            let _ = store.ack(
                &e.binding.recipient,
                &e.binding.principal,
                receipt.ack,
                now(),
            );
        }
        crash("after_ack");
    }
    Ok(())
}
fn receiver(cfg: &Config, name: &str) -> Result<()> {
    let b = cfg
        .targets
        .iter()
        .find(|b| b.recipient == name)
        .ok_or_else(|| Error::Contract("unknown_fixture_target".into()))?;
    let bytes = read_bounded(std::io::stdin(), 1024 * 1024)?;
    let e: Envelope = serde_json::from_slice(&bytes)?;
    if e.schema != "work-engine.delivery-envelope/v1"
        || !e.fixture_only
        || e.authority != "observation_only"
        || &e.binding != b
        || e.binding_sha256 != b.sha()
        || e.subscription_id != b.recipient
        || e.attempt_id.is_empty()
        || now() >= b.expires_at
        || !b.active
        || digest(e.event.raw_payload.as_bytes()) != e.event.payload_sha256
    {
        return fail("fixture_target_mismatch");
    }
    if b.fixture_mode == "busy" {
        println!(
            "{}",
            json!({"outcome":"busy","attempt_id":e.attempt_id,"binding_sha256":e.binding_sha256})
        );
        return Ok(());
    }
    if b.fixture_mode == "offline" {
        return fail("fixture_offline");
    }
    let db = cfg
        .database
        .with_extension(format!("receiver-{}.sqlite", digest(name.as_bytes())));
    let conn = rusqlite::Connection::open(db)?;
    conn.execute_batch("PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS observations(id TEXT PRIMARY KEY, envelope_digest TEXT NOT NULL, raw TEXT NOT NULL);")?;
    let hash = e.sha();
    conn.execute(
        "INSERT OR IGNORE INTO observations VALUES(?1,?2,?3)",
        rusqlite::params![e.delivery_id, hash, serde_json::to_string(&e)?],
    )?;
    let saved: String = conn.query_row(
        "SELECT envelope_digest FROM observations WHERE id=?1",
        [&e.delivery_id],
        |r| r.get(0),
    )?;
    if saved != hash {
        return fail("fixture_delivery_conflict");
    }
    if b.fixture_mode == "lost_ack" {
        return Ok(());
    }
    let receipt = Receipt {
        schema: "isolated-session-receipt-v1".into(),
        fixture_only: true,
        transport_accepted: true,
        ack: Ack {
            schema: "work-engine.delivery-observation/v1".into(),
            attempt_id: e.attempt_id.clone(),
            delivery_id: e.delivery_id.clone(),
            event_id: e.event.event_id.clone(),
            binding_sha256: e.binding_sha256.clone(),
            envelope_sha256: hash,
            payload_sha256: e.event.payload_sha256.clone(),
            generation: b.generation,
            stage: "target_observed".into(),
        },
    };
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}
fn rpc(store: &mut Store, input: &[u8]) -> Result<Value> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Request {
        schema: String,
        method: String,
        principal: String,
        recipient: String,
        ack: Option<Ack>,
        after: Option<String>,
    }
    let request: Request = serde_json::from_slice(input)?;
    if request.schema != "session-delivery-rpc-v1" {
        return fail("protocol_mismatch");
    }
    match request.method.as_str() {
        "subscribe" if request.ack.is_none() && request.after.is_none() => {
            store.subscribe(&request.recipient, &request.principal)
        }
        "status" if request.ack.is_none() => store.status_after(
            &request.recipient,
            &request.principal,
            request.after.as_deref(),
        ),
        "ack" if request.after.is_none() => store.ack(
            &request.recipient,
            &request.principal,
            request
                .ack
                .ok_or_else(|| Error::Contract("missing_ack".into()))?,
            now(),
        ),
        _ => fail("unsupported_method"),
    }
}
fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("schemas") {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({"envelope":schemars::schema_for!(Envelope),"observation":schemars::schema_for!(Ack),"fixture_config":schemars::schema_for!(Config),"receipt":schemars::schema_for!(Receipt)})
            )?
        );
        return Ok(());
    }
    if args.len() < 4 || !["--config", "--config-fd"].contains(&args[1].as_str()) {
        return fail(
            "usage: --config PRIVATE_FIXTURE_CONFIG rpc|observe|dispatch|run|fixture-receiver",
        );
    }
    let config_bytes = if args[1] == "--config-fd" {
        if args[3] != "rpc" {
            return fail("config_fd_rpc_only");
        }
        let fd: i32 = args[2]
            .parse()
            .map_err(|_| Error::Contract("invalid_config_fd".into()))?;
        if fd < 3 {
            return fail("invalid_config_fd");
        }
        // This process owns the explicitly inherited descriptor supplied by the
        // pinned facade. No caller-selected executable/config path is resolved.
        let file = unsafe { fs::File::from_raw_fd(fd) };
        let meta = file.metadata()?;
        if !meta.is_file() || meta.mode() & 0o077 != 0 {
            return fail("private_config_fd_required");
        }
        read_bounded(file, 16384)?
    } else {
        private_file(Path::new(&args[2]), 16384)?
    };
    let cfg: Config = serde_json::from_slice(&config_bytes)?;
    cfg.validate()?;
    let parent = fs::symlink_metadata(
        cfg.database
            .parent()
            .ok_or_else(|| Error::Contract("db_parent".into()))?,
    )?;
    if !parent.is_dir() || parent.mode() & 0o077 != 0 || parent.file_type().is_symlink() {
        return fail("private_store_parent_required");
    }
    if cfg.database.exists() {
        let m = fs::symlink_metadata(&cfg.database)?;
        if !m.is_file() || m.file_type().is_symlink() {
            return fail("unsafe_database");
        }
    }
    if args[3] == "fixture-receiver" {
        return receiver(&cfg, args.get(4).map(String::as_str).unwrap_or(""));
    }
    let mut store = Store::open(cfg.clone())?;
    fs::set_permissions(&cfg.database, fs::Permissions::from_mode(0o600))?;
    match args[3].as_str() {
        "rpc" => {
            let result =
                read_bounded(std::io::stdin(), 128 * 1024).and_then(|v| rpc(&mut store, &v));
            let output = match result {
                Ok(mut v) => {
                    v["schema_version"] = json!(1);
                    json!({"schema_version":1,"protocol":"session-delivery-rpc-v1","ok":true,"result":v})
                }
                Err(e) => {
                    json!({"schema_version":1,"protocol":"session-delivery-rpc-v1","ok":false,"error":e.to_string()})
                }
            };
            println!("{}", serde_json::to_string(&output)?);
        }
        "observe" => {
            let saved = store.state()?;
            match source(&cfg, saved.cursor, saved.source_revision.as_deref())
                .and_then(|v| store.ingest(v))
            {
                Ok(()) => {}
                Err(e) => {
                    store.source_hold(&e.to_string())?;
                    return Err(e);
                }
            }
        }
        "dispatch" => dispatch(&mut store, &args[2])?,
        "run" => {
            let cycles: u64 = args.get(4).and_then(|v| v.parse().ok()).unwrap_or(1);
            if cycles == 0 || cycles > 100 {
                return fail("bounded_cycles_required");
            }
            for _ in 0..cycles {
                let saved = store.state()?;
                match source(&cfg, saved.cursor, saved.source_revision.as_deref())
                    .and_then(|v| store.ingest(v))
                {
                    Ok(()) => {}
                    Err(e) => store.source_hold(&e.to_string())?,
                }
                dispatch(&mut store, &args[2])?;
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        _ => return fail("unknown_command"),
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
