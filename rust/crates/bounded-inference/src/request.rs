use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Local safety limits. They do not imply remote cancellation or provider quota.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bounds {
    pub request_bytes: usize,
    pub stream_bytes: usize,
    pub frame_bytes: usize,
    pub output_bytes: usize,
    pub events: usize,
    pub local_deadline_ms: u64,
}

impl Bounds {
    pub fn validate(&self) -> Result<(), RequestError> {
        if self.request_bytes == 0
            || self.stream_bytes == 0
            || self.frame_bytes == 0
            || self.output_bytes == 0
            || self.events == 0
            || self.local_deadline_ms == 0
            || self.frame_bytes > self.stream_bytes
        {
            return Err(RequestError::InvalidBounds);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolFreeProfile {
    /// Selected configuration, never inferred from a response.
    pub revision: String,
    pub model: String,
    pub bounds: Bounds,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssembledRequest {
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub profile_revision: String,
    pub selected_model: String,
    pub bounds: Bounds,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RequestError {
    #[error("profile revision and model must be nonempty")]
    MissingProfileIdentity,
    #[error("local limits must be positive and frame limit cannot exceed stream limit")]
    InvalidBounds,
    #[error("request exceeds local byte limit")]
    RequestTooLarge,
    #[error("request serialization failed: {0}")]
    Serialization(String),
}

#[derive(Serialize)]
struct WireRequest<'a> {
    model: &'a str,
    instructions: &'a str,
    input: [WireInput<'a>; 1],
    store: bool,
    stream: bool,
}

#[derive(Serialize)]
struct WireInput<'a> {
    role: &'static str,
    content: [WireContent<'a>; 1],
}

#[derive(Serialize)]
struct WireContent<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    text: &'a str,
}

/// Builds the documented tool-free fixture profile. No HTTP or provider operation occurs.
pub fn assemble_request(
    instructions: &str,
    input: &str,
    profile: &ToolFreeProfile,
) -> Result<AssembledRequest, RequestError> {
    profile.bounds.validate()?;
    if profile.model.trim().is_empty() || profile.revision.trim().is_empty() {
        return Err(RequestError::MissingProfileIdentity);
    }
    let wire = WireRequest {
        model: &profile.model,
        instructions,
        input: [WireInput {
            role: "user",
            content: [WireContent {
                kind: "input_text",
                text: input,
            }],
        }],
        store: false,
        stream: true,
    };
    let bytes =
        serde_json::to_vec(&wire).map_err(|e| RequestError::Serialization(e.to_string()))?;
    if bytes.len() > profile.bounds.request_bytes {
        return Err(RequestError::RequestTooLarge);
    }
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    Ok(AssembledRequest {
        bytes,
        sha256,
        profile_revision: profile.revision.clone(),
        selected_model: profile.model.clone(),
        bounds: profile.bounds.clone(),
    })
}
