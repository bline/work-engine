use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::io::Write;
use std::os::unix::fs::symlink;
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
fn fixture() -> PathBuf {
    repo().join("app-server/tests/fixtures/compiler-c2/root")
}
fn case(id: &str) -> (String, String, Vec<u8>) {
    let p = repo()
        .join("rust/crates/work-engine-compiler/tests/fixtures/v1/skills")
        .join(id);
    (
        std::fs::read_to_string(p.join("structure.yaml")).unwrap(),
        std::fs::read_to_string(p.join("interface.yaml")).unwrap(),
        std::fs::read(p.join("expected.md")).unwrap(),
    )
}
fn run(root: &Path, structure: &str, interface: &str, script: Option<&Path>) -> (Output, Value) {
    let script = script.map(Path::to_path_buf).unwrap_or_else(|| {
        fixture().join("skills/agent-environment-graph/scripts/agent_environment_graph.py")
    });
    let req = json!({"schema_version":2,"request_id":"verified-test","operation":"compile_skill_verified","structure_source_base64":STANDARD.encode(structure),"interface_source_base64":STANDARD.encode(interface)});
    let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-compiler"))
        .env("WORK_ENGINE_COMPILER_WORKSPACE_ROOT", root)
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
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&req).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let response = serde_json::from_slice(&output.stdout).unwrap();
    (output, response)
}
fn error(value: &Value) -> &str {
    value["error"]["code"].as_str().unwrap()
}

