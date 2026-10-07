#![cfg(feature = "test-faults")]
#[allow(dead_code)]
mod support;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use review_episode_core::codec::{JsValue, canonical_json, digest, field};
use review_episode_core::identity::Authority;
use review_episode_store::{EpisodeStore, StoreOptions};
use support::*;

#[cfg(feature = "test-faults")]
fn kill_at(root: &Path, registry: &Path, request: &JsValue, cut: &str, barrier: &Path) -> Vec<u8> {
    let mut child = Command::new(BIN)
        .arg("offline")
        .arg("--root")
        .arg(root)
        .arg("--fixture")
        .arg(registry)
        .env("REVIEW_EPISODE_FAULT_CUT", cut)
        .env("REVIEW_EPISODE_FAULT_DIR", barrier)
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
    let ready = barrier.join(format!("{cut}.ready"));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("child exited before {cut}: {status}");
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("child did not reach {cut}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    child.kill().unwrap();
    let out = child.wait_with_output().unwrap();
    out.stdout
}

#[cfg(feature = "test-faults")]
#[test]
fn named_process_kill_cuts_reconcile_by_fresh_read_and_retry() {
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
        let root = temp.path().join("offline");
        let registry = temp.path().join("registry.json");
        let barrier = temp.path().join("barrier");
        fs::create_dir(&barrier).unwrap();
        init(&root);
        let authority = authority();
        let identity = field(&authority, "identity").unwrap().clone();
        let begin_request = request(
            "begin",
            "begin-grant",
            "begin-cut",
            JsValue::object([
                ("authority", authority.clone()),
                ("transitionId", JsValue::text("begin")),
                ("unresolvedQuestions", JsValue::Array(vec![])),
            ]),
        );
        write_fixture(&registry, &root, &begin_request, None);
        let bytes = kill_at(&root, &registry, &begin_request, cut, &barrier);
        if cut == "after_partial_reply" {
            assert!(bytes.len() > 4, "partial reply cut emitted no bytes");
            let claimed = u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize;
            assert!(
                bytes.len() < claimed + 4,
                "partial reply unexpectedly complete"
            );
        }
        let key = Authority::parse(authority).unwrap().identity().key().0;
        let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
        let snapshot = store.read(&key).unwrap();
        let committed = matches!(cut, "after_commit" | "before_reply" | "after_partial_reply");
        assert_eq!(snapshot.current.is_some(), committed, "cut {cut}");
        assert_eq!(snapshot.history.len(), usize::from(committed), "cut {cut}");
        let observed = snapshot.current.as_ref().map(|s| s.revision().0.clone());
        drop(store);
        let content = digest(&JsValue::object([
            ("action", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]));
        let recovery = request(
            "recover",
            "read-grant",
            "recover-cut",
            JsValue::object([
                ("identity", identity),
                ("transitionId", JsValue::text("begin")),
                ("contentDigest", JsValue::text(&content)),
            ]),
        );
        write_fixture(&registry, &root, &recovery, observed.as_deref());
        let recovered = invoke(&root, &registry, &recovery);
        assert_eq!(
            status(&recovered),
            if committed { "committed" } else { "absent" },
            "cut {cut}"
        );
        write_fixture(&registry, &root, &begin_request, observed.as_deref());
        let retry = invoke(&root, &registry, &begin_request);
        assert_eq!(
            status(&retry),
            if committed { "replay" } else { "applied" },
            "cut {cut}"
        );
        let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
        assert_eq!(store.read(&key).unwrap().history.len(), 1, "cut {cut}");
    }
}

fn spawn_paused(
    root: &Path,
    registry: &Path,
    request: &JsValue,
    barrier: &Path,
) -> std::process::Child {
    let mut child = Command::new(BIN)
        .arg("offline")
        .arg("--root")
        .arg(root)
        .arg("--fixture")
        .arg(registry)
        .env("REVIEW_EPISODE_FAULT_CUT", "after_admission")
        .env("REVIEW_EPISODE_FAULT_DIR", barrier)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let body = canonical_json(request);
    {
        let mut input = child.stdin.take().unwrap();
        input.write_all(&(body.len() as u32).to_be_bytes()).unwrap();
        input.write_all(body.as_bytes()).unwrap();
    }
    child
}
fn wait_ready(child: &mut std::process::Child, barrier: &Path) {
    let ready = barrier.join("after_admission.ready");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("racing child exited before admission: {status}");
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("racing child did not reach admission");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn finish(child: std::process::Child) -> JsValue {
    let output = bounded_output(child);
    assert!(
        output.status.success(),
        "race child failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.len() >= 4);
    let len = u32::from_be_bytes(output.stdout[..4].try_into().unwrap()) as usize;
    assert_eq!(output.stdout.len(), len + 4);
    review_episode_core::codec::parse_json(std::str::from_utf8(&output.stdout[4..]).unwrap())
        .unwrap()
}
fn many_grants(path: &Path, root: &Path, requests: &[JsValue], observed: Option<&str>) {
    let grants = requests
        .iter()
        .map(|request| {
            JsValue::object([
                ("grantId", field(request, "grantId").unwrap().clone()),
                ("requestDigest", JsValue::text(&digest(request))),
                (
                    "selectionRevision",
                    field(request, "selectionRevision").unwrap().clone(),
                ),
                (
                    "observedRevision",
                    observed.map_or(JsValue::Null, JsValue::text),
                ),
            ])
        })
        .collect();
    let root = root.canonicalize().unwrap();
    let registry = JsValue::object([
        ("root", JsValue::text(root.to_str().unwrap())),
        ("grants", JsValue::Array(grants)),
    ]);
    fs::write(path, canonical_json(&registry)).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn separate_process_begin_cas_race_has_one_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let authority = authority();
    let key = Authority::parse(authority.clone())
        .unwrap()
        .identity()
        .key()
        .0;
    let args = JsValue::object([
        ("authority", authority),
        ("transitionId", JsValue::text("begin")),
        ("unresolvedQuestions", JsValue::Array(vec![])),
    ]);
    let first = request("begin", "grant-a", "race-a", args.clone());
    let second = request("begin", "grant-b", "race-b", args);
    many_grants(&registry, &root, &[first.clone(), second.clone()], None);
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    let mut child_a = spawn_paused(&root, &registry, &first, &a);
    let mut child_b = spawn_paused(&root, &registry, &second, &b);
    wait_ready(&mut child_a, &a);
    wait_ready(&mut child_b, &b);
    fs::write(a.join("after_admission.release"), b"go").unwrap();
    fs::write(b.join("after_admission.release"), b"go").unwrap();
    let results = [finish(child_a), finish(child_b)];
    let applied = results.iter().filter(|r| status(r) == "applied").count();
    let conflicts = results
        .iter()
        .filter(|r| field(r, "kind") == Ok(&JsValue::text("RevisionConflict")))
        .count();
    assert_eq!((applied, conflicts), (1, 1), "{results:?}");
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    assert_eq!(store.read(&key).unwrap().history.len(), 1);
}

#[test]
fn same_transition_race_conflicts_then_replays_after_fresh_admission() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let authority = authority();
    let key = Authority::parse(authority.clone())
        .unwrap()
        .identity()
        .key()
        .0;
    let begin = request(
        "begin",
        "begin-grant",
        "begin",
        JsValue::object([
            ("authority", authority.clone()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    write_fixture(&registry, &root, &begin, None);
    let begin_response = invoke(&root, &registry, &begin);
    let first_revision = revision(&begin_response);
    let args = JsValue::object([
        ("authority", authority),
        ("expectedRevision", JsValue::text(&first_revision)),
        ("transitionId", JsValue::text("uncertain")),
        ("action", JsValue::text("mark_uncertain")),
        (
            "payload",
            JsValue::object([
                ("reason", JsValue::text("lost")),
                ("reconciliationAction", JsValue::text("reconcile")),
            ]),
        ),
    ]);
    let first = request("transition", "grant-a", "race-a", args.clone());
    let second = request("transition", "grant-b", "race-b", args);
    many_grants(
        &registry,
        &root,
        &[first.clone(), second.clone()],
        Some(&first_revision),
    );
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    let mut child_a = spawn_paused(&root, &registry, &first, &a);
    let mut child_b = spawn_paused(&root, &registry, &second, &b);
    wait_ready(&mut child_a, &a);
    wait_ready(&mut child_b, &b);
    fs::write(a.join("after_admission.release"), b"go").unwrap();
    fs::write(b.join("after_admission.release"), b"go").unwrap();
    let responses = [finish(child_a), finish(child_b)];
    assert_eq!(
        responses.iter().filter(|r| status(r) == "applied").count(),
        1,
        "{responses:?}"
    );
    assert_eq!(
        responses
            .iter()
            .filter(|r| field(r, "kind") == Ok(&JsValue::text("RevisionConflict")))
            .count(),
        1,
        "{responses:?}"
    );
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    let snapshot = store.read(&key).unwrap();
    assert_eq!(snapshot.history.len(), 2);
    let current = snapshot.current.unwrap().revision().0.clone();
    drop(store);
    let loser = if status(&responses[0]) == "applied" {
        second
    } else {
        first
    };
    write_fixture(&registry, &root, &loser, Some(&current));
    let replay = invoke(&root, &registry, &loser);
    assert_eq!(status(&replay), "replay");
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    assert_eq!(store.read(&key).unwrap().history.len(), 2);
}

#[test]
fn different_commands_on_one_expected_revision_have_one_successor() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let authority = authority();
    let key = Authority::parse(authority.clone())
        .unwrap()
        .identity()
        .key()
        .0;
    let begin = request(
        "begin",
        "begin-grant",
        "begin",
        JsValue::object([
            ("authority", authority.clone()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    write_fixture(&registry, &root, &begin, None);
    let first = invoke(&root, &registry, &begin);
    let first_revision = revision(&first);
    let make = |id: &str, reason: &str| {
        request(
            "transition",
            id,
            id,
            JsValue::object([
                ("authority", authority.clone()),
                ("expectedRevision", JsValue::text(&first_revision)),
                ("transitionId", JsValue::text(id)),
                ("action", JsValue::text("mark_uncertain")),
                (
                    "payload",
                    JsValue::object([
                        ("reason", JsValue::text(reason)),
                        ("reconciliationAction", JsValue::text("reconcile")),
                    ]),
                ),
            ]),
        )
    };
    let a_request = make("uncertain-a", "lost A");
    let b_request = make("uncertain-b", "lost B");
    many_grants(
        &registry,
        &root,
        &[a_request.clone(), b_request.clone()],
        Some(&first_revision),
    );
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    let mut child_a = spawn_paused(&root, &registry, &a_request, &a);
    let mut child_b = spawn_paused(&root, &registry, &b_request, &b);
    wait_ready(&mut child_a, &a);
    wait_ready(&mut child_b, &b);
    fs::write(a.join("after_admission.release"), b"go").unwrap();
    fs::write(b.join("after_admission.release"), b"go").unwrap();
    let responses = [finish(child_a), finish(child_b)];
    assert_eq!(
        responses.iter().filter(|r| status(r) == "applied").count(),
        1,
        "{responses:?}"
    );
    assert_eq!(
        responses
            .iter()
            .filter(|r| field(r, "kind") == Ok(&JsValue::text("RevisionConflict")))
            .count(),
        1,
        "{responses:?}"
    );
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    let snapshot = store.read(&key).unwrap();
    assert_eq!(snapshot.history.len(), 2);
}

#[test]
fn replacement_winner_fences_an_already_admitted_old_writer() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let old = authority();
    let identity = field(&old, "identity").unwrap().clone();
    let key = Authority::parse(old.clone()).unwrap().identity().key().0;
    let begin = request(
        "begin",
        "begin-grant",
        "begin",
        JsValue::object([
            ("authority", old.clone()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    write_fixture(&registry, &root, &begin, None);
    let first = invoke(&root, &registry, &begin);
    let first_revision = revision(&first);
    let uncertain = request(
        "transition",
        "old-uncertain",
        "uncertain",
        JsValue::object([
            ("authority", old.clone()),
            ("expectedRevision", JsValue::text(&first_revision)),
            ("transitionId", JsValue::text("uncertain")),
            ("action", JsValue::text("mark_uncertain")),
            (
                "payload",
                JsValue::object([
                    ("reason", JsValue::text("lost")),
                    ("reconciliationAction", JsValue::text("replace")),
                ]),
            ),
        ]),
    );
    write_fixture(&registry, &root, &uncertain, Some(&first_revision));
    let uncertain_state = invoke(&root, &registry, &uncertain);
    let uncertain_revision = revision(&uncertain_state);
    let mut successor = old.clone();
    if let JsValue::Object(map) = &mut successor {
        map.insert("grantId".into(), JsValue::text("grant-2"));
        map.insert(
            "predecessorRevision".into(),
            JsValue::text(&uncertain_revision),
        );
        if let JsValue::Object(writer) = map.get_mut(&"writer".into()).unwrap() {
            writer.insert("generation".into(), JsValue::Number(2.0));
            writer.insert(
                "runtimeSession".into(),
                reference("runtime", "session-2", "generation-2", "session-2"),
            );
        }
    }
    let replacement = request(
        "transition",
        "successor",
        "replace",
        JsValue::object([
            ("authority", successor),
            ("expectedRevision", JsValue::text(&uncertain_revision)),
            ("transitionId", JsValue::text("replace")),
            ("action", JsValue::text("replace_writer")),
            (
                "payload",
                JsValue::object([
                    ("reason", JsValue::text("lost")),
                    ("pendingAction", JsValue::text("reconcile")),
                ]),
            ),
        ]),
    );
    let late = request(
        "transition",
        "old-late",
        "late",
        JsValue::object([
            ("authority", old),
            ("expectedRevision", JsValue::text(&uncertain_revision)),
            ("transitionId", JsValue::text("late")),
            ("action", JsValue::text("mark_uncertain")),
            (
                "payload",
                JsValue::object([
                    ("reason", JsValue::text("still lost")),
                    ("reconciliationAction", JsValue::text("reconcile")),
                ]),
            ),
        ]),
    );
    many_grants(
        &registry,
        &root,
        &[replacement.clone(), late.clone()],
        Some(&uncertain_revision),
    );
    let a = temp.path().join("replacement");
    let b = temp.path().join("old");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    let mut successor_child = spawn_paused(&root, &registry, &replacement, &a);
    let mut old_child = spawn_paused(&root, &registry, &late, &b);
    wait_ready(&mut successor_child, &a);
    wait_ready(&mut old_child, &b);
    fs::write(a.join("after_admission.release"), b"go").unwrap();
    let winner = finish(successor_child);
    assert_eq!(status(&winner), "applied", "{winner:?}");
    fs::write(b.join("after_admission.release"), b"go").unwrap();
    let loser = finish(old_child);
    assert_eq!(
        field(&loser, "kind").unwrap(),
        &JsValue::text("RevisionConflict")
    );
    let current = revision(&winner);
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    assert_eq!(store.read(&key).unwrap().history.len(), 3);
    drop(store);
    write_fixture(&registry, &root, &uncertain, Some(&current));
    let denied = invoke(&root, &registry, &uncertain);
    assert_eq!(field(&denied, "kind").unwrap(), &JsValue::text("Authority"));
    let read = request(
        "history",
        "reader",
        "history",
        JsValue::object([("identity", identity)]),
    );
    write_fixture(&registry, &root, &read, Some(&current));
    assert_eq!(
        field(&invoke(&root, &registry, &read), "history")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn injected_lost_commit_ack_closes_connection_and_needs_authorized_readback() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let authority = authority();
    let identity = field(&authority, "identity").unwrap().clone();
    let begin = request(
        "begin",
        "begin-grant",
        "begin",
        JsValue::object([
            ("authority", authority),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    write_fixture(&registry, &root, &begin, None);
    let mut child = Command::new(BIN)
        .arg("offline")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg(&registry)
        .env("REVIEW_EPISODE_FAULT_CUT", "after_commit_unknown")
        .env("REVIEW_EPISODE_FAULT_DIR", temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let body = canonical_json(&begin);
    {
        let mut input = child.stdin.take().unwrap();
        input.write_all(&(body.len() as u32).to_be_bytes()).unwrap();
        input.write_all(body.as_bytes()).unwrap();
    }
    let outcome = finish(child);
    assert_eq!(
        field(&outcome, "kind").unwrap(),
        &JsValue::text("OutcomeUnknown")
    );
    let key = review_episode_core::identity::Identity::parse(&identity)
        .unwrap()
        .key()
        .0;
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    let snapshot = store.read(&key).unwrap();
    assert_eq!(snapshot.history.len(), 1);
    let committed = snapshot.current.unwrap().revision().0.clone();
    drop(store);
    let content = digest(&JsValue::object([
        ("action", JsValue::text("begin")),
        ("unresolvedQuestions", JsValue::Array(vec![])),
    ]));
    let recover = request(
        "recover",
        "read-grant",
        "recover",
        JsValue::object([
            ("identity", identity),
            ("transitionId", JsValue::text("begin")),
            ("contentDigest", JsValue::text(&content)),
        ]),
    );
    let denied = invoke(&root, &registry, &recover);
    assert_eq!(field(&denied, "kind").unwrap(), &JsValue::text("Admission"));
    write_fixture(&registry, &root, &recover, Some(&committed));
    let resolved = invoke(&root, &registry, &recover);
    assert_eq!(status(&resolved), "committed");
}
