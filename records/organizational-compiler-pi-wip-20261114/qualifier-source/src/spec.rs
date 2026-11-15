//! The root-approved qualification specification (obligation map v2). A small fixed schema, not a
//! rule language: the check kinds, artifact representations and their semantic checkers are
//! finite and implemented here. [`load_spec`] rejects a malformed or incomplete specification,
//! duplicate step/check identities, unknown kinds/classes/representations, unresolved or
//! out-of-order dependencies, missing intrinsic dependencies, and required artifacts without a
//! checker. The explicit `checks` list must equal the identities this schema derives from the
//! processes, artifacts, expectations and controls, so requiredness is stated by the approved
//! map and cross-checked against the fixed schema; it never comes from what a run produced.
use crate::base::{file_sha, sha256_hex};
use crate::strict::{self, is_hex64, Obj};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const MAP_SCHEMA: &str = "org-obligation-map/v3";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repr {
    /// Strict JSON (duplicate keys rejected).
    Json,
    /// Strict JSONL protocol lines.
    Jsonl,
    /// Exact UTF-8 bytes (contract text).
    Utf8,
    /// Exact raw bytes (a received byte stream): hash-accounted here, interpreted by the delivery owner.
    Raw,
    /// Permitted non-evidence text; must be present, UTF-8 and hash-accounted.
    Text,
    /// Exact stored episode state, validated by `ReviewEpisodeState::from_stored_json`.
    CanonicalState,
}

impl Repr {
    fn parse(s: &str) -> Option<Repr> {
        Some(match s {
            "json" => Repr::Json,
            "jsonl" => Repr::Jsonl,
            "raw" => Repr::Raw,
            "utf8" => Repr::Utf8,
            "text" => Repr::Text,
            "canonical_state" => Repr::CanonicalState,
            _ => return None,
        })
    }
}

/// The artifacts the domain joins consume, with the representation each checker requires.
pub const REQUIRED_ARTIFACTS: &[(&str, Repr)] = &[
    ("identities.json", Repr::Json),
    ("candidate-requirements.json", Repr::Json),
    ("authority.json", Repr::Json),
    ("consumer-inputs.json", Repr::Json),
    ("preparation.json", Repr::Json),
    ("contract.bytes", Repr::Utf8),
    ("declaration.json", Repr::Json),
    ("admissions.json", Repr::Json),
    ("synthetic-result.json", Repr::Json),
    ("native-frames-received.jsonl", Repr::Raw),
    ("native-frames-sent.jsonl", Repr::Raw),
    ("request-context.json", Repr::Json),
    ("native-history.json", Repr::Json),
    ("observation.json", Repr::Json),
    ("episode-before.json", Repr::CanonicalState),
    ("episode-after.json", Repr::CanonicalState),
    ("revisions.json", Repr::Json),
    ("consumption.json", Repr::Json),
    ("process.json", Repr::Json),
    ("collection.json", Repr::Json),
    ("stderr.txt", Repr::Raw),
    ("capture.json", Repr::Json),
];

