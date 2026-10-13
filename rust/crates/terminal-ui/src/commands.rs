//! Local UI commands only. No shell execution or engine-operation discovery.
use crate::{
    connection::ReadKind,
    fixture::Scope,
    model::{Model, Pane, View},
};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};

#[derive(Clone, Copy)]
pub enum Action {
    View(View),
    Task(crate::task::Case),
    WorkFixture,
    Detail(usize),
    Context(usize),
    Scope(Scope),
    To(&'static str),
    Inspect(usize),
    Event(usize),
    Case(usize),
    Panel(bool),
    Scroll(bool, i32),
    Dependency,
    Graph(u8),
    Quit,
    Help,
    ChatExample,
    Replay(crate::replay::Section),
    Contract(crate::contract::Case),
    ConnectionRead(ReadKind),
    ConnectionDisconnect,
}
#[derive(Clone)]
pub struct Command {
    pub name: String,
    pub group: &'static str,
    pub description: &'static str,
    pub action: Action,
}
fn base_commands() -> Vec<Command> {
    let mut commands = vec![
        Command {
            name: "replay live".into(),
            group: "HISTORY",
            description: "Read the retained live text proof. Offline archive; no inference or engine connection.",
            action: Action::Replay(crate::replay::Section::Live),
        },
        Command {
            name: "replay rejected".into(),
            group: "HISTORY",
            description: "Read the separate zero-turn admission rejection. Not part of the successful run.",
            action: Action::Replay(crate::replay::Section::Rejected),
        },
        Command {
            name: "replay validation".into(),
            group: "HISTORY",
            description: "Inspect reported tests and controlled probes, separate from live inference.",
            action: Action::Replay(crate::replay::Section::Validation),
        },
        Command {
            name: "chat demo".into(),
            group: "VIEW",
            description: "Show an illustrative handoff interleaved with agent output. No runtime connection.",
            action: Action::ChatExample,
        },
        Command {
            name: "lifecycle".into(),
            group: "VIEW",
            description: "Inspect the reported handoff and illustrative failure states. Read-only; no native commands.",
            action: Action::View(View::Lifecycle),
        },
        Command {
            name: "dashboard".into(),
            group: "VIEW",
            description: "Open the work graph or compact list. Selection and drafts stay with you.",
            action: Action::View(View::Dashboard),
        },
        Command {
            name: "history".into(),
            group: "VIEW",
            description: "Open branch comparison. This changes the view, not the inspected time scope.",
            action: Action::View(View::History),
        },
        Command {
            name: "conversation".into(),
            group: "VIEW",
            description: "Open the shared process conversation. Does not change the message recipient.",
            action: Action::View(View::Conversation),
        },
        Command {
            name: "detail summary".into(),
            group: "DETAIL",
            description: "Show decisions and conversation summaries in the process timeline.",
            action: Action::Detail(0),
        },
        Command {
            name: "detail activity".into(),
            group: "DETAIL",
            description: "Include attributed activity alongside conversation summaries.",
            action: Action::Detail(1),
        },
        Command {
            name: "detail evidence".into(),
            group: "DETAIL",
            description: "Include exact fixture evidence references and activity in the timeline.",
            action: Action::Detail(2),
        },
        Command {
            name: "context overview".into(),
            group: "CONTEXT",
            description: "Inspect meaning, wait, and responsible role for the selected work.",
            action: Action::Context(0),
        },
        Command {
            name: "context evidence".into(),
            group: "CONTEXT",
            description: "Inspect the selected subject's evidence references and their limitations.",
            action: Action::Context(1),
        },
        Command {
            name: "context history".into(),
            group: "CONTEXT",
            description: "Inspect the selected subject's retained revision history.",
            action: Action::Context(2),
        },
        Command {
            name: "scope live".into(),
            group: "SCOPE",
            description: "Return to the live main r18 fixture. Retains selection and every role's draft.",
            action: Action::Scope(Scope::Live),
        },
        Command {
            name: "scope historical".into(),
            group: "SCOPE",
            description: "Inspect approach-a r11. Simulated send and recipient changes become unavailable.",
            action: Action::Scope(Scope::Historical),
        },
        Command {
            name: "to planner:main".into(),
            group: "RECIPIENT",
            description: "Explicitly address planner:main. Save the current draft and restore this role's draft. Sends nothing.",
            action: Action::To("planner:main"),
        },
        Command {
            name: "to reviewer:main".into(),
            group: "RECIPIENT",
            description: "Explicitly address reviewer:main. Save the current draft and restore this role's draft. Sends nothing.",
            action: Action::To("reviewer:main"),
        },
        Command {
            name: "to builder:main".into(),
            group: "RECIPIENT",
            description: "Explicitly address builder:main. Save the current draft and restore this role's draft. Sends nothing.",
            action: Action::To("builder:main"),
        },
        Command {
            name: "help".into(),
            group: "HELP",
            description: "Open the keyboard guide. All commands here are local prototype controls.",
            action: Action::Help,
        },
    ];
    for case in crate::task::Case::ALL {
        commands.push(Command {
            name: format!("task fixture {}", case.label()),
            group: "PROVISIONAL TASK",
            description: "Draft task projection only; no accepted protocol or host connection.",
            action: Action::Task(case),
        });
    }
    commands.push(Command {
        name: "overview".into(),
        group: "VIEW",
        description: "Open task Overview; starts provisional pending fixture if none is selected.",
        action: Action::View(View::Overview),
    });
    commands.push(Command {
        name: "graph".into(),
        group: "VIEW",
        description: "Inspect bound relationships; selection never retargets messaging.",
        action: Action::View(View::Dashboard),
    });
    commands.push(Command {
        name: "work fixture".into(),
        group: "VIEW",
        description: "Return to the original illustrative work fixture; restore drafts.",
        action: Action::WorkFixture,
    });
    for case in crate::contract::Case::ALL {
        commands.push(Command {
            name: format!("contract {}", case.label()),
            group: "S3 CONTRACT",
            description: "Inspect a published lifecycle-wire contract example. Read-only; neither live nor archive evidence.",
            action: Action::Contract(case),
        });
    }
    for (name, description, kind) in [
        (
            "connection refresh",
            "Read the selected endpoint again with identity and cursor checks.",
            ReadKind::Refresh,
        ),
        (
            "connection reconnect",
            "Re-read after an outage; the prior store, stream, and cursor must remain valid.",
            ReadKind::Reconnect,
        ),
    ] {
        commands.push(Command {
            name: name.into(),
            group: "PASSIVE CLIENT",
            description,
            action: Action::ConnectionRead(kind),
        });
    }
    commands.push(Command { name: "connection disconnect".into(), group: "PASSIVE CLIENT", description: "Stop inspecting the selected endpoint; keep its last observation labeled as old.", action: Action::ConnectionDisconnect });
    commands
}
pub struct Palette {
    pub query: String,
    pub selected: usize,
    pub commands: Vec<Command>,
}
impl Palette {
    pub fn matches(&self) -> Vec<&Command> {
        let query = self.query.trim().to_lowercase();
        let mut found: Vec<_> = self
            .commands
            .iter()
            .filter_map(|command| {
                let score = if command.name.starts_with(&query) {
                    0
                } else if command
                    .name
                    .split_whitespace()
                    .any(|word| word.starts_with(&query))
                {
                    1
                } else if query.chars().count() < 3 {
                    return None;
                } else if command.name.contains(&query) {
                    1
                } else if query.split_whitespace().all(|word| {
                    format!("{} {} {}", command.name, command.group, command.description)
                        .to_lowercase()
                        .contains(word)
                }) {
                    2
                } else {
                    let mut letters = query.chars();
                    let mut next = letters.next();
                    for c in command.name.chars() {
                        if next == Some(c) {
                            next = letters.next();
                        }
                    }
                    if next.is_none() {
                        3
                    } else {
                        return None;
                    }
                };
                Some((score, command))
            })
            .collect();
        found.sort_by_key(|(score, _)| *score);
        found.into_iter().map(|(_, command)| command).collect()
    }
}
impl Command {
    pub fn unavailable(&self, scope: Scope, view: View) -> Option<&'static str> {
        if view == View::Lifecycle && matches!(self.action, Action::To(_) | Action::Scope(_)) {
            Some("Unavailable in read-only lifecycle inspection. Return to a work view first.")
        } else if matches!(self.action, Action::To(_)) && scope == Scope::Historical {
            Some("Unavailable in history. Use scope live before changing recipients.")
        } else {
            None
        }
    }
}
impl Model {
    fn available_commands(&self) -> Vec<Command> {
        let mut commands = base_commands();
        let mut add = |name: String, group, description, action| {
            commands.push(Command {
                name,
                group,
                description,
                action,
            })
        };
        add(
            "panel main".into(),
            "PANEL",
            "Show the main view at compact sizes; keyboard stays in composer.",
            Action::Panel(false),
        );
        add(
            "panel context".into(),
            "PANEL",
            "Show context at compact sizes; keyboard stays in composer.",
            Action::Panel(true),
        );
        for (context, target) in [(false, "timeline"), (true, "context")] {
            if context || self.view == View::Conversation || self.task.is_some() {
                for (name, amount) in [
                    ("up", -8),
                    ("down", 8),
                    ("start", i32::MIN),
                    ("end", i32::MAX),
                ] {
                    add(
                        format!("scroll {target} {name}"),
                        "SCROLL",
                        "Move the inspection viewport without changing the draft.",
                        Action::Scroll(context, amount),
                    );
                }
            }
        }
        if let Some(task) = &self.task {
            for (index, node) in task.nodes.iter().enumerate() {
                add(
                    format!("inspect {} {}", node.id, node.title),
                    "TASK NODE",
                    "Scope inspection and Conversation; recipient remains unchanged.",
                    Action::Inspect(index),
                );
            }
        } else if self.view == View::Lifecycle {
            for (i, name) in crate::lifecycle::CASES.iter().enumerate() {
                add(
                    format!("scenario {name}"),
                    "SCENARIO",
                    "Inspect a labeled reported run or illustrative failure scenario.",
                    Action::Case(i),
                );
            }
            for (i, step) in crate::lifecycle::steps(self.lifecycle_case)
                .iter()
                .enumerate()
            {
                add(
                    format!("inspect step {} {}", i + 1, step.title),
                    "STEP",
                    "Inspect this lifecycle step without enabling native commands.",
                    Action::Inspect(i),
                );
            }
        } else if self.view == View::Conversation {
            let conversation = self.conversation();
            for (i, entry) in conversation.visible(self.detail) {
                let excerpt = entry
                    .text
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(70)
                    .collect::<String>();
                add(
                    format!("event {} {} {excerpt}", i + 1, entry.speaker),
                    "EVENT",
                    "Jump to this recorded or fixture message; detail evidence expands provenance.",
                    Action::Event(i),
                );
            }
        } else {
            for (i, item) in self.items().iter().enumerate() {
                add(
                    format!("inspect {} {}", item.id, item.title),
                    "WORK",
                    "Inspect this fixture work item. Message recipient stays unchanged.",
                    Action::Inspect(i),
                );
            }
            add(
                "follow dependency".into(),
                "WORK",
                "Inspect the selected fixture work dependency; no recipient change.",
                Action::Dependency,
            );
            add(
                "graph toggle".into(),
                "VIEW",
                "Switch dashboard graph and list presentation.",
                Action::Graph(0),
            );
        }
        if self.view == View::Dashboard && self.task.is_none() {
            for (i, name) in [
                "fit",
                "zoom in",
                "zoom out",
                "pan left",
                "pan right",
                "pan up",
                "pan down",
            ]
            .iter()
            .enumerate()
            {
                add(
                    format!("graph {name}"),
                    "GRAPH",
                    "Adjust the graph viewport; compact layouts still use the work list.",
                    Action::Graph(i as u8 + 1),
                );
            }
        }
        add(
            "quit".into(),
            "APP",
            "Exit the local prototype; unsent drafts require confirmation.",
            Action::Quit,
        );
        commands
    }
    pub fn open_palette(&mut self) {
        self.synchronize_conversation(self.conversation().source);
        self.palette = Some(Palette {
            query: String::new(),
            selected: 0,
            commands: self.available_commands(),
        });
        self.focus = Pane::Composer;
    }
    pub fn palette_key(&mut self, key: KeyEvent) {
        let read_only = self.input_read_only();
        let palette = self.palette.as_mut().unwrap();
        let matches = palette.matches();
        let index = palette.selected.min(matches.len().saturating_sub(1));
        match key.code {
            Key::Esc => {
                self.palette = None;
                self.focus = Pane::Composer;
            }
            Key::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.palette = None;
                self.focus = Pane::Composer;
            }
            Key::Up | Key::BackTab => {
                palette.selected = if index == 0 {
                    matches.len().saturating_sub(1)
                } else {
                    index - 1
                }
            }
            Key::Down => {
                palette.selected = if matches.is_empty() {
                    0
                } else {
                    (index + 1) % matches.len()
                }
            }
            Key::Tab => {
                if let Some(command) = matches.get(index) {
                    palette.query = command.name.clone();
                    palette.selected = 0;
                }
            }
            Key::Backspace => {
                palette.query.pop();
                palette.selected = 0;
            }
            Key::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                palette.query.push(c);
                palette.selected = 0;
            }
            Key::Enter => {
                let Some(command) = matches.get(index) else {
                    return;
                };
                if let Some(reason) = command.unavailable(
                    self.scope,
                    if read_only {
                        View::Lifecycle
                    } else {
                        self.view
                    },
                ) {
                    self.notice = reason.into();
                    return;
                }
                let command = (*command).clone();
                self.palette = None;
                self.focus = Pane::Composer;
                self.notice = format!("{} · local UI command; nothing sent.", command.name);
                match command.action {
                    Action::Task(case) => self.open_task(case),
                    Action::WorkFixture => {
                        self.task = None;
                        self.view = View::Dashboard;
                    }
                    Action::View(view) => {
                        if view == View::Overview && self.task.is_none() {
                            self.open_task(crate::task::Case::Pending);
                        }
                        self.leave_connection_view();
                        self.inspected_event = None;
                        self.pending_event = None;
                        self.replay = None;
                        self.chat_example = false;
                        self.contract_case = None;
                        self.client_observation = None;
                        self.client_last_known = None;
                        self.context_usage = Default::default();
                        self.view = if self.task.is_some() && view == View::History {
                            View::Overview
                        } else {
                            view
                        };
                        self.show_context = false;
                    }
                    Action::Detail(level) => {
                        self.detail = level;
                        self.pending_event = self.inspected_event;
                        self.scroll = 0;
                        self.view = View::Conversation;
                    }
                    Action::Context(tab) => {
                        self.context_tab = tab;
                        self.context_scroll = 0;
                        self.show_context = true;
                    }
                    Action::Scope(scope) => self.set_scope(scope),
                    Action::To(role) => {
                        self.target_role(role);
                        self.focus = Pane::Composer;
                    }
                    Action::Inspect(index) => {
                        self.select(index);
                        self.show_context = true;
                    }
                    Action::Event(index) => {
                        self.inspected_event = Some(index);
                        self.pending_event = Some(index);
                        self.show_context = false;
                    }
                    Action::Case(index) => {
                        self.lifecycle_case = index;
                        self.lifecycle_selected = 0;
                        self.context_scroll = 0;
                        self.show_context = false;
                    }
                    Action::Panel(context) => self.show_context = context,
                    Action::Scroll(context, amount) => {
                        let scroll = if context {
                            &mut self.context_scroll
                        } else {
                            &mut self.scroll
                        };
                        *scroll = if amount == i32::MIN {
                            0
                        } else if amount == i32::MAX {
                            u16::MAX
                        } else {
                            (*scroll as i32 + amount).clamp(0, u16::MAX as i32) as u16
                        };
                        self.show_context = context;
                    }
                    Action::Dependency => {
                        if let Some(index) = self.selected_work().dependency {
                            self.select(index);
                            self.show_context = true;
                        }
                    }
                    Action::Graph(operation) => match operation {
                        0 => self.graph_mode = !self.graph_mode,
                        1 => self.flow.request_fit_view(),
                        2 => self.flow.zoom_in(),
                        3 => self.flow.zoom_out(),
                        4 => self.flow.pan(5.0, 0.0),
                        5 => self.flow.pan(-5.0, 0.0),
                        6 => self.flow.pan(0.0, 3.0),
                        _ => self.flow.pan(0.0, -3.0),
                    },
                    Action::Quit => {
                        if self.has_drafts() {
                            self.confirm_quit = true;
                        } else {
                            self.quit = true;
                        }
                    }
                    Action::Help => self.help = true,
                    Action::ChatExample => self.open_chat_example(),
                    Action::Replay(section) => self.open_replay(section),
                    Action::Contract(case) => self.open_contract(case),
                    Action::ConnectionRead(kind) => self.request_connection_read(kind),
                    Action::ConnectionDisconnect => self.disconnect_connection(),
                }
            }
            _ => (), // While open, palette owns keyboard input; F8 cannot send the draft.
        }
    }
}
