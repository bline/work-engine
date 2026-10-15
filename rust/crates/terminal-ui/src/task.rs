//! Shared task presentation, fed by labeled fixtures or passive observations.
use crate::presentation::{Conversation, Entry, EntryKind, SourceId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    Pending,
    Result,
    Stale,
    Refused,
}
impl Case {
    pub const ALL: [Self; 4] = [Self::Pending, Self::Result, Self::Stale, Self::Refused];
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Result => "result",
            Self::Stale => "stale",
            Self::Refused => "refused",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.label() == s)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Present,
    Absent,
    NotRequested,
    NotApplicable,
    Unavailable,
    Refused,
    Unknown,
}
impl Availability {
    pub fn label(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Absent => "absent",
            Self::NotRequested => "not_requested",
            Self::NotApplicable => "not_applicable",
            Self::Unavailable => "unavailable",
            Self::Refused => "refused",
            Self::Unknown => "unknown",
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct Component {
    pub name: &'static str,
    pub availability: Availability,
    pub stale: bool,
    pub binding: String,
    pub detail: String,
}
impl Component {
    pub fn summary(&self) -> String {
        format!(
            "{}: {}{}",
            self.name,
            self.availability.label(),
            if self.stale {
                " · STALE / last known"
            } else {
                ""
            }
        )
    }
}
#[derive(Clone, PartialEq)]
pub struct Node {
    pub id: std::borrow::Cow<'static, str>,
    pub title: String,
    pub parent: Option<String>,
    pub text: Option<String>,
    pub source: String,
    pub component: usize,
}
pub struct Task {
    pub live: Option<crate::task_live::Metadata>,
    pub case: Case,
    pub objective: String,
    pub execution: String,
    pub question_pending: bool,
    pub selected: usize,
    pub components: Vec<Component>,
    pub nodes: Vec<Node>,
}
impl Task {
    pub fn fixture(case: Case) -> Self {
        use Availability::*;
        let stale = case == Case::Stale;
        let result = matches!(case, Case::Result | Case::Stale);
        let components = vec![
            Component {
                name: "Owner",
                availability: if stale { Unavailable } else { Present },
                stale,
                binding: "task:ui-17 / subject:fixture-17 / owner revision 7".to_owned(),
                detail: "Last read: fixture receipt A (local receipt, not event time). Objective: inspect task evidence without implying completion. Kind: engineering. Snapshot: fixture://scope/17. Cursor revision: owner 3.".to_owned(),
            },
            Component {
                name: "Lifecycle",
                availability: Present,
                stale: false,
                binding: "subject:fixture-17 / delivery:d1 / stream:fixture / cursor 12".to_owned(),
                detail: (if result {
                    "Last read: fixture receipt B. Delivery unsettled; physical execution stopped. Admission closed. Published result does not establish settlement."
                } else {
                    "Last read: fixture receipt B. Delivery active; physical execution running. Admission open. Pending question does not establish a halt."
                }).to_owned(),
            },
            Component {
                name: "Native segments",
                availability: if case == Case::Refused {
                    Refused
                } else if stale {
                    Unavailable
                } else {
                    Present
                },
                stale,
                binding: "subject:fixture-17 / delivery:d1; native revision not supplied".to_owned(),
                detail: (if case == Case::Refused {
                    "Explicit fixture reason: capacity (more than 64 segments). No collection supplied; not empty. This typed refusal is proposed, not inferred from current v1 unavailable."
                } else if stale {
                    "Read failed: owner_unavailable. Last-known segments retained from fixture receipt A; completeness now unknown."
                } else {
                    "Two supplied segments for delivery:d1, sequence 1 then 2. No shared cursor or cross-component event chronology."
                }).to_owned(),
            },
            Component {
                name: "Command",
                availability: NotRequested,
                stale: false,
                binding: "no command target".to_owned(),
                detail: "No command selected; no result lookup performed.".to_owned(),
            },
            Component {
                name: "Control",
                availability: NotApplicable,
                stale: false,
                binding: "fixture owner declaration".to_owned(),
                detail: "Control observation does not apply to this fixture.".to_owned(),
            },
            Component {
                name: "Legacy auxiliary",
                availability: Unknown,
                stale: false,
                binding: "legacy null".to_owned(),
                detail: "Null does not establish absence, refusal, or successful lookup.".to_owned(),
            },
            Component {
                name: "Published result",
                availability: if result { Present } else { Absent },
                stale,
                binding: "task:ui-17 / owner revision 7".to_owned(),
                detail: (if result {
                    "Attributed to fixture task owner; source fixture://result/r1. Independent of lifecycle settlement."
                } else if stale {
                    "Last-known successful owner lookup had no published result. Current owner read unavailable; absence is stale."
                } else {
                    "Owner lookup established no published result at revision 7."
                }).to_owned(),
            },
        ];
        let mut nodes = vec![
            Node {
                id: "task:ui-17".into(),
                title: "Task · Inspect task evidence".to_owned(),
                parent: None,
                text: None,
                source: "fixture://scope/17".to_owned(),
                component: 0,
            },
            Node {
                id: "input:i1".into(),
                title: "Inputs · owner-published".to_owned(),
                parent: Some("task:ui-17".into()),
                text: Some(str::to_owned(
                    "## Inputs\n\nInspect the retained task and preserve the distinction between a published result and settled execution.",
                )),
                source: "fixture://input/i1".to_owned(),
                component: 0,
            },
            Node {
                id: "question:q1".into(),
                title: (if result {
                    "Question q1 · answered"
                } else {
                    "Question q1 · pending"
                })
                .to_owned(),
                parent: Some("task:ui-17".into()),
                text: Some(str::to_owned(if result {
                    "## Questions and answers\n\n**Question q1** · recipient operator · scope task:ui-17\nMay the result be published before lifecycle settlement?\n\n**Recorded answer** · operator\nYes. Keep settlement visible as a separate observation."
                } else {
                    "## Questions\n\n**Pending q1** · recipient operator · scope task:ui-17\nMay the result be published before lifecycle settlement?\n\nNo answer is recorded. Execution may continue independently."
                })),
                source: "fixture://question/q1".to_owned(),
                component: 0,
            },
            Node {
                id: "effect:e1".into(),
                title: "Effect e1 · recorded consequence".to_owned(),
                parent: Some("task:ui-17".into()),
                text: None,
                source: "fixture://receipt/e1 · effect:e1 / delivery:d1 / digest:fixture-only"
                    .to_owned(),
                component: 0,
            },
            Node {
                id: "delivery:d1".into(),
                title: "Delivery d1 · lifecycle".to_owned(),
                parent: Some("task:ui-17".into()),
                text: None,
                source: "fixture://lifecycle/d1".to_owned(),
                component: 1,
            },
        ];
        if case != Case::Refused {
            nodes.extend([
                Node {
                    id: "segment:n1".into(),
                    title: "Segment n1 · seq 1 · stopped · context A".to_owned(),
                    parent: Some("delivery:d1".into()),
                    text: None,
                    source: "fixture://native/n1".to_owned(),
                    component: 2,
                },
                Node {
                    id: "segment:n2".into(),
                    title: "Segment n2 · seq 2 · context B".to_owned(),
                    parent: Some("delivery:d1".into()),
                    text: None,
                    source: "fixture://native/n2".to_owned(),
                    component: 2,
                },
            ]);
        }
        if result {
            nodes.push(Node { id: "result:r1".into(), title: "Result r1 · published, settlement independent".to_owned(), parent: Some("task:ui-17".into()), text: Some(str::to_owned("## Published result\n\nThe task evidence is ready for inspection. Lifecycle delivery remains unsettled.\n\nAttribution: fixture task owner. Source: fixture://result/r1.")), source: "fixture://result/r1".to_owned(), component:6 });
        }
        Self {
            live: None,
            case,
            objective: "Inspect task evidence without implying completion.".to_owned(),
            execution: (if result {
                "Delivery unsettled · execution stopped · admission closed"
            } else {
                "Delivery active · execution running · admission open"
            })
            .to_owned(),
            question_pending: !result,
            selected: 0,
            components,
            nodes,
        }
    }
    pub fn selected_node(&self) -> &Node {
        &self.nodes[self.selected]
    }
    pub fn identity(&self) -> String {
        if let Some(live) = &self.live {
            return format!(
                "Inspecting: {} · {} · read-only",
                self.selected_node().caption(),
                live.request.task_id
            );
        }
        format!(
            "Inspecting: {} · task:ui-17 · read-only",
            self.selected_node().caption()
        )
    }
    pub fn overview(&self) -> Vec<SummarySection> {
        if self.live.is_some() {
            return crate::task_live::overview(self);
        }
        let owner = &self.components[0];
        let native = &self.components[2];
        let result = &self.components[6];
        let mut attention = Vec::new();
        if owner.stale {
            attention.push("Owner read unavailable · showing last-known owner content.".to_owned());
        }
        if self.question_pending {
            attention.push(
                "Pending question · May the result be published before lifecycle settlement?"
                    .to_owned(),
            );
        } else if !owner.stale {
            attention.push("No pending questions in the owner observation.".to_owned());
        }
        if native.availability == Availability::Refused {
            attention
                .push("Native history refused: capacity limit. No collection supplied.".to_owned());
        } else if native.stale {
            attention.push("Native history unavailable · last-known segments retained.".to_owned());
        }
        let result_node = self.nodes.iter().position(|n| n.component == 6);
        let result_body = if result.stale {
            "Last-known publication · STALE\nThe task evidence is ready for inspection."
        } else if result.availability == Availability::Present {
            "Published by the task owner\nThe task evidence is ready for inspection."
        } else {
            "No result published at this owner read."
        };
        vec![
            SummarySection {
                title: "OBJECTIVE",
                body: if owner.stale {
                    format!("Last known · {}", self.objective)
                } else {
                    self.objective.clone()
                },
                node: Some(0),
                warning: false,
            },
            SummarySection {
                title: "NEEDS ATTENTION",
                body: attention.join("\n"),
                node: if self.question_pending { Some(2) } else { None },
                warning: self.question_pending
                    || owner.stale
                    || native.availability == Availability::Refused,
            },
            SummarySection {
                title: "LATEST RESULT",
                body: result_body.into(),
                node: result_node,
                warning: result.stale,
            },
            SummarySection {
                title: "EXECUTION",
                body: format!(
                    "{}\nExecution does not establish task completion.",
                    self.execution
                ),
                node: self.nodes.iter().position(|n| n.id == "delivery:d1"),
                warning: false,
            },
        ]
    }
    pub fn details(&self) -> String {
        if self.live.is_some() {
            return crate::task_live::details(self);
        }
        let n = self.selected_node();
        let c = &self.components[n.component];
        let describe = |c: &Component| format!("{}\n{}\n{}", c.summary(), c.binding, c.detail);
        let details = if self.selected == 0 {
            self.components
                .iter()
                .map(describe)
                .collect::<Vec<_>>()
                .join("\n\n")
        } else {
            describe(c)
        };
        format!(
            "{}\nTask: task:ui-17\nSubject: fixture-17\nHost: fixture host / incarnation A\nEndpoint: fixture only\n\nSource: {}\n\nIndependent component reads; non-atomic. Revisions belong to each component. No native cursor supplied.\n\n{}\n\nPROVISIONAL TASK FIXTURE\nNot an accepted host protocol.",
            n.title, n.source, details
        )
    }
    pub fn conversation(&self) -> Conversation {
        let mut entries: Vec<_> = self
            .nodes
            .iter()
            .filter(|n| self.selected == 0 || n.id == self.selected_node().id)
            .filter_map(|n| {
                n.text.as_deref().map(|text| Entry {
                    kind: EntryKind::Recorded {
                        details: format!(
                            "Node: {}\nSource: {}\n{}\n{}",
                            n.id,
                            n.source,
                            self.components[n.component].summary(),
                            self.components[n.component].binding
                        ),
                    },
                    recipient: None,
                    subject: match &n.id {
                        std::borrow::Cow::Borrowed(id) => id,
                        std::borrow::Cow::Owned(_) => "task publication",
                    },
                    speaker: if self.live.is_some() {
                        "TASK OWNER"
                    } else {
                        "PROVISIONAL TASK FIXTURE"
                    },
                    text: if self.components[n.component].stale {
                        format!("STALE / LAST KNOWN owner content\n\n{text}")
                    } else {
                        text.into()
                    },
                    detail: 0,
                })
            })
            .collect();
        if entries.is_empty() {
            entries.push(Entry { kind:EntryKind::Message, recipient:None, subject:"task inspection", speaker:"INSPECTION", text:"No attributable conversation text for this node. Inspect its source and component state in Context.".into(), detail:0 });
        }
        Conversation {
            source: self
                .live
                .as_ref()
                .map_or(SourceId::Task(self.case, self.selected), |live| {
                    SourceId::LiveTask(live.revision, self.selected)
                }),
            entries,
            usage: "task groups · usage unavailable".into(),
            banner: "Inputs / questions / result grouped; NOT a chronological transcript".into(),
            simulated_activity: false,
            context: Some(("Task / component evidence", self.details())),
        }
    }
}

/// Human-facing summaries of the same fixture projection; technical bindings stay in Context.
pub struct SummarySection {
    pub title: &'static str,
    pub body: String,
    pub node: Option<usize>,
    pub warning: bool,
}
impl Node {
    pub fn caption(&self) -> &str {
        if self.component == 0 && self.parent.is_none() {
            "Inspect task evidence"
        } else {
            self.title.split(" · ").next().unwrap_or(&self.title)
        }
    }
    pub fn status(&self) -> &str {
        if self.component == 0 && self.parent.is_none() {
            return "Task · read-only";
        }
        self.title
            .split_once(" · ")
            .map_or("Task", |(_, rest)| rest)
    }
}