/// Cross-artifact joins and the artifacts/checks each intrinsically needs.
pub const JOINS: &[(&str, &[&str])] = &[
    (
        "invocation_binding",
        &["artifact:identities.json", "output_manifest", "@producer"],
    ),
    (
        "binding_recompute",
        &[
            "source_manifest",
            "artifact:candidate-requirements.json",
            "artifact:authority.json",
            "artifact:consumer-inputs.json",
            "artifact:declaration.json",
            "artifact:admissions.json",
            "artifact:synthetic-result.json",
            "artifact:episode-before.json",
            "artifact:preparation.json",
            "artifact:contract.bytes",
        ],
    ),
    (
        "native_protocol",
        &[
            "binding_recompute",
            "artifact:native-frames-received.jsonl",
            "artifact:native-frames-sent.jsonl",
            "artifact:observation.json",
            "artifact:identities.json",
        ],
    ),
    (
        "context_history",
        &[
            "native_protocol",
            "artifact:request-context.json",
            "artifact:native-history.json",
        ],
    ),
    (
        "state_revisions",
        &[
            "artifact:episode-before.json",
            "artifact:episode-after.json",
            "artifact:revisions.json",
            "artifact:consumer-inputs.json",
        ],
    ),
    (
        "transition_recompute",
        &[
            "binding_recompute",
            "native_protocol",
            "state_revisions",
            "artifact:consumption.json",
            "artifact:episode-after.json",
        ],
    ),
    (
        "process_disposition",
        &[
            "artifact:process.json",
            "artifact:collection.json",
            "artifact:native-frames-received.jsonl",
            "artifact:native-frames-sent.jsonl",
            "artifact:stderr.txt",
            "@producer",
        ],
    ),
    (
        "capture_complete",
        &[
            "artifact:capture.json",
            "artifact:collection.json",
            "artifact:process.json",
            "artifact:native-frames-received.jsonl",
            "artifact:native-frames-sent.jsonl",
            "artifact:stderr.txt",
            "output_manifest",
        ],
    ),
];

#[derive(Debug, Clone)]
pub struct ControlSpec {
    pub name: String,
    pub scratch: String,
    pub outcome: String,
    pub preserved: Vec<String>,
    pub write_failure: Option<String>,
    /// Prospective source-bound authority input for a negative consumer scenario.
    pub authority_source: Option<String>,
    /// Where the approved scenario stops: `delivered_and_consumed` (default; an expected later
    /// failure or write failure), `delivery_refused` or `consumer_refused`. The bundle cannot
    /// choose its own stage: this decides expected availability and the semantic joins.
    pub stage: String,
}

#[derive(Debug, Clone)]
pub struct ValidatedSpec {
    pub map: Value,
    pub sha256: String,
    pub producer: String,
    pub artifacts: Vec<(String, Repr)>,
    pub controls: Vec<ControlSpec>,
    pub fixture_sources: BTreeMap<String, String>,
    pub frozen_authority_source: String,
    /// Ordered (identity, declared dependencies); equal to the derived set.
    pub checks: Vec<(String, Vec<String>)>,
}

impl ValidatedSpec {
    pub fn processes(&self) -> &Vec<Value> {
        self.map["processes"].as_array().expect("validated")
    }
    pub fn expected(&self) -> &Value {
        &self.map["expected"]
    }
}

fn kind_of(id: &str) -> &str {
    id.split_once(':').map(|(k, _)| k).unwrap_or(id)
}

/// Every check identity the schema derives from the map, with its intrinsic dependencies.
fn derived(
    map: &Value,
    producer: &str,
    artifacts: &[(String, Repr)],
    controls: &[ControlSpec],
) -> Vec<(String, Vec<String>)> {
    let processes = map["processes"].as_array().expect("validated");
    let exp = &map["expected"];
    let rec = |s: &str| format!("record:{s}");
    let mut out: Vec<(String, Vec<String>)> = vec![
        ("pins".into(), vec![]),
        ("source_manifest".into(), vec!["pins".into()]),
        ("summary".into(), vec!["pins".into()]),
    ];
    for p in processes {
        let step = p["step"].as_str().expect("validated");
        let mut deps = vec!["summary".to_owned(), "source_manifest".to_owned()];
        if let Some(e) = p.get("exe") {
            deps.push(rec(e["step"].as_str().expect("validated")));
        }
        out.push((rec(step), deps));
    }
    out.push((
        "step_order".into(),
        processes
            .iter()
            .map(|p| rec(p["step"].as_str().unwrap()))
            .collect(),
    ));
    for c in exp["stdout_checks"].as_array().into_iter().flatten() {
        let step = c["step"].as_str().expect("validated");
        out.push((format!("stdout_identity:{step}"), vec![rec(step)]));
    }
    out.push((
        "closure_root".into(),
        vec![rec(exp["closure_step"].as_str().expect("validated"))],
    ));
    out.push((
        "sdk_verification".into(),
        vec![rec(exp["sdk_step"].as_str().expect("validated"))],
    ));
    out.push(("output_manifest".into(), vec![rec(producer)]));
    for (name, _) in artifacts {
        out.push((format!("artifact:{name}"), vec!["output_manifest".into()]));
    }
    out.push((
        "output_window".into(),
        vec!["output_manifest".into(), rec(producer)],
    ));
    for (join, deps) in JOINS {
        out.push((
            (*join).into(),
            deps.iter()
                .map(|d| {
                    if *d == "@producer" {
                        rec(producer)
                    } else {
                        (*d).to_owned()
                    }
                })
                .collect(),
        ));
    }
    for c in controls {
        out.push((format!("control:{}", c.name), vec![rec(producer)]));
    }
    out
}

