use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::env;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::{MAX_SCAN_ENTRIES, read_line_bounded, valid_session_id};

const MAX_RESULTS: usize = 1_000;
const MAX_DIAGNOSTICS: usize = 16;

/// A child's own latest telemetry, read from its rollout by the same path the
/// single-session report uses. A parent deciding whether to dispatch to a
/// retained child needs the pressure, not just the identity.
#[derive(Serialize, Clone)]
struct ChildUsage {
    /// `observed`, or a distinguishable class of why not. Never a zero standing
    /// in for an unread counter.
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recorded_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_request: Option<crate::Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context_window_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context_fraction_proxy: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lineage_markers: Option<usize>,
}

#[derive(Serialize, Clone)]
struct Agent {
    thread_id: String,
    root_session_id: Option<String>,
    parent_thread_id: Option<String>,
    agent_path: Option<String>,
    rollout: String,
    /// Absent unless `--with-usage` was requested, so the default report keeps
    /// its exact shape.
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<ChildUsage>,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    parent_thread_id: String,
    recursive: bool,
    completeness: &'static str,
    scanned_rollouts: usize,
    skipped_metadata: usize,
    diagnostics: Vec<String>,
    diagnostics_omitted: usize,
    duplicate_thread_ids: Vec<String>,
    conflicting_id_candidates: BTreeMap<String, Vec<Agent>>,
    cycle_edges: Vec<String>,
    ambiguous_agent_paths: BTreeMap<String, Vec<String>>,
    agents: Vec<Agent>,
}

struct Args {
    parent: String,
    recursive: bool,
    with_usage: bool,
    json: bool,
    sessions_dir: PathBuf,
}

fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Args, String> {
    let mut parent = None;
    let mut recursive = false;
    let mut with_usage = false;
    let mut json = false;
    let mut sessions_dir = None;
    let mut it = args.into_iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--recursive" => recursive = true,
            "--with-usage" => with_usage = true,
            "--json" => json = true,
            "--parent-thread" => parent = Some(next_value(&mut it, "--parent-thread")?),
            "--sessions-dir" => {
                sessions_dir = Some(PathBuf::from(next_value(&mut it, "--sessions-dir")?))
            }
            _ => return Err(format!("unknown agents option {flag}")),
        }
    }
    let parent = parent.ok_or("agents requires --parent-thread ID")?;
    if !valid_session_id(&parent) {
        return Err("--parent-thread must be a hexadecimal/hyphen ID".into());
    }
    let sessions_dir = sessions_dir.unwrap_or_else(|| {
        env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|v| PathBuf::from(v).join(".codex")))
            .unwrap_or_default()
            .join("sessions")
    });
    Ok(Args {
        parent,
        recursive,
        with_usage,
        json,
        sessions_dir,
    })
}
fn next_value(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    let value = it
        .next()
        .ok_or_else(|| format!("missing value for {flag}"))?;
    if value.is_empty() || value.starts_with("--") {
        return Err(format!("missing value for {flag}"));
    }
    Ok(value)
}

fn help() -> &'static str {
    "Usage: work-engine-context-status agents --parent-thread ID [--recursive] [--with-usage] [--sessions-dir DIR] [--json]"
}

