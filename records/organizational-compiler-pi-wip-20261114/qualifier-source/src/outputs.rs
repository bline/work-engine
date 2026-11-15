//! Retained-output qualification through the existing domain owners (PLAN-R4).
//!
//! - Stored states: `ReviewEpisodeState::from_stored_json` on the exact retained text (no
//!   parse/reserialize first, so representation defects stay visible).
//! - Delivery join: `ExperimentBinding::new` rebuilt from the retained candidate requirements
//!   (pinned to the frozen fixture in the source closure), request, declaration, before-state,
//!   authority, expected revision, transition id, questions, admissions and synthetic result; its
//!   canonical preparation and frozen contract must equal the retained bytes.
//! - Native correspondence: the delivery adapter's pure `BoundAttempt::bind`/`verify` on the
//!   decoded allocation and observation, plus exact protocol-line accounting.
//! - Transition: the unchanged pure `consume_fixture_result` against the validated before-state;
//!   its canonical state and consumption must equal the retained after-state and consumption.
//!
//! The recomputation is a deterministic consistency check. It is not a newly observed native
//! execution and grants no authority.
use crate::base::{sha256_hex, RunVars};
use crate::ledger::{Defects, Ledger};
use crate::spec::{ControlSpec, Repr, ValidatedSpec};
use crate::strict::{self, is_hex64, Obj};
use crate::{Rec, SourceManifest};
use review_episode_core::codec::{canonical_json, parse_json};
use review_episode_core::identity::{Authority, Revision};
use review_episode_core::reducer::TransitionOutcome;
use review_episode_core::state::ReviewEpisodeState;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use work_engine_organization::{
    derive, OrganizationCandidate, OrganizationalRequirements, Profile,
};
use work_engine_review_delivery::attempt::{
    check_attempt_evidence, AttemptFacts, ProcessFacts, SendRecord, StreamRecord, TimingFacts,
};
use work_engine_review_delivery::binding::{EpisodeInputs, ExperimentBinding};
use work_engine_review_delivery::native::{DeliveryStage, OutcomeProjection};
use work_engine_review_delivery::{
    interpret_stream, Allocation, BoundAttempt, NativeObservation, Transport, VerifiedDelivery,
};
use work_engine_review_preparation::fixture::{
    consume_fixture_result, FixtureConsumption, FixtureDeclaration,
};
use work_engine_review_preparation::{PreparationRequest, RealizationClaim, Subject};

pub const OUTPUT_SCHEMA: &str = "org-run-output-manifest/v2";
const MANIFEST_KEYS: &[&str] = &[
    "schema",
    "outcome",
    "invocation_id",
    "source_manifest_sha256",
    "map_sha256",
    "files",
    "write_failures",
];

/// Typed decoding that refuses unknown, missing or non-canonically represented fields: the
/// value must round-trip through the owner type unchanged.
fn typed<T: DeserializeOwned + Serialize>(v: &Value, what: &str) -> Result<T, String> {
    let t: T = serde_json::from_value(v.clone()).map_err(|e| format!("{what}: {e}"))?;
    let back = serde_json::to_value(&t).map_err(|e| format!("{what}: {e}"))?;
    if back != *v {
        return Err(format!(
            "{what}: unknown or non-canonical fields (owner type does not round-trip)"
        ));
    }
    Ok(t)
}

fn utf8(b: &[u8], what: &str) -> Result<String, String> {
    String::from_utf8(b.to_vec()).map_err(|e| format!("{what}: not UTF-8: {e}"))
}

/// The consumer's outcome and complete condition coverage. Independent of the delivery test's
/// writer; both project the owner's `FixtureConsumption` and must agree byte-for-value.
pub fn consumption_projection(c: &FixtureConsumption) -> Value {
    let outcome = match c.outcome() {
        TransitionOutcome::Applied(_) => "applied",
        TransitionOutcome::Replay(_) => "replay",
    };
    json!({
        "label": format!("{:?}", c.label),
        "evidence": format!("{:?}", c.evidence),
        "production_consequence": format!("{:?}", c.production_consequence),
        "outcome": outcome,
        "phase": format!("{:?}", c.state().phase()),
        "revision": c.state().revision().0,
        "conditions": c.conditions.iter().map(|k| json!({
            "condition": k.condition,
            "acceptor": k.acceptor,
            "unestablished_route": k.unestablished_route,
            "acceptance": format!("{:?}", k.acceptance),
        })).collect::<Vec<_>>(),
    })
}

/// Decoded artifacts; an entry exists only when that artifact's check passed.
#[derive(Default)]
struct Art {
    bytes: BTreeMap<String, Vec<u8>>,
    json: BTreeMap<String, Value>,
    lines: BTreeMap<String, Vec<Value>>,
    states: BTreeMap<String, ReviewEpisodeState>,
}

struct Ctx {
    binding: ExperimentBinding,
    candidate: OrganizationCandidate,
    declaration: FixtureDeclaration,
    authority: Authority,
    before: ReviewEpisodeState,
    expected: Revision,
    transition_id: String,
    questions: Vec<String>,
    admissions: String,
    result: String,
}

struct Native {
    verified: VerifiedDelivery,
}

type Decoded = (
    Vec<u8>,
    Option<Value>,
    Option<Vec<Value>>,
    Option<ReviewEpisodeState>,
);

/// Decodes one retained artifact by its approved representation. `Raw` bytes are accounted
/// only; their meaning belongs to the delivery-owned stream interpretation.
fn decode_one(n: &str, repr: &Repr, b: Vec<u8>, d: &mut Defects) -> Option<Decoded> {
    match repr {
        Repr::Json => Some((b.clone(), Some(d.take(n, strict::parse(&b))?), None, None)),
        Repr::Jsonl => Some((
            b.clone(),
            None,
            Some(d.take(n, strict::parse_lines(&b))?),
            None,
        )),
        Repr::Raw => Some((b, None, None, None)),
        Repr::Utf8 | Repr::Text => {
            d.take(n, utf8(&b, n))?;
            Some((b, None, None, None))
        }
        Repr::CanonicalState => {
            let text = d.take(n, utf8(&b, n))?;
            match ReviewEpisodeState::from_stored_json(&text) {
                Ok(s) => Some((b, None, None, Some(s))),
                Err(e) => {
                    d.owner(n, "review-episode-core from_stored_json", e.to_string());
                    None
                }
            }
        }
    }
}

impl Art {
    fn put(&mut self, name: &str, (b, j, ln, st): Decoded) {
        self.bytes.insert(name.to_owned(), b);
        if let Some(j) = j {
            self.json.insert(name.to_owned(), j);
        }
        if let Some(ln) = ln {
            self.lines.insert(name.to_owned(), ln);
        }
        if let Some(st) = st {
            self.states.insert(name.to_owned(), st);
        }
    }
}

