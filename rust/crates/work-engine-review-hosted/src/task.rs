//! Task rendering: the versioned, digest-pinned instruction template followed by the contract
//! bytes verbatim.

use serde::{Deserialize, Serialize};
use work_engine_review_preparation::Subject;

use crate::revision::Profile;
use crate::{Refusal, sha256_hex};

/// The `openai-codex` profile's template (tool names as the Responses model sees them).
pub const TEMPLATE_ID: &str = "work-engine.review-hosted-task.v1";
pub const TEMPLATE: &str = include_str!("../templates/hosted-review-task-v1.txt");
/// SHA-256 of [`TEMPLATE`]. A changed template is a new template version.
pub const TEMPLATE_SHA256: &str =
    "7ecfbef93557b6770aeaad07c1c89d733426e994ab2427c350b5bff8e8f90718";
/// The `claude-sdk` profile's template: [`TEMPLATE`] with its own id and the tools named as
/// Claude Code presents them to the model (`mcp__pi__<name>`, `pi_coding_analysis.rs:119-121`).
pub const CLAUDE_TEMPLATE_ID: &str = "work-engine.review-hosted-task.claude-sdk.v1";
pub const CLAUDE_TEMPLATE: &str = include_str!("../templates/hosted-review-task-claude-sdk-v1.txt");
pub const CLAUDE_TEMPLATE_SHA256: &str =
    "68118bbdbaaab7c3ec1b53783c704de1c8eb65a885137576cb92481f1b080297";

/// `(id, bytes, pinned sha256)` of the profile's template.
pub fn template(profile: Profile) -> (&'static str, &'static str, &'static str) {
    match profile {
        Profile::OpenaiCodex => (TEMPLATE_ID, TEMPLATE, TEMPLATE_SHA256),
        Profile::ClaudeSdk => (CLAUDE_TEMPLATE_ID, CLAUDE_TEMPLATE, CLAUDE_TEMPLATE_SHA256),
    }
}

/// The Host refuses a prompt above this many bytes
/// (`pi-pressure-context-host/src/analysis.rs:26`, checked at `:74-75`, for every provider).
pub const MAX_PROMPT_BYTES: usize = 32_768;
/// Bytes the Host appends to the task: `"\n\n"` plus its fixed read-only sentence
/// (`analysis.rs:42-47`, the native turn prompt at `:186-197`, with no provider branch). Counted
/// for the bound; its content is checked against [`HOST_SUFFIX_SHA256`]. The suffix text is never
/// copied into the crate.
pub const HOST_SUFFIX_BYTES: usize = 202;
/// SHA-256 of the exact suffix bytes (`"\n\n"` plus the sentence), computed from the Host's format
/// string at `pi-pressure-context-host/src/analysis.rs:42-47` (the bytes after its `{}`). A test
/// recomputes it from that file, so a changed Host suffix fails there rather than silently.
pub const HOST_SUFFIX_SHA256: &str =
    "7edf1b84b4a5489015f0eee80e61cb3bf0f8c665272f6a3efed2a5e77673bbbd";
/// Host limits per job (`work-engine-manual-host/src/pi_coding_analysis.rs:11-12`).
pub const HOST_MAX_READS: u32 = 64;
pub const HOST_MAX_REQUESTS: u32 = 32;

/// The request and read limits the job is admitted with; the template states them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub max_requests: u32,
    pub max_reads: u32,
}

/// [`render_for`] on the `openai-codex` profile.
pub fn render(contract: &str, subject: &Subject, limits: Limits) -> Result<String, Refusal> {
    render_for(Profile::OpenaiCodex, contract, subject, limits)
}

/// Renders the task: the profile's template (placeholders filled) + contract bytes, refused above
/// the prompt bound.
pub fn render_for(
    profile: Profile,
    contract: &str,
    subject: &Subject,
    limits: Limits,
) -> Result<String, Refusal> {
    let (_, template, pinned) = template(profile);
    if sha256_hex(template.as_bytes()) != pinned {
        return Err(Refusal::Task(
            "template bytes differ from the pinned digest".into(),
        ));
    }
    if !(1..=HOST_MAX_REQUESTS).contains(&limits.max_requests)
        || !(1..=HOST_MAX_READS).contains(&limits.max_reads)
    {
        return Err(Refusal::Task(format!(
            "limits outside the Host bounds: {} requests (1..={HOST_MAX_REQUESTS}), {} reads (1..={HOST_MAX_READS})",
            limits.max_requests, limits.max_reads
        )));
    }
    let subject_json = serde_json::to_string(subject).map_err(|e| Refusal::Task(e.to_string()))?;
    let head = template
        .replace("{{SUBJECT}}", &subject_json)
        .replace("{{MAX_REQUESTS}}", &limits.max_requests.to_string())
        .replace("{{MAX_READS}}", &limits.max_reads.to_string());
    if head.contains("{{") {
        return Err(Refusal::Task("unfilled template placeholder".into()));
    }
    let task = format!("{head}{contract}");
    if task.len() + HOST_SUFFIX_BYTES > MAX_PROMPT_BYTES {
        return Err(Refusal::PromptBound {
            task: task.len(),
            suffix: HOST_SUFFIX_BYTES,
            bound: MAX_PROMPT_BYTES,
        });
    }
    Ok(task)
}
