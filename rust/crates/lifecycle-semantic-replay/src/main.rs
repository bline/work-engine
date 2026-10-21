//! Reads one synthetic replay envelope from stdin and prints a fixture verdict.

use lifecycle_semantic::SemanticPolicy;
use lifecycle_semantic_replay::{OfflineReplayInput, replay_fixture};
use std::io::{self, Read};

fn main() {
    let mut bytes = Vec::new();
    if io::stdin()
        .take(2 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .is_err()
    {
        eprintln!("cannot read fixture input");
        std::process::exit(2);
    }
    let input: OfflineReplayInput = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid fixture input: {error}");
            std::process::exit(2);
        }
    };
    match replay_fixture(&input, &SemanticPolicy::default()) {
        Ok(verdict) => println!(
            "{}",
            serde_json::to_string(&verdict).expect("serializable verdict")
        ),
        Err(error) => {
            eprintln!("offline replay failed: {error:?}");
            std::process::exit(1);
        }
    }
}
