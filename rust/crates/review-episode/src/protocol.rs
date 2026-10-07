use std::io::{Read, Write};

use review_episode_core::codec::{JsValue, canonical_json, exact_fields, field, parse_json};
use review_episode_core::identity::Revision;

use crate::{AppError, AppResult};

pub const DEFAULT_REQUEST_LIMIT: usize = 8 * 1024 * 1024;
pub const DEFAULT_RESPONSE_LIMIT: usize = 32 * 1024 * 1024;

/// Parsed request fields cannot be changed independently of the envelope used
/// for admission. External callers can only construct this type through `parse`.
///
/// ```compile_fail
/// use review_episode::protocol::Request;
/// let mut request: Request = todo!();
/// request.args = review_episode_core::codec::JsValue::object([]);
/// ```
#[derive(Clone, Debug)]
pub struct Request {
    pub(crate) envelope: JsValue,
    pub(crate) request_id: String,
    pub(crate) operation: String,
    pub(crate) grant_id: String,
    pub(crate) selection_revision: Revision,
    pub(crate) args: JsValue,
}

impl Request {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn parse(bytes: &[u8]) -> AppResult<Self> {
        let source = std::str::from_utf8(bytes)
            .map_err(|_| AppError::Framing("request is not UTF-8".into()))?;
        let envelope = parse_json(source).map_err(AppError::Domain)?;
        exact_fields(
            &envelope,
            &[
                "version",
                "domain",
                "requestId",
                "operation",
                "grantId",
                "selectionRevision",
                "args",
            ],
            "request envelope",
        )
        .map_err(AppError::Domain)?;
        if field(&envelope, "version")? != &JsValue::Number(1.0)
            || field(&envelope, "domain")? != &JsValue::text("review-episode")
        {
            return Err(AppError::Framing(
                "unsupported request version or domain".into(),
            ));
        }
        let request_id = ascii_control(field(&envelope, "requestId")?, "requestId", 128)?;
        let operation = ascii_control(field(&envelope, "operation")?, "operation", 40)?;
        let grant_id = ascii_control(field(&envelope, "grantId")?, "grantId", 128)?;
        let selection_revision =
            Revision::parse(field(&envelope, "selectionRevision")?, "selectionRevision")?;
        let args = field(&envelope, "args")?.clone();
        args.as_object().map_err(AppError::Domain)?;
        Ok(Self {
            envelope,
            request_id,
            operation,
            grant_id,
            selection_revision,
            args,
        })
    }
}

pub fn ascii_control(value: &JsValue, label: &str, limit: usize) -> AppResult<String> {
    let text = value
        .as_text()
        .map_err(AppError::Domain)?
        .to_string_checked()
        .map_err(AppError::Domain)?;
    if text.is_empty() || text.len() > limit || !text.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(AppError::Framing(format!("{label} must be bounded ASCII")));
    }
    Ok(text)
}

pub fn read_frame<R: Read>(reader: &mut R, max: usize) -> AppResult<Vec<u8>> {
    let mut prefix = [0u8; 4];
    reader
        .read_exact(&mut prefix)
        .map_err(|_| AppError::Framing("incomplete frame length".into()))?;
    let len = u32::from_be_bytes(prefix) as usize;
    if len == 0 || len > max {
        return Err(AppError::Framing(
            "request frame exceeds limit or is empty".into(),
        ));
    }
    let mut body = vec![0u8; len];
    reader
        .read_exact(&mut body)
        .map_err(|_| AppError::Framing("incomplete request frame".into()))?;
    let mut trailing = [0u8; 1];
    if reader
        .read(&mut trailing)
        .map_err(|_| AppError::Framing("trailing input read failed".into()))?
        != 0
    {
        return Err(AppError::Framing("trailing request bytes".into()));
    }
    Ok(body)
}

pub fn write_frame<W: Write>(writer: &mut W, value: &str, max: usize) -> AppResult<()> {
    if value.len() > max || value.len() > u32::MAX as usize {
        return Err(AppError::ResponseTooLarge);
    }
    writer
        .write_all(&(value.len() as u32).to_be_bytes())
        .map_err(|e| AppError::Io(e.to_string()))?;
    writer
        .write_all(value.as_bytes())
        .map_err(|e| AppError::Io(e.to_string()))?;
    writer.flush().map_err(|e| AppError::Io(e.to_string()))
}

pub fn reply(request: &Request, status: &str, extra: Vec<(&str, JsValue)>) -> String {
    let mut fields = vec![
        ("version", JsValue::Number(1.0)),
        ("domain", JsValue::text("review-episode")),
        ("requestId", JsValue::text(&request.request_id)),
        ("source", JsValue::text("offline_fixture")),
        ("status", JsValue::text(status)),
    ];
    fields.extend(extra);
    canonical_json(&JsValue::object(fields))
}

pub fn error_reply(request_id: Option<&str>, kind: &str, message: &str) -> String {
    let mut fields = vec![
        ("version", JsValue::Number(1.0)),
        ("domain", JsValue::text("review-episode")),
        ("source", JsValue::text("offline_fixture")),
        ("status", JsValue::text("error")),
        ("kind", JsValue::text(kind)),
        ("message", JsValue::text(message)),
    ];
    if let Some(id) = request_id {
        fields.push(("requestId", JsValue::text(id)));
    }
    canonical_json(&JsValue::object(fields))
}
