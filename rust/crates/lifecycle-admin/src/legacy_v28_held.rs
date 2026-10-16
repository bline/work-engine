//! Read-only assembly of an A-v2 historical closure for a later trusted handoff.
//! B1 has no production source witness and cannot create a destination.

use std::fs;
use std::path::Path;

use lifecycle_store::{HeldArtifactInput, HeldEvidence, SqliteLifecycleStore};

use crate::legacy_v28::{TranslationReceipt, verify_completed_translation};

const SOURCE: [&str; 7] = [
    "attempt-receipt.json",
    "capture/source-manifest.json",
    "capture/receipt.json",
    "capture/image/semantic-context.sqlite3",
    "capture/image/role-bindings.json",
    "capture/image/generation-state.json",
    "capture/image/switchboard-attachment.json",
];
const OUTPUT: [&str; 9] = [
    "capsule.json",
    "inventory-private.json",
    "report-redacted.json",
    "manifest.json",
    "receipt.json",
    "raw/q3-disposition.json",
    "raw/q3-input.json",
    "raw/q4-disposition.json",
    "raw/q4-input.json",
];

/// Production A readback is repeated against its exact attempt and receipt.
/// The returned bytes are evidence only; no B1 production mint exists.
pub fn inspect_verified_held_translation(
    attempt: &Path,
    output: &Path,
    receipt: &TranslationReceipt,
) -> Result<HeldEvidence, String> {
    verify_completed_translation(attempt, output, receipt)?;
    if receipt.capsule_version.as_deref() != Some("s9-v28-obligations-v2")
        || receipt.translation_status != "qualified"
        || receipt.transfer_eligibility != "blocked_unclassified"
        || receipt.unclassified_rows == 0
    {
        return Err("held import requires a verified blocked A-v2 result".into());
    }
    let mut artifacts = Vec::with_capacity(SOURCE.len() + OUTPUT.len());
    for name in SOURCE {
        artifacts.push(HeldArtifactInput {
            name: format!("source/{name}"),
            raw_bytes: fs::read(attempt.join(name))
                .map_err(|_| format!("source closure unavailable: {name}"))?,
        });
    }
    for name in OUTPUT {
        artifacts.push(HeldArtifactInput {
            name: format!("output/{name}"),
            raw_bytes: fs::read(output.join(name))
                .map_err(|_| format!("output closure unavailable: {name}"))?,
        });
    }
    // Reverify after collecting the bytes and compare the exact same closure
    // members once more. The store then checks their internal hash linkage.
    verify_completed_translation(attempt, output, receipt)?;
    for artifact in &artifacts {
        let path = if let Some(name) = artifact.name.strip_prefix("source/") {
            attempt.join(name)
        } else if let Some(name) = artifact.name.strip_prefix("output/") {
            output.join(name)
        } else {
            return Err("invalid held closure member".into());
        };
        if fs::read(path).map_err(|_| "held closure changed during readback")? != artifact.raw_bytes
        {
            return Err("held closure changed during readback".into());
        }
    }
    let evidence = HeldEvidence { artifacts };
    SqliteLifecycleStore::inspect_held_evidence(&evidence)
        .map_err(|_| "held closure source/output binding invalid".to_owned())?;
    Ok(evidence)
}
