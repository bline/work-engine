use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

mod agents;

const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;
const MAX_SCAN_ENTRIES: usize = 20_000;
const MAX_RECENT: usize = 8;
const MAX_DIAGNOSTICS: usize = 16;
const STALE_SECONDS: u64 = 900;

#[derive(Clone, Debug)]
struct Args {
    session: String,
    rollout: Option<PathBuf>,
    sessions_dir: Option<PathBuf>,
    json: bool,
    fresh_input: Option<u64>,
    preparation: Option<u64>,
    launch: Option<u64>,
    rehydration: Option<u64>,
    reconstruction: Option<u64>,
    fresh_cache_fraction: f64,
    cached_weight: f64,
    remaining_calls: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
struct Usage {
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: Option<u64>,
    timestamp: Option<String>,
    line: usize,
    epoch: u64,
}

struct UsageSignature {
    input: u64,
    cached: u64,
    output: Option<u64>,
    cumulative_input: Option<u64>,
    cumulative_cached: Option<u64>,
    cumulative_output: Option<u64>,
}
impl UsageSignature {
    fn matches_and_merges(&mut self, next: &Self) -> bool {
        fn compatible(a: Option<u64>, b: Option<u64>) -> bool {
            a.is_none() || b.is_none() || a == b
        }
        if self.input != next.input
            || self.cached != next.cached
            || !compatible(self.output, next.output)
            || !compatible(self.cumulative_input, next.cumulative_input)
            || !compatible(self.cumulative_cached, next.cumulative_cached)
            || !compatible(self.cumulative_output, next.cumulative_output)
        {
            return false;
        }
        self.output = self.output.or(next.output);
        self.cumulative_input = self.cumulative_input.or(next.cumulative_input);
        self.cumulative_cached = self.cumulative_cached.or(next.cumulative_cached);
        self.cumulative_output = self.cumulative_output.or(next.cumulative_output);
        true
    }
}

#[derive(Serialize)]
struct Growth {
    first_input_tokens: u64,
    last_input_tokens: u64,
    delta_tokens: i64,
    sample_count: usize,
    provenance: &'static str,
}

#[derive(Serialize)]
struct Economics {
    units: &'static str,
    cached_input_weight: f64,
    continuing_measured_input: u64,
    continuing_measured_cached_input: u64,
    continuing_weighted_per_call: f64,
    fresh_input_estimate: u64,
    fresh_cache_fraction: f64,
    fresh_weighted_per_call: f64,
    preparation: u64,
    launch: u64,
    rehydration: u64,
    reconstruction: u64,
    fixed_overhead: u64,
    break_even_calls: Option<u64>,
    remaining_calls: Option<u32>,
    continuing_scenario: Option<f64>,
    fresh_scenario: Option<f64>,
    interpretation: &'static str,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    session_id: String,
    root_session_id: Option<String>,
    forked_from_id: Option<String>,
    own_history_start_ordinal: Option<u64>,
    inherited_records_skipped: u64,
    rollout: String,
    file_modified_unix_seconds: Option<u64>,
    file_age_seconds: Option<u64>,
    freshness: &'static str,
    observed_at: Option<String>,
    recorded_model: Option<String>,
    recorded_effort: Option<String>,
    lineage: Vec<String>,
    latest_request: Option<Usage>,
    latest_request_status: &'static str,
    context_window_tokens: Option<u64>,
    context_window_source: &'static str,
    context_fraction_proxy: Option<f64>,
    context_headroom_tokens_proxy: Option<u64>,
    recent_requests: Vec<Usage>,
    recent_growth: Option<Growth>,
    economics: Option<Economics>,
    limitations: Vec<String>,
    diagnostic_counts: BTreeMap<&'static str, u64>,
    diagnostics_omitted: u64,
}

fn help() -> &'static str {
    "Usage: work-engine-context-status --session ID [--rollout FILE | --sessions-dir DIR] [--json] [--fresh-input-tokens N --preparation-tokens N --launch-tokens N --rehydration-tokens N --reconstruction-tokens N [--fresh-cache-fraction F] [--cached-weight F] [--remaining-calls N]]\n       work-engine-context-status agents --parent-thread ID [--recursive] [--sessions-dir DIR] [--json]"
}

fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Args, String> {
    let mut a = Args {
        session: String::new(),
        rollout: None,
        sessions_dir: None,
        json: false,
        fresh_input: None,
        preparation: None,
        launch: None,
        rehydration: None,
        reconstruction: None,
        fresh_cache_fraction: 0.0,
        cached_weight: 0.1,
        remaining_calls: None,
    };
    let mut it = args.into_iter();
    while let Some(flag) = it.next() {
        if flag == "--help" || flag == "-h" {
            return Err(help().into());
        }
        if flag == "--json" {
            a.json = true;
            continue;
        }
        let val = it
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--session" => a.session = val,
            "--rollout" => a.rollout = Some(PathBuf::from(val)),
            "--sessions-dir" => a.sessions_dir = Some(PathBuf::from(val)),
            "--fresh-input-tokens" => a.fresh_input = Some(parse_u64(&val, &flag)?),
            "--preparation-tokens" => a.preparation = Some(parse_u64(&val, &flag)?),
            "--launch-tokens" => a.launch = Some(parse_u64(&val, &flag)?),
            "--rehydration-tokens" => a.rehydration = Some(parse_u64(&val, &flag)?),
            "--reconstruction-tokens" => a.reconstruction = Some(parse_u64(&val, &flag)?),
            "--fresh-cache-fraction" => a.fresh_cache_fraction = parse_fraction(&val, &flag)?,
            "--cached-weight" => a.cached_weight = parse_fraction(&val, &flag)?,
            "--remaining-calls" => {
                let n = val.parse::<u32>().map_err(|_| format!("invalid {flag}"))?;
                if n == 0 || n > 1_000_000 {
                    return Err(format!("{flag} must be 1..=1000000"));
                }
                a.remaining_calls = Some(n);
            }
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    if !valid_session_id(&a.session) {
        return Err("--session must be an explicit hexadecimal/hyphen ID".into());
    }
    if a.rollout.is_some() && a.sessions_dir.is_some() {
        return Err("choose --rollout or --sessions-dir".into());
    }
    let econ = [
        a.fresh_input,
        a.preparation,
        a.launch,
        a.rehydration,
        a.reconstruction,
    ];
    if econ.iter().any(Option::is_some) && !econ.iter().all(Option::is_some) {
        return Err("economics requires fresh input and all four overhead components".into());
    }
    if a.remaining_calls.is_some() && a.fresh_input.is_none() {
        return Err("--remaining-calls requires economics inputs".into());
    }
    Ok(a)
}
fn valid_session_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}
fn parse_u64(s: &str, flag: &str) -> Result<u64, String> {
    s.parse().map_err(|_| format!("invalid {flag}"))
}
fn parse_fraction(s: &str, flag: &str) -> Result<f64, String> {
    let n: f64 = s.parse().map_err(|_| format!("invalid {flag}"))?;
    if !(0.0..=1.0).contains(&n) || !n.is_finite() {
        return Err(format!("{flag} must be 0..=1"));
    }
    Ok(n)
}

fn locate(a: &Args) -> Result<PathBuf, String> {
    if let Some(p) = &a.rollout {
        return Ok(p.clone());
    }
    let root = a.sessions_dir.clone().unwrap_or_else(|| {
        let home = env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|v| PathBuf::from(v).join(".codex")))
            .unwrap_or_default();
        home.join("sessions")
    });
    let mut stack = vec![(root, 0_usize)];
    let mut matches = Vec::new();
    let mut count = 0;
    while let Some((dir, depth)) = stack.pop() {
        if depth > 5 {
            return Err("sessions directory depth limit reached".into());
        }
        let entries = fs::read_dir(&dir)
            .map_err(|e| format!("cannot read sessions directory {}: {e}", dir.display()))?;
        for entry in entries {
            count += 1;
            if count > MAX_SCAN_ENTRIES {
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
            } else if ty.is_file() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.ends_with(&format!("-{}.jsonl", a.session)) {
                    matches.push(entry.path());
                }
            }
        }
    }
    match matches.len() {
        0 => Err("requested session rollout not found".into()),
        1 => Ok(matches.remove(0)),
        _ => Err(format!(
            "ambiguous requested session: {} candidate rollouts",
            matches.len()
        )),
    }
}

