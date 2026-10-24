//! Claude Code transcript view: the Claude counterpart of the Codex rollout
//! report. It reads `~/.claude/projects/<project>/<session>.jsonl` or a
//! subagent's `<session>/subagents/agent-<id>.jsonl` and reports recorded
//! per-call usage. Transcripts record no context window, so no fraction or
//! headroom is ever computed. Message content, tool inputs and prompt bodies
//! are never read into the report.

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    MAX_DIAGNOSTICS, MAX_FILE_BYTES, MAX_RECENT, MAX_SCAN_ENTRIES, STALE_SECONDS, bounded_string,
    note, read_line_bounded_as, valid_session_id,
};

const MAX_LINEAGE: usize = 32;
const MAX_MODELS: usize = 32;
const MAX_AGENTS: usize = 1_000;
const MAX_META_BYTES: u64 = 1024 * 1024;
const SYNTHETIC_MODEL: &str = "<synthetic>";
const WEIGHTING: &str = "uncached (input + cache_creation) + cache_read/10 + 5 x output; the 2026-10 baseline unit, not a price";

fn report_help() -> &'static str {
    "Usage: work-engine-context-status claude (--transcript FILE | --session ID [--agent ID] [--projects-dir DIR]) [--json]"
}
fn agents_help() -> &'static str {
    "Usage: work-engine-context-status claude-agents --session ID [--projects-dir DIR] [--with-usage] [--json]"
}

/// Agent IDs are short alphanumeric tokens; this also keeps them path-safe.
fn valid_agent_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
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

fn default_projects_dir() -> PathBuf {
    env::var_os("HOME")
        .map(|v| PathBuf::from(v).join(".claude").join("projects"))
        .unwrap_or_default()
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct Call {
    input_tokens: u64,
    cache_creation_input_tokens: u64,
    cache_read_input_tokens: u64,
    /// input + cache_creation + cache_read: what this call carried.
    context_tokens: u64,
    output_tokens: Option<u64>,
    timestamp: Option<String>,
    line: usize,
    epoch: u64,
    /// Used to apply a later, larger output to the same call; not reported.
    #[serde(skip)]
    message_id: Option<String>,
}

#[derive(Serialize, Clone)]
struct Growth {
    first_context_tokens: u64,
    last_context_tokens: u64,
    delta_tokens: i64,
    sample_count: usize,
    provenance: &'static str,
}

#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum LineageEvent {
    /// A `fork-context-ref` first record: this transcript starts from a copy
    /// of its parent's context. `context_length` is recorded as is; its unit
    /// is not established.
    Fork {
        parent_session_id: Option<String>,
        context_length: Option<u64>,
        line: usize,
    },
    Compaction {
        trigger: Option<String>,
        pre_tokens: Option<u64>,
        post_tokens: Option<u64>,
        line: usize,
        timestamp: Option<String>,
    },
    ModelChange {
        from: String,
        to: String,
        line: usize,
        timestamp: Option<String>,
    },
}

#[derive(Serialize, Clone, Default)]
pub(crate) struct Cumulative {
    distinct_calls: u64,
    calls_without_usage: u64,
    uncached_tokens: u64,
    cached_tokens: u64,
    output_tokens: u64,
    calls_missing_output: u64,
    weighted_tokens: f64,
    weighting: &'static str,
}

#[derive(Serialize)]
pub(crate) struct Report {
    schema_version: u32,
    host: &'static str,
    session_id: Option<String>,
    agent_id: Option<String>,
    transcript: String,
    file_modified_unix_seconds: Option<u64>,
    file_age_seconds: Option<u64>,
    freshness: &'static str,
    observed_models: Vec<String>,
    latest_model: Option<String>,
    recorded_effort: Option<String>,
    latest_call: Option<Call>,
    latest_call_status: &'static str,
    #[serde(skip)]
    pub(crate) last_record_gap_line: Option<usize>,
    context_window: &'static str,
    epoch: u64,
    recent_calls: Vec<Call>,
    recent_growth: Option<Growth>,
    lineage: Vec<LineageEvent>,
    cumulative: Cumulative,
    /// Present for a fork: the parent's copied call(s), excluded from
    /// `cumulative`, the latest call, models and effort.
    #[serde(skip_serializing_if = "Option::is_none")]
    inherited: Option<Cumulative>,
    limitations: Vec<String>,
    diagnostic_counts: BTreeMap<&'static str, u64>,
    diagnostics_omitted: u64,
}

/// Identity a caller expects the transcript to carry. A resolved session or
/// agent is checked against every record that names one.
#[derive(Default, Clone)]
pub(crate) struct Expected {
    session: Option<String>,
    agent: Option<String>,
}

pub(crate) fn control_report(
    path: &Path,
    session: String,
    agent: Option<String>,
) -> Result<Report, String> {
    let report = read_report(
        path,
        &Expected {
            session: Some(session.clone()),
            agent: agent.clone(),
        },
    )?;
    if report.session_id.as_deref() != Some(&session) || report.agent_id != agent {
        return Err("control transcript identity is absent or does not match the binding".into());
    }
    Ok(report)
}

fn freshness_of(path_meta: &fs::Metadata) -> (Option<u64>, Option<u64>, &'static str) {
    let modified = path_meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs()));
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs());
    let age = modified.zip(now).map(|(m, n)| n.saturating_sub(m));
    let freshness = match age {
        Some(n) if n <= STALE_SECONDS => "recent_file_modification",
        Some(_) => "stale_file_modification",
        None => "unknown",
    };
    (modified, age, freshness)
}

