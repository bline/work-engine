#![allow(dead_code)]
//! Shared test support. The compiler and the episode core are the real owners; the requirements,
//! selection and episode authority are the frozen fixture bytes, whose digests are asserted
//! against `provenance.json`'s record. Frozen provenance is a source of fixture values, not a
//! production binding.
use review_episode_core::codec::{JsValue, canonical_json, parse_json};
use review_episode_core::command::BeginCommand;
use review_episode_core::identity::Authority;
use review_episode_core::reducer::begin;
use review_episode_core::state::ReviewEpisodeState;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use work_engine_organization::{
    OrganizationCandidate, OrganizationalRequirements, Profile, derive,
};
use work_engine_review_preparation::{
    MutationClaim, PreparationRequest, RealizationClaim, Subject,
};

pub const REQ: &[u8] =
    include_bytes!("../../../work-engine-organization/tests/fixtures/frozen/requirements.json");
pub const SELECTION: &[u8] =
    include_bytes!("../../../work-engine-organization/tests/fixtures/frozen/selection.json");
pub const AUTHORITY: &str =
    include_str!("../../../work-engine-organization/tests/fixtures/frozen/episode-authority.json");
pub const ACCEPTABLE: &str = include_str!(
    "../../../../../app-server/tests/fixtures/implementation-review/acceptable-as-is.json"
);

pub const OBLIGATION: &str = "rp3-initial-implementation-review";
pub const COND_BUILDER: &str = "cond-builder-projection";
pub const COND_CAMPAIGN: &str = "cond-campaign-terminalization";

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn candidate_from(bytes: &[u8]) -> OrganizationCandidate {
    let requirements = OrganizationalRequirements::from_json_bytes(bytes).expect("owner accepts");
    let mut derived = derive(&requirements, Profile::FixedSliceReviewV1);
    assert!(derived.unresolved.is_empty(), "{:?}", derived.unresolved);
    assert_eq!(derived.candidates.len(), 1);
    derived.candidates.remove(0)
}

pub fn candidate() -> OrganizationCandidate {
    candidate_from(REQ)
}

/// The frozen requirements with `edit` applied, as authored JSON bytes.
pub fn edited(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut value: Value = serde_json::from_slice(REQ).unwrap();
    edit(&mut value);
    serde_json::to_vec(&value).unwrap()
}

pub fn for_each_condition(value: &mut Value, mut edit: impl FnMut(&mut Value)) {
    for condition in value["acceptanceConditions"].as_array_mut().unwrap() {
        edit(condition);
    }
}

/// The exact subject of the frozen vertical (selection.json and episode-authority.json).
pub fn subject() -> Subject {
    Subject {
        commit: "96808de3d565a6c4b0211e8dc263cb7343514d74".to_owned(),
        tree: "9c5d94ba059ea5b4ff497a0540f6b8a7476e4474".to_owned(),
        patch_identity: "5212141d54e001b1fa85f7660fbf9605a957cf11a7636636dc03d11dded58c51"
            .to_owned(),
    }
}

pub fn request(realization: Option<RealizationClaim>) -> PreparationRequest {
    PreparationRequest {
        obligation_id: OBLIGATION.to_owned(),
        subject: subject(),
        realization,
    }
}

/// A descriptor whose every stated value matches the candidate's requirements. Still a claim.
pub fn matching_claim() -> RealizationClaim {
    RealizationClaim {
        provider_family: Some("anthropic".to_owned()),
        execution_confinement: Some("guarded_sandbox".to_owned()),
        mutation: MutationClaim::ReadOnlyClaimed,
        granted_tools: vec![],
        continuity: Some("fresh_initial".to_owned()),
    }
}

/// The frozen episode authority, begun through the real reducer entry point.
pub fn authority() -> Authority {
    Authority::parse(parse_json(AUTHORITY).unwrap()).unwrap()
}

pub fn begun() -> ReviewEpisodeState {
    let command = BeginCommand::new(authority(), "begin-1", vec![]).unwrap();
    begin(None, &command).unwrap().into_state()
}

pub fn canonical(state: &ReviewEpisodeState) -> String {
    canonical_json(state.value())
}

