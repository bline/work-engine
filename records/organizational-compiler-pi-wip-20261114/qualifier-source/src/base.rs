//! Source-closure, plan-derivation and SDK-verification utilities carried unchanged in meaning
//! from the R3 qualifier (map/closure/executable joins). Callers receive only validated specs.
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const MANIFEST_SCHEMA: &str = "org-source-manifest/v2";

pub(crate) fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

pub(crate) fn strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn file_sha(path: &Path) -> Result<String, String> {
    fs::read(path)
        .map(|b| sha256_hex(&b))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// All regular files under `root`, skipping directories named in `skip`. A symlink inside a
/// scoped root is itself reported (`Err`), since custody digests would not cover its target.
pub fn walk(root: &Path, skip: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                return Err(format!("symlink inside scoped root: {}", path.display()));
            }
            if kind.is_dir() {
                if !skip
                    .iter()
                    .any(|n| path.file_name().map(|f| f == n.as_str()).unwrap_or(false))
                {
                    stack.push(path);
                }
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Embedded fixture paths named by `include_bytes!`/`include_str!`/`include!` string literals in
/// `.rs` files (comment lines skipped), resolved relative to the including file.
pub fn embedded_includes(files: &[PathBuf]) -> (BTreeSet<PathBuf>, Vec<String>) {
    let (mut found, mut unresolved) = (BTreeSet::new(), Vec::new());
    for file in files
        .iter()
        .filter(|f| f.extension().map(|e| e == "rs").unwrap_or(false))
    {
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        // A macro's literal may sit on the line after its opening parenthesis; scan the text.
        let code: String = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for mac in ["include_bytes!", "include_str!", "include!"] {
            let mut rest = code.as_str();
            while let Some(at) = rest.find(mac) {
                let after = &rest[at + mac.len()..];
                rest = after;
                let trimmed = after.trim_start();
                let Some(inner) = trimmed.strip_prefix('(') else {
                    continue;
                };
                let inner = inner.trim_start();
                let Some(lit) = inner.strip_prefix('"') else {
                    unresolved.push(format!("{}: non-literal {mac}", file.display()));
                    continue;
                };
                let Some(end) = lit.find('"') else { continue };
                let target = file.parent().unwrap().join(&lit[..end]);
                match fs::canonicalize(&target) {
                    Ok(p) => {
                        found.insert(p);
                    }
                    Err(_) => unresolved.push(format!("{}: {}", file.display(), target.display())),
                }
            }
        }
    }
    (found, unresolved)
}

/// The closure the map requires, computed from the filesystem (not from any manifest or record):
/// every file under each required root, the required files, and every embedded fixture.
pub fn closure(map: &Value) -> Result<(BTreeSet<String>, Vec<String>), String> {
    let skip = strings(map, "excluded_dir_names");
    let mut files: Vec<PathBuf> = Vec::new();
    for root in strings(map, "required_roots") {
        files.extend(walk(Path::new(&root), &skip)?);
    }
    for f in strings(map, "required_files") {
        files.push(PathBuf::from(f));
    }
    let (embedded, mut unresolved) = embedded_includes(&files);
    // A macro-built (non-literal) include is accepted only in a file the map names, and only
    // because its target directory lies inside a required root whose whole tree is digested.
    let accepted = strings(map, "accepted_nonliteral_include_files");
    unresolved.retain(|u| {
        !(u.contains("non-literal") && accepted.iter().any(|f| u.starts_with(&format!("{f}:"))))
    });
    let mut set: BTreeSet<String> = files.iter().map(|p| p.display().to_string()).collect();
    set.extend(embedded.iter().map(|p| p.display().to_string()));
    Ok((set, unresolved))
}

pub(crate) fn subst(text: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = text.to_owned();
    for (k, v) in vars {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

pub(crate) fn subst_value(v: &Value, vars: &BTreeMap<String, String>) -> Value {
    match v {
        Value::String(t) => Value::String(subst(t, vars)),
        Value::Array(a) => Value::Array(a.iter().map(|x| subst_value(x, vars)).collect()),
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, x)| (k.clone(), subst_value(x, vars)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Run-specific values the map's placeholders take.
#[derive(Debug, Clone)]
pub struct RunVars {
    pub invocation_id: String,
    pub run_dir: String,
    pub records_dir: String,
    pub map_sha256: String,
    pub manifest_sha256: String,
}

impl RunVars {
    pub fn vars(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("INVOCATION_ID".into(), self.invocation_id.clone()),
            ("RUN_DIR".into(), self.run_dir.clone()),
            ("RECORDS_DIR".into(), self.records_dir.clone()),
            ("MAP_SHA256".into(), self.map_sha256.clone()),
            (
                "SOURCE_MANIFEST_SHA256".into(),
                self.manifest_sha256.clone(),
            ),
        ])
    }
}

/// Resolves a process's required digest paths: its input classes plus any `{exe:step:target}`.
fn class_paths(
    map: &Value,
    vars: &BTreeMap<String, String>,
    closure: &BTreeSet<String>,
    class: &str,
) -> Vec<String> {
    if class == "@closure" {
        return closure.iter().cloned().collect();
    }
    let list = map
        .get("classes")
        .and_then(|c| c.get(class))
        .map(|c| strings(&json!({"x": c}), "x"))
        .unwrap_or_default();
    list.iter().map(|p| subst(p, vars)).collect()
}

pub(crate) fn required_digests(
    map: &Value,
    vars: &BTreeMap<String, String>,
    closure: &BTreeSet<String>,
    process: &Value,
) -> Vec<String> {
    let mut all: Vec<String> = Vec::new();
    for class in strings(process, "inputs") {
        all.extend(class_paths(map, vars, closure, &class));
    }
    if let Some(exe) = process.get("exe") {
        all.push(format!("{{exe:{}:{}}}", s(exe, "step"), s(exe, "target")));
    }
    all
}

pub(crate) fn process_step(
    map: &Value,
    vars: &BTreeMap<String, String>,
    closure: &BTreeSet<String>,
    p: &Value,
) -> Value {
    let mut step = Map::new();
    step.insert("name".into(), json!(s(p, "step")));
    step.insert("argv".into(), subst_value(&p["argv"], vars));
    step.insert("cwd".into(), json!(subst(s(p, "cwd"), vars)));
    step.insert("env".into(), subst_value(&p["env"], vars));
    let mut digests = required_digests(map, vars, closure, p);
    digests.sort();
    digests.dedup();
    step.insert("digest".into(), json!(digests));
    step.insert("timeout".into(), p["timeout"].clone());
    if p.get("cargo_json").and_then(Value::as_bool) == Some(true) {
        step.insert("cargo_json".into(), json!(true));
    }
    Value::Object(step)
}

/// Derives the custody plan from the map. The plan is a product of the map, never its source;
/// [`preflight`] compares an actual plan with the map independently.
pub fn make_plan(map: &Value, run: &RunVars) -> Result<Value, String> {
    let vars = run.vars();
    let (clo, unresolved) = closure(map)?;
    if !unresolved.is_empty() {
        return Err(format!("unresolved embedded includes: {unresolved:?}"));
    }
    let steps: Vec<Value> = map["processes"]
        .as_array()
        .ok_or("map has no processes")?
        .iter()
        .map(|p| process_step(map, &vars, &clo, p))
        .collect();
    Ok(json!({"env_clear": true, "steps": steps}))
}

/// Builds the retained source manifest from the filesystem closure plus pinned externals.
pub fn build_manifest(map: &Value, map_sha256: &str) -> Result<Value, String> {
    let (clo, unresolved) = closure(map)?;
    if !unresolved.is_empty() {
        return Err(format!("unresolved embedded includes: {unresolved:?}"));
    }
    let mut entries = Vec::new();
    for path in &clo {
        let bytes = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        entries.push(json!({"path": path, "length": bytes.len(), "sha256": sha256_hex(&bytes)}));
    }
    let mut externals = Vec::new();
    for (path, pin) in map["external_inputs"]
        .as_object()
        .ok_or("no external_inputs")?
    {
        externals.push(json!({"path": path, "pinned": pin, "observed": crate::runtime::ObjectBinding::observe(Path::new(path)).ok()}));
    }
    Ok(json!({
        "schema": MANIFEST_SCHEMA, "map_sha256": map_sha256,
        "roots": map["required_roots"], "excluded_dir_names": map["excluded_dir_names"],
        "entries": entries, "externals": externals, "absent_expected": map["absent_expected"],
    }))
}

/// Verifies a read-only SDK tree against its manifest: exact file/symlink set and digests.
pub fn verify_sdk(
    root: &Path,
    manifest_path: &Path,
    expect_manifest_sha: &str,
) -> Result<Value, String> {
    let bytes = fs::read(manifest_path).map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    let manifest_sha = sha256_hex(&bytes);
    let manifest: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let entries = manifest
        .as_object()
        .ok_or("sdk manifest is not an object")?;
    let mut errors: Vec<String> = Vec::new();
    if manifest_sha != expect_manifest_sha {
        errors.push(format!(
            "manifest sha256 {manifest_sha} != expected {expect_manifest_sha}"
        ));
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut counts = (0u64, 0u64);
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap().display().to_string();
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_dir() {
                stack.push(path);
                continue;
            }
            seen.insert(rel.clone());
            match entries.get(&rel) {
                None => errors.push(format!("not in manifest: {rel}")),
                Some(want) if kind.is_symlink() => {
                    counts.1 += 1;
                    let target = fs::read_link(&path)
                        .map(|t| t.display().to_string())
                        .unwrap_or_default();
                    if s(want, "kind") != "symlink" || s(want, "target") != target {
                        errors.push(format!("symlink differs: {rel}"));
                    }
                }
                Some(want) => {
                    counts.0 += 1;
                    if s(want, "kind") != "file" || s(want, "sha256") != file_sha(&path)? {
                        errors.push(format!("file differs: {rel}"));
                    }
                }
            }
        }
    }
    for key in entries.keys().filter(|k| !seen.contains(*k)) {
        errors.push(format!("manifest entry absent from tree: {key}"));
    }
    errors.truncate(50);
    Ok(json!({
        "schema": "org-sdk-verification-gate/v1", "root": root.display().to_string(),
        "manifest": manifest_path.display().to_string(), "manifest_sha256": manifest_sha,
        "counts": {"file": counts.0, "symlink": counts.1}, "errors": errors,
        "exact_manifest_entry_set": errors.is_empty() && manifest_sha == expect_manifest_sha,
    }))
}

pub(crate) fn same_set(a: &BTreeSet<String>, b: &BTreeSet<String>) -> (Vec<String>, Vec<String>) {
    (
        a.difference(b).cloned().collect(),
        b.difference(a).cloned().collect(),
    )
}
