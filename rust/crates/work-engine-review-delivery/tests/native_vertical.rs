//! The native vertical (feature `fixture`): one checked `ExperimentBinding` -> actual Pi SDK 1.5.0
//! session (real prompt/auth/agent-loop/history, deterministic provider-disabled stream) ->
//! observed context and native history -> exact synthetic result -> the existing
//! `consume_fixture_result`. These tests REQUIRE the pinned runtime and the custody-supplied run
//! environment and fail (never skip) without them. Every declaration/admission is a synthetic
//! test assumption; nothing here is evidence of a model review, an independent observer, or
//! production admission.
//!
//! Capture and verdict have separate lifetimes (PLAN-R4): inputs are persisted before the native
//! path, every returned observation is persisted before any assertion, the `passed` outcome is
//! finalized only after all assertions, and scratch is removed only after that finalization.
mod common;

use common::capture::{self, TRANSITION_ID};
use common::run_evidence::{Evidence, RunEnv, Scratch, run_env, scratch_root};
use common::world::{World, world};
use common::*;
use review_episode_core::ErrorKind;
use review_episode_core::codec::parse_json;
use review_episode_core::identity::{Authority, Revision};
use review_episode_core::state::{Phase, ReviewEpisodeState};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::path::{Path, PathBuf};
use std::time::Duration;
use work_engine_review_delivery::binding::{EpisodeInputs, ExperimentBinding};
use work_engine_review_delivery::native::{
    AttemptDriver, AttemptFacts, AttemptPolicy, CANARY_MARKER, Control, DeliverError, Delivered,
    DriverStep, KillFact, KillReason, Limits, NativeConfig, ProcessFacts, ReapFact, SendCancel,
    SendOutcome, SendRecord, StreamCollector, StreamEnd, StreamRecord, TimingFacts, Trailing,
    WaitObserver, WriteShape, check_attempt_evidence, deliver_and_consume, run_attempt,
    run_attempt_observed, run_native,
};
use work_engine_review_delivery::{DeliveryRefusal, Transport};
use work_engine_review_preparation::fixture::{ConsumeRefusal, FixtureDeclaration};
use work_engine_review_preparation::prepare;

const SDK_ROOT: &str = "/home/bline/cbm-work/pi-artifact-handoff-20261113/pi";
const SDK_ENTRY_SHA256: &str = "4569157e6038be9840fa514db4b00a27fd0be9e3622cc7b49a281de6c027d7ff";

fn node() -> PathBuf {
    std::env::var_os("PATH")
        .and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|d| d.join("node"))
                .find(|p| p.is_file())
        })
        .expect("node executable is required for the native vertical; this test does not skip")
}

fn native_config(root: PathBuf, control: Control) -> NativeConfig {
    let entry =
        Path::new(SDK_ROOT).join("node_modules/@earendil-works/pi-coding-agent/dist/index.js");
    let bytes = std::fs::read(&entry)
        .expect("pinned Pi SDK 1.5.0 entry must exist; this test does not skip");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        SDK_ENTRY_SHA256,
        "pinned SDK entry bytes"
    );
    NativeConfig {
        node: node(),
        adapter: NativeConfig::default_adapter(),
        sdk_root: PathBuf::from(SDK_ROOT),
        fixture_root: root,
        timeout: Duration::from_secs(60),
        cleanup: Duration::from_secs(5),
        control,
        limits: Limits::default(),
    }
}

/// A refusal control: its outcome is asserted directly; scratch is removed on drop.
fn config(env: &RunEnv, name: &str, control: Control) -> (NativeConfig, Scratch) {
    let root = scratch_root(env, name);
    (native_config(root.clone(), control), Scratch::cleanup(root))
}

struct Case<'a> {
    w: &'a World,
    state: &'a ReviewEpisodeState,
    authority: &'a Authority,
    revision: Revision,
    declaration: &'a FixtureDeclaration,
    admissions: String,
}

fn admissions_text(w: &World) -> String {
    serde_json::to_string(&w.admissions).unwrap()
}

fn base<'a>(w: &'a World, state: &'a ReviewEpisodeState, authority: &'a Authority) -> Case<'a> {
    Case {
        w,
        state,
        authority,
        revision: state.revision().clone(),
        declaration: &w.declaration,
        admissions: admissions_text(w),
    }
}

fn binding_for(case: &Case<'_>, result: &str) -> ExperimentBinding {
    ExperimentBinding::new(
        &case.w.candidate,
        &request(None),
        case.declaration,
        EpisodeInputs {
            state: case.state.clone(),
            authority: case.authority.clone(),
            expected_revision: case.revision.clone(),
            transition_id: TRANSITION_ID.to_owned(),
            unresolved_questions: vec![],
            admissions_json: case.admissions.clone(),
        },
        result,
        Transport {
            attempt_id: "attempt-1".into(),
            generation: 1,
        },
    )
    .expect("the joined fixture world is canonical")
}

/// Owns inputs, invocation, returned evidence, verdict/finalization and scratch eligibility.
/// The guard remains in the caller's assertion scope, including controls returning owned results.
struct DeliveryCapture {
    evidence: Option<Evidence>,
    scratch: Scratch,
    retain_success: bool,
}
impl DeliveryCapture {
    fn invoke(
        mut ev: Evidence,
        cfg: &NativeConfig,
        binding: &ExperimentBinding,
        retain_success: bool,
        authority_text: &str,
    ) -> (Result<Delivered, DeliverError>, Self) {
        capture::persist_inputs(&mut ev, binding, authority_text, &[]);
        let mut owner = Self {
            evidence: Some(ev),
            scratch: Scratch::retained(cfg.fixture_root.clone()),
            retain_success,
        };
        let outcome = deliver_and_consume(cfg, binding);
        capture::capture_returned(
            owner.evidence.as_mut().unwrap(),
            binding,
            &outcome,
            binding.consumer_invocations(),
        );
        (outcome, owner)
    }
    fn finalize(&mut self, failure: Option<&str>) -> Result<(), String> {
        let Some(mut ev) = self.evidence.take() else {
            return Ok(());
        };
        let writes_failed = !ev.write_failures().is_empty();
        if failure.is_some() || writes_failed {
            ev.put_json("failure.json", &json!({
                "stage": if failure.is_some() { "assertion_after_capture" } else { "evidence_write_after_capture" },
                "message": failure.unwrap_or("evidence write failed after returned facts were captured"),
                "scratch": self.scratch.path,
                "scratch_listing_before_cleanup": tree_listing(&self.scratch.path),
                "scratch_retained": true,
            }));
        }
        let requested = if failure.is_some() {
            "failed"
        } else {
            "passed"
        };
        match ev.finish(requested) {
            Ok(recorded) if recorded == requested && !writes_failed => {
                if failure.is_none() && !self.retain_success {
                    self.scratch.release();
                }
                Ok(())
            }
            Ok(recorded) => Err(format!(
                "evidence bundle recorded `{recorded}`; scratch retained at {}",
                self.scratch.path.display()
            )),
            Err(error) => Err(format!(
                "incomplete/unestablished finalization: {error}; available evidence and scratch retained at {}",
                self.scratch.path.display()
            )),
        }
    }
}
impl Drop for DeliveryCapture {
    fn drop(&mut self) {
        let unwinding = std::thread::panicking();
        let result = self.finalize(
            unwinding.then_some("control assertion unwound after returned evidence capture"),
        );
        if let Err(error) = result {
            if unwinding {
                eprintln!("{error}");
            } else {
                panic!("{error}");
            }
        }
    }
}

fn run_case(
    env: &RunEnv,
    name: &str,
    control: Control,
    case: &Case<'_>,
    result: &str,
) -> (
    Result<Delivered, DeliverError>,
    ExperimentBinding,
    DeliveryCapture,
) {
    let root = scratch_root(env, name);
    let cfg = native_config(root, control);
    let binding = binding_for(case, result);
    let ev = Evidence::create_at(env, env.run_dir.join("controls").join(name)).unwrap();
    let actual_authority = json_value(case.authority.value()).to_string();
    let authority_text = if json_value(case.authority.value())
        == serde_json::from_str::<Value>(AUTHORITY).unwrap()
    {
        AUTHORITY
    } else {
        &actual_authority
    };
    let (outcome, owner) = DeliveryCapture::invoke(ev, &cfg, &binding, true, authority_text);
    (outcome, binding, owner)
}

fn assert_reaped(d: &ProcessFacts) {
    assert!(d.reap.reaped(), "child {} not reaped: {:?}", d.pid, d.reap);
    assert_eq!(
        d.pid_present_after_reap,
        Some(false),
        "child pid {} still present",
        d.pid
    );
    assert!(
        d.descendants_seen.is_empty(),
        "unexpected descendants {:?}",
        d.descendants_seen
    );
    assert!(!d.timed_out, "{d:?}");
}

fn tree_listing(root: &Path) -> Vec<Value> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<Value>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let rel = path
                    .strip_prefix(base)
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                if path.is_dir() {
                    walk(&path, base, out);
                } else if let Ok(meta) = std::fs::metadata(&path) {
                    out.push(json!({"path": rel, "length": meta.len()}));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-text panic payload".to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Inject {
    None,
    /// A failing assertion after `Delivered` is available.
    PanicAfterDelivered,
}

