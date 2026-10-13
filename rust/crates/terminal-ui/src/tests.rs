use super::*;
use crate::{fixture::Scope, model::Update};
use std::sync::mpsc::channel;
use tuirealm::{
    event::{Event, Key, KeyEvent, KeyModifiers},
    terminal::TestEventListener,
};
fn key(code: Key) -> Event<Update> {
    Event::Keyboard(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
    })
}

#[test]
fn waiting_objective_to_decision_to_role_and_simulated_message() {
    let mut m = Model::new();
    command(&mut m, "follow dependency");
    assert_eq!(m.selected_work().id, "review:ui");
    command(&mut m, "follow dependency");
    assert_eq!(m.selected_work().id, "decision:history");
    command(&mut m, "conversation");
    assert_eq!(m.view, View::Conversation);
    assert_eq!(
        m.recipient, "planner:main",
        "inspection must not retarget input"
    );
    m.handle(key(Key::Function(7)));
    assert_eq!(m.recipient, "reviewer:main");
    m.handle(Event::Paste("Keep history inspect-only.\n世界".into()));
    m.handle(key(Key::Function(8)));
    assert!(
        m.entries
            .last()
            .unwrap()
            .text
            .contains("Keep history inspect-only.\n世界")
    );
    assert!(m.notice.contains("No owner acknowledgment"));
    assert_eq!(
        m.selected_work().status,
        "NEEDS YOU",
        "message is not decision acceptance"
    );
}
#[test]
fn per_recipient_drafts_and_historical_scope_preserve_target() {
    let mut m = Model::new();
    m.focus = Pane::Composer;
    m.handle(Event::Paste("planner draft".into()));
    m.select(2);
    m.set_recipient();
    m.handle(Event::Paste("reviewer draft".into()));
    m.set_scope(Scope::Historical);
    let count = m.entries.len();
    m.send();
    m.select(3);
    m.set_recipient();
    assert_eq!(m.recipient, "reviewer:main");
    assert_eq!(m.draft_text(), "reviewer draft");
    assert_eq!(m.entries.len(), count);
    assert!(
        m.visible_entries()
            .iter()
            .all(|e| !e.text.contains("SIMULATED"))
    );
    m.set_scope(Scope::Live);
    m.select(0);
    m.set_recipient();
    assert_eq!(m.draft_text(), "planner draft");
}
#[test]
fn resize_view_detail_and_activity_preserve_draft_selection_and_focus() {
    let (mut app, model) = application(EventListenerCfg::default());
    {
        let mut m = model.borrow_mut();
        m.select(2);
        m.set_recipient();
        m.focus = Pane::Composer;
        m.handle(Event::Paste("Retain this 世界 draft".into()));
    }
    for (w, h) in [(128, 40), (80, 24), (60, 18), (40, 12), (160, 50)] {
        let text = ui::snapshot(&mut app, &model, w, h);
        assert!(text.contains("FIXTURE"));
        let mut m = model.borrow_mut();
        m.handle(key(Key::Function(2)));
        m.handle(key(Key::Function(3)));
        for _ in 0..100 {
            m.handle(Event::User(Update::Pulse));
        }
        assert_eq!(m.selected_work().id, "decision:history");
        assert_eq!(m.recipient, "reviewer:main");
        assert_eq!(m.focus, Pane::Composer);
        assert_eq!(m.draft_text(), "Retain this 世界 draft");
    }
}
#[test]
fn list_graph_and_history_share_subject_but_different_evidence() {
    let (mut app, model) = application(EventListenerCfg::default());
    model.borrow_mut().select(1);
    let graph = ui::snapshot(&mut app, &model, 128, 40);
    assert!(graph.contains("Work graph"));
    assert!(graph.contains("Interaction review"));
    model.borrow_mut().handle(key(Key::Function(6)));
    let list = ui::snapshot(&mut app, &model, 128, 40);
    assert!(list.contains("Work list"));
    assert_eq!(model.borrow().selected, 1);
    let old = model.borrow().selected_work().evidence;
    model.borrow_mut().set_scope(Scope::Historical);
    assert_ne!(model.borrow().selected_work().evidence, old);
    assert_eq!(model.borrow().selected_work().id, "review:ui");
    model.borrow_mut().context_tab = 1;
    assert!(ui::snapshot(&mut app, &model, 128, 40).contains("fixture://branch/approach-a/r11"));
}
#[test]
fn conversation_detail_is_progressive_and_scrolling_is_bounded() {
    let (mut app, model) = application(EventListenerCfg::default());
    model.borrow_mut().view = View::Conversation;
    let summary = ui::snapshot(&mut app, &model, 128, 40);
    assert!(!summary.contains("fixture://review/ui/r7"));
    model.borrow_mut().detail = 2;
    model.borrow_mut().scroll = u16::MAX; // Evidence follows the longer formatted recommendation.
    assert!(ui::snapshot(&mut app, &model, 128, 40).contains("fixture://review/ui/r7"));
    model.borrow_mut().scroll = u16::MAX;
    ui::snapshot(&mut app, &model, 80, 24);
    assert!(model.borrow().scroll < 100);
}
#[test]
fn actual_framework_routes_queued_focus_then_paste_in_order() {
    let (tx, rx) = channel();
    let (mut app, model) = application(EventListenerCfg::default().add_port(
        Box::new(TestEventListener::new(rx)),
        Duration::from_millis(1),
        1,
    ));
    tx.send(key(Key::Tab)).unwrap(); // Tab no longer moves keyboard focus.
    tx.send(key(Key::BackTab)).unwrap();
    tx.send(Event::Paste("queued text".into())).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while model.borrow().draft_text() != "queued text" {
        for msg in app
            .tick(PollStrategy::Once(Duration::from_millis(10)))
            .unwrap()
        {
            apply(&mut app, &model, msg);
        }
        assert!(Instant::now() < deadline);
    }
    assert_eq!(app.focus(), Some(&Pane::Composer));
    // Global shortcuts still work while typing, without leaking into the draft.
    tx.send(key(Key::Function(5))).unwrap();
    while model.borrow().scope != Scope::Historical {
        for msg in app
            .tick(PollStrategy::Once(Duration::from_millis(10)))
            .unwrap()
        {
            apply(&mut app, &model, msg);
        }
        assert!(Instant::now() < deadline);
    }
    assert_eq!(model.borrow().draft_text(), "queued text");
}
#[test]
fn editor_cursor_unicode_and_quit_confirmation_preserve_drafts() {
    let mut m = Model::new();
    m.focus = Pane::Composer;
    m.handle(Event::Paste("a界c".into()));
    m.handle(key(Key::Left));
    m.handle(key(Key::Backspace));
    assert_eq!(m.draft_text(), "ac");
    m.handle(Event::Keyboard(KeyEvent {
        code: Key::Char('c'),
        modifiers: KeyModifiers::CONTROL,
    }));
    assert!(m.confirm_quit);
    assert!(!m.quit);
    m.handle(key(Key::Char('n')));
    assert!(!m.confirm_quit);
    assert_eq!(m.draft_text(), "ac");
    m.handle(key(Key::Esc));
    m.handle(key(Key::Char('q')));
    assert!(!m.confirm_quit);
    assert_eq!(m.draft_text(), "aqc");
    m.handle(ctrl('c'));
    m.handle(key(Key::Char('y')));
    assert!(m.quit);
}

#[test]
fn selected_work_cannot_silently_retarget_a_message() {
    let mut m = Model::new();
    m.select(2);
    m.set_recipient();
    m.focus = Pane::Composer;
    m.handle(Event::Paste("Review this scope".into()));
    m.select(3); // Background/inspection state is not delivery identity.
    m.send();
    assert_eq!(m.entries.last().unwrap().recipient, Some("reviewer:main"));
    assert_eq!(m.entries.last().unwrap().text, "Review this scope");
    assert_eq!(m.selected_work().owner, "builder:main");
}

fn ctrl(code: char) -> Event<Update> {
    Event::Keyboard(KeyEvent {
        code: Key::Char(code),
        modifiers: KeyModifiers::CONTROL,
    })
}

fn mouse_event(kind: tuirealm::event::MouseEventKind, x: u16, y: u16) -> Event<Update> {
    Event::Mouse(tuirealm::event::MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    })
}

#[test]
fn source_selection_preserves_markdown_across_wraps_and_unicode() {
    use crate::mouse::Transcript;
    use tuirealm::ratatui::layout::Rect;
    let markdown =
        "## Heading\n- **bold** [link](https://example.test)\n\n```rust\n\t界 e\u{301} 👩‍💻\n```";
    let mut t = Transcript::default();
    t.area = Rect::new(4, 3, 14, 90);
    t.metadata("role · timestamp · not Markdown".into());
    t.append(markdown, 14);
    let last_y = t.area.y + t.rows.len() as u16 - 1;
    t.down(4, 4);
    t.up(90, last_y);
    assert_eq!(t.selected().as_deref(), Some(markdown));
    t.down(90, last_y);
    t.up(4, 4);
    assert_eq!(t.selected().as_deref(), Some(markdown));
    let mut wide = Transcript::default();
    wide.area = Rect::new(0, 0, 10, 5);
    wide.append("界e\u{301}👩‍💻!", 10);
    // Both cells of a wide glyph refer to the same whole source grapheme.
    assert_eq!(wide.hit(0, 0), wide.hit(1, 0));
    wide.down(1, 0);
    wide.up(4, 0);
    assert_eq!(wide.selected().as_deref(), Some("界e\u{301}👩‍💻"));
}