fn files(root: PathBuf) -> Result<Vec<PathBuf>, String> {
    let mut stack = vec![(root, 0_usize)];
    let mut files = Vec::new();
    let mut entries_seen = 0;
    while let Some((dir, depth)) = stack.pop() {
        if depth > 5 {
            return Err("sessions directory depth limit reached".into());
        }
        let entries = fs::read_dir(&dir)
            .map_err(|e| format!("cannot read sessions directory {}: {e}", dir.display()))?;
        for entry in entries {
            entries_seen += 1;
            if entries_seen > MAX_SCAN_ENTRIES {
                return Err("sessions directory entry limit reached".into());
            }
            let entry = entry.map_err(|e| format!("directory entry error: {e}"))?;
            let ty = entry
                .file_type()
                .map_err(|e| format!("file type error: {e}"))?;
            if ty.is_symlink() {
                continue;
            }
            if ty.is_dir() {
                stack.push((entry.path(), depth + 1));
            } else if ty.is_file() && entry.file_name().to_string_lossy().ends_with(".jsonl") {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    Ok(files)
}

fn optional_id(v: &Value, key: &str) -> Result<Option<String>, &'static str> {
    match v.get(key) {
        None => Ok(None),
        Some(Value::String(s)) if valid_session_id(s) => Ok(Some(s.clone())),
        _ => Err("invalid identity field"),
    }
}
fn optional_path(v: &Value, key: &str) -> Result<Option<String>, &'static str> {
    match v.get(key) {
        None => Ok(None),
        Some(Value::String(s)) if !s.is_empty() && s.len() <= 256 => Ok(Some(s.clone())),
        _ => Err("invalid agent path field"),
    }
}
fn reconcile<T: PartialEq>(left: Option<T>, right: Option<T>) -> Result<Option<T>, &'static str> {
    match (left, right) {
        (Some(a), Some(b)) if a != b => Err("conflicting duplicated metadata fields"),
        (Some(a), _) => Ok(Some(a)),
        (_, Some(b)) => Ok(Some(b)),
        _ => Ok(None),
    }
}
fn read_agent(path: &Path) -> Result<Option<Agent>, &'static str> {
    let file = File::open(path).map_err(|_| "cannot open rollout")?;
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    read_line_bounded(&mut reader, &mut line)
        .map_err(|_| "metadata line unreadable or exceeds 8 MiB")?;
    if line.is_empty() {
        return Err("empty rollout");
    }
    let record: Value =
        serde_json::from_slice(&line).map_err(|_| "malformed first metadata line")?;
    if record.get("type").and_then(Value::as_str) != Some("session_meta") {
        return Err("first record is not session_meta");
    }
    let payload = record
        .get("payload")
        .ok_or("missing session_meta payload")?;
    let thread_id = optional_id(payload, "id")?.ok_or("missing thread id")?;
    let root_session_id = optional_id(payload, "session_id")?;
    let spawn = payload
        .pointer("/source/subagent/thread_spawn")
        .unwrap_or(&Value::Null);
    let parent_thread_id = reconcile(
        optional_id(payload, "parent_thread_id")?,
        optional_id(spawn, "parent_thread_id")?,
    )?;
    let agent_path = reconcile(
        optional_path(payload, "agent_path")?,
        optional_path(spawn, "agent_path")?,
    )?;
    Ok(Some(Agent {
        thread_id,
        root_session_id,
        parent_thread_id,
        agent_path,
        rollout: path.display().to_string(),
        usage: None,
    }))
}

/// Read one child's latest observation. A failure here is reported against that
/// child and never fails the enumeration: a parent asking about five children
/// should not lose four answers because one rollout is unreadable.
fn child_usage(agent: &Agent) -> ChildUsage {
    let child_args = crate::args_for_child(
        agent.thread_id.clone(),
        PathBuf::from(agent.rollout.clone()),
    );
    match crate::read_report(&child_args, Path::new(&agent.rollout)) {
        Err(reason) => ChildUsage {
            status: "unreadable_rollout",
            reason: Some(reason),
            observed_at: None,
            recorded_model: None,
            latest_request: None,
            context_window_tokens: None,
            context_fraction_proxy: None,
            lineage_markers: None,
        },
        Ok(report) => {
            let status = if report.latest_request.is_some() {
                "observed"
            } else {
                "absent_counters"
            };
            let reason = (status == "absent_counters").then(|| {
                String::from("rollout carries no readable last_token_usage for this session")
            });
            ChildUsage {
                status,
                reason,
                observed_at: report.observed_at,
                recorded_model: report.recorded_model,
                latest_request: report.latest_request,
                context_window_tokens: report.context_window_tokens,
                context_fraction_proxy: report.context_fraction_proxy,
                lineage_markers: Some(report.lineage.len()),
            }
        }
    }
}

