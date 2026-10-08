use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{BufRead, Read, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde::Deserialize;
use serde_json::json;
use work_engine_types::CodecContract;

const MAX_FRAME: usize = lifecycle_test_support::MAX_PEER_FRAME;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    subject_id: String,
    effect_id: String,
    attempt_id: String,
    incarnation_id: String,
    input_id: String,
    text: String,
    text_digest: String,
}

fn read_frame(stream: &mut UnixStream) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte)?;
        if byte[0] == b'\n' {
            return Ok(bytes);
        }
        if bytes.len() == MAX_FRAME {
            return Err(std::io::Error::other("frame too large"));
        }
        bytes.push(byte[0]);
    }
}

fn handle(
    mut stream: UnixStream,
    ledger: &Path,
    successor: &str,
    release: Option<&Path>,
) -> std::io::Result<()> {
    let bytes = read_frame(&mut stream)?;
    let request: Request = serde_json::from_slice(&bytes).map_err(std::io::Error::other)?;
    if request.text.len() > 65_536
        || request.text_digest
            != CodecContract::LifecycleTextInputV1
                .digest_binary(request.text.as_bytes())
                .map_err(std::io::Error::other)?
                .hex()
    {
        return Err(std::io::Error::other("invalid input digest"));
    }
    let source = format!("source:{}", request.attempt_id);
    let thread = format!("thread:{}", request.attempt_id);
    let turn = format!("turn:{}", request.attempt_id);
    let successor_thread = format!("successor:{}", request.attempt_id);
    let final_text = json!({
        "protocol_version":1,
        "predecessor_safe":true,
        "source_frozen":true,
        "checkpoint_ready":true,
        "switch_authorized":true,
        "successor_observed":true,
        "rehydration_verified":true,
        "successor_context":successor,
        "successor_thread":successor_thread,
    })
    .to_string();
    let result = json!({
        "source_id":source,"effect_id":request.effect_id,"attempt_id":request.attempt_id,
        "incarnation_id":request.incarnation_id,"provider_thread_id":thread,
        "provider_turn_id":turn,"final_text":final_text,"outcome":"completed",
        "settlement_kind":"established","settlement_evidence":format!("settlement:{source}"),
    });
    let ledger_event = json!({"subject_id":request.subject_id,"input_id":request.input_id,
        "attempt_id":request.attempt_id,"incarnation_id":request.incarnation_id,
        "text_digest":request.text_digest,"source_id":source,"final_text":final_text});
    let mut file = OpenOptions::new().create(true).append(true).open(ledger)?;
    writeln!(file, "{ledger_event}")?;
    file.sync_all()?;
    if let Some(release) = release {
        let started = Instant::now();
        while !release.exists() {
            if started.elapsed() > Duration::from_secs(15) {
                return Err(std::io::Error::other("release timed out"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let mut bytes = serde_json::to_vec(&result).map_err(std::io::Error::other)?;
    bytes.push(b'\n');
    stream.write_all(&bytes)?;
    stream.flush()?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let args: Vec<_> = env::args_os().collect();
    if args.get(1).is_some_and(|arg| arg == "verify") {
        if args.len() != 5 {
            return Err(std::io::Error::other(
                "usage: controlled_peer verify <ledger> <source> <sha256>",
            ));
        }
        let ledger = Path::new(&args[2]);
        let source = args[3]
            .to_str()
            .ok_or_else(|| std::io::Error::other("source encoding"))?;
        let expected = args[4]
            .to_str()
            .ok_or_else(|| std::io::Error::other("digest encoding"))?;
        let verifier_barrier = env::var_os("LIFECYCLE_VERIFIER_BARRIER_DIR").map(PathBuf::from);
        if let Some(root) = &verifier_barrier {
            fs::write(
                root.join("verifier_started.reached"),
                std::process::id().to_string(),
            )?;
        }
        let mut activation = String::new();
        std::io::stdin().lock().read_line(&mut activation)?;
        if activation != "GO\n" {
            return Err(std::io::Error::other("activation missing"));
        }
        if let Some(root) = &verifier_barrier
            && root.join("verifier_hold.enabled").exists()
        {
            fs::write(root.join("verifier_activated.reached"), b"reached\n")?;
            let started = Instant::now();
            while !root.join("verifier_activated.release").exists() {
                if started.elapsed() > Duration::from_secs(45) {
                    return Err(std::io::Error::other("verifier hold timed out"));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let mut matches = 0;
        for line in fs::read_to_string(ledger)?.lines() {
            let event: serde_json::Value =
                serde_json::from_str(line).map_err(std::io::Error::other)?;
            if event["source_id"] == source {
                let text = event["final_text"]
                    .as_str()
                    .ok_or_else(|| std::io::Error::other("missing result"))?;
                let digest = CodecContract::BinaryArtifactV1
                    .digest_binary(text.as_bytes())
                    .map_err(std::io::Error::other)?
                    .hex();
                if digest != expected {
                    return Err(std::io::Error::other("result hash mismatch"));
                }
                matches += 1;
            }
        }
        if matches != 1 {
            return Err(std::io::Error::other("source not unique"));
        }
        return Ok(());
    }
    if !(4..=5).contains(&args.len()) {
        return Err(std::io::Error::other(
            "usage: controlled_peer <socket> <ledger> <successor> [release]",
        ));
    }
    let socket = Path::new(&args[1]);
    let ledger = Path::new(&args[2]);
    let successor = args[3]
        .to_str()
        .ok_or_else(|| std::io::Error::other("invalid successor"))?;
    let release = args.get(4).map(PathBuf::from);
    if socket.exists() {
        fs::remove_file(socket)?;
    }
    let listener = UnixListener::bind(socket)?;
    fs::set_permissions(socket, fs::Permissions::from_mode(0o600))?;
    File::create(ledger)?.sync_all()?;
    for connection in listener.incoming() {
        let stream = connection?;
        let _ = handle(stream, ledger, successor, release.as_deref());
    }
    Ok(())
}