#[test]
fn mouse_selection_copy_resize_and_modal_isolation() {
    use tuirealm::event::{MouseButton::Left, MouseEventKind::*};
    let (mut app, model) = application(EventListenerCfg::default());
    {
        let mut m = model.borrow_mut();
        m.view = View::Conversation;
        m.entries = vec![presentation::Entry {
            kind: crate::presentation::EntryKind::Message,
            recipient: Some("reviewer:main"),
            speaker: "you",
            subject: "operator:input",
            detail: 0,
            text: "**Markdown**\n- source text".into(),
        }];
    }
    ui::snapshot(&mut app, &model, 128, 40);
    let area = model.borrow().transcript.area;
    {
        let mut m = model.borrow_mut();
        m.handle(mouse_event(Down(Left), area.x, area.y + 4));
        m.handle(mouse_event(Drag(Left), area.x + 40, area.y + 5));
        m.handle(mouse_event(Up(Left), area.x + 40, area.y + 5));
        m.handle(ctrl('c'));
        assert_eq!(
            m.clipboard.take().as_deref(),
            Some("**Markdown**\n- source text")
        );
        assert!(!m.quit);
    }
    ui::snapshot(&mut app, &model, 80, 24);
    assert_eq!(
        model.borrow().transcript.selected().as_deref(),
        Some("**Markdown**\n- source text")
    );
    {
        let mut m = model.borrow_mut();
        m.help = true;
        m.handle(mouse_event(Down(Left), 30, 2));
        assert_eq!(m.view, View::Conversation, "no click through help");
        m.help = false;
        m.handle(key(Key::Esc));
        assert!(m.transcript.selected().is_none());
        m.handle(mouse_event(Down(Left), 1, 2));
        assert_eq!(m.view, View::Dashboard);
    }
    ui::snapshot(&mut app, &model, 80, 24);
    assert!(model.borrow().transcript.selected().is_none());
    let (row, item) = model.borrow().mouse.items[1];
    model
        .borrow_mut()
        .handle(mouse_event(Down(Left), row.x, row.y));
    assert_eq!(model.borrow().selected, item);
    let recipient = model.borrow().recipient;
    assert_eq!(
        recipient, "planner:main",
        "mouse inspection cannot retarget"
    );
}

#[test]
fn clipboard_encodes_source_without_terminal_injection() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let text = "# Markdown\n界\n\x1b]0;not a terminal title\x07";
    let mut output = Vec::new();
    clipboard::request(&mut output, text).unwrap();
    assert!(output.starts_with(b"\x1b]52;c;"));
    assert!(output.ends_with(b"\x1b\\"));
    assert_eq!(
        STANDARD.decode(&output[7..output.len() - 2]).unwrap(),
        text.as_bytes()
    );
    assert_eq!(output.iter().filter(|b| **b == 27).count(), 2);
}

#[test]
fn cross_message_copy_omits_chrome_and_survives_scroll_but_not_source_change() {
    use tuirealm::event::{MouseButton::Left, MouseEventKind::*};
    let (mut app, model) = application(EventListenerCfg::default());
    {
        let mut m = model.borrow_mut();
        m.view = View::Conversation;
        m.entries = ["# First\n**one**", "- second\n```\ncode\n```"]
            .into_iter()
            .map(|text| presentation::Entry {
                kind: crate::presentation::EntryKind::Message,
                recipient: None,
                speaker: "fixture role",
                subject: "fixture subject",
                detail: 0,
                text: text.into(),
            })
            .collect();
    }
    ui::snapshot(&mut app, &model, 80, 24);
    let area = model.borrow().transcript.area;
    {
        let mut m = model.borrow_mut();
        m.handle(mouse_event(Down(Left), area.x, area.y + 4));
        m.handle(mouse_event(ScrollDown, area.x, area.y + 5));
    }
    ui::snapshot(&mut app, &model, 80, 24);
    {
        let mut m = model.borrow_mut();
        m.handle(mouse_event(Drag(Left), area.right() - 1, area.bottom() - 1));
        m.handle(mouse_event(Up(Left), area.right() - 1, area.bottom() - 1));
        assert_eq!(
            m.transcript.selected().as_deref(),
            Some("# First\n**one**\n\n- second\n```\ncode\n```")
        );
        m.handle(mouse_event(ScrollUp, area.x, area.y));
    }
    ui::snapshot(&mut app, &model, 128, 40);
    assert!(model.borrow().transcript.selected().is_some());
    model.borrow_mut().entries[0].text = "A replaced source".into();
    ui::snapshot(&mut app, &model, 128, 40);
    assert!(
        model.borrow().transcript.selected().is_none(),
        "never reuse byte offsets against changed source"
    );
}

#[test]
fn stationary_edge_drag_selects_offscreen_history_in_both_directions() {
    use tuirealm::event::{MouseButton::Left, MouseEventKind::*};
    let (mut app, model) = application(EventListenerCfg::default());
    let source = (0..80)
        .map(|n| format!("- **row {n}**"))
        .collect::<Vec<_>>()
        .join("\n");
    {
        let mut m = model.borrow_mut();
        m.view = View::Conversation;
        m.entries = vec![presentation::Entry {
            kind: crate::presentation::EntryKind::Message,
            recipient: None,
            speaker: "fixture",
            subject: "fixture",
            detail: 0,
            text: source.clone(),
        }];
    }
    ui::snapshot(&mut app, &model, 80, 24);
    let area = model.borrow().transcript.area;
    {
        let mut m = model.borrow_mut();
        m.handle(mouse_event(Down(Left), area.x, area.y + 4));
        m.handle(mouse_event(Drag(Left), area.right() - 1, area.bottom() - 1));
    }
    // Only timer ticks, no more mouse movement: stationary edge must keep going.
    for _ in 0..100 {
        model.borrow_mut().drag_scroll_tick();
        ui::snapshot(&mut app, &model, 80, 24);
    }
    {
        let mut m = model.borrow_mut();
        assert_eq!(m.transcript.selected().as_deref(), Some(source.as_str()));
        let bottom = m.scroll;
        m.handle(mouse_event(Up(Left), area.right() - 1, area.bottom() - 1));
        m.drag_scroll_tick();
        assert_eq!(m.scroll, bottom, "release stops scrolling");
        m.handle(mouse_event(Down(Left), area.right() - 1, area.bottom() - 1));
        m.handle(mouse_event(Drag(Left), area.x, area.y));
    }
    for _ in 0..100 {
        model.borrow_mut().drag_scroll_tick();
        ui::snapshot(&mut app, &model, 80, 24);
    }
    let mut m = model.borrow_mut();
    assert_eq!(m.scroll, 0);
    assert_eq!(m.transcript.selected().as_deref(), Some(source.as_str()));
    m.handle(mouse_event(Drag(Left), area.x, area.bottom() - 1));
    m.help = true;
    m.drag_scroll_tick();
    m.help = false;
    m.drag_scroll_tick();
    assert_eq!(m.scroll, 0, "opening a modal cancels held dragging");
    m.handle(mouse_event(Down(Left), area.x, area.y + 4));
    m.handle(mouse_event(Drag(Left), area.x, area.bottom() - 1));
    m.handle(mouse_event(Drag(Left), area.x, area.y + 5));
    m.drag_scroll_tick();
    assert_eq!(m.scroll, 0, "returning inside stops scrolling");
    m.handle(mouse_event(Drag(Left), area.right() + 3, area.bottom()));
    m.drag_scroll_tick();
    assert_eq!(m.scroll, 0, "side panel is not an edge-scroll target");
}

#[test]
fn markdown_and_code_colors_preserve_source_and_selection() {
    use crate::theme::*;
    use tuirealm::ratatui::style::Modifier;
    let source = "# Heading\n**strong** *emphasis* `code` [link](fixture://x) &amp; \\*literal\\*\n\n```rust\nfn greet() { let text = \"界\"; }\n```";
    let entry = presentation::Entry {
        kind: crate::presentation::EntryKind::Message,
        text: source.into(),
        recipient: None,
        speaker: "fixture",
        subject: "fixture",
        detail: 0,
    };
    let mut t = mouse::Transcript::default();
    t.prepare(&[&entry], 80);
    let style_at = |needle: &str| {
        let offset = source.find(needle).unwrap();
        t.rows
            .iter()
            .flat_map(|r| &r.glyphs)
            .find(|g| g.source.contains(&offset))
            .unwrap()
            .style
    };
    assert_eq!(style_at("Heading").fg, Some(ACCENT));
    assert!(style_at("strong").add_modifier.contains(Modifier::BOLD));
    assert!(style_at("emphasis").add_modifier.contains(Modifier::ITALIC));
    assert_eq!(style_at("code`").fg, Some(AMBER));
    assert_eq!(style_at("link").fg, Some(BLUE));
    assert_eq!(style_at("fn greet").fg, Some(LAVENDER));
    assert_eq!(style_at("界").fg, Some(ACCENT));
    assert_eq!(style_at("fn greet").bg, Some(CODE_BG));
    t.selection = Some(0..source.len());
    assert_eq!(
        t.selected().as_deref(),
        Some(source),
        "color cannot normalize entities, escapes, or Markdown"
    );
}

#[test]
fn syntax_detection_is_conservative_and_incomplete_fences_are_readable() {
    for language in ["rust", "js", "json", "python", "bash", "diff"] {
        assert!(
            highlight::language(language, "").is_some(),
            "missing bundled syntax {language}"
        );
    }
    assert!(highlight::language("", "#!/usr/bin/env python").is_some());
    assert!(highlight::language("unknown-language", "#!/usr/bin/env python").is_none());
    assert!(highlight::language("", "some ambiguous prose").is_none());
    let source = "```rust\nlet text = \"unfinished\n界";
    let paints = highlight::markdown(source);
    assert!(paints.iter().all(|p| source.get(p.range.clone()).is_some()));
    assert!(paints.iter().any(|p| p.style.fg == Some(theme::ACCENT)));
    let plain = "```unknown-language\nfn looks_like_rust() {}\n```";
    let offset = plain.find("fn looks").unwrap();
    let style = highlight::markdown(plain)
        .into_iter()
        .filter(|p| p.range.contains(&offset))
        .fold(tuirealm::ratatui::style::Style::default(), |s, p| {
            s.patch(p.style)
        });
    assert_eq!(
        style.fg,
        Some(theme::INK),
        "do not invent a language for an unknown explicit hint"
    );
}

#[test]
fn redraw_and_resize_reuse_highlighting_without_losing_selection() {
    let (mut app, model) = application(EventListenerCfg::default());
    model.borrow_mut().view = View::Conversation;
    ui::snapshot(&mut app, &model, 128, 40);
    model.borrow_mut().transcript.selection = Some(0..3);
    for _ in 0..8 {
        model.borrow_mut().pulse();
        ui::snapshot(&mut app, &model, 128, 40);
    }
    assert_eq!(model.borrow().transcript.layout_builds, 1);
    assert_eq!(model.borrow().transcript.highlight_builds, 1);
    ui::snapshot(&mut app, &model, 80, 24);
    assert_eq!(model.borrow().transcript.layout_builds, 2);
    assert_eq!(model.borrow().transcript.highlight_builds, 1);
    assert_eq!(model.borrow().transcript.selected().as_deref(), Some("The"));
    model.borrow_mut().entries[0].text.push_str(" New content.");
    ui::snapshot(&mut app, &model, 80, 24);
    assert_eq!(model.borrow().transcript.highlight_builds, 2);
    assert!(model.borrow().transcript.selection.is_none());
}

