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
    #[test]
    #[ignore = "requires current Rust U2_VECTOR_DIR and U2_RENDER_OUTPUT"]
    fn repair_vectors_render_and_inspect() {
        use std::{fs, path::PathBuf};
        let dir = PathBuf::from(std::env::var_os("U2_VECTOR_DIR").unwrap());
        let out = PathBuf::from(std::env::var_os("U2_RENDER_OUTPUT").unwrap());
        fs::create_dir_all(&out).unwrap();
        let fragments: Vec<String> =
            serde_json::from_slice(&fs::read(dir.join("lexical-fragments.json")).unwrap()).unwrap();
        let payload: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join("consistency-vectors.json")).unwrap())
                .unwrap();
        let mut records = vec![(
            "lexical".to_owned(),
            fs::read(dir.join("vector-lexical.json")).unwrap(),
        )];
        for case in payload["contract"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["accept"] == true)
        {
            let name = case["name"].as_str().unwrap();
            records.push((
                name.into(),
                serde_json::to_vec(&payload["vectors"][name]).unwrap(),
            ));
        }
        for field in ["owner_revision","cursor_revision"] {
            for n in [9007199254740993u64,9007199254740994,u64::MAX] {
                records.push((format!("r3-{field}-{n}"),fs::read(dir.join(format!("r2-{field}_{n}_fresh.json"))).unwrap()));
            }
        }
        for (name, bytes) in records {
            let s = SelectedTask::parse(&bytes).unwrap();
            for width in [80, 180] {
                let mut terminal =
                    Terminal::new(ratatui::backend::TestBackend::new(width, 60)).unwrap();
                let mut rendered = String::new();
                for route in 0usize..9 {
                    for scroll in (0..320).step_by(40) {
                        terminal
                            .draw(|f| {
                                draw(
                                    f,
                                    Some(&s),
                                    None,
                                    route != 0,
                                    route.saturating_sub(1),
                                    scroll,
                                    (&s.endpoint, &s.request.task_id),
                                )
                            })
                            .unwrap();
                        rendered.push_str(
                            &terminal
                                .backend()
                                .buffer()
                                .content
                                .chunks(width as usize)
                                .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
                                .collect::<Vec<_>>()
                                .join("\n"),
                        );
                        rendered.push('\n');
                    }
                }
                if name.starts_with("r3-") {
                    let digits=name.rsplit('-').next().unwrap();
                    assert!(rendered.contains(digits),"exact revision digits {name}");
                }
                for fragment in &fragments {
                    assert!(!rendered.contains(fragment), "{name}: {fragment}");
                }
                if name == "lexical" {
                    for cue in [
                        "[REDACTED]",
                        "excerpt clipped",
                        "question:u2",
                        "question:pending",
                        "Published result",
                        "keep-row",
                        "safe-row",
                    ] {
                        assert!(rendered.contains(cue), "missing {cue} at {width}");
                    }
                }
                if s.summary.basis == "last_known_owner" {
                    assert!(rendered.contains("last_known_scope"));
                    assert!(rendered.contains("1000"));
                }
                fs::write(out.join(format!("repair-{name}-{width}.txt")), rendered).unwrap();
            }
        }
    }
    #[test]
    fn narrow_and_wide_render_and_inspection() {
        let bytes =
            include_bytes!("../../operator-observation/tests/fixtures/selected-task-v2.json");
        let s = SelectedTask::parse(bytes).unwrap();
        for (width, height) in [(80, 24), (80, 60), (180, 60)] {
            let backend = ratatui::backend::TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|f| {
                    draw(
                        f,
                        Some(&s),
                        None,
                        false,
                        0,
                        0,
                        (&s.endpoint, &s.request.task_id),
                    )
                })
                .unwrap();
            let text = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            if let Ok(dir) = std::env::var("U2_RENDER_OUTPUT") {
                std::fs::create_dir_all(&dir).unwrap();
                let lines = terminal
                    .backend()
                    .buffer()
                    .content
                    .chunks(width as usize)
                    .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
                    .collect::<Vec<_>>()
                    .join("\n");
                std::fs::write(format!("{dir}/overview-{width}x{height}.txt"), lines).unwrap();
            }
            assert!(text.contains("Objective:"));
            assert!(text.contains("Yes, publish"));
            assert!(text.contains("Published result:"));
            assert!(!text.contains("segment_state"));
            for i in 0..8 {
                terminal
                    .draw(|f| {
                        draw(
                            f,
                            Some(&s),
                            None,
                            true,
                            i,
                            0,
                            (&s.endpoint, &s.request.task_id),
                        )
                    })
                    .unwrap();
                let text = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|c| c.symbol())
                    .collect::<String>();
                assert!(text.contains("Detail"));
            }
        }
    }
}
