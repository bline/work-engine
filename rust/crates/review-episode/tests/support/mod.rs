use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use review_episode_core::codec::{JsValue, canonical_json, digest, field, parse_json};
use sha2::{Digest, Sha256};

pub const BIN: &str = env!("CARGO_BIN_EXE_work-engine-review-episode");
pub const ACCEPTABLE: &str = include_str!(
    "../../../../../app-server/tests/fixtures/implementation-review/acceptable-as-is.json"
);
pub const COMMANDS: &str =
    include_str!("../../../../../app-server/tests/fixtures/review-episode-rust/commands.json");

pub fn bounded_output(mut child: Child) -> Output {
    let mut stdout = child.stdout.take().expect("child stdout is piped");
    let mut stderr = child.stderr.take().expect("child stderr is piped");
    let out_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let err_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            let _ = child.wait();
            let _ = out_reader.join();
            let _ = err_reader.join();
            panic!("review-episode child timed out");
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    Output {
        status,
        stdout: out_reader.join().unwrap(),
        stderr: err_reader.join().unwrap(),
    }
}

pub fn raw_sha(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}
pub fn reference(owner: &str, name: &str, revision: &str, raw: &str) -> JsValue {
    JsValue::object([
        ("owner", JsValue::text(owner)),
        ("reference", JsValue::text(name)),
        ("revision", JsValue::text(revision)),
        ("sha256", JsValue::text(&raw_sha(raw))),
        ("freshness", JsValue::text("exact_revision")),
    ])
}
pub fn authority() -> JsValue {
    let fixture = parse_json(COMMANDS).unwrap();
    let identity = field(&fixture, "identity").unwrap().clone();
    let result = parse_json(ACCEPTABLE).unwrap();
    let subject = field(&result, "subject").unwrap();
    let mut initial = reference(
        "checkpoint",
        "candidate",
        "candidate-commit",
        "candidate-commit",
    );
    if let JsValue::Object(map) = &mut initial {
        map.insert("revision".into(), field(subject, "commit").unwrap().clone());
        map.insert("sha256".into(), JsValue::text(&digest(subject)));
    }
    JsValue::object([
        ("schemaVersion", JsValue::Number(1.0)),
        ("grantId", JsValue::text("grant-1")),
        ("identity", identity),
        (
            "source",
            reference("human", "accepted-plan", "plan-v1", "accepted-plan"),
        ),
        (
            "writer",
            JsValue::object([
                ("actorId", JsValue::text("reviewer")),
                ("provider", JsValue::text("fixture")),
                ("generation", JsValue::Number(1.0)),
                (
                    "runtimeSession",
                    reference("runtime", "session-1", "generation-1", "session-1"),
                ),
            ]),
        ),
        (
            "readers",
            JsValue::Array(vec![
                JsValue::text("reviewer"),
                JsValue::text("builder"),
                JsValue::text("supervisor"),
            ]),
        ),
        ("initialSubject", initial),
        ("predecessorRevision", JsValue::Null),
    ])
}
pub fn request(operation: &str, grant: &str, id: &str, args: JsValue) -> JsValue {
    JsValue::object([
        ("version", JsValue::Number(1.0)),
        ("domain", JsValue::text("review-episode")),
        ("requestId", JsValue::text(id)),
        ("operation", JsValue::text(operation)),
        ("grantId", JsValue::text(grant)),
        ("selectionRevision", JsValue::text(&"a".repeat(64))),
        ("args", args),
    ])
}
pub fn init(root: &Path) {
    let child = Command::new(BIN)
        .args(["init-offline", "--root"])
        .arg(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = bounded_output(child);
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
pub fn write_fixture(path: &Path, root: &Path, request: &JsValue, observed: Option<&str>) {
    let root = root.canonicalize().unwrap();
    let grant = field(request, "grantId").unwrap().clone();
    let registry = JsValue::object([
        ("root", JsValue::text(root.to_str().unwrap())),
        (
            "grants",
            JsValue::Array(vec![JsValue::object([
                ("grantId", grant),
                ("requestDigest", JsValue::text(&digest(request))),
                (
                    "selectionRevision",
                    field(request, "selectionRevision").unwrap().clone(),
                ),
                (
                    "observedRevision",
                    observed.map_or(JsValue::Null, JsValue::text),
                ),
            ])]),
        ),
    ]);
    fs::write(path, canonical_json(&registry)).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}
pub fn invoke(root: &Path, fixture: &Path, request: &JsValue) -> JsValue {
    let mut child = Command::new(BIN)
        .arg("offline")
        .arg("--root")
        .arg(root)
        .arg("--fixture")
        .arg(fixture)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let body = canonical_json(request);
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(&(body.len() as u32).to_be_bytes()).unwrap();
        stdin.write_all(body.as_bytes()).unwrap();
    }
    let output = bounded_output(child);
    assert!(
        output.stdout.len() >= 4,
        "no frame: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let len = u32::from_be_bytes(output.stdout[..4].try_into().unwrap()) as usize;
    assert_eq!(output.stdout.len(), len + 4, "partial/trailing frame");
    parse_json(std::str::from_utf8(&output.stdout[4..]).unwrap()).unwrap()
}
pub fn status(response: &JsValue) -> String {
    field(response, "status")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap()
}
pub fn revision(response: &JsValue) -> String {
    field(response, "observedRevision")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap()
}