#[test]
fn lifecycle_inspection_cannot_send_retarget_or_mutate_retained_drafts() {
    let mut m = Model::new();
    m.focus = Pane::Composer;
    m.handle(Event::Paste("retained draft".into()));
    m.select(2); // Inspect a different role before entering lifecycle.
    m.view = View::Lifecycle;
    let recipient = m.recipient;
    let count = m.entries.len();
    for event in [
        key(Key::Enter),
        key(Key::Function(8)),
        key(Key::Function(7)),
        key(Key::Char('x')),
        Event::Paste("must not enter hidden editor".into()),
    ] {
        m.handle(event);
    }
    assert_eq!(m.draft_text(), "retained draft");
    assert_eq!(m.recipient, recipient);
    assert_eq!(m.entries.len(), count);
    assert_eq!(m.view, View::Lifecycle);
    m.open_palette();
    m.palette.as_mut().unwrap().query = "to reviewer:main".into();
    m.handle(key(Key::Enter));
    assert!(m.palette.is_some(), "unavailable action stays in picker");
    assert_eq!(m.recipient, recipient);
    m.handle(key(Key::Esc));
    m.focus = Pane::Main;
    m.handle(key(Key::Char('1')));
    assert_eq!(m.draft_text(), "retained draft");
}

#[test]
fn lifecycle_report_and_failure_scenarios_keep_provenance_and_outcomes_distinct() {
    let (mut app, model) = application(EventListenerCfg::default());
    command(&mut model.borrow_mut(), "lifecycle");
    let report = ui::snapshot(&mut app, &model, 128, 40);
    assert!(report.contains("Reported sequence"));
    assert!(report.contains("parent validation pending"));
    assert!(report.contains("NO LIVE CONNECTION"));
    assert!(report.contains("Input 2 handled once"));
    let original_work = model.borrow().selected;
    command(&mut model.borrow_mut(), "inspect step 2");
    assert_eq!(model.borrow().lifecycle_selected, 1);
    assert_eq!(model.borrow().selected, original_work);
    for case in 1..lifecycle::CASES.len() {
        command(
            &mut model.borrow_mut(),
            &format!("scenario {}", lifecycle::CASES[case]),
        );
        assert_eq!(model.borrow().lifecycle_case, case);
        let screen = ui::snapshot(&mut app, &model, 128, 40);
        assert!(screen.contains("DESIGN FIXTURE"));
        assert!(!screen.contains("ACCEPTED · TEXT"));
        assert!(screen.contains(lifecycle::steps(case)[0].outcome));
        model.borrow_mut().show_context = true;
        let compact = ui::snapshot(&mut app, &model, 80, 24);
        assert!(compact.contains("NATIVE OBSERVATION"));
        assert!(compact.contains("Messaging unavailable"));
        model.borrow_mut().show_context = false;
    }
}

#[test]
fn usage_is_context_bound_and_never_invents_a_measurement() {
    use chat::{ContextUsage, Measurement};
    let mut usage = ContextUsage::default();
    assert!(usage.label().contains("unavailable"));
    usage.replace("A");
    usage.measurement = Some(Measurement {
        context: "A",
        used: 68_000,
        capacity: 100_000,
    });
    assert_eq!(usage.label(), "Context A · 68% used");
    usage.replace("B");
    assert!(usage.label().contains("B · usage unavailable"));
    usage.measurement = Some(Measurement {
        context: "A",
        used: 68,
        capacity: 100,
    });
    assert!(
        usage.label().contains("unavailable"),
        "late old-context sample cannot become B usage"
    );
    for (used, capacity) in [(1, 0), (101, 100)] {
        usage.measurement = Some(Measurement {
            context: "B",
            used,
            capacity,
        });
        assert!(usage.label().contains("unavailable"));
    }
    usage.measurement = Some(Measurement {
        context: "B",
        used: u64::MAX,
        capacity: u64::MAX,
    });
    assert!(usage.label().contains("100% used"));
}

#[test]
fn chat_system_events_are_typed_expandable_and_copy_original_text() {
    use presentation::EntryKind;
    let entries = chat::handoff_example();
    assert!(matches!(entries[0].kind, EntryKind::Message));
    assert!(matches!(entries[1].kind, EntryKind::System { .. }));
    assert!(matches!(entries[3].kind, EntryKind::Message));
    let refs = entries.iter().collect::<Vec<_>>();
    let mut t = mouse::Transcript::default();
    t.prepare_detail(&refs, 80, false);
    assert!(
        t.rows
            .iter()
            .filter_map(|r| r.metadata.as_ref())
            .any(|s| s.contains("SYSTEM · UNCERTAIN"))
    );
    assert!(!t.source.contains("ILLUSTRATIVE ALTERNATE ENDING"));
    t.selection = Some(0..t.source.len());
    assert!(t.selected().unwrap().contains("let next_context = \"B\";"));
    t.prepare_detail(&refs, 80, true);
    assert!(t.source.contains("ILLUSTRATIVE ALTERNATE ENDING"));
    assert!(
        t.selection.is_none(),
        "expanded source invalidates old offsets"
    );
    assert!(
        t.rows
            .iter()
            .flat_map(|r| &r.glyphs)
            .any(|g| g.style.fg == Some(theme::AMBER))
    );
}

#[test]
fn chat_example_preserves_work_drafts_and_never_sends_hidden_input() {
    let (mut app, model) = application(EventListenerCfg::default());
    {
        let mut m = model.borrow_mut();
        m.draft().insert_str("keep my work draft");
        m.open_chat_example();
        m.focus = Pane::Composer;
        m.handle(Event::Paste("hidden".into()));
        m.handle(key(Key::Enter));
        m.handle(key(Key::Function(8)));
        assert_eq!(m.draft_text(), "keep my work draft");
        assert_eq!(m.entries.len(), 5);
    }
    let screen = ui::snapshot(&mut app, &model, 128, 40);
    assert!(screen.contains("Context B · usage unavailable"));
    assert!(screen.contains("ILLUSTRATIVE CHAT"));
    assert!(screen.contains("SYSTEM"));
    {
        let mut m = model.borrow_mut();
        m.open_palette();
        m.palette.as_mut().unwrap().query = "conversation".into();
        m.handle(key(Key::Enter));
        assert!(!m.chat_example);
        assert_eq!(m.draft_text(), "keep my work draft");
        assert!(m.context_usage.active.is_none());
    }
}

#[test]
fn command_completion_preview_execution_and_draft_preservation() {
    let mut m = Model::new();
    m.focus = Pane::Composer;
    m.handle(Event::Paste("My unfinished response".into()));
    m.handle(ctrl('p'));
    m.handle(Event::Paste("to rev".into()));
    assert_eq!(
        m.palette.as_ref().unwrap().matches()[0].name,
        "to reviewer:main"
    );
    m.handle(key(Key::Tab));
    assert_eq!(m.palette.as_ref().unwrap().query, "to reviewer:main");
    assert_eq!(m.recipient, "planner:main", "completion must not execute");
    m.handle(key(Key::Enter));
    assert!(m.palette.is_none());
    assert_eq!(m.recipient, "reviewer:main");
    assert!(m.draft_text().is_empty());
    m.target_role("planner:main");
    assert_eq!(m.draft_text(), "My unfinished response");
    assert_eq!(m.entries.len(), 5, "commands send nothing");
}
#[test]
fn palette_owns_input_and_historical_targeting_stays_unavailable() {
    let mut m = Model::new();
    m.focus = Pane::Composer;
    m.handle(Event::Paste("keep".into()));
    m.set_scope(Scope::Historical);
    m.handle(ctrl('p'));
    m.handle(Event::Paste("to reviewer".into()));
    m.handle(key(Key::Enter));
    assert!(m.palette.is_some());
    assert!(m.notice.contains("Unavailable"));
    m.handle(key(Key::Function(8)));
    assert_eq!(m.draft_text(), "keep");
    assert_eq!(m.entries.len(), 5);
    m.handle(key(Key::Esc));
    assert!(m.palette.is_none());
    assert_eq!(m.focus, Pane::Composer);
    m.handle(ctrl('p'));
    m.handle(Event::Paste("scope live".into()));
    m.handle(key(Key::Enter));
    assert_eq!(m.scope, Scope::Live);
    assert_eq!(m.draft_text(), "keep");
}
#[test]
fn slashes_and_old_shortcuts_are_literal_and_unknown_queries_are_safe() {
    let mut m = Model::new();
    let text = "/qdi1234?[]+-fws";
    for c in text.chars() {
        m.handle(key(Key::Char(c)));
    }
    assert_eq!(m.draft_text(), text);
    assert_eq!(m.view, View::Dashboard);
    assert!(!m.quit && !m.help && m.palette.is_none());
    m.handle(ctrl('p'));
    m.handle(Event::Paste("no_such_command".into()));
    m.handle(key(Key::Enter));
    assert!(m.palette.as_ref().unwrap().matches().is_empty());
    m.handle(key(Key::Esc));
    assert_eq!(m.draft_text(), text);
    assert_eq!(m.focus, Pane::Composer);
}

#[test]
fn command_palette_remains_above_composer_at_small_sizes() {
    let (mut app, model) = application(EventListenerCfg::default());
    {
        let mut m = model.borrow_mut();
        m.focus = Pane::Composer;
        m.handle(Event::Paste("Draft survives palette resize".into()));
        m.open_palette();
        m.handle(Event::Paste("evidence".into()));
    }
    for (w, h) in [(128, 40), (80, 24), (60, 18), (40, 12), (160, 50)] {
        let frame = ui::snapshot(&mut app, &model, w, h);
        if w >= 60 {
            assert!(frame.contains("Commands"));
            assert!(frame.contains("Draft survives palette resize"));
            assert!(frame.contains("> evidence"));
        }
        assert_eq!(model.borrow().draft_text(), "Draft survives palette resize");
    }
}

