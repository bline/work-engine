//! One admitted, read-only historical investigation in the terminal client.
use crossterm::{
    cursor::Show,
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use historical_evidence_client::{InspectAttempt, Inspected, inspect_best};
use historical_evidence_wire::{
    ContractRepairRead, Observation, ObservationV2, ReviewEpisode, ReviewRevision, SourceRead,
    graph, graph_v2,
};
use ratatui::{
    Frame, Terminal,
    backend::{CrosstermBackend, TestBackend},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Candidate,
    Replacement,
    Original,
    Findings,
    Provider,
    Scheduler,
    Graph,
    Gaps,
}
impl Stage {
    fn next(self) -> Self {
        match self {
            Self::Candidate => Self::Replacement,
            Self::Replacement => Self::Original,
            Self::Original => Self::Findings,
            Self::Provider => Self::Scheduler,
            Self::Scheduler => Self::Replacement,
            Self::Graph => Self::Graph,
            Self::Findings | Self::Gaps => Self::Gaps,
        }
    }
    fn previous(self) -> Self {
        match self {
            Self::Candidate | Self::Replacement => Self::Candidate,
            Self::Original => Self::Replacement,
            Self::Findings => Self::Original,
            Self::Provider => Self::Candidate,
            Self::Scheduler => Self::Provider,
            Self::Graph => Self::Candidate,
            Self::Gaps => Self::Findings,
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Candidate => "Candidate and gate",
            Self::Replacement => "Review lineage",
            Self::Original => "Original reported revision",
            Self::Findings => "Finding dispositions",
            Self::Provider => "Provider attempts",
            Self::Scheduler => "Scheduled retry",
            Self::Graph => "Owner evidence graph",
            Self::Gaps => "Evidence gaps",
        }
    }
}

struct GraphView {
    base: ObservationV2,
    nodes: Vec<graph::Node>,
    edges: Vec<graph::Edge>,
    gaps: Vec<String>,
    repair_source: Option<SourceRead>,
    repair_read: Option<ContractRepairRead>,
    interpretation_support: Option<graph_v2::InterpretationSupport>,
}
impl GraphView {
    fn old(observation: graph::Observation) -> Self {
        Self {
            base: observation.base,
            nodes: observation.nodes,
            edges: observation.edges,
            gaps: observation.gaps,
            repair_source: None,
            repair_read: None,
            interpretation_support: None,
        }
    }
    fn repaired(observation: graph_v2::Observation) -> Option<Self> {
        let edges = observation.expanded_edges().ok()?;
        let interpretation_support = Some(
            observation
                .edges
                .iter()
                .find(|edge| edge.kind == "clarifies_reported_continuation_for")?
                .support
                .clone()?,
        );
        Some(Self {
            base: observation.base,
            nodes: observation.nodes,
            edges,
            gaps: observation.gaps,
            repair_source: Some(observation.repair_source),
            repair_read: Some(observation.repair),
            interpretation_support,
        })
    }
    fn source(&self, index: usize) -> Option<&SourceRead> {
        self.base.source_reads.get(index).or_else(|| {
            if index == self.base.source_reads.len() {
                self.repair_source.as_ref()
            } else {
                None
            }
        })
    }
}

struct State {
    observation: Observation,
    extended: Option<ObservationV2>,
    graph: Option<GraphView>,
    graph_index: usize,
    graph_edge: usize,
    graph_back: Vec<usize>,
    selected_attempt: usize,
    selected_provider: usize,
    stage: Stage,
    detail_scroll: u16,
    notice: String,
}
impl State {
    fn new(observation: Observation) -> Self {
        Self {
            observation,
            extended: None,
            graph: None,
            graph_index: 0,
            graph_edge: 0,
            graph_back: Vec::new(),
            selected_attempt: 5,
            selected_provider: 5,
            stage: Stage::Candidate,
            detail_scroll: 0,
            notice: "ADMITTED LOCAL READ · source-limited producer capture · no actions".into(),
        }
    }
    fn from_v2(extended: ObservationV2) -> Self {
        let mut state = Self::new(extended.base.clone());
        state.extended = Some(extended);
        state.notice =
            "ADMITTED LOCAL V2 READ · captured provider and scheduler sources · no actions".into();
        state
    }
    fn from_graph(graph: graph::Observation) -> Self {
        let mut state = Self::from_v2(graph.base.clone());
        state.graph = Some(GraphView::old(graph));
        state.notice = "ADMITTED LOCAL GRAPH READ · exact owner edges · no actions".into();
        state
    }
    fn from_graph_v2(graph: graph_v2::Observation) -> Option<Self> {
        let mut state = Self::from_v2(graph.base.clone());
        state.graph = Some(GraphView::repaired(graph)?);
        state.notice =
            "ADMITTED LOCAL GRAPH V2 READ · source-backed contract revision · no actions".into();
        Some(state)
    }
    fn open_graph(&mut self) {
        if let Some(graph) = &self.graph {
            self.graph_index = if self.stage == Stage::Provider {
                self.extended
                    .as_ref()
                    .and_then(|v2| v2.provider.attempts.get(self.selected_provider))
                    .and_then(|attempt| {
                        graph.nodes.iter().position(|n| {
                            n.kind == "provider_result"
                                && n.local_id == attempt.result_uuid
                                && n.source_revision == attempt.source_line_sha256
                        })
                    })
            } else {
                let candidate = &self.observation.candidates[self.selected_attempt - 1];
                graph.nodes.iter().position(|n| {
                    n.kind == "checkpoint"
                        && n.source_ref == candidate.checkpoint_ref
                        && n.source_revision == candidate.commit_oid
                })
            }
            .unwrap_or(0);
            self.graph_edge = 0;
            self.graph_back.clear();
            self.stage = Stage::Graph;
            self.detail_scroll = 0;
        }
    }
    fn graph_edges(&self) -> Vec<usize> {
        let Some(graph) = &self.graph else {
            return Vec::new();
        };
        let Some(node) = graph.nodes.get(self.graph_index) else {
            return Vec::new();
        };
        graph
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, edge)| (edge.from == node.id || edge.to == node.id).then_some(i))
            .collect()
    }
    fn refresh_with(&mut self, next: Inspected) {
        match next {
            Inspected::GraphV2(next) => {
                let Some(next) = GraphView::repaired(*next) else {
                    self.notice = "LAST OBSERVED · invalid graph v2 view; prior frame stale".into();
                    return;
                };
                self.refresh_graph(next);
            }
            Inspected::Graph(next) => {
                self.refresh_graph(GraphView::old(*next));
            }
            Inspected::V2(next) if self.extended.is_some() => {
                self.graph = None;
                if self.stage == Stage::Graph {
                    self.stage = Stage::Candidate;
                }
                if next.source_identity_sha256
                    != self.extended.as_ref().unwrap().source_identity_sha256
                {
                    self.stage = Stage::Candidate;
                    self.selected_attempt = 5;
                    self.selected_provider = 5;
                    self.detail_scroll = 0;
                    self.notice = "Source identity changed; inspection anchor cleared".into();
                } else {
                    self.notice = "Owner sources refreshed under the same v2 grant".into();
                }
                self.observation = next.base.clone();
                self.extended = Some(*next);
            }
            Inspected::V1(next) if self.extended.is_none() => {
                self.graph = None;
                if self.stage == Stage::Graph {
                    self.stage = Stage::Candidate;
                }
                if next.source_identity_sha256 != self.observation.source_identity_sha256 {
                    self.stage = Stage::Candidate;
                    self.selected_attempt = 5;
                    self.detail_scroll = 0;
                    self.notice = "Source identity changed; inspection anchor cleared".into();
                } else {
                    self.notice = "Owner sources refreshed under the same read grant".into();
                }
                self.observation = *next;
            }
            Inspected::V2(next) => *self = Self::from_v2(*next),
            Inspected::V1(next) => *self = Self::new(*next),
        }
    }
    fn refresh_graph(&mut self, next: GraphView) {
        let old_id = self
            .graph
            .as_ref()
            .and_then(|g| g.nodes.get(self.graph_index))
            .map(|n| n.id.clone());
        let changed = self
            .extended
            .as_ref()
            .is_some_and(|old| old.source_identity_sha256 != next.base.source_identity_sha256);
        self.observation = next.base.base.clone();
        self.extended = Some(next.base.clone());
        self.graph = Some(next);
        if changed {
            self.stage = Stage::Candidate;
            self.selected_attempt = 5;
            self.graph_edge = 0;
            self.graph_back.clear();
            self.detail_scroll = 0;
            self.notice = "Source identity changed; inspection anchor cleared".into();
        } else {
            if let (Some(id), Some(graph)) = (old_id, &self.graph) {
                self.graph_index = graph.nodes.iter().position(|n| n.id == id).unwrap_or(0);
            }
            self.notice = "Owner graph refreshed under admitted read".into();
        }
    }
    fn key(&mut self, code: KeyCode) -> bool {
        if self.stage == Stage::Graph {
            match code {
                KeyCode::Char('q') | KeyCode::Esc => return true,
                KeyCode::Char('v') => {
                    self.stage = Stage::Candidate;
                    self.detail_scroll = 0;
                    return false;
                }
                KeyCode::Char(']') | KeyCode::Down | KeyCode::Char('j') => {
                    let count = self.graph_edges().len();
                    if count > 0 {
                        self.graph_edge = (self.graph_edge + 1) % count;
                        self.detail_scroll = 0;
                    }
                    return false;
                }
                KeyCode::Char('[') | KeyCode::Up | KeyCode::Char('k') => {
                    let count = self.graph_edges().len();
                    if count > 0 {
                        self.graph_edge = (self.graph_edge + count - 1) % count;
                        self.detail_scroll = 0;
                    }
                    return false;
                }
                KeyCode::PageDown => {
                    self.detail_scroll = self.detail_scroll.saturating_add(8);
                    return false;
                }
                KeyCode::PageUp => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(8);
                    return false;
                }
                KeyCode::Char('J') => {
                    self.detail_scroll = self.detail_scroll.saturating_add(1);
                    return false;
                }
                KeyCode::Char('K') => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                    return false;
                }
                KeyCode::Right | KeyCode::Enter | KeyCode::Char('l') | KeyCode::Char('n') => {
                    let target = self.graph.as_ref().and_then(|g| {
                        let edge = *self.graph_edges().get(self.graph_edge)?;
                        let id = if g.edges[edge].from == g.nodes[self.graph_index].id {
                            &g.edges[edge].to
                        } else {
                            &g.edges[edge].from
                        };
                        g.nodes.iter().position(|n| &n.id == id)
                    });
                    if let Some(target) = target {
                        self.graph_back.push(self.graph_index);
                        self.graph_index = target;
                        self.graph_edge = 0;
                        self.detail_scroll = 0;
                    }
                    return false;
                }
                KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') | KeyCode::Char('p') => {
                    if let Some(index) = self.graph_back.pop() {
                        self.graph_index = index;
                        self.graph_edge = 0;
                        self.detail_scroll = 0;
                    }
                    return false;
                }
                KeyCode::Char('c') => {
                    let head = self
                        .observation
                        .reviews
                        .iter()
                        .find(|r| r.label == "replacement-generic");
                    if let (Some(graph), Some(head)) = (&self.graph, head)
                        && let Some(index) = graph.nodes.iter().position(|n| {
                            n.kind == "review_revision"
                                && n.source_ref == head.key
                                && n.source_revision == head.head
                        })
                    {
                        self.graph_back.push(self.graph_index);
                        self.graph_index = index;
                        self.graph_edge = 0;
                        self.detail_scroll = 0;
                        self.notice = "CASE NAVIGATION to candidate-5 review head; no causal edge from prior node".into();
                    }
                    return false;
                }
                KeyCode::Char('a') => {
                    if let (Some(graph), Some(extended)) = (&self.graph, &self.extended)
                        && let Some(attempt) =
                            extended.provider.attempts.get(self.selected_provider)
                        && let Some(index) = graph.nodes.iter().position(|n| {
                            n.kind == "provider_result" && n.local_id == attempt.result_uuid
                        })
                    {
                        self.graph_back.push(self.graph_index);
                        self.graph_index = index;
                        self.graph_edge = 0;
                        self.detail_scroll = 0;
                        self.notice = "CASE NAVIGATION to selected captured CLI result; no scheduler-cause edge".into();
                    }
                    return false;
                }
                KeyCode::Char('s') => {
                    if let Some(graph) = &self.graph
                        && let Some(index) = graph
                            .nodes
                            .iter()
                            .position(|n| n.kind == "scheduler_current_row")
                    {
                        self.graph_back.push(self.graph_index);
                        self.graph_index = index;
                        self.graph_edge = 0;
                        self.detail_scroll = 0;
                        self.notice = "CASE NAVIGATION to current scheduler row; no provider-UUID causal edge".into();
                    }
                    return false;
                }
                KeyCode::Char('g') => {
                    self.stage = Stage::Gaps;
                    return false;
                }
                _ => (),
            }
        }
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('1'..='5') => {
                if let KeyCode::Char(digit) = code {
                    self.selected_attempt = digit.to_digit(10).unwrap() as usize;
                }
                if self.selected_attempt == 4 {
                    self.selected_provider = 5;
                }
                self.stage = Stage::Candidate;
                self.detail_scroll = 0;
            }
            KeyCode::Right | KeyCode::Enter | KeyCode::Char('l') | KeyCode::Char('n') => {
                self.stage = if self.stage == Stage::Candidate
                    && self.selected_attempt == 4
                    && self.extended.is_some()
                {
                    Stage::Provider
                } else {
                    self.stage.next()
                };
                if self.stage == Stage::Scheduler
                    || (self.stage == Stage::Replacement
                        && self.extended.is_some()
                        && self.selected_attempt == 4)
                {
                    // The retained scheduler result is bound to candidate 4, regardless of
                    // the candidate selected before entering this journey.
                    self.selected_attempt = 4;
                }
                self.detail_scroll = 0;
            }
            KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') | KeyCode::Char('p') => {
                self.stage = if self.stage == Stage::Replacement
                    && self.selected_attempt == 4
                    && self.extended.is_some()
                {
                    Stage::Scheduler
                } else {
                    self.stage.previous()
                };
                self.detail_scroll = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.detail_scroll = self.detail_scroll.saturating_add(1)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.detail_scroll = self.detail_scroll.saturating_sub(1)
            }
            KeyCode::Char('g') => {
                self.stage = Stage::Gaps;
                self.detail_scroll = 0;
            }
            KeyCode::Char('v') if self.graph.is_some() => self.open_graph(),
            KeyCode::Char('a') if self.extended.is_some() => {
                self.stage = Stage::Provider;
                self.detail_scroll = 0;
            }
            KeyCode::Char('s') if self.extended.is_some() => {
                self.stage = Stage::Scheduler;
                self.selected_attempt = 4;
                self.detail_scroll = 0;
            }
            KeyCode::Char(']') if self.stage == Stage::Provider => {
                self.selected_provider = (self.selected_provider + 1) % 8;
                self.detail_scroll = 0;
            }
            KeyCode::Char('[') if self.stage == Stage::Provider => {
                self.selected_provider = (self.selected_provider + 7) % 8;
                self.detail_scroll = 0;
            }
            _ => (),
        }
        false
    }
}

