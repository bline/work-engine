use super::*;
use crate::{
    digest,
    pi_docker::{Host, RunEntry},
};
use std::{cell::Cell, collections::VecDeque, sync::mpsc::sync_channel};

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
        docker: artifact("docker", b"inert daemon CLI fixture, never spawned"),
        image_id: format!("sha256:{}", "b".repeat(64)),
        init_sha256: digest(b"synthetic content identity, not measured from an image"),
    };
    fs::create_dir(&setup.fence_root).unwrap();
    (dir, setup)
}

fn image(setup: &Launch) -> Value {
    json!({
        "Id": setup.image_id, "Os": "linux", "Architecture": "amd64", "Variant": "",
        "Config": {"Volumes": null, "Env": [], "Entrypoint": null, "Cmd": null,
                   "Healthcheck": null, "WorkingDir": ""}
    })
}

fn inspect(setup: &Launch, running: bool) -> Value {
    json!({
        "Id": ID, "Name": format!("/we-public-hold-{}", setup.attempt),
        "Image": setup.image_id, "Path": "/work-engine-hold-init", "Args": ["hold-only"],
        "State": {"Running": running, "Status": if running {"running"} else {"created"}},
        "Config": {
            "Labels": {"work-engine.public-hold-attempt": setup.attempt,
                       "work-engine.public-hold-init-sha256": setup.init_sha256},
            "Image": setup.image_id, "Entrypoint": ["/work-engine-hold-init"], "Cmd": ["hold-only"],
            "User": "1000:1000", "WorkingDir": "/", "OpenStdin": false, "StdinOnce": false,
            "AttachStdin": false, "Tty": false, "Healthcheck": {"Test":["NONE"]},
            "Env": null, "Volumes": null,
        },
        "HostConfig": {
            "NetworkMode":"none", "IpcMode":"private", "PidMode":"", "Privileged": false,
            "CgroupnsMode":"private", "Binds":null, "Mounts":[],
            "ReadonlyRootfs": true, "CapDrop": ["ALL"], "SecurityOpt": ["no-new-privileges"],
            "PidsLimit": 16, "Memory": 67108864, "NanoCpus": 1000000000, "Init": false,
            "AutoRemove": false, "RestartPolicy":{"Name":"no"}, "LogConfig": {"Type":"none"},
        },
        "Mounts": []
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
    drive_with(setup, replies, |invoke| {
        Host::run_entry_at_boundary(RunEntry::PublicInferenceSetup, None, Some(setup), invoke)
    })
}

fn drive_with(
    setup: &Launch,
    replies: Vec<Reply>,
    run: impl FnOnce(&mut dyn FnMut(&Path, &[String]) -> Reply) -> Result<Value>,
) -> (Value, Vec<Vec<String>>) {
    let mut replies = VecDeque::from(replies);
    let mut calls = vec![];
    let result = run(&mut |bin, args| {
        assert_eq!(bin, setup.docker.path);
        assert!(
            setup.fence_root.join(setup.name()).is_file(),
            "no daemon work before fence"
        );
        calls.push(args.to_owned());
        replies
            .pop_front()
            .expect("unexpected extra Docker operation")
    })
    .unwrap();
    assert!(
        replies.is_empty(),
        "scenario unexpectedly short-circuited: {result}"
    );
    assert_eq!(result["fenced"], true);
    assert_eq!(result["disposition"], "hold_stop_only");
    assert!(result.get("qualified").is_none());
    assert!(result.get("cleanup_confirmed").is_none());
    (result, calls)
}

fn assert_no_replay(setup: &Launch) {
    let err =
        Host::run_entry_at_boundary(RunEntry::PublicInferenceSetup, None, Some(setup), |_, _| {
            panic!("recovery replayed daemon work")
        })
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("fenced; prior cleanup not established")
    );
}

#[test]
fn public_hold_exact_image_entry_creates_inspects_starts_and_removes_only_fixed_init() {
    let (_dir, setup) = fixture();
    let (result, calls) = drive(
        &setup,
        vec![
            snapshot(image(&setup)),
            ok(ID),
            snapshot(inspect(&setup, false)),
            ok(ID),
            snapshot(inspect(&setup, true)),
            snapshot(inspect(&setup, true)),
            ok(ID),
            ok(""),
        ],
    );
    assert_eq!(result["create_attempted"], true);
    assert_eq!(result["hold_inspected"], true);
    assert_eq!(result["observation_window"], "unavailable");
    assert_eq!(result["docker_remove_cli_success"], true);
    assert_eq!(result["docker_exact_name_list_empty"], true);
    let name = format!("we-public-hold-{}", setup.attempt);
    assert_eq!(
        calls,
        [
            strings(&["image", "inspect", &setup.image_id]),
            strings(&[
                "create",
                "--pull=never",
                "--platform",
                "linux/amd64",
                "--name",
                &name,
                "--label",
                &format!("work-engine.public-hold-attempt={}", setup.attempt),
                "--label",
                &format!("work-engine.public-hold-init-sha256={}", setup.init_sha256),
                "--network=none",
                "--ipc=private",
                "--cgroupns=private",
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
                "--entrypoint",
                "/work-engine-hold-init",
                &setup.image_id,
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

#[derive(Clone, Copy)]
enum WindowCase {
    Complete,
    Failed,
    EarlyExit,
    WrongAttempt,
    WrongId,
    Closed,
    StartSpentBudget,
    LateSignal,
    SilenceAtDeadline,
    CancelBeforeAcceptingSignal,
    InspectEarlyExit,
    UncertainStart,
}

fn check_observation(case: WindowCase, expected: &str, wait_reads: usize) {
    use WindowCase::*;
    let (_dir, setup) = fixture();
    let (sender, receiver) = sync_channel(1);
    if !matches!(case, Closed | SilenceAtDeadline) {
        sender
            .send(ObservationSignal {
                attempt: if matches!(case, WrongAttempt) {
                    "c".repeat(64)
                } else {
                    setup.attempt.clone()
                },
                container_id: if matches!(case, WrongId) { OTHER } else { ID }.into(),
                outcome: match case {
                    Failed => ObservationOutcome::Failed,
                    EarlyExit => ObservationOutcome::EarlyExit,
                    _ => ObservationOutcome::Complete,
                },
            })
            .unwrap();
    }
    let _sender = if matches!(case, Closed) {
        drop(sender);
        None
    } else {
        Some(sender)
    };
    let origin = Instant::now();
    let invokes = Cell::new(0);
    let reads = Cell::new(0);
    let uncertain = matches!(case, UncertainStart);
    let cleanup = if uncertain { 4 } else { 5 };
    let mut replies = vec![
        snapshot(image(&setup)),
        ok(ID),
        snapshot(inspect(&setup, false)),
    ];
    if uncertain {
        replies.push(refuse("inert lost start response"));
    } else {
        replies.extend([
            ok(ID),
            snapshot(inspect(&setup, !matches!(case, InspectEarlyExit))),
        ]);
    }
    replies.extend([snapshot(inspect(&setup, true)), ok(ID), ok("")]);
    let (result, calls) = drive_with(&setup, replies, |invoke| {
        run_attempt(
            Some(&setup),
            |bin, args| {
                let i = invokes.get();
                invokes.set(i + 1);
                if i == cleanup {
                    assert_eq!(reads.get(), 1 + wait_reads, "wait must precede cleanup");
                }
                invoke(bin, args)
            },
            Some(&receiver),
            || {
                let r = reads.get();
                reads.set(r + 1);
                if r == 0 {
                    assert_eq!(invokes.get(), 3, "anchor immediately precedes start");
                    return origin;
                }
                assert_eq!(
                    invokes.get(),
                    5,
                    "wait follows checked start and running inspect"
                );
                origin
                    + match (case, r) {
                        (StartSpentBudget, _) | (LateSignal | SilenceAtDeadline, 2..) => {
                            OBSERVATION_LIMIT
                        }
                        (LateSignal | SilenceAtDeadline, 1) => {
                            OBSERVATION_LIMIT - Duration::from_nanos(1)
                        }
                        _ => Duration::ZERO,
                    }
            },
            || matches!(case, CancelBeforeAcceptingSignal) && reads.get() >= 2,
        )
    });
    assert_eq!(calls.iter().filter(|c| c[0] == "start").count(), 1);
    assert_eq!(calls[3], ["start", ID]);
    if !uncertain {
        assert_eq!(calls[4], ["container", "inspect", ID]);
    }
    assert_eq!(
        calls[cleanup..],
        [
            strings(&["container", "inspect", ID]),
            strings(&["rm", "--force", ID]),
            exact_name_list(&setup.name()),
        ]
    );
    assert_eq!(result["observation_window"], expected);
    assert_eq!(
        result["hold_inspected"],
        !matches!(case, UncertainStart | InspectEarlyExit)
    );
    assert_eq!(result["docker_remove_cli_success"], true);
    assert_eq!(result["docker_exact_name_list_empty"], true);
    assert_no_replay(&setup);
}

#[test]
fn public_hold_observation_checks_binding_and_never_authorizes_more_work() {
    use WindowCase::*;
    for (case, outcome) in [
        (Complete, "signal_complete"),
        (Failed, "failed"),
        (EarlyExit, "early_exit"),
        (WrongAttempt, "misbound"),
        (WrongId, "misbound"),
        (Closed, "unavailable"),
    ] {
        check_observation(case, outcome, 2);
    }
}

#[test]
fn public_hold_observation_deadline_includes_start_and_defeats_late_ready() {
    use WindowCase::*;
    check_observation(StartSpentBudget, "deadline", 1);
    check_observation(LateSignal, "deadline", 2);
    check_observation(SilenceAtDeadline, "deadline", 2);
}

#[test]
fn public_hold_observation_cancellation_and_uncertain_start_go_to_cleanup() {
    use WindowCase::*;
    check_observation(CancelBeforeAcceptingSignal, "cancelled", 1);
    check_observation(InspectEarlyExit, "not_opened", 0);
    check_observation(UncertainStart, "not_opened", 0);
}

#[test]
fn public_hold_remove_status_does_not_substitute_for_exact_name_query() {
    for (list_reply, empty, failed) in [
        (Ok((false, vec![])), Value::Null, true),
        (refuse("inert list command timed out"), Value::Null, true),
        (ok(OTHER), json!(false), false),
    ] {
        let (_dir, setup) = fixture();
        let (result, calls) = drive(
            &setup,
            vec![
                snapshot(image(&setup)),
                ok(ID),
                snapshot(inspect(&setup, false)),
                ok(ID),
                snapshot(inspect(&setup, true)),
                snapshot(inspect(&setup, true)),
                ok(""),
                list_reply,
            ],
        );
        assert_eq!(calls.last().unwrap(), &exact_name_list(&setup.name()));
        assert_eq!(result["docker_remove_cli_success"], true);
        assert_eq!(result["docker_exact_name_list_empty"], empty);
        assert_eq!(result["docker_exact_name_list_error"].is_string(), failed);
        assert_no_replay(&setup);
    }
}

#[test]
fn public_hold_absent_crossed_and_invalid_recipe_refuse_before_any_effect() {
    let (_dir, mut setup) = fixture();
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
    let canonical = setup.image_id.clone();
    for invalid in [
        "hold:latest",
        "sha256:abcd",
        &"a".repeat(64),
        &format!("sha256:{}", "A".repeat(64)),
    ] {
        setup.image_id = invalid.into();
        let err = Host::run_entry_at_boundary(
            RunEntry::PublicInferenceSetup,
            None,
            Some(&setup),
            |_, _| panic!("invalid local ID reached Docker"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid public hold recipe"));
    }
    setup.image_id = canonical;
    let expected_init = std::mem::replace(&mut setup.init_sha256, "not-a-content-id".into());
    assert!(
        Host::run_entry_at_boundary(
            RunEntry::PublicInferenceSetup,
            None,
            Some(&setup),
            |_, _| panic!("invalid init content identity reached Docker"),
        )
        .unwrap_err()
        .to_string()
        .contains("invalid public hold recipe")
    );
    setup.init_sha256 = expected_init;
    fs::set_permissions(&setup.docker.path, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&setup.docker.path, b"changed").unwrap();
    let err = Host::run_entry_at_boundary(
        RunEntry::PublicInferenceSetup,
        None,
        Some(&setup),
        |_, _| panic!("changed client reached Docker"),
    )
    .unwrap_err();
    assert!(err.to_string().contains("pin mismatch"));
    assert_eq!(fs::read_dir(&setup.fence_root).unwrap().count(), 0);
}

fn assert_preflight_only(setup: &Launch, reply: Reply, error: &str) {
    let (result, calls) = drive(setup, vec![reply]);
    assert_eq!(calls, [strings(&["image", "inspect", &setup.image_id])]);
    assert_eq!(result["create_attempted"], false);
    assert_eq!(result["hold_inspected"], false);
    assert_eq!(result["docker_remove_cli_success"], Value::Null);
    assert_eq!(result["docker_exact_name_list_empty"], Value::Null);
    assert_eq!(result["cleanup_target_error"], Value::Null);
    assert!(
        result["launch_error"].as_str().unwrap().contains(error),
        "{result}"
    );
    assert_no_replay(setup);
}

#[test]
fn public_hold_invalid_or_missing_local_image_never_allocates_or_pulls() {
    let (_dir, setup) = fixture();
    assert_preflight_only(
        &setup,
        Ok((false, vec![])),
        "local image inspect unavailable",
    );
    let (_dir, setup) = fixture();
    assert_preflight_only(&setup, ok("[]"), "inspect identity ambiguous");
    for (pointer, value) in [
        ("/Id", json!(format!("sha256:{OTHER}"))),
        ("/Id", json!("sha256:bbbb")),
        ("/Os", json!("windows")),
        ("/Architecture", json!("arm64")),
        ("/Variant", json!("v3")),
    ] {
        let (_dir, setup) = fixture();
        let mut wrong = image(&setup);
        *wrong.pointer_mut(pointer).unwrap() = value;
        assert_preflight_only(
            &setup,
            snapshot(wrong),
            "local image identity/platform conflict",
        );
    }
}

#[test]
fn public_hold_image_defaults_or_volumes_refuse_before_create() {
    for (key, value) in [
        ("Volumes", json!({"/payload": {}})),
        ("Volumes", json!([])),
        ("Env", json!(["LD_PRELOAD=unexpected.so"])),
        ("Env", json!(["PATH=/usr/bin:/bin"])),
        ("Entrypoint", json!(["/bin/sh"])),
        ("Cmd", json!(["node"])),
        ("Healthcheck", json!({"Test": ["CMD-SHELL", "true"]})),
        ("ExposedPorts", json!({"8080/tcp": {}})),
        ("OnBuild", json!(["RUN true"])),
    ] {
        let (_dir, setup) = fixture();
        let mut wrong = image(&setup);
        wrong["Config"][key] = value;
        assert_preflight_only(
            &setup,
            snapshot(wrong),
            "local image configuration conflict",
        );
    }
}

#[test]
fn public_hold_config_drift_does_not_start_but_can_remove_exact_container() {
    for (pointer, value) in [
        ("/Path", json!("node")),
        ("/Config/Image", json!("unexpected:tag")),
        ("/Config/Entrypoint", json!(["/bin/sh"])),
        ("/Config/OpenStdin", json!(true)),
        ("/Config/Healthcheck/Test", json!(["CMD", "node"])),
        (
            "/Mounts",
            json!([{"Type":"volume", "Destination":"/payload"}]),
        ),
        ("/Mounts", Value::Null),
        ("/Config/Volumes", json!({"/payload": {}})),
        (
            "/HostConfig/Mounts",
            json!([{"Type":"tmpfs", "Target":"/payload"}]),
        ),
        ("/HostConfig/Binds", json!(["/host:/payload:ro"])),
        ("/HostConfig/Privileged", json!(true)),
        ("/HostConfig/CgroupnsMode", json!("host")),
        ("/Config/Env", json!(["LD_PRELOAD=unexpected.so"])),
        ("/Config/Env", json!(["PATH=/usr/bin:/bin"])),
    ] {
        let (_dir, setup) = fixture();
        let mut wrong = inspect(&setup, false);
        *wrong.pointer_mut(pointer).unwrap() = value;
        let (result, calls) = drive(
            &setup,
            vec![
                snapshot(image(&setup)),
                ok(ID),
                snapshot(wrong.clone()),
                snapshot(wrong),
                ok(ID),
                ok(""),
            ],
        );
        assert_eq!(result["hold_inspected"], false);
        assert_eq!(result["docker_remove_cli_success"], true);
        assert_eq!(result["docker_exact_name_list_empty"], true);
        assert_eq!(calls[4], ["rm", "--force", ID]);
        assert!(!calls.iter().any(|a| a[0] == "start"));
    }
}

#[test]
fn public_hold_crossed_inspection_cannot_start_or_remove_foreign_identity() {
    for (pointer, value) in [
        ("/Id", json!(OTHER)),
        ("/Image", json!(format!("sha256:{OTHER}"))),
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
            vec![
                snapshot(image(&setup)),
                ok(ID),
                snapshot(foreign.clone()),
                snapshot(foreign),
            ],
        );
        assert_eq!(result["hold_inspected"], false);
        assert_eq!(result["docker_remove_cli_success"], Value::Null);
        assert_eq!(result["docker_exact_name_list_empty"], Value::Null);
        assert!(
            result["cleanup_target_error"]
                .as_str()
                .unwrap()
                .contains("identity conflict")
        );
        assert_eq!(calls.len(), 4);
        assert!(
            !calls
                .iter()
                .any(|a| matches!(a[0].as_str(), "start" | "rm"))
        );
    }
}

#[test]
fn public_hold_uncertain_start_has_only_cleanup_and_cannot_replay_on_reentry() {
    let (_dir, mut setup) = fixture();
    let (result, calls) = drive(
        &setup,
        vec![
            snapshot(image(&setup)),
            ok(ID),
            snapshot(inspect(&setup, false)),
            refuse("inert CLI timeout after start request"),
            snapshot(inspect(&setup, true)),
            Ok((false, vec![])),
        ],
    );
    assert_eq!(result["hold_inspected"], false);
    assert_eq!(result["docker_remove_cli_success"], false);
    assert_eq!(result["docker_exact_name_list_empty"], Value::Null);
    assert_eq!(calls[3], ["start", ID]);
    assert_eq!(calls[5], ["rm", "--force", ID]);
    // Reenter with changed (still pinned) recipe bytes for the same attempt.
    setup.image_id = format!("sha256:{}", "f".repeat(64));
    assert_no_replay(&setup);
}

#[test]
fn public_hold_uncertain_create_never_starts_or_treats_absence_as_settled() {
    let (_dir, setup) = fixture();
    let (result, calls) = drive(
        &setup,
        vec![
            snapshot(image(&setup)),
            refuse("inert create timeout"),
            Ok((false, vec![])),
        ],
    );
    assert_eq!(result["create_attempted"], true);
    assert_eq!(result["hold_inspected"], false);
    assert_eq!(result["docker_remove_cli_success"], Value::Null);
    assert!(result["cleanup_target_error"].is_string());
    assert_eq!(calls[2], strings(&["container", "inspect", &setup.name()]));
    assert_no_replay(&setup);

    let (_dir, setup) = fixture();
    let (result, calls) = drive(
        &setup,
        vec![
            snapshot(image(&setup)),
            refuse("inert create timeout"),
            snapshot(inspect(&setup, false)),
            ok(ID),
            ok(""),
        ],
    );
    assert_eq!(result["hold_inspected"], false);
    assert_eq!(result["docker_remove_cli_success"], true);
    assert_eq!(result["docker_exact_name_list_empty"], true);
    assert_eq!(calls[3], ["rm", "--force", ID]);
    assert_no_replay(&setup);

    let (_dir, setup) = fixture();
    let mut foreign = inspect(&setup, false);
    foreign["Id"] = json!(OTHER);
    foreign["Image"] = json!(format!("sha256:{OTHER}"));
    let (result, calls) = drive(
        &setup,
        vec![
            snapshot(image(&setup)),
            refuse("inert create timeout"),
            snapshot(foreign),
        ],
    );
    assert_eq!(result["hold_inspected"], false);
    assert_eq!(result["docker_remove_cli_success"], Value::Null);
    assert!(result["cleanup_target_error"].is_string());
    assert_eq!(calls.len(), 3);
    assert_no_replay(&setup);
}
