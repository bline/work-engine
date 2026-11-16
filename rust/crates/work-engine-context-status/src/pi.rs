//! Read-only context telemetry from one Pi session file.
//!
//! Pi normalizes every provider's usage into `input` (uncached), `cacheRead`,
//! `cacheWrite` and `output`, so one reader serves every Pi inference route.
//! The active branch is the parent chain of the last appended entry; each
//! `compaction` entry on it (summary or package-owned epoch boundary such as
//! `new_context`) starts a new epoch. A new epoch has no reading until its
//! first valid assistant response, matching Pi's own `getContextUsage`.
use crate::{MAX_FILE_BYTES, MAX_RECENT, note, read_line_bounded_as, valid_session_id};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

const HELP: &str = "Usage: work-engine-context-status pi --session ID --session-file FILE\nReads one Pi session file and prints a JSON context report. The file records no context window; no fraction or headroom is reported.";

#[derive(Serialize, Clone, Debug)]
pub(crate) struct Request {
    /// uncached input + cache read + cache write: what this request carried.
    input_tokens: u64,
    uncached_input_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    output_tokens: u64,
    /// Pi's own `calculateContextTokens` (totalTokens, else the four-field
    /// sum including output); Pi's automatic threshold compares this value.
    pi_context_tokens: u64,
    provider: Option<String>,
    model: Option<String>,
    timestamp: Option<String>,
    line: usize,
    epoch: u64,
}

#[derive(Serialize)]
pub(crate) struct Report {
    schema_version: u32,
    host: &'static str,
    session_id: String,
    parent_session: Option<String>,
    session_file: String,
    file_modified_unix_seconds: Option<u64>,
    file_age_seconds: Option<u64>,
    freshness: &'static str,
    recorded_model: Option<String>,
    recorded_api: Option<String>,
    leaf_line: Option<usize>,
    epoch: u64,
    epoch_started_line: Option<usize>,
    latest_request: Option<Request>,
    latest_request_status: &'static str,
    #[serde(skip)]
    pub(crate) last_record_gap_line: Option<usize>,
    context_window_tokens: Option<u64>,
    recent_requests: Vec<Request>,
    limitations: Vec<String>,
    diagnostic_counts: BTreeMap<&'static str, u64>,
    diagnostics_omitted: u64,
}

enum Kind {
    Request(Box<Request>),
    Compaction,
    Other,
}

struct Entry {
    parent: Option<String>,
    line: usize,
    kind: Kind,
}

fn counter(usage: &Value, key: &str) -> Option<u64> {
    match usage.get(key) {
        None | Some(Value::Null) => Some(0),
        Some(v) => v.as_u64(),
    }
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().map(|s| s.chars().take(256).collect())
}

/// A usable request mirrors Pi's `getAssistantUsage`: not aborted or errored,
/// valid nonnegative counters, and a positive context sum.
fn request(message: &Value, timestamp: Option<String>, line: usize) -> Option<Request> {
    let stop = message.get("stopReason").and_then(Value::as_str);
    if matches!(stop, Some("aborted" | "error")) {
        return None;
    }
    let usage = message.get("usage")?;
    let input = counter(usage, "input")?;
    let read = counter(usage, "cacheRead")?;
    let write = counter(usage, "cacheWrite")?;
    let output = counter(usage, "output")?;
    let carried = input.checked_add(read)?.checked_add(write)?;
    let total = counter(usage, "totalTokens")?;
    let pi_context = if total > 0 {
        total
    } else {
        carried.checked_add(output)?
    };
    (carried > 0).then(|| Request {
        input_tokens: carried,
        uncached_input_tokens: input,
        cache_read_tokens: read,
        cache_write_tokens: write,
        output_tokens: output,
        pi_context_tokens: pi_context,
        provider: text(message, "provider"),
        model: text(message, "model"),
        timestamp,
        line,
        epoch: 0,
    })
}

