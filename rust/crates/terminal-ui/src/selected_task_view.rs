//! File-delivered shared Rust selected-task record. Read-only and explicitly selected.
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, enable_raw_mode},
};
use operator_observation::{now_ms, selected_task::SelectedTask, terminal_text};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::{
    io::{self, IsTerminal},
    time::{Duration, Instant},
};
fn load(path: &str, endpoint: &str, task: &str) -> Result<SelectedTask, &'static str> {
    let s = SelectedTask::parse(&operator_observation::read_snapshot_file(path)?)?;
    if s.endpoint != endpoint || s.request.task_id != task {
        return Err("selected_endpoint_or_task_mismatch");
    };
    Ok(s)
}
fn draw(
    f: &mut Frame,
    s: Option<&SelectedTask>,
    error: Option<&str>,
    detail: bool,
    component: usize,
    scroll: u16,
    selection: (&str, &str),
) {
    let (endpoint, task) = selection;
    let a = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(4),
        Constraint::Length(3),
    ])
    .split(f.area());
    let title = match s {
        Some(s) => format!(
            "SELECTED TASK {task}\nEndpoint {endpoint}\n{} · private operator data · {}\nIndependent components; no atomic cut",
            s.standing, s.transport
        ),
        None => {
            format!("SELECTED TASK {task}\nEndpoint {endpoint}\nNo shared observation established")
        }
    };
    f.render_widget(Paragraph::new(terminal_text(&title)), a[0]);
    let mut body = String::new();
    if let Some(s) = s {
        body = s.human_view_width(
            if detail {
                Some(operator_observation::selected_task::INSPECTORS[component])
            } else {
                None
            },
            now_ms(),
            error.is_some(),
            f.area().width as usize,
        );
        if !detail {
            for (i, name) in operator_observation::selected_task::INSPECTORS
                .iter()
                .enumerate()
            {
                body.push_str(&format!(
                    "{} Inspect {name}\n",
                    if i == component { "▶" } else { " " }
                ));
            }
        }
    }
    f.render_widget(
        Paragraph::new(terminal_text(&body))
            .block(Block::default().borders(Borders::ALL).title(if detail {
                "Detail"
            } else {
                "Owner observations"
            }))
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        a[1],
    );
    let footer = format!(
        "{}\n↑/↓ select {} · Enter inspect · b/Esc back · q quit\nPgUp/PgDn scroll · r refresh",
        error.unwrap_or(
            "File reconciliation 1s; source read times independent; age is a display cue."
        ),
        operator_observation::selected_task::INSPECTORS[component]
    );
    f.render_widget(Paragraph::new(footer), a[2]);
}
pub fn run(path: &str, endpoint: &str, task: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !io::stdout().is_terminal() {
        return Err("selected-task requires a terminal".into());
    }
    enable_raw_mode()?;
    let _guard = crate::TerminalGuard;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let (mut snapshot, mut error, mut detail, mut component, mut scroll) =
        (None, None, false, 0usize, 0u16);
    let mut next = Instant::now();
    loop {
        if Instant::now() >= next {
            match load(path, endpoint, task) {
                Ok(s) => {
                    if snapshot.as_ref().is_some_and(|p: &SelectedTask| {
                        p.owner_basis_identity() != s.owner_basis_identity()
                    }) {
                        detail = false;
                        component = 0;
                        scroll = 0;
                    }
                    snapshot = Some(s);
                    error = None
                }
                Err(e) => {
                    if e == "selected_endpoint_or_task_mismatch" {
                        snapshot = None;
                        detail = false;
                    }
                    error = Some(e)
                }
            }
            next = Instant::now() + Duration::from_secs(1);
        }
        terminal.draw(|f| {
            draw(
                f,
                snapshot.as_ref(),
                error,
                detail,
                component,
                scroll,
                (endpoint, task),
            )
        })?;
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(k) = event::read()?
            && k.kind == KeyEventKind::Press
        {
            match k.code {
                KeyCode::Char('q') => break,
                KeyCode::Esc | KeyCode::Char('b') => {
                    detail = false;
                    scroll = 0
                }
                KeyCode::Enter => {
                    detail = true;
                    scroll = 0
                }
                KeyCode::Up => {
                    component = component.saturating_sub(1);
                    scroll = 0;
                }
                KeyCode::Down => {
                    component = (component + 1).min(7);
                    scroll = 0;
                }
                KeyCode::PageDown => scroll = scroll.saturating_add(10),
                KeyCode::PageUp => scroll = scroll.saturating_sub(10),
                KeyCode::Char('r') => next = Instant::now(),
                _ => (),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! fixture {
        ($name:literal) => {
            (
                $name,
                include_bytes!(concat!(
                    "../../operator-observation/tests/fixtures/selected-task/",
                    $name,
                    ".json"
                ))
                .as_slice(),
            )
        };
    }
    /// The generated consumer set (operator-observation `tests/fixture_set.rs`).
    const SET: [(&str, &[u8]); 7] = [
        fixture!("joined-fresh"),
        fixture!("large-revision-redacted"),
        fixture!("observed-absent"),
        fixture!("retained-anchor-refused"),
        fixture!("stale-present"),
        fixture!("summary-unavailable"),
        fixture!("unobserved"),
    ];
    fn render(
        s: &SelectedTask,
        width: u16,
        height: u16,
        detail: bool,
        route: usize,
        scroll: u16,
    ) -> String {
        let mut terminal =
            Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    Some(s),
                    None,
                    detail,
                    route,
                    scroll,
                    (&s.endpoint, &s.request.task_id),
                )
            })
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .chunks(width as usize)
            .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
    /// Unwrapped text of every route and scroll position at one width.
    fn all_routes(s: &SelectedTask, width: u16) -> String {
        let mut out = String::new();
        for route in 0usize..9 {
            for scroll in (0..400).step_by(20) {
                out.push_str(&render(
                    s,
                    width,
                    60,
                    route != 0,
                    route.saturating_sub(1),
                    scroll,
                ));
                out.push('\n');
            }
        }
        out.split_whitespace().collect::<Vec<_>>().join(" ")
    }
    #[test]
    fn fixture_set_renders_shared_facts_at_narrow_and_wide_widths() {
        for (name, bytes) in SET {
            let s = SelectedTask::parse(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
            let c = &s.components["owner"];
            let retained = s.summary.basis == "last_known_owner";
            let revision = if retained {
                &c.last_known_revision
            } else {
                &c.revision
            };
            let read = if retained {
                c.last_known_at_ms.unwrap()
            } else {
                c.read_at_ms
            };
            for (width, height) in [(80, 24), (80, 60), (180, 60)] {
                let overview = render(&s, width, height, false, 0, 0);
                assert!(overview.contains("Owner observations"), "{name}");
                assert!(!overview.contains("segment_state"), "{name}");
                for i in 0..8 {
                    assert!(
                        render(&s, width, height, true, i, 0).contains("Detail"),
                        "{name}"
                    );
                }
            }
            for width in [80, 180] {
                let text = all_routes(&s, width);
                assert!(
                    text.contains(&format!("Owner record {} · {}", c.state, s.summary.basis)),
                    "{name} {width}"
                );
                for key in ["owner_revision", "cursor_revision"] {
                    if let Some(digits) = revision[key].as_str() {
                        // The labelled field; the closing quote bounds the digits.
                        assert!(
                            text.contains(&format!("\"{key}\": \"{digits}\"")),
                            "{name} {width}: {key} {digits}"
                        );
                    }
                }
                assert!(
                    text.contains(&format!("\"read_at_ms\": {},", c.read_at_ms)),
                    "{name} {width}: read_at_ms {}",
                    c.read_at_ms
                );
                if let Some(t) = c.last_known_at_ms {
                    assert!(
                        text.contains(&format!("\"last_known_at_ms\": {t},")),
                        "{name} {width}: last_known_at_ms {t}"
                    );
                }
                match &s.summary.value {
                    Some(v) => {
                        let counts = format!(
                            "Questions: {} pending · {} answered",
                            v.pending_count, v.answered_count
                        );
                        assert!(text.contains(&counts), "{name} {width}");
                        assert!(text.contains(match v.result {
                            operator_observation::selected_task::ResultSummary::NotPublished => "No result published",
                            _ => "Published result:",
                        }), "{name} {width}");
                    }
                    None => assert!(
                        text.contains("Compact summary unavailable"),
                        "{name} {width}"
                    ),
                }
                assert!(
                    !text.contains("hunter2") && !text.contains("abc.def"),
                    "{name}"
                );
                if name == "large-revision-redacted" {
                    assert!(
                        text.contains("[REDACTED]") && text.contains("excerpt clipped"),
                        "{width}"
                    );
                }
                if name == "joined-fresh" {
                    assert!(
                        text.contains("Yes, publish")
                            && text.contains("Independent synthetic result")
                    );
                }
            }
            println!(
                "TUI {name} | {} | {} | {} | read {read} | {} | {}",
                s.summary.basis,
                c.state,
                if revision.is_null() {
                    "none".into()
                } else {
                    format!(
                        "{}/{}",
                        revision["owner_revision"].as_str().unwrap(),
                        revision["cursor_revision"].as_str().unwrap()
                    )
                },
                s.summary
                    .value
                    .as_ref()
                    .map(|v| format!(
                        "{} pending · {} answered",
                        v.pending_count, v.answered_count
                    ))
                    .unwrap_or_else(|| "unavailable".into()),
                match s.summary.value.as_ref().map(|v| &v.result) {
                    Some(operator_observation::selected_task::ResultSummary::NotPublished) =>
                        "not_published",
                    Some(_) => "published",
                    None => "unavailable",
                }
            );
        }
    }
}
