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
    let arguments: Vec<_> = env::args().skip(1).collect();
    if arguments.is_empty()
        || arguments
            .iter()
            .any(|v| matches!(v.as_str(), "--help" | "-h" | "help"))
    {
        println!(
            "Usage: work-engine-manual-host <command> [options]\n\
Commands:\n\
  dispatch --draft FILE [--bind-only] [--wait] [--ack-receipt ID]\n\
  dispatch --request FILE [--wait] [--ack-receipt ID]\n\
  collect --state DIR --operation ID [--wait] [--reconcile-accounting] [--ack-receipt ID]\n\
  close --state DIR --request FILE [--ack-receipt ID]\n\
  ack --state DIR --ack-receipt ID\n\
  inspect --state DIR [--summary [--operation ID]]\n\
  schema | --version\n\
ack only acknowledges existing receipt bytes; it does not poll or issue a receipt.\n\
inspect reads recorded state; it does not refresh observations."
        );
        return Ok(());
    }
    let mut args = arguments.into_iter();
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
    let mut reconcile_accounting = false;
    let mut summary = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--request" => request = args.next(),
            "--draft" => draft = args.next(),
            "--bind-only" => bind_only = true,
            "--state" => state = args.next(),
            "--operation" => operation = args.next(),
            "--ack-receipt" => ack = args.next(),
            "--wait" => wait = true,
            "--reconcile-accounting" => reconcile_accounting = true,
            "--summary" => summary = true,
            _ => {
                return Err(work_engine_manual_host::Error::Refused(format!(
                    "unknown argument {a}"
                )));
            }
        }
    }
    if summary && verb != "inspect" {
        return Err(work_engine_manual_host::Error::Refused(
            "--summary is supported only by inspect".into(),
        ));
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
        "ack" => {
            if request.is_some()
                || draft.is_some()
                || operation.is_some()
                || wait
                || bind_only
                || reconcile_accounting
                || summary
            {
                return Err(work_engine_manual_host::Error::Refused(
                    "ack accepts only --state and --ack-receipt".into(),
                ));
            }
            let dir = PathBuf::from(state.ok_or_else(|| {
                work_engine_manual_host::Error::Refused("--state required".into())
            })?);
            let receipt = ack.ok_or_else(|| {
                work_engine_manual_host::Error::Refused("--ack-receipt required".into())
            })?;
            Host::open(&dir, None)?.acknowledge_receipt(&receipt)?
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
                let operation = operation.ok_or_else(|| {
                    work_engine_manual_host::Error::Refused("--operation required".into())
                })?;
                if reconcile_accounting {
                    h.reconcile_accounting(&operation)?
                } else {
                    h.collect(&operation, wait)?
                }
            }
        }
        "inspect" => {
            let dir = PathBuf::from(state.ok_or_else(|| {
                work_engine_manual_host::Error::Refused("--state required".into())
            })?);
            work_engine_manual_host::inspect(&dir, operation.as_deref(), summary)?
        }
        _ => {
            return Err(work_engine_manual_host::Error::Refused(
                "use --help for dispatch/inspect/collect/close/ack".into(),
            ));
        }
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