/// Check one record's identity field against the established or expected
/// value. A conflict fails the report: two identities in one file means its
/// counters cannot be attributed.
fn bind_identity(
    record: &Value,
    key: &str,
    slot: &mut Option<String>,
    line: usize,
) -> Result<(), String> {
    let Some(v) = record.get(key) else {
        return Ok(());
    };
    let Some(s) = v.as_str().filter(|s| valid_agent_id(s)) else {
        return Err(format!("malformed {key} at line {line}"));
    };
    match slot {
        Some(known) if known != s => Err(format!("{key} mismatch at line {line}")),
        Some(_) => Ok(()),
        None => {
            *slot = Some(s.to_owned());
            Ok(())
        }
    }
}

fn u64_field(v: &Value, key: &str) -> Option<u64> {
    v.get(key)?.as_u64()
}

/// First-seen counters of one `message.id`, and the largest output seen so far.
struct Seen {
    counters: Option<(u64, u64, u64)>,
    output: Option<u64>,
    inherited: bool,
}

/// Running totals for one accounting scope (own calls or inherited calls).
struct Account {
    totals: Cumulative,
    weighted: f64,
}
impl Account {
    fn new() -> Self {
        Self {
            totals: Cumulative {
                weighting: WEIGHTING,
                ..Cumulative::default()
            },
            weighted: 0.0,
        }
    }
    fn add(&mut self, uncached: u64, cached: u64, output: Option<u64>) {
        let t = &mut self.totals;
        t.distinct_calls = t.distinct_calls.saturating_add(1);
        t.uncached_tokens = t.uncached_tokens.saturating_add(uncached);
        t.cached_tokens = t.cached_tokens.saturating_add(cached);
        self.weighted += uncached as f64 + cached as f64 / 10.0;
        match output {
            Some(o) => {
                t.output_tokens = t.output_tokens.saturating_add(o);
                self.weighted += 5.0 * o as f64;
            }
            None => t.calls_missing_output = t.calls_missing_output.saturating_add(1),
        }
    }
    /// A later record of the same call carries a larger output: replace the
    /// earlier (streaming partial) value.
    fn raise_output(&mut self, old: Option<u64>, new: u64) {
        let t = &mut self.totals;
        let before = old.unwrap_or(0);
        if old.is_none() {
            t.calls_missing_output = t.calls_missing_output.saturating_sub(1);
        }
        t.output_tokens = t.output_tokens.saturating_sub(before).saturating_add(new);
        self.weighted += 5.0 * (new as f64 - before as f64);
    }
    fn finish(mut self) -> Cumulative {
        self.totals.weighted_tokens = self.weighted;
        self.totals
    }
}

