#![allow(dead_code)]
use claim_evidence::sha256_bytes;
use review_workflow::startup::*;
use serde_json::{Value, json};
use slice_campaign::{AcceptedBoundary, Baseline, CampaignIdentity};
use slice_campaign::{codec::historical_digest, contract::episode_digest};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

pub const HOST: &str = env!("CARGO_BIN_EXE_work-engine-review-host");
pub const PEER: &str = env!("CARGO_BIN_EXE_review-host-controlled-peer");
pub(crate) fn fixture(hash: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../slice-campaign/tests/fixtures/vectors")
            .join(format!("{hash}.json")),
    )
    .unwrap()
}

pub(crate) fn setup_paths(
    root: &Path,
) -> (
    PathBuf,
    PathBuf,
    CampaignIdentity,
    Baseline,
    AcceptedBoundary,
) {
    let repo = root.join("repo");
    let workspace = root.join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let bundle=Path::new(env!("CARGO_MANIFEST_DIR")).join("../slice-campaign/tests/fixtures/vectors/4a4b1ad328857904b2aa50ecc7a3206d3d0e5962f88b69f3d12b703c33b99d45.bundle");
    assert!(
        Command::new("/usr/bin/git")
            .args(["init", "--bare", "-q"])
            .arg(&repo)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("/usr/bin/git")
            .arg("-C")
            .arg(&repo)
            .args(["fetch", "-q"])
            .arg(bundle)
            .args(["refs/*:refs/*"])
            .status()
            .unwrap()
            .success()
    );
    let identity = CampaignIdentity {
        run_id: "sc0-oracle-c2".into(),
        slice_number: 1,
        attempt_id: "sc0-attempt".into(),
        plan_version: "sc0-controlled-v1".into(),
    };
    let baseline = Baseline {
        accepted_commit: "a8e074d765d94a7e816b7ab49db428fd4a760a9f".into(),
        accepted_tree: "17868afbf46ea53715981fce83171001c632b957".into(),
        inter_slice_commit: "a8e074d765d94a7e816b7ab49db428fd4a760a9f".into(),
    };
    let boundary = AcceptedBoundary {
        reference:
            "controlled SC0 candidate-2 frozen source-tree gate fixture under accepted HP2 SC0 plan"
                .into(),
        sha256: "f45b94d3ef985a446002e21a795a51004a5c732e5b150208be09f964152662ef".into(),
    };
    (repo, workspace, identity, baseline, boundary)
}

