//! Checked native tool-registry evidence for one controlled turn.
//!
//! The expected tool set is derived from the exact `thread/start` `dynamicTools` value that
//! Work Engine sent, never from a second list. Every row is native-produced bytes read once
//! when the turn completes; a dispatch appended after that read is outside this check, so the
//! controlled fixture re-digests the file after the native process is waited.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read};

/// One row, including its newline. Observed snapshot rows are about 52 KB.
const ROW_LIMIT: usize = 1024 * 1024;
/// The bytes retained for one turn; rows of other turns are not retained or counted.
const RETAINED_LIMIT: usize = 2 * 1024 * 1024;
/// Rows of the current turn.
const ROW_COUNT_LIMIT: usize = 2048;
const CODE_MODE_NAMES: [&str; 2] = ["exec", "wait"];

/// Native-core utilities the private native build registers under the selected launch
/// profile although Work Engine did not request them. Root approved them by exact name,
/// exposure and spec digest (decision recorded in 003-build/CONTINUATION.md, 2026-11-17).
/// Digests are pinned from the strict snapshot of native binary
/// `77fbfa4bfd4d62f095b182a6606025345f84c126653a5713ec47388275734d39` and reproduced by
/// `testdata/native_utilities_77fbfa4b.json`. Tuple: namespace, name, exposure, nested
/// code-mode name (present only when code mode is hosted), SHA-256 of the serialized spec.
/// Any other extra tool, a missing one, another exposure or another digest is refused.
type ApprovedNativeUtility = (
    Option<&'static str>,
    &'static str,
    &'static str,
    Option<&'static str>,
    &'static str,
);
const APPROVED_NATIVE_UTILITIES: [ApprovedNativeUtility; 7] = [
    (
        None,
        "list_mcp_resources",
        "direct",
        Some("list_mcp_resources"),
        "5aef64ff99b28c2c096ea2dc4e5673502a4c291736d64132dc9e4e939f509f6e",
    ),
    (
        None,
        "list_mcp_resource_templates",
        "direct",
        Some("list_mcp_resource_templates"),
        "7f87264594875ab2a6f833c1b92164f6a75641ea1dcb2d2fab4facb35c3f7dfe",
    ),
    (
        None,
        "read_mcp_resource",
        "direct",
        Some("read_mcp_resource"),
        "23f2d43fece9ce15ecf4d9b3e283cd56d1f9dd14b5ee54455e9aca97a7accd9f",
    ),
    (
        None,
        "request_user_input",
        "direct_model_only",
        None,
        "d966b55e964da3155fb6956528a375b7cea02b3fc189086b7d9225f4d025fa6c",
    ),
    (
        None,
        "request_user_input_async",
        "direct_model_only",
        None,
        "1758e0ef7a3d190edbdad3fe9dadda9bd000d896a3eb2bd82f8dec5ff7cbef0a",
    ),
    (
        Some("clock"),
        "curr_time",
        "direct",
        Some("clock__curr_time"),
        "b556f2120a2088545bd8017ac712122cd877288d6b9403602829947228a1a14b",
    ),
    (
        Some("clock"),
        "sleep",
        "direct_model_only",
        None,
        "b91c24597c3acf3a1a35d85a764f631374f17c327ede58ce518808aff1e5c24d",
    ),
];

fn approved_utility(key: &ToolKey) -> Option<&'static ApprovedNativeUtility> {
    APPROVED_NATIVE_UTILITIES
        .iter()
        .find(|u| u.0 == key.0.as_deref() && u.1 == key.1)
}

type ToolKey = (Option<String>, String);

#[derive(Debug, Clone)]
struct ExpectedTool {
    key: ToolKey,
    description: String,
    input_schema: Value,
    deferred: bool,
    /// The sent namespace description, for a namespaced tool.
    namespace_description: Option<String>,
    /// The sent `outputSchema`, when present. Native never finalizes one.
    output_schema: Option<Value>,
}

/// The tool set Work Engine itself requested for one thread.
#[derive(Debug, Clone)]
pub struct ExpectedToolSet {
    tools: Vec<ExpectedTool>,
    code_mode_host: bool,
}

impl ExpectedToolSet {
    /// `dynamic_tools` is the exact `dynamicTools` value of the `thread/start` request.
    pub fn from_dynamic_tools(dynamic_tools: &Value, code_mode_host: bool) -> Option<Self> {
        let mut tools = Vec::new();
        for entry in dynamic_tools.as_array()? {
            match entry["type"].as_str()? {
                "namespace" => {
                    let namespace = entry["name"].as_str()?;
                    let description = match entry.get("description") {
                        Some(sent) => Some(sent.as_str()?),
                        None => None,
                    };
                    for tool in entry["tools"].as_array()? {
                        let mut function = expected_function(Some(namespace), tool)?;
                        function.namespace_description = description.map(str::to_owned);
                        tools.push(function);
                    }
                }
                "function" => tools.push(expected_function(None, entry)?),
                _ => return None,
            }
        }
        let keys = tools.iter().map(|t| &t.key).collect::<BTreeSet<_>>();
        (keys.len() == tools.len() && !tools.is_empty()).then_some(Self {
            tools,
            code_mode_host,
        })
    }

    fn keys(&self) -> BTreeSet<ToolKey> {
        let mut keys = self
            .tools
            .iter()
            .map(|t| t.key.clone())
            .collect::<BTreeSet<_>>();
        if self.code_mode_host {
            keys.extend(CODE_MODE_NAMES.map(|name| (None, name.to_owned())));
        }
        keys.extend(
            APPROVED_NATIVE_UTILITIES
                .iter()
                .map(|u| (u.0.map(str::to_owned), u.1.to_owned())),
        );
        keys
    }
}

fn expected_function(namespace: Option<&str>, tool: &Value) -> Option<ExpectedTool> {
    Some(ExpectedTool {
        key: (
            namespace.map(str::to_owned),
            tool["name"].as_str()?.to_owned(),
        ),
        description: tool["description"].as_str()?.to_owned(),
        input_schema: tool.get("inputSchema")?.clone(),
        deferred: tool["deferLoading"] == true,
        namespace_description: None,
        output_schema: tool.get("outputSchema").cloned(),
    })
}

