//! Closed serde types for the estimate-set record (contract 2.1).
//!
//! Every struct and enum carries `deny_unknown_fields`. Collections are `Vec`; nothing here
//! is a map, a float or an untyped JSON value, so output order is the declared field order
//! and no float can enter a record (contract S3, S4). A nullable field is a required key whose
//! value may be `null`: the `required` deserializer below refuses an absent key, which serde
//! would otherwise default to `None`.

use serde::{Deserialize, Deserializer, Serialize};

pub const ESTIMATE_SET_SCHEMA: &str = "proposal-evaluation-estimate-set-v1";

/// Deserialise a nullable field so that the key must be present (contract S4).
fn required<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// `{path, sha256}`: an absolute normalised path and the sha256 of the bytes found there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRef {
    pub path: String,
    pub sha256: String,
}

/// `{proposal_id, packet}`: the proposal revision an estimate set is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalRef {
    pub proposal_id: String,
    pub packet: FileRef,
}

/// Who issued the estimates (provenance, EI:99).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Estimator {
    pub role: String,
    pub identity: String,
}

/// Constant non-authority block; every member must be `false` (contract S10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    pub readiness_judgment_issued: bool,
    pub priority_authorized: bool,
    pub acceptance_authorized: bool,
    /// Added by contract erratum E1-1 (EI:129, EV:27).
    pub implementation_authorized: bool,
}

/// Which way is better on a measure's scale (EV:15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    HigherIsBetter,
    LowerIsBetter,
}

/// The estimator's own confidence (UD3). Declaration order is the order `low < medium < high`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// A measure's scale. Ordinal levels are listed lowest to highest; a quantity is an `i64`
/// meaning `value x 10^exponent` (no floats).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Scale {
    Ordinal { levels: Vec<String> },
    Quantity { unit: String, exponent: i32 },
}

/// A value on a scale: `{"quantity": n}` or `{"level": label}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Value {
    Quantity(i64),
    Level(String),
}

/// What is measured (EI:108, EV:87-88).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measure {
    pub definition: String,
    pub scale: Scale,
    pub direction: Direction,
    pub scope: String,
}

/// An exact claim revision an estimate rests on (names from claim-evidence `reliance.rs`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasisRef {
    pub claim_id: String,
    pub claim_revision_id: String,
    pub claim_revision_sha256: String,
}

/// `{low, high}` in scale order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interval {
    pub low: Value,
    pub high: Value,
}

