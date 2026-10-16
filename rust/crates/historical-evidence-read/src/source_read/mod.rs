//! Private source adapters. They inspect admitted historical bytes; none owns or mutates workflow state.
pub mod frozen_metrics;
pub mod provider_capture;
pub mod scheduler_item;

use historical_evidence_wire::{
    MetricsSource, ProviderSources, SchedulerSource, validate_v2_sources,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictV2Grant {
    schema_version: u8,
    profile: String,
    grant_id: String,
    repository: String,
    run_id: String,
    slice_number: u8,
    issued_at: String,
    expires_at: String,
    issuer_provenance: String,
    peer_uid: u32,
    audience: String,
    fields: Vec<String>,
    checkpoints: Vec<Value>,
    gates: Vec<Value>,
    reviews: Vec<Value>,
    provider: ProviderSources,
    scheduler: SchedulerSource,
    metrics: MetricsSource,
}

pub fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing {key}"))
}
pub fn u64_field(value: &Value, key: &str) -> Result<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing {key}"))
}
pub fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing {key}"))
}
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn sha(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub fn regular_read(path: &Path, cap: u64) -> Result<Vec<u8>> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| e.to_string())?;
    let info = file.metadata().map_err(|e| e.to_string())?;
    if !info.is_file() || info.len() > cap {
        return Err("source is not a bounded regular file".into());
    }
    let mut out = Vec::with_capacity(info.len() as usize);
    file.take(cap + 1)
        .read_to_end(&mut out)
        .map_err(|e| e.to_string())?;
    if out.len() as u64 > cap {
        return Err("source grew beyond limit".into());
    }
    Ok(out)
}

pub fn grant(path: &Path, expected_sha: &str) -> Result<Value> {
    if !path.is_absolute() {
        return Err("grant path is not absolute".into());
    }
    let parent = path.parent().ok_or("grant has no directory")?;
    let dir = std::fs::symlink_metadata(parent).map_err(|e| e.to_string())?;
    let uid = unsafe { libc::geteuid() };
    if !dir.is_dir()
        || dir.file_type().is_symlink()
        || dir.uid() != uid
        || dir.mode() & 0o777 != 0o700
    {
        return Err("grant directory is not private".into());
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| e.to_string())?;
    let info = file.metadata().map_err(|e| e.to_string())?;
    if !info.is_file() || info.uid() != uid || info.mode() & 0o777 != 0o600 || info.len() > 65536 {
        return Err("grant file is not private".into());
    }
    let mut bytes = Vec::new();
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 || sha(&bytes) != expected_sha {
        return Err("grant bytes changed".into());
    }
    let strict: StrictV2Grant = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if strict.schema_version != 2
        || strict.profile != "historical-s5-local-inspection-v2"
        || strict.run_id != "skills-migration-app-server-20260828"
        || strict.slice_number != 6
        || strict.grant_id.is_empty()
        || strict.issuer_provenance.is_empty()
        || strict.peer_uid != uid
        || strict.audience != "local_terminal_inspection"
        || strict.fields
            != [
                "checkpoint",
                "gate",
                "metric_discrepancy",
                "metrics_comparison",
                "provider_attempts",
                "review_summary",
                "scheduler_item",
                "source_limit",
            ]
        || strict.checkpoints.len() != 6
        || strict.gates.len() != 5
        || strict.reviews.len() != 3
    {
        return Err("grant is not fixed S5 v2".into());
    }
    let gate_digest = string(&strict.gates[4], "receipt_sha256")?;
    validate_v2_sources(
        &strict.provider,
        &strict.scheduler,
        &strict.metrics,
        &strict.repository,
        gate_digest,
    )
    .map_err(str::to_string)?;
    let issued =
        chrono::DateTime::parse_from_rfc3339(&strict.issued_at).map_err(|e| e.to_string())?;
    let expires =
        chrono::DateTime::parse_from_rfc3339(&strict.expires_at).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now();
    if issued > now || expires <= now || expires <= issued {
        return Err("source read grant time window invalid".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub fn canonical_row_sha(row: &Value) -> Result<String> {
    let bytes = serde_json::to_vec(row).map_err(|e| e.to_string())?;
    Ok(sha(&bytes))
}

pub fn decimal_scaled(literal: &str) -> Result<(u128, u32)> {
    if literal.is_empty() || literal.starts_with('-') || literal.contains(['e', 'E', '+']) {
        return Err("unsupported decimal".into());
    }
    let mut parts = literal.split('.');
    let whole = parts.next().unwrap();
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some()
        || whole.is_empty()
        || fraction.len() > 18
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err("invalid bounded decimal".into());
    }
    let digits = format!("{whole}{fraction}")
        .parse::<u128>()
        .map_err(|e| e.to_string())?;
    Ok((digits, fraction.len() as u32))
}

pub fn decimal_sum(literals: &[String]) -> Result<String> {
    let terms = literals
        .iter()
        .map(|s| decimal_scaled(s))
        .collect::<Result<Vec<_>>>()?;
    let scale = terms.iter().map(|(_, n)| *n).max().unwrap_or(0);
    let mut total = 0u128;
    for (value, n) in terms {
        let multiplier = 10u128
            .checked_pow(scale - n)
            .ok_or("decimal scale overflow")?;
        total = total
            .checked_add(value.checked_mul(multiplier).ok_or("decimal overflow")?)
            .ok_or("decimal overflow")?;
    }
    if scale == 0 {
        return Ok(total.to_string());
    }
    let unit = 10u128.pow(scale);
    Ok(format!(
        "{}.{:0width$}",
        total / unit,
        total % unit,
        width = scale as usize
    ))
}