#[test]
fn chat_enter_sends_shift_and_alt_enter_insert_newlines() {
    let mut m = Model::new();
    m.focus = Pane::Composer;
    m.handle(Event::Paste("one".into()));
    for modifier in [
        crossterm::event::KeyModifiers::SHIFT,
        crossterm::event::KeyModifiers::ALT,
    ] {
        m.handle(
            crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Enter,
                modifier,
            ))
            .into(),
        );
        m.handle(key(Key::Char('x')));
    }
    assert_eq!(m.draft_text(), "one\nx\nx");
    assert_eq!(m.entries.len(), 5);
    m.handle(key(Key::Enter));
    assert_eq!(m.entries.len(), 6);
    assert!(m.draft_text().is_empty());
    assert!(m.entries.last().unwrap().text.ends_with("one\nx\nx"));
    m.handle(key(Key::Enter));
    assert_eq!(m.entries.len(), 6, "blank Enter cannot send");
}
#[test]
fn multiline_paste_is_literal_normalized_and_undoable_without_sending() {
    let mut m = Model::new();
    m.focus = Pane::Composer;
    m.handle(key(Key::Char('x')));
    m.handle(crossterm::event::Event::Paste("/scope live\r\nsecond\rthird\n\n".into()).into());
    assert_eq!(m.draft_text(), "x/scope live\nsecond\nthird\n\n");
    assert!(m.palette.is_none());
    assert_eq!(m.entries.len(), 5);
    m.handle(ctrl('z'));
    assert_eq!(m.draft_text(), "x");
    m.handle(ctrl('y'));
    assert_eq!(m.draft_text(), "x/scope live\nsecond\nthird\n\n");
    m.set_scope(Scope::Historical);
    m.handle(key(Key::Enter));
    assert_eq!(m.entries.len(), 5);
    assert!(m.draft_text().contains("second"));
}

