use claim_evidence::codec::{JsValue, canonical_json, parse_json};
use review_episode_core::implementation_review_v1::ReviewResult;

use crate::{EvidenceReadError, RESULT_MAX_BYTES, STREAM_MAX_BYTES};

use super::profile::TrustedClaudeProfile;

/// The pinned CLI's JSON result is transport data. Its structured output is
/// validated independently with the owning episode codec.
pub struct ValidatedResponse {
    pub result: Vec<u8>,
    pub observed_model: String,
    pub session_id: String,
    pub provider_reported_entry: bool,
}

fn text_field(value: &JsValue, key: &str) -> Result<String, EvidenceReadError> {
    value
        .get(key)
        .and_then(|item| item.as_text().ok())
        .and_then(|item| item.to_string_checked().ok())
        .filter(|text| !text.is_empty())
        .ok_or(EvidenceReadError::Conflicting(
            "Claude envelope identity missing",
        ))
}

pub fn validate_response(
    raw: &[u8],
    profile: &TrustedClaudeProfile,
    expected_session: &str,
    expected_subject: &JsValue,
) -> Result<ValidatedResponse, EvidenceReadError> {
    if raw.len() > STREAM_MAX_BYTES {
        return Err(EvidenceReadError::Capacity(
            "Claude response stream exceeds limit",
        ));
    }
    let source = std::str::from_utf8(raw)
        .map_err(|_| EvidenceReadError::Conflicting("Claude response is not UTF-8"))?;
    // The owning codec rejects duplicate keys, including nested duplicates.
    let envelope = parse_json(source)
        .map_err(|_| EvidenceReadError::Conflicting("Claude envelope malformed or duplicated"))?;
    if text_field(&envelope, "type")? != "result" || text_field(&envelope, "subtype")? != "success"
    {
        return Err(EvidenceReadError::Conflicting(
            "Claude result is not successful",
        ));
    }
    let session = text_field(&envelope, "session_id")?;
    if session != expected_session {
        return Err(EvidenceReadError::Conflicting("Claude session mismatch"));
    }
    let model = text_field(&envelope, "model")?;
    if !profile.accepts_observed_model(&model) {
        return Err(EvidenceReadError::Conflicting(
            "Claude observed model unacceptable",
        ));
    }
    let usage = envelope
        .get("modelUsage")
        .ok_or(EvidenceReadError::Conflicting("Claude model usage missing"))?;
    let JsValue::Object(models) = usage else {
        return Err(EvidenceReadError::Conflicting(
            "Claude model usage malformed",
        ));
    };
    if models.len() != 1
        || models
            .keys()
            .next()
            .and_then(|name| name.to_string_checked().ok())
            .as_deref()
            != Some(model.as_str())
    {
        return Err(EvidenceReadError::Conflicting(
            "Claude model identities mixed",
        ));
    }
    let structured = envelope
        .get("structured_output")
        .ok_or(EvidenceReadError::Conflicting(
            "Claude structured result absent",
        ))?;
    let result = canonical_json(structured)
        .map_err(|_| EvidenceReadError::Conflicting("Claude result cannot be canonicalized"))?
        .into_bytes();
    if result.len() > RESULT_MAX_BYTES {
        return Err(EvidenceReadError::Capacity("Claude result exceeds limit"));
    }
    let result_text = std::str::from_utf8(&result)
        .map_err(|_| EvidenceReadError::Conflicting("Claude result UTF-8 invalid"))?;
    let episode_value = review_episode_core::codec::parse_json(result_text)
        .map_err(|_| EvidenceReadError::Conflicting("episode result JSON invalid"))?;
    ReviewResult::parse(episode_value)
        .map_err(|_| EvidenceReadError::Conflicting("episode result contract invalid"))?;
    if structured.get("subject") != Some(expected_subject) {
        return Err(EvidenceReadError::Conflicting(
            "Claude result subject mismatch",
        ));
    }
    Ok(ValidatedResponse {
        result,
        observed_model: model,
        session_id: session,
        provider_reported_entry: true,
    })
}