/// Reads one bundle's output manifest strictly and every listed file. Returns (manifest, bytes).
fn read_bundle(
    dir: &Path,
    run: &RunVars,
    d: &mut Defects,
) -> Option<(Value, BTreeMap<String, Vec<u8>>)> {
    let a = format!("{}/output-manifest.json", dir.display());
    let a = a.as_str();
    let bytes = match fs::read(dir.join("output-manifest.json")) {
        Ok(b) => b,
        Err(e) => {
            d.missing(a, e.to_string());
            return None;
        }
    };
    let m = d.take(a, strict::parse(&bytes))?;
    let o = d.take(a, Obj::new(a, &m))?;
    d.take(a, o.exact(MANIFEST_KEYS))?;
    if d.take(a, o.str("schema"))? != OUTPUT_SCHEMA {
        d.mismatch(a, "schema", OUTPUT_SCHEMA, &m["schema"], "");
    }
    for (key, want) in [
        ("invocation_id", &run.invocation_id),
        ("source_manifest_sha256", &run.manifest_sha256),
        ("map_sha256", &run.map_sha256),
    ] {
        let got = d.take(a, o.str(key))?;
        if got != want.as_str() {
            d.mismatch(a, key, want, got, "foreign invocation");
        }
    }
    d.take(a, o.str("outcome"))?;
    for (n, w) in d.take(a, o.arr("write_failures"))?.iter().enumerate() {
        let wo = d.take(a, Obj::new(format!("write_failures[{n}]"), w))?;
        d.take(a, wo.exact(&["name", "error"]))?;
        d.take(a, wo.str("name"))?;
        d.take(a, wo.str("error"))?;
    }
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for (n, f) in d.take(a, o.arr("files"))?.iter().enumerate() {
        let fo = d.take(a, Obj::new(format!("files[{n}]"), f))?;
        d.take(a, fo.exact(&["name", "length", "sha256"]))?;
        let (name, len, sha) = (
            d.take(a, fo.str("name"))?.to_owned(),
            d.take(a, fo.u64("length"))?,
            d.take(a, fo.str("sha256"))?.to_owned(),
        );
        if name.contains('/') || name == "output-manifest.json" {
            d.malformed(a, format!("reserved or nested name `{name}`"));
            continue;
        }
        if !fs::symlink_metadata(dir.join(&name))
            .map(|m| m.file_type().is_file())
            .unwrap_or(false)
        {
            d.malformed(
                &name,
                "manifest artifacts must be observed regular files, never links or directories",
            );
            continue;
        }
        match fs::read(dir.join(&name)) {
            Ok(b) if sha256_hex(&b) == sha && b.len() as u64 == len => {
                if files.insert(name.clone(), b).is_some() {
                    d.mismatch(a, "files", "unique names", &name, "listed twice");
                }
            }
            Ok(b) => d.mismatch(
                &name,
                "sha256/length",
                format!("{sha}/{len}"),
                format!("{}/{}", sha256_hex(&b), b.len()),
                "file differs from the output manifest",
            ),
            Err(e) => d.missing(&name, e.to_string()),
        }
    }
    let mut expected: BTreeSet<String> = files.keys().cloned().collect();
    expected.insert("output-manifest.json".into());
    for w in m["write_failures"].as_array()? {
        let name = w["name"].as_str()?;
        if name.contains('/') || name == "output-manifest.json" || files.contains_key(name) {
            d.malformed(a, "invalid or surviving write-failure target");
        }
        match fs::symlink_metadata(dir.join(name)) {
            Ok(meta) => {
                expected.insert(name.to_owned());
                // The only approved remnant is the actual empty blocking directory used by
                // the create_new control. Declarations cannot conceal surviving bytes/links.
                let empty = meta.file_type().is_dir()
                    && fs::read_dir(dir.join(name))
                        .map(|mut entries| entries.next().is_none())
                        .unwrap_or(false);
                if !empty {
                    d.malformed(
                        name,
                        "write-failure remnant must be an observed empty blocking directory",
                    );
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => d.malformed(name, format!("cannot observe write-failure remnant: {e}")),
        }
    }
    let actual: BTreeSet<String> = d
        .take(a, fs::read_dir(dir).map_err(|e| e.to_string()))?
        .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()
        .ok()?;
    if actual != expected {
        d.mismatch(
            a,
            "directory entries",
            format!("{expected:?}"),
            format!("{actual:?}"),
            "all supplied entries need an applicable declared role",
        );
    }
    Some((m, files))
}

pub fn evaluate(
    spec: &ValidatedSpec,
    l: &mut Ledger,
    done: &BTreeMap<String, Rec>,
    run: &RunVars,
    manifest: Option<&SourceManifest>,
    facts: &mut Map<String, Value>,
) {
    let producer = spec.producer.clone();
    let dir = PathBuf::from(&run.run_dir).join("evidence");
    let listed = l.run("output_manifest", |d| {
        let (m, files) = read_bundle(&dir, run, d)?;
        if m["outcome"] != "passed" {
            d.failed(
                "output-manifest.json",
                "outcome",
                "passed",
                &m["outcome"],
                "",
            );
        }
        if m["write_failures"] != json!([]) {
            d.failed(
                "output-manifest.json",
                "write_failures",
                "[]",
                &m["write_failures"],
                "a bundle with write failures is incomplete",
            );
        }
        let want: BTreeSet<String> = spec.artifacts.iter().map(|(n, _)| n.clone()).collect();
        let have: BTreeSet<String> = files.keys().cloned().collect();
        if have != want {
            d.mismatch(
                "output-manifest.json",
                "files",
                format!("{want:?}"),
                format!("{have:?}"),
                "retained artifacts differ from the specification's required set",
            );
        }
        let mut on_disk: BTreeSet<String> = BTreeSet::new();
        for e in fs::read_dir(&dir).ok()?.flatten() {
            on_disk.insert(e.file_name().to_string_lossy().into_owned());
        }
        let mut expected = have.clone();
        expected.insert("output-manifest.json".into());
        if on_disk != expected {
            d.mismatch(
                "evidence directory",
                "entries",
                format!("{expected:?}"),
                format!("{on_disk:?}"),
                "the directory holds entries the manifest does not account for",
            );
        }
        Some(files)
    });
    let mut art = Art::default();
    for (name, repr) in &spec.artifacts {
        let decoded = l.run(&format!("artifact:{name}"), |d| {
            let b = listed.as_ref()?.get(name)?.clone();
            decode_one(name, repr, b, d)
        });
        if let Some((b, j, ln, st)) = decoded {
            art.bytes.insert(name.clone(), b);
            if let Some(j) = j {
                art.json.insert(name.clone(), j);
            }
            if let Some(ln) = ln {
                art.lines.insert(name.clone(), ln);
            }
            if let Some(st) = st {
                art.states.insert(name.clone(), st);
            }
        }
    }
    l.run("output_window", |d| {
        let rec = done.get(&producer)?;
        let mut names: Vec<String> = listed.as_ref()?.keys().cloned().collect();
        names.push("output-manifest.json".into());
        for name in names {
            let t = fs::metadata(dir.join(&name))
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|x| x.as_secs_f64());
            match t {
                Some(t) if t >= rec.started - 1.0 && t <= rec.ended + 1.0 => {}
                Some(t) => d.mismatch(
                    &name,
                    "mtime",
                    format!("[{}, {}]", rec.started, rec.ended),
                    t,
                    "written outside the producer's custody window",
                ),
                None => d.missing(&name, "modification time unavailable"),
            }
        }
        Some(())
    });
    l.run("invocation_binding", |d| {
        let ident = Obj::new("identities.json", art.json.get("identities.json")?).ok()?;
        for (key, want) in [
            ("invocation_id", &run.invocation_id),
            ("source_manifest_sha256", &run.manifest_sha256),
            ("map_sha256", &run.map_sha256),
        ] {
            let got = d.take("identities.json", ident.str(key))?;
            if got != want.as_str() {
                d.mismatch("identities.json", key, want, got, "foreign invocation");
            }
        }
        let env = &done.get(&producer)?.value["env_set"];
        for (key, want) in [
            ("WE_INVOCATION_ID", &run.invocation_id),
            ("WE_RUN_DIR", &run.run_dir),
            ("WE_MAP_SHA256", &run.map_sha256),
            ("WE_SOURCE_MANIFEST_SHA256", &run.manifest_sha256),
        ] {
            if env[key].as_str() != Some(want.as_str()) {
                d.mismatch(
                    &format!("{producer}.json"),
                    &format!("env_set.{key}"),
                    want,
                    &env[key],
                    "the producer did not run under this invocation",
                );
            }
        }
        Some(())
    });
    let ctx = l.run("binding_recompute", |d| {
        binding_recompute(spec, &art, manifest?, d)
    });
    let native = l.run("native_protocol", |d| {
        native_protocol(spec, &art, ctx.as_ref()?, run, Completeness::Complete, d)
    });
    l.run("context_history", |d| {
        context_history(&art, ctx.as_ref()?, d)
    });
    l.run("state_revisions", |d| state_revisions(&art, d));
    l.run("transition_recompute", |d| {
        transition_recompute(&art, ctx.as_ref()?, native.as_ref()?, d)
    });
    l.run("process_disposition", |d| process_disposition(&art, d));
    l.run("capture_complete", |d| {
        appraise_bundle(spec, &art, &Scenario::positive(), run, manifest?, d)
    });
    for c in &spec.controls {
        l.run(&format!("control:{}", c.name), |d| {
            let _ = done.get(&producer)?;
            let cdir = PathBuf::from(&run.run_dir).join("controls").join(&c.name);
            let (m, files) = read_bundle(&cdir, run, d)?;
            let a = format!("controls/{}", c.name);
            if m["outcome"] != json!(c.outcome) {
                d.mismatch(&a, "outcome", &c.outcome, &m["outcome"], "");
            }
            for p in &c.preserved {
                if !files.contains_key(p) {
                    d.missing(&a, format!("{p} was not preserved"));
                }
            }
            let wf = m["write_failures"].as_array().cloned().unwrap_or_default();
            match &c.write_failure {
                Some(name)
                    if wf.len() != 1
                        || wf.first().and_then(|w| w["name"].as_str()) != Some(name.as_str()) =>
                {
                    d.mismatch(&a, "write_failures", name, format!("{wf:?}"), "")
                }
                None if !wf.is_empty() => {
                    d.mismatch(&a, "write_failures", "[]", format!("{wf:?}"), "")
                }
                _ => {}
            }
            // Complete semantic evaluation of every surviving artifact under the approved
            // scenario (stage and absences come from the control definition, not the bundle).
            control_semantics(spec, c, &files, run, manifest, d)?;
            Some(())
        });
    }
    facts.insert("artifacts_decoded".into(), json!(art.bytes.len()));
}

/// Which stage of the vertical a retained bundle claims, and which artifacts the approved
/// scenario definition (not the bundle) allows to be absent.
pub struct Scenario {
    pub stage: String,
    /// Artifact names the approved definition allows to be absent (e.g. the injected write failure).
    pub write_failure: Option<String>,
}

impl Scenario {
    pub fn positive() -> Self {
        Scenario {
            stage: "delivered_and_consumed".into(),
            write_failure: None,
        }
    }
    fn consumed(&self) -> bool {
        self.stage == "delivered_and_consumed"
    }
    /// Runtime consumer invocations the approved scenario requires.
    fn invocations(&self) -> u64 {
        u64::from(matches!(
            self.stage.as_str(),
            "delivered_and_consumed" | "consumer_refused"
        ))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Completeness {
    /// The whole attempt must be a complete clean run.
    Complete,
    /// A refusal scenario: the bundle must carry evidence that justifies the refusal.
    Refused,
}

fn context_history(art: &Art, ctx: &Ctx, d: &mut Defects) -> Option<()> {
    let obs: NativeObservation = d.take(
        "observation.json",
        typed(art.json.get("observation.json")?, "observation.json"),
    )?;
    let rc = art.json.get("request-context.json");
    if obs.context.as_ref() != rc {
        d.mismatch(
            "request-context.json",
            "",
            "the raw observation's context",
            "differs",
            "retained request context is not the observed context",
        );
    }
    let hist = art.json.get("native-history.json")?;
    if *hist != Value::Array(obs.history.clone()) {
        d.mismatch(
            "native-history.json",
            "",
            "the raw observation's history",
            "differs",
            "retained native history is not the observed history",
        );
    }
    let contract = utf8(art.bytes.get("contract.bytes")?, "contract.bytes").ok()?;
    if let Some(rc) = rc {
        let msgs = d.take(
            "request-context.json",
            Obj::new("request-context", rc).and_then(|o| o.arr("messages").cloned()),
        )?;
        let roles: Vec<&str> = msgs
            .iter()
            .map(|m| m["role"].as_str().unwrap_or("?"))
            .collect();
        if roles != ["system", "user"] {
            d.mismatch(
                "request-context.json",
                "messages",
                "[system, user]",
                format!("{roles:?}"),
                "",
            );
        } else if single_text(&msgs[1]).as_deref() != Some(contract.as_str()) {
            d.mismatch(
                "request-context.json",
                "messages[1]",
                "exact contract bytes",
                "differs",
                "",
            );
        }
    }
    let h = d.take(
        "native-history.json",
        hist.as_array()
            .cloned()
            .ok_or_else(|| "not an array".to_owned()),
    )?;
    let hroles: Vec<&str> = h
        .iter()
        .map(|m| m["role"].as_str().unwrap_or("?"))
        .collect();
    if hroles != ["system", "user", "assistant"] {
        d.mismatch(
            "native-history.json",
            "roles",
            "[system, user, assistant]",
            format!("{hroles:?}"),
            "",
        );
    } else {
        if single_text(&h[1]).as_deref() != Some(contract.as_str()) {
            d.mismatch(
                "native-history.json",
                "[1]",
                "exact contract bytes",
                "differs",
                "",
            );
        }
        if h[2]["stopReason"] != "stop"
            || single_text(&h[2]).as_deref() != Some(ctx.result.as_str())
        {
            d.mismatch(
                "native-history.json",
                "[2]",
                "final synthetic result, stop",
                "differs",
                "the native final result is not the synthetic result",
            );
        }
    }
    Some(())
}

fn state_revisions(art: &Art, d: &mut Defects) -> Option<()> {
    let before = art.states.get("episode-before.json")?;
    let after = art.states.get("episode-after.json");
    let rv = Obj::new("revisions.json", art.json.get("revisions.json")?).ok()?;
    d.take("revisions.json", rv.exact(&["before", "expected", "after"]))?;
    let (b, x, a) = (
        d.take("revisions.json", rv.str("before"))?,
        d.take("revisions.json", rv.str("expected"))?,
        d.take("revisions.json", rv.str("after"))?,
    );
    let ci = Obj::new(
        "consumer-inputs.json",
        art.json.get("consumer-inputs.json")?,
    )
    .ok()?;
    let ci_expected = d.take("consumer-inputs.json", ci.str("expected_revision"))?;
    if b != before.revision().0 {
        d.mismatch(
            "revisions.json",
            "before",
            &before.revision().0,
            b,
            "retained before revision is not the owner-validated before-state's",
        );
    }
    if let Some(after) = after {
        if a != after.revision().0 {
            d.mismatch(
                "revisions.json",
                "after",
                &after.revision().0,
                a,
                "retained after revision is not the owner-validated after-state's",
            );
        }
    }
    if x != ci_expected {
        d.mismatch(
            "revisions.json",
            "expected",
            ci_expected,
            x,
            "expected revision differs from the consumer input",
        );
    }
    if x != before.revision().0 {
        d.mismatch(
            "revisions.json",
            "expected",
            &before.revision().0,
            x,
            "expected revision is not the validated before revision",
        );
    }
    if let Some(after) = after {
        if before.revision() == after.revision() {
            d.mismatch(
                "episode-after.json",
                "revision",
                "advanced",
                "unchanged",
                "",
            );
        }
        if canonical_json(&before.identity().value()) != canonical_json(&after.identity().value()) {
            d.mismatch(
                "episode-after.json",
                "identity",
                "the before-state's episode identity",
                "differs",
                "",
            );
        }
    }
    Some(())
}

fn transition_recompute(art: &Art, ctx: &Ctx, native: &Native, d: &mut Defects) -> Option<()> {
    let questions: Vec<&str> = ctx.questions.iter().map(String::as_str).collect();
    let c = match consume_fixture_result(
        &ctx.candidate,
        &ctx.declaration,
        &ctx.before,
        &ctx.authority,
        ctx.expected.clone(),
        &ctx.transition_id,
        &native.verified.result_text,
        &questions,
        &ctx.admissions,
    ) {
        Ok(c) => c,
        Err(e) => {
            d.owner(
                "consumer inputs",
                "consume_fixture_result",
                format!("{e:?}"),
            );
            return None;
        }
    };
    let recomputed = canonical_json(c.state().value());
    match art.bytes.get("episode-after.json") {
            Some(retained) if recomputed.as_bytes() != retained.as_slice() => d.mismatch("episode-after.json", "state", sha256_hex(recomputed.as_bytes()), sha256_hex(retained), "the retained after-state is not the existing consumer's transition of the validated before-state"),
            Some(_) => {}
            // Approved write failure: the observed state bytes do not exist and are never
            // substituted by the recomputation; the retained revision must still agree with it.
            None => {
                let rv = Obj::new("revisions.json", art.json.get("revisions.json")?).ok()?;
                if d.take("revisions.json", rv.str("after"))? != c.state().revision().0 {
                    d.mismatch("revisions.json", "after", &c.state().revision().0, rv.str("after").unwrap_or(""), "retained after revision is not the consumer's transition revision");
                }
            }
        }
    let projection = consumption_projection(&c);
    if Some(&projection) != art.json.get("consumption.json") {
        d.mismatch(
            "consumption.json",
            "",
            &projection,
            art.json
                .get("consumption.json")
                .cloned()
                .unwrap_or(Value::Null),
            "retained consumption outcome/condition coverage differs from the recomputation",
        );
    }
    Some(())
}

fn need_json<'a>(art: &'a Art, name: &str, d: &mut Defects) -> Option<&'a Value> {
    let v = art.json.get(name);
    if v.is_none() {
        d.missing(name, "required decoded artifact absent");
    }
    v
}

fn need_bytes<'a>(art: &'a Art, name: &str, d: &mut Defects) -> Option<&'a [u8]> {
    let v = art.bytes.get(name).map(Vec::as_slice);
    if v.is_none() {
        d.missing(name, "required artifact absent");
    }
    v
}

