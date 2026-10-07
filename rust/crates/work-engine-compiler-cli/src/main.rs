mod aeg;
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
    match result {
        Ok(bytes) => {
            if let Err(issue) = std::io::stdout().write_all(&bytes) {
                eprintln!("stdout write failed: {issue}");
                std::process::exit(1);
            }
        }
        Err(issue) => {
            let _ = std::io::stdout().write_all(&protocol::error_envelope(
                version,
                id.as_deref(),
                &issue,
            ));
            eprintln!("{}", issue.message);
            std::process::exit(issue.exit_code());
        }
    }
}