/// One physical request's join evidence, taken from the strict transport rows.
#[derive(Debug, Clone)]
pub(crate) struct SendCitation {
    pub(crate) router_snapshot_id: String,
    /// The request's top-level `tools` field, when present.
    pub(crate) tools: Option<Value>,
    /// `tools` of each `additional_tools` input item: Responses Lite models carry their
    /// tool declarations there instead of in the top-level field.
    pub(crate) additional_tools: Vec<Value>,
}

/// Native registry rows for one turn, after every check below passed.
#[derive(Debug, Clone)]
pub struct CheckedToolRegistry {
    pub(crate) bytes: Vec<u8>,
    pub(crate) snapshots: usize,
    pub(crate) dispatches: usize,
}

impl CheckedToolRegistry {
    /// The exact retained row bytes, in file order, scoped to the turn.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn snapshots(&self) -> usize {
        self.snapshots
    }
    pub fn dispatches(&self) -> usize {
        self.dispatches
    }
}

type Refused = &'static str;
const UNAVAILABLE: Refused = "tool_registry_unavailable_or_malformed";
const EXTRA_OR_MISSING: Refused = "tool_registry_set_mismatch";
const MODE: Refused = "tool_registry_mode_mismatch";
const SPEC: Refused = "tool_registry_spec_mismatch";
const COLLISION: Refused = "tool_registry_collision";
const ROWS: Refused = "tool_registry_row_set_refused";
const DISPATCH: Refused = "tool_registry_dispatch_mismatch";
const JOIN: Refused = "tool_registry_request_join_mismatch";
const INCREMENTAL: Refused = "tool_registry_incremental_tools_unsupported";
const UNTRACED: Refused = "tool_registry_dispatch_missing";

fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}
fn text(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| !s.is_empty() && s.len() <= 4096)
}
fn key_of(v: &Value) -> Option<ToolKey> {
    let namespace = match &v["namespace"] {
        Value::Null => None,
        n => Some(text(n)?.to_owned()),
    };
    Some((namespace, text(&v["name"])?.to_owned()))
}
fn sha(value: &Value) -> Option<String> {
    Some(crate::compiled_input::sha(&serde_json::to_vec(value).ok()?))
}

struct Snapshot {
    keys: BTreeSet<ToolKey>,
    specs: Value,
    lite_specs: Value,
}

/// `ordinary_started` is the set of `tool_call_started` ids in the ordinary trace.
///
/// `file` is read as a stream of rows. Every row of the file must be a strict JSON row of a
/// known kind and at most `ROW_LIMIT` bytes, so a malformed file is refused wherever the
/// bad row sits. Only rows of this thread and turn are retained, counted and checked; the
/// size bound applies to those retained bytes, not to the file. Snapshot ids are scoped to
/// the turn's own rows, so rows of other turns take no part in the row-set rules.
pub(crate) fn check(
    file: impl Read,
    thread: &str,
    turn: &str,
    expected: &ExpectedToolSet,
    sends: &[SendCitation],
    ordinary_started: &BTreeSet<String>,
) -> Result<CheckedToolRegistry, Refused> {
    let wanted = expected.keys();
    let mut selected = Vec::new();
    let mut retained = Vec::new();
    let mut reader = BufReader::new(file);
    let mut span = Vec::new();
    let mut rows = 0usize;
    loop {
        span.clear();
        let read = (&mut reader)
            .take(ROW_LIMIT as u64 + 1)
            .read_until(b'\n', &mut span)
            .map_err(|_| UNAVAILABLE)?;
        if read == 0 {
            break;
        }
        // An oversized row and an unterminated final row both lack the newline here.
        let Some(line) = span.strip_suffix(b"\n") else {
            return Err(UNAVAILABLE);
        };
        rows += 1;
        let row = lifecycle_wire::parse_strict_json(line).map_err(|_| UNAVAILABLE)?;
        if !matches!(
            row["kind"].as_str(),
            Some("strict_router_snapshot" | "strict_tool_dispatch")
        ) {
            return Err(UNAVAILABLE);
        }
        if row["thread_id"] == thread && row["codex_turn_id"] == turn {
            if selected.len() >= ROW_COUNT_LIMIT || retained.len() + span.len() > RETAINED_LIMIT {
                return Err(UNAVAILABLE);
            }
            retained.extend_from_slice(&span);
            selected.push(row);
        }
    }
    if rows == 0 {
        return Err(UNAVAILABLE);
    }
    let mut snapshots: BTreeMap<String, Snapshot> = BTreeMap::new();
    let mut calls = BTreeSet::new();
    let mut dispatches = 0;
    for row in &selected {
        let id = text(&row["router_snapshot_id"]).ok_or(UNAVAILABLE)?;
        if row["kind"] == "strict_router_snapshot" {
            if !exact(
                row,
                &[
                    "kind",
                    "version",
                    "router_snapshot_id",
                    "thread_id",
                    "codex_turn_id",
                    "tool_mode",
                    "first_collision",
                    "registered_tools",
                    "code_mode_tools",
                    "model_visible_specs",
                    "model_visible_specs_sha256",
                    "model_visible_specs_responses_lite",
                    "model_visible_specs_responses_lite_sha256",
                ],
            ) || row["version"] != 1
            {
                return Err(UNAVAILABLE);
            }
            let snapshot = check_snapshot(row, expected, &wanted)?;
            if snapshots.insert(id.to_owned(), snapshot).is_some() {
                return Err(ROWS);
            }
            continue;
        }
        if !exact(
            row,
            &[
                "kind",
                "version",
                "router_snapshot_id",
                "thread_id",
                "codex_turn_id",
                "tool_call_id",
                "namespace",
                "name",
                "requester",
                "outcome",
            ],
        ) || row["version"] != 1
        {
            return Err(UNAVAILABLE);
        }
        // File order: the cited snapshot must already be published.
        let snapshot = snapshots.get(id).ok_or(DISPATCH)?;
        let call = text(&row["tool_call_id"]).ok_or(UNAVAILABLE)?;
        if !calls.insert(call.to_owned()) {
            return Err(ROWS);
        }
        let key = key_of(row).ok_or(UNAVAILABLE)?;
        let known = snapshot.keys.contains(&key);
        match row["outcome"].as_str() {
            Some("registered") if known => {}
            Some("registry_miss") if !known => {}
            _ => return Err(DISPATCH),
        }
        let requester_ok = match row["requester"]["type"].as_str() {
            Some("model") => exact(&row["requester"], &["type"]),
            Some("code_cell") => {
                exact(
                    &row["requester"],
                    &["type", "runtime_cell_id", "runtime_tool_call_id"],
                ) && text(&row["requester"]["runtime_cell_id"]).is_some()
            }
            _ => false,
        };
        if !requester_ok {
            return Err(UNAVAILABLE);
        }
        dispatches += 1;
    }
    if snapshots.is_empty() || sends.is_empty() {
        return Err(UNAVAILABLE);
    }
    for send in sends {
        let snapshot = snapshots.get(&send.router_snapshot_id).ok_or(JOIN)?;
        let visible = snapshot.specs.as_array().is_some_and(|s| !s.is_empty());
        let top_level_empty = send
            .tools
            .as_ref()
            .is_none_or(|tools| tools.as_array().is_some_and(Vec::is_empty));
        match send.additional_tools.as_slice() {
            // The classic request path: top-level `tools` equal to the snapshot's specs.
            [] if !top_level_empty => {
                if send.tools.as_ref() != Some(&snapshot.specs) {
                    return Err(JOIN);
                }
            }
            // The Responses Lite path: exactly one declaration item, no top-level field.
            [declared] if top_level_empty => {
                if declared != &snapshot.lite_specs {
                    return Err(JOIN);
                }
            }
            [_, ..] => return Err(JOIN),
            // No declaration at all, e.g. an incremental send: nothing to join.
            [] if visible => return Err(INCREMENTAL),
            [] => {}
        }
    }
    // One way only: ordinary writes are best effort, strict rows are not.
    if ordinary_started.iter().any(|id| !calls.contains(id)) {
        return Err(UNTRACED);
    }
    Ok(CheckedToolRegistry {
        bytes: retained,
        snapshots: snapshots.len(),
        dispatches,
    })
}

