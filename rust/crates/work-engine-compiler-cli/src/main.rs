mod aeg;
mod manifest_protocol;
mod protocol;
mod source_snapshot;

use std::io::{Read, Write};
use std::sync::{Arc, atomic::AtomicBool};

fn main() {
    let cancelled = Arc::new(AtomicBool::new(false));
    for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
        if let Err(issue) = signal_hook::flag::register(signal, Arc::clone(&cancelled)) {
            eprintln!("signal registration failed: {issue}");
            std::process::exit(1);
        }
    }
    let mut request = Vec::new();
    let mut input = std::io::stdin().take((protocol::MAX_ENVELOPE_BYTES + 1) as u64);
    if let Err(issue) = input.read_to_end(&mut request) {
        let error = protocol::ProtocolError {
            code: "internal_error",
            path: None,
            message: format!("stdin read failed: {issue}"),
        };
        let _ = std::io::stdout().write_all(&protocol::error_envelope(1, None, &error));
        eprintln!("{}", error.message);
        std::process::exit(1);
    }
    let (version, id, result) = protocol::process(&request, &cancelled);
    let operation = if version == 3 {
        protocol::strict_json(&request).ok().and_then(|v| {
            v.get("operation")
                .and_then(|s| s.as_str())
                .map(str::to_owned)
        })
    } else {
        None
    };
    match result {
        Ok(bytes) => {
            if let Err(issue) = std::io::stdout().write_all(&bytes) {
                eprintln!("stdout write failed: {issue}");
                std::process::exit(1);
            }
        }
        Err(issue) => {
            let envelope = if version == 3 {
                manifest_protocol::error_envelope(id.as_deref(), operation.as_deref(), &issue)
            } else {
                protocol::error_envelope(version, id.as_deref(), &issue)
            };
            let _ = std::io::stdout().write_all(&envelope);
            eprintln!("{}", issue.message);
            std::process::exit(issue.exit_code());
        }
    }
}