pub(crate) fn anchored_artifacts(repo: &Path) -> (Vec<u8>, Vec<u8>) {
    let mut candidate: Value = serde_json::from_slice(&fixture(
        "4a2c4f1872f6dfe8de275181f08b1e4d8c33a437c2519cd8b9ce8b3c9b56a5cd",
    ))
    .unwrap();
    let mut profile: Value = serde_json::from_slice(&fixture(
        "5f78af676bf50a7a328b12e0ddcbf619ecfd78fd8e2f46f299330ff0d96095db",
    ))
    .unwrap();
    candidate["repository"] = json!(repo.to_str().unwrap());
    profile["subject"]["repository"] = json!(repo.to_str().unwrap());
    profile["subject_digest"] = json!(historical_digest(&profile["subject"]).unwrap());
    profile["provenance"]["derivation_sources"]["checkpoint_subject"]["identity"]["subject_digest"] =
        profile["subject_digest"].clone();
    let mut unsigned = profile.clone();
    unsigned.as_object_mut().unwrap().remove("profile_digest");
    profile["profile_digest"] = json!(historical_digest(&unsigned).unwrap());
    (
        serde_json::to_vec(&candidate).unwrap(),
        serde_json::to_vec(&profile).unwrap(),
    )
}
fn claim_digest(value: &Value) -> String {
    let encoded = serde_json::to_string(value).unwrap();
    let value = claim_evidence::codec::parse_json(&encoded).unwrap();
    claim_evidence::codec::digest(&value).unwrap()
}
pub(crate) fn claim(
    identity: &CampaignIdentity,
    obligation: &str,
    candidate: &Value,
    boundary: &str,
) -> Value {
    let consumer = if boundary == "builder_projection" {
        format!("slice-builder:{}", identity.key())
    } else {
        format!("slice-campaign:{}", identity.key())
    };
    let episode_id =
        episode_digest(&json!({"identity":identity,"obligationId":obligation})).unwrap();
    let mut claim = json!({"schemaVersion":1,"claimId":"","revision":"","proposition":"independent initial review","subject":{"candidate":candidate,"reviewEpisodeId":&episode_id[..32]},"coveredState":"initial review","consumptionBoundary":boundary,"consumer":consumer,"acceptance":{"owner":"operator","source":"accepted plan","unestablishedRoute":"re-review"},"profile":{"id":"production-path-v1","revision":"production-path-profile-v1","allowedMechanisms":["direct"],"admissibleObservers":["host"],"integrityRequired":true,"requiredRealization":"claude-sonnet-5","requiredCapabilities":[],"continuity":"fresh_initial"}});
    let id_body = json!({"proposition":claim["proposition"],"subject":claim["subject"],"coveredState":claim["coveredState"],"consumptionBoundary":claim["consumptionBoundary"],"consumer":claim["consumer"]});
    claim["claimId"] = json!(format!(
        "production-path-claim-v1@{}",
        claim_digest(&id_body)
    ));
    let mut revision_body = claim.clone();
    revision_body.as_object_mut().unwrap().remove("revision");
    claim["revision"] = json!(format!(
        "production-path-claim-revision-v1@{}",
        claim_digest(&revision_body)
    ));
    claim
}