/// The capture's collection record: delivery-owned facts plus the capture's projections.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CollectionRecord {
    sent: Vec<SendRecord>,
    stdout: StreamRecord,
    stderr: StreamRecord,
    timing: TimingFacts,
    frame_kinds: Vec<String>,
    frame_defects: Vec<String>,
    incompleteness: Vec<String>,
    consumer_invocations: u64,
}

/// Strictly decoded attempt evidence and the delivery owner's verdict over it.
struct Attempt {
    /// Every truthful reason the attempt was not complete and clean (empty: complete and clean).
    reasons: Vec<String>,
    consumer_invocations: u64,
}

/// Strictly decodes process/collection facts into the delivery-owned types (wrong types, missing
/// or foreign fields refuse), joins them to the retained received/stderr/sent bytes through the
/// delivery owner's `check_attempt_evidence`, and requires the captured projections to equal the
/// owner's reading. Contradictory evidence is malformed and establishes no attempt outcome.
fn attempt_evidence(
    art: &Art,
    binding: Option<&ExperimentBinding>,
    d: &mut Defects,
) -> Option<Attempt> {
    const C: &str = "collection.json";
    const P: &str = "process.json";
    let cv = need_json(art, C, d)?;
    let c: CollectionRecord = d.take(C, typed(cv, C))?;
    let pv = need_json(art, P, d)?;
    let process: ProcessFacts = d.take(P, typed(pv, P))?;
    let raw = need_bytes(art, "native-frames-received.jsonl", d)?;
    let stderr = need_bytes(art, "stderr.txt", d)?;
    let sent = need_bytes(art, "native-frames-sent.jsonl", d)?;
    let facts = AttemptFacts {
        sent: c.sent,
        stdout: c.stdout,
        stderr: c.stderr,
        process,
        timing: c.timing,
    };
    let mut reasons = match check_attempt_evidence(&facts, raw, stderr, sent) {
        Ok(r) => r,
        Err(bad) => {
            for b in bad {
                d.malformed(C, format!("attempt evidence contradiction: {b}"));
            }
            return None;
        }
    };
    let i = interpret_stream(raw);
    if i.permitted_success()
        && binding.is_some_and(|b| {
            i.allocation
                .as_ref()
                .is_some_and(|a| BoundAttempt::bind(b.frozen().clone(), a.clone()).is_err())
        })
    {
        reasons.push("first-frame candidates differ from the interpreted stream".to_owned());
    }
    if c.incompleteness != reasons {
        d.mismatch(C, "incompleteness", format!("{reasons:?}"), format!("{:?}", c.incompleteness), "captured incompleteness differs from the delivery owner's verdict over the retained evidence");
    }
    let i = interpret_stream(raw);
    if c.frame_kinds != i.kinds || c.frame_defects != i.defects {
        d.mismatch(C, "frame_kinds/frame_defects", format!("{:?}/{:?}", i.kinds, i.defects), format!("{:?}/{:?}", c.frame_kinds, c.frame_defects), "recorded interpretation differs from the delivery owner's reading of the retained bytes");
    }
    Some(Attempt {
        reasons,
        consumer_invocations: c.consumer_invocations,
    })
}

