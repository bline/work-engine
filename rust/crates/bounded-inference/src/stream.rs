use crate::Bounds;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedTerminal {
    Completed,
    Failed { code: Option<String> },
    Incomplete { reason: Option<String> },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenCounts {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub total: Option<u64>,
    pub cached_input: Option<u64>,
    pub reasoning_output: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageSemantics {
    Incremental,
    Cumulative,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageObservation {
    pub sequence_number: u64,
    pub semantics: UsageSemantics,
    pub counts: TokenCounts,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamObservation {
    pub response_id: Option<String>,
    pub observed_model: Option<String>,
    pub terminal: Option<ObservedTerminal>,
    /// Authoritative only after a coherent `response.completed` and clean stream end.
    pub output: Option<String>,
    /// Per event, never interpreted as context occupancy or subscription balance.
    pub usage: Vec<UsageObservation>,
    pub unique_events: usize,
    /// Local parsing, EOF or timeout failure; any such value blocks completed output.
    pub local_failure: Option<StreamError>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamError {
    #[error("local stream limit exceeded")]
    StreamTooLarge,
    #[error("local frame limit exceeded")]
    FrameTooLarge,
    #[error("local event limit exceeded")]
    TooManyEvents,
    #[error("local output limit exceeded")]
    OutputTooLarge,
    #[error("malformed SSE or JSON event")]
    MalformedEvent,
    #[error("event identity absent or contradictory")]
    ConflictingIdentity,
    #[error("response terminal absent")]
    MissingTerminal,
    #[error("contradictory terminal or event after terminal")]
    ConflictingTerminal,
    #[error("tool output unsupported by this profile")]
    UnsupportedTool,
    #[error("reasoning or refusal content is unsupported by this text profile")]
    UnsupportedContent,
    #[error("local deadline expired; remote outcome is unknown")]
    Timeout,
    #[error("local transport failed; remote outcome is unknown")]
    TransportFailure,
    #[error("HTTP response did not provide a valid stream")]
    HttpFailure,
    #[error("local cancellation observed; remote outcome is unknown")]
    Cancelled,
    #[error("bearer expired after durable intent but before network entry")]
    ExpiredBearer,
    #[error("provider stream echoed active credential; remote outcome is unknown")]
    CredentialEcho,
}

/// A bounded incremental SSE observer. It has no transport or tool execution capability.
pub struct StreamObserver {
    selected_profile: bool,
    siwc_profile: bool,
    siwc_done_items: BTreeMap<usize, Value>,
    bounds: Bounds,
    pending: Vec<u8>,
    stream_bytes: usize,
    seen: HashMap<u64, Vec<u8>>,
    response_id: Option<String>,
    observed_model: Option<String>,
    item_indices: BTreeMap<usize, String>,
    item_types: BTreeMap<usize, String>,
    reasoning: BTreeMap<(usize, usize), ContentEvidence>,
    item_added: BTreeMap<usize, Vec<String>>,
    item_done: BTreeMap<usize, Vec<String>>,
    item_done_status: BTreeMap<usize, Option<String>>,
    content: BTreeMap<(usize, usize), ContentEvidence>,
    delta_bytes: usize,
    terminal: Option<ObservedTerminal>,
    output: Option<String>,
    usage: Vec<UsageObservation>,
    done_marker: bool,
    poisoned: Option<StreamError>,
}

#[derive(Default)]
struct ContentEvidence {
    added: bool,
    delta: String,
    done: Option<String>,
    part_done: Option<String>,
}

impl StreamObserver {
    pub fn new(bounds: Bounds) -> Result<Self, StreamError> {
        bounds.validate().map_err(|_| StreamError::MalformedEvent)?;
        Ok(Self {
            selected_profile: false,
            siwc_profile: false,
            siwc_done_items: BTreeMap::new(),
            bounds,
            pending: Vec::new(),
            stream_bytes: 0,
            seen: HashMap::new(),
            response_id: None,
            observed_model: None,
            item_indices: BTreeMap::new(),
            item_types: BTreeMap::new(),
            reasoning: BTreeMap::new(),
            item_added: BTreeMap::new(),
            item_done: BTreeMap::new(),
            item_done_status: BTreeMap::new(),
            content: BTreeMap::new(),
            delta_bytes: 0,
            terminal: None,
            output: None,
            usage: Vec::new(),
            done_marker: false,
            poisoned: None,
        })
    }

    /// Explicit SIWC candidate profile; the default constructor retains B1a's
    /// text-only rejection behavior. This does not assert an observed live shape.
    pub fn selected(bounds: Bounds) -> Result<Self, StreamError> {
        let mut observer = Self::new(bounds)?;
        observer.selected_profile = true;
        Ok(observer)
    }

    /// Opt-in public SIWC stream compatibility. Metadata is uninterpreted;
    /// sparse completed output requires independently completed item/part text.
    pub const SIWC_PROFILE_REVISION: &'static str = "b2-public-siwc-stream-v2";

    pub fn siwc(bounds: Bounds) -> Result<Self, StreamError> {
        let mut observer = Self::selected(bounds)?;
        observer.siwc_profile = true;
        Ok(observer)
    }

    pub fn push(&mut self, bytes: &[u8]) -> Result<(), StreamError> {
        if let Some(error) = &self.poisoned {
            return Err(error.clone());
        }
        let result = self.push_inner(bytes);
        if let Err(error) = &result {
            self.poisoned = Some(error.clone());
        }
        result
    }

    fn push_inner(&mut self, bytes: &[u8]) -> Result<(), StreamError> {
        self.stream_bytes = self
            .stream_bytes
            .checked_add(bytes.len())
            .ok_or(StreamError::StreamTooLarge)?;
        if self.stream_bytes > self.bounds.stream_bytes {
            return Err(StreamError::StreamTooLarge);
        }
        self.pending.extend_from_slice(bytes);
        while let Some(end) = frame_end(&self.pending) {
            if end > self.bounds.frame_bytes {
                return Err(StreamError::FrameTooLarge);
            }
            let frame = self.pending.drain(..end).collect::<Vec<_>>();
            let delimiter = if self.pending.starts_with(b"\r\n\r\n") {
                4
            } else {
                2
            };
            self.pending.drain(..delimiter);
            self.frame(&frame)?;
        }
        if self
            .pending
            .len()
            .saturating_sub(delimiter_prefix_len(&self.pending))
            > self.bounds.frame_bytes
        {
            return Err(StreamError::FrameTooLarge);
        }
        Ok(())
    }

    pub fn finish(self) -> Result<StreamObservation, StreamError> {
        let evidence = self.finish_evidence();
        if let Some(error) = &evidence.local_failure {
            Err(error.clone())
        } else {
            Ok(evidence)
        }
    }

    /// Preserve known identity and usage when a local failure leaves remote outcome unknown.
    pub fn finish_evidence(self) -> StreamObservation {
        let local_failure = self.poisoned.or_else(|| {
            if !self.pending.is_empty() {
                Some(StreamError::MalformedEvent)
            } else if self.terminal.is_none() {
                Some(StreamError::MissingTerminal)
            } else {
                None
            }
        });
        StreamObservation {
            response_id: self.response_id,
            observed_model: self.observed_model,
            terminal: self.terminal,
            output: if local_failure.is_none() {
                self.output
            } else {
                None
            },
            usage: self.usage,
            unique_events: self.seen.len(),
            local_failure,
        }
    }

    pub fn finish_due_to_timeout(self) -> Result<StreamObservation, StreamError> {
        Err(StreamError::Timeout)
    }

    pub fn timeout_evidence(mut self) -> StreamObservation {
        self.poisoned = Some(StreamError::Timeout);
        self.finish_evidence()
    }

    pub fn failure_evidence(mut self, error: StreamError) -> StreamObservation {
        self.poisoned = Some(error);
        self.finish_evidence()
    }

    fn frame(&mut self, frame: &[u8]) -> Result<(), StreamError> {
        let text = std::str::from_utf8(frame).map_err(|_| StreamError::MalformedEvent)?;
        let mut event_type = None;
        let mut data = Vec::new();
        for raw in text.lines() {
            let line = raw.trim_end_matches('\r');
            if line.starts_with(':') || line.is_empty() {
                continue;
            }
            if let Some(value) = line.strip_prefix("event:") {
                if event_type.replace(value.trim()).is_some() {
                    return Err(StreamError::MalformedEvent);
                }
            } else if let Some(value) = line.strip_prefix("data:") {
                data.push(value.strip_prefix(' ').unwrap_or(value));
            } else if line.starts_with("id:") || line.starts_with("retry:") {
                // SSE metadata carries no provider identity authority.
            } else {
                return Err(StreamError::MalformedEvent);
            }
        }
        if data.is_empty() {
            return Ok(());
        }
        let data = data.join("\n");
        if data == "[DONE]" {
            if self.terminal.is_none() || self.done_marker {
                return Err(StreamError::ConflictingTerminal);
            }
            self.done_marker = true;
            return Ok(());
        }
        if self.done_marker {
            return Err(StreamError::ConflictingTerminal);
        }
        let value: Value = serde_json::from_str(&data).map_err(|_| StreamError::MalformedEvent)?;
        let kind = value
            .get("type")
            .and_then(Value::as_str)
            .ok_or(StreamError::MalformedEvent)?;
        if event_type.is_some_and(|e| e != kind) {
            return Err(StreamError::MalformedEvent);
        }
        let seq = value
            .get("sequence_number")
            .and_then(Value::as_u64)
            .ok_or(StreamError::ConflictingIdentity)?;
        let canonical = serde_json::to_vec(&value).map_err(|_| StreamError::MalformedEvent)?;
        if let Some(prior) = self.seen.get(&seq) {
            return if prior == &canonical {
                Ok(())
            } else {
                Err(StreamError::ConflictingIdentity)
            };
        }
        if self.seen.len() >= self.bounds.events {
            return Err(StreamError::TooManyEvents);
        }
        if self.terminal.is_some() {
            return Err(StreamError::ConflictingTerminal);
        }
        self.handle(kind, seq, &value)?;
        self.seen.insert(seq, canonical);
        Ok(())
    }

    fn handle(&mut self, kind: &str, seq: u64, value: &Value) -> Result<(), StreamError> {
        match kind {
            "response.created" => {
                if self.response_id.is_some() {
                    return Err(StreamError::ConflictingIdentity);
                }
                let response = value.get("response").ok_or(StreamError::MalformedEvent)?;
                self.response_id = Some(required_str(response, "id")?.to_owned());
                self.observed_model = optional_str(response, "model")?.map(str::to_owned);
                validate_response_envelope(response, EnvelopePhase::Created)?;
            }
            "response.in_progress" => {
                self.ensure_created()?;
                let response = value.get("response").ok_or(StreamError::MalformedEvent)?;
                if required_str(response, "id")? != self.response_id.as_deref().unwrap() {
                    return Err(StreamError::ConflictingIdentity);
                }
                validate_response_envelope(response, EnvelopePhase::InProgress)?;
                if let Some(model) = optional_str(response, "model")? {
                    if self.observed_model.as_deref().is_some_and(|m| m != model) {
                        return Err(StreamError::ConflictingIdentity);
                    }
                    self.observed_model = Some(model.to_owned());
                }
            }
            "response.metadata" if self.siwc_profile => {
                self.ensure_created()?;
                only_keys(
                    value,
                    &["type", "sequence_number", "response_id", "metadata"],
                )?;
                if required_str(value, "response_id")? != self.response_id.as_deref().unwrap() {
                    return Err(StreamError::ConflictingIdentity);
                }
                if !value.get("metadata").is_some_and(Value::is_object) {
                    return Err(StreamError::MalformedEvent);
                }
                // Opaque metadata has no output, usage, model or terminal authority.
            }
            "response.output_item.added" | "response.output_item.done" => {
                self.ensure_created()?;
                let item = value.get("item").ok_or(StreamError::MalformedEvent)?;
                self.check_item_type(item)?;
                let id = required_str(item, "id")?;
                let index = required_index(value)?;
                let is_done = kind == "response.output_item.done";
                validate_item_status(
                    item,
                    if is_done {
                        ItemPhase::Done
                    } else {
                        ItemPhase::Added
                    },
                )?;
                let item_type = required_str(item, "type")?;
                if self
                    .item_types
                    .get(&index)
                    .is_some_and(|kind| kind != item_type)
                {
                    return Err(StreamError::ConflictingIdentity);
                }
                self.item_types.insert(index, item_type.to_owned());
                if self
                    .item_indices
                    .iter()
                    .any(|(other_index, other_id)| *other_index != index && other_id == id)
                {
                    return Err(StreamError::ConflictingIdentity);
                }
                if let Some(existing) = self.item_indices.insert(index, id.to_owned()) {
                    if existing != id {
                        return Err(StreamError::ConflictingIdentity);
                    }
                }
                if item_type == "reasoning" {
                    only_keys(value, &["type", "sequence_number", "output_index", "item"])?;
                    self.reasoning_item(index, item, is_done)?;
                    if self.siwc_profile && is_done {
                        self.siwc_done_items.insert(index, item.clone());
                    }
                    return Ok(());
                }
                let content = snapshot_texts(item)?;
                if content
                    .iter()
                    .try_fold(0usize, |sum, text| sum.checked_add(text.len()))
                    .is_none_or(|bytes| bytes > self.bounds.output_bytes)
                {
                    return Err(StreamError::OutputTooLarge);
                }
                if is_done {
                    if self.item_done.insert(index, content.clone()).is_some() {
                        return Err(StreamError::ConflictingIdentity);
                    }
                    self.item_done_status
                        .insert(index, optional_str(item, "status")?.map(str::to_owned));
                    self.reconcile_snapshot(index, &content, true)?;
                    if self.siwc_profile {
                        self.siwc_done_items.insert(index, item.clone());
                    }
                } else {
                    if self.item_added.insert(index, content.clone()).is_some() {
                        return Err(StreamError::ConflictingIdentity);
                    }
                    self.reconcile_snapshot(index, &content, false)?;
                }
            }
            "response.content_part.added" | "response.content_part.done" => {
                self.ensure_created()?;
                let index = required_index(value)?;
                let content_index = required_content_index(value)?;
                self.check_item_identity(index, required_str(value, "item_id")?)?;
                if self.item_types.get(&index).map(String::as_str) != Some("message") {
                    return Err(StreamError::ConflictingIdentity);
                }
                let part = value.get("part").ok_or(StreamError::MalformedEvent)?;
                check_part_type(part)?;
                let text = required_text(part, "text")?;
                if kind.ends_with(".added") && !text.is_empty() {
                    return Err(StreamError::UnsupportedContent);
                }
                if text.len() > self.bounds.output_bytes {
                    return Err(StreamError::OutputTooLarge);
                }
                let evidence = self.content.entry((index, content_index)).or_default();
                if kind.ends_with(".added") {
                    if evidence.added {
                        return Err(StreamError::ConflictingIdentity);
                    }
                    evidence.added = true;
                } else {
                    if evidence.part_done.is_some()
                        || (!evidence.delta.is_empty() && evidence.delta != text)
                        || evidence.done.as_deref().is_some_and(|done| done != text)
                    {
                        return Err(StreamError::ConflictingIdentity);
                    }
                    evidence.part_done = Some(text.to_owned());
                }
            }
            "response.output_text.delta" | "response.output_text.done" => {
                self.ensure_created()?;
                let index = required_index(value)?;
                let content_index = required_content_index(value)?;
                let item_id = required_str(value, "item_id")?;
                self.check_item_identity(index, item_id)?;
                if self.item_types.get(&index).map(String::as_str) != Some("message") {
                    return Err(StreamError::ConflictingIdentity);
                }
                let text = if kind.ends_with(".delta") {
                    required_str(value, "delta")?
                } else {
                    required_str(value, "text")?
                };
                let evidence = self.content.entry((index, content_index)).or_default();
                if kind.ends_with(".delta") {
                    if evidence.done.is_some() || evidence.part_done.is_some() {
                        return Err(StreamError::ConflictingIdentity);
                    }
                    self.delta_bytes = self
                        .delta_bytes
                        .checked_add(text.len())
                        .ok_or(StreamError::OutputTooLarge)?;
                    if self.delta_bytes > self.bounds.output_bytes {
                        return Err(StreamError::OutputTooLarge);
                    }
                    evidence.delta.push_str(text);
                } else if text.len() > self.bounds.output_bytes {
                    return Err(StreamError::OutputTooLarge);
                } else {
                    if evidence.done.is_some()
                        || (!evidence.delta.is_empty() && evidence.delta != text)
                        || evidence
                            .part_done
                            .as_deref()
                            .is_some_and(|done| done != text)
                    {
                        return Err(StreamError::ConflictingIdentity);
                    }
                    evidence.done = Some(text.to_owned());
                }
            }
            "response.usage.delta" => {
                self.ensure_created()?;
                let counts = parse_usage(value.get("usage").ok_or(StreamError::MalformedEvent)?)?;
                self.usage.push(UsageObservation {
                    sequence_number: seq,
                    semantics: UsageSemantics::Incremental,
                    counts,
                });
            }
            "response.reasoning_summary_text.delta" | "response.reasoning_summary_text.done" => {
                if !self.selected_profile {
                    return Err(StreamError::UnsupportedContent);
                }
                if kind.ends_with(".delta") {
                    only_keys(
                        value,
                        &[
                            "type",
                            "sequence_number",
                            "output_index",
                            "item_id",
                            "summary_index",
                            "delta",
                        ],
                    )?;
                } else {
                    only_keys(
                        value,
                        &[
                            "type",
                            "sequence_number",
                            "output_index",
                            "item_id",
                            "summary_index",
                            "text",
                        ],
                    )?;
                }
                self.ensure_created()?;
                let index = required_index(value)?;
                let summary_index = value
                    .get("summary_index")
                    .and_then(Value::as_u64)
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or(StreamError::MalformedEvent)?;
                if summary_index >= self.bounds.events {
                    return Err(StreamError::TooManyEvents);
                }
                self.check_item_identity(index, required_str(value, "item_id")?)?;
                if self.item_types.get(&index).map(String::as_str) != Some("reasoning") {
                    return Err(StreamError::ConflictingIdentity);
                }
                let text = if kind.ends_with(".delta") {
                    required_str(value, "delta")?
                } else {
                    required_text(value, "text")?
                };
                // Summary is observed only for identity checks. It cannot become report text.
                if text.len() > self.bounds.frame_bytes {
                    return Err(StreamError::FrameTooLarge);
                }
                let evidence = self.reasoning.entry((index, summary_index)).or_default();
                if kind.ends_with(".delta") {
                    if evidence.done.is_some() {
                        return Err(StreamError::ConflictingIdentity);
                    }
                    evidence.delta.push_str(text);
                    if evidence.delta.len() > self.bounds.stream_bytes {
                        return Err(StreamError::StreamTooLarge);
                    }
                } else {
                    if evidence.done.is_some()
                        || (!evidence.delta.is_empty() && evidence.delta != text)
                    {
                        return Err(StreamError::ConflictingIdentity);
                    }
                    evidence.done = Some(text.to_owned());
                }
            }
            "response.completed" | "response.failed" | "response.incomplete" => {
                self.ensure_created()?;
                let response = value.get("response").ok_or(StreamError::MalformedEvent)?;
                if required_str(response, "id")? != self.response_id.as_deref().unwrap() {
                    return Err(StreamError::ConflictingIdentity);
                }
                if let Some(model) = optional_str(response, "model")? {
                    if self.observed_model.as_deref().is_some_and(|m| m != model) {
                        return Err(StreamError::ConflictingIdentity);
                    }
                    self.observed_model = Some(model.to_owned());
                }
                let status = required_str(response, "status")?;
                let expected = kind.strip_prefix("response.").unwrap();
                if status != expected {
                    return Err(StreamError::ConflictingTerminal);
                }
                if let Some(usage) = response.get("usage").filter(|v| !v.is_null()) {
                    self.usage.push(UsageObservation {
                        sequence_number: seq,
                        semantics: UsageSemantics::Cumulative,
                        counts: parse_usage(usage)?,
                    });
                }
                // The terminal kind is observed even if later payload validation fails.
                // `push` then poisons the stream and `finish_evidence` withholds output.
                if kind == "response.failed" {
                    self.terminal = Some(ObservedTerminal::Failed { code: None });
                } else if kind == "response.incomplete" {
                    self.terminal = Some(ObservedTerminal::Incomplete { reason: None });
                }
                let phase = match kind {
                    "response.completed" => EnvelopePhase::Completed,
                    "response.failed" => EnvelopePhase::Failed,
                    _ => EnvelopePhase::Incomplete,
                };
                validate_response_envelope(response, phase)?;
                self.terminal = Some(match kind {
                    "response.completed" => {
                        self.output = Some(self.terminal_output(response, true)?);
                        ObservedTerminal::Completed
                    }
                    "response.failed" => {
                        if response.get("output").is_some_and(|v| !v.is_null()) {
                            self.terminal_output(response, false)?;
                        }
                        ObservedTerminal::Failed {
                            code: nested_optional_str(response, "error", "code")?,
                        }
                    }
                    _ => {
                        if response.get("output").is_some_and(|v| !v.is_null()) {
                            self.terminal_output(response, false)?;
                        }
                        ObservedTerminal::Incomplete {
                            reason: nested_optional_str(response, "incomplete_details", "reason")?,
                        }
                    }
                });
            }
            "error" => {
                self.ensure_created()?;
                self.terminal = Some(ObservedTerminal::Failed {
                    code: optional_str(value, "code")?.map(str::to_owned),
                });
            }
            kind if kind.starts_with("response.refusal.")
                || kind.starts_with("response.reasoning_") =>
            {
                return Err(StreamError::UnsupportedContent);
            }
            kind if kind.starts_with("response.function_call_arguments.") => {
                return Err(StreamError::UnsupportedTool);
            }
            _ => return Err(StreamError::MalformedEvent),
        }
        Ok(())
    }

    fn ensure_created(&self) -> Result<(), StreamError> {
        if self.response_id.is_none() {
            Err(StreamError::ConflictingIdentity)
        } else {
            Ok(())
        }
    }

    fn check_item_type(&self, item: &Value) -> Result<(), StreamError> {
        match required_str(item, "type")? {
            "message" => Ok(()),
            "reasoning" if self.selected_profile => Ok(()),
            "reasoning" => Err(StreamError::UnsupportedContent),
            _ => Err(StreamError::UnsupportedTool),
        }
    }

    fn reasoning_item(
        &mut self,
        index: usize,
        item: &Value,
        done: bool,
    ) -> Result<(), StreamError> {
        only_keys(item, &["id", "type", "status", "summary"])?;
        if item.get("content").is_some_and(|v| !v.is_null()) {
            return Err(StreamError::UnsupportedContent);
        }
        let summaries = match item.get("summary") {
            None | Some(Value::Null) => &[][..],
            Some(Value::Array(v)) => v.as_slice(),
            _ => return Err(StreamError::MalformedEvent),
        };
        if summaries.len() > self.bounds.events {
            return Err(StreamError::TooManyEvents);
        }
        let mut texts = Vec::with_capacity(summaries.len());
        for (summary_index, part) in summaries.iter().enumerate() {
            only_keys(part, &["type", "text"])?;
            if required_str(part, "type")? != "summary_text" {
                return Err(StreamError::UnsupportedContent);
            }
            let text = required_text(part, "text")?;
            texts.push(text.to_owned());
            if text.len() > self.bounds.stream_bytes {
                return Err(StreamError::StreamTooLarge);
            }
            if let Some(evidence) = self.reasoning.get(&(index, summary_index)) {
                if (done && !evidence.delta.is_empty() && evidence.delta != text)
                    || evidence.done.as_deref().is_some_and(|seen| seen != text)
                {
                    return Err(StreamError::ConflictingIdentity);
                }
            }
        }
        if done {
            if self.item_added.get(&index).is_some_and(|added| {
                added.len() > texts.len()
                    || added.iter().zip(&texts).any(|(a, b)| !b.starts_with(a))
            }) {
                return Err(StreamError::ConflictingIdentity);
            }
            if self
                .reasoning
                .keys()
                .any(|(i, j)| *i == index && *j >= summaries.len())
            {
                return Err(StreamError::ConflictingIdentity);
            }
            if self.item_done.insert(index, texts).is_some() {
                return Err(StreamError::ConflictingIdentity);
            }
            self.item_done_status
                .insert(index, optional_str(item, "status")?.map(str::to_owned));
        } else if self.item_added.insert(index, texts).is_some() {
            return Err(StreamError::ConflictingIdentity);
        }
        Ok(())
    }

    fn check_item_identity(&self, index: usize, id: &str) -> Result<(), StreamError> {
        match self.item_indices.get(&index) {
            Some(existing) if existing == id => Ok(()),
            _ => Err(StreamError::ConflictingIdentity),
        }
    }

    fn reconcile_snapshot(
        &self,
        index: usize,
        parts: &[String],
        final_snapshot: bool,
    ) -> Result<(), StreamError> {
        for (content_index, text) in parts.iter().enumerate() {
            if let Some(evidence) = self.content.get(&(index, content_index)) {
                if final_snapshot {
                    if (!evidence.delta.is_empty() && evidence.delta != *text)
                        || evidence.done.as_deref().is_some_and(|done| done != text)
                        || evidence
                            .part_done
                            .as_deref()
                            .is_some_and(|done| done != text)
                    {
                        return Err(StreamError::ConflictingIdentity);
                    }
                } else if !evidence.delta.starts_with(text) && !text.starts_with(&evidence.delta) {
                    return Err(StreamError::ConflictingIdentity);
                }
            }
        }
        Ok(())
    }

    fn siwc_completed_output(&self, response: &Value) -> Result<String, StreamError> {
        let mut items = Vec::with_capacity(self.item_indices.len());
        let mut has_message = false;
        for index in 0..self.item_indices.len() {
            if !self.item_indices.contains_key(&index) || !self.item_added.contains_key(&index) {
                return Err(StreamError::ConflictingIdentity);
            }
            let item = self
                .siwc_done_items
                .get(&index)
                .ok_or(StreamError::ConflictingIdentity)?;
            if optional_str(item, "status")? != Some("completed") {
                return Err(StreamError::ConflictingTerminal);
            }
            if required_str(item, "type")? == "message" {
                has_message = true;
                let content = item
                    .get("content")
                    .and_then(Value::as_array)
                    .filter(|v| !v.is_empty())
                    .ok_or(StreamError::MalformedEvent)?;
                for (part_index, part) in content.iter().enumerate() {
                    let text = required_text(part, "text")?;
                    let evidence = self
                        .content
                        .get(&(index, part_index))
                        .ok_or(StreamError::ConflictingIdentity)?;
                    if !evidence.added
                        || evidence.done.as_deref() != Some(text)
                        || evidence.part_done.as_deref() != Some(text)
                    {
                        return Err(StreamError::ConflictingIdentity);
                    }
                }
            }
            items.push(item.clone());
        }
        if !has_message {
            return Err(StreamError::ConflictingIdentity);
        }
        // Reuse all terminal checks (role/type, identities, status, text, bounds,
        // and complete coverage). This internal view never replaces wire evidence.
        let mut view = response.clone();
        view["output"] = Value::Array(items);
        self.terminal_output(&view, true)
    }

    fn terminal_output(&self, response: &Value, completed: bool) -> Result<String, StreamError> {
        let items = response
            .get("output")
            .and_then(Value::as_array)
            .ok_or(StreamError::MalformedEvent)?;
        if self.siwc_profile && completed && items.is_empty() && !self.item_indices.is_empty() {
            return self.siwc_completed_output(response);
        }
        let mut result = String::new();
        let mut item_ids = std::collections::HashSet::new();
        for (index, item) in items.iter().enumerate() {
            self.check_item_type(item)?;
            validate_item_status(
                item,
                if completed {
                    ItemPhase::TerminalCompleted
                } else {
                    ItemPhase::TerminalOther
                },
            )?;
            if self
                .item_done_status
                .get(&index)
                .and_then(|s| s.as_deref())
                .is_some_and(|status| {
                    (completed && status != "completed")
                        || optional_str(item, "status")
                            .ok()
                            .flatten()
                            .is_some_and(|terminal_status| terminal_status != status)
                })
            {
                return Err(StreamError::ConflictingTerminal);
            }
            let id = required_str(item, "id")?;
            if !item_ids.insert(id) {
                return Err(StreamError::ConflictingIdentity);
            }
            if let Some(previous) = self.item_indices.get(&index) {
                if previous != id {
                    return Err(StreamError::ConflictingIdentity);
                }
            }
            let kind = required_str(item, "type")?;
            if self
                .item_types
                .get(&index)
                .is_some_and(|prior| prior != kind)
            {
                return Err(StreamError::ConflictingIdentity);
            }
            if kind == "reasoning" {
                only_keys(item, &["id", "type", "status", "summary"])?;
                if !self.item_indices.contains_key(&index) {
                    return Err(StreamError::ConflictingIdentity);
                }
                let summaries = match item.get("summary") {
                    None | Some(Value::Null) => &[][..],
                    Some(Value::Array(v)) => v.as_slice(),
                    _ => return Err(StreamError::MalformedEvent),
                };
                if let Some(done) = self.item_done.get(&index) {
                    if done.len() != summaries.len() {
                        return Err(StreamError::ConflictingIdentity);
                    }
                }
                if let Some(added) = self.item_added.get(&index) {
                    if added.len() > summaries.len() {
                        return Err(StreamError::ConflictingIdentity);
                    }
                }
                for (summary_index, part) in summaries.iter().enumerate() {
                    only_keys(part, &["type", "text"])?;
                    if required_str(part, "type")? != "summary_text" {
                        return Err(StreamError::UnsupportedContent);
                    }
                    let text = required_text(part, "text")?;
                    if self
                        .item_done
                        .get(&index)
                        .and_then(|v| v.get(summary_index))
                        .is_some_and(|seen| seen != text)
                        || self
                            .item_added
                            .get(&index)
                            .and_then(|v| v.get(summary_index))
                            .is_some_and(|seen| !text.starts_with(seen))
                    {
                        return Err(StreamError::ConflictingIdentity);
                    }
                    if let Some(evidence) = self.reasoning.get(&(index, summary_index)) {
                        if (!evidence.delta.is_empty() && evidence.delta != text)
                            || evidence.done.as_deref().is_some_and(|seen| seen != text)
                        {
                            return Err(StreamError::ConflictingIdentity);
                        }
                    }
                }
                if self
                    .reasoning
                    .keys()
                    .any(|(i, j)| *i == index && *j >= summaries.len())
                {
                    return Err(StreamError::ConflictingIdentity);
                }
                continue;
            }
            if optional_str(item, "role")? != Some("assistant") {
                return Err(StreamError::MalformedEvent);
            }
            let content = item
                .get("content")
                .and_then(Value::as_array)
                .ok_or(StreamError::MalformedEvent)?;
            if let Some(done) = self.item_done.get(&index) {
                if done.len() != content.len() {
                    return Err(StreamError::ConflictingIdentity);
                }
            }
            if let Some(added) = self.item_added.get(&index) {
                if added.len() > content.len() {
                    return Err(StreamError::ConflictingIdentity);
                }
            }
            for (content_index, part) in content.iter().enumerate() {
                check_part_type(part)?;
                let text = required_text(part, "text")?;
                if self
                    .item_done
                    .get(&index)
                    .and_then(|parts| parts.get(content_index))
                    .is_some_and(|done| done != text)
                    || self
                        .item_added
                        .get(&index)
                        .and_then(|parts| parts.get(content_index))
                        .is_some_and(|start| !text.starts_with(start))
                {
                    return Err(StreamError::ConflictingIdentity);
                }
                if let Some(evidence) = self.content.get(&(index, content_index)) {
                    if (!evidence.delta.is_empty() && evidence.delta != text)
                        || evidence.done.as_deref().is_some_and(|done| done != text)
                        || evidence
                            .part_done
                            .as_deref()
                            .is_some_and(|done| done != text)
                    {
                        return Err(StreamError::ConflictingIdentity);
                    }
                }
                if result
                    .len()
                    .checked_add(text.len())
                    .ok_or(StreamError::OutputTooLarge)?
                    > self.bounds.output_bytes
                {
                    return Err(StreamError::OutputTooLarge);
                }
                result.push_str(text);
            }
        }
        if completed {
            for index in self.item_indices.keys() {
                if *index >= items.len() {
                    return Err(StreamError::ConflictingIdentity);
                }
            }
            for (index, content_index) in self.content.keys() {
                let item = items.get(*index).ok_or(StreamError::ConflictingIdentity)?;
                let content = item
                    .get("content")
                    .and_then(Value::as_array)
                    .ok_or(StreamError::MalformedEvent)?;
                if *content_index >= content.len() {
                    return Err(StreamError::ConflictingIdentity);
                }
            }
        }
        Ok(result)
    }
}

fn frame_end(bytes: &[u8]) -> Option<usize> {
    let lf = bytes.windows(2).position(|w| w == b"\n\n");
    let crlf = bytes.windows(4).position(|w| w == b"\r\n\r\n");
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    }
}

fn delimiter_prefix_len(bytes: &[u8]) -> usize {
    [b"\r\n\r".as_slice(), b"\r\n", b"\r", b"\n"]
        .into_iter()
        .find(|prefix| bytes.ends_with(prefix))
        .map_or(0, |prefix| prefix.len())
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, StreamError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or(StreamError::MalformedEvent)
}

fn only_keys(value: &Value, allowed: &[&str]) -> Result<(), StreamError> {
    let object = value.as_object().ok_or(StreamError::MalformedEvent)?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        Err(StreamError::UnsupportedContent)
    } else {
        Ok(())
    }
}

fn optional_str<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>, StreamError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        _ => Err(StreamError::MalformedEvent),
    }
}

fn nested_optional_str(
    value: &Value,
    parent: &str,
    key: &str,
) -> Result<Option<String>, StreamError> {
    match value.get(parent) {
        None | Some(Value::Null) => Ok(None),
        Some(v) if v.is_object() => Ok(optional_str(v, key)?.map(str::to_owned)),
        Some(_) => Err(StreamError::MalformedEvent),
    }
}

#[derive(Clone, Copy)]
enum EnvelopePhase {
    Created,
    InProgress,
    Completed,
    Failed,
    Incomplete,
}

fn validate_response_envelope(response: &Value, phase: EnvelopePhase) -> Result<(), StreamError> {
    if !response.is_object() {
        return Err(StreamError::MalformedEvent);
    }
    let status = optional_str(response, "status")?;
    let status_valid = match phase {
        EnvelopePhase::Created => status.is_none_or(|s| matches!(s, "queued" | "in_progress")),
        EnvelopePhase::InProgress => status == Some("in_progress"),
        EnvelopePhase::Completed => status == Some("completed"),
        EnvelopePhase::Failed => status == Some("failed"),
        EnvelopePhase::Incomplete => status == Some("incomplete"),
    };
    if !status_valid {
        return Err(StreamError::ConflictingTerminal);
    }
    let error = response.get("error").filter(|v| !v.is_null());
    let incomplete = response.get("incomplete_details").filter(|v| !v.is_null());
    match phase {
        EnvelopePhase::Created | EnvelopePhase::InProgress | EnvelopePhase::Completed => {
            if error.is_some() || incomplete.is_some() {
                return Err(StreamError::ConflictingTerminal);
            }
        }
        EnvelopePhase::Failed => {
            if incomplete.is_some() {
                return Err(StreamError::ConflictingTerminal);
            }
            nested_optional_str(response, "error", "code")?;
        }
        EnvelopePhase::Incomplete => {
            if error.is_some() {
                return Err(StreamError::ConflictingTerminal);
            }
            nested_optional_str(response, "incomplete_details", "reason")?;
        }
    }
    if matches!(phase, EnvelopePhase::Created | EnvelopePhase::InProgress) {
        if response.get("usage").is_some_and(|v| !v.is_null()) {
            return Err(StreamError::UnsupportedContent);
        }
        match response.get("output") {
            None | Some(Value::Null) => {}
            Some(Value::Array(items)) if items.is_empty() => {}
            Some(Value::Array(_)) => return Err(StreamError::UnsupportedContent),
            Some(_) => return Err(StreamError::MalformedEvent),
        }
    }
    Ok(())
}

fn required_index(value: &Value) -> Result<usize, StreamError> {
    value
        .get("output_index")
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(StreamError::MalformedEvent)
}

fn required_content_index(value: &Value) -> Result<usize, StreamError> {
    value
        .get("content_index")
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(StreamError::MalformedEvent)
}

fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str, StreamError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or(StreamError::MalformedEvent)
}