/// Parses a complete `{exe:<build step>:<target>}` placeholder (no other text, no further `:`,
/// `{` or `}`), or `None` when `text` is not exactly one.
fn exe_placeholder(text: &str) -> Option<(&str, &str)> {
    let inner = text.strip_prefix("{exe:")?.strip_suffix('}')?;
    let (step, target) = inner.split_once(':')?;
    let clean = |s: &str| !s.is_empty() && !s.contains([':', '{', '}']);
    (clean(step) && clean(target)).then_some((step, target))
}

/// R5: the executable relation of one process, mechanically two-sided. An executable placeholder
/// in `argv[0]` is meaningful only together with the declared `exe` relation naming the same
/// build step and target, and a declared relation requires its placeholder at `argv[0]`. Any other
/// occurrence of the placeholder syntax (malformed, shorthand, in another argv position, in `cwd`
/// or `env`, or naming a different producer/target than the declaration) is refused. Custody's own
/// placeholder resolution only resolves argv; it does not bind the executed file as input
/// evidence, so a placeholder without its declared relation would execute an unaccounted file.
/// The relation target must also be declared exactly once by its producing build (checked with
/// the build declarations in the process loop).
fn exe_relation_errors(step: &str, p: &Value) -> Vec<String> {
    let mut errs = Vec::new();
    let declared = p
        .get("exe")
        .and_then(|x| Some((x.get("step")?.as_str()?, x.get("target")?.as_str()?)));
    let argv: Vec<&str> = p
        .get("argv")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    for (i, a) in argv.iter().enumerate() {
        if !a.contains("{exe") {
            continue;
        }
        match exe_placeholder(a) {
            None => errs.push(format!("{step}: argv[{i}] `{a}` is a malformed or embedded executable placeholder (expected exactly `{{exe:<build>:<target>}}`)")),
            Some(_) if i != 0 => errs.push(format!("{step}: executable placeholder `{a}` at argv[{i}]; only argv[0] may name the executed file")),
            Some(pair) => match declared {
                None => errs.push(format!("{step}: executable placeholder `{a}` has no declared `exe` producer/target relation")),
                Some(d) if d != pair => errs.push(format!("{step}: placeholder `{a}` contradicts the declared relation `{}:{}`", d.0, d.1)),
                Some(_) => {}
            },
        }
    }
    if p.get("exe").is_some() {
        match (declared, argv.first().copied().and_then(exe_placeholder)) {
            (Some(d), Some(pair)) if d == pair => {}
            (Some(d), _) => errs.push(format!(
                "{step}: declared executable relation `{}:{}` is not argv[0]'s placeholder",
                d.0, d.1
            )),
            (None, _) => {} // malformed relation is reported by the exe object validation
        }
    }
    let mut elsewhere: Vec<(&str, &str)> = Vec::new();
    if let Some(c) = p.get("cwd").and_then(Value::as_str) {
        elsewhere.push(("cwd", c));
    }
    for (k, v) in p
        .get("env")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if let Some(t) = v.as_str() {
            elsewhere.push(("env", t));
        }
        if k.contains("{exe") {
            errs.push(format!(
                "{step}: executable placeholder syntax in an env name"
            ));
        }
    }
    for (field, t) in elsewhere {
        if t.contains("{exe") {
            errs.push(format!(
                "{step}: executable placeholder syntax in {field} `{t}`"
            ));
        }
    }
    errs
}