pub(crate) fn read_report(path: &Path, session: &str) -> Result<Report, String> {
    let metadata = fs::metadata(path).map_err(|e| format!("cannot stat session file: {e}"))?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err("session file is not a regular file or exceeds 256 MiB".into());
    }
    let (modified, age, freshness) = crate::claude::freshness_of(&metadata);
    let mut reader =
        BufReader::new(File::open(path).map_err(|e| format!("cannot open session file: {e}"))?);
    let mut line = Vec::new();
    let mut line_no = 0_usize;
    let mut limitations = Vec::new();
    let mut counts = BTreeMap::new();
    let mut gap = None;
    let mut parent_session = None;
    let mut entries: HashMap<String, Entry> = HashMap::new();
    let mut leaf: Option<String> = None;
    let mut api = None;
    note(
        &mut limitations,
        &mut counts,
        "no_context_window",
        "no context window recorded; no fraction or headroom is reported".into(),
    );
    loop {
        let read = read_line_bounded_as(&mut reader, &mut line, "session file")?;
        if read == 0 {
            break;
        }
        line_no += 1;
        let complete = line.last() == Some(&b'\n');
        let record: Value = match serde_json::from_slice(&line) {
            Ok(v) => v,
            Err(_) if line_no == 1 => return Err("first session record is not JSON".into()),
            Err(_) if !complete => {
                gap = Some(line_no);
                note(
                    &mut limitations,
                    &mut counts,
                    "partial_trailing_record",
                    format!(
                        "partial trailing line {line_no} ignored; latest request is only the latest valid observation"
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
                        "malformed complete record at line {line_no} ignored; latest request is only the latest valid observation"
                    ),
                );
                continue;
            }
        };
        if line_no == 1 {
            if record.get("type").and_then(Value::as_str) != Some("session") {
                return Err("first session record must be the Pi session header".into());
            }
            let id = record.get("id").and_then(Value::as_str);
            if id != Some(session) {
                return Err(
                    "session header identity is absent or does not match the binding".into(),
                );
            }
            if let Some(p) = record.get("parentSession") {
                parent_session = Some(
                    p.as_str()
                        .map(|s| s.chars().take(4096).collect())
                        .ok_or("malformed parentSession in session header")?,
                );
            }
            continue;
        }
        let Some(id) = record.get("id").and_then(Value::as_str) else {
            continue;
        };
        let parent = match record.get("parentId") {
            None | Some(Value::Null) => None,
            Some(Value::String(p)) => Some(p.clone()),
            Some(_) => {
                gap = Some(line_no);
                note(
                    &mut limitations,
                    &mut counts,
                    "malformed_record",
                    format!("malformed parentId at line {line_no} ignored"),
                );
                continue;
            }
        };
        let kind = match record.get("type").and_then(Value::as_str) {
            Some("compaction") => Kind::Compaction,
            Some("message") => {
                let message = record.get("message").unwrap_or(&Value::Null);
                if message.get("role").and_then(Value::as_str) == Some("assistant") {
                    match request(message, text(&record, "timestamp"), line_no) {
                        Some(r) => {
                            api = text(message, "api");
                            Kind::Request(Box::new(r))
                        }
                        None => Kind::Other,
                    }
                } else {
                    Kind::Other
                }
            }
            _ => Kind::Other,
        };
        if entries.contains_key(id) {
            return Err(format!(
                "duplicate entry id at line {line_no}; counters cannot be attributed"
            ));
        }
        entries.insert(
            id.to_owned(),
            Entry {
                parent,
                line: line_no,
                kind,
            },
        );
        leaf = Some(id.to_owned());
    }
    if line_no == 0 {
        return Err("session file is empty".into());
    }
    // Active branch: parent chain of the last appended entry.
    let mut branch = Vec::new();
    let mut cursor = leaf.clone();
    while let Some(id) = cursor {
        let Some(entry) = entries.get(&id) else {
            note(
                &mut limitations,
                &mut counts,
                "broken_parent_chain",
                "active branch references an entry absent from the file; earlier epochs are not counted".into(),
            );
            break;
        };
        if branch.len() > entries.len() {
            return Err("cyclic parent chain".into());
        }
        branch.push(entry);
        cursor = entry.parent.clone();
    }
    branch.reverse();
    let mut epoch = 0_u64;
    let mut epoch_started_line = None;
    let mut latest: Option<Request> = None;
    let mut recent: VecDeque<Request> = VecDeque::new();
    for entry in &branch {
        match &entry.kind {
            Kind::Compaction => {
                epoch += 1;
                epoch_started_line = Some(entry.line);
                latest = None;
                recent.clear();
            }
            Kind::Request(r) => {
                let mut r = (**r).clone();
                r.epoch = epoch;
                if recent.len() == MAX_RECENT {
                    recent.pop_front();
                }
                recent.push_back(r.clone());
                latest = Some(r);
            }
            Kind::Other => {}
        }
    }
    note(
        &mut limitations,
        &mut counts,
        "leaf_is_last_appended",
        "active branch is the last appended entry's parent chain; tree navigation without a new entry is not observed".into(),
    );
    if freshness != "recent_file_modification" {
        note(
            &mut limitations,
            &mut counts,
            "stale_file",
            "file recency does not establish active-session identity or current in-memory context"
                .into(),
        );
    }
    let recorded_model = latest.as_ref().and_then(|r| match (&r.provider, &r.model) {
        (Some(p), Some(m)) => Some(format!("{p}/{m}")),
        (None, Some(m)) => Some(m.clone()),
        _ => None,
    });
    let latest_request_status = if latest.is_none() {
        if epoch > 0 {
            "awaiting_first_request_in_epoch"
        } else {
            "unavailable"
        }
    } else if latest.as_ref().is_some_and(|r| r.line <= gap.unwrap_or(0)) {
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
        host: "pi",
        session_id: session.to_owned(),
        parent_session,
        session_file: path.display().to_string(),
        file_modified_unix_seconds: modified,
        file_age_seconds: age,
        freshness,
        recorded_model,
        recorded_api: api,
        leaf_line: leaf.and_then(|id| entries.get(&id).map(|e| e.line)),
        epoch,
        epoch_started_line,
        latest_request: latest,
        latest_request_status,
        last_record_gap_line: gap,
        context_window_tokens: None,
        recent_requests: recent.into_iter().collect(),
        limitations,
        diagnostic_counts: counts,
        diagnostics_omitted,
    })
}

pub(crate) fn run(args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut session = String::new();
    let mut file = PathBuf::new();
    let mut args = args;
    while let Some(flag) = args.next() {
        if flag == "--help" || flag == "-h" {
            println!("{HELP}");
            return Ok(());
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--session" => session = value,
            "--session-file" => file = PathBuf::from(value),
            _ => return Err(format!("unknown pi option {flag}; {HELP}")),
        }
    }
    if !valid_session_id(&session) || !file.is_absolute() {
        return Err(format!(
            "give a valid --session and an absolute --session-file; {HELP}"
        ));
    }
    let report = read_report(&file, &session)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("serializable report")
    );
    Ok(())
}
