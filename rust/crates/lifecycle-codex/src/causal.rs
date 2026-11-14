//! Bounded ordinary trace acquisition. Explicit runtime edges, never log timing.
use super::compiled_input::{NativeInputError, fields, open_child, read_child_bounded, sha};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
};

const TRACE_LIMIT: usize = 1024 * 1024;
const COMBINED_LIMIT: usize = 8 * 1024 * 1024;
const METADATA_LIMIT: usize = 128 * 1024;
type Result<T> = std::result::Result<T, NativeInputError>;
#[derive(Debug, Clone, Serialize)]
pub struct CausalMember {
    pub path: String,
    pub kind: String,
    pub sha256: String,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, Serialize)]
pub struct CausalCall {
    pub call_id: String,
    pub inference_call_id: String,
    pub physical_send: u64,
    pub response_id: String,
    pub root_call_id: String,
    pub runtime_cell_id: Option<String>,
    pub invocation_path: String,
    pub response_path: String,
    pub namespace: String,
    pub tool: String,
    pub arguments: Value,
}
/// Private construction, no Deserialize: acquired only from the harness's pinned root.
#[derive(Debug, Clone, Serialize)]
pub struct AcquiredNativeCausal {
    pub(crate) manifest: Vec<u8>,
    pub(crate) trace: Vec<u8>,
    pub(crate) members: Vec<CausalMember>,
    pub(crate) calls: Vec<CausalCall>,
}
impl AcquiredNativeCausal {
    pub fn manifest(&self) -> &[u8] {
        &self.manifest
    }
    pub fn trace(&self) -> &[u8] {
        &self.trace
    }
    pub fn members(&self) -> &[CausalMember] {
        &self.members
    }
    pub fn calls(&self) -> &[CausalCall] {
        &self.calls
    }
}
// Count derived metadata separately from retained producer bytes. Stream into a
// bounded counter so rejection itself cannot allocate a serialized oversized copy.
fn metadata_len(members: &[CausalMember], calls: &[CausalCall]) -> Result<usize> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(bytes.len())
                .filter(|n| *n <= METADATA_LIMIT)
                .ok_or_else(|| std::io::Error::other("causal metadata capacity"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let descriptors = members
        .iter()
        .map(|m| (&m.path, &m.kind, &m.sha256, m.bytes.len()))
        .collect::<Vec<_>>();
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, &(descriptors, calls)).map_err(|_| NativeInputError)?;
    Ok(counter.0)
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 4096)
        .ok_or(NativeInputError)
}
fn json(bytes: &[u8]) -> Result<Value> {
    lifecycle_wire::parse_strict_json(bytes).map_err(|_| NativeInputError)
}
fn one<'a>(mut values: impl Iterator<Item = &'a Value>) -> Result<&'a Value> {
    let first = values.next().ok_or(NativeInputError)?;
    if values.next().is_some() {
        return Err(NativeInputError);
    }
    Ok(first)
}
fn scoped(v: &Value, thread: &str, turn: &str) -> bool {
    v["thread_id"] == thread && v["codex_turn_id"] == turn
}
fn payload<'a>(
    members: &'a BTreeMap<String, Value>,
    reference: &Value,
    kind: &str,
) -> Result<(&'a Value, String)> {
    fields(reference, &["raw_payload_id", "kind", "path"])?;
    text(reference, "raw_payload_id")?;
    fields(&reference["kind"], &["type"])?;
    if reference["kind"]["type"] != kind {
        return Err(NativeInputError);
    }
    let path = text(reference, "path")?.to_owned();
    Ok((members.get(&path).ok_or(NativeInputError)?, path))
}