fn shortened(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        value.to_string()
    } else if width < 8 {
        value.chars().take(width).collect()
    } else {
        format!(
            "{}…{}",
            value.chars().take(width - 7).collect::<String>(),
            value
                .chars()
                .rev()
                .take(6)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        )
    }
}

fn candidate_reviews(
    observation: &Observation,
    attempt: usize,
) -> Vec<(&ReviewEpisode, &ReviewRevision)> {
    let candidate = &observation.candidates[attempt - 1];
    observation
        .reviews
        .iter()
        .filter_map(|episode| {
            episode
                .revisions
                .iter()
                .find(|revision| {
                    revision.subjects.iter().any(|subject| {
                        subject.owner == "slice-checkpoint"
                            && subject.reference == candidate.checkpoint_ref
                            && subject.revision == candidate.commit_oid
                    })
                })
                .map(|revision| (episode, revision))
        })
        .collect()
}

fn direct_replacement_link<'a>(
    observation: &'a Observation,
    reviews: &[(&ReviewEpisode, &ReviewRevision)],
) -> Option<&'a historical_evidence_wire::ReplacementLink> {
    observation.replacement_links.iter().find(|link| {
        reviews.iter().any(|(episode, revision)| {
            episode.key == link.replacement_key && revision.revision == link.replacement_revision
        })
    })
}

