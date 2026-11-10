//! Typed register records and their Git-free checks.
//!
//! [`decode`] parses the register files ([`crate::parse`]) and decodes each record into one Rust
//! type per record kind, checking closed key sets (R-03, R-18), forbidden global fields (R-04),
//! vocabularies (R-05, R-05b), identifier formats and uniqueness (R-06), value shapes (R-07),
//! dates (R-08, F-3), null studies (F-2), and the record-local parts of R-10 to R-13, R-16 and R-17
//! (formats and required keys). Checks that need other records or Git objects (R-01, R-09, the
//! resolution parts of R-10 to R-15, R-17's revision match) are not made here.
//!
//! Every check of a record runs even after an earlier one failed, so one record can report several
//! violations. Only records whose every field decoded appear in [`Records`]; the identifiers that
//! later reference checks need are gathered in [`Index`] under the conditions Python's validator
//! uses, so that a record with an unrelated defect does not turn every reference to it into a
//! second violation.

use crate::parse::{self, Json};
use crate::rules::{Location, RuleId, Violation};
use std::collections::{BTreeMap, BTreeSet};

pub const QUESTIONS: &str = "questions.json";
pub const EVENTS: &str = "events.jsonl";
pub const JUDGMENTS: &str = "judgments.jsonl";
pub const DECISIONS: &str = "decisions.json";
pub const APPLICABILITY: &str = "applicability.jsonl";
pub const MATERIALIZED: &str = "materialized.jsonl";
/// The register files other than the study files, in the order Python reads them.
pub const REGISTER_FILES: [&str; 6] = [
    QUESTIONS,
    EVENTS,
    JUDGMENTS,
    DECISIONS,
    APPLICABILITY,
    MATERIALIZED,
];
pub const STUDIES_DIR: &str = "studies";

const QUESTION_KEYS: &[&str] = &["id", "text", "area"];
const STUDY_KEYS: &[&str] = &[
    "id",
    "title",
    "kind",
    "questions",
    "lifecycle",
    "stop_reason_class",
    "findings",
    "sources",
    "aliases",
];
const FINDING_KEYS: &[&str] = &[
    "id", "claim", "value", "n", "basis", "scope", "caveats", "sources",
];
const LIFECYCLE_KEYS: &[&str] = &["date", "state", "basis"];
const EVENT_KEYS: &[&str] = &[
    "event_id",
    "kind",
    "date",
    "subject_finding",
    "by_finding",
    "basis",
    "source",
];
const JUDGMENT_KEYS: &[&str] = &["finding", "axis", "value", "basis", "assessor", "date"];
const DECISION_KEYS: &[&str] = &[
    "decision_id",
    "document",
    "anchor",
    "revision",
    "owner",
    "findings",
];
const DECISION_OPTIONAL: &[&str] = &["lines", "historical", "revisions", "evidence_state"];
const REVISION_ENTRY_KEYS: &[&str] = &["revision", "anchor", "superseded_at", "reason"];
const REVISION_ENTRY_OPTIONAL: &[&str] = &["lines", "document"];
const HISTORICAL_KEYS: &[&str] = &["reason", "marked_by", "marked_at"];
const HISTORICAL_OPTIONAL: &[&str] = &["superseded_by"];
const NONE_FOUND_KEYS: &[&str] = &[
    "state",
    "corpus",
    "search_scope",
    "coverage",
    "assessor",
    "date",
];
const CORPUS_KEYS: &[&str] = &["id", "sha256", "pin"];
/// Applicability keys other than `decision_revision` and `authorization`, whose absence is an
/// R-17 violation rather than an R-03 one.
const APPL_KEYS: &[&str] = &[
    "applicability_id",
    "finding",
    "decision_id",
    "relation",
    "rationale",
    "evidence",
    "proposed_by",
    "proposed_at",
];
const APPL_R17_KEYS: &[&str] = &["decision_revision", "authorization"];
const AUTH_KEYS: &[&str] = &["status", "owner"];
const AUTH_OPTIONAL: &[&str] = &["at"];
const MATERIALIZED_KEYS: &[&str] = &["finding", "claim_id", "revision_id", "revision_sha256"];
const MATERIALIZED_OPTIONAL: &[&str] = &["label"];
const PIN_KEYS: &[&str] = &["path", "commit", "blob"];
const PIN_OPTIONAL: &[&str] = &["lines"];
const OFFREPO_KEYS: &[&str] = &["location", "sha256", "custody", "status"];
const OFFREPO_OPTIONAL: &[&str] = &["note"];

/// A closed vocabulary.
pub trait Vocab: Sized + Copy {
    fn parse(s: &str) -> Option<Self>;
    fn names() -> &'static [&'static str];
}

macro_rules! vocab {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $text),+ }
            }
        }
        impl Vocab for $name {
            fn parse(s: &str) -> Option<Self> {
                match s { $($text => Some($name::$variant),)+ _ => None }
            }
            fn names() -> &'static [&'static str] {
                &[$($text),+]
            }
        }
    };
}

vocab!(/// Study kind (R-05b).
StudyKind { Probe = "probe", Pilot = "pilot", ControlledExperiment = "controlled-experiment",
    RetrospectiveAnalysis = "retrospective-analysis", Qualification = "qualification",
    Instrument = "instrument", Baseline = "baseline", PlannedOnly = "planned-only" });
vocab!(/// Lifecycle state (R-05b).
LifecycleState { Planned = "planned", Preregistered = "preregistered", Running = "running",
    Concluded = "concluded", Partial = "partial", Stopped = "stopped", Abandoned = "abandoned",
    Superseded = "superseded" });
vocab!(/// Stop reason class (R-05b).
StopClass { BudgetOrStopRule = "budget_or_stop_rule", GateFailed = "gate_failed",
    DirectionChanged = "direction_changed", Quota = "quota", PremiseFalse = "premise_false",
    ScopeMisread = "scope_misread" });
vocab!(/// Finding basis (R-05b).
FindingBasis { Measured = "measured", Modelled = "modelled", Estimated = "estimated",
    SingleObservation = "single-observation", OwnerReport = "owner-report",
    NotEstablished = "not-established" });
vocab!(/// Event kind (R-05).
EventKind { Correction = "correction", Challenge = "challenge",
    StatusDiscrepancy = "status_discrepancy" });
vocab!(/// Judgment axis (R-05).
Axis { ClaimKind = "claim_kind", EvidenceStrength = "evidence_strength", Standing = "standing",
    Currency = "currency" });
vocab!(/// Values of the `claim_kind` axis (R-05).
ClaimKind { Observation = "observation", Measurement = "measurement", Comparison = "comparison",
    Explanation = "explanation", Recommendation = "recommendation" });
