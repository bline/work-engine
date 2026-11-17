#![allow(dead_code)]
//! Support for the real offline Host integration (ignored tests only). Everything here builds the
//! real fixture Job from the real compiler output and the pinned, read-only handoff trees. Nothing
//! here is production authority: the signing key is the sanctioned fixture constant and the
//! signature directory lives outside Git.
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use work_engine_authority_store::{
    adapter::AdapterInputs,
    fixture::{
        AdmissionBinding, FixtureAcceptance, FixtureAdmission, FixtureGrant, FixtureSchedule,
    },
    ident::sha256_hex,
    store::Store,
};
use work_engine_manual_host::{
    Artifact,
    pi_docker::{
        IMAGE, Tree,
        coding::{PROFILE, Profile, SHIM, SIGNED_PROFILE, WorkspaceWriteBoundary},
        inventory,
    },
};
use work_engine_organization::{OrganizationCandidate, OrganizationView};

pub const HANDOFF: &str = "/home/bline/cbm-work/pi-artifact-handoff-20261113";
pub const PI_MANIFEST_SHA: &str =
    "cf9d096e61d8ffd2c35df238ff74e1f8d7dacfb9bc5813333beef6cc73010985";
pub const TOOLCHAIN_MANIFEST_SHA: &str =
    "90d4c2315715eddc41d2711e3c4d1612e9b7b5fb61541466bcf8066b4c8d8da2";

pub const LIB_RS: &str = r#"#[derive(Debug,PartialEq)]
pub struct DelayParseError;
pub fn parse_delay_ms(value:&str)->Result<u64,DelayParseError>{
 let (digits,scale)=if let Some(n)=value.strip_suffix("ms"){(n,1)}else if let Some(n)=value.strip_suffix('s'){(n,1000)}else if let Some(n)=value.strip_suffix('m'){(n,1000)}else{return Err(DelayParseError)};
 if digits.is_empty()||!digits.bytes().all(|b|b.is_ascii_digit()){return Err(DelayParseError)}
 digits.parse::<u64>().ok().and_then(|n|n.checked_mul(scale)).ok_or(DelayParseError)
}
"#;
pub const ACCEPTANCE: &str = r#"#[path="/workspace/src/lib.rs"] mod parser;
use parser::parse_delay_ms as p;
#[test]fn units(){assert_eq!(p("2m"),Ok(120000));assert_eq!(p("1m"),Ok(60000));assert_eq!(p("2s"),Ok(2000));assert_eq!(p("12ms"),Ok(12));assert_eq!(p("0m"),Ok(0));}
#[test]fn malformed(){for v in ["","m","-1m","+1s","1.2s"," 1s","1s ","1h","1MS","1\ns"]{assert!(p(v).is_err(),"{v}");}}
#[test]fn overflow(){assert!(p("18446744073709551616ms").is_err());assert!(p("18446744073709551615m").is_err());assert!(p("18446744073709551615s").is_err());assert_eq!(p("18446744073709551615ms"),Ok(u64::MAX));}
"#;
/// The strict (SrcLibRs) fixture command: the one the earlier real strict qualification used
/// (`pi_coding_actual.rs`, `S10_STRICT_SOURCE`), except the background writer's delay is 30 s,
/// not 3 s. Attempt 3 ran with 3 s and lost the race (the signed flow needs about 5 s after the
/// tool call before the Host stops the tool container, n=3 runs); the 30 s value was NOT executed
/// in Docker (no attempt remained).
pub const COMMAND: &str = r#"set -eu
rustc --version
if touch /workspace/forbidden; then exit 31; fi
if touch /workspace/src/forbidden; then exit 32; fi
if printf 'forbidden' > /acceptance.rs; then exit 33; fi
printf 'replacement' > /tmp/rename-source
if mv /tmp/rename-source /workspace/src/lib.rs; then exit 34; fi
if ln /workspace/src/lib.rs /workspace/src/alias; then exit 35; fi
printf 'STRICT_PERSISTENT_WRITE_BOUNDARY_PASSED\n'
cc --version | head -1
test ! -e /auth/auth.json
test ! -e /pi
test ! -e /var/run/docker.sock
if grep -a 'FAKE-CANARY' /proc/1/environ; then exit 17; fi
rustc --edition=2024 --test /acceptance.rs -o /tmp/check
if /tmp/check; then echo 'baseline unexpectedly passed'; exit 18; else baseline_status=$?; test "$baseline_status" -eq 101; echo 'EXPECTED_SEEDED_MINUTE_FAILURE'; fi
sed "s/(n,1000)}else{return Err/(n,60000)}else{return Err/" src/lib.rs > /tmp/edited; cat /tmp/edited > src/lib.rs
rustc --edition=2024 --test /acceptance.rs -o /tmp/check
/tmp/check
printf 'ISOLATED_EDIT_AND_TEST_PASSED\n'
(sleep 30; printf 'untested' > src/lib.rs; printf 'out of scope' > extra-background-file) >/dev/null 2>&1 &
printf 'MODEL_BACKGROUND_WRITER_STARTED\n'
"#;

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn art(path: &Path, bytes: &[u8]) -> Artifact {
    std::fs::write(path, bytes).unwrap();
    Artifact {
        path: path.to_owned(),
        sha256: sha256_hex(bytes),
    }
}

