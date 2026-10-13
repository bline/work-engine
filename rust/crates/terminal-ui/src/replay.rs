//! UI-owned offline archive. This is not a runtime wire protocol.
use crate::presentation::{Entry, EntryKind, SystemState};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const ARCHIVE: &str = include_str!("../replays/live-proof-2026-10-07.v1.json");
#[derive(Clone, Copy)]
pub enum Section {
    Live,
    Rejected,
    Validation,
}
impl Section {
    pub fn label(self) -> &'static str {
        match self {
            Self::Live => "Live proof · historical",
            Self::Rejected => "Rejected setup · separate attempt",
            Self::Validation => "Validation · tests & controlled probes",
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Archive {
    format: String,
    version: u32,
    recorded_at: String,
    scope: String,
    sources: BTreeMap<String, Source>,
    limitations: Vec<String>,
    live: Vec<Record>,
    rejected: Vec<Record>,
    validation: Vec<Record>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    path: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Agent,
    Input,
    System,
    Accepted,
    Rejected,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    kind: Kind,
    context: String,
    text: String,
    details: String,
    refs: Vec<String>,
}
pub struct Replay {
    pub entries: Vec<Entry>,
    pub context: String,
    pub label: &'static str,
}
impl Replay {
    pub fn load(input: &str, section: Section) -> Result<Self, String> {
        let archive: Archive =
            serde_json::from_str(input).map_err(|e| format!("Invalid historical archive: {e}"))?;
        if archive.version != 1 || archive.format != "work-engine-ui-history" {
            return Err("Unsupported historical archive version or format".into());
        }
        let records = match section {
            Section::Live => archive.live,
            Section::Rejected => archive.rejected,
            Section::Validation => archive.validation,
        };
        if records.is_empty() {
            return Err("Historical archive section is empty".into());
        }
        let context = format!(
            "{}\n\nArchive recorded: {}\n{}\n\nLogical sequence, not timestamped playback. No running engine connected. Context usage unavailable.\n\n{}\n\nSOURCES\n{}",
            section.label(),
            archive.recorded_at,
            archive.scope,
            archive.limitations.join("\n\n"),
            archive
                .sources
                .iter()
                .map(|(id, s)| format!("{id}: {}\nSHA-256 {}", s.path, s.sha256))
                .collect::<Vec<_>>()
                .join("\n\n")
        );
        let mut entries = Vec::new();
        for record in records {
            let event_context = match record.context.as_str() {
                "A" => "A",
                "B" => "B",
                "A → B" => "A → B",
                "setup" => "setup",
                "validation" => "validation",
                _ => return Err("Unknown historical context label".into()),
            };
            let mut refs = Vec::new();
            for reference in &record.refs {
                let (id, pointer) = reference.split_once('#').ok_or("Missing source pointer")?;
                let source = archive.sources.get(id).ok_or("Missing archive source")?;
                if source.sha256.len() != 64
                    || !source.sha256.bytes().all(|c| c.is_ascii_hexdigit())
                    || !pointer.starts_with('/')
                {
                    return Err("Invalid source hash or pointer".into());
                }
                refs.push(format!(
                    "{}#{pointer}\nSHA-256 {}",
                    source.path, source.sha256
                ));
            }
            if refs.is_empty() {
                return Err("Historical record has no provenance".into());
            }
            let details = format!("{}\n\n{}", record.details, refs.join("\n\n"));
            let (kind, speaker) = match record.kind {
                Kind::Agent => (
                    EntryKind::Recorded { details },
                    if event_context == "A" {
                        "agent / A · recorded"
                    } else {
                        "agent / B · recorded"
                    },
                ),
                Kind::Input => (EntryKind::Recorded { details }, "input / B · recorded"),
                Kind::System | Kind::Accepted | Kind::Rejected => (
                    EntryKind::System {
                        context: event_context,
                        details,
                        state: match record.kind {
                            Kind::Accepted => SystemState::Accepted,
                            Kind::Rejected => SystemState::Rejected,
                            _ => SystemState::Recorded,
                        },
                    },
                    "system / historical",
                ),
            };
            entries.push(Entry {
                kind,
                recipient: None,
                subject: "replay:2026-10-07",
                speaker,
                text: record.text,
                detail: 0,
            });
        }
        Ok(Self {
            entries,
            context,
            label: section.label(),
        })
    }
}