/// The positive verdict. Runs only after capture; any failure unwinds to `run_vertical`.
fn positive_assertions(
    w: &World,
    state: &ReviewEpisodeState,
    before: &str,
    blockers_before: &[work_engine_review_preparation::Blocker],
    binding: &ExperimentBinding,
    delivered: &Delivered,
) {
    let run = &delivered.run;
    let obs = run.observation.as_ref().expect("native observation");
    let bound = run.bound.as_ref().expect("bound allocation");
    let frozen = binding.frozen();

    // The preparation delivered is the canonical recomputation (full equality), blockers included.
    assert_eq!(
        binding.preparation(),
        &prepare(&w.candidate, &request(None)).unwrap()
    );
    // SDK-observed: actual SDK identity, one request, exact bytes, empty toolset, idle, no provider/network.
    assert_eq!(bound.allocation().sdk_version, "1.5.0");
    assert!(bound.allocation().node_version.starts_with('v'));
    assert_eq!(obs.session_id, bound.session_id());
    assert_eq!(obs.request_count, 1);
    assert_eq!(obs.counters.ordinary_stream, 0);
    assert_eq!(obs.counters.fetch, 0);
    assert!(obs.active_tools.is_empty() && obs.terminal_idle);
    let ctx = obs.context.as_ref().unwrap();
    assert_eq!(ctx["messages"][0]["role"], "system");
    assert_eq!(ctx["messages"][1]["role"], "user");
    let delivered_text = ctx["messages"][1]["content"][0]["text"].as_str().unwrap();
    assert_eq!(
        delivered_text,
        frozen.contract(),
        "the SDK received the exact frozen bytes"
    );
    let envelope: Value = serde_json::from_str(delivered_text).unwrap();
    assert_eq!(
        envelope["preparation"],
        serde_json::to_value(binding.preparation()).unwrap()
    );
    assert_eq!(
        ctx["messages"][0]["sections"]["preamble"].as_str().unwrap(),
        bound.allocation().preamble
    );
    assert_eq!(ctx["messages"].as_array().unwrap().len(), 2);
    let everything =
        serde_json::to_string(&(&obs.context, &obs.history, &obs.system_prompt)).unwrap();
    assert!(
        !everything.contains(CANARY_MARKER),
        "an outside-loader canary resource reached the SDK context"
    );
    let calls = &obs.loader_calls;
    assert!(calls["getSystemPrompt"].as_u64().unwrap() >= 1);
    assert!(calls.get("extendResources").is_none());
    assert_eq!(obs.history.len(), 3, "system, user, assistant");
    assert_eq!(delivered.verified.result_text, binding.synthetic_result());
    assert_eq!(delivered.verified.session_id, obs.session_id);

    // Consumer-accepted: the existing reducer advanced the episode; only a fixture observation.
    assert_eq!(
        canonical(state),
        before,
        "input episode state is not modified"
    );
    assert_eq!(delivered.consumption.state().phase(), Phase::Reported);
    assert_ne!(delivered.consumption.state().revision(), state.revision());
    assert_eq!(
        delivered.consumption.production_consequence,
        work_engine_review_preparation::fixture::ProductionConsequence::Unestablished
    );
    assert_eq!(
        prepare(&w.candidate, &request(None)).unwrap().blockers,
        blockers_before
    );
    assert_eq!(binding.preparation().blockers, blockers_before);
    assert_reaped(&run.facts.process);
    assert_eq!(run.facts.process.reap, ReapFact::Exited { code: 0 });
    assert_eq!(run.facts.process.kill, KillFact::NotSent);
    // Complete attempt: the whole stream is allocation then one observation, ended by a real
    // EOF; one completed start write; both collectors finished; the consumer ran exactly once.
    assert_eq!(run.incompleteness(), Vec::<String>::new());
    assert_eq!(run.interpretation().kinds, ["allocated", "observation"]);
    assert_eq!(run.facts.stdout.end, StreamEnd::Eof);
    assert_eq!(run.facts.stderr.end, StreamEnd::Eof);
    assert!(run.facts.sent.len() == 1 && run.facts.sent[0].outcome == SendOutcome::Completed);
    assert_eq!(run.evidence_verdict(), Ok(vec![]));
    assert_eq!(binding.consumer_invocations(), 1);
}

/// One joined vertical run into `ev`. Order is causal: inputs, native path, capture, assertions,
/// then the outcome. A refusal, an assertion unwind or a write failure leaves a `failed` or
/// `incomplete` bundle and retains scratch; only a fully written `passed` bundle releases it.
fn run_vertical(env: &RunEnv, ev: Evidence, scratch_name: &str, inject: Inject) {
    let w = world();
    let state = begun();
    let before = canonical(&state);
    let blockers_before = prepare(&w.candidate, &request(None)).unwrap().blockers;
    assert!(!blockers_before.is_empty());
    let writer = authority();
    let binding = binding_for(&base(&w, &state, &writer), &result_json(|_| {}));
    let root = scratch_root(env, scratch_name);
    let cfg = native_config(root, Control::Positive);
    let (outcome, mut owner) = DeliveryCapture::invoke(ev, &cfg, &binding, false, AUTHORITY);

    let verdict = catch_unwind(AssertUnwindSafe(|| {
        if inject == Inject::PanicAfterDelivered {
            assert!(
                outcome.is_ok(),
                "the injected failure must follow an available Delivered"
            );
            panic!("injected assertion failure after Delivered was available");
        }
        match &outcome {
            Ok(delivered) => {
                positive_assertions(&w, &state, &before, &blockers_before, &binding, delivered)
            }
            Err(error) => panic!("positive path refused: {error}"),
        }
    }));
    match verdict {
        Ok(()) => owner.finalize(None).unwrap_or_else(|e| panic!("{e}")),
        Err(payload) => {
            if let Err(error) = owner.finalize(Some(&panic_text(payload.as_ref()))) {
                eprintln!("{error}");
            }
            resume_unwind(payload)
        }
    }
}

#[test]
fn canonical_preparation_reaches_the_real_sdk_and_the_existing_consumer() {
    let env = run_env();
    let ev = Evidence::create(&env).expect("the positive evidence directory is new");
    run_vertical(&env, ev, "positive", Inject::None);
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn listed(manifest: &Value) -> Vec<String> {
    manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap().to_owned())
        .collect()
}