pub fn json_value(js: &JsValue) -> Value {
    serde_json::from_str(&canonical_json(js)).unwrap()
}

/// The acceptable result sample with its subject replaced by the frozen subject.
pub fn result_json(edit: impl FnOnce(&mut Value)) -> String {
    let mut value: Value = serde_json::from_str(ACCEPTABLE).unwrap();
    value["subject"] = json!({
        "commit": subject().commit,
        "tree": subject().tree,
        "patchIdentity": subject().patch_identity,
    });
    edit(&mut value);
    serde_json::to_string(&value).unwrap()
}

pub fn reference(owner: &str, name: &str, revision: &str) -> Value {
    json!({
        "owner": owner,
        "reference": name,
        "revision": revision,
        "sha256": sha256_hex(name.as_bytes()),
        "freshness": "exact_revision",
    })
}

#[cfg(feature = "fixture")]
pub mod world {
    use super::*;
    use work_engine_review_preparation::fixture::{FIXTURE_LABEL, FixtureDeclaration};
    pub struct World {
        pub candidate: OrganizationCandidate,
        pub declaration: FixtureDeclaration,
        pub admissions: Vec<Value>,
    }

    fn identity_of(authority: &Value, key: &str) -> Value {
        authority["identity"][key].clone()
    }

    /// A synthetic world: the declaration and the admissions that agree with it.
    pub fn world() -> World {
        let candidate = candidate();
        let selection: Value = serde_json::from_slice(SELECTION).unwrap();
        let authority: Value = serde_json::from_str(AUTHORITY).unwrap();
        let mut bindings = Vec::new();
        let mut admissions = Vec::new();
        for claim in selection["specialists"][0]["requiredClaims"]
            .as_array()
            .unwrap()
        {
            let boundary = claim["consumptionBoundary"].as_str().unwrap();
            let consumer = claim["consumer"].as_str().unwrap();
            let condition = match boundary {
                "builder_projection" => COND_BUILDER,
                "campaign_terminalization" => COND_CAMPAIGN,
                other => panic!("unexpected boundary {other}"),
            };
            let claim_ref = reference(
                "fixture-claims",
                claim["claimId"].as_str().unwrap(),
                claim["revision"].as_str().unwrap(),
            );
            let establishment = reference(
                "fixture-establisher",
                &format!("establishment:{boundary}"),
                "est-r1",
            );
            let observation = reference(
                "fixture-observer",
                &format!("observation:{boundary}"),
                "obs-r1",
            );
            let consumption = reference(
                "fixture-consumption",
                &format!("consumption:{boundary}"),
                "cons-r1",
            );
            bindings.push(json!({
            "condition_id": condition,
            "boundary": boundary,
            "consumer_participant": if boundary == "builder_projection" { "vantage-builder" } else { "component:campaign" },
            "admission_consumer": consumer,
            "expected": {
                "claim_revision": claim_ref,
                "establishment": establishment,
                "observation": observation,
                "consumption": consumption,
            },
        }));
            admissions.push(json!({
                "claimRevisionRef": claim_ref,
                "establishmentRef": establishment,
                "observationRef": observation,
                "consumptionRef": consumption,
                "status": "established",
                "boundary": boundary,
                "consumer": consumer,
            }));
        }
        let declaration: FixtureDeclaration = serde_json::from_value(json!({
            "assumption_label": FIXTURE_LABEL,
            "candidate_content_digest": candidate.content_digest().to_hex(),
            "obligation_id": OBLIGATION,
            "problem_scope": {
                "run_id": "production-qualification-20261107",
                "plan_lineage": "timestamp-projection-v1",
                "slice_number": 1,
            },
            "subject": {
                "commit": subject().commit,
                "tree": subject().tree,
                "patchIdentity": subject().patch_identity,
            },
            "episode": {
                "run_id": identity_of(&authority, "runId"),
                "slice_number": identity_of(&authority, "sliceNumber"),
                "attempt_id": identity_of(&authority, "attemptId"),
                "plan_version": identity_of(&authority, "planVersion"),
                "review_obligation_id": identity_of(&authority, "reviewObligationId"),
                "review_episode_id": identity_of(&authority, "reviewEpisodeId"),
            },
            "bindings": bindings,
        }))
        .unwrap();
        World {
            candidate,
            declaration,
            admissions,
        }
    }
}