vocab!(/// Values of the `evidence_strength` axis (R-05).
EvidenceStrength { High = "high", Moderate = "moderate", Low = "low", VeryLow = "very-low" });
vocab!(/// Values of the `standing` axis (R-05).
Standing { Standing = "standing", Qualified = "qualified", Corrected = "corrected",
    Withdrawn = "withdrawn" });
vocab!(/// Values of the `currency` axis (R-05).
Currency { Current = "current", Drifting = "drifting", Historical = "historical",
    NotAssessed = "not-assessed" });
vocab!(/// Applicability relation (R-05).
Relation { Supports = "supports", Constrains = "constrains", Challenges = "challenges",
    DoesNotApplyDirectly = "does_not_apply_directly", NotEstablished = "not_established" });
vocab!(/// Authorization status (R-05).
AuthStatus { Pending = "pending", Authorized = "authorized", Declined = "declined" });
vocab!(/// Off-repo source status (R-05).
OffRepoStatus { Verified = "verified", Unverified = "unverified", Unavailable = "unavailable" });
vocab!(/// `evidence_state` state (R-05).
EvidenceStateKind { NoneLinked = "none_linked", NoneFound = "none_found" });

/// A calendar-valid `YYYY-MM-DD` date (R-08, F-3).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(String);

impl Date {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A 1-based inclusive line range, `N` or `N-M` with `1 <= N <= M`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lines {
    pub first: u64,
    pub last: u64,
}

/// A `git:<7-40 hex>` revision. `text` is the register's string; `commit` is its hex part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    pub text: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    pub at: Location,
    pub path: String,
    /// 7-40 lowercase hex; resolution is checked against Git, not here.
    pub commit: String,
    /// 40 lowercase hex.
    pub blob: String,
    pub lines: Option<Lines>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OffRepo {
    pub at: Location,
    pub location: String,
    pub sha256: Option<String>,
    pub status: OffRepoStatus,
    /// The catalogue constrains only its presence as an optional key.
    pub note: Option<Json>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    Pin(Pin),
    OffRepo(OffRepo),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    pub at: Location,
    pub id: String,
    pub text: String,
    pub area: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleEntry {
    pub date: Date,
    pub state: LifecycleState,
    pub basis: Source,
}

/// A finding's `n`: an integer or text (`null` is `None` on [`Finding::n`]).
#[derive(Debug, Clone, PartialEq)]
pub enum SampleSize {
    Integer(serde_json::Number),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub id: String,
    pub claim: String,
    /// Unconstrained by the catalogue.
    pub value: Json,
    pub n: Option<SampleSize>,
    pub basis: FindingBasis,
    pub scope: String,
    pub caveats: Vec<String>,
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Study {
    pub at: Location,
    pub id: String,
    pub title: String,
    pub kind: StudyKind,
    pub questions: Vec<String>,
    pub lifecycle: Vec<LifecycleEntry>,
    pub stop_reason_class: Option<StopClass>,
    pub findings: Vec<Finding>,
    pub sources: Vec<Source>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub at: Location,
    pub event_id: String,
    pub kind: EventKind,
    pub date: Date,
    pub subject_finding: String,
    pub by_finding: Option<String>,
    pub basis: String,
    pub source: Source,
}

/// A judgment's axis together with a value from that axis's vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assessment {
    ClaimKind(ClaimKind),
    EvidenceStrength(EvidenceStrength),
    Standing(Standing),
    Currency(Currency),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Judgment {
    pub at: Location,
    pub finding: String,
    pub assessment: Assessment,
    pub basis: String,
    pub assessor: String,
    pub date: Date,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevisionEntry {
    pub revision: Revision,
    pub anchor: String,
    pub superseded_at: Date,
    pub reason: String,
    pub lines: Option<Lines>,
    pub document: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Historical {
    pub reason: String,
    pub marked_by: String,
    pub marked_at: Date,
    pub superseded_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Corpus {
    pub id: String,
    pub sha256: Option<String>,
    pub pin: Option<Source>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvidenceState {
    NoneLinked,
    NoneFound(Box<NoneFound>),
}

/// The `none_found` evidence state's fields.
#[derive(Debug, Clone, PartialEq)]
pub struct NoneFound {
    pub corpus: Corpus,
    pub search_scope: String,
    pub coverage: String,
    pub assessor: String,
    pub date: Date,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub at: Location,
    pub decision_id: String,
    pub document: String,
    pub anchor: String,
    pub revision: Revision,
    pub owner: String,
    pub findings: Vec<String>,
    pub lines: Option<Lines>,
    pub historical: Option<Historical>,
    pub revisions: Option<Vec<RevisionEntry>>,
    pub evidence_state: Option<EvidenceState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Authorization {
    pub status: AuthStatus,
    pub owner: String,
    /// R-17 requires its presence for a decided status; its value is unconstrained.
    pub at: Option<Json>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Applicability {
    pub at: Location,
    pub applicability_id: String,
    pub finding: String,
    pub decision_id: String,
    pub decision_revision: Revision,
    pub relation: Relation,
    pub rationale: String,
    pub evidence: Vec<Source>,
    pub proposed_by: String,
    pub proposed_at: Date,
    pub authorization: Authorization,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Materialized {
    pub at: Location,
    pub finding: String,
    /// The values other than `finding` are unconstrained by the catalogue.
    pub label: Option<Json>,
    pub claim_id: Json,
    pub revision_id: Json,
    pub revision_sha256: Json,
}

/// Records whose every field decoded.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Records {
    pub questions: Vec<Question>,
    pub studies: Vec<Study>,
    pub events: Vec<Event>,
    pub judgments: Vec<Judgment>,
    pub decisions: Vec<Decision>,
    pub applicability: Vec<Applicability>,
    pub materialized: Vec<Materialized>,
}

/// A decision's revisions as written, for R-17.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecisionRevisions {
    /// The `revision` string, or `None` if it is not a string.
    pub current: Option<String>,
    /// The string `revision` of each object entry of `revisions`.
    pub listed: BTreeSet<String>,
}

/// Identifiers available to reference checks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Index {
    /// Well-formed, first-seen question ids of complete question records.
    pub question_ids: BTreeSet<String>,
    /// Well-formed finding ids of complete findings in complete studies with well-formed ids.
    pub finding_ids: BTreeSet<String>,
    /// Well-formed, first-seen decision ids of complete decision records.
    pub decisions: BTreeMap<String, DecisionRevisions>,
}

/// The result of [`decode`].
#[derive(Debug, Clone, Default)]
pub struct Decoded {
    pub records: Records,
    pub index: Index,
    /// Every well-formed in-repo pin, in decode order, for the Git-backed R-10 checks.
    pub pins: Vec<Pin>,
    pub violations: Vec<Violation>,
}

/// Parse and decode the register files. `files` maps register-relative paths to blob contents;
/// a missing register file is skipped here (R-01 is checked by the caller). Study files are the
/// entries `studies/<name>.json` directly under `studies/`.
pub fn decode<'a>(files: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Decoded {
    let files: BTreeMap<&str, &[u8]> = files.into_iter().collect();
    let mut st = State::default();
    let out = &mut st.out;
    let questions = files
        .get(QUESTIONS)
        .and_then(|b| parse::document(QUESTIONS, b, out));
    let mut studies = Vec::new();
    for (path, bytes) in &files {
        if let Some(name) = study_name(path) {
            studies.push((*path, name, parse::document(path, bytes, out)));
        }
    }
    let mut jsonl = |file: &str| {
        files
            .get(file)
            .map(|b| parse::lines(file, b, out))
            .unwrap_or_default()
    };
    let events = jsonl(EVENTS);
    let judgments = jsonl(JUDGMENTS);
    let applicability = jsonl(APPLICABILITY);
    let materialized = jsonl(MATERIALIZED);
    let decisions = files
        .get(DECISIONS)
        .and_then(|b| parse::document(DECISIONS, b, out));

    if let Some(doc) = &questions {
        st.document_list(QUESTIONS, doc, State::question);
    }
    for (path, name, doc) in &studies {
        if let Some(doc) = doc {
            st.study(path, name, doc);
        }
    }
    if let Some(doc) = &decisions {
        st.document_list(DECISIONS, doc, State::decision);
    }
    for (n, v) in &events {
        st.event(*n, v);
    }
    for (n, v) in &judgments {
        st.judgment(*n, v);
    }
    for (n, v) in &applicability {
        st.applicability(*n, v);
    }
    for (n, v) in &materialized {
        st.materialized(*n, v);
    }
    Decoded {
        records: st.records,
        index: st.index,
        pins: st.pins,
        violations: st.out,
    }
}

/// The study file name for a path directly under `studies/` ending `.json`.
pub fn study_name(path: &str) -> Option<&str> {
    let name = path.strip_prefix("studies/")?;
    (!name.contains('/') && name.ends_with(".json")).then_some(name)
}

// ---------------------------------------------------------------------------------------------
// decoding context and field helpers

fn child(pointer: &str, key: &str) -> String {
    format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"))
}

fn index(pointer: &str, i: usize) -> String {
    format!("{pointer}/{i}")
}

fn describe(v: &Json) -> String {
    match v {
        Json::String(s) if s.chars().count() <= 60 => format!("{s:?}"),
        Json::String(_) => "a long string".to_owned(),
        other => other.kind().to_owned(),
    }
}

fn is_text(s: &str) -> bool {
    !s.trim().is_empty()
}

fn is_hex(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.len()) && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_kebab(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|seg| {
            !seg.is_empty() && seg.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9'))
        })
}

fn is_finding_id(s: &str) -> bool {
    matches!(s.split_once('#'), Some((study, slug)) if is_kebab(study) && is_kebab(slug))
}

fn is_question_id(s: &str) -> bool {
    matches!(s.strip_prefix('Q'), Some(d) if !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

fn is_decision_id(s: &str) -> bool {
    is_kebab(&s.replace('.', "-"))
}

fn is_global_field(key: &str) -> bool {
    let k = key.to_lowercase();
    k.contains("useful") || k.contains("obsolet")
}

fn parse_revision(s: &str) -> Option<Revision> {
    let commit = s.strip_prefix("git:")?;
    is_hex(commit, 7, 40).then(|| Revision {
        text: s.to_owned(),
        commit: commit.to_owned(),
    })
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) => {
            29
        }
        _ => 28,
    }
}

struct Cx<'s> {
    file: &'s str,
    line: Option<u64>,
    out: &'s mut Vec<Violation>,
    pins: &'s mut Vec<Pin>,
}

/// An object whose key set has been checked.
struct Obj<'j> {
    pointer: String,
    members: &'j [(String, Json)],
    /// Every required key is present.
    complete: bool,
}

impl<'j> Obj<'j> {
    fn get(&self, key: &str) -> Option<&'j Json> {
        self.members.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
    fn at(&self, key: &str) -> String {
        child(&self.pointer, key)
    }
}

impl Cx<'_> {
    fn loc(&self, pointer: &str) -> Location {
        Location {
            file: self.file.to_owned(),
            line: self.line,
            pointer: pointer.to_owned(),
        }
    }

    fn bad(&mut self, rule: RuleId, pointer: &str, detail: impl Into<String>) {
        let location = self.loc(pointer);
        self.out.push(Violation {
            rule,
            location,
            detail: detail.into(),
        });
    }

    /// Check `v` is an object with the closed key set `required` + `optional`. Unknown and missing
    /// keys are `keys_rule` (R-03, or R-18 for materialized rows); forbidden global fields R-04;
    /// a non-object R-07.
    fn object<'j>(
        &mut self,
        pointer: &str,
        v: &'j Json,
        keys_rule: RuleId,
        required: &[&str],
        optional: &[&str],
    ) -> Option<Obj<'j>> {
        let Json::Object(members) = v else {
            self.bad(
                RuleId::R07,
                pointer,
                format!("expected an object, found {}", v.kind()),
            );
            return None;
        };
        let mut keys: Vec<&str> = members.iter().map(|(k, _)| k.as_str()).collect();
        keys.sort_unstable();
        for k in keys {
            if is_global_field(k) {
                self.bad(
                    RuleId::R04,
                    &child(pointer, k),
                    format!("key {k:?}: usefulness or obsolescence is never recorded globally"),
                );
            } else if !required.contains(&k) && !optional.contains(&k) {
                self.bad(
                    keys_rule,
                    &child(pointer, k),
                    format!("key {k:?} is not in the closed key set"),
                );
            }
        }
        let mut complete = true;
        let mut required: Vec<&str> = required.to_vec();
        required.sort_unstable();
        for k in required {
            if !members.iter().any(|(m, _)| m == k) {
                self.bad(keys_rule, pointer, format!("required key {k:?} is absent"));
                complete = false;
            }
        }
        Some(Obj {
            pointer: pointer.to_owned(),
            members,
            complete,
        })
    }