enum ItemPhase {
    Added,
    Done,
    TerminalCompleted,
    TerminalOther,
}

fn validate_item_status(item: &Value, phase: ItemPhase) -> Result<(), StreamError> {
    if let Some(status) = optional_str(item, "status")? {
        let valid = match phase {
            ItemPhase::Added => status == "in_progress",
            ItemPhase::Done => matches!(status, "completed" | "incomplete"),
            ItemPhase::TerminalOther => {
                matches!(status, "in_progress" | "completed" | "incomplete")
            }
            ItemPhase::TerminalCompleted => status == "completed",
        };
        if !valid {
            return Err(StreamError::ConflictingTerminal);
        }
    }
    Ok(())
}

fn snapshot_texts(item: &Value) -> Result<Vec<String>, StreamError> {
    if optional_str(item, "role")?.is_some_and(|role| role != "assistant") {
        return Err(StreamError::MalformedEvent);
    }
    let content = match item.get("content") {
        None => return Ok(Vec::new()),
        Some(v) => v.as_array().ok_or(StreamError::MalformedEvent)?,
    };
    let mut texts = Vec::with_capacity(content.len());
    for part in content {
        check_part_type(part)?;
        texts.push(required_text(part, "text")?.to_owned());
    }
    Ok(texts)
}