#[test]
fn five_pinned_cases_verify_and_drifted_live_builder_refuses() {
    for id in ["p-sup", "p-bld", "p-rep", "p-clm", "p-air"] {
        let (structure, interface, expected) = case(id);
        let (out, response) = run(&fixture(), &structure, &interface, None);
        assert!(
            out.status.success(),
            "{id}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            STANDARD
                .decode(response["output_base64"].as_str().unwrap())
                .unwrap(),
            expected,
            "{id}"
        );
        assert_eq!(
            response["ir"]["runtime_requirements"]["verified_sources"],
            true
        );
        assert_eq!(response["request_id"], "verified-test");
        assert!(
            response["verification"]["sources"]
                .as_array()
                .unwrap()
                .len()
                >= 2
        );
        if id == "p-sup" || id == "p-bld" {
            assert_eq!(
                response["verification"]["python"]["direct_child_reaped"],
                true
            )
        } else {
            assert!(response["verification"].get("python").is_none())
        }
    }
    let temp = TempDir::new().unwrap();
    for rel in [
        "skills/slice-builder/SKILL.md",
        "docs/agent-environments.yaml",
        "docs/workflow-invariants.md",
        "docs/agent-environment-views/slice-builder.yaml",
    ] {
        let target = temp.path().join(rel);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(fixture().join(rel), &target).unwrap();
    }
    std::fs::write(
        temp.path().join("skills/slice-builder/SKILL.md"),
        b"drifted live builder source",
    )
    .unwrap();
    let (structure, interface, _) = case("p-bld");
    let (out, response) = run(temp.path(), &structure, &interface, None);
    assert!(!out.status.success());
    assert_eq!(error(&response), "source_mismatch");
    assert!(response.get("ir").is_none());
    let marker = temp.path().join("python-was-launched");
    let peer = temp.path().join("peer.py");
    std::fs::write(
        &peer,
        format!(
            "from pathlib import Path\nPath({}).touch()\n",
            serde_json::to_string(&marker.to_string_lossy()).unwrap()
        ),
    )
    .unwrap();
    let (out, response) = run(temp.path(), &structure, &interface, Some(&peer));
    assert!(!out.status.success());
    assert_eq!(error(&response), "source_mismatch");
    assert!(!marker.exists(), "mismatched source launched Python");
}
#[test]
fn lexical_paths_symlinks_and_nonregular_sources_are_bounded() {
    let (structure, interface, expected) = case("p-rep");
    let old = "skills/repo-search/SKILL.md";
    for replacement in [
        format!("app-server/../{old}"),
        fixture().join(old).to_string_lossy().into_owned(),
    ] {
        let changed = structure.replace(old, &replacement);
        let (out, response) = run(&fixture(), &changed, &interface, None);
        assert!(
            out.status.success(),
            "{replacement}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            STANDARD
                .decode(response["output_base64"].as_str().unwrap())
                .unwrap(),
            expected
        );
    }
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();
    for other in [
        "skills/repo-search/agents/openai.yaml",
        "skills/repo-search/references/backend-capabilities.md",
    ] {
        let p = root.join(other);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::copy(fixture().join(other), p).unwrap();
    }
    let external = temp.path().join("external.md");
    std::fs::copy(fixture().join(old), &external).unwrap();
    let alias = root.join("alias.md");
    symlink(&external, &alias).unwrap();
    let changed = structure.replace(old, "alias.md");
    let (out, response) = run(&root, &changed, &interface, None);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(response["status"], "ok");
    std::fs::remove_file(&alias).unwrap();
    std::fs::write(&external, b"drifted after first capture").unwrap();
    symlink(&external, &alias).unwrap();
    let (out, response) = run(&root, &changed, &interface, None);
    assert!(!out.status.success());
    assert_eq!(error(&response), "source_mismatch");
    std::fs::remove_file(&alias).unwrap();
    let (out, response) = run(&root, &changed, &interface, None);
    assert!(!out.status.success());
    assert_eq!(error(&response), "source_unavailable");
    assert!(
        Command::new("mkfifo")
            .arg(&alias)
            .status()
            .unwrap()
            .success()
    );
    let (out, response) = run(&root, &changed, &interface, None);
    assert!(!out.status.success());
    assert_eq!(error(&response), "source_unavailable");
    std::fs::remove_file(&alias).unwrap();
    std::fs::write(&alias, vec![b'x'; 4 * 1024 * 1024 + 1]).unwrap();
    let (out, response) = run(&root, &changed, &interface, None);
    assert!(!out.status.success());
    assert_eq!(error(&response), "resource_limit");
}
#[test]
fn v2_request_rejects_duplicate_unknown_and_invalid_fields() {
    let cases=[
        (br#"{"schema_version":2,"request_id":"x","request_id":"y","operation":"compile_skill_unverified","structure_source_base64":"","interface_source_base64":""}"#.as_slice(),"invalid_request"),
        (br#"{"schema_version":2,"request_id":"x","operation":"compile_skill_verified","structure_source_base64":"","interface_source_base64":"","verified_sources":true}"#.as_slice(),"invalid_request"),
        (br#"{"schema_version":2,"request_id":"","operation":"compile_skill_verified","structure_source_base64":"","interface_source_base64":""}"#.as_slice(),"invalid_request"),
    ];
    for (input, expected) in cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-compiler"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(!out.status.success());
        let response: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(error(&response), expected);
        assert!(response.get("ir").is_none());
    }
}

#[test]
fn mutation_after_capture_keeps_captured_skill_and_script_identity() {
    let temp = TempDir::new().unwrap();
    for rel in [
        "skills/slice-supervisor/SKILL.md",
        "docs/agent-environments.yaml",
        "docs/workflow-invariants.md",
        "docs/agent-environment-views/slice-supervisor.yaml",
    ] {
        let target = temp.path().join(rel);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(fixture().join(rel), target).unwrap();
    }
    let skill = temp.path().join("skills/slice-supervisor/SKILL.md");
    let projection = temp
        .path()
        .join("docs/agent-environment-views/slice-supervisor.yaml");
    let peer = temp.path().join("original-peer.py");
    let body = format!(
        "import json,hashlib,yaml,sys\nfrom pathlib import Path\nPath({}).write_bytes(b'changed after capture')\nPath({}).write_bytes(b'original script replaced after capture')\np=yaml.safe_load(Path({}).read_text())\nr={{'schema_version':1,'status':'closed_projection','backend':'work-engine.agent-environment-graph.v1','backend_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'canonical_role_match':True,'projection':p}}\nsys.stdout.write(json.dumps(r,default=str))\n",
        serde_json::to_string(&skill.to_string_lossy()).unwrap(),
        serde_json::to_string(&peer.to_string_lossy()).unwrap(),
        serde_json::to_string(&projection.to_string_lossy()).unwrap()
    );
    std::fs::write(&peer, body).unwrap();
    let original_script_hash = sha256_hex(std::fs::read(&peer).unwrap());
    let (structure, interface, expected) = case("p-sup");
    let (out, response) = run(temp.path(), &structure, &interface, Some(&peer));
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        STANDARD
            .decode(response["output_base64"].as_str().unwrap())
            .unwrap(),
        expected
    );
    assert_eq!(
        response["verification"]["python"]["script_sha256"],
        original_script_hash
    );
    assert_ne!(
        sha256_hex(std::fs::read(&peer).unwrap()),
        original_script_hash
    );
    assert_ne!(
        sha256_hex(std::fs::read(&skill).unwrap()),
        response["ir"]["source"]["sha256"].as_str().unwrap()
    );
}

#[test]
fn real_python_refuses_candidate_role_that_differs_from_canonical() {
    let (structure, interface, _) = case("p-sup");
    let divergent = structure.replacen("label: Slice Supervisor", "label: Divergent Supervisor", 1);
    let (out, response) = run(&fixture(), &divergent, &interface, None);
    assert!(!out.status.success());
    assert_eq!(error(&response), "aeg_failed");
    assert!(response.get("ir").is_none());
}
