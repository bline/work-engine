//! Contract tests for the R4 evidence qualifier.
//!
//! The positive baseline is complete, owner-produced evidence: the canonical binding, the real
//! episode reducer and the unchanged fixture consumer produce the states and consumption, the
//! delivery adapter's pure verifier accepts the native observation, and the bundle is written by
//! the delivery crate's own capture writer (`tests/common`, included by path). Custody records
//! follow custody.py's schema for the same 18 step identities as the campaign map (the 16 product steps and
//! the qualifier's own build and direct test, one inventory under R5). Every negative
//! fixture keeps surrounding hashes and lengths consistent, so a refusal is semantic, never an
//! incidental stale digest.
#[rustfmt::skip]
#[path = "/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery/tests/common/mod.rs"]
mod common;

use common::capture::{self, TRANSITION_ID};
use common::run_evidence::{Evidence, RunEnv};
use common::world::world as fixture_world;
use common::{authority, begun, canonical, request, result_json, AUTHORITY, REQ};
use review_episode_core::codec::parse_json;
use review_episode_core::command::BeginCommand;
use review_episode_core::identity::Authority;
use review_episode_core::reducer::begin;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use we_evidence_gate::ledger::Defects;
use we_evidence_gate::spec::{derive_checks, validate, REQUIRED_ARTIFACTS};
use we_evidence_gate::*;
use work_engine_review_delivery::binding::{EpisodeInputs, ExperimentBinding};
use work_engine_review_delivery::native::{
    check_attempt_evidence, AttemptFacts, DeliverError, Delivered, KillFact, NativeError,
    NativeRun, ProcessFacts, ReapFact, SendOutcome, SendRecord, StreamEnd, StreamRecord,
    TimingFacts,
};
use work_engine_review_delivery::DeliveryRefusal;
use work_engine_review_delivery::{
    Allocation, BoundAttempt, Counters, NativeObservation, Transport,
};
use work_engine_review_preparation::fixture::consume_fixture_result;

type MapEdit = Box<dyn Fn(&mut Value)>;

const T0: u64 = 1_794_564_000; // 2026-11-13T10:00:00Z
const STEPS: [&str; 18] = [
    "toolchain-rustc",
    "toolchain-cargo",
    "node-version",
    "metadata",
    "sdk-verify",
    "fmt",
    "clippy-default",
    "clippy-fixture",
    "build",
    "test-pure",
    "test-binding",
    "test-native",
    "doctest-fixture",
    "build-default",
    "test-pure-default",
    "doctest-default",
    "qualifier-build",
    "qualifier-test",
];
const QUALIFIER_STEPS: [&str; 2] = ["qualifier-build", "qualifier-test"];
const PRODUCER_INDEX: u64 = 11;

struct W {
    dir: PathBuf,
    root: PathBuf,
    map_path: PathBuf,
    plan_path: PathBuf,
    manifest_path: PathBuf,
    run: RunVars,
}

impl W {
    fn records(&self) -> PathBuf {
        PathBuf::from(&self.run.records_dir)
    }
    fn evidence(&self) -> PathBuf {
        PathBuf::from(&self.run.run_dir).join("evidence")
    }
    fn run_env(&self) -> RunEnv {
        RunEnv {
            run_dir: PathBuf::from(&self.run.run_dir),
            invocation_id: self.run.invocation_id.clone(),
            subject_manifest_sha256: self.run.manifest_sha256.clone(),
            map_sha256: self.run.map_sha256.clone(),
        }
    }
}

fn put(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn iso(secs: u64) -> String {
    format!(
        "2026-11-13T10:{:02}:{:02}.000000Z",
        (secs - T0) / 60,
        (secs - T0) % 60
    )
}

fn set_mtime(path: &Path, secs: u64) {
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(secs))
        .unwrap();
}

fn process(step: &str, root: &Path, argv: Value, env: Value, inputs: Value) -> Value {
    json!({"step": step, "argv": argv, "cwd": root.display().to_string(), "env": env, "timeout": 60, "inputs": inputs})
}

fn base_map(dir: &Path, root: &Path) -> Value {
    let env = json!({"PATH": "/usr/bin"});
    let native_env = json!({"PATH": "/usr/bin", "WE_RUN_DIR": "{RUN_DIR}", "WE_INVOCATION_ID": "{INVOCATION_ID}", "WE_MAP_SHA256": "{MAP_SHA256}", "WE_SOURCE_MANIFEST_SHA256": "{SOURCE_MANIFEST_SHA256}"});
    let a = root.join("crates/a").display().to_string();
    let node = dir.join("node").display().to_string();
    let gate = dir.join("gate").display().to_string();
    let cargo = |args: &[&str]| {
        json!(["/usr/bin/cargo"]
            .iter()
            .chain(args.iter())
            .collect::<Vec<_>>())
    };
    let mut ps = vec![
        process(
            "toolchain-rustc",
            root,
            json!(["/usr/bin/rustc", "-vV"]),
            env.clone(),
            json!(["toolchain"]),
        ),
        process(
            "toolchain-cargo",
            root,
            json!(["/usr/bin/cargo", "-vV"]),
            env.clone(),
            json!(["toolchain"]),
        ),
        process(
            "node-version",
            root,
            json!([node, "--version"]),
            env.clone(),
            json!(["node"]),
        ),
        process(
            "metadata",
            root,
            cargo(&["metadata"]),
            env.clone(),
            json!(["@closure", "toolchain"]),
        ),
        process(
            "sdk-verify",
            root,
            json!([
                gate,
                "verify-sdk",
                "--out",
                "{RECORDS_DIR}/sdk-verification.json"
            ]),
            env.clone(),
            json!(["gate"]),
        ),
        process(
            "fmt",
            root,
            cargo(&["fmt", "--check"]),
            env.clone(),
            json!(["@closure", "toolchain"]),
        ),
        process(
            "clippy-default",
            root,
            cargo(&["clippy"]),
            env.clone(),
            json!(["@closure", "toolchain"]),
        ),
        process(
            "clippy-fixture",
            root,
            cargo(&["clippy", "--features", "fixture"]),
            env.clone(),
            json!(["@closure", "toolchain"]),
        ),
        process(
            "build",
            root,
            cargo(&["test", "--no-run", "--message-format=json"]),
            env.clone(),
            json!(["@closure", "toolchain"]),
        ),
        process(
            "test-pure",
            root,
            json!(["{exe:build:pure}"]),
            env.clone(),
            json!(["@closure"]),
        ),
        process(
            "test-binding",
            root,
            json!(["{exe:build:binding}"]),
            env.clone(),
            json!(["@closure"]),
        ),
        process(
            "test-native",
            root,
            json!(["{exe:build:native_vertical}"]),
            native_env,
            json!(["@closure", "native"]),
        ),
        process(
            "doctest-fixture",
            root,
            cargo(&["test", "--doc", "--features", "fixture"]),
            env.clone(),
            json!(["@closure", "toolchain"]),
        ),
        process(
            "build-default",
            root,
            cargo(&["test", "--no-run", "--message-format=json", "-p", "default"]),
            env.clone(),
            json!(["@closure", "toolchain"]),
        ),
        process(
            "test-pure-default",
            root,
            json!(["{exe:build-default:pure}"]),
            env.clone(),
            json!(["@closure"]),
        ),
        process(
            "doctest-default",
            root,
            cargo(&["test", "--doc"]),
            env.clone(),
            json!(["@closure", "toolchain"]),
        ),
    ];
    ps[8]["cargo_json"] = json!(true);
    ps[8]["exe_targets"] = json!(["pure", "binding", "native_vertical"]);
    ps[13]["cargo_json"] = json!(true);
    ps[13]["exe_targets"] = json!(["pure"]);
    for (i, target, build) in [
        (9usize, "pure", "build"),
        (10, "binding", "build"),
        (11, "native_vertical", "build"),
        (14, "pure", "build-default"),
    ] {
        ps[i]["exe"] = json!({"step": build, "target": target});
        ps[i]["min_tests"] = json!(1);
    }
    ps[12]["min_tests"] = json!(1);
    ps[15]["min_tests"] = json!(1);
    // R5: the qualifier's own build and direct test are two more steps of the same inventory.
    ps.push(process(
        "qualifier-build",
        root,
        cargo(&[
            "test",
            "--test",
            "gate",
            "--no-run",
            "--message-format=json",
        ]),
        env.clone(),
        json!(["@closure", "toolchain"]),
    ));
    ps.push(process(
        "qualifier-test",
        root,
        json!(["{exe:qualifier-build:gate}"]),
        env.clone(),
        json!(["@closure"]),
    ));
    ps[16]["cargo_json"] = json!(true);
    ps[16]["exe_targets"] = json!(["gate"]);
    ps[17]["exe"] = json!({"step": "qualifier-build", "target": "gate"});
    ps[17]["min_tests"] = json!(1);
    let preserved = json!([
        "episode-before.json",
        "contract.bytes",
        "native-frames-received.jsonl",
        "native-frames-sent.jsonl",
        "observation.json",
        "request-context.json",
        "native-history.json",
        "episode-after.json",
        "consumption.json",
        "revisions.json",
        "process.json",
        "collection.json",
        "capture.json",
        "failure.json"
    ]);
    let mut map = json!({
        "schema": MAP_SCHEMA, "map_id": "test", "authority": "test", "repo_root": root.display().to_string(),
        "required_roots": [a], "required_files": [root.join("Cargo.toml"), root.join("Cargo.lock")],
        "excluded_dir_names": ["target"], "absent_expected": [root.join(".cargo/config.toml")],
        "external_inputs": {node.clone(): sha256_hex(b"node-bytes"), gate.clone(): sha256_hex(b"gate-bytes"), dir.join("rustc").display().to_string(): sha256_hex(b"rustc-bytes")},
        "classes": {"native": [node.clone(), "{RECORDS_DIR}/sdk-verification.json"], "node": [node], "gate": [gate], "toolchain": [dir.join("rustc").display().to_string()]},
        "run_outputs": ["{RECORDS_DIR}/sdk-verification.json"],
        "processes": ps,
        "output_producer": {"step": "test-native"},
        "artifacts": REQUIRED_ARTIFACTS.iter().map(|(n, r)| json!({"name": n, "representation": match r {
            spec::Repr::Json => "json", spec::Repr::Jsonl => "jsonl", spec::Repr::Raw => "raw", spec::Repr::Utf8 => "utf8", spec::Repr::Text => "text", spec::Repr::CanonicalState => "canonical_state" }})).collect::<Vec<_>>(),
        "expected": {"closure_package": "a", "closure_step": "metadata", "sdk_step": "sdk-verify", "sdk_manifest_sha256": "c".repeat(64), "sdk_files": 3, "sdk_links": 0,
            "sdk_version": "1.5.0", "node_version": "v22.23.2",
            "stdout_checks": [{"step": "toolchain-rustc", "contains": "release: 1.92.0"}, {"step": "node-version", "equals": "v22.23.2"}]},
        "fixture_sources": {"candidate-requirements.json": root.join("crates/a/fixtures/requirements.json"), "authority.json": root.join("crates/a/fixtures/episode-authority.json")},
        "controls": [
            {"name": "assertion-unwind", "scratch": "control-assertion-unwind", "outcome": "failed", "preserved": preserved},
            {"name": "write-failure", "scratch": "control-write-failure", "outcome": "incomplete", "preserved": ["observation.json", "native-frames-received.jsonl", "consumption.json", "capture.json"], "write_failure": "episode-after.json"},
            {"name": "delivery-refusal", "scratch": "control-delivery-refusal", "outcome": "passed", "preserved": ["native-frames-received.jsonl", "process.json", "collection.json", "capture.json"], "stage": "delivery_refused"},
            {"name": "setup-refusal", "scratch": "control-setup-refusal", "outcome": "passed", "preserved": ["consumer-inputs.json", "capture.json", "identities.json"], "stage": "native_error"},
            {"name": "consumer-refusal", "scratch": "control-consumer-refusal", "outcome": "passed", "preserved": ["native-frames-received.jsonl", "observation.json", "process.json", "collection.json", "capture.json"], "stage": "consumer_refused"},
        ],
        "accepted_nonliteral_include_files": [],
        "checks": [],
    });
    for pin in map["external_inputs"].as_object_mut().unwrap().values_mut() {
        *pin = json!({"kind":"file","sha256":pin.clone()});
    }
    map["checks"] = derive_checks(&map).unwrap();
    map
}

fn world_with(name: &str, edit: impl FnOnce(&mut Value)) -> W {
    // The qualifier-test step's environment names an owned work directory; the default is only for
    // exploratory runs under cargo.
    let base = std::env::var_os("WE_GATE_TEST_WORK")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-work"));
    let dir = base.join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let root = dir.join("src-tree");
    put(
        &root.join("crates/a/Cargo.toml"),
        b"[package]\nname=\"a\"\n",
    );
    put(
        &root.join("crates/a/src/lib.rs"),
        b"pub const F: &str = include_str!(\"../fixtures/f.txt\");\n",
    );
    put(&root.join("crates/a/fixtures/f.txt"), b"fixture bytes");
    put(&root.join("crates/a/fixtures/requirements.json"), REQ);
    put(
        &root.join("crates/a/fixtures/episode-authority.json"),
        AUTHORITY.as_bytes(),
    );
    put(&root.join("crates/a/tests/t.rs"), b"#[test] fn t() {}\n");
    put(&root.join("Cargo.toml"), b"[workspace]\n");
    put(&root.join("Cargo.lock"), b"# lock\n");
    put(&dir.join("node"), b"node-bytes");
    put(&dir.join("gate"), b"gate-bytes");
    put(&dir.join("rustc"), b"rustc-bytes");
    let mut map = base_map(&dir, &root);
    edit(&mut map);
    let map_path = dir.join("map.json");
    put(
        &map_path,
        serde_json::to_string_pretty(&map).unwrap().as_bytes(),
    );
    let map_sha = file_sha(&map_path).unwrap();
    let manifest = build_manifest(&map, &map_sha).unwrap();
    let manifest_path = dir.join("manifest.json");
    put(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap().as_bytes(),
    );
    let run = RunVars {
        invocation_id: "inv-1".into(),
        run_dir: dir.join("runs/r1/run").display().to_string(),
        records_dir: dir.join("runs/r1/custody").display().to_string(),
        map_sha256: map_sha,
        manifest_sha256: file_sha(&manifest_path).unwrap(),
    };
    let plan = make_plan(&map, &run).unwrap();
    let plan_path = dir.join("plan.json");
    put(
        &plan_path,
        serde_json::to_string_pretty(&plan).unwrap().as_bytes(),
    );
    W {
        dir,
        root,
        map_path,
        plan_path,
        manifest_path,
        run,
    }
}

fn inputs<'a>(w: &'a W, p: &'a (String, String, String)) -> Inputs<'a> {
    Inputs {
        map_path: &w.map_path,
        plan_path: &w.plan_path,
        manifest_path: &w.manifest_path,
        expect_map_sha256: &p.0,
        expect_plan_sha256: &p.1,
        expect_manifest_sha256: &p.2,
        run: w.run.clone(),
    }
}

fn pins(w: &W) -> (String, String, String) {
    (
        file_sha(&w.map_path).unwrap(),
        file_sha(&w.plan_path).unwrap(),
        file_sha(&w.manifest_path).unwrap(),
    )
}

fn post(w: &W) -> Evaluation {
    let p = pins(w);
    postflight(&inputs(w, &p), &w.records())
}

fn pre(w: &W) -> Evaluation {
    let p = pins(w);
    preflight(&inputs(w, &p))
}

fn entry(path: &str) -> Value {
    let binding = we_evidence_gate::runtime::ObjectBinding::observe(Path::new(path)).unwrap();
    binding.custody(path)
}

fn step_stdout(w: &W, name: &str, exes: &[(String, String)]) -> String {
    match name {
        "toolchain-rustc" => "rustc 1.92.0\nrelease: 1.92.0\n".into(),
        "node-version" => "v22.23.2\n".into(),
        "metadata" => {
            let a = w.root.join("crates/a/Cargo.toml").display().to_string();
            json!({"packages": [{"name": "a", "id": "a-id", "source": null, "manifest_path": a}], "resolve": {"nodes": [{"id": "a-id", "deps": []}]}}).to_string()
        }
        "build" | "build-default" | "qualifier-build" => {
            let mut out = String::from("{\"reason\":\"build-script-executed\"}\n");
            for (target, path) in exes {
                out += &format!("{}\n", json!({"reason": "compiler-artifact", "executable": path, "target": {"name": target, "kind": ["test"]}, "profile": {"test": true}}));
            }
            out += "{\"reason\":\"build-finished\",\"success\":true}\n";
            out
        }
        "test-pure" | "test-binding" | "test-native" | "test-pure-default" | "doctest-fixture" | "doctest-default" | "qualifier-test" => {
            "running 1 test\ntest t ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n\n".into()
        }
        _ => String::new(),
    }
}

