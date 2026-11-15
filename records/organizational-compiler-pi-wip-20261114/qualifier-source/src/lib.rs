//! Campaign-local evidence qualifier (PLAN-R4). Not a runner, not a production service.
//!
//! The custody runner (`custody.py`) owns processes and records. This crate decides whether a
//! plan and the records and retained outputs it produced qualify as campaign evidence, against a
//! root-approved specification (obligation map v2, [`spec`]). Qualification means every required
//! check identity completed successfully ([`ledger`]); missing, malformed, failed, blocked and
//! unperformed checks all prevent it. Domain interpretation is delegated to the existing owners
//! ([`outputs`]): the episode core reads stored states, the canonical experiment binding and the
//! pure fixture consumer recompute the delivery and transition, and the delivery adapter's pure
//! verifier checks the native observation. That recomputation is a consistency check, not a new
//! native execution or authority transition.
//!
//! Limits: observation, not prevention. A hostile concurrent writer between digest time and
//! check time is not excluded. No OS hermeticity is claimed.

pub mod base;
pub mod ledger;
pub mod outputs;
pub mod runtime;
pub mod spec;
pub mod strict;

pub use base::{
    build_manifest, closure, embedded_includes, file_sha, make_plan, sha256_hex, verify_sdk, walk,
    RunVars, MANIFEST_SCHEMA,
};
pub use ledger::{Class, Finding, Ledger, Qualified, Status};
pub use spec::{load_spec, ValidatedSpec, MAP_SCHEMA};

use base::{process_step, required_digests, same_set, subst, subst_value};
use ledger::Defects;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use strict::{is_hex64, parse_iso, Obj};

pub const RUN_SCHEMA: &str = "manual-orchestration.custody.run.v1";
pub const SUMMARY_SCHEMA: &str = "manual-orchestration.custody.summary.v1";

/// Everything both modes compare. Every identity is an approval supplied by the root.
pub struct Inputs<'a> {
    pub map_path: &'a Path,
    pub plan_path: &'a Path,
    pub manifest_path: &'a Path,
    pub expect_map_sha256: &'a str,
    pub expect_plan_sha256: &'a str,
    pub expect_manifest_sha256: &'a str,
    pub run: RunVars,
}

/// The result of one qualifier invocation. Qualified only through [`Evaluation::qualified`].
pub struct Evaluation {
    pub spec_errors: Vec<String>,
    pub ledger: Option<Ledger>,
    pub facts: Map<String, Value>,
}

impl Evaluation {
    fn spec_failed(errors: Vec<String>) -> Self {
        Evaluation {
            spec_errors: errors,
            ledger: None,
            facts: Map::new(),
        }
    }
    pub fn qualified(&self) -> Option<Qualified> {
        if !self.spec_errors.is_empty() {
            return None;
        }
        self.ledger.as_ref()?.conclude().ok()
    }
    pub fn is_qualified(&self) -> bool {
        self.qualified().is_some()
    }
    pub fn status(&self, id: &str) -> Option<&Status> {
        self.ledger.as_ref()?.status(id)
    }
    pub fn incomplete(&self) -> Vec<String> {
        self.ledger
            .as_ref()
            .map(Ledger::incomplete)
            .unwrap_or_default()
    }
    pub fn passed(&self, id: &str) -> bool {
        matches!(self.status(id), Some(Status::Passed))
    }
    pub fn failed(&self, id: &str) -> bool {
        matches!(self.status(id), Some(Status::Failed(_)))
    }
    pub fn blocked(&self, id: &str) -> bool {
        matches!(self.status(id), Some(Status::Blocked(_)))
    }
    /// Whether check `id` failed with a finding of `class`.
    pub fn failed_with(&self, id: &str, class: Class) -> bool {
        matches!(self.status(id), Some(Status::Failed(f)) if f.iter().any(|x| x.class == class))
    }
    pub fn to_json(&self, mode: &str) -> Value {
        match &self.ledger {
            Some(l) => {
                let mut v = l.to_json(mode, &self.facts);
                v["spec_errors"] = json!(self.spec_errors);
                v["qualified"] = json!(self.is_qualified());
                v
            }
            None => {
                json!({"schema": "org-evidence-gate-report/v2", "mode": mode, "qualified": false, "spec_errors": self.spec_errors, "checks": [], "incomplete": ["spec"], "facts": self.facts})
            }
        }
    }
}

/// The source manifest after its check passed: path -> sha256, and the map's closure.
pub struct SourceManifest {
    pub entries: BTreeMap<String, String>,
    pub closure: BTreeSet<String>,
}

fn pins(i: &Inputs, d: &mut Defects) -> Option<()> {
    for (what, path, want) in [
        ("plan", i.plan_path, i.expect_plan_sha256),
        ("source manifest", i.manifest_path, i.expect_manifest_sha256),
    ] {
        match file_sha(path) {
            Ok(got) if got == want => {}
            Ok(got) => d.mismatch(
                what,
                "sha256",
                want,
                got,
                format!("{} is not the approved {what}", path.display()),
            ),
            Err(e) => d.missing(what, e),
        }
    }
    if i.run.map_sha256 != i.expect_map_sha256 || i.run.manifest_sha256 != i.expect_manifest_sha256
    {
        d.mismatch(
            "run vars",
            "map/manifest sha256",
            i.expect_map_sha256,
            &i.run.map_sha256,
            "run-variable identities differ from the approvals",
        );
    }
    Some(())
}