pub(crate) fn read_report(path: &Path, expected: &Expected) -> Result<Report, String> {
    let metadata = fs::metadata(path).map_err(|e| format!("cannot stat transcript: {e}"))?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err("transcript is not a regular file or exceeds 256 MiB".into());
    }
    let (modified, age, freshness) = freshness_of(&metadata);
    let mut reader =
        BufReader::new(File::open(path).map_err(|e| format!("cannot open transcript: {e}"))?);
    let mut session = expected.session.clone();
    let mut agent = expected.agent.clone();
    let mut line = Vec::new();
    let mut line_no = 0_usize;
    let mut limitations = Vec::new();
    let mut counts = BTreeMap::new();
    let mut gap = None;
    let mut seen_ids: HashMap<String, Seen> = HashMap::new();
    let mut fork = false;
    let mut in_inherited_prefix = false;
    let mut own = Account::new();
    let mut inherited = Account::new();
    let mut models = BTreeSet::new();
    let mut models_truncated = false;
    let mut latest_model: Option<String> = None;
    let mut effort: Option<String> = None;
    let mut latest: Option<Call> = None;
    let mut recent: VecDeque<Call> = VecDeque::new();
    let mut epoch = 0_u64;
    let mut lineage = Vec::new();
    let mut lineage_truncated = false;
    // First, so the diagnostic cap can never drop it.
    note(
        &mut limitations,
        &mut counts,
        "no_context_window",
        "no context window recorded; no fraction or headroom is reported".into(),
    );
    let mut push_lineage = |lineage: &mut Vec<LineageEvent>, e: LineageEvent| {
        if lineage.len() < MAX_LINEAGE {
            lineage.push(e);
        } else {
            lineage_truncated = true;
        }
    };
    loop {
        let read = read_line_bounded_as(&mut reader, &mut line, "transcript")?;
        if read == 0 {
            break;
        }
        line_no += 1;
        let complete = line.last() == Some(&b'\n');
        let record: Value = match serde_json::from_slice(&line) {
            Ok(v) => v,
            Err(_) if !complete => {
                gap = Some(line_no);
                note(
                    &mut limitations,
                    &mut counts,
                    "partial_trailing_record",
                    format!(
                        "partial trailing line {line_no} ignored; latest call is only the latest valid observation"
                    ),
                );
                break;
            }
            Err(_) => {
                gap = Some(line_no);
                note(
                    &mut limitations,
                    &mut counts,
                    "malformed_record",
                    format!(
                        "malformed complete record at line {line_no} ignored; latest call is only the latest valid observation"
                    ),
                );
                continue;
            }
        };
        if !record.is_object() {
            continue;
        }
        bind_identity(&record, "sessionId", &mut session, line_no)?;
        bind_identity(&record, "agentId", &mut agent, line_no)?;
        let timestamp = record
            .get("timestamp")
            .and_then(Value::as_str)
            .map(bounded_string);
        let ty = record.get("type").and_then(Value::as_str);
        if ty == Some("fork-context-ref") {
            if line_no == 1 {
                fork = true;
                in_inherited_prefix = true;
                push_lineage(
                    &mut lineage,
                    LineageEvent::Fork {
                        parent_session_id: record
                            .get("parentSessionId")
                            .and_then(Value::as_str)
                            .filter(|s| valid_session_id(s))
                            .map(str::to_owned),
                        context_length: u64_field(&record, "contextLength"),
                        line: line_no,
                    },
                );
            } else {
                note(
                    &mut limitations,
                    &mut counts,
                    "fork_reference_not_first",
                    format!(
                        "fork-context-ref at line {line_no} is not the first record; no history excluded"
                    ),
                );
            }
            continue;
        }
        // In the observed fork format the parent's spawning call is copied
        // directly after the reference, as assistant record(s) with a null
        // parentUuid. The prefix ends at the fork's first own record.
        let is_inherited = in_inherited_prefix
            && ty == Some("assistant")
            && record.get("parentUuid").is_some_and(Value::is_null);
        if in_inherited_prefix && !is_inherited {
            in_inherited_prefix = false;
        }
        if record.get("compactMetadata").is_some_and(Value::is_object)
            || record.get("subtype").and_then(Value::as_str) == Some("compact_boundary")
        {
            let cm = record.get("compactMetadata").unwrap_or(&Value::Null);
            epoch = epoch.saturating_add(1);
            recent.clear();
            latest = None;
            push_lineage(
                &mut lineage,
                LineageEvent::Compaction {
                    trigger: cm
                        .get("trigger")
                        .and_then(Value::as_str)
                        .map(bounded_string),
                    pre_tokens: u64_field(cm, "preTokens"),
                    post_tokens: u64_field(cm, "postTokens"),
                    line: line_no,
                    timestamp,
                },
            );
            continue;
        }
        if ty != Some("assistant") {
            continue;
        }
        let message = record.get("message").unwrap_or(&Value::Null);
        let model = message
            .get("model")
            .and_then(Value::as_str)
            .map(bounded_string);
        if model.as_deref() == Some(SYNTHETIC_MODEL) {
            if !is_inherited {
                gap = Some(line_no);
            }
            note(
                &mut limitations,
                &mut counts,
                "synthetic_record_skipped",
                format!(
                    "locally generated assistant record at line {line_no} is not an API call; skipped"
                ),
            );
            continue;
        }
        let message_id = message
            .get("id")
            .and_then(Value::as_str)
            .map(bounded_string);
        let usage = message.get("usage").filter(|u| u.is_object());
        let counters = usage
            .and_then(|u| {
                Some((
                    u64_field(u, "input_tokens")?,
                    u64_field(u, "cache_creation_input_tokens")?,
                    u64_field(u, "cache_read_input_tokens")?,
                    u64_field(u, "output_tokens"),
                ))
            })
            .filter(|(i, c, r, _)| {
                i.checked_add(*c)
                    .and_then(|n| n.checked_add(*r))
                    .is_some_and(|n| n > 0)
            });
        // One API response is written as several records sharing message.id.
        // It counts once: input and cache counters from its first record,
        // output as the largest recorded value, since an early record can
        // hold a streaming partial.
        if let Some(id) = &message_id {
            if let Some(seen) = seen_ids.get_mut(id) {
                match (seen.counters, counters) {
                    (Some(first), Some((i, c, r, o))) if first == (i, c, r) => {
                        if let Some(new) = o.filter(|n| Some(*n) > seen.output) {
                            if seen.inherited {
                                inherited.raise_output(seen.output, new);
                            } else {
                                own.raise_output(seen.output, new);
                                for call in latest.iter_mut().chain(recent.iter_mut()) {
                                    if call.message_id.as_deref() == Some(id.as_str()) {
                                        call.output_tokens = Some(new);
                                    }
                                }
                            }
                            seen.output = Some(new);
                        }
                    }
                    (_, Some(_)) => note(
                        &mut limitations,
                        &mut counts,
                        "duplicate_message_usage_differs",
                        format!(
                            "line {line_no} repeats an earlier message.id with different input/cache usage; first record kept"
                        ),
                    ),
                    _ => {}
                }
                continue;
            }
        } else {
            note(
                &mut limitations,
                &mut counts,
                "missing_message_id",
                format!(
                    "assistant record at line {line_no} lacks message.id; counted once as its own call"
                ),
            );
        }
        if is_inherited {
            match counters {
                Some((i, c, r, o)) => {
                    inherited.add(i.saturating_add(c), r, o);
                    if let Some(id) = message_id {
                        seen_ids.insert(
                            id,
                            Seen {
                                counters: Some((i, c, r)),
                                output: o,
                                inherited: true,
                            },
                        );
                    }
                }
                None => {
                    inherited.totals.calls_without_usage =
                        inherited.totals.calls_without_usage.saturating_add(1)
                }
            }
            continue;
        }
        if let Some(m) = &model {
            if let Some(prev) = &latest_model
                && prev != m
            {
                epoch = epoch.saturating_add(1);
                recent.clear();
                latest = None;
                effort = None;
                push_lineage(
                    &mut lineage,
                    LineageEvent::ModelChange {
                        from: prev.clone(),
                        to: m.clone(),
                        line: line_no,
                        timestamp: timestamp.clone(),
                    },
                );
            }
            if models.len() < MAX_MODELS {
                models.insert(m.clone());
            } else if !models.contains(m) {
                models_truncated = true;
            }
            latest_model = Some(m.clone());
        }
        if let Some(e) = record.get("effort").and_then(Value::as_str) {
            effort = Some(bounded_string(e));
        }
        let Some((input, creation, read_tokens, output)) = counters else {
            gap = Some(line_no);
            own.totals.calls_without_usage = own.totals.calls_without_usage.saturating_add(1);
            if let Some(id) = message_id {
                seen_ids.insert(
                    id,
                    Seen {
                        counters: None,
                        output: None,
                        inherited: false,
                    },
                );
            }
            note(
                &mut limitations,
                &mut counts,
                "missing_usage",
                format!(
                    "assistant call at line {line_no} lacks safe input/cache_creation/cache_read counters; unavailable, not zero"
                ),
            );
            continue;
        };
        if let Some(id) = &message_id {
            seen_ids.insert(
                id.clone(),
                Seen {
                    counters: Some((input, creation, read_tokens)),
                    output,
                    inherited: false,
                },
            );
        }
        let Some(context) = input
            .checked_add(creation)
            .and_then(|x| x.checked_add(read_tokens))
        else {
            gap = Some(line_no);
            note(
                &mut limitations,
                &mut counts,
                "counter_overflow",
                format!("usage counters at line {line_no} overflow; call ignored"),
            );
            continue;
        };
        own.add(input.saturating_add(creation), read_tokens, output);
        let call = Call {
            input_tokens: input,
            cache_creation_input_tokens: creation,
            cache_read_input_tokens: read_tokens,
            context_tokens: context,
            output_tokens: output,
            timestamp,
            line: line_no,
            epoch,
            message_id,
        };
        latest = Some(call.clone());
        if recent.len() == MAX_RECENT {
            recent.pop_front();
        }
        recent.push_back(call);
    }
    let cumulative = own.finish();
    let inherited = fork.then(|| inherited.finish());
    if let Some(inh) = &inherited {
        note(
            &mut limitations,
            &mut counts,
            "inherited_history_excluded",
            format!(
                "fork transcript: {} copied parent call(s) excluded from cumulative, latest call, models and effort",
                inh.distinct_calls + inh.calls_without_usage
            ),
        );
    }
    if latest.is_none() {
        note(
            &mut limitations,
            &mut counts,
            "missing_current_usage",
            "no valid per-call usage in the current epoch; current context unavailable".into(),
        );
    }
    if cumulative.calls_missing_output > 0 {
        note(
            &mut limitations,
            &mut counts,
            "output_incomplete",
            format!(
                "{} call(s) lack output_tokens; cumulative output and weighted totals exclude them",
                cumulative.calls_missing_output
            ),
        );
    }
    if effort.is_none() {
        note(
            &mut limitations,
            &mut counts,
            "effort_unknown",
            "no effort recorded for the current model; effort unknown".into(),
        );
    }
    if freshness != "recent_file_modification" {
        note(
            &mut limitations,
            &mut counts,
            "stale_file",
            "file recency does not establish active-session identity or current in-memory context"
                .into(),
        );
    }
    if lineage_truncated {
        note(
            &mut limitations,
            &mut counts,
            "lineage_truncated",
            format!("lineage events truncated at {MAX_LINEAGE}"),
        );
    }
    if models_truncated {
        note(
            &mut limitations,
            &mut counts,
            "models_truncated",
            format!("observed models truncated at {MAX_MODELS}"),
        );
    }
    let recent: Vec<_> = recent
        .into_iter()
        .filter(|u| u.line > gap.unwrap_or(0))
        .collect();
    let recent_growth = (recent.len() >= 2).then(|| {
        let first = &recent[0];
        let end = &recent[recent.len() - 1];
        Growth {
            first_context_tokens: first.context_tokens,
            last_context_tokens: end.context_tokens,
            delta_tokens: (end.context_tokens as i128 - first.context_tokens as i128)
                .clamp(i64::MIN as i128, i64::MAX as i128) as i64,
            sample_count: recent.len(),
            provenance: "distinct message.id calls in the current epoch, at most 8; endpoint difference of context_tokens, not a sum",
        }
    });
    let latest_call_status = if latest.is_none() {
        "unavailable"
    } else if latest.as_ref().is_some_and(|u| u.line <= gap.unwrap_or(0)) {
        "historical_observation"
    } else if gap.is_some() {
        "latest_valid_observation_with_record_gaps"
    } else {
        "latest_valid_observation"
    };
    let diagnostics_omitted = counts
        .values()
        .copied()
        .sum::<u64>()
        .saturating_sub(limitations.len() as u64);
    Ok(Report {
        schema_version: 1,
        host: "claude-code",
        session_id: session,
        agent_id: agent,
        transcript: path.display().to_string(),
        file_modified_unix_seconds: modified,
        file_age_seconds: age,
        freshness,
        observed_models: models.into_iter().collect(),
        latest_model,
        recorded_effort: effort,
        latest_call: latest,
        latest_call_status,
        last_record_gap_line: gap,
        context_window: "not_recorded",
        epoch,
        recent_calls: recent,
        recent_growth,
        lineage,
        cumulative,
        inherited,
        limitations,
        diagnostic_counts: counts,
        diagnostics_omitted,
    })
}