fn check_part_type(part: &Value) -> Result<(), StreamError> {
    match required_str(part, "type")? {
        "output_text" => Ok(()),
        "refusal" | "reasoning_text" => Err(StreamError::UnsupportedContent),
        _ => Err(StreamError::UnsupportedTool),
    }
}

fn parse_usage(value: &Value) -> Result<TokenCounts, StreamError> {
    if !value.is_object() {
        return Err(StreamError::MalformedEvent);
    }
    let field = |name: &str| -> Result<Option<u64>, StreamError> {
        match value.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => v.as_u64().map(Some).ok_or(StreamError::MalformedEvent),
        }
    };
    let nested = |parent: &str, name: &str| -> Result<Option<u64>, StreamError> {
        match value.get(parent) {
            None | Some(Value::Null) => Ok(None),
            Some(v) if !v.is_object() => Err(StreamError::MalformedEvent),
            Some(v) => match v.get(name) {
                None | Some(Value::Null) => Ok(None),
                Some(n) => n.as_u64().map(Some).ok_or(StreamError::MalformedEvent),
            },
        }
    };
    Ok(TokenCounts {
        input: field("input_tokens")?,
        output: field("output_tokens")?,
        total: field("total_tokens")?,
        cached_input: nested("input_tokens_details", "cached_tokens")?,
        reasoning_output: nested("output_tokens_details", "reasoning_tokens")?,
    })
}
