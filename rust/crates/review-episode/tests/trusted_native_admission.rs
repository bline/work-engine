#![cfg(unix)]
#[allow(dead_code)]
mod support;

use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

use review_episode::admission::HostAdmissionPort;
use review_episode::host_admission::NativeHostAdmission;
use review_episode::in_process_admission::{
    EpisodeReadRequest, EpisodeReadTarget, TrustedEpisodeReadScope,
};
use review_episode::protocol::{DEFAULT_RESPONSE_LIMIT, Request};
use review_episode::{AppResult, Application};
use review_episode_core::codec::{JsValue, canonical_json, digest, field, parse_json};
use review_episode_core::identity::{Authority, Revision};
use review_episode_store::{
    EpisodeStore, NativeRootSelection, StoreOptions, init_native_root, read_native_selection,
};
use support::{ACCEPTABLE, BIN, authority, bounded_output, revision, status};

fn changed(value: &JsValue, name: &str, replacement: JsValue) -> JsValue {
    let mut fields = value.as_object().unwrap().clone();
    fields.insert(name.into(), replacement);
    JsValue::Object(fields)
}

fn selected_authority() -> Authority {
    Authority::parse(changed(
        &authority(),
        "readers",
        JsValue::Array(vec![
            JsValue::text("reviewer"),
            JsValue::text("native-review-host"),
        ]),
    ))
    .unwrap()
}

fn request(operation: &str, id: &str, selection: &NativeRootSelection, args: JsValue) -> JsValue {
    JsValue::object([
        ("version", JsValue::Number(2.0)),
        ("domain", JsValue::text("review-episode")),
        ("profile", JsValue::text("native-host-v1")),
        ("requestId", JsValue::text(id)),
        ("operation", JsValue::text(operation)),
        ("grantId", JsValue::text(id)),
        ("selectionRevision", JsValue::text(&"a".repeat(64))),
        (
            "rootSelectionDigest",
            JsValue::text(&selection.selection_digest),
        ),
        ("args", args),
    ])
}

