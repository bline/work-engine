//! PROVISIONAL provider header names, quota dimensions and failure
//! classification.
//!
//! Nothing here is verified against a live provider: names and shapes are
//! plausible fixtures until slice 5 retains real evidence. Every
//! provider-specific assumption lives in [`TABLE`]: the dimensions per
//! funding, header names and units, the utilization scale, the error rules
//! and each class's effect on a dimension. Its version and SHA-256 identify
//! it in every output. A change to the parsing code here (header, reset and
//! error-name reading) must bump the version.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The pinned table.
///
/// - `dimensions`: quota dimensions per funding.
/// - `utilization`: the reported share at which a window counts as exhausted,
///   and the range of operator low-quota thresholds (exclusive minimum,
///   inclusive maximum).
/// - `windows`: per-window quota headers.
/// - `retry_after`: the relative retry header.
/// - `head_rules`: (provider, funding, HTTP status, error code or type) of a
///   non-success head to a class.
/// - `stream_rules`: (provider, funding, error code or type) of a provider
///   failure event after a success head to a class.
/// - `effects`: what each (class, funding) means for the dimensions. A class
///   without an effect row for its funding is unclassified.
pub const TABLE: &str = r#"{
  "version": "hosted-route-availability-table.provisional.2",
  "provisional": true,
  "dimensions": {
    "subscription": ["subscription_5h", "subscription_weekly"],
    "api": ["api_rate", "api_credit"]
  },
  "utilization": {"exhausted_at": 1.0, "threshold_min_exclusive": 0.0, "threshold_max": 1.0},
  "windows": [
    {
      "provider": "anthropic",
      "dimension": "subscription_5h",
      "status": "anthropic-ratelimit-unified-5h-status",
      "utilization": "anthropic-ratelimit-unified-5h-utilization",
      "reset": "anthropic-ratelimit-unified-5h-reset",
      "reset_unit": "epoch_s",
      "exhausted": ["rejected"]
    },
    {
      "provider": "anthropic",
      "dimension": "subscription_weekly",
      "status": "anthropic-ratelimit-unified-7d-status",
      "utilization": "anthropic-ratelimit-unified-7d-utilization",
      "reset": "anthropic-ratelimit-unified-7d-reset",
      "reset_unit": "epoch_s",
      "exhausted": ["rejected"]
    }
  ],
  "retry_after": "retry-after",
  "head_rules": [
    {"provider": "anthropic", "funding": "subscription", "status": 429, "error": "rate_limit_error", "class": "subscription_quota_exhausted"},
    {"provider": "anthropic", "funding": "api", "status": 429, "error": "rate_limit_error", "class": "rate_limited"},
    {"provider": "anthropic", "funding": "api", "status": 402, "error": "billing_error", "class": "api_credit_exhausted"},
    {"provider": "anthropic", "funding": "subscription", "status": 529, "error": "overloaded_error", "class": "overload"},
    {"provider": "anthropic", "funding": "api", "status": 529, "error": "overloaded_error", "class": "overload"},
    {"provider": "anthropic", "funding": "subscription", "status": 401, "error": "authentication_error", "class": "authorization_failure"},
    {"provider": "anthropic", "funding": "api", "status": 401, "error": "authentication_error", "class": "authorization_failure"},
    {"provider": "openai", "funding": "subscription", "status": 429, "error": "usage_limit_reached", "class": "subscription_quota_exhausted"},
    {"provider": "openai", "funding": "api", "status": 429, "error": "rate_limit_exceeded", "class": "rate_limited"},
    {"provider": "openai", "funding": "api", "status": 429, "error": "insufficient_quota", "class": "api_credit_exhausted"}
  ],
  "stream_rules": [
    {"provider": "anthropic", "funding": "subscription", "error": "overloaded_error", "class": "overload"},
    {"provider": "anthropic", "funding": "api", "error": "overloaded_error", "class": "overload"},
    {"provider": "openai", "funding": "subscription", "error": "usage_limit_reached", "class": "subscription_quota_exhausted"},
    {"provider": "openai", "funding": "api", "error": "rate_limit_exceeded", "class": "rate_limited"},
    {"provider": "openai", "funding": "api", "error": "insufficient_quota", "class": "api_credit_exhausted"}
  ],
  "effects": [
    {"class": "subscription_quota_exhausted", "funding": "subscription", "effect": "windows"},
    {"class": "rate_limited", "funding": "api", "effect": "retry_after", "dimension": "api_rate"},
    {"class": "api_credit_exhausted", "funding": "api", "effect": "operator_action", "dimension": "api_credit"},
    {"class": "overload", "funding": "subscription", "effect": "non_quota"},
    {"class": "overload", "funding": "api", "effect": "non_quota"},
    {"class": "authorization_failure", "funding": "subscription", "effect": "non_quota"},
    {"class": "authorization_failure", "funding": "api", "effect": "non_quota"}
  ]
}"#;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    pub version: String,
    pub provisional: bool,
    pub dimensions: std::collections::BTreeMap<String, Vec<String>>,
    pub utilization: Utilization,
    pub windows: Vec<Window>,
    pub retry_after: String,
    pub head_rules: Vec<HeadRule>,
    pub stream_rules: Vec<StreamRule>,
    pub effects: Vec<EffectRow>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Utilization {
    pub exhausted_at: f64,
    pub threshold_min_exclusive: f64,
    pub threshold_max: f64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub provider: String,
    pub dimension: String,
    pub status: String,
    pub utilization: String,
    pub reset: String,
    pub reset_unit: ResetUnit,
    /// Status values that mean the window is exhausted.
    pub exhausted: Vec<String>,
}
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetUnit {
    /// Absolute Unix seconds.
    EpochS,
    /// Seconds after the response head.
    RelativeS,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeadRule {
    pub provider: String,
    pub funding: String,
    pub status: u16,
    pub error: String,
    pub class: Class,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamRule {
    pub provider: String,
    pub funding: String,
    pub error: String,
    pub class: Class,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    SubscriptionQuotaExhausted,
    RateLimited,
    ApiCreditExhausted,
    Overload,
    AuthorizationFailure,
    Unclassified,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectRow {
    pub class: Class,
    pub funding: String,
    pub effect: Effect,
    /// The affected dimension, for `retry_after` and `operator_action`.
    #[serde(default)]
    pub dimension: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// The exhausted windows named by the head's window headers.
    Windows,
    /// `dimension` exhausted until the retry header's reset.
    RetryAfter,
    /// `dimension` exhausted with no reset until the operator clears it.
    OperatorAction,
    /// Not a quota signal.
    NonQuota,
}

/// A reset time: absolute epoch ms, or unknown with a reason.
#[derive(Clone, Debug, PartialEq)]
pub enum Reset {
    At(u64),
    Unknown(&'static str),
}

/// One window's reading from a response head's kept headers.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowReading {
    pub dimension: String,
    /// The status header names an exhausted value; `None` when absent.
    pub exhausted: Option<bool>,
    pub utilization: Option<f64>,
    /// `None` when the reset header is absent.
    pub reset: Option<Reset>,
    /// A header was present but unusable: the window's reading is unknown.
    pub malformed: Option<&'static str>,
    /// The status header name, else the first present header name.
    pub source: String,
    /// The utilization header name.
    pub utilization_header: String,
}

pub fn table() -> Table {
    serde_json::from_str(TABLE).expect("the pinned table parses")
}
pub fn table_sha256() -> String {
    format!("{:x}", Sha256::digest(TABLE.as_bytes()))
}

/// The single value of `name`; a repeated header with differing values is
/// malformed.
fn header<'a>(headers: &'a [(String, String)], name: &str) -> Result<Option<&'a str>, ()> {
    let mut values = headers
        .iter()
        .filter(|(k, _)| k == name)
        .map(|(_, v)| v.as_str());
    let first = values.next();
    if values.any(|v| Some(v) != first) {
        return Err(());
    }
    Ok(first)
}
fn reset(value: &str, unit: ResetUnit, head_ms: Option<u64>) -> Reset {
    let Ok(n) = value.trim().parse::<u64>() else {
        return Reset::Unknown("unparseable_reset");
    };
    let Some(ms) = n.checked_mul(1000) else {
        return Reset::Unknown("unparseable_reset");
    };
    match (unit, head_ms) {
        (ResetUnit::EpochS, _) => Reset::At(ms),
        (ResetUnit::RelativeS, Some(head)) => head
            .checked_add(ms)
            .map_or(Reset::Unknown("unparseable_reset"), Reset::At),
        (ResetUnit::RelativeS, None) => Reset::Unknown("relative_reset_without_head_time"),
    }
}

impl Table {
    /// The quota dimensions of a funding.
    pub fn dimensions(&self, funding: &str) -> &[String] {
        self.dimensions.get(funding).map_or(&[], Vec::as_slice)
    }
    /// Dimensions that only the operator clears.
    pub fn operator_action_dimensions(&self, funding: &str) -> Vec<&str> {
        self.effects
            .iter()
            .filter(|e| e.funding == funding && e.effect == Effect::OperatorAction)
            .filter_map(|e| e.dimension.as_deref())
            .collect()
    }
    pub fn effect(&self, class: Class, funding: &str) -> Option<&EffectRow> {
        self.effects
            .iter()
            .find(|e| e.class == class && e.funding == funding)
    }
    pub fn threshold_valid(&self, t: f64) -> bool {
        t.is_finite()
            && t > self.utilization.threshold_min_exclusive
            && t <= self.utilization.threshold_max
    }
    /// A valid window reading names its window exhausted.
    pub fn window_exhausted(&self, w: &WindowReading) -> bool {
        w.malformed.is_none()
            && (w.exhausted == Some(true)
                || w.utilization
                    .is_some_and(|u| u >= self.utilization.exhausted_at))
    }
    /// Readings for each of `provider`'s windows with at least one header.
    pub fn windows(
        &self,
        provider: &str,
        headers: &[(String, String)],
        head_ms: Option<u64>,
    ) -> Vec<WindowReading> {
        let mut out = vec![];
        for w in self.windows.iter().filter(|w| w.provider == provider) {
            let (status, utilization, reset_value) = match (
                header(headers, &w.status),
                header(headers, &w.utilization),
                header(headers, &w.reset),
            ) {
                (Ok(s), Ok(u), Ok(r)) => (s, u, r),
                _ => {
                    out.push(WindowReading {
                        dimension: w.dimension.clone(),
                        exhausted: None,
                        utilization: None,
                        reset: None,
                        malformed: Some("conflicting_header"),
                        source: w.status.clone(),
                        utilization_header: w.utilization.clone(),
                    });
                    continue;
                }
            };
            if status.is_none() && utilization.is_none() && reset_value.is_none() {
                continue;
            }
            let source = [
                (status.is_some(), &w.status),
                (utilization.is_some(), &w.utilization),
                (reset_value.is_some(), &w.reset),
            ]
            .iter()
            .find(|(present, _)| *present)
            .map(|(_, n)| (*n).clone())
            .unwrap_or_default();
            let parsed = utilization.map(|u| u.trim().parse::<f64>());
            let malformed = match &parsed {
                Some(Ok(u)) if !u.is_finite() || *u < 0.0 => Some("malformed_header"),
                Some(Err(_)) => Some("malformed_header"),
                _ => None,
            };
            out.push(WindowReading {
                dimension: w.dimension.clone(),
                exhausted: status.map(|s| w.exhausted.iter().any(|e| e == s)),
                utilization: parsed.and_then(|p| p.ok()).filter(|_| malformed.is_none()),
                reset: reset_value.map(|r| reset(r, w.reset_unit, head_ms)),
                malformed,
                source,
                utilization_header: w.utilization.clone(),
            });
        }
        out
    }
    /// The relative retry header converted with the head time; `None` when
    /// absent.
    pub fn retry_after(&self, headers: &[(String, String)], head_ms: Option<u64>) -> Option<Reset> {
        match header(headers, &self.retry_after) {
            Err(()) => Some(Reset::Unknown("conflicting_header")),
            Ok(v) => v.map(|v| reset(v, ResetUnit::RelativeS, head_ms)),
        }
    }
    /// The class of a non-success head; `Unclassified` unless a head rule
    /// matches exactly, including the error name from `body`.
    pub fn classify_head(&self, provider: &str, funding: &str, status: u16, body: &[u8]) -> Class {
        let Some(error) = error_name(body) else {
            return Class::Unclassified;
        };
        self.head_rules
            .iter()
            .find(|r| {
                r.provider == provider
                    && r.funding == funding
                    && r.status == status
                    && r.error == error
            })
            .map_or(Class::Unclassified, |r| r.class)
    }
    /// The class of a provider failure event in a stream relayed after a
    /// success head; `None` when the stream carries no failure event.
    pub fn classify_stream(&self, provider: &str, funding: &str, stream: &[u8]) -> Option<Class> {
        let error = stream_error_name(stream)?;
        Some(
            self.stream_rules
                .iter()
                .find(|r| r.provider == provider && r.funding == funding && r.error == error)
                .map_or(Class::Unclassified, |r| r.class),
        )
    }
}

fn sse_events(bytes: &[u8]) -> Vec<serde_json::Value> {
    std::str::from_utf8(bytes)
        .map(|text| {
            text.lines()
                .filter_map(|l| l.strip_prefix("data:"))
                .filter_map(|d| serde_json::from_str(d.trim()).ok())
                .collect()
        })
        .unwrap_or_default()
}
/// `error.code`, else `error.type` (the Host's order).
fn code_or_type(error: &serde_json::Value) -> Option<String> {
    error["code"]
        .as_str()
        .or(error["type"].as_str())
        .map(str::to_owned)
}

/// The error name of a non-success head's body: `error.code`, else
/// `error.type`, from a JSON document or the first SSE `data` line carrying
/// an `error` object.
pub fn error_name(body: &[u8]) -> Option<String> {
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(body) {
        return code_or_type(&v["error"]);
    }
    sse_events(body)
        .iter()
        .find_map(|v| code_or_type(&v["error"]))
}

/// The error name of the first provider failure event in a relayed stream:
/// `response.failed` (`response.error`) or `error` (top-level `code`, else
/// `error`), as the Host reads them; `"failed"` for an unnamed failure event.
pub fn stream_error_name(stream: &[u8]) -> Option<String> {
    sse_events(stream)
        .iter()
        .find_map(|v| match v["type"].as_str() {
            Some("response.failed") => {
                Some(code_or_type(&v["response"]["error"]).unwrap_or_else(|| "failed".into()))
            }
            Some("error") => Some(
                v["code"]
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| code_or_type(&v["error"]))
                    .unwrap_or_else(|| "failed".into()),
            ),
            _ => None,
        })
}
