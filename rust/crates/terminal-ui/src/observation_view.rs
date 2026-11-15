//! Passive consumer of the same selected-session observation used by the web dashboard.
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, enable_raw_mode},
};
use operator_observation::{Observation, Snapshot, now_ms, terminal_text};
use ratatui::{
    Frame, Terminal,
    backend::{CrosstermBackend, TestBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::{
    io::{self, IsTerminal},
    path::PathBuf,
    time::{Duration, Instant},
};

fn read(path: &str) -> Result<Snapshot, &'static str> {
    let bytes = operator_observation::read_snapshot_file(path)?;
    Snapshot::parse(&bytes)
}
fn draw(
    f: &mut Frame,
    snapshot: Option<&Snapshot>,
    error: Option<&str>,
    selected: usize,
    scroll: u16,
) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(f.area());
    let header = match snapshot {
        Some(s) => format!(
            "{}\n{} selected sessions · observed {} · {}\nScope {}",
            terminal_text(&s.label),
            s.sessions.len(),
            s.observed_at_ms,
            age(s.observed_at_ms),
            s.scope_id
        ),
        None => "Session activity\nWaiting for an observation snapshot".into(),
    };
    f.render_widget(
        Paragraph::new(header).style(Style::default().fg(Color::Cyan)),
        areas[0],
    );
    let mut body = String::new();
    if let Some(s) = snapshot {
        for (i, row) in s.sessions.iter().enumerate() {
            let title = match &row.observation {
                Observation::Available { title, .. } => {
                    title.as_deref().unwrap_or("Untitled session")
                }
                Observation::Unavailable { .. } => "Source unavailable",
            };
            body.push_str(&format!(
                "{} {} · {}\n",
                if i == selected { "▶" } else { " " },
                row.session_id,
                terminal_text(title)
            ));
        }
        if let Some(row) = s.sessions.get(selected) {
            body.push_str(&format!(
                "\nSelected session · {} · {}\n",
                row.session_id,
                age(row.observed_at_ms)
            ));
            match &row.observation {
                Observation::Unavailable { code } => {
                    body.push_str(&format!("Observation unavailable: {code}\n"))
                }
                Observation::Available { host, entries, .. } => {
                    body.push_str(&format!("Provider host: {host}\nRecent published transcript entries (bounded tail)\n\n"));
                    if entries.is_empty() {
                        body.push_str("No matching entries in this bounded read.\n");
                    }
                    for entry in entries {
                        body.push_str(&format!(
                            "{} · {} · {}\n{}\n{}\n\n",
                            entry.role,
                            terminal_text(entry.tool.as_deref().unwrap_or("message")),
                            terminal_text(entry.timestamp.as_deref().unwrap_or("time unavailable")),
                            terminal_text(&entry.text),
                            if entry.truncated {
                                "[excerpt truncated]"
                            } else {
                                ""
                            }
                        ));
                    }
                }
            }
        }
    }
    f.render_widget(
        Paragraph::new(body)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Activity · read-only"),
            )
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        areas[1],
    );
    let status = error
        .map(|e| {
            format!(
                "Refresh failed: {e}. {}",
                if snapshot.is_some() {
                    "Last successful observation only."
                } else {
                    "No observation available."
                }
            )
        })
        .unwrap_or_else(|| {
            "Session activity does not establish task execution, completion or settled usage."
                .into()
        });
    f.render_widget(
        Paragraph::new(format!(
            "{status}\n←/→ select session · PgUp/PgDn scroll · r refresh · q/Esc quit"
        ))
        .style(Style::default().fg(if error.is_some() {
            Color::Yellow
        } else {
            Color::Gray
        })),
        areas[2],
    );
}
fn age(at: u64) -> String {
    let now = now_ms();
    if at > now {
        return "future timestamp; freshness unavailable".into();
    }
    let age = (now - at) / 1000;
    format!(
        "last read {age}s ago{}",
        if age > 120 { " · stale" } else { "" }
    )
}
pub fn snapshot(path: &str, width: u16, height: u16) -> Result<String, Box<dyn std::error::Error>> {
    if !(40..=300).contains(&width) || !(12..=100).contains(&height) {
        return Err("snapshot dimensions must be 40..300 by 12..100".into());
    }
    let s = read(path)?;
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    terminal.draw(|f| draw(f, Some(&s), None, 0, 0))?;
    let mut text = String::new();
    for y in 0..height {
        for x in 0..width {
            text.push_str(terminal.backend().buffer()[(x, y)].symbol());
        }
        text.push('\n');
    }
    Ok(text)
}
pub fn run(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !path.is_absolute() || !io::stdout().is_terminal() {
        return Err("observation mode requires an absolute path and terminal".into());
    }
    let path = path.to_str().ok_or("invalid observation path")?;
    enable_raw_mode()?;
    let _guard = crate::TerminalGuard;
    execute!(io::stdout(), EnterAlternateScreen)?;
    std::panic::set_hook(Box::new(|info| {
        crate::restore();
        eprintln!("{info}");
    }));
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let (mut snapshot, mut error, mut selected, mut scroll) = (None, None, 0, 0u16);
    let mut next = Instant::now();
    loop {
        if Instant::now() >= next {
            match read(path) {
                Ok(s) => {
                    if snapshot
                        .as_ref()
                        .is_some_and(|old: &Snapshot| old.scope_id != s.scope_id)
                    {
                        selected = 0;
                        scroll = 0;
                    }
                    snapshot = Some(s);
                    error = None;
                }
                Err(code) => error = Some(code),
            }
            next = Instant::now() + Duration::from_secs(1);
        }
        terminal.draw(|f| draw(f, snapshot.as_ref(), error, selected, scroll))?;
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('r') => next = Instant::now(),
                KeyCode::PageDown => scroll = scroll.saturating_add(10),
                KeyCode::PageUp => scroll = scroll.saturating_sub(10),
                KeyCode::Left => {
                    selected = selected.saturating_sub(1);
                    scroll = 0;
                }
                KeyCode::Right => {
                    selected = (selected + 1).min(
                        snapshot
                            .as_ref()
                            .map_or(0, |s| s.sessions.len().saturating_sub(1)),
                    );
                    scroll = 0;
                }
                _ => (),
            }
        }
    }
    Ok(())
}