fn number(v: &Value, key: &str) -> Option<u64> {
    v.get(key)?.as_u64()
}
fn bounded_string(s: &str) -> String {
    s.chars().take(256).collect()
}
fn field_string(v: &Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().map(bounded_string)
}
fn note(
    limitations: &mut Vec<String>,
    counts: &mut BTreeMap<&'static str, u64>,
    category: &'static str,
    message: String,
) {
    let count = counts.entry(category).or_default();
    *count = count.saturating_add(1);
    if limitations.len() < MAX_DIAGNOSTICS {
        limitations.push(message);
    }
}
fn weighted(input: u64, cached: u64, weight: f64) -> f64 {
    (input - cached) as f64 + cached as f64 * weight
}

fn economics(a: &Args, latest: Option<&Usage>) -> Option<Economics> {
    let (
        Some(fresh),
        Some(prep),
        Some(launch),
        Some(rehydration),
        Some(reconstruction),
        Some(last),
    ) = (
        a.fresh_input,
        a.preparation,
        a.launch,
        a.rehydration,
        a.reconstruction,
        latest,
    )
    else {
        return None;
    };
    let fixed = prep
        .checked_add(launch)?
        .checked_add(rehydration)?
        .checked_add(reconstruction)?;
    let continuing = weighted(last.input_tokens, last.cached_input_tokens, a.cached_weight);
    let fresh_weighted =
        fresh as f64 * (1.0 - a.fresh_cache_fraction + a.fresh_cache_fraction * a.cached_weight);
    let delta = continuing - fresh_weighted;
    let break_even = if delta > 0.0 {
        Some(((fixed as f64 / delta).floor() as u64).saturating_add(1))
    } else {
        None
    };
    let n = a.remaining_calls.map(f64::from);
    Some(Economics {
        units: "declared weighted input tokens; cached token weight is an assumption",
        cached_input_weight: a.cached_weight,
        continuing_measured_input: last.input_tokens,
        continuing_measured_cached_input: last.cached_input_tokens,
        continuing_weighted_per_call: continuing,
        fresh_input_estimate: fresh,
        fresh_cache_fraction: a.fresh_cache_fraction,
        fresh_weighted_per_call: fresh_weighted,
        preparation: prep,
        launch,
        rehydration,
        reconstruction,
        fixed_overhead: fixed,
        break_even_calls: break_even,
        remaining_calls: a.remaining_calls,
        continuing_scenario: n.map(|x| x * continuing),
        fresh_scenario: n.map(|x| x * fresh_weighted + fixed as f64),
        interpretation: "conditional scenario: constant per-call input/cache assumptions; break-even is the first integer call count where fresh plus fixed overhead is strictly lower",
    })
}

fn read_line_bounded(reader: &mut impl BufRead, line: &mut Vec<u8>) -> Result<usize, String> {
    line.clear();
    loop {
        let buf = reader
            .fill_buf()
            .map_err(|e| format!("rollout read error: {e}"))?;
        if buf.is_empty() {
            return Ok(line.len());
        }
        let take = buf
            .iter()
            .position(|b| *b == b'\n')
            .map_or(buf.len(), |n| n + 1);
        if line.len().saturating_add(take) > MAX_LINE_BYTES {
            return Err("rollout line exceeds 8 MiB bound".into());
        }
        let complete = buf[take - 1] == b'\n';
        line.extend_from_slice(&buf[..take]);
        reader.consume(take);
        if complete {
            return Ok(line.len());
        }
    }
}

fn optional_fork_id(payload: &Value, own_id: &str) -> Result<Option<String>, String> {
    match payload.get("forked_from_id") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(id)) if valid_session_id(id) && id != own_id => Ok(Some(id.clone())),
        _ => Err("invalid or self-referencing forked_from_id".into()),
    }
}