/// Regular, non-symlink file at `path`.
fn is_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_file())
}
fn is_real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_dir())
}

/// Project directories directly under the projects root, skipping symlinks.
fn project_dirs(root: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(root)
        .map_err(|e| format!("cannot read projects directory {}: {e}", root.display()))?;
    let mut dirs = Vec::new();
    for (count, entry) in entries.enumerate() {
        if count >= MAX_SCAN_ENTRIES {
            return Err("projects directory entry limit reached".into());
        }
        let entry = entry.map_err(|e| format!("directory entry error: {e}"))?;
        let ty = entry
            .file_type()
            .map_err(|e| format!("file type error: {e}"))?;
        if ty.is_dir() && !ty.is_symlink() {
            dirs.push(entry.path());
        }
    }
    dirs.sort();
    Ok(dirs)
}

fn exactly_one(mut matches: Vec<PathBuf>, what: &str) -> Result<PathBuf, String> {
    match matches.len() {
        0 => Err(format!("requested {what} not found")),
        1 => Ok(matches.remove(0)),
        n => Err(format!("ambiguous requested {what}: {n} candidates")),
    }
}

fn locate_transcript(root: &Path, session: &str, agent: Option<&str>) -> Result<PathBuf, String> {
    let mut matches = Vec::new();
    for dir in project_dirs(root)? {
        let candidate = match agent {
            None => dir.join(format!("{session}.jsonl")),
            Some(a) => {
                // Same rule as claude-agents: no symlinked component.
                let sub = dir.join(session).join("subagents");
                if !is_real_dir(&dir.join(session)) || !is_real_dir(&sub) {
                    continue;
                }
                sub.join(format!("agent-{a}.jsonl"))
            }
        };
        if is_regular_file(&candidate) {
            matches.push(candidate);
        }
    }
    exactly_one(
        matches,
        if agent.is_some() {
            "agent transcript"
        } else {
            "session transcript"
        },
    )
}

