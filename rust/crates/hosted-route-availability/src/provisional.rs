//! PROVISIONAL provider header names and failure classification.
//!
//! Nothing here is verified against a live provider: the names and shapes are
//! plausible fixtures until slice 5 retains real evidence. Every
//! provider-specific header name, unit and error name lives in [`TABLE`],
//! whose version and SHA-256 identify it in every output, so a real name
//! change touches only this module.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The pinned table. `windows` read per-window quota headers; `retry_after`
/// names the relative retry header; `rules` map (provider, funding, HTTP
/// status, error type or code) to a failure class.
pub const TABLE: &str = r#"{
  "version": "hosted-route-availability-table.provisional.1",
  "provisional": true,
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
  "rules": [
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
  ]
}"#;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    pub version: String,
    pub provisional: bool,
    pub windows: Vec<Window>,
    pub retry_after: String,
    pub rules: Vec<Rule>,
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
pub struct Rule {
    pub provider: String,
    pub funding: String,
    pub status: u16,
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
    /// The failure class of a non-success response; `Unclassified` unless a
    /// rule matches exactly, including the error name from `body`.
    pub fn classify(&self, provider: &str, funding: &str, status: u16, body: &[u8]) -> Class {
        let Some(error) = error_name(body) else {
            return Class::Unclassified;
        };
        self.rules
            .iter()
            .find(|r| {
                r.provider == provider
                    && r.funding == funding
                    && r.status == status
                    && r.error == error
            })
            .map_or(Class::Unclassified, |r| r.class)
    }
}

/// The error name of an error body: Messages `error.type`, else Responses
/// `error.code`, from a JSON document or the first SSE `data` line carrying
/// an `error` object.
pub fn error_name(body: &[u8]) -> Option<String> {
    let name = |v: &serde_json::Value| {
        let e = &v["error"];
        e["type"].as_str().or(e["code"].as_str()).map(str::to_owned)
    };
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(body) {
        return name(&v);
    }
    let text = std::str::from_utf8(body).ok()?;
    text.lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .filter_map(|d| serde_json::from_str::<serde_json::Value>(d.trim()).ok())
        .find_map(|v| name(&v))
}