/// A complete clean process and collection: the delivery owner's verdict names no reason.
fn process_disposition(art: &Art, d: &mut Defects) -> Option<()> {
    let a = attempt_evidence(art, None, d)?;
    if !a.reasons.is_empty() {
        d.failed(
            "process.json",
            "attempt",
            "complete clean run",
            format!("{:?}", a.reasons),
            "the native attempt was not complete and clean",
        );
    }
    Some(())
}

/// Capture and collection evidence under the approved scenario. Absences are checked against
/// the approved definition and the actual file set, never against the bundle's own say-so.
fn capture_complete(
    artifacts: &[(String, Repr)],
    art: &Art,
    sc: &Scenario,
    ctx: &Ctx,
    d: &mut Defects,
) -> Option<()> {
    const A: &str = "capture.json";
    const C: &str = "collection.json";
    let ov = need_json(art, A, d)?;
    let o = d.take(A, Obj::new(A, ov))?;
    d.take(
        A,
        o.exact(&[
            "stage",
            "outcome",
            "error",
            "absent",
            "consumer_invocations",
        ]),
    )?;
    if d.take(A, o.str("stage"))? != sc.stage {
        d.failed(A, "stage", &sc.stage, o.str("stage").unwrap_or(""), "");
    }
    let error = if sc.consumed() {
        d.take(A, o.null("error"))?;
        None
    } else {
        Some(d.take(A, o.str("error"))?)
    };
    // Absent names must equal the artifacts actually missing, less the approved write failure.
    let mut claimed: BTreeSet<String> = BTreeSet::new();
    for (n, e) in d.take(A, o.arr("absent"))?.iter().enumerate() {
        let eo = d.take(A, Obj::new(format!("absent[{n}]"), e))?;
        d.take(A, eo.exact(&["name", "reason"]))?;
        d.take(A, eo.str("reason"))?;
        if !claimed.insert(d.take(A, eo.str("name"))?.to_owned()) {
            d.malformed(A, "duplicate absence entry");
        }
    }
    let missing: BTreeSet<String> = artifacts
        .iter()
        .map(|(n, _)| n.clone())
        .filter(|n| !art.bytes.contains_key(n) && Some(n) != sc.write_failure.as_ref())
        .collect();
    if claimed != missing {
        d.mismatch(A, "absent", format!("{missing:?}"), format!("{claimed:?}"), "capture.json's absences differ from the artifacts actually missing outside the approved write failure");
    }
    let inv = d.take(A, o.u64("consumer_invocations"))?;
    if inv != sc.invocations() {
        d.failed(
            A,
            "consumer_invocations",
            sc.invocations(),
            inv,
            "the runtime consumer invocation count does not fit the approved scenario",
        );
    }
    let projection: OutcomeProjection =
        d.take(A, typed(o.get("outcome").ok()?, "capture.outcome"))?;
    let expected_stage: DeliveryStage = d.take(
        A,
        serde_json::from_value(json!(sc.stage)).map_err(|e| e.to_string()),
    )?;
    if projection.stage != expected_stage
        || projection.native_started != (expected_stage != DeliveryStage::NativeError)
    {
        d.mismatch(
            A,
            "outcome",
            format!("{expected_stage:?}"),
            format!("{projection:?}"),
            "stage must agree with the owning result projection",
        );
    }
    match (&projection.native_error, expected_stage) {
        (Some(e), DeliveryStage::NativeError) if Some(e.error_text().as_str()) == error => {}
        (None, stage) if stage != DeliveryStage::NativeError => {}
        _ => d.malformed(A, "native failure attribution contradicts the stage/error"),
    }
    if expected_stage == DeliveryStage::NativeError {
        // The bundle shape was validated before this projection check: no native artifacts can
        // exist in a setup/spawn outcome. This completes the no-attempt proposition only.
        return Some(());
    }
    let at = attempt_evidence(art, Some(&ctx.binding), d)?;
    if at.consumer_invocations != inv {
        d.mismatch(
            C,
            "consumer_invocations",
            inv,
            at.consumer_invocations,
            "collection and capture disagree about consumption",
        );
    }
    if sc.stage != "delivery_refused" && !at.reasons.is_empty() {
        d.failed(
            C,
            "attempt",
            "complete clean run",
            format!("{:?}", at.reasons),
            "the native attempt was not complete",
        );
    }
    Some(())
}

