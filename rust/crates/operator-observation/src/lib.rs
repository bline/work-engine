//! Bounded presentation of session-owner observations; no task or execution authority.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

pub const SCHEMA: &str = "work-engine.operator-observation.v1";
pub const MAX_BYTES: usize = 1024 * 1024;
pub fn read_snapshot_file(path: &str) -> Result<Vec<u8>, &'static str> {
    if !std::path::Path::new(path).is_absolute() {
        return Err("snapshot_path_must_be_absolute");
    }
    session_query::read_file(path, MAX_BYTES, true)
}
const ROLES: &[&str] = &[
    "user",
    "assistant",
    "tool_call",
    "tool_result",
    "agent_message",
    "turn_end",
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema: String,
    pub scope_id: String,
    pub source_id: String,
    pub label: String,
    pub observed_at_ms: u64,
    pub sessions: Vec<Session>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub session_id: String,
    pub observed_at_ms: u64,
    pub observation: Observation,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observation {
    Available {
        host: String,
        #[serde(deserialize_with = "Option::deserialize")]
        title: Option<String>,
        entries: Vec<Entry>,
    },
    Unavailable {
        code: String,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub role: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub tool: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub timestamp: Option<String>,
    pub text: String,
    pub truncated: bool,
    pub redactions: u64,
}
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn valid_id(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}
pub fn validate_selection(label: &str, ids: &[String]) -> Result<(), &'static str> {
    let unique: HashSet<_> = ids.iter().collect();
    if label.trim().is_empty()
        || label.chars().count() > 200
        || ids.is_empty()
        || ids.len() > 20
        || unique.len() != ids.len()
        || ids.iter().any(|s| !valid_id(s))
    {
        return Err("invalid_selection");
    }
    Ok(())
}
impl Snapshot {
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_selection(
            &self.label,
            &self
                .sessions
                .iter()
                .map(|s| s.session_id.clone())
                .collect::<Vec<_>>(),
        )?;
        if self.schema != SCHEMA
            || !hash(&self.source_id)
            || !hash(&self.scope_id)
            || self.observed_at_ms == 0
            || self.observed_at_ms > 8_640_000_000_000_000
        {
            return Err("invalid_snapshot");
        }
        let ids = self
            .sessions
            .iter()
            .map(|s| s.session_id.clone())
            .collect::<Vec<_>>();
        if self.scope_id != scope_id(&self.source_id, &self.label, &ids) {
            return Err("scope_mismatch");
        }
        for session in &self.sessions {
            if session.observed_at_ms == 0 || session.observed_at_ms > self.observed_at_ms {
                return Err("invalid_observation_time");
            }
            match &session.observation {
                Observation::Unavailable { code } => {
                    if code != "source_unavailable" && code != "invalid_owner_observation" {
                        return Err("invalid_error");
                    }
                }
                Observation::Available {
                    host,
                    title,
                    entries,
                } => {
                    if !matches!(host.as_str(), "codex" | "claude")
                        || title.as_ref().is_some_and(|s| s.chars().count() > 1000)
                        || entries.len() > 10
                    {
                        return Err("invalid_observation");
                    }
                    let mut seen = HashSet::new();
                    for e in entries {
                        let parts = e.id.split('.').collect::<Vec<_>>();
                        if e.id.len() > 40
                            || parts.len() > 2
                            || parts
                                .iter()
                                .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
                            || !seen.insert(&e.id)
                            || !ROLES.contains(&e.role.as_str())
                            || e.text.chars().count() > 10000
                            || e.tool.as_ref().is_some_and(|s| s.chars().count() > 256)
                            || e.timestamp
                                .as_ref()
                                .is_some_and(|s| s.chars().count() > 128)
                            || e.redactions > 1_000_000
                        {
                            return Err("invalid_entry");
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > MAX_BYTES {
            return Err("snapshot_too_large");
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| "invalid_snapshot")?;
        value.validate()?;
        Ok(value)
    }
}
pub fn scope_id(source: &str, label: &str, ids: &[String]) -> String {
    let mut ids = ids.to_vec();
    ids.sort();
    digest(&serde_json::to_vec(&(source, label, ids)).expect("string encoding"))
}
fn owner_observation(id: &str, value: Value) -> Result<Observation, &'static str> {
    if value.get("sessionId").and_then(Value::as_str) != Some(id) {
        return Err("invalid_owner_observation");
    }
    let entries = value["entries"]
        .as_array()
        .ok_or("invalid_owner_observation")?
        .iter()
        .map(|e| {
            Ok(Entry {
                id: e["entryId"]
                    .as_str()
                    .ok_or("invalid_owner_observation")?
                    .into(),
                role: e["role"]
                    .as_str()
                    .ok_or("invalid_owner_observation")?
                    .into(),
                tool: serde_json::from_value(e["tool"].clone())
                    .map_err(|_| "invalid_owner_observation")?,
                timestamp: serde_json::from_value(e["timestamp"].clone())
                    .map_err(|_| "invalid_owner_observation")?,
                text: e["text"]
                    .as_str()
                    .ok_or("invalid_owner_observation")?
                    .into(),
                truncated: e["textTruncated"]
                    .as_bool()
                    .ok_or("invalid_owner_observation")?,
                redactions: e["redactions"]
                    .as_u64()
                    .ok_or("invalid_owner_observation")?,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    Ok(Observation::Available {
        host: value["host"]
            .as_str()
            .ok_or("invalid_owner_observation")?
            .into(),
        title: serde_json::from_value(value["sessionTitle"].clone())
            .map_err(|_| "invalid_owner_observation")?,
        entries,
    })
}
/// Query capability accepts only explicit read_session requests. Failures stay per session.
pub fn collect(
    source_id: &str,
    label: &str,
    ids: &[String],
    mut query: impl FnMut(Value) -> Result<Value, &'static str>,
) -> Result<Snapshot, &'static str> {
    validate_selection(label, ids)?;
    if !hash(source_id) {
        return Err("invalid_source_id");
    }
    let mut sessions = Vec::new();
    for id in ids {
        let observation =
            match query(json!({"session_id":id,"tail":10,"max_chars":2000,"roles":ROLES})) {
                Ok(value) => owner_observation(id, value).unwrap_or(Observation::Unavailable {
                    code: "invalid_owner_observation".into(),
                }),
                Err(_) => Observation::Unavailable {
                    code: "source_unavailable".into(),
                },
            };
        let mut session = Session {
            session_id: id.clone(),
            observed_at_ms: now_ms(),
            observation,
        };
        // Validate the complete bounded owner payload before making it available.
        let candidate = Snapshot {
            schema: SCHEMA.into(),
            scope_id: scope_id(source_id, label, std::slice::from_ref(id)),
            source_id: source_id.into(),
            label: label.into(),
            observed_at_ms: session.observed_at_ms,
            sessions: vec![session.clone()],
        };
        if candidate.validate().is_err() {
            session.observation = Observation::Unavailable {
                code: "invalid_owner_observation".into(),
            };
        }
        sessions.push(session);
    }
    let snapshot = Snapshot {
        schema: SCHEMA.into(),
        scope_id: scope_id(source_id, label, ids),
        source_id: source_id.into(),
        label: label.into(),
        observed_at_ms: now_ms(),
        sessions,
    };
    snapshot.validate()?;
    Ok(snapshot)
}

/// Plain terminal output must not interpret transcript control sequences.
pub fn terminal_text(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
        .collect()
}

pub mod selected_task;
pub mod task;
