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

/// The estimator's own confidence (UD3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