const PROCESS_KEYS: &[&str] = &[
    "step",
    "argv",
    "cwd",
    "env",
    "timeout",
    "inputs",
    "exe",
    "cargo_json",
    "exe_targets",
    "min_tests",
    "stdout_contains",
    "runtime_inputs",
];
const MAP_KEYS: &[&str] = &[
    "schema",
    "map_id",
    "authority",
    "repo_root",
    "required_roots",
    "required_files",
    "excluded_dir_names",
    "absent_expected",
    "external_inputs",
    "classes",
    "run_outputs",
    "processes",
    "output_producer",
    "artifacts",
    "expected",
    "fixture_sources",
    "controls",
    "checks",
    "limits",
    "accepted_nonliteral_include_files",
];

/// Loads and validates the specification at `path`, whose exact bytes must have `expect_sha256`.
pub fn load_spec(path: &Path, expect_sha256: &str) -> Result<ValidatedSpec, Vec<String>> {
    let bytes = std::fs::read(path).map_err(|e| vec![format!("{}: {e}", path.display())])?;
    let sha = sha256_hex(&bytes);
    if sha != expect_sha256 {
        return Err(vec![format!(
            "map {} has sha256 {sha}, approved {expect_sha256}",
            path.display()
        )]);
    }
    let map = strict::parse(&bytes).map_err(|e| vec![format!("map: {e}")])?;
    validate(map, sha)
}

