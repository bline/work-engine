use super::*;
use crate::{
    digest,
    pi_docker::{Host, RunEntry},
};
use std::collections::VecDeque;

const ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const OTHER: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn fixture() -> (tempfile::TempDir, Launch) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let artifact = |name: &str, bytes: &[u8]| {
        let path = root.join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o500)).unwrap();
        Artifact {
            path,
            sha256: digest(bytes),
        }
    };
    let setup = Launch {
        attempt: "a".repeat(64),
        fence_root: root.join("fences"),
        init: artifact("init", b"inert init fixture, not an executable format"),
        docker: artifact("docker", b"inert daemon CLI fixture, never spawned"),
        image_id: format!("sha256:{}", "b".repeat(64)),
    };
    fs::create_dir(&setup.fence_root).unwrap();
    (dir, setup)
}

fn inspect(setup: &Launch, running: bool) -> Value {
    json!({
        "Id": ID, "Name": format!("/we-public-hold-{}", setup.attempt),
        "Image": setup.image_id, "Path": "/work-engine-hold-init", "Args": ["hold-only"],
        "State": {"Running": running, "Status": if running {"running"} else {"created"}},
        "Config": {
            "Labels": {"work-engine.public-hold-attempt": setup.attempt,
                       "work-engine.public-hold-init-sha256": setup.init.sha256},
            "Image": IMAGE, "Entrypoint": ["/work-engine-hold-init"], "Cmd": ["hold-only"],
            "User": "1000:1000", "WorkingDir": "/", "OpenStdin": false, "StdinOnce": false,
            "AttachStdin": false, "Tty": false, "Healthcheck": {"Test":["NONE"]},
            "Env": ["PATH=/usr/bin:/bin", "NODE_VERSION=unused"],
        },
        "HostConfig": {
            "NetworkMode":"none", "IpcMode":"private", "PidMode":"", "Privileged": false,
            "ReadonlyRootfs": true, "CapDrop": ["ALL"], "SecurityOpt": ["no-new-privileges"],
            "PidsLimit": 16, "Memory": 67108864, "NanoCpus": 1000000000, "Init": false,
            "AutoRemove": false, "RestartPolicy":{"Name":"no"}, "LogConfig": {"Type":"none"},
        },
        "Mounts": [{"Source": setup.init.path, "Destination":"/work-engine-hold-init",
                      "RW": false, "Propagation":"rprivate", "Type":"bind"}]
    })
}

type Reply = Result<(bool, Vec<u8>)>;
fn snapshot(v: Value) -> Reply {
    Ok((true, serde_json::to_vec(&[v]).unwrap()))
}
fn ok(s: &str) -> Reply {
    Ok((true, s.as_bytes().to_owned()))
}

fn drive(setup: &Launch, replies: Vec<Reply>) -> (Value, Vec<Vec<String>>) {
    let mut replies = VecDeque::from(replies);
    let mut calls = vec![];
    let result = Host::run_entry_at_boundary(
        RunEntry::PublicInferenceSetup,
        None,
        Some(setup),
        |bin, args| {
            assert_eq!(bin, setup.docker.path);
            assert!(
                setup.fence_root.join(setup.name()).is_file(),
                "no daemon work before fence"
            );
            calls.push(args.to_owned());
            replies
                .pop_front()
                .expect("unexpected extra daemon operation")
        },
    )
    .unwrap();
    assert!(
        replies.is_empty(),
        "scenario unexpectedly short-circuited: {result}"
    );
    assert_eq!(result["fenced"], true);
    assert_eq!(result["disposition"], "hold_stop_only");
    assert!(result.get("qualified").is_none());
    (result, calls)
}

#[test]
fn public_hold_exact_entry_creates_inspects_starts_and_removes_only_fixed_init() {
    let (_dir, setup) = fixture();
    let (result, calls) = drive(
        &setup,
        vec![
            ok(ID),
            snapshot(inspect(&setup, false)),
            ok(ID),
            snapshot(inspect(&setup, true)),
            snapshot(inspect(&setup, true)),
            ok(ID),
            ok(""),
        ],
    );
    assert_eq!(result["hold_inspected"], true);
    assert_eq!(result["cleanup_confirmed"], true);
    let name = format!("we-public-hold-{}", setup.attempt);
    assert_eq!(
        calls,
        [
            strings(&[
                "create",
                "--pull=never",
                "--name",
                &name,
                "--label",
                &format!("work-engine.public-hold-attempt={}", setup.attempt),
                "--label",
                &format!("work-engine.public-hold-init-sha256={}", setup.init.sha256),
                "--network=none",
                "--ipc=private",
                "--read-only",
                "--cap-drop=ALL",
                "--security-opt=no-new-privileges",
                "--pids-limit=16",
                "--memory=64m",
                "--cpus=1",
                "--user=1000:1000",
                "--workdir=/",
                "--no-healthcheck",
                "--restart=no",
                "--init=false",
                "--log-driver=none",
                "--mount",
                &format!(
                    "type=bind,src={},dst=/work-engine-hold-init,readonly,bind-propagation=rprivate",
                    setup.init.path.display()
                ),
                "--entrypoint",
                "/work-engine-hold-init",
                IMAGE,
                "hold-only"
            ]),
            strings(&["container", "inspect", ID]),
            strings(&["start", ID]),
            strings(&["container", "inspect", ID]),
            strings(&["container", "inspect", ID]),
            strings(&["rm", "--force", ID]),
            strings(&[
                "ps",
                "--all",
                "--no-trunc",
                "--filter",
                &format!("name=^/{name}$"),
                "--format",
                "{{.ID}}"
            ]),
        ]
    );
}