/// One competing reading of a contested measure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Competing {
    pub value: Value,
    pub basis: Vec<BasisRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Estimated {
    pub measure_id: String,
    pub measure: Measure,
    #[serde(deserialize_with = "required")]
    pub evidence_cutoff: Option<String>,
    #[serde(deserialize_with = "required")]
    pub baseline: Option<Value>,
    pub basis: Vec<BasisRef>,
    pub limitations: Vec<String>,
    pub value: Value,
    #[serde(deserialize_with = "required")]
    pub interval: Option<Interval>,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contested {
    pub measure_id: String,
    pub measure: Measure,
    #[serde(deserialize_with = "required")]
    pub evidence_cutoff: Option<String>,
    #[serde(deserialize_with = "required")]
    pub baseline: Option<Value>,
    pub basis: Vec<BasisRef>,
    pub limitations: Vec<String>,
    pub competing: Vec<Competing>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotEstimated {
    pub measure_id: String,
    pub measure: Measure,
    #[serde(deserialize_with = "required")]
    pub evidence_cutoff: Option<String>,
    #[serde(deserialize_with = "required")]
    pub baseline: Option<Value>,
    pub basis: Vec<BasisRef>,
    pub limitations: Vec<String>,
    pub reason: String,
}

/// One estimate, tagged by `status` (EV:46, EV:168).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Estimate {
    Estimated(Estimated),
    Contested(Contested),
    NotEstimated(NotEstimated),
}

impl Estimate {
    pub fn measure_id(&self) -> &str {
        match self {
            Estimate::Estimated(e) => &e.measure_id,
            Estimate::Contested(e) => &e.measure_id,
            Estimate::NotEstimated(e) => &e.measure_id,
        }
    }

    pub fn measure(&self) -> &Measure {
        match self {
            Estimate::Estimated(e) => &e.measure,
            Estimate::Contested(e) => &e.measure,
            Estimate::NotEstimated(e) => &e.measure,
        }
    }

    pub fn baseline(&self) -> Option<&Value> {
        match self {
            Estimate::Estimated(e) => e.baseline.as_ref(),
            Estimate::Contested(e) => e.baseline.as_ref(),
            Estimate::NotEstimated(e) => e.baseline.as_ref(),
        }
    }

    /// The estimate's own cutoff, if it states one.
    pub fn own_cutoff(&self) -> Option<&str> {
        match self {
            Estimate::Estimated(e) => e.evidence_cutoff.as_deref(),
            Estimate::Contested(e) => e.evidence_cutoff.as_deref(),
            Estimate::NotEstimated(e) => e.evidence_cutoff.as_deref(),
        }
    }

    /// The estimate's own cutoff when non-null, else the set's (contract 2.1).
    pub fn effective_cutoff<'a>(&'a self, set_cutoff: &'a str) -> &'a str {
        self.own_cutoff().unwrap_or(set_cutoff)
    }

    pub fn basis(&self) -> &[BasisRef] {
        match self {
            Estimate::Estimated(e) => &e.basis,
            Estimate::Contested(e) => &e.basis,
            Estimate::NotEstimated(e) => &e.basis,
        }
    }

    pub fn limitations(&self) -> &[String] {
        match self {
            Estimate::Estimated(e) => &e.limitations,
            Estimate::Contested(e) => &e.limitations,
            Estimate::NotEstimated(e) => &e.limitations,
        }
    }

    /// Every basis entry the estimate carries, including those of competing readings.
    pub fn all_basis(&self) -> Vec<&BasisRef> {
        let mut all: Vec<&BasisRef> = self.basis().iter().collect();
        if let Estimate::Contested(c) = self {
            all.extend(c.competing.iter().flat_map(|c| c.basis.iter()));
        }
        all
    }
}

/// One estimate set per (proposal revision, estimator issue).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstimateSet {
    pub schema: String,
    pub estimate_set_id: String,
    pub proposal: ProposalRef,
    pub estimator: Estimator,
    pub issued_at: String,
    pub evidence_cutoff: String,
    #[serde(deserialize_with = "required")]
    pub claim_projection: Option<FileRef>,
    pub estimates: Vec<Estimate>,
    pub limitations: Vec<String>,
    pub authority: Authority,
}

// ---------------------------------------------------------------------------
// E2: the comparison contract (contract 3.1)
// ---------------------------------------------------------------------------

pub const CONTRACT_SCHEMA: &str = "proposal-comparison-contract-v1";

/// The authority that selected the comparison surface (EI:112).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectingAuthority {
    pub decision_owner: String,
    pub exercised_by: String,
    pub evidence: String,
}

/// A proposal revision the contract compares: the sha256 of its `packet.json` bytes (UD1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractProposal {
    pub proposal_id: String,
    pub packet_sha256: String,
}

/// One measure the contract includes, with the confidence an estimate must reach (UD3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractMeasure {
    pub measure_id: String,
    pub definition: String,
    pub scale: Scale,
    pub direction: Direction,
    pub scope: String,
    pub minimum_confidence: Confidence,
}

impl ContractMeasure {
    /// The comparability key an estimate's `measure` must equal (EV:114).
    pub fn measure(&self) -> Measure {
        Measure {
            definition: self.definition.clone(),
            scale: self.scale.clone(),
            direction: self.direction,
            scope: self.scope.clone(),
        }
    }
}

/// A concern the contract intentionally leaves out (EI:107).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExcludedConcern {
    pub concern: String,
    pub reason: String,
}