/// Identity implied by the transcript's own file name, so `--transcript`
/// still rejects a file whose records name another session or agent.
fn expected_from_name(path: &Path) -> Expected {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let Some(stem) = name.strip_suffix(".jsonl") else {
        return Expected::default();
    };
    if let Some(agent) = stem.strip_prefix("agent-") {
        let session = path
            .parent()
            .filter(|p| p.file_name().is_some_and(|n| n == "subagents"))
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|s| valid_session_id(s));
        return Expected {
            session,
            agent: valid_agent_id(agent).then(|| agent.to_owned()),
        };
    }
    Expected {
        session: valid_session_id(stem).then(|| stem.to_owned()),
        agent: None,
    }
}

fn render_call(c: &Call) -> String {
    format!(
        "context {} tokens (input {}, cache_creation {}, cache_read {}), output {}, line {}, timestamp {}",
        c.context_tokens,
        c.input_tokens,
        c.cache_creation_input_tokens,
        c.cache_read_input_tokens,
        c.output_tokens
            .map_or_else(|| "unknown".to_string(), |o| o.to_string()),
        c.line,
        c.timestamp.as_deref().unwrap_or("unknown")
    )
}

fn render_report(r: &Report) {
    println!(
        "Claude Code session {}{}",
        r.session_id.as_deref().unwrap_or("unknown"),
        r.agent_id
            .as_deref()
            .map(|a| format!(" agent {a}"))
            .unwrap_or_default()
    );
    println!("Transcript: {}", r.transcript);
    println!(
        "Freshness: {} (file age: {})",
        r.freshness,
        r.file_age_seconds
            .map(|x| format!("{x}s"))
            .unwrap_or_else(|| "unknown".into())
    );
    println!(
        "Model/effort: {}/{} (observed models: {})",
        r.latest_model.as_deref().unwrap_or("unknown"),
        r.recorded_effort.as_deref().unwrap_or("unknown"),
        if r.observed_models.is_empty() {
            "none".to_string()
        } else {
            r.observed_models.join(", ")
        }
    );
    match &r.latest_call {
        Some(c) => println!("Latest call: {} [{}]", render_call(c), r.latest_call_status),
        None => println!("Latest call: unavailable"),
    }
    println!("Context window: not recorded; no fraction or headroom");
    if let Some(g) = &r.recent_growth {
        println!(
            "Recent context change: {:+} tokens across {} distinct calls (epoch {})",
            g.delta_tokens, g.sample_count, r.epoch
        );
    }
    for e in &r.lineage {
        match e {
            LineageEvent::Compaction {
                trigger,
                pre_tokens,
                post_tokens,
                line,
                ..
            } => println!(
                "Compaction at line {line}: trigger {}, pre {}, post {}",
                trigger.as_deref().unwrap_or("unknown"),
                pre_tokens.map_or_else(|| "unknown".to_string(), |n| n.to_string()),
                post_tokens.map_or_else(|| "unknown".to_string(), |n| n.to_string()),
            ),
            LineageEvent::Fork {
                parent_session_id,
                context_length,
                line,
            } => println!(
                "Fork at line {line}: parent session {}, recorded contextLength {}",
                parent_session_id.as_deref().unwrap_or("unknown"),
                context_length.map_or_else(|| "unknown".to_string(), |n| n.to_string()),
            ),
            LineageEvent::ModelChange { from, to, line, .. } => {
                println!("Model change at line {line}: {from} -> {to}")
            }
        }
    }
    let c = &r.cumulative;
    println!(
        "Cumulative: {} distinct calls ({} without usage); uncached {}, cached {}, output {}; weighted {:.0}",
        c.distinct_calls,
        c.calls_without_usage,
        c.uncached_tokens,
        c.cached_tokens,
        c.output_tokens,
        c.weighted_tokens
    );
    if let Some(i) = &r.inherited {
        println!(
            "Inherited (fork copy, excluded above): {} call(s); weighted {:.0}",
            i.distinct_calls, i.weighted_tokens
        );
    }
    for note in &r.limitations {
        println!("Limit: {note}");
    }
    if r.diagnostics_omitted > 0 {
        println!(
            "Additional diagnostics omitted: {} (counts by category in --json)",
            r.diagnostics_omitted
        );
    }
}