/// Unique run-owned evidence (RA003). The custody plan supplies a fresh `WE_RUN_DIR` and the
/// identities the output manifest must bind. These tests REQUIRE them and fail, never default.
#[cfg(feature = "fixture")]
pub mod run_evidence {
    use super::sha256_hex;
    use serde_json::{Value, json};
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;

    #[derive(Debug, Clone)]
    pub struct RunEnv {
        pub run_dir: PathBuf,
        pub invocation_id: String,
        pub subject_manifest_sha256: String,
        pub map_sha256: String,
    }

    fn required(name: &str) -> String {
        std::env::var(name).unwrap_or_else(|_| {
            panic!("{name} is required: qualified native runs get a fresh run directory and identities from the custody plan")
        })
    }

    pub fn run_env() -> RunEnv {
        let run_dir = PathBuf::from(required("WE_RUN_DIR"));
        assert!(run_dir.is_absolute(), "WE_RUN_DIR must be absolute");
        RunEnv {
            run_dir,
            invocation_id: required("WE_INVOCATION_ID"),
            subject_manifest_sha256: required("WE_SOURCE_MANIFEST_SHA256"),
            map_sha256: required("WE_MAP_SHA256"),
        }
    }

    /// A fresh scratch root for one native control; fails if it already exists.
    pub fn scratch_root(env: &RunEnv, name: &str) -> PathBuf {
        let parent = env.run_dir.join("scratch");
        fs::create_dir_all(&parent).expect("run directory is writable");
        let root = parent.join(format!("{name}-{}", std::process::id()));
        fs::create_dir(&root).expect("scratch root must be new (fail on existing)");
        root
    }

    /// A scratch root whose removal depends on preservation (R4 capture lifetime). `cleanup`
    /// roots are removed on drop; `retained` roots are removed only after [`Scratch::release`],
    /// which a caller invokes once the evidence bundle has been finalized `passed`. Removal never
    /// panics, so unwinding through a failed assertion cannot delete the only recoverable copy.
    pub struct Scratch {
        pub path: PathBuf,
        release: bool,
    }

    impl Scratch {
        pub fn cleanup(path: PathBuf) -> Self {
            Self {
                path,
                release: true,
            }
        }
        pub fn retained(path: PathBuf) -> Self {
            Self {
                path,
                release: false,
            }
        }
        pub fn release(&mut self) {
            self.release = true;
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            if self.release {
                let _ = fs::remove_dir_all(&self.path);
            }
        }
    }

    /// One invocation-owned evidence bundle. The directory is created with `create_dir`, so a
    /// second run into the same directory fails instead of replacing earlier evidence, and every
    /// file is written `create_new`. Writes never panic: a failed write is recorded and makes the
    /// bundle `incomplete`, whatever outcome the caller asks for. The manifest is written last.
    pub struct Evidence {
        dir: PathBuf,
        env: RunEnv,
        files: Vec<(String, usize, String)>,
        write_failures: Vec<Value>,
    }

    pub const OUTPUT_SCHEMA: &str = "org-run-output-manifest/v2";

