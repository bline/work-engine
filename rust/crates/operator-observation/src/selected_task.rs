//! Versioned private presentation of independent TO2 reads and exact session links.
//! Domain decoding and joins happen here, never in a JavaScript consumer.
use crate::task::{self, Component, Request, Selection};
use crate::{Session, Snapshot, digest, now_ms, terminal_text, valid_id};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

#[path = "selected_task_summary.rs"]
mod summary;
pub use summary::*;
pub const SCHEMA: &str = "work-engine.selected-task.v2";
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectedTask {
    pub schema: String,
    pub endpoint: String,
    pub request: Request,
    pub source_id: String,
    pub scope_id: String,
    pub standing: String,
    pub confidentiality: String,
    pub assembled_at_ms: u64,
    pub host_identity: Option<task::HostIdentity>,
    pub transport: String,
    #[schemars(schema_with = "components_schema")]
    pub components: BTreeMap<String, Presentation>,
    pub activity: Vec<Session>,
    pub activity_coverage: ActivityCoverage,
    pub activity_source_id: Option<String>,
    pub linkage: String,
    pub summary: Summary,
}
/// Coverage of the independently bounded Events read, never native completeness.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActivityCoverage {
    pub state: String,
    pub linked_session_ids: Vec<String>,
    pub limit: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Presentation {
    pub state: String,
    pub scope: Value,
    pub revision: Value,
    pub completeness: Value,
    pub read_at_ms: u64,
    pub last_known_at_ms: Option<u64>,
    pub stale: bool,
    pub reason: String,
    pub rows: Vec<String>,
    pub last_known_scope: Value,
    pub last_known_revision: Value,
    pub last_known_completeness: Value,
}
fn components_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    let presentation = generator.subschema_for::<Presentation>();
    let decimal = json!({"type":"string", "pattern":"^(0|[1-9][0-9]*)$", "maxLength":20, "description":"Canonical ASCII u64 decimal, at most 18446744073709551615; runtime checks upper bound."});
    let revision = json!({"type":"object", "additionalProperties":false, "required":["kind","owner_revision","cursor_revision"], "properties":{"kind":{"const":"task"},"owner_revision":decimal,"cursor_revision":decimal}});
    let scope = json!({"type":"object","additionalProperties":false,"required":["task_id"],"properties":{"task_id":{"type":"string"},"subject_id":{"type":["string","null"]}}});
    let owner = json!({"allOf":[presentation, {"type":"object","properties":{
        "scope":{"anyOf":[scope,{"type":"object","maxProperties":0,"description":"Only exact unobserved zero-time unavailable sentinel; runtime checks all relations."}]},
        "revision":{"anyOf":[revision,{"type":"null"}]},"completeness":{"type":"null"},
        "last_known_scope":{"anyOf":[scope,{"type":"null"}]},
        "last_known_revision":{"anyOf":[revision,{"type":"null"}]},"last_known_completeness":{"type":"null"}
    }}]});
    serde_json::from_value(json!({"type":"object", "additionalProperties":false,
        "required":task::COMPONENTS, "properties":{"owner":owner,"lifecycle":presentation,"native_delivery_status":presentation,"command_result":presentation,"control_result":presentation}})).expect("static schema")
}
// Only qualified numeric TO2 owner metadata is mapped; extensions are refused, never discarded.
fn owner_wire_revision(value: &Value) -> Result<Value, &'static str> {
    if value.is_null() { return Ok(Value::Null); }
    let object = value.as_object().ok_or("unsupported_owner_metadata")?;
    if object.len() != 3 || value["kind"] != "task" { return Err("unsupported_owner_metadata"); }
    let owner = value["owner_revision"].as_u64().ok_or("unsupported_owner_metadata")?;
    let cursor = value["cursor_revision"].as_u64().ok_or("unsupported_owner_metadata")?;
    Ok(json!({"kind":"task", "owner_revision":owner.to_string(), "cursor_revision":cursor.to_string()}))
}
pub(super) fn owner_revision_valid(value: &Value) -> bool {
    value.as_object().is_some_and(|o| o.len() == 3)
        && value["kind"] == "task"
        && ["owner_revision", "cursor_revision"].iter().all(|key| value[*key].as_str().is_some_and(|s|
            !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
            && (s == "0" || !s.starts_with('0')) && s.parse::<u64>().is_ok()))
}
pub(super) fn owner_scope_valid(value: &Value, task_id: &str) -> bool {
    value.as_object().is_some_and(|o| o.keys().all(|k| matches!(k.as_str(), "task_id" | "subject_id")))
        && value["task_id"] == task_id
        && value.get("subject_id").is_none_or(|v| v.is_null() || v.is_string())
}
/// Consumer-clock display policy only; it does not expire an owner's record.
pub fn age_label(at: u64, now: u64) -> String {
    if at == 0 || at > now {
        return "Observation time unavailable or in the future · stale".into();
    }
    let seconds = (now - at) / 1000;
    format!(
        "Observed {seconds}s ago{}",
        if seconds > 120 { " · stale" } else { "" }
    )
}
fn safe_value(value: &Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| {
                    let normalized = key.to_ascii_lowercase().replace('-', "_");
                    let secret = [
                        "password",
                        "secret",
                        "token",
                        "api_key",
                        "authorization",
                        "credential",
                    ]
                    .iter()
                    .any(|name| normalized.contains(name));
                    (
                        key.clone(),
                        if secret {
                            Value::String("[REDACTED]".into())
                        } else {
                            safe_value(value)
                        },
                    )
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(safe_value).collect()),
        Value::String(text) => Value::String(safe_text(text)),
        _ => value.clone(),
    }
}
/// Finite assignment lexer; private defense in depth, never public declassification.
/// Keys are ASCII identifiers (alphanumeric, underscore, hyphen), optionally quoted.
/// Matching is ASCII case-insensitive, hyphen -> underscore, sensitive-family substring.
/// Values: complete escaped single/double quote, or token through whitespace , ; & } ].
/// Unterminated quoted values consume through end. Bearer consumes its credential too.
pub fn safe_text(text: &str) -> String {
    let chars: Vec<char> = terminal_text(text).chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let boundary = i == 0 || !key_char(chars[i - 1]);
        if boundary {
            let key = if matches!(chars[i], '\'' | '"') {
                let end = quoted_end(&chars, i);
                if end > i + 1 && chars.get(end - 1) == Some(&chars[i]) {
                    Some((chars[i + 1..end - 1].iter().collect::<String>(), end))
                } else {
                    None
                }
            } else if key_char(chars[i]) {
                let mut end = i;
                while end < chars.len() && key_char(chars[end]) {
                    end += 1;
                }
                Some((chars[i..end].iter().collect::<String>(), end))
            } else {
                None
            };
            if let Some((key, end)) = key {
                let mut separator = end;
                while separator < chars.len() && chars[separator].is_whitespace() {
                    separator += 1;
                }
                if key.chars().all(key_char)
                    && sensitive_key(&key)
                    && chars.get(separator).is_some_and(|c| matches!(c, ':' | '='))
                {
                    let mut value = separator + 1;
                    while value < chars.len() && chars[value].is_whitespace() {
                        value += 1;
                    }
                    out.extend(chars[i..value].iter());
                    out.push_str("[REDACTED]");
                    i = value_end(&chars, value);
                    continue;
                }
                if key.eq_ignore_ascii_case("bearer") && separator > end && separator < chars.len()
                {
                    out.extend(chars[i..end].iter());
                    out.push_str(" [REDACTED]");
                    i = plain_value_end(&chars, separator);
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    // Preserve the existing finite provider-token recognition across full tokens.
    out.split_whitespace()
        .map(|word| {
            let lower = word.to_ascii_lowercase();
            if ["sk-", "ghp_", "github_pat_", "xoxb-", "xoxp-", "eyj"]
                .iter()
                .any(|p| lower.contains(p))
            {
                "[REDACTED]".to_owned()
            } else {
                word.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
fn sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace('-', "_");
    [
        "password",
        "secret",
        "token",
        "api_key",
        "authorization",
        "credential",
    ]
    .iter()
    .any(|k| normalized.contains(k))
}
fn key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-')
}
fn quoted_end(chars: &[char], start: usize) -> usize {
    let quote = chars[start];
    let mut i = start + 1;
    while i < chars.len() {
        if chars[i] == '\\' {
            i = (i + 2).min(chars.len());
        } else if chars[i] == quote {
            return i + 1;
        } else {
            i += 1;
        }
    }
    chars.len()
}
fn plain_value_end(chars: &[char], start: usize) -> usize {
    if chars.get(start).is_some_and(|c| matches!(c, '\'' | '"')) {
        return quoted_end(chars, start);
    }
    let mut end = start;
    while end < chars.len()
        && !chars[end].is_whitespace()
        && !matches!(chars[end], ',' | ';' | '&' | '}' | ']')
    {
        end += 1;
    }
    end
}
fn value_end(chars: &[char], start: usize) -> usize {
    let mut end = start;
    while end < chars.len() && key_char(chars[end]) {
        end += 1;
    }
    if chars[start..end]
        .iter()
        .collect::<String>()
        .eq_ignore_ascii_case("bearer")
        && chars.get(end).is_some_and(|c| c.is_whitespace())
    {
        while end < chars.len() && chars[end].is_whitespace() {
            end += 1;
        }
        return plain_value_end(chars, end);
    }
    plain_value_end(chars, start)
}
fn rows(name: &str, c: &Component) -> Vec<String> {
    let Some(value) = c.value() else {
        return vec![];
    };
    let mut out = Vec::new();
    match name {
        "owner" => {
            for key in ["objective", "engineering_question"] {
                if let Some(s) = value[key].as_str() {
                    out.push(format!("{key}: {}", safe_text(s)));
                }
            }
            if let Some(questions) = value["questions"].as_array() {
                for q in questions.iter().take(64) {
                    out.push(format!(
                        "Question {}: {} · {}",
                        safe_text(q["id"].as_str().unwrap_or("unprovided")),
                        safe_text(q["text"].as_str().unwrap_or("unprovided")),
                        safe_value(&q["state"])
                    ));
                }
            }
            if !value["result"].is_null() {
                out.push(format!("Result: {}", safe_value(&value["result"])));
            }
        }
        "native_delivery_status" => {
            if let Some(segments) = value["segments"].as_array() {
                out.push(format!(
                    "{} native segments (complete subject scope; limit 64)",
                    segments.len()
                ));
                for row in segments {
                    out.push(safe_value(row).to_string());
                }
            }
        }
        _ => out.push(safe_value(value).to_string()),
    }
    out
}
/// Exact input membership is required; subject equality and transcript prose are insufficient.
pub fn linked_sessions(selection: &Selection) -> Vec<String> {
    let Some(owner) = selection
        .components
        .get("owner")
        .filter(|c| !c.stale)
        .and_then(|c| c.latest.value())
    else {
        return vec![];
    };
    let Some(inputs) = owner["inputs"].as_array() else {
        return vec![];
    };
    let Some(native) = selection
        .components
        .get("native_delivery_status")
        .filter(|c| !c.stale)
        .and_then(|c| c.latest.value())
    else {
        return vec![];
    };
    let mut ids = Vec::new();
    if let Some(segments) = native["segments"].as_array() {
        for segment in segments {
            let input = segment["input_id"].as_str();
            if input.is_none() || !inputs.iter().any(|i| i["input_id"].as_str() == input) {
                continue;
            }
            if selection
                .request()
                .and_then(|r| r.delivery_id.as_deref())
                .is_some_and(|d| segment["delivery_id"] != d)
            {
                continue;
            }
            if let Some(id) = segment["thread_id"].as_str().filter(|s| valid_id(s))
                && !ids.iter().any(|i| i == id)
            {
                ids.push(id.to_owned());
            }
        }
    }
    ids.sort();
    ids
}
impl SelectedTask {
    pub fn project(
        selection: &Selection,
        source_id: &str,
        activity: Option<&Snapshot>,
        replay: bool,
    ) -> Result<Self, &'static str> {
        Self::project_requested(selection, source_id, activity, replay, activity.is_some())
    }
    pub fn project_requested(
        selection: &Selection,
        source_id: &str,
        activity: Option<&Snapshot>,
        replay: bool,
        activity_requested: bool,
    ) -> Result<Self, &'static str> {
        let request = selection.request().ok_or("selection_missing")?.clone();
        let endpoint = selection
            .endpoint()
            .ok_or("selection_missing")?
            .to_string_lossy()
            .into_owned();
        let mut components = BTreeMap::new();
        for name in task::COMPONENTS {
            if let Some(view) = selection.components.get(name) {
                let raw = serde_json::to_value(&view.latest).map_err(|_| "invalid_projection")?;
                let anchor = view.last_known.as_ref();
                components.insert(
                    name.into(),
                    Presentation {
                        state: raw["state"].as_str().unwrap_or("unavailable").into(),
                        scope: view.latest.scope().clone(),
                        revision: if name == "owner" { owner_wire_revision(&raw["revision"])? } else { raw["revision"].clone() },
                        completeness: raw["completeness"].clone(),
                        read_at_ms: view.read_at_ms,
                        last_known_at_ms: view.last_known_at_ms,
                        stale: view.stale,
                        reason: raw["reason"]
                            .as_str()
                            .or(raw["record_kind"].as_str())
                            .unwrap_or("")
                            .into(),
                        rows: if view.stale {
                            anchor.map(|c| rows(name, c)).unwrap_or_default()
                        } else {
                            rows(name, &view.latest)
                        },
                        last_known_scope: anchor.map(|c| c.scope().clone()).unwrap_or(Value::Null),
                        last_known_revision: {
                            let revision = anchor.and_then(|c| serde_json::to_value(c).ok()).map(|c| c["revision"].clone()).unwrap_or(Value::Null);
                            if name == "owner" { owner_wire_revision(&revision)? } else { revision }
                        },
                        last_known_completeness: anchor
                            .and_then(|c| serde_json::to_value(c).ok())
                            .map(|c| c["completeness"].clone())
                            .unwrap_or(Value::Null),
                    },
                );
            } else {
                components.insert(
                    name.into(),
                    Presentation {
                        state: "unavailable".into(),
                        scope: json!({}),
                        revision: Value::Null,
                        completeness: Value::Null,
                        read_at_ms: 0,
                        last_known_at_ms: None,
                        stale: true,
                        reason: "transport_unavailable".into(),
                        rows: vec![],
                        last_known_scope: Value::Null,
                        last_known_revision: Value::Null,
                        last_known_completeness: Value::Null,
                    },
                );
            }
        }
        let linked = linked_sessions(selection);
        let sessions = match activity {
            Some(s) => {
                s.validate()?;
                if s.source_id != source_id {
                    return Err("activity_source_mismatch");
                }
                s.sessions
                    .iter()
                    .filter(|s| linked.contains(&s.session_id))
                    .cloned()
                    .collect()
            }
            None => vec![],
        };
        let scope_id = digest(
            &serde_json::to_vec(&json!([endpoint, request, source_id, selection.identity()]))
                .map_err(|_| "invalid_projection")?,
        );
        let result = Self {
            summary: Summary::project(selection),
            schema: SCHEMA.into(),
            endpoint,
            request,
            source_id: source_id.into(),
            scope_id,
            standing: if replay {
                "fixture_replay"
            } else {
                "new_endpoint_read"
            }
            .into(),
            confidentiality: "private_operator_data".into(),
            assembled_at_ms: now_ms(),
            host_identity: selection.identity().cloned(),
            transport: if selection.transport_error.is_some() {
                "unavailable"
            } else {
                "read_completed"
            }
            .into(),
            components,
            activity_coverage: ActivityCoverage {
                state: if linked.is_empty() {
                    "unlinked"
                } else if !activity_requested {
                    "not_requested"
                } else if linked.len() > 20 {
                    "refused_capacity"
                } else if activity.is_some() {
                    "read_completed"
                } else {
                    "unavailable"
                }
                .into(),
                linked_session_ids: linked.clone(),
                limit: 20,
            },
            activity: sessions,
            activity_source_id: activity.map(|s| s.source_id.clone()),
            linkage: if linked.is_empty() {
                "unestablished"
            } else {
                "exact_task_input_native_session; session_wide_not_per_turn"
            }
            .into(),
        };
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        self.request.validate().map_err(|_| "invalid_selection")?;
        if self.schema != SCHEMA
            || !Path::new(&self.endpoint).is_absolute()
            || self.confidentiality != "private_operator_data"
            || !matches!(
                self.standing.as_str(),
                "fixture_replay" | "new_endpoint_read"
            )
            || self.components.len() != 5
            || task::COMPONENTS
                .iter()
                .any(|k| !self.components.contains_key(*k))
            || self.source_id.len() != 64
            || self.scope_id.len() != 64
            || self.activity.len() > 20
        {
            return Err("invalid_projection");
        }
        self.summary.validate(
            &self.components["owner"],
            &self.request.task_id,
            &self.transport,
        )?;
        let coverage = &self.activity_coverage;
        let mut ids = coverage.linked_session_ids.clone();
        ids.sort();
        ids.dedup();
        if coverage.limit != 20
            || ids != coverage.linked_session_ids
            || ids.len() > 64
            || ids.iter().any(|id| !valid_id(id))
            || self.activity.iter().any(|s| !ids.contains(&s.session_id))
            || match coverage.state.as_str() {
                "unlinked" => !ids.is_empty() || !self.activity.is_empty(),
                "not_requested" | "unavailable" => ids.is_empty() || !self.activity.is_empty(),
                "refused_capacity" => ids.len() <= 20 || !self.activity.is_empty(),
                "read_completed" => {
                    ids.is_empty()
                        || ids.len() > 20
                        || self.activity.len() != ids.len()
                        || self.activity_source_id.is_none()
                }
                _ => true,
            }
        {
            return Err("invalid_activity_coverage");
        }
        let expected = digest(
            &serde_json::to_vec(&json!([
                self.endpoint,
                self.request,
                self.source_id,
                self.host_identity
            ]))
            .map_err(|_| "invalid_projection")?,
        );
        if expected != self.scope_id
            || !self
                .source_id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || !matches!(self.transport.as_str(), "unavailable" | "read_completed")
            || self.assembled_at_ms == 0
            || self.assembled_at_ms > 8_640_000_000_000_000
        {
            return Err("invalid_projection");
        }
        for c in self.components.values() {
            if !matches!(
                c.state.as_str(),
                "present"
                    | "absent"
                    | "not_requested"
                    | "not_applicable"
                    | "unavailable"
                    | "refused"
            ) || !c.scope.is_object()
                || c.rows.len() > 130
                || c.rows.iter().any(|r| r.chars().count() > 60000)
                || c.read_at_ms > self.assembled_at_ms
                || c.last_known_at_ms.is_some_and(|t| t > c.read_at_ms)
            {
                return Err("invalid_projection");
            }
        }
        if self
            .activity_source_id
            .as_ref()
            .is_some_and(|s| s != &self.source_id)
        {
            return Err("activity_source_mismatch");
        }
        if !self.activity.is_empty() {
            let ids = self
                .activity
                .iter()
                .map(|s| s.session_id.clone())
                .collect::<Vec<_>>();
            Snapshot {
                schema: crate::SCHEMA.into(),
                source_id: self.source_id.clone(),
                scope_id: crate::scope_id(&self.source_id, "Owner-linked session activity", &ids),
                label: "Owner-linked session activity".into(),
                observed_at_ms: self.assembled_at_ms,
                sessions: self.activity.clone(),
            }
            .validate()?;
        }
        Ok(())
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > crate::MAX_BYTES {
            return Err("projection_too_large");
        }
        let raw: Value = serde_json::from_slice(bytes).map_err(|_| "invalid_projection")?;
        if raw
            .get("schema")
            .and_then(Value::as_str)
            .is_some_and(|v| v != SCHEMA)
        {
            return Err("unsupported_projection_version");
        }
        let value: Self = serde_json::from_value(raw).map_err(|_| "invalid_projection")?;
        value.validate()?;
        Ok(value)
    }
}

pub const INSPECTORS: [&str; 8] = [
    "questions",
    "result",
    "owner",
    "lifecycle",
    "native_delivery_status",
    "command_result",
    "control_result",
    "activity",
];
impl SelectedTask {
    /// Shared terminal prose. Presentation only; never reconstruct facts from rows.
    pub fn human_view(&self, inspect: Option<&str>, now: u64, failed: bool) -> String {
        self.human_view_width(inspect, now, failed, 80)
    }
    /// Conservatively budget two terminal cells per scalar, without a new dependency.
    pub fn human_view_width(
        &self,
        inspect: Option<&str>,
        now: u64,
        failed: bool,
        width: usize,
    ) -> String {
        let budget = |lines: usize, prefix: usize| {
            width
                .saturating_sub(2)
                .saturating_mul(lines)
                .saturating_sub(prefix + 32)
                .checked_div(2)
                .unwrap_or(0)
                .max(1)
        };
        if let Some(route) = inspect {
            let c = &self.components["owner"];
            let retained = self.summary.basis == "last_known_owner";
            let context = format!(
                "Owner {} · {} · {}{}\nBasis scope {} · revision {} · completeness {}\nLatest component state {} · reason {} · recorded read_at_ms {} (transport failure adds no component read time)\n",
                c.state,
                self.summary.basis,
                age_label(
                    if retained {
                        c.last_known_at_ms.unwrap_or(0)
                    } else {
                        c.read_at_ms
                    },
                    now
                ),
                if failed || c.stale {
                    " · stale last known; refresh unavailable"
                } else {
                    ""
                },
                if retained {
                    &c.last_known_scope
                } else {
                    &c.scope
                },
                if retained {
                    &c.last_known_revision
                } else {
                    &c.revision
                },
                if retained {
                    &c.last_known_completeness
                } else {
                    &c.completeness
                },
                c.state,
                c.reason,
                c.read_at_ms
            );
            let body = match route {
                "questions" => self
                    .summary
                    .value
                    .as_ref()
                    .map(|v| {
                        format!(
                            "{} pending questions · {} recorded answers\n{}",
                            v.pending_count,
                            v.answered_count,
                            v.questions
                                .iter()
                                .map(question_text)
                                .collect::<Vec<_>>()
                                .join("\n\n")
                        )
                    })
                    .unwrap_or_else(|| format!("Questions unavailable: {:?}", self.summary.reason)),
                "result" => self
                    .summary
                    .value
                    .as_ref()
                    .map(|v| result_text(&v.result))
                    .unwrap_or_else(|| format!("Result unavailable: {:?}", self.summary.reason)),
                "activity" => format!(
                    "Activity read: {} · {} linked sessions · limit {}\nScope {:?}\n{}\nSession-wide, no per-turn association\n{}",
                    self.activity_coverage.state,
                    self.activity_coverage.linked_session_ids.len(),
                    self.activity_coverage.limit,
                    self.activity_coverage.linked_session_ids,
                    self.linkage,
                    serde_json::to_string_pretty(&self.activity).unwrap_or_default()
                ),
                name => self
                    .components
                    .get(name)
                    .map(|c| {
                        format!(
                            "{name} detail\n{}",
                            serde_json::to_string_pretty(c).unwrap_or_default()
                        )
                    })
                    .unwrap_or_else(|| "Unknown inspector".into()),
            };
            return if matches!(route, "questions" | "result") {
                format!("{context}{body}")
            } else {
                body
            };
        }
        let c = &self.components["owner"];
        let anchor = self.summary.basis == "last_known_owner";
        let time = if anchor {
            c.last_known_at_ms.unwrap_or(0)
        } else {
            c.read_at_ms
        };
        let mut out = format!(
            "Owner record {} · {} · {}{}\n",
            c.state,
            self.summary.basis,
            age_label(time, now),
            if failed || c.stale {
                " · stale last known; refresh unavailable"
            } else {
                ""
            }
        );
        if let Some(v) = &self.summary.value {
            out = format!(
                "Objective: {}\n{}Engineering question: {}\nQuestions: {} pending · {} answered\n",
                preview(&v.objective, budget(3, 11)),
                out,
                preview(&v.engineering_question, budget(2, 22)),
                v.pending_count,
                v.answered_count
            );
            for q in v.questions.iter().take(3) {
                out.push_str(&format!(
                    "Question {} · {}\n  {}\n",
                    preview(&q.id, budget(1, 20)),
                    match q.state {
                        QuestionState::Pending => "pending",
                        QuestionState::Answered { .. } => "answered",
                    },
                    preview(&q.text, budget(1, 2))
                ));
                if let QuestionState::Answered { answer, .. } = &q.state {
                    out.push_str(&format!("Answer: {}\n", preview(answer, budget(1, 8))));
                }
            }
            if v.questions.len() > 3 {
                out.push_str(&format!(
                    "{} more questions; inspect questions\n",
                    v.questions.len() - 3
                ));
            }
            out.push_str(&match &v.result {
                ResultSummary::NotPublished => {
                    "No result published in this owner observation\n".into()
                }
                ResultSummary::Published { body, .. } => {
                    format!("Published result: {}\n", preview(body, budget(3, 18)))
                }
            });
        } else {
            out.push_str(&format!(
                "Compact summary unavailable: {}\n",
                self.summary.reason.as_deref().unwrap_or("unknown")
            ));
        }
        out.push_str("Independent components: ");
        out.push_str(
            &self
                .components
                .iter()
                .map(|(n, c)| format!("{n}: {}", c.state))
                .collect::<Vec<_>>()
                .join(" · "),
        );
        out.push_str("\nInspect questions/result/owner/lifecycle/native_delivery_status/command_result/control_result/activity\n");
        out
    }
    pub fn owner_basis_identity(&self) -> Value {
        let c = &self.components["owner"];
        json!([
            self.scope_id,
            self.summary.basis,
            if self.summary.basis == "last_known_owner" {
                &c.last_known_scope
            } else {
                &c.scope
            },
            if self.summary.basis == "last_known_owner" {
                &c.last_known_revision
            } else {
                &c.revision
            }
        ])
    }
}
fn preview(e: &Excerpt, cap: usize) -> String {
    format!(
        "{}{}",
        e.text.chars().take(cap).collect::<String>(),
        if e.truncated {
            " … [excerpt clipped; inspect]"
        } else if e.text.chars().count() > cap {
            " … [inspect for more]"
        } else {
            ""
        }
    )
}
fn question_text(q: &QuestionSummary) -> String {
    format!(
        "Question {}: {}\nRecipient {} · scope {}\n{}",
        q.id.display(),
        q.text.display(),
        q.recipient_ref.display(),
        q.scope_ref.display(),
        match &q.state {
            QuestionState::Pending => "Pending".into(),
            QuestionState::Answered {
                answer,
                authority_ref,
            } => format!(
                "Answered: {} · authority {}",
                answer.display(),
                authority_ref.display()
            ),
        }
    )
}
fn result_text(r: &ResultSummary) -> String {
    match r {
        ResultSummary::NotPublished => "No result published in this owner observation".into(),
        ResultSummary::Published {
            result_id,
            operation_id,
            body,
            attribution,
            source_ref_count,
        } => format!(
            "Published result: {}\nResult {} · operation {}\nAttribution {} · {} source refs",
            body.display(),
            result_id.display(),
            operation_id.display(),
            attribution.display(),
            source_ref_count
        ),
    }
}
