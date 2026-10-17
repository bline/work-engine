use crate::claude::session_read::SessionManifest;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteCapture {
    pub schema_version: u32,
    pub pid: u32,
    pub exit_code: i32,
    pub response_sha256: String,
    pub stderr_sha256: String,
    pub result_sha256: String,
    pub observed_model: String,
    pub observed_session_id: String,
    pub provider_reported_entry: bool,
    pub prompt_submitted: bool,
    pub local_reaped: bool,
    pub guard_ready: bool,
    pub cbm_observation_sha256: Option<String>,
    pub session: SessionManifest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualifiedTerminal {
    pub schema_version: u32,
    pub capture_event_sha256: String,
    pub result_sha256: String,
    pub observation_sha256: String,
    pub session_manifest_sha256: String,
    pub artifact_references: Vec<String>,
}

use super::selection::Prepared;
use crate::{EvidenceReadError, contract::sha256, store_read};
use claim_evidence::codec::{JsValue, digest, parse_json};
use serde_json::json;

pub fn observation(
    prepared: &Prepared,
    capture: &CompleteCapture,
    result: &[u8],
    observed_at: Option<&str>,
) -> Result<JsValue, EvidenceReadError> {
    let result_value = parse_json(
        std::str::from_utf8(result)
            .map_err(|_| EvidenceReadError::Conflicting("Claude result text invalid"))?,
    )
    .map_err(|_| EvidenceReadError::Conflicting("Claude result JSON invalid"))?;
    let result_digest = digest(&result_value)
        .map_err(|_| EvidenceReadError::Conflicting("Claude result digest invalid"))?;
    let artifact = |kind: &str, sha: &str| {
        json!({"owner":store_read::OWNER,
        "reference":format!("execution-artifact:{}:{}:{kind}", prepared.root_id, prepared.attempt_id),
        "digest":sha,"status":"verified"})
    };
    let session_manifest = serde_json::to_vec(&capture.session)
        .map_err(|_| EvidenceReadError::Conflicting("session manifest serialization failed"))?;
    let mut value = json!({
        "schema_version":2,"id":"pending",
        "event_identity":format!("{}:{}:{}",if prepared.provenance_class == "qualified_provider" {
            "claude-qualified" } else { "claude-offline" },prepared.root_id,prepared.attempt_id),
        "kind":"production_path",
        "selection":{"id":prepared.selection_id,"revision":prepared.selection_digest},
        "obligationId":prepared.obligation_id,
        "subject":{"candidate":{"commit":prepared.candidate_commit,"tree":prepared.candidate_tree,
            "patchIdentity":prepared.candidate_patch_identity},"reviewEpisodeId":prepared.review_episode_id},
        "coveredState":"review-result",
        "execution":{"attemptId":prepared.attempt_id,"resultDigest":result_digest},
        "realization":{"requested":if capture.guard_ready {"rp0-guarded-claude-v1"} else {"offline-peer-v1"},
            "observed":if capture.guard_ready {"rp0-guarded-claude-v1"} else {"offline-peer-v1"}},
        "capabilityEnvelope":{"capabilities":[],"mutationAuthorized":!capture.guard_ready},
        "continuity":{"mode":"fresh_initial","sessionId":capture.observed_session_id},
        "transport":{"mechanism":if prepared.provenance_class == "qualified_provider" {
            "claude-cli-json-v1" } else { "offline-claude-json-v1" },"digest":capture.response_sha256},
        "observer":{"identity":"review-execution-evidence","kind":if capture.guard_ready {
            "guarded-launch-owner"} else {"offline-peer-owner"}},
        "observedAt":observed_at.map(str::to_owned).unwrap_or_else(canonical_now_utc),
        "adapterVersion":if capture.guard_ready {"claude-qualified-v1"} else {"claude-offline-v1"},
        "artifacts":[artifact("response",&capture.response_sha256),
            artifact("stderr",&capture.stderr_sha256),artifact("session",&sha256(&session_manifest))]
    });
    if let Some(cbm) = &capture.cbm_observation_sha256 {
        value["artifacts"]
            .as_array_mut()
            .unwrap()
            .push(artifact("cbm", cbm));
    }
    let mut claim = parse_json(&value.to_string())
        .map_err(|_| EvidenceReadError::Conflicting("Claude observation malformed"))?;
    if let JsValue::Object(fields) = &mut claim {
        fields.remove(&"id".into());
    }
    let id = format!(
        "production-path-observation-v1@{}",
        digest(&claim)
            .map_err(|_| EvidenceReadError::Conflicting("Claude observation digest invalid"))?
    );
    value["id"] = json!(id);
    let observation = parse_json(&value.to_string())
        .map_err(|_| EvidenceReadError::Conflicting("Claude observation malformed"))?;
    claim_evidence::validate_production_path_observation(&observation)
        .map_err(|_| EvidenceReadError::Conflicting("Claude observation schema invalid"))?;
    Ok(observation)
}

fn canonical_now_utc() -> String {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = elapsed.as_secs();
    let days = (seconds / 86_400) as i64;
    let secs = seconds % 86_400;
    // Gregorian civil date from days since 1970-01-01.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60,
        elapsed.subsec_millis()
    )
}