/// A consumer refusal must be the existing owner's refusal of exactly the retained inputs and
/// the verified native result; a claimed refusal the owner does not produce refuses.
fn consumer_refusal(art: &Art, ctx: &Ctx, native: &Native, d: &mut Defects) -> Option<()> {
    let questions: Vec<&str> = ctx.questions.iter().map(String::as_str).collect();
    let claimed = need_json(art, "capture.json", d)?["error"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    match consume_fixture_result(
        &ctx.candidate,
        &ctx.declaration,
        &ctx.before,
        &ctx.authority,
        ctx.expected.clone(),
        &ctx.transition_id,
        &native.verified.result_text,
        &questions,
        &ctx.admissions,
    ) {
        Ok(_) => d.failed(
            "capture.json",
            "stage",
            "an owner refusal",
            "the existing consumer accepts the retained inputs",
            "claimed consumer refusal the owner does not produce",
        ),
        Err(e) => {
            let want = format!("consumer refused: {e}");
            if claimed != want {
                d.mismatch(
                    "capture.json",
                    "error",
                    want,
                    claimed,
                    "the recorded refusal is not the owner's refusal of the retained inputs",
                );
            }
        }
    }
    Some(())
}

/// A delivery refusal must be justified by evidence, must not have invoked the consumer, and
/// must keep whatever observation exists consistent with the received bytes.
fn refusal_evidence(art: &Art, ctx: &Ctx, d: &mut Defects) -> Option<()> {
    const R: &str = "native-frames-received.jsonl";
    let i = interpret_stream(need_bytes(art, R, d)?);
    for name in [
        "observation.json",
        "request-context.json",
        "native-history.json",
    ] {
        let present = art.bytes.contains_key(name);
        let expected = if name == "request-context.json" {
            i.observation
                .as_ref()
                .and_then(|o| o.context.as_ref())
                .is_some()
        } else {
            i.observation.is_some()
        };
        if present != expected {
            d.mismatch(name, "presence", expected, present, "retained observation artifacts must exist exactly when the stream carried an observation");
        }
    }
    let at = attempt_evidence(art, Some(&ctx.binding), d)?;
    let mut justified = !at.reasons.is_empty() || !i.permitted_success();
    let mut expected_error = if !at.reasons.is_empty() {
        Some(format!(
            "delivery refused: {}",
            work_engine_review_delivery::DeliveryRefusal::Incomplete(at.reasons.join("; "))
        ))
    } else {
        None
    };
    if let Some(obs) = &i.observation {
        let retained: NativeObservation = d.take(
            "observation.json",
            typed(art.json.get("observation.json")?, "observation.json"),
        )?;
        if &retained != obs {
            d.mismatch(
                "observation.json",
                "",
                "the received observation frame",
                "differs",
                "retained observation is not the raw protocol observation",
            );
        }
        if obs.context.as_ref() != art.json.get("request-context.json") {
            d.mismatch(
                "request-context.json",
                "",
                "the raw observation's context",
                "differs",
                "",
            );
        }
        if art.json.get("native-history.json") != Some(&Value::Array(obs.history.clone())) {
            d.mismatch(
                "native-history.json",
                "",
                "the raw observation's history",
                "differs",
                "",
            );
        }
        if let Some(a) = &i.allocation {
            match BoundAttempt::bind(ctx.binding.frozen().clone(), a.clone()) {
                Ok(bound) => {
                    if let Err(e) = bound.verify(obs, &ctx.result) {
                        justified = true;
                        if expected_error.is_none() {
                            expected_error = Some(format!("delivery refused: {e}"));
                        }
                    }
                }
                Err(e) => {
                    justified = true;
                    if expected_error.is_none() {
                        expected_error = Some(format!("delivery refused: {e}"));
                    }
                }
            }
        }
    }
    if let Some(want) = expected_error {
        let got = need_json(art, "capture.json", d)?["error"].clone();
        if got != json!(want) {
            d.mismatch(
                "capture.json",
                "error",
                want,
                got,
                "delivery owner refusal must agree with retained attempt/correspondence",
            );
        }
    }
    if !justified {
        d.mismatch(
            "capture.json",
            "stage",
            "a justified delivery refusal",
            "a complete, verifying delivery",
            "the bundle claims a refusal its own evidence does not support",
        );
    }
    Some(())
}

/// Complete semantic evaluation of a failure-control bundle. Surviving artifacts get their
/// owning semantics; only approved absences are admissible; recomputed state is consistency
/// evidence, never a substitute for missing observed bytes.
fn control_semantics(
    spec: &ValidatedSpec,
    c: &ControlSpec,
    files: &BTreeMap<String, Vec<u8>>,
    run: &RunVars,
    manifest: Option<&SourceManifest>,
    d: &mut Defects,
) -> Option<()> {
    let sc = Scenario {
        stage: c.stage.clone(),
        write_failure: c.write_failure.clone(),
    };
    let mut art = Art::default();
    for (name, bytes) in files {
        if let Some((_, repr)) = spec.artifacts.iter().find(|(n, _)| n == name) {
            if let Some(x) = decode_one(name, repr, bytes.clone(), d) {
                art.put(name, x);
            }
        } else {
            // Fixed support roles: neither changes the stage nor substitutes semantic bytes.
            match name.as_str() {
                "failure.json" if matches!(c.outcome.as_str(), "failed" | "incomplete") => {
                    d.take(name, strict::parse(bytes))?;
                }
                "native-error-frame.txt" if c.stage == "delivery_refused" => {
                    if d.take(name, utf8(bytes, name))?.is_empty() {
                        d.malformed(name, "empty delivery diagnostic");
                    }
                }
                _ => d.malformed(name, "artifact has no applicable approved role"),
            }
        }
    }
    failure_support(c, files, run, d)?;
    let mut scenario_spec = spec.clone();
    if let Some(source) = &c.authority_source {
        // This changes only the input whose exact source bytes are approved for this control.
        // Binding reconstruction and the existing consumer still appraise the actual bytes.
        scenario_spec
            .fixture_sources
            .insert("authority.json".into(), source.clone());
    }
    appraise_bundle(&scenario_spec, &art, &sc, run, manifest?, d)
}

/// Appraise the later failure independently of the underlying delivery stage.
fn failure_support(
    c: &ControlSpec,
    files: &BTreeMap<String, Vec<u8>>,
    run: &RunVars,
    d: &mut Defects,
) -> Option<()> {
    let a = format!("controls/{}", c.name);
    let prefix = format!("{}-", c.scratch);
    let hits: Vec<PathBuf> = d
        .take(
            &a,
            fs::read_dir(PathBuf::from(&run.run_dir).join("scratch")).map_err(|e| e.to_string()),
        )?
        .filter_map(Result::ok)
        .filter(|e| {
            let name = e.file_name();
            name.to_str()
                .and_then(|name| name.strip_prefix(&prefix))
                .is_some_and(|suffix| {
                    suffix
                        .parse::<u32>()
                        .is_ok_and(|pid| pid > 0 && pid.to_string() == suffix)
                })
        })
        .map(|e| e.path())
        .collect();
    if hits.len() != 1
        || !fs::symlink_metadata(&hits[0])
            .map(|m| m.file_type().is_dir())
            .unwrap_or(false)
    {
        d.mismatch(
            &a,
            "scratch",
            "one retained scratch directory",
            hits.len(),
            "scratch was not retained",
        );
        return None;
    }
    if !matches!(c.outcome.as_str(), "failed" | "incomplete") {
        return Some(());
    }
    let f = d.take(
        &a,
        files
            .get("failure.json")
            .ok_or_else(|| "failure.json absent".to_owned())
            .and_then(|b| strict::parse(b)),
    )?;
    let fo = d.take(&a, Obj::new("failure.json", &f))?;
    d.take(
        &a,
        fo.exact(&[
            "stage",
            "message",
            "scratch",
            "scratch_listing_before_cleanup",
            "scratch_retained",
        ]),
    )?;
    let expected_stage = if c.outcome == "failed" {
        "assertion_after_capture"
    } else {
        "evidence_write_after_capture"
    };
    if d.take(&a, fo.str("stage"))? != expected_stage {
        d.malformed(&a, "failure stage contradicts approved later failure");
    }
    if d.take(&a, fo.str("message"))?.trim().is_empty() {
        d.malformed(&a, "failure diagnostic must be nonempty");
    }
    if !d.take(&a, fo.bool("scratch_retained"))?
        || Path::new(d.take(&a, fo.str("scratch"))?) != hits[0]
    {
        d.malformed(
            &a,
            "failure scratch identity disagrees with retained directory",
        );
    }
    let mut declared = BTreeMap::new();
    for item in d.take(&a, fo.arr("scratch_listing_before_cleanup"))? {
        let o = d.take(&a, Obj::new("scratch listing", item))?;
        d.take(&a, o.exact(&["path", "length"]))?;
        let name = d.take(&a, o.str("path"))?;
        if name.is_empty()
            || Path::new(name)
                .components()
                .any(|x| !matches!(x, std::path::Component::Normal(_)))
            || declared
                .insert(name.to_owned(), d.take(&a, o.u64("length"))?)
                .is_some()
        {
            d.malformed(
                &a,
                "scratch listing paths must be unique relative file paths",
            );
        }
    }
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, u64>) -> Result<(), String> {
        for e in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let p = e.map_err(|e| e.to_string())?.path();
            let m = fs::symlink_metadata(&p).map_err(|e| e.to_string())?;
            if m.file_type().is_dir() {
                walk(root, &p, out)?;
            } else if m.file_type().is_file() {
                out.insert(
                    p.strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .into_owned(),
                    m.len(),
                );
            } else {
                return Err(format!(
                    "unsupported retained scratch entry {}",
                    p.display()
                ));
            }
        }
        Ok(())
    }
    let mut actual = BTreeMap::new();
    d.take(&a, walk(&hits[0], &hits[0], &mut actual))?;
    // Setup refusal may have an empty retained scratch; later-stage failures must retain facts.
    if declared != actual || (c.stage != "native_error" && actual.is_empty()) {
        d.mismatch(
            &a,
            "scratch_listing_before_cleanup",
            format!("{actual:?}"),
            format!("{declared:?}"),
            "listing disagrees with physically retained evidence",
        );
    }
    Some(())
}