    /// Non-empty text at `o[key]`; absent keys were reported by [`Cx::object`].
    fn text(&mut self, o: &Obj, key: &str, rule: RuleId) -> Option<String> {
        let v = o.get(key)?;
        match v {
            Json::String(s) if is_text(s) => Some(s.clone()),
            _ => {
                self.bad(
                    rule,
                    &o.at(key),
                    format!("{key} must be non-empty text, found {}", describe(v)),
                );
                None
            }
        }
    }

    /// Optional non-empty text: `Some(None)` when absent.
    fn opt_text(&mut self, o: &Obj, key: &str, rule: RuleId) -> Option<Option<String>> {
        if !o.has(key) {
            return Some(None);
        }
        self.text(o, key, rule).map(Some)
    }

    /// Any string at `o[key]` (a reference; resolution is checked elsewhere).
    fn string(&mut self, o: &Obj, key: &str) -> Option<String> {
        let v = o.get(key)?;
        match v {
            Json::String(s) => Some(s.clone()),
            _ => {
                self.bad(
                    RuleId::R07,
                    &o.at(key),
                    format!("{key} must be a string, found {}", v.kind()),
                );
                None
            }
        }
    }

    fn date_value(&mut self, pointer: &str, key: &str, v: &Json) -> Option<Date> {
        let Json::String(s) = v else {
            self.bad(
                RuleId::R08,
                pointer,
                format!("{key} {} is not YYYY-MM-DD", describe(v)),
            );
            return None;
        };
        let b = s.as_bytes();
        let shaped = b.len() == 10
            && b.iter().enumerate().all(|(i, c)| {
                if i == 4 || i == 7 {
                    *c == b'-'
                } else {
                    c.is_ascii_digit()
                }
            });
        if !shaped {
            self.bad(
                RuleId::R08,
                pointer,
                format!("{key} {s:?} is not YYYY-MM-DD"),
            );
            return None;
        }
        let num = |r: std::ops::Range<usize>| s[r].parse::<u32>().unwrap_or(0);
        let (y, m, d) = (num(0..4), num(5..7), num(8..10));
        if y < 1 || !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
            self.bad(
                RuleId::F3,
                pointer,
                format!("{key} {s:?} is not a calendar date"),
            );
            return None;
        }
        Some(Date(s.clone()))
    }

    fn date(&mut self, o: &Obj, key: &str) -> Option<Date> {
        let v = o.get(key)?;
        self.date_value(&o.at(key), key, v)
    }

    fn vocab_value<T: Vocab>(
        &mut self,
        pointer: &str,
        key: &str,
        v: &Json,
        rule: RuleId,
    ) -> Option<T> {
        let found = v.as_str().and_then(T::parse);
        if found.is_none() {
            self.bad(
                rule,
                pointer,
                format!("{key} {} is not one of {:?}", describe(v), T::names()),
            );
        }
        found
    }

    fn vocab<T: Vocab>(&mut self, o: &Obj, key: &str, rule: RuleId) -> Option<T> {
        let v = o.get(key)?;
        self.vocab_value(&o.at(key), key, v, rule)
    }

    /// A list at `o[key]`, non-empty when `nonempty`.
    fn list<'j>(&mut self, o: &Obj<'j>, key: &str, nonempty: bool) -> Option<&'j [Json]> {
        match o.get(key)? {
            Json::Array(items) if nonempty && items.is_empty() => {
                self.bad(
                    RuleId::R07,
                    &o.at(key),
                    format!("{key} must be a non-empty list"),
                );
                None
            }
            Json::Array(items) => Some(items),
            v => {
                self.bad(
                    RuleId::R07,
                    &o.at(key),
                    format!("{key} must be a list, found {}", v.kind()),
                );
                None
            }
        }
    }

    /// A list of strings; `text` additionally requires each to be non-empty text.
    fn strings(&mut self, o: &Obj, key: &str, text: bool) -> Option<Vec<String>> {
        let items = self.list(o, key, false)?;
        let pointer = o.at(key);
        let mut out = Vec::new();
        let mut ok = true;
        for (i, item) in items.iter().enumerate() {
            match item {
                Json::String(s) if !text || is_text(s) => out.push(s.clone()),
                _ => {
                    let want = if text { "non-empty text" } else { "a string" };
                    self.bad(
                        RuleId::R07,
                        &index(&pointer, i),
                        format!("{key} item must be {want}, found {}", describe(item)),
                    );
                    ok = false;
                }
            }
        }
        ok.then_some(out)
    }

    fn lines_value(&mut self, pointer: &str, v: &Json, rule: RuleId) -> Option<Lines> {
        let parsed = v.as_str().and_then(|s| {
            let (a, b) = s.split_once('-').unwrap_or((s, s));
            let num = |t: &str| {
                (!t.is_empty() && t.bytes().all(|c| c.is_ascii_digit()))
                    .then(|| t.parse::<u64>().ok())
                    .flatten()
            };
            Some((num(a)?, num(b)?))
        });
        match parsed {
            Some((first, last)) if first >= 1 && last >= first => Some(Lines { first, last }),
            _ => {
                self.bad(
                    rule,
                    pointer,
                    format!("lines {} is not N or N-M ascending from 1", describe(v)),
                );
                None
            }
        }
    }

    /// A source: an off-repo source when it has `location`, otherwise an in-repo pin.
    fn source(&mut self, pointer: &str, v: &Json) -> Option<Source> {
        if !matches!(v, Json::Object(_)) {
            self.bad(
                RuleId::R10,
                pointer,
                format!("source must be an object, found {}", v.kind()),
            );
            return None;
        }
        if v.get("location").is_some() {
            let o = self.object(pointer, v, RuleId::R03, OFFREPO_KEYS, OFFREPO_OPTIONAL)?;
            let location = self.text(&o, "location", RuleId::R11);
            let sha256 = match o.get("sha256") {
                None => None,
                Some(Json::Null) => Some(None),
                Some(Json::String(s)) if is_hex(s, 64, 64) => Some(Some(s.clone())),
                Some(other) => {
                    self.bad(
                        RuleId::R11,
                        &o.at("sha256"),
                        format!(
                            "sha256 {} must be 64 lowercase hex or null",
                            describe(other)
                        ),
                    );
                    None
                }
            };
            let custody = match o.get("custody") {
                None => None,
                Some(Json::String(s)) if s == "off_repo" => Some(()),
                Some(other) => {
                    self.bad(
                        RuleId::R11,
                        &o.at("custody"),
                        format!("custody {} must be \"off_repo\"", describe(other)),
                    );
                    None
                }
            };
            let status = self.vocab::<OffRepoStatus>(&o, "status", RuleId::R05);
            if status == Some(OffRepoStatus::Verified) && sha256 == Some(None) {
                self.bad(RuleId::R11, pointer, "status verified requires a sha256");
                return None;
            }
            custody?;
            return Some(Source::OffRepo(OffRepo {
                at: self.loc(pointer),
                location: location?,
                sha256: sha256?,
                status: status?,
                note: o.get("note").cloned(),
            }));
        }
        let o = self.object(pointer, v, RuleId::R03, PIN_KEYS, PIN_OPTIONAL)?;
        let hex = |cx: &mut Self, key: &str, min: usize, max: usize| {
            let v = o.get(key)?;
            match v {
                Json::String(s) if is_hex(s, min, max) => Some(s.clone()),
                _ => {
                    cx.bad(
                        RuleId::R10,
                        &o.at(key),
                        format!("{key} {} is not {min}-{max} lowercase hex", describe(v)),
                    );
                    None
                }
            }
        };
        let commit = hex(self, "commit", 7, 40);
        let blob = hex(self, "blob", 40, 40);
        let path = self.text(&o, "path", RuleId::R10);
        let lines = match o.get("lines") {
            None => Some(None),
            Some(v) => self.lines_value(&o.at("lines"), v, RuleId::R10).map(Some),
        };
        let pin = Pin {
            at: self.loc(pointer),
            path: path?,
            commit: commit?,
            blob: blob?,
            lines: lines?,
        };
        self.pins.push(pin.clone());
        Some(Source::Pin(pin))
    }

    fn sources(&mut self, o: &Obj, key: &str, nonempty: bool) -> Option<Vec<Source>> {
        let items = self.list(o, key, nonempty)?;
        let pointer = o.at(key);
        let decoded: Vec<Option<Source>> = items
            .iter()
            .enumerate()
            .map(|(i, s)| self.source(&index(&pointer, i), s))
            .collect();
        decoded.into_iter().collect()
    }
}

