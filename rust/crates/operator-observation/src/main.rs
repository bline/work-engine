use operator_observation::{Observation, Snapshot, collect, digest, terminal_text};
use session_query::{Profile, Query};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    time::Duration,
};

fn publish(path: &str, bytes: &[u8]) -> Result<(), &'static str> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err("output_path_must_be_absolute");
    }
    let tmp = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|s| s.to_str())
            .ok_or("invalid_output")?,
        std::process::id()
    ));
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
        .map_err(|_| "output_unavailable")?;
    let result = (|| {
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|_| "output_unavailable")?;
        fs::rename(&tmp, path).map_err(|_| "output_unavailable")
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}
fn human(s: &Snapshot) {
    println!(
        "{} · {} selected sessions · observed {} ms since epoch",
        terminal_text(&s.label),
        s.sessions.len(),
        s.observed_at_ms
    );
    println!("Session activity only; task execution, completion and spend are not established.");
    for session in &s.sessions {
        match &session.observation {
            Observation::Unavailable { code } => {
                println!("{} · unavailable: {}", session.session_id, code)
            }
            Observation::Available {
                host,
                title,
                entries,
            } => {
                println!(
                    "{} · {} · {}",
                    session.session_id,
                    host,
                    terminal_text(title.as_deref().unwrap_or("Untitled session"))
                );
                for e in entries {
                    println!(
                        "  {} {}\n{}",
                        e.role,
                        terminal_text(e.tool.as_deref().unwrap_or("")),
                        terminal_text(&e.text)
                    );
                }
            }
        }
    }
}
fn run() -> Result<(), &'static str> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("selected-task") {
        return selected_task(&args[1..]);
    }
    if args.is_empty() || args == ["--help"] {
        println!(
            "work-engine-observe PROFILE --label LABEL --session UUID [--session UUID ...] [--human] [--output /absolute/snapshot.json --interval SECONDS]\nAlso: work-engine-observe selected-task ENDPOINT TASK_ID [--delivery ID --profile PROFILE --replay FILE --output FILE --interval SECONDS --human [--inspect owner|questions|result|lifecycle|native_delivery_status|command_result|control_result|activity]].\nRead-only selected-session observation through the pinned Events compositor. Output is private, atomically replaced JSON. Default is one observation; --interval (2..3600) repeats only with --output. No provider or command operation is available."
        );
        return Ok(());
    }
    let (mut label, mut ids, mut output, mut interval, mut is_human) =
        (String::new(), Vec::new(), None, None, false);
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--human" {
            is_human = true;
            i += 1;
            continue;
        }
        let v = args.get(i + 1).ok_or("invalid_arguments")?;
        match args[i].as_str() {
            "--label" if label.is_empty() => label = v.clone(),
            "--session" => ids.push(v.clone()),
            "--output" if output.is_none() => output = Some(v.clone()),
            "--interval" if interval.is_none() => {
                interval = Some(v.parse::<u64>().map_err(|_| "invalid_interval")?)
            }
            _ => return Err("invalid_arguments"),
        };
        i += 2;
    }
    operator_observation::validate_selection(&label, &ids)?;
    if interval.is_some_and(|n| !(2..=3600).contains(&n))
        || (interval.is_some() && output.is_none())
    {
        return Err("invalid_interval");
    }
    let bytes = session_query::read_file(&args[0], 16384, true)?;
    let profile: Profile = serde_json::from_slice(&bytes).map_err(|_| "invalid_profile")?;
    profile.validate()?;
    if profile.bridge.is_some() {
        return Err("observation_profile_must_have_no_bridge");
    }
    let source_id = digest(&bytes);
    loop {
        let snapshot = collect(&source_id, &label, &ids, |arguments| {
            session_query::query(&profile, &Query::parse("read_session", arguments)?)
        })?;
        let bytes = serde_json::to_vec(&snapshot).map_err(|_| "invalid_snapshot")?;
        Snapshot::parse(&bytes)?;
        if let Some(path) = &output {
            publish(path, &bytes)?;
        }
        if is_human {
            human(&snapshot);
        } else if output.is_none() {
            println!(
                "{}",
                String::from_utf8(bytes).map_err(|_| "invalid_snapshot")?
            );
        }
        match interval {
            Some(n) => std::thread::sleep(Duration::from_secs(n)),
            None => return Ok(()),
        }
    }
}
fn main() {
    if let Err(code) = run() {
        eprintln!("{code}");
        std::process::exit(1);
    }
}

