use crate::{
    fixture::Scope,
    model::{Model, Msg, Pane, Update, View},
};
use std::{cell::RefCell, rc::Rc};
use tuirealm::{
    application::Application,
    command::{Cmd, CmdResult},
    component::{AppComponent, Component},
    event::Event,
    props::{AttrValue, Attribute, QueryResult},
    ratatui::{
        Frame, Terminal,
        backend::TestBackend,
        layout::{Constraint, Layout, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap},
    },
    state::State,
};
pub type App = Application<Pane, Msg, Update>;
pub type Shared = Rc<RefCell<Model>>;
use crate::theme::*;

pub struct Panel {
    pub id: Pane,
    pub model: Shared,
}
impl Component for Panel {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let mut m = self.model.borrow_mut();
        m.mouse.panes.push((area, self.id));
        match self.id {
            Pane::Main => main_view(frame, area, &mut m),
            Pane::Context => context(frame, area, &mut m),
            Pane::Composer => composer(frame, area, &mut m),
        }
    }
    fn query(&self, _: Attribute) -> Option<QueryResult<'_>> {
        None
    }
    fn attr(&mut self, _: Attribute, _: AttrValue) {}
    fn state(&self) -> State {
        State::None
    }
    fn perform(&mut self, _: Cmd) -> CmdResult {
        CmdResult::NoChange
    }
}
impl AppComponent<Msg, Update> for Panel {
    fn on(&mut self, event: &Event<Update>) -> Option<Msg> {
        Some(Msg::Input(self.id, event.clone()))
    }
}
pub(crate) fn block(title: impl Into<String>, focused: bool) -> Block<'static> {
    Block::default()
        .title(format!(
            " {} {} ",
            if focused { "▌" } else { " " },
            title.into()
        ))
        .title_style(
            Style::default()
                .fg(if focused { INK } else { MUTED })
                .add_modifier(if focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        )
        .padding(Padding::new(1, 1, 0, 1))
        .style(
            Style::default()
                .fg(if focused { INK } else { QUIET })
                .bg(BG),
        )
}
fn heading(s: impl Into<String>) -> Line<'static> {
    Line::styled(
        s.into(),
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    )
}
fn muted(s: impl Into<String>) -> Line<'static> {
    Line::styled(s.into(), Style::default().fg(MUTED))
}
fn status_color(s: &str) -> Color {
    match s {
        "NEEDS YOU" | "WAITING" | "BLOCKED" => AMBER,
        "ACTIVE" => ACCENT,
        "AVAILABLE" => ACCENT,
        _ => MUTED,
    }
}