fn build_report(args: &Args) -> Result<Report, String> {
    let paths = files(args.sessions_dir.clone())?;
    let scanned_rollouts = paths.len();
    let mut discovered = Vec::new();
    let mut diagnostics = Vec::new();
    let mut skipped_metadata = 0;
    for path in paths {
        match read_agent(&path) {
            Ok(Some(agent)) => discovered.push(agent),
            Ok(None) => {}
            Err(reason) => {
                skipped_metadata += 1;
                if diagnostics.len() < MAX_DIAGNOSTICS {
                    diagnostics.push(format!("{}: {reason}", path.display()));
                }
            }
        }
    }
    let mut by_id = BTreeMap::<String, Vec<Agent>>::new();
    let mut by_path = BTreeMap::<String, Vec<String>>::new();
    for agent in &discovered {
        by_id
            .entry(agent.thread_id.clone())
            .or_default()
            .push(agent.clone());
        if let Some(path) = &agent.agent_path {
            by_path
                .entry(path.clone())
                .or_default()
                .push(agent.thread_id.clone());
        }
    }
    let mut frontier = VecDeque::from([(args.parent.clone(), vec![args.parent.clone()])]);
    let mut expanded = HashSet::new();
    let mut matched = Vec::new();
    let mut cycle_edges = Vec::new();
    while let Some((parent, ancestors)) = frontier.pop_front() {
        if !expanded.insert(parent.clone()) {
            continue;
        }
        if by_id.get(&parent).is_some_and(|rows| rows.len() > 1) {
            continue;
        }
        for agent in discovered
            .iter()
            .filter(|x| x.parent_thread_id.as_deref() == Some(parent.as_str()))
        {
            if ancestors.contains(&agent.thread_id) {
                if cycle_edges.len() >= MAX_RESULTS {
                    return Err(
                        "cycle result limit reached (1000); narrow the sessions directory".into(),
                    );
                }
                cycle_edges.push(format!("{} -> {}", parent, agent.thread_id));
                continue;
            }
            if matched.len() >= MAX_RESULTS {
                return Err(
                    "agent result limit reached (1000); narrow the sessions directory".into(),
                );
            }
            matched.push(agent.clone());
            if args.recursive
                && by_id
                    .get(&agent.thread_id)
                    .is_some_and(|rows| rows.len() == 1)
            {
                let mut child_ancestors = ancestors.clone();
                child_ancestors.push(agent.thread_id.clone());
                frontier.push_back((agent.thread_id.clone(), child_ancestors));
            }
        }
        if !args.recursive {
            break;
        }
    }
    matched.sort_by(|a, b| {
        (&a.agent_path, &a.thread_id, &a.rollout).cmp(&(&b.agent_path, &b.thread_id, &b.rollout))
    });
    if args.with_usage {
        for agent in &mut matched {
            agent.usage = Some(child_usage(agent));
        }
    }
    let mut relevant_ids = matched
        .iter()
        .map(|a| a.thread_id.clone())
        .collect::<HashSet<_>>();
    relevant_ids.insert(args.parent.clone());
    let conflicting_id_candidates = by_id
        .into_iter()
        .filter(|(id, rows)| relevant_ids.contains(id) && rows.len() > 1)
        .collect::<BTreeMap<_, _>>();
    let duplicate_thread_ids = conflicting_id_candidates
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let candidate_count: usize = conflicting_id_candidates.values().map(Vec::len).sum();
    if matched.len().saturating_add(candidate_count) > MAX_RESULTS {
        return Err("agent and conflict evidence result limit reached (1000); narrow the sessions directory".into());
    }
    let selected_paths = matched
        .iter()
        .filter_map(|a| a.agent_path.as_ref())
        .collect::<HashSet<_>>();
    let ambiguous_agent_paths = by_path
        .into_iter()
        .filter_map(|(path, mut ids)| {
            if !selected_paths.contains(&path) {
                return None;
            }
            ids.sort();
            ids.dedup();
            (ids.len() > 1).then_some((path, ids))
        })
        .collect::<BTreeMap<_, _>>();
    let completeness = if skipped_metadata == 0
        && duplicate_thread_ids.is_empty()
        && cycle_edges.is_empty()
        && ambiguous_agent_paths.is_empty()
    {
        "complete_within_scanned_directory"
    } else {
        "partial_or_ambiguous"
    };
    Ok(Report {
        schema_version: 1,
        parent_thread_id: args.parent.clone(),
        recursive: args.recursive,
        completeness,
        scanned_rollouts,
        skipped_metadata,
        diagnostics_omitted: skipped_metadata.saturating_sub(diagnostics.len()),
        diagnostics,
        duplicate_thread_ids,
        conflicting_id_candidates,
        cycle_edges,
        ambiguous_agent_paths,
        agents: matched,
    })
}

