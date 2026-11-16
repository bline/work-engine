//! Checked native tool-registry evidence for one controlled turn.
//!
//! The expected tool set is derived from the exact `thread/start` `dynamicTools` value that
//! Work Engine sent, never from a second list. Every row is native-produced bytes read once
//! when the turn completes; a dispatch appended after that read is outside this check, so the
//! controlled fixture re-digests the file after the native process is waited.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const FILE_LIMIT: usize = 2 * 1024 * 1024;
const ROW_LIMIT: usize = 512 * 1024;
const ROW_COUNT_LIMIT: usize = 2048;
const CODE_MODE_NAMES: [&str; 2] = ["exec", "wait"];

type ToolKey = (Option<String>, String);

#[derive(Debug, Clone)]
struct ExpectedTool {
    key: ToolKey,
    description: String,
    input_schema: Value,
    deferred: bool,
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
                    for tool in entry["tools"].as_array()? {
                        tools.push(expected_function(Some(namespace), tool)?);
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
pub(crate) fn check(
    file: &[u8],
    thread: &str,
    turn: &str,
    expected: &ExpectedToolSet,
    sends: &[SendCitation],
    ordinary_started: &BTreeSet<String>,
) -> Result<CheckedToolRegistry, Refused> {
    if file.is_empty() || file.len() > FILE_LIMIT || !file.ends_with(b"\n") {
        return Err(UNAVAILABLE);
    }
    let wanted = expected.keys();
    let mut selected = Vec::new();
    let mut retained = Vec::new();
    for (index, span) in file.split_inclusive(|b| *b == b'\n').enumerate() {
        if index >= ROW_COUNT_LIMIT || span.len() > ROW_LIMIT {
            return Err(UNAVAILABLE);
        }
        let line = span.strip_suffix(b"\n").unwrap_or(span);
        let row = lifecycle_wire::parse_strict_json(line).map_err(|_| UNAVAILABLE)?;
        if !matches!(
            row["kind"].as_str(),
            Some("strict_router_snapshot" | "strict_tool_dispatch")
        ) {
            return Err(UNAVAILABLE);
        }
        if row["thread_id"] == thread && row["codex_turn_id"] == turn {
            retained.extend_from_slice(span);
            selected.push(row);
        }
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
        &requested
    } else {
        &BTreeSet::new()
    };
    if &reported_keys != nested_expected || reported.len() != nested.len() {
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

fn spec_matches(spec: &Value, requested: &ExpectedTool) -> bool {
    let function = match &requested.key.0 {
        Some(namespace) => {
            if spec["type"] != "namespace" || spec["name"] != namespace.as_str() {
                return false;
            }
            match spec["tools"].as_array().map(Vec::as_slice) {
                Some([only]) => only,
                _ => return false,
            }
        }
        None => spec,
    };
    function["type"] == "function"
        && function["name"] == requested.key.1.as_str()
        && function["description"] == requested.description.as_str()
        && function["parameters"] == requested.input_schema
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

    pub(crate) fn dynamic_tools() -> Value {
        json!([
            {"type":"namespace","name":"workspace","description":"read","tools":[
                {"type":"function","name":"snapshot_read","description":"Read one member",
                 "inputSchema":{"type":"object","properties":{"member_id":{"type":"string"}},"additionalProperties":false}}]},
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
                "strict":false,"parameters":tool["inputSchema"]}]})
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
            &file(rows),
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
            check(b"", "thread", "turn", &expected(), &sends, &BTreeSet::new()).unwrap_err(),
            UNAVAILABLE
        );
        let mut unterminated = file(&rows);
        unterminated.pop();
        assert_eq!(
            check(
                &unterminated,
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
                &file(&rows),
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
                &file(&rows),
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
        assert_eq!(direct.keys().len(), 2);
        assert_eq!(expected().keys().len(), 4);
    }
}
