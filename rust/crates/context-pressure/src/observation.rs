//! The pressure observation: the controller's input record and the projector's output record.

use serde::Serialize;

pub const PRESSURE_OBSERVATION_SCHEMA_VERSION: u8 = 1;

/// A validated pressure observation. The projector produces these; the controller accepts the
/// same JSON shape through [`PressureObservation::from_value`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PressureObservation {
    schema_version: u8,
    observation_id: String,
    sequence: u64,
    observed_at: String,
    pressure_basis_points: u16,
    source: String,
    source_revision: String,
}

impl PressureObservation {
    pub(crate) fn new(
        observation_id: String,
        sequence: u64,
        observed_at: String,
        pressure_basis_points: u16,
        source: String,
        source_revision: String,
    ) -> Self {
        Self {
            schema_version: PRESSURE_OBSERVATION_SCHEMA_VERSION,
            observation_id,
            sequence,
            observed_at,
            pressure_basis_points,
            source,
            source_revision,
        }
    }

    pub fn observation_id(&self) -> &str {
        &self.observation_id
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn observed_at(&self) -> &str {
        &self.observed_at
    }

    pub fn pressure_basis_points(&self) -> u16 {
        self.pressure_basis_points
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }
}
