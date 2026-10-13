//! Standalone UI with an explicit passive lifecycle read connection.
mod chat;
mod clipboard;
mod commands;
mod connection;
mod contract;
mod draft;
mod fixture;
mod highlight;
mod lifecycle;
mod model;
mod mouse;
mod presentation;
mod replay;
#[cfg(test)]
mod tests;
mod theme;
mod ui;
use crossterm::{
    cursor::Show,
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use model::{Model, Msg, Pane, View};
use std::path::PathBuf;
use std::{
    cell::RefCell,
    io::{self, IsTerminal},
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tuirealm::{
    application::PollStrategy,
    listener::EventListenerCfg,
    ratatui::{Terminal, backend::CrosstermBackend},
};
use ui::{App, Panel, Shared};
static KEYBOARD_ENHANCEMENT_PUSHED: AtomicBool = AtomicBool::new(false);
fn application(config: EventListenerCfg<model::Update>) -> (App, Shared) {
    let model = Rc::new(RefCell::new(Model::new()));
    let mut app = App::init(config);
    for id in [Pane::Main, Pane::Context, Pane::Composer] {
        app.mount(
            id,
            Box::new(Panel {
                id,
                model: Rc::clone(&model),
            }),
            vec![],
        )
        .unwrap();
    }
    app.active(&Pane::Composer).unwrap();
    (app, model)
}
fn apply(app: &mut App, model: &Shared, msg: Msg) {
    let Msg::Input(_, event) = msg;
    model.borrow_mut().handle(event);
    app.active(&model.borrow().focus).unwrap();
}
fn restore() {
    // Panic hook and guard may both restore; pop only the mode we pushed, once.
    if KEYBOARD_ENHANCEMENT_PUSHED.swap(false, Ordering::SeqCst) {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    let _ = execute!(
        io::stdout(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen,
        Show
    );
    let _ = disable_raw_mode();
}
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--help" || a == "-h") {
        println!(
            "Work Engine terminal UI — fixture and explicit passive observation\n\ncargo run --manifest-path rust/crates/terminal-ui/Cargo.toml --locked -- --connect /path/to/socket --subject SUBJECT\n\n--snapshot [WIDTH HEIGHT] [dashboard|history|conversation|commands|paste|compose|selection|colors|lifecycle|lifecycle-uncertain|chat|replay|replay-rejected|replay-validation|contract-failed|contract-historical|contract-unavailable|contract-unsupported] prints a fixture frame.\n--snapshot-ansi preserves colors for preview export.\nInteractive: F1 help; Ctrl-P commands (including connection refresh/reconnect/disconnect); F7 recipient in fixture; Enter simulates sending in fixture.\nA connection only issues bounded passive reads. Drafts exist only until exit."
        );
        return Ok(());
    }
    if args
        .first()
        .is_some_and(|a| a == "--snapshot" || a == "--snapshot-ansi")
    {
        let width = args.get(1).map(|s| s.parse()).transpose()?.unwrap_or(128);
        let height = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(40);
        if !(1..=300).contains(&width) || !(1..=100).contains(&height) {
            return Err("snapshot dimensions must be 1..300 by 1..100".into());
        }
        let (mut app, model) = application(EventListenerCfg::default());
        match args.get(3).map(String::as_str) {
            None | Some("dashboard") => (),
            Some("history") => {
                let mut m = model.borrow_mut();
                m.view = View::History;
                m.set_scope(fixture::Scope::Historical);
            }
            Some("conversation") => {
                let mut m = model.borrow_mut();
                m.select(2);
                m.view = View::Conversation;
                m.detail = 2;
                m.set_recipient();
                m.draft().insert_str(
                    "Keep historical views inspect-only. Show the live target before any action.",
                );
            }
            Some("replay") | Some("replay-rejected") | Some("replay-validation") => {
                let section = match args[3].as_str() {
                    "replay-rejected" => replay::Section::Rejected,
                    "replay-validation" => replay::Section::Validation,
                    _ => replay::Section::Live,
                };
                model.borrow_mut().open_replay(section);
            }
            Some("chat") => {
                model.borrow_mut().open_chat_example();
            }
            Some("contract-failed") => model
                .borrow_mut()
                .open_contract(contract::Case::FailedUnsettled),
            Some("contract-historical") => model
                .borrow_mut()
                .open_contract(contract::Case::HistoricalBlocked),
            Some("contract-unavailable") => model
                .borrow_mut()
                .open_contract(contract::Case::Unavailable),
            Some("contract-unsupported") => model
                .borrow_mut()
                .open_contract(contract::Case::Unsupported),
            Some("lifecycle") | Some("lifecycle-uncertain") => {
                let mut m = model.borrow_mut();
                m.view = View::Lifecycle;
                if args[3] == "lifecycle-uncertain" {
                    m.lifecycle_case = 1;
                }
            }
            Some("colors") => {
                let mut m = model.borrow_mut();
                m.view = View::Conversation;
                m.entries = vec![presentation::Entry { kind: crate::presentation::EntryKind::Message,
                    recipient: None, speaker: "builder:main", subject: "build:prototype", detail: 0,
                    text: "## A clearer conversation\n\n**Important decisions**, *supporting context*, and `inline code`.\nInspect the [review evidence](fixture://review/ui/r7).\n\n```rust\n// Keep inspection separate from delivery.\nfn can_send(live: bool, draft: &str) -> bool {\n    live && !draft.trim().is_empty()\n}\nlet message = \"Ready for review\";\n```\n\n```json\n{ \"scope\": \"live\", \"attempt\": 3, \"ready\": true }\n```\n\n```diff\n- runtime-coupled client\n+ independent supervision desk\n```\n\n> Color is presentation; copying preserves the source Markdown.".into(),
                }];
            }
            Some("selection") => {
                let mut m = model.borrow_mut();
                m.view = View::Conversation;
                m.entries = vec![presentation::Entry { kind: crate::presentation::EntryKind::Message,
                    recipient: Some("reviewer:main"), speaker: "you", subject: "operator:input", detail: 0,
                    text: "## Review plan\n\n- **Keep history inspect-only.**\n- Preserve the [evidence](fixture://review/ui/r7) and its limits.\n\n```text\nobjective -> review -> decision\n```\n\nSelect across wrapped lines: copying preserves the Markdown source and its original line breaks, without role labels or UI decoration.".into(),
                }];
            }
            Some("paste") => {
                let mut m = model.borrow_mut();
                m.focus = Pane::Composer;
                m.handle(tuirealm::event::Event::Paste(
                    "Please review this trace: ".into(),
                ));
                let payload = (1..=40)
                    .map(|n| format!("trace line {n}: sample activity for inspection"))
                    .collect::<Vec<_>>()
                    .join("\n");
                m.handle(tuirealm::event::Event::Paste(payload));
                m.handle(tuirealm::event::Event::Paste(
                    "\nExplain the wait and the next decision.".into(),
                ));
            }
            Some("compose") => {
                let mut m = model.borrow_mut();
                m.focus = Pane::Composer;
                m.handle(tuirealm::event::Event::Paste("Help me understand the current review wait and which decision needs my attention.\nKeep historical views inspect-only while we evaluate the standalone terminal client.\n\nShow the evidence behind the recommendation, including its limits and the responsible role.\nThen explain how the parallel build relates to that decision without assuming it is blocked.".into()));
            }
            Some("commands") => {
                let mut m = model.borrow_mut();
                m.focus = Pane::Composer;
                m.draft()
                    .insert_str("Keep this draft while choosing a command.");
                m.open_palette();
                m.palette.as_mut().unwrap().query = "to".into();
            }
            _ => return Err("unknown snapshot view".into()),
        }
        if args.get(3).is_some_and(|v| v == "selection") {
            ui::snapshot(&mut app, &model, width, height);
            let mut m = model.borrow_mut();
            let end = m.transcript.source.find("\n\nSelect").unwrap();
            m.transcript.selection = Some(0..end);
        }
        if args[0] == "--snapshot-ansi" {
            print!("{}", ui::snapshot_ansi(&mut app, &model, width, height));
        } else {
            println!("{}", ui::snapshot(&mut app, &model, width, height));
        }
        return Ok(());
    }
    let selected_connection = if args.is_empty() {
        None
    } else if args.len() == 4 && args[0] == "--connect" && args[2] == "--subject" {
        let socket = PathBuf::from(&args[1]);
        if !socket.is_absolute() || args[1].is_empty() {
            return Err("--connect requires an absolute Unix socket path".into());
        }
        Some((
            socket,
            lifecycle_client::SubjectRef::parse(args[3].clone())?,
        ))
    } else {
        return Err("unknown option; use --help".into());
    };
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("interactive mode needs a terminal; use --snapshot for text output".into());
    }
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        old_hook(info);
    }));
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        EnableBracketedPaste,
        EnableMouseCapture
    )?;
    if cfg!(unix) {
        execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
        KEYBOARD_ENHANCEMENT_PUSHED.store(true, Ordering::SeqCst);
    }
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let (mut app, model) = application(
        EventListenerCfg::default().crossterm_input_listener(Duration::from_millis(10), 1),
    );
    let worker = selected_connection.map(|(socket, subject)| {
        model.borrow_mut().connect(socket, subject);
        connection::Worker::start()
    });
    let mut last_pulse = Instant::now();
    let mut last_drag_scroll = Instant::now();
    while !model.borrow().quit {
        if let Some(worker) = &worker {
            while let Some(reply) = worker.try_reply() {
                model.borrow_mut().apply_connection_reply(reply);
            }
            let request = model.borrow_mut().take_connection_request();
            let rejection = request.and_then(|request| worker.request(request).err());
            if let Some(reason) = rejection {
                model.borrow_mut().connection_request_failed(reason);
            }
        }
        terminal.draw(|f| ui::render(f, &mut app, &model))?;
        // Apply each message before dispatching the next key: batching loses focus transitions.
        for msg in app.tick(PollStrategy::Once(Duration::from_millis(16)))? {
            apply(&mut app, &model, msg);
        }
        if last_drag_scroll.elapsed() >= Duration::from_millis(80) {
            model.borrow_mut().drag_scroll_tick();
            last_drag_scroll = Instant::now();
        }
        let copy = model.borrow_mut().clipboard.take();
        if let Some(markdown) = copy {
            model.borrow_mut().notice = match clipboard::request(&mut io::stdout(), &markdown) {
                Ok(()) => format!(
                    "Markdown copy requested ({} bytes) · requires terminal OSC 52 support · Esc clears",
                    markdown.len()
                ),
                Err(error) => format!("Clipboard request failed: {error}"),
            };
        }
        if last_pulse.elapsed() >= Duration::from_secs(4) {
            model
                .borrow_mut()
                .handle(tuirealm::event::Event::User(model::Update::Pulse));
            last_pulse = Instant::now();
        }
    }
    Ok(())
}