/// R4 capture lifetime: a failure after `Delivered` is available, or a failed evidence write,
/// leaves every available native observation and actual state in a non-passed bundle, retains
/// scratch, and never overwrites an existing bundle.
#[test]
fn post_delivery_failures_keep_native_evidence_states_and_scratch() {
    let env = run_env();
    let controls = env.run_dir.join("controls");
    let pid = std::process::id();

    let dir = controls.join("assertion-unwind");
    let ev = Evidence::create_at(&env, dir.clone()).unwrap();
    let unwound = catch_unwind(AssertUnwindSafe(|| {
        run_vertical(
            &env,
            ev,
            "control-assertion-unwind",
            Inject::PanicAfterDelivered,
        )
    }));
    assert!(unwound.is_err(), "the injected assertion must fail the run");
    let manifest = read_json(&dir.join("output-manifest.json"));
    assert_eq!(manifest["outcome"], "failed");
    let names = listed(&manifest);
    for name in [
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
        "capture.json",
        "failure.json",
    ] {
        assert!(names.iter().any(|n| n == name), "{name} preserved");
        assert!(dir.join(name).is_file(), "{name} on disk");
    }
    let failure = read_json(&dir.join("failure.json"));
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("injected assertion failure")
    );
    let scratch = env
        .run_dir
        .join(format!("scratch/control-assertion-unwind-{pid}"));
    assert!(scratch.is_dir(), "scratch retained after the unwind");
    assert!(
        !failure["scratch_listing_before_cleanup"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let dir = controls.join("write-failure");
    let ev = Evidence::create_at(&env, dir.clone()).unwrap();
    // A real OS-level create_new failure: the name is already taken by a directory.
    std::fs::create_dir(dir.join("episode-after.json")).unwrap();
    let failed = catch_unwind(AssertUnwindSafe(|| {
        run_vertical(&env, ev, "control-write-failure", Inject::None)
    }));
    assert!(failed.is_err(), "an incomplete bundle cannot pass");
    let manifest = read_json(&dir.join("output-manifest.json"));
    assert_eq!(manifest["outcome"], "incomplete");
    assert_eq!(manifest["write_failures"][0]["name"], "episode-after.json");
    let names = listed(&manifest);
    for name in [
        "observation.json",
        "native-frames-received.jsonl",
        "consumption.json",
        "capture.json",
    ] {
        assert!(
            names.iter().any(|n| n == name),
            "{name} partial evidence kept"
        );
    }
    let scratch = env
        .run_dir
        .join(format!("scratch/control-write-failure-{pid}"));
    assert!(scratch.is_dir(), "scratch retained after a write failure");

    let prior = std::fs::read(dir.join("output-manifest.json")).unwrap();
    assert!(
        Evidence::create_at(&env, dir.clone()).is_err(),
        "an existing bundle is never reused"
    );
    assert_eq!(
        std::fs::read(dir.join("output-manifest.json")).unwrap(),
        prior
    );
}

fn refused(outcome: Result<Delivered, DeliverError>) -> DeliverError {
    match outcome {
        Ok(_) => panic!("must be refused"),
        Err(e) => e,
    }
}

#[test]
fn missing_runtime_key_fails_the_genuine_sdk_auth_preflight_before_any_fixture_stream() {
    let env = run_env();
    let w = world();
    let state = begun();
    let before = canonical(&state);
    let writer = authority();
    let (outcome, _, _capture) = run_case(
        &env,
        "missing-key",
        Control::MissingKey,
        &base(&w, &state, &writer),
        &result_json(|_| {}),
    );
    let DeliverError::Delivery { refusal, run } = refused(outcome) else {
        panic!("expected a delivery refusal")
    };
    assert!(
        matches!(&refusal, DeliveryRefusal::PromptRefused(m) if m.to_lowercase().contains("api key")),
        "{refusal:?}"
    );
    let obs = run.observation.as_ref().unwrap();
    assert_eq!(
        obs.request_count, 0,
        "no fixture stream observation before the auth refusal"
    );
    assert!(obs.context.is_none());
    assert_eq!(obs.counters.ordinary_stream, 0);
    assert_eq!(canonical(&state), before);
    assert_reaped(&run.facts.process);
}

#[test]
fn the_ordinary_sdk_stream_hits_the_provider_entry_refusal_and_yields_no_usable_result() {
    let env = run_env();
    let w = world();
    let state = begun();
    let before = canonical(&state);
    let writer = authority();
    let (outcome, _, _capture) = run_case(
        &env,
        "ordinary-stream",
        Control::OrdinaryStream,
        &base(&w, &state, &writer),
        &result_json(|_| {}),
    );
    let DeliverError::Delivery { refusal, run } = refused(outcome) else {
        panic!("expected a delivery refusal")
    };
    assert!(
        matches!(&refusal, DeliveryRefusal::ProviderEntry { ordinary_stream, fetch: 0 } if *ordinary_stream >= 1),
        "{refusal:?}"
    );
    let obs = run.observation.as_ref().unwrap();
    assert_eq!(obs.request_count, 0);
    assert!(
        !serde_json::to_string(&obs.history)
            .unwrap()
            .contains("\"synthetic\""),
        "no usable result was produced"
    );
    assert_eq!(canonical(&state), before);
    assert_reaped(&run.facts.process);
}

#[test]
fn a_possible_send_without_observation_stays_uncertain_and_advances_nothing() {
    let env = run_env();
    let w = world();
    let state = begun();
    let before = canonical(&state);
    let writer = authority();
    let (outcome, _, _capture) = run_case(
        &env,
        "die-during-prompt",
        Control::DieDuringPrompt,
        &base(&w, &state, &writer),
        &result_json(|_| {}),
    );
    let DeliverError::Delivery { refusal, run } = refused(outcome) else {
        panic!("expected a delivery refusal")
    };
    assert!(refusal.is_uncertain(), "{refusal:?}");
    assert!(run.observation.is_none());
    assert_eq!(run.facts.process.reap, ReapFact::Exited { code: 3 });
    assert_eq!(canonical(&state), before);
    assert_reaped(&run.facts.process);
}

#[test]
fn the_adapter_refuses_a_wrong_session_and_a_duplicate_start() {
    let env = run_env();
    let w = world();
    let state = begun();
    let writer = authority();
    for (name, control, needle) in [
        ("wrong-session", Control::WrongSession, "did not allocate"),
        // The same duplicate under three transport groupings (coalesced, one frame per write,
        // small writes splitting frames). The refusal must name the second host frame: that is
        // the adapter's own record of having processed it, which a completed write is not.
        (
            "duplicate-start",
            Control::DuplicateStart,
            "duplicate start refused (host frame 2,",
        ),
        (
            "duplicate-start-per-frame",
            Control::DuplicateStartPerFrame,
            "duplicate start refused (host frame 2,",
        ),
        (
            "duplicate-start-chunked",
            Control::DuplicateStartChunked,
            "duplicate start refused (host frame 2,",
        ),
    ] {
        let (cfg, _root) = config(&env, &format!("{name}-raw"), control);
        let binding = binding_for(&base(&w, &state, &writer), &result_json(|_| {}));
        let run = run_native(&cfg, &binding).unwrap();
        let error = run.error_frame.as_deref().unwrap_or_default();
        assert!(
            error.contains(needle),
            "{name}: {error} / {} / {:?} / {:?}",
            run.stderr_lossy(),
            run.facts,
            String::from_utf8_lossy(&run.raw_stdout)
        );
        assert_eq!(
            run.facts.process.reap,
            ReapFact::Exited { code: 2 },
            "{name}"
        );
        assert!(run.observation.is_none(), "{name}");
        assert_reaped(&run.facts.process);
        if name.starts_with("duplicate") {
            // Both frames were accepted by the pipe (transport fact); only the adapter's refusal
            // above establishes that the peer processed the second one.
            let s = &run.facts.sent;
            assert_eq!(s.len(), 2, "{name}: {s:?}");
            assert!(
                s.iter()
                    .all(|r| r.outcome == SendOutcome::Completed && r.written == r.frame_len),
                "{name}: {s:?}"
            );
            assert_eq!(s[0].frame_sha256, s[1].frame_sha256, "{name}");
        }
        let (outcome, _, _capture) = run_case(
            &env,
            name,
            control,
            &base(&w, &state, &writer),
            &result_json(|_| {}),
        );
        let DeliverError::Delivery { refusal, .. } = refused(outcome) else {
            panic!("expected a delivery refusal")
        };
        assert!(refusal.is_uncertain(), "{name}: {refusal:?}");
    }
}

/// Native correspondence holds in each case below; the refusal comes from the existing consumer
/// or the episode owner, unchanged, and the episode state is not modified. (Declaration/subject
/// mismatches are refused earlier, by the binding, before any child: see tests/binding.rs.)
#[test]
fn consumer_and_episode_owners_keep_their_checks_after_a_correct_native_delivery() {
    let env = run_env();
    let w = world();
    let state = begun();
    let before = canonical(&state);
    let good = result_json(|_| {});
    let writer = authority();
    let check = |name: &str, case: Case<'_>, result: &str| -> (ConsumeRefusal, DeliveryCapture) {
        let (outcome, _, _capture) = run_case(&env, name, Control::Positive, &case, result);
        let refusal = match refused(outcome) {
            DeliverError::Consumer { refusal, run, .. } => {
                assert!(
                    run.observation.is_some(),
                    "{name}: native correspondence held"
                );
                assert_reaped(&run.facts.process);
                refusal
            }
            other => panic!("{name}: expected a consumer refusal, got {other}"),
        };
        (refusal, _capture)
    };

    let (e, _capture) = check(
        "stale-revision",
        Case {
            revision: Revision("f".repeat(64)),
            ..base(&w, &state, &writer)
        },
        &good,
    );
    assert!(
        matches!(&e, ConsumeRefusal::Episode(x) if x.kind == ErrorKind::RevisionConflict),
        "{e:?}"
    );
    let mut other: Value = serde_json::from_str(AUTHORITY).unwrap();
    other["writer"]["actorId"] = json!("a-different-writer");
    let other = Authority::parse(parse_json(&other.to_string()).unwrap()).unwrap();
    let (e, _capture) = check(
        "wrong-writer",
        Case {
            authority: &other,
            ..base(&w, &state, &writer)
        },
        &good,
    );
    assert!(
        matches!(&e, ConsumeRefusal::Episode(x) if x.kind == ErrorKind::Authority),
        "{e:?}"
    );
    let (e, _capture) = check(
        "wrong-subject-result",
        base(&w, &state, &writer),
        &result_json(|v| v["subject"]["tree"] = json!("0".repeat(40))),
    );
    assert!(
        matches!(&e, ConsumeRefusal::Episode(x) if x.kind == ErrorKind::ResultContract),
        "{e:?}"
    );
    let (e, _capture) = check(
        "missing-evidence",
        Case {
            admissions: serde_json::to_string(&w.admissions[..1]).unwrap(),
            ..base(&w, &state, &writer)
        },
        &good,
    );
    assert!(
        matches!(&e, ConsumeRefusal::AdmissionMissing { .. }),
        "{e:?}"
    );
    let mut weak = w.admissions.clone();
    weak[0]["status"] = json!("unestablished");
    let (e, _capture) = check(
        "not-established",
        Case {
            admissions: serde_json::to_string(&weak).unwrap(),
            ..base(&w, &state, &writer)
        },
        &good,
    );
    assert!(
        matches!(
            &e,
            ConsumeRefusal::EvidenceNotEstablished { .. } | ConsumeRefusal::Input { .. }
        ),
        "{e:?}"
    );

    assert_eq!(
        canonical(&state),
        before,
        "no refusal modified the episode state"
    );
    assert_eq!(state.phase(), Phase::InitialReview);
}

// ---- R6: a complete native attempt precedes consumption ------------------------------------
// Each SDK control drives the real provider-disabled SDK session, then misbehaves at the actual
// child/pipe boundary. Every case refuses before consumption with the consumer observed (runtime
// counter, not an absent after-state) to have been invoked zero times, retained bytes, and
// truthful, internally consistent delivery-owned facts.

struct Refusal {
    run: Box<work_engine_review_delivery::native::NativeRun>,
    refusal: DeliveryRefusal,
    consumer_invocations: u64,
    state_unchanged: bool,
    _capture: DeliveryCapture,
}

fn refused_attempt(
    env: &RunEnv,
    name: &str,
    control: Control,
    tweak: impl FnOnce(&mut NativeConfig),
) -> Refusal {
    let w = world();
    let state = begun();
    let before = canonical(&state);
    let writer = authority();
    let root = scratch_root(env, name);
    let mut cfg = native_config(root, control);
    tweak(&mut cfg);
    let binding = binding_for(&base(&w, &state, &writer), &result_json(|_| {}));
    let ev = Evidence::create_at(env, env.run_dir.join("controls").join(name)).unwrap();
    let (outcome, owner) = DeliveryCapture::invoke(ev, &cfg, &binding, true, AUTHORITY);
    let DeliverError::Delivery { refusal, run } = refused(outcome) else {
        panic!("{name}: expected an incomplete-attempt delivery refusal")
    };
    Refusal {
        run,
        refusal,
        consumer_invocations: binding.consumer_invocations(),
        state_unchanged: canonical(&state) == before,
        _capture: owner,
    }
}

/// Bounded completion on the recorded spawn-to-return duration (which includes post-loop digests,
/// pipe release and finalization): cleanup starts at the deadline at the latest and ends at its own
/// bound; the 1 000 ms tolerance covers work run between deadline checks on this host.
fn assert_bounded(name: &str, t: &TimingFacts) {
    assert!(
        t.elapsed_ms <= t.attempt_ms + t.cleanup_ms + 1_000,
        "{name}: attempt took {} ms, bound {} + {} ms",
        t.elapsed_ms,
        t.attempt_ms,
        t.cleanup_ms
    );
}

/// The same bound observed independently by the caller around the whole `run_attempt` call; the
/// recorded duration is a sub-interval of the caller's and can never exceed it.
fn assert_caller_bounded(name: &str, caller: Duration, t: &TimingFacts) {
    assert_bounded(name, t);
    let c = caller.as_millis() as u64;
    assert!(
        t.elapsed_ms <= c,
        "{name}: recorded {} ms exceeds the caller-observed {c} ms",
        t.elapsed_ms
    );
    assert!(
        c <= t.attempt_ms + t.cleanup_ms + 1_000,
        "{name}: caller observed {c} ms, bound {} + {} ms",
        t.attempt_ms,
        t.cleanup_ms
    );
}

