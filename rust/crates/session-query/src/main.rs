use serde_json::{Value, json};
use std::{
    env, fs,
    io::{self, Read},
    os::unix::fs::PermissionsExt,
    process,
};
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let a: Vec<String> = env::args().collect();
    if a.len() != 4 && a.len() != 5 {
        return Err(
            "usage: work-engine-query PROFILE QUERY --json|--human [ARGUMENTS_FILE|-]".into(),
        );
    }
    if a[3] != "--json" && a[3] != "--human" {
        return Err("invalid format".into());
    }
    let profile: Value =
        serde_json::from_slice(&fs::read(&a[1]).map_err(|_| "profile unavailable")?)
            .map_err(|_| "invalid profile")?;
    let mut input = Vec::new();
    if a.len() == 5 {
        if a[4] == "-" {
            io::stdin()
                .take(262145)
                .read_to_end(&mut input)
                .map_err(|_| "input unavailable")?;
        } else {
            let meta = fs::symlink_metadata(&a[4]).map_err(|_| "input unavailable")?;
            if !meta.file_type().is_file()
                || meta.permissions().mode() & 0o077 != 0
                || meta.len() > 262144
            {
                return Err("input file must be private and bounded".into());
            }
            input = fs::read(&a[4]).map_err(|_| "input unavailable")?;
        }
    } else {
        input.extend_from_slice(b"{}");
    }
    if input.len() > 262144 {
        return Err("input too large".into());
    }
    let args: Value = serde_json::from_slice(&input).map_err(|_| "invalid arguments")?;
    let result = session_query::query(profile, &a[2], args)?;
    if result.get("isError").and_then(Value::as_bool) == Some(true) {
        return Err(format!("query failed: {}", result));
    }
    let content = result.get("structuredContent").ok_or("invalid response")?;
    if a[3] == "--json" {
        println!(
            "{}",
            json!({"schema":"work-engine.session-query.v1", "result":content})
        );
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(content).map_err(|_| "invalid response")?
        );
    }
    Ok(())
}