pub fn validate(map: Value, sha256: String) -> Result<ValidatedSpec, Vec<String>> {
    let mut errs: Vec<String> = Vec::new();
    let mut e = |m: String| errs.push(m);
    let Ok(top) = Obj::new("map", &map) else {
        return Err(vec!["map: not an object".into()]);
    };
    if let Some(unknown) = map.as_object().map(|o| {
        o.keys()
            .filter(|k| !MAP_KEYS.contains(&k.as_str()))
            .cloned()
            .collect::<Vec<_>>()
    }) {
        if !unknown.is_empty() {
            e(format!("map: unknown keys {unknown:?}"));
        }
    }
    match top.str("schema") {
        Ok(MAP_SCHEMA) => {}
        Ok(other) => e(format!("map.schema `{other}` is not {MAP_SCHEMA}")),
        Err(x) => e(x),
    }
    for key in ["map_id", "authority", "repo_root"] {
        if let Err(x) = top.str(key) {
            e(x);
        }
    }
    for key in [
        "required_roots",
        "required_files",
        "excluded_dir_names",
        "absent_expected",
        "run_outputs",
        "accepted_nonliteral_include_files",
    ] {
        match top.strings(key) {
            Ok(v) if key == "required_roots" && v.is_empty() => {
                e("map.required_roots: empty".into())
            }
            Ok(v) => {
                let set: BTreeSet<&String> = v.iter().collect();
                if set.len() != v.len() {
                    e(format!("map.{key}: duplicate entries"));
                }
            }
            Err(x) => e(x),
        }
    }
    match top.obj("external_inputs") {
        Ok(o) => {
            for (k, v) in map["external_inputs"].as_object().unwrap() {
                if !serde_json::from_value::<crate::runtime::ObjectBinding>(v.clone())
                    .map(|b| is_hex64(b.sha()))
                    .unwrap_or(false)
                {
                    e(format!("map.external_inputs[{k}]: not a sha256"));
                }
            }
            let _ = o;
        }
        Err(x) => e(x),
    }
    let classes: BTreeSet<String> = match map["classes"].as_object() {
        Some(o) => {
            for (k, v) in o {
                if !v
                    .as_array()
                    .map(|a| !a.is_empty() && a.iter().all(Value::is_string))
                    .unwrap_or(false)
                {
                    e(format!("map.classes[{k}]: not a non-empty string array"));
                }
            }
            o.keys().cloned().collect()
        }
        None => {
            e("map.classes: not an object".into());
            BTreeSet::new()
        }
    };
    // Processes.
    let mut steps: Vec<String> = Vec::new();
    let mut builds: BTreeMap<String, Vec<String>> = BTreeMap::new();
    match map["processes"].as_array() {
        Some(ps) if !ps.is_empty() => {
            for (i, p) in ps.iter().enumerate() {
                let Ok(po) = Obj::new(format!("map.processes[{i}]"), p) else {
                    e(format!("map.processes[{i}]: not an object"));
                    continue;
                };
                for k in p.as_object().unwrap().keys() {
                    if !PROCESS_KEYS.contains(&k.as_str()) {
                        e(format!("map.processes[{i}]: unknown key `{k}`"));
                    }
                }
                let step = match po.str("step") {
                    Ok(s) if !s.is_empty() && !s.contains(':') && !s.contains('/') => s.to_owned(),
                    Ok(s) => {
                        e(format!(
                            "map.processes[{i}].step `{s}` is not a valid identity"
                        ));
                        continue;
                    }
                    Err(x) => {
                        e(x);
                        continue;
                    }
                };
                if steps.contains(&step) {
                    e(format!("duplicate step identity `{step}`"));
                }
                match po.strings("argv") {
                    Ok(a) if !a.is_empty() => {}
                    Ok(_) => e(format!("{step}: empty argv")),
                    Err(x) => e(x),
                }
                if let Err(x) = po.str("cwd") {
                    e(x);
                }
                match po.obj("env") {
                    Ok(_) if p["env"].as_object().unwrap().values().all(Value::is_string) => {}
                    Ok(_) => e(format!("{step}: env values must be strings")),
                    Err(x) => e(x),
                }
                if let Err(x) = po.u64("timeout") {
                    e(x);
                }
                match po.strings("inputs") {
                    Ok(v) if v.is_empty() => e(format!("{step}: no input classes")),
                    Ok(v) => {
                        for c in v {
                            if c != "@closure" && !classes.contains(&c) {
                                e(format!("{step}: unknown input class `{c}`"));
                            }
                        }
                    }
                    Err(x) => e(x),
                }
                match p.get("cargo_json") {
                    None => {
                        if p.get("exe_targets").is_some() {
                            e(format!("{step}: exe_targets without cargo_json"));
                        }
                    }
                    Some(Value::Bool(true)) => match po.strings("exe_targets") {
                        Ok(t) => {
                            let unique: BTreeSet<&String> = t.iter().collect();
                            if unique.len() != t.len() {
                                e(format!(
                                    "{step}: duplicate exe_targets (a target must name exactly one emitted executable)"
                                ));
                            }
                            builds.insert(step.clone(), t);
                        }
                        Err(x) => e(x),
                    },
                    Some(_) => e(format!("{step}: cargo_json must be true when present")),
                }
                if let Some(x) = p.get("exe") {
                    match Obj::new(format!("{step}.exe"), x)
                        .and_then(|o| o.exact(&["step", "target"]).map(|_| o))
                    {
                        Ok(o) => {
                            let (bs, bt) =
                                (o.str("step").unwrap_or(""), o.str("target").unwrap_or(""));
                            match builds.get(bs) {
                                Some(t) if t.iter().any(|x| x == bt) => {}
                                Some(_) => e(format!(
                                    "{step}: exe target `{bt}` not declared by build `{bs}`"
                                )),
                                None => e(format!(
                                    "{step}: exe refers to `{bs}`, which is not an earlier cargo_json build"
                                )),
                            }
                        }
                        Err(x) => e(x),
                    }
                }
                for x in exe_relation_errors(&step, p) {
                    e(x);
                }
                if p.get("min_tests").is_some() {
                    if let Err(x) = po.u64("min_tests") {
                        e(x);
                    }
                }
                if p.get("stdout_contains").is_some() {
                    if let Err(x) = po.strings("stdout_contains") {
                        e(x);
                    }
                }
                if let Some(rs) = p.get("runtime_inputs") {
                    match serde_json::from_value::<Vec<crate::runtime::Invocation>>(rs.clone()) {
                        Ok(rs) => {
                            let ids: BTreeSet<_> = rs.iter().map(|r| &r.id).collect();
                            if ids.len() != rs.len() {
                                e(format!("{step}: duplicate runtime invocation ids"));
                            }
                        }
                        Err(x) => e(format!("{step}: malformed runtime_inputs: {x}")),
                    }
                }
                steps.push(step);
            }
        }
        _ => e("map.processes: not a non-empty array".into()),
    }
    let has_step = |s: &str| steps.iter().any(|x| x == s);
    let producer = map["output_producer"]["step"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    if !has_step(&producer) || map["output_producer"].as_object().map(|o| o.len()) != Some(1) {
        e(format!(
            "map.output_producer: `{producer}` is not exactly one known step"
        ));
    }
    // Expectations.
    let exp = &map["expected"];
    match Obj::new("map.expected", exp) {
        Ok(o) => {
            if let Err(x) = o.exact(&[
                "closure_package",
                "closure_step",
                "sdk_step",
                "sdk_manifest_sha256",
                "sdk_files",
                "sdk_links",
                "sdk_version",
                "node_version",
                "stdout_checks",
            ]) {
                e(x);
            }
            for k in ["closure_package", "sdk_version", "node_version"] {
                if let Err(x) = o.str(k) {
                    e(x);
                }
            }
            for k in ["closure_step", "sdk_step"] {
                match o.str(k) {
                    Ok(s) if has_step(s) => {}
                    Ok(s) => e(format!("map.expected.{k}: unknown step `{s}`")),
                    Err(x) => e(x),
                }
            }
            if !o.str("sdk_manifest_sha256").map(is_hex64).unwrap_or(false) {
                e("map.expected.sdk_manifest_sha256: not a sha256".into());
            }
            for k in ["sdk_files", "sdk_links"] {
                if let Err(x) = o.u64(k) {
                    e(x);
                }
            }
            match o.arr("stdout_checks") {
                Ok(list) => {
                    let mut seen = BTreeSet::new();
                    for c in list {
                        let ok = Obj::new("stdout_check", c).ok().and_then(|co| {
                            let step = co.str("step").ok()?;
                            let n = c.as_object()?.len();
                            let rule = (co.str("contains").is_ok() as u8)
                                + (co.str("equals").is_ok() as u8);
                            (n == 2 && rule == 1 && has_step(step)).then_some(step.to_owned())
                        });
                        match ok {
                            Some(step) if seen.insert(step.clone()) => {}
                            Some(step) => e(format!(
                                "map.expected.stdout_checks: duplicate step `{step}`"
                            )),
                            None => e(format!("map.expected.stdout_checks: malformed entry {c}")),
                        }
                    }
                }
                Err(x) => e(x),
            }
        }
        Err(x) => e(x),
    }
    // Artifacts: exactly the fixed checked set.
    let mut artifacts: Vec<(String, Repr)> = Vec::new();
    match map["artifacts"].as_array() {
        Some(list) => {
            for a in list {
                let parsed = Obj::new("artifact", a).ok().and_then(|o| {
                    o.exact(&["name", "representation"]).ok()?;
                    Some((
                        o.str("name").ok()?.to_owned(),
                        Repr::parse(o.str("representation").ok()?)?,
                    ))
                });
                match parsed {
                    Some((name, repr)) => {
                        if artifacts.iter().any(|(n, _)| *n == name) {
                            e(format!("duplicate artifact `{name}`"));
                        }
                        match REQUIRED_ARTIFACTS.iter().find(|(n, _)| *n == name) {
                            Some((_, want)) if *want == repr => {}
                            Some((_, want)) => e(format!(
                                "artifact `{name}`: representation {repr:?}, its checker requires {want:?}"
                            )),
                            None => e(format!("artifact `{name}` has no semantic checker")),
                        }
                        artifacts.push((name, repr));
                    }
                    None => e(format!(
                        "map.artifacts: malformed entry {a} (unknown representation or keys)"
                    )),
                }
            }
            for (name, _) in REQUIRED_ARTIFACTS {
                if !artifacts.iter().any(|(n, _)| n == name) {
                    e(format!("required artifact `{name}` is not declared"));
                }
            }
        }
        None => e("map.artifacts: not an array".into()),
    }
    // Fixture sources pinned through the source manifest.
    let mut fixture_sources = BTreeMap::new();
    match Obj::new("map.fixture_sources", &map["fixture_sources"]) {
        Ok(o) => {
            if let Err(x) = o.exact(&["candidate-requirements.json", "authority.json"]) {
                e(x);
            }
            for k in ["candidate-requirements.json", "authority.json"] {
                match o.str(k) {
                    Ok(p) => {
                        fixture_sources.insert(k.to_owned(), p.to_owned());
                    }
                    Err(x) => e(x),
                }
            }
        }
        Err(x) => e(x),
    }
    // Controls.
    let mut controls = Vec::new();
    match map["controls"].as_array() {
        Some(list) => {
            for c in list {
                let parsed = (|| -> Result<ControlSpec, String> {
                    let o = Obj::new("control", c)?;
                    let keys: Vec<&str> =
                        c.as_object().unwrap().keys().map(String::as_str).collect();
                    if keys.iter().any(|k| {
                        ![
                            "name",
                            "scratch",
                            "outcome",
                            "preserved",
                            "write_failure",
                            "authority_source",
                            "stage",
                        ]
                        .contains(k)
                    }) {
                        return Err(format!("control: unknown keys {keys:?}"));
                    }
                    let outcome = o.str("outcome")?.to_owned();
                    if !["failed", "incomplete", "passed"].contains(&outcome.as_str()) {
                        return Err(format!(
                            "control outcome `{outcome}` is not failed|incomplete|passed"
                        ));
                    }
                    let write_failure = match o.opt("write_failure") {
                        None => None,
                        Some(_) => Some(o.str("write_failure")?.to_owned()),
                    };
                    let authority_source = o
                        .opt("authority_source")
                        .map(|_| o.str("authority_source").map(str::to_owned))
                        .transpose()?;
                    if authority_source.is_some() && o.str("stage")? != "consumer_refused" {
                        return Err(
                            "authority override requires an actual consumer-refusal scenario"
                                .into(),
                        );
                    }
                    let stage = match o.opt("stage") {
                        None => "delivered_and_consumed".to_owned(),
                        Some(_) => o.str("stage")?.to_owned(),
                    };
                    if ![
                        "delivered_and_consumed",
                        "delivery_refused",
                        "consumer_refused",
                        "native_error",
                    ]
                    .contains(&stage.as_str())
                    {
                        return Err(format!(
                            "control stage `{stage}` is not delivered_and_consumed|delivery_refused|consumer_refused|native_error"
                        ));
                    }
                    // A refusal control passes when the expected refusal was observed; a consumed
                    // control's bundle is a failure/incomplete control by definition here.
                    if outcome == "passed" && stage == "delivered_and_consumed" {
                        return Err(format!(
                            "control outcome `{outcome}` does not fit stage `{stage}`"
                        ));
                    }
                    let preserved = o.strings("preserved")?;
                    if stage != "delivered_and_consumed" {
                        if let Some(p) = preserved.iter().find(|p| {
                            ["episode-after.json", "consumption.json", "revisions.json"]
                                .contains(&p.as_str())
                        }) {
                            return Err(format!(
                                "control stage `{stage}` stops before consumption but preserves `{p}`"
                            ));
                        }
                    }
                    Ok(ControlSpec {
                        name: o.str("name")?.to_owned(),
                        scratch: o.str("scratch")?.to_owned(),
                        outcome,
                        preserved,
                        write_failure,
                        authority_source,
                        stage,
                    })
                })();
                match parsed {
                    Ok(cs) if controls.iter().any(|x: &ControlSpec| x.name == cs.name) => {
                        e(format!("duplicate control `{}`", cs.name))
                    }
                    Ok(cs) if cs.preserved.is_empty() => {
                        e(format!("control `{}` preserves nothing", cs.name))
                    }
                    Ok(cs) => controls.push(cs),
                    Err(x) => e(x),
                }
            }
        }
        None => e("map.controls: not an array".into()),
    }
    if !errs.is_empty() {
        return Err(errs);
    }
    // Checks: exactly the derived identities, in a dependency-respecting order, with every
    // intrinsic dependency declared.
    let want = derived(&map, &producer, &artifacts, &controls);
    let mut checks: Vec<(String, Vec<String>)> = Vec::new();
    match map["checks"].as_array() {
        Some(list) => {
            for c in list {
                let parsed = Obj::new("check", c).ok().and_then(|o| {
                    o.exact(&["id", "depends_on"]).ok()?;
                    Some((o.str("id").ok()?.to_owned(), o.strings("depends_on").ok()?))
                });
                let Some((id, deps)) = parsed else {
                    errs.push(format!("map.checks: malformed entry {c}"));
                    continue;
                };
                if checks.iter().any(|(x, _)| *x == id) {
                    errs.push(format!("duplicate check identity `{id}`"));
                    continue;
                }
                for d in &deps {
                    if !checks.iter().any(|(x, _)| x == d) {
                        errs.push(format!("check `{id}`: dependency `{d}` is unresolved or not earlier in the list"));
                    }
                }
                match want.iter().find(|(w, _)| *w == id) {
                    None => errs.push(format!(
                        "check `{id}` (kind `{}`) is unknown to the fixed schema",
                        kind_of(&id)
                    )),
                    Some((_, intrinsic)) => {
                        for d in intrinsic {
                            if !deps.contains(d) {
                                errs.push(format!(
                                    "check `{id}`: intrinsic dependency `{d}` not declared"
                                ));
                            }
                        }
                    }
                }
                checks.push((id, deps));
            }
        }
        None => errs.push("map.checks: not an array".into()),
    }
    for (w, _) in &want {
        if !checks.iter().any(|(x, _)| x == w) {
            errs.push(format!("required check `{w}` is missing from map.checks"));
        }
    }
    if !errs.is_empty() {
        return Err(errs);
    }
    Ok(ValidatedSpec {
        map,
        sha256,
        producer,
        artifacts,
        controls,
        frozen_authority_source: fixture_sources["authority.json"].clone(),
        fixture_sources,
        checks,
    })
}

/// The canonical `checks` list for a map, for authors: the derived identities with their
/// intrinsic dependencies, in schema order.
pub fn derive_checks(map: &Value) -> Result<Value, Vec<String>> {
    let mut m = map.clone();
    m["checks"] = Value::Array(vec![]);
    let errs = match validate(m.clone(), String::new()) {
        Ok(_) => return Ok(Value::Array(vec![])),
        Err(errs) => errs,
    };
    // The only acceptable errors are the missing checks themselves.
    if errs.iter().any(|x| !x.starts_with("required check")) {
        return Err(errs);
    }
    let producer = m["output_producer"]["step"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    let artifacts: Vec<(String, Repr)> = REQUIRED_ARTIFACTS
        .iter()
        .map(|(n, r)| ((*n).to_owned(), *r))
        .collect();
    let controls: Vec<ControlSpec> = m["controls"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| ControlSpec {
            name: c["name"].as_str().unwrap_or("").into(),
            scratch: String::new(),
            outcome: String::new(),
            preserved: vec![],
            write_failure: None,
            authority_source: None,
            stage: String::new(),
        })
        .collect();
    Ok(Value::Array(
        derived(&m, &producer, &artifacts, &controls)
            .into_iter()
            .map(|(id, deps)| serde_json::json!({"id": id, "depends_on": deps}))
            .collect(),
    ))
}

/// Pinned identity of a file for approval comparisons.
pub fn pin(path: &Path, expect: &str) -> Result<(), String> {
    match file_sha(path) {
        Ok(got) if got == expect => Ok(()),
        Ok(got) => Err(format!(
            "{} has sha256 {got}, approved {expect}",
            path.display()
        )),
        Err(e) => Err(e),
    }
}
