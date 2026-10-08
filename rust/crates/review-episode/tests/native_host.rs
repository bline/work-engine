#[allow(dead_code)]
mod support;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
#[cfg(feature = "test-faults")]
use std::time::{Duration, Instant};

use review_episode_core::codec::{JsValue, canonical_json, digest, field, parse_json};
use review_episode_core::identity::Authority;
use support::{ACCEPTABLE, BIN, authority, bounded_output, revision, status};

fn changed(value: &JsValue, name: &str, replacement: JsValue) -> JsValue {
    let mut fields = value.as_object().unwrap().clone();
    fields.insert(name.into(), replacement);
    JsValue::Object(fields)
}

fn native_request(
    operation: &str,
    grant: &str,
    request_id: &str,
    selection: &JsValue,
    args: JsValue,
) -> JsValue {
    JsValue::object([
        ("version", JsValue::Number(2.0)),
        ("domain", JsValue::text("review-episode")),
        ("profile", JsValue::text("native-host-v1")),
        ("requestId", JsValue::text(request_id)),
        ("operation", JsValue::text(operation)),
        ("grantId", JsValue::text(grant)),
        ("selectionRevision", JsValue::text(&"a".repeat(64))),
        (
            "rootSelectionDigest",
            field(selection, "selectionDigest").unwrap().clone(),
        ),
        ("args", args),
    ])
}

fn descriptor(request: &JsValue, selection: &JsValue, observed: Option<&str>) -> JsValue {
    let operation = field(request, "operation")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let args = field(request, "args").unwrap();
    let authority = args.get("authority");
    let identity = authority
        .and_then(|a| a.get("identity"))
        .unwrap_or_else(|| args.get("identity").unwrap());
    let key = review_episode_core::identity::Identity::parse(identity)
        .unwrap()
        .key()
        .0;
    let payload = args.get("payload");
    let result = args
        .get("result")
        .or_else(|| payload.and_then(|p| p.get("result")));
    let evidence = payload.and_then(|p| p.get("evidenceAdmissions"));
    let content = match operation.as_str() {
        "begin" => Some(JsValue::object([
            ("action", JsValue::text("begin")),
            (
                "unresolvedQuestions",
                args.get("unresolvedQuestions").unwrap().clone(),
            ),
        ])),
        "transition" => Some(JsValue::object([
            ("action", args.get("action").unwrap().clone()),
            ("payload", payload.unwrap().clone()),
        ])),
        _ => None,
    };
    let optional = |value: Option<&JsValue>| {
        value.map_or(JsValue::Null, |value| JsValue::text(&digest(value)))
    };
    let access = if matches!(operation.as_str(), "read" | "history" | "recover") {
        "read"
    } else {
        "write"
    };
    JsValue::object([
        ("schemaVersion", JsValue::Number(1.0)),
        ("profile", JsValue::text("native-host-v1")),
        ("root", field(selection, "root").unwrap().clone()),
        (
            "executableSha256",
            field(selection, "executableSha256").unwrap().clone(),
        ),
        (
            "rootSelectionDigest",
            field(selection, "selectionDigest").unwrap().clone(),
        ),
        ("requestId", field(request, "requestId").unwrap().clone()),
        ("operation", field(request, "operation").unwrap().clone()),
        ("grantId", field(request, "grantId").unwrap().clone()),
        (
            "selectionRevision",
            field(request, "selectionRevision").unwrap().clone(),
        ),
        ("requestDigest", JsValue::text(&digest(request))),
        ("identityKey", JsValue::text(&key)),
        ("argsDigest", JsValue::text(&digest(args))),
        ("authorityDigest", optional(authority)),
        ("resultDigest", optional(result)),
        ("evidenceDigest", optional(evidence)),
        ("contentDigest", optional(content.as_ref())),
        (
            "observedRevision",
            observed.map_or(JsValue::Null, JsValue::text),
        ),
        (
            "principal",
            JsValue::object([
                ("id", JsValue::text("native-review-host")),
                ("identityKey", JsValue::text(&key)),
                ("access", JsValue::text(access)),
            ]),
        ),
    ])
}

