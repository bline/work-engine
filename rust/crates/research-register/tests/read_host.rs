#![cfg(target_os = "linux")]
use research_register::read_host::*;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
fn delegation() -> PathBuf {
    std::env::var_os("RESEARCH_READ_CGROUP_ROOT").map(PathBuf::from).expect("set RESEARCH_READ_CGROUP_ROOT to a delegated cgroup-v2 parent with memory+pids enabled")
}
fn input(probe: &str) -> WorkerInput {
    let exe = Path::new(env!("CARGO_BIN_EXE_work-engine-research-read"));
    WorkerInput {
        request: None,
        profile: HostProfile {
            cpu_seconds: 1,
            wall_millis: 4000,
            memory_bytes: 64 << 20,
            output_bytes: 8192,
            ..HostProfile::default()
        },
        producer: {
            static PIN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
            PIN.get_or_init(|| producer(exe).unwrap()).clone()
        },
        probe: Some(probe.into()),
    }
}
#[test]
#[ignore = "requires RESEARCH_READ_CGROUP_ROOT delegated Linux cgroup-v2 memory+pids parent"]
fn shipping_host_controls_and_complete_output() {
    let exe = Path::new(env!("CARGO_BIN_EXE_work-engine-research-read"));
    let root = delegation();
    let cancel = AtomicBool::new(false);
    let before: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .filter(|name| {
            name.to_string_lossy()
                .starts_with(&format!("research-read-{}-", std::process::id()))
        })
        .collect();
    assert_eq!(
        run(exe, &root, input("success"), &cancel).unwrap()["probe"],
        "success"
    );
    for (probe, expected) in [
        ("cpu", "WorkerExit"),
        ("memory", "MemoryLimit"),
        ("output", "OutputLimit"),
        ("crash", "WorkerExit"),
        ("truncated", "Frame"),
    ] {
        assert!(
            run(exe, &root, input("success"), &cancel).is_ok(),
            "control before {probe}"
        );
        let mut negative = input(probe);
        if probe == "memory" {
            negative.profile.cpu_seconds = 60;
            negative.profile.wall_millis = 10000;
            negative.profile.memory_bytes = 16 << 20;
            let mut control = input("success");
            control.profile = negative.profile.clone();
            assert!(run(exe, &root, control, &cancel).is_ok());
        }
        let error = run(exe, &root, negative, &cancel).unwrap_err();
        assert!(error.contains(expected), "{probe}: {error}");
        println!("shipping boundary {probe}: {error}");
    }
    assert!(run(exe, &root, input("success"), &cancel).is_ok());
    let mut wall = input("wall");
    // The envelope now includes sealed-image copy/hash and process setup.
    wall.profile.wall_millis = 2000;
    let mut control = input("success");
    control.profile = wall.profile.clone();
    assert!(run(exe, &root, control, &cancel).is_ok());
    assert_eq!(run(exe, &root, wall, &cancel).unwrap_err(), "WallLimit");
    assert!(run(exe, &root, input("success"), &cancel).is_ok());
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(50));
            cancel.store(true, Ordering::SeqCst);
        });
        assert_eq!(
            run(exe, &root, input("wall"), &cancel).unwrap_err(),
            "Cancelled"
        );
    });
    cancel.store(false, Ordering::SeqCst);
    assert!(run(exe, &root, input("success"), &cancel).is_ok());
    let mut wrong = input("success");
    wrong.producer = "0".repeat(64);
    assert_eq!(
        run(exe, &root, wrong, &cancel).unwrap_err(),
        "ProducerMismatch"
    );
    assert!(run(exe, &root, input("success"), &cancel).is_ok());
    assert!(
        run(
            exe,
            Path::new("/unavailable/cgroup"),
            input("success"),
            &cancel
        )
        .unwrap_err()
        .contains("ControlsUnavailable")
    );
    let after: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .filter(|name| {
            name.to_string_lossy()
                .starts_with(&format!("research-read-{}-", std::process::id()))
        })
        .collect();
    assert_eq!(before, after, "all owned cgroups collected");
    real_frozen_eleven_register_resource_control_in_shipping_host();
}

fn real_frozen_eleven_register_resource_control_in_shipping_host() {
    use research_register::{GitObjects, use_bundle::BlobPin};
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let at = GitObjects::open(&repo)
        .unwrap()
        .resolve("HEAD")
        .unwrap()
        .to_string();
    let request = ReadRequest {
        repository: repo.clone(),
        commit: at.clone(),
        register_path: "metrics/register".into(),
        bundle: BlobPin {
            commit: String::new(),
            path: String::new(),
            blob: String::new(),
            sha256: String::new(),
        },
    };
    let exe = Path::new(env!("CARGO_BIN_EXE_work-engine-research-read"));
    let profile = HostProfile {
        repository_scope: Some(repo),
        ..HostProfile::default()
    };
    let report = run(
        exe,
        &delegation(),
        WorkerInput {
            request: Some(request),
            profile,
            producer: producer(exe).unwrap(),
            probe: Some("register-only".into()),
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report["schema"], "research-read-qualification-v1");
    assert_eq!(report["outcome"], "control_complete");
    assert_eq!(report["m1_bundle"], "NOT_EXERCISED");
    assert_eq!(report["decisions"], 11);
    println!("REAL_SOURCE_REGISTER_ONLY_RESOURCE_CONTROL {at} {report}");
}

#[test]
fn public_cli_open_stdin_obeys_wall_and_cancellation() {
    use std::{
        io::Write,
        process::{Command, Stdio},
        time::Instant,
    };
    let root = std::env::temp_dir().join(format!("research-cli-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let profile = root.join("profile.json");
    let request = serde_json::json!({"repository":"/unused", "commit":"0".repeat(40), "register_path":"research", "bundle":{"commit":"0".repeat(40),"path":"bundle","blob":"0".repeat(40),"sha256":"0".repeat(64)}});
    for cancelled in [false, true] {
        std::fs::write(
            &profile,
            serde_json::to_vec(&HostProfile {
                wall_millis: if cancelled { 4000 } else { 100 },
                ..HostProfile::default()
            })
            .unwrap(),
        )
        .unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-research-read"))
            .arg("/unused/cgroup")
            .arg(&profile)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(&serde_json::to_vec(&request).unwrap())
            .unwrap();
        let start = Instant::now();
        if cancelled {
            std::thread::sleep(Duration::from_millis(50));
            unsafe {
                libc::kill(child.id() as i32, libc::SIGTERM);
            }
        }
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break Some(status);
            }
            if start.elapsed() > Duration::from_secs(2) {
                child.kill().unwrap();
                child.wait().unwrap();
                break None;
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        drop(stdin);
        let mut output = String::new();
        std::io::Read::read_to_string(&mut child.stdout.take().unwrap(), &mut output).unwrap();
        assert!(status.is_some(), "CLI blocked beyond deadline: {output}");
        assert!(
            output.contains(if cancelled { "Cancelled" } else { "WallLimit" }),
            "{output}"
        );
    }
    // Closed stdin reaches request parsing rather than an input wait.
    let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-research-read"))
        .arg("/unused/cgroup")
        .arg(&profile)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"{}").unwrap();
    let output = child.wait_with_output().unwrap();
    let output = String::from_utf8(output.stdout).unwrap();
    assert!(output.contains("missing field"), "{output}");
    std::fs::remove_dir_all(root).unwrap();
}