pub fn render(frame: &mut Frame, app: &mut App, model: &Shared) {
    let area = frame.area();
    model.borrow_mut().mouse = Default::default();
    model.borrow_mut().transcript.area = Rect::default();
    frame.render_widget(
        Block::default().style(Style::default().bg(BG).fg(INK)),
        area,
    );
    if area.width < 60 || area.height < 18 {
        frame.render_widget(Paragraph::new(format!("WORK ENGINE / {}\nResize to at least 60x18.\nDrafts retained. Ctrl-C exits (y confirms if a draft exists).", if model.borrow().task.as_ref().is_some_and(|t| t.live.is_some()) { "TASK OBSERVATION · READ-ONLY" } else if model.borrow().task.is_some() { "PROVISIONAL TASK FIXTURE · READ-ONLY" } else if model.borrow().connection.is_some() { "PASSIVE CLIENT · READ-ONLY" } else if model.borrow().replay.is_some() { "HISTORICAL REPLAY" } else { "FIXTURE" }))
            .wrap(Wrap {trim:false}),area);
        return;
    }
    let composer_height = {
        let m = model.borrow();
        m.drafts[m.recipient].height(area.width, area.height)
    };
    let [header, tabs, body, compose, notice, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(composer_height),
        Constraint::Length(1),
        Constraint::Length(2),
    ])
    .areas(area);
    {
        let mut m = model.borrow_mut();
        if m.view != View::Conversation {
            m.transcript.clear();
        }
        let views = if m.task.is_some() {
            vec![
                View::Overview,
                View::Dashboard,
                View::Conversation,
                View::Lifecycle,
            ]
        } else {
            vec![
                View::Dashboard,
                View::History,
                View::Conversation,
                View::Lifecycle,
            ]
        };
        let mut x = tabs.x;
        for view in views.iter().copied() {
            let width = view.label().len() as u16 + 2;
            m.mouse.tabs.push((Rect::new(x, tabs.y, width, 1), view));
            x += width;
        }
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        " WORK ENGINE ",
                        Style::default()
                            .bg(ACCENT)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  Supervision desk"),
                    Span::styled(
                        if let Some(task) = &m.task {
                            if task.live.is_some() {
                                "    TASK OBSERVATION · READ-ONLY"
                            } else {
                                "    PROVISIONAL TASK FIXTURE"
                            }
                        } else if let Some(connection) = m.connection.as_ref().filter(|_| {
                            m.client_observation.is_some() && m.view == View::Conversation
                        }) {
                            if connection.disconnected {
                                "    PASSIVE CLIENT • DISCONNECTED"
                            } else if connection.refreshing {
                                "    PASSIVE CLIENT • READING"
                            } else {
                                "    PASSIVE CLIENT • READ-ONLY"
                            }
                        } else if m.contract_case.is_some() && m.view == View::Conversation {
                            "    S3 CONTRACT EXAMPLE • OFFLINE"
                        } else if m.client_observation.is_some() && m.view == View::Conversation {
                            "    PASSIVE CLIENT • READ-ONLY"
                        } else if m.replay.is_some() && m.view == View::Conversation {
                            "    HISTORICAL • OFFLINE REPLAY"
                        } else if m.view == View::Lifecycle {
                            "    READ-ONLY • NO LIVE CONNECTION"
                        } else {
                            "    FIXTURE • NO ENGINE CONNECTION"
                        },
                        Style::default().fg(AMBER),
                    ),
                ]),
                muted(if let Some(task) = &m.task {
                    format!(" {}", task.identity())
                } else if let Some(connection) = m
                    .connection
                    .as_ref()
                    .filter(|_| m.client_observation.is_some() && m.view == View::Conversation)
                {
                    format!(
                        " subject {}  |  socket {}  |  read-only",
                        connection.subject.as_str(),
                        connection.socket.display()
                    )
                } else {
                    format!(
                        " {}  |  {}  |  cost: unavailable",
                        if let Some(case) = m.contract_case.filter(|_| m.view == View::Conversation)
                        {
                            case.label()
                        } else if m.client_observation.is_some() && m.view == View::Conversation {
                            "CLIENT OBSERVATION"
                        } else if let Some(replay) =
                            m.replay.as_ref().filter(|_| m.view == View::Conversation)
                        {
                            replay.label
                        } else if m.view == View::Lifecycle {
                            if m.lifecycle_case == 0 {
                                "EXPERIMENTAL LIFECYCLE / SIMULATED INFERENCE"
                            } else {
                                "LIFECYCLE / DESIGN FIXTURE"
                            }
                        } else {
                            m.scope.label()
                        },
                        if m.input_read_only() {
                            "messaging unavailable · Ctrl-P commands".into()
                        } else if m.scope == Scope::Live {
                            format!("sample activity {}", m.pulses)
                        } else {
                            "partial retained snapshot".into()
                        }
                    )
                }),
            ]),
            header,
        );
        let tab_line = Line::from(
            views
                .into_iter()
                .map(|v| {
                    Span::styled(
                        format!(" {} ", v.label()),
                        if m.view == v {
                            Style::default()
                                .bg(SELECTED)
                                .fg(ACCENT)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(MUTED)
                        },
                    )
                })
                .chain(std::iter::once(Span::styled(
                    format!(
                        "   Detail: {}",
                        ["Summary", "Activity", "Evidence"][m.detail]
                    ),
                    Style::default().fg(MUTED),
                )))
                .collect::<Vec<_>>(),
        );
        frame.render_widget(Paragraph::new(tab_line), tabs);
        frame.render_widget(
            Paragraph::new(format!(
                " {}",
                if m.view == View::Lifecycle && m.notice.starts_with("Start:") {
                    "Read-only lifecycle inspection · Ctrl-P scenarios and steps"
                } else {
                    &m.notice
                }
            ))
            .style(Style::default().fg(AMBER)),
            notice,
        );
        frame.render_widget(Paragraph::new(if m.input_read_only() || m.scope==Scope::Historical {
            " Messaging unavailable · drafts retained · Ctrl-P commands · F1 help\n Drag text to select Markdown · Ctrl-C copies selection; otherwise quits"
        } else {
            " Enter send · Shift-Enter / Alt-Enter newline · Ctrl-P commands · F1 help\n Ctrl-O full draft · Drag text to select · Ctrl-C copies selection; otherwise quits"
        }).style(Style::default().fg(MUTED)),footer);
        let control = Rect::new(tabs.right().saturating_sub(19), tabs.y + 1, 19, 1);
        m.mouse.commands = control;
        frame.render_widget(
            Paragraph::new(" Commands  Ctrl-P ").style(Style::default().bg(SELECTED).fg(ACCENT)),
            control,
        );
    }
    if area.width >= 110 {
        let [main, side] =
            Layout::horizontal([Constraint::Min(55), Constraint::Length(40)]).areas(body);
        app.view(&Pane::Main, frame, main);
        app.view(&Pane::Context, frame, side);
    } else {
        let pane = if model.borrow().show_context {
            Pane::Context
        } else {
            Pane::Main
        };
        app.view(&pane, frame, body);
    }
    app.view(&Pane::Composer, frame, compose);
    let mut m = model.borrow_mut();
    if m.palette.is_some() {
        command_palette(frame, Rect::new(area.x, area.y, area.width, compose.y), &m);
    }
    if m.paste_preview.is_some() {
        paste_preview(frame, body, &mut m);
    }
    if m.help || m.confirm_quit {
        let width = area.width.saturating_sub(4).min(82);
        let height = area
            .height
            .saturating_sub(2)
            .min(if m.confirm_quit { 7 } else { 24 });
        let popup = Rect::new(
            (area.width - width) / 2,
            (area.height - height) / 2,
            width,
            height,
        );
        frame.render_widget(Clear, popup);
        let text = if m.confirm_quit {
            "Drafts exist in this session.\nExit and discard all drafts? y = discard and exit; n / Esc = keep working.\nNothing has been sent to an engine."
        } else {
            "A supervision desk with labeled fixtures, offline historical evidence, and explicit passive observation.\n\nCtrl-P           Commands from any view; composer draft is untouched\nPalette          Type to search; arrows select, Tab completes, Enter runs\nConnection       Refresh/reconnect/disconnect only issue bounded passive reads\nEsc              Close palette or clear message selection\nTyping           Always addresses the composer; / is ordinary text\nTab              Never cycles panel focus\nInspect / event  Select work or jump to timeline entries without retargeting\nPanel main       Show main content on a narrow terminal\nPanel context    Show context on a narrow terminal\nScroll commands  Timeline / context up, down, start, end\nScenario         Inspect lifecycle examples; inspect step opens details\nF3               Summary / Activity / Evidence detail\nF5               Live fixture / historical fixture scope\nF7               Explicitly target selected work owner in fixture\nEnter            Simulate send in fixture; Shift-Enter / Alt-Enter adds a line\nCtrl-O           Inspect full draft including folded pastes\nEditor           Arrows, selection, Ctrl-Z undo, Ctrl-Y redo\nMouse            Click view tabs / work items; wheel scrolls under pointer\nDrag in chat     Select source Markdown; Ctrl-C copies; Esc clears\nCtrl-C           Exit when no chat selection; drafts require confirmation\n\nConnected observations are read-only; no command authority is wired.\nF1 or Esc closes this help."
        };
        let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
        m.help_scroll = m.help_scroll.min(
            paragraph
                .line_count(width.saturating_sub(2))
                .saturating_sub(height.saturating_sub(2) as usize) as u16,
        );
        frame.render_widget(
            paragraph
                .scroll((if m.confirm_quit { 0 } else { m.help_scroll }, 0))
                .block(
                    block(
                        if m.confirm_quit {
                            "Exit prototype"
                        } else {
                            "Keyboard guide · Up/Down scroll · Esc close"
                        },
                        true,
                    )
                    .style(Style::default().bg(PALETTE).fg(INK)),
                ),
            popup,
        );
    }
}
fn main_view(frame: &mut Frame, area: Rect, m: &mut Model) {
    if m.task.is_some() && m.view != View::Conversation {
        return task_view(frame, area, m);
    }
    match m.view {
        View::Overview => dashboard(frame, area, m),
        View::Dashboard => dashboard(frame, area, m),
        View::History => history(frame, area, m),
        View::Conversation => conversation(frame, area, m),
        View::Lifecycle => crate::lifecycle::main(frame, area, m),
    }
}
fn dashboard(frame: &mut Frame, area: Rect, m: &mut Model) {
    let use_graph = m.graph_mode && area.width >= 65 && area.height >= 24;
    let b = block(
        if use_graph {
            "Work graph · arrows show work relationships"
        } else if m.graph_mode {
            "Work list · compact layout"
        } else {
            "Work list · same subjects as graph"
        },
        !m.show_context,
    );
    let inner = b.inner(area);
    frame.render_widget(b, area);
    let [summary, content, legend] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(inner);
    frame.render_widget(Paragraph::new(if m.scope==Scope::Live {" 1 decision needs you    1 active build\n Objective → review → decision; build proceeds in parallel"} else {" approach-a · retained snapshot · superseded\n Compare with main: F5 changes time without changing subject"}).style(Style::default().bg(WARM).fg(if m.scope==Scope::Live {AMBER} else {MUTED})),summary);
    if use_graph {
        if m.last_graph_size != (content.width, content.height) {
            m.flow.request_fit_view();
            m.last_graph_size = (content.width, content.height);
        }
        frame.render_widget(&mut m.flow, content);
        // Use the graph's post-render transform, clipped exactly like node painting.
        // Register only visible cells; negative/offscreen bounds must not wrap to u16.
        for (index, item) in m.items().iter().enumerate() {
            if let Some((left, top, right, bottom)) = m.flow.node_terminal_rect(item.id) {
                let left = left.max(i32::from(content.x));
                let top = top.max(i32::from(content.y));
                let right = right.min(i32::from(content.right()));
                let bottom = bottom.min(i32::from(content.bottom()));
                if left < right && top < bottom {
                    m.mouse.items.push((
                        Rect::new(
                            left as u16,
                            top as u16,
                            (right - left) as u16,
                            (bottom - top) as u16,
                        ),
                        index,
                    ));
                }
            }
        }
    } else {
        let items = m
            .items()
            .iter()
            .map(|w| {
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!(" {:11} ", w.status),
                            Style::default().fg(status_color(w.status)),
                        ),
                        Span::raw(w.title),
                    ]),
                    muted(format!("              {} · {}", w.kind, w.owner)),
                ])
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default().with_selected(Some(m.selected));
        frame.render_stateful_widget(
            List::new(items)
                .highlight_symbol("› ")
                .highlight_style(Style::default().bg(SELECTED)),
            content,
            &mut state,
        );
        for (row, index) in (state.offset()..m.items().len()).enumerate() {
            let y = content.y + row as u16 * 2;
            if y < content.bottom() {
                m.mouse.items.push((
                    Rect::new(content.x, y, content.width, 2.min(content.bottom() - y)),
                    index,
                ));
            }
        }
    }
    frame.render_widget(
        Paragraph::new(format!(
            " Selected: {}  |  Ctrl-P inspect",
            m.selected_work().title
        ))
        .style(Style::default().fg(ACCENT)),
        legend,
    );
}
fn context(frame: &mut Frame, area: Rect, m: &mut Model) {
    if (m.view == View::Conversation || m.task.is_some())
        && let Some((title, content)) = m.conversation().context
    {
        let b = block(title, m.show_context).style(
            Style::default()
                .bg(if m.task.is_some() { CODE_BG } else { CONTEXT })
                .fg(INK),
        );
        let inner = b.inner(area);
        let text = Paragraph::new(content).wrap(Wrap { trim: false });
        m.context_scroll = m.context_scroll.min(
            text.line_count(inner.width)
                .saturating_sub(inner.height as usize)
                .min(u16::MAX as usize) as u16,
        );
        frame.render_widget(text.scroll((m.context_scroll, 0)).block(b), area);
        return;
    }

    if m.view == View::Lifecycle {
        return crate::lifecycle::context(frame, area, m);
    }
    let w = m.selected_work();
    let b = block(
        format!(
            "Context / {}",
            ["Overview", "Evidence", "History"][m.context_tab]
        ),
        m.show_context,
    )
    .style(
        Style::default()
            .bg(CONTEXT)
            .fg(if m.show_context { INK } else { QUIET }),
    );
    let mut lines = vec![
        heading(w.title),
        muted(format!("{} · {}", w.kind, w.id)),
        Line::styled(w.status, Style::default().fg(status_color(w.status))),
        Line::raw(""),
    ];
    match m.context_tab {
        0 => lines.extend(vec![
            heading("OBJECTIVE / MEANING"),
            Line::raw(w.summary),
            Line::raw(""),
            heading("WAIT / NEXT DECISION"),
            Line::raw(w.wait),
            Line::raw(""),
            heading("RESPONSIBLE ROLE"),
            Line::raw(w.owner),
            Line::raw(""),
            muted("Ctrl-P: conversation / inspect"),
            muted(if m.scope == Scope::Live {
                "F7: target this role explicitly"
            } else {
                "F5: return to live before targeting"
            }),
            muted("F4: evidence / history"),
        ]),
        1 => {
            lines.push(heading("EVIDENCE & LIMITS"));
            lines.extend(w.evidence.lines().map(Line::raw));
            lines.extend([
                Line::raw(""),
                muted("All references here are invented."),
                muted("No inference of completion, delivery, or authority."),
            ]);
        }
        _ => {
            lines.push(heading("SUBJECT HISTORY"));
            lines.extend(w.history.lines().map(Line::raw));
            lines.extend([
                Line::raw(""),
                muted(m.scope.label()),
                muted("Branch history is partial fixture data."),
            ]);
        }
    }
    if m.scope == Scope::Historical {
        lines.push(Line::styled(
            "HISTORICAL · INSPECT ONLY",
            Style::default().fg(AMBER),
        ));
    }
    let inner = b.inner(area);
    let text = Paragraph::new(lines).wrap(Wrap { trim: false });
    let max = text
        .line_count(inner.width)
        .saturating_sub(inner.height as usize)
        .min(u16::MAX as usize) as u16;
    m.context_scroll = m.context_scroll.min(max);
    frame.render_widget(text.scroll((m.context_scroll, 0)).block(b), area);
}
fn history(frame: &mut Frame, area: Rect, m: &mut Model) {
    let b = block("Branches & history · F5 switches snapshot", !m.show_context);
    let inner = b.inner(area);
    frame.render_widget(b, area);
    let compact = inner.height < 16;
    let [branches, subjects] = Layout::vertical([
        Constraint::Length(if compact {
            inner.height.saturating_sub(1).min(4)
        } else {
            9
        }),
        Constraint::Min(1),
    ])
    .areas(inner);
    frame.render_widget(
        Paragraph::new(if compact {
            vec![
                muted(" ├─ approach-a r11 · superseded"),
                heading(" └─ main r18 · waiting"),
                muted(format!(" Inspecting: {}", m.scope.label())),
            ]
        } else {
            vec![
                heading(" objective:ui"),
                muted(" ├─ approach-a @ r11   embedded UI / superseded"),
                heading(" └─ main @ r18         standalone UI / waiting"),
                Line::raw(""),
                Line::raw(" Comparison: runtime-coupled → independent client"),
                muted(" Candidate A / adapter B → candidate B / adapter A"),
                Line::styled(
                    format!(" Inspecting: {}", m.scope.label()),
                    Style::default().fg(AMBER),
                ),
                muted(" Same subject IDs; different revisions and evidence."),
            ]
        }),
        branches,
    );
    let items = m
        .items()
        .into_iter()
        .map(|w| ListItem::new(format!("{}  ·  {}", w.title, w.status)))
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(Some(m.selected));
    frame.render_stateful_widget(
        List::new(items)
            .highlight_symbol("› ")
            .highlight_style(Style::default().bg(SELECTED)),
        subjects,
        &mut state,
    );
    for (row, index) in (state.offset()..m.items().len()).enumerate() {
        let y = subjects.y + row as u16;
        if y < subjects.bottom() {
            m.mouse
                .items
                .push((Rect::new(subjects.x, y, subjects.width, 1), index));
        }
    }
}
fn conversation(frame: &mut Frame, area: Rect, m: &mut Model) {
    let conversation = m.conversation();
    m.synchronize_conversation(conversation.source);
    let b = block(format!("Chat · {}", conversation.usage), !m.show_context);
    let inner = b.inner(area);
    let entries = &conversation.entries;
    let visible = conversation
        .visible(m.detail)
        .map(|(_, entry)| entry)
        .collect::<Vec<_>>();
    m.transcript
        .prepare_detail(&visible, inner.width, m.detail > 0);
    if let Some(index) = m.pending_event.take() {
        if entries
            .get(index)
            .is_some_and(|entry| entry.detail <= m.detail)
        {
            let visible_index = entries
                .iter()
                .take(index)
                .filter(|entry| entry.detail <= m.detail)
                .count();
            if let Some(row) = m.transcript.entry_rows.get(visible_index) {
                m.scroll = (*row).min(u16::MAX as usize) as u16;
            }
        } else {
            m.inspected_event = None;
        }
    }
    m.transcript.rows[0].metadata = Some(conversation.banner);
    m.transcript.rows.last_mut().unwrap().metadata =
        Some(if m.detail >= 1 && conversation.simulated_activity {
            format!(
                "Sample background activity: {} / builder:main / no real execution",
                m.pulses
            )
        } else {
            String::new()
        });
    let max = m
        .transcript
        .rows
        .len()
        .saturating_sub(inner.height as usize)
        .min(u16::MAX as usize) as u16;
    m.scroll = m.scroll.min(max);
    m.transcript.area = inner;
    m.transcript.scroll = m.scroll as usize;
    frame.render_widget(b, area);
    let lines =
        m.transcript
            .rows
            .iter()
            .skip(m.scroll as usize)
            .take(inner.height as usize)
            .map(|row| {
                if let Some(text) = &row.metadata {
                    return muted(text.clone());
                }
                Line::from(
                    row.glyphs
                        .iter()
                        .map(|g| {
                            let selected =
                                m.transcript.selection.as_ref().is_some_and(|r| {
                                    r.start < g.source.end && r.end > g.source.start
                                });
                            Span::styled(
                                g.text.clone(),
                                if selected {
                                    Style::default().bg(SELECTED).fg(INK)
                                } else {
                                    g.style
                                },
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), inner);
}
fn composer(frame: &mut Frame, area: Rect, m: &mut Model) {
    if m.input_read_only() || m.scope == Scope::Historical {
        frame.render_widget(Paragraph::new("Read-only inspection. No native commands, inference, or replayed execution.\nYour work-fixture drafts are retained. Ctrl-P navigation and inspection remain available.")
            .wrap(Wrap{trim:false}).block(block(format!("Messaging unavailable · draft → {} retained", m.recipient), m.focus == Pane::Composer)
                .style(Style::default().bg(if m.task.is_some() { BG } else { COMPOSER }).fg(MUTED))), area);
        return;
    }
    let title = format!(
        "Draft → {} | {}",
        m.recipient,
        if m.scope == Scope::Historical {
            "HISTORY: SEND DISABLED"
        } else {
            "Enter SEND · Shift-Enter newline · SIMULATED"
        }
    );
    let focused = m.focus == Pane::Composer && m.palette.is_none() && m.paste_preview.is_none();
    let historical = m.scope == Scope::Historical;
    let folds = m.drafts[m.recipient].folded_count();
    let title = if folds > 0 {
        format!(
            "Draft → {} | {} paste(s) · Ctrl-O inspect · {}",
            m.recipient,
            folds,
            if historical {
                "SEND DISABLED"
            } else {
                "Enter send"
            }
        )
    } else {
        title
    };
    let draft = m.draft().view();
    draft.set_placeholder_text(if historical {
        "Drafts are retained. F5 returns to live; simulated send is disabled here."
    } else {
        "Enter sends · Shift-Enter or Alt-Enter newline · Ctrl-P commands · paste stays in draft."
    });
    draft.set_block(
        block(title, focused).style(Style::default().bg(COMPOSER).fg(if focused {
            INK
        } else {
            QUIET
        })),
    );
    draft.set_style(
        Style::default()
            .bg(COMPOSER)
            .fg(if focused { INK } else { QUIET }),
    );
    draft.set_cursor_style(if focused {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    });
    frame.render_widget(&*draft, area);
}
pub fn snapshot(app: &mut App, model: &Shared, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| render(f, app, model)).unwrap();
    tuirealm::testing::buffer_to_string(terminal.backend().buffer())
}

fn command_palette(frame: &mut Frame, body: Rect, m: &Model) {
    let palette = m.palette.as_ref().unwrap();
    let matches = palette.matches();
    let selected = palette.selected.min(matches.len().saturating_sub(1));
    let width = body.width.saturating_sub(2);
    let height = body.height.saturating_sub(1).min(15);
    let area = Rect::new(body.x + 1, body.y + 1, width, height);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default().style(Style::default().bg(PALETTE).fg(INK)),
        area,
    );
    let inner = Rect::new(
        area.x + 1,
        area.y,
        area.width.saturating_sub(2),
        area.height,
    );
    let [title, search, content, hint] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(inner);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "▌ Commands",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("    {} matches · LOCAL UI", matches.len()),
                Style::default().fg(MUTED),
            ),
        ])),
        title,
    );
    let query: String = palette
        .query
        .chars()
        .rev()
        .take(inner.width.saturating_sub(4) as usize)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    frame.render_widget(
        Paragraph::new(format!("> {}▏", query)).style(Style::default().fg(INK)),
        search,
    );
    let (list, preview) = if content.width >= 90 {
        let [a, b] = Layout::horizontal([Constraint::Percentage(48), Constraint::Percentage(52)])
            .areas(content);
        (a, Some(b))
    } else if content.height >= 6 {
        let [a, b] = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).areas(content);
        (a, Some(b))
    } else {
        (content, None)
    };
    if matches.is_empty() {
        frame.render_widget(
            Paragraph::new("No matches. Try a view, role, or 'evidence'.")
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(MUTED)),
            list,
        );
    } else {
        let rows = matches
            .iter()
            .map(|c| {
                let unavailable = c
                    .unavailable(
                        m.scope,
                        if m.input_read_only() {
                            View::Lifecycle
                        } else {
                            m.view
                        },
                    )
                    .is_some();
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<24} ", c.name),
                        Style::default().fg(if unavailable { MUTED } else { INK }),
                    ),
                    Span::styled(
                        if unavailable { "UNAVAILABLE" } else { c.group },
                        Style::default().fg(if unavailable { AMBER } else { LAVENDER }),
                    ),
                ]))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default().with_selected(Some(selected));
        frame.render_stateful_widget(
            List::new(rows)
                .highlight_symbol("› ")
                .highlight_style(Style::default().bg(SELECTED)),
            list,
            &mut state,
        );
    }
    if let Some(preview) = preview {
        let lines = if let Some(command) = matches.get(selected) {
            vec![
                Line::styled(
                    format!(" {}", command.group),
                    Style::default().fg(LAVENDER).add_modifier(Modifier::BOLD),
                ),
                Line::raw(format!(
                    " {}",
                    command
                        .unavailable(
                            m.scope,
                            if m.input_read_only() {
                                View::Lifecycle
                            } else {
                                m.view
                            }
                        )
                        .unwrap_or(command.description)
                )),
                Line::styled(
                    format!(
                        " {}",
                        command
                            .unavailable(
                                m.scope,
                                if m.input_read_only() {
                                    View::Lifecycle
                                } else {
                                    m.view
                                }
                            )
                            .unwrap_or("Local only · no message sent")
                    ),
                    Style::default().fg(
                        if command
                            .unavailable(
                                m.scope,
                                if m.input_read_only() {
                                    View::Lifecycle
                                } else {
                                    m.view
                                },
                            )
                            .is_some()
                        {
                            AMBER
                        } else {
                            ACCENT
                        },
                    ),
                ),
            ]
        } else {
            vec![muted(" Search names, descriptions, or abbreviations.")]
        };
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(CONTEXT).fg(INK)),
            preview,
        );
    }
    frame.render_widget(
        Paragraph::new("↑↓ select   Tab complete   Enter run   Esc cancel · draft retained")
            .style(Style::default().fg(MUTED)),
        hint,
    );
}

