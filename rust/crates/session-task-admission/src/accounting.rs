//! Parse the supported canonical usage-ledger control observation. This is an
//! observation, not proof of an exclusive controller or a provider reservation.
use serde::Deserialize;
use serde_json::{Number, Value};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Deserialize)]
struct Spend {
    quota_usd_low: Number,
    quota_usd_high: Number,
    unpriced_calls: u64,
}
#[derive(Deserialize)]
struct Observed {
    quota_usd_low: Number,
    quota_usd_high: Number,
    unpriced_calls: u64,
    cost_basis: String,
}
#[derive(Deserialize)]
struct Control {
    pool: String,
    #[serde(deserialize_with = "crate::required_option")]
    node_id: Option<Value>,
    max_usd: Number,
    basis: String,
    observed: Observed,
    remaining_usd: Option<Number>,
}
#[derive(Deserialize)]
struct Totals {
    codex: Spend,
    claude_all_models: Spend,
    claude_fable: Spend,
}
#[derive(Deserialize)]
struct Snapshot {
    snapshot_at: String,
    status: String,
    totals: Totals,
    cost_controls: Vec<Control>,
}
#[derive(Deserialize)]
struct Document {
    schema: String,
    topic: String,
    #[serde(deserialize_with = "crate::required_option")]
    closed_at: Option<Value>,
    snapshot: Snapshot,
}
#[allow(dead_code)]
pub(crate) struct Observation {
    pub(crate) source_sha: String,
    pub(crate) observed_spend: String,
    pub(crate) at: OffsetDateTime,
}
fn exact(a: &str, b: &str) -> bool {
    crate::budget_within(a, b, "0") && crate::budget_within(b, a, "0")
}
fn settled(s: &Spend) -> bool {
    s.unpriced_calls == 0 && exact(&s.quota_usd_low.to_string(), &s.quota_usd_high.to_string())
}
/// Raw bytes must come directly from the fixed, pinned canonical ledger helper
/// and configured record; this routine cannot establish their provenance.
#[allow(dead_code)]
pub(crate) fn observe(
    raw: &str,
    objective: &str,
    maximum: &str,
    productive: &str,
    reserve: &str,
    now: OffsetDateTime,
) -> Option<Observation> {
    let d: Document = serde_json::from_str(raw).ok()?;
    let at = OffsetDateTime::parse(&d.snapshot.snapshot_at, &Rfc3339).ok()?;
    if d.schema != "objective-budget-control-v1"
        || d.topic != format!("control:budget:{objective}")
        || d.closed_at.is_some()
        || d.snapshot.status != "complete"
        || at.offset() != time::UtcOffset::UTC
        || at > now
        || now - at > Duration::seconds(30)
        || !settled(&d.snapshot.totals.codex)
        || !settled(&d.snapshot.totals.claude_all_models)
        || !settled(&d.snapshot.totals.claude_fable)
        || !exact(
            &d.snapshot
                .totals
                .claude_all_models
                .quota_usd_high
                .to_string(),
            "0",
        )
        || !exact(
            &d.snapshot.totals.claude_fable.quota_usd_high.to_string(),
            "0",
        )
    {
        return None;
    }
    let selected: Vec<_> = d
        .snapshot
        .cost_controls
        .iter()
        .filter(|c| c.pool == "codex" && c.node_id.is_none())
        .collect();
    if selected.len() != 1 {
        return None;
    }
    let c = selected[0];
    let spend = d.snapshot.totals.codex.quota_usd_high.to_string();
    if c.basis != "speed_adjusted_api_usd"
        || c.observed.cost_basis != "speed_adjusted_api_usd"
        || c.observed.unpriced_calls != 0
        || c.remaining_usd
            .as_ref()
            .is_none_or(|v| !crate::decimal(&v.to_string()))
        || !exact(&c.max_usd.to_string(), maximum)
        || !exact(&c.observed.quota_usd_high.to_string(), &spend)
        || !exact(&c.observed.quota_usd_low.to_string(), &spend)
        || !crate::budget_within(maximum, &spend, reserve)
        || !crate::budget_within(productive, &spend, "0")
    {
        return None;
    }
    Some(Observation {
        source_sha: crate::hash(raw),
        observed_spend: spend,
        at,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use time::macros::datetime;
    #[test]
    fn unknown_stale_unpriced_and_insufficient_headroom_hold() {
        let spend = json!({"quota_usd_low":1.0,"quota_usd_high":1.0,"unpriced_calls":0});
        let zero = json!({"quota_usd_low":0.0,"quota_usd_high":0.0,"unpriced_calls":0});
        let mut v = json!({"schema":"objective-budget-control-v1","topic":"control:budget:o","closed_at":null,"snapshot":{"snapshot_at":"2026-11-02T00:00:00Z","status":"complete","totals":{"codex":spend,"claude_all_models":zero,"claude_fable":zero},"cost_controls":[{"pool":"codex","node_id":null,"max_usd":5.0,"basis":"speed_adjusted_api_usd","remaining_usd":4.0,"observed":{"quota_usd_low":1.0,"quota_usd_high":1.0,"unpriced_calls":0,"cost_basis":"speed_adjusted_api_usd"}}]}});
        let now = datetime!(2026-11-02 00:00:10 UTC);
        assert!(observe(&v.to_string(), "o", "5", "4", "1", now).is_some());
        assert!(
            observe(
                &v.to_string(),
                "o",
                "5",
                "4",
                "1",
                datetime!(2026-11-02 00:01 UTC)
            )
            .is_none()
        );
        v["snapshot"]["status"] = "partial".into();
        assert!(observe(&v.to_string(), "o", "5", "4", "1", now).is_none());
        v["snapshot"]["status"] = "complete".into();
        v["snapshot"]["totals"]["codex"]["unpriced_calls"] = 1.into();
        assert!(observe(&v.to_string(), "o", "5", "4", "1", now).is_none());
        v["snapshot"]["totals"]["codex"]["unpriced_calls"] = 0.into();
        assert!(observe(&v.to_string(), "o", "5", "0.5", "1", now).is_none());
    }
}