fn spawn_native(
    root: &Path,
    request: &JsValue,
    descriptor: &JsValue,
    fault: Option<(&str, &Path)>,
) -> std::process::Child {
    let file = root.parent().unwrap().join("native-descriptor");
    fs::write(&file, canonical_json(descriptor)).unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    let read_only = OpenOptions::new().read(true).open(&file).unwrap();
    fs::remove_file(&file).unwrap();
    let fd = read_only.as_raw_fd();
    let mut command = Command::new(BIN);
    command
        .arg("native-host-v1")
        .arg("--root")
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some((cut, barrier)) = fault {
        command
            .env("REVIEW_EPISODE_FAULT_CUT", cut)
            .env("REVIEW_EPISODE_FAULT_DIR", barrier);
    }
    unsafe {
        command.pre_exec(move || {
            unsafe extern "C" {
                fn dup2(oldfd: i32, newfd: i32) -> i32;
                fn fcntl(fd: i32, command: i32, value: i32) -> i32;
            }
            if dup2(fd, 3) < 0 || (fd == 3 && fcntl(3, 2, 0) < 0) {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    let body = canonical_json(request);
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(&(body.len() as u32).to_be_bytes()).unwrap();
        stdin.write_all(body.as_bytes()).unwrap();
    }
    child
}

fn invoke(root: &Path, request: &JsValue, descriptor: &JsValue) -> JsValue {
    let child = spawn_native(root, request, descriptor, None);
    let output = bounded_output(child);
    assert!(
        output.status.success(),
        "native command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.len() >= 4);
    let length = u32::from_be_bytes(output.stdout[..4].try_into().unwrap()) as usize;
    assert_eq!(output.stdout.len(), length + 4);
    parse_json(std::str::from_utf8(&output.stdout[4..]).unwrap()).unwrap()
}

#[test]
fn native_binary_begin_result_reopen_and_exact_read() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("native-root");
    let init = Command::new(BIN)
        .args(["init-native", "--root"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let marker =
        parse_json(&fs::read_to_string(root.join(".review-episode-native-host-v1")).unwrap())
            .unwrap();
    let authority = authority();
    let key = Authority::parse(authority.clone())
        .unwrap()
        .identity()
        .key()
        .0;
    let begin = native_request(
        "begin",
        "native-begin",
        "begin-1",
        &marker,
        JsValue::object([
            ("authority", authority.clone()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    let first = invoke(&root, &begin, &descriptor(&begin, &marker, None));
    assert_eq!(status(&first), "applied", "{first:?}");
    assert_eq!(
        field(&first, "source").unwrap(),
        &JsValue::text("native_host")
    );
    assert_eq!(
        field(&first, "requestDigest").unwrap(),
        &JsValue::text(&digest(&begin))
    );
    let first_revision = revision(&first);
    let result = parse_json(ACCEPTABLE).unwrap();
    let transition = native_request(
        "transition",
        "native-result",
        "result-1",
        &marker,
        JsValue::object([
            ("authority", authority),
            ("expectedRevision", JsValue::text(&first_revision)),
            ("transitionId", JsValue::text("result")),
            ("action", JsValue::text("record_result")),
            (
                "payload",
                JsValue::object([
                    ("result", result),
                    ("unresolvedQuestions", JsValue::Array(vec![])),
                ]),
            ),
        ]),
    );
    let second = invoke(
        &root,
        &transition,
        &descriptor(&transition, &marker, Some(&first_revision)),
    );
    assert_eq!(status(&second), "applied", "{second:?}");
    let read = native_request(
        "read",
        "native-reader",
        "read-1",
        &marker,
        JsValue::object([
            (
                "identity",
                Authority::parse(
                    field(field(&begin, "args").unwrap(), "authority")
                        .unwrap()
                        .clone(),
                )
                .unwrap()
                .identity()
                .value(),
            ),
            ("revision", JsValue::text(&first_revision)),
        ]),
    );
    let exact = invoke(&root, &read, &descriptor(&read, &marker, None));
    assert_eq!(status(&exact), "observed", "{exact:?}");
    assert_eq!(
        field(field(&exact, "state").unwrap(), "revision").unwrap(),
        &JsValue::text(&first_revision)
    );
    assert_eq!(key.len(), 64);
}

#[test]
fn native_descriptor_rejects_wrong_binding_before_disclosing_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("native-root");
    let init = Command::new(BIN)
        .args(["init-native", "--root"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(init.status.success());
    let marker =
        parse_json(&fs::read_to_string(root.join(".review-episode-native-host-v1")).unwrap())
            .unwrap();
    let request = native_request(
        "begin",
        "native-begin",
        "begin-1",
        &marker,
        JsValue::object([
            ("authority", authority()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    let valid = descriptor(&request, &marker, None);
    for (name, replacement) in [
        ("root", JsValue::text("/other-root")),
        ("executableSha256", JsValue::text(&"f".repeat(64))),
        ("rootSelectionDigest", JsValue::text(&"e".repeat(64))),
        ("requestDigest", JsValue::text(&"d".repeat(64))),
        ("argsDigest", JsValue::text(&"c".repeat(64))),
        ("identityKey", JsValue::text(&"b".repeat(64))),
        ("authorityDigest", JsValue::Null),
        ("contentDigest", JsValue::Null),
        ("observedRevision", JsValue::text(&"a".repeat(64))),
    ] {
        let rejected = invoke(&root, &request, &changed(&valid, name, replacement));
        assert_eq!(status(&rejected), "error", "{name}: {rejected:?}");
        assert_eq!(
            field(&rejected, "kind").unwrap(),
            &JsValue::text("Admission")
        );
        assert!(rejected.get("state").is_none(), "{name} disclosed state");
    }
    let principal = changed(
        field(&valid, "principal").unwrap(),
        "access",
        JsValue::text("read"),
    );
    let rejected = invoke(&root, &request, &changed(&valid, "principal", principal));
    assert_eq!(
        field(&rejected, "kind").unwrap(),
        &JsValue::text("Admission")
    );
    let applied = invoke(&root, &request, &valid);
    assert_eq!(status(&applied), "applied");
    let replay = invoke(
        &root,
        &request,
        &descriptor(&request, &marker, Some(&revision(&applied))),
    );
    assert_eq!(status(&replay), "replay");
}

#[cfg(feature = "test-faults")]
#[test]
fn native_host_process_cuts_recover_exactly_one_transition() {
    for cut in [
        "after_admission",
        "after_begin_immediate",
        "after_history_insert",
        "after_current_update",
        "before_commit",
        "after_commit",
        "before_reply",
        "after_partial_reply",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("native-root");
        let barrier = temp.path().join("barrier");
        fs::create_dir(&barrier).unwrap();
        let init = Command::new(BIN)
            .args(["init-native", "--root"])
            .arg(&root)
            .output()
            .unwrap();
        assert!(init.status.success(), "cut {cut}");
        let marker =
            parse_json(&fs::read_to_string(root.join(".review-episode-native-host-v1")).unwrap())
                .unwrap();
        let authority = authority();
        let identity = field(&authority, "identity").unwrap().clone();
        let begin = native_request(
            "begin",
            "begin-grant",
            "begin-cut",
            &marker,
            JsValue::object([
                ("authority", authority),
                ("transitionId", JsValue::text("begin")),
                ("unresolvedQuestions", JsValue::Array(vec![])),
            ]),
        );
        let mut child = spawn_native(
            &root,
            &begin,
            &descriptor(&begin, &marker, None),
            Some((cut, &barrier)),
        );
        let ready = barrier.join(format!("{cut}.ready"));
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            if let Some(exit) = child.try_wait().unwrap() {
                panic!("native child exited before {cut}: {exit}");
            }
            if Instant::now() > deadline {
                child.kill().unwrap();
                panic!("native child missed {cut} barrier");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        child.kill().unwrap();
        let output = child.wait_with_output().unwrap();
        if cut == "after_partial_reply" {
            let claimed = u32::from_be_bytes(output.stdout[..4].try_into().unwrap()) as usize;
            assert!(output.stdout.len() < claimed + 4, "partial reply completed");
        }
        let content = digest(&JsValue::object([
            ("action", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]));
        let recovery = native_request(
            "recover",
            "read-grant",
            "recover-cut",
            &marker,
            JsValue::object([
                ("identity", identity.clone()),
                ("transitionId", JsValue::text("begin")),
                ("contentDigest", JsValue::text(&content)),
            ]),
        );
        let recovered = invoke(&root, &recovery, &descriptor(&recovery, &marker, None));
        let committed = matches!(cut, "after_commit" | "before_reply" | "after_partial_reply");
        assert_eq!(
            status(&recovered),
            if committed { "committed" } else { "absent" },
            "cut {cut}: {recovered:?}"
        );
        let observed = recovered.get("observedRevision").unwrap();
        let observed = if observed == &JsValue::Null {
            None
        } else {
            Some(observed.as_text().unwrap().to_string_checked().unwrap())
        };
        let retry = invoke(
            &root,
            &begin,
            &descriptor(&begin, &marker, observed.as_deref()),
        );
        assert_eq!(
            status(&retry),
            if committed { "replay" } else { "applied" },
            "cut {cut}"
        );
        let history = native_request(
            "history",
            "read-grant",
            "history-cut",
            &marker,
            JsValue::object([("identity", identity)]),
        );
        let states = invoke(&root, &history, &descriptor(&history, &marker, None));
        assert_eq!(
            field(&states, "history").unwrap().as_array().unwrap().len(),
            1,
            "cut {cut}"
        );
    }
}