fn render_usage(usage: &ChildUsage) -> String {
    let Some(latest) = &usage.latest_request else {
        return match &usage.reason {
            Some(reason) => format!("{} ({reason})", usage.status),
            None => usage.status.to_string(),
        };
    };
    let window = match usage.context_window_tokens {
        Some(window) => window.to_string(),
        None => "unknown-window".to_string(),
    };
    let fraction = match usage.context_fraction_proxy {
        Some(fraction) => format!("{:.1}%", fraction * 100.0),
        None => "unknown-fraction".to_string(),
    };
    format!(
        "{} input={}/{} ({fraction}) epoch={} lineage={} at={}",
        usage.status,
        latest.input_tokens,
        window,
        latest.epoch,
        usage
            .lineage_markers
            .map_or_else(|| "unknown".to_string(), |n| n.to_string()),
        usage.observed_at.as_deref().unwrap_or("unknown-time"),
    )
}

pub(crate) fn run<I: IntoIterator<Item = String>>(args: I) -> Result<(), String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if args.iter().any(|x| x == "--help" || x == "-h") {
        println!("{}", help());
        return Ok(());
    }
    let args = parse_args(args)?;
    let report = build_report(&args)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|_| "cannot serialize agent report")?
        );
    } else {
        println!(
            "Parent {}: {} child candidate(s), {} ({} rollout(s) scanned)",
            report.parent_thread_id,
            report.agents.len(),
            report.completeness,
            report.scanned_rollouts
        );
        for agent in &report.agents {
            println!(
                "{}  {}  parent={}  root={}  {}",
                agent.thread_id,
                agent.agent_path.as_deref().unwrap_or("unknown-path"),
                agent
                    .parent_thread_id
                    .as_deref()
                    .unwrap_or("unknown-parent"),
                agent.root_session_id.as_deref().unwrap_or("unknown-root"),
                agent.rollout
            );
            if let Some(usage) = &agent.usage {
                println!("  usage: {}", render_usage(usage));
            }
        }
        for id in &report.duplicate_thread_ids {
            println!("Ambiguous thread id: {id}");
            if let Some(candidates) = report.conflicting_id_candidates.get(id) {
                for candidate in candidates {
                    println!(
                        "  candidate parent={} root={} path={} rollout={}",
                        candidate
                            .parent_thread_id
                            .as_deref()
                            .unwrap_or("unknown-parent"),
                        candidate
                            .root_session_id
                            .as_deref()
                            .unwrap_or("unknown-root"),
                        candidate.agent_path.as_deref().unwrap_or("unknown-path"),
                        candidate.rollout
                    );
                }
            }
        }
        for edge in &report.cycle_edges {
            println!("Cyclic parent edge: {edge}");
        }
        for (path, ids) in &report.ambiguous_agent_paths {
            println!(
                "Multiple thread ids for agent path {path}: {}",
                ids.join(", ")
            );
        }
        for item in &report.diagnostics {
            println!("Skipped: {item}");
        }
        if report.diagnostics_omitted > 0 {
            println!(
                "Additional skipped metadata: {}",
                report.diagnostics_omitted
            );
        }
    }
    Ok(())
}