/// The manifest versus the filesystem and the map: exact file-set completeness for the scoped
/// roots, every digest, every embedded fixture, the pinned externals and files that must stay absent.
fn check_manifest(spec: &ValidatedSpec, path: &Path, d: &mut Defects) -> Option<SourceManifest> {
    const A: &str = "source-manifest";
    let bytes = d.take(A, fs::read(path).map_err(|e| e.to_string()))?;
    let m = d.take(A, strict::parse(&bytes))?;
    let o = d.take(A, Obj::new(A, &m))?;
    d.take(
        A,
        o.exact(&[
            "schema",
            "map_sha256",
            "roots",
            "excluded_dir_names",
            "entries",
            "externals",
            "absent_expected",
        ]),
    )?;
    if o.str("schema").ok() != Some(MANIFEST_SCHEMA) {
        d.mismatch(
            A,
            "schema",
            MANIFEST_SCHEMA,
            o.str("schema").unwrap_or(""),
            "",
        );
    }
    if o.str("map_sha256").ok() != Some(spec.sha256.as_str()) {
        d.mismatch(
            A,
            "map_sha256",
            &spec.sha256,
            o.str("map_sha256").unwrap_or(""),
            "the source manifest is not bound to this map",
        );
    }
    let (clo, unresolved) = match closure(&spec.map) {
        Ok(c) => c,
        Err(e) => {
            d.missing("closure", e);
            return None;
        }
    };
    for u in unresolved {
        d.add(
            "closure",
            "include",
            Class::Missing,
            "resolved literal include",
            "unresolved",
            u,
        );
    }
    let mut entries: BTreeMap<String, String> = BTreeMap::new();
    for (n, e) in d.take(A, o.arr("entries"))?.iter().enumerate() {
        let eo = d.take(A, Obj::new(format!("entries[{n}]"), e))?;
        d.take(A, eo.exact(&["path", "length", "sha256"]))?;
        let (p, sha, len) = (
            d.take(A, eo.str("path"))?.to_owned(),
            d.take(A, eo.str("sha256"))?.to_owned(),
            d.take(A, eo.u64("length"))?,
        );
        if entries.insert(p.clone(), sha.clone()).is_some() {
            d.mismatch(A, "entries", "unique path", &p, "duplicate manifest entry");
        }
        match fs::read(&p) {
            Ok(b) if sha256_hex(&b) == sha && b.len() as u64 == len => {}
            Ok(b) => d.mismatch(
                A,
                &p,
                &sha,
                sha256_hex(&b),
                "source bytes differ from the manifest",
            ),
            Err(err) => d.missing(&p, err.to_string()),
        }
    }
    let listed: BTreeSet<String> = entries.keys().cloned().collect();
    let (not_listed, extra) = same_set(&clo, &listed);
    for p in not_listed {
        d.mismatch(
            A,
            "entries",
            "listed",
            "absent",
            format!("required by the map's closure but not in the manifest: {p}"),
        );
    }
    for p in extra {
        d.mismatch(
            A,
            "entries",
            "absent",
            "listed",
            format!("in the manifest but not in the map's closure: {p}"),
        );
    }
    if m["roots"] != spec.map["required_roots"] {
        d.mismatch(
            A,
            "roots",
            &spec.map["required_roots"],
            &m["roots"],
            "manifest roots differ from the map's required roots",
        );
    }
    let expected: BTreeMap<String, runtime::ObjectBinding> = d.take(
        A,
        serde_json::from_value(spec.map["external_inputs"].clone()).map_err(|e| e.to_string()),
    )?;
    let observed: Value = json!(expected.iter().map(|(p, pin)| json!({"path":p,"pinned":pin,"observed":runtime::ObjectBinding::observe(Path::new(p)).ok()})).collect::<Vec<_>>());
    if m["externals"] != observed {
        d.mismatch(
            A,
            "externals",
            &observed,
            &m["externals"],
            "manifest object observations disagree",
        );
    }
    for (p, pin) in &expected {
        match runtime::ObjectBinding::observe(Path::new(p)) {
            Ok(got) if &got == pin => {}
            Ok(got) => d.mismatch(
                "external",
                p,
                format!("{pin:?}"),
                format!("{got:?}"),
                "pinned object differs",
            ),
            Err(err) => d.missing(p, err),
        }
    }
    for p in base::strings(&spec.map, "absent_expected") {
        if Path::new(&p).symlink_metadata().is_ok() {
            d.mismatch(
                "absent_expected",
                &p,
                "absent",
                "present",
                "the map pins this configuration file's absence",
            );
        }
    }
    Some(SourceManifest {
        entries,
        closure: clo,
    })
}

fn check_runtime(
    spec: &ValidatedSpec,
    process: &Value,
    vars: &BTreeMap<String, String>,
    m: &SourceManifest,
    captured: Option<&BTreeMap<String, runtime::ObjectBinding>>,
    d: &mut Defects,
) {
    let objects: BTreeMap<String, runtime::ObjectBinding> =
        match serde_json::from_value(spec.map["external_inputs"].clone()) {
            Ok(o) => o,
            Err(e) => {
                d.malformed("runtime", e.to_string());
                return;
            }
        };
    let required: BTreeSet<String> = required_digests(&spec.map, vars, &m.closure, process)
        .into_iter()
        .collect();
    for v in process
        .get("runtime_inputs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let resolved = subst_value(v, vars);
        let r: runtime::Invocation = match serde_json::from_value(resolved) {
            Ok(r) => r,
            Err(e) => {
                d.malformed("runtime", e.to_string());
                continue;
            }
        };
        if let Some(source) = &r.source {
            if !m.closure.contains(source) || !required.contains(source) {
                d.malformed(
                    "runtime",
                    format!("descendant declaration source outside closure: {source}"),
                );
            }
        } else if json!(r.argv) != subst_value(&process["argv"], vars)
            || r.cwd != subst(process["cwd"].as_str().unwrap(), vars)
            || r.path_env.as_deref() != process["env"].get("PATH").and_then(Value::as_str)
        {
            d.malformed(
                "runtime",
                "top-level invocation contradicts process argv/context",
            );
        }
        for p in r.links.iter().chain(std::iter::once(&r.endpoint)) {
            if !required.contains(p) {
                d.malformed(
                    "runtime",
                    format!("resolution object not required by process: {p}"),
                );
            }
        }
        if let Err(e) = r.validate(&objects, captured) {
            d.malformed("runtime", format!("{}: {e}", r.id));
        }
    }
}