#[test]
fn folded_pastes_send_full_payloads_in_order_and_delete_atomically() {
    let mut m = Model::new();
    m.focus = Pane::Composer;
    let first = "one\ntwo\nthree\nfour\nfive\nsix";
    let second = "界".repeat(1100);
    m.handle(Event::Paste("Before ".into()));
    m.handle(Event::Paste(first.into()));
    m.handle(Event::Paste(" between ".into()));
    m.handle(Event::Paste(second.clone()));
    assert_eq!(m.drafts[m.recipient].folded_count(), 2);
    assert!(m.drafts[m.recipient].display_lines()[0].contains("Paste #2"));
    m.handle(key(Key::Backspace));
    assert_eq!(m.drafts[m.recipient].folded_count(), 1);
    assert_eq!(m.draft_text(), format!("Before {first} between "));
    m.handle(ctrl('z'));
    assert_eq!(m.drafts[m.recipient].folded_count(), 2);
    m.handle(ctrl('y'));
    assert_eq!(m.drafts[m.recipient].folded_count(), 1);
    m.handle(ctrl('z'));
    m.handle(Event::Paste(" after".into()));
    let expected = format!("Before {first} between {second} after");
    assert_eq!(m.draft_text(), expected);
    m.handle(key(Key::Enter));
    assert!(m.entries.last().unwrap().text.ends_with(&expected));
    assert_eq!(m.drafts[m.recipient].folded_count(), 0);
}
#[test]
fn folded_paste_inspection_is_read_only_and_role_drafts_keep_payloads() {
    let (mut app, model) = application(EventListenerCfg::default());
    let text = (0..30)
        .map(|i| format!("unique line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    {
        let mut m = model.borrow_mut();
        m.focus = Pane::Composer;
        m.handle(Event::Paste(text.clone()));
        m.handle(ctrl('o'));
    }
    assert!(ui::snapshot(&mut app, &model, 80, 24).contains("Full draft"));
    {
        let mut m = model.borrow_mut();
        assert_eq!(m.paste_preview.as_ref().unwrap().text, text);
        m.handle(Event::Paste("must not insert".into()));
        m.handle(key(Key::Enter));
        assert_eq!(m.entries.len(), 5);
        assert_eq!(m.draft_text(), text);
        m.handle(key(Key::End));
    }
    assert!(ui::snapshot(&mut app, &model, 80, 24).contains("unique line 29"));
    {
        let mut m = model.borrow_mut();
        m.handle(key(Key::Esc));
        m.target_role("reviewer:main");
        assert!(m.draft_text().is_empty());
        m.target_role("planner:main");
        assert_eq!(m.draft_text(), text);
        m.set_scope(Scope::Historical);
        m.handle(key(Key::Enter));
        assert_eq!(m.entries.len(), 5);
    }
    assert!(ui::snapshot(&mut app, &model, 80, 24).contains("Messaging unavailable"));
}
#[test]
fn labels_and_private_unicode_cannot_alias_stored_paste_objects() {
    let mut m = Model::new();
    m.focus = Pane::Composer;
    let text = "z".repeat(1000);
    m.handle(Event::Paste(text.clone()));
    let label = m.drafts[m.recipient].display_lines()[0].clone();
    m.handle(Event::Paste(label.clone()));
    m.handle(Event::Paste("\u{f0000}".into()));
    m.handle(key(Key::Char('\u{f0000}')));
    assert_eq!(m.draft_text(), format!("{text}{label}\u{f0000}\u{f0000}"));
    m.handle(key(Key::Enter));
    assert!(
        m.entries
            .last()
            .unwrap()
            .text
            .ends_with(&format!("{text}{label}\u{f0000}\u{f0000}"))
    );
}
#[test]
fn composer_grows_soft_wraps_caps_and_shrinks_without_losing_draft() {
    let (mut app, model) = application(EventListenerCfg::default());
    {
        let mut m = model.borrow_mut();
        m.focus = Pane::Composer;
        assert_eq!(m.drafts[m.recipient].height(80, 40), 5);
        m.handle(Event::Paste("abcd ".repeat(100))); // Below fold threshold; long visible text.
        assert!(m.drafts[m.recipient].height(80, 40) > 5);
    }
    let frame = ui::snapshot(&mut app, &model, 80, 24);
    assert!(
        frame.lines().filter(|l| l.contains("abcd")).count() > 1,
        "long draft should wrap"
    );
    {
        let mut m = model.borrow_mut();
        m.handle(ctrl('z'));
        assert_eq!(m.drafts[m.recipient].height(80, 40), 5);
        // Typed newlines grow normally; they are not paste folding triggers.
        for _ in 0..40 {
            m.draft().insert_str("line\n");
        }
        assert_eq!(m.drafts[m.recipient].height(80, 40), 12);
        assert_eq!(m.drafts[m.recipient].height(80, 24), 8);
        assert_eq!(m.drafts[m.recipient].height(60, 18), 6);
    }
    for (w, h) in [(128, 40), (80, 24), (60, 18), (40, 12)] {
        ui::snapshot(&mut app, &model, w, h);
    }
    assert_eq!(model.borrow().draft_text().matches("line").count(), 40);
}
#[test]
fn folded_token_selection_and_wrapped_cursor_navigation_are_consistent() {
    let (mut app, model) = application(EventListenerCfg::default());
    {
        let mut m = model.borrow_mut();
        m.focus = Pane::Composer;
        m.handle(Event::Paste("long ".repeat(90)));
    }
    ui::snapshot(&mut app, &model, 80, 24);
    let cursor = model.borrow().drafts["planner:main"].cursor();
    model.borrow_mut().handle(key(Key::Up));
    assert!(
        model.borrow().drafts["planner:main"].cursor().1 < cursor.1,
        "Up should move a visual row"
    );
    {
        let mut m = model.borrow_mut();
        m.handle(key(Key::End));
        // Put the chip on its own row so its selected label fits the viewport.
        m.handle(Event::Keyboard(KeyEvent {
            code: Key::Enter,
            modifiers: KeyModifiers::SHIFT,
        }));
        m.handle(Event::Paste("z".repeat(1000)));
        m.handle(Event::Keyboard(KeyEvent {
            code: Key::Left,
            modifiers: KeyModifiers::SHIFT,
        }));
    }
    let frame = ui::snapshot(&mut app, &model, 80, 24);
    assert!(frame.contains("Paste #1"), "{frame}");
    model.borrow_mut().handle(key(Key::Delete));
    assert_eq!(model.borrow().drafts["planner:main"].folded_count(), 0);
    model.borrow_mut().handle(ctrl('z'));
    assert_eq!(model.borrow().drafts["planner:main"].folded_count(), 1);
}

#[test]
fn capped_composer_can_scroll_without_moving_the_insertion_point() {
    let (mut app, model) = application(EventListenerCfg::default());
    {
        let mut m = model.borrow_mut();
        m.focus = Pane::Composer;
        for n in 0..30 {
            m.draft().insert_str(format!("unique draft row {n}\n"));
        }
    }
    let bottom = ui::snapshot(&mut app, &model, 80, 24);
    let cursor = model.borrow().drafts["planner:main"].cursor();
    model.borrow_mut().handle(key(Key::PageUp));
    let scrolled = ui::snapshot(&mut app, &model, 80, 24);
    assert_ne!(bottom, scrolled);
    assert_eq!(model.borrow().drafts["planner:main"].cursor(), cursor);
    assert_eq!(
        scrolled,
        ui::snapshot(&mut app, &model, 80, 24),
        "redraw must retain scroll"
    );
}

#[test]
fn historical_archive_keeps_outputs_outcomes_and_attempts_separate() {
    use crate::replay::{ARCHIVE, Replay, Section};
    use presentation::{EntryKind, SystemState};
    let live = Replay::load(ARCHIVE, Section::Live).unwrap();
    let outputs = live
        .entries
        .iter()
        .filter(|e| e.speaker.starts_with("agent"))
        .map(|e| e.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        outputs,
        vec![
            "FACT:ORION-7",
            "READY:9f6c1c4e109f7269e72acf6dd30de98c1aded619b3c74c2544a764eb2e784465:FACT:ORION-7",
            "SECOND:ORION-7"
        ]
    );
    assert_eq!(
        live.entries
            .iter()
            .filter(|e| matches!(
                e.kind,
                EntryKind::System {
                    state: SystemState::Accepted,
                    ..
                }
            ))
            .count(),
        3
    );
    assert!(
        live.entries
            .iter()
            .find(|e| e.speaker.starts_with("input"))
            .is_some_and(|e| !e.text.contains("ORION-7"))
    );
    let rejected = Replay::load(ARCHIVE, Section::Rejected).unwrap();
    assert_eq!(rejected.entries.len(), 1);
    assert!(matches!(
        rejected.entries[0].kind,
        EntryKind::System {
            state: SystemState::Rejected,
            ..
        }
    ));
    assert!(rejected.entries[0].text.contains("0 provider turns"));
    let validation = Replay::load(ARCHIVE, Section::Validation).unwrap();
    assert_eq!(validation.entries.len(), 1);
    assert!(
        validation.entries[0]
            .text
            .contains("30 default tests · 3 controlled probes")
    );
    assert!(
        !live
            .entries
            .iter()
            .any(|e| e.text.contains("default tests") || e.text.contains("Admission rejected"))
    );
}

#[test]
fn historical_archive_rejects_unsupported_or_unattributed_records() {
    use crate::replay::{ARCHIVE, Replay, Section};
    let base: serde_json::Value = serde_json::from_str(ARCHIVE).unwrap();
    for (pointer, value) in [
        ("/version", serde_json::json!(2)),
        ("/live/0/kind", serde_json::json!("success")),
        ("/live/0/context", serde_json::json!("unknown")),
        ("/live/0/refs", serde_json::json!([])),
        ("/live/0/refs", serde_json::json!(["missing#/output"])),
        ("/sources/result/sha256", serde_json::json!("not-a-hash")),
    ] {
        let mut data = base.clone();
        *data.pointer_mut(pointer).unwrap() = value;
        assert!(
            Replay::load(&data.to_string(), Section::Live).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn historical_output_copy_and_expanded_provenance_preserve_source() {
    use crate::replay::{ARCHIVE, Replay, Section};
    let replay = Replay::load(ARCHIVE, Section::Live).unwrap();
    let mut transcript = mouse::Transcript::default();
    let refs = vec![&replay.entries[0]];
    transcript.prepare_detail(&refs, 40, false);
    transcript.selection = Some(0..transcript.source.len());
    assert_eq!(transcript.selected().as_deref(), Some("FACT:ORION-7"));
    transcript.prepare_detail(&refs, 40, true);
    assert!(transcript.source.contains("result.json#/first/output"));
    assert!(transcript.source.contains("SHA-256 ea64b318"));
    assert!(transcript.selection.is_none());
}

#[test]
fn historical_replay_commands_preserve_drafts_and_disable_actions() {
    let (mut app, model) = application(EventListenerCfg::default());
    {
        let mut m = model.borrow_mut();
        m.draft().insert_str("retain my draft");
        m.open_palette();
        m.palette.as_mut().unwrap().query = "replay live".into();
        m.handle(key(Key::Enter));
        assert!(m.replay.is_some());
        m.focus = Pane::Composer;
        m.handle(Event::Paste("hidden".into()));
        m.handle(key(Key::Enter));
        m.handle(key(Key::Function(8)));
        m.target_role("reviewer:main");
        m.set_scope(Scope::Historical);
        assert_eq!(m.scope, Scope::Live);
        assert_eq!(m.recipient, "planner:main");
        assert_eq!(m.draft_text(), "retain my draft");
        assert_eq!(m.entries.len(), 5);
    }
    let screen = ui::snapshot(&mut app, &model, 128, 40);
    assert!(screen.contains("HISTORICAL • OFFLINE REPLAY"));
    assert!(screen.contains("historical usage unavailable"));
    assert!(screen.contains("FACT:ORION-7"));
    assert!(!screen.contains("SIMULATED"));
    for (w, h) in [(80, 24), (60, 18), (40, 12)] {
        assert!(ui::snapshot(&mut app, &model, w, h).contains("HISTORICAL"));
    }
    let mut m = model.borrow_mut();
    m.open_palette();
    m.palette.as_mut().unwrap().query = "conversation".into();
    m.handle(key(Key::Enter));
    assert!(m.replay.is_none());
    assert!(!m.input_read_only());
    assert_eq!(m.draft_text(), "retain my draft");
}

fn command(m: &mut Model, query: &str) {
    m.handle(ctrl('p'));
    m.handle(Event::Paste(query.into()));
    m.handle(key(Key::Enter));
    assert!(m.palette.is_none(), "command did not execute: {query}");
}

#[test]
fn palette_inspection_preserves_cursor_recipient_and_keyboard_target() {
    use tuirealm::event::{MouseButton, MouseEventKind};
    let (mut app, model) = application(EventListenerCfg::default());
    {
        let mut m = model.borrow_mut();
        m.handle(Event::Paste("ab".into()));
        m.handle(key(Key::Left));
        command(&mut m, "inspect decision:history");
        assert_eq!(m.recipient, "planner:main");
        assert!(m.show_context);
        assert_eq!(m.focus, Pane::Composer);
    }
    assert!(ui::snapshot(&mut app, &model, 80, 24).contains("History action scope"));
    let commands = model.borrow().mouse.commands;
    model.borrow_mut().handle(mouse_event(
        MouseEventKind::Down(MouseButton::Left),
        commands.x,
        commands.y,
    ));
    assert!(model.borrow().palette.is_some());
    model.borrow_mut().handle(key(Key::Esc));
    model.borrow_mut().handle(key(Key::Char('/')));
    assert_eq!(model.borrow().draft_text(), "a/b");
    command(&mut model.borrow_mut(), "panel main");
    assert!(ui::snapshot(&mut app, &model, 80, 24).contains("Work list"));
    let item = model.borrow().mouse.items.last().unwrap().0;
    model.borrow_mut().handle(mouse_event(
        MouseEventKind::Down(MouseButton::Left),
        item.x,
        item.y,
    ));
    model.borrow_mut().handle(key(Key::Tab));
    model.borrow_mut().handle(key(Key::Char('q')));
    assert_eq!(model.borrow().draft_text(), "a/qb");
    assert_eq!(model.borrow().recipient, "planner:main");
    assert_eq!(model.borrow().focus, Pane::Composer);
}

#[test]
fn palette_reaches_readonly_events_details_and_scroll_without_messaging() {
    let (mut app, model) = application(EventListenerCfg::default());
    {
        let mut m = model.borrow_mut();
        m.handle(Event::Paste("retained".into()));
        command(&mut m, "replay live");
        m.handle(key(Key::Char('/')));
        m.handle(Event::Paste("blocked".into()));
        m.handle(key(Key::Enter));
        assert_eq!(m.draft_text(), "retained");
        command(&mut m, "event 4");
    }
    ui::snapshot(&mut app, &model, 128, 40);
    assert!(model.borrow().scroll > 0);
    model.borrow_mut().handle(key(Key::Function(3)));
    ui::snapshot(&mut app, &model, 128, 40);
    assert_eq!(model.borrow().inspected_event, Some(3));
    assert_eq!(
        model.borrow().scroll as usize,
        model.borrow().transcript.entry_rows[3]
    );
    command(&mut model.borrow_mut(), "detail evidence");
    command(&mut model.borrow_mut(), "scroll timeline end");
    assert!(ui::snapshot(&mut app, &model, 128, 40).contains("SHA-256"));
    command(&mut model.borrow_mut(), "panel context");
    assert!(ui::snapshot(&mut app, &model, 80, 24).contains("Historical evidence"));
    command(&mut model.borrow_mut(), "scroll context end");
    ui::snapshot(&mut app, &model, 80, 24);
    assert!(model.borrow().context_scroll > 0);
    command(&mut model.borrow_mut(), "scroll context start");
    assert_eq!(model.borrow().context_scroll, 0);
    assert_eq!(model.borrow().recipient, "planner:main");
    assert_eq!(model.borrow().draft_text(), "retained");
}

#[test]
fn every_conversation_source_feeds_the_same_palette_and_transcript_rows() {
    use crate::presentation::SourceId;
    let (mut app, model) = application(EventListenerCfg::default());
    for source in [
        "conversation",
        "chat demo",
        "replay live",
        "replay rejected",
        "replay validation",
        "contract failed unsettled",
        "contract historical complete current blocked",
        "contract observation unavailable",
        "contract unsupported variant",
    ] {
        command(&mut model.borrow_mut(), source);
        for detail in [0, 2] {
            model.borrow_mut().detail = detail;
            ui::snapshot(&mut app, &model, 128, 40);
            let mut m = model.borrow_mut();
            let presentation = m.conversation();
            let expected = presentation
                .visible(detail)
                .map(|(_, e)| e.display_text(detail > 0).into_owned())
                .collect::<Vec<_>>()
                .join("\n\n");
            assert_eq!(m.transcript.source, expected, "source {source}");
            m.open_palette();
            let indices = m
                .palette
                .as_ref()
                .unwrap()
                .commands
                .iter()
                .filter_map(|command| {
                    if let commands::Action::Event(index) = command.action {
                        Some(index)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(
                indices,
                presentation
                    .visible(detail)
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>()
            );
            m.handle(key(Key::Esc));
            match presentation.source {
                SourceId::WorkFixture(_) => assert!(presentation.simulated_activity),
                SourceId::ChatExample => {
                    assert!(presentation.banner.contains("not captured output"));
                    assert!(!presentation.simulated_activity);
                }
                SourceId::Archive(_) => {
                    assert!(presentation.banner.contains("NOT a running engine"));
                    assert_eq!(presentation.usage, "historical usage unavailable");
                    assert!(!presentation.simulated_activity);
                    let (_, context) = presentation.context.as_ref().unwrap();
                    assert!(context.contains("Archive recorded:"));
                    assert!(context.contains("Logical sequence, not timestamped playback"));
                }
                SourceId::Contract(_) => {
                    assert!(presentation.banner.contains("CONTRACT EXAMPLE"));
                    assert!(!presentation.simulated_activity);
                }
                SourceId::Task(_, _) => panic!("task fixture has separate projection tests"),
                SourceId::Client(_) => {
                    panic!("client observations are opened through the passive client path")
                }
            }
        }
    }
    command(&mut model.borrow_mut(), "conversation");
    command(&mut model.borrow_mut(), "scope historical");
    let m = model.borrow();
    assert_eq!(
        m.conversation().source,
        SourceId::WorkFixture(Scope::Historical)
    );
    assert!(!m.conversation().simulated_activity);
    assert_eq!(m.visible_entries(), fixture::timeline(Scope::Historical));
}

#[test]
fn dataset_identity_clears_anchors_even_when_text_matches_but_redraw_does_not() {
    let (mut app, model) = application(EventListenerCfg::default());
    command(&mut model.borrow_mut(), "chat demo");
    ui::snapshot(&mut app, &model, 128, 40);
    let builds = model.borrow().transcript.highlight_builds;
    {
        let mut m = model.borrow_mut();
        m.transcript.selection = Some(0..4);
        m.inspected_event = Some(1);
    }
    ui::snapshot(&mut app, &model, 100, 30);
    assert_eq!(model.borrow().transcript.selection, Some(0..4));
    assert_eq!(model.borrow().transcript.highlight_builds, builds);
    assert_eq!(model.borrow().inspected_event, Some(1));
    {
        let mut m = model.borrow_mut();
        // Deliberately identical data from a different local source: equality of
        // rendered text cannot establish the same observation identity.
        m.entries = m.conversation().entries;
        m.chat_example = false;
        m.pending_event = Some(1);
        m.draft().insert_str("retained");
        let source = m.conversation().source;
        m.synchronize_conversation(source);
        assert!(m.transcript.selection.is_none());
        assert!(m.inspected_event.is_none());
        assert!(m.pending_event.is_none());
        assert_eq!(m.recipient, "planner:main");
        assert_eq!(m.draft_text(), "retained");
    }
    ui::snapshot(&mut app, &model, 100, 30);
    assert_eq!(model.borrow().transcript.highlight_builds, builds);
}

#[test]
fn graph_clicks_inspect_rendered_nodes_without_retargeting_or_moving_viewport() {
    use tuirealm::event::{MouseButton::Left, MouseEventKind::Down};
    let (mut app, model) = application(EventListenerCfg::default());
    model.borrow_mut().draft().insert_str("keep draft");
    for (w, h) in [(128, 40), (160, 50), (128, 40)] {
        ui::snapshot(&mut app, &model, w, h);
        let hits = model.borrow().mouse.items.clone();
        assert_eq!(hits.len(), 5);
        for (rect, index) in hits {
            let mut m = model.borrow_mut();
            let bounds = m.flow.node_terminal_rect(m.items()[index].id);
            let cursor = m.drafts[m.recipient].cursor();
            m.handle(mouse_event(
                Down(Left),
                rect.x + rect.width / 2,
                rect.y + rect.height / 2,
            ));
            assert_eq!(m.selected, index);
            assert_eq!(
                m.flow.first_selected_node_id().as_deref(),
                Some(m.items()[index].id)
            );
            assert_eq!(m.flow.node_terminal_rect(m.items()[index].id), bounds);
            assert_eq!(m.recipient, "planner:main");
            assert_eq!(m.focus, Pane::Composer);
            assert_eq!(m.draft_text(), "keep draft");
            assert_eq!(m.drafts[m.recipient].cursor(), cursor);
        }
    }
    // Post-render hit areas follow zoom/pan and cannot escape into other panels.
    {
        let mut m = model.borrow_mut();
        m.flow.zoom_in();
        m.flow.pan(-12.0, -4.0);
    }
    ui::snapshot(&mut app, &model, 128, 40);
    let hits = model.borrow().mouse.items.clone();
    assert!(!hits.is_empty());
    for (rect, index) in hits {
        let mut m = model.borrow_mut();
        assert!(m.flow.is_in_bounds(i32::from(rect.x), i32::from(rect.y)));
        assert!(
            m.flow
                .is_in_bounds(i32::from(rect.right() - 1), i32::from(rect.bottom() - 1))
        );
        m.handle(mouse_event(Down(Left), rect.x, rect.y));
        assert_eq!(m.selected, index);
    }
    // Modal isolation and offscreen-node removal prevent stale click targets.
    let (rect, _) = model.borrow().mouse.items[0];
    {
        let mut m = model.borrow_mut();
        let selected = m.selected;
        m.open_palette();
        m.handle(mouse_event(Down(Left), rect.x, rect.y));
        assert_eq!(m.selected, selected);
        m.handle(key(Key::Esc));
        m.flow.pan(-10000.0, -10000.0);
    }
    ui::snapshot(&mut app, &model, 128, 40);
    assert!(model.borrow().mouse.items.is_empty());
    // Compact mode repopulates list hit areas, not stale graph rectangles.
    ui::snapshot(&mut app, &model, 80, 24);
    assert!(!model.borrow().mouse.items.is_empty());
}

#[test]
fn s3_published_contract_examples_keep_outcome_settlement_and_admission_separate() {
    let (mut app, model) = application(EventListenerCfg::default());
    {
        let mut m = model.borrow_mut();
        m.handle(Event::Paste("retained draft".into()));
        command(&mut m, "contract failed unsettled");
        assert!(m.input_read_only());
        let rows = m.conversation();
        let detail = &rows.context.as_ref().unwrap().1;
        for exact in [
            "subject-1",
            "context-a",
            "store-1",
            "stream subject-1",
            "commit 9",
            "delivery-1",
            "effect-1",
            "attempt-1",
            "failed",
            "unresolved",
            "evidence:failure-1",
            "fenced",
            "unsettled_effect",
            "unavailable",
        ] {
            assert!(
                detail.contains(exact)
                    || rows
                        .entries
                        .iter()
                        .any(|e| e.display_text(true).contains(exact)),
                "missing {exact}"
            );
        }
        assert!(!rows.usage.contains('%'));
        m.send();
        assert_eq!(m.draft_text(), "retained draft");
        assert_eq!(m.recipient, "planner:main");
    }
    let screen = ui::snapshot(&mut app, &model, 128, 40);
    assert!(screen.contains("S3 CONTRACT EXAMPLE"));
    command(
        &mut model.borrow_mut(),
        "contract historical complete current blocked",
    );
    let text = model.borrow().conversation().context.unwrap().1;
    assert!(text.contains("transition-older"));
    assert!(text.contains("completed true"));
    assert!(text.contains("transition-current"));
    assert!(text.contains("blocked by current transition"));
    command(&mut model.borrow_mut(), "contract observation unavailable");
    let unavailable = model.borrow().conversation();
    assert!(unavailable.entries[0].text.contains("Observation error"));
    assert!(!unavailable.entries[0].text.contains("completed"));
    command(&mut model.borrow_mut(), "contract unsupported variant");
    let unsupported = model.borrow().conversation();
    assert!(
        unsupported.entries[0]
            .text
            .contains("unsupported observation variant")
    );
    assert!(!unsupported.entries[0].text.contains("completed"));
}

#[test]
fn passive_client_gap_deadline_disconnect_and_reconnect_project_truthfully() {
    use lifecycle_client::{ClientError, ObservationTransport, ReadClient, SubjectRef, WaitHintV1};
    use lifecycle_wire::{
        CursorV1, ExecutionOutcomeV1, FieldV1, LifecycleSnapshotV1, WaitResultV1, WaitTargetV1,
    };
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::time::Duration;

    struct MemoryPort {
        reads: Cell<usize>,
        snapshots: RefCell<VecDeque<LifecycleSnapshotV1>>,
        hint: WaitHintV1,
    }
    impl ObservationTransport for MemoryPort {
        fn snapshot(
            &self,
            _: &SubjectRef,
            target: Option<&WaitTargetV1>,
            budget: Duration,
        ) -> Result<LifecycleSnapshotV1, ClientError> {
            assert!(!budget.is_zero());
            if self.hint == WaitHintV1::CursorGap {
                assert!(target.is_some());
            }
            self.reads.set(self.reads.get() + 1);
            self.snapshots
                .borrow_mut()
                .pop_front()
                .ok_or(ClientError::ObservationUnavailable)
        }
        fn wait_hint(
            &self,
            _: &SubjectRef,
            _: &WaitTargetV1,
            _: &CursorV1,
            budget: Duration,
        ) -> Result<WaitHintV1, ClientError> {
            assert!(!budget.is_zero());
            Ok(self.hint)
        }
    }
    let failed = lifecycle_wire::parse_snapshot(
        include_bytes!("../../lifecycle-wire/tests/fixtures/failed-unsettled-v1.json"),
        "subject-1",
    )
    .unwrap();
    let mut pending = failed.clone();
    pending.cursor.commit_sequence = "5".into();
    if let FieldV1::Known(delivery) = &mut pending.delivery {
        delivery.outcome = ExecutionOutcomeV1::Pending;
    }
    let subject = SubjectRef::parse("subject-1").unwrap();
    let target = WaitTargetV1::DeliveryOutcome {
        delivery_id: "delivery-1".into(),
        outcome: ExecutionOutcomeV1::Failed,
    };
    let initial = ReadClient::new(MemoryPort {
        reads: Cell::new(0),
        snapshots: RefCell::new(VecDeque::from([pending.clone()])),
        hint: WaitHintV1::NoChange,
    })
    .snapshot(&subject, Duration::from_secs(1));
    let mut model = Model::new();
    model.handle(Event::Paste("planner draft".into()));
    model.show_client_snapshot(&subject, initial);
    assert!(
        model
            .conversation()
            .context
            .unwrap()
            .1
            .contains("outcome pending")
    );
    let port = MemoryPort {
        reads: Cell::new(0),
        snapshots: RefCell::new(VecDeque::from([pending.clone(), failed.clone()])),
        hint: WaitHintV1::CursorGap,
    };
    let result = ReadClient::new(port).wait(&subject, &target, Duration::from_secs(1));
    assert!(matches!(result, WaitResultV1::Observed(_)));
    model.show_client_wait(&subject, result);
    let text = model.conversation().context.unwrap().1;
    assert!(text.contains("outcome failed"));
    assert!(text.contains("settlement unresolved"));
    assert!(model.input_read_only());
    model.send();
    assert_eq!(model.draft_text(), "planner draft");

    let no_change = MemoryPort {
        reads: Cell::new(0),
        snapshots: RefCell::new(VecDeque::from([pending.clone()])),
        hint: WaitHintV1::NoChange,
    };
    model.show_client_wait(
        &subject,
        ReadClient::new(no_change).wait(&subject, &target, Duration::from_secs(1)),
    );
    let deadline = model.conversation();
    assert!(deadline.banner.contains("deadline expired"));
    assert!(deadline.banner.contains("target unobserved"));
    assert!(deadline.context.unwrap().1.contains("outcome pending"));
    model.show_client_snapshot(&subject, Err(ClientError::ObservationUnavailable));
    assert!(model.conversation().banner.contains("last known only"));
    assert!(!model.conversation().banner.contains("disconnected"));
    model.show_client_snapshot(&subject, Err(ClientError::ObservationUnavailable));
    assert!(model.conversation().banner.contains("last known only"));
    model.show_client_snapshot(&subject, Err(ClientError::DeadlineExpired));
    assert!(model.conversation().banner.contains("deadline expired"));
    assert!(!model.conversation().banner.contains("disconnected"));
    let other_subject = SubjectRef::parse("subject-2").unwrap();
    model.show_client_snapshot(&other_subject, Err(ClientError::ObservationUnavailable));
    assert!(
        model.conversation().entries[0]
            .text
            .contains("Observation error")
    );
    assert!(!model.conversation().entries[0].text.contains("subject-1"));
    model.show_client_snapshot(&other_subject, Ok(pending.clone()));
    assert!(
        model.conversation().entries[0]
            .text
            .contains("semantically invalid")
    );

    let wrong_stream = MemoryPort {
        reads: Cell::new(0),
        snapshots: RefCell::new(VecDeque::from([LifecycleSnapshotV1 {
            cursor: CursorV1 {
                stream_id: "other-stream".into(),
                ..failed.cursor.clone()
            },
            ..failed.clone()
        }])),
        hint: WaitHintV1::NoChange,
    };
    model.show_client_reconnect(
        &subject,
        ReadClient::new(wrong_stream).reconnect(&subject, &pending.cursor, Duration::from_secs(1)),
    );
    assert!(
        model.conversation().entries[0]
            .text
            .contains("observation belongs to another subject or store")
    );
    assert!(
        !model.conversation().entries[0]
            .text
            .contains("outcome failed")
    );
    let regressed = MemoryPort {
        reads: Cell::new(0),
        snapshots: RefCell::new(VecDeque::from([LifecycleSnapshotV1 {
            cursor: CursorV1 {
                commit_sequence: "4".into(),
                ..failed.cursor.clone()
            },
            ..failed
        }])),
        hint: WaitHintV1::NoChange,
    };
    model.show_client_reconnect(
        &subject,
        ReadClient::new(regressed).reconnect(&subject, &pending.cursor, Duration::from_secs(1)),
    );
    assert!(
        model.conversation().entries[0]
            .text
            .contains("semantically invalid")
    );
    model.show_client_wait(
        &subject,
        WaitResultV1::ObservationUnavailable {
            code: lifecycle_wire::WireErrorCode::UnsupportedObservationVariant,
        },
    );
    assert!(
        model.conversation().entries[0]
            .text
            .contains("unsupported observation variant")
    );
}

#[test]
fn connected_read_preserves_draft_and_rejects_stale_identity_and_revision() {
    use crate::connection::{ReadKind, Reply};
    use lifecycle_client::{ClientError, SubjectRef};
    let subject = SubjectRef::parse("subject-1").unwrap();
    let socket = std::path::PathBuf::from("/tmp/work-engine-ui-test.sock");
    let snapshot = lifecycle_wire::parse_snapshot(
        include_bytes!("../../lifecycle-wire/tests/fixtures/failed-unsettled-v1.json"),
        subject.as_str(),
    )
    .unwrap();
    let mut model = Model::new();
    model.handle(Event::Paste("retained draft".into()));
    model.connect(socket.clone(), subject.clone());
    assert!(model.input_read_only());
    let initial = model.take_connection_request().unwrap();
    assert_eq!(initial.kind, ReadKind::Initial);
    model.apply_connection_reply(Reply {
        generation: initial.generation,
        kind: ReadKind::Initial,
        result: Ok(snapshot.clone()),
    });
    assert!(model.conversation().banner.contains("coherent read"));
    model.scroll = 5;
    model.transcript.selection = Some(0..5);
    model.request_connection_read(ReadKind::Refresh);
    let refresh = model.take_connection_request().unwrap();
    assert_eq!(refresh.previous, Some(snapshot.cursor.clone()));
    model.apply_connection_reply(Reply {
        generation: refresh.generation,
        kind: ReadKind::Refresh,
        result: Ok(snapshot.clone()),
    });
    assert_eq!(
        model.scroll, 5,
        "an unchanged observation keeps the inspection viewport"
    );
    assert_eq!(model.transcript.selection, Some(0..5));
    model.send();
    assert_eq!(model.draft_text(), "retained draft");

    model.request_connection_read(ReadKind::Reconnect);
    let reconnect = model.take_connection_request().unwrap();
    let mut wrong_stream = snapshot.clone();
    wrong_stream.cursor.stream_id = "other-stream".into();
    model.apply_connection_reply(Reply {
        generation: reconnect.generation,
        kind: ReadKind::Reconnect,
        result: Ok(wrong_stream),
    });
    assert!(model.conversation().banner.contains("last known only"));
    assert_eq!(
        model.connection.as_ref().unwrap().last_known,
        Some(snapshot.clone())
    );

    model.request_connection_read(ReadKind::Reconnect);
    let reconnect = model.take_connection_request().unwrap();
    let mut rollback = snapshot.clone();
    rollback.cursor.commit_sequence = "0".into();
    model.apply_connection_reply(Reply {
        generation: reconnect.generation,
        kind: ReadKind::Reconnect,
        result: Ok(rollback),
    });
    assert!(model.conversation().banner.contains("semantically invalid"));
    assert_eq!(
        model.connection.as_ref().unwrap().last_known,
        Some(snapshot.clone())
    );

    model.request_connection_read(ReadKind::Refresh);
    let stale = model.take_connection_request().unwrap();
    model.disconnect_connection();
    assert!(
        model
            .conversation()
            .banner
            .contains("explicitly disconnected")
    );
    model.apply_connection_reply(Reply {
        generation: stale.generation,
        kind: ReadKind::Refresh,
        result: Err(ClientError::ObservationUnavailable),
    });
    assert!(
        model
            .conversation()
            .banner
            .contains("explicitly disconnected")
    );
    model.request_connection_read(ReadKind::Reconnect);
    let retry = model.take_connection_request().unwrap();
    assert!(retry.generation > stale.generation);
    model.apply_connection_reply(Reply {
        generation: retry.generation,
        kind: ReadKind::Reconnect,
        result: Err(ClientError::DeadlineExpired),
    });
    assert!(model.conversation().banner.contains("last known only"));
    assert!(
        !model
            .conversation()
            .banner
            .contains("explicitly disconnected")
    );

    model.request_connection_read(ReadKind::Refresh);
    let abandoned = model.take_connection_request().unwrap();
    command(&mut model, "dashboard");
    assert_eq!(model.view, View::Dashboard);
    assert!(model.client_observation.is_none());
    model.apply_connection_reply(Reply {
        generation: abandoned.generation,
        kind: ReadKind::Refresh,
        result: Ok(snapshot.clone()),
    });
    assert_eq!(
        model.view,
        View::Dashboard,
        "a late read cannot steal navigation"
    );
    assert!(model.client_observation.is_none());
    model.request_connection_read(ReadKind::Reconnect);
    let returned = model.take_connection_request().unwrap();
    assert_eq!(model.view, View::Conversation);
    assert_eq!(returned.previous, Some(snapshot.cursor.clone()));
    assert!(model.input_read_only());
    model.apply_connection_reply(Reply {
        generation: returned.generation,
        kind: ReadKind::Reconnect,
        result: Ok(snapshot.clone()),
    });

    model.request_connection_read(ReadKind::Refresh);
    let abandoned = model.take_connection_request().unwrap();
    model.open_replay(crate::replay::Section::Live);
    assert!(model.replay.is_some());
    model.apply_connection_reply(Reply {
        generation: abandoned.generation,
        kind: ReadKind::Refresh,
        result: Ok(snapshot.clone()),
    });
    assert!(
        model.replay.is_some(),
        "late read cannot replace the archive"
    );
    assert!(model.client_observation.is_none());

    model.connect(socket, SubjectRef::parse("subject-2").unwrap());
    let new_subject = model.take_connection_request().unwrap();
    assert!(new_subject.previous.is_none());
    model.apply_connection_reply(Reply {
        generation: retry.generation,
        kind: ReadKind::Reconnect,
        result: Ok(snapshot),
    });
    assert!(model.conversation().banner.contains("pending"));
    assert!(!model.conversation().entries[0].text.contains("subject-1"));
    model.apply_connection_reply(Reply {
        generation: new_subject.generation,
        kind: ReadKind::Initial,
        result: Err(ClientError::ObservationUnavailable),
    });
    assert!(!model.conversation().entries[0].text.contains("subject-1"));
    model.connect(
        std::path::PathBuf::from("/tmp/another-ui-test.sock"),
        subject,
    );
    let other_endpoint = model.take_connection_request().unwrap();
    assert!(
        other_endpoint.previous.is_none(),
        "last known state is scoped to the endpoint too"
    );
    assert!(other_endpoint.generation > new_subject.generation);
}

#[test]
fn bounded_worker_keeps_ui_responsive_when_socket_never_replies() {
    use crate::connection::{ReadKind, Request, Worker};
    use lifecycle_client::SubjectRef;
    use std::{io::BufRead, os::unix::net::UnixListener, time::Instant};
    let root = std::env::temp_dir().join(format!("we-ui-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let socket = root.join("read.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut line = String::new();
        std::io::BufReader::new(stream)
            .read_line(&mut line)
            .unwrap();
        let request: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["op"], "snapshot");
        assert_eq!(request["subject_id"], "subject-1");
        std::thread::sleep(std::time::Duration::from_millis(800));
    });
    let worker = Worker::start();
    let started = Instant::now();
    worker
        .request(Request {
            generation: 1,
            socket,
            subject: SubjectRef::parse("subject-1").unwrap(),
            previous: None,
            kind: ReadKind::Initial,
        })
        .unwrap();
    assert!(started.elapsed() < std::time::Duration::from_millis(100));
    let deadline = Instant::now() + std::time::Duration::from_secs(2);
    let reply = loop {
        if let Some(reply) = worker.try_reply() {
            break reply;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    assert!(reply.result.is_err());
    assert!(started.elapsed() < std::time::Duration::from_millis(750));
    drop(worker);
    server.join().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn typed_contract_projection_preserves_conflict_and_qualified_unknown_fields() {
    use lifecycle_wire::{AdmissionV1, EffectSettlementV1, FieldV1, RuntimeAvailabilityV1};
    let mut snapshot = lifecycle_wire::parse_snapshot(
        include_bytes!("../../lifecycle-wire/tests/fixtures/failed-unsettled-v1.json"),
        "subject-1",
    )
    .unwrap();
    snapshot.context_generation = "context-b".into();
    snapshot.current_admission = AdmissionV1::Unknown;
    snapshot.transition = FieldV1::Unknown;
    snapshot.blocking_reason = FieldV1::NotApplicable;
    snapshot.runtime_availability = RuntimeAvailabilityV1::Unknown;
    snapshot.measurement = FieldV1::Known("42".into());
    if let FieldV1::Known(delivery) = &mut snapshot.delivery {
        delivery.settlement = EffectSettlementV1::Conflict {
            evidence_ref: "evidence:conflict-2".into(),
        };
    }
    let projected = contract::snapshot_conversation(
        &snapshot,
        presentation::SourceId::Client(17),
        "controlled typed test",
    );
    let detail = projected.context.unwrap().1;
    for exact in [
        "context-b",
        "Current admission: unknown",
        "Blocking reason: not applicable",
        "Historical/target transition: unknown",
        "settlement conflict / evidence:conflict-2",
        "Runtime availability: unknown",
        "Measurement (raw, unqualified; no percentage): 42",
    ] {
        assert!(detail.contains(exact), "missing {exact}");
    }
    assert!(!projected.usage.contains('%'));
}

#[test]
fn task_publications_are_grouped_and_component_freshness_is_independent() {
    use crate::task::{Availability, Case, Task};
    let pending = Task::fixture(Case::Pending);
    assert_eq!(pending.components[6].availability, Availability::Absent);
    assert!(pending.components[1].detail.contains("running"));
    assert!(pending.nodes[2].text.unwrap().contains("Pending q1"));
    let result = Task::fixture(Case::Result);
    assert_eq!(result.components[6].availability, Availability::Present);
    assert!(result.components[1].detail.contains("unsettled"));
    let stale = Task::fixture(Case::Stale);
    assert_eq!(stale.components[0].availability, Availability::Unavailable);
    assert!(stale.components[6].stale);
    assert!(!stale.components[1].stale);
    let conversation = stale.conversation();
    assert_eq!(conversation.entries.len(), 3);
    assert_eq!(
        conversation
            .entries
            .iter()
            .map(|e| e.subject)
            .collect::<Vec<_>>(),
        vec!["input:i1", "question:q1", "result:r1"]
    );
    assert!(
        conversation
            .entries
            .iter()
            .all(|e| e.text.starts_with("STALE / LAST KNOWN"))
    );
    assert!(
        conversation.entries[2]
            .text
            .contains("Lifecycle delivery remains unsettled")
    );
    assert!(!conversation.simulated_activity);
    assert!(
        conversation
            .banner
            .contains("NOT a chronological transcript")
    );
    assert!(
        conversation.entries[2]
            .display_text(true)
            .contains("fixture://result/r1")
    );
    let refused = Task::fixture(Case::Refused);
    assert_eq!(refused.components[2].availability, Availability::Refused);
    assert!(!refused.nodes.iter().any(|n| n.id.starts_with("segment:")));
    assert!(refused.components[2].detail.contains("capacity"));
    assert!(
        refused
            .details()
            .contains("Explicit fixture reason: capacity")
    );
    assert_eq!(
        refused.components[3].availability,
        Availability::NotRequested
    );
    assert_eq!(
        refused.components[4].availability,
        Availability::NotApplicable
    );
    assert_eq!(refused.components[5].availability, Availability::Unknown);
    for node in &pending.nodes {
        if let Some(parent) = node.parent {
            assert!(pending.nodes.iter().any(|n| n.id == parent));
        }
    }
}

#[test]
fn task_mouse_scope_double_click_and_read_only_drafts_survive_navigation() {
    use tuirealm::event::{MouseButton::Left, MouseEventKind::Down};
    let (mut app, model) = application(EventListenerCfg::default());
    model.borrow_mut().draft().insert_str("retained draft");
    command(&mut model.borrow_mut(), "task fixture pending");
    assert_eq!(model.borrow().view, View::Overview);
    command(&mut model.borrow_mut(), "graph");
    ui::snapshot(&mut app, &model, 128, 40);
    let rect = model
        .borrow()
        .mouse
        .items
        .iter()
        .find(|(_, i)| *i == 2)
        .unwrap()
        .0;
    model
        .borrow_mut()
        .handle(mouse_event(Down(Left), rect.x, rect.y));
    assert_eq!(model.borrow().task.as_ref().unwrap().selected, 2);
    assert_eq!(model.borrow().view, View::Dashboard);
    model
        .borrow_mut()
        .handle(mouse_event(Down(Left), rect.x, rect.y));
    assert_eq!(model.borrow().view, View::Conversation);
    assert_eq!(model.borrow().visible_entries().len(), 1);
    assert_eq!(model.borrow().visible_entries()[0].subject, "question:q1");
    let frame = ui::snapshot(&mut app, &model, 80, 30);
    assert!(frame.contains("Inspecting: Question q1 · task:ui-17"));
    assert!(frame.contains("Pending q1"));
    let source_len = model.borrow().transcript.source.len();
    model.borrow_mut().transcript.selection = Some(0..source_len);
    model.borrow_mut().handle(Event::Keyboard(KeyEvent {
        code: Key::Char('c'),
        modifiers: KeyModifiers::CONTROL,
    }));
    assert!(
        model
            .borrow()
            .clipboard
            .as_ref()
            .unwrap()
            .starts_with("## Questions")
    );
    model
        .borrow_mut()
        .handle(Event::Paste("must not edit".into()));
    model.borrow_mut().send();
    model.borrow_mut().set_recipient();
    command(&mut model.borrow_mut(), "overview");
    command(&mut model.borrow_mut(), "inspect segment:n1");
    command(&mut model.borrow_mut(), "conversation");
    assert!(
        model.borrow().visible_entries()[0]
            .text
            .contains("No attributable conversation text")
    );
    assert_eq!(model.borrow().recipient, "planner:main");
    assert_eq!(model.borrow().draft_text(), "retained draft");
    command(&mut model.borrow_mut(), "work fixture");
    assert!(model.borrow().task.is_none());
    assert_eq!(model.borrow().draft_text(), "retained draft");
}

#[test]
fn task_overview_rendering_keeps_failure_and_result_separate_at_both_sizes() {
    let (mut app, model) = application(EventListenerCfg::default());
    for (w, h) in [(128, 40), (80, 30)] {
        model.borrow_mut().open_task(task::Case::Stale);
        let frame = ui::snapshot(&mut app, &model, w, h);
        assert!(frame.contains("PROVISIONAL TASK FIXTURE"));
        assert!(frame.contains("Owner read unavailable"));
        assert!(frame.contains("Last-known publication · STALE"));
        assert!(frame.contains("Delivery unsettled"));
        assert!(frame.find("OBJECTIVE").unwrap() < frame.find("NEEDS ATTENTION").unwrap());
        assert!(frame.find("NEEDS ATTENTION").unwrap() < frame.find("LATEST RESULT").unwrap());
        assert!(frame.find("LATEST RESULT").unwrap() < frame.find("EXECUTION").unwrap());
        if w == 80 {
            assert!(!frame.contains("owner revision"));
            assert!(!frame.contains("not_requested"));
        }
    }
}

#[test]
fn task_visual_graph_edges_selection_and_responsive_fallback_share_identity() {
    use tuirealm::event::{MouseButton::Left, MouseEventKind::Down};
    let (mut app, model) = application(EventListenerCfg::default());
    model.borrow_mut().open_task(task::Case::Result);
    command(&mut model.borrow_mut(), "graph");
    let frame = ui::snapshot(&mut app, &model, 128, 40);
    assert!(frame.contains('│') && frame.contains('─') && frame.contains('›'));
    assert!(!frame.contains("Compact graph"));
    let hits = model.borrow().mouse.items.clone();
    assert_eq!(
        hits.len(),
        model.borrow().task.as_ref().unwrap().nodes.len()
    );
    for (rect, index) in &hits {
        assert_eq!(rect.height, 3);
        assert!(
            !hits.iter().any(|(other, other_index)| other_index != index
                && !rect.intersection(*other).is_empty())
        );
        model
            .borrow_mut()
            .handle(mouse_event(Down(Left), rect.x + 1, rect.y + 1));
        assert_eq!(model.borrow().task.as_ref().unwrap().selected, *index);
        assert_eq!(model.borrow().recipient, "planner:main");
    }
    let inspected = model.borrow().task.as_ref().unwrap().selected;
    let narrow = ui::snapshot(&mut app, &model, 80, 30);
    assert!(narrow.contains("Compact graph"));
    assert_eq!(model.borrow().task.as_ref().unwrap().selected, inspected);
    let rect = model
        .borrow()
        .mouse
        .items
        .iter()
        .find(|(_, i)| *i == 2)
        .unwrap()
        .0;
    model
        .borrow_mut()
        .handle(mouse_event(Down(Left), rect.x, rect.y));
    ui::snapshot(&mut app, &model, 80, 30);
    let tab = model
        .borrow()
        .mouse
        .tabs
        .iter()
        .find(|(_, view)| *view == View::Conversation)
        .unwrap()
        .0;
    model
        .borrow_mut()
        .handle(mouse_event(Down(Left), tab.x, tab.y));
    assert_eq!(model.borrow().visible_entries().len(), 1);
    assert_eq!(model.borrow().visible_entries()[0].subject, "question:q1");
    assert_eq!(model.borrow().recipient, "planner:main");
}

#[test]
fn task_overview_empty_result_refusal_and_current_execution_remain_distinct() {
    let (mut app, model) = application(EventListenerCfg::default());
    for case in [task::Case::Pending, task::Case::Result, task::Case::Refused] {
        model.borrow_mut().open_task(case);
        let frame = ui::snapshot(&mut app, &model, 80, 30);
        assert!(frame.contains("Inspecting: Inspect task evidence · task:ui-17"));
        assert!(frame.contains("OBJECTIVE") && frame.contains("NEEDS ATTENTION"));
        if case == task::Case::Result {
            assert!(frame.contains("Published by the task owner"));
            assert!(frame.contains("Delivery unsettled"));
            assert!(frame.contains("No pending questions"));
        } else {
            assert!(frame.contains("Pending question"));
            assert!(frame.contains("No result published"));
            assert!(frame.contains("Delivery active"));
        }
        if case == task::Case::Refused {
            assert!(frame.contains("Native history refused: capacity"));
        }
    }
}
