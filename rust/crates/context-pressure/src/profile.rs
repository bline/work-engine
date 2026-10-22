//! Measurement profile: how a provider usage observation becomes a pressure value. Closed
//! enums today; each is a dispatch point for a later profile (for example absolute tokens).

use crate::error::{ErrorKind, PressureError};
use crate::{fields, js};
use serde::Serialize;
use serde_json::{Value, json};

pub const TOKEN_USAGE_PRESSURE_PROFILE_SCHEMA_VERSION: u8 = 1;

const PROFILE_FIELDS: [&str; 5] = [
    "schemaVersion",
    "usageField",
    "windowField",
    "rounding",
    "saturation",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum UsageField {
    #[serde(rename = "last.totalTokens")]
    LastTotalTokens,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum WindowField {
    #[serde(rename = "modelContextWindow")]
    ModelContextWindow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rounding {
    Floor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Saturation {
    #[serde(rename = "clamp_10000")]
    Clamp10000,
}

/// A validated profile. Construct with [`TokenUsagePressureProfile::from_value`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsagePressureProfile {
    schema_version: u8,
    usage_field: UsageField,
    window_field: WindowField,
    rounding: Rounding,
    saturation: Saturation,
    profile_revision: String,
}

impl TokenUsagePressureProfile {
    /// `validateTokenUsagePressureProfile`.
    pub fn from_value(value: &Value) -> Result<Self, PressureError> {
        let label = "token usage pressure profile";
        let map = fields::record(Some(value), label)?;
        fields::reject_unknown(map, &PROFILE_FIELDS, label)?;
        if !fields::is_number(
            map.get("schemaVersion"),
            f64::from(TOKEN_USAGE_PRESSURE_PROFILE_SCHEMA_VERSION),
        ) {
            return Err(PressureError::new(
                ErrorKind::UnsupportedSchemaVersion,
                "unsupported token usage pressure profile schema version",
            ));
        }
        if !(fields::is_string(map.get("usageField"), "last.totalTokens")
            && fields::is_string(map.get("windowField"), "modelContextWindow")
            && fields::is_string(map.get("rounding"), "floor")
            && fields::is_string(map.get("saturation"), "clamp_10000"))
        {
            return Err(PressureError::new(
                ErrorKind::UnsupportedProfile,
                "unsupported token usage pressure measurement profile",
            ));
        }
        let body = json!({
            "schemaVersion": TOKEN_USAGE_PRESSURE_PROFILE_SCHEMA_VERSION,
            "usageField": "last.totalTokens",
            "windowField": "modelContextWindow",
            "rounding": "floor",
            "saturation": "clamp_10000",
        });
        Ok(Self {
            schema_version: TOKEN_USAGE_PRESSURE_PROFILE_SCHEMA_VERSION,
            usage_field: UsageField::LastTotalTokens,
            window_field: WindowField::ModelContextWindow,
            rounding: Rounding::Floor,
            saturation: Saturation::Clamp10000,
            profile_revision: js::revision(&body),
        })
    }

    pub fn profile_revision(&self) -> &str {
        &self.profile_revision
    }

    pub fn usage_field(&self) -> UsageField {
        self.usage_field
    }

    pub fn window_field(&self) -> WindowField {
        self.window_field
    }

    pub fn rounding(&self) -> Rounding {
        self.rounding
    }

    pub fn saturation(&self) -> Saturation {
        self.saturation
    }
}
