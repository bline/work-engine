//! Actual disposable qualification. Explicitly disabled unless the owner sets
//! task-owned evidence roots and runs the ignored test. Never uses credentials.
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use work_engine_manual_host::{
    Artifact, digest,
    pi_docker::{
        Host, IMAGE, Job, SCHEMA, Tree,
        coding::{PROFILE, Profile, SHIM, WorkspaceWriteBoundary},
        inventory,
    },
};
fn artifact(path: &Path, bytes: &[u8]) -> Artifact {
    fs::write(path, bytes).unwrap();
    Artifact {
        path: path.into(),
        sha256: digest(bytes),
    }
}
fn tree(root: PathBuf, manifest: &Path) -> Tree {
    let manifest = artifact(
        manifest,
        &serde_json::to_vec(&inventory(&root).unwrap()).unwrap(),
    );
    Tree { root, manifest }
}
#[test]
#[ignore = "actual Docker; requires explicit provider-disabled qualification packet"]
fn actual_sdk_rust_isolated_tool_edit_test_cleanup() {
    let root = PathBuf::from(
        std::env::var_os("S10_CODING_EVIDENCE").expect("private evidence directory required"),
    );
    fs::create_dir(&root).unwrap();
    let input = root.join("input");
    let state = root.join("state");
    fs::create_dir(&input).unwrap();
    fs::create_dir(input.join("src")).unwrap();
    fs::create_dir(&state).unwrap();
    fs::write(input.join("src/lib.rs"),r#"#[derive(Debug,PartialEq)]
pub struct DelayParseError;
pub fn parse_delay_ms(value:&str)->Result<u64,DelayParseError>{
 let (digits,scale)=if let Some(n)=value.strip_suffix("ms"){(n,1)}else if let Some(n)=value.strip_suffix('s'){(n,1000)}else if let Some(n)=value.strip_suffix('m'){(n,1000)}else{return Err(DelayParseError)};
 if digits.is_empty()||!digits.bytes().all(|b|b.is_ascii_digit()){return Err(DelayParseError)}
 digits.parse::<u64>().ok().and_then(|n|n.checked_mul(scale)).ok_or(DelayParseError)
}
"#).unwrap();
    let attack = std::env::var_os("S10_VALIDATION_MUTATION").is_some();
    if attack {
        let path = input.join("src/lib.rs");
        let original = fs::read_to_string(&path).unwrap();
        fs::write(path,original.replace(" let (digits", " if !std::fs::read_to_string(\"/proc/1/comm\").unwrap_or_default().contains(\"sleep\") { std::fs::write(\"/workspace/extra-file\",b\"must fail read-only\").unwrap(); }\n let (digits")).unwrap();
    }
    let acceptance=artifact(&root.join("acceptance.rs"),br#"#[path="/workspace/src/lib.rs"] mod parser;
use parser::parse_delay_ms as p;
#[test]fn units(){assert_eq!(p("2m"),Ok(120000));assert_eq!(p("1m"),Ok(60000));assert_eq!(p("2s"),Ok(2000));assert_eq!(p("12ms"),Ok(12));assert_eq!(p("0m"),Ok(0));}
#[test]fn malformed(){for v in ["","m","-1m","+1s","1.2s"," 1s","1s ","1h","1MS","1\ns"]{assert!(p(v).is_err(),"{v}");}}
#[test]fn overflow(){assert!(p("18446744073709551616ms").is_err());assert!(p("18446744073709551615m").is_err());assert!(p("18446744073709551615s").is_err());assert_eq!(p("18446744073709551615ms"),Ok(u64::MAX));}
"#);
    let runtime = tree(
        PathBuf::from(std::env::var_os("S10_PI_RUNTIME").expect("pinned runtime root required")),
        &root.join("runtime.json"),
    );
    let toolchain = tree(
        PathBuf::from(
            std::env::var_os("S10_RUST_TOOLCHAIN").expect("pinned offline toolchain required"),
        ),
        &root.join("toolchain.json"),
    );
    let workspace = tree(input, &root.join("input.json"));
    let shim = artifact(&root.join("pi-coding.mjs"), SHIM);
    let docker_path = PathBuf::from("/usr/bin/docker");
    let docker = Artifact {
        sha256: digest(&fs::read(&docker_path).unwrap()),
        path: docker_path,
    };
    let mut command=r#"set -eu
rustc --version
cc --version | head -1
test ! -e /auth/auth.json
test ! -e /pi
test ! -e /var/run/docker.sock
if grep -a 'FAKE-CANARY' /proc/1/environ; then exit 17; fi
rustc --edition=2024 --test /acceptance.rs -o /tmp/check
if /tmp/check; then echo 'baseline unexpectedly passed'; exit 18; else baseline_status=$?; test "$baseline_status" -eq 101; echo 'EXPECTED_SEEDED_MINUTE_FAILURE'; fi
sed -i "s/(n,1000)}else{return Err/(n,60000)}else{return Err/" src/lib.rs
rustc --edition=2024 --test /acceptance.rs -o /tmp/check
/tmp/check
printf 'ISOLATED_EDIT_AND_TEST_PASSED\n'
(sleep 3; printf 'untested' > src/lib.rs; printf 'out of scope' > extra-background-file) >/dev/null 2>&1 &
printf 'MODEL_BACKGROUND_WRITER_STARTED\n'
"#.to_string();
    let strict = std::env::var_os("S10_STRICT_SOURCE").is_some();
    if strict {
        command = command.replace(
            "sed -i \"s/(n,1000)}else{return Err/(n,60000)}else{return Err/\" src/lib.rs",
            "sed \"s/(n,1000)}else{return Err/(n,60000)}else{return Err/\" src/lib.rs > /tmp/edited; cat /tmp/edited > src/lib.rs",
        );
        command = command.replace(
            "rustc --version",
            r#"rustc --version
if touch /workspace/forbidden; then exit 31; fi
if touch /workspace/src/forbidden; then exit 32; fi
if printf 'forbidden' > /acceptance.rs; then exit 33; fi
printf 'replacement' > /tmp/rename-source
if mv /tmp/rename-source /workspace/src/lib.rs; then exit 34; fi
if ln /workspace/src/lib.rs /workspace/src/alias; then exit 35; fi
printf 'STRICT_PERSISTENT_WRITE_BOUNDARY_PASSED\n'"#,
        );
    }
    let job = Job {
        schema: SCHEMA.into(),
        job_id: if strict {
            "mcp-strict-coding-disabled"
        } else {
            "s10-coding-disabled"
        }
        .into(),
        state_dir: state.clone(),
        authority: artifact(
            &root.join("authority.json"),
            b"explicit provider-disabled fixture qualification only",
        ),
        packet: artifact(
            &root.join("packet.json"),
            b"real SDK tool callback; canned fake stream, not inference",
        ),
        docker,
        image: IMAGE.into(),
        runtime,
        workspace,
        deadline_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 240,
        command_timeout_seconds: 30,
        idle_seconds: 0,
        receiver: None,
        coding: Some(Profile {
            schema: PROFILE.into(),
            shim,
            toolchain,
            acceptance,
            fixture_command: command,
            max_tool_calls: 1,
            workspace_write_boundary: if strict {
                WorkspaceWriteBoundary::SrcLibRs
            } else {
                WorkspaceWriteBoundary::LegacyWorkspace
            },
            provider: None,
        }),
    };
    artifact(
        &root.join("job.json"),
        &serde_json::to_vec_pretty(&job).unwrap(),
    );
    let mut host = Host::admit(job).unwrap();
    let result = host.run().unwrap();
    fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    let id = result["receipt_id"].as_str().unwrap();
    let ack = host.acknowledge(id).unwrap();
    fs::write(
        root.join("ack.json"),
        serde_json::to_vec_pretty(&ack).unwrap(),
    )
    .unwrap();
    assert_eq!(result["receipt"]["coding"]["validation_removed"], true);
    if attack {
        assert_eq!(result["receipt"]["status"], "not_established");
        assert!(result["receipt"]["coding"]["final_acceptance"].is_null());
        assert!(!state.join("validation-workspace/extra-file").exists());
        assert!(
            !result["receipt"]["exported_files"]
                .as_object()
                .unwrap()
                .keys()
                .any(|k| k.starts_with("workspace/"))
        );
        return;
    }
    assert!(result["receipt"]["coding"]["final_acceptance"].is_object());
    assert!(
        !state
            .join("validation-workspace/extra-background-file")
            .exists()
    );
    assert_eq!(
        result["receipt"]["status"],
        "component_passed",
        "{}",
        serde_json::to_string(&result).unwrap()
    );
    assert!(
        fs::read_to_string(state.join("coding-workspace/src/lib.rs"))
            .unwrap()
            .contains("(n,60000)")
    );
    assert_eq!(result["receipt"]["coding"]["native_tool_ends"], 1);
    assert_eq!(result["receipt"]["coding"]["tool_removed"], true);
    drop(host);
    let path = state.join("pi-state.json");
    let mut record: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    record["coding"]["final_acceptance"] = serde_json::Value::Null;
    fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
    let mut recovered = Host::open(&state).unwrap();
    let before = recovered.journal().processes.len();
    let recovery = recovered.reconcile().unwrap();
    assert_eq!(recovery["receipt"]["status"], "not_established");
    assert_eq!(recovered.journal().processes.len(), before);
    fs::write(
        root.join("missing-final-proof-recovery.json"),
        serde_json::to_vec_pretty(&recovery).unwrap(),
    )
    .unwrap();
}
