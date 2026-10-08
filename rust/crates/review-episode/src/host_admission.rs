//! Trusted native-host descriptor admission. The inherited descriptor is a
//! transport capability within the selected host/launcher boundary, not an
//! authentication mechanism against arbitrary code running as the same UID.
use std::fs::File;
use std::io::Read;

use review_episode_core::codec::{
    JsValue, canonical_json, digest, exact_fields, field, parse_json,
};
use review_episode_core::identity::Identity;
use review_episode_store::NativeRootSelection;

use crate::admission::{AdmittedResult, HostAdmissionPort};
use crate::protocol::Request;
use crate::{AppError, AppResult};

pub struct NativeHostAdmission {
    descriptor: JsValue,
    selection: NativeRootSelection,
}

fn invalid(message: &str) -> AppError {
    AppError::Admission(message.into())
}

fn optional_digest(value: Option<&JsValue>) -> JsValue {
    value.map_or(JsValue::Null, |value| JsValue::text(&digest(value)))
}

fn identity_key(request: &Request) -> AppResult<String> {
    let identity = match request.operation.as_str() {
        "begin" | "transition" | "resumeInitial" | "validateResult" => {
            field(field(&request.args, "authority")?, "identity")?
        }
        "read" | "history" | "recover" => field(&request.args, "identity")?,
        _ => return Err(invalid("native operation is unavailable")),
    };
    Ok(Identity::parse(identity)?.key().0)
}

fn content_digest(request: &Request) -> AppResult<JsValue> {
    let content = match request.operation.as_str() {
        "begin" => Some(JsValue::object([
            ("action", JsValue::text("begin")),
            (
                "unresolvedQuestions",
                field(&request.args, "unresolvedQuestions")?.clone(),
            ),
        ])),
        "transition" => Some(JsValue::object([
            ("action", field(&request.args, "action")?.clone()),
            ("payload", field(&request.args, "payload")?.clone()),
        ])),
        _ => None,
    };
    Ok(optional_digest(content.as_ref()))
}

impl NativeHostAdmission {
    pub fn from_inherited_fd(selection: NativeRootSelection) -> AppResult<Self> {
        #[cfg(not(unix))]
        {
            let _ = selection;
            return Err(invalid("native host descriptor requires Unix"));
        }
        #[cfg(unix)]
        {
            use rustix::fs::OFlags;
            use std::os::fd::FromRawFd;
            use std::os::unix::fs::MetadataExt;

            let file = unsafe { File::from_raw_fd(3) };
            let metadata = file
                .metadata()
                .map_err(|_| invalid("native descriptor fd 3 is unavailable"))?;
            unsafe extern "C" {
                fn geteuid() -> u32;
            }
            let uid = unsafe { geteuid() };
            let flags = rustix::fs::fcntl_getfl(&file)
                .map_err(|_| invalid("native descriptor access flags unavailable"))?;
            if !metadata.is_file()
                || metadata.nlink() != 0
                || metadata.uid() != uid
                || metadata.mode() & 0o077 != 0
                || flags.intersects(OFlags::WRONLY | OFlags::RDWR)
            {
                return Err(invalid(
                    "native descriptor is not private read-only unlinked file",
                ));
            }
            let mut bytes = Vec::new();
            file.take(64 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| invalid("native descriptor cannot be read"))?;
            if bytes.len() > 64 * 1024 {
                return Err(invalid("native descriptor exceeds limit"));
            }
            let source = std::str::from_utf8(&bytes)
                .map_err(|_| invalid("native descriptor is not UTF-8"))?;
            let descriptor =
                parse_json(source).map_err(|_| invalid("native descriptor JSON is invalid"))?;
            if canonical_json(&descriptor) != source {
                return Err(invalid("native descriptor is not canonical"));
            }
            exact_fields(
                &descriptor,
                &[
                    "schemaVersion",
                    "profile",
                    "root",
                    "executableSha256",
                    "rootSelectionDigest",
                    "requestId",
                    "operation",
                    "grantId",
                    "selectionRevision",
                    "requestDigest",
                    "identityKey",
                    "argsDigest",
                    "authorityDigest",
                    "resultDigest",
                    "evidenceDigest",
                    "contentDigest",
                    "observedRevision",
                    "principal",
                ],
                "native host descriptor",
            )
            .map_err(|_| invalid("native descriptor shape differs"))?;
            if field(&descriptor, "schemaVersion")? != &JsValue::Number(1.0)
                || field(&descriptor, "profile")? != &JsValue::text("native-host-v1")
                || field(&descriptor, "root")? != &JsValue::text(&selection.root)
                || field(&descriptor, "executableSha256")?
                    != &JsValue::text(&selection.executable_sha256)
                || field(&descriptor, "rootSelectionDigest")?
                    != &JsValue::text(&selection.selection_digest)
            {
                return Err(invalid("native descriptor selection differs"));
            }
            Ok(Self {
                descriptor,
                selection,
            })
        }
    }

