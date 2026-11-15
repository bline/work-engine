//! Narrow read-only owner projection. No task status or session-derived success.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Excerpt {
    pub text: String,
    pub truncated: bool,
}
impl Excerpt {
    fn new(s: &str, cap: usize) -> Self {
        let clean = safe_text(s);
        Self {
            truncated: clean.chars().count() > cap,
            text: clean.chars().take(cap).collect(),
        }
    }
    fn valid(&self, cap: usize) -> bool {
        self.text.chars().count() <= cap && (!self.truncated || self.text.chars().count() == cap)
    }
    pub fn display(&self) -> String {
        format!(
            "{}{}",
            self.text,
            if self.truncated {
                " … [excerpt clipped]"
            } else {
                ""
            }
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum QuestionState {
    Pending,
    Answered {
        answer: Excerpt,
        authority_ref: Excerpt,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuestionSummary {
    pub id: Excerpt,
    pub recipient_ref: Excerpt,
    pub scope_ref: Excerpt,
    pub text: Excerpt,
    pub state: QuestionState,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResultSummary {
    NotPublished,
    Published {
        result_id: Excerpt,
        operation_id: Excerpt,
        body: Excerpt,
        attribution: Excerpt,
        source_ref_count: u32,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OwnerSummary {
    pub objective: Excerpt,
    pub engineering_question: Excerpt,
    pub questions: Vec<QuestionSummary>,
    pub pending_count: u32,
    pub answered_count: u32,
    pub result: ResultSummary,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub state: String,
    pub basis: String,
    pub reason: Option<String>,
    pub value: Option<OwnerSummary>,
}
fn field<'a>(v: &'a Value, key: &str) -> Result<&'a Value, &'static str> {
    v.as_object()
        .ok_or("malformed_owner_fields")?
        .get(key)
        .ok_or("missing_owner_fields")
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str, &'static str> {
    field(v, key)?.as_str().ok_or("malformed_owner_fields")
}
fn ex(v: &Value, key: &str, cap: usize) -> Result<Excerpt, &'static str> {
    Ok(Excerpt::new(string(v, key)?, cap))
}
fn decode(v: &Value) -> Result<OwnerSummary, &'static str> {
    let objective = ex(v, "objective", 1024)?;
    let engineering_question = ex(v, "engineering_question", 1024)?;
    let source = field(v, "questions")?
        .as_array()
        .ok_or("malformed_owner_fields")?;
    if source.len() > 64 {
        return Err("summary_capacity");
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut questions = Vec::new();
    let mut pending_count = 0;
    let mut answered_count = 0;
    for q in source {
        let id = string(q, "id")?;
        if safe_text(id).trim().is_empty() || !ids.insert(id) {
            return Err("malformed_owner_fields");
        }
        let state = field(q, "state")?;
        let state = if state == "pending" {
            pending_count += 1;
            QuestionState::Pending
        } else if let Some(a) = state.get("answered") {
            if state.as_object().is_none_or(|o| o.len() != 1) {
                return Err("malformed_owner_fields");
            }
            answered_count += 1;
            QuestionState::Answered {
                answer: ex(a, "answer", 1024)?,
                authority_ref: ex(a, "authority_ref", 512)?,
            }
        } else if state.as_object().is_some_and(|o| o.is_empty()) {
            return Err("malformed_owner_fields");
        } else if state.is_string() || state.is_object() {
            return Err("unsupported_owner_value");
        } else {
            return Err("malformed_owner_fields");
        };
        questions.push(QuestionSummary {
            id: Excerpt::new(id, 512),
            recipient_ref: ex(q, "recipient_ref", 512)?,
            scope_ref: ex(q, "scope_ref", 512)?,
            text: ex(q, "text", 1024)?,
            state,
        });
    }
    let r = field(v, "result")?;
    let result = if r.is_null() {
        ResultSummary::NotPublished
    } else {
        ResultSummary::Published {
            result_id: ex(r, "result_id", 512)?,
            operation_id: ex(r, "operation_id", 512)?,
            body: ex(r, "body", 4096)?,
            attribution: ex(r, "attribution", 512)?,
            source_ref_count: field(r, "source_refs")?
                .as_array()
                .ok_or("malformed_owner_fields")?
                .len()
                .try_into()
                .map_err(|_| "summary_capacity")?,
        }
    };
    Ok(OwnerSummary {
        objective,
        engineering_question,
        questions,
        pending_count,
        answered_count,
        result,
    })
}
impl Summary {
    pub(super) fn project(selection: &Selection) -> Self {
        let chosen = selection.components.get("owner").and_then(|c| {
            if c.stale {
                c.last_known
                    .as_ref()
                    .and_then(|a| a.value())
                    .map(|v| ("last_known_owner", v))
            } else {
                c.latest.value().map(|v| ("latest_owner", v))
            }
        });
        let Some((basis, v)) = chosen else {
            return Self {
                state: "unavailable".into(),
                basis: "none".into(),
                reason: Some("no_owner_value".into()),
                value: None,
            };
        };
        match decode(v) {
            Ok(value) => Self {
                state: "available".into(),
                basis: basis.into(),
                reason: None,
                value: Some(value),
            },
            Err(reason) => Self {
                state: "unavailable".into(),
                basis: basis.into(),
                reason: Some(reason.into()),
                value: None,
            },
        }
    }
    /// OwnerBasisCoherence: stateless published tuple relation, not historical attestation.
    pub(super) fn validate(
        &self,
        owner: &Presentation,
        task_id: &str,
        transport: &str,
    ) -> Result<(), &'static str> {
        let sentinel = owner.scope.as_object().is_some_and(|o| o.is_empty())
            && owner.state == "unavailable" && owner.stale
            && owner.reason == "transport_unavailable" && owner.read_at_ms == 0
            && owner.revision.is_null() && owner.completeness.is_null()
            && transport == "unavailable";
        if !(owner_scope_valid(&owner.scope, task_id) || sentinel)
            || !owner.completeness.is_null()
            || (owner.state == "present" && !owner_revision_valid(&owner.revision))
            || (owner.state != "present" && !owner.revision.is_null()) {
            return Err("unsupported_owner_metadata");
        }
        let empty_anchor = owner.last_known_at_ms.is_none()
            && owner.last_known_scope.is_null()
            && owner.last_known_revision.is_null()
            && owner.last_known_completeness.is_null();
        let qualified_anchor = owner.last_known_at_ms.is_some()
            && owner_scope_valid(&owner.last_known_scope, task_id)
            && owner.last_known_scope["subject_id"].is_string()
            && owner_revision_valid(&owner.last_known_revision)
            && owner.last_known_completeness.is_null();
        let equal_tuple = owner.scope == owner.last_known_scope
            && owner.revision == owner.last_known_revision
            && owner.completeness == owner.last_known_completeness
            && owner.last_known_at_ms == Some(owner.read_at_ms);
        let no_value = || empty_anchor && owner.rows.is_empty() && self.basis == "none";
        let coherent = !(transport == "unavailable" && !owner.stale)
            && match owner.state.as_str() {
                "present" => {
                    qualified_anchor
                        && equal_tuple
                        && if owner.stale {
                            transport == "unavailable" && self.basis == "last_known_owner"
                        } else {
                            transport == "read_completed" && self.basis == "latest_owner"
                        }
                }
                "unavailable" | "refused" => {
                    owner.stale
                        && if empty_anchor {
                            no_value()
                        } else {
                            qualified_anchor
                                && self.basis == "last_known_owner"
                                && (owner.scope == owner.last_known_scope
                                    || (owner.scope["task_id"] == task_id
                                        && owner.scope.get("subject_id").is_none()))
                        }
                }
                "absent" => no_value() && owner.stale == (transport == "unavailable"),
                _ => false,
            };
        if !coherent {
            return Err("invalid_summary_basis");
        }
        match (&self.value, self.state.as_str(), self.reason.as_deref()) {
            (None, "unavailable", Some("no_owner_value")) if self.basis == "none" => Ok(()),
            (
                None,
                "unavailable",
                Some(
                    "missing_owner_fields"
                    | "malformed_owner_fields"
                    | "unsupported_owner_value"
                    | "summary_capacity",
                ),
            ) if self.basis != "none" => Ok(()),
            (Some(v), "available", None) if self.basis != "none" && v.valid() => Ok(()),
            _ => Err("invalid_summary"),
        }
    }
}
impl OwnerSummary {
    fn valid(&self) -> bool {
        self.objective.valid(1024)
            && self.engineering_question.valid(1024)
            && self.questions.len() <= 64
            && self.questions.iter().all(|q| {
                !q.id.text.trim().is_empty()
                    && q.id.valid(512)
                    && q.recipient_ref.valid(512)
                    && q.scope_ref.valid(512)
                    && q.text.valid(1024)
                    && match &q.state {
                        QuestionState::Pending => true,
                        QuestionState::Answered {
                            answer,
                            authority_ref,
                        } => answer.valid(1024) && authority_ref.valid(512),
                    }
            })
            && self.pending_count as usize
                == self
                    .questions
                    .iter()
                    .filter(|q| matches!(q.state, QuestionState::Pending))
                    .count()
            && self.answered_count as usize == self.questions.len() - self.pending_count as usize
            && match &self.result {
                ResultSummary::NotPublished => true,
                ResultSummary::Published {
                    result_id,
                    operation_id,
                    body,
                    attribution,
                    ..
                } => {
                    result_id.valid(512)
                        && operation_id.valid(512)
                        && body.valid(4096)
                        && attribution.valid(512)
                }
            }
    }
}