// ---------------------------------------------------------------------------------------------
// record decoders

#[derive(Default)]
struct State {
    out: Vec<Violation>,
    pins: Vec<Pin>,
    records: Records,
    index: Index,
    study_ids: BTreeSet<String>,
    finding_ids: BTreeSet<String>,
    aliases: BTreeMap<String, String>,
    event_ids: BTreeSet<String>,
    applicability_ids: BTreeSet<String>,
}

impl State {
    fn cx<'s>(
        out: &'s mut Vec<Violation>,
        pins: &'s mut Vec<Pin>,
        file: &'s str,
        line: Option<u64>,
    ) -> Cx<'s> {
        Cx {
            file,
            line,
            out,
            pins,
        }
    }

    /// A JSON document that must be a list of records.
    fn document_list(&mut self, file: &str, doc: &Json, each: fn(&mut State, &str, usize, &Json)) {
        let Json::Array(items) = doc else {
            let mut cx = State::cx(&mut self.out, &mut self.pins, file, None);
            cx.bad(
                RuleId::R07,
                "",
                format!("{file} must be a list, found {}", doc.kind()),
            );
            return;
        };
        for (i, item) in items.iter().enumerate() {
            each(self, file, i, item);
        }
    }

    fn question(&mut self, file: &str, i: usize, v: &Json) {
        let mut cx = State::cx(&mut self.out, &mut self.pins, file, None);
        let pointer = index("", i);
        let Some(o) = cx.object(&pointer, v, RuleId::R03, QUESTION_KEYS, &[]) else {
            return;
        };
        let id = match o.get("id") {
            Some(Json::String(s)) if is_question_id(s) => {
                if !o.complete || self.index.question_ids.insert(s.clone()) {
                    Some(s.clone())
                } else {
                    cx.bad(
                        RuleId::R06,
                        &o.at("id"),
                        format!("duplicate question id {s}"),
                    );
                    None
                }
            }
            Some(other) => {
                cx.bad(
                    RuleId::R06,
                    &o.at("id"),
                    format!("id {} is not Q<number>", describe(other)),
                );
                None
            }
            None => None,
        };
        let text = cx.text(&o, "text", RuleId::R07);
        let area = cx.text(&o, "area", RuleId::R07);
        if let (Some(id), Some(text), Some(area)) = (id, text, area) {
            let at = cx.loc(&pointer);
            self.records.questions.push(Question { at, id, text, area });
        }
    }

    fn study(&mut self, path: &str, name: &str, v: &Json) {
        let mut cx = State::cx(&mut self.out, &mut self.pins, path, None);
        if matches!(v, Json::Null) {
            cx.bad(RuleId::F2, "", "a study must be an object, not null");
            return;
        }
        let Some(o) = cx.object("", v, RuleId::R03, STUDY_KEYS, &[]) else {
            return;
        };
        let id = match o.get("id") {
            Some(Json::String(s)) if is_kebab(s) => {
                let mut ok = true;
                if format!("{s}.json") != name {
                    cx.bad(
                        RuleId::R06,
                        &o.at("id"),
                        format!("study id {s} requires the file name {s}.json"),
                    );
                    ok = false;
                }
                if o.complete && !self.study_ids.insert(s.clone()) {
                    cx.bad(RuleId::R06, &o.at("id"), format!("duplicate study id {s}"));
                    ok = false;
                }
                ok.then(|| s.clone())
            }
            Some(other) => {
                cx.bad(
                    RuleId::R06,
                    &o.at("id"),
                    format!("study id {} is not kebab-case", describe(other)),
                );
                None
            }
            None => None,
        };
        let sid = o
            .get("id")
            .and_then(Json::as_str)
            .filter(|s| is_kebab(s))
            .map(str::to_owned);
        let title = cx.text(&o, "title", RuleId::R07);
        let kind = cx.vocab::<StudyKind>(&o, "kind", RuleId::R05b);
        let stop_reason_class = match o.get("stop_reason_class") {
            None => None,
            Some(Json::Null) => Some(None),
            Some(v) => cx
                .vocab_value::<StopClass>(
                    &o.at("stop_reason_class"),
                    "stop_reason_class",
                    v,
                    RuleId::R05b,
                )
                .map(Some),
        };
        let questions = cx.strings(&o, "questions", false);
        let lifecycle = cx.list(&o, "lifecycle", true).and_then(|items| {
            let lp = o.at("lifecycle");
            let entries: Vec<Option<LifecycleEntry>> = items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let p = index(&lp, i);
                    let lo = cx.object(&p, item, RuleId::R03, LIFECYCLE_KEYS, &[])?;
                    let date = cx.date(&lo, "date");
                    let state = cx.vocab::<LifecycleState>(&lo, "state", RuleId::R05b);
                    let basis = lo.get("basis").and_then(|b| cx.source(&lo.at("basis"), b));
                    Some(LifecycleEntry {
                        date: date?,
                        state: state?,
                        basis: basis?,
                    })
                })
                .collect();
            entries.into_iter().collect::<Option<Vec<_>>>()
        });
        let sources = cx.sources(&o, "sources", false);
        let aliases = cx.strings(&o, "aliases", true);
        if let (Some(list), Some(sid)) = (&aliases, &sid) {
            for (i, a) in list.iter().enumerate() {
                if let Some(other) = self.aliases.get(a) {
                    cx.bad(
                        RuleId::R06,
                        &index(&o.at("aliases"), i),
                        format!("alias {a} also on study {other}"),
                    );
                }
                self.aliases.insert(a.clone(), sid.clone());
            }
        }
        let findings = cx.list(&o, "findings", true).and_then(|items| {
            let fp = o.at("findings");
            let decoded: Vec<Option<Finding>> = items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    finding(
                        &mut cx,
                        &index(&fp, i),
                        item,
                        sid.as_deref(),
                        o.complete,
                        &mut self.finding_ids,
                        &mut self.index.finding_ids,
                    )
                })
                .collect();
            decoded.into_iter().collect::<Option<Vec<_>>>()
        });
        let study = (|| {
            Some(Study {
                at: cx.loc(""),
                id: id?,
                title: title?,
                kind: kind?,
                questions: questions?,
                lifecycle: lifecycle?,
                stop_reason_class: stop_reason_class?,
                findings: findings?,
                sources: sources?,
                aliases: aliases?,
            })
        })();
        if let Some(study) = study {
            self.records.studies.push(study);
        }
    }

    fn decision(&mut self, file: &str, i: usize, v: &Json) {
        let mut cx = State::cx(&mut self.out, &mut self.pins, file, None);
        let pointer = index("", i);
        let Some(o) = cx.object(&pointer, v, RuleId::R03, DECISION_KEYS, DECISION_OPTIONAL) else {
            return;
        };
        let decision_id = match o.get("decision_id") {
            Some(Json::String(s)) if is_decision_id(s) => {
                if o.complete && self.index.decisions.contains_key(s) {
                    cx.bad(
                        RuleId::R06,
                        &o.at("decision_id"),
                        format!("duplicate decision id {s}"),
                    );
                    None
                } else {
                    if o.complete {
                        let listed = match o.get("revisions") {
                            Some(Json::Array(items)) => items
                                .iter()
                                .filter_map(|r| {
                                    r.get("revision").and_then(Json::as_str).map(str::to_owned)
                                })
                                .collect(),
                            _ => BTreeSet::new(),
                        };
                        let current = o.get("revision").and_then(Json::as_str).map(str::to_owned);
                        self.index
                            .decisions
                            .insert(s.clone(), DecisionRevisions { current, listed });
                    }
                    Some(s.clone())
                }
            }
            Some(other) => {
                cx.bad(
                    RuleId::R06,
                    &o.at("decision_id"),
                    format!(
                        "decision_id {} is not kebab-case with optional dots",
                        describe(other)
                    ),
                );
                None
            }
            None => None,
        };
        let document = cx.text(&o, "document", RuleId::R07);
        let anchor = cx.text(&o, "anchor", RuleId::R07);
        let owner = cx.text(&o, "owner", RuleId::R07);
        let revision = o.get("revision").and_then(|v| {
            let r = v.as_str().and_then(parse_revision);
            if r.is_none() {
                cx.bad(
                    RuleId::R12,
                    &o.at("revision"),
                    format!("revision {} must be git:<7-40 lowercase hex>", describe(v)),
                );
            }
            r
        });
        let lines = match o.get("lines") {
            None => Some(None),
            Some(_) if document.is_none() && o.has("document") => None,
            Some(_) if document.is_none() => {
                cx.bad(RuleId::R12, &o.at("lines"), "lines needs a document");
                None
            }
            Some(v) => cx.lines_value(&o.at("lines"), v, RuleId::R12).map(Some),
        };
        let findings_empty = matches!(o.get("findings"), Some(Json::Array(a)) if a.is_empty());
        let findings = match o.get("findings") {
            Some(Json::Array(a)) if a.is_empty() => {
                if !o.has("evidence_state") {
                    cx.bad(
                        RuleId::R16,
                        &o.at("findings"),
                        "findings is [] and evidence_state is absent",
                    );
                    None
                } else {
                    Some(Vec::new())
                }
            }
            Some(_) => cx.strings(&o, "findings", false),
            None => None,
        };
        let evidence_state = match o.get("evidence_state") {
            None => Some(None),
            Some(v) => {
                if !findings_empty {
                    cx.bad(
                        RuleId::R16,
                        &o.at("evidence_state"),
                        "evidence_state is allowed only when findings is []",
                    );
                }
                let decoded = evidence_state(&mut cx, &o.at("evidence_state"), v);
                if findings_empty {
                    decoded.map(Some)
                } else {
                    None
                }
            }
        };
        let doc_ok = document.is_some();
        let revisions = match o.get("revisions") {
            None => Some(None),
            Some(_) => cx.list(&o, "revisions", true).and_then(|items| {
                let rp = o.at("revisions");
                let decoded: Vec<Option<RevisionEntry>> = items
                    .iter()
                    .enumerate()
                    .map(|(j, item)| revision_entry(&mut cx, &index(&rp, j), item, doc_ok))
                    .collect();
                decoded.into_iter().collect::<Option<Vec<_>>>().map(Some)
            }),
        };
        let historical = match o.get("historical") {
            None => Some(None),
            Some(v) => {
                let hp = o.at("historical");
                cx.object(&hp, v, RuleId::R03, HISTORICAL_KEYS, HISTORICAL_OPTIONAL)
                    .and_then(|h| {
                        let reason = cx.text(&h, "reason", RuleId::R07);
                        let marked_by = cx.text(&h, "marked_by", RuleId::R07);
                        let marked_at = cx.date(&h, "marked_at");
                        let superseded_by = cx.opt_text(&h, "superseded_by", RuleId::R07);
                        Some(Historical {
                            reason: reason?,
                            marked_by: marked_by?,
                            marked_at: marked_at?,
                            superseded_by: superseded_by?,
                        })
                    })
                    .map(Some)
            }
        };
        let decision = (|| {
            Some(Decision {
                at: cx.loc(&pointer),
                decision_id: decision_id?,
                document: document?,
                anchor: anchor?,
                revision: revision?,
                owner: owner?,
                findings: findings?,
                lines: lines?,
                historical: historical?,
                revisions: revisions?,
                evidence_state: evidence_state?,
            })
        })();
        if let Some(d) = decision {
            self.records.decisions.push(d);
        }
    }

    fn event(&mut self, n: u64, v: &Json) {
        let mut cx = State::cx(&mut self.out, &mut self.pins, EVENTS, Some(n));
        let Some(o) = cx.object("", v, RuleId::R03, EVENT_KEYS, &[]) else {
            return;
        };
        let event_id = unique_kebab(&mut cx, &o, "event_id", &mut self.event_ids);
        let kind = cx.vocab::<EventKind>(&o, "kind", RuleId::R05);
        let date = cx.date(&o, "date");
        let subject_finding = cx.string(&o, "subject_finding");
        let by_finding = match o.get("by_finding") {
            None => None,
            Some(Json::Null) => Some(None),
            Some(_) => cx.string(&o, "by_finding").map(Some),
        };
        let basis = cx.text(&o, "basis", RuleId::R07);
        let source = o.get("source").and_then(|s| cx.source(&o.at("source"), s));
        let event = (|| {
            Some(Event {
                at: cx.loc(""),
                event_id: event_id?,
                kind: kind?,
                date: date?,
                subject_finding: subject_finding?,
                by_finding: by_finding?,
                basis: basis?,
                source: source?,
            })
        })();
        if let Some(e) = event {
            self.records.events.push(e);
        }
    }

    fn judgment(&mut self, n: u64, v: &Json) {
        let mut cx = State::cx(&mut self.out, &mut self.pins, JUDGMENTS, Some(n));
        let Some(o) = cx.object("", v, RuleId::R03, JUDGMENT_KEYS, &[]) else {
            return;
        };
        let finding = cx.string(&o, "finding");
        let axis = cx.vocab::<Axis>(&o, "axis", RuleId::R05);
        let assessment = match (axis, o.get("value")) {
            (Some(axis), Some(v)) => {
                let p = o.at("value");
                let r = RuleId::R05;
                match axis {
                    Axis::ClaimKind => cx.vocab_value(&p, "value", v, r).map(Assessment::ClaimKind),
                    Axis::EvidenceStrength => cx
                        .vocab_value(&p, "value", v, r)
                        .map(Assessment::EvidenceStrength),
                    Axis::Standing => cx.vocab_value(&p, "value", v, r).map(Assessment::Standing),
                    Axis::Currency => cx.vocab_value(&p, "value", v, r).map(Assessment::Currency),
                }
            }
            _ => None,
        };
        let basis = cx.text(&o, "basis", RuleId::R07);
        let assessor = cx.text(&o, "assessor", RuleId::R07);
        let date = cx.date(&o, "date");
        let judgment = (|| {
            Some(Judgment {
                at: cx.loc(""),
                finding: finding?,
                assessment: assessment?,
                basis: basis?,
                assessor: assessor?,
                date: date?,
            })
        })();
        if let Some(j) = judgment {
            self.records.judgments.push(j);
        }
    }

    fn applicability(&mut self, n: u64, v: &Json) {
        let mut cx = State::cx(&mut self.out, &mut self.pins, APPLICABILITY, Some(n));
        let decision_revision = match v.get("decision_revision") {
            None | Some(Json::Null) if matches!(v, Json::Object(_)) => {
                cx.bad(RuleId::R17, "", "decision_revision is absent or null; applicability is per (finding, decision@revision)");
                None
            }
            Some(r) => {
                let parsed = r.as_str().and_then(parse_revision);
                if parsed.is_none() {
                    cx.bad(
                        RuleId::R17,
                        "/decision_revision",
                        format!(
                            "decision_revision {} must be git:<7-40 lowercase hex>",
                            describe(r)
                        ),
                    );
                }
                parsed
            }
            None => None,
        };
        let auth = v.get("authorization");
        let auth_ok = matches!(auth, Some(a @ Json::Object(_)) if a.get("status").is_some());
        if matches!(v, Json::Object(_)) && !auth_ok {
            cx.bad(RuleId::R17, "", "authorization.status is required");
        }
        let mut required: Vec<&str> = APPL_KEYS.to_vec();
        let optional = APPL_R17_KEYS;
        required.sort_unstable();
        let Some(o) = cx.object("", v, RuleId::R03, &required, optional) else {
            return;
        };
        let applicability_id =
            unique_kebab(&mut cx, &o, "applicability_id", &mut self.applicability_ids);
        let finding = cx.string(&o, "finding");
        let decision_id = cx.string(&o, "decision_id");
        let relation = cx.vocab::<Relation>(&o, "relation", RuleId::R05);
        let rationale = cx.text(&o, "rationale", RuleId::R07);
        let proposed_by = cx.text(&o, "proposed_by", RuleId::R07);
        let proposed_at = cx.date(&o, "proposed_at");
        let evidence = cx.sources(&o, "evidence", true);
        let authorization = match auth {
            Some(a) if auth_ok => {
                let ap = o.at("authorization");
                cx.object(&ap, a, RuleId::R03, AUTH_KEYS, AUTH_OPTIONAL)
                    .and_then(|ao| {
                        let status = cx.vocab::<AuthStatus>(&ao, "status", RuleId::R05);
                        let owner = cx.text(&ao, "owner", RuleId::R07);
                        if status.is_some_and(|s| s != AuthStatus::Pending) && !ao.has("at") {
                            cx.bad(RuleId::R17, &ap, "a decided authorization needs 'at'");
                            return None;
                        }
                        Some(Authorization {
                            status: status?,
                            owner: owner?,
                            at: ao.get("at").cloned(),
                        })
                    })
            }
            _ => None,
        };
        let record = (|| {
            Some(Applicability {
                at: cx.loc(""),
                applicability_id: applicability_id?,
                finding: finding?,
                decision_id: decision_id?,
                decision_revision: decision_revision?,
                relation: relation?,
                rationale: rationale?,
                evidence: evidence?,
                proposed_by: proposed_by?,
                proposed_at: proposed_at?,
                authorization: authorization?,
            })
        })();
        if let Some(a) = record {
            self.records.applicability.push(a);
        }
    }

    fn materialized(&mut self, n: u64, v: &Json) {
        let mut cx = State::cx(&mut self.out, &mut self.pins, MATERIALIZED, Some(n));
        let Some(o) = cx.object("", v, RuleId::R18, MATERIALIZED_KEYS, MATERIALIZED_OPTIONAL)
        else {
            return;
        };
        let finding = cx.string(&o, "finding");
        let record = (|| {
            Some(Materialized {
                at: cx.loc(""),
                finding: finding?,
                label: o.get("label").cloned(),
                claim_id: o.get("claim_id")?.clone(),
                revision_id: o.get("revision_id")?.clone(),
                revision_sha256: o.get("revision_sha256")?.clone(),
            })
        })();
        if let Some(m) = record {
            self.records.materialized.push(m);
        }
    }
}