/// One complete scenario appraisal. Shape validation precedes any stage-specific success:
/// presence cannot choose an easier evaluator, and support roles cannot waive semantic joins.
fn appraise_bundle(
    spec: &ValidatedSpec,
    art: &Art,
    sc: &Scenario,
    run: &RunVars,
    manifest: &SourceManifest,
    d: &mut Defects,
) -> Option<()> {
    let stage: DeliveryStage = d.take(
        "scenario",
        serde_json::from_value(json!(sc.stage)).map_err(|e| e.to_string()),
    )?;
    let native = stage != DeliveryStage::NativeError;
    let consumed = stage == DeliveryStage::DeliveredAndConsumed;
    let interp = if native {
        Some(interpret_stream(need_bytes(
            art,
            "native-frames-received.jsonl",
            d,
        )?))
    } else {
        None
    };
    let observation = interp.as_ref().and_then(|i| i.observation.as_ref());
    let native_files = [
        "native-frames-received.jsonl",
        "native-frames-sent.jsonl",
        "stderr.txt",
        "process.json",
        "collection.json",
    ];
    let consumption_files = ["episode-after.json", "consumption.json", "revisions.json"];
    for (name, _) in &spec.artifacts {
        let required = if native_files.contains(&name.as_str()) {
            native
        } else if consumption_files.contains(&name.as_str()) {
            consumed
        } else if name == "observation.json" || name == "native-history.json" {
            observation.is_some()
        } else if name == "request-context.json" {
            observation.and_then(|o| o.context.as_ref()).is_some()
        } else {
            true
        };
        let present = art.bytes.contains_key(name);
        let write_failed = sc.write_failure.as_ref() == Some(name);
        if present != (required && !write_failed) {
            d.mismatch(
                name,
                "presence",
                required && !write_failed,
                present,
                "artifact presence contradicts the approved stage and actual owner observation",
            );
        }
        if write_failed && !required {
            d.malformed(
                name,
                "write failure cannot justify an artifact excluded by this stage",
            );
        }
    }
    // Fail closed before stage dispatch. Shape is a distinct predicate, not an absence permit.
    if !d.is_empty() {
        return None;
    }
    let ctx = binding_recompute(spec, art, manifest, d)?;
    capture_complete(&spec.artifacts, art, sc, &ctx, d)?;
    identity_projection(art, &ctx, interp.as_ref(), stage, run, d)?;
    match stage {
        DeliveryStage::NativeError => {}
        DeliveryStage::DeliveryRefused => {
            refusal_evidence(art, &ctx, d)?;
        }
        DeliveryStage::ConsumerRefused | DeliveryStage::DeliveredAndConsumed => {
            let native = native_protocol(spec, art, &ctx, run, Completeness::Complete, d)?;
            context_history(art, &ctx, d)?;
            process_disposition(art, d)?;
            if consumed {
                state_revisions(art, d)?;
                transition_recompute(art, &ctx, &native, d)?;
            } else {
                consumer_refusal(art, &ctx, &native, d)?;
            }
        }
    }
    if d.is_empty() {
        Some(())
    } else {
        None
    }
}