fn assert_incomplete(name: &str, r: &Refusal) {
    assert!(
        matches!(&r.refusal, DeliveryRefusal::Incomplete(_)),
        "{name}: {:?}",
        r.refusal
    );
    assert!(r.refusal.is_uncertain(), "{name}");
    assert_eq!(
        r.consumer_invocations, 0,
        "{name}: the consumer must not be invoked"
    );
    assert!(r.state_unchanged, "{name}");
    // The evidence is consistent and itself establishes the failure.
    let why = r
        .run
        .evidence_verdict()
        .unwrap_or_else(|bad| panic!("{name}: contradictory facts {bad:?}"));
    assert!(
        !why.is_empty(),
        "{name}: consistent evidence must name the failure"
    );
    let p = &r.run.facts.process;
    assert!(p.reap.reaped(), "{name}: {:?}", p.reap);
    assert_eq!(
        p.pid_present_after_reap,
        Some(false),
        "{name}: child pid still present"
    );
    assert!(p.descendants_seen.is_empty(), "{name}");
    assert_bounded(name, &r.run.facts.timing);
}

#[test]
fn trailing_data_after_a_valid_observation_refuses_before_consumption_prompt_and_delayed() {
    let env = run_env();
    for (kind, label, defect) in [
        (Trailing::DuplicateObservation, "duplicate", None),
        (Trailing::Malformed, "malformed", Some("not JSON")),
        (Trailing::ErrorFrame, "error", None),
        (Trailing::InvalidUtf8, "invalid-utf8", Some("invalid UTF-8")),
        (
            Trailing::IncompleteFrame,
            "incomplete",
            Some("incomplete trailing frame"),
        ),
    ] {
        for delayed in [false, true] {
            let name = format!(
                "trail-{label}-{}",
                if delayed { "delayed" } else { "prompt" }
            );
            let r = refused_attempt(&env, &name, Control::Trail(kind, delayed), |_| {});
            assert_incomplete(&name, &r);
            assert!(r.run.observation.is_some(), "{name}");
            let interp = r.run.interpretation();
            assert!(!interp.permitted_success(), "{name}");
            assert_eq!(&interp.kinds[..2], ["allocated", "observation"], "{name}");
            assert!(interp.kinds.len() >= 3, "{name}: {:?}", interp.kinds);
            if let Some(needle) = defect {
                assert!(
                    interp.defects.iter().any(|d| d.contains(needle)),
                    "{name}: {:?}",
                    interp.defects
                );
            }
            match kind {
                Trailing::InvalidUtf8 => assert!(
                    r.run.raw_stdout.windows(2).any(|w| w == [0xff, 0xfe]),
                    "{name}"
                ),
                Trailing::IncompleteFrame => {
                    assert!(r.run.raw_stdout.ends_with(b"\"cut"), "{name}")
                }
                _ => {}
            }
            // The child itself behaved: real EOF, clean exit. Only the stream refused.
            assert_eq!(r.run.facts.stdout.end, StreamEnd::Eof, "{name}");
            assert_eq!(
                r.run.facts.process.reap,
                ReapFact::Exited { code: 0 },
                "{name}"
            );
        }
    }
}

#[test]
fn nonzero_exit_timeout_early_eof_and_limits_refuse_with_retained_evidence() {
    let env = run_env();

    let r = refused_attempt(
        &env,
        "exit-nonzero",
        Control::ExitNonzeroAfterObservation,
        |_| {},
    );
    assert_incomplete("exit-nonzero", &r);
    assert_eq!(r.run.facts.process.reap, ReapFact::Exited { code: 5 });
    assert!(
        r.run.interpretation().permitted_success(),
        "the stream alone was a valid protocol"
    );

    let r = refused_attempt(&env, "hang", Control::HangAfterObservation, |c| {
        c.timeout = Duration::from_secs(20);
        c.cleanup = Duration::from_secs(3);
    });
    assert_incomplete("hang", &r);
    let p = &r.run.facts.process;
    assert!(p.timed_out);
    assert_eq!(
        p.kill,
        KillFact::Sent {
            reason: KillReason::Deadline
        }
    );
    assert_eq!(p.reap, ReapFact::Signalled { signal: 9 });
    assert!(
        r.run.observation.is_some(),
        "the observation was received before the timeout"
    );

    let r = refused_attempt(&env, "early-eof", Control::CleanExitNoObservation, |_| {});
    assert_incomplete("early-eof", &r);
    assert_eq!(r.run.facts.stdout.end, StreamEnd::Eof);
    assert_eq!(r.run.interpretation().kinds, ["allocated"]);
    assert!(r.run.observation.is_none());

    let r = refused_attempt(
        &env,
        "die-after-allocation",
        Control::DieAfterAllocation,
        |_| {},
    );
    assert_incomplete("die-after-allocation", &r);
    assert!(r.run.observation.is_none());
    // A frame is recorded from its first write on; completed, failed or never attempted are all
    // truthful here (the exit races the write) and none of them is a completed attempt.
    assert!(r.run.facts.sent.len() <= 1);

    let r = refused_attempt(
        &env,
        "frame-limit",
        Control::OversizeFrameAfterObservation,
        |_| {},
    );
    assert_incomplete("frame-limit", &r);
    let s = &r.run.facts.stdout;
    assert_eq!(s.end, StreamEnd::FrameLimit);
    // The adapter may exit on its own before the owner stops reading; either fact is truthful.
    assert!(matches!(
        r.run.facts.process.kill,
        KillFact::NotSent
            | KillFact::Sent {
                reason: KillReason::CollectionStopped
            }
    ));
    let tail = r.run.raw_stdout.rsplit(|b| *b == b'\n').next().unwrap();
    assert_eq!(
        tail.len(),
        (1 << 20) + 1,
        "frame bound + 1 byte of the oversize frame retained"
    );

    let r = refused_attempt(&env, "total-limit", Control::FloodAfterObservation, |c| {
        c.limits.total_bytes = 2 << 20
    });
    assert_incomplete("total-limit", &r);
    assert_eq!(r.run.facts.stdout.end, StreamEnd::TotalLimit);
    assert_eq!(
        r.run.raw_stdout.len() as u64,
        2 << 20,
        "exactly the bounded prefix is retained"
    );
    assert!(
        r.run.facts.stdout.overflow_bytes_observed > 0,
        "known overflow is reported"
    );
}

// ---- Owned-attempt lifecycle over REAL OS pipes with a scripted /bin/sh child ----------------
// The child is a deterministic shell script, not the SDK: these exercise the owner's send/read/
// deadline/cleanup behaviour at the real pipe boundary. They do not replace the SDK positive.

struct Script {
    send: Option<Vec<u8>>,
    sent: bool,
}
impl AttemptDriver for Script {
    fn on_stdout(&mut self, _raw: &[u8]) -> DriverStep {
        let mut step = DriverStep::default();
        if !self.sent {
            self.sent = true;
            if let Some(bytes) = self.send.take() {
                step.send = vec![String::from_utf8(bytes).unwrap()];
            }
        }
        step.close_stdin = true;
        step
    }
}

fn sh(
    script: &str,
    send: Option<Vec<u8>>,
    timeout_ms: u64,
    cleanup_ms: u64,
) -> work_engine_review_delivery::native::AttemptRecord {
    let mut c = std::process::Command::new("/bin/sh");
    c.args(["-c", script]).env_clear();
    let mut d = Script { send, sent: false };
    let policy = AttemptPolicy {
        timeout: Duration::from_millis(timeout_ms),
        cleanup: Duration::from_millis(cleanup_ms),
        limits: Limits::default(),
        write_shape: WriteShape::default(),
    };
    let called = std::time::Instant::now();
    let rec = run_attempt(c, &mut d, &policy).expect("spawn /bin/sh");
    assert_caller_bounded(script, called.elapsed(), &rec.facts.timing);
    // A child that reports its pipe identities on stderr (`L <fd0> <fd1> <fd2>`) gets the
    // independent resource-release observation.
    if let Some(line) = rec.raw_stderr.split(|b| *b == b'\n').next()
        && let Some(rest) = std::str::from_utf8(line)
            .ok()
            .and_then(|l| l.strip_prefix("L "))
    {
        let links: Vec<String> = rest.split(' ').map(str::to_owned).collect();
        assert_released(script, &links, &rec);
    }
    rec
}

fn verdict(rec: &work_engine_review_delivery::native::AttemptRecord) -> Vec<String> {
    check_attempt_evidence(
        &rec.facts,
        &rec.raw_stdout,
        &rec.raw_stderr,
        &rec.sent_bytes,
    )
    .unwrap_or_else(|bad| panic!("contradictory facts {bad:?}"))
}

#[test]
fn a_child_that_stops_reading_stdin_cannot_block_the_deadline_owner() {
    // 4 MiB frame, far beyond any pipe buffer; the child never reads stdin.
    let frame = vec![b'a'; 4 << 20];
    let rec = sh(
        r#"printf 'L %s %s %s\n' "$(readlink /proc/$$/fd/0)" "$(readlink /proc/$$/fd/1)" "$(readlink /proc/$$/fd/2)" >&2; printf '{"type":"x"}\n'; exec sleep 30"#,
        Some(frame),
        1_000,
        2_000,
    );
    let s = &rec.facts.sent;
    assert_eq!(s.len(), 1, "the send is recorded from its first write");
    assert_eq!(
        s[0].outcome,
        SendOutcome::Cancelled {
            reason: SendCancel::Deadline
        }
    );
    assert!(
        s[0].written > 0 && s[0].written < s[0].frame_len,
        "partial write: {} of {}",
        s[0].written,
        s[0].frame_len
    );
    assert_eq!(
        rec.sent_bytes.len() as u64,
        s[0].frame_len,
        "the intended frame bytes are retained"
    );
    let p = &rec.facts.process;
    assert!(p.timed_out);
    assert_eq!(
        p.kill,
        KillFact::Sent {
            reason: KillReason::Deadline
        }
    );
    assert_eq!(p.reap, ReapFact::Signalled { signal: 9 });
    assert_eq!(rec.raw_stdout, b"{\"type\":\"x\"}\n");
    let why = verdict(&rec);
    assert!(
        why.iter().any(|w| w.contains("timed out"))
            && why.iter().any(|w| w.contains("send did not complete")),
        "{why:?}"
    );
}

