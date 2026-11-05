//! Strict public contract for coding-job owner results, checked at the client boundary.
//!
//! Variants mirror `schemas/coding-job-v1.json`. Adapter-owned fields are typed and unknown fields
//! are rejected; Host-owned payload (`receipt`, collector records) stays opaque beyond the
//! identity fields that tie it to the original operation.
// Typed fields exist to be rejected on mismatch; several are never read afterwards.
#![allow(dead_code)]
use serde::Deserialize;
use serde_json::Value;

const SCHEMA: &str = "work-engine.coding-job.v1";

fn hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}
fn id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}
fn uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cleanup {
    run: Option<Cleaned>,
    reconcile: Option<Cleaned>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Cleaned {
    OwnedProcessGroupCollected,
    NotEstablished,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum State {
    BoundNotEntered,
    Collecting,
    Reconciling,
    Collected,
    EntryUncertain,
    HostStateUnreadable,
}
#[derive(Deserialize)]
enum Entry {
    #[serde(rename = "collector_started")]
    Started,
    #[serde(rename = "existing_operation")]
    Existing,
}
#[derive(Deserialize)]
enum Settlement {
    #[serde(rename = "not_established_by_submission")]
    NotEstablished,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    schema: String,
    operation_id: String,
    request_id: String,
    authorization_id: String,
    job_id: String,
    state_dir: String,
    job_sha256: String,
    expected_host_input_digest: String,
    host_input_digest: Option<String>,
    state: State,
    entered_possible: bool,
    collector_live: bool,
    host_phase: Option<String>,
    host_removed: Option<bool>,
    host_receipt_id: Option<String>,
    collector_run: Option<serde_json::Map<String, Value>>,
    collector_reconcile: Option<serde_json::Map<String, Value>>,
    process_cleanup: Cleanup,
    limitations: Vec<String>,
    settlement: Settlement,
    /// Present on submission only.
    entry: Option<Entry>,
}

impl Operation {
    fn check(&self, arguments: &Value, submission: bool) -> Result<(), &'static str> {
        if self.schema != SCHEMA
            || !hex64(&self.operation_id)
            || !uuid(&self.request_id)
            || !id(&self.authorization_id)
            || !id(&self.job_id)
            || !self.state_dir.starts_with('/')
            || !hex64(&self.job_sha256)
            || !hex64(&self.expected_host_input_digest)
            || self.host_input_digest.as_ref().is_some_and(|d| {
                !hex64(d) || *d != self.expected_host_input_digest
            })
            || self.host_receipt_id.as_ref().is_some_and(|r| !hex64(r))
            || self.entry.is_some() != submission
            // The response must describe the operation the caller named.
            || arguments["request_id"] != self.request_id.as_str()
            || arguments["authorization_id"] != self.authorization_id.as_str()
            // State, entry and liveness must not contradict each other.
            || (matches!(self.state, State::BoundNotEntered) && self.entered_possible)
            || matches!(self.state, State::Collecting) != self.collector_live
            || (matches!(self.state, State::Collected) && self.host_removed != Some(true))
        {
            return Err("invalid_response");
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cancel {
    schema: String,
    operation_id: String,
    request_id: String,
    cancellation: Cancellation,
    cancellation_established: bool,
    container_closure_established: bool,
    settlement: NotClaimed,
    next: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Cancellation {
    Requested,
    RequestNotRecorded,
}
#[derive(Deserialize)]
enum NotClaimed {
    #[serde(rename = "not_claimed")]
    NotClaimed,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Unavailable {
    schema: String,
    result: Availability,
    operation: Operation,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Availability {
    NotAvailable,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HostOutput {
    action: HostAction,
    exit_code: Value,
    note: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum HostAction {
    Run,
    Reconcile,
}
#[derive(Deserialize)]
enum Available {
    #[serde(rename = "available")]
    Available,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AvailableResult {
    schema: String,
    result: Available,
    operation: Operation,
    terminal: serde_json::Map<String, Value>,
    validation: serde_json::Map<String, Value>,
    cleanup: serde_json::Map<String, Value>,
    acceptance: Value,
    domain_effect_settlement: Value,
    receipt_id: String,
    input_digest: String,
    receipt: serde_json::Map<String, Value>,
    host_output: HostOutput,
}

/// Validates one owner result for `operation`. `arguments` are the caller's own request arguments.
pub fn validate(
    operation: &str,
    arguments: &Value,
    owner_result: &Value,
) -> std::result::Result<(), &'static str> {
    let bad = |_| "invalid_response";
    match operation {
        "submit_coding_job" | "read_coding_job_operation" => {
            let o: Operation = serde_json::from_value(owner_result.clone()).map_err(bad)?;
            o.check(arguments, operation == "submit_coding_job")
        }
        "cancel_coding_job" => {
            let c: Cancel = serde_json::from_value(owner_result.clone()).map_err(bad)?;
            // Cancellation is a request; closure is established only by the operation and Host receipt.
            if c.schema != SCHEMA
                || !hex64(&c.operation_id)
                || c.request_id != arguments["request_id"]
                || c.cancellation_established
                || c.container_closure_established
                || c.next.is_empty()
            {
                return Err("invalid_response");
            }
            Ok(())
        }
        "read_coding_job_result" => match owner_result["result"].as_str() {
            Some("not_available") => {
                let u: Unavailable = serde_json::from_value(owner_result.clone()).map_err(bad)?;
                if u.schema != SCHEMA || !matches!(u.result, Availability::NotAvailable) {
                    return Err("invalid_response");
                }
                u.operation.check(arguments, false)
            }
            Some("available") => {
                let r: AvailableResult =
                    serde_json::from_value(owner_result.clone()).map_err(bad)?;
                r.operation.check(arguments, false)?;
                if r.schema != SCHEMA
                    || !matches!(r.result, Available::Available)
                    || !hex64(&r.receipt_id)
                    || r.input_digest != r.operation.expected_host_input_digest
                    // The wrapper ID must be the one the operation's Host journal recorded, if any.
                    || r.operation.host_receipt_id.as_ref().is_some_and(|j| *j != r.receipt_id)
                    || r.receipt.get("input_digest").and_then(Value::as_str)
                        != Some(r.input_digest.as_str())
                    || r.receipt.get("job_id").and_then(Value::as_str)
                        != Some(r.operation.job_id.as_str())
                    || r.host_output.note.is_empty()
                    || !(r.host_output.exit_code.is_number()
                        || r.host_output.exit_code == "not_established")
                    || !r.acceptance.is_string()
                    || !r.domain_effect_settlement.is_string()
                {
                    return Err("invalid_response");
                }
                Ok(())
            }
            _ => Err("invalid_response"),
        },
        _ => Err("invalid_request"),
    }
}