fn read_report(a: &Args, path: &Path) -> Result<Report, String> {
    let metadata = fs::metadata(path).map_err(|e| format!("cannot stat rollout: {e}"))?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err("rollout is not a regular file or exceeds 256 MiB".into());
    }
    let modified = metadata
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
    let mut reader =
        BufReader::new(File::open(path).map_err(|e| format!("cannot open rollout: {e}"))?);
    let mut line = Vec::new();
    let mut line_no = 0_usize;
    let mut meta_count = 0_usize;
    let mut meta_id = None;
    let mut root_session_id = None;
    let mut forked_from_id = None;
    let mut own_history_start_ordinal = None;
    let mut expected_inherited_meta = None;
    let mut inherited_meta_ids = BTreeSet::new();
    let mut inherited_records_skipped = 0_u64;
    let mut limitations = Vec::new();
    let mut diagnostic_counts = BTreeMap::new();
    let mut latest = None;
    let mut recent: VecDeque<Usage> = VecDeque::new();
    let mut epoch = 0_u64;
    let mut gap = false;
    let mut window = None;
    let mut last_valid_window = None;
    let mut model = None;
    let mut effort = None;
    let mut observed = None;
    let mut lineage = Vec::new();
    let mut last_signature: Option<UsageSignature> = None;
    loop {
        let read = read_line_bounded(&mut reader, &mut line)?;
        if read == 0 {
            break;
        }
        line_no += 1;
        let complete = line.last() == Some(&b'\n');
        let record: Value = match serde_json::from_slice(&line) {
            Ok(v) => v,
            Err(_) if !complete => {
                gap = true;
                note(
                    &mut limitations,
                    &mut diagnostic_counts,
                    "partial_trailing_record",
                    format!(
                        "partial trailing line {line_no} ignored; latest request is only the latest valid observation"
                    ),
                );
                break;
            }
            Err(_) => {
                if line_no == 1 {
                    return Err("first rollout record must be valid session_meta".into());
                }
                if own_history_start_ordinal.is_some() {
                    return Err(format!(
                        "malformed record in ordinal-bound fork at line {line_no}"
                    ));
                }
                gap = true;
                note(
                    &mut limitations,
                    &mut diagnostic_counts,
                    "malformed_record",
                    format!(
                        "malformed complete record at line {line_no} ignored; latest request is only the latest valid observation"
                    ),
                );
                continue;
            }
        };
        let ty = record.get("type").and_then(Value::as_str).unwrap_or("");
        let payload = record.get("payload").unwrap_or(&Value::Null);
        // The first record owns file identity. In the observed fork format,
        // [1, subagent_history_start_ordinal) is copied history, even though
        // its outer timestamps have been rewritten to the child's launch time.
        if line_no == 1 && ty != "session_meta" {
            return Err("first rollout record must be session_meta".into());
        }
        if let Some(start) = own_history_start_ordinal {
            let ordinal = number(&record, "ordinal");
            if ordinal != Some((line_no - 1) as u64) {
                return Err(format!(
                    "noncontiguous or missing fork ordinal at line {line_no}"
                ));
            }
            if ordinal.is_some_and(|n| n < start) {
                if ty == "session_meta" {
                    let id = field_string(payload, "id");
                    if expected_inherited_meta.is_none() || id != expected_inherited_meta {
                        return Err(format!(
                            "inherited session_meta.id mismatch at line {line_no}"
                        ));
                    }
                    if !inherited_meta_ids.insert(id.clone()) {
                        return Err(format!(
                            "duplicate inherited session_meta at line {line_no}"
                        ));
                    }
                    if let Some(root) = payload.get("session_id") {
                        let root = root.as_str().filter(|id| valid_session_id(id));
                        if root.is_none()
                            || root_session_id
                                .as_deref()
                                .is_some_and(|own| Some(own) != root)
                        {
                            return Err(format!(
                                "inherited root-session mismatch at line {line_no}"
                            ));
                        }
                    }
                    expected_inherited_meta = optional_fork_id(payload, &a.session)?;
                }
                inherited_records_skipped += 1;
                continue;
            }
            // These records explicitly identify their originating thread.
            // Do not accept a foreign producer in the child's own suffix.
            if (ty == "token_usage_record"
                || (ty == "event_msg"
                    && payload.get("type").and_then(Value::as_str)
                        == Some("thread_settings_applied")))
                && payload.get("thread_id").and_then(Value::as_str) != Some(&a.session)
            {
                return Err(format!(
                    "own-history thread identity mismatch at line {line_no}"
                ));
            }
        }
        if ty == "session_meta" {
            meta_count += 1;
            let id = field_string(payload, "id");
            if id.as_deref() != Some(&a.session) {
                return Err(format!("session_meta.id mismatch at line {line_no}"));
            }
            if let Some(root) = payload.get("session_id") {
                let Some(root) = root.as_str().filter(|id| valid_session_id(id)) else {
                    return Err(format!(
                        "session_meta.session_id is malformed at line {line_no}"
                    ));
                };
                root_session_id = Some(root.to_owned());
            }
            meta_id = id;
            if line_no == 1 {
                forked_from_id = optional_fork_id(payload, &a.session)?;
                let boundary = payload.get("subagent_history_start_ordinal");
                if forked_from_id.is_some() {
                    let start = boundary.and_then(Value::as_u64).filter(|n| *n > 0)
                        .ok_or("forked rollout lacks a positive subagent_history_start_ordinal; usage attribution unavailable")?;
                    if number(&record, "ordinal") != Some(0) {
                        return Err("forked rollout must start at ordinal zero".into());
                    }
                    own_history_start_ordinal = Some(start);
                    expected_inherited_meta = forked_from_id.clone();
                    note(
                        &mut limitations,
                        &mut diagnostic_counts,
                        "inherited_history_excluded",
                        format!(
                            "inherited records before ordinal {start} excluded from usage, model and compaction observations"
                        ),
                    );
                } else if boundary.is_some_and(|v| !v.is_null()) {
                    return Err(
                        "history boundary without forked_from_id; usage attribution unavailable"
                            .into(),
                    );
                }
            }
            continue;
        }
        if ty == "compacted"
            || (ty == "event_msg"
                && payload.get("type").and_then(Value::as_str) == Some("compacted"))
        {
            epoch = epoch.saturating_add(1);
            recent.clear();
            latest = None;
            window = None;
            last_valid_window = None;
            last_signature = None;
            if lineage.len() < 32 {
                lineage.push(format!("compaction marker at line {line_no}"));
            }
            continue;
        }
        if ty == "turn_context" {
            let new_model = field_string(payload, "model");
            if model.is_some() && new_model.is_some() && model != new_model {
                epoch = epoch.saturating_add(1);
                recent.clear();
                latest = None;
                window = None;
                last_valid_window = None;
                last_signature = None;
                effort = None;
                if lineage.len() < 32 {
                    lineage.push(format!("model changed at line {line_no}"));
                }
            }
            if new_model.is_some() {
                model = new_model;
            }
            let new_effort = field_string(payload, "effort").or_else(|| {
                payload
                    .pointer("/collaboration_mode/settings/reasoning_effort")
                    .and_then(Value::as_str)
                    .map(bounded_string)
            });
            if new_effort.is_some() {
                effort = new_effort;
            }
            continue;
        }
        if ty != "event_msg" || payload.get("type").and_then(Value::as_str) != Some("token_count") {
            continue;
        }
        let info = payload.get("info").unwrap_or(&Value::Null);
        let Some(last) = info.get("last_token_usage") else {
            gap = true;
            note(
                &mut limitations,
                &mut diagnostic_counts,
                "missing_last_usage",
                format!("token_count line {line_no} lacks last_token_usage"),
            );
            continue;
        };
        let (Some(input), Some(cached)) = (
            number(last, "input_tokens"),
            number(last, "cached_input_tokens"),
        ) else {
            gap = true;
            note(
                &mut limitations,
                &mut diagnostic_counts,
                "invalid_usage_counter",
                format!("token_count line {line_no} lacks safe input/cache counters"),
            );
            continue;
        };
        if cached > input {
            gap = true;
            note(
                &mut limitations,
                &mut diagnostic_counts,
                "cache_inconsistent",
                format!("cache exceeds input at line {line_no}; snapshot ignored"),
            );
            continue;
        }
        let new_window = number(info, "model_context_window").filter(|w| *w > 0);
        if new_window.is_none() {
            gap = true;
            note(
                &mut limitations,
                &mut diagnostic_counts,
                "invalid_window",
                format!(
                    "token_count line {line_no} lacks a positive safe model_context_window; proxy unavailable"
                ),
            );
        }
        let known_change =
            matches!((last_valid_window, new_window), (Some(old), Some(new)) if old != new);
        let availability_change = latest.is_some() && window.is_some() != new_window.is_some();
        if known_change || availability_change {
            epoch = epoch.saturating_add(1);
            recent.clear();
            last_signature = None;
            if lineage.len() < 32 {
                lineage.push(format!(
                    "context window or its availability changed at line {line_no}"
                ));
            }
        }
        window = new_window;
        if new_window.is_some() {
            last_valid_window = new_window;
        }
        if let Some(prev) = recent.back()
            && input < prev.input_tokens
        {
            epoch = epoch.saturating_add(1);
            recent.clear();
            last_signature = None;
            if lineage.len() < 32 {
                lineage.push(format!(
                    "input counter decreased at line {line_no}; possible reset"
                ));
            }
        }
        let timestamp = field_string(&record, "timestamp");
        let cumulative = info.get("total_token_usage").unwrap_or(&Value::Null);
        let signature = UsageSignature {
            input,
            cached,
            output: number(last, "output_tokens"),
            cumulative_input: number(cumulative, "input_tokens"),
            cumulative_cached: number(cumulative, "cached_input_tokens"),
            cumulative_output: number(cumulative, "output_tokens"),
        };
        let usage = Usage {
            input_tokens: input,
            cached_input_tokens: cached,
            output_tokens: number(last, "output_tokens"),
            timestamp,
            line: line_no,
            epoch,
        };
        let repeat = last_signature
            .as_mut()
            .is_some_and(|previous| previous.matches_and_merges(&signature));
        if !repeat {
            observed = usage.timestamp.clone().or(observed);
            latest = Some(usage.clone());
            if recent.len() == MAX_RECENT {
                recent.pop_front();
            }
            recent.push_back(usage);
            last_signature = Some(signature);
        }
    }
    if meta_count != 1 || meta_id.is_none() {
        return Err(format!(
            "expected exactly one matching session_meta; found {meta_count}"
        ));
    }
    let recent: Vec<_> = recent.into_iter().collect();
    let growth = if recent.len() >= 2 {
        let first = &recent[0];
        let end = &recent[recent.len() - 1];
        Some(Growth {
            first_input_tokens: first.input_tokens,
            last_input_tokens: end.input_tokens,
            delta_tokens: (end.input_tokens as i128 - first.input_tokens as i128)
                .clamp(i64::MIN as i128, i64::MAX as i128) as i64,
            sample_count: recent.len(),
            provenance: "distinct token_count.last_token_usage snapshots in current observed epoch, at most 8; endpoint difference, not a sum",
        })
    } else {
        None
    };
    let fraction = latest
        .as_ref()
        .zip(window)
        .map(|(u, w)| u.input_tokens as f64 / w as f64);
    let headroom = latest
        .as_ref()
        .zip(window)
        .map(|(u, w)| w.saturating_sub(u.input_tokens));
    if latest.is_none() {
        note(
            &mut limitations,
            &mut diagnostic_counts,
            "missing_current_usage",
            "no valid per-request usage snapshot; current context unavailable".into(),
        );
    }
    if window.is_none() {
        note(
            &mut limitations,
            &mut diagnostic_counts,
            "missing_current_window",
            "no valid model_context_window; fraction and headroom unavailable".into(),
        );
    }
    if let Some(x) = fraction
        && x > 1.0
    {
        note(
            &mut limitations,
            &mut diagnostic_counts,
            "input_exceeds_window",
            "latest input exceeds recorded window; proxy is not a measured remaining capacity"
                .into(),
        );
    }
    if freshness != "recent_file_modification" {
        note(
            &mut limitations,
            &mut diagnostic_counts,
            "stale_file",
            "file recency does not establish active-session identity or current in-memory context"
                .into(),
        );
    }
    if lineage.len() == 32 {
        note(
            &mut limitations,
            &mut diagnostic_counts,
            "lineage_truncated",
            "lineage markers truncated at 32".into(),
        );
    }
    if a.fresh_input.is_some() && latest.is_none() {
        note(
            &mut limitations,
            &mut diagnostic_counts,
            "economics_unavailable",
            "economics unavailable without a valid measured continuing input".into(),
        );
    }
    let econ = economics(a, latest.as_ref());
    if a.fresh_input.is_some() && econ.is_none() && latest.is_some() {
        note(
            &mut limitations,
            &mut diagnostic_counts,
            "economics_unavailable",
            "economics unavailable: overhead total overflow".into(),
        );
    }
    let latest_request_status = if latest.is_none() {
        "unavailable"
    } else if gap {
        "latest_valid_observation_with_record_gaps"
    } else {
        "latest_valid_observation"
    };
    let diagnostics_omitted = diagnostic_counts
        .values()
        .copied()
        .sum::<u64>()
        .saturating_sub(limitations.len() as u64);
    Ok(Report {
        schema_version: 1,
        session_id: a.session.clone(),
        root_session_id,
        forked_from_id,
        own_history_start_ordinal,
        inherited_records_skipped,
        rollout: path.display().to_string(),
        file_modified_unix_seconds: modified,
        file_age_seconds: age,
        freshness,
        observed_at: observed,
        recorded_model: model,
        recorded_effort: effort,
        lineage,
        latest_request: latest,
        latest_request_status,
        context_window_tokens: window,
        context_window_source: "event_msg.token_count.info.model_context_window",
        context_fraction_proxy: fraction,
        context_headroom_tokens_proxy: headroom,
        recent_requests: recent,
        recent_growth: growth,
        economics: econ,
        limitations,
        diagnostic_counts,
        diagnostics_omitted,
    })
}