fn descriptor(
    request: &JsValue,
    selection: &NativeRootSelection,
    observed: Option<&str>,
) -> JsValue {
    let operation = field(request, "operation")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let args = field(request, "args").unwrap();
    let authority = args.get("authority");
    let identity = authority.and_then(|a| a.get("identity")).unwrap();
    let key = review_episode_core::identity::Identity::parse(identity)
        .unwrap()
        .key()
        .0;
    let content = match operation.as_str() {
        "begin" => JsValue::object([
            ("action", JsValue::text("begin")),
            (
                "unresolvedQuestions",
                args.get("unresolvedQuestions").unwrap().clone(),
            ),
        ]),
        "transition" => JsValue::object([
            ("action", args.get("action").unwrap().clone()),
            ("payload", args.get("payload").unwrap().clone()),
        ]),
        _ => unreachable!(),
    };
    let result = args.get("payload").and_then(|p| p.get("result"));
    let evidence = args
        .get("payload")
        .and_then(|p| p.get("evidenceAdmissions"));
    JsValue::object([
        ("schemaVersion", JsValue::Number(1.0)),
        ("profile", JsValue::text("native-host-v1")),
        ("root", JsValue::text(&selection.root)),
        (
            "executableSha256",
            JsValue::text(&selection.executable_sha256),
        ),
        (
            "rootSelectionDigest",
            JsValue::text(&selection.selection_digest),
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
        (
            "authorityDigest",
            authority.map_or(JsValue::Null, |a| JsValue::text(&digest(a))),
        ),
        (
            "resultDigest",
            result.map_or(JsValue::Null, |r| JsValue::text(&digest(r))),
        ),
        (
            "evidenceDigest",
            evidence.map_or(JsValue::Null, |value| JsValue::text(&digest(value))),
        ),
        ("contentDigest", JsValue::text(&digest(&content))),
        (
            "observedRevision",
            observed.map_or(JsValue::Null, JsValue::text),
        ),
        (
            "principal",
            JsValue::object([
                ("id", JsValue::text("native-review-host")),
                ("identityKey", JsValue::text(&key)),
                ("access", JsValue::text("write")),
            ]),
        ),
    ])
}

fn begin(selection: &NativeRootSelection) -> JsValue {
    request(
        "begin",
        "begin-1",
        selection,
        JsValue::object([
            ("authority", selected_authority().value().clone()),
            ("transitionId", JsValue::text("begin-1")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    )
}

fn record(selection: &NativeRootSelection, expected: &str) -> JsValue {
    request(
        "transition",
        "result-1",
        selection,
        JsValue::object([
            ("authority", selected_authority().value().clone()),
            ("transitionId", JsValue::text("result-1")),
            ("expectedRevision", JsValue::text(expected)),
            ("action", JsValue::text("record_result")),
            (
                "payload",
                JsValue::object([
                    ("result", parse_json(ACCEPTABLE).unwrap()),
                    ("unresolvedQuestions", JsValue::Array(vec![])),
                ]),
            ),
        ]),
    )
}

fn parsed(request: &JsValue) -> Request {
    Request::parse(canonical_json(request).as_bytes()).unwrap()
}

fn direct(selection: &NativeRootSelection, bytes: &[u8]) -> AppResult<NativeHostAdmission> {
    NativeHostAdmission::from_trusted_descriptor(selection.clone(), bytes)
}

fn check(
    admission: AppResult<NativeHostAdmission>,
    request: &JsValue,
    observed: Option<&str>,
) -> String {
    let result = admission.and_then(|port| {
        let request = parsed(request);
        port.precheck(&request)?;
        port.admit(&request, observed).map(|_| ())
    });
    match result {
        Ok(()) => "admitted".into(),
        Err(error) => format!("{}: {error}", error.kind()),
    }
}

// FD 3 belongs only to this subprocess, never to the parallel test runner.
#[test]
fn inherited_child_entry() {
    let Ok(root) = std::env::var("H0_ROOT") else {
        return;
    };
    let selection = read_native_selection(Path::new(&root)).unwrap();
    let request =
        parse_json(&fs::read_to_string(std::env::var("H0_REQUEST").unwrap()).unwrap()).unwrap();
    let observed = std::env::var("H0_OBSERVED").ok();
    let admission = NativeHostAdmission::from_inherited_fd(selection);
    let response = check(admission, &request, observed.as_deref());
    fs::write(std::env::var("H0_REPLY").unwrap(), response).unwrap();
}

fn inherited(
    selection: &NativeRootSelection,
    bytes: &[u8],
    request: &JsValue,
    observed: Option<&str>,
    provenance: &str,
) -> String {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("descriptor");
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(
        &path,
        fs::Permissions::from_mode(if provenance == "public" { 0o644 } else { 0o600 }),
    )
    .unwrap();
    let file = OpenOptions::new()
        .read(true)
        .write(provenance == "writable")
        .open(&path)
        .unwrap();
    if provenance != "linked" {
        fs::remove_file(&path).unwrap();
    }
    let request_path = temp.path().join("request");
    let reply_path = temp.path().join("reply");
    fs::write(&request_path, canonical_json(request)).unwrap();
    let fd = file.as_raw_fd();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "inherited_child_entry", "--nocapture"])
        .env("H0_ROOT", &selection.root)
        .env("H0_REQUEST", &request_path)
        .env("H0_REPLY", &reply_path)
        .env_remove("H0_OBSERVED")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(observed) = observed {
        command.env("H0_OBSERVED", observed);
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
    let output = bounded_output(command.spawn().unwrap());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read_to_string(reply_path).unwrap()
}

fn assert_parity(
    selection: &NativeRootSelection,
    bytes: &[u8],
    request: &JsValue,
    observed: Option<&str>,
    accepted: bool,
) {
    let actual = check(direct(selection, bytes), request, observed);
    assert_eq!(actual == "admitted", accepted, "{actual}");
    assert_eq!(
        inherited(selection, bytes, request, observed, "valid"),
        actual
    );
}

fn execute(
    selection: &NativeRootSelection,
    request: &JsValue,
    observed: Option<&str>,
) -> AppResult<JsValue> {
    let bytes = canonical_json(&descriptor(request, selection, observed));
    let admission = direct(selection, bytes.as_bytes())?;
    let (store, _) =
        EpisodeStore::open_native(Path::new(&selection.root), StoreOptions::default())?;
    let mut app = Application::new(store, admission, DEFAULT_RESPONSE_LIMIT);
    Ok(parse_json(&app.execute(&parsed(request))?)?)
}

#[test]
fn direct_and_inherited_bind_exact_commands_and_observation() {
    let temp = tempfile::tempdir().unwrap();
    let selection = init_native_root(&temp.path().join("root")).unwrap();
    let begin = begin(&selection);
    assert_parity(
        &selection,
        canonical_json(&descriptor(&begin, &selection, None)).as_bytes(),
        &begin,
        None,
        true,
    );
    let observed = "b".repeat(64);
    let record = record(&selection, &observed);
    let exact = descriptor(&record, &selection, Some(&observed));
    assert_parity(
        &selection,
        canonical_json(&exact).as_bytes(),
        &record,
        Some(&observed),
        true,
    );
    assert_parity(
        &selection,
        canonical_json(&exact).as_bytes(),
        &record,
        None,
        false,
    );
    assert_parity(
        &selection,
        canonical_json(&exact).as_bytes(),
        &record,
        Some(&"c".repeat(64)),
        false,
    );

    for name in [
        "requestId",
        "operation",
        "grantId",
        "selectionRevision",
        "requestDigest",
        "identityKey",
        "argsDigest",
        "authorityDigest",
        "resultDigest",
        "evidenceDigest",
        "contentDigest",
    ] {
        let altered = changed(&exact, name, JsValue::text(&"d".repeat(64)));
        assert_parity(
            &selection,
            canonical_json(&altered).as_bytes(),
            &record,
            Some(&observed),
            false,
        );
    }
    for (name, replacement) in [
        ("id", "other-host"),
        ("identityKey", "other-identity"),
        ("access", "read"),
    ] {
        let principal = changed(
            field(&exact, "principal").unwrap(),
            name,
            JsValue::text(replacement),
        );
        assert_parity(
            &selection,
            canonical_json(&changed(&exact, "principal", principal)).as_bytes(),
            &record,
            Some(&observed),
            false,
        );
    }
}

#[test]
fn changed_request_authority_result_evidence_and_content_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    let selection = init_native_root(&temp.path().join("root")).unwrap();
    let observed = "b".repeat(64);
    let original = record(&selection, &observed);
    let bytes = canonical_json(&descriptor(&original, &selection, Some(&observed)));
    let args = field(&original, "args").unwrap();
    let payload = field(args, "payload").unwrap();
    let changed_payloads = [
        changed(payload, "result", JsValue::Null),
        changed(
            payload,
            "evidenceAdmissions",
            JsValue::Array(vec![JsValue::text("changed")]),
        ),
        changed(
            payload,
            "unresolvedQuestions",
            JsValue::Array(vec![JsValue::text("changed")]),
        ),
    ];
    for payload in changed_payloads {
        let altered = changed(&original, "args", changed(args, "payload", payload));
        assert_parity(
            &selection,
            bytes.as_bytes(),
            &altered,
            Some(&observed),
            false,
        );
    }
    let authority = changed(
        field(args, "authority").unwrap(),
        "grantId",
        JsValue::text("other-grant"),
    );
    let altered = changed(&original, "args", changed(args, "authority", authority));
    assert_parity(
        &selection,
        bytes.as_bytes(),
        &altered,
        Some(&observed),
        false,
    );
    let altered = changed(&original, "requestId", JsValue::text("other-request"));
    assert_parity(
        &selection,
        bytes.as_bytes(),
        &altered,
        Some(&observed),
        false,
    );
}

#[test]
fn both_transports_refuse_malformed_noncanonical_and_oversized_descriptors() {
    let temp = tempfile::tempdir().unwrap();
    let selection = init_native_root(&temp.path().join("root")).unwrap();
    let request = begin(&selection);
    let valid = descriptor(&request, &selection, None);
    let source = canonical_json(&valid);
    let invalid = [
        vec![0xff],
        b"{".to_vec(),
        b"{}".to_vec(),
        format!("{source}\n").into_bytes(),
        vec![b' '; 64 * 1024 + 1],
        canonical_json(&changed(&valid, "unexpected", JsValue::Null)).into_bytes(),
        canonical_json(&changed(&valid, "schemaVersion", JsValue::Number(2.0))).into_bytes(),
        canonical_json(&changed(&valid, "profile", JsValue::text("offline"))).into_bytes(),
    ];
    for bytes in invalid {
        assert_parity(&selection, &bytes, &request, None, false);
    }
}

#[test]
fn inherited_transport_still_requires_private_readonly_unlinked_file() {
    let temp = tempfile::tempdir().unwrap();
    let selection = init_native_root(&temp.path().join("root")).unwrap();
    let request = begin(&selection);
    let bytes = canonical_json(&descriptor(&request, &selection, None));
    assert_eq!(
        check(direct(&selection, bytes.as_bytes()), &request, None),
        "admitted"
    );
    for provenance in ["writable", "linked", "public"] {
        let failure = inherited(&selection, bytes.as_bytes(), &request, None, provenance);
        assert!(
            failure.contains("not private read-only unlinked file"),
            "{provenance}: {failure}"
        );
    }
}

#[test]
fn actual_native_selection_is_checked_even_when_descriptor_matches_supplied_selection() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let selection = init_native_root(&root).unwrap();
    for fake in [
        NativeRootSelection {
            executable_sha256: "f".repeat(64),
            ..selection.clone()
        },
        NativeRootSelection {
            selection_digest: "e".repeat(64),
            ..selection.clone()
        },
    ] {
        let request = begin(&fake);
        let bytes = canonical_json(&descriptor(&request, &fake, None));
        assert!(direct(&fake, bytes.as_bytes()).is_err());
    }
    let moved = temp.path().join("moved-root");
    fs::rename(&root, &moved).unwrap();
    let fake = NativeRootSelection {
        root: moved.to_str().unwrap().into(),
        ..selection
    };
    let request = begin(&fake);
    let bytes = canonical_json(&descriptor(&request, &fake, None));
    let error = direct(&fake, bytes.as_bytes()).err().unwrap();
    assert!(
        error.to_string().contains("native selection root differs"),
        "{error}"
    );
}

#[test]
fn root_initialized_by_another_executable_cannot_issue_direct_admission() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("other-executable-root");
    let output = Command::new(BIN)
        .args(["init-native", "--root"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let marker =
        parse_json(&fs::read_to_string(root.join(".review-episode-native-host-v1")).unwrap())
            .unwrap();
    let text = |name| {
        field(&marker, name)
            .unwrap()
            .as_text()
            .unwrap()
            .to_string_checked()
            .unwrap()
    };
    let selection = NativeRootSelection {
        root: text("root"),
        executable_sha256: text("executableSha256"),
        selection_digest: text("selectionDigest"),
    };
    let request = begin(&selection);
    let bytes = canonical_json(&descriptor(&request, &selection, None));
    let error = direct(&selection, bytes.as_bytes()).err().unwrap();
    assert!(
        error
            .to_string()
            .contains("native selection executableSha256 differs"),
        "{error}"
    );
}

#[test]
fn direct_writes_preserve_observed_revision_cas_and_checked_read_only_scope() {
    let temp = tempfile::tempdir().unwrap();
    let selection = init_native_root(&temp.path().join("root")).unwrap();
    let begin = begin(&selection);
    let begun = execute(&selection, &begin, None).unwrap();
    assert_eq!(status(&begun), "applied");
    let first = revision(&begun);
    let record = record(&selection, &first);
    assert_eq!(
        execute(&selection, &record, None).unwrap_err().kind(),
        "Admission"
    );
    let recorded = execute(&selection, &record, Some(&first)).unwrap();
    assert_eq!(status(&recorded), "applied");
    let second = revision(&recorded);
    assert_eq!(
        execute(&selection, &record, Some(&first))
            .unwrap_err()
            .kind(),
        "Admission"
    );
    assert_eq!(
        status(&execute(&selection, &record, Some(&second)).unwrap()),
        "replay"
    );

    let stale = changed(
        field(&record, "args").unwrap(),
        "transitionId",
        JsValue::text("result-2"),
    );
    let stale = changed(&record, "args", stale);
    assert_eq!(
        execute(&selection, &stale, Some(&second))
            .unwrap_err()
            .kind(),
        "RevisionConflict"
    );

    let authority = selected_authority();
    let scope = TrustedEpisodeReadScope::new(
        authority.clone(),
        vec![EpisodeReadTarget::Revision(Revision(second.clone()))],
    )
    .unwrap();
    let mut reader = Application::open_read_only(
        Path::new(&selection.root),
        selection.clone(),
        scope,
        StoreOptions::default(),
        DEFAULT_RESPONSE_LIMIT,
    )
    .unwrap();
    let checked = reader
        .read_checked(&EpisodeReadRequest::Revision {
            identity: authority.identity().clone(),
            revision: Revision(second.clone()),
        })
        .unwrap();
    assert_eq!(checked.resolved_revision().unwrap().0, second);
    assert!(checked.result_digest().is_some());
    assert_eq!(
        reader.execute(&parsed(&record)).unwrap_err().kind(),
        "Admission"
    );
    let (mut store, _) =
        EpisodeStore::open_native(Path::new(&selection.root), StoreOptions::default()).unwrap();
    let snapshot = store.read(&authority.identity().key().0).unwrap();
    assert_eq!(snapshot.history.len(), 2);
    assert_eq!(snapshot.current.unwrap().revision().0, second);
}

#[test]
fn direct_constructor_rechecks_marker_integrity_before_issuing_admission() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let selection = init_native_root(&root).unwrap();
    let request = begin(&selection);
    let bytes = canonical_json(&descriptor(&request, &selection, None));
    let marker_path = root.join(".review-episode-native-host-v1");
    let original = fs::read(&marker_path).unwrap();
    let marker = parse_json(std::str::from_utf8(&original).unwrap()).unwrap();
    let tampered = changed(&marker, "selectionDigest", JsValue::text(&"f".repeat(64)));
    fs::write(&marker_path, canonical_json(&tampered)).unwrap();
    let error = direct(&selection, bytes.as_bytes()).err().unwrap();
    assert!(
        error
            .to_string()
            .contains("native selection digest differs"),
        "{error}"
    );
    fs::write(&marker_path, &original).unwrap();
    assert!(direct(&selection, bytes.as_bytes()).is_ok());
    fs::remove_file(&marker_path).unwrap();
    assert!(direct(&selection, bytes.as_bytes()).is_err());
}