/// All stages retain the same invocation/binding identities; bound allocation and successful
/// verification add their owner fields. A refusal cannot smuggle successful identity evidence.
fn identity_projection(
    art: &Art,
    ctx: &Ctx,
    interp: Option<&work_engine_review_delivery::StreamInterpretation>,
    stage: DeliveryStage,
    run: &RunVars,
    d: &mut Defects,
) -> Option<()> {
    let bound = interp
        .filter(|i| i.kinds.first().map(String::as_str) == Some("allocated"))
        .and_then(|i| i.allocation.as_ref())
        .and_then(|a| BoundAttempt::bind(ctx.binding.frozen().clone(), a.clone()).ok());
    let verified = if matches!(
        stage,
        DeliveryStage::ConsumerRefused | DeliveryStage::DeliveredAndConsumed
    ) {
        Some(
            bound
                .as_ref()?
                .verify(interp?.observation.as_ref()?, &ctx.result)
                .ok()?,
        )
    } else {
        None
    };
    let want = json!({
        "invocation_id":run.invocation_id,"source_manifest_sha256":run.manifest_sha256,"map_sha256":run.map_sha256,
        "attempt_id":ctx.binding.frozen().transport().attempt_id,"generation":ctx.binding.frozen().transport().generation,
        "contract_sha256":ctx.binding.frozen().contract_sha256(),
        "session_id":bound.as_ref().map(|b| b.session_id()),
        "sdk_version":bound.as_ref().map(|b| &b.allocation().sdk_version),
        "node_version":bound.as_ref().map(|b| &b.allocation().node_version),
        "effective_system_context_sha256":verified.as_ref().map(|v| &v.effective_system_context_sha256),
        "verified_result_text_sha256":verified.as_ref().map(|v| sha256_hex(v.result_text.as_bytes())),
    });
    let actual = need_json(art, "identities.json", d)?;
    if *actual != want {
        d.mismatch(
            "identities.json",
            "",
            want,
            actual,
            "captured identities differ from the actual stage/owner binding",
        );
    }
    Some(())
}

/// Complete newline-terminated sent frames: each a strict JSON object with a string `type`.
fn sent_frames(bytes: &[u8]) -> Result<Vec<Value>, String> {
    if bytes.is_empty() {
        return Ok(vec![]);
    }
    let body = bytes
        .strip_suffix(b"\n")
        .ok_or("the last sent frame is not newline-terminated")?;
    body.split(|b| *b == b'\n')
        .enumerate()
        .map(|(n, line)| {
            let v = strict::parse(line).map_err(|e| format!("frame {n}: {e}"))?;
            if !v.get("type").is_some_and(Value::is_string) {
                return Err(format!("frame {n}: not an object with a string `type`"));
            }
            Ok(v)
        })
        .collect()
}

/// The single text part of a message: `content` is a string or exactly one `{type: text, text}`.
fn single_text(m: &Value) -> Option<String> {
    match m.get("content")? {
        Value::String(s) => Some(s.clone()),
        Value::Array(parts) if parts.len() == 1 => {
            let p = parts[0].as_object()?;
            (p.len() == 2 && p.get("type")? == "text")
                .then(|| p.get("text")?.as_str().map(str::to_owned))?
        }
        _ => None,
    }
}

fn binding_recompute(
    spec: &ValidatedSpec,
    art: &Art,
    m: &SourceManifest,
    d: &mut Defects,
) -> Option<Ctx> {
    for (name, path) in &spec.fixture_sources {
        let got = sha256_hex(art.bytes.get(name)?);
        match m.entries.get(path) {
            Some(want) if *want == got => {}
            Some(want) => d.mismatch(
                name,
                "sha256",
                want,
                &got,
                format!("retained input is not the frozen fixture {path} in the source closure"),
            ),
            None => d.missing(
                name,
                format!("fixture source {path} is not in the source manifest"),
            ),
        }
    }
    const CI: &str = "consumer-inputs.json";
    let ci = d.take(CI, Obj::new(CI, art.json.get(CI)?))?;
    d.take(
        CI,
        ci.exact(&[
            "profile",
            "request",
            "transition_id",
            "unresolved_questions",
            "expected_revision",
            "transport",
            "authority_is_frozen_fixture",
        ]),
    )?;
    if d.take(CI, ci.str("profile"))? != Profile::FixedSliceReviewV1.as_str() {
        d.mismatch(
            CI,
            "profile",
            Profile::FixedSliceReviewV1.as_str(),
            ci.str("profile").unwrap_or(""),
            "",
        );
        return None;
    }
    let actual_frozen = m
        .entries
        .get(&spec.frozen_authority_source)
        .is_some_and(|h| *h == sha256_hex(&art.bytes["authority.json"]));
    if d.take(CI, ci.bool("authority_is_frozen_fixture"))? != actual_frozen {
        d.mismatch(
            CI,
            "authority_is_frozen_fixture",
            actual_frozen,
            &art.json[CI]["authority_is_frozen_fixture"],
            "must describe actual source correspondence",
        );
    }
    let requirements = match OrganizationalRequirements::from_json_bytes(
        art.bytes.get("candidate-requirements.json")?,
    ) {
        Ok(r) => r,
        Err(e) => {
            d.owner(
                "candidate-requirements.json",
                "organization compiler",
                format!("{e:?}"),
            );
            return None;
        }
    };
    let mut derived = derive(&requirements, Profile::FixedSliceReviewV1);
    if !derived.unresolved.is_empty() || derived.candidates.len() != 1 {
        d.owner(
            "candidate-requirements.json",
            "organization derive",
            format!(
                "{} candidates, unresolved {:?}",
                derived.candidates.len(),
                derived.unresolved
            ),
        );
        return None;
    }
    let candidate = derived.candidates.remove(0);
    let ro = d.take(CI, ci.obj("request"))?;
    d.take(CI, ro.exact(&["obligation_id", "subject", "realization"]))?;
    let subject: Subject = d.take(CI, typed(ro.get("subject").ok()?, "request.subject"))?;
    let realization: Option<RealizationClaim> = match ro.get("realization").ok()? {
        Value::Null => None,
        v => Some(d.take(CI, typed(v, "request.realization"))?),
    };
    let request = PreparationRequest {
        obligation_id: d.take(CI, ro.str("obligation_id"))?.to_owned(),
        subject,
        realization,
    };
    let transition_id = d.take(CI, ci.str("transition_id"))?.to_owned();
    let questions = d.take(CI, ci.strings("unresolved_questions"))?;
    let expected_text = d.take(CI, ci.str("expected_revision"))?;
    if !is_hex64(expected_text) {
        d.malformed(CI, "expected_revision is not a revision digest");
        return None;
    }
    let expected = Revision(expected_text.to_owned());
    let transport: Transport = d.take(CI, typed(ci.get("transport").ok()?, "transport"))?;
    let authority_text = d.take(
        "authority.json",
        utf8(art.bytes.get("authority.json")?, "authority.json"),
    )?;
    let authority = match parse_json(&authority_text).and_then(Authority::parse) {
        Ok(a) => a,
        Err(e) => {
            d.owner(
                "authority.json",
                "review-episode-core Authority::parse",
                e.to_string(),
            );
            return None;
        }
    };
    let declaration: FixtureDeclaration = d.take(
        "declaration.json",
        typed(art.json.get("declaration.json")?, "declaration.json"),
    )?;
    let admissions = d.take(
        "admissions.json",
        utf8(art.bytes.get("admissions.json")?, "admissions.json"),
    )?;
    let result = d.take(
        "synthetic-result.json",
        utf8(
            art.bytes.get("synthetic-result.json")?,
            "synthetic-result.json",
        ),
    )?;
    let before = art.states.get("episode-before.json")?.clone();
    let binding = match ExperimentBinding::new(
        &candidate,
        &request,
        &declaration,
        EpisodeInputs {
            state: before.clone(),
            authority: authority.clone(),
            expected_revision: expected.clone(),
            transition_id: transition_id.clone(),
            unresolved_questions: questions.clone(),
            admissions_json: admissions.clone(),
        },
        &result,
        transport,
    ) {
        Ok(b) => b,
        Err(e) => {
            d.owner("consumer inputs", "ExperimentBinding::new", e.to_string());
            return None;
        }
    };
    let prep = serde_json::to_vec(binding.preparation()).ok()?;
    if prep.as_slice() != art.bytes.get("preparation.json")?.as_slice() {
        d.mismatch(
            "preparation.json",
            "",
            sha256_hex(&prep),
            sha256_hex(art.bytes.get("preparation.json")?),
            "retained preparation is not the canonical recomputation",
        );
    }
    if binding.frozen().contract().as_bytes() != art.bytes.get("contract.bytes")?.as_slice() {
        d.mismatch(
            "contract.bytes",
            "",
            binding.frozen().contract_sha256(),
            sha256_hex(art.bytes.get("contract.bytes")?),
            "retained contract is not the frozen canonical contract",
        );
    }
    Some(Ctx {
        binding,
        candidate,
        declaration,
        authority,
        before,
        expected,
        transition_id,
        questions,
        admissions,
        result,
    })
}

