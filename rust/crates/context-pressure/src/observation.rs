//! The pressure observation: the controller's input record and the projector's output record.

use crate::error::{ErrorKind, PressureError};
use crate::fields;
use serde::Serialize;
use serde_json::Value;

pub const PRESSURE_OBSERVATION_SCHEMA_VERSION: u8 = 1;

const OBSERVATION_FIELDS: [&str; 7] = [
    "schemaVersion",
    "observationId",
    "sequence",
    "observedAt",
    "pressureBasisPoints",
    "source",
    "sourceRevision",
];

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

    /// `normalizeObservation`: validates a plain observation object.
    pub fn from_value(value: &Value) -> Result<Self, PressureError> {
        let label = "context pressure observation";
        let map = fields::record(Some(value), label)?;
        fields::reject_unknown(map, &OBSERVATION_FIELDS, label)?;
        if !fields::is_number(
            map.get("schemaVersion"),
            f64::from(PRESSURE_OBSERVATION_SCHEMA_VERSION),
        ) {
            return Err(PressureError::new(
                ErrorKind::UnsupportedSchemaVersion,
                "unsupported context pressure observation schema version",
            ));
        }
        Ok(Self::new(
            fields::text(map.get("observationId"), "context pressure observation id")?.to_owned(),
            fields::positive_integer(map.get("sequence"), "context pressure observation sequence")?,
            fields::timestamp(map.get("observedAt"), "context pressure observedAt")?.to_owned(),
            fields::basis_points(
                map.get("pressureBasisPoints"),
                "context pressure observation pressureBasisPoints",
            )?,
            fields::text(map.get("source"), "context pressure observation source")?.to_owned(),
            fields::sha_revision(
                map.get("sourceRevision"),
                "context pressure observation source revision",
            )?
            .to_owned(),
        ))
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
