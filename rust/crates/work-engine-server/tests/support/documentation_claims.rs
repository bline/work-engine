//! Staged private qualification fixture. This is not a claim-refresh owner.
use repository_evidence_cbm::sha256;
use serde_json::{Value, json};
use std::{
    env,
    error::Error,
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
const PATHS: [&str; 3] = [
    "src/ledger.rs",
    "docs/ledger.md",
    "runtime-observation.json",
];
const KEY: &str = "ledger-documentation-sample-agreement";
const SCOPE: &str = "documentation-sample-qualification";

pub fn write_json(path: &Path, value: &Value) -> Result<String> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let temporary = path.with_extension("pending");
    fs::write(&temporary, &bytes)?;
    fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
    fs::rename(temporary, path)?;
    Ok(sha256(&bytes))
}
pub fn load(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing {key}").into())
}
fn path(value: &Value, key: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(text(value, key)?))
}
fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}
fn binary(name: &str) -> Result<Value> {
    let selected = PathBuf::from(env::var(name).map_err(|_| format!("explicit {name} required"))?);
    if !selected.is_absolute() || !selected.is_file() {
        return Err(format!("invalid {name}").into());
    }
    Ok(json!({"path":selected,"sha256":sha256(&fs::read(&selected)?)}))
}
fn verify_binaries(state: &Value) -> Result<()> {
    for name in ["cbm", "host", "server", "capture"] {
        let item = &state["binaries"][name];
        if sha256(&fs::read(text(item, "path")?)?) != text(item, "sha256")? {
            return Err(format!("selected {name} changed").into());
        }
    }
    Ok(())
}
fn command(state: &Value, binary: &Path) -> Result<Command> {
    let mut cmd = Command::new(binary);
    // Git selectors and injected config override cwd, including in capture's
    // nested Git calls. Remove inherited Git state for every fixture child.
    for (name, _) in env::vars_os() {
        if name.as_encoded_bytes().starts_with(b"GIT_") {
            cmd.env_remove(name);
        }
    }
    cmd.current_dir(path(state, "repository")?)
        .env("CBM_RUNTIME_DIR", path(state, "runtime")?)
        .env("CBM_CACHE_DIR", path(state, "cache")?)
        .env("TMPDIR", path(state, "tmp")?)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null");
    Ok(cmd)
}
fn execute(mut command: Command, log: &Path, input: Option<&Value>) -> Result<Vec<u8>> {
    command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let input_bytes = input.map(serde_json::to_vec).transpose()?;
    let mut child = command.spawn()?;
    let input_error = input_bytes.and_then(|bytes| {
        child
            .stdin
            .take()
            .and_then(|mut stdin| stdin.write_all(&bytes).err())
    });
    // Reap even when a child closes its input before the request is written.
    let output = child.wait_with_output()?;
    fs::write(log.with_extension("stdout"), &output.stdout)?;
    fs::write(log.with_extension("stderr"), &output.stderr)?;
    write_json(
        &log.with_extension("status.json"),
        &json!({"success":output.status.success(),"code":output.status.code()}),
    )?;
    if let Some(error) = input_error {
        return Err(error.into());
    }
    if !output.status.success() {
        return Err(format!(
            "command failed; inspect {}: {}",
            log.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output.stdout)
}
fn run_json(cmd: Command, log: &Path, input: Option<&Value>) -> Result<Value> {
    Ok(serde_json::from_slice(&execute(cmd, log, input)?)?)
}
fn cbm(state: &Value, args: &[&str], log: &Path) -> Result<Value> {
    let mut cmd = command(state, Path::new(text(&state["binaries"]["cbm"], "path")?))?;
    cmd.args(args);
    let value = run_json(cmd, log, None)?;
    if value["isError"] == true {
        return Err(format!("CBM refused; inspect {}", log.display()).into());
    }
    Ok(value)
}
pub fn publish(state: &Value, action: &str, request: Option<&Value>, log: &Path) -> Result<Value> {
    verify_binaries(state)?;
    let mut cmd = command(state, Path::new(text(&state["binaries"]["host"], "path")?))?;
    cmd.arg(action)
        .arg(path(state, "startup")?)
        .arg(text(state, "startup_sha256")?);
    run_json(cmd, log, request)
}
pub fn cleanup(state_path: &Path) -> Result<Value> {
    let mut state = load(state_path)?;
    verify_binaries(&state)?;
    let mut cmd = command(&state, Path::new(text(&state["binaries"]["cbm"], "path")?))?;
    cmd.args(["daemon", "stop"]);
    execute(cmd, &path(&state, "base")?.join("cleanup"), None)?;
    state["private_daemon"] = json!("stopped");
    write_json(state_path, &state)?;
    Ok(state)
}
fn stop_private_daemon(state: &Value, stage: &Path) -> Result<()> {
    let mut cmd = command(state, Path::new(text(&state["binaries"]["cbm"], "path")?))?;
    cmd.args(["daemon", "stop"]);
    execute(cmd, &stage.join("daemon-stop"), None)?;
    Ok(())
}
pub fn initialize_git(state: &Value, stage: &Path) -> Result<()> {
    for (name, args) in [
        ("git-init", vec!["init", "-q"]),
        (
            "git-add",
            vec![
                "add",
                "src/ledger.rs",
                "docs/ledger.md",
                "runtime-observation.json",
            ],
        ),
        (
            "git-commit",
            vec![
                "-c",
                "user.name=Fixture Producer",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "baseline fixture",
            ],
        ),
    ] {
        let mut cmd = command(state, Path::new("git"))?;
        cmd.args(args);
        execute(cmd, &stage.join(name), None)?;
    }
    Ok(())
}
fn observe(state: &Value, case: &str, stage: &Path, baseline: bool) -> Result<Value> {
    let repository = path(state, "repository")?;
    let body = match case {
        "baseline" | "docs" | "unresolved" => "values.iter().sum()",
        "code" => "values.iter().copied().max().unwrap_or(0)",
        _ => return Err("unknown case".into()),
    };
    let source = format!(
        "pub fn balance(values: &[i32]) -> i32 {{\n    {body}\n}}\n\nfn main() {{\n    println!(\"{{}}\", balance(&[2, 3, 5]));\n}}\n"
    );
    let symbol = if case == "unresolved" {
        "missing_balance"
    } else {
        "balance"
    };
    let sentence = if case == "docs" {
        "returns sum plus one; the sample [2, 3, 5] returns 11."
    } else {
        "returns the sum; the sample [2, 3, 5] returns 10."
    };
    fs::write(repository.join(PATHS[0]), &source)?;
    fs::write(
        repository.join(PATHS[1]),
        format!("# Ledger\n\n`src/ledger.rs::{symbol}` {sentence}\n"),
    )?;
    let subject = stage.join("ledger-subject");
    let mut compile = command(state, Path::new("rustc"))?;
    compile
        .arg("--edition=2024")
        .arg(repository.join(PATHS[0]))
        .arg("-o")
        .arg(&subject);
    execute(compile, &stage.join("compile"), None)?;
    let output = execute(command(state, &subject)?, &stage.join("subject"), None)?;
    let observed = String::from_utf8(output)?.trim().to_string();
    let expected = if case == "code" { "5" } else { "10" };
    if observed != expected {
        return Err(format!("compiled sample expected {expected}, observed {observed}").into());
    }
    let observation = json!({"schema_version":1,"case":case,"source_path":PATHS[0],"source_sha256":sha256(source.as_bytes()),"binary":subject,"binary_sha256":sha256(&fs::read(&subject)?),"argv":[],"sample":[2,3,5],"stdout":observed,"interpretation":"Observed sample only; no general semantic equivalence or applicability inferred."});
    write_json(&repository.join(PATHS[2]), &observation)?;
    write_json(&stage.join("runtime-observation.json"), &observation)?;
    if baseline {
        initialize_git(state, stage)?;
    }
    let index =
        json!({"repo_path":repository,"name":state["project"],"mode":"full","persistence":false})
            .to_string();
    let work: Result<Value> = (|| {
        cbm(
            state,
            &["cli", "--quiet", "--json", "index_repository", &index],
            &stage.join("index"),
        )?;
        let request = json!({"source_paths":PATHS,"tools":[
            {"tool":"get_code_snippet","arguments":{"qualified_name":"balance","source_mode":"full"}},
            {"tool":"check_index_coverage","arguments":{"paths":PATHS,"diagnostics":"full"}},
            {"tool":"query_graph","arguments":{"query":"MATCH (d)-[r:MENTIONS]->(s) RETURN d, r, s LIMIT 20"}},
            {"tool":"detect_changes","arguments":{"scope":"doc_references","limit":1}}
        ]});
        let request_path = stage.join("capture-request.json");
        write_json(&request_path, &request)?;
        let mut cmd = command(
            state,
            Path::new(text(&state["binaries"]["capture"], "path")?),
        )?;
        cmd.arg("capture")
            .arg("--binding")
            .arg(path(state, "binding")?)
            .arg("--request")
            .arg(&request_path)
            .arg("--artifacts")
            .arg(path(state, "artifacts")?);
        let capture = run_json(cmd, &stage.join("capture"), None)?;
        write_json(&stage.join("capture-result.json"), &capture)?;
        let mut cmd = command(
            state,
            Path::new(text(&state["binaries"]["capture"], "path")?),
        )?;
        cmd.arg("read")
            .arg("--binding")
            .arg(path(state, "binding")?)
            .arg("--sha256")
            .arg(text(&capture["reference"], "integrity_sha256")?)
            .arg("--artifacts")
            .arg(path(state, "artifacts")?);
        let content = run_json(cmd, &stage.join("capture-read"), None)?;
        write_json(&stage.join("capture-full.json"), &content)?;
        if baseline {
            let navigation = content["capture"]["observations"]
                .as_array()
                .ok_or("missing capture observations")?
                .iter()
                .find(|o| o["request"]["tool"] == "query_graph")
                .ok_or("missing navigation observation")?;
            let response: Value = serde_json::from_str(text(navigation, "response")?)?;
            let data = &response["result"]["structuredContent"]["data"];
            if data["total"] != 1
                || data["total_relation"] != "eq"
                || data["has_more"] != false
                || data["truncated"] != false
                || data["rows"][0][2] != "balance"
            {
                return Err("baseline needs complete exact documentation MENTIONS evidence".into());
            }
        }
        // Preserve exact navigation and coverage responses, including unresolved
        // targets. Their raw bytes are evidence, not an agreement decision.
        Ok(
            json!({"case":case,"directory":stage,"observation":observation,"capture":capture,"capture_full":stage.join("capture-full.json"),"navigation_status":if case=="unresolved" {"requested target missing_balance; inspect raw navigation evidence; sample10 does not prove target absence"}else{"inspect exact MENTIONS and coverage observations"}}),
        )
    })();
    let stopped = stop_private_daemon(state, stage);
    finish_observation(work, stopped)
}
pub fn finish_observation(work: Result<Value>, stopped: Result<()>) -> Result<Value> {
    match (work, stopped) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(work), Ok(())) => Err(work),
        (Ok(_), Err(stop)) => Err(format!("unresolved private daemon cleanup: {stop}").into()),
        (Err(work), Err(stop)) => Err(format!(
            "observation failed: {work}; unresolved private daemon cleanup: {stop}"
        )
        .into()),
    }
}
fn revision(repository_id: &str, capture: &Value) -> Value {
    json!({"proposition":"For sample [2, 3, 5], the documented balance result 10 agrees with the compiled Rust subject output 10.","support_qualification":"fixture-producer sample-scoped judgment, not a model consumer decision","assumptions":["Only the recorded sample and captured source/doc bytes are covered."],"limitations":capture["limitations"],"confidence":{"label":"bounded observed sample"},"evidence_references":[capture["reference"]],"sensitivity_references":[],"evidence_mode":"cbm_capture_and_compiled_sample","judgment_kind":"repository_fact","decision_scope":SCOPE,"profile_payload":{"repository_id":repository_id,"assertion_key":KEY,"assertion_scope":"documentation and Rust balance agreement for sample [2,3,5]"},"reopening_conditions":["Changed code, documentation, runtime observation or unresolved documentation navigation"],"tombstone":false})
}
pub fn prepare(state_path: &Path) -> Result<Value> {
    if !state_path.is_absolute() || state_path.exists() {
        return Err(
            "new absolute --state path required; existing state is never re-prepared".into(),
        );
    }
    let binaries = json!({"cbm":binary("WORK_ENGINE_CBM_BINARY")?,"host":binary("WORK_ENGINE_CLAIMS_HOST")?,"server":binary("WORK_ENGINE_CLAIMS_SERVER")?,"capture":binary("WORK_ENGINE_CLAIMS_CAPTURE")?});
    let temp = tempfile::Builder::new()
        .prefix("we-doc-claims-")
        .tempdir_in("/tmp")?;
    let base = temp.keep();
    fs::set_permissions(&base, fs::Permissions::from_mode(0o700))?;
    for name in [
        "repository",
        "runtime",
        "cache",
        "tmp",
        "artifacts",
        "stages",
    ] {
        private_dir(&base.join(name))?;
    }
    let repository = base.join("repository");
    for name in ["src", "docs"] {
        private_dir(&repository.join(name))?;
    }
    let unique = base.file_name().ok_or("base")?.to_string_lossy();
    let repository_id = format!("documentation-fixture-{unique}");
    let mut state = json!({"schema_version":1,"status":"preparing","base":base,"repository":repository,"runtime":base.join("runtime"),"cache":base.join("cache"),"tmp":base.join("tmp"),"artifacts":base.join("artifacts"),"project":format!("doc-claims-{unique}"),"repository_id":repository_id,"decision_scope":SCOPE,"binaries":binaries,"binding":base.join("capture-binding.json"),"startup":base.join("producer-startup.json"),"read_profile":base.join("read-profile.json"),"root":base.join("claims"),"stages":[],"private_daemon":"stopped after each stage"});
    write_json(state_path, &state)?;
    let prepared: Result<()> = (|| {
        write_json(
            &path(&state, "binding")?,
            &json!({"repository_id":repository_id,"repository":repository,"project":state["project"],"executable":state["binaries"]["cbm"]["path"],"executable_sha256":state["binaries"]["cbm"]["sha256"]}),
        )?;
        let stage = base.join("stages/0-baseline");
        private_dir(&stage)?;
        let baseline = observe(&state, "baseline", &stage, true)?;
        state["current_capture"] = baseline["capture"].clone();
        state["stages"]
            .as_array_mut()
            .ok_or("stages")?
            .push(baseline);
        let source = base.join("authority.json");
        let config = base.join("claims-config.json");
        let authority_sha = write_json(
            &source,
            &json!({"schema_version":3,"owner":"trusted-private-fixture-issuer","reference":"fixture:documentation-agreement","revision":"1","freshness":"pinned fixture","profile":"repository-claims-v1","repository_id":repository_id,"actors":["fixture-producer"],"decision_scopes":[SCOPE],"grant_ids":["fixture-grant"]}),
        )?;
        let config_sha = write_json(
            &config,
            &json!({"schema_version":3,"profile":"repository-claims-v1","root_id":repository_id,"repository_id":repository_id,"grants":[{"schema_version":1,"grant_id":"fixture-grant","actor":"fixture-producer","profile":"repository-fact-v1","permissions":["create_claim","publish_revision","record_reliance"],"decision_scope":SCOPE,"authority_reference":{"owner":"trusted-private-fixture-issuer","reference":"fixture:documentation-agreement","revision":"1","freshness":"pinned fixture","integrity_sha256":authority_sha,"status":"verified"}}]}),
        )?;
        let read = json!({"schema_version":1,"root":state["root"],"config":config,"source":source,"expected_config_sha256":config_sha,"expected_source_sha256":authority_sha,"repository_id":repository_id,"decision_scope":SCOPE});
        let mut startup = read.clone();
        startup["actor"] = json!("fixture-producer");
        startup["grant_id"] = json!("fixture-grant");
        startup["allowed_actions"] = json!(["create_claim", "publish_revision", "record_reliance"]);
        state["startup_sha256"] = json!(write_json(&path(&state, "startup")?, &startup)?);
        write_json(state_path, &state)?;
        publish(&state, "initialize", None, &stage.join("host-initialize"))?;
        let initial = revision(&repository_id, &state["current_capture"]);
        state["initial_revision_payload"] = initial.clone();
        let create = json!({"schema_version":1,"operation_id":"baseline-agreement","action":"create_claim","profile":"repository-fact-v1","expected_state":null,"payload":{"subject":{"namespace":format!("repository-fact:{repository_id}"),"subject_kind":"repository_fact","stable_subject_id":KEY,"evidence_baseline":state["current_capture"]["reference"],"content_set":PATHS},"statement_identity":KEY,"initial_revision":initial}});
        write_json(&stage.join("create-request.json"), &create)?;
        state["status"] = json!("publication_entering_do_not_retry_unknown");
        write_json(state_path, &state)?;
        let receipt = publish(&state, "execute", Some(&create), &stage.join("host-create"))?;
        if receipt["status"] != "applied" {
            return Err(format!("baseline not applied: {receipt}").into());
        }
        write_json(&stage.join("create-receipt.json"), &receipt)?;
        state["initial_exact"] = json!({"claim_id":receipt["result"]["record"]["claim_id"],"revision_id":receipt["result"]["record"]["id"],"revision_sha256":receipt["result"]["sha256"]});
        let mut read = read;
        read["evidence"] = json!({"artifacts":state["artifacts"],"repository":repository,"project":state["project"]});
        write_json(&path(&state, "read_profile")?, &read)?;
        state["status"] = json!("baseline_ready");
        Ok(())
    })();
    if let Err(error) = prepared {
        state["failure"] = json!(error.to_string());
        write_json(state_path, &state)?;
        return Err(error);
    }
    write_json(state_path, &state)?;
    Ok(state)
}
pub fn advance(state_path: &Path, case: &str) -> Result<Value> {
    if !["code", "docs", "unresolved"].contains(&case) {
        return Err("case must be code|docs|unresolved".into());
    }
    let mut state = load(state_path)?;
    verify_binaries(&state)?;
    let stage = path(&state, "base")?.join(format!(
        "stages/{}-{case}",
        state["stages"].as_array().ok_or("stages")?.len()
    ));
    private_dir(&stage)?;
    state["status"] = json!("capturing");
    write_json(state_path, &state)?;
    match observe(&state, case, &stage, false) {
        Ok(observation) => {
            state["current_capture"] = observation["capture"].clone();
            state["stages"]
                .as_array_mut()
                .ok_or("stages")?
                .push(observation);
            state["status"] = json!("capture_ready_manual_judgment_required");
        }
        Err(error) => {
            state["failure"] = json!(error.to_string());
            write_json(state_path, &state)?;
            return Err(error);
        }
    }
    write_json(state_path, &state)?;
    Ok(state)
}
