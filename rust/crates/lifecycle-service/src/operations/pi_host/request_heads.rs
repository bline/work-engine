//! Verify the pinned Host's Messages request-head v1 records. The two digests
//! attest consistency of Host records, not independent observation of a send.
use super::{Expected, sha};
use lifecycle_store::StoreError;
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::Read;
use std::path::Path;

const SCHEMA: &str = "work-engine.pi-host-request-head.v1";
const REDACTED: &str = "[redacted]";
// Separate intake bounds. Neither counts towards the 8 MiB request-body cap.
// The encoded head must fit even if every individual field fits its writer
// bound; excess JSON escaping can cause a truthful intake refusal.
const MAX_HEAD_BYTES: u64 = 256 * 1024;
const MAX_JOURNAL_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Head {
    schema: String,
    request: u64,
    attempt: u64,
    route: String,
    redacted: String,
    head: String,
}

// The state directory is owned by the launched Host/lifecycle process, not
// the container. Require the exact regular file (not an alias or nested path).
fn read_named(root: &Path, name: &str, limit: u64) -> Result<Vec<u8>, StoreError> {
    let path = root.join(name);
    let meta = path.symlink_metadata().map_err(|_| StoreError::Rejected)?;
    if !meta.is_file() || meta.len() > limit {
        return Err(StoreError::Rejected);
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| StoreError::Rejected)?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| StoreError::Rejected)?;
    if bytes.len() as u64 > limit {
        return Err(StoreError::Rejected);
    }
    Ok(bytes)
}

pub(super) fn read_journal(root: &Path) -> Result<Value, StoreError> {
    let bytes = read_named(root, "pi-state.json", MAX_JOURNAL_BYTES)?;
    lifecycle_wire::parse_strict_json(&bytes).map_err(|_| StoreError::Rejected)
}

/// Require a single record at this coordinate, not merely a matching digest.
fn unique<'a>(rows: &'a Value, at: impl Fn(&Value) -> bool) -> Result<&'a Value, StoreError> {
    let mut matches = rows
        .as_array()
        .ok_or(StoreError::Rejected)?
        .iter()
        .filter(|r| at(r));
    let one = matches.next().ok_or(StoreError::Rejected)?;
    if matches.next().is_some() {
        return Err(StoreError::Rejected);
    }
    Ok(one)
}

pub(super) fn verify(
    payload: &Value,
    capture: &Value,
    journal: &Value,
    expected: &Expected,
    request: u64,
    attempt: u64,
    body_bytes: usize,
) -> Result<Value, StoreError> {
    // The journal spans the job; a head claim must lie within this turn's
    // inclusive Host-recorded request interval.
    let interval = capture["turn"]["requests"]
        .as_array()
        .ok_or(StoreError::Rejected)?;
    let [first, last] = interval.as_slice() else {
        return Err(StoreError::Rejected);
    };
    if !first
        .as_u64()
        .zip(last.as_u64())
        .is_some_and(|(a, b)| a > 0 && (a..=b).contains(&request))
    {
        return Err(StoreError::Rejected);
    }
    let same_attempt = |r: &Value| r["attempt"].as_u64() == Some(attempt);
    let claim = unique(&capture["attempts"], |r| {
        r["request"].as_u64() == Some(request) && same_attempt(r)
    })?;
    let recorded = unique(
        &journal["coding"]["provider_requests"][request.to_string()]["attempts"],
        same_attempt,
    )?;
    for key in ["route", "payer", "disposition"] {
        if !payload[key].as_str().is_some_and(|s| !s.is_empty())
            || payload[key] != claim[key]
            || payload[key] != recorded[key]
        {
            return Err(StoreError::Rejected);
        }
    }
    if !matches!(
        payload["disposition"].as_str(),
        Some("pending" | "refused" | "completed" | "uncertain")
    ) || payload["sha256"] != recorded["request_body_sha256"]
    {
        return Err(StoreError::Rejected);
    }
    let route = unique(&expected.routes, |r| r["id"] == payload["route"])?;
    if route["payer"] != payload["payer"] {
        return Err(StoreError::Rejected);
    }
    let endpoint = route["endpoint"].as_str().ok_or(StoreError::Rejected)?;
    let reference = &payload["request_head"];
    let name = format!("coding-hosted-{request}-{attempt}.request-head.json");
    if reference.as_object().map(|o| o.len()) != Some(2)
        || reference["path"].as_str().map(Path::new)
            != Some(expected.state_dir.join(&name).as_path())
    {
        return Err(StoreError::Rejected);
    }
    let bytes = read_named(&expected.state_dir, &name, MAX_HEAD_BYTES)?;
    let digest = sha(&bytes);
    if reference["sha256"].as_str() != Some(digest.as_str())
        || recorded["request_head_sha256"].as_str() != Some(digest.as_str())
    {
        return Err(StoreError::Rejected);
    }
    let value = lifecycle_wire::parse_strict_json(&bytes).map_err(|_| StoreError::Rejected)?;
    let head: Head = serde_json::from_value(value).map_err(|_| StoreError::Rejected)?;
    if head.schema != SCHEMA
        || head.request != request
        || head.attempt != attempt
        || payload["route"].as_str() != Some(head.route.as_str())
        || head.redacted != REDACTED
        || !check_http(&head.head, endpoint, request, attempt, body_bytes)
    {
        return Err(StoreError::Rejected);
    }
    // No raw head, journal, or path in retained lifecycle metadata. This is
    // an intake verdict on the digested artifact, not a replayable HTTP head.
    Ok(
        json!({"inference_call_id":format!("request-{request}"),"physical_send":attempt,
        "route":payload["route"],"payer":payload["payer"],"disposition":payload["disposition"],
        "body_sha256":payload["sha256"],"head_sha256":digest}),
    )
}