fn lines(state: &State) -> Vec<Line<'static>> {
    let observation = &state.observation;
    let candidate = &observation.candidates[state.selected_attempt - 1];
    let matched = candidate_reviews(observation, state.selected_attempt);
    let link = direct_replacement_link(observation, &matched);
    let original = link.and_then(|link| {
        observation
            .reviews
            .iter()
            .find(|r| r.key == link.original_key)
    });
    let mut out = vec![
        Line::raw(format!(
            "{} · selected candidate {}",
            state.stage.title(),
            state.selected_attempt
        )),
        Line::raw(""),
    ];
    match state.stage {
        Stage::Candidate => {
            out.extend([
                Line::raw(format!("Checkpoint  {}", candidate.checkpoint_ref)),
                Line::raw(format!("Commit      {}", candidate.commit_oid)),
                Line::raw(format!("Tree        {}", candidate.tree_oid)),
                Line::raw(format!("Gate digest {}", candidate.gate_digest)),
            ]);
            if let Some(gate) = &candidate.gate {
                out.push(Line::raw(format!(
                    "Gate        {} / {} top-level checks passed",
                    gate.passed_count, gate.check_count
                )));
                out.push(Line::raw(format!(
                    "Source      {} · capture verified",
                    gate.source_kind
                )));
                out.push(Line::raw(""));
                for check in &gate.checks {
                    out.push(Line::raw(format!(
                        "  {}  {}",
                        check.outcome, check.identity
                    )));
                }
            }
            if state.selected_attempt == 5 {
                out.push(Line::raw(""));
                if let Some(metrics) = state.extended.as_ref().and_then(|v| v.metrics.as_ref()) {
                    out.push(Line::raw(format!(
                        "Frozen metric stage_total {} · row {}",
                        metrics.stage_total, metrics.row_sha256
                    )));
                    out.push(Line::raw(
                        "Five receipt checks versus eight metric stages: meaning unresolved.",
                    ));
                } else {
                    out.push(Line::raw(
                        if state.extended.is_some() {
                            "Frozen metrics unavailable through this admitted read; stage comparison unavailable."
                        } else {
                            "UI evidence map reports 8 metric stages; meaning unestablished."
                        },
                    ));
                    if state.extended.is_none() {
                        out.push(Line::raw(
                            "That metrics row is unavailable through this admitted read.",
                        ));
                    }
                }
            }
        }
        Stage::Replacement => {
            if matched.is_empty() {
                out.push(Line::raw("No exact review subject for this checkpoint."));
            }
            for (review, revision) in &matched {
                out.extend([
                    Line::raw(format!("Episode     {}", review.label)),
                    Line::raw(format!("Review rev  {}", revision.revision)),
                    Line::raw(format!(
                        "Phase       {} · status {}",
                        revision.phase, revision.status
                    )),
                    Line::raw(format!(
                        "Linked      {} revisions to null predecessor",
                        review.revisions.len()
                    )),
                ]);
                if revision.revision == review.head {
                    out.push(Line::raw(format!("Episode head {}", review.head)));
                } else {
                    out.push(Line::raw(
                        "Later case-level revisions have different checkpoint subjects.",
                    ));
                }
                out.push(Line::raw(""));
            }
            if let Some(link) = link {
                out.push(Line::raw(format!(
                    "Authority   {}",
                    link.authority_reference
                )));
                out.push(Line::raw(format!(
                    "Original    {} @ {}",
                    link.original_key, link.original_reported_revision
                )));
            } else {
                out.push(Line::raw(
                    "No direct replacement edge from this selected review revision.",
                ));
            }
            out.push(Line::raw(
                "Reported review phase is not retirement or acceptance.",
            ));
        }
        Stage::Original => {
            if let (Some(link), Some(review)) = (link, original) {
                let reported = review
                    .revisions
                    .iter()
                    .find(|r| r.revision == link.original_reported_revision);
                out.extend([
                    Line::raw(format!("Episode     {}", review.key)),
                    Line::raw(format!("Reported    {}", link.original_reported_revision)),
                    Line::raw(format!("Head        {}", review.head)),
                ]);
                if let Some(revision) = reported {
                    out.push(Line::raw(format!(
                        "Payload     {}",
                        revision.payload_sha256
                    )));
                    out.push(Line::raw(format!(
                        "Phase       {} · status {}",
                        revision.phase, revision.status
                    )));
                    for subject in &revision.subjects {
                        out.push(Line::raw(format!(
                            "Subject     {} @ {}",
                            subject.reference, subject.revision
                        )));
                    }
                }
                out.push(Line::raw(
                    "The replacement edge is an exact retained authority reference.",
                ));
            } else {
                out.push(Line::raw(
                    "No direct original-report edge from this selected candidate revision.",
                ));
                out.push(Line::raw(
                    "Select candidate 5 to follow the later case-level replacement report.",
                ));
            }
        }
        Stage::Findings => {
            if matched.is_empty() {
                out.push(Line::raw("No exact finding revision for this checkpoint."));
            }
            for (review, revision) in &matched {
                out.push(Line::raw(format!(
                    "{} · {}",
                    review.label, revision.revision
                )));
                for finding in &revision.findings {
                    out.push(Line::raw(format!(
                        "  {}  {}  ({})",
                        finding.status, finding.id, finding.severity
                    )));
                }
            }
            out.push(Line::raw(
                "Finding text is withheld; status and lineage are owner summaries.",
            ));
        }
        Stage::Provider => {
            if let Some(v2) = &state.extended {
                let attempt = &v2.provider.attempts[state.selected_provider];
                out.extend([
                    Line::raw(format!(
                        "Captured result {} / {}",
                        state.selected_provider + 1,
                        v2.provider.attempts.len()
                    )),
                    Line::raw(format!(
                        "Eight UUIDs: {} success / {} error · ${} total",
                        v2.provider.successes, v2.provider.errors, v2.provider.cost_literal_sum_usd
                    )),
                    Line::raw(format!("UUID       {}", attempt.result_uuid)),
                    Line::raw(format!(
                        "Outcome    {} · HTTP {} · exit {}",
                        if attempt.is_error { "error" } else { "success" },
                        attempt
                            .api_error_status
                            .map_or("—".into(), |n| n.to_string()),
                        attempt.exit_code
                    )),
                    Line::raw(format!(
                        "Cost       ${} reported CLI literal",
                        attempt.cost_usd
                    )),
                    Line::raw(format!(
                        "Tokens     input {} · cache create {} · cache read {}",
                        attempt.usage.input_tokens,
                        attempt.usage.cache_creation_input_tokens,
                        attempt.usage.cache_read_input_tokens
                    )),
                    Line::raw(format!(
                        "           output {} · thinking {}",
                        attempt.usage.output_tokens, attempt.usage.thinking_tokens
                    )),
                    Line::raw(format!(
                        "Session    {} · {}",
                        attempt.session_id, attempt.review_label
                    )),
                    Line::raw(format!(
                        "Reason     {} · {}",
                        attempt.terminal_reason, attempt.command_kind
                    )),
                    Line::raw(format!(
                        "Capture    event {} · line {}",
                        attempt.event_id, attempt.source_line
                    )),
                    Line::raw(format!("Line SHA   {}", attempt.source_line_sha256)),
                    Line::raw(format!(
                        "Binding    {} · source limited",
                        attempt.attribution
                    )),
                    Line::raw(""),
                ]);
                if let Some(metrics) = &v2.metrics {
                    out.push(Line::raw(format!(
                        "Frozen metric ${} · equal at 7 decimal places, not exact",
                        metrics.provider_cost_usd
                    )));
                    out.push(Line::raw(format!(
                        "Thinking   {} captured vs {} metric; failed 429 contributes 27",
                        v2.provider.usage_total.thinking_tokens, metrics.provider_thinking_tokens
                    )));
                    out.push(Line::raw(format!(
                        "Calls      {} review successes; metric {} across roles",
                        v2.provider.successes, metrics.provider_successful_calls
                    )));
                } else {
                    out.push(Line::raw(format!(
                        "Frozen metrics comparison unavailable through this read: {}",
                        v2.metrics_unavailable
                            .as_deref()
                            .unwrap_or("unknown source error")
                    )));
                    out.push(Line::raw(
                        "Failed 429 result separately reports 27 thinking tokens.",
                    ));
                }
                out.push(Line::raw(
                    "[ / ] selects captured result · n follows schedule",
                ));
            } else {
                out.push(Line::raw("Provider capture is not admitted by this grant."));
            }
        }
        Stage::Scheduler => {
            if let Some(v2) = &state.extended {
                let item = &v2.scheduler;
                let revision = item
                    .result_ref
                    .split_once('@')
                    .map_or("unavailable", |(_, r)| r);
                out.extend([
                    Line::raw("Current candidate-4 quota retry item"),
                    Line::raw(format!(
                        "Status      {} · store rev {}",
                        item.status, item.store_revision
                    )),
                    Line::raw(format!("Result rev  {revision}")),
                    Line::raw(format!("Due         {}", item.due_at)),
                    Line::raw(format!(
                        "Presented   {}",
                        item.presented_at
                            .as_deref()
                            .unwrap_or("null — no presentation event established")
                    )),
                    Line::raw("No mutation history or result-UUID causal edge."),
                    Line::raw(format!(
                        "Intent      candidate {} · session {}",
                        item.candidate_attempt, item.session_id
                    )),
                    Line::raw(format!("Item ID     {}", item.item_id)),
                    Line::raw(format!("Row SHA     {}", item.row_sha256)),
                    Line::raw(format!("Updated     {}", item.updated_at)),
                    Line::raw(format!("Result      {}", item.result_ref)),
                    Line::raw(
                        "Exact result ref resolves to admitted candidate-4 replacement revision.",
                    ),
                    Line::raw("n follows candidate-4 review · a returns to captured attempts"),
                ]);
            } else {
                out.push(Line::raw("Scheduler item is not admitted by this grant."));
            }
        }
        Stage::Graph => {
            if let Some(graph) = &state.graph
                && let Some(node) = graph.nodes.get(state.graph_index)
            {
                let Some(source) = graph.source(node.source_index) else {
                    out.push(Line::raw("Graph source index unavailable; frame refused."));
                    return out;
                };
                out.extend([
                    Line::raw(format!("Node        {}", node.kind)),
                    Line::raw(format!("Identity    {}", shortened(&node.id, 38))),
                    Line::raw(format!("Class       {:?}", node.evidence_class)),
                    Line::raw(format!("Revision    {}", node.source_revision)),
                    Line::raw(format!("Meaning     {}", shortened(&node.label, 44))),
                ]);
                let outgoing = state.graph_edges();
                if outgoing.is_empty() {
                    out.push(Line::raw("No admitted edge at this node."));
                }
                for (position, edge_index) in outgoing.iter().enumerate() {
                    let edge = &graph.edges[*edge_index];
                    let forward = edge.from == node.id;
                    let peer = if forward { &edge.to } else { &edge.from };
                    let arrow = if forward { "→" } else { "←" };
                    out.push(Line::raw(format!(
                        "{} {} {} {}",
                        if position == state.graph_edge {
                            "▶"
                        } else {
                            " "
                        },
                        shortened(&edge.kind, 23),
                        arrow,
                        shortened(peer, 17)
                    )));
                }
                out.push(Line::raw(""));
                if let Some(edge_index) = outgoing.get(state.graph_edge) {
                    let edge = &graph.edges[*edge_index];
                    out.push(Line::raw(format!(
                        "Selected edge  {} · {:?}",
                        edge.owner, edge.evidence_class
                    )));
                    out.push(Line::raw(format!(
                        "Edge source    {} @ {}",
                        edge.source_ref, edge.source_revision
                    )));
                    out.push(Line::raw(format!("Edge limit     {}", edge.limit)));
                    if edge.kind == "clarifies_reported_continuation_for" {
                        if let Some(support) = &graph.interpretation_support {
                            out.push(Line::raw(format!("Interpreter     {}", edge.owner)));
                            out.push(Line::raw(format!(
                                "Rule/version    {} @ {}",
                                edge.source_ref, edge.source_revision
                            )));
                            out.push(Line::raw(format!(
                                "Git support     {}",
                                support.repair_commit_oid
                            )));
                            out.push(Line::raw(format!(
                                "Script blob     {}",
                                support.repair_script_blob_oid
                            )));
                            out.push(Line::raw(format!(
                                "SKILL blob      {}",
                                support.repair_skill_blob_oid
                            )));
                            out.push(Line::raw(format!(
                                "Review support  {}",
                                support.review_episode_key
                            )));
                            out.push(Line::raw(format!(
                                "Review revision {}",
                                support.review_revision
                            )));
                        }
                    }
                }
                out.push(Line::raw(format!("Owner       {}", node.owner)));
                out.push(Line::raw(format!(
                    "Source      {} · {}",
                    node.source_class, node.source_ref
                )));
                out.push(Line::raw(format!(
                    "Coverage    {} · {}",
                    source.completeness, source.confidentiality
                )));
                out.push(Line::raw(format!("Limit       {}", source.limit)));
                if node.kind == "contract_revision" {
                    if let Some(repair) = &graph.repair_read {
                        out.push(Line::raw(format!("Commit      {}", repair.commit_oid)));
                        out.push(Line::raw(format!("Parent      {}", repair.parent_oid)));
                        out.push(Line::raw(format!("Script path {}", repair.script_path)));
                        out.push(Line::raw(format!("Before rule {}", repair.before_rule)));
                        out.push(Line::raw(format!("After rule  {}", repair.after_rule)));
                        out.push(Line::raw(
                            "Source revision is not S5 deployment or effect evidence.",
                        ));
                    }
                }
                if let Some(time) = &node.observed_time {
                    out.push(Line::raw(format!(
                        "Observed    {}: {}",
                        time.meaning, time.value
                    )));
                }
                if let Some(time) = &node.effective_time {
                    out.push(Line::raw(format!(
                        "Effective   {}: {}",
                        time.meaning, time.value
                    )));
                }
                if let Some(measurement) = &node.measurement {
                    out.push(Line::raw(format!(
                        "Measurement {} = {} {}",
                        measurement.name, measurement.value, measurement.unit
                    )));
                }
                out.push(Line::raw(
                    "[ / ] edge · Enter follow · ← back · c head · a CLI · s current row · v close",
                ));
                out.push(Line::raw(
                    "Case navigation is not a causal edge; unsupported relationships remain gaps.",
                ));
            } else {
                out.push(Line::raw("No admitted owner graph available."));
            }
        }
        Stage::Gaps => {
            for gap in state
                .extended
                .as_ref()
                .map_or(&observation.gaps, |v| &v.gaps)
            {
                out.push(Line::raw(format!("• {gap}")));
            }
            if let Some(graph) = &state.graph {
                for gap in &graph.gaps {
                    out.push(Line::raw(format!("• {gap}")));
                }
            }
            if state.extended.is_none() {
                out.push(Line::raw(
                    "No provider or scheduler inference is made from this view.",
                ));
            }
            out.push(Line::raw(""));
            out.push(Line::raw(format!(
                "Current read grant: {}",
                observation.grant_id
            )));
            out.push(Line::raw(format!(
                "Bound cutoff: {} · local read: {}",
                observation.cutoff, observation.read_at
            )));
            out.push(Line::raw(format!(
                "Source anchor: {}",
                observation.source_identity_sha256
            )));
            for source in state
                .extended
                .as_ref()
                .map_or(&observation.source_reads, |v| &v.source_reads)
            {
                out.push(Line::raw(format!(
                    "{} · {} · {} · {}",
                    source.owner, source.integrity, source.confidentiality, source.completeness
                )));
                out.push(Line::raw(format!("  {}", source.identity)));
            }
        }
    }
    out
}

