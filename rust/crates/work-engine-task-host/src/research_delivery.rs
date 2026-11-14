//! Checked composition of retained native delivery and original task-owner evidence.
//! These values have private construction and never confer research/execution authority.
use crate::{HostError, exchange};
use base64::Engine;
use retained_task_workflow::{
    TaskError, TaskStore,
    research_admission::{
        BasisBundle, ResearchOriginalEvidence, ResearchProducer, RetainedArtifact, VerifiedBasis,
        research_digest,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
const LIMIT: usize = 8 * 1024 * 1024;
fn sha(b: &[u8]) -> String {
    retained_task_workflow::sha256(b)
}
fn refuse<T>() -> Result<T, HostError> {
    Err(TaskError::Unavailable.into())
}
fn delivery_error(status: &str, stage: &str, reason: &str, sources: Value) -> HostError {
    HostError::ResearchDelivery(
        json!({"version":1,"schema":"research-consumer-delivery-v1","status":status,"stage":stage,"reason":reason,"sources":sources,"scope":"delivery attribution only; no research, model-use or execution authority"}),
    )
}
fn scoped_error(error: HostError, status: &str, stage: &str, sources: Value) -> HostError {
    if matches!(error, HostError::ResearchDelivery(_)) {
        return error;
    }
    delivery_error(status, stage, &error.to_string(), sources)
}
fn strict(b: &[u8]) -> Result<Value, HostError> {
    lifecycle_wire::parse_strict_json(b).map_err(|_| HostError::Configuration)
}
use lifecycle_wire::NativeCausalChunkV1 as Chunk;
/// Acquired through the scoped actor read; no caller artifact path or operation locator.
pub(crate) struct RetainedDelivery {
    pub(super) bytes: Vec<u8>,
    pub(super) value: Value,
}
pub(crate) fn acquire(
    path: &Path,
    task: &str,
    subject: &str,
) -> Result<RetainedDelivery, HostError> {
    let mut bytes = Vec::new();
    let mut digest = None;
    let mut operation = None;
    let mut length = None;
    let mut store = None;
    loop {
        let value = exchange(
            path,
            &json!({"op":"native_causal_read","task_id":task,"subject_id":subject,"offset":bytes.len(),"expected_sha256":digest}),
        ).map_err(|e|scoped_error(e,"unavailable","actor-read",json!({"task":task,"subject":subject,"offset":bytes.len()})))?;
        let chunk: Chunk = serde_json::from_value(value).map_err(|e|delivery_error("unknown","actor-read","unsupported or malformed causal chunk response",json!({"task":task,"subject":subject,"offset":bytes.len(),"encoding_error":e.to_string()})))?;
        if chunk.version != 1
            || chunk.encoding != "json-utf8-v1"
            || chunk.length == 0
            || chunk.length > LIMIT
            || chunk.store_id.is_empty()
            || store.as_ref().is_some_and(|s| s != &chunk.store_id)
            || chunk.offset != bytes.len()
            || length.is_some_and(|l| l != chunk.length)
            || digest.as_ref().is_some_and(|d| d != &chunk.content_sha256)
            || operation.as_ref().is_some_and(|o| o != &chunk.operation_id)
        {
            return Err(delivery_error(
                "mismatch",
                "actor-read",
                "chunk identity, offset, length, encoding or content digest mismatch",
                json!({"task":task,"subject":subject,"offset":bytes.len(),"operation":operation}),
            ));
        }
        let data = base64::engine::general_purpose::STANDARD
            .decode(chunk.bytes_base64)
            .map_err(|_| delivery_error("unknown", "actor-read", "invalid-base64-chunk", json!({"task":task,"subject":subject,"offset":bytes.len(),"operation":operation})))?;
        if data.is_empty() || data.len() != (chunk.length - chunk.offset).min(48 * 1024) {
            return Err(delivery_error(
                "mismatch",
                "actor-read",
                "chunk identity, offset, length, encoding or content digest mismatch",
                json!({"task":task,"subject":subject,"offset":bytes.len(),"operation":operation}),
            ));
        }
        length = Some(chunk.length);
        digest = Some(chunk.content_sha256);
        operation = Some(chunk.operation_id);
        store = Some(chunk.store_id);
        bytes.extend(data);
        if bytes.len() == chunk.length {
            break;
        }
    }
    if Some(sha(&bytes)) != digest {
        return Err(delivery_error(
            "mismatch",
            "actor-read",
            "chunk identity, offset, length, encoding or content digest mismatch",
            json!({"task":task,"subject":subject,"offset":bytes.len(),"operation":operation}),
        ));
    }
    let value = strict(&bytes).map_err(|e| {
        scoped_error(
            e,
            "unknown",
            "actor-read-json",
            json!({"task":task,"subject":subject,"operation":operation,"content_sha256":digest}),
        )
    })?;
    if value["version"] != 1
        || value["causal"]["version"] != 1
        || value["causal"]["operation_id"].as_str() != operation.as_deref()
    {
        return Err(delivery_error(
            "mismatch",
            "actor-read",
            "chunk identity, offset, length, encoding or content digest mismatch",
            json!({"task":task,"subject":subject,"offset":bytes.len(),"operation":operation}),
        ));
    }
    Ok(RetainedDelivery { bytes, value })
}
#[derive(Serialize)]
struct Span {
    inference_call_id: String,
    physical_send: u64,
    json_pointer: String,
    message: usize,
    tool_call: Option<String>,
    utf8_start: usize,
    utf8_end: usize,
    transformation: &'static str,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<Value>,
}
struct VerifiedConsumerDelivery {
    projection: Vec<u8>,
    artifacts: Vec<RetainedArtifact>,
    context: String,
    attempt: String,
}
struct VerifiedRecommendation {
    projection: Vec<u8>,
    action: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recommendation {
    dependencies: std::collections::BTreeMap<String, String>,
    text: String,
}
fn artifact(id: &str, bytes: Vec<u8>) -> RetainedArtifact {
    RetainedArtifact {
        id: id.into(),
        digest: sha(&bytes),
        bytes,
    }
}
fn byte_vec(v: &Value) -> Result<Vec<u8>, HostError> {
    Ok(serde_json::from_value(v.clone())?)
}
fn text_projection(
    request: &Value,
    inference: &str,
    send: u64,
) -> Result<(Vec<u8>, Vec<Span>), HostError> {
    let mut out = Vec::new();
    let mut spans = Vec::new();
    let input = request["input"]
        .as_array()
        .ok_or(HostError::Configuration)?;
    for (i, item) in input.iter().enumerate() {
        let mut texts = Vec::new();
        if let Some(content) = item["content"].as_array() {
            for (j, c) in content.iter().enumerate() {
                if matches!(c["type"].as_str(), Some("input_text" | "output_text")) {
                    texts.push((
                        format!("/input/{i}/content/{j}/text"),
                        c["text"].as_str().ok_or(HostError::Configuration)?,
                    ));
                }
            }
        } else if let Some(content) = item["content"].as_str() {
            texts.push((format!("/input/{i}/content"), content));
        }
        if let Some(output) = item["output"].as_str() {
            texts.push((format!("/input/{i}/output"), output));
        }
        for (pointer, text) in texts {
            if !out.is_empty() {
                out.push(b'\n');
            }
            let start = out.len();
            out.extend_from_slice(text.as_bytes());
            spans.push(Span {
                inference_call_id: inference.into(),
                physical_send: send,
                json_pointer: pointer,
                message: i,
                tool_call: item["call_id"].as_str().map(str::to_owned),
                utf8_start: start,
                utf8_end: out.len(),
                transformation: "strict-json-unescape-utf8-v1",
                text: text.into(),
                snapshot: None,
            });
        }
    }
    if out.is_empty() {
        return refuse();
    }
    Ok((out, spans))
}
// A snapshot read is model-visible as a JSON string in a function output.
// Match its original generated call, then decode and join complete byte ranges.
// The returned projection is not the request wire representation.
fn snapshot_projection(
    request: &Value,
    inference: &str,
    send: u64,
    snapshot_id: &str,
    expected: &[u8],
) -> Result<Option<(Vec<u8>, Vec<Span>)>, HostError> {
    use std::collections::{BTreeMap, BTreeSet};
    let input = request["input"]
        .as_array()
        .ok_or(HostError::Configuration)?;
    let mut chunks = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let mut member = None;
    for (i, output) in input.iter().enumerate() {
        if output["type"] != "function_call_output" {
            continue;
        }
        let call_id = output["call_id"].as_str().ok_or(HostError::Configuration)?;
        let matching = input
            .iter()
            .enumerate()
            .filter(|(_, c)| c["type"] == "function_call" && c["call_id"] == call_id)
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return refuse();
        }
        let (call_index, call) = matching[0];
        if call["namespace"] != "workspace" || call["name"] != "snapshot_read" {
            continue;
        }
        if call_index >= i || !ids.insert(call_id) {
            return refuse();
        }
        let args = strict(
            call["arguments"]
                .as_str()
                .ok_or(HostError::Configuration)?
                .as_bytes(),
        )?;
        let raw = output["output"].as_str().ok_or(HostError::Configuration)?;
        let value = strict(raw.as_bytes())?;
        let snapshot = value["snapshot_id"]
            .as_str()
            .ok_or(HostError::Configuration)?;
        let name = value["member_id"]
            .as_str()
            .ok_or(HostError::Configuration)?;
        let offset = value["offset"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or(HostError::Configuration)?;
        let length = value["length"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or(HostError::Configuration)?;
        if snapshot != snapshot_id
            || name.is_empty()
            || member.as_ref().is_some_and(|m| m != name)
            || args["snapshot_id"] != snapshot
            || args["member_id"] != name
            || args["offset"] != offset
            || args["length"] != length
            || length == 0
            || length > 65536
            || offset
                .checked_add(length)
                .is_none_or(|end| end > expected.len())
        {
            return refuse();
        }
        member = Some(name.to_owned());
        let (bytes, transformation, field) = match value["encoding"].as_str() {
            Some("utf8") => (
                value["text"]
                    .as_str()
                    .ok_or(HostError::Configuration)?
                    .as_bytes()
                    .to_vec(),
                "strict-json-unescape-utf8-snapshot-v1",
                "text",
            ),
            Some("hex") => {
                let hex = value["bytes_hex"]
                    .as_str()
                    .ok_or(HostError::Configuration)?;
                if hex.len() != length * 2
                    || !hex
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return refuse();
                }
                let bytes = hex
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect::<Vec<_>>();
                (bytes, "strict-json-hex-snapshot-v1", "bytes_hex")
            }
            _ => return refuse(),
        };
        if bytes.len() != length
            || value["sha256"] != sha(&bytes)
            || bytes != expected[offset..offset + length]
        {
            return refuse();
        }
        let span = Span {
            inference_call_id: inference.into(),
            physical_send: send,
            json_pointer: format!("/input/{i}/output"),
            message: i,
            tool_call: Some(call_id.into()),
            utf8_start: offset,
            utf8_end: offset + length,
            transformation,
            text: value[field].as_str().unwrap().into(),
            snapshot: Some(
                json!({"snapshot_id":snapshot,"member_id":name,"offset":offset,"length":length,
                "sha256":sha(&bytes),"output_item_id":output["id"],"call_item_id":call["id"],
                "arguments_pointer":format!("/input/{call_index}/arguments"),"decoded_pointer":format!("/{field}"),
                "source_utf8_start":0,"source_utf8_end":value[field].as_str().unwrap().len()}),
            ),
        };
        if chunks.insert(offset, (bytes, span)).is_some() {
            return refuse();
        }
    }
    if chunks.is_empty() {
        return Ok(None);
    }
    let mut out = Vec::new();
    let mut spans = Vec::new();
    for (offset, (bytes, span)) in chunks {
        if offset != out.len() {
            return refuse();
        }
        out.extend(bytes);
        spans.push(span);
    }
    if out != expected || std::str::from_utf8(&out).is_err() {
        return refuse();
    }
    Ok(Some((out, spans)))
}
fn verify_delivery(
    retained: RetainedDelivery,
    original: &ResearchOriginalEvidence,
    request: &[u8],
    card: &[u8],
    reconstruction: &[u8],
) -> Result<VerifiedConsumerDelivery, HostError> {
    let sources = json!({"request_sha256":sha(request),"card_sha256":sha(card),"reconstruction_sha256":sha(reconstruction),"retained_sha256":sha(&retained.bytes),"advisory_task":original.advisory().task_id,"advisory_subject":original.advisory().subject_id,"operation_id":original.advisory().result.as_ref().map(|r|&r.operation_id)});
    let reconstruction_bytes = reconstruction;
    let reconstruction_value = strict(reconstruction)?;
    let reconstruction: research_register::card_host::Reconstruction =
        serde_json::from_value(reconstruction_value)?;
    let reconstructed = research_register::card_host::reconstruct(
        &reconstruction,
        &sha(&serde_json::to_vec(&reconstruction)?),
    )
    .map_err(|_| HostError::Configuration)?;
    if reconstructed != card || reconstruction.request.current.request_sha256 != sha(request) {
        return Err(delivery_error(
            "mismatch",
            "native-delivery",
            "reconstruction-card-or-request-digest-mismatch",
            sources.clone(),
        ));
    }
    validate_original_request(request, &reconstruction)?;
    let envelope =
        research_register::card_wire::decode(card).map_err(|_| HostError::Configuration)?;
    if envelope.outcome != "card"
        || envelope.request.as_ref() != Some(&reconstruction.request.current)
        || envelope.card.as_ref().is_none_or(|c| c.coverage.truncated)
    {
        return Err(delivery_error(
            "refused",
            "native-delivery",
            "card-refused-truncated-or-request-mismatch",
            sources.clone(),
        ));
    }
    let causal = &retained.value["causal"];
    let input = &retained.value["input"];
    let result = original
        .advisory()
        .result
        .as_ref()
        .ok_or(TaskError::Unavailable)?;
    let entry = strict(&byte_vec(&causal["entry"])?)?;
    let ingress = strict(&byte_vec(&causal["ingress"])?)?;
    let args = &ingress["params"]["arguments"];
    let owner_commit = strict(&byte_vec(&causal["owner_commit"])?)?;
    let local_commit = strict(original.operation_commit())?;
    if input["eligible"] != true {
        let status = if input["outcome"] == "unknown" {
            "unknown"
        } else {
            "refused"
        };
        return Err(delivery_error(
            status,
            "native-delivery",
            "native-input-not-eligible-checked-completed",
            json!({"retained":sources,"outcome":input["outcome"]}),
        ));
    }
    if causal["operation_id"] != result.operation_id
        || owner_commit["operation_id"] != result.operation_id
        || owner_commit["task_id"] != original.advisory().task_id
        || owner_commit["sha256"] != local_commit["digest"]
        || owner_commit["owner_revision"] != local_commit["owner_revision"]
        || entry["task_id"] != original.advisory().task_id
        || entry["subject_id"] != original.advisory().subject_id
        || causal["invocation_id"] != input["invocation_id"]
        || causal["session_id"] != input["session_id"]
        || args["result_id"] != result.result_id
        || args["body"] != result.body
        || args["attribution"] != result.attribution
        || args["source_refs"] != serde_json::to_value(&result.source_refs)?
    {
        return Err(delivery_error(
            "mismatch",
            "native-delivery",
            "original-owner-ingress-commit-result-join-mismatch",
            sources.clone(),
        ));
    }
    let bundle = strict(&byte_vec(&causal["bundle"])?)?;
    let calls = bundle["calls"].as_array().ok_or(HostError::Configuration)?;
    let calls = calls
        .iter()
        .filter(|c| c["call_id"] == causal["call_id"])
        .collect::<Vec<_>>();
    if calls.len() != 1 {
        return Err(delivery_error(
            "mismatch",
            "native-delivery",
            "publication-call-not-unique",
            sources.clone(),
        ));
    }
    let call = calls[0];
    if call["arguments"] != *args
        || call["namespace"] != "task"
        || call["tool"] != "result_publish"
        || ingress["params"]["callId"] != causal["call_id"]
        || ingress["params"]["threadId"] != input["thread_id"]
        || ingress["params"]["turnId"] != input["turn_id"]
    {
        return Err(delivery_error(
            "mismatch",
            "native-delivery",
            "publication-ingress-call-thread-turn-mismatch",
            sources.clone(),
        ));
    }
    let inference = call["inference_call_id"]
        .as_str()
        .ok_or(HostError::Configuration)?;
    let send = call["physical_send"]
        .as_u64()
        .ok_or(HostError::Configuration)?;
    let mut total = request.len() + card.len() + reconstruction_bytes.len() + result.body.len();
    let payloads = input["payloads"]
        .as_array()
        .ok_or(HostError::Configuration)?;
    let mut artifacts = Vec::new();
    let mut selected = None;
    for (i, p) in payloads.iter().enumerate() {
        let bytes = byte_vec(&p["bytes"])?;
        total = total.checked_add(bytes.len()).ok_or(TaskError::Capacity)?;
        if total > LIMIT || p["original_sha256"] != sha(&bytes) {
            return Err(delivery_error(
                "refused",
                "native-delivery",
                "combined-payload-capacity-or-original-digest-failed",
                sources.clone(),
            ));
        }
        if p["inference_call_id"] == inference
            && p["physical_send"] == send
            && p["representation"] == "wire"
        {
            let decoded = match p["original_codec"].as_str() {
                Some("identity") => bytes.clone(),
                Some("zstd") => {
                    use std::io::Read;
                    let mut decoder = zstd::stream::read::Decoder::new(bytes.as_slice())?;
                    decoder.window_log_max(23)?;
                    let mut out = Vec::new();
                    decoder.take(LIMIT as u64 + 1).read_to_end(&mut out)?;
                    if out.len() > LIMIT {
                        return Err(delivery_error(
                            "refused",
                            "native-delivery",
                            "decompressed-payload-capacity-exceeded",
                            sources.clone(),
                        ));
                    }
                    out
                }
                _ => {
                    return Err(delivery_error(
                        "refused",
                        "native-delivery",
                        "unsupported-wire-codec",
                        json!({"retained":sources,"payload":i,"codec":p["original_codec"]}),
                    ));
                }
            };
            if selected.replace(decoded).is_some() {
                return Err(delivery_error(
                    "mismatch",
                    "native-delivery",
                    "multiple-selected-wire-payloads",
                    sources.clone(),
                ));
            }
        }
        artifacts.push(artifact(&format!("native-input-{i}"), bytes));
    }
    let decoded = selected.ok_or_else(|| {
        delivery_error(
            "unavailable",
            "native-delivery",
            "selected-wire-payload-missing",
            sources.clone(),
        )
    })?;
    for p in payloads.iter().filter(|p| {
        p["inference_call_id"] == inference
            && p["physical_send"] == send
            && p["representation"] == "decoded"
    }) {
        if byte_vec(&p["bytes"])? != decoded {
            return Err(delivery_error(
                "mismatch",
                "native-delivery",
                "decoded-payload-differs-from-wire",
                sources.clone(),
            ));
        }
    }
    let request_value = strict(&decoded)
        .map_err(|e| scoped_error(e, "refused", "strict-wire-json", sources.clone()))?;
    if request_value
        .get("previous_response_id")
        .is_some_and(|v| !v.is_null())
    {
        return Err(delivery_error(
            "refused",
            "native-delivery",
            "stateful-previous-response-request-unsupported",
            sources.clone(),
        ));
    }
    let mut expected = request.to_vec();
    expected.push(b'\n');
    expected.extend_from_slice(card);
    expected.push(b'\n');
    expected.extend_from_slice(reconstruction_bytes);
    let (projection, spans) = match snapshot_projection(
        &request_value,
        inference,
        send,
        &original.advisory().snapshot_ref,
        &expected,
    )
    .map_err(|_| {
        delivery_error(
            "mismatch",
            "snapshot-projection",
            "snapshot-call-identity-content-or-range-mismatch",
            sources.clone(),
        )
    })? {
        Some(value) => value,
        None => text_projection(&request_value, inference, send).map_err(|_| {
            delivery_error(
                "mismatch",
                "text-projection",
                "model-visible-text-missing-or-malformed",
                sources.clone(),
            )
        })?,
    };
    let visible = std::str::from_utf8(&projection).map_err(|_| HostError::Configuration)?;
    // Exact decoded model-visible strings, not digest echoes or escaped wire substrings.
    for required in [request, card, reconstruction_bytes] {
        let literal = std::str::from_utf8(required).map_err(|_| HostError::Configuration)?;
        if visible.match_indices(literal).count() != 1 {
            return Err(delivery_error(
                "mismatch",
                "native-delivery",
                "full-request-card-reconstruction-not-uniquely-visible",
                sources.clone(),
            ));
        }
    }
    artifacts.push(artifact("native-causal-delivery", retained.bytes));
    artifacts.push(artifact("research-request", request.to_vec()));
    artifacts.push(artifact("research-card", card.to_vec()));
    artifacts.push(artifact(
        "research-reconstruction",
        reconstruction_bytes.to_vec(),
    ));
    artifacts.push(artifact(
        "research-decoding-map",
        serde_json::to_vec(&spans)?,
    ));
    artifacts.push(artifact("research-consumer-delivery-v1",serde_json::to_vec(&json!({"version":1,"schema":"research-consumer-delivery-v1","status":"complete","reason":"full-payload-card-explicit-cause-original-owner-joins-checked","sources":sources,"scope":"explicit-cause delivery attribution; no model-use or authority claim","operation_id":result.operation_id,"inference_call_id":inference,"physical_send":send,"response_id":call["response_id"],"request_sha256":sha(request),"card_sha256":sha(card)}))?));
    let context = input["context_id"]
        .as_str()
        .ok_or(HostError::Configuration)?
        .into();
    let attempt = input["attempt_id"]
        .as_str()
        .ok_or(HostError::Configuration)?
        .into();
    Ok(VerifiedConsumerDelivery {
        projection,
        artifacts,
        context,
        attempt,
    })
}
fn verify_recommendation(
    original: &ResearchOriginalEvidence,
    basis: &BasisBundle,
) -> Result<VerifiedRecommendation, HostError> {
    let result = original
        .advisory()
        .result
        .as_ref()
        .ok_or(TaskError::Unavailable)?;
    let value = strict(result.body.as_bytes())?;
    let recommendation: Recommendation = serde_json::from_value(value)?;
    // The existing owner codec is {dependencies,text}. Retain the complete
    // original prose, including qualifications, rather than imposing a new
    // recommendation schema on an already published owner result.
    if recommendation.dependencies != basis.dependencies
        || !matches!(
            basis.recommendation.action.as_str(),
            "audit" | "propose" | "clarify" | "refuse"
        )
        || recommendation.text.is_empty()
    {
        return refuse();
    }
    Ok(VerifiedRecommendation {
        projection: serde_json::to_vec(
            &json!({"dependencies":recommendation.dependencies,"text":recommendation.text}),
        )?,
        action: basis.recommendation.action.clone(),
    })
}
/// Private controlled composition. Caller provides the original owner snapshot
/// used during validation; minting refuses if any owner row has since changed.
#[cfg(feature = "controlled-proof")]
pub(super) fn checked_unminted_basis(
    mut basis: BasisBundle,
    original: ResearchOriginalEvidence,
    retained: RetainedDelivery,
    request: &[u8],
    card: &[u8],
    reconstruction: &[u8],
) -> Result<CheckedHostBasis, HostError> {
    let sources = json!({"advisory_task":basis.delivery.advisory_task,"advisory_subject":basis.delivery.advisory_subject,"request_sha256":sha(request),"card_sha256":sha(card)});
    let recommendation = verify_recommendation(&original, &basis)
        .map_err(|e| scoped_error(e, "mismatch", "original-recommendation", sources.clone()))?;
    let delivery = verify_delivery(retained, &original, request, card, reconstruction)
        .map_err(|e| scoped_error(e, "unknown", "delivery-decoding", sources.clone()))?;
    validate_basis_target(&basis, request, reconstruction)
        .map_err(|e| scoped_error(e, "mismatch", "request-target", sources.clone()))?;
    if basis.delivery.context != delivery.context
        || basis.delivery.attempt != delivery.attempt
        || basis.delivery.advisory_task != original.advisory().task_id
        || basis.delivery.advisory_subject != original.advisory().subject_id
        || basis.delivery.request_digest != sha(request)
        || basis.delivery.card_digest != sha(card)
        || basis.key.request_digest != sha(request)
        || basis.reconstruction_digest != sha(reconstruction)
    {
        return Err(delivery_error(
            "mismatch",
            "checked-basis",
            "basis-original-execution-task-request-card-or-reconstruction-mismatch",
            sources.clone(),
        ));
    }
    basis.artifacts.retain(|a| {
        a.id != basis.delivery.input_artifact
            && a.id != basis.recommendation.result_artifact
            && !matches!(
                a.id.as_str(),
                "research-request"
                    | "research-card"
                    | "research-reconstruction"
                    | "research-decoding-map"
                    | "research-consumer-delivery-v1"
                    | "native-causal-delivery"
                    | "research-original-result"
                    | "research-original-operation"
            )
            && !a.id.starts_with("native-input-")
    });
    basis.artifacts.push(artifact(
        &basis.delivery.input_artifact,
        delivery.projection,
    ));
    basis.artifacts.push(artifact(
        &basis.recommendation.result_artifact,
        recommendation.projection,
    ));
    basis.artifacts.extend(delivery.artifacts);
    basis.artifacts.push(artifact(
        "research-original-result",
        serde_json::to_vec(
            original
                .advisory()
                .result
                .as_ref()
                .ok_or(TaskError::Unavailable)?,
        )?,
    ));
    basis.artifacts.push(artifact(
        "research-original-operation",
        original.operation_commit().to_vec(),
    ));
    basis.recommendation.action = recommendation.action;
    basis.recommendation.delivery_digest = research_digest("delivery", &basis.delivery)?;
    Ok(CheckedHostBasis { basis, original })
}

#[cfg(feature = "controlled-proof")]
pub(super) struct CheckedHostBasis {
    basis: BasisBundle,
    original: ResearchOriginalEvidence,
}
#[cfg(feature = "controlled-proof")]
impl CheckedHostBasis {
    pub(super) fn mint(
        self,
        store: &TaskStore,
        producer: &ResearchProducer,
    ) -> Result<VerifiedBasis, HostError> {
        Ok(store.controlled_verify_joined_research_basis(self.basis, self.original, producer)?)
    }
}
#[cfg(feature = "controlled-proof")]
pub(crate) fn checked_basis(
    store: &TaskStore,
    basis: BasisBundle,
    original: ResearchOriginalEvidence,
    retained: RetainedDelivery,
    request: &[u8],
    card: &[u8],
    reconstruction: &[u8],
    producer: &ResearchProducer,
) -> Result<VerifiedBasis, HostError> {
    checked_unminted_basis(basis, original, retained, request, card, reconstruction)?
        .mint(store, producer)
}

/// The controlled startup fixture fixes selector meaning and original delivery
/// bytes; an RPC only selects its already pinned basis digest. The service
/// resolves the original result operation through its existing task-owner port.
#[cfg(feature = "controlled-proof")]
pub(crate) fn checked_fixture_basis(
    store: &TaskStore,
    socket: &Path,
    basis: BasisBundle,
    producer: &ResearchProducer,
) -> Result<VerifiedBasis, HostError> {
    let pinned = |id: &str| -> Result<Vec<u8>, HostError> {
        let values = basis
            .artifacts
            .iter()
            .filter(|a| a.id == id)
            .collect::<Vec<_>>();
        if values.len() != 1 || sha(&values[0].bytes) != values[0].digest {
            return refuse();
        }
        Ok(values[0].bytes.clone())
    };
    let request = pinned("research-request")?;
    let card = pinned("research-card")?;
    let reconstruction = pinned("research-reconstruction")?;
    let mapping: std::collections::BTreeMap<String, (String, lifecycle_semantic::SourceReference)> =
        serde_json::from_value(strict(&pinned("research-selector-map")?)?)?;
    if mapping.keys().ne(basis.dependencies.keys()) {
        return refuse();
    }
    let original = store.research_original_evidence(&basis.delivery.advisory_task, &mapping)?;
    let retained = acquire(
        socket,
        &basis.delivery.advisory_task,
        &basis.delivery.advisory_subject,
    )?;
    checked_basis(
        store,
        basis,
        original,
        retained,
        &request,
        &card,
        &reconstruction,
        producer,
    )
}

#[cfg(feature = "controlled-proof")]
pub(crate) fn joined_candidate(basis: &BasisBundle) -> bool {
    basis.artifacts.iter().any(|a| {
        matches!(
            a.id.as_str(),
            "research-request"
                | "research-card"
                | "research-reconstruction"
                | "research-selector-map"
        )
    })
}
#[cfg(feature = "controlled-proof")]
pub(crate) fn validate_fixture_inputs(basis: &BasisBundle) -> Result<(), HostError> {
    let pinned = |id: &str| -> Result<&[u8], HostError> {
        let mut matches = basis.artifacts.iter().filter(|a| a.id == id);
        let a = matches.next().ok_or(TaskError::Unavailable)?;
        if matches.next().is_some() || sha(&a.bytes) != a.digest {
            return refuse();
        }
        Ok(&a.bytes)
    };
    let request = pinned("research-request")?;
    let card = pinned("research-card")?;
    let reconstruction: research_register::card_host::Reconstruction =
        serde_json::from_value(strict(pinned("research-reconstruction")?)?)?;
    let bytes = research_register::card_host::reconstruct(
        &reconstruction,
        &sha(&serde_json::to_vec(&reconstruction)?),
    )
    .map_err(|_| HostError::Configuration)?;
    let envelope =
        research_register::card_wire::decode(card).map_err(|_| HostError::Configuration)?;
    if bytes != card
        || reconstruction.request.current.request_sha256 != sha(request)
        || envelope.outcome != "card"
        || envelope.card.as_ref().is_none_or(|c| c.coverage.truncated)
        || envelope.request.as_ref() != Some(&reconstruction.request.current)
    {
        return refuse();
    }
    validate_original_request(request, &reconstruction)?;
    validate_basis_target(basis, request, pinned("research-reconstruction")?)?;
    let mapping: std::collections::BTreeMap<String, (String, lifecycle_semantic::SourceReference)> =
        serde_json::from_value(strict(pinned("research-selector-map")?)?)?;
    if mapping.keys().ne(basis.dependencies.keys()) {
        return refuse();
    }
    Ok(())
}

// Reconstruction replays the full Git pin (commit/path/blob); bind its exact
// target bytes to the fixed semantic selector and both preparation identities.
fn validate_basis_target(
    basis: &BasisBundle,
    request: &[u8],
    reconstruction: &[u8],
) -> Result<(), HostError> {
    let reconstruction: research_register::card_host::Reconstruction =
        serde_json::from_value(strict(reconstruction)?)?;
    let target = validate_original_request(request, &reconstruction)?;
    let mut copies = reconstruction
        .objects
        .iter()
        .filter(|o| o.oid == target.blob && o.kind == "blob");
    let copy = copies.next().ok_or(TaskError::Unavailable)?;
    let id = basis
        .dependency_artifacts
        .get("decision.target")
        .ok_or(TaskError::Unavailable)?;
    let mut artifacts = basis.artifacts.iter().filter(|a| &a.id == id);
    let artifact = artifacts.next().ok_or(TaskError::Unavailable)?;
    if copies.next().is_some()
        || artifacts.next().is_some()
        || sha(&copy.bytes) != target.sha256
        || basis.target_digest != target.sha256
        || basis.recommendation.target_digest != target.sha256
        || basis.dependencies.get("decision.target") != Some(&target.sha256)
        || artifact.digest != target.sha256
        || artifact.bytes != copy.bytes
    {
        return refuse();
    }
    Ok(())
}

fn validate_original_request(
    request: &[u8],
    reconstruction: &research_register::card_host::Reconstruction,
) -> Result<research_register::use_bundle::BlobPin, HostError> {
    let document: research_register::use_bundle::RecordDocument =
        serde_json::from_value(strict(request)?)?;
    let research_register::use_bundle::Record::Request {
        decision,
        decision_revision,
        target,
        ..
    } = document.record
    else {
        return refuse();
    };
    if document.schema != research_register::use_bundle::VERSION
        || decision != reconstruction.request.current.decision_id
        || decision_revision != reconstruction.request.current.decision_revision
        || sha(request) != reconstruction.request.current.request_sha256
    {
        return refuse();
    }
    Ok(target)
}

#[cfg(test)]
mod projection_tests {
    use super::*;
    #[test]
    fn scoped_chunk_assembly_checks_identity_ranges_content_and_encoding() {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixListener;
        let bytes = serde_json::to_vec(&json!({"version":1,"causal":{"version":1,"operation_id":"original"},"padding":"x".repeat(48*1024)})).unwrap();
        let replies: Vec<Value> = bytes.chunks(48*1024).enumerate().map(|(i,b)| json!({"version":1,"store_id":"store","operation_id":"original","content_sha256":sha(&bytes),"length":bytes.len(),"offset":i*48*1024,"encoding":"json-utf8-v1","bytes_base64":base64::engine::general_purpose::STANDARD.encode(b)})).collect();
        for case in [
            "complete",
            "changed-store",
            "changed-operation",
            "changed-length",
            "changed-digest",
            "offset",
            "encoding",
            "short-chunk",
            "changed-content",
            "missing-chunk",
            "base64",
            "json",
            "capacity",
        ] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("actor.sock");
            let listener = UnixListener::bind(&path).unwrap();
            let mut response = replies.clone();
            match case {
                "changed-store" => response[1]["store_id"] = json!("foreign"),
                "changed-operation" => response[1]["operation_id"] = json!("foreign"),
                "changed-length" => response[1]["length"] = json!(bytes.len() + 1),
                "changed-digest" => response[1]["content_sha256"] = json!("f".repeat(64)),
                "offset" => response[1]["offset"] = json!(0),
                "encoding" => {
                    response[0]["encoding"] = json!("unsupported");
                    response.truncate(1);
                }
                "short-chunk" => {
                    response[0]["bytes_base64"] = json!("YQ==");
                    response.truncate(1);
                }
                "changed-content" => {
                    response[1]["bytes_base64"] =
                        json!(base64::engine::general_purpose::STANDARD.encode(vec![
                            b'x';
                            bytes.len()
                                - 48 * 1024
                        ]))
                }
                "missing-chunk" => {
                    response.truncate(1);
                }
                "base64" => {
                    response[0]["bytes_base64"] = json!("invalid-base64");
                    response.truncate(1);
                }
                "json" => {
                    let invalid = b"invalid-json";
                    response = vec![
                        json!({"version":1,"store_id":"store","operation_id":"original","content_sha256":sha(invalid),"length":invalid.len(),"offset":0,"encoding":"json-utf8-v1","bytes_base64":base64::engine::general_purpose::STANDARD.encode(invalid)}),
                    ];
                }
                "capacity" => {
                    response[0]["length"] = json!(LIMIT + 1);
                    response.truncate(1);
                }
                _ => {}
            }
            let server = std::thread::spawn(move || {
                for reply in response {
                    let (mut stream, _) = listener.accept().unwrap();
                    let mut request = String::new();
                    BufReader::new(stream.try_clone().unwrap())
                        .read_line(&mut request)
                        .unwrap();
                    let request: Value = serde_json::from_str(&request).unwrap();
                    assert_eq!(request["task_id"], "advisory");
                    assert_eq!(request["subject_id"], "subject");
                    let encoded = serde_json::to_vec(&reply).unwrap();
                    assert!(encoded.len() < 128 * 1024);
                    stream.write_all(&encoded).unwrap();
                    stream.write_all(b"\n").unwrap();
                }
            });
            let actual = acquire(&path, "advisory", "subject");
            server.join().unwrap();
            if case == "complete" {
                assert_eq!(actual.unwrap().bytes, bytes);
            } else {
                assert!(
                    matches!(actual, Err(HostError::ResearchDelivery(_))),
                    "{case}"
                );
            }
        }
    }
    fn fixture() -> (Value, Vec<u8>) {
        let bytes = "request\n{\"card\":\"雪\"}\nreconstruction"
            .as_bytes()
            .to_vec();
        let mut input = Vec::new();
        for (n, (start, end)) in [(0, 18), (18, bytes.len())].into_iter().enumerate() {
            let chunk = &bytes[start..end];
            let call = format!("read-{n}");
            input.push(json!({"type":"function_call","id":format!("call-item-{n}"),"namespace":"workspace","name":"snapshot_read","call_id":call,"arguments":json!({"snapshot_id":"snapshot:original","member_id":"research","offset":start,"length":chunk.len()}).to_string()}));
            let result = json!({"encoding":"hex","bytes_hex":chunk.iter().map(|b| format!("{b:02x}")).collect::<String>(),"snapshot_id":"snapshot:original","member_id":"research","offset":start,"length":chunk.len(),"sha256":sha(chunk)});
            input.push(json!({"type":"function_call_output","id":format!("output-{n}"),"call_id":call,"output":result.to_string()}));
        }
        (json!({"input":input}), bytes)
    }
    fn check(v: &Value, expected: &[u8]) -> Result<Option<(Vec<u8>, Vec<Span>)>, HostError> {
        snapshot_projection(v, "inference:original", 1, "snapshot:original", expected)
    }
    #[test]
    fn snapshot_unicode_split_reassembly_binds_original_calls_and_ranges() {
        let (v, bytes) = fixture();
        let (projection, spans) = check(&v, &bytes).unwrap().unwrap();
        assert_eq!(projection, bytes);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].tool_call.as_deref(), Some("read-0"));
        assert_eq!(spans[0].utf8_end, spans[1].utf8_start);
        assert_eq!(spans[1].utf8_end, bytes.len());
        assert_eq!(
            spans[0].snapshot.as_ref().unwrap()["output_item_id"],
            "output-0"
        );
    }
    #[test]
    fn snapshot_changed_identity_ranges_encoding_and_content_refuse() {
        let (base, bytes) = fixture();
        for (field, value) in [
            ("snapshot_id", json!("foreign")),
            ("member_id", json!("foreign")),
            ("offset", json!(1)),
            ("length", json!(17)),
            ("sha256", json!(sha(b"wrong"))),
            ("bytes_hex", json!("00".repeat(18))),
            ("encoding", json!("base64")),
        ] {
            let mut v = base.clone();
            let mut output = strict(v["input"][1]["output"].as_str().unwrap().as_bytes()).unwrap();
            output[field] = value;
            v["input"][1]["output"] = json!(output.to_string());
            assert!(check(&v, &bytes).is_err(), "{field}");
        }
        let mut v = base.clone();
        v["input"].as_array_mut().unwrap().pop();
        assert!(check(&v, &bytes).is_err());
        let mut v = base.clone();
        let duplicate = v["input"][1].clone();
        v["input"].as_array_mut().unwrap().push(duplicate);
        assert!(check(&v, &bytes).is_err());
        let mut v = base.clone();
        v["input"][0]["call_id"] = json!("foreign");
        assert!(check(&v, &bytes).is_err());
        let mut v = base.clone();
        v["input"][0]["arguments"] = json!("{\"offset\":0,\"offset\":0}");
        assert!(check(&v, &bytes).is_err());
        let mut v = base.clone();
        v["input"][1]["output"] = json!("{\"encoding\":\"hex\",\"encoding\":\"hex\"}");
        assert!(check(&v, &bytes).is_err());
        let mut changed = bytes.clone();
        changed[0] ^= 1;
        assert!(check(&base, &changed).is_err());
    }
}
