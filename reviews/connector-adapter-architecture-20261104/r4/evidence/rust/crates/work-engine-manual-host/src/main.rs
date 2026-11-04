use std::{env, fs, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf};
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
  pi-docker run|reconcile|cancel|inspect|ack|inventory (provider-disabled component)\n\
Execution/inspection output: --compact, --out FILE (new absolute JSON log; reserved before effects)\n\
ack only acknowledges existing receipt bytes; it does not poll or issue a receipt.\n\
inspect reads recorded state; it does not refresh observations."
        );
        return Ok(());
    }
    let mut args = arguments.into_iter();
    let verb = args.next().unwrap_or_default();
    if verb == "pi-docker" {
        return pi_docker(args.collect());
    }
    if matches!(verb.as_str(), "schema" | "--version") && args.len() != 0 {
        return Err(work_engine_manual_host::Error::Refused(
            "schema and --version accept no options".into(),
        ));
    }
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
    let mut compact = false;
    let mut out = None;
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
            "--compact" => compact = true,
            "--out" => {
                out = Some(PathBuf::from(args.next().ok_or_else(|| {
                    work_engine_manual_host::Error::Refused("--out requires a path".into())
                })?))
            }
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
    if compact && ((verb == "inspect" && !summary) || bind_only) {
        return Err(work_engine_manual_host::Error::Refused(
            "--compact requires a receipt or inspect --summary".into(),
        ));
    }
    // A new log is reserved before any dispatch/closure effects. Reusing an
    // existing path refuses rather than truncating state, packets or results.
    let mut log = if let Some(path) = &out {
        if !path.is_absolute() {
            return Err(work_engine_manual_host::Error::Refused(
                "--out path must be absolute".into(),
            ));
        }
        let parent = path.parent().ok_or_else(|| {
            work_engine_manual_host::Error::Refused("--out parent missing".into())
        })?;
        fs::create_dir_all(parent)?;
        Some(
            fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(path)?,
        )
    } else {
        None
    };
    let result = (|| -> Result<serde_json::Value> {
        Ok(match verb.as_str() {
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
                        work_engine_manual_host::Error::Refused(
                            "--request or --draft required".into(),
                        )
                    })?)?)?
                };
                if bind_only {
                    return Ok(serde_json::to_value(&r)?);
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
                        "ack accepts --state, --ack-receipt and output options".into(),
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
        })
    })();
    if let Some(file) = log.as_mut() {
        match &result {
            Ok(value) => serde_json::to_writer(&mut *file, value)?,
            Err(error) => serde_json::to_writer(
                &mut *file,
                &serde_json::json!({"status":"not_established","detail":error.to_string()}),
            )?,
        }
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::File::open(out.as_ref().unwrap().parent().unwrap())?.sync_all()?;
    }
    let result = result?;
    let shown = if compact {
        work_engine_manual_host::compact_output(&result, out.as_deref())
    } else {
        result
    };
    println!("{}", serde_json::to_string(&shown)?);
    Ok(())
}

fn pi_docker(args: Vec<String>) -> Result<()> {
    use work_engine_manual_host::{
        Error,
        pi_docker::{Host as PiHost, Job},
    };
    let refused = || {
        Error::Refused("pi-docker run --request FILE | reconcile|cancel|inspect --state DIR | ack --state DIR --receipt ID | inventory --root DIR".into())
    };
    let action = args.first().ok_or_else(refused)?.as_str();
    let mut options = std::collections::BTreeMap::new();
    let mut rest = args[1..].iter();
    while let Some(key) = rest.next() {
        let value = rest.next().ok_or_else(refused)?;
        if options.insert(key.as_str(), value.as_str()).is_some() {
            return Err(refused());
        }
    }
    let get = |key: &str| options.get(key).copied().ok_or_else(refused);
    let output = match action {
        "inventory" if options.len() == 1 => serde_json::to_value(
            work_engine_manual_host::pi_docker::inventory(&PathBuf::from(get("--root")?))?,
        )?,
        "run" if options.len() == 1 => {
            let job: Job = serde_json::from_slice(&fs::read(get("--request")?)?)?;
            PiHost::admit(job)?.run()?
        }
        "reconcile" if options.len() == 1 => {
            PiHost::open(&PathBuf::from(get("--state")?))?.reconcile()?
        }
        "cancel" if options.len() == 1 => {
            work_engine_manual_host::pi_docker::request_cancel(&PathBuf::from(get("--state")?))?
        }
        "inspect" if options.len() == 1 => {
            let dir = PathBuf::from(get("--state")?);
            let host = PiHost::open(&dir)?;
            serde_json::to_value(host.journal())?
        }
        "ack" if options.len() == 2 => {
            PiHost::open(&PathBuf::from(get("--state")?))?.acknowledge(get("--receipt")?)?
        }
        _ => return Err(refused()),
    };
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
