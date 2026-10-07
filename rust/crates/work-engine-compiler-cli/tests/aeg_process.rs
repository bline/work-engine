use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;
use work_engine_compiler::sha256_hex;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}
fn root() -> PathBuf {
    repo().join("app-server/tests/fixtures/compiler-c2/root")
}
fn request() -> Vec<u8> {
    let case = repo().join("rust/crates/work-engine-compiler/tests/fixtures/v1/skills/p-sup");
    serde_json::to_vec(&json!({"schema_version":2,"request_id":"process-test","operation":"compile_skill_verified","structure_source_base64":STANDARD.encode(std::fs::read(case.join("structure.yaml")).unwrap()),"interface_source_base64":STANDARD.encode(std::fs::read(case.join("interface.yaml")).unwrap())})).unwrap()
}
fn run(script: &Path, env: &[(&str, &str)]) -> (Output, Value) {
    let root = root();
    let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-compiler"))
        .env("WORK_ENGINE_COMPILER_WORKSPACE_ROOT", &root)
        .env("WORK_ENGINE_COMPILER_PYTHON", "/usr/bin/python3")
        .env("WORK_ENGINE_COMPILER_AEG_SCRIPT", script)
        .env(
            "WORK_ENGINE_COMPILER_AEG_SCRIPT_SHA256",
            sha256_hex(std::fs::read(script).unwrap()),
        )
        .env(
            "WORK_ENGINE_COMPILER_AEG_INVARIANTS",
            root.join("docs/workflow-invariants.md"),
        )
        .env(
            "WORK_ENGINE_COMPILER_AEG_ENVIRONMENTS",
            root.join("docs/agent-environments.yaml"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .envs(env.iter().copied())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&request()).unwrap();
    let output = child.wait_with_output().unwrap();
    let envelope = serde_json::from_slice(&output.stdout).unwrap();
    (output, envelope)
}
fn peer(temp: &TempDir, body: &str) -> (PathBuf, PathBuf) {
    let script = temp.path().join("peer.py");
    let pid = temp.path().join("pid");
    let source = format!(
        "import os,sys,time,json,hashlib,signal\nfrom pathlib import Path\nPath({}).write_text(str(os.getpid()))\n{}\n",
        serde_json::to_string(&pid.to_string_lossy()).unwrap(),
        body
    );
    std::fs::write(&script, source).unwrap();
    (script, pid)
}
fn reaped(pid: &Path) {
    let id: usize = std::fs::read_to_string(pid).unwrap().parse().unwrap();
    assert!(
        !PathBuf::from(format!("/proc/{id}")).exists(),
        "Python direct child {id} remains"
    );
}
fn code(response: &Value) -> &str {
    response["error"]["code"].as_str().unwrap()
}

#[test]
fn actual_child_timeout_and_both_pipe_overflows_reap_before_error() {
    for (body, limits, expected) in [
        (
            "time.sleep(30)",
            vec![("WORK_ENGINE_COMPILER_AEG_TIMEOUT_MS", "100")],
            "timeout",
        ),
        (
            "sys.stdout.write('x'*4096);sys.stdout.flush();time.sleep(30)",
            vec![("WORK_ENGINE_COMPILER_AEG_PIPE_LIMIT", "1024")],
            "resource_limit",
        ),
        (
            "sys.stderr.write('x'*4096);sys.stderr.flush();time.sleep(30)",
            vec![("WORK_ENGINE_COMPILER_AEG_PIPE_LIMIT", "1024")],
            "resource_limit",
        ),
    ] {
        let temp = TempDir::new().unwrap();
        let (script, pid) = peer(&temp, body);
        let (output, response) = run(&script, &limits);
        assert!(!output.status.success());
        assert_eq!(code(&response), expected);
        reaped(&pid);
        assert!(response.get("ir").is_none());
    }
}
#[test]
fn success_looking_bytes_cannot_override_stderr_nonzero_signal_or_malformed_output() {
    let projection = root().join("docs/agent-environment-views/slice-supervisor.yaml");
    let produce = format!(
        "import yaml\np=yaml.safe_load(Path({}).read_text())\nr={{'schema_version':1,'status':'closed_projection','backend':'work-engine.agent-environment-graph.v1','backend_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'canonical_role_match':True,'projection':p}}\nsys.stdout.write(json.dumps(r,default=str));sys.stdout.flush()",
        serde_json::to_string(&projection.to_string_lossy()).unwrap()
    );
    for (suffix, expected) in [
        (
            "sys.stderr.write('unexpected');sys.stderr.flush()",
            "aeg_failed",
        ),
        ("sys.exit(7)", "aeg_failed"),
        ("os.kill(os.getpid(),signal.SIGTERM)", "aeg_failed"),
    ] {
        let temp = TempDir::new().unwrap();
        let (script, pid) = peer(&temp, &format!("{produce}\n{suffix}"));
        let (output, response) = run(&script, &[]);
        assert!(!output.status.success());
        assert_eq!(code(&response), expected);
        reaped(&pid);
    }
    let temp = TempDir::new().unwrap();
    let (script, pid) = peer(&temp, "sys.stdout.write('{');sys.stdout.flush()");
    let (output, response) = run(&script, &[]);
    assert!(!output.status.success());
    assert_eq!(code(&response), "aeg_protocol");
    reaped(&pid);
}
#[test]
fn private_peer_exact_pipe_limit_and_backend_digest_are_checked() {
    let projection = root().join("docs/agent-environment-views/slice-supervisor.yaml");
    let produce = format!(
        "import yaml\np=yaml.safe_load(Path({}).read_text())\nr={{'schema_version':1,'status':'closed_projection','backend':'work-engine.agent-environment-graph.v1','backend_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'canonical_role_match':True,'projection':p}}\nsys.stdout.write(json.dumps(r,default=str));sys.stdout.flush()",
        serde_json::to_string(&projection.to_string_lossy()).unwrap()
    );
    let temp = TempDir::new().unwrap();
    let (script, pid) = peer(&temp, &produce);
    let (output, response) = run(&script, &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(response["status"], "ok");
    reaped(&pid);
    let raw = Command::new("/usr/bin/python3")
        .arg("-I")
        .arg("-B")
        .arg(&script)
        .output()
        .unwrap();
    assert!(raw.status.success());
    let exact_limit = raw.stdout.len().to_string();
    let (output, response) = run(
        &script,
        &[("WORK_ENGINE_COMPILER_AEG_PIPE_LIMIT", &exact_limit)],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(response["status"], "ok");
    reaped(&pid);
    let bad = produce.replace(
        "hashlib.sha256(Path(__file__).read_bytes()).hexdigest()",
        "'f'*64",
    );
    let temp = TempDir::new().unwrap();
    let (script, pid) = peer(&temp, &bad);
    let (output, response) = run(&script, &[]);
    assert!(!output.status.success());
    assert_eq!(code(&response), "aeg_protocol");
    reaped(&pid);
    let changed=produce.replace("sys.stdout.write(json.dumps(r,default=str))","r['projection']['environment_context']['document_id']='other-document';sys.stdout.write(json.dumps(r,default=str))");
    let temp = TempDir::new().unwrap();
    let (script, pid) = peer(&temp, &changed);
    let (output, response) = run(&script, &[]);
    assert!(!output.status.success());
    assert_eq!(code(&response), "source_mismatch");
    reaped(&pid);
}

#[test]
fn python_envelope_requires_complete_strict_identity_and_projection() {
    let projection = root().join("docs/agent-environment-views/slice-supervisor.yaml");
    let base = format!(
        "import yaml\np=yaml.safe_load(Path({}).read_text())\nr={{'schema_version':1,'status':'closed_projection','backend':'work-engine.agent-environment-graph.v1','backend_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'canonical_role_match':True,'projection':p}}\n",
        serde_json::to_string(&projection.to_string_lossy()).unwrap()
    );
    for (mutation, expected) in [
        ("r.pop('status')", "aeg_protocol"),
        ("r['extra']=1", "aeg_protocol"),
        ("r['status']=True", "aeg_protocol"),
        ("r['schema_version']=2", "aeg_protocol"),
        ("r['backend']='wrong'", "aeg_protocol"),
        ("r['canonical_role_match']=False", "aeg_protocol"),
        ("r['backend_sha256']='f'*64", "aeg_protocol"),
        ("r['projection']=[]", "aeg_protocol"),
        (
            "r['projection']['source_inputs']['invariant_catalog']['sha256']='f'*64",
            "aeg_protocol",
        ),
        (
            "r['projection']['environment_context']['document_id']='wrong-document'",
            "source_mismatch",
        ),
    ] {
        let temp = TempDir::new().unwrap();
        let (script, pid) = peer(
            &temp,
            &format!("{base}{mutation}\nsys.stdout.write(json.dumps(r,default=str))"),
        );
        let (output, response) = run(&script, &[]);
        assert!(!output.status.success(), "{mutation}");
        assert_eq!(code(&response), expected, "{mutation}");
        assert!(response.get("ir").is_none());
        reaped(&pid);
    }
    for suffix in [
        "sys.stdout.write(json.dumps(r,default=str)[:-1]+',\"status\":\"closed_projection\"}')",
        "sys.stdout.write(json.dumps(r,default=str)+'{}')",
    ] {
        let temp = TempDir::new().unwrap();
        let (script, pid) = peer(&temp, &format!("{base}{suffix}"));
        let (output, response) = run(&script, &[]);
        assert!(!output.status.success(), "{suffix}");
        assert_eq!(code(&response), "aeg_protocol", "{suffix}");
        reaped(&pid);
    }
}

#[test]
fn missing_script_and_mismatched_script_pin_refuse_before_python() {
    let temp = TempDir::new().unwrap();
    let (script, pid) = peer(&temp, "time.sleep(0.01)");
    let missing = temp.path().join("missing.py");
    let (output, response) = run(
        &script,
        &[("WORK_ENGINE_COMPILER_AEG_SCRIPT", missing.to_str().unwrap())],
    );
    assert!(!output.status.success());
    assert_eq!(code(&response), "source_unavailable");
    assert!(!pid.exists());
    let (output, response) = run(
        &script,
        &[(
            "WORK_ENGINE_COMPILER_AEG_SCRIPT_SHA256",
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        )],
    );
    assert!(!output.status.success());
    assert_eq!(code(&response), "source_mismatch");
    assert!(!pid.exists());
}

#[test]
fn rust_signals_cancel_and_reap_its_direct_python_child() {
    for signal in ["-TERM", "-INT"] {
        let temp = TempDir::new().unwrap();
        let (script, pid) = peer(&temp, "time.sleep(30)");
        let root = root();
        let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-compiler"))
            .env("WORK_ENGINE_COMPILER_WORKSPACE_ROOT", &root)
            .env("WORK_ENGINE_COMPILER_PYTHON", "/usr/bin/python3")
            .env("WORK_ENGINE_COMPILER_AEG_SCRIPT", &script)
            .env(
                "WORK_ENGINE_COMPILER_AEG_SCRIPT_SHA256",
                sha256_hex(std::fs::read(&script).unwrap()),
            )
            .env(
                "WORK_ENGINE_COMPILER_AEG_INVARIANTS",
                root.join("docs/workflow-invariants.md"),
            )
            .env(
                "WORK_ENGINE_COMPILER_AEG_ENVIRONMENTS",
                root.join("docs/agent-environments.yaml"),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&request()).unwrap();
        let start = std::time::Instant::now();
        while !pid.exists() && start.elapsed() < std::time::Duration::from_secs(2) {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(pid.exists(), "Python peer did not start");
        assert!(
            Command::new("kill")
                .arg(signal)
                .arg(child.id().to_string())
                .status()
                .unwrap()
                .success()
        );
        let output = child.wait_with_output().unwrap();
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(!output.status.success());
        assert_eq!(code(&response), "cancelled");
        reaped(&pid);
    }
}

#[test]
fn missing_interpreter_and_absent_pyyaml_refuse_without_success() {
    let temp = TempDir::new().unwrap();
    let (script, pid) = peer(&temp, "time.sleep(0.01)");
    let (output, response) = run(
        &script,
        &[("WORK_ENGINE_COMPILER_PYTHON", "/nonexistent/python")],
    );
    assert!(!output.status.success());
    assert_eq!(code(&response), "aeg_unavailable");
    assert!(!pid.exists());
    let venv = temp.path().join("venv");
    assert!(
        Command::new("/usr/bin/python3")
            .arg("-m")
            .arg("venv")
            .arg("--without-pip")
            .arg(&venv)
            .status()
            .unwrap()
            .success()
    );
    let venv_python = venv.join("bin/python");
    let root = root();
    let real_script =
        root.join("skills/agent-environment-graph/scripts/agent_environment_graph.py");
    let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-compiler"))
        .env("WORK_ENGINE_COMPILER_WORKSPACE_ROOT", &root)
        .env("WORK_ENGINE_COMPILER_PYTHON", &venv_python)
        .env("WORK_ENGINE_COMPILER_AEG_SCRIPT", &real_script)
        .env(
            "WORK_ENGINE_COMPILER_AEG_SCRIPT_SHA256",
            sha256_hex(std::fs::read(&real_script).unwrap()),
        )
        .env(
            "WORK_ENGINE_COMPILER_AEG_INVARIANTS",
            root.join("docs/workflow-invariants.md"),
        )
        .env(
            "WORK_ENGINE_COMPILER_AEG_ENVIRONMENTS",
            root.join("docs/agent-environments.yaml"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&request()).unwrap();
    let output = child.wait_with_output().unwrap();
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!output.status.success());
    assert_eq!(code(&response), "aeg_failed");
    assert!(response.get("ir").is_none());
}
