//! UI-owned display values. Not runtime DTOs, authority, or lifecycle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemState {
    Recorded,
    Accepted,
    Rejected,
    Progress,
    Ready,
    Uncertain,
}
impl SystemState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Recorded => "RECORDED",
            Self::Accepted => "TEXT ACCEPTED",
            Self::Rejected => "REJECTED",
            Self::Progress => "IN PROGRESS",
            Self::Ready => "READY",
            Self::Uncertain => "UNCERTAIN",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryKind {
    Message,
    Recorded {
        details: String,
    },
    System {
        state: SystemState,
        context: &'static str,
        details: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub kind: EntryKind,
    pub recipient: Option<&'static str>,
    pub subject: &'static str,
    pub speaker: &'static str,
    pub text: String,
    pub detail: usize,
}

impl Entry {
    pub fn display_text(&self, expanded: bool) -> std::borrow::Cow<'_, str> {
        if expanded
            && let EntryKind::System { details, .. } | EntryKind::Recorded { details } = &self.kind
        {
            format!("{}\n\n{}", self.text, details).into()
        } else {
            self.text.as_str().into()
        }
    }
}

/// Local dataset identity, not a service cursor, revision or provider identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceId {
    WorkFixture(crate::fixture::Scope),
    ChatExample,
    Task(crate::task::Case, usize),
    Archive(&'static str),
    Contract(crate::contract::Case),
    #[allow(dead_code)]
    Client(u64),
}

/// One selected conversation feeds rendering, event lookup and source copying.
/// Owned rows let source adapters remain independent of renderer lifetimes.
#[derive(Clone)]
pub struct Conversation {
    pub source: SourceId,
    pub entries: Vec<Entry>,
    pub usage: String,
    pub banner: String,
    pub simulated_activity: bool,
    pub context: Option<(&'static str, String)>,
}
impl Conversation {
    pub fn work(
        scope: crate::fixture::Scope,
        entries: Vec<Entry>,
        usage: String,
        banner: String,
    ) -> Self {
        Self {
            source: SourceId::WorkFixture(scope),
            context: None,
            entries,
            usage: if scope == crate::fixture::Scope::Historical {
                "historical usage unavailable".into()
            } else {
                usage
            },
            banner,
            simulated_activity: scope == crate::fixture::Scope::Live,
        }
    }
    pub fn example(usage: String) -> Self {
        Self {
            source: SourceId::ChatExample,
            context: Some(("Chat / lifecycle context", "ILLUSTRATIVE CHAT\n\nAgent output and typed lifecycle system events share the timeline.\n\nF3 expands system-event details.\n\nContext B usage is unavailable. There is no runtime measurement or live subscription.\n\nThe uncertain ending is invented to exercise the UI; it is not the controlled run's outcome.\n\nCommand conversation returns to the work fixture. Drafts remain retained.".into())),
            entries: crate::chat::handoff_example(),
            usage,
            banner: "ILLUSTRATIVE CHAT · not captured output · F3 expands system events".into(),
            simulated_activity: false,
        }
    }
    pub fn archive(replay: &crate::replay::Replay) -> Self {
        Self {
            source: SourceId::Archive(replay.label),
            context: Some(("Historical evidence / provenance", replay.context.clone())),
            entries: replay.entries.clone(),
            usage: "historical usage unavailable".into(),
            banner: format!(
                "{} · F3 source details · NOT a running engine",
                replay.label
            ),
            simulated_activity: false,
        }
    }
    pub fn visible(&self, detail: usize) -> impl Iterator<Item = (usize, &Entry)> {
        self.entries
            .iter()
            .enumerate()
            .filter(move |(_, entry)| entry.detail <= detail)
    }
}