#[test]
fn partial_stdout_line_and_stderr_prefix_survive_a_missing_terminal_report() {
    let rec = sh(
        r#"printf '{"partial":'; printf 'err-prefix' >&2; exec sleep 30"#,
        None,
        800,
        2_000,
    );
    assert_eq!(
        rec.raw_stdout, b"{\"partial\":",
        "open frame retained without a newline"
    );
    assert_eq!(
        rec.raw_stderr, b"err-prefix",
        "stderr prefix retained without a terminal report"
    );
    assert!(rec.facts.process.timed_out);
    assert_eq!(rec.facts.stdout.retained_bytes, rec.raw_stdout.len() as u64);
    let why = verdict(&rec);
    assert!(
        why.iter().any(|w| w.contains("incomplete trailing frame")),
        "{why:?}"
    );
}

#[test]
fn an_inherited_pipe_that_never_ends_is_cancelled_at_the_cleanup_bound() {
    // A grandchild keeps stdout open after the child is killed: EOF never comes.
    let rec = sh(r#"sleep 4 & printf 'kept'; exec sleep 30"#, None, 600, 700);
    let p = &rec.facts.process;
    assert!(p.timed_out);
    assert_eq!(p.reap, ReapFact::Signalled { signal: 9 });
    assert_eq!(
        rec.facts.stdout.end,
        StreamEnd::Cancelled,
        "unknown coverage after the prefix"
    );
    assert_eq!(rec.raw_stdout, b"kept");
    assert!(
        !p.descendants_seen.is_empty(),
        "the grandchild was observed"
    );
    for pid in &p.descendants_seen {
        let _ = std::process::Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .status();
    }
    let why = verdict(&rec);
    assert!(why.iter().any(|w| w.contains("Cancelled")), "{why:?}");
}

// ---- Pipe-boundary facts that a write result alone cannot distinguish (labelled test probe) ----
// The scripted child first reports its pid and the identities of its stdin, stdout and stderr
// pipes. The probe keeps apart: the peer's fd 0 being closed (only a NotFound `/proc/<pid>/fd/0`
// counts as closed; any other metadata error is unknown); a PARTIAL diagnostic scan for read-end
// references in the /proc fd tables this uid can read (other-uid tables are unreadable, traversal
// and readlink errors are counted, and the scan is not an atomic snapshot, so it never
// establishes global absence); bytes accepted by the pipe (the delivery send record); and bytes
// accepted but not consumed (FIONREAD on the pipe through the peer's fd 0). The decisive
// failed-send evidence is the kernel's own outcome: a BrokenPipe write with zero accepted bytes.
//
// Write progress is established by observation, never by a sleep interval: the probe looks for
// unread bytes only in a callback strictly AFTER the one that returned the send (a single owner
// thread cannot write while its driver callback runs), and keeps looking in later callbacks until
// the bytes are present. The child prints a TICK every 50 ms, so callbacks recur however the
// markers coalesce or the owner is delayed; a missing observation fails the control.

const PIPE_ID: &str = r#"printf 'P %s %s %s %s\n' $$ "$(readlink /proc/$$/fd/0)" "$(readlink /proc/$$/fd/1)" "$(readlink /proc/$$/fd/2)""#;

/// Partial diagnostic observation of read-end references (see the section comment).
#[derive(Debug)]
struct ReadableScan {
    first_holders: Vec<String>,
    waited_ms: u128,
    unreadable_processes: usize,
    traversal_errors: usize,
}

fn readable_read_end_holders(link: &str) -> (Vec<String>, usize, usize) {
    let mut holders = Vec::new();
    let (mut unreadable, mut errors) = (0, 0);
    let Ok(procs) = std::fs::read_dir("/proc") else {
        return (vec!["/proc unreadable".into()], 0, 1);
    };
    for e in procs {
        let Ok(e) = e else {
            errors += 1;
            continue;
        };
        let name = e.file_name();
        let Some(pid) = name
            .to_str()
            .filter(|n| n.bytes().all(|b| b.is_ascii_digit()))
        else {
            continue;
        };
        let fds = match std::fs::read_dir(e.path().join("fd")) {
            Ok(f) => f,
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                unreadable += 1;
                continue;
            }
            Err(_) => {
                // Exited meanwhile, or otherwise not observed.
                errors += 1;
                continue;
            }
        };
        for fd in fds {
            let Ok(fd) = fd else {
                errors += 1;
                continue;
            };
            match std::fs::read_link(fd.path()) {
                Ok(t) if t.as_os_str() == link => {}
                Ok(_) => continue,
                Err(_) => {
                    errors += 1;
                    continue;
                }
            }
            let fdname = fd.file_name();
            let info =
                std::fs::read_to_string(e.path().join("fdinfo").join(&fdname)).unwrap_or_default();
            let flags = info
                .lines()
                .find_map(|l| l.strip_prefix("flags:"))
                .and_then(|v| u32::from_str_radix(v.trim(), 8).ok());
            // O_ACCMODE 1 is write-only; read-only, read-write or unknown count as a reader.
            if flags.map(|f| f & 3) != Some(1) {
                let comm = std::fs::read_to_string(e.path().join("comm")).unwrap_or_default();
                holders.push(format!(
                    "pid {pid} ({}) fd {} flags {flags:?}",
                    comm.trim(),
                    fdname.to_string_lossy()
                ));
            }
        }
    }
    (holders, unreadable, errors)
}