    impl Evidence {
        /// The positive run's bundle: `<run>/evidence`.
        pub fn create(env: &RunEnv) -> Result<Self, String> {
            Self::create_at(env, env.run_dir.join("evidence"))
        }
        /// A bundle at an explicit new directory (controls live under `<run>/controls/<name>`).
        pub fn create_at(env: &RunEnv, dir: PathBuf) -> Result<Self, String> {
            if let Some(parent) = dir.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            fs::create_dir(&dir).map_err(|e| {
                format!(
                    "{}: {e} (evidence directories are new; refusing to reuse)",
                    dir.display()
                )
            })?;
            Ok(Self {
                dir,
                env: env.clone(),
                files: Vec::new(),
                write_failures: Vec::new(),
            })
        }
        pub fn dir(&self) -> &std::path::Path {
            &self.dir
        }
        pub fn env(&self) -> &RunEnv {
            &self.env
        }
        pub fn write_failures(&self) -> &[Value] {
            &self.write_failures
        }
        /// Writes one file create-new. A failure is recorded, not raised.
        pub fn put(&mut self, name: &str, bytes: &[u8]) -> bool {
            let outcome = if name.contains('/') || name == "output-manifest.json" {
                Err(format!("reserved or nested evidence name `{name}`"))
            } else {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(self.dir.join(name))
                    .and_then(|mut file| file.write_all(bytes).and_then(|_| file.sync_all()))
                    .map_err(|e| e.to_string())
            };
            match outcome {
                Ok(()) => {
                    self.files
                        .push((name.to_owned(), bytes.len(), sha256_hex(bytes)));
                    true
                }
                Err(error) => {
                    self.write_failures
                        .push(json!({"name": name, "error": error}));
                    false
                }
            }
        }
        pub fn put_json(&mut self, name: &str, value: &Value) -> bool {
            match serde_json::to_vec_pretty(value) {
                Ok(bytes) => self.put(name, &bytes),
                Err(e) => {
                    self.write_failures
                        .push(json!({"name": name, "error": e.to_string()}));
                    false
                }
            }
        }
        /// Writes the output manifest (last) and returns the outcome actually recorded:
        /// `requested` only when every write succeeded, otherwise `incomplete`. `Err` means the
        /// manifest itself could not be written; the partial files stay where they are.
        pub fn finish(self, requested: &str) -> Result<String, String> {
            let outcome = if self.write_failures.is_empty() {
                requested.to_owned()
            } else {
                "incomplete".to_owned()
            };
            let files: Vec<Value> = self
                .files
                .iter()
                .map(|(n, l, h)| json!({"name": n, "length": l, "sha256": h}))
                .collect();
            let manifest = json!({
                "schema": OUTPUT_SCHEMA,
                "outcome": outcome,
                "invocation_id": self.env.invocation_id,
                "source_manifest_sha256": self.env.subject_manifest_sha256,
                "map_sha256": self.env.map_sha256,
                "files": files,
                "write_failures": self.write_failures,
            });
            let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.dir.join("output-manifest.json"))
                .and_then(|mut file| file.write_all(&bytes).and_then(|_| file.sync_all()))
                .map_err(|e| format!("output manifest: {e}"))?;
            Ok(outcome)
        }
    }
}

/// The fixture evidence writer (R4 capture lifetime). Inputs are persisted before the native path
/// starts; whatever `deliver_and_consume` returns, every available raw frame, native
/// context/history, process disposition, result and actual state is persisted before any
/// assertion or cleanup can run. Absence is described per observation and failure stage. Nothing
/// is reconstructed. The campaign qualifier re-reads exactly these files.
#[cfg(feature = "fixture")]
pub mod capture {
    use super::run_evidence::Evidence;
    use super::{AUTHORITY, REQ, canonical};
    use serde_json::{Value, json};
    use work_engine_organization::Profile;
    use work_engine_review_delivery::binding::ExperimentBinding;
    use work_engine_review_delivery::native::{DeliverError, Delivered, NativeRun};
    use work_engine_review_preparation::fixture::FixtureConsumption;

    pub const TRANSITION_ID: &str = "result-1";

    /// The delivery-owned process facts, serialized exactly (kill, reap, timeout, descendants).
    pub fn process_json(run: &NativeRun) -> Value {
        serde_json::to_value(&run.facts.process).unwrap_or(Value::Null)
    }

    /// The delivery-owned send/stream/timing facts, the delivery-owned interpretation of the whole
    /// stream, the delivery-owned incompleteness verdict and the consumer invocation count the
    /// caller supplies (the binding's runtime counter for a real run). Written before any
    /// assertion; nothing here is reconstructed.
    pub fn collection_json(run: &NativeRun, consumer_invocations: u64) -> Value {
        let interp = run.interpretation();
        json!({
            "sent": run.facts.sent,
            "stdout": run.facts.stdout,
            "stderr": run.facts.stderr,
            "timing": run.facts.timing,
            "frame_kinds": interp.kinds,
            "frame_defects": interp.defects,
            "incompleteness": run.incompleteness(),
            "consumer_invocations": consumer_invocations,
        })
    }