/// Row-major color snapshot, without entering or changing terminal modes.
pub fn snapshot_ansi(app: &mut App, model: &Shared, width: u16, height: u16) -> String {
    use std::fmt::Write;
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| render(f, app, model)).unwrap();
    let buffer = terminal.backend().buffer();
    let mut output = String::new();
    for y in 0..height {
        for x in 0..width {
            let cell = &buffer[(x, y)];
            let fg = rgb(cell.fg, true);
            let bg = rgb(cell.bg, false);
            write!(
                output,
                "\x1b[38;2;{};{};{};48;2;{};{};{}m{}",
                fg.0,
                fg.1,
                fg.2,
                bg.0,
                bg.1,
                bg.2,
                cell.symbol()
            )
            .unwrap();
        }
        output.push_str("\x1b[0m\n");
    }
    output
}
fn rgb(color: Color, foreground: bool) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::White => (255, 255, 255),
        Color::Indexed(n) if n >= 232 => {
            let v = 8 + (n - 232) * 10;
            (v, v, v)
        }
        Color::Indexed(n) if n >= 16 => {
            let n = n - 16;
            let v = |c| if c == 0 { 0 } else { 55 + c * 40 };
            (v(n / 36), v(n / 6 % 6), v(n % 6))
        }
        _ => {
            if foreground {
                (237, 231, 219)
            } else {
                (28, 31, 36)
            }
        }
    }
}