    fn check_request(&self, request: &Request) -> AppResult<()> {
        if request.version != 2
            || request.root_selection_digest.as_ref().map(|r| &r.0)
                != Some(&self.selection.selection_digest)
        {
            return Err(invalid("native request profile or root selection differs"));
        }
        let descriptor = &self.descriptor;
        for (name, expected) in [
            ("requestId", JsValue::text(&request.request_id)),
            ("operation", JsValue::text(&request.operation)),
            ("grantId", JsValue::text(&request.grant_id)),
            ("selectionRevision", request.selection_revision.value()),
            ("requestDigest", JsValue::text(&digest(&request.envelope))),
            ("identityKey", JsValue::text(&identity_key(request)?)),
            ("argsDigest", JsValue::text(&digest(&request.args))),
            (
                "authorityDigest",
                optional_digest(request.args.get("authority")),
            ),
            (
                "resultDigest",
                optional_digest(
                    request
                        .args
                        .get("result")
                        .or_else(|| request.args.get("payload").and_then(|p| p.get("result"))),
                ),
            ),
            (
                "evidenceDigest",
                optional_digest(
                    request
                        .args
                        .get("payload")
                        .and_then(|p| p.get("evidenceAdmissions")),
                ),
            ),
            ("contentDigest", content_digest(request)?),
        ] {
            if field(descriptor, name)? != &expected {
                return Err(invalid("native request binding differs"));
            }
        }
        let principal = field(descriptor, "principal")?;
        exact_fields(
            principal,
            &["id", "identityKey", "access"],
            "native principal",
        )
        .map_err(|_| invalid("native principal shape differs"))?;
        let access = if matches!(request.operation.as_str(), "read" | "history" | "recover") {
            "read"
        } else {
            "write"
        };
        if field(principal, "id")? != &JsValue::text("native-review-host")
            || field(principal, "identityKey")? != &JsValue::text(&identity_key(request)?)
            || field(principal, "access")? != &JsValue::text(access)
        {
            return Err(invalid("native principal scope differs"));
        }
        Ok(())
    }
}

impl HostAdmissionPort for NativeHostAdmission {
    fn request_version(&self) -> u8 {
        2
    }

    fn precheck(&self, request: &Request) -> AppResult<()> {
        self.check_request(request)
    }

    fn admit(
        &self,
        request: &Request,
        observed_revision: Option<&str>,
    ) -> AppResult<AdmittedResult> {
        self.check_request(request)?;
        let expected = field(&self.descriptor, "observedRevision")?;
        if matches!(request.operation.as_str(), "read" | "history" | "recover") {
            if expected != &JsValue::Null {
                return Err(invalid("native reader observation must be unbound"));
            }
        } else if expected != &observed_revision.map_or(JsValue::Null, JsValue::text) {
            return Err(invalid("native writer observation differs"));
        }
        Ok(AdmittedResult::checked())
    }
}