pub struct Fixture {
    pub temp: tempfile::TempDir,
    pub startup: PathBuf,
    pub document: StartupDocument,
    pub host: PathBuf,
    pub count: PathBuf,
}
fn pin(path: &Path) -> FilePin {
    FilePin {
        path: path.canonicalize().unwrap(),
        sha256: sha256_bytes(&fs::read(path).unwrap()),
        mode: fs::metadata(path).unwrap().permissions().mode() & 0o7777,
    }
}
fn captured(base: &Path, name: &str, bytes: &[u8]) -> FilePin {
    let path = base.join(name);
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    pin(&path)
}
fn data(base: &Path, name: &str, value: &Value) -> FilePin {
    captured(base, name, &serde_json::to_vec(value).unwrap())
}
fn runtime_pins() -> Vec<FilePin> {
    let mut paths = std::collections::BTreeSet::new();
    for executable in [HOST, PEER, "/usr/bin/git", "/usr/bin/timeout"] {
        if executable.starts_with("/usr/bin/") {
            paths.insert(fs::canonicalize(executable).unwrap());
        }
        let output = Command::new("/usr/bin/ldd")
            .arg(executable)
            .output()
            .unwrap();
        assert!(output.status.success());
        for token in std::str::from_utf8(&output.stdout)
            .unwrap()
            .split_whitespace()
        {
            let path = Path::new(token);
            if path.is_absolute() && path.is_file() {
                paths.insert(path.canonicalize().unwrap());
            }
        }
    }
    paths.iter().map(|path| pin(path)).collect()
}
impl Fixture {
    pub fn new(delay: u64, mode: &str) -> Self {
        let scratch = Path::new("/home/bline/code/.work-engine-tmp/h1-fixtures");
        fs::create_dir_all(scratch).unwrap();
        let temp = tempfile::tempdir_in(scratch).unwrap();
        let base = temp.path();
        let (repo, workspace, identity, baseline, boundary) = setup_paths(base);
        let (candidate, physical) = anchored_artifacts(&repo);
        let candidate_value: Value = serde_json::from_slice(&candidate).unwrap();
        let subject = json!({"commit":candidate_value["checkpoint_commit_oid"],"tree":candidate_value["checkpoint_tree_oid"],"patchIdentity":candidate_value["task_patch_digest"]});
        let obligation = "review-0";
        let selection = json!({"schemaVersion":2,"owner":"slice-supervisor","selectionId":"h1-selection","subject":subject,"specialists":[{"obligationId":obligation,"skill":"implementation-review","selection":"selected","requiredClaims":[claim(&identity,obligation,&subject,"builder_projection"),claim(&identity,obligation,&subject,"campaign_terminalization")]}]});
        let episode =
            episode_digest(&json!({"identity":identity,"obligationId":obligation})).unwrap();
        let subject_sha = review_episode_core::codec::digest(
            &review_episode_core::codec::parse_json(&subject.to_string()).unwrap(),
        );
        let authority = json!({"schemaVersion":1,"grantId":"h1-episode-grant","identity":{"runId":identity.run_id,"sliceNumber":identity.slice_number,"attemptId":identity.attempt_id,"planVersion":identity.plan_version,"reviewObligationId":obligation,"reviewEpisodeId":&episode[..32]},"source":{"owner":"human","reference":"accepted-sc0-profile","revision":"h1","sha256":boundary.sha256,"freshness":"exact_revision"},"writer":{"actorId":"native-review-host","provider":"controlled","generation":1,"runtimeSession":{"owner":"runtime","reference":"h1-session","revision":"initial","sha256":sha256_bytes(b"h1-session"),"freshness":"exact_revision"}},"readers":["native-review-host","builder","supervisor"],"initialSubject":{"owner":"checkpoint","reference":"candidate","revision":subject["commit"],"sha256":subject_sha,"freshness":"exact_revision"},"predecessorRevision":null});
        let custody = json!({"owner":"review-execution-evidence","verifier":"review-execution-evidence-process-v1","profile":"controlled-process-v1","artifact_schemes":["execution-artifact:"],"reference_schemes":["execution-evidence:"]});
        let custody_hash = claim_digest(&custody);
        let source = json!({"schema_version":2,"owner":"h1-launcher","reference":"h1-authority","revision":"1","freshness":"exact immutable revision","profile":"native-review-claims-v1","actors":["campaign"],"decision_scopes":["h1-initial"],"grant_ids":["h1-production"],"custody_config_sha256":custody_hash});
        let source_pin = data(base, "claims-source.json", &source);
        let reference = json!({"owner":"h1-launcher","reference":"h1-authority","revision":"1","integrity_sha256":source_pin.sha256,"freshness":"exact immutable revision","status":"verified"});
        let grant = json!({"schema_version":1,"grant_id":"h1-production","actor":"campaign","profile":"production-path-v1","permissions":["record_observation","establish_claim","read_admission"],"decision_scope":"h1-initial","authority_reference":reference});
        let grant_sha = claim_digest(&grant);
        let config = json!({"schema_version":2,"profile":"native-review-claims-v1","root_id":"h1-claims","trusted_custody":custody,"grants":[grant]});
        let host = base.join("work-engine-review-host");
        fs::copy(HOST, &host).unwrap();
        fs::set_permissions(&host, fs::Permissions::from_mode(0o500)).unwrap();
        let peer = base.join("controlled-peer");
        fs::copy(PEER, &peer).unwrap();
        fs::set_permissions(&peer, fs::Permissions::from_mode(0o500)).unwrap();
        let count = base.join("peer-entries");
        let result: Value = serde_json::from_str(include_str!(
            "../../../../../app-server/tests/fixtures/implementation-review/acceptable-as-is.json"
        ))
        .unwrap();
        let manifest = json!({"schema_version":1,"manifest_id":"controlled-initial","roles":{"reviewer":{"contract":"review.md","skills":[{"name":"review","path":"review.md"}],"capabilities":[],"effects":[],"continuity":"ephemeral"}}});
        let options = json!({"base_directory":workspace,"identity_base_directory":workspace,"requirements_base_directory":workspace,"runtime_requirements_by_role":{}});
        let toolchain = Command::new("rustc").arg("-Vv").output().unwrap();
        assert!(toolchain.status.success());
        let source_archive = [
            include_str!("../../src/main.rs"),
            include_str!("../../src/control.rs"),
            include_str!("../../src/worker.rs"),
            include_str!("../../../review-workflow/src/lib.rs"),
            include_str!("../../../review-workflow/src/startup.rs"),
            include_str!("../../../review-workflow/src/initial.rs"),
            include_str!("../../../review-workflow/src/recovery.rs"),
            include_str!("../../../review-workflow/src/protocol.rs"),
        ]
        .join("\n");
        let d = StartupDocument {
            schema_version: 1,
            profile: PROFILE.into(),
            evidence_profile: "controlled-process-v1".into(),
            identity,
            repository: repo,
            workspace,
            accepted_boundary: boundary,
            baseline,
            roots: Roots {
                campaign: base.join("campaign"),
                episode: base.join("episode"),
                claims: base.join("claims"),
                evidence: base.join("evidence"),
            },
            host: ReleasePins {
                executable: pin(&host),
                source_archive: captured(base, "source-archive.txt", source_archive.as_bytes()),
                toolchain: captured(base, "toolchain.txt", &toolchain.stdout),
                lockfile: captured(base, "Cargo.lock", include_bytes!("../../../../Cargo.lock")),
                target: std::str::from_utf8(&toolchain.stdout)
                    .unwrap()
                    .lines()
                    .find_map(|s| s.strip_prefix("host: "))
                    .unwrap()
                    .into(),
                features: if cfg!(feature = "test-faults") {
                    vec!["test-peer".into(), "test-faults".into()]
                } else {
                    vec!["test-peer".into()]
                },
                runtime_dependencies: runtime_pins(),
            },
            gate_capture: captured(
                base,
                "gate-capture.json",
                &fixture("f52792e22d2a1b73e007bc7078607d6657989248a35de1371a1180d23101d206"),
            ),
            gate_manifest: captured(
                base,
                "gate-manifest.json",
                &fixture("d155cc2ac1cf1ab26b0e186d6a6ac86f547b1a0b9c3ce74f0ba69eecd37a2f34"),
            ),
            gate_receipt: captured(
                base,
                "gate-receipt.json",
                &fixture("c927b51b887e37f6fdc02e1fabfadddd211d0ac30712cbc6c5028993ad3889c2"),
            ),
            candidate: captured(base, "candidate.json", &candidate),
            physical_profile: captured(base, "physical.json", &physical),
            selection: data(base, "selection.json", &selection),
            episode_authority: data(base, "authority.json", &authority),
            compiler: CompilerInputs {
                mode: "manifest-only-unverified".into(),
                document: data(base, "manifest.json", &manifest),
                options: data(base, "options.json", &options),
                role: "reviewer".into(),
                requirements: None,
            },
            request_input: data(
                base,
                "request-input.json",
                &json!({"resultTemplate":result,"delayMillis":delay,"mode":mode}),
            ),
            peer: Peer {
                executable: pin(&peer),
                source: captured(base, "peer.rs", include_bytes!("peer.rs")),
                arguments: vec![count.to_str().unwrap().into()],
                session: "h1-session".into(),
                evidence_root_id: "h1-evidence".into(),
            },
            claims: Claims {
                root_id: "h1-claims".into(),
                source: source_pin,
                configuration: data(base, "claims-config.json", &config),
                production_grant_id: "h1-production".into(),
                production_grant_sha256: grant_sha,
            },
            obligation_id: obligation.into(),
            operation_id: "h1-initial".into(),
        };
        let startup = base.join("startup.json");
        let fixture = Self {
            temp,
            startup,
            document: d,
            host,
            count,
        };
        fixture.save();
        fixture
    }
    pub fn save(&self) {
        if self.startup.exists() {
            fs::set_permissions(&self.startup, fs::Permissions::from_mode(0o600)).unwrap();
        }
        fs::write(&self.startup, serde_json::to_vec(&self.document).unwrap()).unwrap();
        fs::set_permissions(&self.startup, fs::Permissions::from_mode(0o400)).unwrap();
        if let Ok(root) = std::env::var("H1_EVIDENCE_ROOT") {
            let artifacts = Path::new(&root).join("artifacts");
            fs::create_dir_all(&artifacts).unwrap();
            let bytes = fs::read(&self.startup).unwrap();
            fs::write(
                artifacts.join(format!("startup-{}.json", sha256_bytes(&bytes))),
                bytes,
            )
            .unwrap();
            let d = &self.document;
            let mut pins = vec![
                &d.host.executable,
                &d.host.source_archive,
                &d.host.toolchain,
                &d.host.lockfile,
                &d.peer.executable,
                &d.peer.source,
                &d.gate_capture,
                &d.gate_manifest,
                &d.gate_receipt,
                &d.candidate,
                &d.physical_profile,
                &d.selection,
                &d.episode_authority,
                &d.compiler.document,
                &d.compiler.options,
                &d.request_input,
                &d.claims.source,
                &d.claims.configuration,
            ];
            pins.extend(d.host.runtime_dependencies.iter());
            if let Some(pin) = &d.compiler.requirements {
                pins.push(pin);
            }
            for pin in pins {
                let destination = artifacts.join(&pin.sha256);
                if !destination.exists() {
                    fs::copy(&pin.path, destination).unwrap();
                }
            }
        }
    }
    pub fn initialize(&self) -> Value {
        let out = Command::new(&self.host)
            .arg("initialize")
            .arg("--startup")
            .arg(&self.startup)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
    pub fn entries(&self) -> usize {
        fs::read_to_string(&self.count).map_or(0, |s| s.lines().count())
    }
    pub fn start(&self, env: &[(&str, &str)]) -> Client {
        let mut c = Command::new(&self.host);
        c.arg("serve")
            .arg("--startup")
            .arg(&self.startup)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        for (k, v) in env {
            c.env(k, v);
        }
        let mut child = c.spawn().unwrap();
        let input = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                tx.send(serde_json::from_str::<Value>(&line).expect("host JSON line"))
                    .ok();
            }
        });
        Client {
            child,
            input: Some(input),
            rx,
        }
    }
}
pub struct Client {
    pub child: Child,
    pub input: Option<std::process::ChildStdin>,
    pub rx: mpsc::Receiver<Value>,
}
impl Client {
    pub fn send(&mut self, v: Value) {
        writeln!(self.input.as_mut().unwrap(), "{v}").unwrap();
    }
    pub fn receive(&self) -> Value {
        self.rx
            // Debug hashing of the pinned host is deliberately unoptimized.
            // The release responsiveness gate still measures every probe independently.
            .recv_timeout(Duration::from_secs(if cfg!(debug_assertions) {
                90
            } else {
                30
            }))
            .expect("bounded host response")
    }
    pub fn ready(&mut self) -> Value {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            self.send(json!({"version":1,"requestId":"status","operation":"status"}));
            let v = self.receive();
            assert!(
                v["result"]["lastObservation"]["startupError"].is_null(),
                "{v}"
            );
            if v["result"]["busy"] == false {
                return v;
            }
            assert!(Instant::now() < deadline, "host startup timed out");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    pub fn domain(&mut self, operation: &str, revision: &str) {
        self.send(json!({"version":1,"requestId":operation,"operation":operation,"obligationId":"review-0","operationId":"h1-initial","expectedRevision":revision}));
    }
    pub fn finish(&mut self) {
        self.input.take();
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(s) = self.child.try_wait().unwrap() {
                assert!(s.success());
                return;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