fn draw(frame: &mut Frame, state: &State) {
    let area = frame.area();
    if area.width < 60 || area.height < 18 {
        frame.render_widget(
            Paragraph::new("S5 HISTORICAL READ · resize to 60×18 · q exits"),
            area,
        );
        return;
    }
    let [header, body, gaps, footer] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(8),
        Constraint::Length(6),
        Constraint::Length(1),
    ])
    .areas(area);
    let identity = &state.observation;
    let head = vec![
        Line::from(vec![
            Span::styled(
                " S5 HISTORICAL INVESTIGATION ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  READ-ONLY · exact owner sources"),
        ]),
        Line::raw(format!(
            " {} / slice {} · grant {} · cutoff {}",
            identity.run_id, identity.slice_number, identity.grant_id, identity.cutoff
        )),
        Line::raw(format!(
            " Source anchor {} · read {}",
            shortened(
                state
                    .extended
                    .as_ref()
                    .map_or(&identity.source_identity_sha256, |v| &v
                        .source_identity_sha256),
                24
            ),
            identity.read_at
        )),
        Line::raw(if state.stage == Stage::Graph {
            format!(
                " [ ] edge  Enter follow  ← back  PgDn/PgUp detail +{}  J/K line  v close",
                state.detail_scroll
            )
        } else if state.graph.is_some() {
            " 1–5 candidate  → follow  a attempts  s schedule  v owner graph  g gaps  q".into()
        } else if state.extended.is_some() {
            " 1–5 candidate  → follow  a attempts  s schedule  [ ] result  ↑↓ scroll  g gaps  q"
                .into()
        } else {
            " 1–5 candidate   →/Enter follow   ← back   ↑↓ scroll   g gaps   q exit".into()
        }),
    ];
    frame.render_widget(
        Paragraph::new(head).style(Style::default().fg(Color::White).bg(Color::Rgb(20, 25, 32))),
        header,
    );
    let [list, details] =
        Layout::horizontal([Constraint::Length(25), Constraint::Min(25)]).areas(body);
    let mut list_lines = vec![Line::raw("Candidates")];
    for candidate in identity.candidates.iter().take(5) {
        let selected = state.selected_attempt == candidate.attempt as usize;
        let checks = candidate.gate.as_ref().map_or("unavailable".into(), |g| {
            format!("{}/{} passed", g.passed_count, g.check_count)
        });
        list_lines.push(Line::styled(
            format!(
                "{} {}  {}",
                if selected { "▶" } else { " " },
                candidate.kind,
                checks
            ),
            Style::default().fg(if selected { Color::Cyan } else { Color::White }),
        ));
    }
    list_lines.push(Line::raw(""));
    list_lines.push(Line::raw("Review episodes"));
    for review in &identity.reviews {
        list_lines.push(Line::raw(format!(
            "{} · {} rev",
            review.label,
            review.revisions.len()
        )));
    }
    if let Some(v2) = &state.extended {
        list_lines.push(Line::raw(""));
        list_lines.push(Line::raw(format!(
            "Provider {} results",
            v2.provider.attempts.len()
        )));
        list_lines.push(Line::raw("429 · captured error"));
        list_lines.push(Line::raw(format!("Retry · {}", v2.scheduler.status)));
    }
    if let Some(graph) = &state.graph {
        list_lines.push(Line::raw(""));
        list_lines.push(Line::raw(format!(
            "Owner graph {} nodes",
            graph.nodes.len()
        )));
        list_lines.push(Line::raw(format!("{} justified edges", graph.edges.len())));
        list_lines.push(Line::raw("v opens exact graph"));
    }
    list_lines.push(Line::raw(""));
    list_lines.push(Line::raw("Accepted is a distinct"));
    list_lines.push(Line::raw("checkpoint subject."));
    frame.render_widget(
        Paragraph::new(list_lines)
            .block(Block::default().borders(Borders::ALL).title(" Exact case ")),
        list,
    );
    frame.render_widget(
        Paragraph::new(lines(state))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Owner-backed detail "),
            )
            .scroll((state.detail_scroll, 0))
            .wrap(Wrap { trim: false }),
        details,
    );
    let gap_text = if let Some(v2) = &state.extended {
        vec![
            Line::styled(
                "VISIBLE LIMITS",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw("CLI result capture is not billing authority; no UUID→schedule causal edge."),
            Line::raw("Scheduler current row: presented_at null; no mutation history."),
            Line::raw(format!(
                "5 receipt checks / {} frozen metric stages unresolved · full UI-0 open",
                v2.metrics
                    .as_ref()
                    .map_or("unavailable".into(), |m| m.stage_total.to_string())
            )),
        ]
    } else {
        vec![
            Line::styled(
                "VISIBLE LIMITS",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw("Gate capture proves digest binding, not test execution."),
            Line::raw("5 receipt checks; UI map says 8 metric stages (row unadmitted)."),
            Line::raw("Metrics/provider/quota/schedule lack admission; full UI-0 remains open."),
        ]
    };
    frame.render_widget(
        Paragraph::new(gap_text).block(Block::default().borders(Borders::ALL)),
        gaps,
    );
    frame.render_widget(
        Paragraph::new(format!(" {}", state.notice)).style(Style::default().fg(Color::DarkGray)),
        footer,
    );
}

fn read_state(socket: &Path) -> Result<State, Box<dyn std::error::Error>> {
    match inspect_best(socket) {
        Ok(Inspected::GraphV2(observation)) => State::from_graph_v2(*observation)
            .ok_or_else(|| "invalid graph v2 display projection".into()),
        Ok(Inspected::Graph(observation)) => Ok(State::from_graph(*observation)),
        Ok(Inspected::V2(observation)) => Ok(State::from_v2(*observation)),
        Ok(Inspected::V1(observation)) => Ok(State::new(*observation)),
        Err(_) => Err("admitted historical read unavailable".into()),
    }
}

pub fn snapshot(
    socket: &Path,
    width: u16,
    height: u16,
    stage: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if !(60..=300).contains(&width) || !(18..=100).contains(&height) {
        return Err("history snapshot size is invalid".into());
    }
    let mut state = read_state(socket)?;
    state.stage = match stage {
        "candidate" => Stage::Candidate,
        "replacement" => Stage::Replacement,
        "original" => Stage::Original,
        "findings" => Stage::Findings,
        "provider" if state.extended.is_some() => Stage::Provider,
        "scheduler" if state.extended.is_some() => Stage::Scheduler,
        "graph" if state.graph.is_some() => {
            state.open_graph();
            Stage::Graph
        }
        "gaps" => Stage::Gaps,
        _ => return Err("unknown historical stage".into()),
    };
    if matches!(state.stage, Stage::Provider | Stage::Scheduler) {
        state.selected_attempt = 4;
        state.selected_provider = 5;
    }
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    terminal.draw(|frame| draw(frame, &state))?;
    let buffer = terminal.backend().buffer();
    let mut output = String::new();
    for y in 0..height {
        for x in 0..width {
            output.push_str(buffer[(x, y)].symbol());
        }
        output.push('\n');
    }
    Ok(output)
}

struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
        let _ = disable_raw_mode();
    }
}