/// A kebab-case id unique within `seen` (R-06). Uniqueness is tracked only for complete records.
fn unique_kebab(cx: &mut Cx, o: &Obj, key: &str, seen: &mut BTreeSet<String>) -> Option<String> {
    match o.get(key)? {
        Json::String(s) if is_kebab(s) => {
            if o.complete && !seen.insert(s.clone()) {
                cx.bad(RuleId::R06, &o.at(key), format!("duplicate {key} {s}"));
                None
            } else {
                Some(s.clone())
            }
        }
        other => {
            cx.bad(
                RuleId::R06,
                &o.at(key),
                format!("{key} {} is not kebab-case", describe(other)),
            );
            None
        }
    }
}

fn finding(
    cx: &mut Cx,
    pointer: &str,
    v: &Json,
    study_id: Option<&str>,
    study_complete: bool,
    seen: &mut BTreeSet<String>,
    indexed: &mut BTreeSet<String>,
) -> Option<Finding> {
    let o = cx.object(pointer, v, RuleId::R03, FINDING_KEYS, &[])?;
    let id = match o.get("id") {
        Some(Json::String(s)) if is_finding_id(s) => {
            let mut ok = true;
            if let Some(sid) = study_id
                && s.split('#').next() != Some(sid)
            {
                cx.bad(
                    RuleId::R06,
                    &o.at("id"),
                    format!("finding id {s} does not start with its study id {sid}#"),
                );
                ok = false;
            }
            if o.complete && study_complete && study_id.is_some() {
                indexed.insert(s.clone());
                if !seen.insert(s.clone()) {
                    cx.bad(
                        RuleId::R06,
                        &o.at("id"),
                        format!("duplicate finding id {s}"),
                    );
                    ok = false;
                }
            }
            ok.then(|| s.clone())
        }
        Some(other) => {
            cx.bad(
                RuleId::R06,
                &o.at("id"),
                format!("finding id {} is not <study-id>#<slug>", describe(other)),
            );
            None
        }
        None => None,
    };
    let basis = cx.vocab::<FindingBasis>(&o, "basis", RuleId::R05b);
    let claim = cx.text(&o, "claim", RuleId::R07);
    let scope = cx.text(&o, "scope", RuleId::R07);
    let n = match o.get("n") {
        None => None,
        Some(Json::Null) => Some(None),
        Some(Json::Number(num)) if num.is_i64() || num.is_u64() => {
            Some(Some(SampleSize::Integer(num.clone())))
        }
        Some(Json::String(s)) => Some(Some(SampleSize::Text(s.clone()))),
        Some(other) => {
            cx.bad(
                RuleId::R07,
                &o.at("n"),
                format!("n must be an integer, text or null, found {}", other.kind()),
            );
            None
        }
    };
    let caveats = cx.strings(&o, "caveats", true);
    let sources = cx.sources(&o, "sources", true);
    Some(Finding {
        id: id?,
        claim: claim?,
        value: o.get("value")?.clone(),
        n: n?,
        basis: basis?,
        scope: scope?,
        caveats: caveats?,
        sources: sources?,
    })
}