fn check_snapshot(
    row: &Value,
    expected: &ExpectedToolSet,
    wanted: &BTreeSet<ToolKey>,
) -> Result<Snapshot, Refused> {
    if !row["first_collision"].is_null() {
        return Err(COLLISION);
    }
    // The expected mode comes from Work Engine's own launch, never from the snapshot.
    let mode = row["tool_mode"].as_str().ok_or(UNAVAILABLE)?;
    let mode_ok = if expected.code_mode_host {
        matches!(mode, "code_mode" | "code_mode_only")
    } else {
        mode == "direct"
    };
    if !mode_ok {
        return Err(MODE);
    }
    let registered = row["registered_tools"].as_array().ok_or(UNAVAILABLE)?;
    let mut keys = BTreeSet::new();
    let mut nested = BTreeMap::new();
    for tool in registered {
        if !exact(
            tool,
            &[
                "namespace",
                "name",
                "exposure",
                "cached_runtime",
                "code_mode_name",
                "spec",
                "spec_sha256",
            ],
        ) {
            return Err(UNAVAILABLE);
        }
        let key = key_of(tool).ok_or(UNAVAILABLE)?;
        if sha(&tool["spec"]).as_deref() != tool["spec_sha256"].as_str() {
            return Err(SPEC);
        }
        if !keys.insert(key.clone()) {
            return Err(COLLISION);
        }
        if let Some(rendered) = tool["code_mode_name"].as_str() {
            nested.insert(rendered.to_owned(), key.clone());
        } else if !tool["code_mode_name"].is_null() {
            return Err(UNAVAILABLE);
        }
        if let Some(approved) = approved_utility(&key) {
            let nested_name = expected.code_mode_host.then_some(approved.3).flatten();
            if tool["exposure"] != approved.2
                || tool["code_mode_name"].as_str() != nested_name
                || tool["spec_sha256"] != approved.4
            {
                return Err(SPEC);
            }
        }
        if let Some(requested) = expected.tools.iter().find(|t| t.key == key) {
            let exposure = if requested.deferred {
                "deferred"
            } else {
                "direct"
            };
            if tool["exposure"] != exposure || !spec_matches(&tool["spec"], requested) {
                return Err(SPEC);
            }
        }
    }
    if &keys != wanted {
        return Err(EXTRA_OR_MISSING);
    }
    // Nested names must be exactly the requested dynamic tools, as rendered by native.
    let reported = row["code_mode_tools"].as_object().ok_or(UNAVAILABLE)?;
    let requested = expected
        .tools
        .iter()
        .map(|t| t.key.clone())
        .collect::<BTreeSet<_>>();
    let reported_keys = reported
        .iter()
        .map(|(rendered, tool)| {
            let key = key_of(tool).filter(|_| exact(tool, &["namespace", "name"]))?;
            (nested.get(rendered) == Some(&key)).then_some(key)
        })
        .collect::<Option<BTreeSet<_>>>()
        .ok_or(UNAVAILABLE)?;
    let nested_expected = if expected.code_mode_host {
        let mut all = requested;
        all.extend(
            APPROVED_NATIVE_UTILITIES
                .iter()
                .filter(|u| u.3.is_some())
                .map(|u| (u.0.map(str::to_owned), u.1.to_owned())),
        );
        all
    } else {
        BTreeSet::new()
    };
    if reported_keys != nested_expected || reported.len() != nested.len() {
        return Err(EXTRA_OR_MISSING);
    }
    let specs = row["model_visible_specs"].clone();
    if sha(&specs).as_deref() != row["model_visible_specs_sha256"].as_str() {
        return Err(SPEC);
    }
    let lite_specs = row["model_visible_specs_responses_lite"].clone();
    if !lite_specs.is_array()
        || sha(&lite_specs).as_deref() != row["model_visible_specs_responses_lite_sha256"].as_str()
    {
        return Err(SPEC);
    }
    for spec in specs.as_array().ok_or(UNAVAILABLE)? {
        if !visible_spec_allowed(spec, wanted) {
            return Err(EXTRA_OR_MISSING);
        }
    }
    Ok(Snapshot {
        keys,
        specs,
        lite_specs,
    })
}