/// How a missing, contested or stale condition is handled (UD6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreatmentKind {
    Block,
    DropMeasure,
}

/// Every member is required: a closed schema has no implicit default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Treatment {
    pub missing: TreatmentKind,
    pub contested: TreatmentKind,
    pub stale: TreatmentKind,
}

/// Whether intervals are used in the comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Uncertainty {
    Point,
    IntervalRobust,
}

/// Constant non-authority block of a contract (EV:140); every member must be `false`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractAuthority {
    pub priority_authorized: bool,
    pub acceptance_authorized: bool,
}

/// The authored comparison contract. There is deliberately no weights field (EV:122).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonContract {
    pub schema: String,
    pub contract_id: String,
    pub version: u64,
    pub selecting_authority: SelectingAuthority,
    pub decision_scope: String,
    pub proposals: Vec<ContractProposal>,
    pub measures: Vec<ContractMeasure>,
    pub excluded_concerns: Vec<ExcludedConcern>,
    pub treatment: Treatment,
    pub uncertainty: Uncertainty,
    pub evidence_cutoff: String,
    pub stale_before: String,
    pub authority: ContractAuthority,
}

// ---------------------------------------------------------------------------
// E2: the derived comparison finding (contract 3.3)
// ---------------------------------------------------------------------------

pub const COMPARISON_SCHEMA: &str = "proposal-evaluation-comparison-v1";

/// Per-measure relation of side `a` to side `b` (contract 3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    Better,
    Worse,
    Equal,
    Overlap,
    Blocked,
}

/// Pair outcome (UD2): three determinations and no outcome that names equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    ADominatesB,
    BDominatesA,
    Incomparable,
    Blocked,
}

/// Why a measure is blocked or dropped for a pair. Declaration order is the order reasons are
/// listed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Missing,
    Contested,
    BelowMinimumConfidence,
    Stale,
    DefinitionMismatch,
    NoComparableMeasures,
}

/// The tool that derived a record (contract S9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolRecord {
    pub name: String,
    pub version: String,
    pub binary_sha256: String,
}

/// The contract file a finding was derived from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractRef {
    pub path: String,
    pub sha256: String,
    pub contract_id: String,
    pub version: u64,
}

/// What the finding is conditional on (EI:114-118).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionalOn {
    pub contract_id: String,
    pub contract_version: u64,
    pub contract_sha256: String,
    pub evidence_cutoff: String,
}

/// One estimate set used in the derivation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstimateSetRef {
    pub proposal_id: String,
    pub packet_sha256: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairMeasure {
    pub measure_id: String,
    pub relation: Relation,
    pub reasons: Vec<Reason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DroppedMeasure {
    pub measure_id: String,
    pub reasons: Vec<Reason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pair {
    pub a: String,
    pub b: String,
    pub measures: Vec<PairMeasure>,
    pub dropped_measures: Vec<DroppedMeasure>,
    pub outcome: Outcome,
    pub blocked_reasons: Vec<Reason>,
}

/// The partition of the contract's proposals (contract 3.2); each array is sorted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frontier {
    pub non_dominated: Vec<String>,
    pub dominated: Vec<String>,
    pub undetermined: Vec<String>,
}

/// Constant non-authority block of a finding (EV:140, EV:171); every member must be `false`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonAuthority {
    pub priority_authorized: bool,
    pub acceptance_authorized: bool,
    pub implementation_authorized: bool,
    pub readiness_judgment_issued: bool,
}

/// The derived comparison finding. It is conditional on the contract and cutoff it names and
/// carries no priority, acceptance or readiness authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonFinding {
    pub schema: String,
    pub tool: ToolRecord,
    pub derived_at: String,
    pub contract: ContractRef,
    pub conditional_on: ConditionalOn,
    pub estimate_sets: Vec<EstimateSetRef>,
    pub pairs: Vec<Pair>,
    pub frontier: Frontier,
    pub authority: ComparisonAuthority,
    pub limitations: Vec<String>,
}
