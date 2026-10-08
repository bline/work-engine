#![cfg(unix)]
#[allow(dead_code)]
mod support;

use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use review_episode::Application;
use review_episode::host_admission::NativeHostAdmission;
use review_episode::in_process_admission::{
    EpisodeReadRequest, EpisodeReadTarget, TrustedEpisodeReadScope,
};
use review_episode::operation::Operation;
use review_episode::protocol::{DEFAULT_RESPONSE_LIMIT, DirectRequest, Request, RequestSelection};
use review_episode_core::codec::{JsString, JsValue, canonical_json, digest, field, parse_json};
use review_episode_core::identity::{Authority, Revision};
use review_episode_store::{
    EpisodeStore, NativeRootSelection, StoreOptions, WriteDisposition, init_native_root,
    read_native_selection,
};
use rusqlite::Connection;
use support::{ACCEPTABLE, authority};

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

#[test]
fn sc2_child_entry() {
    let Ok(mode) = std::env::var("SC2_CHILD_MODE") else {
        return;
    };
    let root = std::path::PathBuf::from(std::env::var("SC2_ROOT").unwrap());
    if mode == "init" {
        init_native_root(&root).unwrap();
        return;
    }
    let body = fs::read(std::env::var("SC2_REQUEST").unwrap()).unwrap();
    let request = Request::parse(&body).unwrap();
    let (store, selection) = EpisodeStore::open_native(&root, StoreOptions::default()).unwrap();
    let admission = NativeHostAdmission::from_inherited_fd(selection).unwrap();
    let mut app = Application::new(store, admission, DEFAULT_RESPONSE_LIMIT);
    let reply = app.execute(&request).unwrap();
    fs::write(std::env::var("SC2_REPLY").unwrap(), reply).unwrap();
    if let Ok(release) = std::env::var("SC2_HOLD_RELEASE") {
        let deadline = Instant::now() + Duration::from_secs(30);
        while !Path::new(&release).exists() {
            assert!(Instant::now() < deadline, "held native writer timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

fn init(root: &Path) -> NativeRootSelection {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "sc2_child_entry", "--nocapture"])
        .env("SC2_CHILD_MODE", "init")
        .env("SC2_ROOT", root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    read_native_selection(root).unwrap()
}

fn write_native(
    root: &Path,
    selection: &NativeRootSelection,
    request: &JsValue,
    observed: Option<&str>,
) -> JsValue {
    let (child, reply_path) = spawn_native_write(root, selection, request, observed, None);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    parse_json(&fs::read_to_string(reply_path).unwrap()).unwrap()
}

fn spawn_native_write(
    root: &Path,
    selection: &NativeRootSelection,
    request: &JsValue,
    observed: Option<&str>,
    release: Option<&Path>,
) -> (std::process::Child, std::path::PathBuf) {
    let parent = root.parent().unwrap();
    let descriptor_path = parent.join("descriptor");
    fs::write(
        &descriptor_path,
        canonical_json(&descriptor(request, selection, observed)),
    )
    .unwrap();
    fs::set_permissions(&descriptor_path, fs::Permissions::from_mode(0o600)).unwrap();
    let descriptor_file = OpenOptions::new()
        .read(true)
        .open(&descriptor_path)
        .unwrap();
    fs::remove_file(&descriptor_path).unwrap();
    let request_path = parent.join("request.json");
    let reply_path = parent.join("reply.json");
    if reply_path.exists() {
        fs::remove_file(&reply_path).unwrap();
    }
    fs::write(&request_path, canonical_json(request)).unwrap();
    let fd = descriptor_file.as_raw_fd();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "sc2_child_entry", "--nocapture"])
        .env("SC2_CHILD_MODE", "write")
        .env("SC2_ROOT", root)
        .env("SC2_REQUEST", &request_path)
        .env("SC2_REPLY", &reply_path)
        .stdin(Stdio::null());
    if let Some(release) = release {
        command.env("SC2_HOLD_RELEASE", release);
    }
    unsafe {
        command.pre_exec(move || {
            unsafe extern "C" {
                fn dup2(oldfd: i32, newfd: i32) -> i32;
                fn fcntl(fd: i32, cmd: i32, arg: i32) -> i32;
            }
            if dup2(fd, 3) < 0 || (fd == 3 && fcntl(3, 2, 0) < 0) {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.stdout(Stdio::null()).stderr(Stdio::piped());
    (command.spawn().unwrap(), reply_path)
}

fn revision(reply: &JsValue) -> Revision {
    Revision::parse(field(reply, "observedRevision").unwrap(), "reply revision").unwrap()
}

#[test]
fn checked_historical_readback_and_refusals() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("native-root");
    let selection = init(&root);
    let authority = selected_authority();
    let identity = authority.identity().clone();
    let begin_content = JsValue::object([
        ("action", JsValue::text("begin")),
        ("unresolvedQuestions", JsValue::Array(vec![])),
    ]);
    let begin = request(
        "begin",
        "begin",
        &selection,
        JsValue::object([
            ("authority", authority.value().clone()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    let first = revision(&write_native(&root, &selection, &begin, None));
    let result = parse_json(ACCEPTABLE).unwrap();
    let evidence_admission = JsValue::object([
        (
            "claimRevisionRef",
            support::reference("claim-evidence", "claim-1", "r1", "claim-1"),
        ),
        (
            "establishmentRef",
            support::reference("claim-evidence", "establishment-1", "r1", "establishment-1"),
        ),
        (
            "observationRef",
            support::reference("claim-evidence", "observation-1", "r1", "observation-1"),
        ),
        (
            "consumptionRef",
            support::reference("slice-campaign", "consumption-1", "r1", "consumption-1"),
        ),
        ("status", JsValue::text("established")),
        ("boundary", JsValue::text("initial-review")),
        ("consumer", JsValue::text("native-review-host")),
    ]);
    let result_payload = JsValue::object([
        ("result", result.clone()),
        ("unresolvedQuestions", JsValue::Array(vec![])),
        (
            "evidenceAdmissions",
            JsValue::Array(vec![evidence_admission.clone()]),
        ),
    ]);
    let result_content = JsValue::object([
        ("action", JsValue::text("record_result")),
        ("payload", result_payload.clone()),
    ]);
    let record = request(
        "transition",
        "result",
        &selection,
        JsValue::object([
            ("authority", authority.value().clone()),
            ("expectedRevision", first.value()),
            ("transitionId", JsValue::text("result")),
            ("action", JsValue::text("record_result")),
            ("payload", result_payload),
        ]),
    );
    let target = EpisodeReadTarget::Transition {
        transition_id: JsString::new("result"),
        content_digest: Revision(digest(&result_content)),
    };
    let scoped_result = EpisodeReadRequest::Transition {
        identity: identity.clone(),
        transition_id: JsString::new("result"),
        content_digest: Revision(digest(&result_content)),
    };
    let db = root.join("review-episodes.sqlite");
    let keeper = Connection::open(&db).unwrap();
    keeper
        .execute_batch("PRAGMA wal_autocheckpoint=0; BEGIN")
        .unwrap();
    let _: i64 = keeper
        .query_row("SELECT COUNT(*) FROM review_episode_current", [], |row| {
            row.get(0)
        })
        .unwrap();
    let main_before = fs::read(&db).unwrap();
    let scope = || TrustedEpisodeReadScope::new(authority.clone(), vec![target.clone()]).unwrap();
    let mut opened_before_commit = Application::open_read_only(
        &root,
        selection.clone(),
        scope(),
        StoreOptions::default(),
        DEFAULT_RESPONSE_LIMIT,
    )
    .unwrap();
    let absent_before = opened_before_commit.read_checked(&scoped_result).unwrap();
    assert!(absent_before.resolved_revision().is_none());
    assert_eq!(absent_before.observed_revision(), Some(&first));

    let release = temp.path().join("release-native-result");
    let (mut native_writer, reply_path) =
        spawn_native_write(&root, &selection, &record, Some(&first.0), Some(&release));
    let deadline = Instant::now() + Duration::from_secs(15);
    while !reply_path.exists() {
        assert!(
            native_writer.try_wait().unwrap().is_none(),
            "native writer exited before reply"
        );
        assert!(
            Instant::now() < deadline,
            "native writer did not publish reply"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let second = revision(&parse_json(&fs::read_to_string(&reply_path).unwrap()).unwrap());
    let wal = root.join("review-episodes.sqlite-wal");
    assert!(
        fs::metadata(&wal).unwrap().len() > 32,
        "committed WAL frames are absent"
    );
    assert_eq!(
        fs::read(&db).unwrap(),
        main_before,
        "main database was checkpointed"
    );
    assert_eq!(
        opened_before_commit
            .read_checked(&scoped_result)
            .unwrap()
            .resolved_revision(),
        Some(&second)
    );
    let mut reopened = Application::open_read_only(
        &root,
        selection.clone(),
        scope(),
        StoreOptions::default(),
        DEFAULT_RESPONSE_LIMIT,
    )
    .unwrap();
    assert_eq!(
        reopened
            .read_checked(&scoped_result)
            .unwrap()
            .resolved_revision(),
        Some(&second)
    );
    drop(reopened);
    drop(opened_before_commit);
    fs::write(&release, b"release").unwrap();
    assert!(native_writer.wait_with_output().unwrap().status.success());
    drop(keeper);
    let uncertain_payload = JsValue::object([
        ("reason", JsValue::text("awaiting exact reconciliation")),
        ("reconciliationAction", JsValue::text("inspect evidence")),
    ]);
    let uncertain = request(
        "transition",
        "uncertain",
        &selection,
        JsValue::object([
            ("authority", authority.value().clone()),
            ("expectedRevision", second.value()),
            ("transitionId", JsValue::text("uncertain")),
            ("action", JsValue::text("mark_uncertain")),
            ("payload", uncertain_payload),
        ]),
    );
    let third = revision(&write_native(
        &root,
        &selection,
        &uncertain,
        Some(&second.0),
    ));

    let scope = TrustedEpisodeReadScope::new(
        authority.clone(),
        vec![
            EpisodeReadTarget::Revision(first.clone()),
            target.clone(),
            EpisodeReadTarget::Transition {
                transition_id: JsString::new("result"),
                content_digest: Revision("d".repeat(64)),
            },
            EpisodeReadTarget::Transition {
                transition_id: JsString::new("begin"),
                content_digest: Revision(digest(&begin_content)),
            },
            EpisodeReadTarget::Revision(Revision("f".repeat(64))),
            EpisodeReadTarget::Transition {
                transition_id: JsString::new("missing"),
                content_digest: Revision("e".repeat(64)),
            },
        ],
    )
    .unwrap();
    let mut app = Application::open_read_only(
        &root,
        selection.clone(),
        scope,
        StoreOptions::default(),
        DEFAULT_RESPONSE_LIMIT,
    )
    .unwrap();
    let checked = app
        .read_checked(&EpisodeReadRequest::Transition {
            identity: identity.clone(),
            transition_id: JsString::new("result"),
            content_digest: Revision(digest(&result_content)),
        })
        .unwrap();
    assert_eq!(checked.resolved_revision(), Some(&second));
    assert_eq!(checked.observed_revision(), Some(&third));
    assert_eq!(checked.state().unwrap().revision(), &second);
    assert_eq!(checked.result_digest(), Some(&Revision(digest(&result))));
    assert_eq!(checked.state().unwrap().schema_version(), 2);
    assert_eq!(
        checked.state().unwrap().get("evidenceAdmissions"),
        &JsValue::Array(vec![evidence_admission.clone()])
    );
    assert_eq!(
        checked.authority_digest(),
        &Revision(digest(authority.value()))
    );
    assert!(
        app.read_checked(&EpisodeReadRequest::Transition {
            identity: identity.clone(),
            transition_id: JsString::new("result"),
            content_digest: Revision("d".repeat(64)),
        })
        .is_err()
    );
    let first_read = app
        .read_checked(&EpisodeReadRequest::Revision {
            identity: identity.clone(),
            revision: first.clone(),
        })
        .unwrap();
    assert!(first_read.result_digest().is_none());
    assert_eq!(first_read.state().unwrap().revision(), &first);
    for absent in [
        EpisodeReadRequest::Revision {
            identity: identity.clone(),
            revision: Revision("f".repeat(64)),
        },
        EpisodeReadRequest::Transition {
            identity: identity.clone(),
            transition_id: JsString::new("missing"),
            content_digest: Revision("e".repeat(64)),
        },
    ] {
        let read = app.read_checked(&absent).unwrap();
        assert!(read.resolved_revision().is_none());
        assert!(read.state().is_none());
        assert_eq!(read.observed_revision(), Some(&third));
    }
    assert!(
        app.read_checked(&EpisodeReadRequest::Revision {
            identity: identity.clone(),
            revision: third.clone()
        })
        .is_err()
    );
    let mut wrong_identity = identity.clone();
    wrong_identity.review_episode_id = JsString::new("other");
    assert!(
        app.read_checked(&EpisodeReadRequest::Revision {
            identity: wrong_identity,
            revision: first.clone()
        })
        .is_err()
    );
    let direct = DirectRequest::new(
        RequestSelection::Native {
            request_id: "direct".into(),
            grant_id: "grant-1".into(),
            selection_revision: Revision("a".repeat(64)),
            root_selection_digest: Revision(selection.selection_digest.clone()),
        },
        Operation::Read {
            identity: identity.clone(),
            revision: Some(first.clone()),
        },
    )
    .unwrap();
    assert!(app.execute_direct(&direct).is_err());
    let wrong_source = changed(
        authority.value(),
        "source",
        changed(
            field(authority.value(), "source").unwrap(),
            "revision",
            JsValue::text("other-plan"),
        ),
    );
    let wrong_scope = TrustedEpisodeReadScope::new(
        Authority::parse(wrong_source).unwrap(),
        vec![EpisodeReadTarget::Revision(first.clone())],
    )
    .unwrap();
    let mut wrong_app = Application::open_read_only(
        &root,
        selection.clone(),
        wrong_scope,
        StoreOptions::default(),
        DEFAULT_RESPONSE_LIMIT,
    )
    .unwrap();
    assert!(
        wrong_app
            .read_checked(&EpisodeReadRequest::Revision {
                identity: identity.clone(),
                revision: first.clone()
            })
            .is_err()
    );
    let wrong_writer = changed(
        authority.value(),
        "writer",
        changed(
            field(authority.value(), "writer").unwrap(),
            "provider",
            JsValue::text("different-provider"),
        ),
    );
    let wrong_scope = TrustedEpisodeReadScope::new(
        Authority::parse(wrong_writer).unwrap(),
        vec![EpisodeReadTarget::Revision(first.clone())],
    )
    .unwrap();
    let mut wrong_app = Application::open_read_only(
        &root,
        selection.clone(),
        wrong_scope,
        StoreOptions::default(),
        DEFAULT_RESPONSE_LIMIT,
    )
    .unwrap();
    assert!(
        wrong_app
            .read_checked(&EpisodeReadRequest::Revision {
                identity: identity.clone(),
                revision: first.clone()
            })
            .is_err()
    );
    let read_store = EpisodeStore::open_native_read_only(&root, StoreOptions::default())
        .unwrap()
        .0;
    let mut read_store = read_store;
    assert!(
        read_store
            .write(
                &identity.key().0,
                Some(&third.0),
                DEFAULT_RESPONSE_LIMIT,
                |_| Ok(WriteDisposition::Replay(Box::new(
                    checked.state().unwrap().clone()
                ))),
                |_, _| String::new()
            )
            .is_err()
    );
    assert!(read_store.checkpoint().is_err());
    assert_eq!(
        read_store
            .read(&identity.key().0)
            .unwrap()
            .current
            .unwrap()
            .revision(),
        &third
    );

    // Alter only an evidence admission in a retained history row. The row is
    // still valid JSON and has the same identity, but its revision digest no
    // longer authenticates the admission bytes.
    let conn = Connection::open(&db).unwrap();
    let original: String = conn
        .query_row(
            "SELECT state_json FROM review_episode_history WHERE revision=?1",
            [&second.0],
            |row| row.get(0),
        )
        .unwrap();
    let state = parse_json(&original).unwrap();
    let altered_admission = changed(&evidence_admission, "status", JsValue::text("false"));
    let altered = changed(
        &state,
        "evidenceAdmissions",
        JsValue::Array(vec![altered_admission]),
    );
    conn.execute(
        "UPDATE review_episode_history SET state_json=?1 WHERE revision=?2",
        [canonical_json(&altered), second.0.clone()],
    )
    .unwrap();
    assert!(app.read_checked(&scoped_result).is_err());
}

#[test]
fn selected_root_scope_and_schema_refuse_without_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("native-root");
    let selection = init(&root);
    let authority = selected_authority();
    let target = EpisodeReadTarget::Revision(Revision("a".repeat(64)));
    assert!(TrustedEpisodeReadScope::new(authority.clone(), vec![]).is_err());
    assert!(
        TrustedEpisodeReadScope::new(authority.clone(), vec![target.clone(), target.clone()])
            .is_err()
    );
    let no_reader = Authority::parse(changed(
        authority.value(),
        "readers",
        JsValue::Array(vec![JsValue::text("builder")]),
    ))
    .unwrap();
    assert!(TrustedEpisodeReadScope::new(no_reader, vec![target.clone()]).is_err());
    let generation_two = Authority::parse(changed(
        &changed(
            authority.value(),
            "writer",
            changed(
                field(authority.value(), "writer").unwrap(),
                "generation",
                JsValue::Number(2.0),
            ),
        ),
        "predecessorRevision",
        JsValue::text(&"b".repeat(64)),
    ))
    .unwrap();
    assert!(TrustedEpisodeReadScope::new(generation_two, vec![target.clone()]).is_err());
    let scope = || TrustedEpisodeReadScope::new(authority.clone(), vec![target.clone()]).unwrap();
    let mut bad = selection.clone();
    bad.executable_sha256 = "f".repeat(64);
    assert!(
        Application::open_read_only(
            &root,
            bad,
            scope(),
            StoreOptions::default(),
            DEFAULT_RESPONSE_LIMIT
        )
        .is_err()
    );
    let other = temp.path().join("other-root");
    init(&other);
    assert!(
        Application::open_read_only(
            &other,
            selection.clone(),
            scope(),
            StoreOptions::default(),
            DEFAULT_RESPONSE_LIMIT
        )
        .is_err()
    );
    let marker = root.join(".review-episode-native-host-v1");
    let marker_before = fs::read(&marker).unwrap();
    let mut marker_json = parse_json(std::str::from_utf8(&marker_before).unwrap()).unwrap();
    marker_json = changed(
        &marker_json,
        "executableSha256",
        JsValue::text(&"f".repeat(64)),
    );
    fs::write(&marker, canonical_json(&marker_json)).unwrap();
    assert!(
        Application::open_read_only(
            &root,
            selection.clone(),
            scope(),
            StoreOptions::default(),
            DEFAULT_RESPONSE_LIMIT
        )
        .is_err()
    );
    fs::write(&marker, &marker_before).unwrap();
    let sidecar = root.join("review-episodes.sqlite-wal");
    if sidecar.exists() {
        fs::remove_file(&sidecar).unwrap();
    }
    std::os::unix::fs::symlink(root.join("review-episodes.sqlite"), &sidecar).unwrap();
    assert!(
        Application::open_read_only(
            &root,
            selection.clone(),
            scope(),
            StoreOptions::default(),
            DEFAULT_RESPONSE_LIMIT
        )
        .is_err()
    );
    fs::remove_file(&sidecar).unwrap();
    let db = root.join("review-episodes.sqlite");
    let before = fs::read(&db).unwrap();
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch("CREATE TABLE extra (id INTEGER)")
        .unwrap();
    drop(conn);
    let changed = fs::read(&db).unwrap();
    assert_ne!(before, changed);
    assert!(
        Application::open_read_only(
            &root,
            selection,
            scope(),
            StoreOptions::default(),
            DEFAULT_RESPONSE_LIMIT
        )
        .is_err()
    );
    assert_eq!(fs::read(&db).unwrap(), changed);
}

#[test]
fn opened_reader_refuses_later_marker_schema_and_history_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("native-root");
    let selection = init(&root);
    let authority = selected_authority();
    let identity = authority.identity().clone();
    let begin = request(
        "begin",
        "begin",
        &selection,
        JsValue::object([
            ("authority", authority.value().clone()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    let first = revision(&write_native(&root, &selection, &begin, None));
    let scope = || {
        TrustedEpisodeReadScope::new(
            authority.clone(),
            vec![EpisodeReadTarget::Revision(first.clone())],
        )
        .unwrap()
    };
    let read = || EpisodeReadRequest::Revision {
        identity: identity.clone(),
        revision: first.clone(),
    };
    let mut app = Application::open_read_only(
        &root,
        selection.clone(),
        scope(),
        StoreOptions::default(),
        DEFAULT_RESPONSE_LIMIT,
    )
    .unwrap();
    assert_eq!(
        app.read_checked(&read()).unwrap().resolved_revision(),
        Some(&first)
    );

    let marker = root.join(".review-episode-native-host-v1");
    let original_marker = fs::read(&marker).unwrap();
    fs::write(&marker, b"invalid marker").unwrap();
    assert!(app.read_checked(&read()).is_err());
    fs::write(&marker, original_marker).unwrap();

    let db = root.join("review-episodes.sqlite");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch("CREATE TABLE later_unapproved (id INTEGER)")
        .unwrap();
    assert!(app.read_checked(&read()).is_err());
    conn.execute_batch("DROP TABLE later_unapproved").unwrap();
    assert_eq!(
        app.read_checked(&read()).unwrap().resolved_revision(),
        Some(&first)
    );
    conn.execute(
        "UPDATE review_episode_history SET state_json='{}' WHERE identity_key=?1",
        [&identity.key().0],
    )
    .unwrap();
    assert!(app.read_checked(&read()).is_err());
}