fn render_text(r: &Report) {
    println!("Session {}", r.session_id);
    if let Some(root) = &r.root_session_id {
        println!("Recorded root session: {root}");
    }
    println!("Rollout: {}", r.rollout);
    println!(
        "Freshness: {} (file age: {})",
        r.freshness,
        r.file_age_seconds
            .map(|x| format!("{x}s"))
            .unwrap_or_else(|| "unknown".into())
    );
    println!(
        "Recorded model/effort: {}/{}",
        r.recorded_model.as_deref().unwrap_or("unknown"),
        r.recorded_effort.as_deref().unwrap_or("unknown")
    );
    if let Some(u) = &r.latest_request {
        println!(
            "Latest valid observed request input: {} tokens ({} cached), line {}, timestamp {} [{}]",
            u.input_tokens,
            u.cached_input_tokens,
            u.line,
            u.timestamp.as_deref().unwrap_or("unknown"),
            r.latest_request_status
        );
    } else {
        println!("Latest request input: unavailable");
    }
    if let (Some(w), Some(f), Some(h)) = (
        r.context_window_tokens,
        r.context_fraction_proxy,
        r.context_headroom_tokens_proxy,
    ) {
        println!(
            "Context proxy: {:.1}% of {} tokens; headroom proxy {} tokens",
            f * 100.0,
            w,
            h
        );
    } else {
        println!("Context proxy: unavailable");
    }
    if let Some(g) = &r.recent_growth {
        println!(
            "Recent input change: {:+} tokens across {} distinct snapshots (current observed epoch)",
            g.delta_tokens, g.sample_count
        );
    }
    if let Some(e) = &r.economics {
        println!(
            "Weighted input scenario: continue {:.1}/call; fresh {:.1}/call + {} fixed; break-even calls {}",
            e.continuing_weighted_per_call,
            e.fresh_weighted_per_call,
            e.fixed_overhead,
            e.break_even_calls
                .map(|n| n.to_string())
                .unwrap_or_else(|| "unavailable".into())
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

fn main() {
    if env::args()
        .nth(1)
        .as_deref()
        .is_some_and(|x| x == "--help" || x == "-h")
    {
        println!("{}", help());
        return;
    }
    if env::args().nth(1).as_deref() == Some("agents") {
        if let Err(e) = agents::run(env::args().skip(2)) {
            eprintln!("context-status agents: {e}");
            std::process::exit(1);
        }
        return;
    }
    let a = match parse_args(env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let result = locate(&a).and_then(|p| read_report(&a, &p));
    match result {
        Ok(report) => {
            if a.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("serializable report")
                );
            } else {
                render_text(&report);
            }
        }
        Err(e) => {
            eprintln!("context-status: {e}");
            std::process::exit(1);
        }
    }
}
