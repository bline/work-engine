mod protocol;

use std::io::{Read, Write};

fn main() {
    let mut request = Vec::new();
    let mut input = std::io::stdin().take((protocol::MAX_ENVELOPE_BYTES + 1) as u64);
    if let Err(issue) = input.read_to_end(&mut request) {
        let error = protocol::ProtocolError {
            code: "internal_error",
            path: None,
            message: format!("stdin read failed: {issue}"),
        };
        let _ = std::io::stdout().write_all(&protocol::error_envelope(&error));
        eprintln!("{}", error.message);
        std::process::exit(1);
    }
    let result = protocol::process(&request);
    match result {
        Ok(bytes) => {
            if let Err(issue) = std::io::stdout().write_all(&bytes) {
                eprintln!("stdout write failed: {issue}");
                std::process::exit(1);
            }
        }
        Err(issue) => {
            let _ = std::io::stdout().write_all(&protocol::error_envelope(&issue));
            eprintln!("{}", issue.message);
            std::process::exit(if issue.code == "internal_error" { 1 } else { 2 });
        }
    }
}
