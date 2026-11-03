//! Read-only retrospective measurement. Tool/command text is data, never executed.
//! Input comes from the existing usage ledger and frozen rollout/state prefixes.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    env, fs,
    io::{BufRead, BufReader},
    path::Path,
};

fn ts(s: &str) -> Option<f64> {
    let s = s.strip_suffix('Z').or_else(|| s.strip_suffix("+00:00"))?;
    let year: i32 = s.get(..4)?.parse().ok()?;
    let month: usize = s.get(5..7)?.parse().ok()?;
    let day: i64 = s.get(8..10)?.parse().ok()?;
    let hour: f64 = s.get(11..13)?.parse().ok()?;
    let minute: f64 = s.get(14..16)?.parse().ok()?;
    let second: f64 = s.get(17..)?.parse().ok()?;
    let leap = |y: i32| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    if !(1970..2100).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    let mut days: i64 = (1970..year).map(|y| if leap(y) { 366 } else { 365 }).sum();
    let lengths = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day < 1 || day > lengths[month - 1] {
        return None;
    }
    days += lengths[..month - 1].iter().sum::<i64>() + day - 1;
    Some(days as f64 * 86400.0 + hour * 3600.0 + minute * 60.0 + second)
}
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").to_owned()
}
fn word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}
fn token(s: &str, needle: &str) -> bool {
    !needle.is_empty()
        && s.match_indices(needle).any(|(i, _)| {
            (i == 0 || !word(s.as_bytes()[i - 1]))
                && (i + needle.len() == s.len() || !word(s.as_bytes()[i + needle.len()]))
        })
}
fn string_at(s: &str, i: usize) -> Option<(String, usize)> {
    let b = s.as_bytes();
    let quote = *b.get(i)?;
    if ![b'\'', b'"', b'`'].contains(&quote) {
        return None;
    }
    let mut j = i + 1;
    let mut out = vec![];
    while j < b.len() {
        if b[j] == quote {
            return Some((String::from_utf8_lossy(&out).into_owned(), j + 1));
        }
        if b[j] == b'\\' {
            j += 1;
            let c = *b.get(j)?;
            match c {
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'\n' => {}
                b'\\' | b'\'' | b'"' | b'`' => out.push(c),
                _ => {
                    out.push(b'\\');
                    out.push(c);
                }
            }
        } else {
            out.push(b[j]);
        }
        j += 1;
    }
    None
}
fn commands(s: &str) -> (Vec<String>, usize) {
    if let Ok(v) = serde_json::from_str::<Value>(s)
        && let Some(c) = v["cmd"].as_str()
    {
        return (vec![c.into()], 0);
    }
    let b = s.as_bytes();
    let mut i = 0;
    let mut out = vec![];
    let mut unresolved = 0;
    while i < b.len() {
        if s.get(i..).is_some_and(|v| v.starts_with("//")) {
            i += s[i..].find('\n').unwrap_or(s.len() - i);
            continue;
        }
        if s.get(i..).is_some_and(|v| v.starts_with("/*")) {
            i += s[i + 2..].find("*/").map(|n| n + 4).unwrap_or(s.len() - i);
            continue;
        }
        let key = if let Some((value, end)) = string_at(s, i) {
            i = end;
            value == "cmd"
        } else if s.get(i..).is_some_and(|v| v.starts_with("cmd"))
            && (i == 0 || !word(b[i - 1]))
            && b.get(i + 3).is_none_or(|v| !word(*v))
        {
            i += 3;
            true
        } else {
            i += 1;
            false
        };
        if !key {
            continue;
        }
        while b.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if b.get(i) != Some(&b':') {
            unresolved += 1;
            continue;
        }
        i += 1;
        while b.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if let Some((value, end)) = string_at(s, i) {
            out.push(value);
            i = end;
        } else {
            unresolved += 1;
        }
    }
    (out, unresolved)
}
fn python_bodies(command: &str) -> Vec<String> {
    let lines: Vec<_> = command.lines().collect();
    let mut out = vec![];
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.contains("python")
            && let Some((_, tail)) = line.split_once("<<")
        {
            let delimiter = tail
                .trim_start_matches('-')
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(['\'', '"']);
            if !delimiter.is_empty() {
                let start = i + 1;
                i = start;
                while i < lines.len() && lines[i].trim() != delimiter {
                    i += 1;
                }
                if i < lines.len() {
                    out.push(lines[start..i].join("\n"));
                }
            }
        }
        i += 1;
    }
    out
}
// Decode structured JSON nested in a tool's output wrapper. Bounds prevent
// treating arbitrary long tool output as an unlimited recursive document.
fn acknowledged(v: &Value, ids: &mut BTreeSet<String>, depth: usize) {
    if depth > 12 {
        return;
    }
    match v {
        Value::Object(o) => {
            if o.get("delivery").and_then(Value::as_str) == Some("acknowledged")
                && let Some(id) = o.get("receipt_id").and_then(Value::as_str)
            {
                ids.insert(id.into());
            }
            for v in o.values() {
                acknowledged(v, ids, depth + 1);
            }
        }
        Value::Array(a) => {
            for v in a {
                acknowledged(v, ids, depth + 1);
            }
        }
        Value::String(s) if s.len() <= 8 * 1024 * 1024 && s.contains("acknowledged") => {
            if let Ok(v) = serde_json::from_str::<Value>(s) {
                acknowledged(&v, ids, depth + 1);
                return;
            }
            let mut pos = 0;
            let mut attempts = 0;
            while pos < s.len() && attempts < 512 {
                let Some(n) = s[pos..].find('{') else { break };
                pos += n;
                attempts += 1;
                let mut stream = serde_json::Deserializer::from_str(&s[pos..]).into_iter::<Value>();
                if let Some(Ok(v)) = stream.next() {
                    pos += stream.byte_offset();
                    acknowledged(&v, ids, depth + 1);
                } else {
                    pos += 1;
                }
            }
        }
        _ => {}
    }
}
#[derive(Clone)]
struct Cell {
    root: String,
    line: usize,
    timestamp: f64,
    at: String,
    name: String,
    input: String,
    segment: usize,
    call: Option<usize>,
    commands: Vec<String>,
    unresolved: usize,
    python: Vec<String>,
    output_at: Option<f64>,
    ack: BTreeSet<String>,
}
fn linked(cell: &Cell, op: &Value) -> bool {
    if cell.root != op["root_id"] {
        return false;
    }
    token(&cell.input, op["state_dir"].as_str().unwrap())
        || op["aliases"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .any(|a| token(&cell.input, a))
        || (cell.input.contains(op["run_root"].as_str().unwrap())
            && op["relative_aliases"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .any(|a| token(&cell.input, a)))
}
fn dispatch(cell: &Cell) -> bool {
    cell.commands.iter().any(|s| {
        s.contains("work-engine-manual-host dispatch")
            || s.contains(" dispatch --draft ")
            || s.contains("\"dispatch\"")
            || s.contains("'dispatch'")
    })
}
fn state_inspection(s: &str) -> bool {
    (s.contains("state.json")
        && ["read_text", "read_bytes", "json.load", "open("]
            .iter()
            .any(|k| s.contains(k)))
        || (s.contains("work-engine-manual-host")
            && (s.contains("'inspect'") || s.contains("\"inspect\"")))
}
fn numeric_sum<'a>(values: impl Iterator<Item = &'a Value>, key: &str) -> Value {
    let mut total = 0.0;
    let mut missing = 0;
    for v in values {
        if let Some(x) = v[key].as_f64() {
            total += x;
        } else {
            missing += 1;
        }
    }
    json!({"known_sum":total,"missing":missing})
}
fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    Some(if n.is_multiple_of(2) {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    } else {
        values[n / 2]
    })
}
fn aggregate(rows: &[&Value]) -> Value {
    let mut cohort = json!({"reviews":rows.len()});
    for field in [
        "root_window_calls",
        "root_linked_calls",
        "linked_inline_python",
        "linked_python_excluding_dispatch_cell",
        "linked_calls_excluding_dispatch_call",
        "linked_state_inspection_programs",
        "linked_input_chars",
        "linked_python_chars",
        "root_window_latency_proxy_clipped_s",
        "window_seconds",
        "native_duration_seconds",
        "median_root_input_tokens",
    ] {
        cohort[format!("median_{field}")] = json!(median(
            rows.iter().filter_map(|r| r[field].as_f64()).collect()
        ));
        cohort[format!("sum_{field}")] =
            json!(rows.iter().filter_map(|r| r[field].as_f64()).sum::<f64>());
    }
    cohort["median_root_window_cost"] = json!(median(
        rows.iter()
            .filter_map(|r| r["root_window_cost"]["known_sum"].as_f64())
            .collect()
    ));
    cohort["median_root_linked_cost"] = json!(median(
        rows.iter()
            .filter_map(|r| r["root_linked_call_cost"]["known_sum"].as_f64())
            .collect()
    ));
    cohort
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        return Err("usage: efficiency_report INPUT.json OUTPUT.json".into());
    }
    let input: Value = serde_json::from_reader(fs::File::open(&args[1])?)?;
    let calls: Vec<Value> = BufReader::new(fs::File::open(text(&input, "calls"))?)
        .lines()
        .map(|s| Ok(serde_json::from_str(&s?)?))
        .collect::<Result<_, Box<dyn std::error::Error>>>()?;
    let mut cells: Vec<Cell> = vec![];
    let mut call_segment = HashMap::new();
    let mut source_hashes = vec![];
    for root in input["roots"].as_array().ok_or("roots missing")? {
        let sid = text(root, "session_id");
        let path = text(root, "snapshot");
        let mut index = HashMap::new();
        for (i, c) in calls.iter().enumerate().filter(|(_, c)| c["thread"] == sid) {
            index.insert(
                c["source"]
                    .as_str()
                    .ok_or("call source missing")?
                    .rsplit(':')
                    .next()
                    .ok_or("source line missing")?
                    .parse::<usize>()?,
                i,
            );
        }
        let mut segment = 0;
        let mut ids = HashMap::new();
        let mut hasher = Sha256::new();
        for (i, raw) in BufReader::new(fs::File::open(&path)?)
            .split(b'\n')
            .enumerate()
        {
            let mut raw = raw?;
            if raw.is_empty() {
                continue;
            }
            raw.push(b'\n');
            hasher.update(&raw);
            let v: Value = serde_json::from_slice(&raw)?;
            let p = &v["payload"];
            let typ = text(&v, "type");
            let pt = text(p, "type");
            let incoming = typ == "response_item"
                && (matches!(
                    pt.as_str(),
                    "function_call_output" | "custom_tool_call_output"
                ) || (pt == "message" && p["role"] == "user"))
                || typ == "event_msg" && pt == "task_started";
            if incoming {
                segment += 1;
            }
            if let Some(c) = index.get(&(i + 1)) {
                call_segment.insert(*c, (sid.clone(), segment));
            }
            if typ == "response_item" && matches!(pt.as_str(), "function_call" | "custom_tool_call")
            {
                let body = p
                    .get("arguments")
                    .or_else(|| p.get("input"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                let (commands, unresolved) = commands(&body);
                let python = commands.iter().flat_map(|s| python_bodies(s)).collect();
                ids.insert(text(p, "call_id"), cells.len());
                cells.push(Cell {
                    root: sid.clone(),
                    line: i + 1,
                    timestamp: ts(v["timestamp"].as_str().unwrap_or(""))
                        .ok_or("tool timestamp invalid")?,
                    at: text(&v, "timestamp"),
                    name: text(p, "name"),
                    input: body,
                    segment,
                    call: None,
                    commands,
                    unresolved,
                    python,
                    output_at: None,
                    ack: BTreeSet::new(),
                });
            }
            if typ == "response_item"
                && matches!(
                    pt.as_str(),
                    "function_call_output" | "custom_tool_call_output"
                )
                && let Some(c) = ids.get(&text(p, "call_id"))
            {
                cells[*c].output_at = ts(v["timestamp"].as_str().unwrap_or(""));
                acknowledged(p, &mut cells[*c].ack, 0);
            }
        }
        let actual = format!("{:x}", hasher.finalize());
        if actual != root["prefix_sha256"] {
            return Err(format!("snapshot changed: {path}").into());
        }
        source_hashes.push(json!({"session_id":sid,"sha256":actual}));
    }
    let mut segments: HashMap<(String, usize), Vec<usize>> = HashMap::new();
    for (index, key) in &call_segment {
        segments.entry(key.clone()).or_default().push(*index);
    }
    for cell in &mut cells {
        if let Some(indices) = segments.get(&(cell.root.clone(), cell.segment))
            && indices.len() == 1
        {
            cell.call = Some(indices[0]);
        }
    }
    let mut reports = vec![];
    let mut verified_states = BTreeMap::new();
    for op in input["operations"].as_array().ok_or("operations missing")? {
        let path = text(op, "state_snapshot");
        let actual = format!("{:x}", Sha256::digest(fs::read(&path)?));
        if actual != op["state_sha256"] {
            return Err(format!("state snapshot changed: {path}").into());
        }
        verified_states.insert(path, actual);
        if !text(op, "operation_id").contains("review")
            || op["retained"] == true
            || op["native_session_id"].is_null()
            || op["phase"] != "closed"
        {
            continue;
        }
        let Some(started) = op["started_at"].as_f64() else {
            continue;
        };
        let Some(end) = op["first_closed"]["mtime"].as_str().and_then(ts) else {
            continue;
        };
        let first_receipt = op["first_receipt"]["mtime"]
            .as_str()
            .and_then(ts)
            .unwrap_or(started);
        let candidates: Vec<_> = cells
            .iter()
            .filter(|c| {
                c.timestamp >= started - 900.0
                    && c.timestamp <= first_receipt + 1.0
                    && linked(c, op)
                    && dispatch(c)
            })
            .collect();
        let first = candidates
            .iter()
            .min_by(|a, b| a.timestamp.total_cmp(&b.timestamp));
        let annotated = op["dispatch_cell_line"]
            .as_u64()
            .map(|n| {
                cells
                    .iter()
                    .find(|c| c.root == op["root_id"] && c.line == n as usize)
                    .ok_or("annotated dispatch cell missing")
            })
            .transpose()?;
        let first = annotated.or(first.copied());
        if let Some(c) = annotated
            && op["dispatch_cell_sha256"] != hash(&c.input)
        {
            return Err("annotated dispatch cell hash mismatch".into());
        }
        let start = first.map(|c| c.timestamp).unwrap_or(started);
        let window_calls: Vec<_> = calls
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c["thread"] == op["root_id"]
                    && c["ts"]
                        .as_str()
                        .and_then(ts)
                        .is_some_and(|t| t >= start - 0.5 && t <= end)
            })
            .collect();
        let window_cells: Vec<_> = cells
            .iter()
            .filter(|c| c.root == op["root_id"] && c.timestamp >= start - 0.5 && c.timestamp <= end)
            .collect();
        let relevant: Vec<_> = window_cells
            .iter()
            .copied()
            .filter(|c| linked(c, op))
            .collect();
        let relevant_calls: BTreeSet<_> = relevant.iter().filter_map(|c| c.call).collect();
        let programs: Vec<_> = relevant
            .iter()
            .flat_map(|c| c.python.iter().map(move |p| (*c, p)))
            .collect();
        let first_ack = cells
            .iter()
            .filter(|c| {
                c.root == op["root_id"]
                    && c.output_at.is_some_and(|t| t >= end - 1.0)
                    && c.ack
                        .contains(op["first_closed"]["id"].as_str().unwrap_or(""))
            })
            .min_by(|a, b| a.output_at.unwrap().total_cmp(&b.output_at.unwrap()));
        let latency_clipped: f64 = window_calls
            .iter()
            .filter_map(|(_, c)| {
                let latency = c["latency_s"].as_f64()?;
                let t = ts(c["ts"].as_str()?)?;
                Some((t - (t - latency).max(start)).max(0.0))
            })
            .sum();
        let details:Vec<_>=relevant.iter().map(|c|json!({"root":c.root,"line":c.line,"at":c.at,"tool":c.name,"input_chars":c.input.chars().count(),"input_sha256":hash(&c.input),"call_id":c.call.map(|i|calls[i]["call_id"].clone()),"commands":c.commands.len(),"unresolved_cmd_values":c.unresolved,"inline_python":c.python.iter().map(|s|json!({"sha256":hash(s),"chars":s.chars().count(),"host_state_inspection":state_inspection(s),"thread_resume_call":s.contains(".call(\"thread/resume\"") || s.contains(".call('thread/resume'")})).collect::<Vec<_>>()})).collect();
        let row = json!({"id":op["id"],"version":op["version"],"root":op["root_id"],"state":op["state_source"],"latest_receipt_acked":op["acked"],"captured":!op["collection"].is_null() && op["collection"]["result"].is_object(),"start_basis":if first.is_some(){"first linked dispatch/preparation tool cell within 15 minutes before native start"}else{"native started_at; dispatch boundary not found"},"start_tool_line":first.map(|c|c.line),"start_at":first.map(|c|c.at.clone()),"start_epoch":start,"first_close_epoch":end,"first_closed_receipt":op["first_closed"],"first_ack_tool_line":first_ack.map(|c|c.line),"first_ack_output_epoch":first_ack.and_then(|c|c.output_at),"window_seconds":end-start,"native_duration_seconds":op["native_turn"]["durationMs"].as_f64().map(|n|n/1000.0),"root_window_calls":window_calls.len(),"root_window_cost":numeric_sum(window_calls.iter().map(|(_,c)|*c),"speed_adjusted_api_usd_high"),"root_window_latency_proxy":numeric_sum(window_calls.iter().map(|(_,c)|*c),"latency_s"),"root_window_latency_proxy_clipped_s":latency_clipped,"root_linked_calls":relevant_calls.len(),"root_linked_call_cost":numeric_sum(relevant_calls.iter().map(|i|&calls[*i]),"speed_adjusted_api_usd_high"),"root_window_tool_cells":window_cells.len(),"root_linked_tool_cells":relevant.len(),"linked_unmapped_cells":relevant.iter().filter(|c|c.call.is_none()).count(),"linked_input_chars":relevant.iter().map(|c|c.input.chars().count()).sum::<usize>(),"linked_large_cells":relevant.iter().filter(|c|c.input.chars().count()>=1500).count(),"linked_inline_python":programs.len(),"linked_unique_python":programs.iter().map(|(_,p)|hash(p)).collect::<BTreeSet<_>>().len(),"linked_python_chars":programs.iter().map(|(_,p)|p.chars().count()).sum::<usize>(),"linked_state_inspection_programs":details.iter().flat_map(|d|d["inline_python"].as_array().unwrap()).filter(|d|d["host_state_inspection"]==true).count(),"linked_resume_programs":details.iter().flat_map(|d|d["inline_python"].as_array().unwrap()).filter(|d|d["thread_resume_call"]==true).count(),"linked_unresolved_cmd_values":relevant.iter().map(|c|c.unresolved).sum::<usize>(),"root_models":window_calls.iter().map(|(_,c)|format!("{}/{}/{}",text(c,"model"),text(c,"effort"),text(c,"service_tier"))).collect::<BTreeSet<_>>(),"median_root_input_tokens":median(window_calls.iter().filter_map(|(_,c)|c["input_context_tokens"].as_f64()).collect()),"tool_cell_audit":details});
        let mut row = row;
        row["review_kind"] = op["review_kind"].clone();
        row["closure_disposition"] = op["closure"]["disposition"].clone();
        row["dispatch_annotation"] = op["dispatch_annotation"].clone();
        row["dispatch_cell_sha256"] = first.map(|c| json!(hash(&c.input))).unwrap_or(Value::Null);
        if annotated.is_some() {
            row["start_basis"] = json!("manually verified operation-specific wrapper invocation");
        }
        row["linked_python_excluding_dispatch_cell"] = json!(
            programs
                .iter()
                .filter(|(c, _)| first.is_none_or(|f| c.line != f.line))
                .count()
        );
        row["linked_calls_excluding_dispatch_call"] = json!(
            relevant_calls
                .iter()
                .filter(|i| first.and_then(|f| f.call) != Some(**i))
                .count()
        );
        row["linked_cost_excluding_dispatch_call"] = numeric_sum(
            relevant_calls
                .iter()
                .filter(|i| first.and_then(|f| f.call) != Some(**i))
                .map(|i| &calls[*i]),
            "speed_adjusted_api_usd_high",
        );
        row["root_window_call_audit"] = json!(
            window_calls
                .iter()
                .map(|(_, c)| {
                    let mut selected = serde_json::Map::new();
                    for key in [
                        "call_id",
                        "source",
                        "ts",
                        "model",
                        "effort",
                        "service_tier",
                        "input_context_tokens",
                        "usage",
                        "api_long_context_input",
                        "pricing_scenario_id",
                        "latency_s",
                        "latency_basis",
                        "speed_adjusted_api_usd_high",
                        "speed_adjusted_api_usd_low",
                    ] {
                        selected.insert(key.into(), c[key].clone());
                    }
                    Value::Object(selected)
                })
                .collect::<Vec<_>>()
        );
        reports.push(row);
    }
    let mut cohorts = BTreeMap::new();
    for key in reports
        .iter()
        .map(|r| text(r, "version"))
        .collect::<BTreeSet<_>>()
    {
        let rows: Vec<_> = reports.iter().filter(|r| r["version"] == key).collect();
        cohorts.insert(key, aggregate(&rows));
    }
    let mut strata = BTreeMap::new();
    for root in reports
        .iter()
        .map(|r| text(r, "root"))
        .collect::<BTreeSet<_>>()
    {
        for version in reports
            .iter()
            .map(|r| text(r, "version"))
            .collect::<BTreeSet<_>>()
        {
            let rows: Vec<_> = reports
                .iter()
                .filter(|r| r["root"] == root && r["version"] == version)
                .collect();
            if !rows.is_empty() {
                strata.insert(format!("root/{root}/{version}"), aggregate(&rows));
            }
        }
    }
    for version in reports
        .iter()
        .map(|r| text(r, "version"))
        .collect::<BTreeSet<_>>()
    {
        for (name, kind) in [
            ("code_only", "instruction_plan"),
            ("non_delta_code", "code_review"),
        ] {
            let rows: Vec<_> = reports
                .iter()
                .filter(|r| {
                    r["version"] == version
                        && if name == "code_only" {
                            r["review_kind"] != kind
                        } else {
                            r["review_kind"] == kind
                        }
                })
                .collect();
            if !rows.is_empty() {
                strata.insert(format!("{name}/{version}"), aggregate(&rows));
            }
        }
    }
    let inventory: Vec<_> = input["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|op| {
            let mut selected = serde_json::Map::new();
            for key in [
                "id",
                "version",
                "binary_sha256",
                "root_id",
                "phase",
                "outcome",
                "retained",
                "native_session_id",
                "native_turn_id",
                "started_at",
                "first_closed",
                "acked",
                "state_source",
                "state_snapshot",
                "state_sha256",
            ] {
                selected.insert(key.into(), op[key].clone());
            }
            selected.insert(
                "closure_disposition".into(),
                op["closure"]["disposition"].clone(),
            );
            selected.insert(
                "included_review".into(),
                json!(reports.iter().any(|r| r["id"] == op["id"])),
            );
            Value::Object(selected)
        })
        .collect();
    let output = json!({"schema":"manual-host.efficiency-observation.v1","analyzer_sha256":hash(include_str!("efficiency_report.rs")),"cutoff":input["cutoff"],"input_sha256":format!("{:x}",Sha256::digest(fs::read(&args[1])?)),"calls_sha256":format!("{:x}",Sha256::digest(fs::read(text(&input,"calls"))?)),"source_reader":input["source_reader"],"roots":input["roots"],"verified_rollouts":source_hashes,"verified_states":verified_states,"normalized_calls":calls.len(),"tool_cells":cells.len(),"tool_cells_with_single_call_mapping":cells.iter().filter(|c|c.call.is_some()).count(),"operation_inventory":inventory,"reviews":reports,"cohorts":cohorts,"strata":strata,"limitations":["Descriptive unequal workloads, no randomized or matched current manual arm","Root window cost includes judgment and unrelated/concurrent activity; linked-call cost is association, not isolated bookkeeping","Time is a recorded response-latency proxy, not isolated active thinking time","Closed task window excludes final ACK, objective-end accounting and later reporting tails","Inline Python counts cover decoded cmd literals with Python heredocs; unresolved values are reported, not assumed empty","State-file receipt mtimes establish local persistence, not receipt delivery","Packet authorship before dispatch is excluded unless batched into the dispatch cell; comparisons excluding that cell are also necessary"]});
    fs::write(Path::new(&args[2]), serde_json::to_vec_pretty(&output)?)?;
    println!(
        "Wrote {} review observations to {}",
        output["reviews"].as_array().unwrap().len(),
        args[2]
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timestamps_preserve_utc_and_fraction() {
        assert_eq!(ts("1970-01-01T00:00:00Z"), Some(0.0));
        assert_eq!(
            ts("2026-11-03T00:00:00.125Z").unwrap() - ts("2026-11-02T00:00:00Z").unwrap(),
            86400.125
        );
    }
    #[test]
    fn nested_strings_are_not_extra_commands() {
        let (c, u) = commands(
            r#"text(await tools.exec_command({cmd:"python3 - <<'PY'\nprint({'cmd':'not another invocation'})\nPY"}))"#,
        );
        assert_eq!(c.len(), 1);
        assert_eq!(u, 0);
        assert_eq!(python_bodies(&c[0]).len(), 1);
    }
    #[test]
    fn unresolved_commands_do_not_become_zero_programs_silently() {
        let (c, u) = commands("tools.exec_command({cmd: generated})");
        assert!(c.is_empty());
        assert_eq!(u, 1);
    }
    #[test]
    fn state_inspection_includes_the_old_inspect_wrapper() {
        assert!(state_inspection(
            "pin='path/work-engine-manual-host'; subprocess.check_output([pin,'inspect','--state',path])"
        ));
        assert!(state_inspection(
            "json.loads(Path('state.json').read_text())"
        ));
        assert!(!state_inspection(
            "json.loads(Path('result.json').read_text())"
        ));
    }
    #[test]
    fn state_path_prefix_does_not_match_a_different_executor() {
        assert!(token(
            "--state /run/review-host/state.json",
            "/run/review-host"
        ));
        assert!(!token(
            "--state /run/review-host-b03/state.json",
            "/run/review-host"
        ));
    }
    #[test]
    fn ack_proof_is_receipt_specific() {
        let mut ids = BTreeSet::new();
        acknowledged(
            &json!({"output":"{\"receipt_id\":\"A\",\"delivery\":\"pending\"}\n{\"receipt_id\":\"B\",\"delivery\":\"acknowledged\"}"}),
            &mut ids,
            0,
        );
        assert_eq!(ids, BTreeSet::from(["B".into()]));
    }
}