fn selected_task(args: &[String]) -> Result<(), &'static str> {
    use operator_observation::{selected_task::SelectedTask, task};
    if args.len() < 2 {
        return Err(
            "selected-task requires absolute ENDPOINT and TASK_ID; optional --profile PROFILE --replay FILE --output FILE --interval SECONDS --human [--inspect owner|questions|result|lifecycle|native_delivery_status|command_result|control_result|activity]",
        );
    }
    let endpoint = Path::new(&args[0]);
    let mut request = task::Request {
        task_id: args[1].clone(),
        delivery_id: None,
        command_id: None,
    };
    let (mut profile_path, mut replay, mut output, mut interval, mut human) =
        (None, None, None, None, false);
    let mut inspect: Option<String> = None;
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--human" {
            human = true;
            i += 1;
            continue;
        }
        let value = args.get(i + 1).ok_or("invalid_arguments")?.clone();
        match args[i].as_str() {
            "--inspect" => inspect = Some(value),
            "--profile" => profile_path = Some(value),
            "--replay" => replay = Some(value),
            "--output" => output = Some(value),
            "--delivery" => request.delivery_id = Some(value),
            "--command" => request.command_id = Some(value),
            "--interval" => interval = Some(value.parse::<u64>().map_err(|_| "invalid_interval")?),
            _ => return Err("invalid_arguments"),
        }
        i += 2;
    }
    if inspect
        .as_deref()
        .is_some_and(|v| !operator_observation::selected_task::INSPECTORS.contains(&v))
        || (inspect.is_some() && !human)
    {
        return Err("invalid_inspector");
    }
    request.validate().map_err(|_| "invalid_selection")?;
    if !endpoint.is_absolute()
        || interval.is_some_and(|n| !(2..=3600).contains(&n))
        || (interval.is_some() && (output.is_none() || replay.is_some()))
    {
        return Err("invalid_selection");
    }
    let mut selection = task::Selection::default();
    let mut prior_source = None;
    loop {
        // A changed protected profile is a new source/account selection. Never carry
        // anchors from the previous source into a read under that identity.
        let profile_bytes = profile_path
            .as_ref()
            .map(|p| session_query::read_file(p, 16384, true))
            .transpose()?;
        let profile: Option<Profile> = profile_bytes
            .as_ref()
            .map(|b| serde_json::from_slice(b).map_err(|_| "invalid_profile"))
            .transpose()?;
        if let Some(p) = &profile {
            p.validate()?;
            if p.bridge.is_some() {
                return Err("observation_profile_must_have_no_bridge");
            }
        }
        let source_id = digest(profile_bytes.as_deref().unwrap_or(b"no_events_profile"));
        if prior_source
            .as_ref()
            .is_some_and(|source| source != &source_id)
        {
            selection = task::Selection::default();
        }
        prior_source = Some(source_id.clone());
        let generation = selection.begin(endpoint.to_path_buf(), request.clone());
        let result = match &replay {
            Some(path) => task::decode(
                &session_query::read_file(path, task::MAX_FRAME, true)?,
                &request,
            ),
            None => task::read(endpoint, &request, Duration::from_secs(5)),
        };
        if replay.is_some() {
            selection.apply_at(generation, result, 0);
        } else {
            selection.apply(generation, result);
        }
        let ids = operator_observation::selected_task::linked_sessions(&selection);
        let activity = if ids.is_empty() || ids.len() > 20 {
            None
        } else if let Some(p) = &profile {
            Some(collect(
                &source_id,
                "Owner-linked session activity",
                &ids,
                |a| session_query::query(p, &Query::parse("read_session", a)?),
            )?)
        } else {
            None
        };
        let snapshot = SelectedTask::project_requested(
            &selection,
            &source_id,
            activity.as_ref(),
            replay.is_some(),
            profile.is_some(),
        )?;
        let bytes = serde_json::to_vec(&snapshot).map_err(|_| "invalid_projection")?;
        SelectedTask::parse(&bytes)?;
        if let Some(path) = &output {
            publish(path, &bytes)?;
        }
        if human {
            println!(
                "Selected endpoint {} · task {} · {} · private operator data",
                terminal_text(&snapshot.endpoint),
                snapshot.request.task_id,
                snapshot.standing
            );
            println!(
                "Independent components; no atomic cut. Reconciliation {:?}; task read budget 5s. U3 delivery/latency unqualified.",
                interval
            );
            println!(
                "{}",
                snapshot.human_view(inspect.as_deref(), operator_observation::now_ms(), false)
            );
        } else if output.is_none() {
            println!(
                "{}",
                String::from_utf8(bytes).map_err(|_| "invalid_projection")?
            );
        }
        match interval {
            Some(n) => std::thread::sleep(Duration::from_secs(n)),
            None => return Ok(()),
        }
    }
}