fn paste_preview(frame: &mut Frame, area: Rect, m: &mut Model) {
    let preview = m.paste_preview.as_mut().unwrap();
    frame.render_widget(Clear, area);
    let b = block("Full draft · read-only · ↑↓/PgUp/PgDn · Esc close", true)
        .style(Style::default().bg(PALETTE).fg(INK));
    let inner = b.inner(area);
    let text = Paragraph::new(preview.text.as_str()).wrap(Wrap { trim: false });
    let max = text
        .line_count(inner.width)
        .saturating_sub(inner.height as usize)
        .min(u16::MAX as usize) as u16;
    preview.scroll = preview.scroll.min(max);
    frame.render_widget(text.scroll((preview.scroll, 0)).block(b), area);
}

fn task_view(frame: &mut Frame, area: Rect, m: &mut Model) {
    let task = m.task.as_ref().unwrap();
    if m.view == View::Lifecycle {
        let text = [1, 2]
            .into_iter()
            .map(|i| {
                let c = &task.components[i];
                format!("{}\n{}\n{}", c.summary(), c.binding, c.detail)
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let b = block("Task lifecycle · independent components", !m.show_context);
        let inner = b.inner(area);
        let paragraph = Paragraph::new(format!("{} · non-atomic reads\n\n{text}\n\nNative state is not task completion. Source references and bound segments are available in Graph.", if task.live.is_some() { "TASK OBSERVATION" } else { "PROVISIONAL TASK FIXTURE" })).wrap(Wrap { trim:false });
        m.scroll = m.scroll.min(
            paragraph
                .line_count(inner.width)
                .saturating_sub(inner.height as usize) as u16,
        );
        frame.render_widget(paragraph.scroll((m.scroll, 0)).block(b), area);
        return;
    }
    if m.view == View::Dashboard {
        task_graph(frame, area, m);
    } else {
        task_overview(frame, area, m);
    }
}

fn task_overview(frame: &mut Frame, area: Rect, m: &mut Model) {
    let task = m.task.as_ref().unwrap();
    let b = block("Overview", false);
    let inner = b.inner(area);
    let mut lines = Vec::new();
    let mut hits = Vec::new();
    let mut row = 0;
    for section in task.overview() {
        let selected = section.node == Some(task.selected);
        let first_row = row;
        let heading = Line::styled(
            format!("{} {}", if selected { "▸" } else { " " }, section.title),
            Style::default()
                .fg(if section.warning { AMBER } else { ACCENT })
                .add_modifier(Modifier::BOLD),
        );
        row += Paragraph::new(heading.clone())
            .wrap(Wrap { trim: false })
            .line_count(inner.width);
        lines.push(heading);
        for text in section.body.lines() {
            let line = Line::styled(
                format!("  {text}"),
                Style::default().fg(if section.warning { AMBER } else { INK }),
            );
            row += Paragraph::new(line.clone())
                .wrap(Wrap { trim: false })
                .line_count(inner.width);
            lines.push(line);
        }
        if let Some(index) = section.node {
            hits.push((first_row, row, index));
        }
        lines.push(Line::raw(""));
        row += 1;
    }
    lines.push(muted(
        "  Inputs and recorded effects → Graph · sources → Context",
    ));
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    m.scroll = m.scroll.min(
        paragraph
            .line_count(inner.width)
            .saturating_sub(inner.height as usize) as u16,
    );
    for (start, end, index) in hits {
        let visible_start = start.max(usize::from(m.scroll));
        let visible_end = end.min(usize::from(m.scroll + inner.height));
        if visible_start < visible_end {
            m.mouse.items.push((
                Rect::new(
                    inner.x,
                    inner.y + (visible_start - usize::from(m.scroll)) as u16,
                    inner.width,
                    (visible_end - visible_start) as u16,
                ),
                index,
            ));
        }
    }
    frame.render_widget(paragraph.scroll((m.scroll, 0)).block(b), area);
}

/// A small identifier-bound graph. Layout is presentation only; edges come from Node.parent.
fn task_graph(frame: &mut Frame, area: Rect, m: &mut Model) {
    let b = block(
        "Graph · click to inspect · double-click for Conversation",
        false,
    );
    let inner = b.inner(area);
    frame.render_widget(b, area);
    let task = m.task.as_ref().unwrap();
    if inner.width < 82 || inner.height < 24 {
        task_graph_list(frame, inner, m);
        return;
    }
    let width = ((inner.width - 12) / 3).min(28);
    let columns = [inner.x + 1, inner.x + width + 6, inner.x + 2 * width + 11];
    let mut placements = Vec::new();
    if task.live.is_some() {
        let roots: Vec<_> = task
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.parent.is_none())
            .collect();
        let children: Vec<_> = task
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.parent.as_deref() == Some(task.nodes[0].id.as_ref()))
            .collect();
        let leaves: Vec<_> = task
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.parent.as_deref() == Some("native_delivery_status"))
            .collect();
        let capacity = (inner.height.saturating_sub(1) / 4) as usize;
        if roots.len().max(children.len()).max(leaves.len()) > capacity {
            task_graph_list(frame, inner, m);
            return;
        }
        for (column, nodes) in [roots, children, leaves].into_iter().enumerate() {
            for (row, (index, _)) in nodes.into_iter().enumerate() {
                placements.push((
                    index,
                    Rect::new(columns[column], inner.y + row as u16 * 4, width, 3),
                ));
            }
        }
    } else {
        let root = task.nodes.iter().position(|n| n.parent.is_none()).unwrap();
        placements.push((root, Rect::new(columns[0], inner.y + 10, width, 3)));
        let mut child_row = 0;
        for (index, node) in task.nodes.iter().enumerate() {
            if node.parent.as_deref() == Some(task.nodes[root].id.as_ref()) {
                placements.push((
                    index,
                    Rect::new(columns[1], inner.y + child_row * 5, width, 3),
                ));
                child_row += 1;
            }
        }
        let delivery = placements
            .iter()
            .find(|(i, _)| task.nodes[*i].id == "delivery:d1")
            .map(|(_, r)| *r);
        if let Some(delivery) = delivery {
            for (offset, (index, _)) in task
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.parent.as_deref() == Some("delivery:d1"))
                .enumerate()
            {
                placements.push((
                    index,
                    Rect::new(columns[2], delivery.y + offset as u16 * 5, width, 3),
                ));
            }
        }
    }
    let edge_style = Style::default().fg(MUTED);
    for (index, rect) in &placements {
        let node = &task.nodes[*index];
        if let Some(parent) = node.parent.as_deref().and_then(|id| {
            placements
                .iter()
                .find(|(i, _)| task.nodes[*i].id == id)
                .map(|(_, r)| *r)
        }) {
            let from = (parent.right(), parent.y + 1);
            let to = (rect.x, rect.y + 1);
            let bend = from.0 + 2;
            for x in from.0..to.0 {
                frame.render_widget(
                    Paragraph::new("─").style(edge_style),
                    Rect::new(x, if x < bend { from.1 } else { to.1 }, 1, 1),
                );
            }
            for y in from.1.min(to.1)..=from.1.max(to.1) {
                frame.render_widget(
                    Paragraph::new("│").style(edge_style),
                    Rect::new(bend, y, 1, 1),
                );
            }
            frame.render_widget(
                Paragraph::new("›").style(edge_style),
                Rect::new(to.0 - 1, to.1, 1, 1),
            );
        }
    }
    for (index, rect) in placements {
        let node = &task.nodes[index];
        let component = &task.components[node.component];
        let selected = index == task.selected;
        let style = Style::default()
            .bg(if selected { SELECTED } else { CODE_BG })
            .fg(if selected { INK } else { QUIET });
        frame.render_widget(Block::default().style(style), rect);
        let title = Line::styled(
            format!("{} {}", if selected { "▸" } else { " " }, node.caption()),
            style
                .fg(if selected { ACCENT } else { INK })
                .add_modifier(Modifier::BOLD),
        );
        let status = if component.stale {
            "STALE · last known"
        } else if task.live.is_some() {
            node.status()
        } else if node.id.starts_with("segment:") {
            if node.id == "segment:n1" {
                "seq 1 · context A"
            } else {
                "seq 2 · context B"
            }
        } else if node.id == "delivery:d1" {
            if task.question_pending {
                "Running · admission open"
            } else {
                "Unsettled · stopped"
            }
        } else {
            node.status()
        };
        frame.render_widget(
            Paragraph::new(vec![
                title,
                Line::styled(
                    format!("  {status}"),
                    style.fg(if component.stale { AMBER } else { MUTED }),
                ),
            ])
            .wrap(Wrap { trim: false }),
            rect,
        );
        m.mouse.items.push((rect, index));
    }
    let note = if task.live.is_some() {
        "Edges show supplied scope bindings; command/task binding is not established."
    } else if task.components[2].availability == crate::task::Availability::Refused {
        "Native history refused: capacity · retained nodes, if any, are last known"
    } else if task.components[0].stale {
        "Owner/native reads unavailable · STALE nodes retain last-known data"
    } else {
        "Edges show identifier bindings, not chronology. Details → Context"
    };
    frame.render_widget(
        Paragraph::new(note).style(Style::default().fg(
            if task.case == crate::task::Case::Refused {
                AMBER
            } else {
                MUTED
            },
        )),
        Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
    );
}

