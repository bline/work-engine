//! One admitted, read-only historical investigation in the terminal client.
use crossterm::{
    cursor::Show,
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use historical_evidence_client::{ReadError, inspect, inspect_v2};
use historical_evidence_wire::{
    Observation, ObservationV2, Request, RequestV2, ReviewEpisode, ReviewRevision,
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
            Self::Gaps => "Evidence gaps",
        }
    }
}

struct State {
    observation: Observation,
    extended: Option<ObservationV2>,
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
    fn refresh(&mut self, socket: &Path) {
        if self.extended.is_some() {
            match inspect_v2(socket, RequestV2::all()) {
                Ok(next) => {
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
                    self.extended = Some(next);
                }
                Err(error) => {
                    self.notice =
                        format!("Historical v2 read unavailable: {error:?}; prior frame is stale")
                }
            }
            return;
        }
        match inspect(socket, Request::all()) {
            Ok(next) => {
                if next.source_identity_sha256 != self.observation.source_identity_sha256 {
                    self.stage = Stage::Candidate;
                    self.selected_attempt = 5;
                    self.detail_scroll = 0;
                    self.notice = "Source identity changed; inspection anchor cleared".into();
                } else {
                    self.notice = "Owner sources refreshed under the same read grant".into();
                }
                self.observation = next;
            }
            Err(error) => {
                self.notice =
                    format!("Historical read unavailable: {error:?}; prior frame is stale");
            }
        }
    }
    fn key(&mut self, code: KeyCode) -> bool {
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
        Stage::Gaps => {
            for gap in state
                .extended
                .as_ref()
                .map_or(&observation.gaps, |v| &v.gaps)
            {
                out.push(Line::raw(format!("• {gap}")));
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
        Line::raw(if state.extended.is_some() {
            " 1–5 candidate  → follow  a attempts  s schedule  [ ] result  ↑↓ scroll  g gaps  q"
        } else {
            " 1–5 candidate   →/Enter follow   ← back   ↑↓ scroll   g gaps   q exit"
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
    match inspect_v2(socket, RequestV2::all()) {
        Ok(observation) => Ok(State::from_v2(observation)),
        Err(ReadError::Refused) => {
            let observation = inspect(socket, Request::all())
                .map_err(|_| "admitted historical v1 read unavailable")?;
            Ok(State::new(observation))
        }
        Err(_) => Err("admitted historical v2 read unavailable".into()),
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

pub fn run(socket: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !socket.is_absolute() {
        return Err("--history-connect requires an absolute Unix socket".into());
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("history mode needs a terminal".into());
    }
    let mut state = read_state(&socket)?;
    enable_raw_mode()?;
    let _restore = Restore;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut last_read = Instant::now();
    loop {
        terminal.draw(|frame| draw(frame, &state))?;
        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
            && state.key(key.code)
        {
            break;
        }
        if last_read.elapsed() >= Duration::from_secs(4) {
            state.refresh(&socket);
            last_read = Instant::now();
        }
    }
    Ok(())
}