    /// The consumer's outcome and full condition coverage, as retained for comparison with the
    /// qualifier's pure recomputation.
    pub fn consumption_json(c: &FixtureConsumption) -> Value {
        let outcome = match c.outcome() {
            review_episode_core::reducer::TransitionOutcome::Applied(_) => "applied",
            review_episode_core::reducer::TransitionOutcome::Replay(_) => "replay",
        };
        json!({
            "label": format!("{:?}", c.label),
            "evidence": format!("{:?}", c.evidence),
            "production_consequence": format!("{:?}", c.production_consequence),
            "outcome": outcome,
            "phase": format!("{:?}", c.state().phase()),
            "revision": c.state().revision().0,
            "conditions": c.conditions.iter().map(|k| json!({
                "condition": k.condition, "acceptor": k.acceptor,
                "unestablished_route": k.unestablished_route, "acceptance": format!("{:?}", k.acceptance),
            })).collect::<Vec<_>>(),
        })
    }

    /// Stage 1, before any native child exists: the exact consumer inputs, preparation, contract
    /// and before-state. `authority_text` and `questions` are the values the binding was built from.
    pub fn persist_inputs(
        ev: &mut Evidence,
        binding: &ExperimentBinding,
        authority_text: &str,
        questions: &[String],
    ) {
        let frozen = binding.frozen();
        let request = binding.request();
        ev.put("candidate-requirements.json", REQ);
        ev.put("authority.json", authority_text.as_bytes());
        ev.put_json("consumer-inputs.json", &json!({
            "profile": Profile::FixedSliceReviewV1.as_str(),
            "request": {"obligation_id": request.obligation_id, "subject": request.subject, "realization": request.realization},
            "transition_id": binding.transition_id(),
            "unresolved_questions": questions,
            "expected_revision": binding.expected_revision().0,
            "transport": frozen.transport(),
            "authority_is_frozen_fixture": authority_text == AUTHORITY,
        }));
        ev.put(
            "preparation.json",
            &serde_json::to_vec(binding.preparation()).unwrap_or_default(),
        );
        ev.put("contract.bytes", frozen.contract().as_bytes());
        ev.put_json(
            "declaration.json",
            &serde_json::to_value(binding.declaration()).unwrap_or(Value::Null),
        );
        ev.put("admissions.json", binding.admissions_json().as_bytes());
        ev.put(
            "synthetic-result.json",
            binding.synthetic_result().as_bytes(),
        );
        ev.put(
            "episode-before.json",
            canonical(binding.episode_state()).as_bytes(),
        );
    }

    fn run_of(outcome: &Result<Delivered, DeliverError>) -> Option<&NativeRun> {
        match outcome {
            Ok(d) => Some(&d.run),
            Err(DeliverError::Delivery { run, .. }) | Err(DeliverError::Consumer { run, .. }) => {
                Some(run)
            }
            Err(DeliverError::Native(_)) => None,
        }
    }