fn task_graph_list(frame: &mut Frame, area: Rect, m: &mut Model) {
    let task = m.task.as_ref().unwrap();
    let mut lines = vec![muted("Compact graph · bound relationships")];
    if task.components[2].availability == crate::task::Availability::Refused {
        lines.push(Line::styled(
            "Native history refused: capacity · no collection supplied",
            Style::default().fg(AMBER),
        ));
    } else if task.components[0].stale {
        lines.push(Line::styled(
            "Owner/native reads unavailable · nodes marked STALE",
            Style::default().fg(AMBER),
        ));
    }
    let prefix = lines.len();
    for (index, node) in task.nodes.iter().enumerate() {
        let selected = index == task.selected;
        let style = Style::default()
            .fg(if selected { ACCENT } else { INK })
            .bg(if selected { SELECTED } else { BG });
        let parent = node
            .parent
            .as_deref()
            .and_then(|id| task.nodes.iter().find(|n| n.id == id))
            .map(|n| n.caption());
        lines.push(Line::styled(
            format!("{} {}", if selected { "▸" } else { " " }, node.caption()),
            style,
        ));
        lines.push(Line::styled(
            format!(
                "  {}{}",
                parent.map(|p| format!("{p} → ")).unwrap_or_default(),
                if task.components[node.component].stale {
                    "STALE / last known"
                } else {
                    node.status()
                }
            ),
            Style::default().fg(MUTED),
        ));
    }
    let paragraph = Paragraph::new(lines);
    m.scroll = m.scroll.min(
        paragraph
            .line_count(area.width)
            .saturating_sub(area.height as usize) as u16,
    );
    for index in 0..task.nodes.len() {
        let start = prefix + index * 2;
        let lo = start.max(m.scroll as usize);
        let hi = (start + 2).min(usize::from(m.scroll + area.height));
        if lo < hi {
            m.mouse.items.push((
                Rect::new(
                    area.x,
                    area.y + (lo - m.scroll as usize) as u16,
                    area.width,
                    (hi - lo) as u16,
                ),
                index,
            ));
        }
    }
    frame.render_widget(paragraph.scroll((m.scroll, 0)), area);
}