/// Models custody.py: writes the same record/summary shapes for the plan's steps.
fn execute(w: &W) {
    let records = w.records();
    fs::create_dir_all(&records).unwrap();
    let plan: Value = serde_json::from_slice(&fs::read(&w.plan_path).unwrap()).unwrap();
    let mut exes: Vec<(String, String, String)> = Vec::new(); // (build, target, path)
    for (build, targets) in [
        ("build", vec!["pure", "binding", "native_vertical"]),
        ("build-default", vec!["pure"]),
        ("qualifier-build", vec!["gate"]),
    ] {
        for t in targets {
            let p = w.dir.join(format!("exe/{build}/{t}"));
            put(&p, format!("{build}:{t}:bytes").as_bytes());
            exes.push((build.into(), t.into(), p.display().to_string()));
        }
    }
    let resolve = |text: &str| -> String {
        let mut out = text.to_owned();
        for (b, t, p) in &exes {
            out = out.replace(&format!("{{exe:{b}:{t}}}"), p);
        }
        out
    };
    let mut rows = vec![];
    for (n, step) in plan["steps"].as_array().unwrap().iter().enumerate() {
        let name = step["name"].as_str().unwrap();
        if name == "sdk-verify" {
            let v = json!({"schema": "org-sdk-verification-gate/v1", "root": "/sdk", "manifest": "/sdk.json", "manifest_sha256": "c".repeat(64), "counts": {"file": 3, "symlink": 0}, "errors": [], "exact_manifest_entry_set": true});
            put(
                &records.join("sdk-verification.json"),
                serde_json::to_string_pretty(&v).unwrap().as_bytes(),
            );
        }
        let mine: Vec<(String, String)> = exes
            .iter()
            .filter(|e| e.0 == name)
            .map(|e| (e.1.clone(), e.2.clone()))
            .collect();
        let out = step_stdout(w, name, &mine);
        let (so, se) = (
            records.join(format!("{name}.stdout")),
            records.join(format!("{name}.stderr")),
        );
        put(&so, out.as_bytes());
        put(&se, b"permitted non-evidence text\n");
        let argv: Vec<String> = step["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| resolve(a.as_str().unwrap()))
            .collect();
        let digests: Vec<String> = step["digest"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| resolve(d.as_str().unwrap()))
            .collect();
        let start = T0 + 3 * n as u64;
        let mut rec = json!({
            "schema": RUN_SCHEMA, "name": name, "argv": argv, "cwd": step["cwd"], "env_clear": true, "env_set": step["env"], "env_inherited": false,
            "wrapper_pid": 1, "timeout_s": 60, "pid": 100 + n, "pgid": 100 + n, "started_at": iso(start), "ended_at": iso(start + 2), "duration_s": 2.0,
            "returncode": 0, "signal": null, "timed_out": false,
            "stdout": {"path": so.display().to_string(), "length": out.len(), "sha256": sha256_hex(out.as_bytes())},
            "stderr": {"path": se.display().to_string(), "length": 28, "sha256": sha256_hex(b"permitted non-evidence text\n")},
            "leftover_processes": [], "leftovers_killed": false, "leftovers_after_kill": null,
            "inputs": digests.iter().map(|d| entry(d)).collect::<Vec<_>>(),
        });
        if !mine.is_empty() {
            rec["cargo_executables"] = json!(mine.iter().map(|(t, p)| json!({"name": t, "kind": ["test"], "path": p, "sha256": sha256_hex(&fs::read(p).unwrap())})).collect::<Vec<_>>());
        }
        let path = records.join(format!("{name}.json"));
        let text = serde_json::to_string_pretty(&rec).unwrap();
        put(&path, text.as_bytes());
        rows.push(json!({"name": name, "outcome": "0", "record": path.display().to_string(), "record_sha256": sha256_hex(text.as_bytes()), "leftovers": 0}));
    }
    let summary = json!({"schema": SUMMARY_SCHEMA, "plan": w.plan_path.display().to_string(), "plan_sha256": file_sha(&w.plan_path).unwrap(), "status": "passed", "steps": rows, "not_run": []});
    put(
        &records.join("summary.json"),
        serde_json::to_string_pretty(&summary).unwrap().as_bytes(),
    );
}

struct Fixture {
    binding: ExperimentBinding,
    delivered: Delivered,
}

fn binding_from(
    state: review_episode_core::state::ReviewEpisodeState,
    result: &str,
) -> ExperimentBinding {
    let w = fixture_world();
    ExperimentBinding::new(
        &w.candidate,
        &request(None),
        &w.declaration,
        EpisodeInputs {
            expected_revision: state.revision().clone(),
            state,
            authority: authority(),
            transition_id: TRANSITION_ID.to_owned(),
            unresolved_questions: vec![],
            admissions_json: serde_json::to_string(&w.admissions).unwrap(),
        },
        result,
        Transport {
            attempt_id: "attempt-1".into(),
            generation: 1,
        },
    )
    .unwrap()
}

fn system_message() -> Value {
    json!({"role": "system", "content": "sys", "sections": {"preamble": "PRE"}, "timestamp": 1})
}

fn frames(
    binding: &ExperimentBinding,
    alloc: &Allocation,
    obs: &NativeObservation,
    result: &str,
) -> (Vec<String>, Vec<String>) {
    let mut a = serde_json::to_value(alloc).unwrap();
    a["type"] = json!("allocated");
    let mut o = serde_json::to_value(obs).unwrap();
    o["type"] = json!("observation");
    let f = binding.frozen();
    let start = json!({"type": "start", "session_id": alloc.session_id, "attempt_id": f.transport().attempt_id, "generation": f.transport().generation,
        "contract": f.contract(), "contract_sha256": f.contract_sha256(), "result_text": result});
    (vec![a.to_string(), o.to_string()], vec![start.to_string()])
}

/// Owner-produced native evidence: the adapter's verifier and the existing consumer accept it.
fn owner_fixture() -> Fixture {
    let result = result_json(|_| {});
    let binding = binding_from(begun(), &result);
    let f = binding.frozen();
    let alloc = Allocation {
        session_id: "sess-1".into(),
        sdk_version: "1.5.0".into(),
        node_version: "v22.23.2".into(),
        model: json!({"provider": "none"}),
        system_prompt: "sys".into(),
        active_tools: vec![],
        loader: json!({"kind": "inert"}),
        preamble: "PRE".into(),
    };
    let bound = BoundAttempt::bind(f.clone(), alloc.clone()).unwrap();
    let user = json!({"role": "user", "content": [{"type": "text", "text": f.contract()}], "timestamp": 2});
    let assistant = json!({"role": "assistant", "content": [{"type": "text", "text": result}], "stopReason": "stop", "timestamp": 3});
    let obs = NativeObservation {
        session_id: "sess-1".into(),
        attempt_id: "attempt-1".into(),
        generation: 1,
        request_count: 1,
        context: Some(json!({"messages": [system_message(), user.clone()], "tools": []})),
        history: vec![system_message(), user, assistant],
        active_tools: vec![],
        terminal_idle: true,
        counters: Counters::default(),
        prompt_error: None,
        system_prompt: "sys".into(),
        loader_calls: json!({"getSystemPrompt": 1}),
    };
    let verified = bound.verify(&obs, &result).unwrap();
    let w = fixture_world();
    let consumption = consume_fixture_result(
        &w.candidate,
        &w.declaration,
        binding.episode_state(),
        &authority(),
        binding.expected_revision().clone(),
        TRANSITION_ID,
        &result,
        &[],
        binding.admissions_json(),
    )
    .unwrap();
    let (frames, sent_frames) = frames(&binding, &alloc, &obs, &result);
    let run = synthetic_run(
        Some(bound),
        Some(obs),
        format!("{}\n", frames.join("\n")).into_bytes(),
        &sent_frames,
        ReapFact::Exited { code: 0 },
    );
    Fixture {
        binding,
        delivered: Delivered {
            verified,
            consumption,
            run,
        },
    }
}

/// SYNTHETIC (test-local) native run with consistent delivery-owned facts; the modelled consumer
/// invocation count is supplied explicitly at capture, never through a product counter.
fn synthetic_run(
    bound: Option<BoundAttempt>,
    observation: Option<NativeObservation>,
    raw_stdout: Vec<u8>,
    sent_frames: &[String],
    reap: ReapFact,
) -> NativeRun {
    let sent_bytes: Vec<u8> = sent_frames
        .iter()
        .flat_map(|t| format!("{t}\n").into_bytes())
        .collect();
    let sent = sent_frames
        .iter()
        .map(|t| {
            let b = format!("{t}\n");
            SendRecord {
                frame_sha256: sha256_hex(b.as_bytes()),
                frame_len: b.len() as u64,
                written: b.len() as u64,
                outcome: SendOutcome::Completed,
            }
        })
        .collect();
    let facts = AttemptFacts {
        sent,
        stdout: StreamRecord {
            end: StreamEnd::Eof,
            retained_bytes: raw_stdout.len() as u64,
            overflow_bytes_observed: 0,
            limit_bytes: 8 << 20,
            frame_limit_bytes: Some(1 << 20),
        },
        stderr: StreamRecord {
            end: StreamEnd::Eof,
            retained_bytes: 0,
            overflow_bytes_observed: 0,
            limit_bytes: 64 << 10,
            frame_limit_bytes: None,
        },
        process: ProcessFacts {
            pid: 4242,
            kill: KillFact::NotSent,
            reap,
            timed_out: false,
            descendants_seen: vec![],
            pid_present_after_reap: Some(false),
            wait_errors: vec![],
        },
        timing: TimingFacts {
            attempt_ms: 60_000,
            cleanup_ms: 5_000,
            elapsed_ms: 10,
        },
    };
    NativeRun {
        bound,
        observation,
        error_frame: None,
        raw_stdout,
        raw_stderr: vec![],
        sent_bytes,
        facts,
    }
}

/// Recomputes a bundle's collection projections (stdout/stderr/sent records, frame reading and
/// the delivery verdict) from its retained bytes, so a test edit stays internally consistent.
fn resync_collection(dir: &Path) {
    let raw = fs::read(dir.join("native-frames-received.jsonl")).unwrap();
    let sent = fs::read(dir.join("native-frames-sent.jsonl")).unwrap();
    let stderr = fs::read(dir.join("stderr.txt")).unwrap();
    let mut c: Value =
        serde_json::from_slice(&fs::read(dir.join("collection.json")).unwrap()).unwrap();
    let process: ProcessFacts =
        serde_json::from_slice(&fs::read(dir.join("process.json")).unwrap()).unwrap();
    c["stdout"]["retained_bytes"] = json!(raw.len());
    c["sent"][0]["frame_sha256"] = json!(sha256_hex(&sent));
    c["sent"][0]["frame_len"] = json!(sent.len());
    c["sent"][0]["written"] = json!(sent.len());
    let facts = AttemptFacts {
        sent: serde_json::from_value(c["sent"].clone()).unwrap(),
        stdout: serde_json::from_value(c["stdout"].clone()).unwrap(),
        stderr: serde_json::from_value(c["stderr"].clone()).unwrap(),
        process,
        timing: serde_json::from_value(c["timing"].clone()).unwrap(),
    };
    let i = work_engine_review_delivery::interpret_stream(&raw);
    c["frame_kinds"] = json!(i.kinds);
    c["frame_defects"] = json!(i.defects);
    c["incompleteness"] = json!(check_attempt_evidence(&facts, &raw, &stderr, &sent).unwrap());
    rewrite_bundle_file(
        dir,
        "collection.json",
        &serde_json::to_vec_pretty(&c).unwrap(),
    );
}

/// Rewrites one file of a bundle directory and keeps its output manifest's hash/length exact.
fn rewrite_bundle_file(dir: &Path, name: &str, bytes: &[u8]) {
    fs::write(dir.join(name), bytes).unwrap();
    let mp = dir.join("output-manifest.json");
    let mut m: Value = serde_json::from_slice(&fs::read(&mp).unwrap()).unwrap();
    for f in m["files"].as_array_mut().unwrap() {
        if f["name"] == name {
            f["length"] = json!(bytes.len());
            f["sha256"] = json!(sha256_hex(bytes));
        }
    }
    fs::write(&mp, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    touch_all(dir);
}

/// Like `resync_collection`, but leaves the collection untouched when the process facts no longer
/// decode (the point of the edit under test).
fn resync_collection_tolerant(dir: &Path) {
    if serde_json::from_slice::<ProcessFacts>(&fs::read(dir.join("process.json")).unwrap()).is_ok()
    {
        resync_collection(dir);
    }
}

fn touch_all(dir: &Path) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        if e.path().is_file() {
            set_mtime(&e.path(), T0 + 3 * PRODUCER_INDEX + 1);
        }
    }
}

/// Writes the positive bundle and both control bundles through the delivery crate's writer.
fn write_outputs(w: &W) {
    let env = w.run_env();
    let fx = owner_fixture();
    let outcome = Ok(fx.delivered);
    let mut ev = Evidence::create(&env).unwrap();
    capture::persist_inputs(&mut ev, &fx.binding, AUTHORITY, &[]);
    capture::capture_returned(&mut ev, &fx.binding, &outcome, 1); // modelled count: synthetic bundle
    assert_eq!(ev.finish("passed").unwrap(), "passed");
    touch_all(&w.evidence());

    let controls = PathBuf::from(&w.run.run_dir).join("controls");
    let scratch = PathBuf::from(&w.run.run_dir).join("scratch");
    let fx = owner_fixture();
    let outcome = Ok(fx.delivered);
    let mut ev = Evidence::create_at(&env, controls.join("assertion-unwind")).unwrap();
    capture::persist_inputs(&mut ev, &fx.binding, AUTHORITY, &[]);
    capture::capture_returned(&mut ev, &fx.binding, &outcome, 1); // modelled count: synthetic bundle
    let sdir = scratch.join("control-assertion-unwind-77");
    put(&sdir.join("home/x"), b"scratch");
    ev.put_json("failure.json", &json!({"stage": "assertion_after_capture", "message": "injected assertion failure after Delivered was available", "scratch": sdir, "scratch_listing_before_cleanup": [{"path": "home/x", "length": 7}], "scratch_retained": true}));
    assert_eq!(ev.finish("failed").unwrap(), "failed");

    let fx = owner_fixture();
    let outcome = Ok(fx.delivered);
    let cdir = controls.join("write-failure");
    let mut ev = Evidence::create_at(&env, cdir.clone()).unwrap();
    fs::create_dir(cdir.join("episode-after.json")).unwrap();
    capture::persist_inputs(&mut ev, &fx.binding, AUTHORITY, &[]);
    capture::capture_returned(&mut ev, &fx.binding, &outcome, 1); // modelled count: synthetic bundle
    put(&scratch.join("control-write-failure-77/home/x"), b"scratch");
    ev.put_json("failure.json", &json!({"stage": "evidence_write_after_capture", "message": "episode-after.json create_new failed", "scratch": scratch.join("control-write-failure-77"), "scratch_listing_before_cleanup": [{"path": "home/x", "length": 7}], "scratch_retained": true}));
    assert_eq!(ev.finish("passed").unwrap(), "incomplete");

    // SYNTHETIC refusal-scenario bundles (the real ones are produced by the native vertical).
    // Delivery refusal: a valid protocol whose child exited 5, so the owner refuses before consuming.
    let fx = owner_fixture();
    let run = fx.delivered.run;
    let mut run = synthetic_run(
        run.bound,
        run.observation,
        run.raw_stdout,
        &[String::from_utf8(run.sent_bytes[..run.sent_bytes.len() - 1].to_vec()).unwrap()],
        ReapFact::Exited { code: 5 },
    );
    run.error_frame = None;
    let why = run.incompleteness().join("; ");
    let outcome: Result<Delivered, DeliverError> = Err(DeliverError::Delivery {
        refusal: DeliveryRefusal::Incomplete(why),
        run: Box::new(run),
    });
    let mut ev = Evidence::create_at(&env, controls.join("delivery-refusal")).unwrap();
    capture::persist_inputs(&mut ev, &fx.binding, AUTHORITY, &[]);
    capture::capture_returned(&mut ev, &fx.binding, &outcome, 0);
    put(
        &scratch.join("control-delivery-refusal-77/home/x"),
        b"scratch",
    );
    assert_eq!(ev.finish("passed").unwrap(), "passed");
    // Setup/spawn refusal: no child existed.
    let fx = owner_fixture();
    let outcome: Result<Delivered, DeliverError> = Err(DeliverError::Native(NativeError::Spawn(
        "No such file or directory (os error 2)".into(),
    )));
    let mut ev = Evidence::create_at(&env, controls.join("setup-refusal")).unwrap();
    capture::persist_inputs(&mut ev, &fx.binding, AUTHORITY, &[]);
    capture::capture_returned(&mut ev, &fx.binding, &outcome, 0);
    put(&scratch.join("control-setup-refusal-77/home/x"), b"scratch");
    assert_eq!(ev.finish("passed").unwrap(), "passed");
    // Consumer refusal: correct delivery; the owner refuses a stale expected revision.
    let fx = owner_fixture();
    let mut stale = binding_from(begun(), &result_json(|_| {}));
    let w0 = fixture_world();
    let state = begun();
    stale = {
        let _ = stale;
        ExperimentBinding::new(
            &w0.candidate,
            &request(None),
            &w0.declaration,
            EpisodeInputs {
                expected_revision: review_episode_core::identity::Revision("f".repeat(64)),
                state,
                authority: authority(),
                transition_id: TRANSITION_ID.to_owned(),
                unresolved_questions: vec![],
                admissions_json: serde_json::to_string(&w0.admissions).unwrap(),
            },
            &result_json(|_| {}),
            Transport {
                attempt_id: "attempt-1".into(),
                generation: 1,
            },
        )
        .unwrap()
    };
    let refusal = consume_fixture_result(
        &w0.candidate,
        &w0.declaration,
        stale.episode_state(),
        &authority(),
        stale.expected_revision().clone(),
        TRANSITION_ID,
        &result_json(|_| {}),
        &[],
        stale.admissions_json(),
    )
    .unwrap_err();
    let Delivered { verified, run, .. } = fx.delivered;
    let outcome: Result<Delivered, DeliverError> = Err(DeliverError::Consumer {
        refusal,
        verified: Box::new(verified),
        run: Box::new(run),
    });
    let mut ev = Evidence::create_at(&env, controls.join("consumer-refusal")).unwrap();
    capture::persist_inputs(&mut ev, &stale, AUTHORITY, &[]);
    capture::capture_returned(&mut ev, &stale, &outcome, 1); // modelled count: synthetic bundle
    put(
        &scratch.join("control-consumer-refusal-77/home/x"),
        b"scratch",
    );
    assert_eq!(ev.finish("passed").unwrap(), "passed");
    for c in ["delivery-refusal", "setup-refusal", "consumer-refusal"] {
        touch_all(&controls.join(c));
    }
}

