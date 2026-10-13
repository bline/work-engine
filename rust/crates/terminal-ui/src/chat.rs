//! Presentation state only. No runtime subscription or lifecycle authority.
use crate::presentation::{Entry, EntryKind, SystemState};

#[derive(Default)]
pub struct ContextUsage {
    pub active: Option<&'static str>,
    pub measurement: Option<Measurement>,
}
pub struct Measurement {
    pub context: &'static str,
    pub used: u64,
    pub capacity: u64,
}
impl ContextUsage {
    pub fn replace(&mut self, context: &'static str) {
        self.active = Some(context);
        self.measurement = None;
    }
    pub fn label(&self) -> String {
        let name = self.active.unwrap_or("unbound");
        match &self.measurement {
            Some(m) if Some(m.context) == self.active && m.capacity > 0 && m.used <= m.capacity => {
                format!(
                    "Context {name} · {}% used",
                    u128::from(m.used) * 100 / u128::from(m.capacity)
                )
            }
            _ => format!("Context {name} · usage unavailable"),
        }
    }
}

/// Authored mixed-message layout example. These are not captured agent outputs.
pub fn handoff_example() -> Vec<Entry> {
    let message = |speaker, text: &str| Entry {
        kind: EntryKind::Message,
        recipient: None,
        subject: "chat:handoff-example",
        speaker,
        text: text.into(),
        detail: 0,
    };
    let system = |context, state, text: &str, details: &str| Entry {
        kind: EntryKind::System {
            context,
            state,
            details: details.into(),
        },
        recipient: None,
        subject: "chat:handoff-example",
        speaker: "system",
        text: text.into(),
        detail: 0,
    };
    vec![
        message(
            "agent / context A",
            "I have reviewed the first input. The continuation needs to retain **the objective, commitments, and unresolved work**.",
        ),
        system(
            "A",
            SystemState::Progress,
            "Capturing continuation package…",
            "ILLUSTRATIVE EVENT · Capture is in progress. A package alone does not establish successor readiness or activation. No runtime connected.",
        ),
        system(
            "A → B",
            SystemState::Ready,
            "Context handoff · B acknowledged the package and is active.",
            "ILLUSTRATIVE EVENT · This example combines acknowledgment and subsequent activation for display. A real projection must supply both accepted facts separately. Usage for A must not carry into B.",
        ),
        message(
            "agent / context B",
            "Continuing with the retained package. I’ll handle the second input here.\n\n```rust\nlet next_context = \"B\";\n```",
        ),
        system(
            "B",
            SystemState::Uncertain,
            "Completion uncertain · awaiting exact final evidence.",
            "ILLUSTRATIVE ALTERNATE ENDING · Native Completed alone is insufficient. Later input stays blocked; no automatic retry. This is not the successful controlled-run report or a captured failure trace.",
        ),
    ]
}