fn native_protocol(
    spec: &ValidatedSpec,
    art: &Art,
    ctx: &Ctx,
    run: &RunVars,
    _completeness: Completeness,
    d: &mut Defects,
) -> Option<Native> {
    const R: &str = "native-frames-received.jsonl";
    const S: &str = "native-frames-sent.jsonl";
    // The whole received byte stream, interpreted by the delivery owner: nothing trimmed or
    // skipped. Duplicate, malformed, invalid-UTF-8, trailing or incomplete frames all refuse.
    let raw = need_bytes(art, R, d)?;
    let sb = need_bytes(art, S, d)?;
    let sent = d.take(S, sent_frames(sb))?;
    let interp = interpret_stream(raw);
    if !interp.permitted_success() {
        d.mismatch(
            R,
            "stream",
            "allocated, observation; every frame complete and valid; nothing after",
            format!("frames {:?}, defects {:?}", interp.kinds, interp.defects),
            "the received bytes are not exactly one allocation then one observation",
        );
    }
    let stypes: Vec<&str> = sent
        .iter()
        .map(|f| f["type"].as_str().unwrap_or("?"))
        .collect();
    if stypes != ["start"] {
        d.mismatch(
            S,
            "sequence",
            "[start]",
            format!("{stypes:?}"),
            "sent protocol lines are not exactly one start frame",
        );
    }
    if !d.is_empty() {
        return None;
    }
    let frozen = ctx.binding.frozen();
    let allocation: Allocation = interp.allocation.clone()?;
    let bound = match BoundAttempt::bind(frozen.clone(), allocation.clone()) {
        Ok(b) => b,
        Err(e) => {
            d.owner(R, "BoundAttempt::bind", e.to_string());
            return None;
        }
    };
    let start = d.take(S, Obj::new("start frame", &sent[0]))?;
    d.take(
        S,
        start.exact(&[
            "type",
            "session_id",
            "attempt_id",
            "generation",
            "contract",
            "contract_sha256",
            "result_text",
        ]),
    )?;
    let contract = utf8(art.bytes.get("contract.bytes")?, "contract.bytes").ok()?;
    let checks: [(&str, Value, Value); 6] = [
        (
            "session_id",
            json!(allocation.session_id),
            sent[0]["session_id"].clone(),
        ),
        (
            "attempt_id",
            json!(frozen.transport().attempt_id),
            sent[0]["attempt_id"].clone(),
        ),
        (
            "generation",
            json!(frozen.transport().generation),
            sent[0]["generation"].clone(),
        ),
        ("contract", json!(contract), sent[0]["contract"].clone()),
        (
            "contract_sha256",
            json!(sha256_hex(contract.as_bytes())),
            sent[0]["contract_sha256"].clone(),
        ),
        (
            "result_text",
            json!(ctx.result),
            sent[0]["result_text"].clone(),
        ),
    ];
    for (field, want, got) in checks {
        if want != got {
            let shown = |v: &Value| {
                if field == "contract" || field == "result_text" {
                    format!("sha256 {}", sha256_hex(v.as_str().unwrap_or("").as_bytes()))
                } else {
                    v.to_string()
                }
            };
            d.mismatch(S, field, shown(&want), shown(&got), "the start frame does not carry the bound session/attempt/generation/contract/result");
        }
    }
    let obs: NativeObservation = interp.observation.clone()?;
    let retained_obs: NativeObservation = d.take(
        "observation.json",
        typed(art.json.get("observation.json")?, "observation.json"),
    )?;
    if retained_obs != obs {
        d.mismatch(
            "observation.json",
            "",
            "the received observation frame",
            "differs",
            "retained observation is not the raw protocol observation",
        );
    }
    let verified = match bound.verify(&obs, &ctx.result) {
        Ok(v) => v,
        Err(e) => {
            d.owner(R, "BoundAttempt::verify", e.to_string());
            return None;
        }
    };
    if verified.result_text != ctx.result {
        d.mismatch(
            R,
            "result",
            sha256_hex(ctx.result.as_bytes()),
            sha256_hex(verified.result_text.as_bytes()),
            "",
        );
    }
    const I: &str = "identities.json";
    let id = d.take(I, Obj::new(I, art.json.get(I)?))?;
    d.take(
        I,
        id.exact(&[
            "invocation_id",
            "source_manifest_sha256",
            "map_sha256",
            "attempt_id",
            "generation",
            "contract_sha256",
            "session_id",
            "sdk_version",
            "node_version",
            "effective_system_context_sha256",
            "verified_result_text_sha256",
        ]),
    )?;
    let exp = spec.expected();
    let want: [(&str, Value); 11] = [
        ("invocation_id", json!(run.invocation_id)),
        ("source_manifest_sha256", json!(run.manifest_sha256)),
        ("map_sha256", json!(run.map_sha256)),
        ("attempt_id", json!(verified.attempt_id)),
        ("generation", json!(verified.generation)),
        ("contract_sha256", json!(verified.contract_sha256)),
        ("session_id", json!(verified.session_id)),
        ("sdk_version", exp["sdk_version"].clone()),
        ("node_version", exp["node_version"].clone()),
        (
            "effective_system_context_sha256",
            json!(verified.effective_system_context_sha256),
        ),
        (
            "verified_result_text_sha256",
            json!(sha256_hex(verified.result_text.as_bytes())),
        ),
    ];
    for (k, w) in want {
        if id.get(k).ok() != Some(&w) {
            d.mismatch(I, k, &w, id.get(k).cloned().unwrap_or(Value::Null), "");
        }
    }
    if json!(allocation.sdk_version) != exp["sdk_version"]
        || json!(allocation.node_version) != exp["node_version"]
    {
        d.mismatch(
            R,
            "allocation",
            format!("{}/{}", exp["sdk_version"], exp["node_version"]),
            format!("{}/{}", allocation.sdk_version, allocation.node_version),
            "the SDK allocation is not the selected SDK/Node identity",
        );
    }
    Some(Native { verified })
}