pub(crate) fn run_report<I: IntoIterator<Item = String>>(args: I) -> Result<(), String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if args.iter().any(|x| x == "--help" || x == "-h") {
        println!("{}", report_help());
        return Ok(());
    }
    let mut transcript = None;
    let mut session = None;
    let mut agent = None;
    let mut projects_dir = None;
    let mut json = false;
    let mut it = args.into_iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--json" => json = true,
            "--transcript" => transcript = Some(PathBuf::from(next_value(&mut it, &flag)?)),
            "--session" => session = Some(next_value(&mut it, &flag)?),
            "--agent" => agent = Some(next_value(&mut it, &flag)?),
            "--projects-dir" => projects_dir = Some(PathBuf::from(next_value(&mut it, &flag)?)),
            _ => return Err(format!("unknown claude option {flag}")),
        }
    }
    let (path, expected) = match (transcript, session) {
        (Some(_), Some(_)) => return Err("choose --transcript or --session".into()),
        (None, None) => return Err("claude requires --transcript FILE or --session ID".into()),
        (Some(p), None) => {
            if agent.is_some() || projects_dir.is_some() {
                return Err("--agent and --projects-dir require --session".into());
            }
            let expected = expected_from_name(&p);
            (p, expected)
        }
        (None, Some(s)) => {
            if !valid_session_id(&s) {
                return Err("--session must be an explicit hexadecimal/hyphen ID".into());
            }
            if agent.as_deref().is_some_and(|a| !valid_agent_id(a)) {
                return Err("--agent must be an alphanumeric/hyphen ID".into());
            }
            let root = projects_dir.unwrap_or_else(default_projects_dir);
            let path = locate_transcript(&root, &s, agent.as_deref())?;
            (
                path,
                Expected {
                    session: Some(s),
                    agent,
                },
            )
        }
    };
    let report = read_report(&path, &expected)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|_| "cannot serialize report")?
        );
    } else {
        render_report(&report);
    }
    Ok(())
}

// ---------------------------------------------------------------- agents

#[derive(Serialize)]
struct AgentUsage {
    /// `observed`, or a distinguishable class of why not; never a zero
    /// standing in for an unread counter.
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_call: Option<Call>,
    #[serde(skip_serializing_if = "Option::is_none")]
    epoch: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compactions: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cumulative: Option<Cumulative>,
    /// For a fork: copied parent calls excluded from `cumulative`.
    #[serde(skip_serializing_if = "Option::is_none")]
    inherited_calls: Option<u64>,
}

#[derive(Serialize)]
struct AgentRow {
    agent_id: String,
    /// `read`, `missing`, or `unreadable`.
    meta_status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta_reason: Option<String>,
    agent_type: Option<String>,
    description: Option<String>,
    /// Model requested at spawn (meta.json), distinct from observed models.
    requested_model: Option<String>,
    /// Effort requested at spawn (meta.json), distinct from recorded effort.
    requested_effort: Option<String>,
    /// meta.json `isFork`: the agent started from a copy of its parent's
    /// context (retained, not fresh).
    is_fork: Option<bool>,
    spawn_depth: Option<u64>,
    tool_use_id: Option<String>,
    finished: Option<bool>,
    transcript: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<AgentUsage>,
}

#[derive(Serialize)]
struct AgentsReport {
    schema_version: u32,
    host: &'static str,
    session_id: String,
    subagents_dir: Option<String>,
    completeness: &'static str,
    agents_listed: usize,
    ignored_entries: usize,
    /// Agent-named entries that are symlinks or not regular files; any makes
    /// the listing partial.
    skipped_agent_entries: usize,
    diagnostics: Vec<String>,
    agents: Vec<AgentRow>,
}

fn read_meta(path: &Path) -> Result<Value, String> {
    let file = File::open(path).map_err(|e| format!("cannot open meta.json: {e}"))?;
    let mut buf = Vec::new();
    file.take(MAX_META_BYTES + 1)
        .read_to_end(&mut buf)
        .map_err(|e| format!("cannot read meta.json: {e}"))?;
    if buf.len() as u64 > MAX_META_BYTES {
        return Err("meta.json exceeds 1 MiB".into());
    }
    let v: Value = serde_json::from_slice(&buf).map_err(|_| "malformed meta.json".to_string())?;
    if !v.is_object() {
        return Err("meta.json is not an object".into());
    }
    Ok(v)
}