fn full(name: &str) -> W {
    full_edit(name, |_| {})
}

fn full_edit(name: &str, edit: impl FnOnce(&mut Value)) -> W {
    let w = world_with(name, edit);
    let p = pre(&w);
    assert!(
        p.is_qualified(),
        "preflight on a fresh world: {}",
        p.to_json("preflight")
    );
    execute(&w);
    write_outputs(&w);
    w
}

/// Rewrites one retained output and keeps the output manifest's hash/length exact.
fn rewrite_output(w: &W, name: &str, bytes: &[u8]) {
    let dir = w.evidence();
    fs::write(dir.join(name), bytes).unwrap();
    let mp = dir.join("output-manifest.json");
    let mut m: Value = serde_json::from_slice(&fs::read(&mp).unwrap()).unwrap();
    for f in m["files"].as_array_mut().unwrap() {
        if f["name"] == name {
            f["length"] = json!(bytes.len());
            f["sha256"] = json!(sha256_hex(bytes));
        }
    }
    fs::write(&mp, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    touch_all(&dir);
}

fn read_out(w: &W, name: &str) -> Vec<u8> {
    fs::read(w.evidence().join(name)).unwrap()
}

fn edit_json_output(w: &W, name: &str, f: impl FnOnce(&mut Value)) {
    let mut v: Value = serde_json::from_slice(&read_out(w, name)).unwrap();
    f(&mut v);
    rewrite_output(w, name, &serde_json::to_vec_pretty(&v).unwrap());
}

fn edit_lines(w: &W, name: &str, f: impl FnOnce(&mut Vec<String>)) {
    let text = String::from_utf8(read_out(w, name)).unwrap();
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    f(&mut lines);
    rewrite_output(w, name, lines.join("\n").as_bytes());
}

/// Rewrites one custody record and its summary row so the defect is the only difference.
fn rewrite_record(w: &W, step: &str, bytes: &[u8]) {
    let records = w.records();
    fs::write(records.join(format!("{step}.json")), bytes).unwrap();
    let sp = records.join("summary.json");
    let mut s: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    for row in s["steps"].as_array_mut().unwrap() {
        if row["name"] == step {
            row["record_sha256"] = json!(sha256_hex(bytes));
        }
    }
    fs::write(&sp, serde_json::to_vec_pretty(&s).unwrap()).unwrap();
}

fn edit_record(w: &W, step: &str, f: impl FnOnce(&mut Value)) {
    let mut v: Value =
        serde_json::from_slice(&fs::read(w.records().join(format!("{step}.json"))).unwrap())
            .unwrap();
    f(&mut v);
    rewrite_record(w, step, &serde_json::to_vec_pretty(&v).unwrap());
}

/// Rewrites one step's stdout and keeps its record (and summary row) hash-exact.
fn rewrite_stdout(w: &W, step: &str, text: &str) {
    let p = w.records().join(format!("{step}.stdout"));
    fs::write(&p, text).unwrap();
    edit_record(w, step, |r| {
        r["stdout"]["length"] = json!(text.len());
        r["stdout"]["sha256"] = json!(sha256_hex(text.as_bytes()));
    });
}

/// `{"k":v,"k":v,...}` for the first key of an object (or of an array's first object element).
fn with_duplicate_key(v: &Value) -> Option<String> {
    match v {
        Value::Object(m) => {
            let (k, x) = m.iter().next()?;
            let compact = serde_json::to_string(v).unwrap();
            Some(format!(
                "{{{}:{},{}",
                serde_json::to_string(k).unwrap(),
                serde_json::to_string(x).unwrap(),
                &compact[1..]
            ))
        }
        Value::Array(a) => {
            let first = with_duplicate_key(a.first()?)?;
            let rest: Vec<String> = a[1..]
                .iter()
                .map(|x| serde_json::to_string(x).unwrap())
                .collect();
            Some(format!(
                "[{}]",
                std::iter::once(first)
                    .chain(rest)
                    .collect::<Vec<_>>()
                    .join(",")
            ))
        }
        _ => None,
    }
}

/// Whether check `id` failed with a finding naming `artifact` (isolates which join refused).
fn failed_on(e: &Evaluation, id: &str, artifact: &str) -> bool {
    matches!(e.status(id), Some(Status::Failed(f)) if f.iter().any(|x| x.artifact == artifact))
}

fn restore(w: &W, name: &str, bytes: &[u8]) {
    rewrite_output(w, name, bytes);
    assert!(post(w).is_qualified(), "{name} restored");
}

// ---------------------------------------------------------------------------------------------

#[test]
fn complete_owner_produced_evidence_qualifies_every_required_check() {
    let w = full("valid");
    let e = post(&w);
    let report = e.to_json("postflight");
    assert!(e.is_qualified(), "{report}");
    let spec = load_spec(&w.map_path, &file_sha(&w.map_path).unwrap()).unwrap();
    assert_eq!(report["required_checks"], json!(spec.checks.len()));
    assert_eq!(report["passed_checks"], json!(spec.checks.len()));
    assert!(e.incomplete().is_empty());
    assert_eq!(e.facts["records_checked"], json!(18));
    let q = e.qualified().unwrap();
    assert_eq!(q.checks().len(), spec.checks.len());
}

#[test]
fn every_required_custody_record_corruption_refuses_and_seventeen_records_cannot_qualify_eighteen()
{
    let w = full("records");
    for step in STEPS {
        let path = w.records().join(format!("{step}.json"));
        let original = fs::read(&path).unwrap();
        rewrite_record(
            &w,
            step,
            b"{\"schema\": \"manual-orchestration.custody.run.v1\", \"name\": ",
        );
        let e = post(&w);
        assert!(!e.is_qualified(), "{step}");
        assert!(
            e.failed_with(&format!("record:{step}"), Class::Malformed),
            "{step}: {}",
            e.to_json("x")
        );
        assert!(
            e.passed("summary"),
            "{step}: summary membership and hashes stay valid"
        );
        assert!(e.incomplete().contains(&format!("record:{step}")));
        assert!(
            e.incomplete().contains(&"step_order".to_owned()),
            "{step}: dependent checks stay incomplete"
        );
        if step == "fmt" {
            assert_eq!(e.facts["records_checked"], json!(17));
            assert_eq!(e.facts["records_required"], json!(18));
        }
        rewrite_record(&w, step, &original);
    }
    assert!(post(&w).is_qualified());
    // Duplicate, missing and wrong-type fields in a record.
    let path = w.records().join("fmt.json");
    let original = String::from_utf8(fs::read(&path).unwrap()).unwrap();
    let dup = original.replacen(
        "\"returncode\": 0",
        "\"returncode\": 1, \"returncode\": 0",
        1,
    );
    assert_ne!(dup, original);
    rewrite_record(&w, "fmt", dup.as_bytes());
    assert!(
        post(&w).failed_with("record:fmt", Class::Malformed),
        "a shadowed duplicate field must not pass"
    );
    rewrite_record(&w, "fmt", original.as_bytes());
    edit_record(&w, "fmt", |r| {
        r.as_object_mut().unwrap().remove("timed_out");
    });
    assert!(post(&w).failed_with("record:fmt", Class::Malformed));
    rewrite_record(&w, "fmt", original.as_bytes());
    edit_record(&w, "fmt", |r| r["returncode"] = json!("0"));
    assert!(post(&w).failed_with("record:fmt", Class::Malformed));
    rewrite_record(&w, "fmt", original.as_bytes());
    // Summary itself: malformed and duplicate rows.
    let sp = w.records().join("summary.json");
    let summary = fs::read(&sp).unwrap();
    fs::write(&sp, b"{").unwrap();
    let e = post(&w);
    assert!(
        e.failed_with("summary", Class::Malformed)
            && STEPS.iter().all(|s| e.blocked(&format!("record:{s}")))
    );
    let mut s: Value = serde_json::from_slice(&summary).unwrap();
    let first = s["steps"][0].clone();
    s["steps"].as_array_mut().unwrap().push(first);
    fs::write(&sp, serde_json::to_vec_pretty(&s).unwrap()).unwrap();
    assert!(post(&w).failed("summary"));
    fs::write(&sp, &summary).unwrap();
    assert!(post(&w).is_qualified());
}

#[test]
fn required_json_outputs_refuse_malformed_duplicate_missing_and_wrong_type_fields() {
    let w = full("json-fields");
    for (name, repr) in REQUIRED_ARTIFACTS {
        if *repr != spec::Repr::Json {
            continue;
        }
        let original = read_out(&w, name);
        let value: Value = serde_json::from_slice(&original).unwrap();
        rewrite_output(&w, name, b"{\"truncated\": ");
        let e = post(&w);
        assert!(
            e.passed("output_manifest")
                && e.failed_with(&format!("artifact:{name}"), Class::Malformed),
            "{name} malformed"
        );
        let dup =
            with_duplicate_key(&value).unwrap_or_else(|| panic!("{name}: no object to duplicate"));
        rewrite_output(&w, name, dup.as_bytes());
        let e = post(&w);
        assert!(
            e.passed("output_manifest")
                && e.failed_with(&format!("artifact:{name}"), Class::Malformed),
            "{name} duplicate key: {}",
            e.to_json("x")
        );
        let mut missing = value.clone();
        let target = if missing.is_array() {
            &mut missing[0]
        } else {
            &mut missing
        };
        let key = target.as_object().unwrap().keys().next().unwrap().clone();
        target.as_object_mut().unwrap().remove(&key);
        rewrite_output(&w, name, &serde_json::to_vec(&missing).unwrap());
        let e = post(&w);
        assert!(
            e.passed("output_manifest")
                && e.passed(&format!("artifact:{name}"))
                && !e.is_qualified(),
            "{name} missing `{key}` must be refused semantically"
        );
        let mut wrong = value.clone();
        let target = if wrong.is_array() {
            &mut wrong[0]
        } else {
            &mut wrong
        };
        target[&key] = if target[&key].is_number() {
            json!("12")
        } else {
            json!(12)
        };
        rewrite_output(&w, name, &serde_json::to_vec(&wrong).unwrap());
        let e = post(&w);
        assert!(
            e.passed("output_manifest") && !e.is_qualified(),
            "{name} wrong type for `{key}`"
        );
        restore(&w, name, &original);
    }
    // Indirectly consumed JSON: SDK verification, cargo metadata, cargo build messages, test results.
    let sp = w.records().join("sdk-verification.json");
    let sdk = fs::read(&sp).unwrap();
    let dup = with_duplicate_key(&serde_json::from_slice(&sdk).unwrap()).unwrap();
    fs::write(&sp, dup).unwrap();
    assert!(post(&w).failed_with("sdk_verification", Class::Malformed));
    fs::write(&sp, &sdk).unwrap();
    let meta = fs::read_to_string(w.records().join("metadata.stdout")).unwrap();
    rewrite_stdout(&w, "metadata", &meta[..meta.len() - 3]);
    assert!(post(&w).failed_with("closure_root", Class::Malformed));
    rewrite_stdout(&w, "metadata", &meta);
    let build = fs::read_to_string(w.records().join("build.stdout")).unwrap();
    rewrite_stdout(&w, "build", &format!("Compiling a v0.1.0\n{build}"));
    assert!(
        post(&w).failed_with("record:build", Class::Malformed),
        "non-JSON text in a required cargo message stream"
    );
    rewrite_stdout(&w, "build", &build);
    let tests = fs::read_to_string(w.records().join("test-pure.stdout")).unwrap();
    rewrite_stdout(&w, "test-pure", &tests.replace("1 passed;", "one passed;"));
    assert!(post(&w).failed_with("record:test-pure", Class::Malformed));
    rewrite_stdout(&w, "test-pure", &tests);
    assert!(post(&w).is_qualified());
}

#[test]
fn protocol_lines_are_fully_accounted_and_correspond() {
    let w = full("protocol");
    const R: &str = "native-frames-received.jsonl";
    const S: &str = "native-frames-sent.jsonl";
    let (recv, sent) = (read_out(&w, R), read_out(&w, S));
    edit_lines(&w, R, |l| l.push("{not json".into()));
    assert!(post(&w).failed_with("native_protocol", Class::Mismatch));
    restore(&w, R, &recv);
    edit_lines(&w, R, |l| l.insert(1, String::new()));
    assert!(post(&w).failed_with("native_protocol", Class::Mismatch));
    restore(&w, R, &recv);
    edit_lines(&w, R, |l| {
        let a = l[0].clone();
        l.insert(1, a)
    });
    assert!(
        post(&w).failed_with("native_protocol", Class::Mismatch),
        "duplicate allocation"
    );
    restore(&w, R, &recv);
    edit_lines(&w, R, |l| {
        let o = l[1].clone();
        l.push(o)
    });
    assert!(
        post(&w).failed_with("native_protocol", Class::Mismatch),
        "duplicate observation"
    );
    restore(&w, R, &recv);
    rewrite_output(&w, S, &[sent.clone(), sent.clone()].concat());
    assert!(
        post(&w).failed_with("native_protocol", Class::Mismatch),
        "duplicate start"
    );
    restore(&w, S, &sent);
    for (field, value) in [
        ("session_id", json!("sess-other")),
        ("attempt_id", json!("attempt-9")),
        ("generation", json!(2)),
        ("contract", json!("{}")),
        ("result_text", json!("{}")),
        ("contract_sha256", json!("0".repeat(64))),
    ] {
        edit_lines(&w, S, |l| {
            let mut v: Value = serde_json::from_str(&l[0]).unwrap();
            v[field] = value.clone();
            l[0] = v.to_string();
        });
        assert!(
            post(&w).failed_with("native_protocol", Class::Mismatch),
            "start frame {field}"
        );
        restore(&w, S, &sent);
    }
    // Observation identities changed consistently in the frame and the retained observation.
    let obs = read_out(&w, "observation.json");
    for (field, value) in [
        ("attempt_id", json!("attempt-9")),
        ("generation", json!(7)),
        ("session_id", json!("sess-other")),
    ] {
        edit_lines(&w, R, |l| {
            let mut v: Value = serde_json::from_str(&l[1]).unwrap();
            v[field] = value.clone();
            l[1] = v.to_string();
        });
        edit_json_output(&w, "observation.json", |v| v[field] = value.clone());
        assert!(
            post(&w).failed_with("native_protocol", Class::OwnerRejected),
            "observation {field}"
        );
        rewrite_output(&w, "observation.json", &obs);
        restore(&w, R, &recv);
    }
    // A raw observation that differs from the retained observation file.
    edit_json_output(&w, "observation.json", |v| v["request_count"] = json!(2));
    assert!(post(&w).failed_with("native_protocol", Class::Mismatch));
    restore(&w, "observation.json", &obs);
}

#[test]
fn context_and_history_must_equal_the_raw_observation() {
    let w = full("context");
    let ctx = read_out(&w, "request-context.json");
    edit_json_output(&w, "request-context.json", |v| {
        v["messages"][1]["content"][0]["text"] = json!("other")
    });
    assert!(post(&w).failed_with("context_history", Class::Mismatch));
    restore(&w, "request-context.json", &ctx);
    let hist = read_out(&w, "native-history.json");
    edit_json_output(&w, "native-history.json", |v| {
        v.as_array_mut().unwrap().pop();
    });
    assert!(post(&w).failed_with("context_history", Class::Mismatch));
    restore(&w, "native-history.json", &hist);
}

/// Replaces the synthetic result everywhere the native path carries it, consistently.
fn substitute_result(w: &W, result: &str) {
    rewrite_output(w, "synthetic-result.json", result.as_bytes());
    edit_lines(w, "native-frames-sent.jsonl", |l| {
        let mut v: Value = serde_json::from_str(&l[0]).unwrap();
        v["result_text"] = json!(result);
        l[0] = v.to_string();
    });
    edit_lines(w, "native-frames-received.jsonl", |l| {
        let mut v: Value = serde_json::from_str(&l[1]).unwrap();
        v["history"][2]["content"][0]["text"] = json!(result);
        l[1] = v.to_string();
    });
    edit_json_output(w, "observation.json", |v| {
        v["history"][2]["content"][0]["text"] = json!(result)
    });
    edit_json_output(w, "native-history.json", |v| {
        v[2]["content"][0]["text"] = json!(result)
    });
    edit_json_output(w, "identities.json", |v| {
        v["verified_result_text_sha256"] = json!(sha256_hex(result.as_bytes()))
    });
    resync_collection(&w.evidence());
}

#[test]
fn states_are_owner_validated_and_valid_foreign_states_or_results_refuse_correspondence() {
    let w = full("states");
    let (before, after, revisions) = (
        read_out(&w, "episode-before.json"),
        read_out(&w, "episode-after.json"),
        read_out(&w, "revisions.json"),
    );
    // Abbreviated, noncanonical and stale-revision states: the episode owner refuses them.
    rewrite_output(&w, "episode-after.json", br#"{"phase":"reported"}"#);
    assert!(post(&w).failed_with("artifact:episode-after.json", Class::OwnerRejected));
    restore(&w, "episode-after.json", &after);
    let pretty =
        serde_json::to_vec_pretty(&serde_json::from_slice::<Value>(&before).unwrap()).unwrap();
    rewrite_output(&w, "episode-before.json", &pretty);
    assert!(
        post(&w).failed_with("artifact:episode-before.json", Class::OwnerRejected),
        "noncanonical stored state"
    );
    restore(&w, "episode-before.json", &before);
    let stale = String::from_utf8(after.clone()).unwrap().replacen(
        "\"unresolvedQuestions\":[]",
        "\"unresolvedQuestions\":[\"x\"]",
        1,
    );
    assert_ne!(stale.as_bytes(), after.as_slice());
    rewrite_output(&w, "episode-after.json", stale.as_bytes());
    assert!(
        post(&w).failed_with("artifact:episode-after.json", Class::OwnerRejected),
        "stale embedded revision"
    );
    restore(&w, "episode-after.json", &after);
    // Detached revision strings.
    edit_json_output(&w, "revisions.json", |v| v["after"] = json!("f".repeat(64)));
    assert!(post(&w).failed_with("state_revisions", Class::Mismatch));
    restore(&w, "revisions.json", &revisions);
    edit_json_output(&w, "revisions.json", |v| {
        v["expected"] = json!("e".repeat(64))
    });
    assert!(post(&w).failed_with("state_revisions", Class::Mismatch));
    restore(&w, "revisions.json", &revisions);

    // A valid, owner-produced, correctly rehashed after-state of another transition, with every
    // revision field updated to it: only the transition correspondence can refuse it.
    let world = fixture_world();
    let result = result_json(|_| {});
    let foreign = consume_fixture_result(
        &world.candidate,
        &world.declaration,
        &begun(),
        &authority(),
        begun().revision().clone(),
        "result-foreign",
        &result,
        &[],
        &serde_json::to_string(&world.admissions).unwrap(),
    )
    .unwrap();
    let foreign_after = canonical(foreign.state());
    // consumption.json is left as the true recomputation, so only the state join can refuse.
    rewrite_output(&w, "episode-after.json", foreign_after.as_bytes());
    edit_json_output(&w, "revisions.json", |v| {
        v["after"] = json!(foreign.state().revision().0)
    });
    let e = post(&w);
    assert!(
        e.passed("artifact:episode-after.json") && e.passed("state_revisions"),
        "the foreign state is internally valid: {}",
        e.to_json("x")
    );
    assert!(
        failed_on(&e, "transition_recompute", "episode-after.json"),
        "{}",
        e.to_json("x")
    );
    rewrite_output(&w, "revisions.json", &revisions);
    restore(&w, "episode-after.json", &after);

    // A valid, owner-begun before-state for a foreign subject, with revisions and the consumer's
    // expected revision updated to it: the canonical binding refuses the subject.
    let mut av: Value = serde_json::from_str(AUTHORITY).unwrap();
    av["initialSubject"]["sha256"] = json!("0".repeat(64));
    let foreign_authority = Authority::parse(parse_json(&av.to_string()).unwrap()).unwrap();
    let foreign_before = begin(
        None,
        &BeginCommand::new(foreign_authority, "begin-1", vec![]).unwrap(),
    )
    .unwrap()
    .into_state();
    let ci = read_out(&w, "consumer-inputs.json");
    rewrite_output(
        &w,
        "episode-before.json",
        canonical(&foreign_before).as_bytes(),
    );
    edit_json_output(&w, "revisions.json", |v| {
        v["before"] = json!(foreign_before.revision().0);
        v["expected"] = json!(foreign_before.revision().0);
    });
    edit_json_output(&w, "consumer-inputs.json", |v| {
        v["expected_revision"] = json!(foreign_before.revision().0)
    });
    let e = post(&w);
    assert!(
        e.passed("artifact:episode-before.json"),
        "the foreign before-state is owner-valid"
    );
    assert!(
        e.failed_with("binding_recompute", Class::OwnerRejected),
        "{}",
        e.to_json("x")
    );
    rewrite_output(&w, "consumer-inputs.json", &ci);
    rewrite_output(&w, "revisions.json", &revisions);
    restore(&w, "episode-before.json", &before);

    // A valid result for a foreign subject, carried consistently through the native evidence:
    // native correspondence holds, the existing consumer refuses it.
    let snapshot: Vec<(String, Vec<u8>)> = [
        "synthetic-result.json",
        "native-frames-sent.jsonl",
        "native-frames-received.jsonl",
        "observation.json",
        "native-history.json",
        "identities.json",
        "collection.json",
    ]
    .iter()
    .map(|n| (n.to_string(), read_out(&w, n)))
    .collect();
    substitute_result(
        &w,
        &result_json(|v| v["subject"]["tree"] = json!("0".repeat(40))),
    );
    let e = post(&w);
    assert!(e.passed("native_protocol"), "{}", e.to_json("x"));
    assert!(
        e.failed_with("transition_recompute", Class::OwnerRejected),
        "{}",
        e.to_json("x")
    );
    for (n, b) in &snapshot {
        rewrite_output(&w, n, b);
    }
    // The synthetic result alone differs from what the native path carried.
    rewrite_output(
        &w,
        "synthetic-result.json",
        result_json(|v| v["subject"]["tree"] = json!("1".repeat(40))).as_bytes(),
    );
    assert!(!post(&w).passed("native_protocol"));
    for (n, b) in &snapshot {
        rewrite_output(&w, n, b);
    }
    assert!(post(&w).is_qualified());
}

#[test]
fn admissions_and_declarations_refuse_through_owner_coverage_and_reference_semantics() {
    let w = full("admissions");
    let admissions = read_out(&w, "admissions.json");
    let list: Vec<Value> = serde_json::from_slice(&admissions).unwrap();
    for (label, edited) in [
        ("missing", json!([list[0]])),
        ("duplicate", json!([list[0], list[0], list[1]])),
        ("foreign", {
            let mut l = list.clone();
            l[0]["claimRevisionRef"]["sha256"] = json!("0".repeat(64));
            json!(l)
        }),
        ("not established", {
            let mut l = list.clone();
            l[1]["status"] = json!("unestablished");
            json!(l)
        }),
    ] {
        rewrite_output(&w, "admissions.json", &serde_json::to_vec(&edited).unwrap());
        let e = post(&w);
        assert!(
            e.passed("binding_recompute") && e.failed("transition_recompute") && !e.is_qualified(),
            "{label}: {}",
            e.to_json("x")
        );
    }
    restore(&w, "admissions.json", &admissions);
    let decl = read_out(&w, "declaration.json");
    edit_json_output(&w, "declaration.json", |v| {
        v["episode"]["review_episode_id"] = json!("other-episode")
    });
    let e = post(&w);
    assert!(
        e.failed_with("transition_recompute", Class::OwnerRejected),
        "declared episode conflicts with the state: {}",
        e.to_json("x")
    );
    restore(&w, "declaration.json", &decl);
    edit_json_output(&w, "declaration.json", |v| {
        v["subject"]["tree"] = json!("0".repeat(40))
    });
    assert!(post(&w).failed_with("binding_recompute", Class::OwnerRejected));
    restore(&w, "declaration.json", &decl);
    edit_json_output(&w, "declaration.json", |v| {
        v["bindings"].as_array_mut().unwrap().truncate(1)
    });
    assert!(
        post(&w).failed_with("binding_recompute", Class::OwnerRejected),
        "coverage of the declared bindings"
    );
    restore(&w, "declaration.json", &decl);
    edit_json_output(&w, "declaration.json", |v| v["extra"] = json!(true));
    assert!(
        post(&w).failed_with("binding_recompute", Class::Malformed),
        "unknown declaration field"
    );
    restore(&w, "declaration.json", &decl);
    // Consumer inputs and the frozen fixtures.
    let ci = read_out(&w, "consumer-inputs.json");
    edit_json_output(&w, "consumer-inputs.json", |v| {
        v["transition_id"] = json!("result-2")
    });
    let e = post(&w);
    assert!(
        failed_on(&e, "transition_recompute", "episode-after.json")
            && failed_on(&e, "transition_recompute", "consumption.json"),
        "a different transition id recomputes a different state"
    );
    restore(&w, "consumer-inputs.json", &ci);
    let auth = read_out(&w, "authority.json");
    let mut av: Value = serde_json::from_slice(&auth).unwrap();
    av["writer"]["actorId"] = json!("a-different-writer");
    rewrite_output(&w, "authority.json", &serde_json::to_vec(&av).unwrap());
    assert!(
        post(&w).failed_with("binding_recompute", Class::Mismatch),
        "authority is pinned to the frozen fixture"
    );
    restore(&w, "authority.json", &auth);
    let consumption = read_out(&w, "consumption.json");
    edit_json_output(&w, "consumption.json", |v| {
        v["conditions"].as_array_mut().unwrap().pop();
    });
    assert!(
        post(&w).failed_with("transition_recompute", Class::Mismatch),
        "condition coverage"
    );
    restore(&w, "consumption.json", &consumption);
}

#[test]
fn malformed_or_unknown_process_disposition_and_unavailable_timing_refuse() {
    let w = full("process");
    let p = read_out(&w, "process.json");
    for (label, edit, class) in [
        ("string pid", json!({"pid": "4242"}), Class::Malformed),
        (
            "wrong-type exit code",
            json!({"reap": {"kind": "exited", "code": "0"}}),
            Class::Malformed,
        ),
        (
            "foreign reap field",
            json!({"reap": {"kind": "exited", "code": 0, "signal": null}}),
            Class::Malformed,
        ),
        (
            "unknown reap kind",
            json!({"reap": {"kind": "probably_exited"}}),
            Class::Malformed,
        ),
        (
            "nonzero exit",
            json!({"reap": {"kind": "exited", "code": 3}}),
            Class::Failed,
        ),
        (
            "signalled",
            json!({"reap": {"kind": "signalled", "signal": 9}}),
            Class::Failed,
        ),
        (
            "not reaped",
            json!({"reap": {"kind": "not_reaped"}, "pid_present_after_reap": null}),
            Class::Failed,
        ),
        (
            "wait failed",
            json!({"reap": {"kind": "wait_failed", "error": "ECHILD"}, "wait_errors": ["ECHILD"], "pid_present_after_reap": null}),
            Class::Failed,
        ),
        (
            "killed",
            json!({"kill": {"kind": "sent", "reason": "deadline"}, "timed_out": true}),
            Class::Failed,
        ),
        (
            "kill failed",
            json!({"kill": {"kind": "failed", "reason": "collection_stopped", "error": "ESRCH"}}),
            Class::Failed,
        ),
        (
            "descendants",
            json!({"descendants_seen": [4243]}),
            Class::Failed,
        ),
        (
            "still present",
            json!({"pid_present_after_reap": true}),
            Class::Failed,
        ),
        ("unknown key", json!({"reaped": "maybe"}), Class::Malformed),
        ("wrong type", json!({"timed_out": "no"}), Class::Malformed),
        (
            "deadline kill without timeout",
            json!({"kill": {"kind": "sent", "reason": "deadline"}}),
            Class::Malformed,
        ),
        (
            "unreaped with known presence",
            json!({"reap": {"kind": "not_reaped"}}),
            Class::Malformed,
        ),
    ] {
        let mut v: Value = serde_json::from_slice(&p).unwrap();
        for (k, x) in edit.as_object().unwrap() {
            v[k] = x.clone();
        }
        rewrite_output(&w, "process.json", &serde_json::to_vec(&v).unwrap());
        assert!(
            post(&w).failed_with("process_disposition", class),
            "{label}"
        );
    }
    let mut v: Value = serde_json::from_slice(&p).unwrap();
    v.as_object_mut().unwrap().remove("pid");
    rewrite_output(&w, "process.json", &serde_json::to_vec(&v).unwrap());
    assert!(post(&w).failed_with("process_disposition", Class::Malformed));
    restore(&w, "process.json", &p);
    // Required timing unavailable: the producer record refuses; timing-dependent checks stay incomplete.
    let rec = fs::read(w.records().join("test-native.json")).unwrap();
    edit_record(&w, "test-native", |r| r["started_at"] = json!("yesterday"));
    let e = post(&w);
    assert!(e.failed_with("record:test-native", Class::Malformed));
    for id in [
        "output_window",
        "output_manifest",
        "invocation_binding",
        "process_disposition",
        "transition_recompute",
        "control:assertion-unwind",
        "step_order",
    ] {
        assert!(
            e.blocked(id) || e.incomplete().contains(&id.to_owned()),
            "{id} must remain incomplete"
        );
        assert!(!e.passed(id), "{id}");
    }
    rewrite_record(&w, "test-native", &rec);
    // Outputs written outside the producer's custody window.
    set_mtime(&w.evidence().join("process.json"), T0 + 999);
    assert!(post(&w).failed_with("output_window", Class::Mismatch));
    touch_all(&w.evidence());
    assert!(post(&w).is_qualified());
}

#[test]
fn capture_bundles_and_controls_must_show_preserved_failure_evidence() {
    let w = full("controls");
    let run = PathBuf::from(&w.run.run_dir);
    let cap = read_out(&w, "capture.json");
    edit_json_output(&w, "capture.json", |v| {
        v["absent"] = json!([{"name": "observation.json", "reason": "x"}])
    });
    assert!(
        post(&w).failed_with("capture_complete", Class::Mismatch),
        "capture claims an absence the bundle contradicts"
    );
    restore(&w, "capture.json", &cap);
    let mp = w.evidence().join("output-manifest.json");
    let manifest = fs::read(&mp).unwrap();
    let mut m: Value = serde_json::from_slice(&manifest).unwrap();
    m["outcome"] = json!("incomplete");
    m["write_failures"] = json!([{"name": "episode-after.json", "error": "disk full"}]);
    fs::write(&mp, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    touch_all(&w.evidence());
    assert!(post(&w).failed_with("output_manifest", Class::Failed));
    fs::write(&mp, &manifest).unwrap();
    touch_all(&w.evidence());
    // Unaccounted entry in the evidence directory.
    fs::write(w.evidence().join("stray.txt"), b"x").unwrap();
    assert!(post(&w).failed_with("output_manifest", Class::Mismatch));
    fs::remove_file(w.evidence().join("stray.txt")).unwrap();
    // Scratch not retained after the failure.
    let s = run.join("scratch/control-write-failure-77");
    fs::rename(&s, run.join("scratch/gone")).unwrap();
    assert!(post(&w).failed_with("control:write-failure", Class::Mismatch));
    fs::rename(run.join("scratch/gone"), &s).unwrap();
    // A control whose preserved observation is missing.
    let c = run.join("controls/assertion-unwind");
    let cm = fs::read(c.join("output-manifest.json")).unwrap();
    let mut v: Value = serde_json::from_slice(&cm).unwrap();
    v["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["name"] != "observation.json");
    fs::write(
        c.join("output-manifest.json"),
        serde_json::to_vec_pretty(&v).unwrap(),
    )
    .unwrap();
    assert!(post(&w).failed_with("control:assertion-unwind", Class::Missing));
    fs::write(c.join("output-manifest.json"), &cm).unwrap();
    // A control bundle that is absent entirely.
    fs::rename(
        run.join("controls/write-failure"),
        run.join("controls-aside"),
    )
    .unwrap();
    assert!(post(&w).failed_with("control:write-failure", Class::Missing));
    fs::rename(
        run.join("controls-aside"),
        run.join("controls/write-failure"),
    )
    .unwrap();
    assert!(post(&w).is_qualified());
}

#[test]
fn retained_r3_controls_still_refuse() {
    // Empty inputs and an omitted doctest source.
    let w = full("r3-controls");
    let rec = fs::read(w.records().join("doctest-default.json")).unwrap();
    edit_record(&w, "doctest-default", |r| r["inputs"] = json!([]));
    assert!(post(&w).failed_with("record:doctest-default", Class::Missing));
    rewrite_record(&w, "doctest-default", &rec);
    let lib = w.root.join("crates/a/src/lib.rs").display().to_string();
    edit_record(&w, "doctest-default", |r| {
        r["inputs"]
            .as_array_mut()
            .unwrap()
            .retain(|e| e["path"] != json!(lib))
    });
    assert!(post(&w).failed_with("record:doctest-default", Class::Missing));
    rewrite_record(&w, "doctest-default", &rec);
    // Wrong executable path, digest or bytes.
    let native = fs::read(w.records().join("test-native.json")).unwrap();
    edit_record(&w, "test-native", |r| r["argv"][0] = json!("/usr/bin/true"));
    assert!(post(&w).failed("record:test-native"));
    rewrite_record(&w, "test-native", &native);
    let exe = w.dir.join("exe/build/native_vertical");
    fs::write(&exe, b"different executable").unwrap();
    assert!(post(&w).failed_with("record:test-native", Class::Mismatch));
    fs::write(&exe, b"build:native_vertical:bytes").unwrap();
    // Foreign invocation in the retained identities.
    let ident = read_out(&w, "identities.json");
    edit_json_output(&w, "identities.json", |v| {
        v["invocation_id"] = json!("inv-other")
    });
    let e = post(&w);
    assert!(e.failed_with("invocation_binding", Class::Mismatch) && e.failed("native_protocol"));
    restore(&w, "identities.json", &ident);
    // A required output removed from the bundle and its manifest.
    let mp = w.evidence().join("output-manifest.json");
    let manifest = fs::read(&mp).unwrap();
    let mut m: Value = serde_json::from_slice(&manifest).unwrap();
    m["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["name"] != "episode-after.json");
    fs::write(&mp, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let after = read_out(&w, "episode-after.json");
    fs::remove_file(w.evidence().join("episode-after.json")).unwrap();
    touch_all(&w.evidence());
    let e = post(&w);
    assert!(
        e.failed("output_manifest")
            && e.blocked("artifact:episode-after.json")
            && e.blocked("transition_recompute")
    );
    fs::write(w.evidence().join("episode-after.json"), &after).unwrap();
    fs::write(&mp, &manifest).unwrap();
    touch_all(&w.evidence());
    // Source drift after the manifest, and a foreign plan identity.
    let f = w.root.join("crates/a/fixtures/f.txt");
    fs::write(&f, b"drifted").unwrap();
    assert!(post(&w).failed("source_manifest"));
    fs::write(&f, b"fixture bytes").unwrap();
    let p = pins(&w);
    let wrong = (p.0.clone(), "0".repeat(64), p.2.clone());
    assert!(postflight(&inputs(&w, &wrong), &w.records()).failed("pins"));
    assert!(post(&w).is_qualified());
}

#[test]
fn malformed_plans_never_reach_execution() {
    let w = world_with("plans", |_| {});
    assert!(pre(&w).is_qualified());
    let original = fs::read(&w.plan_path).unwrap();
    let edits: Vec<(&str, MapEdit)> = vec![
        (
            "env_clear",
            Box::new(|p: &mut Value| p["env_clear"] = json!(false)),
        ),
        (
            "unknown key",
            Box::new(|p: &mut Value| p["extra"] = json!(1)),
        ),
        (
            "reordered",
            Box::new(|p: &mut Value| p["steps"].as_array_mut().unwrap().swap(0, 1)),
        ),
        (
            "dropped step",
            Box::new(|p: &mut Value| {
                p["steps"].as_array_mut().unwrap().remove(5);
            }),
        ),
        (
            "digest omitted",
            Box::new(|p: &mut Value| {
                p["steps"][15]["digest"].as_array_mut().unwrap().pop();
            }),
        ),
        (
            "argv changed",
            Box::new(|p: &mut Value| p["steps"][0]["argv"] = json!(["/bin/true"])),
        ),
        (
            "env changed",
            Box::new(|p: &mut Value| p["steps"][11]["env"]["WE_RUN_DIR"] = json!("/tmp/elsewhere")),
        ),
    ];
    for (label, edit) in edits {
        let mut plan: Value = serde_json::from_slice(&original).unwrap();
        edit(&mut plan);
        fs::write(&w.plan_path, serde_json::to_vec_pretty(&plan).unwrap()).unwrap();
        assert!(!pre(&w).is_qualified(), "{label}");
    }
    fs::write(&w.plan_path, &original).unwrap();
    fs::create_dir_all(&w.run.records_dir).unwrap();
    assert!(pre(&w).failed("run_dirs_absent"));
}

#[test]
fn the_specification_loader_rejects_incomplete_or_inconsistent_maps() {
    let w = world_with("spec", |_| {});
    let map: Value = serde_json::from_slice(&fs::read(&w.map_path).unwrap()).unwrap();
    assert!(validate(map.clone(), String::new()).is_ok());
    let cases: Vec<(&str, MapEdit)> = vec![
        (
            "missing check",
            Box::new(|m: &mut Value| {
                m["checks"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|c| c["id"] != "transition_recompute");
            }),
        ),
        (
            "duplicate check",
            Box::new(|m: &mut Value| {
                let c = m["checks"][0].clone();
                m["checks"].as_array_mut().unwrap().push(c);
            }),
        ),
        (
            "unknown kind",
            Box::new(|m: &mut Value| {
                m["checks"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"id": "bogus", "depends_on": []}))
            }),
        ),
        (
            "unresolved dependency",
            Box::new(|m: &mut Value| {
                m["checks"][3]["depends_on"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("nowhere"))
            }),
        ),
        (
            "missing intrinsic dependency",
            Box::new(|m: &mut Value| {
                for c in m["checks"].as_array_mut().unwrap() {
                    if c["id"] == "transition_recompute" {
                        c["depends_on"]
                            .as_array_mut()
                            .unwrap()
                            .retain(|d| d != "state_revisions");
                    }
                }
            }),
        ),
        (
            "artifact without checker",
            Box::new(|m: &mut Value| {
                m["artifacts"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"name": "extra.json", "representation": "json"}))
            }),
        ),
        (
            "wrong representation",
            Box::new(|m: &mut Value| m["artifacts"][14]["representation"] = json!("json")),
        ),
        (
            "undeclared required artifact",
            Box::new(|m: &mut Value| {
                m["artifacts"].as_array_mut().unwrap().pop();
            }),
        ),
        (
            "duplicate step",
            Box::new(|m: &mut Value| {
                let p = m["processes"][0].clone();
                m["processes"].as_array_mut().unwrap().push(p);
            }),
        ),
        (
            "unknown input class",
            Box::new(|m: &mut Value| m["processes"][0]["inputs"] = json!(["nope"])),
        ),
        (
            "exe without build",
            Box::new(|m: &mut Value| {
                m["processes"][9]["exe"] = json!({"step": "fmt", "target": "pure"})
            }),
        ),
        (
            "null exe",
            Box::new(|m: &mut Value| m["processes"][0]["exe"] = Value::Null),
        ),
        (
            "unknown map key",
            Box::new(|m: &mut Value| m["required_checks_override"] = json!([])),
        ),
        (
            "v1 schema",
            Box::new(|m: &mut Value| m["schema"] = json!("org-obligation-map/v1")),
        ),
        (
            "missing expectation",
            Box::new(|m: &mut Value| {
                m["expected"].as_object_mut().unwrap().remove("sdk_step");
            }),
        ),
    ];
    for (label, edit) in cases {
        let mut m = map.clone();
        edit(&mut m);
        assert!(validate(m, String::new()).is_err(), "{label}");
    }
    // The approval pin is checked before the specification is trusted.
    assert!(load_spec(&w.map_path, &"0".repeat(64)).is_err());
    let dup = with_duplicate_key(&map).unwrap();
    fs::write(&w.map_path, &dup).unwrap();
    assert!(
        load_spec(&w.map_path, &sha256_hex(dup.as_bytes())).is_err(),
        "duplicate map keys"
    );
}

#[test]
fn the_ledger_cannot_conclude_from_an_empty_finding_list() {
    let mut l = Ledger::new(&[
        ("a".into(), vec![]),
        ("b".into(), vec!["a".into()]),
        ("c".into(), vec![]),
    ]);
    assert!(l.conclude().is_err(), "nothing performed");
    l.run("a", |_d: &mut Defects| Some(()));
    assert!(l.conclude().is_err(), "b and c unperformed");
    l.run("b", |_d: &mut Defects| None::<()>);
    assert!(
        matches!(l.status("b"), Some(Status::Failed(_))),
        "an early return is a failure, not a pass"
    );
    l.run("c", |_d: &mut Defects| -> Option<()> {
        panic!("evaluator bug")
    });
    assert!(
        matches!(l.status("c"), Some(Status::Failed(f)) if f[0].class == Class::EvaluatorPanic)
    );
    assert_eq!(l.incomplete(), vec!["b".to_owned(), "c".to_owned()]);
    let mut l = Ledger::new(&[("a".into(), vec![]), ("b".into(), vec!["a".into()])]);
    l.run("a", |d: &mut Defects| {
        d.malformed("x", "bad");
        Some(())
    });
    assert!(l.run("b", |_d: &mut Defects| Some(())).is_none());
    assert!(matches!(l.status("b"), Some(Status::Blocked(_))));
    assert!(l.conclude().is_err());
}

#[test]
fn a_non_literal_include_fails_the_closure_unless_the_map_names_its_file() {
    let w = world_with("include", |_| {});
    let lib = w.root.join("crates/a/src/lib.rs");
    fs::write(
        &lib,
        "pub const F: &[u8] = include_bytes!(concat!(\"../fixtures/\", \"f.txt\"));\n",
    )
    .unwrap();
    let map: Value = serde_json::from_slice(&fs::read(&w.map_path).unwrap()).unwrap();
    assert!(build_manifest(&map, "x").is_err());
    let mut named = map.clone();
    named["accepted_nonliteral_include_files"] = json!([lib.display().to_string()]);
    assert!(build_manifest(&named, "x").is_ok());
}

// ---------------------------------------------------------------------------------------------
// R5: the qualifier's own build and direct test are required processes of the same inventory.
// Controls use complete valid baselines and keep surrounding hashes consistent.

const PRODUCT_STEPS: usize = 16;

fn edit_summary(w: &W, f: impl FnOnce(&mut Value)) {
    let sp = w.records().join("summary.json");
    let mut s: Value = serde_json::from_slice(&fs::read(&sp).unwrap()).unwrap();
    f(&mut s);
    fs::write(&sp, serde_json::to_vec_pretty(&s).unwrap()).unwrap();
}

fn product_checks_pass(e: &Evaluation) {
    for s in &STEPS[..PRODUCT_STEPS] {
        assert!(
            e.passed(&format!("record:{s}")),
            "product record {s} must still pass: {}",
            e.to_json("x")
        );
    }
}

fn edit_plan(w: &W, f: impl FnOnce(&mut Value)) -> Vec<u8> {
    let original = fs::read(&w.plan_path).unwrap();
    let mut plan: Value = serde_json::from_slice(&original).unwrap();
    f(&mut plan);
    fs::write(&w.plan_path, serde_json::to_vec_pretty(&plan).unwrap()).unwrap();
    original
}

#[test]
fn the_positive_baseline_carries_both_qualifier_processes_through_one_plan_summary_and_postflight()
{
    let w = full("r5-baseline");
    let map: Value = serde_json::from_slice(&fs::read(&w.map_path).unwrap()).unwrap();
    let names: Vec<&str> = map["processes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["step"].as_str().unwrap())
        .collect();
    assert_eq!(
        names, STEPS,
        "one process list: 16 product steps then the qualifier build and test"
    );
    let plan: Value = serde_json::from_slice(&fs::read(&w.plan_path).unwrap()).unwrap();
    let plan_names: Vec<&str> = plan["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(plan_names, STEPS, "one generated plan");
    // The generated plan binds the emitted gate executable by an explicit digest entry.
    assert!(plan["steps"][17]["digest"]
        .as_array()
        .unwrap()
        .contains(&json!("{exe:qualifier-build:gate}")));
    let summary: Value =
        serde_json::from_slice(&fs::read(w.records().join("summary.json")).unwrap()).unwrap();
    assert_eq!(
        summary["steps"].as_array().unwrap().len(),
        STEPS.len(),
        "one summary"
    );
    let e = post(&w);
    assert!(e.is_qualified(), "{}", e.to_json("postflight"));
    for s in QUALIFIER_STEPS {
        assert!(e.passed(&format!("record:{s}")));
    }
    // The direct test's record binds the actual executed file with the digest the build emitted.
    let build: Value =
        serde_json::from_slice(&fs::read(w.records().join("qualifier-build.json")).unwrap())
            .unwrap();
    let test: Value =
        serde_json::from_slice(&fs::read(w.records().join("qualifier-test.json")).unwrap())
            .unwrap();
    let emitted = &build["cargo_executables"][0];
    assert_eq!(test["argv"][0], emitted["path"]);
    assert!(test["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["path"] == emitted["path"] && i["sha256"] == emitted["sha256"]));
}

#[test]
fn control_1_a_qualifier_process_omitted_from_the_plan_or_its_record_cannot_qualify() {
    // Preflight needs a world whose run directories are still absent.
    let planned = world_with("r5-control-1-plan", |_| {});
    assert!(pre(&planned).is_qualified());
    let w = full("r5-control-1");
    for step in QUALIFIER_STEPS {
        // Plan without the process, map retained, plan hash consistent: preflight refuses membership.
        let original = edit_plan(&planned, |p| {
            p["steps"]
                .as_array_mut()
                .unwrap()
                .retain(|s| s["name"] != step)
        });
        let e = pre(&planned);
        assert!(
            !e.is_qualified() && e.failed("plan_shape"),
            "{step}: {}",
            e.to_json("pre")
        );
        assert!(
            e.incomplete().contains(&format!("plan_step:{step}")),
            "{step}"
        );
        fs::write(&planned.plan_path, original).unwrap();
        assert!(pre(&planned).is_qualified(), "{step} plan restored");
        // Record and summary row removed with consistent surrounding hashes: still incomplete.
        let record = fs::read(w.records().join(format!("{step}.json"))).unwrap();
        let summary = fs::read(w.records().join("summary.json")).unwrap();
        fs::remove_file(w.records().join(format!("{step}.json"))).unwrap();
        edit_summary(&w, |s| {
            s["steps"]
                .as_array_mut()
                .unwrap()
                .retain(|r| r["name"] != step)
        });
        let e = post(&w);
        assert!(!e.is_qualified(), "{step}");
        assert!(e.failed("summary"), "{step}: membership");
        assert!(e.incomplete().contains(&format!("record:{step}")), "{step}");
        assert!(
            e.facts["records_checked"].as_u64().unwrap() < STEPS.len() as u64,
            "{step}"
        );
        fs::write(w.records().join(format!("{step}.json")), record).unwrap();
        fs::write(w.records().join("summary.json"), summary).unwrap();
        assert!(post(&w).is_qualified(), "{step} restored");
    }
}

#[test]
fn control_2_removing_both_processes_from_map_plan_and_checks_is_refused_by_the_unchanged_approval_pins(
) {
    let approved = full("r5-control-2-approved");
    let altered = full_edit("r5-control-2-altered", |m| {
        m["processes"]
            .as_array_mut()
            .unwrap()
            .truncate(PRODUCT_STEPS);
        m["checks"] = derive_checks(m).unwrap();
    });
    let am: Value = serde_json::from_slice(&fs::read(&altered.map_path).unwrap()).unwrap();
    assert_eq!(am["processes"].as_array().unwrap().len(), PRODUCT_STEPS);
    let want = pins(&approved);
    // Altered map, plan, manifest and records are internally consistent; the pins are the approvals.
    fn inputs_with_approved_pins<'a>(w: &'a W, want: &'a (String, String, String)) -> Inputs<'a> {
        Inputs {
            map_path: &w.map_path,
            plan_path: &w.plan_path,
            manifest_path: &w.manifest_path,
            expect_map_sha256: &want.0,
            expect_plan_sha256: &want.1,
            expect_manifest_sha256: &want.2,
            run: RunVars {
                map_sha256: want.0.clone(),
                manifest_sha256: want.2.clone(),
                ..w.run.clone()
            },
        }
    }
    let e = preflight(&inputs_with_approved_pins(&altered, &want));
    assert!(
        !e.is_qualified() && !e.spec_errors.is_empty(),
        "approved map pin refuses the altered map"
    );
    let e = postflight(
        &inputs_with_approved_pins(&altered, &want),
        &altered.records(),
    );
    assert!(!e.is_qualified() && !e.spec_errors.is_empty());
    // Limit, disclosed: a newly *approved* omission is internally valid. Whether the inventory
    // covers every requirement is the root's and the reviewer's judgment, not this tool's.
    let own = post(&altered);
    assert!(own.is_qualified(), "{}", own.to_json("post"));
    assert_eq!(own.facts["records_required"], json!(PRODUCT_STEPS));
    assert!(post(&approved).is_qualified());
}

#[test]
fn control_3_a_passing_qualifier_test_without_the_executed_file_digest_fails_its_record_check() {
    let w = full("r5-control-3");
    let exe = w.dir.join("exe/qualifier-build/gate").display().to_string();
    edit_record(&w, "qualifier-test", |r| {
        r["inputs"]
            .as_array_mut()
            .unwrap()
            .retain(|i| i["path"] != json!(exe))
    });
    let rec: Value =
        serde_json::from_slice(&fs::read(w.records().join("qualifier-test.json")).unwrap())
            .unwrap();
    assert_eq!(rec["argv"][0], json!(exe), "same pathname executed");
    assert_eq!(rec["returncode"], json!(0));
    let e = post(&w);
    assert!(!e.is_qualified());
    assert!(
        e.failed_with("record:qualifier-test", Class::Missing),
        "{}",
        e.to_json("post")
    );
    assert!(
        e.failed_with("record:qualifier-test", Class::Mismatch),
        "the build-emitted digest is not bound either"
    );
    assert!(e.passed("summary") && e.passed("record:qualifier-build"));
}

#[test]
fn control_4_different_bytes_or_executable_with_consistent_hashes_fail_build_to_test_correspondence(
) {
    let w = full("r5-control-4");
    let exe_path = w.dir.join("exe/qualifier-build/gate");
    let exe = exe_path.display().to_string();
    let original = fs::read(&exe_path).unwrap();
    // Different bytes at the same pathname, replacement input digest and summary hash consistent.
    fs::write(&exe_path, b"substituted gate executable").unwrap();
    edit_record(&w, "qualifier-test", |r| {
        for i in r["inputs"].as_array_mut().unwrap() {
            if i["path"] == json!(exe) {
                *i = entry(&exe);
            }
        }
    });
    let e = post(&w);
    assert!(!e.is_qualified());
    assert!(
        e.failed_with("record:qualifier-test", Class::Mismatch),
        "{}",
        e.to_json("post")
    );
    assert!(e.passed("summary"), "summary hashes stay consistent");
    // A different executable at another pathname, bound and executed consistently.
    fs::write(&exe_path, &original).unwrap();
    let other = w.dir.join("exe/other-gate");
    put(&other, b"another executable");
    let other_s = other.display().to_string();
    edit_record(&w, "qualifier-test", |r| {
        r["argv"][0] = json!(other_s);
        for i in r["inputs"].as_array_mut().unwrap() {
            if i["path"] == json!(exe) {
                *i = entry(&other_s);
            }
        }
    });
    let e = post(&w);
    assert!(
        !e.is_qualified() && e.failed_with("record:qualifier-test", Class::Mismatch),
        "{}",
        e.to_json("post")
    );
}

#[test]
fn control_5_an_executable_placeholder_without_or_against_its_declared_relation_is_refused_by_specification_validation(
) {
    let w = world_with("r5-control-5", |_| {});
    let map: Value = serde_json::from_slice(&fs::read(&w.map_path).unwrap()).unwrap();
    assert!(validate(map.clone(), String::new()).is_ok());
    type Edit = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(&str, Edit)> = vec![
        (
            "qualifier-test: relation omitted, placeholder retained",
            Box::new(|m| {
                m["processes"][17].as_object_mut().unwrap().remove("exe");
            }),
        ),
        (
            "test-pure: relation omitted, placeholder retained (the R4 failure shape)",
            Box::new(|m| {
                m["processes"][9].as_object_mut().unwrap().remove("exe");
            }),
        ),
        (
            "relation target contradicts the placeholder",
            Box::new(|m| m["processes"][17]["exe"]["target"] = json!("native_vertical")),
        ),
        (
            "relation names an undeclared target of the build",
            Box::new(|m| {
                m["processes"][17]["exe"]["target"] = json!("other");
                m["processes"][17]["argv"] = json!(["{exe:qualifier-build:other}"]);
            }),
        ),
        (
            "relation names a different build step",
            Box::new(|m| m["processes"][17]["exe"]["step"] = json!("build")),
        ),
        (
            "relation names a non-build step",
            Box::new(|m| {
                m["processes"][17]["exe"]["step"] = json!("fmt");
                m["processes"][17]["argv"] = json!(["{exe:fmt:gate}"]);
            }),
        ),
        (
            "placeholder names another producer than the relation",
            Box::new(|m| m["processes"][17]["argv"] = json!(["{exe:build:pure}"])),
        ),
        (
            "shorthand placeholder without target",
            Box::new(|m| m["processes"][17]["argv"] = json!(["{exe:qualifier-build}"])),
        ),
        (
            "embedded placeholder",
            Box::new(|m| m["processes"][17]["argv"] = json!(["/x/{exe:qualifier-build:gate}"])),
        ),
        (
            "relation declared but argv[0] is a literal path",
            Box::new(|m| m["processes"][17]["argv"] = json!(["/usr/bin/true"])),
        ),
        (
            "placeholder in a later argv position",
            Box::new(|m| {
                m["processes"][17]["argv"] = json!(["/usr/bin/true", "{exe:qualifier-build:gate}"])
            }),
        ),
        (
            "placeholder in env",
            Box::new(|m| m["processes"][17]["env"]["X"] = json!("{exe:qualifier-build:gate}")),
        ),
        (
            "placeholder in cwd",
            Box::new(|m| m["processes"][17]["cwd"] = json!("{exe:qualifier-build:gate}")),
        ),
        (
            "placeholder on a step with no relation at all",
            Box::new(|m| m["processes"][0]["argv"] = json!(["{exe:qualifier-build:gate}"])),
        ),
        (
            "relation to a build that comes later",
            Box::new(|m| {
                let build = m["processes"].as_array_mut().unwrap().remove(16);
                m["processes"].as_array_mut().unwrap().push(build);
            }),
        ),
        (
            "duplicate emitted target declaration",
            Box::new(|m| m["processes"][16]["exe_targets"] = json!(["gate", "gate"])),
        ),
    ];
    for (label, edit) in cases {
        let mut m = map.clone();
        edit(&mut m);
        let r = validate(m, String::new());
        assert!(
            r.is_err(),
            "{label} must be refused by specification validation"
        );
    }
    // Refusal is at specification level: no run can be qualified from such a map.
    let mut bad = map.clone();
    bad["processes"][17].as_object_mut().unwrap().remove("exe");
    let bad_path = w.dir.join("bad-map.json");
    put(
        &bad_path,
        serde_json::to_string_pretty(&bad).unwrap().as_bytes(),
    );
    let sha = file_sha(&bad_path).unwrap();
    let errs = load_spec(&bad_path, &sha).unwrap_err();
    assert!(
        errs.iter()
            .any(|x| x.contains("qualifier-test") && x.contains("no declared `exe`")),
        "{errs:?}"
    );
}

#[test]
fn control_6_missing_malformed_failed_or_unperformed_qualifier_evidence_prevents_common_qualification(
) {
    let w = full("r5-control-6");
    assert!(post(&w).is_qualified(), "complete positive baseline");
    for step in QUALIFIER_STEPS {
        let path = w.records().join(format!("{step}.json"));
        let record = fs::read(&path).unwrap();
        let summary = fs::read(w.records().join("summary.json")).unwrap();
        let check = |what: &str, w: &W| {
            let e = post(w);
            assert!(!e.is_qualified(), "{step} {what}");
            assert!(
                e.incomplete().contains(&format!("record:{step}")) || e.failed("summary"),
                "{step} {what}"
            );
            // A failed summary blocks every record; otherwise the product checks all still pass.
            if e.passed("summary") {
                product_checks_pass(&e);
            }
        };
        // Missing: record absent, summary row retained.
        fs::remove_file(&path).unwrap();
        check("missing", &w);
        fs::write(&path, &record).unwrap();
        // Malformed.
        rewrite_record(&w, step, b"{\"schema\": ");
        check("malformed", &w);
        rewrite_record(&w, step, &record);
        // Failed: non-zero exit, and (for the test) a failing libtest result.
        edit_record(&w, step, |r| r["returncode"] = json!(101));
        check("failed exit", &w);
        rewrite_record(&w, step, &record);
        if step == "qualifier-test" {
            rewrite_stdout(
                &w,
                step,
                "running 1 test\ntest t ... FAILED\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n",
            );
            check("failed tests", &w);
            rewrite_stdout(&w, step, "no test result line\n");
            check("tests not performed", &w);
            rewrite_stdout(
                &w,
                step,
                "running 1 test\ntest t ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n\n",
            );
            assert!(post(&w).is_qualified(), "stdout restored");
        }
        // Unperformed: custody never ran it.
        edit_summary(&w, |s| {
            s["steps"]
                .as_array_mut()
                .unwrap()
                .retain(|r| r["name"] != step);
            s["not_run"] = json!([step]);
            s["status"] = json!("failed");
        });
        check("unperformed", &w);
        fs::write(w.records().join("summary.json"), summary).unwrap();
        assert!(post(&w).is_qualified(), "{step} restored");
    }
}

// ---- R6: complete evidence semantics ---------------------------------------------------------

/// Hash-consistent Cargo terminal corruption: the build record, summary row, stdout length/hash
/// and executable list all stay coherent; only the terminal semantics change.
#[test]
fn cargo_terminal_semantics_refuse_hash_consistent_corruption() {
    let w = full("r6-cargo");
    let build = fs::read_to_string(w.records().join("build.stdout")).unwrap();
    let terminal = "{\"reason\":\"build-finished\",\"success\":true}\n";
    assert!(build.ends_with(terminal));
    let head = build.strip_suffix(terminal).unwrap().to_owned();
    let artifact_line = build.lines().nth(1).unwrap().to_owned();
    let variants: Vec<(&str, String)> = vec![
        ("terminal missing", head.clone()),
        (
            "success false",
            format!("{head}{{\"reason\":\"build-finished\",\"success\":false}}\n"),
        ),
        (
            "success absent",
            format!("{head}{{\"reason\":\"build-finished\"}}\n"),
        ),
        (
            "success wrong type",
            format!("{head}{{\"reason\":\"build-finished\",\"success\":\"true\"}}\n"),
        ),
        ("duplicate terminal", format!("{head}{terminal}{terminal}")),
        (
            "contradictory terminal",
            format!("{head}{terminal}{{\"reason\":\"build-finished\",\"success\":false}}\n"),
        ),
        (
            "artifact after terminal",
            format!("{head}{terminal}{artifact_line}\n"),
        ),
        ("empty output", String::new()),
    ];
    for (what, text) in variants {
        rewrite_stdout(&w, "build", &text);
        let e = post(&w);
        assert!(
            !e.passed("record:build") && !e.is_qualified(),
            "{what}: a hash-consistent cargo terminal defect must refuse"
        );
        rewrite_stdout(&w, "build", &build);
    }
    assert!(post(&w).is_qualified());
}

/// Bytes after a valid observation, however well- or ill-formed, refuse; recomputation never
/// substitutes for the observed stream.
#[test]
fn bytes_after_the_observation_refuse_in_the_positive_bundle() {
    let w = full("r6-trailing");
    const R: &str = "native-frames-received.jsonl";
    let recv = read_out(&w, R);
    let obs_line: Vec<u8> = recv.split(|b| *b == b'\n').nth(1).unwrap().to_vec();
    let mut extras: Vec<(&str, Vec<u8>)> = vec![
        (
            "duplicate observation",
            [obs_line.clone(), b"\n".to_vec()].concat(),
        ),
        ("malformed", b"{\"type\":\"observation\", nope\n".to_vec()),
        (
            "error frame",
            b"{\"type\":\"error\",\"message\":\"late\"}\n".to_vec(),
        ),
        ("invalid utf-8", vec![0x7b, 0xff, 0xfe, 0x7d, 0x0a]),
        (
            "incomplete",
            b"{\"type\":\"observation\",\"session_id\":\"cut".to_vec(),
        ),
    ];
    extras.push(("empty frame", b"\n".to_vec()));
    for (what, extra) in extras {
        rewrite_output(&w, R, &[recv.clone(), extra].concat());
        assert!(!post(&w).passed("native_protocol"), "{what}");
        restore(&w, R, &recv);
    }
    // A complete frame without its newline is an incomplete trailing frame too.
    rewrite_output(&w, R, recv.strip_suffix(b"\n").unwrap());
    assert!(!post(&w).passed("native_protocol"));
    restore(&w, R, &recv);
}

fn control_dir(w: &W, name: &str) -> PathBuf {
    PathBuf::from(&w.run.run_dir).join("controls").join(name)
}

/// Rewrites one retained file of a control bundle and keeps that bundle's manifest exact.
fn rewrite_control(w: &W, ctrl: &str, name: &str, bytes: &[u8]) {
    let dir = control_dir(w, ctrl);
    fs::write(dir.join(name), bytes).unwrap();
    let mp = dir.join("output-manifest.json");
    let mut m: Value = serde_json::from_slice(&fs::read(&mp).unwrap()).unwrap();
    for f in m["files"].as_array_mut().unwrap() {
        if f["name"] == name {
            f["length"] = json!(bytes.len());
            f["sha256"] = json!(sha256_hex(bytes));
        }
    }
    fs::write(&mp, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
}

fn edit_control_json(w: &W, ctrl: &str, name: &str, f: impl FnOnce(&mut Value)) {
    let mut v: Value =
        serde_json::from_slice(&fs::read(control_dir(w, ctrl).join(name)).unwrap()).unwrap();
    f(&mut v);
    rewrite_control(w, ctrl, name, &serde_json::to_vec_pretty(&v).unwrap());
}

/// Every applicable semantic role in each failure control: malformed, well-formed foreign and
/// contradictory evidence refuses; the unmodified control still qualifies.
#[test]
fn failure_controls_get_complete_semantic_evaluation() {
    let w = full("r6-controls");
    for ctrl in ["assertion-unwind", "write-failure"] {
        let id = format!("control:{ctrl}");
        let mut roles: Vec<(&str, Vec<u8>)> = vec![];
        for name in [
            "observation.json",
            "request-context.json",
            "native-history.json",
            "consumption.json",
            "revisions.json",
            "collection.json",
            "process.json",
            "capture.json",
            "identities.json",
        ] {
            roles.push((name, b"{not json".to_vec()));
        }
        roles.push((
            "native-frames-received.jsonl",
            b"{\"type\":\"allocated\"}\n".to_vec(),
        ));
        roles.push((
            "native-frames-received.jsonl",
            [
                fs::read(control_dir(&w, ctrl).join("native-frames-received.jsonl")).unwrap(),
                b"{garbage\n".to_vec(),
            ]
            .concat(),
        ));
        for (name, bytes) in roles {
            let original = fs::read(control_dir(&w, ctrl).join(name)).unwrap();
            rewrite_control(&w, ctrl, name, &bytes);
            assert!(
                post(&w).failed(&id),
                "{ctrl}/{name}: malformed or foreign bytes must refuse"
            );
            rewrite_control(&w, ctrl, name, &original);
            assert!(post(&w).passed(&id), "{ctrl}/{name} restored");
        }
        // Well-formed but foreign or contradictory.
        type Edit = Box<dyn Fn(&mut Value)>;
        let edits: Vec<(&str, Edit)> = vec![
            (
                "observation.json",
                Box::new(|v| v["session_id"] = json!("foreign-session")),
            ),
            (
                "observation.json",
                Box::new(|v| v["request_count"] = json!(2)),
            ),
            (
                "request-context.json",
                Box::new(|v| v["messages"][1]["content"][0]["text"] = json!("foreign contract")),
            ),
            (
                "native-history.json",
                Box::new(|v| v[2]["content"][0]["text"] = json!("foreign result")),
            ),
            (
                "revisions.json",
                Box::new(|v| v["after"] = json!("0".repeat(64))),
            ),
            (
                "revisions.json",
                Box::new(|v| v["expected"] = json!("1".repeat(64))),
            ),
            (
                "consumption.json",
                Box::new(|v| v["revision"] = json!("2".repeat(64))),
            ),
            (
                "collection.json",
                Box::new(|v| v["consumer_invocations"] = json!(0)),
            ),
            (
                "collection.json",
                Box::new(|v| v["stdout"]["end"] = json!({"kind": "total_limit"})),
            ),
            (
                "collection.json",
                Box::new(|v| v["stdout"]["end"] = json!("Eof")),
            ),
            (
                "collection.json",
                Box::new(|v| v["stdout"]["retained_bytes"] = json!("5")),
            ),
            (
                "collection.json",
                Box::new(|v| v["stderr"]["end"] = json!({"kind": "cancelled"})),
            ),
            (
                "collection.json",
                Box::new(|v| v["sent"][0]["outcome"] = json!({"kind": "failed", "error": "x"})),
            ),
            (
                "collection.json",
                Box::new(|v| v["sent"][0]["error"] = json!(null)),
            ),
            (
                "collection.json",
                Box::new(|v| {
                    v["sent"][0].as_object_mut().unwrap().remove("written");
                }),
            ),
            (
                "collection.json",
                Box::new(|v| v["sent"][0]["frame_sha256"] = json!("a".repeat(64))),
            ),
            ("collection.json", Box::new(|v| v["sent"] = json!([]))),
            (
                "collection.json",
                Box::new(|v| v["incompleteness"] = json!(["made-up reason"])),
            ),
            (
                "collection.json",
                Box::new(|v| v["frame_kinds"] = json!(["allocated"])),
            ),
            ("collection.json", Box::new(|v| *v = json!([]))),
            ("capture.json", Box::new(|v| *v = json!([]))),
            ("process.json", Box::new(|v| *v = json!([]))),
            (
                "process.json",
                Box::new(|v| v["reap"] = json!({"kind": "exited", "code": 3})),
            ),
            ("process.json", Box::new(|v| v["timed_out"] = json!(true))),
            (
                "capture.json",
                Box::new(|v| v["stage"] = json!("delivery_refused")),
            ),
            (
                "capture.json",
                Box::new(|v| v["consumer_invocations"] = json!(0)),
            ),
            (
                "capture.json",
                Box::new(|v| {
                    v["absent"] = json!([{"name": "observation.json", "reason": "claimed"}])
                }),
            ),
            (
                "identities.json",
                Box::new(|v| v["invocation_id"] = json!("foreign")),
            ),
        ];
        for (name, edit) in edits {
            let original = fs::read(control_dir(&w, ctrl).join(name)).unwrap();
            edit_control_json(&w, ctrl, name, edit);
            assert!(
                post(&w).failed(&id),
                "{ctrl}/{name}: contradictory evidence must refuse"
            );
            rewrite_control(&w, ctrl, name, &original);
            assert!(post(&w).passed(&id), "{ctrl}/{name} restored");
        }
    }
    assert!(post(&w).is_qualified());
}

/// Approved absences only: the write-failure control may lack the injected file and nothing
/// else; an unapproved absence and a bundle that tries to waive an obligation refuse.
#[test]
fn only_approved_scenario_absences_are_admissible() {
    let w = full("r6-absence");
    let dir = control_dir(&w, "write-failure");
    for victim in [
        "consumption.json",
        "observation.json",
        "native-history.json",
    ] {
        let original = fs::read(dir.join(victim)).unwrap();
        let mp = dir.join("output-manifest.json");
        let manifest = fs::read(&mp).unwrap();
        let mut m: Value = serde_json::from_slice(&manifest).unwrap();
        m["files"]
            .as_array_mut()
            .unwrap()
            .retain(|f| f["name"] != victim);
        fs::remove_file(dir.join(victim)).unwrap();
        fs::write(&mp, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        // Even a capture.json that honestly lists the absence cannot waive the obligation.
        edit_control_json(&w, "write-failure", "capture.json", |v| {
            v["absent"] = json!([{"name": victim, "reason": "self-declared partial"}])
        });
        assert!(
            post(&w).failed("control:write-failure"),
            "{victim}: a self-declared absence is not an approved one"
        );
        fs::write(dir.join(victim), &original).unwrap();
        fs::write(&mp, &manifest).unwrap();
        let cap = serde_json::to_vec_pretty(&json!({"stage": "delivered_and_consumed", "error": null, "absent": [], "consumer_invocations": 1})).unwrap();
        rewrite_control(&w, "write-failure", "capture.json", &cap);
    }
}

#[test]
fn control_stage_is_approved_by_the_map_not_the_bundle() {
    let w = world_with("r6-stage", |m| m["controls"][0]["stage"] = json!("bogus"));
    let e = pre(&w);
    assert!(
        !e.is_qualified(),
        "an unknown scenario stage cannot be approved"
    );
}

/// Refusal stage (approved by the map for this control): the consumer is never invoked, the
/// consumption trio is the only approved absence, and the refusal must be justified by the
/// retained process/collection/stream evidence rather than by the bundle's own label.
#[test]
fn a_delivery_refusal_control_is_justified_by_evidence_and_shows_no_consumption() {
    let w = full_edit("r6-refusal", |m| {
        m["controls"][0]["stage"] = json!("delivery_refused");
        m["controls"][0]["preserved"]
            .as_array_mut()
            .unwrap()
            .retain(|p| {
                !["episode-after.json", "consumption.json", "revisions.json"]
                    .contains(&p.as_str().unwrap())
            });
    });
    let ctrl = "assertion-unwind";
    let dir = control_dir(&w, ctrl);
    let mp = dir.join("output-manifest.json");
    let mut m: Value = serde_json::from_slice(&fs::read(&mp).unwrap()).unwrap();
    for victim in ["episode-after.json", "consumption.json", "revisions.json"] {
        m["files"]
            .as_array_mut()
            .unwrap()
            .retain(|f| f["name"] != victim);
        fs::remove_file(dir.join(victim)).unwrap();
    }
    fs::write(&mp, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let id = format!("control:{ctrl}");
    edit_control_json(&w, ctrl, "capture.json", |v| {
        v["stage"] = json!("delivery_refused");
        v["outcome"] =
            json!({"stage":"delivery_refused","native_started":true,"native_error":null});
        v["error"] = json!(
            "delivery refused: incomplete native attempt: child did not reap as a clean exit: Exited { code: 3 }"
        );
        v["consumer_invocations"] = json!(0);
        v["absent"] = json!([{"name": "episode-after.json", "reason": "no consumption"}, {"name": "consumption.json", "reason": "no consumption"}, {"name": "revisions.json", "reason": "no consumption"}]);
    });
    edit_control_json(&w, ctrl, "collection.json", |v| {
        v["consumer_invocations"] = json!(0)
    });
    // Unjustified so far: the attempt in the retained evidence was complete and verified.
    assert!(
        post(&w).failed(&id),
        "a refusal its own evidence does not support must refuse"
    );
    edit_control_json(&w, ctrl, "identities.json", |v| {
        v["effective_system_context_sha256"] = Value::Null;
        v["verified_result_text_sha256"] = Value::Null;
    });
    // Justified by the retained process disposition (nonzero exit): qualifies.
    let original_process = fs::read(dir.join("process.json")).unwrap();
    edit_control_json(&w, ctrl, "process.json", |v| {
        v["reap"] = json!({"kind": "exited", "code": "3"})
    });
    resync_collection_tolerant(&dir);
    assert!(
        post(&w).failed(&id),
        "a wrong-type exit is malformed evidence, never a valid refusal justification"
    );
    rewrite_control(&w, ctrl, "process.json", &original_process);
    edit_control_json(&w, ctrl, "process.json", |v| {
        v["reap"] = json!({"kind": "exited", "code": 3})
    });
    resync_collection(&dir);
    {
        let e = post(&w);
        assert!(
            e.passed(&id),
            "a justified refusal without consumption qualifies: {}",
            e.to_json("post")["checks"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["id"] == id.as_str())
                .map(|c| c.to_string())
                .collect::<String>()
        );
    }
    // A refusal that claims the consumer was invoked contradicts its stage.
    let cap = fs::read(dir.join("capture.json")).unwrap();
    edit_control_json(&w, ctrl, "capture.json", |v| {
        v["consumer_invocations"] = json!(1)
    });
    assert!(post(&w).failed(&id));
    rewrite_control(&w, ctrl, "capture.json", &cap);
    // A refusal bundle that still carries a consumption artifact keeps it to its owning semantics
    // only if approved; an unrelated absence (the observation) is not approved here when the stream carried one.
    let obs = fs::read(dir.join("observation.json")).unwrap();
    let mut m2: Value = serde_json::from_slice(&fs::read(&mp).unwrap()).unwrap();
    m2["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["name"] != "observation.json");
    let manifest_before = fs::read(&mp).unwrap();
    fs::write(&mp, serde_json::to_vec_pretty(&m2).unwrap()).unwrap();
    fs::remove_file(dir.join("observation.json")).unwrap();
    assert!(
        post(&w).failed(&id),
        "observation absent although the received stream carried one"
    );
    fs::write(dir.join("observation.json"), &obs).unwrap();
    fs::write(&mp, &manifest_before).unwrap();
    assert!(post(&w).passed(&id));
}

#[test]
fn a_refusal_stage_cannot_preserve_consumption_artifacts() {
    let w = world_with("r6-stage-preserved", |m| {
        m["controls"][0]["stage"] = json!("delivery_refused")
    });
    assert!(
        !pre(&w).is_qualified(),
        "preserved consumption artifacts contradict a refusal stage"
    );
}

/// Approved refusal scenarios need the owner's meaning, not the bundle's label: a consumer
/// refusal must be the existing consumer's refusal of exactly the retained inputs, a setup
/// refusal carries a setup/spawn owner error and no consumption, and a delivery refusal is
/// justified only by consistent evidence (malformed or contradictory facts never justify it).
#[test]
fn refusal_scenarios_need_owner_meaning_not_labels() {
    let w = full("r6-refusal-scenarios");
    for c in ["delivery-refusal", "setup-refusal", "consumer-refusal"] {
        assert!(post(&w).passed(&format!("control:{c}")), "{c} baseline");
    }
    let check = |ctrl: &str, name: &str, edit: Box<dyn Fn(&mut Value)>, why: &str| {
        let id = format!("control:{ctrl}");
        let original = fs::read(control_dir(&w, ctrl).join(name)).unwrap();
        edit_control_json(&w, ctrl, name, edit);
        assert!(post(&w).failed(&id), "{ctrl}/{name}: {why}");
        rewrite_control(&w, ctrl, name, &original);
        assert!(post(&w).passed(&id), "{ctrl}/{name} restored");
    };
    let before = begun().revision().0.clone();
    check(
        "consumer-refusal",
        "consumer-inputs.json",
        Box::new(move |v| v["expected_revision"] = json!(before.clone())),
        "the owner accepts these inputs: claimed refusal it does not produce",
    );
    check(
        "consumer-refusal",
        "capture.json",
        Box::new(|v| v["error"] = json!("consumer refused: some other refusal")),
        "not the owner's refusal text",
    );
    check(
        "consumer-refusal",
        "capture.json",
        Box::new(|v| v["consumer_invocations"] = json!(0)),
        "a consumer refusal is an invocation that refused",
    );
    check(
        "consumer-refusal",
        "process.json",
        Box::new(|v| v["reap"] = json!({"kind": "exited", "code": 1})),
        "consumer refusal needs a complete delivery",
    );
    check(
        "setup-refusal",
        "capture.json",
        Box::new(|v| v["error"] = json!("delivery refused: no")),
        "not a setup/spawn owner error",
    );
    check(
        "setup-refusal",
        "capture.json",
        Box::new(|v| v["consumer_invocations"] = json!(1)),
        "no consumption without a child",
    );
    check(
        "setup-refusal",
        "capture.json",
        Box::new(|v| v["absent"] = json!([])),
        "absences must equal the missing native artifacts",
    );
    check(
        "delivery-refusal",
        "collection.json",
        Box::new(|v| v["incompleteness"] = json!([])),
        "contradictory incompleteness",
    );
    check(
        "delivery-refusal",
        "process.json",
        Box::new(|v| v["reap"] = json!({"kind": "exited", "code": "5"})),
        "a wrong-type exit is malformed, not a refusal",
    );
    check(
        "delivery-refusal",
        "capture.json",
        Box::new(|v| v["consumer_invocations"] = json!(1)),
        "a delivery refusal never invokes the consumer",
    );
    check(
        "delivery-refusal",
        "collection.json",
        Box::new(|v| v["stdout"]["end"] = json!({"kind": "frame_limit"})),
        "a limit the retained bytes do not show",
    );
    assert!(post(&w).is_qualified());
}

fn runtime_world(name: &str) -> W {
    use std::os::unix::fs::{symlink, PermissionsExt};
    world_with(name, |m| {
        let node = m["classes"]["node"][0].as_str().unwrap().to_owned();
        let dir = Path::new(&node).parent().unwrap();
        let real = dir.join("real-tools");
        fs::create_dir(&real).unwrap();
        put(&real.join("tool"), b"same executable bytes");
        fs::set_permissions(real.join("tool"), fs::Permissions::from_mode(0o755)).unwrap();
        symlink("real-tools", dir.join("tool-dir")).unwrap();
        symlink("tool-dir/tool", dir.join("command-a")).unwrap();
        symlink("command-a", dir.join("command-b")).unwrap();
        let argv = vec![dir.join("command-b").display().to_string(), "arg".into()];
        let (links, endpoint) = runtime::resolve(Path::new(&argv[0])).unwrap();
        let invocation = runtime::Invocation {
            id: "node-top".into(),
            source: None,
            argv: argv.clone(),
            cwd: m["processes"][2]["cwd"].as_str().unwrap().into(),
            path_env: Some("/usr/bin".into()),
            lookup: argv[0].clone(),
            links: links.clone(),
            endpoint: endpoint.clone(),
        };
        m["processes"][2]["argv"] = json!(argv);
        m["processes"][2]["runtime_inputs"] = json!([invocation]);
        for p in links.iter().chain(std::iter::once(&endpoint)) {
            m["external_inputs"][p] =
                serde_json::to_value(runtime::ObjectBinding::observe(Path::new(p)).unwrap())
                    .unwrap();
            m["classes"]["node"].as_array_mut().unwrap().push(json!(p));
        }
    })
}

#[test]
fn r7_link_chain_directory_links_and_actual_custody_shape_qualify() {
    let w = runtime_world("r7-chain");
    assert!(pre(&w).is_qualified());
    execute(&w);
    write_outputs(&w);
    assert!(
        post(&w).is_qualified(),
        "{}",
        post(&w).to_json("postflight")
    );
}

#[test]
fn r7_changed_endpoint_or_link_identity_refuses_preflight() {
    for changed in ["endpoint", "link", "dangling", "cyclic"] {
        let w = runtime_world(&format!("r7-{changed}"));
        let m: Value = serde_json::from_slice(&fs::read(&w.map_path).unwrap()).unwrap();
        let r: &Value = &m["processes"][2]["runtime_inputs"][0];
        if changed == "endpoint" {
            fs::write(r["endpoint"].as_str().unwrap(), b"changed bytes").unwrap();
        } else {
            let link = Path::new(r["links"][0].as_str().unwrap());
            fs::remove_file(link).unwrap();
            let target = match changed {
                "link" => "tool-dir/tool",
                "dangling" => "absent",
                _ => "command-b",
            };
            std::os::unix::fs::symlink(target, link).unwrap();
        }
        assert!(!pre(&w).is_qualified(), "{changed}");
    }
}

#[test]
fn r7_missing_endpoint_and_contradictory_custody_kind_refuse_postflight() {
    for control in ["missing", "kind", "link-text"] {
        let w = runtime_world(&format!("r7-record-{control}"));
        execute(&w);
        write_outputs(&w);
        let m: Value = serde_json::from_slice(&fs::read(&w.map_path).unwrap()).unwrap();
        let r = &m["processes"][2]["runtime_inputs"][0];
        edit_record(&w, "node-version", |rec| {
            let inputs = rec["inputs"].as_array_mut().unwrap();
            if control == "missing" {
                inputs.retain(|e| e["path"] != r["endpoint"]);
            } else {
                let e = inputs
                    .iter_mut()
                    .find(|e| e["path"] == r["links"][0])
                    .unwrap();
                if control == "kind" {
                    e["type"] = json!("file");
                } else {
                    e["link_target"] = json!("wrong");
                }
            }
        });
        assert!(!post(&w).is_qualified(), "{control}");
    }
}

#[test]
fn r7_regular_pin_at_link_is_unproducible_and_same_bytes_do_not_merge_invocations() {
    let mut w = runtime_world("r7-identities");
    let m: Value = serde_json::from_slice(&fs::read(&w.map_path).unwrap()).unwrap();
    let a: runtime::Invocation =
        serde_json::from_value(m["processes"][2]["runtime_inputs"][0].clone()).unwrap();
    let objects = serde_json::from_value(m["external_inputs"].clone()).unwrap();
    let mut b = a.clone();
    b.id = "other-command".into();
    b.argv[0] = Path::new(&a.lookup)
        .parent()
        .unwrap()
        .join("command-a")
        .display()
        .to_string();
    b.lookup = b.argv[0].clone();
    b.links = runtime::resolve(Path::new(&b.lookup)).unwrap().0;
    assert_ne!(a, b);
    assert_eq!(a.endpoint, b.endpoint);
    a.validate(&objects, None).unwrap();
    b.validate(&objects, None).unwrap();
    let link = Path::new(&a.links[0]);
    let bad = runtime::ObjectBinding::File {
        sha256: file_sha(link).unwrap(),
    };
    assert_ne!(runtime::ObjectBinding::observe(link).unwrap(), bad);
    edit_map(&mut w, |m| {
        m["external_inputs"][a.links[0].clone()] = serde_json::to_value(bad).unwrap()
    });
    assert!(!pre(&w).is_qualified());
}

#[test]
fn r7_unchanged_real_custody_producer_agrees_with_typed_resolution() {
    let w = runtime_world("r7-real-producer");
    let m: Value = serde_json::from_slice(&fs::read(&w.map_path).unwrap()).unwrap();
    let r: runtime::Invocation =
        serde_json::from_value(m["processes"][2]["runtime_inputs"][0].clone()).unwrap();
    let helper = "/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py";
    let out = w.dir.join("actual-producer");
    let mut cmd = std::process::Command::new("/usr/bin/python3");
    cmd.args(["-I", helper, "run", "--out"])
        .arg(&out)
        .args(["--name", "agreement", "--cwd"])
        .arg(&w.dir)
        .args([
            "--env-clear",
            "--env",
            "PATH=/usr/bin:/bin",
            "--digest",
            helper,
        ]);
    for p in r.links.iter().chain(std::iter::once(&r.endpoint)) {
        cmd.args(["--digest", p]);
    }
    let status = cmd.args(["--", "/usr/bin/true"]).status().unwrap();
    assert!(status.success());
    let actual: Value =
        serde_json::from_slice(&fs::read(out.join("agreement.json")).unwrap()).unwrap();
    assert_eq!(actual["returncode"], 0);
    assert_eq!(actual["leftover_processes"], json!([]));
    let captured = actual["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                v["path"].as_str().unwrap().to_owned(),
                runtime::ObjectBinding::from_custody(v).unwrap(),
            )
        })
        .collect();
    r.validate(
        &serde_json::from_value(m["external_inputs"].clone()).unwrap(),
        Some(&captured),
    )
    .unwrap();
}

fn edit_map(w: &mut W, f: impl FnOnce(&mut Value)) {
    let mut m: Value = serde_json::from_slice(&fs::read(&w.map_path).unwrap()).unwrap();
    f(&mut m);
    put(&w.map_path, &serde_json::to_vec_pretty(&m).unwrap());
    w.run.map_sha256 = file_sha(&w.map_path).unwrap();
    let manifest = build_manifest(&m, &w.run.map_sha256).unwrap();
    put(
        &w.manifest_path,
        &serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    w.run.manifest_sha256 = file_sha(&w.manifest_path).unwrap();
    put(
        &w.plan_path,
        &serde_json::to_vec_pretty(&make_plan(&m, &w.run).unwrap()).unwrap(),
    );
}

#[test]
fn r7_relative_and_path_lookup_preserve_command_arguments_and_context() {
    let w = runtime_world("r7-lookup-context");
    let m: Value = serde_json::from_slice(&fs::read(&w.map_path).unwrap()).unwrap();
    let a: runtime::Invocation =
        serde_json::from_value(m["processes"][2]["runtime_inputs"][0].clone()).unwrap();
    let pins = serde_json::from_value(m["external_inputs"].clone()).unwrap();
    let mut r = a.clone();
    r.cwd = Path::new(&a.lookup).parent().unwrap().display().to_string();
    r.argv[0] = "command-b".into();
    r.path_env = Some(r.cwd.clone());
    r.validate(&pins, None).unwrap();
    r.argv[0] = "./command-b".into();
    r.lookup = format!("{}/./command-b", r.cwd);
    r.validate(&pins, None).unwrap();
    r.lookup = a.endpoint.clone();
    assert!(r.validate(&pins, None).is_err());
    r.lookup = format!("{}/./command-b", r.cwd);
    r.links.clear();
    assert!(r.validate(&pins, None).is_err());
}

/// Add a genuinely supplied artifact with correct surrounding accounting and absence list.
fn supply_control(w: &W, ctrl: &str, name: &str, bytes: &[u8]) {
    let dir = control_dir(w, ctrl);
    fs::write(dir.join(name), bytes).unwrap();
    let mp = dir.join("output-manifest.json");
    let mut m: Value = serde_json::from_slice(&fs::read(&mp).unwrap()).unwrap();
    m["files"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":name,"length":bytes.len(),"sha256":sha256_hex(bytes)}));
    fs::write(mp, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    edit_control_json(w, ctrl, "capture.json", |v| {
        v["absent"]
            .as_array_mut()
            .unwrap()
            .retain(|a| a["name"] != name)
    });
    touch_all(&dir);
}

#[test]
fn r8_stage_composition_refuses_hash_consistent_surviving_contradictions() {
    // Exact WR7-002: [] parses as JSON, hashes/lengths and the absence list all agree.
    let w = full("r8-wr7-002");
    assert!(post(&w).passed("control:setup-refusal"));
    supply_control(&w, "setup-refusal", "process.json", b"[]");
    assert!(post(&w).failed("control:setup-refusal"));
    // Well-formed native evidence also contradicts no native start.
    let w = full("r8-native-after-setup");
    for name in [
        "process.json",
        "collection.json",
        "native-frames-received.jsonl",
        "native-frames-sent.jsonl",
        "stderr.txt",
        "observation.json",
        "request-context.json",
        "native-history.json",
    ] {
        supply_control(&w, "setup-refusal", name, &read_out(&w, name));
    }
    assert!(post(&w).failed("control:setup-refusal"));
    for ctrl in ["setup-refusal", "delivery-refusal", "consumer-refusal"] {
        let w = full(&format!("r8-consumption-after-{ctrl}"));
        for name in ["episode-after.json", "consumption.json", "revisions.json"] {
            supply_control(&w, ctrl, name, &read_out(&w, name));
        }
        assert!(post(&w).failed(&format!("control:{ctrl}")));
    }
    let w = full("r8-unapproved-support");
    supply_control(&w, "setup-refusal", "diagnostic.json", b"{}");
    assert!(post(&w).failed("control:setup-refusal"));
    let w = full("r8-unlisted-file");
    fs::write(control_dir(&w, "setup-refusal").join("hidden.json"), b"{}").unwrap();
    assert!(post(&w).failed("control:setup-refusal"));
}

/// Rebuild a delivery-refusal bundle using the ACTUAL capture projection of a labelled
/// synthetic NativeRun. It covers unavailable collection without pretending the child never ran.
fn replace_refusal(w: &W, mut run: NativeRun) {
    let fx = owner_fixture();
    // Keep the candidate fields exactly the owner interpretation of retained bytes.
    let i = run.interpretation();
    run.bound = i
        .allocation
        .and_then(|a| BoundAttempt::bind(fx.binding.frozen().clone(), a).ok());
    run.observation = i.observation;
    run.error_frame = None;
    let why = run.incompleteness();
    let refusal = if why.is_empty() {
        run.bound
            .as_ref()
            .unwrap()
            .verify(
                run.observation.as_ref().unwrap(),
                fx.binding.synthetic_result(),
            )
            .unwrap_err()
    } else {
        DeliveryRefusal::Incomplete(why.join("; "))
    };
    let outcome = Err(DeliverError::Delivery {
        refusal,
        run: Box::new(run),
    });
    let dir = control_dir(w, "delivery-refusal");
    fs::remove_dir_all(&dir).unwrap();
    let mut ev = Evidence::create_at(&w.run_env(), dir.clone()).unwrap();
    capture::persist_inputs(&mut ev, &fx.binding, AUTHORITY, &[]);
    capture::capture_returned(&mut ev, &fx.binding, &outcome, 0);
    assert_eq!(ev.finish("passed").unwrap(), "passed");
    touch_all(&dir);
}

#[test]
fn r8_actual_capture_projects_optional_context_partial_and_unknown_collection() {
    let w = full("r8-optional-context");
    let fx = owner_fixture();
    let mut run = fx.delivered.run;
    let mut obs = run.observation.clone().unwrap();
    obs.context = None;
    let allocation = run.bound.as_ref().unwrap().allocation();
    let mut a = serde_json::to_value(allocation).unwrap();
    a["type"] = json!("allocated");
    let mut o = serde_json::to_value(obs).unwrap();
    o["type"] = json!("observation");
    run.raw_stdout = format!("{a}\n{o}\n").into_bytes();
    run.facts.stdout.retained_bytes = run.raw_stdout.len() as u64;
    replace_refusal(&w, run);
    assert!(
        post(&w).passed("control:delivery-refusal"),
        "missing optional context is valid refusal evidence: {}",
        post(&w).to_json("post")
    );
    let w = full("r8-partial-unknown");
    let mut run = owner_fixture().delivered.run;
    run.raw_stdout = b"{\"type\":\"alloc".to_vec();
    run.raw_stderr = b"retained-prefix".to_vec();
    run.facts.stdout.retained_bytes = run.raw_stdout.len() as u64;
    run.facts.stdout.end = StreamEnd::Cancelled;
    run.facts.stderr.retained_bytes = run.raw_stderr.len() as u64;
    run.facts.stderr.end = StreamEnd::Cancelled;
    run.facts.process.reap = ReapFact::NotReaped;
    run.facts.process.pid_present_after_reap = None;
    replace_refusal(&w, run);
    assert!(
        post(&w).passed("control:delivery-refusal"),
        "started, partial, unknown collection remains a refusal attempt"
    );
    let cap: Value = serde_json::from_slice(
        &fs::read(control_dir(&w, "delivery-refusal").join("capture.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(cap["outcome"]["native_started"], true);
    assert_ne!(cap["stage"], "native_error");
    edit_control_json(&w, "delivery-refusal", "process.json", |p| *p = json!([]));
    assert!(
        post(&w).failed("control:delivery-refusal"),
        "malformed attempt facts cannot qualify as an expected failure"
    );
}

#[test]
fn r8_recursive_duplicate_wire_refuses_positive_but_valid_refusal_bundle_qualifies() {
    for target in ["allocation", "observation", "context", "history"] {
        for conflict in [false, true] {
            for last in [false, true] {
                let w = full(&format!("r8-duplicate-{target}-{conflict}-{last}"));
                let raw = String::from_utf8(read_out(&w, "native-frames-received.jsonl")).unwrap();
                let mut lines: Vec<Value> = raw
                    .lines()
                    .map(|l| serde_json::from_str(l).unwrap())
                    .collect();
                let obj = match target {
                    "allocation" => &mut lines[0],
                    "observation" => &mut lines[1],
                    "context" => &mut lines[1]["context"],
                    _ => &mut lines[1]["history"][0],
                };
                let key = match target {
                    "allocation" => "sdk_version",
                    "observation" => "request_count",
                    "context" => "messages",
                    _ => "role",
                };
                let original = obj.to_string();
                let value = if conflict {
                    json!("contradictory")
                } else {
                    obj[key].clone()
                };
                let member = format!("{}:{}", json!(key), value);
                let modified = if last {
                    format!("{},{}{}", &original[..original.len() - 1], member, "}")
                } else {
                    format!("{}{},{}", "{", member, &original[1..])
                };
                let mutated = lines
                    .iter()
                    .map(Value::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
                    .replace(&original, &modified)
                    + "\n";
                rewrite_output(&w, "native-frames-received.jsonl", mutated.as_bytes());
                resync_collection(&w.evidence());
                assert!(post(&w).failed("native_protocol"));
                let mut run = owner_fixture().delivered.run;
                run.raw_stdout = mutated.into_bytes();
                run.facts.stdout.retained_bytes = run.raw_stdout.len() as u64;
                replace_refusal(&w, run);
                assert!(
                    post(&w).passed("control:delivery-refusal"),
                    "{target}/{conflict}/{last}: expected malformed-protocol refusal evidence: {}",
                    post(&w).to_json("post")
                );
            }
        }
    }
}

#[test]
fn r9_write_failure_cannot_hide_surviving_entries() {
    for kind in ["regular", "symlink", "nonempty-directory"] {
        let w = full(&format!("r9-remnant-{kind}"));
        let p = control_dir(&w, "write-failure").join("episode-after.json");
        fs::remove_dir(&p).unwrap();
        match kind {
            "regular" => fs::write(&p, b"[]").unwrap(),
            "symlink" => {
                std::os::unix::fs::symlink(w.evidence().join("episode-after.json"), &p).unwrap()
            }
            _ => {
                fs::create_dir(&p).unwrap();
                fs::write(p.join("surviving-state"), b"[]").unwrap();
            }
        }
        assert!(
            post(&w).failed("control:write-failure"),
            "{kind} cannot be erased by a declaration"
        );
    }
}

#[test]
fn r9_failure_support_requires_meaning_and_exact_retained_scratch() {
    type FailureEdit = (&'static str, fn(&mut Value));
    let edits: [FailureEdit; 7] = [
        ("null-fields", |v| {
            v["stage"] = Value::Null;
            v["message"] = Value::Null;
            v["scratch"] = Value::Null;
            v["scratch_listing_before_cleanup"] = json!([null]);
        }),
        ("wrong-stage", |v| v["stage"] = json!("before_invocation")),
        ("empty-message", |v| v["message"] = json!("  ")),
        ("wrong-scratch", |v| {
            v["scratch"] = json!("/unrelated/scratch")
        }),
        ("wrong-length", |v| {
            v["scratch_listing_before_cleanup"][0]["length"] = json!(999)
        }),
        ("traversal", |v| {
            v["scratch_listing_before_cleanup"][0]["path"] = json!("../home/x")
        }),
        ("wrong-listing", |v| {
            v["scratch_listing_before_cleanup"][0]["path"] = json!("missing")
        }),
    ];
    for (name, edit) in edits {
        let w = full(&format!("r9-failure-{name}"));
        assert!(post(&w).is_qualified(), "valid supported failure baseline");
        edit_control_json(&w, "assertion-unwind", "failure.json", edit);
        assert!(post(&w).failed("control:assertion-unwind"), "{name}");
    }
}

#[test]
fn r9_scratch_identity_excludes_siblings_and_requires_one_real_directory() {
    let w = full("r9-scratch-siblings");
    let scratch = PathBuf::from(&w.run.run_dir).join("scratch");
    for sibling in [
        "control-assertion-unwind-per-frame-77",
        "control-assertion-unwind-chunked-77",
    ] {
        put(&scratch.join(sibling).join("home/x"), b"scratch");
    }
    assert!(
        post(&w).is_qualified(),
        "siblings retain their own identity"
    );
    for kind in [
        "missing",
        "empty",
        "nonnumeric",
        "zero",
        "leading-zero",
        "overflow",
        "multiple",
        "file",
        "symlink",
    ] {
        let w = full(&format!("r9-scratch-{kind}"));
        let scratch = PathBuf::from(&w.run.run_dir).join("scratch");
        let original = scratch.join("control-assertion-unwind-77");
        match kind {
            "multiple" => put(
                &scratch.join("control-assertion-unwind-78/home/x"),
                b"scratch",
            ),
            "file" => {
                fs::remove_dir_all(&original).unwrap();
                fs::write(&original, b"scratch").unwrap();
            }
            "symlink" => {
                let retained = scratch.join("retained-original");
                fs::rename(&original, &retained).unwrap();
                std::os::unix::fs::symlink(&retained, &original).unwrap();
            }
            _ => {
                let suffix = match kind {
                    "empty" => "",
                    "nonnumeric" => "per-frame-77",
                    "zero" => "0",
                    "leading-zero" => "077",
                    "overflow" => "4294967296",
                    _ => "gone",
                };
                fs::rename(
                    &original,
                    scratch.join(format!("control-assertion-unwind-{suffix}")),
                )
                .unwrap();
            }
        }
        assert!(
            post(&w).failed("control:assertion-unwind"),
            "{kind} cannot establish retained scratch identity"
        );
    }
}