fn revision_entry(
    cx: &mut Cx,
    pointer: &str,
    v: &Json,
    decision_document_ok: bool,
) -> Option<RevisionEntry> {
    let o = cx.object(
        pointer,
        v,
        RuleId::R03,
        REVISION_ENTRY_KEYS,
        REVISION_ENTRY_OPTIONAL,
    )?;
    let revision = o.get("revision").and_then(|r| {
        let parsed = r.as_str().and_then(parse_revision);
        if parsed.is_none() {
            cx.bad(
                RuleId::R13,
                &o.at("revision"),
                format!("revision {} must be git:<7-40 lowercase hex>", describe(r)),
            );
        }
        parsed
    });
    let anchor = cx.text(&o, "anchor", RuleId::R07);
    let reason = cx.text(&o, "reason", RuleId::R07);
    let superseded_at = cx.date(&o, "superseded_at");
    let document = cx.opt_text(&o, "document", RuleId::R07);
    let lines = match o.get("lines") {
        None => Some(None),
        Some(_) if o.has("document") && document.is_none() => None,
        Some(_) if !o.has("document") && !decision_document_ok => {
            cx.bad(RuleId::R13, &o.at("lines"), "lines needs a document");
            None
        }
        Some(v) => cx.lines_value(&o.at("lines"), v, RuleId::R13).map(Some),
    };
    Some(RevisionEntry {
        revision: revision?,
        anchor: anchor?,
        superseded_at: superseded_at?,
        reason: reason?,
        lines: lines?,
        document: document?,
    })
}