fn agent_usage(path: &Path, session: &str, agent: &str) -> AgentUsage {
    let expected = Expected {
        session: Some(session.to_owned()),
        agent: Some(agent.to_owned()),
    };
    match read_report(path, &expected) {
        Err(reason) => AgentUsage {
            status: "unreadable_transcript",
            reason: Some(reason),
            latest_model: None,
            latest_call: None,
            epoch: None,
            compactions: None,
            cumulative: None,
            inherited_calls: None,
        },
        Ok(r) => {
            let observed = r.latest_call.is_some();
            AgentUsage {
                status: if r.latest_call_status == "historical_observation" {
                    "historical_observation"
                } else if observed {
                    "observed"
                } else {
                    "absent_counters"
                },
                reason: (!observed)
                    .then(|| "transcript has no valid per-call usage in its current epoch".into()),
                latest_model: r.latest_model,
                latest_call: r.latest_call,
                epoch: Some(r.epoch),
                compactions: Some(
                    r.lineage
                        .iter()
                        .filter(|e| matches!(e, LineageEvent::Compaction { .. }))
                        .count(),
                ),
                cumulative: Some(r.cumulative),
                inherited_calls: r
                    .inherited
                    .map(|i| i.distinct_calls.saturating_add(i.calls_without_usage)),
            }
        }
    }
}