#[test]
fn public_hold_absent_crossed_and_changed_bytes_refuse_before_any_effect() {
    let (_dir, setup) = fixture();
    for (request, selection) in [
        (None, None),
        (Some(Path::new("must-not-open")), Some(&setup)),
    ] {
        Host::run_entry_at_boundary(
            RunEntry::PublicInferenceSetup,
            request,
            selection,
            |_, _| panic!("refusal reached Docker"),
        )
        .unwrap_err();
    }
    fs::set_permissions(&setup.init.path, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&setup.init.path, b"changed").unwrap();
    let err = Host::run_entry_at_boundary(
        RunEntry::PublicInferenceSetup,
        None,
        Some(&setup),
        |_, _| panic!("changed init reached Docker"),
    )
    .unwrap_err();
    assert!(err.to_string().contains("pin mismatch"));
    assert_eq!(fs::read_dir(&setup.fence_root).unwrap().count(), 0);
}

#[test]
fn public_hold_config_drift_does_not_start_but_can_remove_exact_container() {
    for (pointer, value) in [
        ("/Path", json!("node")),
        ("/Config/OpenStdin", json!(true)),
        ("/Config/Healthcheck/Test", json!(["CMD", "node"])),
        ("/Mounts/0/RW", json!(true)),
        ("/HostConfig/Privileged", json!(true)),
        ("/Config/Env", json!(["LD_PRELOAD=unexpected.so"])),
    ] {
        let (_dir, setup) = fixture();
        let mut wrong = inspect(&setup, false);
        *wrong.pointer_mut(pointer).unwrap() = value;
        let (result, calls) = drive(
            &setup,
            vec![
                ok(ID),
                snapshot(wrong.clone()),
                snapshot(wrong),
                ok(ID),
                ok(""),
            ],
        );
        assert_eq!(result["hold_inspected"], false);
        assert_eq!(result["cleanup_confirmed"], true);
        assert_eq!(calls[3], ["rm", "--force", ID]);
        assert!(!calls.iter().any(|a| a[0] == "start"));
    }
}

#[test]
fn public_hold_crossed_inspection_cannot_start_or_remove_foreign_identity() {
    for (pointer, value) in [
        ("/Id", json!(OTHER)),
        (
            "/Config/Labels/work-engine.public-hold-attempt",
            json!("wrong"),
        ),
    ] {
        let (_dir, setup) = fixture();
        let mut foreign = inspect(&setup, false);
        *foreign.pointer_mut(pointer).unwrap() = value;
        let (result, calls) = drive(
            &setup,
            vec![ok(ID), snapshot(foreign.clone()), snapshot(foreign)],
        );
        assert_eq!(result["hold_inspected"], false);
        assert_eq!(result["cleanup_confirmed"], false);
        assert!(
            result["cleanup_error"]
                .as_str()
                .unwrap()
                .contains("identity conflict")
        );
        assert_eq!(calls.len(), 3);
    }
}

#[test]
fn public_hold_uncertain_start_has_only_cleanup_and_cannot_replay_on_reentry() {
    let (_dir, mut setup) = fixture();
    let (result, calls) = drive(
        &setup,
        vec![
            ok(ID),
            snapshot(inspect(&setup, false)),
            refuse("inert CLI timeout after start request"),
            snapshot(inspect(&setup, true)),
            Ok((false, vec![])),
        ],
    );
    assert_eq!(result["hold_inspected"], false);
    assert_eq!(result["cleanup_confirmed"], false);
    assert_eq!(calls[2], ["start", ID]);
    assert_eq!(calls[4], ["rm", "--force", ID]);
    // Reenter with changed (still pinned) recipe bytes for the same attempt.
    setup.image_id = format!("sha256:{}", "f".repeat(64));
    let err = Host::run_entry_at_boundary(
        RunEntry::PublicInferenceSetup,
        None,
        Some(&setup),
        |_, _| panic!("recovery replayed daemon work"),
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("fenced; prior cleanup not established")
    );
}

#[test]
fn public_hold_uncertain_create_never_starts_or_treats_absence_as_settled() {
    let (_dir, setup) = fixture();
    let (result, calls) = drive(
        &setup,
        vec![refuse("inert create timeout"), Ok((false, vec![]))],
    );
    assert_eq!(result["hold_inspected"], false);
    assert_eq!(result["cleanup_confirmed"], false);
    assert_eq!(calls[1], strings(&["container", "inspect", &setup.name()]));

    let (_dir, setup) = fixture();
    let (result, calls) = drive(
        &setup,
        vec![
            refuse("inert create timeout"),
            snapshot(inspect(&setup, false)),
            ok(ID),
            ok(""),
        ],
    );
    assert_eq!(result["hold_inspected"], false);
    assert_eq!(result["cleanup_confirmed"], true);
    assert_eq!(calls[2], ["rm", "--force", ID]);
}