fn check_http(head: &str, endpoint: &str, request: u64, attempt: u64, len: usize) -> bool {
    let (host, access) = match endpoint {
        "https:anthropic-oauth" => ("api.anthropic.com", "Authorization"),
        "https:anthropic-api" => ("api.anthropic.com", "x-api-key"),
        e if e
            .strip_prefix("loopback:")
            .and_then(|p| p.parse::<u16>().ok())
            .is_some_and(|p| p != 0) =>
        {
            ("localhost", "Authorization")
        }
        _ => return false,
    };
    let Some(head) = head.strip_suffix("\r\n\r\n") else {
        return false;
    };
    let mut lines = head.split("\r\n");
    if !matches!(
        lines.next(),
        Some("POST /v1/messages HTTP/1.1" | "POST /v1/messages?beta=true HTTP/1.1")
    ) || lines.next() != Some(format!("Host: {host}").as_str())
        || lines.next() != Some(format!("X-Request-Id: {request}-{attempt}").as_str())
        || lines.next() != Some(format!("Content-Length: {len}").as_str())
        || lines.next() != Some("Connection: close")
        || lines.next() != Some(format!("{access}: {REDACTED}").as_str())
    {
        return false;
    }
    // Pinned Messages allowlist from Host Endpoint::forwarded. Anything
    // else (including another access or transport header) is not v1 evidence.
    // The writer iterates lower-case BTreeMap keys. Client retries are
    // disabled; the client field remains zero across Host physical retries.
    let mut previous = "";
    let mut client_retry_zero = false;
    lines.all(|line| {
        let Some((key, value)) = line.split_once(": ") else {
            return false;
        };
        if key <= previous || value.len() > 4096 || value.contains(['\r', '\n']) {
            return false;
        }
        previous = key;
        if key == "x-stainless-retry-count" {
            client_retry_zero = value == "0";
        }
        matches!(
            key,
            "accept"
                | "content-type"
                | "anthropic-version"
                | "anthropic-beta"
                | "anthropic-dangerous-direct-browser-access"
                | "user-agent"
                | "x-app"
                | "x-client-app"
                | "x-claude-code-session-id"
                | "x-stainless-arch"
                | "x-stainless-lang"
                | "x-stainless-os"
                | "x-stainless-package-version"
                | "x-stainless-retry-count"
                | "x-stainless-runtime"
                | "x-stainless-runtime-version"
                | "x-stainless-timeout"
        )
    }) && client_retry_zero
}