fn evidence_state(cx: &mut Cx, pointer: &str, v: &Json) -> Option<EvidenceState> {
    let o = cx.object(pointer, v, RuleId::R03, &[], NONE_FOUND_KEYS)?;
    let Some(state) = o.get("state") else {
        cx.bad(RuleId::R16, pointer, "required key \"state\" is absent");
        return None;
    };
    match cx.vocab_value::<EvidenceStateKind>(&o.at("state"), "state", state, RuleId::R05)? {
        EvidenceStateKind::NoneLinked => {
            let mut ok = true;
            for (k, _) in o
                .members
                .iter()
                .filter(|(k, _)| k != "state" && NONE_FOUND_KEYS.contains(&k.as_str()))
            {
                cx.bad(
                    RuleId::R16,
                    &o.at(k),
                    format!("key {k:?} is not allowed with state none_linked"),
                );
                ok = false;
            }
            ok.then_some(EvidenceState::NoneLinked)
        }
        EvidenceStateKind::NoneFound => {
            let required_text = |cx: &mut Cx, key: &str| {
                if !o.has(key) {
                    cx.bad(RuleId::R16, pointer, format!("none_found requires {key:?}"));
                    return None;
                }
                cx.text(&o, key, RuleId::R07)
            };
            let search_scope = required_text(cx, "search_scope");
            let coverage = required_text(cx, "coverage");
            let assessor = required_text(cx, "assessor");
            let date = if o.has("date") {
                cx.date(&o, "date")
            } else {
                cx.bad(RuleId::R16, pointer, "none_found requires \"date\"");
                None
            };
            let corpus = match o.get("corpus") {
                None => {
                    cx.bad(
                        RuleId::R16,
                        pointer,
                        "none_found requires a corpus {id, sha256|pin}",
                    );
                    None
                }
                Some(c) => {
                    let cp = o.at("corpus");
                    cx.object(&cp, c, RuleId::R03, &[], CORPUS_KEYS)
                        .and_then(|co| {
                            let id = if co.has("id") {
                                cx.text(&co, "id", RuleId::R07)
                            } else {
                                cx.bad(RuleId::R16, &cp, "none_found requires corpus \"id\"");
                                None
                            };
                            if !co.has("sha256") && !co.has("pin") {
                                cx.bad(
                                    RuleId::R16,
                                    &cp,
                                    "none_found requires corpus \"sha256\" or \"pin\"",
                                );
                                return None;
                            }
                            let sha256 = match co.get("sha256") {
                                None => Some(None),
                                Some(Json::String(s)) if is_hex(s, 64, 64) => Some(Some(s.clone())),
                                Some(other) => {
                                    cx.bad(
                                        RuleId::R16,
                                        &co.at("sha256"),
                                        format!(
                                            "sha256 {} must be 64 lowercase hex",
                                            describe(other)
                                        ),
                                    );
                                    None
                                }
                            };
                            let pin = match co.get("pin") {
                                None => Some(None),
                                Some(p) => cx.source(&co.at("pin"), p).map(Some),
                            };
                            Some(Corpus {
                                id: id?,
                                sha256: sha256?,
                                pin: pin?,
                            })
                        })
                }
            };
            Some(EvidenceState::NoneFound(Box::new(NoneFound {
                corpus: corpus?,
                search_scope: search_scope?,
                coverage: coverage?,
                assessor: assessor?,
                date: date?,
            })))
        }
    }
}
