//! One admitted, read-only historical investigation in the terminal client.
use crossterm::{
    cursor::Show,
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use historical_evidence_client::inspect;
use historical_evidence_wire::{Observation, Request, ReviewEpisode, ReviewRevision};
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
    Gaps,
}
impl Stage {
    fn next(self) -> Self {
        match self {
            Self::Candidate => Self::Replacement,
            Self::Replacement => Self::Original,
            Self::Original => Self::Findings,
            Self::Findings | Self::Gaps => Self::Gaps,
        }
    }
    fn previous(self) -> Self {
        match self {
            Self::Candidate | Self::Replacement => Self::Candidate,
            Self::Original => Self::Replacement,
            Self::Findings => Self::Original,
            Self::Gaps => Self::Findings,
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Candidate => "Candidate and gate",
            Self::Replacement => "Review lineage",
            Self::Original => "Original reported revision",
            Self::Findings => "Finding dispositions",
            Self::Gaps => "Evidence gaps",
        }
    }
}

struct State {
    observation: Observation,
    selected_attempt: usize,
    stage: Stage,
    detail_scroll: u16,
    notice: String,
}
impl State {
    fn new(observation: Observation) -> Self {
        Self {
            observation,
            selected_attempt: 5,
            stage: Stage::Candidate,
            detail_scroll: 0,
            notice: "ADMITTED LOCAL READ · source-limited producer capture · no actions".into(),
        }
    }
    fn refresh(&mut self, socket: &Path) {
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
                self.stage = Stage::Candidate;
                self.detail_scroll = 0;
            }
            KeyCode::Right | KeyCode::Enter | KeyCode::Char('l') | KeyCode::Char('n') => {
                self.stage = self.stage.next();
                self.detail_scroll = 0;
            }
            KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') | KeyCode::Char('p') => {
                self.stage = self.stage.previous();
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
                out.push(Line::raw(
                    "UI evidence map reports 8 metric stages; meaning unestablished.",
                ));
                out.push(Line::raw(
                    "That metrics row is unavailable through this admitted read.",
                ));
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
        Stage::Gaps => {
            for gap in &observation.gaps {
                out.push(Line::raw(format!("• {gap}")));
            }
            out.push(Line::raw(
                "No provider or scheduler inference is made from this view.",
            ));
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
            for source in &observation.source_reads {
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
            shortened(&identity.source_identity_sha256, 24),
            identity.read_at
        )),
        Line::raw(" 1–5 candidate   →/Enter follow   ← back   ↑↓ scroll   g gaps   q exit"),
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
    let gap_text = vec![
        Line::styled(
            "VISIBLE LIMITS",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Line::raw("Gate capture proves digest binding, not test execution."),
        Line::raw("5 receipt checks; UI map says 8 metric stages (row unadmitted)."),
        Line::raw("Metrics/provider/quota/schedule lack admission; full UI-0 remains open."),
    ];
    frame.render_widget(
        Paragraph::new(gap_text).block(Block::default().borders(Borders::ALL)),
        gaps,
    );
    frame.render_widget(
        Paragraph::new(format!(" {}", state.notice)).style(Style::default().fg(Color::DarkGray)),
        footer,
    );
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
    let observation =
        inspect(socket, Request::all()).map_err(|_| "admitted historical read unavailable")?;
    let mut state = State::new(observation);
    state.stage = match stage {
        "candidate" => Stage::Candidate,
        "replacement" => Stage::Replacement,
        "original" => Stage::Original,
        "findings" => Stage::Findings,
        "gaps" => Stage::Gaps,
        _ => return Err("unknown historical stage".into()),
    };
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
    let observation =
        inspect(&socket, Request::all()).map_err(|_| "admitted historical read unavailable")?;
    let mut state = State::new(observation);
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