/// The finalized function object carries exactly `type`, `name`, `description`, `strict`
/// and `parameters`, plus `defer_loading: true` when the tool was sent deferred and
/// `output_schema` when one was sent. Native finalizes dynamic tools with `strict: false`
/// (`tool_definition_to_responses_api_tool`) and never serializes `output_schema`, so a
/// sent `outputSchema` does not match. Any other key or value refuses.
fn spec_matches(spec: &Value, requested: &ExpectedTool) -> bool {
    let function = match &requested.key.0 {
        Some(namespace) => {
            if !exact(spec, &["type", "name", "description", "tools"])
                || spec["type"] != "namespace"
                || spec["name"] != namespace.as_str()
                || requested
                    .namespace_description
                    .as_deref()
                    .is_some_and(|d| spec["description"] != d)
            {
                return false;
            }
            match spec["tools"].as_array().map(Vec::as_slice) {
                Some([only]) => only,
                _ => return false,
            }
        }
        None => spec,
    };
    let mut keys = vec!["type", "name", "description", "strict", "parameters"];
    if requested.deferred {
        keys.push("defer_loading");
    }
    if requested.output_schema.is_some() {
        keys.push("output_schema");
    }
    exact(function, &keys)
        && function["type"] == "function"
        && function["name"] == requested.key.1.as_str()
        && function["description"] == requested.description.as_str()
        && function["strict"] == false
        && (!requested.deferred || function["defer_loading"] == true)
        && requested
            .output_schema
            .as_ref()
            .is_none_or(|schema| &function["output_schema"] == schema)
        && function["parameters"] == native_sanitized(&requested.input_schema)
}

/// Keywords of native's `JsonSchema` struct (`codex-rs/tools/src/json_schema/types.rs`).
/// `parse_tool_input_schema` (`json_schema.rs`, used by `parse_dynamic_tool`) deserializes
/// every dynamic tool schema into that struct, so serde drops every other keyword
/// (`minimum`, `maximum`, `format`, `pattern`, `default`, ...). That closed set is the only
/// difference this check tolerates between sent and finalized schemas. The sanitizer also
/// rewrites some shapes (boolean schemas, `const`, missing `items`/`properties`) and
/// compacts schemas above 5000 bytes; those are deliberately not modeled, so a tool needing
/// them does not match and is refused.
const NATIVE_SCHEMA_KEYWORDS: [&str; 15] = [
    "$ref",
    "type",
    "description",
    "encrypted",
    "enum",
    "items",
    "minItems",
    "properties",
    "required",
    "additionalProperties",
    "anyOf",
    "oneOf",
    "allOf",
    "$defs",
    "definitions",
];

fn native_sanitized(schema: &Value) -> Value {
    let Some(map) = schema.as_object() else {
        return schema.clone();
    };
    let schema_child = |key: &str, value: &Value| -> Value {
        match key {
            "items" => native_sanitized(value),
            "additionalProperties" if value.is_object() => native_sanitized(value),
            "properties" | "$defs" | "definitions" => match value.as_object() {
                Some(named) => Value::Object(
                    named
                        .iter()
                        .map(|(name, child)| (name.clone(), native_sanitized(child)))
                        .collect(),
                ),
                None => value.clone(),
            },
            "anyOf" | "oneOf" | "allOf" => match value.as_array() {
                Some(list) => Value::Array(list.iter().map(native_sanitized).collect()),
                None => value.clone(),
            },
            _ => value.clone(),
        }
    };
    Value::Object(
        map.iter()
            .filter(|(key, _)| NATIVE_SCHEMA_KEYWORDS.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), schema_child(key, value)))
            .collect(),
    )
}

