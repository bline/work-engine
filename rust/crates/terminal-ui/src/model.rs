use crate::connection::{ReadKind, Reply, Request};
use crate::draft::Draft;
use crate::fixture::{self, Scope, Work};
use crate::presentation::{Conversation, Entry, SourceId};
use crate::theme::*;
use rataflow::{Edge, Flow, HandlePosition, Node, StepEdge, Theme};
use ratatui_textarea::{Input, Key as EditKey};
use std::collections::HashMap;
use std::path::PathBuf;
use tuirealm::event::{Event, Key, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use tuirealm::ratatui::style::Style;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pane {
    Main,
    Context,
    Composer,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Overview,
    Dashboard,
    History,
    Conversation,
    Lifecycle,
}
impl View {
    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Dashboard => "Graph",
            Self::History => "History",
            Self::Conversation => "Conversation",
            Self::Lifecycle => "Lifecycle",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Update {
    Pulse,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    Input(Pane, Event<Update>),
}

pub struct PastePreview {
    pub text: String,
    pub scroll: u16,
}

pub struct Connection {
    pub socket: PathBuf,
    pub subject: lifecycle_client::SubjectRef,
    pub generation: u64,
    pub disconnected: bool,
    pub refreshing: bool,
    pub last_known: Option<lifecycle_wire::LifecycleSnapshotV1>,
}

pub struct Model {
    pub task: Option<crate::task::Task>,
    last_task_click: Option<(usize, std::time::Instant)>,
    conversation_source: Option<SourceId>,
    pub contract_case: Option<crate::contract::Case>,
    pub client_observation: Option<Conversation>,
    pub connection: Option<Connection>,
    pending_connection_read: Option<ReadKind>,
    connection_generation: u64,
    // Exercised by U0's controlled transport tests; S4 owns the live adapter.
    #[allow(dead_code)]
    pub(crate) client_last_known: Option<lifecycle_wire::LifecycleSnapshotV1>,
    #[allow(dead_code)]
    client_source_sequence: u64,
    pub replay: Option<crate::replay::Replay>,
    pub context_usage: crate::chat::ContextUsage,
    pub chat_example: bool,
    pub lifecycle_case: usize,
    pub lifecycle_selected: usize,
    pub mouse: crate::mouse::MouseLayout,
    pub transcript: crate::mouse::Transcript,
    pub clipboard: Option<String>,
    pub paste_preview: Option<PastePreview>,
    pub palette: Option<crate::commands::Palette>,
    pub view: View,
    pub scope: Scope,
    pub focus: Pane,
    pub show_context: bool,
    pub pending_event: Option<usize>,
    pub inspected_event: Option<usize>,
    pub selected: usize,
    pub detail: usize,
    pub context_tab: usize,
    pub graph_mode: bool,
    pub flow: Flow,
    pub recipient: &'static str,
    pub drafts: HashMap<&'static str, Draft>,
    pub entries: Vec<Entry>,
    pub notice: String,
    pub pulses: usize,
    pub scroll: u16,
    pub context_scroll: u16,
    pub help: bool,
    pub help_scroll: u16,
    pub quit: bool,
    pub confirm_quit: bool,
    pub last_graph_size: (u16, u16),
}
impl Model {
    pub fn input_read_only(&self) -> bool {
        self.task.is_some()
            || self.view == View::Lifecycle
            || (self.connection.is_some()
                && self.view == View::Conversation
                && self.client_observation.is_some())
            || (self.chat_example
                || self.replay.is_some()
                || self.contract_case.is_some()
                || self.client_observation.is_some())
                && self.view == View::Conversation
    }
    pub fn open_task(&mut self, case: crate::task::Case) {
        self.leave_connection_view();
        self.task = Some(crate::task::Task::fixture(case));
        self.client_observation = None;
        self.replay = None;
        self.contract_case = None;
        self.chat_example = false;
        self.view = View::Overview;
        self.scroll = 0;
        self.context_scroll = 0;
        self.last_task_click = None;
        self.inspected_event = None;
        self.pending_event = None;
        self.show_context = false;
        self.transcript.clear();
        self.notice =
            "PROVISIONAL TASK FIXTURE · non-atomic components · no host connection".into();
    }
    pub fn connect(&mut self, socket: PathBuf, subject: lifecycle_client::SubjectRef) {
        self.task = None;
        self.connection_generation += 1;
        self.client_source_sequence += 1;
        self.connection = Some(Connection {
            socket,
            subject,
            generation: self.connection_generation,
            disconnected: false,
            refreshing: true,
            last_known: None,
        });
        self.pending_connection_read = Some(ReadKind::Initial);
        self.replay = None;
        self.chat_example = false;
        self.contract_case = None;
        self.client_observation = Some(crate::contract::client_pending(SourceId::Client(
            self.client_source_sequence,
        )));
        self.view = View::Conversation;
        self.focus = Pane::Composer;
        self.show_context = false;
        self.notice = "PASSIVE CLIENT · connecting · read-only; drafts retained".into();
    }
    pub fn request_connection_read(&mut self, kind: ReadKind) {
        let Some(connection) = &mut self.connection else {
            self.notice = "No endpoint selected. Launch with --connect PATH --subject ID.".into();
            return;
        };
        if connection.refreshing {
            self.notice = "A bounded read is already in progress.".into();
            return;
        }
        if connection.disconnected && kind == ReadKind::Refresh {
            self.notice = "Disconnected. Use connection reconnect.".into();
            return;
        }
        self.task = None;
        connection.disconnected = false;
        connection.refreshing = true;
        self.pending_connection_read = Some(kind);
        self.replay = None;
        self.chat_example = false;
        self.contract_case = None;
        if self.client_observation.is_none() {
            self.client_observation = Some(crate::contract::client_status(
                connection.last_known.as_ref(),
                SourceId::Client(self.client_source_sequence),
                "returning to connection · last known only",
            ));
        }
        self.view = View::Conversation;
        self.notice = format!(
            "PASSIVE CLIENT · {kind:?} in progress · previous observation is last known only"
        );
    }
    /// Leaving passive inspection invalidates any in-flight reply without losing
    /// the endpoint's prior observation for an explicit later reconnect.
    pub fn leave_connection_view(&mut self) {
        if let Some(connection) = &mut self.connection {
            self.connection_generation += 1;
            connection.generation = self.connection_generation;
            connection.disconnected = true;
            connection.refreshing = false;
            self.pending_connection_read = None;
            self.client_observation = None;
        }
    }
    pub fn disconnect_connection(&mut self) {
        if let Some(connection) = &mut self.connection {
            self.task = None;
            self.connection_generation += 1;
            connection.generation = self.connection_generation;
            connection.disconnected = true;
            connection.refreshing = false;
            self.pending_connection_read = None;
            let source = SourceId::Client(self.client_source_sequence);
            self.client_observation = Some(crate::contract::client_status(
                connection.last_known.as_ref(),
                source,
                "explicitly disconnected · last known only",
            ));
            self.view = View::Conversation;
            self.notice =
                "PASSIVE CLIENT · explicitly disconnected · no new reads; pending replies ignored"
                    .into();
        }
    }
    pub fn take_connection_request(&mut self) -> Option<Request> {
        let kind = self.pending_connection_read.take()?;
        let connection = self.connection.as_ref()?;
        Some(Request {
            generation: connection.generation,
            socket: connection.socket.clone(),
            subject: connection.subject.clone(),
            previous: connection.last_known.as_ref().map(|s| s.cursor.clone()),
            kind,
        })
    }
    pub fn connection_request_failed(&mut self, reason: &str) {
        if let Some(connection) = &mut self.connection {
            connection.refreshing = false;
        }
        self.notice =
            format!("PASSIVE CLIENT · {reason} · previous observation remains last known");
    }
    pub fn apply_connection_reply(&mut self, reply: Reply) {
        let Some(connection) = &mut self.connection else {
            return;
        };
        if connection.generation != reply.generation
            || connection.disconnected
            || !connection.refreshing
        {
            return;
        }
        connection.refreshing = false;
        let previous = connection.last_known.clone();
        let result = reply.result.and_then(|snapshot| {
            snapshot
                .validate_header(connection.subject.as_str())
                .map_err(lifecycle_client::ClientError::from)?;
            if let Some(old) = &previous {
                if old.cursor.store_id != snapshot.cursor.store_id
                    || old.cursor.stream_id != snapshot.cursor.stream_id
                {
                    return Err(lifecycle_client::ClientError::IdentityMismatch);
                }
                let old_seq = old
                    .cursor
                    .commit_sequence
                    .parse::<u64>()
                    .map_err(|_| lifecycle_client::ClientError::InvalidObservation)?;
                let new_seq = snapshot
                    .cursor
                    .commit_sequence
                    .parse::<u64>()
                    .map_err(|_| lifecycle_client::ClientError::InvalidObservation)?;
                if new_seq < old_seq {
                    return Err(lifecycle_client::ClientError::InvalidObservation);
                }
            }
            Ok(snapshot)
        });
        let source = SourceId::Client(self.client_source_sequence);
        let changed = match &result {
            Ok(snapshot) => {
                previous.as_ref() != Some(snapshot)
                    || self
                        .client_observation
                        .as_ref()
                        .is_none_or(|c| c.banner != "coherent read · current snapshot · read-only")
            }
            Err(_) => true,
        };
        if changed {
            self.transcript.clear();
            self.inspected_event = None;
            self.pending_event = None;
            self.scroll = 0;
            self.context_scroll = 0;
        }
        match result {
            Ok(snapshot) => {
                connection.last_known = Some(snapshot.clone());
                self.client_observation = Some(crate::contract::snapshot_conversation(
                    &snapshot,
                    source,
                    "coherent read",
                ));
                self.notice = format!("PASSIVE CLIENT · {:?} complete · read-only", reply.kind);
            }
            Err(error) => {
                self.client_observation = Some(crate::contract::client_status(
                    previous.as_ref(),
                    source,
                    &format!("{:?}: {error} · last known only", reply.kind),
                ));
                self.notice = format!("PASSIVE CLIENT · {:?}: {error} · read-only", reply.kind);
            }
        }
        self.view = View::Conversation;
    }
    #[allow(dead_code)]
    fn show_observation(&mut self, project: impl FnOnce(SourceId) -> Conversation) {
        self.client_source_sequence += 1;
        let conversation = project(SourceId::Client(self.client_source_sequence));
        self.replay = None;
        self.chat_example = false;
        self.contract_case = None;
        self.client_observation = Some(conversation);
        self.inspected_event = None;
        self.pending_event = None;
        self.show_context = false;
        self.view = View::Conversation;
        self.detail = 0;
        self.scroll = 0;
        self.context_scroll = 0;
        self.notice = "PASSIVE CLIENT OBSERVATION · read-only · no native commands".into();
    }
    #[allow(dead_code)]
    pub fn show_client_snapshot(
        &mut self,
        subject: &lifecycle_client::SubjectRef,
        result: Result<lifecycle_wire::LifecycleSnapshotV1, lifecycle_client::ClientError>,
    ) {
        let previous = self
            .client_last_known
            .take()
            .filter(|snapshot| snapshot.subject_id == subject.as_str());
        let result = result.and_then(|snapshot| {
            snapshot
                .validate_header(subject.as_str())
                .map_err(lifecycle_client::ClientError::from)?;
            Ok(snapshot)
        });
        self.client_last_known = match &result {
            Ok(snapshot) => Some(snapshot.clone()),
            Err(
                lifecycle_client::ClientError::ObservationUnavailable
                | lifecycle_client::ClientError::DeadlineExpired,
            ) => previous.clone(),
            Err(_) => None,
        };
        self.show_observation(|source| {
            crate::contract::client_snapshot(result, previous.as_ref(), source, "snapshot")
        });
    }
    #[allow(dead_code)]
    pub fn show_client_reconnect(
        &mut self,
        subject: &lifecycle_client::SubjectRef,
        result: Result<lifecycle_wire::LifecycleSnapshotV1, lifecycle_client::ClientError>,
    ) {
        let previous = self
            .client_last_known
            .take()
            .filter(|snapshot| snapshot.subject_id == subject.as_str());
        let result = result.and_then(|snapshot| {
            snapshot
                .validate_header(subject.as_str())
                .map_err(lifecycle_client::ClientError::from)?;
            Ok(snapshot)
        });
        self.client_last_known = match &result {
            Ok(snapshot) => Some(snapshot.clone()),
            Err(
                lifecycle_client::ClientError::ObservationUnavailable
                | lifecycle_client::ClientError::DeadlineExpired,
            ) => previous.clone(),
            Err(_) => None,
        };
        self.show_observation(|source| {
            crate::contract::client_snapshot(result, previous.as_ref(), source, "reconnect")
        });
    }
    #[allow(dead_code)]
    pub fn show_client_wait(
        &mut self,
        subject: &lifecycle_client::SubjectRef,
        result: lifecycle_wire::WaitResultV1<lifecycle_wire::LifecycleSnapshotV1>,
    ) {
        let previous = self
            .client_last_known
            .take()
            .filter(|snapshot| snapshot.subject_id == subject.as_str());
        let observed = match &result {
            lifecycle_wire::WaitResultV1::Observed(snapshot)
            | lifecycle_wire::WaitResultV1::DeadlineExpired {
                last_seen: Some(snapshot),
            } => Some(snapshot),
            _ => None,
        };
        if let Some(error) =
            observed.and_then(|snapshot| snapshot.validate_header(subject.as_str()).err())
        {
            self.show_observation(|source| {
                crate::contract::client_snapshot(Err(error.into()), None, source, "wait")
            });
            return;
        }
        self.client_last_known = match &result {
            lifecycle_wire::WaitResultV1::Observed(snapshot) => Some(snapshot.clone()),
            lifecycle_wire::WaitResultV1::DeadlineExpired {
                last_seen: Some(snapshot),
            } => Some(snapshot.clone()),
            lifecycle_wire::WaitResultV1::DeadlineExpired { last_seen: None } => previous.clone(),
            lifecycle_wire::WaitResultV1::ObservationUnavailable {
                code: lifecycle_wire::WireErrorCode::ObservationUnavailable,
            } => previous.clone(),
            _ => None,
        };
        self.show_observation(|source| {
            crate::contract::client_wait(result, previous.as_ref(), source)
        });
    }
    pub fn open_contract(&mut self, case: crate::contract::Case) {
        self.task = None;
        self.leave_connection_view();
        self.client_observation = None;
        self.client_last_known = None;
        self.replay = None;
        self.chat_example = false;
        self.contract_case = Some(case);
        self.inspected_event = None;
        self.pending_event = None;
        self.show_context = false;
        self.view = View::Conversation;
        self.detail = 0;
        self.scroll = 0;
        self.context_scroll = 0;
        self.notice = "S3 CONTRACT EXAMPLE · not live or archive evidence · read-only".into();
    }
    pub fn open_replay(&mut self, section: crate::replay::Section) {
        self.task = None;
        self.leave_connection_view();
        match crate::replay::Replay::load(crate::replay::ARCHIVE, section) {
            Ok(replay) => {
                self.inspected_event = None;
                self.pending_event = None;
                self.show_context = false;
                self.replay = Some(replay);
                self.contract_case = None;
                self.client_observation = None;
                self.client_last_known = None;
                self.chat_example = false;
                self.context_usage = Default::default();
                self.view = View::Conversation;
                self.focus = Pane::Composer;
                self.detail = 0;
                self.scroll = 0;
                self.context_scroll = 0;
                self.notice = "HISTORICAL REPLAY · retained evidence · read-only · no running engine connected".into();
            }
            Err(error) => self.notice = error,
        }
    }
    pub fn open_chat_example(&mut self) {
        self.task = None;
        self.leave_connection_view();
        self.replay = None;
        self.contract_case = None;
        self.client_observation = None;
        self.client_last_known = None;
        self.inspected_event = None;
        self.pending_event = None;
        self.show_context = false;
        self.detail = 0;
        self.context_scroll = 0;
        self.chat_example = true;
        self.context_usage.replace("B");
        self.view = View::Conversation;
        self.focus = Pane::Composer;
        self.scroll = 0;
        self.notice = "ILLUSTRATIVE CHAT · invented messages and alternate uncertain ending · no runtime connection".into();
    }

    pub fn drag_scroll_tick(&mut self) {
        if self.view != View::Conversation
            || self.help
            || self.confirm_quit
            || self.palette.is_some()
            || self.paste_preview.is_some()
            || self.transcript.area.is_empty()
        {
            self.transcript.stop_drag();
        } else if self.transcript.auto_scroll() {
            self.scroll = self.transcript.scroll as u16;
        }
    }
    fn handle_mouse(&mut self, event: MouseEvent) {
        use MouseEventKind::*;
        let up = event.kind == ScrollUp;
        let wheel = matches!(event.kind, ScrollUp | ScrollDown);
        let adjust = |value: u16| {
            if up {
                value.saturating_sub(3)
            } else {
                value.saturating_add(3)
            }
        };
        // Modals own mouse input too: no click-through or accidental send.
        if self.confirm_quit || self.palette.is_some() {
            return;
        }
        if self.help {
            if wheel {
                self.help_scroll = adjust(self.help_scroll);
            }
            return;
        }
        if let Some(preview) = &mut self.paste_preview {
            if wheel {
                preview.scroll = adjust(preview.scroll);
            }
            return;
        }
        let point = (event.column, event.row).into();
        if event.kind == Down(MouseButton::Left) {
            if self.mouse.commands.contains(point) {
                self.open_palette();
                return;
            }
            if let Some((_, view)) = self.mouse.tabs.iter().find(|(r, _)| r.contains(point)) {
                self.view = *view;
                if self.view != View::Conversation {
                    self.leave_connection_view();
                    self.contract_case = None;
                    self.client_observation = None;
                    self.client_last_known = None;
                }
                self.show_context = false;
                self.focus = Pane::Composer;
                self.transcript.clear();
                return;
            }

            if let Some((_, item)) = self.mouse.items.iter().find(|(r, _)| r.contains(point)) {
                let item = *item;
                if self.task.is_some() {
                    let now = std::time::Instant::now();
                    let double = self.last_task_click.is_some_and(|(old, at)| {
                        old == item && now.duration_since(at).as_millis() <= 450
                    });
                    self.select(item);
                    self.last_task_click = Some((item, now));
                    if double {
                        self.view = View::Conversation;
                        self.last_task_click = None;
                    }
                } else {
                    self.select(item);
                }
                return;
            }
        }
        if wheel {
            match self
                .mouse
                .panes
                .iter()
                .find(|(r, _)| r.contains(point))
                .map(|(_, p)| p)
            {
                Some(Pane::Context) => self.context_scroll = adjust(self.context_scroll),
                Some(Pane::Main) if self.view == View::Conversation => {
                    self.scroll = adjust(self.scroll)
                }
                Some(Pane::Main) if self.task.is_some() => self.scroll = adjust(self.scroll),
                Some(Pane::Main) if self.view == View::Lifecycle => self.select(if up {
                    self.lifecycle_selected.saturating_sub(1)
                } else {
                    self.lifecycle_selected + 1
                }),
                Some(Pane::Main) => self.select(if up {
                    (self.selected + self.items().len() - 1) % self.items().len()
                } else {
                    self.selected + 1
                }),
                Some(Pane::Composer) if self.input_read_only() => (),
                Some(Pane::Composer) => {
                    self.draft().input(Input {
                        key: if up {
                            EditKey::PageUp
                        } else {
                            EditKey::PageDown
                        },
                        ..Input::default()
                    });
                }
                None => (),
            }
        }
        if self.view == View::Conversation {
            match event.kind {
                Down(MouseButton::Left) if self.transcript.area.contains(point) => {
                    self.transcript.down(event.column, event.row)
                }
                Down(MouseButton::Left) => self.transcript.clear(),
                Drag(MouseButton::Left) => self.transcript.drag(event.column, event.row),
                Up(MouseButton::Left) => self.transcript.up(event.column, event.row),
                _ => (),
            }
        }
    }
    pub fn new() -> Self {
        let mut drafts = HashMap::new();
        drafts.insert("planner:main", editor());
        Self {
            task: None,
            last_task_click: None,
            conversation_source: None,
            contract_case: None,
            client_observation: None,
            connection: None,
            pending_connection_read: None,
            connection_generation: 0,
            client_last_known: None,
            client_source_sequence: 0,
            replay: None,
            context_usage: Default::default(),
            chat_example: false,
            lifecycle_case: 0,
            lifecycle_selected: 0,
            mouse: Default::default(),
            transcript: Default::default(),
            clipboard: None,
            palette: None,
            paste_preview: None,
            view: View::Dashboard,
            scope: Scope::Live,
            focus: Pane::Composer,
            show_context: false,
            pending_event: None,
            inspected_event: None,
            selected: 0,
            detail: 0,
            context_tab: 0,
            graph_mode: true,
            flow: graph(Scope::Live, 0),
            recipient: "planner:main",
            drafts,
            entries: fixture::timeline(Scope::Live),
            notice:
                "Ctrl-P opens commands. Type to compose; inspection never changes the recipient."
                    .into(),
            pulses: 0,
            scroll: 0,
            context_scroll: 0,
            help: false,
            help_scroll: 0,
            quit: false,
            confirm_quit: false,
            last_graph_size: (0, 0),
        }
    }
    pub fn conversation(&self) -> Conversation {
        if let Some(task) = &self.task {
            task.conversation()
        } else if let Some(observation) = &self.client_observation {
            observation.clone()
        } else if let Some(case) = self.contract_case {
            crate::contract::example(case)
        } else if let Some(replay) = &self.replay {
            Conversation::archive(replay)
        } else if self.chat_example {
            Conversation::example(self.context_usage.label())
        } else {
            let entries = if self.scope == Scope::Historical {
                fixture::timeline(self.scope)
            } else {
                self.entries.clone()
            };
            let work = self.selected_work();
            Conversation::work(
                self.scope,
                entries,
                self.context_usage.label(),
                format!("Inspecting {} / {}", work.owner, work.title),
            )
        }
    }
    /// Source-local row indices and mouse offsets cannot cross datasets.
    pub fn synchronize_conversation(&mut self, source: SourceId) {
        if self.conversation_source != Some(source) {
            self.conversation_source = Some(source);
            self.transcript.clear();
            self.inspected_event = None;
            self.pending_event = None;
            self.scroll = 0;
        }
    }
    pub fn items(&self) -> Vec<Work> {
        fixture::work(self.scope)
    }
    pub fn selected_work(&self) -> Work {
        self.items()[self.selected]
    }
    pub fn draft(&mut self) -> &mut Draft {
        self.drafts.get_mut(self.recipient).unwrap()
    }
    pub fn draft_text(&self) -> String {
        self.drafts[self.recipient].text()
    }
    pub fn has_drafts(&self) -> bool {
        self.drafts
            .values()
            .any(|d| d.lines().iter().any(|l| !l.is_empty()))
    }
    pub fn set_scope(&mut self, scope: Scope) {
        if self.input_read_only() {
            self.notice =
                "Use Ctrl-P scenario to inspect lifecycle examples. Work-fixture scope unchanged."
                    .into();
            return;
        }
        self.transcript.clear();
        self.inspected_event = None;
        self.pending_event = None;
        self.scope = scope;
        self.flow = graph(scope, self.selected);
        self.last_graph_size = (0, 0);
        self.scroll = 0;
        self.context_scroll = 0;
        self.notice = if scope == Scope::Historical {
            "Historical snapshot. Drafts retained; simulated send disabled. F5 returns to live."
                .into()
        } else {
            "Returned to live fixture. Recipient and drafts retained.".into()
        };
    }
    pub fn select(&mut self, index: usize) {
        if let Some(task) = &mut self.task {
            task.selected = index % task.nodes.len();
            self.scroll = 0;
            self.context_scroll = 0;
            self.transcript.clear();
            return;
        }
        if self.view == View::Lifecycle {
            self.lifecycle_selected =
                index.min(crate::lifecycle::steps(self.lifecycle_case).len() - 1);
            self.context_scroll = 0;
            return;
        }
        self.selected = index % self.items().len();
        self.flow.select_node(self.selected_work().id);
        self.context_scroll = 0;
    }
    pub fn pulse(&mut self) {
        // A bounded, invented activity counter; never changes the selected subject or scope.
        self.pulses = (self.pulses + 1) % 1000;
    }
    #[cfg(test)]
    pub fn visible_entries(&self) -> Vec<Entry> {
        self.conversation()
            .visible(self.detail)
            .map(|(_, entry)| entry.clone())
            .collect()
    }
    pub fn set_recipient(&mut self) {
        self.target_role(self.selected_work().owner);
    }
    pub fn target_role(&mut self, role: &'static str) {
        if self.input_read_only() {
            self.notice =
                "Lifecycle inspection cannot retarget drafts. Native commands are unwired.".into();
            return;
        }
        if self.scope == Scope::Historical {
            self.notice =
                "Recipient unchanged: historical roles are inspect-only. F5 returns to live."
                    .into();
            return;
        }
        self.recipient = role;
        self.drafts.entry(self.recipient).or_insert_with(editor);
        self.notice = format!(
            "Composer now targets {}. Each role keeps its own draft.",
            self.recipient
        );
    }
    pub fn send(&mut self) {
        if self.input_read_only() {
            self.notice =
                "Lifecycle is read-only. Draft retained; no command or simulated send.".into();
            return;
        }
        if self.scope == Scope::Historical {
            self.notice = "Not sent: historical snapshot is inspect-only. Draft retained.".into();
        } else if self.focus != Pane::Composer {
            self.notice = "Focus the composer before F8 to simulate send.".into();
        } else if self.draft_text().trim().is_empty() {
            self.notice = "Nothing to simulate: write a message first.".into();
        } else {
            if self.chat_example || self.replay.is_some() {
                self.replay = None;
                self.chat_example = false;
                self.context_usage = Default::default();
            }
            let text = self.draft_text();
            self.entries.push(Entry {
                kind: crate::presentation::EntryKind::Message,
                subject: "operator:input",
                speaker: "you",
                recipient: Some(self.recipient),
                text,
                detail: 0,
            });
            // The demo retains at most 50 entries; this is not a durable transcript.
            if self.entries.len() > 50 {
                self.entries.remove(0);
            }
            self.drafts.insert(self.recipient, editor());
            self.notice = format!(
                "Simulated locally to {}. No owner acknowledgment or real effect.",
                self.recipient
            );
            self.view = View::Conversation;
            self.scroll = u16::MAX;
        }
    }
    pub fn handle(&mut self, event: Event<Update>) {
        if let Event::User(Update::Pulse) = event {
            self.pulse();
            return;
        }
        if let Event::WindowResize(..) = event {
            self.transcript.stop_drag();
            self.last_graph_size = (0, 0);
            return;
        }
        if let Event::Paste(text) = event {
            if let Some(palette) = &mut self.palette {
                palette
                    .query
                    .extend(text.chars().filter(|c| !c.is_control()));
                palette.selected = 0;
                return;
            }
            if self.focus == Pane::Composer
                && !self.input_read_only()
                && self.scope == Scope::Live
                && !self.help
                && !self.confirm_quit
                && self.paste_preview.is_none()
            {
                self.notice = match self.draft().paste(&text) {
                    Ok(true) => {
                        "Large paste folded · Ctrl-O inspects full draft · full text retained."
                            .into()
                    }
                    Ok(false) => "Pasted into draft · not sent · Ctrl-Z undoes paste.".into(),
                    Err(reason) => reason.into(),
                };
            }
            return;
        }
        if let Event::Mouse(mouse) = event {
            self.handle_mouse(mouse);
            return;
        }
        let Event::Keyboard(key) = event else {
            return;
        };
        self.focus = Pane::Composer;
        self.transcript.stop_drag();
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == Key::Char('p') && !self.confirm_quit && self.palette.is_none() {
            self.help = false;
            self.paste_preview = None;
            self.open_palette();
            return;
        }
        if let Some(preview) = &mut self.paste_preview {
            match key.code {
                Key::Esc => self.paste_preview = None,
                Key::Char('o') if ctrl => self.paste_preview = None,
                Key::Up => preview.scroll = preview.scroll.saturating_sub(1),
                Key::Down => preview.scroll = preview.scroll.saturating_add(1),
                Key::PageUp => preview.scroll = preview.scroll.saturating_sub(8),
                Key::PageDown => preview.scroll = preview.scroll.saturating_add(8),
                Key::Home => preview.scroll = 0,
                Key::End => preview.scroll = u16::MAX,
                Key::Char('c') if ctrl => self.paste_preview = None,
                _ => (),
            }
            return;
        }
        if self.palette.is_some() {
            self.palette_key(key);
            return;
        }
        if self.confirm_quit {
            match key.code {
                Key::Char('y') => self.quit = true,
                Key::Esc | Key::Char('n') => self.confirm_quit = false,
                _ => (),
            }
            return;
        }
        if !self.help && self.view == View::Conversation {
            if ctrl
                && matches!(key.code, Key::Char('c') | Key::Char('C'))
                && let Some(text) = self.transcript.selected()
            {
                self.clipboard = Some(text);
                return;
            }
            if key.code == Key::Esc && self.transcript.selection.is_some() {
                self.transcript.clear();
                return;
            }
        }
        if ctrl && key.code == Key::Char('c') {
            if self.has_drafts() {
                self.confirm_quit = true;
            } else {
                self.quit = true;
            }
            return;
        }
        if self.help {
            if matches!(key.code, Key::Esc | Key::Function(1) | Key::Char('?')) {
                self.help = false;
            }
            match key.code {
                Key::Up | Key::PageUp => self.help_scroll = self.help_scroll.saturating_sub(1),
                Key::Down | Key::PageDown => self.help_scroll = self.help_scroll.saturating_add(1),
                _ => (),
            }
            return;
        }
        match key.code {
            Key::Char('o') if ctrl && self.focus == Pane::Composer && !self.input_read_only() => {
                self.paste_preview = Some(PastePreview {
                    text: self.draft_text(),
                    scroll: 0,
                });
            }
            Key::Char('p') if ctrl => self.open_palette(),
            Key::Function(1) => self.help = true,
            Key::Function(2) => {
                self.view = match self.view {
                    View::Overview => View::Dashboard,
                    View::Dashboard if self.task.is_some() => View::Conversation,
                    View::Dashboard => View::History,
                    View::History => View::Conversation,
                    View::Conversation => View::Lifecycle,
                    View::Lifecycle if self.task.is_some() => View::Overview,
                    View::Lifecycle => View::Dashboard,
                };
                if self.view != View::Conversation {
                    self.leave_connection_view();
                    self.contract_case = None;
                    self.client_observation = None;
                    self.client_last_known = None;
                }
            }
            Key::Function(3) => {
                self.detail = (self.detail + 1) % 3;
                self.pending_event = self.inspected_event;
                self.scroll = 0;
            }
            Key::Function(4) => {
                self.context_tab = (self.context_tab + 1) % 3;
                self.context_scroll = 0;
            }
            Key::Function(5) => self.set_scope(if self.scope == Scope::Live {
                Scope::Historical
            } else {
                Scope::Live
            }),
            Key::Function(6) => self.graph_mode = !self.graph_mode,
            Key::Function(7) => self.set_recipient(),
            Key::Function(8) => self.send(),
            Key::Function(9) => self.pulse(),
            Key::Tab | Key::BackTab | Key::Esc => (),
            _ if self.input_read_only() || self.scope == Scope::Historical => {
                self.notice = "Read-only view · Ctrl-P commands · drafts retained.".into();
            }
            Key::Enter if self.focus == Pane::Composer => {
                if key.modifiers.is_empty() {
                    self.send();
                } else if key
                    .modifiers
                    .intersects(KeyModifiers::SHIFT | KeyModifiers::ALT)
                {
                    self.draft().insert_str("\n");
                }
            }
            _ if self.focus == Pane::Composer => {
                let edit_key = match key.code {
                    Key::Char('z') if ctrl => EditKey::Char('u'),
                    Key::Char('y') if ctrl => EditKey::Char('r'),
                    Key::Char(c) => EditKey::Char(c),
                    Key::Enter => EditKey::Enter,
                    Key::Backspace => EditKey::Backspace,
                    Key::Delete => EditKey::Delete,
                    Key::Left => EditKey::Left,
                    Key::Right => EditKey::Right,
                    Key::Up => EditKey::Up,
                    Key::Down => EditKey::Down,
                    Key::Home => EditKey::Home,
                    Key::End => EditKey::End,
                    Key::PageUp => EditKey::PageUp,
                    Key::PageDown => EditKey::PageDown,
                    _ => return,
                };
                self.draft().input(Input {
                    key: edit_key,
                    ctrl,
                    alt: key.modifiers.contains(KeyModifiers::ALT),
                    shift: key.modifiers.contains(KeyModifiers::SHIFT),
                });
            }
            _ => (),
        }
    }
}
fn editor() -> Draft {
    Draft::new()
}

fn graph(scope: Scope, selected: usize) -> Flow {
    let items = fixture::work(scope);
    let nodes = items
        .iter()
        .map(|item| {
            let mut node = Node::from_text(
                item.id,
                item.position,
                format!("{}\n{}", item.title, item.status),
            )
            .with_dimensions(27.0, 4.0)
            .with_draggable(false)
            .with_deletable(false)
            .with_connectable(false)
            .with_source_position(HandlePosition::Bottom)
            .with_target_position(HandlePosition::Top);
            node.content = node
                .content
                .with_border_style(Style::default().fg(MUTED))
                .with_text_style(Style::default().fg(INK))
                .with_background(if item.kind == "DECISION" {
                    WARM
                } else {
                    CONTEXT
                })
                .with_selected_border_style(Style::default().fg(ACCENT))
                .with_selected_background(SELECTED);
            node
        })
        .collect();
    let edges: Vec<Edge<StepEdge>> = vec![
        Edge::new("waits-on", "objective:ui", "review:ui"),
        Edge::new("needs-decision", "review:ui", "decision:history"),
        Edge::new("parallel-work", "objective:ui", "build:prototype"),
        Edge::new("produces", "build:prototype", "evidence:resize"),
    ];
    let mut palette = Theme::Dark.palette();
    palette.canvas_bg = BG;
    palette.surface = CONTEXT;
    palette.muted = MUTED;
    palette.accent = ACCENT;
    palette.text = INK;
    let mut flow = Flow::with_graph(nodes, edges)
        .expect("static fixture graph is valid")
        .with_theme(Theme::Custom(palette))
        .with_min_zoom(0.5)
        .with_max_zoom(1.5);
    flow.select_node(items[selected].id);
    flow.request_fit_view();
    flow
}