/// Opens a pinned manifest at its absolute handoff path and checks the pinned sha256.
pub fn pinned_tree(name: &str, manifest_sha: &str) -> Tree {
    let root = PathBuf::from(HANDOFF).join(name);
    let path = PathBuf::from(HANDOFF).join(format!("{name}.manifest.json"));
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(sha256_hex(&bytes), manifest_sha, "{name} manifest pin");
    Tree {
        root,
        manifest: Artifact {
            path,
            sha256: manifest_sha.to_owned(),
        },
    }
}

/// `Tree::verify` is private to the Host; this is the same comparison it makes:
/// the re-inventory of the root must equal the pinned manifest.
pub fn tree_matches_manifest(t: &Tree) -> bool {
    let expected: std::collections::BTreeMap<String, work_engine_manual_host::pi_docker::Entry> =
        serde_json::from_slice(&std::fs::read(&t.manifest.path).unwrap()).unwrap();
    inventory(&t.root).unwrap() == expected
}

pub struct Rig {
    pub root: PathBuf,
    pub inputs: AdapterInputs,
}

/// Creates the run directory `run` (must not exist) and the Job inputs for it.
pub fn rig(run: &Path, signed: bool, deadline: u64) -> Rig {
    std::fs::create_dir(run).unwrap();
    let root = std::fs::canonicalize(run).unwrap();
    for d in ["state", "packets", "meta", "workspace", "workspace/src"] {
        std::fs::create_dir(root.join(d)).unwrap();
    }
    std::fs::write(root.join("workspace/src/lib.rs"), LIB_RS).unwrap();
    let meta = root.join("meta");
    let workspace = {
        let r = root.join("workspace");
        let inv = inventory(&r).unwrap();
        Tree {
            root: r,
            manifest: art(
                &meta.join("workspace-manifest.json"),
                &serde_json::to_vec(&inv).unwrap(),
            ),
        }
    };
    let docker_path = PathBuf::from("/usr/bin/docker");
    let docker = Artifact {
        sha256: sha256_hex(&std::fs::read(&docker_path).unwrap()),
        path: docker_path,
    };
    let coding = Profile {
        schema: if signed { SIGNED_PROFILE } else { PROFILE }.into(),
        shim: art(&meta.join("pi-coding.mjs"), SHIM),
        toolchain: Some(pinned_tree("toolchain", TOOLCHAIN_MANIFEST_SHA)),
        acceptance: Some(art(&meta.join("acceptance.rs"), ACCEPTANCE.as_bytes())),
        fixture_command: COMMAND.into(),
        max_tool_calls: 1,
        workspace_write_boundary: WorkspaceWriteBoundary::SrcLibRs,
        provider: None,
    };
    let inputs = AdapterInputs {
        state_dir: root.join("state"),
        packet_dir: root.join("packets"),
        authority: art(
            &meta.join("authority.json"),
            b"fixture-only signed fake-provider qualification; not production authority",
        ),
        docker,
        runtime: pinned_tree("pi", PI_MANIFEST_SHA),
        workspace,
        deadline_unix_seconds: deadline,
        command_timeout_seconds: 30,
        coding: Some(coding),
    };
    Rig { root, inputs }
}

pub struct Chain {
    pub store: Store,
    pub cand: OrganizationCandidate,
    pub view: OrganizationView,
    pub adm: FixtureAdmission,
    pub grant: FixtureGrant,
    pub sched: FixtureSchedule,
}

/// Real requirements -> derive/view -> fixture install/admit/grant -> fixture schedule.
pub fn chain(rig: &Rig, store_dir: &Path) -> Chain {
    let (_, cand, view) = crate::common::candidate();
    let mut store = Store::create(store_dir, "p1f-e2e-store", Duration::from_millis(5000)).unwrap();
    store
        .fixture_install(&FixtureAcceptance::synthetic("e2e-acceptance"), &cand)
        .unwrap();
    let binding = AdmissionBinding {
        image: IMAGE.into(),
        runtime_manifest_sha256: rig.inputs.runtime.manifest.sha256.clone(),
    };
    let adm = store.fixture_admit(&cand, &binding).unwrap();
    let grant = store.fixture_grant(&adm).unwrap();
    let ob = view.obligations[0].id.as_str().to_owned();
    let sched = store
        .fixture_schedule(&adm, &view, &cand, &ob, &rig.inputs)
        .unwrap();
    Chain {
        store,
        cand,
        view,
        adm,
        grant,
        sched,
    }
}

pub fn exe_sha() -> String {
    sha256_hex(&std::fs::read(std::env::current_exe().unwrap()).unwrap())
}

/// Runs the existing signer test (`export_signed_pi_fixture`) for exactly this job and executable.
pub fn sign(
    out: &Path,
    job_sha: &str,
    exe_sha: &str,
    lifetime: u64,
) -> session_task_admission::canonical_source::Pin {
    let signer =
        std::env::var("P1F_SIGNER").expect("P1F_SIGNER (session-task-admission test binary)");
    let status = std::process::Command::new(signer)
        .args([
            "--ignored",
            "--exact",
            "pi_pilot::tests::export_signed_pi_fixture",
            "--nocapture",
        ])
        .env("PI_FIXTURE_EXPORT", out)
        .env("PI_FIXTURE_JOB_SHA", job_sha)
        .env("PI_FIXTURE_EXECUTOR_SHA", exe_sha)
        .env("PI_FIXTURE_LIFETIME_SECONDS", lifetime.to_string())
        .status()
        .unwrap();
    assert!(status.success(), "signer failed");
    serde_json::from_slice(&std::fs::read(out.join("descriptor-pin.json")).unwrap()).unwrap()
}