/// Waits (bounded) until the partial scan observes no readable holder. Success means only that.
fn await_no_readable_holder(link: &str, bound: Duration) -> Result<ReadableScan, Vec<String>> {
    let start = std::time::Instant::now();
    let mut first: Option<Vec<String>> = None;
    loop {
        let (holders, unreadable, errors) = readable_read_end_holders(link);
        let first_holders = first.get_or_insert_with(|| holders.clone()).clone();
        if holders.is_empty() {
            return Ok(ReadableScan {
                first_holders,
                waited_ms: start.elapsed().as_millis(),
                unreadable_processes: unreadable,
                traversal_errors: errors,
            });
        }
        if start.elapsed() >= bound {
            return Err(holders);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Bytes accepted by the pipe but not consumed by any reader, read through the peer's fd 0.
fn unread_bytes(pid: u32) -> Result<i32, String> {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;
    let f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(format!("/proc/{pid}/fd/0"))
        .map_err(|e| e.to_string())?;
    let mut n: libc::c_int = 0;
    // SAFETY: FIONREAD stores one c_int into `n` for the pipe descriptor owned by `f`.
    if unsafe { libc::ioctl(f.as_raw_fd(), libc::FIONREAD, &mut n) } < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(n)
}

/// Descriptors of THIS process that still refer to any of `links` (own fd table: fully readable).
fn own_fds_referring(links: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for fd in std::fs::read_dir("/proc/self/fd")
        .expect("own fd table")
        .flatten()
    {
        if let Ok(t) = std::fs::read_link(fd.path())
            && links.iter().any(|l| t.as_os_str() == l.as_str())
        {
            found.push(format!(
                "fd {} -> {}",
                fd.file_name().to_string_lossy(),
                t.display()
            ));
        }
    }
    found
}

/// Independent resource-release observation after `run_attempt` returned: no descriptor of this
/// process still refers to the child's pipes, and a reaped child's pid is gone.
fn assert_released(
    name: &str,
    links: &[String],
    rec: &work_engine_review_delivery::native::AttemptRecord,
) {
    assert_eq!(
        links.len(),
        3,
        "{name}: the child reported its three pipe identities"
    );
    let held = own_fds_referring(links);
    assert!(
        held.is_empty(),
        "{name}: pipes still held after return: {held:?}"
    );
    if rec.facts.process.reap.reaped() {
        assert!(
            !Path::new(&format!("/proc/{}", rec.facts.process.pid)).exists(),
            "{name}: reaped child still present after return"
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ProbeMode {
    /// Send after the peer closed fd 0 (the CLOSED marker); expect the kernel to refuse it.
    SendAfterClose,
    /// Observe the accepted bytes unconsumed, then end the child (SIGTERM).
    ObserveUnread,
    /// Observe an accepted prefix, then make the peer close fd 0 (SIGUSR1).
    PrefixThenClose,
}

struct PipeProbe {
    frame: Option<Vec<u8>>,
    send_at: &'static str,
    mode: ProbeMode,
    /// Delay inside the callback that returns the send: the owner cannot write meanwhile, and
    /// markers printed during it arrive coalesced in a later drain.
    owner_delay: Duration,
    seen: usize,
    callbacks: usize,
    send_returned_in: Option<usize>,
    close: bool,
    pipe: Option<(u32, Vec<String>)>,
    peer_fd0_closed: Option<Result<bool, String>>,
    scan: Option<Result<ReadableScan, Vec<String>>>,
    unread_attempts: usize,
    unread_after_send: Option<i32>,
    unread_errors: Vec<String>,
    signalled: Option<i32>,
    max_coalesced_lines: usize,
}

impl PipeProbe {
    fn new(frame: Vec<u8>, send_at: &'static str, mode: ProbeMode) -> Self {
        PipeProbe {
            frame: Some(frame),
            send_at,
            mode,
            owner_delay: Duration::ZERO,
            seen: 0,
            callbacks: 0,
            send_returned_in: None,
            close: false,
            pipe: None,
            peer_fd0_closed: None,
            scan: None,
            unread_attempts: 0,
            unread_after_send: None,
            unread_errors: Vec::new(),
            signalled: None,
            max_coalesced_lines: 0,
        }
    }
    fn delayed(mut self, d: Duration) -> Self {
        self.owner_delay = d;
        self
    }
    fn scan(&self, outcome: &[SendRecord]) -> &ReadableScan {
        match self.scan.as_ref() {
            Some(Ok(a)) => a,
            Some(Err(holders)) => panic!(
                "readable holders persisted after the peer's close: {holders:?}; outcome {outcome:?}"
            ),
            None => panic!("the peer's close was never observed; outcome {outcome:?}"),
        }
    }
}

impl AttemptDriver for PipeProbe {
    fn on_stdout(&mut self, raw: &[u8]) -> DriverStep {
        let k = self.callbacks;
        self.callbacks += 1;
        let mut step = DriverStep::default();
        let text = String::from_utf8_lossy(raw).into_owned();
        let lines: Vec<&str> = text
            .split_inclusive('\n')
            .filter(|l| l.ends_with('\n'))
            .map(str::trim_end)
            .collect();
        self.max_coalesced_lines = self
            .max_coalesced_lines
            .max(lines.len().saturating_sub(self.seen));
        let mut returned_send = false;
        for line in lines.iter().skip(self.seen).copied() {
            if let Some(rest) = line.strip_prefix("P ") {
                let mut it = rest.split(' ');
                if let Some(Ok(pid)) = it.next().map(str::parse) {
                    self.pipe = Some((pid, it.map(str::to_owned).collect()));
                }
            }
            if line == "CLOSED"
                && let Some((pid, links)) = &self.pipe
            {
                self.peer_fd0_closed = Some(
                    match std::fs::symlink_metadata(format!("/proc/{pid}/fd/0")) {
                        Ok(_) => Ok(false),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
                        Err(e) => Err(e.to_string()),
                    },
                );
                self.scan = Some(await_no_readable_holder(&links[0], Duration::from_secs(3)));
            }
            if line == self.send_at
                && let Some(f) = self.frame.take()
            {
                step.send.push(String::from_utf8(f).unwrap());
                self.send_returned_in = Some(k);
                returned_send = true;
            }
        }
        self.seen = lines.len();
        // Unread bytes are looked for only in a callback after the one that returned the send.
        if self.mode != ProbeMode::SendAfterClose
            && self.unread_after_send.is_none()
            && self.send_returned_in.is_some_and(|j| j < k)
            && let Some((pid, _)) = &self.pipe
        {
            self.unread_attempts += 1;
            match unread_bytes(*pid) {
                Ok(n) if n > 0 => self.unread_after_send = Some(n),
                Ok(_) => {}
                Err(e) => self.unread_errors.push(e),
            }
            if self.unread_after_send.is_some() && self.signalled.is_none() {
                let sig = match self.mode {
                    ProbeMode::ObserveUnread => libc::SIGTERM,
                    _ => libc::SIGUSR1,
                };
                // SAFETY: kill(2) on the owned child's pid with a catchable signal it traps.
                let r = unsafe { libc::kill(*pid as libc::pid_t, sig) };
                self.signalled = Some(if r == 0 { sig } else { -1 });
            }
        }
        match self.mode {
            ProbeMode::SendAfterClose => self.close |= returned_send,
            ProbeMode::ObserveUnread => self.close |= self.unread_after_send.is_some(),
            ProbeMode::PrefixThenClose => {}
        }
        if returned_send && !self.owner_delay.is_zero() {
            std::thread::sleep(self.owner_delay);
        }
        step.close_stdin = self.close;
        step
    }
}

fn probe(
    script: &str,
    mut d: PipeProbe,
) -> (
    work_engine_review_delivery::native::AttemptRecord,
    PipeProbe,
) {
    let mut c = std::process::Command::new("/bin/sh");
    c.args(["-c", script]).env_clear();
    let policy = AttemptPolicy {
        timeout: Duration::from_millis(8_000),
        cleanup: Duration::from_millis(1_000),
        limits: Limits::default(),
        write_shape: WriteShape::default(),
    };
    let called = std::time::Instant::now();
    let rec = run_attempt(c, &mut d, &policy).expect("spawn /bin/sh");
    let caller = called.elapsed();
    assert_caller_bounded(script, caller, &rec.facts.timing);
    let (pid, links) = d.pipe.clone().expect("the child reported its pipes");
    assert_eq!(pid, rec.facts.process.pid);
    assert_released(script, &links, &rec);
    (rec, d)
}

/// The child ticks until the probe signals it; it never reads stdin. Exit 3: never signalled.
fn ticking(traps: &str) -> String {
    format!(
        "{PIPE_ID}; {traps}; printf 'READY\\n'; i=0; c=0; while [ $i -lt 120 ]; do printf 'TICK\\n'; sleep 0.05; i=$((i+1)); if [ -n \"$closed\" ]; then c=$((c+1)); [ $c -ge 10 ] && exit 0; fi; done; exit 3"
    )
}

#[test]
fn a_send_after_the_peer_closed_its_read_end_is_a_failed_send_with_zero_accepted_bytes() {
    let frame = b"{\"type\":\"start\"}".to_vec();
    let script = format!("{PIPE_ID}; exec 0<&-; printf 'CLOSED\\n'; sleep 1");
    let (rec, d) = probe(
        &script,
        PipeProbe::new(frame.clone(), "CLOSED", ProbeMode::SendAfterClose),
    );
    let s = &rec.facts.sent;
    assert_eq!(
        d.peer_fd0_closed,
        Some(Ok(true)),
        "peer fd 0 closed before the send was returned"
    );
    let scan = d.scan(s);
    eprintln!(
        "partial diagnostic scan before the send (not a global absence claim): first readable holders {:?}, waited {} ms, unreadable processes {}, traversal errors {}",
        scan.first_holders, scan.waited_ms, scan.unreadable_processes, scan.traversal_errors
    );
    assert_eq!(s.len(), 1);
    // Decisive: the kernel refused the write and accepted nothing.
    match &s[0].outcome {
        SendOutcome::Failed { error } => assert!(error.contains("Broken pipe"), "{error}"),
        other => panic!(
            "expected the kernel to refuse the send, got {other:?}: {:?}",
            s[0]
        ),
    }
    assert_eq!(s[0].written, 0, "no byte was accepted");
    let mut intended = frame;
    intended.push(b'\n');
    assert_eq!(s[0].frame_len, intended.len() as u64);
    assert_eq!(rec.sent_bytes, intended, "the intended bytes are retained");
    assert_eq!(rec.facts.process.reap, ReapFact::Exited { code: 0 });
    assert!(!rec.facts.process.timed_out);
    assert!(
        verdict(&rec)
            .iter()
            .any(|w| w.contains("send did not complete"))
    );
}

#[test]
fn a_completed_send_means_accepted_by_the_pipe_not_consumed_by_the_peer() {
    let frame = b"{\"type\":\"start\"}".to_vec();
    let script = ticking("trap 'exit 0' TERM");
    for delay in [Duration::ZERO, Duration::from_millis(300)] {
        let (rec, d) = probe(
            &script,
            PipeProbe::new(frame.clone(), "READY", ProbeMode::ObserveUnread).delayed(delay),
        );
        let s = &rec.facts.sent;
        let ctx = format!(
            "owner delay {delay:?}, probe attempts {}, errors {:?}, max coalesced {}",
            d.unread_attempts, d.unread_errors, d.max_coalesced_lines
        );
        assert_eq!(s.len(), 1, "{ctx}");
        assert_eq!(s[0].outcome, SendOutcome::Completed, "{ctx}: {:?}", s[0]);
        assert_eq!(s[0].written, frame.len() as u64 + 1, "{ctx}");
        assert_eq!(s[0].written, s[0].frame_len, "{ctx}");
        // Observed in a later callback: every accepted byte (one atomic write < PIPE_BUF) was
        // present and unconsumed after the write, and the peer then ended on the probe's signal.
        assert_eq!(
            d.unread_after_send,
            Some(frame.len() as i32 + 1),
            "{ctx}: accepted bytes unconsumed after the completed send"
        );
        assert_eq!(d.signalled, Some(libc::SIGTERM), "{ctx}");
        assert_eq!(
            rec.facts.process.reap,
            ReapFact::Exited { code: 0 },
            "{ctx}"
        );
        let why = verdict(&rec);
        assert!(
            !why.iter().any(|w| w.contains("send did not complete")),
            "{ctx}: {why:?}"
        );
        let mut intended = frame.clone();
        intended.push(b'\n');
        assert_eq!(rec.sent_bytes, intended, "{ctx}");
        eprintln!("{ctx}: unread observation {:?}", d.unread_after_send);
    }
}

#[test]
fn a_partially_accepted_send_that_then_fails_keeps_intended_and_accepted_bytes() {
    // Larger than any pipe buffer: the pipe accepts a prefix (observed), then the peer closes.
    let frame = vec![b'a'; 200_000];
    let script = ticking(r#"trap 'exec 0<&-; closed=1; printf "CLOSED\n"' USR1"#);
    for delay in [Duration::ZERO, Duration::from_millis(300)] {
        let (rec, d) = probe(
            &script,
            PipeProbe::new(frame.clone(), "READY", ProbeMode::PrefixThenClose).delayed(delay),
        );
        let s = &rec.facts.sent;
        let ctx = format!(
            "owner delay {delay:?}, probe attempts {}, errors {:?}, max coalesced {}",
            d.unread_attempts, d.unread_errors, d.max_coalesced_lines
        );
        let prefix = d
            .unread_after_send
            .unwrap_or_else(|| panic!("{ctx}: no accepted prefix was observed before closure"));
        assert_eq!(d.signalled, Some(libc::SIGUSR1), "{ctx}");
        assert_eq!(d.peer_fd0_closed, Some(Ok(true)), "{ctx}");
        let scan = d.scan(s);
        eprintln!(
            "{ctx}: observed prefix {prefix}; partial diagnostic scan after the peer's close: first readable holders {:?}, waited {} ms, unreadable processes {}, traversal errors {}",
            scan.first_holders, scan.waited_ms, scan.unreadable_processes, scan.traversal_errors
        );
        assert_eq!(s.len(), 1, "{ctx}");
        match &s[0].outcome {
            SendOutcome::Failed { error } => {
                assert!(error.contains("Broken pipe"), "{ctx}: {error}")
            }
            other => panic!("{ctx}: expected a failed send, got {other:?}: {:?}", s[0]),
        }
        assert!(
            s[0].written >= prefix as u64 && s[0].written < s[0].frame_len,
            "{ctx}: accepted {} of {}, observed prefix {prefix}",
            s[0].written,
            s[0].frame_len
        );
        let mut intended = frame.clone();
        intended.push(b'\n');
        assert_eq!(
            rec.sent_bytes, intended,
            "{ctx}: the intended bytes are retained"
        );
        assert_eq!(
            rec.facts.process.reap,
            ReapFact::Exited { code: 0 },
            "{ctx}"
        );
        assert!(
            verdict(&rec)
                .iter()
                .any(|w| w.contains("send did not complete")),
            "{ctx}"
        );
    }
}

/// Labelled deterministic seam over the probe itself: markers coalesced into ONE drain never let
/// the probe observe unread bytes in the callback that returned the send.
#[test]
fn coalesced_markers_never_move_the_unread_observation_into_the_sending_callback() {
    let mut d = PipeProbe::new(b"{}".to_vec(), "READY", ProbeMode::ObserveUnread);
    let step = d.on_stdout(b"P 1 pipe:[1] pipe:[2] pipe:[3]\nREADY\nTICK\nTICK\n");
    assert_eq!(step.send, vec!["{}".to_owned()]);
    assert_eq!(
        d.unread_attempts, 0,
        "no observation before the owner could write"
    );
    assert_eq!(d.unread_after_send, None);
    assert!(!step.close_stdin);
    let mut d = PipeProbe::new(vec![b'a'; 8], "READY", ProbeMode::PrefixThenClose);
    let step = d.on_stdout(b"P 1 pipe:[1] pipe:[2] pipe:[3]\nREADY\nTICK\n");
    assert_eq!(step.send.len(), 1);
    assert_eq!(d.unread_attempts, 0);
    assert_eq!(
        d.signalled, None,
        "closure is never arranged before observed progress"
    );
}

/// Labelled deterministic seam: wait failures injected into the owner's REAL lifecycle loop
/// (the child, pipes, kill and cleanup are real; only the wait result is replaced).
struct InjectedWait {
    /// `None`: every wait fails. `Some(n)`: the first `n` waits fail, later ones ask the OS.
    fail_first: Option<usize>,
    calls: usize,
}

impl WaitObserver for InjectedWait {
    fn try_wait(
        &mut self,
        child: &mut std::process::Child,
    ) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.calls += 1;
        match self.fail_first {
            Some(n) if self.calls > n => child.try_wait(),
            _ => Err(std::io::Error::other("injected wait failure")),
        }
    }
}

fn with_wait(
    script: &str,
    waiter: &mut InjectedWait,
    timeout_ms: u64,
    cleanup_ms: u64,
) -> work_engine_review_delivery::native::AttemptRecord {
    let mut c = std::process::Command::new("/bin/sh");
    c.args(["-c", script]).env_clear();
    let mut d = Script {
        send: None,
        sent: false,
    };
    let policy = AttemptPolicy {
        timeout: Duration::from_millis(timeout_ms),
        cleanup: Duration::from_millis(cleanup_ms),
        limits: Limits::default(),
        write_shape: WriteShape::default(),
    };
    let called = std::time::Instant::now();
    let rec = run_attempt_observed(c, &mut d, &policy, waiter).expect("spawn /bin/sh");
    assert_caller_bounded(script, called.elapsed(), &rec.facts.timing);
    rec
}

#[test]
fn an_injected_wait_failure_is_not_termination_and_the_child_is_still_stopped() {
    let injected = vec!["injected wait failure".to_owned()];
    // Every wait fails: termination is never established, yet the child is killed, waiting
    // continues for the cleanup bound, and the unknown end is the recorded disposition.
    let mut w = InjectedWait {
        fail_first: None,
        calls: 0,
    };
    let rec = with_wait("printf 'x'; exec sleep 30", &mut w, 20_000, 1_500);
    let p = &rec.facts.process;
    assert_eq!(
        p.kill,
        KillFact::Sent {
            reason: KillReason::WaitFailed
        },
        "{p:?}"
    );
    assert_eq!(
        p.reap,
        ReapFact::WaitFailed {
            error: injected[0].clone()
        }
    );
    assert_eq!(p.wait_errors, injected);
    assert_eq!(p.pid_present_after_reap, None);
    assert!(!p.timed_out);
    assert!(
        rec.facts.timing.elapsed_ms < 20_000,
        "bounded by cleanup, not the deadline: {:?}",
        rec.facts.timing
    );
    assert!(w.calls > 1, "waiting continued after the first failure");
    let why = verdict(&rec);
    assert!(
        why.iter().any(|x| x.contains("wait observations failed")),
        "{why:?}"
    );
    // The real child: the recorded kill was effective; this test collects it.
    let mut status = 0;
    // SAFETY: waitpid on this process's own child pid with a valid status pointer.
    let r = unsafe { libc::waitpid(p.pid as i32, &mut status, 0) };
    assert_eq!(r, p.pid as i32, "the child was still ours to collect");
    assert!(
        libc::WIFSIGNALED(status) && libc::WTERMSIG(status) == libc::SIGKILL,
        "status {status:#x}"
    );

    // The first wait fails, a later wait establishes the end: the reap is established, the
    // wait error is preserved, and the kill that followed it is recorded.
    let mut w = InjectedWait {
        fail_first: Some(1),
        calls: 0,
    };
    let rec = with_wait("printf 'x'; exec sleep 30", &mut w, 20_000, 5_000);
    let p = &rec.facts.process;
    assert_eq!(
        p.kill,
        KillFact::Sent {
            reason: KillReason::WaitFailed
        },
        "{p:?}"
    );
    assert_eq!(p.reap, ReapFact::Signalled { signal: 9 });
    assert_eq!(p.wait_errors, injected);
    assert_reaped(p);
    assert!(!p.timed_out);
    let why = verdict(&rec);
    assert!(
        why.iter().any(|x| x.contains("wait observations failed"))
            && why.iter().any(|x| x.contains("had to kill")),
        "{why:?}"
    );
}

type FactEdit = Box<dyn Fn(&mut AttemptFacts)>;

fn clean_facts() -> (AttemptFacts, Vec<u8>, Vec<u8>) {
    let raw = b"x\n".to_vec();
    let frame = b"{}\n".to_vec();
    let stream = |n: u64, frame: Option<u64>| StreamRecord {
        end: StreamEnd::Eof,
        retained_bytes: n,
        overflow_bytes_observed: 0,
        limit_bytes: 100,
        frame_limit_bytes: frame,
    };
    let facts = AttemptFacts {
        sent: vec![SendRecord {
            frame_sha256: format!("{:x}", Sha256::digest(&frame)),
            frame_len: 3,
            written: 3,
            outcome: SendOutcome::Completed,
        }],
        stdout: stream(2, Some(10)),
        stderr: stream(0, None),
        process: ProcessFacts {
            pid: 7,
            kill: KillFact::NotSent,
            reap: ReapFact::Exited { code: 0 },
            timed_out: false,
            descendants_seen: vec![],
            pid_present_after_reap: Some(false),
            wait_errors: vec![],
        },
        timing: TimingFacts {
            attempt_ms: 1,
            cleanup_ms: 1,
            elapsed_ms: 1,
        },
    };
    (facts, raw, frame)
}

/// Deterministic synthetic seams (labelled; not the SDK positive): stream accounting at exact
/// bounds, and the delivery-owned evidence verdict over constructed facts for kill/wait/reap
/// uncertainty and read errors that a real pipe cannot be made to produce reliably.
#[test]
fn deterministic_seams_distinguish_bounds_uncertainty_and_contradiction() {
    let mut c = StreamCollector::new(4, Some(100));
    assert_eq!(
        c.want(),
        5,
        "room plus one distinguishes an exact bound from more bytes"
    );
    assert_eq!(c.absorb(b"abc\n"), None);
    assert_eq!(c.want(), 1);
    c.finish(StreamEnd::Eof);
    let (raw, rec) = c.into_parts();
    assert_eq!(
        (raw.as_slice(), rec.end, rec.overflow_bytes_observed),
        (&b"abc\n"[..], StreamEnd::Eof, 0)
    );
    let mut c = StreamCollector::new(4, Some(100));
    assert_eq!(c.absorb(b"abc\nde"), Some(StreamEnd::TotalLimit));
    let (raw, rec) = c.into_parts();
    assert_eq!(
        (raw.as_slice(), rec.overflow_bytes_observed),
        (&b"abc\n"[..], 2)
    );
    let mut c = StreamCollector::new(100, Some(4));
    assert_eq!(c.absorb(b"012"), None);
    assert_eq!(
        c.absorb(b"3456789"),
        Some(StreamEnd::FrameLimit),
        "the bound spans reads"
    );
    let (raw, rec) = c.into_parts();
    assert_eq!(
        (raw.as_slice(), rec.overflow_bytes_observed),
        (&b"01234"[..], 5)
    );

    // The constructed stream is not the SDK protocol, so the consistent baseline names exactly that.
    let (facts, raw, frame) = clean_facts();
    let baseline = check_attempt_evidence(&facts, &raw, b"", &frame).unwrap();
    assert!(
        baseline.len() == 1 && baseline[0].contains("permitted"),
        "{baseline:?}"
    );
    // Valid failures: consistent facts that name the failure.
    let fails: Vec<(&str, FactEdit)> = vec![
        (
            "read error",
            Box::new(|f| {
                f.stderr.end = StreamEnd::ReadError {
                    error: "EIO".into(),
                }
            }),
        ),
        (
            "kill failed",
            Box::new(|f| {
                f.process.kill = KillFact::Failed {
                    reason: KillReason::Deadline,
                    error: "ESRCH".into(),
                };
                f.process.timed_out = true;
            }),
        ),
        (
            "wait failed",
            Box::new(|f| {
                f.process.reap = ReapFact::WaitFailed {
                    error: "ECHILD".into(),
                };
                f.process.wait_errors = vec!["ECHILD".into()];
                f.process.pid_present_after_reap = None;
            }),
        ),
        (
            "not reaped",
            Box::new(|f| {
                f.process.reap = ReapFact::NotReaped;
                f.process.pid_present_after_reap = None;
            }),
        ),
        (
            "signalled",
            Box::new(|f| f.process.reap = ReapFact::Signalled { signal: 9 }),
        ),
        (
            "cancelled stream",
            Box::new(|f| f.stdout.end = StreamEnd::Cancelled),
        ),
        (
            "partial send",
            Box::new(|f| {
                f.sent[0].written = 1;
                f.sent[0].outcome = SendOutcome::Cancelled {
                    reason: SendCancel::Deadline,
                };
                f.process.timed_out = true;
            }),
        ),
    ];
    for (name, edit) in &fails {
        let (mut f, raw, frame) = clean_facts();
        edit(&mut f);
        let why = check_attempt_evidence(&f, &raw, b"", &frame)
            .unwrap_or_else(|b| panic!("{name}: {b:?}"));
        assert!(why.len() > baseline.len(), "{name}: {why:?}");
    }
    // Contradictions: malformed evidence never establishes a failed attempt.
    let bad: Vec<(&str, FactEdit)> = vec![
        ("count", Box::new(|f| f.stdout.retained_bytes = 3)),
        ("completed short", Box::new(|f| f.sent[0].written = 2)),
        (
            "incomplete full",
            Box::new(|f| f.sent[0].outcome = SendOutcome::Failed { error: "x".into() }),
        ),
        (
            "foreign hash",
            Box::new(|f| f.sent[0].frame_sha256 = "0".repeat(64)),
        ),
        (
            "reaped unknown presence",
            Box::new(|f| f.process.pid_present_after_reap = None),
        ),
        (
            "unreaped known presence",
            Box::new(|f| f.process.reap = ReapFact::NotReaped),
        ),
        (
            "deadline kill without timeout",
            Box::new(|f| {
                f.process.kill = KillFact::Sent {
                    reason: KillReason::Deadline,
                }
            }),
        ),
        (
            "overflow at eof",
            Box::new(|f| f.stdout.overflow_bytes_observed = 1),
        ),
        (
            "total limit short",
            Box::new(|f| f.stdout.end = StreamEnd::TotalLimit),
        ),
        (
            "frame limit stderr",
            Box::new(|f| f.stderr.end = StreamEnd::FrameLimit),
        ),
        ("pid 0", Box::new(|f| f.process.pid = 0)),
    ];
    for (name, edit) in &bad {
        let (mut f, raw, frame) = clean_facts();
        edit(&mut f);
        assert!(
            check_attempt_evidence(&f, &raw, b"", &frame).is_err(),
            "{name}"
        );
    }
    let (f, raw, _) = clean_facts();
    assert!(
        check_attempt_evidence(&f, &raw, b"", b"{}").is_err(),
        "sent bytes must join the records"
    );
}

/// Real captured refusal bundles for the qualifier's approved refusal scenarios: a delivery
/// refusal from the real SDK (valid observation, then nonzero exit), a setup/spawn refusal (real
/// spawn failure) and a consumer refusal (correct native delivery, stale expected revision).
#[test]
fn refusal_controls_are_captured_for_the_qualifier() {
    let env = run_env();
    let controls = env.run_dir.join("controls");
    let w = world();
    let state = begun();
    let writer = authority();
    let cases: [(&str, Control, Option<Revision>, bool); 3] = [
        (
            "delivery-refusal",
            Control::ExitNonzeroAfterObservation,
            None,
            false,
        ),
        ("setup-refusal", Control::Positive, None, true),
        (
            "consumer-refusal",
            Control::Positive,
            Some(Revision("f".repeat(64))),
            false,
        ),
    ];
    for (name, control, revision, spawn_fails) in cases {
        let ev = Evidence::create_at(&env, controls.join(name)).unwrap();
        let mut case = base(&w, &state, &writer);
        if let Some(r) = revision {
            case.revision = r;
        }
        let binding = binding_for(&case, &result_json(|_| {}));
        let root = scratch_root(&env, &format!("control-{name}"));
        let mut cfg = native_config(root, control);
        if spawn_fails {
            cfg.node = env.run_dir.join("no-such-node-executable");
        }
        let (outcome, _capture) = DeliveryCapture::invoke(ev, &cfg, &binding, true, AUTHORITY);
        match (name, &outcome) {
            ("delivery-refusal", Err(DeliverError::Delivery { .. })) => {
                assert_eq!(binding.consumer_invocations(), 0)
            }
            ("setup-refusal", Err(DeliverError::Native(_))) => {
                assert_eq!(binding.consumer_invocations(), 0)
            }
            ("consumer-refusal", Err(DeliverError::Consumer { .. })) => {
                assert_eq!(binding.consumer_invocations(), 1)
            }
            (n, o) => panic!(
                "{n}: unexpected outcome {:?}",
                o.as_ref().err().map(|e| e.to_string())
            ),
        }
    }
}

/// Every duplicate family reaches the actual native stdout/capture boundary and must refuse
/// with zero actual consumer invocations. Raw frames remain byte-identical in capture.
#[test]
fn recursive_wire_duplicates_refuse_before_consumption_and_retain_exact_bytes() {
    use work_engine_review_delivery::native::WireObject;
    let env = run_env();
    let w = world();
    let state = begun();
    let writer = authority();
    for object in [
        WireObject::Allocation,
        WireObject::Observation,
        WireObject::Context,
        WireObject::History,
    ] {
        for conflict in [false, true] {
            for reversed in [false, true] {
                let name = format!("wire-{object:?}-{conflict}-{reversed}");
                let binding = binding_for(&base(&w, &state, &writer), &result_json(|_| {}));
                let root = scratch_root(&env, &name);
                let cfg = native_config(root, Control::DuplicateMember(object, conflict, reversed));
                let dir = env.run_dir.join("controls").join(&name);
                let ev = Evidence::create_at(&env, dir.clone()).unwrap();
                let (outcome, _capture) =
                    DeliveryCapture::invoke(ev, &cfg, &binding, true, AUTHORITY);
                assert_eq!(binding.consumer_invocations(), 0, "{name}");
                let Err(DeliverError::Delivery { run, .. }) = &outcome else {
                    panic!("{name}: expected delivery refusal: {outcome:?}")
                };
                assert!(!run.interpretation().permitted_success(), "{name}");
                assert!(
                    run.interpretation()
                        .defects
                        .iter()
                        .any(|d| d.contains("duplicate key")),
                    "{name}: {:?}",
                    run.interpretation()
                );
                assert_eq!(
                    std::fs::read(dir.join("native-frames-received.jsonl")).unwrap(),
                    run.raw_stdout
                );
                let cap = read_json(&dir.join("capture.json"));
                assert_eq!(cap["outcome"]["native_started"], true);
                assert_eq!(cap["consumer_invocations"], 0);
                assert_reaped(&run.facts.process);
            }
        }
    }
}

/// Each supported underlying stage survives a later wrong-verdict assertion. Positive delivery
/// is also the nonzero duplicate-consumption counterexample: assertions cannot discard count=1.
#[test]
fn wrong_control_verdicts_and_manifest_failure_retain_returned_facts() {
    let env = run_env();
    let w = world();
    let state = begun();
    let writer = authority();
    for name in [
        "wrong-delivered",
        "wrong-delivery-refusal",
        "wrong-setup-refusal",
        "wrong-consumer-refusal",
    ] {
        let mut case = base(&w, &state, &writer);
        if name == "wrong-consumer-refusal" {
            case.revision = Revision("f".repeat(64));
        }
        let binding = binding_for(&case, &result_json(|_| {}));
        let dir = env.run_dir.join("controls").join(name);
        let ev = Evidence::create_at(&env, dir.clone()).unwrap();
        let root = scratch_root(&env, name);
        let mut cfg = native_config(
            root.clone(),
            if name == "wrong-delivery-refusal" {
                Control::ExitNonzeroAfterObservation
            } else {
                Control::Positive
            },
        );
        if name == "wrong-setup-refusal" {
            cfg.node = env.run_dir.join("no-node");
        }
        let unwound = catch_unwind(AssertUnwindSafe(|| {
            let (outcome, _capture) = DeliveryCapture::invoke(ev, &cfg, &binding, true, AUTHORITY);
            if name == "wrong-delivered" {
                assert_eq!(
                    binding.consumer_invocations(),
                    0,
                    "duplicate control unexpectedly consumed"
                );
            }
            panic!("injected wrong control outcome: {outcome:?}");
        }));
        assert!(unwound.is_err());
        let manifest = read_json(&dir.join("output-manifest.json"));
        assert_eq!(manifest["outcome"], "failed");
        assert!(dir.join("capture.json").is_file());
        assert!(dir.join("failure.json").is_file());
        assert!(root.is_dir());
        if name == "wrong-delivered" {
            assert_eq!(
                read_json(&dir.join("capture.json"))["consumer_invocations"],
                1
            );
            assert!(dir.join("episode-after.json").is_file());
        }
    }
    // This test's successful verdict establishes preservation under failed finalization,
    // never a successful underlying bundle. It is deliberately outside complete controls.
    let dir = env.run_dir.join("unfinalized-manifest-control");
    let ev = Evidence::create_at(&env, dir.clone()).unwrap();
    std::fs::create_dir(dir.join("output-manifest.json")).unwrap();
    let root = scratch_root(&env, "manifest-write-failure");
    let cfg = native_config(root.clone(), Control::Positive);
    let binding = binding_for(&base(&w, &state, &writer), &result_json(|_| {}));
    let failed = catch_unwind(AssertUnwindSafe(|| {
        let (_outcome, _capture) = DeliveryCapture::invoke(ev, &cfg, &binding, false, AUTHORITY);
    }));
    assert!(failed.is_err(), "manifest failure cannot authorize success");
    assert!(dir.join("output-manifest.json").is_dir());
    assert!(dir.join("native-frames-received.jsonl").is_file());
    assert!(dir.join("episode-after.json").is_file());
    assert!(root.is_dir(), "manifest failure cannot authorize cleanup");
}