    /// Stage 2, immediately after `deliver_and_consume` returns, before any assertion: every
    /// available observation, plus `capture.json` naming each absent one and why.
    /// `consumer_invocations` is the binding's runtime counter for a real run; a test-local
    /// synthetic bundle supplies its modelled count here and is labelled synthetic by its test.
    pub fn capture_returned(
        ev: &mut Evidence,
        binding: &ExperimentBinding,
        outcome: &Result<Delivered, DeliverError>,
        consumer_invocations: u64,
    ) {
        let env_ids = (
            ev_invocation(ev),
            binding.frozen().transport().clone(),
            binding.frozen().contract_sha256().to_owned(),
        );
        let mut absent: Vec<Value> = Vec::new();
        let projection = work_engine_review_delivery::native::OutcomeProjection::of(outcome);
        let (stage, error) = match outcome {
            Ok(_) => ("delivered_and_consumed", None),
            Err(e @ DeliverError::Native(_)) => ("native_error", Some(e.to_string())),
            Err(e @ DeliverError::Delivery { .. }) => ("delivery_refused", Some(e.to_string())),
            Err(e @ DeliverError::Consumer { .. }) => ("consumer_refused", Some(e.to_string())),
        };
        let run = run_of(outcome);
        match run {
            Some(run) => {
                // Raw bytes exactly as received/sent/collected: no joining, trimming or decoding.
                ev.put("native-frames-received.jsonl", &run.raw_stdout);
                // Exact bytes of every frame whose send began (text plus newline each).
                ev.put("native-frames-sent.jsonl", &run.sent_bytes);
                ev.put("stderr.txt", &run.raw_stderr);
                ev.put_json("process.json", &process_json(run));
                ev.put_json(
                    "collection.json",
                    &collection_json(run, consumer_invocations),
                );
                if let Some(e) = &run.error_frame {
                    ev.put("native-error-frame.txt", e.as_bytes());
                }
                match &run.observation {
                    Some(obs) => {
                        ev.put_json(
                            "observation.json",
                            &serde_json::to_value(obs).unwrap_or(Value::Null),
                        );
                        match &obs.context {
                            Some(ctx) => {
                                ev.put_json("request-context.json", ctx);
                            }
                            None => absent.push(json!({"name": "request-context.json", "reason": "the native observation carries no request context"})),
                        }
                        ev.put_json("native-history.json", &json!(obs.history));
                    }
                    None => {
                        for name in [
                            "observation.json",
                            "request-context.json",
                            "native-history.json",
                        ] {
                            absent.push(json!({"name": name, "reason": "the native run returned no observation frame"}));
                        }
                    }
                }
            }
            None => {
                for name in [
                    "native-frames-received.jsonl",
                    "native-frames-sent.jsonl",
                    "stderr.txt",
                    "process.json",
                    "collection.json",
                    "observation.json",
                    "request-context.json",
                    "native-history.json",
                ] {
                    absent.push(json!({"name": name, "reason": "the native child was not started (setup/spawn error carries no run)"}));
                }
            }
        }
        let bound = run.and_then(|r| r.bound.as_ref());
        let verified = match outcome {
            Ok(d) => Some(&d.verified),
            Err(DeliverError::Consumer { verified, .. }) => Some(verified.as_ref()),
            _ => None,
        };
        ev.put_json("identities.json", &json!({
            "invocation_id": env_ids.0.0, "source_manifest_sha256": env_ids.0.1, "map_sha256": env_ids.0.2,
            "attempt_id": env_ids.1.attempt_id, "generation": env_ids.1.generation,
            "contract_sha256": env_ids.2,
            "session_id": bound.map(|b| b.session_id().to_owned()),
            "sdk_version": bound.map(|b| b.allocation().sdk_version.clone()),
            "node_version": bound.map(|b| b.allocation().node_version.clone()),
            "effective_system_context_sha256": verified.map(|v| v.effective_system_context_sha256.clone()),
            "verified_result_text_sha256": verified.map(|v| super::sha256_hex(v.result_text.as_bytes())),
        }));
        match outcome {
            Ok(d) => {
                let after = d.consumption.state();
                ev.put("episode-after.json", canonical(after).as_bytes());
                ev.put_json("consumption.json", &consumption_json(&d.consumption));
                ev.put_json(
                    "revisions.json",
                    &json!({
                        "before": binding.episode_state().revision().0,
                        "expected": binding.expected_revision().0,
                        "after": after.revision().0,
                    }),
                );
            }
            Err(_) => {
                for name in ["episode-after.json", "consumption.json", "revisions.json"] {
                    absent
                        .push(json!({"name": name, "reason": format!("no consumption: {stage}")}));
                }
            }
        }
        ev.put_json(
            "capture.json",
            &json!({"stage": stage, "outcome": projection, "error": error, "absent": absent,
                "consumer_invocations": consumer_invocations}),
        );
    }

    fn ev_invocation(ev: &Evidence) -> (String, String, String) {
        let env = ev.env();
        (
            env.invocation_id.clone(),
            env.subject_manifest_sha256.clone(),
            env.map_sha256.clone(),
        )
    }
}
