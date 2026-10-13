//! Invented projection data, not a schema for an engine service.
use crate::presentation::{Entry, EntryKind};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Live,
    Historical,
}
impl Scope {
    pub fn label(self) -> &'static str {
        match self {
            Self::Live => "LIVE FIXTURE / main @ r18",
            Self::Historical => "HISTORICAL FIXTURE / approach-a @ r11",
        }
    }
}
#[derive(Clone, Copy)]
pub struct Work {
    pub id: &'static str,
    pub title: &'static str,
    pub kind: &'static str,
    pub owner: &'static str,
    pub status: &'static str,
    pub summary: &'static str,
    pub wait: &'static str,
    pub dependency: Option<usize>,
    pub evidence: &'static str,
    pub history: &'static str,
    pub position: (f64, f64),
}
pub fn work(scope: Scope) -> Vec<Work> {
    let mut items = vec![
        Work {
            id: "objective:ui",
            title: "Supervise work",
            kind: "OBJECTIVE",
            owner: "planner:main",
            status: "WAITING",
            summary: "Make objectives, progress, decisions, and intervention understandable across providers.",
            wait: "Review is waiting on a human scope decision.",
            dependency: Some(1),
            evidence: "fixture://objective/ui/r18\nOwner: objective projection\nObserved: waiting on review:ui\nNo completion estimate available.",
            history: "r11  Explore embedded UI\nr14  Separate UI process\nr18  Wait for review scope",
            position: (2.0, 1.0),
        },
        Work {
            id: "review:ui",
            title: "Interaction review",
            kind: "REVIEW",
            owner: "reviewer:main",
            status: "BLOCKED",
            summary: "Review the graph-to-conversation journey and verify that drafts retain their recipient.",
            wait: "Decision: include historical actions in this prototype?",
            dependency: Some(2),
            evidence: "fixture://review/ui/r7\nCandidate: fixture-candidate-b\nFinding F1: historical context must not imply live authority.\nReview outcome: unresolved.",
            history: "r3  Review requested\nr5  Finding F1 recorded\nr7  Scope question raised",
            position: (2.0, 8.0),
        },
        Work {
            id: "decision:history",
            title: "History action scope",
            kind: "DECISION",
            owner: "reviewer:main",
            status: "NEEDS YOU",
            summary: "Should historical views allow new actions? Suggested scope: inspect history, then explicitly return to live work.",
            wait: "Human decision needed. No approval operation is wired in this prototype.",
            dependency: None,
            evidence: "fixture://decision/history/r2\nRaised by: reviewer:main\nAuthority: human operator\nRecommendation is not an accepted decision.",
            history: "r1  Question opened\nr2  Reviewer added a recommendation\nNo decision receipt exists.",
            position: (2.0, 15.0),
        },
        Work {
            id: "build:prototype",
            title: "Terminal prototype",
            kind: "BUILD",
            owner: "builder:main",
            status: "ACTIVE",
            summary: "Exercise responsive layout, graph navigation, and a persistent conversation composer.",
            wait: "No dependency wait. Running in parallel with review.",
            dependency: Some(4),
            evidence: "fixture://build/prototype/r9\nCandidate: fixture-candidate-b\nProvider: adapter A (fixture)\nUsage and cost: unavailable.",
            history: "r4  Shell layout assembled\nr7  Graph selection connected\nr9  Resize checks in progress",
            position: (36.0, 8.0),
        },
        Work {
            id: "evidence:resize",
            title: "Resize evidence",
            kind: "EVIDENCE",
            owner: "builder:main",
            status: "AVAILABLE",
            summary: "A sample evidence reference. Inspect exact candidates and observation limits before drawing conclusions.",
            wait: "No wait. This is invented evidence, not a test result.",
            dependency: None,
            evidence: "fixture://evidence/resize/r3\nSubject: fixture-candidate-b\nClaim: draft retained at 80x24\nClass: sample observed evidence\nNot evidence about production behavior.",
            history: "r1  Observation requested\nr2  Sample observation attached\nr3  Limitations recorded",
            position: (36.0, 15.0),
        },
    ];
    if scope == Scope::Historical {
        items[0].status = "SUPERSEDED";
        items[0].summary = "Earlier approach: embed the interface in a provider-specific runtime. Superseded by the standalone client direction.";
        items[0].wait = "Historical snapshot. No current wait can be inferred.";
        items[0].evidence = "fixture://objective/ui/r11\nBranch: approach-a\nHistorical cutoff: sample 09:20\nSuperseded by main r14.";
        items[0].history =
            "r8  Embedded approach proposed\nr11  Candidate saved\nLater: superseded on main r14";
        for item in &mut items[1..] {
            item.status = "ARCHIVED";
            item.summary = "Archived work on the embedded UI approach. This snapshot is separate from the current main branch.";
            item.wait = "Historical record; live operations unavailable.";
            item.evidence = "fixture://branch/approach-a/r11\nCandidate: fixture-candidate-a\nProvider: adapter B (fixture)\nPartial historical evidence; no live state implied.";
            item.history = "09:05  Branch created\n09:12  Embedded candidate explored\n09:20  Snapshot retained";
        }
    }
    items
}
pub fn timeline(scope: Scope) -> Vec<Entry> {
    if scope == Scope::Historical {
        return vec![
            Entry {
                kind: EntryKind::Message,
                subject: "objective:ui",
                speaker: "planner:main",
                recipient: None,
                text: "Explore the embedded UI approach. [historical fixture]".into(),
                detail: 0,
            },
            Entry {
                kind: EntryKind::Message,
                subject: "review:ui",
                speaker: "reviewer:main",
                recipient: None,
                text: "The client would inherit runtime lifetime and provider assumptions.".into(),
                detail: 0,
            },
            Entry {
                kind: EntryKind::Message,
                subject: "build:prototype",
                speaker: "builder:main",
                recipient: None,
                text: "Saved fixture-candidate-a; adapter B. No current runtime attached.".into(),
                detail: 1,
            },
            Entry {
                kind: EntryKind::Message,
                subject: "objective:ui",
                speaker: "evidence",
                recipient: None,
                text: "fixture://branch/approach-a/r11 — partial retained snapshot".into(),
                detail: 2,
            },
        ];
    }
    vec![
        Entry { kind: EntryKind::Message, recipient: None, subject:"objective:ui", speaker:"planner:main", text:"The standalone UI should make work and intervention understandable.".into(), detail:0 },
        Entry { kind: EntryKind::Message, recipient: None, subject:"build:prototype", speaker:"builder:main", text:"Graph and context panel are being explored in parallel with review.".into(), detail:1 },
        Entry { kind: EntryKind::Message, recipient: None, subject:"review:ui", speaker:"reviewer:main", text:"The waiting review needs a decision about historical actions.".into(), detail:0 },
        Entry { kind: EntryKind::Message, recipient: None, subject:"decision:history", speaker:"reviewer:main", text:"## History stays inspect-only\n\n- **Return to live context** before taking action.\n- Keep `recipient` separate from the selected subject.\n- Inspect the [decision context](fixture://decision/history).\n\n```rust\n// Illustrative UI rule, not engine authority.\nlet can_send = scope == Scope::Live;\n```\n\nThis is a recommendation, not a decision.".into(), detail:0 },
        Entry { kind: EntryKind::Message, recipient: None, subject:"review:ui", speaker:"evidence", text:"fixture://review/ui/r7 | candidate-b | unresolved F1 | observed sample".into(), detail:2 },
    ]
}
