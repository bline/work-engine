//! Presentation fixtures, not an adapter or a canonical lifecycle schema.
//! Positive sequence: shared board 1119; meaning/limits: NATIVE_VERTICAL.md.
use crate::{model::Model, theme::*};
use tuirealm::ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Wrap},
};

pub const CASES: [&str; 7] = [
    "Reported handoff",
    "Missing completion evidence",
    "Contradictory evidence",
    "Restart after entry",
    "Stale context command",
    "Interaction request rejected",
    "Readiness acknowledgment missing",
];
pub struct Step {
    pub title: &'static str,
    pub context: &'static str,
    pub native: &'static str,
    pub outcome: &'static str,
    pub explanation: &'static str,
    pub next: &'static str,
}
pub fn steps(case: usize) -> Vec<Step> {
    if case == 0 {
        vec![
            Step {
                title: "Input 1 handled",
                context: "A / epoch 1",
                native: "Completed reported",
                outcome: "ACCEPTED · TEXT",
                explanation: "A completed the first input. The reported controlled run used a domain-specific text receipt; native Completed alone is insufficient.",
                next: "Capture continuation package.",
            },
            Step {
                title: "Continuation package captured",
                context: "A → B",
                native: "Not a native turn",
                outcome: "CAPTURED",
                explanation: "The captured package carries continuation material. Possession of a package does not activate successor authority or prove understanding.",
                next: "Successor must acknowledge the exact package.",
            },
            Step {
                title: "Successor acknowledges package",
                context: "B / epoch 2",
                native: "Completed reported",
                outcome: "ACKNOWLEDGED",
                explanation: "B's readiness turn acknowledged the package. Readiness is its own coordinator-owned effect, distinct from delivering input 2.",
                next: "Activation still requires current package and explicit adequacy attestation.",
            },
            Step {
                title: "Successor activated",
                context: "B / epoch 2",
                native: "Not a native turn",
                outcome: "ACTIVE",
                explanation: "The handoff sequence includes successor activation before its next input. This is a logical ordering view, not a timestamped event log.",
                next: "Only successor authority may admit the next input.",
            },
            Step {
                title: "Input 2 handled once",
                context: "B / epoch 2",
                native: "Completed reported",
                outcome: "ACCEPTED · TEXT",
                explanation: "The report records one local dispatch for input 2 on B, after readiness, under the same coordinator. This is not a network-wide exactly-once claim.",
                next: "Command integration and live-provider validation remain pending.",
            },
        ]
    } else {
        let (title, native, outcome, explanation, next) = match case {
            1 => (
                "Terminal status without exact final evidence",
                "Completed",
                "UNCERTAIN",
                "A native terminal status is present, but exact identity-bound final text/readback is missing. No accepted text completion can be inferred.",
                "Later input stays blocked. Await authoritative evidence; do not retry automatically.",
            ),
            2 => (
                "Finalized evidence contradicts prior evidence",
                "Completed / conflicting content",
                "CONFLICT",
                "Contradictory finalized content must remain visible and fence further work. The UI must not replace it with the latest green status.",
                "Context fenced; explicit reconciliation required. Previously performed effects are not undone.",
            ),
            3 => (
                "Coordinator restarts after entry",
                "Unavailable after restart",
                "UNKNOWN",
                "Local certainty was lost after an operation entered. A restart marker is not proof of success, failure, or absence of a remote effect.",
                "No automatic resend. Keep later input blocked pending exact reconciliation.",
            ),
            4 => (
                "Old context attempts a new command",
                "No new turn implied",
                "REJECTED",
                "The command uses stale context authority. Rejection is a command result, not failure of an already admitted effect.",
                "Do not dispatch this request. Existing in-flight work retains its own outcome.",
            ),
            5 => (
                "Unsupported interaction requested",
                "May later report Completed",
                "BLOCKED / UNKNOWN",
                "An interaction request cannot satisfy bounded text completion. Request-ID RPCs are denied; an async question is a durable violation, even if native Completed follows.",
                "Do not forward a question or invent an answer. Keep this turn unsettled.",
            ),
            _ => (
                "Successor has not acknowledged the package",
                "Readiness not verified",
                "AWAITING EVIDENCE",
                "A package exists, but its identity-bound readiness acknowledgment is missing. Captured content alone cannot activate the successor.",
                "Successor remains inactive; input 2 stays blocked until readiness and activation requirements are met.",
            ),
        };
        vec![Step {
            title,
            context: "ILLUSTRATIVE · not a captured run",
            native,
            outcome,
            explanation,
            next,
        }]
    }
}
pub fn main(frame: &mut Frame, area: Rect, m: &mut Model) {
    let b = crate::ui::block("Lifecycle · read-only · Ctrl-P scenario", !m.show_context);
    let inner = b.inner(area);
    frame.render_widget(b, area);
    let [intro, list, limits] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .areas(inner);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(CASES[m.lifecycle_case], Style::default().fg(ACCENT)),
            Line::raw(if m.lifecycle_case == 0 {
                "Reported sequence · parent validation pending"
            } else {
                "DESIGN FIXTURE · failure-state illustration"
            }),
            Line::styled(
                if m.lifecycle_case == 0 {
                    "Real CLI · simulated inference · NO LIVE CONNECTION"
                } else {
                    "Illustrative contract state · NO LIVE CONNECTION"
                },
                Style::default().fg(AMBER),
            ),
        ])
        .wrap(Wrap { trim: false }),
        intro,
    );
    let steps = steps(m.lifecycle_case);
    let rows = steps
        .iter()
        .enumerate()
        .map(|(i, s)| {
            ListItem::new(vec![
                Line::from(vec![Span::styled(
                    format!("{}  {}", i + 1, s.title),
                    Style::default().fg(INK),
                )]),
                Line::styled(
                    format!("   {} · {}", s.context, s.outcome),
                    Style::default().fg(if m.lifecycle_case == 0 { ACCENT } else { AMBER }),
                ),
                Line::raw(""),
            ])
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(Some(m.lifecycle_selected));
    frame.render_stateful_widget(
        List::new(rows)
            .highlight_symbol("› ")
            .highlight_style(Style::default().bg(SELECTED)),
        list,
        &mut state,
    );
    for (row, index) in (state.offset()..steps.len()).enumerate() {
        let y = list.y + row as u16 * 3;
        if y < list.bottom() {
            m.mouse.items.push((
                Rect::new(list.x, y, list.width, 3.min(list.bottom() - y)),
                index,
            ));
        }
    }
    frame.render_widget(Paragraph::new(if m.lifecycle_case == 0 {"One coordinator across A → B · 3 reported simulated turns\nNative Completed ≠ accepted lifecycle completion\nCtrl-P: inspect step / scenario / panel context"} else {"DESIGN FIXTURE · not a captured failure trace\nNative Completed ≠ accepted lifecycle completion\nCtrl-P: inspect step / scenario / panel context"}).style(Style::default().fg(MUTED)).wrap(Wrap {trim:false}),limits);
}
pub fn context(frame: &mut Frame, area: Rect, m: &mut Model) {
    let steps = steps(m.lifecycle_case);
    let step = &steps[m.lifecycle_selected.min(steps.len() - 1)];
    let b = crate::ui::block("Lifecycle / evidence & limits", m.show_context)
        .style(Style::default().bg(CONTEXT).fg(INK));
    let inner = b.inner(area);
    let lines = vec![
        Line::styled(step.title, Style::default().fg(ACCENT)),
        Line::raw(step.context),
        Line::raw(""),
        Line::styled("NATIVE OBSERVATION", Style::default().fg(LAVENDER)),
        Line::raw(step.native),
        Line::raw(""),
        Line::styled("LIFECYCLE OUTCOME", Style::default().fg(LAVENDER)),
        Line::raw(step.outcome),
        Line::raw(step.explanation),
        Line::raw(""),
        Line::styled("WHAT HAPPENS NEXT", Style::default().fg(AMBER)),
        Line::raw(step.next),
        Line::raw(""),
        Line::styled("PROVENANCE", Style::default().fg(LAVENDER)),
        Line::raw(if m.lifecycle_case == 0 {
            "Shared board #1119 · worker-reported milestone. Parent validation pending at that report."
        } else {
            "Authored design scenario informed by the lifecycle contract. Not a captured failure trace or claim that a test passed."
        }),
        Line::raw("experiments/context-lifecycle-rust/NATIVE_VERTICAL.md"),
        Line::raw(""),
        Line::raw(
            "No live provider. No native commands wired. Process exit does not establish settlement.",
        ),
    ];
    let text = Paragraph::new(lines).wrap(Wrap { trim: false });
    m.context_scroll = m.context_scroll.min(
        text.line_count(inner.width)
            .saturating_sub(inner.height as usize)
            .min(u16::MAX as usize) as u16,
    );
    frame.render_widget(text.scroll((m.context_scroll, 0)).block(b), area);
}
