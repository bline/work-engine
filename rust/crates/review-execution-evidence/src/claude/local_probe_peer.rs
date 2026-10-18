//! Synthetic installed executable for the local no-egress boundary test.
//! This is never a provider and is unavailable without test-support.
use std::io::Read;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !args.iter().any(|arg| arg == "--model")
        || !args.iter().any(|arg| arg == "--session-id")
        || !args.iter().any(|arg| arg == "--strict-mcp-config")
        || std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok().as_deref()
            != Some("RP3_LOCAL_NO_EGRESS_CANARY")
        || std::env::var_os("WORK_ENGINE_OFFLINE_PEER").is_some()
    {
        std::process::exit(81);
    }
    let mut prompt = String::new();
    if std::io::stdin().read_to_string(&mut prompt).is_err()
        || !prompt.starts_with("RP3 local launch probe.")
    {
        std::process::exit(82);
    }
    if std::fs::write("/session/local-probe-witness", b"spawned and framed\n").is_err() {
        std::process::exit(83);
    }
    println!("{{}}");
}