fn draw_pending(frame: &mut Frame, message: &str, elapsed: Duration) {
    let lines = vec![
        Line::raw("Local historical evidence read"),
        Line::raw(""),
        Line::raw(message.to_string()),
        Line::raw(format!("Elapsed: {:.1}s", elapsed.as_secs_f64())),
        Line::raw("q/Esc: leave    r: retry"),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Historical read")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
        frame.area(),
    );
}

pub fn run(socket: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !socket.is_absolute() {
        return Err("--history-connect requires an absolute Unix socket".into());
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("history mode needs a terminal".into());
    }
    enable_raw_mode()?;
    let _restore = Restore;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut state: Option<State> = None;
    let mut message = "Local evidence read in progress; no evidence loaded".to_string();
    let mut started = Instant::now();
    let mut last_read = started;
    let mut generation = 0u64;
    // Draw before even beginning connection/admission. The first frame asserts no
    // historical fact and remains interactive while the owner reads.
    terminal.draw(|frame| draw_pending(frame, &message, started.elapsed()))?;
    let mut active =
        match InspectAttempt::start_graph_v2(&socket, started + Duration::from_secs(35)) {
            Ok(attempt) => {
                generation += 1;
                Some((generation, attempt))
            }
            Err(error) => {
                message = format!("Local evidence read unavailable: {error:?}; press r to retry");
                None
            }
        };
    loop {
        if let Some(state) = state.as_ref() {
            terminal.draw(|frame| draw(frame, state))?;
        } else {
            terminal.draw(|frame| draw_pending(frame, &message, started.elapsed()))?;
        }
        if event::poll(Duration::from_millis(20))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('r') => {
                    drop(active.take()); // Close the prior socket before retry.
                    started = Instant::now();
                    message = "Local evidence read in progress; no evidence loaded".into();
                    if let Some(state) = state.as_mut() {
                        state.notice =
                            "LAST OBSERVED · local refresh requested; admission pending".into();
                    }
                    generation += 1;
                    active = match InspectAttempt::start_graph_v2(
                        &socket,
                        started + Duration::from_secs(35),
                    ) {
                        Ok(attempt) => Some((generation, attempt)),
                        Err(error) => {
                            message = format!(
                                "Local evidence read unavailable: {error:?}; press r to retry"
                            );
                            if let Some(state) = state.as_mut() {
                                state.notice = format!(
                                    "LAST OBSERVED · refresh unavailable: {error:?}; stale"
                                );
                            }
                            None
                        }
                    };
                }
                code => {
                    if let Some(state) = state.as_mut() {
                        state.key(code);
                    }
                }
            }
        }
        if let Some((attempt_generation, attempt)) = active.as_mut()
            && let Some(result) = attempt.poll()
        {
            let completed_generation = *attempt_generation;
            active = None;
            if completed_generation == generation {
                match result {
                    Ok(observation) => {
                        if let Some(state) = state.as_mut() {
                            state.refresh_with(observation);
                        } else {
                            state = Some(match observation {
                                Inspected::GraphV2(observation) => {
                                    let Some(next) = State::from_graph_v2(*observation) else {
                                        message = "Local graph v2 display projection invalid; press r to retry".into();
                                        continue;
                                    };
                                    next
                                }
                                Inspected::V1(observation) => State::new(*observation),
                                Inspected::V2(observation) => State::from_v2(*observation),
                                Inspected::Graph(observation) => State::from_graph(*observation),
                            });
                        }
                    }
                    Err(error) => {
                        message =
                            format!("Local evidence read unavailable: {error:?}; press r to retry");
                        if let Some(state) = state.as_mut() {
                            state.notice =
                                format!("LAST OBSERVED · refresh {error:?}; prior frame stale");
                        }
                    }
                }
            }
            last_read = Instant::now();
        }
        if active.is_none() && state.is_some() && last_read.elapsed() >= Duration::from_secs(4) {
            started = Instant::now();
            if let Some(state) = state.as_mut() {
                state.notice = "LAST OBSERVED · refreshing; admission pending".into();
            }
            generation += 1;
            active =
                match InspectAttempt::start_graph_v2(&socket, started + Duration::from_secs(35)) {
                    Ok(attempt) => Some((generation, attempt)),
                    Err(error) => {
                        if let Some(state) = state.as_mut() {
                            state.notice =
                                format!("LAST OBSERVED · refresh {error:?}; prior frame stale");
                        }
                        last_read = Instant::now();
                        None
                    }
                };
        }
    }
    Ok(())
}