fn build_agents(root: &Path, session: &str, with_usage: bool) -> Result<AgentsReport, String> {
    let mut session_dirs = Vec::new();
    let mut main_transcripts = 0;
    for dir in project_dirs(root)? {
        if is_real_dir(&dir.join(session)) {
            session_dirs.push(dir.join(session));
        } else if is_regular_file(&dir.join(format!("{session}.jsonl"))) {
            main_transcripts += 1;
        }
    }
    let mut diagnostics = Vec::new();
    let empty = |subagents_dir: Option<String>, diagnostics: Vec<String>| AgentsReport {
        schema_version: 1,
        host: "claude-code",
        session_id: session.to_owned(),
        subagents_dir,
        completeness: "complete_within_subagents_directory",
        agents_listed: 0,
        ignored_entries: 0,
        skipped_agent_entries: 0,
        diagnostics,
        agents: Vec::new(),
    };
    let session_dir = match session_dirs.len() {
        0 if main_transcripts == 1 => {
            return Ok(empty(
                None,
                vec!["session transcript found but it has no subagents directory".into()],
            ));
        }
        0 if main_transcripts == 0 => return Err("requested session not found".into()),
        0 => {
            return Err(format!(
                "ambiguous requested session: {main_transcripts} candidates"
            ));
        }
        1 => session_dirs.remove(0),
        n => return Err(format!("ambiguous requested session: {n} candidates")),
    };
    let sub = session_dir.join("subagents");
    if !is_real_dir(&sub) {
        return Ok(empty(
            None,
            vec!["session directory has no subagents directory".into()],
        ));
    }
    let entries =
        fs::read_dir(&sub).map_err(|e| format!("cannot read subagents directory: {e}"))?;
    let mut transcripts = BTreeSet::new();
    let mut metas = BTreeSet::new();
    let mut ignored = 0;
    let mut skipped_agent_entries = 0;
    for (count, entry) in entries.enumerate() {
        if count >= MAX_SCAN_ENTRIES {
            return Err("subagents directory entry limit reached".into());
        }
        let entry = entry.map_err(|e| format!("directory entry error: {e}"))?;
        let ty = entry
            .file_type()
            .map_err(|e| format!("file type error: {e}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let id_of = |suffix: &str| {
            name.strip_prefix("agent-")
                .and_then(|n| n.strip_suffix(suffix))
                .filter(|id| valid_agent_id(id))
                .map(str::to_owned)
        };
        let agent_shaped = id_of(".meta.json").is_some() || id_of(".jsonl").is_some();
        if !ty.is_file() || ty.is_symlink() {
            if agent_shaped {
                skipped_agent_entries += 1;
            } else {
                ignored += 1;
            }
        } else if let Some(id) = id_of(".meta.json") {
            metas.insert(id);
        } else if let Some(id) = id_of(".jsonl") {
            transcripts.insert(id);
        } else {
            ignored += 1;
        }
    }
    let ids: BTreeSet<_> = transcripts.union(&metas).cloned().collect();
    if ids.len() > MAX_AGENTS {
        return Err(format!("agent result limit reached ({MAX_AGENTS})"));
    }
    let mut partial = skipped_agent_entries > 0;
    let mut agents = Vec::new();
    for id in ids {
        let meta_path = sub.join(format!("agent-{id}.meta.json"));
        let transcript_path = sub.join(format!("agent-{id}.jsonl"));
        let (meta_status, meta_reason, meta) = if !metas.contains(&id) {
            partial = true;
            ("missing", None, Value::Null)
        } else {
            match read_meta(&meta_path) {
                Ok(v) => ("read", None, v),
                Err(e) => {
                    partial = true;
                    ("unreadable", Some(e), Value::Null)
                }
            }
        };
        let has_transcript = transcripts.contains(&id);
        if !has_transcript {
            partial = true;
        }
        let s = |k: &str| meta.get(k).and_then(Value::as_str).map(bounded_string);
        let usage = with_usage.then(|| {
            if has_transcript {
                agent_usage(&transcript_path, session, &id)
            } else {
                AgentUsage {
                    status: "missing_transcript",
                    reason: Some("no agent transcript file beside meta.json".into()),
                    latest_model: None,
                    latest_call: None,
                    epoch: None,
                    compactions: None,
                    cumulative: None,
                    inherited_calls: None,
                }
            }
        });
        if usage
            .as_ref()
            .is_some_and(|u| u.status == "unreadable_transcript")
        {
            partial = true;
        }
        agents.push(AgentRow {
            agent_type: s("agentType"),
            description: s("description"),
            requested_model: s("model"),
            requested_effort: s("effort"),
            is_fork: meta.get("isFork").and_then(Value::as_bool),
            spawn_depth: meta.get("spawnDepth").and_then(Value::as_u64),
            tool_use_id: s("toolUseId"),
            finished: meta.get("finished").and_then(Value::as_bool),
            transcript: has_transcript.then(|| transcript_path.display().to_string()),
            agent_id: id,
            meta_status,
            meta_reason,
            usage,
        });
    }
    if skipped_agent_entries > 0 {
        diagnostics.push(format!(
            "{skipped_agent_entries} agent-named entr(ies) in subagents/ are symlinks or not regular files; skipped, listing partial"
        ));
    }
    if ignored > 0 && diagnostics.len() < MAX_DIAGNOSTICS {
        diagnostics.push(format!(
            "{ignored} entr(ies) in subagents/ are not agent-<id>.jsonl or .meta.json regular files; ignored"
        ));
    }
    Ok(AgentsReport {
        schema_version: 1,
        host: "claude-code",
        session_id: session.to_owned(),
        subagents_dir: Some(sub.display().to_string()),
        completeness: if partial {
            "partial"
        } else {
            "complete_within_subagents_directory"
        },
        agents_listed: agents.len(),
        ignored_entries: ignored,
        skipped_agent_entries,
        diagnostics,
        agents,
    })
}

fn render_agent_usage(u: &AgentUsage) -> String {
    let Some(c) = &u.latest_call else {
        return match &u.reason {
            Some(r) => format!("{} ({r})", u.status),
            None => u.status.to_string(),
        };
    };
    let cum = u.cumulative.as_ref();
    format!(
        "{} model={} context={} epoch={} compactions={} calls={} weighted={:.0}{} at={}",
        u.status,
        u.latest_model.as_deref().unwrap_or("unknown"),
        c.context_tokens,
        u.epoch.map_or_else(|| "unknown".into(), |e| e.to_string()),
        u.compactions
            .map_or_else(|| "unknown".into(), |n| n.to_string()),
        cum.map_or_else(|| "unknown".into(), |c| c.distinct_calls.to_string()),
        cum.map_or(f64::NAN, |c| c.weighted_tokens),
        u.inherited_calls
            .map(|n| format!(" inherited_calls={n}"))
            .unwrap_or_default(),
        c.timestamp.as_deref().unwrap_or("unknown-time"),
    )
}

pub(crate) fn run_agents<I: IntoIterator<Item = String>>(args: I) -> Result<(), String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if args.iter().any(|x| x == "--help" || x == "-h") {
        println!("{}", agents_help());
        return Ok(());
    }
    let mut session = None;
    let mut projects_dir = None;
    let mut with_usage = false;
    let mut json = false;
    let mut it = args.into_iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--json" => json = true,
            "--with-usage" => with_usage = true,
            "--session" => session = Some(next_value(&mut it, &flag)?),
            "--projects-dir" => projects_dir = Some(PathBuf::from(next_value(&mut it, &flag)?)),
            _ => return Err(format!("unknown claude-agents option {flag}")),
        }
    }
    let session = session.ok_or("claude-agents requires --session ID")?;
    if !valid_session_id(&session) {
        return Err("--session must be an explicit hexadecimal/hyphen ID".into());
    }
    let root = projects_dir.unwrap_or_else(default_projects_dir);
    let report = build_agents(&root, &session, with_usage)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|_| "cannot serialize agent report")?
        );
        return Ok(());
    }
    println!(
        "Session {}: {} subagent(s), {}",
        report.session_id, report.agents_listed, report.completeness
    );
    for a in &report.agents {
        println!(
            "{}  {}  fork={}  effort={}  finished={}  meta={}  {}  {}",
            a.agent_id,
            a.agent_type.as_deref().unwrap_or("unknown-type"),
            a.is_fork
                .map_or_else(|| "unknown".to_string(), |f| f.to_string()),
            a.requested_effort.as_deref().unwrap_or("unknown"),
            a.finished
                .map_or_else(|| "unknown".to_string(), |f| f.to_string()),
            a.meta_status,
            a.description.as_deref().unwrap_or("unknown-description"),
            a.transcript.as_deref().unwrap_or("no-transcript"),
        );
        if let Some(reason) = &a.meta_reason {
            println!("  meta: {reason}");
        }
        if let Some(u) = &a.usage {
            println!("  usage: {}", render_agent_usage(u));
        }
    }
    for d in &report.diagnostics {
        println!("Note: {d}");
    }
    Ok(())
}
