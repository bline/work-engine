//! Controlled test peer: actual process I/O, with an independent append-only entry witness.
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    time::Duration,
};
fn main() {
    let count = std::env::args().nth(1).expect("entry witness path");
    let mut witness = OpenOptions::new()
        .create(true)
        .append(true)
        .open(count)
        .unwrap();
    writeln!(witness, "{}", std::process::id()).unwrap();
    witness.sync_all().unwrap();
    let mut input = Vec::new();
    std::io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut input)
        .unwrap();
    assert!(input.len() <= 1024 * 1024);
    let request: Value = serde_json::from_slice(&input).unwrap();
    assert_eq!(request["profile"], "review-host-controlled-initial-v1");
    assert!(
        request["compilerProjection"]["roles"]
            .get(request["selectedRole"].as_str().unwrap())
            .is_some()
    );
    let input = &request["input"];
    std::thread::sleep(Duration::from_millis(
        input["delayMillis"].as_u64().unwrap_or(0),
    ));
    match input["mode"].as_str().unwrap_or("result") {
        "malformed" => print!("{{invalid"),
        "oversized" => {
            std::io::stdout()
                .write_all(&vec![b'x'; 1024 * 1024 + 1])
                .unwrap();
        }
        _ => {
            let candidate = &request["preparedRequest"]["candidate"];
            let mut result = input["resultTemplate"].clone();
            result["subject"] = json!({"commit":candidate["commit"],"tree":candidate["tree"],"patchIdentity":candidate["patchIdentity"]});
            print!("{result}");
        }
    }
}
