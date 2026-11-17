//! Per-turn capture: retained request payloads, the calls bundle joined from
//! response streams and the relay, and an eligibility verdict with reasons.

use crate::driver::Job;
use crate::proxy::RequestRecord;
use crate::spec;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

pub const CAPTURE_SCHEMA: &str = "work-engine.claude-host-capture.v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RelayCall {
    pub seq: u64,
    pub tool_use_id: String,
    pub tool: String,
    pub arguments: Value,
    pub text: String,
    pub is_error: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Payload {
    pub inference_call_id: String,
    pub physical_send: u64,
    pub representation: String,
    pub original_codec: String,
    pub original_sha256: String,
    pub length: usize,
    pub file: String,
    pub response_id: Option<String>,
    pub response_sha256: Option<String>,
    pub stop_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Call {
    pub call_id: String,
    pub inference_call_id: String,
    pub physical_send: u64,
    pub response_id: Option<String>,
    pub namespace: String,
    pub tool: String,
    pub arguments: Value,
    pub result_sha256: String,
    pub is_error: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Capture {
    pub schema: String,
    pub model: String,
    pub tools_digest: String,
    pub payloads: Vec<Payload>,
    pub calls: Vec<Call>,
    pub requests: Vec<RequestRecord>,
    pub relay: Vec<RelayCall>,
    pub stream_sha256: Option<String>,
    pub relay_log_sha256: Option<String>,
    pub result_subtype: Option<String>,
    pub final_text: Option<String>,
    pub exit_code: Option<i32>,
    pub eligible: bool,
    pub outcome: String,
    pub reasons: Vec<String>,
}

fn file_sha(path: &Path) -> Option<String> {
    std::fs::read(path).ok().map(|b| spec::sha256_hex(&b))
}

#[allow(clippy::too_many_arguments)]
pub fn build(
    out: &Path,
    job: &Job,
    tools_digest: &str,
    records: Vec<RequestRecord>,
    relay: Vec<RelayCall>,
    result_event: Option<Value>,
    failure: Option<String>,
    exit_code: Option<i32>,
) -> Capture {
    let mut reasons = Vec::new();
    if let Some(f) = failure {
        reasons.push(format!("turn failure: {f}"));
    }
    let mut payloads = Vec::new();
    for r in &records {
        if r.refused.is_some() {
            reasons.push(format!(
                "request {} refused: {}",
                r.n,
                r.refused.clone().unwrap_or_default()
            ));
            continue;
        }
        if let Some(f) = &r.failure {
            reasons.push(format!("request {} failed: {f}", r.n));
        }
        if !r.forwarded {
            continue;
        }
        if !r.complete || r.status != Some(200) {
            reasons.push(format!(
                "request {} response not complete (status {:?})",
                r.n, r.status
            ));
        }
        let file = format!("requests/{:04}.body", r.n);
        if file_sha(&out.join(&file)) != r.body_sha256 || r.body_sha256.is_none() {
            reasons.push(format!("request {} retained body digest mismatch", r.n));
        }
        let resp = out.join(format!("requests/{:04}.response", r.n));
        if r.response_sha256.is_some() && file_sha(&resp) != r.response_sha256 {
            reasons.push(format!("request {} retained response digest mismatch", r.n));
        }
        payloads.push(Payload {
            inference_call_id: format!("request-{}", r.n),
            physical_send: 1,
            representation: "wire".into(),
            original_codec: "identity".into(),
            original_sha256: r.body_sha256.clone().unwrap_or_default(),
            length: r.body_len,
            file,
            response_id: r.message_id.clone(),
            response_sha256: r.response_sha256.clone(),
            stop_reason: r.stop_reason.clone(),
        });
    }
    if payloads.is_empty() {
        reasons.push("no request was forwarded".into());
    }
    // Every relayed call must match exactly one tool_use in a retained response,
    // and every tool_use must have been relayed.
    let mut calls = Vec::new();
    for c in &relay {
        let name = format!("mcp__{}__{}", spec::SERVER, c.tool);
        let matches: Vec<_> = records
            .iter()
            .filter(|r| r.forwarded)
            .flat_map(|r| r.tool_uses.iter().map(move |t| (r, t)))
            .filter(|(_, t)| t.0 == c.tool_use_id)
            .collect();
        if matches.len() != 1 {
            reasons.push(format!(
                "relayed call {} not matched to exactly one tool_use",
                c.seq
            ));
            continue;
        }
        let (r, (id, model_name, input)) = matches[0];
        if *model_name != name || *input != c.arguments {
            reasons.push(format!("relayed call {} differs from its tool_use", c.seq));
            continue;
        }
        let Some((namespace, tool)) = spec::lifecycle_identity(model_name) else {
            reasons.push(format!("relayed call {} has no lifecycle identity", c.seq));
            continue;
        };
        calls.push(Call {
            call_id: id.clone(),
            inference_call_id: format!("request-{}", r.n),
            physical_send: 1,
            response_id: r.message_id.clone(),
            namespace: namespace.into(),
            tool,
            arguments: c.arguments.clone(),
            result_sha256: spec::sha256_hex(c.text.as_bytes()),
            is_error: c.is_error,
        });
    }
    for r in records.iter().filter(|r| r.forwarded) {
        for (id, _, _) in &r.tool_uses {
            if !relay.iter().any(|c| &c.tool_use_id == id) {
                reasons.push(format!("tool_use {id} in request {} was not relayed", r.n));
            }
        }
    }
    if let Some(last) = records.iter().rev().find(|r| r.forwarded)
        && last.stop_reason.as_deref() != Some("end_turn")
    {
        reasons.push("last response did not end the turn".into());
    }
    let subtype = result_event
        .as_ref()
        .and_then(|v| v["subtype"].as_str())
        .map(str::to_owned);
    let is_error = result_event
        .as_ref()
        .is_none_or(|v| v["is_error"].as_bool().unwrap_or(true));
    if subtype.as_deref() != Some("success") || is_error {
        reasons.push(format!("claude result not success ({subtype:?})"));
    }
    let eligible = reasons.is_empty();
    Capture {
        schema: CAPTURE_SCHEMA.into(),
        model: job.model.clone(),
        tools_digest: tools_digest.into(),
        payloads,
        calls,
        requests: records,
        relay,
        stream_sha256: file_sha(&out.join("stream.jsonl")),
        relay_log_sha256: file_sha(&out.join("relay.jsonl")),
        result_subtype: subtype,
        final_text: result_event
            .as_ref()
            .and_then(|v| v["result"].as_str())
            .map(str::to_owned),
        exit_code,
        eligible,
        outcome: if eligible {
            "checked_completed".into()
        } else {
            reasons[0].clone()
        },
        reasons,
    }
}