/// Every model-visible spec must belong to the expected set: hosted, search and other
/// native specs are mismatches.
fn visible_spec_allowed(spec: &Value, wanted: &BTreeSet<ToolKey>) -> bool {
    match spec["type"].as_str() {
        Some("function" | "custom") => spec["name"]
            .as_str()
            .is_some_and(|name| wanted.contains(&(None, name.to_owned()))),
        Some("namespace") => {
            let Some(namespace) = spec["name"].as_str() else {
                return false;
            };
            spec["tools"].as_array().is_some_and(|tools| {
                tools.iter().all(|tool| {
                    tool["name"].as_str().is_some_and(|name| {
                        wanted.contains(&(Some(namespace.to_owned()), name.to_owned()))
                    })
                })
            })
        }
        _ => false,
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use serde_json::json;

    /// The seven approved native utilities exactly as the native snapshot of binary
    /// 77fbfa4b recorded them.
    pub(crate) fn approved_utilities() -> Vec<Value> {
        serde_json::from_str::<Vec<Value>>(include_str!(
            "../testdata/native_utilities_77fbfa4b.json"
        ))
        .unwrap()
    }
    pub(crate) fn dynamic_tools() -> Value {
        json!([
            {"type":"namespace","name":"workspace","description":"read","tools":[
                {"type":"function","name":"snapshot_read","description":"Read one member",
                 "inputSchema":{"type":"object","properties":{"member_id":{"type":"string"},"length":{"type":"integer","minimum":1,"maximum":65536}},"additionalProperties":false}}]},
            {"type":"namespace","name":"task","description":"task","tools":[
                {"type":"function","name":"result_publish","description":"Publish a result",
                 "inputSchema":{"type":"object","properties":{"body":{"type":"string"}},"additionalProperties":false}}]}
        ])
    }
    pub(crate) fn expected() -> ExpectedToolSet {
        ExpectedToolSet::from_dynamic_tools(&dynamic_tools(), true).unwrap()
    }
    fn namespace_spec(entry: &Value) -> Value {
        let tool = &entry["tools"][0];
        json!({"type":"namespace","name":entry["name"],"description":entry["description"],
            "tools":[{"type":"function","name":tool["name"],"description":tool["description"],
                "strict":false,"parameters":native_sanitized(&tool["inputSchema"])}]})
    }
    pub(crate) fn seal(snapshot: &mut Value) {
        for tool in snapshot["registered_tools"].as_array_mut().unwrap() {
            tool["spec_sha256"] = json!(sha(&tool["spec"]).unwrap());
        }
        snapshot["model_visible_specs_sha256"] =
            json!(sha(&snapshot["model_visible_specs"]).unwrap());
        snapshot["model_visible_specs_responses_lite_sha256"] =
            json!(sha(&snapshot["model_visible_specs_responses_lite"]).unwrap());
    }
    /// A native-shaped snapshot for the expected set with code mode on.
    pub(crate) fn snapshot(id: &str, thread: &str, turn: &str) -> Value {
        let dynamic = dynamic_tools();
        let workspace = namespace_spec(&dynamic[0]);
        let task = namespace_spec(&dynamic[1]);
        let exec = json!({"type":"custom","name":"exec","description":"run code","format":{"type":"text"}});
        let wait = json!({"type":"function","name":"wait","description":"wait","strict":false,"parameters":{"type":"object"}});
        let tool = |namespace: Option<&str>, name: &str, spec: &Value, nested: Option<&str>| {
            json!({"namespace":namespace,"name":name,"exposure":"direct","cached_runtime":false,
                "code_mode_name":nested,"spec":spec,"spec_sha256":""})
        };
        let mut row = json!({"kind":"strict_router_snapshot","version":1,"router_snapshot_id":id,
            "thread_id":thread,"codex_turn_id":turn,"tool_mode":"code_mode","first_collision":null,
            "registered_tools":[
                tool(Some("workspace"),"snapshot_read",&workspace,Some("workspace__snapshot_read")),
                tool(Some("task"),"result_publish",&task,Some("task__result_publish")),
                tool(None,"exec",&exec,None),
                tool(None,"wait",&wait,None)],
            "code_mode_tools":{
                "workspace__snapshot_read":{"namespace":"workspace","name":"snapshot_read"},
                "task__result_publish":{"namespace":"task","name":"result_publish"}},
            "model_visible_specs":[workspace,task,exec,wait],
            "model_visible_specs_sha256":"","model_visible_specs_responses_lite":[],
            "model_visible_specs_responses_lite_sha256":""});
        for utility in approved_utilities() {
            if let Some(nested) = utility["code_mode_name"].as_str() {
                row["code_mode_tools"][nested] =
                    json!({"namespace":utility["namespace"],"name":utility["name"]});
            }
            row["registered_tools"]
                .as_array_mut()
                .unwrap()
                .push(utility);
        }
        row["model_visible_specs_responses_lite"] = json!([{"type":"namespace","name":"functions",
            "description":"functions","tools":[row["model_visible_specs"][2].clone(), row["model_visible_specs"][3].clone()]}]);
        seal(&mut row);
        row
    }
    pub(crate) fn dispatch(
        id: &str,
        call: &str,
        namespace: Option<&str>,
        name: &str,
        outcome: &str,
        cell: bool,
    ) -> Value {
        json!({"kind":"strict_tool_dispatch","version":1,"router_snapshot_id":id,"thread_id":"thread",
            "codex_turn_id":"turn","tool_call_id":call,"namespace":namespace,"name":name,
            "requester":if cell {json!({"type":"code_cell","runtime_cell_id":"cell-1","runtime_tool_call_id":call})} else {json!({"type":"model"})},
            "outcome":outcome})
    }
    pub(crate) fn file(rows: &[Value]) -> Vec<u8> {
        rows.iter()
            .map(|r| format!("{r}\n"))
            .collect::<String>()
            .into_bytes()
    }
    /// One step: snapshot, a direct task call, a nested workspace call, one send citing it.
    pub(crate) fn positive() -> (Vec<Value>, Vec<SendCitation>) {
        let snapshot = snapshot("snap-1", "thread", "turn");
        let sends = vec![SendCitation {
            router_snapshot_id: "snap-1".into(),
            tools: Some(snapshot["model_visible_specs"].clone()),
            additional_tools: Vec::new(),
        }];
        (
            vec![
                snapshot,
                dispatch("snap-1", "call-1", None, "exec", "registered", false),
                dispatch(
                    "snap-1",
                    "call-2",
                    Some("workspace"),
                    "snapshot_read",
                    "registered",
                    true,
                ),
            ],
            sends,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;
    use serde_json::json;

    fn run(rows: &[Value], sends: &[SendCitation]) -> Result<CheckedToolRegistry, Refused> {
        check(
            &file(rows)[..],
            "thread",
            "turn",
            &expected(),
            sends,
            &BTreeSet::new(),
        )
    }
    fn refused(rows: &[Value], sends: &[SendCitation], reason: Refused) {
        assert_eq!(run(rows, sends).unwrap_err(), reason);
    }

    #[test]
    fn approved_native_utilities_are_pinned_to_the_observed_snapshot() {
        let observed = approved_utilities();
        assert_eq!(observed.len(), APPROVED_NATIVE_UTILITIES.len());
        for approved in APPROVED_NATIVE_UTILITIES {
            let tool = observed
                .iter()
                .find(|t| t["name"] == approved.1 && t["namespace"].as_str() == approved.0)
                .unwrap();
            assert_eq!(tool["exposure"], approved.2);
            assert_eq!(tool["code_mode_name"].as_str(), approved.3);
            assert_eq!(sha(&tool["spec"]).unwrap(), approved.4);
            assert_eq!(tool["spec_sha256"], approved.4);
        }
    }

    #[test]
    fn unapproved_native_utility_or_changed_approved_utility_is_refused() {
        let (rows, sends) = positive();
        // An extra native utility outside the approved list.
        let mut extra = rows.clone();
        let mut view_image = approved_utilities().remove(0);
        view_image["name"] = json!("view_image");
        view_image["code_mode_name"] = json!("view_image");
        extra[0]["registered_tools"]
            .as_array_mut()
            .unwrap()
            .push(view_image);
        extra[0]["code_mode_tools"]["view_image"] = json!({"namespace":null,"name":"view_image"});
        seal(&mut extra[0]);
        refused(&extra, &sends, EXTRA_OR_MISSING);
        // A changed spec for an approved utility changes the pinned digest.
        let mut changed = rows.clone();
        for tool in changed[0]["registered_tools"].as_array_mut().unwrap() {
            if tool["name"] == "read_mcp_resource" {
                tool["spec"]["description"] = json!("changed after approval");
            }
        }
        seal(&mut changed[0]);
        refused(&changed, &sends, SPEC);
        // A changed exposure of an approved utility.
        let mut exposure = rows.clone();
        for tool in exposure[0]["registered_tools"].as_array_mut().unwrap() {
            if tool["name"] == "sleep" {
                tool["exposure"] = json!("direct");
            }
        }
        refused(&exposure, &sends, SPEC);
        // A missing approved utility.
        let mut missing = rows.clone();
        missing[0]["registered_tools"]
            .as_array_mut()
            .unwrap()
            .retain(|t| t["name"] != "curr_time");
        refused(&missing, &sends, EXTRA_OR_MISSING);
    }

    #[test]
    fn native_schema_sanitizer_keywords_are_tolerated_and_others_are_refused() {
        let requested = json!({"type":"object","properties":{
            "length":{"type":"integer","minimum":1,"maximum":65536,"format":"int64"},
            "minimum":{"type":"string","pattern":"^a"}},"required":["length"],
            "additionalProperties":false});
        // Native drops exactly the keywords outside its JsonSchema struct; property names
        // that spell a dropped keyword are kept.
        assert_eq!(
            native_sanitized(&requested),
            json!({"type":"object","properties":{
                "length":{"type":"integer"},"minimum":{"type":"string"}},
                "required":["length"],"additionalProperties":false})
        );
        let mut rows = positive().0;
        let sends = positive().1;
        for tool in rows[0]["registered_tools"].as_array_mut().unwrap() {
            if tool["name"] == "result_publish" {
                tool["spec"]["tools"][0]["parameters"]["properties"]["body"]["type"] =
                    json!("integer");
            }
        }
        for spec in rows[0]["model_visible_specs"].as_array_mut().unwrap() {
            if spec["name"] == "task" {
                spec["tools"][0]["parameters"]["properties"]["body"]["type"] = json!("integer");
            }
        }
        seal(&mut rows[0]);
        refused(&rows, &sends, SPEC);
    }

    #[test]
    fn finalized_function_object_must_match_the_sent_keys_and_values_exactly() {
        let (rows, sends) = positive();
        // `result_publish` is registered at index 1; its function object is `tools[0]`.
        let mutate = |change: &dyn Fn(&mut Value, &mut Value)| {
            let mut changed = rows.clone();
            let tool = &mut changed[0]["registered_tools"][1]["spec"];
            let mut namespace_description = Value::Null;
            change(&mut tool["tools"][0], &mut namespace_description);
            if !namespace_description.is_null() {
                tool["description"] = namespace_description;
            }
            seal(&mut changed[0]);
            changed
        };
        refused(&mutate(&|f, _| f["extra"] = json!(true)), &sends, SPEC);
        refused(&mutate(&|f, _| f["strict"] = json!(true)), &sends, SPEC);
        refused(
            &mutate(&|f, _| f["defer_loading"] = json!(true)),
            &sends,
            SPEC,
        );
        refused(
            &mutate(&|f, _| f["output_schema"] = json!({})),
            &sends,
            SPEC,
        );
        refused(
            &mutate(&|f, _| {
                f.as_object_mut().unwrap().remove("strict");
            }),
            &sends,
            SPEC,
        );
        refused(
            &mutate(&|_, description| *description = json!("another namespace description")),
            &sends,
            SPEC,
        );
        // A sent deferLoading requires the exposure and the `defer_loading` key together.
        let mut deferred = dynamic_tools();
        deferred[1]["tools"][0]["deferLoading"] = json!(true);
        let expected = ExpectedToolSet::from_dynamic_tools(&deferred, true).unwrap();
        let requested = expected
            .tools
            .iter()
            .find(|t| t.key.1 == "result_publish")
            .unwrap();
        let mut spec = rows[0]["registered_tools"][1]["spec"].clone();
        assert!(!spec_matches(&spec, requested));
        spec["tools"][0]["defer_loading"] = json!(true);
        assert!(spec_matches(&spec, requested));
        // A sent outputSchema is never finalized by native, so it cannot match.
        let mut with_output = dynamic_tools();
        with_output[1]["tools"][0]["outputSchema"] = json!({"type":"object"});
        let expected = ExpectedToolSet::from_dynamic_tools(&with_output, true).unwrap();
        let requested = expected
            .tools
            .iter()
            .find(|t| t.key.1 == "result_publish")
            .unwrap();
        assert!(!spec_matches(
            &rows[0]["registered_tools"][1]["spec"],
            requested
        ));
    }

    #[test]
    fn earlier_turn_rows_beyond_the_retained_bound_do_not_refuse_the_current_turn() {
        let (rows, sends) = positive();
        let mut earlier = Vec::new();
        let mut size = 0;
        let mut index = 0;
        while size <= RETAINED_LIMIT + 1024 {
            let row = snapshot(&format!("snap-earlier-{index}"), "thread", "earlier-turn");
            size += file(std::slice::from_ref(&row)).len();
            earlier.push(row);
            index += 1;
        }
        let mut all = earlier;
        all.extend(rows.iter().cloned());
        let bytes = file(&all);
        assert!(bytes.len() > RETAINED_LIMIT);
        let checked = check(
            &bytes[..],
            "thread",
            "turn",
            &expected(),
            &sends,
            &BTreeSet::new(),
        )
        .unwrap();
        assert_eq!(checked.bytes(), file(&rows));
        assert_eq!(checked.snapshots(), 1);
        // Rows of other turns are still parsed strictly: a bad one anywhere refuses.
        let mut bad = all.clone();
        bad[0].as_object_mut().unwrap().remove("kind");
        assert_eq!(
            check(
                &file(&bad)[..],
                "thread",
                "turn",
                &expected(),
                &sends,
                &BTreeSet::new()
            )
            .unwrap_err(),
            UNAVAILABLE
        );
    }

    #[test]
    fn one_oversized_row_or_turn_retained_bytes_beyond_the_bound_refuse() {
        let (rows, sends) = positive();
        let run_bytes = |bytes: Vec<u8>| {
            check(
                &bytes[..],
                "thread",
                "turn",
                &expected(),
                &sends,
                &BTreeSet::new(),
            )
        };
        // One row above the row limit, even for another turn.
        let mut oversized = snapshot("snap-big", "thread", "earlier-turn");
        oversized["pad"] = json!("x".repeat(ROW_LIMIT));
        let mut with_big = vec![oversized];
        with_big.extend(rows.iter().cloned());
        assert_eq!(run_bytes(file(&with_big)).unwrap_err(), UNAVAILABLE);
        // A row of another turn just under the limit is read and skipped.
        let mut large = dispatch("snap-x", "call-x", None, "exec", "registered", false);
        large["codex_turn_id"] = json!("earlier-turn");
        large["pad"] = json!("x".repeat(ROW_LIMIT - 4096));
        let mut with_large = vec![large];
        with_large.extend(rows.iter().cloned());
        assert_eq!(run_bytes(file(&with_large)).unwrap().snapshots(), 1);
        // Many current-turn rows beyond the retained bound.
        let mut crowded = rows.clone();
        let mut index = 0;
        while file(&crowded).len() <= RETAINED_LIMIT {
            crowded.push(snapshot(&format!("snap-crowd-{index}"), "thread", "turn"));
            index += 1;
        }
        assert_eq!(run_bytes(file(&crowded)).unwrap_err(), UNAVAILABLE);
    }

    #[test]
    fn matching_snapshot_dispatches_and_request_tools_are_accepted() {
        let (rows, sends) = positive();
        let checked = run(&rows, &sends).unwrap();
        assert_eq!((checked.snapshots(), checked.dispatches()), (1, 2));
        assert_eq!(checked.bytes(), file(&rows));
    }

    #[test]
    fn unknown_name_rejection_is_consistent_only_as_registry_miss_with_matching_snapshot() {
        let (mut rows, sends) = positive();
        rows.push(dispatch(
            "snap-1",
            "call-3",
            None,
            "shell",
            "registry_miss",
            false,
        ));
        assert!(run(&rows, &sends).is_ok());
        let last = rows.len() - 1;
        rows[last]["outcome"] = json!("registered");
        refused(&rows, &sends, DISPATCH);
        // A miss on a name the snapshot does list means a handler existed.
        let (mut rows, sends) = positive();
        rows.push(dispatch(
            "snap-1",
            "call-3",
            None,
            "exec",
            "registry_miss",
            false,
        ));
        refused(&rows, &sends, DISPATCH);
    }

    #[test]
    fn extra_missing_or_nested_tool_sets_are_refused() {
        let (rows, sends) = positive();
        let mut extra = rows.clone();
        let spec = json!({"type":"function","name":"exec_command","description":"x","strict":false,"parameters":{}});
        extra[0]["registered_tools"].as_array_mut().unwrap().push(json!({"namespace":null,
            "name":"exec_command","exposure":"direct","cached_runtime":false,"code_mode_name":null,"spec":spec,"spec_sha256":""}));
        seal(&mut extra[0]);
        refused(&extra, &sends, EXTRA_OR_MISSING);
        let mut missing = rows.clone();
        missing[0]["registered_tools"]
            .as_array_mut()
            .unwrap()
            .remove(1);
        seal(&mut missing[0]);
        refused(&missing, &sends, EXTRA_OR_MISSING);
        let mut nested = rows.clone();
        nested[0]["code_mode_tools"]
            .as_object_mut()
            .unwrap()
            .remove("task__result_publish");
        nested[0]["registered_tools"][1]["code_mode_name"] = Value::Null;
        seal(&mut nested[0]);
        refused(&nested, &sends, EXTRA_OR_MISSING);
        let mut hosted = rows.clone();
        hosted[0]["model_visible_specs"]
            .as_array_mut()
            .unwrap()
            .push(json!({"type":"web_search"}));
        seal(&mut hosted[0]);
        let mut hosted_sends = sends.clone();
        hosted_sends[0].tools = Some(hosted[0]["model_visible_specs"].clone());
        refused(&hosted, &hosted_sends, EXTRA_OR_MISSING);
    }

    #[test]
    fn schema_drop_collision_mode_and_tamper_are_refused_with_reasons() {
        let (rows, sends) = positive();
        let mut dropped = rows.clone();
        dropped[0]["registered_tools"][0]["spec"]["tools"][0]["parameters"] =
            json!({"type":"object","properties":{}});
        seal(&mut dropped[0]);
        refused(&dropped, &sends, SPEC);
        let mut collision = rows.clone();
        collision[0]["first_collision"] = json!({"namespace":null,"name":"exec"});
        refused(&collision, &sends, COLLISION);
        let mut mode = rows.clone();
        mode[0]["tool_mode"] = json!("direct");
        refused(&mode, &sends, MODE);
        let mut deferred = rows.clone();
        deferred[0]["registered_tools"][0]["exposure"] = json!("deferred");
        refused(&deferred, &sends, SPEC);
        let mut tampered = rows.clone();
        tampered[0]["registered_tools"][0]["spec"]["tools"][0]["description"] = json!("changed");
        refused(&tampered, &sends, SPEC);
        let mut specs = rows.clone();
        specs[0]["model_visible_specs"][2]["description"] = json!("changed");
        refused(&specs, &sends, SPEC);
    }

    #[test]
    fn hostile_row_sets_are_refused() {
        let (rows, sends) = positive();
        let mut twice = rows.clone();
        twice.insert(1, rows[0].clone());
        refused(&twice, &sends, ROWS);
        let mut same_call = rows.clone();
        same_call.push(rows[1].clone());
        refused(&same_call, &sends, ROWS);
        let mut orphan = rows.clone();
        orphan.push(dispatch(
            "unknown",
            "call-9",
            None,
            "exec",
            "registered",
            false,
        ));
        refused(&orphan, &sends, DISPATCH);
        let mut early = rows.clone();
        early.swap(0, 1);
        refused(&early, &sends, DISPATCH);
        let mut fields = rows.clone();
        fields[1]["extra"] = json!(1);
        refused(&fields, &sends, UNAVAILABLE);
        let mut duplicate_keys = file(&rows);
        duplicate_keys.splice(
            0..1,
            *b"{\"kind\":\"strict_router_snapshot\",\"kind\":\"strict_router_snapshot\",",
        );
        assert!(
            check(
                &duplicate_keys[..duplicate_keys.len()],
                "thread",
                "turn",
                &expected(),
                &sends,
                &BTreeSet::new()
            )
            .is_err()
        );
        assert_eq!(
            check(
                &b""[..],
                "thread",
                "turn",
                &expected(),
                &sends,
                &BTreeSet::new()
            )
            .unwrap_err(),
            UNAVAILABLE
        );
        let mut unterminated = file(&rows);
        unterminated.pop();
        assert_eq!(
            check(
                &unterminated[..],
                "thread",
                "turn",
                &expected(),
                &sends,
                &BTreeSet::new()
            )
            .unwrap_err(),
            UNAVAILABLE
        );
    }

    #[test]
    fn request_tools_must_equal_the_cited_snapshot_by_id_not_position() {
        let (rows, sends) = positive();
        let mut changed = sends.clone();
        changed[0]
            .tools
            .as_mut()
            .unwrap()
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        refused(&rows, &changed, JOIN);
        let mut other = sends.clone();
        other[0].router_snapshot_id = "snap-2".into();
        refused(&rows, &other, JOIN);
        let mut absent = sends.clone();
        absent[0].tools = None;
        refused(&rows, &absent, INCREMENTAL);
        let mut empty = sends.clone();
        empty[0].tools = Some(json!([]));
        refused(&rows, &empty, INCREMENTAL);
        refused(&rows, &[], UNAVAILABLE);
    }

    #[test]
    fn responses_lite_requests_join_through_the_single_additional_tools_item() {
        let (rows, sends) = positive();
        let lite = rows[0]["model_visible_specs_responses_lite"].clone();
        let lite_send = SendCitation {
            router_snapshot_id: "snap-1".into(),
            tools: None,
            additional_tools: vec![lite.clone()],
        };
        assert!(run(&rows, std::slice::from_ref(&lite_send)).is_ok());
        let mut empty_top = lite_send.clone();
        empty_top.tools = Some(json!([]));
        assert!(run(&rows, &[empty_top]).is_ok());
        // Both declarations, two declarations, or a different declaration are refused.
        let mut both = lite_send.clone();
        both.tools = sends[0].tools.clone();
        refused(&rows, &[both], JOIN);
        let mut twice = lite_send.clone();
        twice.additional_tools.push(lite);
        refused(&rows, &[twice], JOIN);
        let mut other = lite_send.clone();
        other.additional_tools = vec![json!([{"type":"namespace","name":"functions","tools":[]}])];
        refused(&rows, &[other], JOIN);
        // Classic equality is not satisfied by the lite value, nor the reverse.
        let mut classic_with_lite_value = sends[0].clone();
        classic_with_lite_value.tools = Some(rows[0]["model_visible_specs_responses_lite"].clone());
        refused(&rows, &[classic_with_lite_value], JOIN);
    }

    #[test]
    fn other_turns_are_ignored_and_untraced_ordinary_calls_are_refused() {
        let (mut rows, sends) = positive();
        rows.insert(0, snapshot("snap-other", "thread", "other-turn"));
        let checked = run(&rows, &sends).unwrap();
        assert_eq!(checked.snapshots(), 1);
        assert!(
            !String::from_utf8(checked.bytes().to_vec())
                .unwrap()
                .contains("snap-other")
        );
        let started = BTreeSet::from(["call-1".to_owned(), "call-2".to_owned()]);
        assert!(
            check(
                &file(&rows)[..],
                "thread",
                "turn",
                &expected(),
                &sends,
                &started
            )
            .is_ok()
        );
        let started = BTreeSet::from(["call-unrecorded".to_owned()]);
        assert_eq!(
            check(
                &file(&rows)[..],
                "thread",
                "turn",
                &expected(),
                &sends,
                &started
            )
            .unwrap_err(),
            UNTRACED
        );
    }

    #[test]
    fn expected_set_derives_from_the_sent_value_and_rejects_ambiguity() {
        assert!(ExpectedToolSet::from_dynamic_tools(&json!([]), true).is_none());
        assert!(ExpectedToolSet::from_dynamic_tools(&json!([{"type":"other"}]), true).is_none());
        let mut duplicate = dynamic_tools();
        let first = duplicate[0].clone();
        duplicate.as_array_mut().unwrap().push(first);
        assert!(ExpectedToolSet::from_dynamic_tools(&duplicate, true).is_none());
        let direct = ExpectedToolSet::from_dynamic_tools(&dynamic_tools(), false).unwrap();
        assert_eq!(direct.keys().len(), 9);
        assert_eq!(expected().keys().len(), 11);
    }
}
