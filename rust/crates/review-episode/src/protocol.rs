use std::io::{Read, Write};

use review_episode_core::codec::{JsValue, canonical_json, exact_fields, field, parse_json};
use review_episode_core::identity::Revision;

use crate::operation::Operation;
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
    pub(crate) version: u8,
    pub(crate) root_selection_digest: Option<Revision>,
    pub(crate) request_id: String,
    pub(crate) operation: String,
    pub(crate) grant_id: String,
    pub(crate) selection_revision: Revision,
    pub(crate) args: JsValue,
}

/// The caller selects a previously established admission port separately. These
/// fields bind one direct operation to the same grant/digest checks as the CLI.
#[derive(Clone, Debug)]
pub enum RequestSelection {
    Offline {
        request_id: String,
        grant_id: String,
        selection_revision: Revision,
    },
    Native {
        request_id: String,
        grant_id: String,
        selection_revision: Revision,
        root_selection_digest: Revision,
    },
}

pub struct DirectRequest {
    request: Request,
    operation: Operation,
}

impl DirectRequest {
    pub fn new(selection: RequestSelection, operation: Operation) -> AppResult<Self> {
        let (version, request_id, grant_id, selection_revision, root_selection_digest) =
            match selection {
                RequestSelection::Offline {
                    request_id,
                    grant_id,
                    selection_revision,
                } => (1, request_id, grant_id, selection_revision, None),
                RequestSelection::Native {
                    request_id,
                    grant_id,
                    selection_revision,
                    root_selection_digest,
                } => (
                    2,
                    request_id,
                    grant_id,
                    selection_revision,
                    Some(root_selection_digest),
                ),
            };
        for (label, value, limit) in [("requestId", &request_id, 128), ("grantId", &grant_id, 128)]
        {
            ascii_control(&JsValue::text(value), label, limit)?;
        }
        Revision::parse(&selection_revision.value(), "selectionRevision")?;
        if let Some(digest) = &root_selection_digest {
            Revision::parse(&digest.value(), "rootSelectionDigest")?;
        }
        if version == 1
            && matches!(
                operation,
                Operation::Recover {
                    transition: None,
                    ..
                }
            )
        {
            return Err(AppError::Framing(
                "offline recovery requires transition identity".into(),
            ));
        }
        operation.validate()?;
        let args = operation.args();
        let mut fields = vec![
            ("version", JsValue::Number(version as f64)),
            ("domain", JsValue::text("review-episode")),
            ("requestId", JsValue::text(&request_id)),
            ("operation", JsValue::text(operation.name())),
            ("grantId", JsValue::text(&grant_id)),
            ("selectionRevision", selection_revision.value()),
            ("args", args.clone()),
        ];
        if let Some(digest) = &root_selection_digest {
            fields.extend([
                ("profile", JsValue::text("native-host-v1")),
                ("rootSelectionDigest", digest.value()),
            ]);
        }
        let envelope = JsValue::object(fields);
        Ok(Self {
            request: Request {
                envelope,
                version,
                root_selection_digest,
                request_id,
                operation: operation.name().into(),
                grant_id,
                selection_revision,
                args,
            },
            operation,
        })
    }

    pub fn request(&self) -> &Request {
        &self.request
    }
    pub fn operation(&self) -> &Operation {
        &self.operation
    }
}

impl Request {
    pub fn envelope(&self) -> &JsValue {
        &self.envelope
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn operation(&self) -> &str {
        &self.operation
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn parse(bytes: &[u8]) -> AppResult<Self> {
        let source = std::str::from_utf8(bytes)
            .map_err(|_| AppError::Framing("request is not UTF-8".into()))?;
        let envelope = parse_json(source).map_err(AppError::Domain)?;
        let version = match field(&envelope, "version")? {
            JsValue::Number(1.0) => 1,
            JsValue::Number(2.0) => 2,
            _ => return Err(AppError::Framing("unsupported request version".into())),
        };
        let mut expected = vec![
            "version",
            "domain",
            "requestId",
            "operation",
            "grantId",
            "selectionRevision",
            "args",
        ];
        if version == 2 {
            expected.extend(["profile", "rootSelectionDigest"]);
        }
        exact_fields(&envelope, &expected, "request envelope").map_err(AppError::Domain)?;
        if field(&envelope, "domain")? != &JsValue::text("review-episode") {
            return Err(AppError::Framing(
                "unsupported request version or domain".into(),
            ));
        }
        let root_selection_digest = if version == 2 {
            if field(&envelope, "profile")? != &JsValue::text("native-host-v1") {
                return Err(AppError::Framing("unsupported native host profile".into()));
            }
            Some(Revision::parse(
                field(&envelope, "rootSelectionDigest")?,
                "rootSelectionDigest",
            )?)
        } else {
            None
        };
        let request_id = ascii_control(field(&envelope, "requestId")?, "requestId", 128)?;
        let operation = ascii_control(field(&envelope, "operation")?, "operation", 40)?;
        let grant_id = ascii_control(field(&envelope, "grantId")?, "grantId", 128)?;
        let selection_revision =
            Revision::parse(field(&envelope, "selectionRevision")?, "selectionRevision")?;
        let args = field(&envelope, "args")?.clone();
        args.as_object().map_err(AppError::Domain)?;
        Ok(Self {
            envelope,
            version,
            root_selection_digest,
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
        ("version", JsValue::Number(request.version as f64)),
        ("domain", JsValue::text("review-episode")),
        ("requestId", JsValue::text(&request.request_id)),
        ("source", JsValue::text(request.source())),
        ("status", JsValue::text(status)),
    ];
    if request.version == 2 {
        fields.extend(request.native_reply_binding());
    }
    fields.extend(extra);
    canonical_json(&JsValue::object(fields))
}

impl Request {
    pub fn source(&self) -> &'static str {
        if self.version == 2 {
            "native_host"
        } else {
            "offline_fixture"
        }
    }

    fn native_reply_binding(&self) -> Vec<(&'static str, JsValue)> {
        vec![
            ("profile", JsValue::text("native-host-v1")),
            ("operation", JsValue::text(&self.operation)),
            (
                "requestDigest",
                JsValue::text(&review_episode_core::codec::digest(&self.envelope)),
            ),
            (
                "rootSelectionDigest",
                self.root_selection_digest
                    .as_ref()
                    .expect("native request has root selection")
                    .value(),
            ),
        ]
    }
}

pub fn error_reply_for_request(request: &Request, kind: &str, message: &str) -> String {
    let mut fields = vec![
        ("version", JsValue::Number(request.version as f64)),
        ("domain", JsValue::text("review-episode")),
        ("requestId", JsValue::text(&request.request_id)),
        ("source", JsValue::text(request.source())),
        ("status", JsValue::text("error")),
        ("kind", JsValue::text(kind)),
        ("message", JsValue::text(message)),
    ];
    if request.version == 2 {
        fields.extend(request.native_reply_binding());
    }
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
