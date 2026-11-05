#[path = "../tests/support/documentation_claims.rs"]
mod documentation_claims;
use std::path::Path;
fn main() {
    if let Err(error) = run() {
        eprintln!("claims-documentation: {error}");
        std::process::exit(1);
    }
}
fn run() -> documentation_claims::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 3 || args[1] != "--state" {
        return Err("usage: prepare|cleanup --state ABS_MANIFEST | advance --state ABS_MANIFEST --case code|docs|unresolved".into());
    }
    let state = Path::new(&args[2]);
    let value = match (args[0].as_str(), args.len()) {
        ("prepare", 3) => documentation_claims::prepare(state)?,
        ("cleanup", 3) => documentation_claims::cleanup(state)?,
        ("advance", 5) if args[3] == "--case" => documentation_claims::advance(state, &args[4])?,
        _ => return Err("unknown action or flags".into()),
    };
    println!("{}", serde_json::to_string(&value)?);
    Ok(())
}
