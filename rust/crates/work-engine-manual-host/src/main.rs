use std::{env, fs, path::PathBuf};
use work_engine_manual_host::{CloseRequest, Host, Request, Result, SCHEMA};
fn main() {
    work_engine_manual_host::install_signals();
    if let Err(e) = run() {
        eprintln!(
            "{}",
            serde_json::json!({"status":"not_established","detail":e.to_string()})
        );
        std::process::exit(32);
    }
}
fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let verb = args.next().unwrap_or_default();
    if verb == "schema" {
        println!(
            "{}",
            serde_json::to_string_pretty(&work_engine_manual_host::public_schemas())?
        );
        return Ok(());
    }
    if verb == "--version" {
        println!("{SCHEMA}");
        return Ok(());
    }
    let mut request = None;
    let mut state = None;
    let mut operation = None;
    let mut ack = None;
    let mut wait = false;
    let mut draft = None;
    let mut bind_only = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--request" => request = args.next(),
            "--draft" => draft = args.next(),
            "--bind-only" => bind_only = true,
            "--state" => state = args.next(),
            "--operation" => operation = args.next(),
            "--ack-receipt" => ack = args.next(),
            "--wait" => wait = true,
            _ => {
                return Err(work_engine_manual_host::Error::Refused(format!(
                    "unknown argument {a}"
                )));
            }
        }
    }
    let result = match verb.as_str() {
        "dispatch" => {
            if request.is_some() && draft.is_some() {
                return Err(work_engine_manual_host::Error::Refused(
                    "choose --request or --draft".into(),
                ));
            }
            let r: Request = if let Some(path) = draft {
                work_engine_manual_host::bind_draft(serde_json::from_slice(&fs::read(path)?)?)?
            } else {
                serde_json::from_slice(&fs::read(request.ok_or_else(|| {
                    work_engine_manual_host::Error::Refused("--request or --draft required".into())
                })?)?)?
            };
            if bind_only {
                println!("{}", serde_json::to_string_pretty(&r)?);
                return Ok(());
            }
            let mut h = Host::open(&r.state_dir, Some(&r))?;
            if let Some(a) = ack {
                h.acknowledge(&a)?;
            }
            let id = h.dispatch(r)?;
            if wait {
                h.collect(&id, true)?
            } else {
                h.receipt(&id)?
            }
        }
        "collect" | "close" => {
            let dir = PathBuf::from(state.ok_or_else(|| {
                work_engine_manual_host::Error::Refused("--state required".into())
            })?);
            let mut h = Host::open(&dir, None)?;
            if let Some(a) = ack {
                h.acknowledge(&a)?;
            }
            if verb == "close" {
                let c: CloseRequest =
                    serde_json::from_slice(&fs::read(request.ok_or_else(|| {
                        work_engine_manual_host::Error::Refused("--request required".into())
                    })?)?)?;
                h.close(c)?
            } else {
                h.collect(
                    &operation.ok_or_else(|| {
                        work_engine_manual_host::Error::Refused("--operation required".into())
                    })?,
                    wait,
                )?
            }
        }
        "inspect" => {
            let dir = PathBuf::from(state.ok_or_else(|| {
                work_engine_manual_host::Error::Refused("--state required".into())
            })?);
            // Atomic state replacement permits concurrent read-only inspection.
            serde_json::from_slice::<serde_json::Value>(&fs::read(dir.join("state.json"))?)?
        }
        _ => {
            return Err(work_engine_manual_host::Error::Refused(
                "use dispatch/inspect/collect/close; --version reports schema".into(),
            ));
        }
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