fn exe_token(text: &str) -> Option<(String, String)> {
    let inner = text.strip_prefix("{exe:")?.strip_suffix('}')?;
    let (step, target) = inner.split_once(':')?;
    Some((step.to_owned(), target.to_owned()))
}

/// Preflight: the plan is exactly the map's plan before any command is admitted as qualified
/// execution. Its check identities derive from the validated specification.
pub fn preflight(i: &Inputs) -> Evaluation {
    let spec = match load_spec(i.map_path, i.expect_map_sha256) {
        Ok(s) => s,
        Err(e) => return Evaluation::spec_failed(e),
    };
    let mut required: Vec<(String, Vec<String>)> = vec![
        ("pins".into(), vec![]),
        ("source_manifest".into(), vec!["pins".into()]),
        ("run_dirs_absent".into(), vec![]),
        ("plan_shape".into(), vec!["pins".into()]),
    ];
    for p in spec.processes() {
        required.push((
            format!("plan_step:{}", p["step"].as_str().unwrap()),
            vec!["plan_shape".into(), "source_manifest".into()],
        ));
    }
    let mut l = Ledger::new(&required);
    let mut facts = Map::new();
    l.run("pins", |d| pins(i, d));
    let manifest = l.run("source_manifest", |d| {
        check_manifest(&spec, i.manifest_path, d)
    });
    l.run("run_dirs_absent", |d| {
        for dir in [&i.run.run_dir, &i.run.records_dir] {
            if Path::new(dir).symlink_metadata().is_ok() {
                d.mismatch(
                    "run directory",
                    dir,
                    "absent",
                    "exists",
                    "each invocation owns new directories",
                );
            }
        }
        Some(())
    });
    let vars = i.run.vars();
    let plan = l.run("plan_shape", |d| {
        let bytes = d.take("plan", fs::read(i.plan_path).map_err(|e| e.to_string()))?;
        let plan = d.take("plan", strict::parse(&bytes))?;
        let o = d.take("plan", Obj::new("plan", &plan))?;
        d.take("plan", o.exact(&["env_clear", "steps"]))?;
        if o.bool("env_clear").ok() != Some(true) {
            d.mismatch(
                "plan",
                "env_clear",
                true,
                &plan["env_clear"],
                "the plan must clear the environment",
            );
        }
        let names: Vec<String> = d
            .take("plan", o.arr("steps"))?
            .iter()
            .map(|s| s["name"].as_str().unwrap_or("").to_owned())
            .collect();
        let want: Vec<String> = spec
            .processes()
            .iter()
            .map(|p| p["step"].as_str().unwrap().to_owned())
            .collect();
        if names != want {
            d.mismatch(
                "plan",
                "steps",
                format!("{want:?}"),
                format!("{names:?}"),
                "plan steps differ from the map's required processes (identity and order)",
            );
        }
        Some(plan)
    });
    let externals: BTreeSet<String> = spec.map["external_inputs"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let run_outputs: BTreeSet<String> = base::strings(&spec.map, "run_outputs")
        .iter()
        .map(|p| subst(p, &vars))
        .collect();
    for p in spec.processes() {
        let name = p["step"].as_str().unwrap();
        l.run(&format!("plan_step:{name}"), |d| {
            let (plan, m) = (plan.as_ref()?, manifest.as_ref()?);
            let step = plan["steps"]
                .as_array()?
                .iter()
                .find(|s| s["name"] == json!(name))?;
            let so = d.take("plan", Obj::new(format!("plan step {name}"), step))?;
            for k in step.as_object()?.keys() {
                if ![
                    "name",
                    "argv",
                    "cwd",
                    "env",
                    "digest",
                    "timeout",
                    "cargo_json",
                ]
                .contains(&k.as_str())
                {
                    d.malformed("plan", format!("{name}: unknown step key `{k}`"));
                }
            }
            let want = process_step(&spec.map, &vars, &m.closure, p);
            for key in ["argv", "cwd", "timeout", "env"] {
                if step.get(key) != want.get(key) {
                    d.mismatch(
                        "plan",
                        &format!("{name}.{key}"),
                        &want[key],
                        step.get(key).unwrap_or(&Value::Null),
                        "differs from the map",
                    );
                }
            }
            if step.get("cargo_json") != want.get("cargo_json") {
                d.mismatch(
                    "plan",
                    &format!("{name}.cargo_json"),
                    &want["cargo_json"],
                    step.get("cargo_json").unwrap_or(&Value::Null),
                    "",
                );
            }
            let digests = d.take("plan", so.strings("digest"))?;
            let set: BTreeSet<String> = digests.iter().cloned().collect();
            if set.len() != digests.len() {
                d.malformed("plan", format!("{name}: duplicate digest bindings"));
            }
            if digests.is_empty() {
                d.add(
                    "plan",
                    &format!("{name}.digest"),
                    Class::Missing,
                    "per-step digest bindings",
                    "none",
                    "",
                );
            }
            for r in required_digests(&spec.map, &vars, &m.closure, p) {
                if !set.contains(&r) {
                    d.add(
                        "plan",
                        &format!("{name}.digest"),
                        Class::Missing,
                        &r,
                        "unbound",
                        "required input not bound",
                    );
                }
            }
            for g in &digests {
                if !(m.closure.contains(g)
                    || externals.contains(g)
                    || run_outputs.contains(g)
                    || exe_token(g).is_some())
                {
                    d.mismatch(
                        "plan",
                        &format!("{name}.digest"),
                        "a required class",
                        g,
                        "bound path is in no required class",
                    );
                }
            }
            for g in &digests {
                if m.closure.contains(g)
                    && !matches!(
                        runtime::ObjectBinding::observe(Path::new(g)),
                        Ok(runtime::ObjectBinding::File { .. })
                    )
                {
                    d.malformed(
                        "plan",
                        format!("required source regular-file binding is unproducible: {g}"),
                    );
                }
            }
            check_runtime(&spec, p, &vars, m, None, d);
            Some(())
        });
    }
    facts.insert("steps".into(), json!(spec.processes().len()));
    Evaluation {
        spec_errors: vec![],
        ledger: Some(l),
        facts,
    }
}

/// One custody record after its check passed.
#[derive(Debug, Clone)]
pub struct Rec {
    pub value: Value,
    pub started: f64,
    pub ended: f64,
    pub stdout: Vec<u8>,
    pub executables: Vec<(String, String, String)>,
}

#[derive(Debug, Clone)]
struct Row {
    record_sha256: String,
}

fn check_summary(
    spec: &ValidatedSpec,
    i: &Inputs,
    records: &Path,
    d: &mut Defects,
) -> Option<BTreeMap<String, Row>> {
    const A: &str = "summary.json";
    let bytes = d.take(A, fs::read(records.join(A)).map_err(|e| e.to_string()))?;
    let v = d.take(A, strict::parse(&bytes))?;
    let o = d.take(A, Obj::new(A, &v))?;
    d.take(
        A,
        o.exact(&[
            "schema",
            "plan",
            "plan_sha256",
            "status",
            "steps",
            "not_run",
        ]),
    )?;
    if d.take(A, o.str("schema"))? != SUMMARY_SCHEMA {
        d.mismatch(
            A,
            "schema",
            SUMMARY_SCHEMA,
            o.str("schema").unwrap_or(""),
            "",
        );
    }
    let plan_sha = d.take(A, o.str("plan_sha256"))?;
    if plan_sha != i.expect_plan_sha256 {
        d.mismatch(
            A,
            "plan_sha256",
            i.expect_plan_sha256,
            plan_sha,
            "the custody summary is for another plan",
        );
    }
    if d.take(A, o.str("status"))? != "passed" {
        d.failed(A, "status", "passed", o.str("status").unwrap_or(""), "");
    }
    if !d.take(A, o.arr("not_run"))?.is_empty() {
        d.failed(
            A,
            "not_run",
            "[]",
            &v["not_run"],
            "every step must have run",
        );
    }
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    let mut names: Vec<String> = Vec::new();
    for (n, r) in d.take(A, o.arr("steps"))?.iter().enumerate() {
        let ro = d.take(A, Obj::new(format!("steps[{n}]"), r))?;
        d.take(
            A,
            ro.exact(&["name", "outcome", "record", "record_sha256", "leftovers"]),
        )?;
        let name = d.take(A, ro.str("name"))?.to_owned();
        if d.take(A, ro.str("outcome"))? != "0" {
            d.failed(
                A,
                &format!("{name}.outcome"),
                "0",
                ro.str("outcome").unwrap_or(""),
                "",
            );
        }
        if d.take(A, ro.u64("leftovers"))? != 0 {
            d.failed(A, &format!("{name}.leftovers"), 0, &r["leftovers"], "");
        }
        let want_rec = records.join(format!("{name}.json")).display().to_string();
        if d.take(A, ro.str("record"))? != want_rec {
            d.mismatch(
                A,
                &format!("{name}.record"),
                &want_rec,
                ro.str("record").unwrap_or(""),
                "",
            );
        }
        let sha = d.take(A, ro.str("record_sha256"))?.to_owned();
        if !is_hex64(&sha) {
            d.malformed(A, format!("{name}.record_sha256 is not a sha256"));
        }
        if rows
            .insert(name.clone(), Row { record_sha256: sha })
            .is_some()
        {
            d.mismatch(
                A,
                "steps",
                "unique step identities",
                &name,
                "duplicate summary row",
            );
        }
        names.push(name);
    }
    let want: Vec<String> = spec
        .processes()
        .iter()
        .map(|p| p["step"].as_str().unwrap().to_owned())
        .collect();
    let (have_set, want_set): (BTreeSet<&String>, BTreeSet<&String>) =
        (names.iter().collect(), want.iter().collect());
    if have_set != want_set || names != want {
        d.mismatch(
            A,
            "steps",
            format!("{want:?}"),
            format!("{names:?}"),
            "summary step identities differ from the map's (as a set and in order)",
        );
    }
    Some(rows)
}

/// `test result:` lines from libtest/rustdoc stdout. Other stdout lines are permitted
/// non-evidence text; a result line that does not parse strictly is malformed evidence.
fn test_results(stdout: &str) -> Result<(u64, u64, usize), String> {
    let (mut passed, mut failed, mut lines) = (0u64, 0u64, 0usize);
    for line in stdout.lines().filter(|l| l.starts_with("test result:")) {
        lines += 1;
        let rest = line
            .strip_prefix("test result: ")
            .ok_or_else(|| format!("malformed result line `{line}`"))?;
        let (status, rest) = rest
            .split_once(". ")
            .ok_or_else(|| format!("malformed result line `{line}`"))?;
        if status != "ok" && status != "FAILED" {
            return Err(format!("unknown result status `{status}`"));
        }
        let parts: Vec<&str> = rest.split("; ").collect();
        let labels = ["passed", "failed", "ignored", "measured", "filtered out"];
        if parts.len() != 6 || !parts[5].starts_with("finished in ") {
            return Err(format!("malformed result line `{line}`"));
        }
        let mut nums = [0u64; 5];
        for (k, label) in labels.iter().enumerate() {
            let (n, l) = parts[k]
                .split_once(' ')
                .ok_or_else(|| format!("malformed count in `{line}`"))?;
            if l != *label {
                return Err(format!("expected `{label}` in `{line}`"));
            }
            nums[k] = n
                .parse()
                .map_err(|_| format!("malformed count `{n}` in `{line}`"))?;
        }
        if status == "FAILED" && nums[1] == 0 {
            return Err(format!("FAILED status with zero failures in `{line}`"));
        }
        passed += nums[0];
        failed += nums[1];
    }
    Ok((passed, failed, lines))
}

/// Executables re-derived from cargo's own JSON messages, together with the build's terminal
/// semantics. Every stdout line of a `--message-format=json` build is a required message: a line
/// that is not a JSON object with a string `reason` is malformed (cargo's human text goes to
/// stderr, which is not evidence here). A successful build is exactly one `build-finished`
/// message with boolean `success: true`, and it is the final message: a missing, false,
/// wrong-typed, duplicate or contradictory terminal, or any artifact/diagnostic/build-script
/// message after it, refuses. Other message classes are not validated beyond their decoding,
/// and unrelated fields or future cargo versions are not constrained. This evaluator is called
/// only for builds whose custody record already reports a zero exit status, so the terminal
/// `success` must agree with that outcome.
fn cargo_executables(stdout: &[u8]) -> Result<Vec<(String, String)>, String> {
    let text = std::str::from_utf8(stdout).map_err(|e| e.to_string())?;
    let mut found = Vec::new();
    let mut terminal: Option<usize> = None;
    let mut last = 0usize;
    for (n, line) in text.lines().enumerate() {
        last = n;
        let m = strict::parse(line.as_bytes())
            .map_err(|e| format!("cargo message line {}: {e}", n + 1))?;
        let reason = m
            .get("reason")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cargo message line {}: no string `reason`", n + 1))?;
        if let Some(first) = terminal {
            return Err(format!(
                "cargo message line {}: `{reason}` follows the terminal build-finished message on line {}",
                n + 1,
                first + 1
            ));
        }
        if reason == "build-finished" {
            match m.get("success") {
                Some(Value::Bool(true)) => terminal = Some(n),
                Some(Value::Bool(false)) => {
                    return Err(format!(
                        "cargo message line {}: build-finished reports success false",
                        n + 1
                    ))
                }
                Some(other) => {
                    return Err(format!(
                        "cargo message line {}: build-finished `success` is not a boolean: {other}",
                        n + 1
                    ))
                }
                None => {
                    return Err(format!(
                        "cargo message line {}: build-finished carries no `success`",
                        n + 1
                    ))
                }
            }
            continue;
        }
        if reason == "compiler-artifact" && m["profile"]["test"] == json!(true) {
            if let Some(path) = m["executable"].as_str() {
                let name = m["target"]["name"]
                    .as_str()
                    .ok_or_else(|| format!("cargo message line {}: no target name", n + 1))?;
                found.push((name.to_owned(), path.to_owned()));
            }
        }
    }
    match terminal {
        Some(t) if t == last => Ok(found),
        Some(_) => unreachable!("a message after the terminal returns above"),
        None => Err("cargo output has no terminal build-finished message (empty or truncated build evidence)".to_owned()),
    }
}

#[allow(clippy::too_many_arguments)]
fn check_record(
    spec: &ValidatedSpec,
    i: &Inputs,
    records: &Path,
    m: &SourceManifest,
    rows: &BTreeMap<String, Row>,
    done: &BTreeMap<String, Rec>,
    process: &Value,
    d: &mut Defects,
) -> Option<Rec> {
    let name = process["step"].as_str().unwrap();
    let vars = i.run.vars();
    let a = format!("{name}.json");
    let a = a.as_str();
    let bytes = match fs::read(records.join(a)) {
        Ok(b) => b,
        Err(e) => {
            d.missing(a, e.to_string());
            return None;
        }
    };
    let row = rows.get(name)?;
    if sha256_hex(&bytes) != row.record_sha256 {
        d.mismatch(
            a,
            "record_sha256",
            &row.record_sha256,
            sha256_hex(&bytes),
            "record bytes differ from the summary row",
        );
    }
    let rec = d.take(a, strict::parse(&bytes))?;
    let o = d.take(a, Obj::new(a, &rec))?;
    if d.take(a, o.str("schema"))? != RUN_SCHEMA || d.take(a, o.str("name"))? != name {
        d.mismatch(
            a,
            "schema/name",
            format!("{RUN_SCHEMA}/{name}"),
            format!(
                "{}/{}",
                o.str("schema").unwrap_or(""),
                o.str("name").unwrap_or("")
            ),
            "",
        );
    }
    if o.opt("launch_error").is_some() {
        d.failed(a, "launch_error", "absent", &rec["launch_error"], "");
    }
    if d.take(a, o.i64("returncode"))? != 0 {
        d.failed(a, "returncode", 0, &rec["returncode"], "");
    }
    d.take(a, o.null("signal"))?;
    if d.take(a, o.bool("timed_out"))? {
        d.failed(a, "timed_out", false, true, "");
    }
    if !d.take(a, o.arr("leftover_processes"))?.is_empty()
        || d.take(a, o.bool("leftovers_killed"))?
    {
        d.failed(
            a,
            "leftover_processes",
            "none",
            &rec["leftover_processes"],
            "leftover or killed processes",
        );
    }
    if !d.take(a, o.bool("env_clear"))? {
        d.mismatch(a, "env_clear", true, false, "");
    }
    let want_env = subst_value(&process["env"], &vars);
    let env = d.take(a, o.obj("env_set"))?;
    let _ = env;
    if rec["env_set"] != want_env {
        d.mismatch(
            a,
            "env_set",
            &want_env,
            &rec["env_set"],
            "recorded environment differs from the map",
        );
    }
    let want_cwd = subst(process["cwd"].as_str().unwrap(), &vars);
    if d.take(a, o.str("cwd"))? != want_cwd {
        d.mismatch(a, "cwd", &want_cwd, o.str("cwd").unwrap_or(""), "");
    }
    let started_text = d.take(a, o.str("started_at"))?;
    let ended_text = d.take(a, o.str("ended_at"))?;
    let (Some(started), Some(ended)) = (parse_iso(started_text), parse_iso(ended_text)) else {
        d.malformed(a, format!("started_at/ended_at `{started_text}`/`{ended_text}` are not custody timestamps; timing-dependent checks cannot run"));
        return None;
    };
    if ended < started {
        d.malformed(a, "ended_at precedes started_at");
    }
    // argv with {exe:...} resolved from the exact earlier build record.
    let argv = d.take(a, o.strings("argv"))?;
    let mut want_argv = Vec::new();
    for t in process["argv"].as_array().unwrap() {
        let t = subst(t.as_str().unwrap(), &vars);
        match exe_token(&t) {
            Some((step, target)) => {
                let hits: Vec<&(String, String, String)> = done
                    .get(&step)
                    .map(|b| b.executables.iter().filter(|e| e.0 == target).collect())
                    .unwrap_or_default();
                if hits.len() != 1 {
                    d.mismatch(
                        a,
                        "argv",
                        "one executable",
                        hits.len(),
                        format!("{t} does not resolve to exactly one executable of build `{step}`"),
                    );
                    return None;
                }
                want_argv.push(hits[0].1.clone());
            }
            None => want_argv.push(t),
        }
    }
    if argv != want_argv {
        d.mismatch(
            a,
            "argv",
            format!("{want_argv:?}"),
            format!("{argv:?}"),
            "recorded argv differs from the map",
        );
    }
    // Logs: placed in the records directory, hash- and length-exact.
    let mut stdout = Vec::new();
    for stream in ["stdout", "stderr"] {
        let so = d.take(a, o.obj(stream))?;
        let (p, len, sha) = (
            d.take(a, so.str("path"))?,
            d.take(a, so.u64("length"))?,
            d.take(a, so.str("sha256"))?,
        );
        if !Path::new(p).starts_with(records) {
            d.mismatch(
                a,
                &format!("{stream}.path"),
                records.display(),
                p,
                "log outside the records directory",
            );
        }
        match fs::read(p) {
            Ok(b) if sha256_hex(&b) == sha && b.len() as u64 == len => {
                if stream == "stdout" {
                    stdout = b;
                }
            }
            Ok(b) => d.mismatch(
                a,
                stream,
                sha,
                sha256_hex(&b),
                "log bytes differ from the record",
            ),
            Err(e) => d.missing(p, e.to_string()),
        }
    }
    // Inputs: present, regular, unique, digest-correct and complete.
    let inputs = d.take(a, o.arr("inputs"))?;
    if inputs.is_empty() {
        d.add(
            a,
            "inputs",
            Class::Missing,
            "bound inputs",
            "none",
            "the record binds no inputs",
        );
    }
    let external: BTreeMap<String, runtime::ObjectBinding> = d.take(
        a,
        serde_json::from_value(spec.map["external_inputs"].clone()).map_err(|e| e.to_string()),
    )?;
    let mut bound: BTreeMap<String, String> = BTreeMap::new();
    let mut captured = BTreeMap::new();
    for (n, e) in inputs.iter().enumerate() {
        let eo = d.take(a, Obj::new(format!("inputs[{n}]"), e))?;
        let path = d.take(a, eo.str("path"))?.to_owned();
        let got = d.take(a, runtime::ObjectBinding::from_custody(e))?;
        let now = d.take(a, runtime::ObjectBinding::observe(Path::new(&path)))?;
        let expected = external
            .get(&path)
            .cloned()
            .or_else(|| {
                m.entries
                    .get(&path)
                    .map(|sha| runtime::ObjectBinding::File {
                        sha256: sha.clone(),
                    })
            })
            .unwrap_or_else(|| now.clone());
        if got != expected || now != expected {
            d.mismatch(
                a,
                &path,
                format!("{expected:?}"),
                format!("record={got:?}, current={now:?}"),
                "input object identity differs",
            );
        }
        if bound.insert(path.clone(), got.sha().to_owned()).is_some() {
            d.mismatch(a, &path, "bound once", "bound twice", "duplicate input");
        }
        captured.insert(path, got);
    }
    check_runtime(spec, process, &vars, m, Some(&captured), d);
    let run_outputs: BTreeSet<String> = base::strings(&spec.map, "run_outputs")
        .iter()
        .map(|p| subst(p, &vars))
        .collect();
    for required in required_digests(&spec.map, &vars, &m.closure, process) {
        let resolved = match exe_token(&required) {
            Some((step, target)) => match done
                .get(&step)
                .and_then(|b| b.executables.iter().find(|e| e.0 == target))
            {
                Some(e) => e.1.clone(),
                None => {
                    d.add(
                        a,
                        &required,
                        Class::Missing,
                        "resolved executable",
                        "unresolved",
                        "",
                    );
                    continue;
                }
            },
            None => required.clone(),
        };
        if !bound.contains_key(&resolved) {
            d.add(
                a,
                &resolved,
                Class::Missing,
                "bound",
                "unbound",
                "required input not bound in the record",
            );
        }
        if run_outputs.contains(&resolved) {
            match (bound.get(&resolved), file_sha(Path::new(&resolved))) {
                (Some(b), Ok(now)) if *b == now => {}
                _ => d.mismatch(
                    a,
                    &resolved,
                    "recorded digest",
                    "differs",
                    "run output changed after custody",
                ),
            }
        }
    }
    // Build: executables re-derived from cargo's messages, never taken from the record alone.
    let mut executables = Vec::new();
    if process.get("cargo_json").is_some() {
        let derived = d.take(a, cargo_executables(&stdout))?;
        let recorded = d.take(a, o.arr("cargo_executables"))?;
        let mut rec_list = Vec::new();
        for (n, e) in recorded.iter().enumerate() {
            let eo = d.take(a, Obj::new(format!("cargo_executables[{n}]"), e))?;
            rec_list.push((
                d.take(a, eo.str("name"))?.to_owned(),
                d.take(a, eo.str("path"))?.to_owned(),
                d.take(a, eo.str("sha256"))?.to_owned(),
            ));
        }
        let rec_pairs: Vec<(String, String)> = rec_list
            .iter()
            .map(|(n, p, _)| (n.clone(), p.clone()))
            .collect();
        if rec_pairs != derived {
            d.mismatch(
                a,
                "cargo_executables",
                format!("{derived:?}"),
                format!("{rec_pairs:?}"),
                "recorded executables differ from those re-derived from the build output",
            );
        }
        for t in base::strings(process, "exe_targets") {
            if derived.iter().filter(|e| e.0 == t).count() != 1 {
                d.mismatch(
                    a,
                    "exe_targets",
                    format!("one `{t}`"),
                    derived.iter().filter(|e| e.0 == t).count(),
                    "",
                );
            }
        }
        executables = rec_list;
    }
    // Direct test executable: the one the exact build record emitted, unchanged on disk.
    if let Some(exe) = process.get("exe") {
        let (bs, bt) = (
            exe["step"].as_str().unwrap(),
            exe["target"].as_str().unwrap(),
        );
        match done
            .get(bs)
            .and_then(|b| b.executables.iter().find(|e| e.0 == bt))
        {
            None => d.add(
                a,
                "exe",
                Class::Missing,
                format!("{bs}:{bt}"),
                "unbound",
                "no such executable in the build record",
            ),
            Some((_, path, sha)) => {
                if argv.first() != Some(path) {
                    d.mismatch(
                        a,
                        "argv[0]",
                        path,
                        argv.first().cloned().unwrap_or_default(),
                        "ran a different executable",
                    );
                }
                if bound.get(path) != Some(sha) {
                    d.mismatch(
                        a,
                        "inputs",
                        sha,
                        bound.get(path).cloned().unwrap_or_default(),
                        "the record does not bind the executable digest the build emitted",
                    );
                }
                if file_sha(Path::new(path)).ok().as_deref() != Some(sha.as_str()) {
                    d.mismatch(
                        a,
                        path,
                        sha,
                        "changed",
                        "the executable on disk is not the one the build emitted",
                    );
                }
            }
        }
    }
    let text = String::from_utf8_lossy(&stdout).into_owned();
    if let Some(min) = process.get("min_tests").and_then(Value::as_u64) {
        match test_results(&text) {
            Ok((passed, failed, lines)) => {
                if lines == 0 || failed != 0 || passed < min {
                    d.failed(
                        a,
                        "tests",
                        format!(">= {min} passed, 0 failed"),
                        format!("{passed} passed, {failed} failed, {lines} result lines"),
                        "",
                    );
                }
            }
            Err(e) => d.malformed(a, format!("stdout test results: {e}")),
        }
    }
    for needle in base::strings(process, "stdout_contains") {
        if !text.contains(&needle) {
            d.mismatch(a, "stdout", &needle, "absent", "");
        }
    }
    Some(Rec {
        value: rec,
        started,
        ended,
        stdout,
        executables,
    })
}

/// Postflight: the custody summary, every record, toolchain/closure/SDK identity, retained outputs
/// and controls, each as an explicit required check of the validated specification.
pub fn postflight(i: &Inputs, records: &Path) -> Evaluation {
    let spec = match load_spec(i.map_path, i.expect_map_sha256) {
        Ok(s) => s,
        Err(e) => return Evaluation::spec_failed(e),
    };
    let mut l = Ledger::new(&spec.checks);
    let mut facts = Map::new();
    l.run("pins", |d| pins(i, d));
    let manifest = l.run("source_manifest", |d| {
        check_manifest(&spec, i.manifest_path, d)
    });
    let rows = l.run("summary", |d| check_summary(&spec, i, records, d));
    let mut done: BTreeMap<String, Rec> = BTreeMap::new();
    for process in spec.processes() {
        let name = process["step"].as_str().unwrap().to_owned();
        if let Some(rec) = l.run(&format!("record:{name}"), |d| {
            check_record(
                &spec,
                i,
                records,
                manifest.as_ref()?,
                rows.as_ref()?,
                &done,
                process,
                d,
            )
        }) {
            done.insert(name, rec);
        }
    }
    l.run("step_order", |d| {
        let mut last: Option<(&str, f64)> = None;
        for p in spec.processes() {
            let name = p["step"].as_str().unwrap();
            let r = done.get(name)?;
            if let Some((prev, end)) = last {
                if r.started < end {
                    d.mismatch(
                        &format!("{name}.json"),
                        "started_at",
                        format!(">= end of {prev}"),
                        r.started,
                        "steps overlap",
                    );
                }
            }
            last = Some((name, r.ended));
        }
        Some(())
    });
    let exp = spec.expected().clone();
    for c in exp["stdout_checks"].as_array().unwrap() {
        let step = c["step"].as_str().unwrap();
        l.run(&format!("stdout_identity:{step}"), |d| {
            let text = String::from_utf8_lossy(&done.get(step)?.stdout).into_owned();
            let ok = match (
                c.get("contains").and_then(Value::as_str),
                c.get("equals").and_then(Value::as_str),
            ) {
                (Some(x), _) => text.contains(x),
                (_, Some(x)) => text.trim() == x,
                _ => false,
            };
            if !ok {
                d.mismatch(
                    &format!("{step}.stdout"),
                    "identity",
                    c,
                    text.trim(),
                    "toolchain/runtime identity expectation",
                );
            }
            Some(())
        });
    }
    l.run("closure_root", |d| {
        let rec = done.get(exp["closure_step"].as_str().unwrap())?;
        let v = d.take("cargo metadata", strict::parse(&rec.stdout))?;
        let o = d.take("cargo metadata", Obj::new("metadata", &v))?;
        let pkgs = d.take("cargo metadata", o.arr("packages"))?;
        let nodes_v = d.take(
            "cargo metadata",
            o.obj("resolve").and_then(|r| r.arr("nodes")),
        )?;
        let nodes: BTreeMap<&str, &Value> = nodes_v
            .iter()
            .filter_map(|n| Some((n["id"].as_str()?, n)))
            .collect();
        let start = pkgs
            .iter()
            .find(|p| p["name"] == exp["closure_package"] && p["source"].is_null())
            .and_then(|p| p["id"].as_str());
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut stack: Vec<&str> = start.into_iter().collect();
        while let Some(id) = stack.pop() {
            if seen.insert(id) {
                for dep in nodes
                    .get(id)
                    .and_then(|n| n["deps"].as_array())
                    .into_iter()
                    .flatten()
                {
                    match dep["pkg"].as_str() {
                        Some(p) => stack.push(p),
                        None => d.malformed("cargo metadata", "resolve dependency without pkg"),
                    }
                }
            }
        }
        let roots: BTreeSet<String> = pkgs
            .iter()
            .filter(|p| {
                p["source"].is_null()
                    && p["id"]
                        .as_str()
                        .map(|id| seen.contains(id))
                        .unwrap_or(false)
            })
            .filter_map(|p| {
                Path::new(p["manifest_path"].as_str()?)
                    .parent()
                    .map(|x| x.display().to_string())
            })
            .collect();
        let required: BTreeSet<String> = base::strings(&spec.map, "required_roots")
            .into_iter()
            .collect();
        if start.is_none() || roots != required {
            d.mismatch(
                "cargo metadata",
                "local closure",
                format!("{required:?}"),
                format!("{roots:?}"),
                "cargo's local dependency closure differs from the map's required roots",
            );
        }
        Some(())
    });
    l.run("sdk_verification", |d| {
        const A: &str = "sdk-verification.json";
        let _ = done.get(exp["sdk_step"].as_str().unwrap())?;
        let bytes = d.take(A, fs::read(records.join(A)).map_err(|e| e.to_string()))?;
        let v = d.take(A, strict::parse(&bytes))?;
        let o = d.take(A, Obj::new(A, &v))?;
        d.take(
            A,
            o.exact(&[
                "schema",
                "root",
                "manifest",
                "manifest_sha256",
                "counts",
                "errors",
                "exact_manifest_entry_set",
            ]),
        )?;
        let counts = d.take(A, o.obj("counts"))?;
        d.take(A, counts.exact(&["file", "symlink"]))?;
        let (files, links) = (
            d.take(A, counts.u64("file"))?,
            d.take(A, counts.u64("symlink"))?,
        );
        if d.take(A, o.str("schema"))? != "org-sdk-verification-gate/v1"
            || !d.take(A, o.arr("errors"))?.is_empty()
            || !d.take(A, o.bool("exact_manifest_entry_set"))?
        {
            d.failed(
                A,
                "verification",
                "exact clean verification",
                &v["errors"],
                "",
            );
        }
        if d.take(A, o.str("manifest_sha256"))? != exp["sdk_manifest_sha256"].as_str().unwrap() {
            d.mismatch(
                A,
                "manifest_sha256",
                &exp["sdk_manifest_sha256"],
                &v["manifest_sha256"],
                "",
            );
        }
        if json!(files) != exp["sdk_files"] || json!(links) != exp["sdk_links"] {
            d.mismatch(
                A,
                "counts",
                format!("{}/{}", exp["sdk_files"], exp["sdk_links"]),
                format!("{files}/{links}"),
                "",
            );
        }
        Some(())
    });
    outputs::evaluate(&spec, &mut l, &done, &i.run, manifest.as_ref(), &mut facts);
    facts.insert("records_checked".into(), json!(done.len()));
    facts.insert("records_required".into(), json!(spec.processes().len()));
    Evaluation {
        spec_errors: vec![],
        ledger: Some(l),
        facts,
    }
}

/// The run directory's evidence path for a bundle name.
pub fn bundle_dir(run: &RunVars, rel: &str) -> PathBuf {
    Path::new(&run.run_dir).join(rel)
}
