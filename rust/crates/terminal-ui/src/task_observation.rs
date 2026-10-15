//! TO2 is a passive, independently qualified read, never a task command port.
use lifecycle_wire::{
    CommandObservationKind, parse_snapshot, parse_strict_json, validate_command_observation,
    validate_native_delivery_status,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use socket2::{Domain, SockAddr, Socket, Type};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    os::{fd::OwnedFd, unix::net::UnixStream},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub const MAX_FRAME: usize = 131_072;
pub const COMPONENTS: [&str; 5] = [
    "owner",
    "lifecycle",
    "native_delivery_status",
    "command_result",
    "control_result",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadError {
    InvalidRequest,
    Unavailable,
    Deadline,
    InvalidObservation,
    UnsupportedVersion,
    UnsupportedState,
    IdentityMismatch,
    Refused,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
}
impl Request {
    pub fn validate(&self) -> Result<(), ReadError> {
        if self.task_id.is_empty()
            || self.task_id.len() > 128
            || !self
                .task_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
            || [&self.delivery_id, &self.command_id]
                .into_iter()
                .flatten()
                .any(|id| {
                    id.is_empty()
                        || id.len() > 128
                        || !id
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
                })
        {
            return Err(ReadError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Component {
    Present {
        scope: Value,
        revision: Value,
        value: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        completeness: Option<Value>,
    },
    Absent {
        scope: Value,
        record_kind: String,
    },
    NotRequested {
        scope: Value,
    },
    NotApplicable {
        scope: Value,
        reason: String,
    },
    Unavailable {
        scope: Value,
        reason: String,
    },
    Refused {
        scope: Value,
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<u64>,
    },
}
impl Component {
    pub fn scope(&self) -> &Value {
        match self {
            Self::Present { scope, .. }
            | Self::Absent { scope, .. }
            | Self::NotRequested { scope }
            | Self::NotApplicable { scope, .. }
            | Self::Unavailable { scope, .. }
            | Self::Refused { scope, .. } => scope,
        }
    }
    pub fn value(&self) -> Option<&Value> {
        match self {
            Self::Present { value, .. } => Some(value),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostIdentity {
    pub pid: u32,
    pub linux_starttime_ticks: String,
    pub incarnation: String,
    pub build_id: String,
    pub profile: String,
    pub executable_sha256: String,
    pub source_sha256: String,
    pub lock_sha256: String,
    pub toolchain_sha256: String,
    pub features: Vec<String>,
    pub provider_identity: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub protocol_version: u64,
    pub status: String,
    pub consistency: String,
    pub request: Request,
    pub host_identity: HostIdentity,
    pub components: BTreeMap<String, Component>,
}

pub fn decode(bytes: &[u8], request: &Request) -> Result<Observation, ReadError> {
    request.validate()?;
    if bytes.len() > MAX_FRAME {
        return Err(ReadError::InvalidObservation);
    }
    let raw = parse_strict_json(bytes).map_err(|_| ReadError::InvalidObservation)?;
    if raw["code"] == "unsupported_observation_version" {
        return Err(ReadError::UnsupportedVersion);
    }
    if raw["status"] == "error" {
        return Err(ReadError::Refused);
    }
    if raw["protocol_version"] != 2 {
        return Err(ReadError::UnsupportedVersion);
    }
    let components = raw["components"]
        .as_object()
        .ok_or(ReadError::InvalidObservation)?;
    for c in components.values() {
        if !matches!(
            c["state"].as_str(),
            Some(
                "present"
                    | "absent"
                    | "not_requested"
                    | "not_applicable"
                    | "unavailable"
                    | "refused"
            )
        ) {
            return Err(ReadError::UnsupportedState);
        }
        if !c["scope"].is_object() {
            return Err(ReadError::InvalidObservation);
        }
    }
    // Optional request keys must be omitted, not null. Serde Option alone would erase that distinction.
    if ["delivery_id", "command_id"]
        .into_iter()
        .any(|key| raw["request"].get(key).is_some_and(|v| !v.is_string()))
    {
        return Err(ReadError::InvalidObservation);
    }
    let observation: Observation =
        serde_json::from_value(raw).map_err(|_| ReadError::InvalidObservation)?;
    if &observation.request != request {
        return Err(ReadError::IdentityMismatch);
    }
    if observation.status != "observation"
        || observation.consistency != "independent_components"
        || observation.components.len() != 5
        || COMPONENTS
            .iter()
            .any(|name| !observation.components.contains_key(*name))
    {
        return Err(ReadError::InvalidObservation);
    }
    let identity = &observation.host_identity;
    if identity.pid == 0
        || identity.incarnation.is_empty()
        || identity.linux_starttime_ticks.parse::<u64>().is_err()
        || identity.build_id.is_empty()
        || identity.profile != "retained-repository-analysis-v1"
        || [
            &identity.executable_sha256,
            &identity.source_sha256,
            &identity.lock_sha256,
            &identity.toolchain_sha256,
        ]
        .into_iter()
        .any(|h| h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err(ReadError::InvalidObservation);
    }
    let owner = &observation.components["owner"];
    if owner.scope()["task_id"] != request.task_id {
        return Err(ReadError::IdentityMismatch);
    }
    let subject = if let Component::Present {
        scope,
        revision,
        value,
        completeness,
    } = owner
    {
        if completeness.is_some()
            || value["task_id"] != request.task_id
            || !scope["subject_id"].is_string()
            || value["subject_id"] != scope["subject_id"]
            || revision["kind"] != "task"
            || !value["revision"].is_u64()
            || !value["cursor_revision"].is_u64()
            || revision["owner_revision"] != value["revision"]
            || revision["cursor_revision"] != value["cursor_revision"]
        {
            return Err(ReadError::IdentityMismatch);
        }
        Some(scope["subject_id"].clone())
    } else {
        if !matches!(owner,Component::Absent{record_kind,..} if record_kind=="task")
            && !matches!(
                owner,
                Component::Unavailable { .. } | Component::Refused { .. }
            )
        {
            return Err(ReadError::InvalidObservation);
        }
        None
    };
    for name in ["lifecycle", "native_delivery_status"] {
        let component = &observation.components[name];
        let scope = component.scope();
        if scope.get("subject_id").is_none()
            || scope["subject_id"] != subject.clone().unwrap_or(Value::Null)
        {
            return Err(ReadError::IdentityMismatch);
        }
        if name == "lifecycle"
            && (scope.get("delivery_id").is_none()
                || scope["delivery_id"] != json!(request.delivery_id))
        {
            return Err(ReadError::IdentityMismatch);
        }
        if name == "native_delivery_status"
            && (scope["collection"] != "native_delivery_segments" || scope["filter"] != "subject")
        {
            return Err(ReadError::InvalidObservation);
        }
        if subject.is_none() {
            let valid = if matches!(owner, Component::Absent { .. }) {
                matches!(component,Component::NotApplicable{reason,..} if reason=="task_absent")
            } else {
                matches!(component,Component::Unavailable{reason,..} if reason=="task_dependency_unavailable")
            };
            if !valid {
                return Err(ReadError::InvalidObservation);
            }
            continue;
        }
        match component {
            Component::Present {
                revision,
                value,
                completeness,
                ..
            } if name == "lifecycle" => {
                let bytes = serde_json::to_vec(value).map_err(|_| ReadError::InvalidObservation)?;
                parse_snapshot(
                    &bytes,
                    subject
                        .as_ref()
                        .and_then(Value::as_str)
                        .ok_or(ReadError::InvalidObservation)?,
                )
                .map_err(|_| ReadError::InvalidObservation)?;
                if completeness.is_some()
                    || revision["kind"] != "lifecycle"
                    || revision["cursor"] != value["cursor"]
                {
                    return Err(ReadError::InvalidObservation);
                }
                if let (Some(expected), Some(actual)) = (
                    request.delivery_id.as_deref(),
                    value
                        .pointer("/delivery/value/delivery_id")
                        .and_then(Value::as_str),
                ) && expected != actual
                {
                    return Err(ReadError::IdentityMismatch);
                }
            }
            Component::Present {
                revision,
                value,
                completeness,
                ..
            } => {
                if revision != &json!({"kind":"not_provided"})
                    || completeness.as_ref()
                        != Some(&json!({"kind":"complete_for_scope","limit":64}))
                    || scope["subject_id"].as_str().is_none_or(|subject| {
                        validate_native_delivery_status(value, subject).is_err()
                    })
                {
                    return Err(ReadError::InvalidObservation);
                }
            }
            Component::Refused { reason, limit, .. } if reason == "capacity_exceeded" => {
                if name != "native_delivery_status" || *limit != Some(64) {
                    return Err(ReadError::InvalidObservation);
                }
            }
            Component::Unavailable { .. } | Component::Refused { limit: None, .. } => {}
            _ => return Err(ReadError::InvalidObservation),
        }
    }
    for (name, kind) in [
        ("command_result", "admission_command_locator"),
        ("control_result", "control_command_locator"),
    ] {
        let component = &observation.components[name];
        let scope = component.scope();
        if scope["task_binding"] != "not_established"
            || !scope["subject_id"].is_string()
            || scope["command_id"] != json!(request.command_id)
        {
            return Err(ReadError::IdentityMismatch);
        }
        if request.command_id.is_none() {
            if !matches!(component, Component::NotRequested { .. }) {
                return Err(ReadError::InvalidObservation);
            }
            continue;
        }
        match component {
            Component::Present {
                revision,
                value,
                completeness,
                ..
            } => {
                let kind = if name == "command_result" {
                    CommandObservationKind::Admission
                } else {
                    CommandObservationKind::Control
                };
                if revision != &json!({"kind":"not_provided"})
                    || completeness.is_some()
                    || validate_command_observation(value, scope, kind).is_err()
                {
                    return Err(ReadError::InvalidObservation);
                }
            }
            Component::Absent { record_kind, .. } if record_kind == kind => {}
            Component::Unavailable { .. } | Component::Refused { limit: None, .. } => {}
            _ => return Err(ReadError::InvalidObservation),
        }
    }
    Ok(observation)
}

fn io_error(error: std::io::Error) -> ReadError {
    if matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) {
        ReadError::Deadline
    } else {
        ReadError::Unavailable
    }
}
fn remaining(deadline: Instant) -> Result<Duration, ReadError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or(ReadError::Deadline)
}

/// Each call creates exactly one bounded connection. No retry or version fallback.
pub fn read(
    endpoint: &Path,
    request: &Request,
    budget: Duration,
) -> Result<Observation, ReadError> {
    request.validate()?;
    if !endpoint.is_absolute() || budget.is_zero() || budget > Duration::from_secs(30) {
        return Err(ReadError::InvalidRequest);
    }
    let deadline = Instant::now() + budget;
    let socket = Socket::new(Domain::UNIX, Type::STREAM, None).map_err(io_error)?;
    let address = SockAddr::unix(endpoint).map_err(io_error)?;
    socket
        .connect_timeout(&address, remaining(deadline)?)
        .map_err(io_error)?;
    let fd: OwnedFd = socket.into();
    let mut stream: UnixStream = fd.into();
    let mut value = serde_json::to_value(request).map_err(|_| ReadError::InvalidRequest)?;
    value["op"] = json!("task.observe.v2");
    let mut bytes = serde_json::to_vec(&value).map_err(|_| ReadError::InvalidRequest)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_FRAME {
        return Err(ReadError::InvalidRequest);
    }
    let mut written = 0;
    while written < bytes.len() {
        stream
            .set_write_timeout(Some(remaining(deadline)?))
            .map_err(io_error)?;
        let n = stream.write(&bytes[written..]).map_err(io_error)?;
        if n == 0 {
            return Err(ReadError::Unavailable);
        }
        written += n;
    }
    let mut frame = Vec::new();
    loop {
        stream
            .set_read_timeout(Some(remaining(deadline)?))
            .map_err(io_error)?;
        let mut byte = [0];
        stream.read_exact(&mut byte).map_err(io_error)?;
        if byte[0] == b'\n' {
            break;
        }
        if frame.len() == MAX_FRAME {
            return Err(ReadError::InvalidObservation);
        }
        frame.push(byte[0]);
    }
    remaining(deadline)?;
    let result = decode(&frame, request)?;
    remaining(deadline)?;
    Ok(result)
}

/// Only an observed present component becomes an anchor; failures never become empty data.
#[derive(Clone, Debug)]
pub struct ComponentView {
    pub latest: Component,
    pub last_known: Option<Component>,
    pub stale: bool,
}
#[derive(Default)]
pub struct Selection {
    generation: u64,
    endpoint: Option<PathBuf>,
    request: Option<Request>,
    identity: Option<HostIdentity>,
    pub components: BTreeMap<String, ComponentView>,
    pub transport_error: Option<ReadError>,
}
impl Selection {
    pub fn begin(&mut self, endpoint: PathBuf, request: Request) -> u64 {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("selection generation exhausted");
        if self.endpoint.as_ref() != Some(&endpoint) || self.request.as_ref() != Some(&request) {
            self.components.clear();
            self.identity = None;
        }
        self.endpoint = Some(endpoint);
        self.request = Some(request);
        self.transport_error = None;
        self.generation
    }
    pub fn apply(&mut self, generation: u64, result: Result<Observation, ReadError>) -> bool {
        if generation != self.generation {
            return false;
        }
        let observation = match result {
            Ok(value) => value,
            Err(error) => {
                self.transport_error = Some(error);
                for c in self.components.values_mut() {
                    c.stale = true;
                }
                return true;
            }
        };
        if self.request.as_ref() != Some(&observation.request) {
            return false;
        }
        if self.identity.as_ref() != Some(&observation.host_identity) {
            self.components.clear();
        }
        self.identity = Some(observation.host_identity);
        self.transport_error = None;
        let established_subject = match &observation.components["owner"] {
            Component::Present { scope, .. } => scope.get("subject_id").cloned(),
            _ => None,
        };
        for (name, component) in observation.components {
            let failed = matches!(
                component,
                Component::Unavailable { .. } | Component::Refused { .. }
            );
            let known = if matches!(component, Component::Present { .. }) {
                Some(component.clone())
            } else if failed {
                self.components
                    .get(&name)
                    .and_then(|prior| prior.last_known.as_ref())
                    .filter(|anchor| {
                        anchor_compatible(&name, anchor, &component, established_subject.as_ref())
                    })
                    .cloned()
            } else {
                None
            };
            self.components.insert(
                name,
                ComponentView {
                    latest: component,
                    last_known: known,
                    stale: failed,
                },
            );
        }
        true
    }
}

fn anchor_compatible(
    name: &str,
    anchor: &Component,
    latest: &Component,
    established_subject: Option<&Value>,
) -> bool {
    let prior = anchor.scope();
    let scope = latest.scope();
    if matches!(name, "owner" | "lifecycle" | "native_delivery_status")
        && established_subject.is_some_and(|subject| prior.get("subject_id") != Some(subject))
    {
        return false;
    }
    if prior == scope {
        return true;
    }
    if established_subject.is_some() {
        return false;
    }
    if name == "owner" {
        return prior.get("task_id") == scope.get("task_id") && scope.get("subject_id").is_none();
    }
    if matches!(name, "lifecycle" | "native_delivery_status")
        && matches!(latest, Component::Unavailable { reason, .. } if reason == "task_dependency_unavailable")
        && scope.get("subject_id") == Some(&Value::Null)
    {
        let mut original = prior.clone();
        original["subject_id"] = Value::Null;
        return original == *scope;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    fn absent() -> (Request, Value) {
        let request = Request {
            task_id: "task:test".into(),
            delivery_id: None,
            command_id: None,
        };
        let raw = json!({"protocol_version":2,"status":"observation","consistency":"independent_components","request":{"task_id":"task:test"},"host_identity":{"pid":1,"linux_starttime_ticks":"1","incarnation":"host:1","build_id":"build:1","profile":"retained-repository-analysis-v1","executable_sha256":"a".repeat(64),"source_sha256":"b".repeat(64),"lock_sha256":"c".repeat(64),"toolchain_sha256":"d".repeat(64),"features":[],"provider_identity":"test"},"components":{"owner":{"state":"absent","scope":{"task_id":"task:test"},"record_kind":"task"},"lifecycle":{"state":"not_applicable","scope":{"subject_id":null,"delivery_id":null},"reason":"task_absent"},"native_delivery_status":{"state":"not_applicable","scope":{"subject_id":null,"collection":"native_delivery_segments","filter":"subject"},"reason":"task_absent"},"command_result":{"state":"not_requested","scope":{"subject_id":"subject:test","command_id":null,"task_binding":"not_established"}},"control_result":{"state":"not_requested","scope":{"subject_id":"subject:test","command_id":null,"task_binding":"not_established"}}}});
        (request, raw)
    }
    #[test]
    fn absence_is_typed_and_unknown_or_null_is_not_empty() {
        let (request, mut raw) = absent();
        assert!(decode(&serde_json::to_vec(&raw).unwrap(), &request).is_ok());
        raw["request"]["delivery_id"] = Value::Null;
        assert_eq!(
            decode(&serde_json::to_vec(&raw).unwrap(), &request),
            Err(ReadError::InvalidObservation)
        );
        raw["request"]
            .as_object_mut()
            .unwrap()
            .remove("delivery_id");
        raw["components"]["owner"]["state"] = json!("future");
        assert_eq!(
            decode(&serde_json::to_vec(&raw).unwrap(), &request),
            Err(ReadError::UnsupportedState)
        );
        raw["protocol_version"] = json!(3);
        assert_eq!(
            decode(&serde_json::to_vec(&raw).unwrap(), &request),
            Err(ReadError::UnsupportedVersion)
        );
    }
    #[test]
    fn stale_generation_and_identity_change_clear_anchors() {
        let (request, raw) = absent();
        let observation = decode(&serde_json::to_vec(&raw).unwrap(), &request).unwrap();
        let mut selection = Selection::default();
        let old = selection.begin("/endpoint".into(), request.clone());
        let current = selection.begin("/endpoint".into(), request.clone());
        assert!(!selection.apply(old, Ok(observation.clone())));
        assert!(selection.apply(current, Ok(observation.clone())));
        assert!(selection.apply(current, Err(ReadError::Unavailable)));
        assert!(selection.components.values().all(|c| c.stale));
        let mut changed = observation;
        changed.host_identity.incarnation = "host:2".into();
        assert!(selection.apply(current, Ok(changed)));
        assert!(selection.components.values().all(|c| !c.stale));
    }
    #[test]
    fn frame_and_budget_are_bounded() {
        let (request, _) = absent();
        assert_eq!(
            decode(&vec![b' '; MAX_FRAME + 1], &request),
            Err(ReadError::InvalidObservation)
        );
        assert_eq!(
            read(Path::new("/not-used"), &request, Duration::ZERO),
            Err(ReadError::InvalidRequest)
        );
    }
    fn native_fixture() -> (Request, Value) {
        let (request, mut raw) = absent();
        raw["components"]["owner"] = json!({"state":"present","scope":{"task_id":"task:test","subject_id":"subject:test"},"revision":{"kind":"task","owner_revision":7,"cursor_revision":3},"value":{"task_id":"task:test","subject_id":"subject:test","revision":7,"cursor_revision":3}});
        raw["components"]["lifecycle"] = json!({"state":"unavailable","scope":{"subject_id":"subject:test","delivery_id":null},"reason":"lifecycle_read_unavailable"});
        raw["components"]["native_delivery_status"] = json!({"state":"present","scope":{"subject_id":"subject:test","collection":"native_delivery_segments","filter":"subject"},"revision":{"kind":"not_provided"},"value":{"protocol_version":1,"subject_id":"subject:test","recovery_only":false,"segments":[]},"completeness":{"kind":"complete_for_scope","limit":64}});
        (request, raw)
    }
    #[test]
    fn native_empty_64_65_capacity_and_failure_remain_distinct() {
        let (request, mut raw) = native_fixture();
        let decode_raw = |r: &Value| decode(&serde_json::to_vec(r).unwrap(), &request);
        assert!(decode_raw(&raw).is_ok());
        let segment = json!({"delivery_id":"d","input_id":"i","sequence":1,"delivery_state":"active","effect_id":"e","queue_attempt_id":"qa","segment_no":1,"invocation_id":"inv","native_attempt_id":"na","predecessor_invocation_id":null,"transition_id":null,"segment_state":"completed","native_state":"completed","send_intent":true,"thread_id":"th","turn_id":"tu","context_generation":"g","final_sha256":null,"history_status":"complete"});
        raw["components"]["native_delivery_status"]["value"]["segments"] =
            json!(vec![segment.clone(); 64]);
        assert!(decode_raw(&raw).is_ok());
        raw["components"]["native_delivery_status"]["value"]["segments"] = json!(vec![segment; 65]);
        assert_eq!(decode_raw(&raw), Err(ReadError::InvalidObservation));
        let scope = raw["components"]["native_delivery_status"]["scope"].clone();
        raw["components"]["native_delivery_status"] =
            json!({"state":"refused","scope":scope,"reason":"capacity_exceeded","limit":64});
        assert!(decode_raw(&raw).is_ok());
        raw["components"]["native_delivery_status"]["limit"] = json!(65);
        assert_eq!(decode_raw(&raw), Err(ReadError::InvalidObservation));
        raw["components"]["native_delivery_status"] =
            json!({"state":"unavailable","scope":scope,"reason":"native_read_unavailable"});
        assert!(decode_raw(&raw).is_ok());
        raw["components"]["owner"]["revision"]["owner_revision"] = json!(8);
        assert_eq!(decode_raw(&raw), Err(ReadError::IdentityMismatch));
    }
    #[test]
    fn missing_scope_null_payload_and_false_atomicity_are_invalid() {
        let (request, raw) = absent();
        for pointer in [
            "/components/owner/scope",
            "/components/lifecycle/scope/subject_id",
        ] {
            let mut bad = raw.clone();
            *bad.pointer_mut(pointer).unwrap() = json!(4);
            assert!(decode(&serde_json::to_vec(&bad).unwrap(), &request).is_err());
        }
        let mut bad = raw.clone();
        bad["components"]["owner"]["value"] = Value::Null;
        assert!(decode(&serde_json::to_vec(&bad).unwrap(), &request).is_err());
        let mut bad = raw;
        bad["consistency"] = json!("atomic");
        assert!(decode(&serde_json::to_vec(&bad).unwrap(), &request).is_err());
    }
    #[test]
    fn command_locator_absence_differs_from_unknown_and_rejected_outcome() {
        let (mut request, mut raw) = absent();
        request.command_id = Some("command:test".into());
        raw["request"] = serde_json::to_value(&request).unwrap();
        for (name, kind) in [
            ("command_result", "admission_command_locator"),
            ("control_result", "control_command_locator"),
        ] {
            raw["components"][name] = json!({"state":"absent","scope":{"subject_id":"subject:test","command_id":"command:test","task_binding":"not_established"},"record_kind":kind});
        }
        assert!(decode(&serde_json::to_vec(&raw).unwrap(), &request).is_ok());
        for value in [
            json!({"status":"unknown"}),
            json!({"protocol_version":2,"command_id":"command:test","accepted":false,"outcome_kind":"rejected","outcome_ref":null,"rejection_code":"rejected","resulting_revision":"1"}),
        ] {
            raw["components"]["command_result"] = json!({"state":"present","scope":{"subject_id":"subject:test","command_id":"command:test","task_binding":"not_established","request_digest":"a".repeat(64)},"revision":{"kind":"not_provided"},"value":value});
            assert!(decode(&serde_json::to_vec(&raw).unwrap(), &request).is_ok());
        }
        raw["components"]["command_result"]["scope"]["task_binding"] = json!("established");
        assert!(decode(&serde_json::to_vec(&raw).unwrap(), &request).is_err());
    }
    #[test]
    fn failed_refresh_keeps_scoped_last_known_but_changed_selection_does_not() {
        let (request, raw) = native_fixture();
        let known = decode(&serde_json::to_vec(&raw).unwrap(), &request).unwrap();
        let mut selection = Selection::default();
        let first = selection.begin("/a".into(), request.clone());
        selection.apply(first, Ok(known.clone()));
        let mut failed = known;
        failed.components.insert(
            "native_delivery_status".into(),
            Component::Unavailable {
                scope: raw["components"]["native_delivery_status"]["scope"].clone(),
                reason: "failed".into(),
            },
        );
        let refresh = selection.begin("/a".into(), request.clone());
        selection.apply(refresh, Ok(failed));
        assert!(selection.components["native_delivery_status"].stale);
        assert!(
            selection.components["native_delivery_status"]
                .last_known
                .is_some()
        );
        selection.begin("/b".into(), request);
        assert!(selection.components.is_empty());
    }
    #[test]
    fn partial_and_stalled_socket_frames_do_not_become_observations() {
        use std::{
            fs,
            os::unix::net::UnixListener,
            sync::atomic::{AtomicU64, Ordering},
            thread,
        };
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "to2-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("s");
        let listener = UnixListener::bind(&path).unwrap();
        let (request, _) = absent();
        let writer = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.write_all(b"{\"protocol_version\":2").unwrap();
        });
        assert_eq!(
            read(&path, &request, Duration::from_secs(1)),
            Err(ReadError::Unavailable)
        );
        writer.join().unwrap();
        fs::remove_file(&path).unwrap();
        let listener = UnixListener::bind(&path).unwrap();
        let writer = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_millis(150));
        });
        assert_eq!(
            read(&path, &request, Duration::from_millis(30)),
            Err(ReadError::Deadline)
        );
        writer.join().unwrap();
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&dir).unwrap();
    }

    fn with_lifecycle() -> (Request, Value) {
        let (request, mut raw) = native_fixture();
        let cursor =
            json!({"store_id":"store:test","stream_id":"stream:test","commit_sequence":"1"});
        raw["components"]["lifecycle"] = json!({"state":"present","scope":{"subject_id":"subject:test","delivery_id":null},"revision":{"kind":"lifecycle","cursor":cursor},"value":{"protocol_version":1,"subject_id":"subject:test","context_generation":"g","cursor":cursor,"transition":{"state":"unknown"},"current_admission":{"kind":"unknown"},"delivery":{"state":"unknown"},"runtime_availability":"unknown","runtime_provenance":{"state":"unknown"},"blocking_reason":{"state":"unknown"},"authority_refs":[],"evidence_refs":[],"measurement":{"state":"unknown"}}});
        (request, raw)
    }
    #[test]
    fn owner_failures_preserve_original_scopes_and_changed_subject_cannot_reuse_them() {
        let (request, raw) = with_lifecycle();
        let decode_raw = |v: &Value| decode(&serde_json::to_vec(v).unwrap(), &request).unwrap();
        let known = decode_raw(&raw);
        for state in ["unavailable", "refused"] {
            let mut selection = Selection::default();
            let generation = selection.begin("/a".into(), request.clone());
            selection.apply(generation, Ok(known.clone()));
            let mut failed = raw.clone();
            failed["components"]["owner"] =
                json!({"state":state,"scope":{"task_id":"task:test"},"reason":"store_failure"});
            for name in ["lifecycle", "native_delivery_status"] {
                let mut scope = raw["components"][name]["scope"].clone();
                scope["subject_id"] = Value::Null;
                failed["components"][name] = json!({"state":"unavailable","scope":scope,"reason":"task_dependency_unavailable"});
            }
            for _ in 0..2 {
                selection.apply(generation, Ok(decode_raw(&failed)));
                for name in ["owner", "lifecycle", "native_delivery_status"] {
                    let view = &selection.components[name];
                    assert!(view.stale);
                    assert_eq!(
                        view.last_known.as_ref().unwrap().scope()["subject_id"],
                        "subject:test"
                    );
                    assert_eq!(
                        view.latest
                            .scope()
                            .get("subject_id")
                            .unwrap_or(&Value::Null),
                        &Value::Null
                    );
                }
            }
            selection.apply(generation, Ok(known.clone()));
            assert!(selection.components.values().all(|v| !v.stale));
            let mut changed = raw.clone();
            changed["components"]["owner"]["scope"]["subject_id"] = json!("subject:other");
            changed["components"]["owner"]["value"]["subject_id"] = json!("subject:other");
            for name in ["lifecycle", "native_delivery_status"] {
                let mut scope = raw["components"][name]["scope"].clone();
                scope["subject_id"] = json!("subject:other");
                changed["components"][name] =
                    json!({"state":"unavailable","scope":scope,"reason":"read_failed"});
            }
            selection.apply(generation, Ok(decode_raw(&changed)));
            assert!(selection.components["lifecycle"].last_known.is_none());
            assert!(
                selection.components["native_delivery_status"]
                    .last_known
                    .is_none()
            );
            let (_, absent) = absent();
            selection.apply(generation, Ok(decode_raw(&absent)));
            assert!(
                selection
                    .components
                    .values()
                    .all(|v| v.last_known.is_none())
            );
            selection.apply(generation, Ok(known.clone()));
            let mut host_changed = decode_raw(&failed);
            host_changed.host_identity.incarnation = "other".into();
            selection.apply(generation, Ok(host_changed));
            assert!(
                selection
                    .components
                    .values()
                    .all(|v| v.last_known.is_none())
            );
            selection.apply(generation, Ok(known.clone()));
            let new_generation = selection.begin("/b".into(), request.clone());
            assert!(!selection.apply(generation, Ok(known.clone())));
            assert!(selection.components.is_empty());
            let mut request_changed = request.clone();
            request_changed.task_id = "task:other".into();
            selection.begin("/b".into(), request_changed);
            assert!(!selection.apply(new_generation, Ok(known.clone())));
        }
    }
    #[test]
    fn malformed_command_frames_never_install_anchors() {
        let (mut request, mut raw) = absent();
        request.command_id = Some("command:test".into());
        raw["request"] = serde_json::to_value(&request).unwrap();
        raw["components"]["control_result"] = json!({"state":"absent","scope":{"subject_id":"subject:test","command_id":"command:test","task_binding":"not_established"},"record_kind":"control_command_locator"});
        for value in [
            json!({"protocol_version":999,"command_id":"command:test","accepted":true}),
            json!({"status":"unknown","command_id":"other"}),
            json!({"protocol_version":2,"command_id":"command:test","accepted":"yes"}),
        ] {
            raw["components"]["command_result"] = json!({"state":"present","scope":{"subject_id":"subject:test","command_id":"command:test","task_binding":"not_established","request_digest":"a".repeat(64)},"revision":{"kind":"not_provided"},"value":value});
            let result = decode(&serde_json::to_vec(&raw).unwrap(), &request);
            assert!(result.is_err());
            let mut selection = Selection::default();
            let g = selection.begin("/a".into(), request.clone());
            selection.apply(g, result);
            assert!(selection.components.is_empty());
        }
    }
}
