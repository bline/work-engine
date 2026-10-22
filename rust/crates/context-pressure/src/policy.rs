//! Pressure policy: three hysteresis bands, validated and revision-bound.

use crate::error::{ErrorKind, PressureError};
use crate::{fields, js};
use serde::Serialize;
use serde_json::{Value, json};

pub const CONTEXT_PRESSURE_POLICY_SCHEMA_VERSION: u8 = 1;

const POLICY_FIELDS: [&str; 5] = [
    "schemaVersion",
    "unit",
    "approaching",
    "replacementCandidate",
    "critical",
];
const BAND_FIELDS: [&str; 2] = ["enter", "exit"];

/// A pressure disposition. Scheduling evidence only: it authorizes no retirement and carries no
/// checkpoint or transition progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Comfortable,
    Approaching,
    ReplacementCandidate,
    Critical,
}

impl Disposition {
    pub const ALL: [Disposition; 4] = [
        Self::Comfortable,
        Self::Approaching,
        Self::ReplacementCandidate,
        Self::Critical,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Comfortable => "comfortable",
            Self::Approaching => "approaching",
            Self::ReplacementCandidate => "replacement_candidate",
            Self::Critical => "critical",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.as_str() == text)
    }
}

/// The measurement unit thresholds are stated in. The controller reads thresholds only through
/// the policy, so a later unit (for example absolute tokens) adds a variant here and a matching
/// observation reading without changing controller logic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyUnit {
    BasisPoints,
}

/// Entry and exit thresholds of one disposition, in the policy's unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Band {
    pub enter: u16,
    pub exit: u16,
}

/// A validated policy. Construct with [`ContextPressurePolicy::from_value`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPressurePolicy {
    schema_version: u8,
    unit: PolicyUnit,
    approaching: Band,
    replacement_candidate: Band,
    critical: Band,
    policy_revision: String,
}

impl ContextPressurePolicy {
    /// `validateContextPressurePolicy`.
    pub fn from_value(value: &Value) -> Result<Self, PressureError> {
        let map = fields::record(Some(value), "context pressure policy")?;
        fields::reject_unknown(map, &POLICY_FIELDS, "context pressure policy")?;
        if !fields::is_number(
            map.get("schemaVersion"),
            f64::from(CONTEXT_PRESSURE_POLICY_SCHEMA_VERSION),
        ) {
            return Err(PressureError::new(
                ErrorKind::UnsupportedSchemaVersion,
                "unsupported context pressure policy schema version",
            ));
        }
        if !fields::is_string(map.get("unit"), "basis_points") {
            return Err(PressureError::new(
                ErrorKind::UnsupportedUnit,
                "context pressure policy unit must be basis_points",
            ));
        }
        let approaching = band(map.get("approaching"), "context pressure approaching band")?;
        let replacement_candidate = band(
            map.get("replacementCandidate"),
            "context pressure replacementCandidate band",
        )?;
        let critical = band(map.get("critical"), "context pressure critical band")?;
        let ordered = [
            approaching.exit,
            approaching.enter,
            replacement_candidate.exit,
            replacement_candidate.enter,
            critical.exit,
            critical.enter,
        ];
        if ordered.windows(2).any(|pair| pair[1] < pair[0]) {
            return Err(PressureError::new(
                ErrorKind::ThresholdsNotOrdered,
                "context pressure thresholds must be ordered across approaching, replacementCandidate, and critical bands",
            ));
        }
        let body = json!({
            "schemaVersion": CONTEXT_PRESSURE_POLICY_SCHEMA_VERSION,
            "unit": "basis_points",
            "approaching": { "enter": approaching.enter, "exit": approaching.exit },
            "replacementCandidate": {
                "enter": replacement_candidate.enter,
                "exit": replacement_candidate.exit,
            },
            "critical": { "enter": critical.enter, "exit": critical.exit },
        });
        Ok(Self {
            schema_version: CONTEXT_PRESSURE_POLICY_SCHEMA_VERSION,
            unit: PolicyUnit::BasisPoints,
            approaching,
            replacement_candidate,
            critical,
            policy_revision: js::revision(&body),
        })
    }

    pub fn unit(&self) -> PolicyUnit {
        self.unit
    }

    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
    }

    pub fn approaching(&self) -> Band {
        self.approaching
    }

    pub fn replacement_candidate(&self) -> Band {
        self.replacement_candidate
    }

    pub fn critical(&self) -> Band {
        self.critical
    }

    /// The disposition `pressure` selects when the controller currently holds `current`
    /// (`dispositionFor`). Entry thresholds are used to climb and exit thresholds to hold.
    pub fn next_disposition(&self, current: Disposition, pressure: u16) -> Disposition {
        match current {
            Disposition::Comfortable => {
                if pressure >= self.critical.enter {
                    Disposition::Critical
                } else if pressure >= self.replacement_candidate.enter {
                    Disposition::ReplacementCandidate
                } else if pressure >= self.approaching.enter {
                    Disposition::Approaching
                } else {
                    Disposition::Comfortable
                }
            }
            Disposition::Approaching => {
                if pressure >= self.critical.enter {
                    Disposition::Critical
                } else if pressure >= self.replacement_candidate.enter {
                    Disposition::ReplacementCandidate
                } else if pressure >= self.approaching.exit {
                    Disposition::Approaching
                } else {
                    Disposition::Comfortable
                }
            }
            Disposition::ReplacementCandidate => {
                if pressure >= self.critical.enter {
                    Disposition::Critical
                } else if pressure >= self.replacement_candidate.exit {
                    Disposition::ReplacementCandidate
                } else if pressure >= self.approaching.exit {
                    Disposition::Approaching
                } else {
                    Disposition::Comfortable
                }
            }
            Disposition::Critical => {
                if pressure >= self.critical.exit {
                    Disposition::Critical
                } else if pressure >= self.replacement_candidate.exit {
                    Disposition::ReplacementCandidate
                } else if pressure >= self.approaching.exit {
                    Disposition::Approaching
                } else {
                    Disposition::Comfortable
                }
            }
        }
    }

    /// Entry threshold of a non-comfortable disposition; `None` for comfortable.
    pub fn entry_threshold(&self, disposition: Disposition) -> Option<u16> {
        match disposition {
            Disposition::Comfortable => None,
            Disposition::Approaching => Some(self.approaching.enter),
            Disposition::ReplacementCandidate => Some(self.replacement_candidate.enter),
            Disposition::Critical => Some(self.critical.enter),
        }
    }
}

fn band(value: Option<&Value>, label: &str) -> Result<Band, PressureError> {
    let map = fields::record(value, label)?;
    fields::reject_unknown(map, &BAND_FIELDS, label)?;
    let enter = fields::basis_points(map.get("enter"), &format!("{label}.enter"))?;
    let exit = fields::basis_points(map.get("exit"), &format!("{label}.exit"))?;
    if exit >= enter {
        return Err(PressureError::new(
            ErrorKind::BandExitNotBelowEnter,
            format!("{label}.exit must be lower than {label}.enter"),
        ));
    }
    Ok(Band { enter, exit })
}