pub(crate) fn acquire(
    dir: &File,
    strict: &[u8],
    input_size: usize,
    thread: &str,
    turn: &str,
) -> Result<AcquiredNativeCausal> {
    let manifest = read_child_bounded(dir, "manifest.json", 128 * 1024)?;
    let trace = read_child_bounded(dir, "trace.jsonl", TRACE_LIMIT)?;
    let mut remaining = COMBINED_LIMIT
        .checked_sub(input_size + strict.len() + manifest.len() + trace.len())
        .ok_or(NativeInputError)?;
    let m = json(&manifest)?;
    fields(
        &m,
        &[
            "schema_version",
            "trace_id",
            "rollout_id",
            "root_thread_id",
            "started_at_unix_ms",
            "raw_event_log",
            "payloads_dir",
        ],
    )?;
    if m["schema_version"] != 1
        || m["root_thread_id"] != thread
        || m["raw_event_log"] != "trace.jsonl"
        || m["payloads_dir"] != "payloads"
        || m["started_at_unix_ms"].as_i64().is_none()
        || !trace.ends_with(b"\n")
    {
        return Err(NativeInputError);
    }
    text(&m, "trace_id")?;
    text(&m, "rollout_id")?;
    let mut events = Vec::new();
    let mut identities = BTreeSet::new();
    for (i, line) in trace[..trace.len() - 1].split(|b| *b == b'\n').enumerate() {
        if i >= 4096 {
            return Err(NativeInputError);
        }
        let event = json(line)?;
        fields(
            &event,
            &[
                "schema_version",
                "seq",
                "wall_time_unix_ms",
                "rollout_id",
                "thread_id",
                "codex_turn_id",
                "payload",
            ],
        )?;
        if event["schema_version"] != 1
            || event["seq"].as_u64() != Some(i as u64 + 1)
            || event["rollout_id"] != m["rollout_id"]
            || event["wall_time_unix_ms"].as_i64().is_none()
        {
            return Err(NativeInputError);
        }
        for k in ["thread_id", "codex_turn_id"] {
            if !event[k].is_null() {
                text(&event, k)?;
            }
        }
        let p = &event["payload"];
        let kind = text(p, "type")?;
        // Unknown raw variants cannot silently hide another causal root.
        if !matches!(
            kind,
            "rollout_started"
                | "rollout_ended"
                | "thread_started"
                | "thread_ended"
                | "codex_turn_started"
                | "codex_turn_ended"
                | "inference_started"
                | "inference_completed"
                | "inference_failed"
                | "inference_cancelled"
                | "tool_call_started"
                | "tool_call_ended"
                | "tool_call_runtime_started"
                | "tool_call_runtime_ended"
                | "code_cell_started"
                | "code_cell_initial_response"
                | "code_cell_ended"
                | "mcp_tool_call_correlation_assigned"
                | "compaction_request_started"
                | "compaction_request_completed"
                | "compaction_request_failed"
                | "compaction_installed"
                | "agent_result_observed"
                | "protocol_event_observed"
                | "other"
        ) {
            return Err(NativeInputError);
        }
        let identity_key = match kind {
            "inference_started"
            | "inference_completed"
            | "inference_failed"
            | "inference_cancelled" => Some((kind, "inference_call_id")),
            "tool_call_started" | "tool_call_ended" => Some((kind, "tool_call_id")),
            "code_cell_started" | "code_cell_ended" => Some((kind, "runtime_cell_id")),
            _ => None,
        };
        if let Some((kind, key)) = identity_key {
            if !identities.insert((kind.to_owned(), text(p, key)?.to_owned())) {
                return Err(NativeInputError);
            }
        }
        events.push(event);
    }
    let first = events.first().ok_or(NativeInputError)?;
    if first["payload"]["type"] != "rollout_started"
        || first["payload"]["trace_id"] != m["trace_id"]
        || first["payload"]["root_thread_id"] != thread
    {
        return Err(NativeInputError);
    }
    let end = one(events
        .iter()
        .filter(|e| scoped(e, thread, turn) && e["payload"]["type"] == "codex_turn_ended"))?;
    if end["payload"]["codex_turn_id"] != turn || end["payload"]["status"] != "completed" {
        return Err(NativeInputError);
    }
    let mut refs = BTreeMap::new();
    // Retain every selected turn response and invocation, including failed partials.
    for e in events.iter().filter(|e| scoped(e, thread, turn)) {
        let p = &e["payload"];
        for key in [
            "response_payload",
            "partial_response_payload",
            "invocation_payload",
        ] {
            if let Some(r) = p.get(key).filter(|v| !v.is_null()) {
                fields(r, &["raw_payload_id", "kind", "path"])?;
                let path = text(r, "path")?.to_owned();
                let kind = text(&r["kind"], "type")?.to_owned();
                if !matches!(kind.as_str(), "inference_response" | "tool_invocation") {
                    continue;
                }
                if refs.insert(path, (kind, r.clone())).is_some() {
                    return Err(NativeInputError);
                }
            }
        }
    }
    if refs.len() + 2 > 128 {
        return Err(NativeInputError);
    }
    let payload_dir = open_child(dir, "payloads", true)?;
    let mut members = Vec::new();
    let mut values = BTreeMap::new();
    let mut raw_ids = BTreeSet::new();
    for (path, (kind, r)) in refs {
        if !raw_ids.insert(text(&r, "raw_payload_id")?.to_owned()) {
            return Err(NativeInputError);
        }
        let name = path.strip_prefix("payloads/").ok_or(NativeInputError)?;
        let bytes = read_child_bounded(&payload_dir, name, remaining)?;
        remaining = remaining.checked_sub(bytes.len()).ok_or(NativeInputError)?;
        values.insert(path.clone(), json(&bytes)?);
        members.push(CausalMember {
            path,
            kind,
            sha256: sha(&bytes),
            bytes,
        });
    }
    let strict_rows = strict
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .map(json)
        .collect::<Result<Vec<_>>>()?;
    let mut calls = Vec::new();
    for event in events
        .iter()
        .filter(|e| scoped(e, thread, turn) && e["payload"]["type"] == "tool_call_started")
    {
        let p = &event["payload"];
        let Some(reference) = p.get("invocation_payload").filter(|r| !r.is_null()) else {
            continue;
        };
        let (invocation, invocation_path) = payload(&values, reference, "tool_invocation")?;
        if invocation["tool_namespace"] != "task" || invocation["tool_name"] != "result_publish" {
            continue;
        }
        fields(invocation, &["tool_name", "tool_namespace", "payload"])?;
        if invocation["payload"]["type"] != "function" {
            return Err(NativeInputError);
        }
        let arguments = json(text(&invocation["payload"], "arguments")?.as_bytes())?;
        let call_id = text(p, "tool_call_id")?;
        let ended = one(events.iter().filter(|e| {
            scoped(e, thread, turn)
                && e["payload"]["type"] == "tool_call_ended"
                && e["payload"]["tool_call_id"] == call_id
        }))?;
        if ended["payload"]["status"] != "completed" || ended["payload"]["result_payload"].is_null()
        {
            return Err(NativeInputError);
        }
        let (root, cell) = match p["requester"]["type"].as_str() {
            Some("model") => {
                if p["model_visible_call_id"] != call_id
                    || !p["code_mode_runtime_tool_id"].is_null()
                {
                    return Err(NativeInputError);
                }
                (call_id.to_owned(), None)
            }
            Some("code_cell") => {
                if !p["model_visible_call_id"].is_null() {
                    return Err(NativeInputError);
                }
                text(p, "code_mode_runtime_tool_id")?;
                let id = text(&p["requester"], "runtime_cell_id")?;
                let cell = one(events.iter().filter(|e| {
                    e["payload"]["type"] == "code_cell_started"
                        && e["payload"]["runtime_cell_id"] == id
                }))?;
                if !scoped(cell, thread, turn) {
                    return Err(NativeInputError);
                }
                let end = one(events.iter().filter(|e| {
                    scoped(e, thread, turn)
                        && e["payload"]["type"] == "code_cell_ended"
                        && e["payload"]["runtime_cell_id"] == id
                }))?;
                if end["payload"]["status"] != "completed" {
                    return Err(NativeInputError);
                }
                (
                    text(&cell["payload"], "model_visible_call_id")?.to_owned(),
                    Some(id.to_owned()),
                )
            }
            _ => return Err(NativeInputError),
        };
        let mut roots = Vec::new();
        for response_event in events
            .iter()
            .filter(|e| scoped(e, thread, turn) && e["payload"]["type"] == "inference_completed")
        {
            let rp = &response_event["payload"];
            let (response, response_path) =
                payload(&values, &rp["response_payload"], "inference_response")?;
            if response["response_id"] != rp["response_id"] {
                return Err(NativeInputError);
            }
            for item in response["output_items"]
                .as_array()
                .ok_or(NativeInputError)?
            {
                if item["call_id"] != root {
                    continue;
                }
                if cell.is_some() {
                    if item["type"] != "custom_tool_call" || item["name"] != "exec" {
                        return Err(NativeInputError);
                    }
                } else if item["type"] != "function_call"
                    || item["name"] != "result_publish"
                    || item["namespace"] != "task"
                    || json(text(item, "arguments")?.as_bytes())? != arguments
                {
                    return Err(NativeInputError);
                }
                roots.push((rp, response_path.clone()));
            }
        }
        if roots.len() != 1 {
            return Err(NativeInputError);
        }
        let (rp, response_path) = roots.pop().unwrap();
        let inference = text(rp, "inference_call_id")?;
        let response_id = text(rp, "response_id")?;
        if events.iter().any(|e| {
            scoped(e, thread, turn)
                && e["payload"]["inference_call_id"] == inference
                && matches!(
                    e["payload"]["type"].as_str(),
                    Some("inference_failed" | "inference_cancelled")
                )
        }) {
            return Err(NativeInputError);
        }
        let started = one(events.iter().filter(|e| {
            scoped(e, thread, turn)
                && e["payload"]["type"] == "inference_started"
                && e["payload"]["inference_call_id"] == inference
        }))?;
        if started["payload"]["thread_id"] != thread || started["payload"]["codex_turn_id"] != turn
        {
            return Err(NativeInputError);
        }
        let completed = one(strict_rows.iter().filter(|r| {
            r["stage"] == "provider_completed"
                && r["inference_call_id"] == inference
                && r["response_id"] == response_id
        }))?;
        let physical_send = completed["physical_send"]
            .as_u64()
            .ok_or(NativeInputError)?;
        if strict_rows
            .iter()
            .filter(|r| r["stage"] == "before_send" && r["inference_call_id"] == inference)
            .filter_map(|r| r["physical_send"].as_u64())
            .max()
            != Some(physical_send)
        {
            return Err(NativeInputError);
        }
        calls.push(CausalCall {
            call_id: call_id.into(),
            inference_call_id: inference.into(),
            physical_send,
            response_id: response_id.into(),
            root_call_id: root,
            runtime_cell_id: cell,
            invocation_path,
            response_path,
            namespace: "task".into(),
            tool: "result_publish".into(),
            arguments,
        });
    }
    let derived = metadata_len(&members, &calls)?;
    remaining.checked_sub(derived).ok_or(NativeInputError)?;
    // Detect append/replacement during acquisition, including unchanged inode edits.
    if read_child_bounded(dir, "manifest.json", 128 * 1024)? != manifest
        || read_child_bounded(dir, "trace.jsonl", TRACE_LIMIT)? != trace
    {
        return Err(NativeInputError);
    }
    for member in &members {
        let name = member
            .path
            .strip_prefix("payloads/")
            .ok_or(NativeInputError)?;
        if read_child_bounded(&payload_dir, name, member.bytes.len())? != member.bytes {
            return Err(NativeInputError);
        }
    }
    Ok(AcquiredNativeCausal {
        manifest,
        trace,
        members,
        calls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};
    #[test]
    fn separate_derived_metadata_bound_excludes_raw_bytes_and_counts_arguments() {
        let members = vec![CausalMember {
            path: "payloads/response".into(),
            kind: "inference_response".into(),
            sha256: "a".repeat(64),
            bytes: vec![0; 512 * 1024],
        }];
        assert!(metadata_len(&members, &[]).unwrap() < 1024);
        let mut call = CausalCall {
            call_id: "call".into(),
            inference_call_id: "inference".into(),
            physical_send: 1,
            response_id: "response".into(),
            root_call_id: "call".into(),
            runtime_cell_id: None,
            invocation_path: "payloads/tool".into(),
            response_path: "payloads/response".into(),
            namespace: "task".into(),
            tool: "result_publish".into(),
            arguments: serde_json::json!({"body":"x".repeat(120 * 1024)}),
        };
        assert!(metadata_len(&members, &[call.clone()]).is_ok());
        call.arguments = serde_json::json!({"body":"x".repeat(METADATA_LIMIT)});
        assert!(metadata_len(&members, &[call]).is_err());
    }
    fn run_case(
        bundle: &Value,
        input: &Value,
        mutate: impl FnOnce(&mut Vec<Value>, &mut BTreeMap<String, Vec<u8>>),
    ) -> bool {
        run_case_sized(bundle, input, None, mutate)
    }
    fn run_case_sized(
        bundle: &Value,
        input: &Value,
        input_size: Option<usize>,
        mutate: impl FnOnce(&mut Vec<Value>, &mut BTreeMap<String, Vec<u8>>),
    ) -> bool {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("payloads")).unwrap();
        let manifest: Vec<u8> = serde_json::from_value(bundle["manifest"].clone()).unwrap();
        let trace: Vec<u8> = serde_json::from_value(bundle["trace"].clone()).unwrap();
        let mut events = trace
            .split(|b| *b == b'\n')
            .filter(|b| !b.is_empty())
            .map(|b| serde_json::from_slice(b).unwrap())
            .collect::<Vec<Value>>();
        let mut members = bundle["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| {
                (
                    m["path"].as_str().unwrap().to_owned(),
                    serde_json::from_value(m["bytes"].clone()).unwrap(),
                )
            })
            .collect::<BTreeMap<String, Vec<u8>>>();
        mutate(&mut events, &mut members);
        for (n, event) in events.iter_mut().enumerate() {
            event["seq"] = serde_json::json!(n + 1);
        }
        let mut trace = Vec::new();
        for event in events {
            serde_json::to_writer(&mut trace, &event).unwrap();
            trace.push(b'\n');
        }
        fs::write(root.path().join("manifest.json"), manifest).unwrap();
        fs::write(root.path().join("trace.jsonl"), trace).unwrap();
        for (path, bytes) in members {
            fs::write(root.path().join(path), bytes).unwrap();
        }
        let strict: Vec<u8> = serde_json::from_value(input["metadata"].clone()).unwrap();
        let size = input["payloads"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["bytes"].as_array().unwrap().len())
            .sum();
        acquire(
            &File::open(root.path()).unwrap(),
            &strict,
            input_size.unwrap_or(size),
            input["thread_id"].as_str().unwrap(),
            input["turn_id"].as_str().unwrap(),
        )
        .is_ok_and(|v| {
            v.calls
                .iter()
                .any(|c| c.call_id == bundle["calls"][0]["call_id"].as_str().unwrap())
        })
    }
    /// Capacity failures use a retained actual native baseline. Added records
    /// are explicit decoder controls, not claimed native production observations.
    #[test]
    #[ignore = "requires retained native causal and input artifact paths"]
    fn actual_native_causal_capacity_boundaries() {
        let record: Value =
            serde_json::from_slice(&fs::read(std::env::var("WE_CAUSAL_RECORD").unwrap()).unwrap())
                .unwrap();
        let bytes: Vec<u8> = serde_json::from_value(record["bundle"].clone()).unwrap();
        let bundle: Value = serde_json::from_slice(&bytes).unwrap();
        let input: Value =
            serde_json::from_slice(&fs::read(std::env::var("WE_CAUSAL_INPUT").unwrap()).unwrap())
                .unwrap();
        assert!(run_case(&bundle, &input, |_, _| {}));
        assert!(!run_case_sized(
            &bundle,
            &input,
            Some(COMBINED_LIMIT),
            |_, _| {}
        ));
        for (count, expected) in [(4096, true), (4097, false)] {
            assert_eq!(
                run_case(&bundle, &input, |events, _| {
                    let mut padding = events[0].clone();
                    padding["thread_id"] = Value::Null;
                    padding["codex_turn_id"] = Value::Null;
                    padding["payload"] = serde_json::json!({"type":"other"});
                    while events.len() < count {
                        events.push(padding.clone());
                    }
                }),
                expected,
                "event capacity {count}"
            );
        }
        for (length, expected) in [(TRACE_LIMIT, true), (TRACE_LIMIT + 1, false)] {
            assert_eq!(
                run_case(&bundle, &input, |events, _| {
                    let mut padding = events[0].clone();
                    padding["thread_id"] = Value::Null;
                    padding["codex_turn_id"] = Value::Null;
                    padding["payload"] = serde_json::json!({"type":"other", "padding":""});
                    events.push(padding);
                    for (n, event) in events.iter_mut().enumerate() {
                        event["seq"] = serde_json::json!(n + 1);
                    }
                    let used: usize = events
                        .iter()
                        .map(|e| serde_json::to_vec(e).unwrap().len() + 1)
                        .sum();
                    events.last_mut().unwrap()["payload"]["padding"] =
                        Value::String("x".repeat(length - used));
                }),
                expected,
                "trace capacity {length}"
            );
        }
        for (count, expected) in [(126, true), (127, false)] {
            assert_eq!(
                run_case(&bundle, &input, |events, members| {
                    let mut padding = events
                        .iter()
                        .find(|e| e["codex_turn_id"] == input["turn_id"])
                        .unwrap()
                        .clone();
                    while members.len() < count {
                        let n = members.len();
                        let path = format!("payloads/capacity-{n}");
                        members.insert(path.clone(), b"{}".to_vec());
                        padding["payload"] = serde_json::json!({"type":"other", "response_payload":{"raw_payload_id":format!("capacity-{n}"),"kind":{"type":"inference_response"},"path":path}});
                        events.push(padding.clone());
                    }
                }),
                expected,
                "member capacity {count}"
            );
        }
    }
    /// Decoder controls derived from retained real native output, not fabricated positive logs.
    #[test]
    #[ignore = "requires retained native causal and input artifact paths"]
    fn actual_native_causal_corruption_controls() {
        let record: Value =
            serde_json::from_slice(&fs::read(std::env::var("WE_CAUSAL_RECORD").unwrap()).unwrap())
                .unwrap();
        let bytes: Vec<u8> = serde_json::from_value(record["bundle"].clone()).unwrap();
        let bundle: Value = serde_json::from_slice(&bytes).unwrap();
        let input: Value = serde_json::from_slice(
            &fs::read(Path::new(&std::env::var("WE_CAUSAL_INPUT").unwrap())).unwrap(),
        )
        .unwrap();
        assert!(
            run_case(&bundle, &input, |_, _| {}),
            "actual retained baseline must qualify"
        );
        for case in [
            "wrong-response",
            "wrong-root",
            "duplicate-call",
            "foreign-turn",
            "failed-response",
            "cancelled-response",
            "missing-invocation",
            "missing-response",
            "failed-publication",
            "partial-trace",
            "foreign-rollout",
            "duplicate-response",
        ] {
            let accepted = run_case(&bundle, &input, |events, members| {
                let publication = events
                    .iter()
                    .position(|e| {
                        e["payload"]["type"] == "tool_call_started"
                            && e["payload"]["tool_call_id"] == record["call_id"]
                    })
                    .unwrap();
                let response = events
                    .iter()
                    .position(|e| e["payload"]["type"] == "inference_completed")
                    .unwrap();
                match case {
                    "wrong-response" => {
                        events[response]["payload"]["response_id"] =
                            serde_json::json!("foreign-response")
                    }
                    "wrong-root" => {
                        events[publication]["payload"]["tool_call_id"] =
                            serde_json::json!("foreign-call")
                    }
                    "duplicate-call" => events.push(events[publication].clone()),
                    "foreign-turn" => {
                        events[publication]["codex_turn_id"] = serde_json::json!("foreign-turn")
                    }
                    "failed-response" | "cancelled-response" => {
                        let mut e = events[response].clone();
                        e["payload"]["type"] = serde_json::json!(if case == "failed-response" {
                            "inference_failed"
                        } else {
                            "inference_cancelled"
                        });
                        events.push(e);
                    }
                    "missing-invocation" => {
                        let path = events[publication]["payload"]["invocation_payload"]["path"]
                            .as_str()
                            .unwrap();
                        members.remove(path);
                    }
                    "missing-response" => {
                        let path = events[response]["payload"]["response_payload"]["path"]
                            .as_str()
                            .unwrap();
                        members.remove(path);
                    }
                    "failed-publication" => {
                        let e = events
                            .iter_mut()
                            .find(|e| {
                                e["payload"]["type"] == "tool_call_ended"
                                    && e["payload"]["tool_call_id"] == record["call_id"]
                            })
                            .unwrap();
                        e["payload"]["status"] = serde_json::json!("failed");
                    }
                    "partial-trace" => {
                        events.retain(|e| e["payload"]["type"] != "codex_turn_ended")
                    }
                    "foreign-rollout" => {
                        events[publication]["rollout_id"] = serde_json::json!("foreign-rollout")
                    }
                    "duplicate-response" => events.push(events[response].clone()),
                    _ => unreachable!(),
                }
            });
            // A missing scoped call can yield an empty, nonqualifying companion;
            // every retained publication still requires one exact matching call.
            assert!(!accepted, "corruption unexpectedly acquired: {case}");
        }
        if bundle["calls"][0]["runtime_cell_id"].is_string() {
            for case in ["wrong-cell", "old-cell", "duplicate-cell", "failed-cell"] {
                assert!(
                    !run_case(&bundle, &input, |events, _| {
                        let i = events
                            .iter()
                            .position(|e| e["payload"]["type"] == "code_cell_started")
                            .unwrap();
                        match case {
                            "wrong-cell" => {
                                events[i]["payload"]["runtime_cell_id"] =
                                    serde_json::json!("foreign-cell")
                            }
                            "old-cell" => {
                                events[i]["codex_turn_id"] = serde_json::json!("prior-turn")
                            }
                            "duplicate-cell" => events.push(events[i].clone()),
                            "failed-cell" => {
                                let e = events
                                    .iter_mut()
                                    .find(|e| e["payload"]["type"] == "code_cell_ended")
                                    .unwrap();
                                e["payload"]["status"] = serde_json::json!("failed");
                            }
                            _ => unreachable!(),
                        }
                    }),
                    "cell corruption accepted: {case}"
                );
            }
        }
    }
}
