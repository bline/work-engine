//! Copy-only translation of a qualified, sidecar-absent schema-12 capture.
//! This module deliberately has no writable lifecycle-store or launch capability.

use rusqlite::{Connection, OpenFlags, types::ValueRef};
use serde::Deserialize;
use serde::Serialize;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const QUALIFIED_BINARY_SHA256: &str =
    "f9b67b1a05b1ab3e975e9c0fe012a1909d29e7f66ca54e77c092c68e6a1eaea9";
const QUALIFIED_SOURCE_AGGREGATE: &str =
    "969e52ee4254bd9e8a1f6ad2290b77e08d30f59f880da747ba145b3835e7d5e5";
const IMAGE_NAMES: [&str; 4] = [
    "semantic-context.sqlite3",
    "role-bindings.json",
    "generation-state.json",
    "switchboard-attachment.json",
];
const WAL: &str = "semantic-context.sqlite3-wal";
const SHM: &str = "semantic-context.sqlite3-shm";

/// A copy-only result. This is evidence, never an admission or custody permit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationReceipt {
    pub output_root: PathBuf,
    pub capsule_sha256: String,
    pub manifest_sha256: String,
    /// Exact in-directory receipt bytes; receipt.json cannot hash itself via manifest.json.
    pub receipt_sha256: String,
    pub translation_status: String,
    pub transfer_eligibility: String,
    pub unclassified_rows: usize,
    /// Returned only after the no-replace rename, parent sync, and content readback.
    pub publication_status: String,
    pub source_attempt_sha256: String,
}

#[derive(Debug, thiserror::Error)]
#[error("{primary_error}; phase={phase}; actual artifact={actual_artifact}")]
pub struct TranslationError {
    pub primary_error: String,
    pub phase: String,
    pub actual_artifact: String,
    pub receipt_error: Option<String>,
    pub cleanup_error: Option<String>,
}

type T<T> = Result<T, String>;

#[derive(Clone)]
struct Identity {
    dev: u64,
    ino: u64,
    len: u64,
    mtime_ns: i128,
}

impl Identity {
    fn of(path: &Path, max_len: u64) -> T<Self> {
        let meta =
            fs::symlink_metadata(path).map_err(|e| format!("metadata {}: {e}", path.display()))?;
        if !meta.file_type().is_file() || meta.nlink() != 1 || meta.len() > max_len {
            return Err(format!(
                "unsafe or oversized copied file {}",
                path.display()
            ));
        }
        Ok(Self {
            dev: meta.dev(),
            ino: meta.ino(),
            len: meta.len(),
            mtime_ns: i128::from(meta.mtime()) * 1_000_000_000 + i128::from(meta.mtime_nsec()),
        })
    }

    fn matches(&self, other: &Self) -> bool {
        self.dev == other.dev
            && self.ino == other.ino
            && self.len == other.len
            && self.mtime_ns == other.mtime_ns
    }
}

#[derive(Serialize)]
struct InventoryRow {
    table: String,
    identity: String,
    revision: Option<String>,
    status: Option<String>,
    source_projection_sha256: String,
    classification: String,
    reason: Option<String>,
}

struct TableSpec {
    name: &'static str,
    columns: &'static [&'static str],
    key: &'static str,
    revision: Option<&'static str>,
    status: Option<&'static str>,
}

// Column order is pinned to the reached schema-12 migration DDL, not inferred
// from a copied database. SQL below is built only from these static identifiers.
const TABLES: &[TableSpec] = &[
    TableSpec {
        name: "legacy_context_input_custody_lineages",
        columns: &["custody_lineage_id", "lineage_json", "created_at"],
        key: "custody_lineage_id",
        revision: None,
        status: None,
    },
    TableSpec {
        name: "legacy_context_input_custody_owners",
        columns: &[
            "custody_lineage_id",
            "owner_revision",
            "owner_generation_id",
            "owner_state_root",
            "owner_epoch",
            "state",
            "owner_json",
            "updated_at",
        ],
        key: "custody_lineage_id",
        revision: Some("owner_revision"),
        status: Some("state"),
    },
    TableSpec {
        name: "context_input_queue",
        columns: &[
            "sequence",
            "queue_id",
            "logical_role_instance_id",
            "transition_revision",
            "client_user_message_id",
            "input_revision",
            "input_json",
            "status",
            "queued_at",
            "released_at",
            "delivery_json",
        ],
        key: "queue_id",
        revision: Some("input_revision"),
        status: Some("status"),
    },
    TableSpec {
        name: "legacy_context_input_dispositions",
        columns: &[
            "queue_id",
            "logical_role_instance_id",
            "thread_id",
            "binding_revision",
            "transition_revision",
            "client_user_message_id",
            "input_revision",
            "input_json_sha256",
            "disposition",
            "canonical_inventory_revision",
            "authority_revision",
            "successor_acceptance_revision",
            "successor_logical_role_instance_id",
            "successor_delivery_identity",
            "record_revision",
            "record_json",
            "committed_at",
        ],
        key: "queue_id",
        revision: Some("record_revision"),
        status: Some("disposition"),
    },
    TableSpec {
        name: "legacy_context_input_successor_deliveries",
        columns: &[
            "queue_id",
            "claim_revision",
            "claim_json",
            "status",
            "claimed_at",
            "completed_at",
            "delivery_revision",
            "delivery_json",
        ],
        key: "queue_id",
        revision: Some("claim_revision"),
        status: Some("status"),
    },
    TableSpec {
        name: "context_input_admissions",
        columns: &[
            "logical_role_instance_id",
            "thread_id",
            "binding_revision",
            "transition_revision",
            "status",
            "reconciliation_revision",
            "closed_at",
            "reopened_at",
        ],
        key: "logical_role_instance_id",
        revision: Some("reconciliation_revision"),
        status: Some("status"),
    },
    TableSpec {
        name: "aborted_context_input_releases",
        columns: &[
            "authorization_revision",
            "logical_role_instance_id",
            "thread_id",
            "binding_revision",
            "transition_revision",
            "authorization_json",
            "authorized_at",
        ],
        key: "authorization_revision",
        revision: Some("authorization_revision"),
        status: None,
    },
    TableSpec {
        name: "context_domain_input_custody",
        columns: &[
            "transfer_id",
            "source_queue_id",
            "logical_role_instance_id",
            "client_message_id",
            "transition_id",
            "effect_id",
            "input_revision",
            "source_sequence",
            "destination_scheduler_identity",
            "custody_state",
            "receipt_revision",
            "accepted_at",
            "payload_json",
        ],
        key: "transfer_id",
        revision: Some("receipt_revision"),
        status: Some("custody_state"),
    },
    TableSpec {
        name: "context_transitions",
        columns: &[
            "transition_id",
            "logical_role_instance_id",
            "binding_revision",
            "provider_thread_id",
            "predecessor_turn_id",
            "semantic_state",
            "committed_transition_revision",
            "terminal",
            "created_at",
            "payload_json",
        ],
        key: "transition_id",
        revision: Some("committed_transition_revision"),
        status: Some("semantic_state"),
    },
    TableSpec {
        name: "context_transition_provider_observations",
        columns: &[
            "provider_observation_id",
            "provider_event_kind",
            "provider_thread_id",
            "provider_turn_id",
            "provider_event_identity",
            "raw_evidence_revision",
            "observed_at",
            "payload_json",
        ],
        key: "provider_observation_id",
        revision: Some("raw_evidence_revision"),
        status: Some("provider_event_kind"),
    },
    TableSpec {
        name: "context_transition_host_facts",
        columns: &[
            "host_fact_id",
            "host_fact_kind",
            "logical_role_instance_id",
            "host_thread_id",
            "host_turn_id",
            "stable_host_source_identity",
            "raw_evidence_revision",
            "observed_at",
            "payload_json",
        ],
        key: "host_fact_id",
        revision: Some("raw_evidence_revision"),
        status: Some("host_fact_kind"),
    },
    TableSpec {
        name: "context_transition_events",
        columns: &[
            "transition_event_id",
            "transition_id",
            "transition_event_kind",
            "classification_basis_revision",
            "source_kind",
            "provider_observation_id",
            "host_fact_id",
            "created_at",
            "payload_json",
        ],
        key: "transition_event_id",
        revision: Some("classification_basis_revision"),
        status: Some("transition_event_kind"),
    },
    TableSpec {
        name: "context_transition_event_applications",
        columns: &[
            "application_id",
            "transition_event_id",
            "transition_id",
            "expected_transition_revision",
            "observed_transition_revision",
            "semantic_disposition",
            "storage_result",
            "committed_transition_revision",
            "application_json",
        ],
        key: "application_id",
        revision: Some("committed_transition_revision"),
        status: Some("semantic_disposition"),
    },
    TableSpec {
        name: "context_transition_admission_owners",
        columns: &[
            "logical_role_instance_id",
            "owner_kind",
            "transition_id",
            "recovery_operation_id",
            "owner_revision",
            "projection_json",
        ],
        key: "logical_role_instance_id",
        revision: Some("owner_revision"),
        status: Some("owner_kind"),
    },
    TableSpec {
        name: "context_transition_effects",
        columns: &[
            "effect_id",
            "transition_id",
            "effect_kind",
            "predecessor_effect_id",
            "expected_transition_revision",
            "input_revision",
            "status",
            "effect_revision",
            "payload_json",
        ],
        key: "effect_id",
        revision: Some("effect_revision"),
        status: Some("status"),
    },
    TableSpec {
        name: "context_transition_effect_applications",
        columns: &[
            "application_id",
            "effect_id",
            "expected_effect_revision",
            "observed_effect_revision",
            "storage_result",
            "committed_effect_revision",
            "application_json",
        ],
        key: "application_id",
        revision: Some("committed_effect_revision"),
        status: Some("storage_result"),
    },
    TableSpec {
        name: "context_transition_effect_evidence",
        columns: &[
            "evidence_id",
            "effect_id",
            "evidence_revision",
            "observed_at",
            "payload_json",
        ],
        key: "evidence_id",
        revision: Some("evidence_revision"),
        status: None,
    },
    TableSpec {
        name: "context_transition_recovery_operations",
        columns: &[
            "recovery_operation_id",
            "logical_role_instance_id",
            "failed_transition_id",
            "failed_transition_revision",
            "authority_revision",
            "custody_disposition_revision",
            "recovery_state",
            "recovery_revision",
            "payload_json",
        ],
        key: "recovery_operation_id",
        revision: Some("recovery_revision"),
        status: Some("recovery_state"),
    },
    TableSpec {
        name: "context_transition_queue_drain_receipts",
        columns: &[
            "queue_drain_receipt_id",
            "transition_id",
            "effect_id",
            "receipt_revision",
            "payload_json",
        ],
        key: "queue_drain_receipt_id",
        revision: Some("receipt_revision"),
        status: None,
    },
    TableSpec {
        name: "active_turn_lifecycle",
        columns: &[
            "identity_key",
            "turn_key",
            "revision",
            "status",
            "next_eligible_at",
            "state_json",
        ],
        key: "identity_key",
        revision: Some("revision"),
        status: Some("status"),
    },
    TableSpec {
        name: "active_turn_lifecycle_observations",
        columns: &[
            "identity_key",
            "turn_key",
            "owner_identity_key",
            "disposition",
            "triggering_observation_sequence",
            "observed_at",
        ],
        key: "identity_key",
        revision: None,
        status: Some("disposition"),
    },
];

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn fail<R>(message: impl Into<String>) -> T<R> {
    Err(message.into())
}

fn field<'a>(value: &'a Value, name: &str) -> T<&'a Value> {
    value.get(name).ok_or_else(|| format!("missing {name}"))
}

fn string<'a>(value: &'a Value, name: &str) -> T<&'a str> {
    field(value, name)?
        .as_str()
        .ok_or_else(|| format!("invalid {name}"))
}

fn nonempty<'a>(value: &'a Value, name: &str) -> T<&'a str> {
    let text = string(value, name)?;
    if text.trim().is_empty() {
        return fail(format!("empty {name}"));
    }
    Ok(text)
}

// Match the v28 JavaScript String.trim blankness test for reached identities.
// No identity is normalized: only an all-whitespace string is refused.
fn js_whitespace(ch: char) -> bool {
    matches!(ch,
        '\u{0009}'..='\u{000D}' | '\u{0020}' | '\u{00A0}' | '\u{1680}'
        | '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}'
        | '\u{205F}' | '\u{3000}' | '\u{FEFF}')
}

fn legacy_required_text<'a>(value: &'a Value, name: &str) -> T<&'a str> {
    let text = string(value, name)?;
    if text.chars().all(js_whitespace) {
        return fail(format!("empty legacy {name}"));
    }
    Ok(text)
}

// A documented fail-closed subset of v28 Date.parse: extended UTC with no
// fraction or one through nine fractional digits. Other parseable JS dates remain
// unsupported evidence, never silently reinterpreted.
fn supported_legacy_utc_timestamp(text: &str) -> bool {
    let bytes = text.as_bytes();
    if !(bytes.len() == 20 || (22..=30).contains(&bytes.len()))
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[bytes.len() - 1] != b'Z'
        || (bytes.len() != 20 && bytes[19] != b'.')
    {
        return false;
    }
    let number = |start: usize, end: usize| -> Option<u32> {
        bytes[start..end]
            .iter()
            .all(u8::is_ascii_digit)
            .then(|| std::str::from_utf8(&bytes[start..end]).ok()?.parse().ok())?
    };
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(0, 4),
        number(5, 7),
        number(8, 10),
        number(11, 13),
        number(14, 16),
        number(17, 19),
    ) else {
        return false;
    };
    if bytes.len() != 20 && !bytes[20..bytes.len() - 1].iter().all(u8::is_ascii_digit) {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    year > 0 && day >= 1 && day <= max_day && hour < 24 && minute < 60 && second < 60
}

fn producer_timestamp(text: &str) -> bool {
    let Some((seconds, fraction)) = text.strip_suffix('Z').and_then(|s| s.split_once('.')) else {
        return false;
    };
    !seconds.is_empty()
        && seconds.bytes().all(|b| b.is_ascii_digit())
        && fraction.len() == 9
        && fraction.bytes().all(|b| b.is_ascii_digit())
}

fn exact_keys(value: &Value, expected: &[&str], label: &str) -> T<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{label} is not an object"))?;
    let actual: BTreeSet<_> = object.keys().map(String::as_str).collect();
    let expected: BTreeSet<_> = expected.iter().copied().collect();
    if actual != expected {
        return fail(format!("{label} has unsupported or missing fields"));
    }
    Ok(())
}

fn sha_revision<'a>(value: &'a Value, name: &str) -> T<&'a str> {
    let text = nonempty(value, name)?;
    if !is_revision(text) {
        return fail(format!("invalid {name} revision"));
    }
    Ok(text)
}

fn is_revision(text: &str) -> bool {
    text.len() == 71
        && text.starts_with("sha256:")
        && text[7..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn json_file(path: &Path, max_len: u64) -> T<(Value, String)> {
    let bytes = checked_file(path, max_len)?;
    let value = parsed_json(&bytes, &format!("JSON {}", path.display()))?;
    Ok((value, sha(&bytes)))
}

// Capture receipts contain timing metadata, not legacy revision-bearing values.
// Keep their exact byte digest and duplicate-key rejection while allowing finite
// JSON numbers. Image JSON and historical records still use the strict parser.
fn capture_json_file(path: &Path, max_len: u64) -> T<(Value, String)> {
    let bytes = checked_file(path, max_len)?;
    let value = serde_json::from_slice::<UniqueValue<true>>(&bytes)
        .map_err(|e| format!("capture JSON {}: {e}", path.display()))?;
    Ok((value.0, sha(&bytes)))
}

fn checked_file(path: &Path, max_len: u64) -> T<Vec<u8>> {
    let before = Identity::of(path, max_len)?;
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let after = Identity::of(path, max_len)?;
    if !before.matches(&after) || bytes.len() as u64 != before.len {
        return fail(format!("copied file drift {}", path.display()));
    }
    Ok(bytes)
}

fn checked_dir(path: &Path) -> T<()> {
    let meta =
        fs::symlink_metadata(path).map_err(|e| format!("directory {}: {e}", path.display()))?;
    if !meta.file_type().is_dir() || fs::canonicalize(path).map_err(|e| e.to_string())? != path {
        return fail(format!(
            "noncanonical or unsafe directory {}",
            path.display()
        ));
    }
    Ok(())
}

fn image_names(image: &Path) -> T<()> {
    checked_dir(image)?;
    let mut observed = BTreeSet::new();
    for entry in fs::read_dir(image).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        observed.insert(entry.file_name().to_string_lossy().into_owned());
    }
    let expected: BTreeSet<_> = IMAGE_NAMES.iter().map(|s| (*s).to_string()).collect();
    if observed != expected {
        return fail("copied image has missing, unexpected, or WAL/SHM sidecar files");
    }
    Ok(())
}

fn sidecars_absent(outer: &Value, key: &str) -> T<()> {
    let metadata = field(outer, key)?;
    for sidecar in [WAL, SHM] {
        let present = field(metadata, sidecar)?
            .get("present")
            .and_then(Value::as_bool);
        if present != Some(false) {
            return fail(format!(
                "capture {key} does not establish {sidecar} absence"
            ));
        }
    }
    Ok(())
}

fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:09}Z", now.as_secs(), now.subsec_nanos())
}

struct Loaded {
    db_path: PathBuf,
    image_dir: PathBuf,
    image_hashes: BTreeMap<String, String>,
    identities: BTreeMap<String, Identity>,
    registry: Value,
    outer_hash: String,
    child_hash: String,
    manifest_hash: String,
    selection_claim: String,
    coherence: String,
    source_root: String,
}

fn validate_capture(attempt_root: &Path) -> T<Loaded> {
    if !attempt_root.is_absolute() {
        return fail("attempt root must be absolute");
    }
    checked_dir(attempt_root)?;
    let capture = attempt_root.join("capture");
    checked_dir(&capture)?;
    let image = capture.join("image");
    // The file-set gate precedes even constructing a SQLite connection.
    image_names(&image)?;

    let (outer, outer_hash) =
        capture_json_file(&attempt_root.join("attempt-receipt.json"), 4_000_000)?;
    let (child, child_hash) = capture_json_file(&capture.join("receipt.json"), 4_000_000)?;
    let (manifest, manifest_hash) =
        capture_json_file(&capture.join("source-manifest.json"), 4_000_000)?;
    for (label, value) in [
        ("outer", &outer),
        ("child", &child),
        ("manifest", &manifest),
    ] {
        if field(value, "schema_version")?.as_u64() != Some(1) {
            return fail(format!("unsupported {label} receipt/manifest version"));
        }
    }
    for (label, value) in [("outer", &outer), ("child", &child)] {
        if string(value, "status")? != "qualified_candidate_root"
            || !field(value, "primary_error")?.is_null()
            || !field(value, "cleanup_error")?.is_null()
        {
            return fail(format!("capture {label} was not cleanly qualified"));
        }
    }
    if outer.get("raw_exit_status").and_then(Value::as_i64) != Some(0)
        || string(&outer, "child_status")? != "qualified_candidate_root"
        || !field(&outer, "child_primary_error")?.is_null()
        || !field(&outer, "child_cleanup_error")?.is_null()
        || outer
            .get("child_event_ledger")
            .and_then(Value::as_array)
            .is_none_or(|v| !v.is_empty())
        || child
            .get("event_ledger")
            .and_then(Value::as_array)
            .is_none_or(|v| !v.is_empty())
    {
        return fail("capture child status, event, or error mismatch");
    }
    sidecars_absent(&outer, "preflight_source_metadata")?;
    sidecars_absent(&outer, "postflight_source_metadata")?;
    let binary = field(&outer, "binary")?;
    if string(binary, "sha256")? != QUALIFIED_BINARY_SHA256
        || string(&child, "executable_sha256")? != QUALIFIED_BINARY_SHA256
        || string(&outer, "source_aggregate_sha256")? != QUALIFIED_SOURCE_AGGREGATE
    {
        return fail("capture binary/source identity is not the qualified subject");
    }
    if string(field(&outer, "child_receipt")?, "sha256")? != child_hash
        || string(&outer, "child_manifest_sha256")? != manifest_hash
        || string(&child, "manifest_sha256")? != manifest_hash
        || field(&outer, "source_hashes_before")? != field(&outer, "source_hashes_after")?
        || field(&outer, "source_hashes_before")? != field(&child, "source_hashes")?
    {
        return fail("capture receipt/hash/producer-source binding mismatch");
    }
    let selection_claim = string(&manifest, "selection_claim")?;
    if selection_claim != "current_selection_unestablished"
        || string(&child, "selection_claim")? != selection_claim
        || string(&outer, "selection_claim")? != selection_claim
        || manifest.get("selector_present").and_then(Value::as_bool) != Some(false)
    {
        return fail("unsupported selection claim or selector input");
    }
    let coherence = string(&manifest, "coherence")?;
    if coherence
        != "conditional_no_relevant_inotify_event_and_stable_bytes_under_normal_writer_model"
    {
        return fail("unsupported capture coherence label");
    }
    let source_root = string(&manifest, "source_root")?;
    if string(&child, "source_root")? != source_root
        || outer
            .get("source_root")
            .is_some_and(|root| root.as_str() != Some(source_root))
    {
        return fail("capture source-root provenance mismatch");
    }
    // The qualified outer producer records COMMAND as argv, without a top-level
    // source_root. Its flag/value operand must agree with the hash-bound child
    // and manifest. Original source paths remain provenance only; never open them.
    let argv = field(&outer, "argv")?
        .as_array()
        .ok_or("capture source-root argv invalid")?;
    if argv.is_empty() || argv.iter().any(|arg| !arg.is_string()) {
        return fail("capture source-root argv invalid");
    }
    let mut roots = 0;
    for pair in argv[1..].chunks(2) {
        if pair.len() != 2 || pair[0].as_str().unwrap().starts_with("--source-root=") {
            return fail("capture source-root argv invalid");
        }
        if pair[0] == "--source-root" {
            roots += 1;
            if pair[1].as_str() != Some(source_root) {
                return fail("capture source-root argv mismatch");
            }
        }
    }
    if roots != 1 {
        return fail("capture source-root argv missing or repeated");
    }
    let files = field(&manifest, "files")?
        .as_array()
        .ok_or("manifest files invalid")?;
    if files.len() != IMAGE_NAMES.len() {
        return fail("manifest is not the four-file sidecar-absent capture");
    }
    let mut declared = BTreeSet::new();
    let mut hashes = BTreeMap::new();
    let mut identities = BTreeMap::new();
    for item in files {
        let copy = Path::new(string(item, "copy")?);
        let name = copy
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("manifest copy basename invalid")?;
        if !IMAGE_NAMES.contains(&name)
            || copy != image.join(name)
            || !declared.insert(name.to_string())
        {
            return fail("manifest has unexpected, duplicated, or unbound copy path");
        }
        let source = Path::new(string(item, "source")?);
        if source.file_name().and_then(|s| s.to_str()) != Some(name)
            || !source.starts_with(Path::new(string(&manifest, "source_root")?))
        {
            return fail("manifest source provenance does not bind copied basename");
        }
        let copy_hash = string(item, "copy_sha256")?;
        if string(item, "sha256")? != copy_hash {
            return fail("source/copy digest mismatch");
        }
        let artifact_key = format!("capture/image/{name}");
        let outer_hash_for_image =
            string(field(field(&outer, "artifacts")?, &artifact_key)?, "sha256")?;
        if outer_hash_for_image != copy_hash {
            return fail("outer attempt does not bind copied image digest");
        }
        let max_len = if name == "semantic-context.sqlite3" {
            128_000_000
        } else {
            16_000_000
        };
        let before = Identity::of(copy, max_len)?;
        let bytes = checked_file(copy, max_len)?;
        if sha(&bytes) != copy_hash {
            return fail(format!("copied image digest mismatch for {name}"));
        }
        hashes.insert(name.to_string(), copy_hash.to_string());
        identities.insert(name.to_string(), before);
    }
    image_names(&image)?;
    let expected: BTreeSet<_> = IMAGE_NAMES.iter().map(|s| (*s).to_string()).collect();
    if declared != expected {
        return fail("manifest omitted expected copied image file");
    }
    let (registry, _) = json_file(&image.join("role-bindings.json"), 16_000_000)?;
    if registry.get("schemaVersion").and_then(Value::as_u64) != Some(1)
        || registry.get("revision").and_then(Value::as_u64).is_none()
        || registry
            .get("bindings")
            .and_then(Value::as_object)
            .is_none()
        || registry
            .get("deliveries")
            .and_then(Value::as_object)
            .is_none()
    {
        return fail("copied registry schema or revision unsupported");
    }
    for name in ["generation-state.json", "switchboard-attachment.json"] {
        let _ = json_file(&image.join(name), 16_000_000)?;
    }
    Ok(Loaded {
        db_path: image.join("semantic-context.sqlite3"),
        image_dir: image,
        image_hashes: hashes,
        identities,
        registry,
        outer_hash,
        child_hash,
        manifest_hash,
        selection_claim: selection_claim.to_string(),
        coherence: coherence.to_string(),
        source_root: string(&manifest, "source_root")?.to_string(),
    })
}

fn postflight_image(capture: &Loaded) -> T<()> {
    image_names(&capture.image_dir)?;
    for name in IMAGE_NAMES {
        let path = capture.image_dir.join(name);
        let max_len = if name == "semantic-context.sqlite3" {
            128_000_000
        } else {
            16_000_000
        };
        let current = Identity::of(&path, max_len)?;
        if !current.matches(capture.identities.get(name).ok_or("image identity lost")?)
            || sha(&checked_file(&path, max_len)?)
                != *capture.image_hashes.get(name).ok_or("image hash lost")?
        {
            return fail(format!("copied image drift after SQLite read: {name}"));
        }
    }
    Ok(())
}

fn canonical(value: &Value) -> T<String> {
    match value {
        Value::Null => Ok("null".to_string()),
        Value::Bool(v) => Ok(v.to_string()),
        Value::String(v) => serde_json::to_string(v).map_err(|e| e.to_string()),
        Value::Number(v) => {
            const MAX_SAFE: u64 = 9_007_199_254_740_991;
            if let Some(n) = v.as_i64()
                && n.unsigned_abs() <= MAX_SAFE
            {
                return Ok(n.to_string());
            }
            if let Some(n) = v.as_u64()
                && n <= MAX_SAFE
            {
                return Ok(n.to_string());
            }
            fail("unsupported legacy canonical JSON number")
        }
        Value::Array(values) => {
            let mut parts = Vec::with_capacity(values.len());
            for item in values {
                parts.push(canonical(item)?);
            }
            Ok(format!("[{}]", parts.join(",")))
        }
        Value::Object(values) => {
            let mut keys: Vec<_> = values.keys().collect();
            // JS's key sort is UTF-16. Reached identity field names are ASCII;
            // reject other keys instead of silently using a different codec.
            if keys.iter().any(|key| !key.is_ascii()) {
                return fail("unsupported non-ASCII legacy canonical JSON key");
            }
            keys.sort();
            let mut parts = Vec::with_capacity(keys.len());
            for key in keys {
                parts.push(format!(
                    "{}:{}",
                    serde_json::to_string(key).map_err(|e| e.to_string())?,
                    canonical(&values[key])?
                ));
            }
            Ok(format!("{{{}}}", parts.join(",")))
        }
    }
}

fn revision_without(value: &Value, field_name: &str) -> T<String> {
    let mut body = value
        .as_object()
        .ok_or("legacy revision envelope is not an object")?
        .clone();
    body.remove(field_name)
        .ok_or_else(|| format!("legacy revision missing {field_name}"))?;
    Ok(format!(
        "sha256:{}",
        sha(canonical(&Value::Object(body))?.as_bytes())
    ))
}

fn parsed_json(bytes: &[u8], label: &str) -> T<Value> {
    std::str::from_utf8(bytes).map_err(|_| format!("{label} is not UTF-8"))?;
    serde_json::from_slice::<UniqueValue>(bytes)
        .map(|value| value.0)
        .map_err(|e| format!("{label} JSON: {e}"))
}

// serde_json::Value accepts duplicate object keys with last-wins semantics.
// Legacy revision validation must reject them before a lossy reparse.
struct UniqueValue<const CAPTURE_METADATA: bool = false>(Value);

impl<'de, const CAPTURE_METADATA: bool> Deserialize<'de> for UniqueValue<CAPTURE_METADATA> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor<const CAPTURE_METADATA: bool>;
        impl<'de, const CAPTURE_METADATA: bool> Visitor<'de> for UniqueVisitor<CAPTURE_METADATA> {
            type Value = UniqueValue<CAPTURE_METADATA>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("JSON without duplicate object keys")
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Bool(value)))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(value.into())))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(value.into())))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                if !CAPTURE_METADATA {
                    return Err(E::custom("floating-point legacy JSON is unsupported"));
                }
                serde_json::Number::from_f64(value)
                    .map(|number| UniqueValue(Value::Number(number)))
                    .ok_or_else(|| E::custom("non-finite capture JSON number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(value.to_string())))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(value)))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<UniqueValue<CAPTURE_METADATA>>()? {
                    values.push(value.0);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, value)) =
                    map.next_entry::<String, UniqueValue<CAPTURE_METADATA>>()?
                {
                    if values.insert(key.clone(), value.0).is_some() {
                        return Err(de::Error::custom(format!("duplicate JSON key {key}")));
                    }
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor::<CAPTURE_METADATA>)
    }
}

fn check_input(value: &Value, row: &Queue) -> T<()> {
    let object = value.as_object().ok_or("context input is not an object")?;
    let expected: BTreeSet<_> = [
        "schemaVersion",
        "type",
        "logicalRoleInstanceId",
        "roleId",
        "instanceId",
        "threadId",
        "bindingRevision",
        "clientUserMessageId",
        "sourceKind",
        "text",
        "inputRevision",
    ]
    .into_iter()
    .collect();
    let actual: BTreeSet<_> = object.keys().map(String::as_str).collect();
    if actual != expected
        || value.get("schemaVersion").and_then(Value::as_u64) != Some(1)
        || value.get("type").and_then(Value::as_str) != Some("work-engine.context-transition-input")
        || string(value, "logicalRoleInstanceId")? != row.role
        || string(value, "clientUserMessageId")? != row.client_message
        || string(value, "inputRevision")? != row.input_revision
        || revision_without(value, "inputRevision")? != row.input_revision
    {
        return fail("context input codec or queue identity mismatch");
    }
    let role = string(value, "roleId")?;
    let instance = string(value, "instanceId")?;
    if format!("{role}:{instance}") != row.role
        || value
            .get("bindingRevision")
            .and_then(Value::as_u64)
            .is_none_or(|n| n == 0)
        || !matches!(string(value, "sourceKind")?, "human" | "agent")
    {
        return fail("context input identity or body invalid");
    }
    for name in [
        "logicalRoleInstanceId",
        "roleId",
        "instanceId",
        "threadId",
        "clientUserMessageId",
        "text",
    ] {
        let _ = legacy_required_text(value, name)?;
    }
    Ok(())
}

fn uri(path: &Path) -> String {
    let mut encoded = String::from("file:");
    for byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-_.~".contains(byte) {
            encoded.push(char::from(*byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded.push_str("?immutable=1");
    encoded
}

fn open_copy(path: &Path) -> T<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(uri(path), flags)
        .map_err(|e| format!("immutable copied SQLite open: {e}"))?;
    conn.pragma_update(None, "query_only", "ON")
        .map_err(|e| e.to_string())?;
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if version != 12 {
        return fail(format!("unsupported copied SQLite schema {version}"));
    }
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if integrity != "ok" {
        return fail("copied SQLite integrity check failed");
    }
    let mut fk = conn
        .prepare("PRAGMA foreign_key_check")
        .map_err(|e| e.to_string())?;
    if fk
        .query([])
        .map_err(|e| e.to_string())?
        .next()
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return fail("copied SQLite foreign-key check failed");
    }
    drop(fk);
    let mut versions = conn
        .prepare("SELECT version FROM schema_migrations ORDER BY version")
        .map_err(|e| format!("schema migrations: {e}"))?;
    let observed: Vec<i64> = versions
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    if observed != (1..=12).collect::<Vec<_>>() {
        return fail("schema migration ordinal history is not 1..12");
    }
    drop(versions);
    let mut table_list = conn
        .prepare("PRAGMA table_list")
        .map_err(|e| e.to_string())?;
    let mut kinds = BTreeMap::new();
    let mut listed = table_list.query([]).map_err(|e| e.to_string())?;
    while let Some(row) = listed.next().map_err(|e| e.to_string())? {
        let name: String = row.get(1).map_err(|e| e.to_string())?;
        let kind: String = row.get(2).map_err(|e| e.to_string())?;
        let strict: i64 = row.get(5).map_err(|e| e.to_string())?;
        kinds.insert(name, (kind, strict));
    }
    drop(listed);
    drop(table_list);
    for name in ["schema_migrations"]
        .into_iter()
        .chain(TABLES.iter().map(|s| s.name))
    {
        if kinds
            .get(name)
            .map(|(kind, strict)| (kind.as_str(), *strict))
            != Some(("table", 1))
        {
            return fail(format!(
                "copied schema-12 relation kind/STRICT mismatch: {name}"
            ));
        }
    }
    for spec in TABLES {
        let sql = format!("PRAGMA table_info({})", spec.name);
        let mut statement = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let actual: Vec<String> = statement
            .query_map([], |row| row.get(1))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        if actual.iter().map(String::as_str).collect::<Vec<_>>() != spec.columns {
            return fail(format!(
                "copied schema-12 table/column mismatch: {}",
                spec.name
            ));
        }
    }
    validate_material_schema(&conn)?;
    Ok(conn)
}

// The reached schema-12 migration 1/2/11/12 shapes at published v28 commit
// 2cfdf83ec5bee9c95bf746b7d8ec70037165d187. In these STRICT tables,
// TEXT PRIMARY KEY columns report notnull=1 in SQLite table_info, while the
// INTEGER PRIMARY KEY rowid columns report notnull=0.
fn validate_material_schema(conn: &Connection) -> T<()> {
    type ColumnShape = (&'static str, &'static str, i64, i64);
    type TableShape = (&'static str, &'static [ColumnShape]);
    const SHAPES: &[TableShape] = &[
        (
            "schema_migrations",
            &[("version", "INTEGER", 0, 1), ("applied_at", "TEXT", 1, 0)],
        ),
        (
            "legacy_context_input_custody_lineages",
            &[
                ("custody_lineage_id", "TEXT", 1, 1),
                ("lineage_json", "TEXT", 1, 0),
                ("created_at", "TEXT", 1, 0),
            ],
        ),
        (
            "legacy_context_input_custody_owners",
            &[
                ("custody_lineage_id", "TEXT", 1, 1),
                ("owner_revision", "TEXT", 1, 0),
                ("owner_generation_id", "TEXT", 1, 0),
                ("owner_state_root", "TEXT", 1, 0),
                ("owner_epoch", "INTEGER", 1, 0),
                ("state", "TEXT", 1, 0),
                ("owner_json", "TEXT", 1, 0),
                ("updated_at", "TEXT", 1, 0),
            ],
        ),
        (
            "context_input_queue",
            &[
                ("sequence", "INTEGER", 0, 1),
                ("queue_id", "TEXT", 1, 0),
                ("logical_role_instance_id", "TEXT", 1, 0),
                ("transition_revision", "TEXT", 1, 0),
                ("client_user_message_id", "TEXT", 1, 0),
                ("input_revision", "TEXT", 1, 0),
                ("input_json", "TEXT", 1, 0),
                ("status", "TEXT", 1, 0),
                ("queued_at", "TEXT", 1, 0),
                ("released_at", "TEXT", 0, 0),
                ("delivery_json", "TEXT", 0, 0),
            ],
        ),
        (
            "legacy_context_input_dispositions",
            &[
                ("queue_id", "TEXT", 1, 1),
                ("logical_role_instance_id", "TEXT", 1, 0),
                ("thread_id", "TEXT", 1, 0),
                ("binding_revision", "INTEGER", 1, 0),
                ("transition_revision", "TEXT", 1, 0),
                ("client_user_message_id", "TEXT", 1, 0),
                ("input_revision", "TEXT", 1, 0),
                ("input_json_sha256", "TEXT", 1, 0),
                ("disposition", "TEXT", 1, 0),
                ("canonical_inventory_revision", "TEXT", 1, 0),
                ("authority_revision", "TEXT", 1, 0),
                ("successor_acceptance_revision", "TEXT", 0, 0),
                ("successor_logical_role_instance_id", "TEXT", 0, 0),
                ("successor_delivery_identity", "TEXT", 0, 0),
                ("record_revision", "TEXT", 1, 0),
                ("record_json", "TEXT", 1, 0),
                ("committed_at", "TEXT", 1, 0),
            ],
        ),
        (
            "legacy_context_input_successor_deliveries",
            &[
                ("queue_id", "TEXT", 1, 1),
                ("claim_revision", "TEXT", 1, 0),
                ("claim_json", "TEXT", 1, 0),
                ("status", "TEXT", 1, 0),
                ("claimed_at", "TEXT", 1, 0),
                ("completed_at", "TEXT", 0, 0),
                ("delivery_revision", "TEXT", 0, 0),
                ("delivery_json", "TEXT", 0, 0),
            ],
        ),
    ];
    for (table, expected) in SHAPES {
        let mut statement = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(|e| e.to_string())?;
        let observed: Vec<(String, String, i64, i64)> = statement
            .query_map([], |row| {
                Ok((row.get(1)?, row.get(2)?, row.get(3)?, row.get(5)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        if observed.len() != expected.len()
            || observed.iter().zip(*expected).any(
                |((name, kind, required, pk), (want_name, want_kind, want_required, want_pk))| {
                    name.as_str() != *want_name
                        || kind.as_str() != *want_kind
                        || required != want_required
                        || pk != want_pk
                },
            )
        {
            return fail(format!(
                "copied schema-12 type/nullability/key mismatch: {table}"
            ));
        }
    }
    for (table, expected) in [
        (
            "legacy_context_input_custody_owners",
            Some((
                "legacy_context_input_custody_lineages",
                "custody_lineage_id",
                "custody_lineage_id",
            )),
        ),
        (
            "legacy_context_input_dispositions",
            Some(("context_input_queue", "queue_id", "queue_id")),
        ),
        (
            "legacy_context_input_successor_deliveries",
            Some(("legacy_context_input_dispositions", "queue_id", "queue_id")),
        ),
        ("schema_migrations", None),
        ("context_input_queue", None),
        ("legacy_context_input_custody_lineages", None),
    ] {
        let mut statement = conn
            .prepare(&format!("PRAGMA foreign_key_list({table})"))
            .map_err(|e| e.to_string())?;
        let observed: Vec<(String, String, String)> = statement
            .query_map([], |row| Ok((row.get(2)?, row.get(3)?, row.get(4)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        let wanted = expected
            .map(|(a, b, c)| vec![(a.to_string(), b.to_string(), c.to_string())])
            .unwrap_or_default();
        if observed != wanted {
            return fail(format!("copied schema-12 foreign-key mismatch: {table}"));
        }
    }
    for (table, required) in [
        (
            "context_input_queue",
            &[
                "queue_idtextnotnullunique",
                "unique(logical_role_instance_id,client_user_message_id)",
                "check(statusin('queued','releasing','released'))",
            ][..],
        ),
        (
            "legacy_context_input_dispositions",
            &[
                "record_revisiontextnotnullunique",
                "check(binding_revision>0)",
                "check(dispositionin('terminally_retired','withdrawn','durable_restart'))",
                "check((disposition='durable_restart'and",
                "successor_delivery_identityisnotnull)",
                "(disposition!='durable_restart'and",
                "successor_delivery_identityisnull))",
            ][..],
        ),
        (
            "legacy_context_input_successor_deliveries",
            &[
                "claim_revisiontextnotnullunique",
                "check((status='pending'",
                "delivery_jsonisnull)or(status='completed'",
                "delivery_jsonisnotnull))",
            ][..],
        ),
        (
            "legacy_context_input_custody_owners",
            &[
                "owner_revisiontextnotnullunique",
                "check(owner_epoch>0)",
                "check(statein('owned','adopted_inactive','transferred','recovery_required'))",
            ][..],
        ),
    ] {
        let sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let compact: String = sql
            .chars()
            .filter(|c| !c.is_ascii_whitespace())
            .flat_map(char::to_lowercase)
            .collect();
        if required.iter().any(|fragment| !compact.contains(fragment)) {
            return fail(format!(
                "copied schema-12 check constraint mismatch: {table}"
            ));
        }
    }
    for (name, required) in [
        (
            "context_input_queue_release_order",
            "oncontext_input_queue(logical_role_instance_id,transition_revision,status,sequence)",
        ),
        (
            "legacy_context_input_dispositions_release_fence",
            "onlegacy_context_input_dispositions(logical_role_instance_id,transition_revision,queue_id)",
        ),
        (
            "legacy_context_input_dispositions_successor_delivery",
            "onlegacy_context_input_dispositions(successor_delivery_identity)wheresuccessor_delivery_identityisnotnull",
        ),
    ] {
        let (sql, unique): (String, i64) = conn.query_row(
            "SELECT sql, CASE WHEN sql LIKE 'CREATE UNIQUE INDEX%' THEN 1 ELSE 0 END FROM sqlite_schema WHERE type='index' AND name=?1",
            [name], |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| format!("copied schema-12 index absent: {name}: {e}"))?;
        let compact: String = sql
            .chars()
            .filter(|c| !c.is_ascii_whitespace())
            .flat_map(char::to_lowercase)
            .collect();
        let expected_unique =
            i64::from(name == "legacy_context_input_dispositions_successor_delivery");
        if unique != expected_unique || !compact.contains(required) {
            return fail(format!("copied schema-12 index mismatch: {name}"));
        }
    }
    Ok(())
}

fn cell_string(cell: ValueRef<'_>) -> T<Option<String>> {
    match cell {
        ValueRef::Null => Ok(None),
        ValueRef::Text(value) => Ok(Some(
            String::from_utf8(value.to_vec()).map_err(|_| "non-UTF-8 SQLite text")?,
        )),
        ValueRef::Integer(value) => Ok(Some(value.to_string())),
        _ => fail("unsupported SQLite value type in identity/status projection"),
    }
}

fn source_row_hash(row: &rusqlite::Row<'_>, count: usize) -> T<String> {
    let mut hasher = Sha256::new();
    for index in 0..count {
        let (tag, bytes): (u8, Vec<u8>) = match row.get_ref(index).map_err(|e| e.to_string())? {
            ValueRef::Null => (0, Vec::new()),
            ValueRef::Integer(n) => (1, n.to_le_bytes().to_vec()),
            ValueRef::Real(_) => return fail("unsupported real-valued schema-12 projection"),
            ValueRef::Text(value) => (2, value.to_vec()),
            ValueRef::Blob(value) => (3, value.to_vec()),
        };
        hasher.update([tag]);
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn scan_rows(conn: &Connection) -> T<Vec<InventoryRow>> {
    let mut inventory = Vec::new();
    let mut seen = BTreeSet::new();
    for spec in TABLES {
        let sql = format!(
            "SELECT {} FROM {} ORDER BY {}",
            spec.columns.join(","),
            spec.name,
            spec.key
        );
        let mut statement = conn
            .prepare(&sql)
            .map_err(|e| format!("{}: {e}", spec.name))?;
        let key_index = spec
            .columns
            .iter()
            .position(|column| *column == spec.key)
            .ok_or("schema key not found")?;
        let revision_index = spec
            .revision
            .and_then(|name| spec.columns.iter().position(|column| *column == name));
        let status_index = spec
            .status
            .and_then(|name| spec.columns.iter().position(|column| *column == name));
        let mut cursor = statement.query([]).map_err(|e| e.to_string())?;
        while let Some(row) = cursor.next().map_err(|e| e.to_string())? {
            let key = cell_string(row.get_ref(key_index).map_err(|e| e.to_string())?)?
                .ok_or("null row identity")?;
            if !seen.insert((spec.name, key.clone())) {
                return fail(format!("duplicate identity in {}", spec.name));
            }
            let revision = match revision_index {
                Some(index) => cell_string(row.get_ref(index).map_err(|e| e.to_string())?)?,
                None => None,
            };
            let status = match status_index {
                Some(index) => cell_string(row.get_ref(index).map_err(|e| e.to_string())?)?,
                None => None,
            };
            let benign_snapshot = matches!(
                spec.name,
                "legacy_context_input_custody_lineages"
                    | "legacy_context_input_custody_owners"
                    | "active_turn_lifecycle_observations"
            ) || (spec.name == "context_input_admissions"
                && status.as_deref() == Some("closed"))
                || (spec.name == "active_turn_lifecycle"
                    && status.as_deref() == Some("reconciled"));
            inventory.push(InventoryRow {
                table: spec.name.to_string(),
                identity: key,
                revision,
                status,
                source_projection_sha256: source_row_hash(row, spec.columns.len())?,
                classification: if benign_snapshot {
                    "historical_observation"
                } else {
                    "unclassified"
                }
                .to_string(),
                reason: if benign_snapshot {
                    None
                } else {
                    Some(
                        "lineage or effect not conclusively settled by copied evidence".to_string(),
                    )
                },
            });
        }
    }
    Ok(inventory)
}

struct Queue {
    queue_id: String,
    role: String,
    transition: String,
    client_message: String,
    input_revision: String,
    input_bytes: Vec<u8>,
    status: String,
    released_at: Option<String>,
    delivery_bytes: Option<Vec<u8>>,
}

struct Disposition {
    queue_id: String,
    role: String,
    thread: String,
    binding: i64,
    transition: String,
    client_message: String,
    input_revision: String,
    input_sha256: String,
    kind: String,
    inventory_revision: String,
    authority: String,
    acceptance: Option<String>,
    successor: Option<String>,
    delivery_identity: Option<String>,
    record_revision: String,
    record_bytes: Vec<u8>,
}

#[derive(Serialize)]
struct QReference {
    queue_id: String,
    input_revision: String,
    disposition_revision: String,
    authority_revision: String,
    successor_acceptance_revision: Option<String>,
    successor_logical_role_instance_id: Option<String>,
    successor_delivery_identity: Option<String>,
    physical_delivery: String,
}

struct QEvidence {
    reference: QReference,
    input_bytes: Vec<u8>,
    record_bytes: Vec<u8>,
}

struct Classified {
    inventory: Vec<InventoryRow>,
    q3: QEvidence,
    q4: QEvidence,
    local_owner: Value,
}

fn classify_row(
    inventory: &mut [InventoryRow],
    table: &str,
    identity: &str,
    class: &str,
    reason: Option<&str>,
) -> T<()> {
    let item = inventory
        .iter_mut()
        .find(|item| item.table == table && item.identity == identity)
        .ok_or_else(|| format!("missing inventory row {table}/{identity}"))?;
    item.classification = class.to_string();
    item.reason = reason.map(str::to_string);
    Ok(())
}

fn registry_rows(capture: &Loaded, dispositions: &[Disposition]) -> T<Vec<InventoryRow>> {
    let mut result = Vec::new();
    let bindings = capture.registry["bindings"]
        .as_object()
        .ok_or("registry bindings invalid")?;
    for (key, entry) in bindings {
        exact_keys(
            entry,
            &[
                "logicalRoleInstanceId",
                "provider",
                "threadId",
                "protocolVersion",
                "environmentFingerprint",
                "bindingRevision",
                "boundAt",
            ],
            "registry binding",
        )?;
        if legacy_required_text(entry, "logicalRoleInstanceId")? != key
            || string(entry, "provider")? != "codex-app-server"
            || entry["bindingRevision"].as_u64().is_none_or(|v| v == 0)
        {
            return fail("registry binding key/provider/revision mismatch");
        }
        for name in ["threadId", "protocolVersion", "environmentFingerprint"] {
            let _ = legacy_required_text(entry, name)?;
        }
        let _ = nonempty(entry, "boundAt")?;
        let related: Vec<_> = dispositions.iter().filter(|row| row.role == *key).collect();
        let exact_link = related.iter().any(|row| {
            row.thread == string(entry, "threadId").unwrap_or("")
                && Some(row.binding as u64) == entry["bindingRevision"].as_u64()
        });
        result.push(InventoryRow {
            table: "role-bindings.json/bindings".to_string(),
            identity: key.clone(),
            revision: Some(entry["bindingRevision"].to_string()),
            status: Some("bound".to_string()),
            source_projection_sha256: sha(canonical(entry)?.as_bytes()),
            classification: "unclassified".to_string(),
            reason: Some(
                if exact_link {
                    "copied binding matches a legacy disposition identity; current custody not inferred"
                } else if !related.is_empty() {
                    "copied binding overlaps legacy role but thread/revision differs; blocked lineage"
                } else {
                    "copied binding has no established translated obligation lineage"
                }
                .to_string(),
            ),
        });
    }
    let deliveries = capture.registry["deliveries"]
        .as_object()
        .ok_or("registry deliveries invalid")?;
    for (key, entry) in deliveries {
        let key_value = parsed_json(key.as_bytes(), "registry delivery key")?;
        let pair = key_value
            .as_array()
            .ok_or("registry delivery key not pair")?;
        if pair.len() != 2 {
            return fail("registry delivery key size invalid");
        }
        let required = [
            "logicalRoleInstanceId",
            "clientUserMessageId",
            "threadId",
            "requestFingerprint",
            "status",
            "turnId",
            "startedAt",
            "completedAt",
        ];
        let mut allowed = required.to_vec();
        if entry.get("terminalOutput").is_some() {
            allowed.push("terminalOutput");
        }
        exact_keys(entry, &allowed, "registry delivery")?;
        if pair[0].as_str() != Some(legacy_required_text(entry, "logicalRoleInstanceId")?)
            || pair[1].as_str() != Some(legacy_required_text(entry, "clientUserMessageId")?)
        {
            return fail("registry delivery key/entry identity mismatch");
        }
        for name in ["threadId", "requestFingerprint"] {
            let _ = legacy_required_text(entry, name)?;
        }
        let _ = nonempty(entry, "startedAt")?;
        let status = string(entry, "status")?;
        match status {
            "pending"
                if entry["turnId"].is_null()
                    && entry["completedAt"].is_null()
                    && entry.get("terminalOutput").is_none() => {}
            "completed"
                if entry["turnId"]
                    .as_str()
                    .is_some_and(|s| !s.chars().all(js_whitespace))
                    && entry["completedAt"]
                        .as_str()
                        .is_some_and(|s| !s.trim().is_empty()) => {}
            _ => return fail("registry delivery status/projection invalid"),
        }
        if let Some(terminal) = entry.get("terminalOutput")
            && !terminal.is_object()
        {
            return fail("registry terminal output invalid");
        }
        let exact_link = dispositions.iter().any(|row| {
            row.role == string(entry, "logicalRoleInstanceId").unwrap_or("")
                && row.client_message == string(entry, "clientUserMessageId").unwrap_or("")
                && row.thread == string(entry, "threadId").unwrap_or("")
        });
        result.push(InventoryRow { table:"role-bindings.json/deliveries".to_string(), identity:key.clone(),
            revision:None, status:Some(status.to_string()),
            source_projection_sha256:sha(canonical(entry)?.as_bytes()),
            classification:"unclassified".to_string(),
            reason:Some(if exact_link {"copied registry delivery matches legacy target identity; physical effect still unproved"}
                else {"copied registry delivery has no exact legacy target lineage; blocked"}.to_string()) });
    }
    Ok(result)
}

fn load_queues(conn: &Connection) -> T<BTreeMap<String, Queue>> {
    let mut statement = conn
        .prepare(
            "SELECT queue_id,logical_role_instance_id,transition_revision,client_user_message_id,\
         input_revision,CAST(input_json AS BLOB),status,released_at,CAST(delivery_json AS BLOB) FROM context_input_queue ORDER BY sequence",
        )
        .map_err(|e| e.to_string())?;
    let mut cursor = statement.query([]).map_err(|e| e.to_string())?;
    let mut queues = BTreeMap::new();
    while let Some(row) = cursor.next().map_err(|e| e.to_string())? {
        let item = Queue {
            queue_id: row.get(0).map_err(|e| e.to_string())?,
            role: row.get(1).map_err(|e| e.to_string())?,
            transition: row.get(2).map_err(|e| e.to_string())?,
            client_message: row.get(3).map_err(|e| e.to_string())?,
            input_revision: row.get(4).map_err(|e| e.to_string())?,
            input_bytes: row.get(5).map_err(|e| e.to_string())?,
            status: row.get(6).map_err(|e| e.to_string())?,
            released_at: row.get(7).map_err(|e| e.to_string())?,
            delivery_bytes: row.get(8).map_err(|e| e.to_string())?,
        };
        if !matches!(item.status.as_str(), "queued" | "releasing" | "released") {
            return fail("unsupported context queue status");
        }
        if queues.insert(item.queue_id.clone(), item).is_some() {
            return fail("duplicate context queue identity");
        }
    }
    Ok(queues)
}

fn load_dispositions(conn: &Connection) -> T<Vec<Disposition>> {
    let mut statement = conn.prepare(
        "SELECT queue_id,logical_role_instance_id,thread_id,binding_revision,transition_revision,\
         client_user_message_id,input_revision,input_json_sha256,disposition,canonical_inventory_revision,authority_revision,\
         successor_acceptance_revision,successor_logical_role_instance_id,successor_delivery_identity,\
         record_revision,CAST(record_json AS BLOB) FROM legacy_context_input_dispositions ORDER BY queue_id"
    ).map_err(|e| e.to_string())?;
    let mut cursor = statement.query([]).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    while let Some(row) = cursor.next().map_err(|e| e.to_string())? {
        result.push(Disposition {
            queue_id: row.get(0).map_err(|e| e.to_string())?,
            role: row.get(1).map_err(|e| e.to_string())?,
            thread: row.get(2).map_err(|e| e.to_string())?,
            binding: row.get(3).map_err(|e| e.to_string())?,
            transition: row.get(4).map_err(|e| e.to_string())?,
            client_message: row.get(5).map_err(|e| e.to_string())?,
            input_revision: row.get(6).map_err(|e| e.to_string())?,
            input_sha256: row.get(7).map_err(|e| e.to_string())?,
            kind: row.get(8).map_err(|e| e.to_string())?,
            inventory_revision: row.get(9).map_err(|e| e.to_string())?,
            authority: row.get(10).map_err(|e| e.to_string())?,
            acceptance: row.get(11).map_err(|e| e.to_string())?,
            successor: row.get(12).map_err(|e| e.to_string())?,
            delivery_identity: row.get(13).map_err(|e| e.to_string())?,
            record_revision: row.get(14).map_err(|e| e.to_string())?,
            record_bytes: row.get(15).map_err(|e| e.to_string())?,
        });
    }
    Ok(result)
}

fn validate_disposition(disposition: &Disposition, queue: &Queue) -> T<()> {
    if disposition.queue_id != queue.queue_id
        || disposition.role != queue.role
        || disposition.transition != queue.transition
        || disposition.client_message != queue.client_message
        || disposition.input_revision != queue.input_revision
        || disposition.input_sha256 != sha(&queue.input_bytes)
        || disposition.binding <= 0
    {
        return fail("disposition/queue identity or exact input-byte digest mismatch");
    }
    let input = parsed_json(&queue.input_bytes, "context input")?;
    check_input(&input, queue)?;
    if string(&input, "threadId")? != disposition.thread
        || input.get("bindingRevision").and_then(Value::as_i64) != Some(disposition.binding)
    {
        return fail("disposition/input thread or binding mismatch");
    }
    let record = parsed_json(&disposition.record_bytes, "legacy disposition")?;
    exact_keys(
        &record,
        &[
            "schemaVersion",
            "type",
            "canonicalInventoryRevision",
            "target",
            "disposition",
            "authorityRevision",
            "evidenceRefs",
            "successorAcceptance",
            "recordRevision",
        ],
        "legacy disposition",
    )?;
    if record.get("schemaVersion").and_then(Value::as_u64) != Some(1)
        || string(&record, "type")? != "work-engine.legacy-context-input-disposition"
        || sha_revision(&record, "canonicalInventoryRevision")? != disposition.inventory_revision
        || sha_revision(&record, "authorityRevision")? != disposition.authority
        || sha_revision(&record, "recordRevision")? != disposition.record_revision
        || !matches!(
            disposition.kind.as_str(),
            "terminally_retired" | "withdrawn" | "durable_restart"
        )
    {
        return fail("legacy disposition envelope or inventory projection invalid");
    }
    let refs = field(&record, "evidenceRefs")?
        .as_array()
        .ok_or("disposition evidence refs not an array")?;
    if refs.is_empty()
        || refs
            .iter()
            .any(|item| item.as_str().is_none_or(|s| s.chars().all(js_whitespace)))
        || refs
            .windows(2)
            .any(|pair| pair[0].as_str() >= pair[1].as_str())
    {
        return fail("disposition evidence refs not canonical/nonempty");
    }
    if string(&record, "recordRevision")? != disposition.record_revision
        || revision_without(&record, "recordRevision")? != disposition.record_revision
        || string(&record, "disposition")? != disposition.kind
        || string(&record, "authorityRevision")? != disposition.authority
    {
        return fail("legacy disposition codec/revision mismatch");
    }
    let target = field(&record, "target")?;
    exact_keys(
        target,
        &[
            "queueId",
            "logicalRoleInstanceId",
            "threadId",
            "bindingRevision",
            "transitionRevision",
            "clientUserMessageId",
            "inputRevision",
            "inputJsonSha256",
        ],
        "disposition target",
    )?;
    for name in [
        "queueId",
        "logicalRoleInstanceId",
        "threadId",
        "clientUserMessageId",
    ] {
        let _ = legacy_required_text(target, name)?;
    }
    if !is_revision(&queue.transition)
        || !is_revision(&disposition.transition)
        || !is_revision(string(target, "transitionRevision")?)
    {
        return fail("legacy transition revision shape invalid");
    }
    for (json_name, expected) in [
        ("queueId", disposition.queue_id.as_str()),
        ("logicalRoleInstanceId", disposition.role.as_str()),
        ("threadId", disposition.thread.as_str()),
        ("transitionRevision", disposition.transition.as_str()),
        ("clientUserMessageId", disposition.client_message.as_str()),
        ("inputRevision", disposition.input_revision.as_str()),
        ("inputJsonSha256", disposition.input_sha256.as_str()),
    ] {
        if string(target, json_name)? != expected {
            return fail(format!("legacy disposition target mismatch: {json_name}"));
        }
    }
    if target.get("bindingRevision").and_then(Value::as_i64) != Some(disposition.binding) {
        return fail("legacy disposition target binding mismatch");
    }
    let acceptance = field(&record, "successorAcceptance")?;
    if disposition.kind == "durable_restart" {
        exact_keys(
            acceptance,
            &[
                "schemaVersion",
                "type",
                "canonicalInventoryRevision",
                "target",
                "successorLogicalRoleInstanceId",
                "successorSchedulerIdentity",
                "successorDeliveryIdentity",
                "transferAuthorityRevision",
                "acceptedAt",
                "acceptanceRevision",
            ],
            "successor acceptance",
        )?;
        if acceptance.get("schemaVersion").and_then(Value::as_u64) != Some(1)
            || string(acceptance, "type")? != "work-engine.context-input-successor-acceptance"
            || sha_revision(acceptance, "canonicalInventoryRevision")?
                != disposition.inventory_revision
            || sha_revision(acceptance, "transferAuthorityRevision")? != disposition.authority
            || sha_revision(acceptance, "acceptanceRevision")? == disposition.authority
            || legacy_required_text(acceptance, "successorLogicalRoleInstanceId")?
                == disposition.role
            || legacy_required_text(acceptance, "successorLogicalRoleInstanceId")?
                == disposition.queue_id
        {
            return fail("successor acceptance envelope invalid");
        }
        for name in ["successorSchedulerIdentity", "successorDeliveryIdentity"] {
            let _ = legacy_required_text(acceptance, name)?;
        }
        if !supported_legacy_utc_timestamp(string(acceptance, "acceptedAt")?) {
            return fail("successor acceptedAt unsupported or invalid UTC timestamp");
        }
        if disposition.acceptance.as_deref() != Some(string(acceptance, "acceptanceRevision")?)
            || revision_without(acceptance, "acceptanceRevision")?
                != string(acceptance, "acceptanceRevision")?
            || disposition.successor.as_deref()
                != Some(string(acceptance, "successorLogicalRoleInstanceId")?)
            || disposition.delivery_identity.as_deref()
                != Some(string(acceptance, "successorDeliveryIdentity")?)
            || string(acceptance, "transferAuthorityRevision")? != disposition.authority
            || field(acceptance, "target")? != target
        {
            return fail("successor acceptance does not bind Q4 disposition");
        }
    } else if !acceptance.is_null()
        || disposition.acceptance.is_some()
        || disposition.successor.is_some()
        || disposition.delivery_identity.is_some()
    {
        return fail("terminal legacy disposition carries successor fields");
    }
    Ok(())
}

struct LocalCustody {
    projection: Value,
    queue_ids: BTreeSet<String>,
    disposition_revisions: BTreeSet<String>,
    inventory_revision: String,
}

fn local_owner(conn: &Connection, source_root: &str) -> T<LocalCustody> {
    let mut statement = conn.prepare(
        "SELECT custody_lineage_id,owner_revision,owner_generation_id,owner_state_root,owner_epoch,state,CAST(owner_json AS BLOB) \
         FROM legacy_context_input_custody_owners ORDER BY custody_lineage_id"
    ).map_err(|e| e.to_string())?;
    let mut cursor = statement.query([]).map_err(|e| e.to_string())?;
    let row = cursor
        .next()
        .map_err(|e| e.to_string())?
        .ok_or("no copied local custody owner")?;
    let lineage_id: String = row.get(0).map_err(|e| e.to_string())?;
    let owner_revision: String = row.get(1).map_err(|e| e.to_string())?;
    let generation: String = row.get(2).map_err(|e| e.to_string())?;
    let root: String = row.get(3).map_err(|e| e.to_string())?;
    let epoch: i64 = row.get(4).map_err(|e| e.to_string())?;
    let state: String = row.get(5).map_err(|e| e.to_string())?;
    let owner_bytes: Vec<u8> = row.get(6).map_err(|e| e.to_string())?;
    if cursor.next().map_err(|e| e.to_string())?.is_some() || epoch <= 0 {
        return fail("ambiguous or invalid copied local custody owner");
    }
    let owner = parsed_json(&owner_bytes, "legacy custody owner")?;
    exact_keys(
        &owner,
        &[
            "schemaVersion",
            "type",
            "custodyLineageId",
            "ownerGenerationId",
            "ownerStateRoot",
            "ownerEpoch",
            "state",
            "handoffRevision",
            "capsuleRevision",
            "predecessorOwnerRevision",
            "successorOwner",
            "manifestOwnershipRevision",
            "ownerRevision",
        ],
        "legacy custody owner",
    )?;
    if owner["schemaVersion"] != 1
        || string(&owner, "type")? != "work-engine.legacy-context-input-custody-owner"
        || sha_revision(&owner, "ownerRevision")? != owner_revision
        || revision_without(&owner, "ownerRevision")? != owner_revision
        || sha_revision(&owner, "custodyLineageId")? != lineage_id
        || legacy_required_text(&owner, "ownerGenerationId")? != generation
        || legacy_required_text(&owner, "ownerStateRoot")? != root
        || owner["ownerEpoch"].as_i64() != Some(epoch)
        || string(&owner, "state")? != state
        || root != source_root
    {
        return fail("legacy custody owner revision, scalar projection, or source-root mismatch");
    }
    if state != "owned" || !owner["successorOwner"].is_null() {
        return fail("copied custody owner is not a local owned, untransferred source");
    }
    for key in [
        "handoffRevision",
        "capsuleRevision",
        "predecessorOwnerRevision",
    ] {
        if !owner[key].is_null() && !is_revision(string(&owner, key)?) {
            return fail(format!("invalid owner {key}"));
        }
    }
    if !owner["manifestOwnershipRevision"].is_null() {
        let digest = string(&owner, "manifestOwnershipRevision")?;
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return fail("invalid owner manifest ownership digest");
        }
    }
    let mut lineages = conn.prepare("SELECT custody_lineage_id,CAST(lineage_json AS BLOB) FROM legacy_context_input_custody_lineages ORDER BY custody_lineage_id").map_err(|e| e.to_string())?;
    let mut rows = lineages.query([]).map_err(|e| e.to_string())?;
    let lineage_row = rows
        .next()
        .map_err(|e| e.to_string())?
        .ok_or("missing custody lineage")?;
    let scalar_id: String = lineage_row.get(0).map_err(|e| e.to_string())?;
    let lineage_bytes: Vec<u8> = lineage_row.get(1).map_err(|e| e.to_string())?;
    if rows.next().map_err(|e| e.to_string())?.is_some() || scalar_id != lineage_id {
        return fail("ambiguous custody lineage");
    }
    let lineage = parsed_json(&lineage_bytes, "legacy custody lineage")?;
    exact_keys(
        &lineage,
        &[
            "schemaVersion",
            "type",
            "canonicalInventoryRevision",
            "queueIds",
            "dispositionRecordRevisions",
            "sourceGenerationId",
            "custodyLineageId",
        ],
        "legacy custody lineage",
    )?;
    if lineage["schemaVersion"] != 1
        || string(&lineage, "type")? != "work-engine.legacy-context-input-custody-lineage"
        || sha_revision(&lineage, "custodyLineageId")? != lineage_id
        || revision_without(&lineage, "custodyLineageId")? != lineage_id
        || legacy_required_text(&lineage, "sourceGenerationId")? != generation
        || !is_revision(string(&lineage, "canonicalInventoryRevision")?)
    {
        return fail("legacy custody lineage revision or source generation mismatch");
    }
    let ids = field(&lineage, "queueIds")?
        .as_array()
        .ok_or("lineage queueIds invalid")?;
    let revisions = field(&lineage, "dispositionRecordRevisions")?
        .as_array()
        .ok_or("lineage disposition revisions invalid")?;
    if ids.is_empty() || revisions.len() != ids.len() {
        return fail("lineage membership sizes invalid");
    }
    let queue_ids: BTreeSet<_> = ids
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();
    let disposition_revisions: BTreeSet<_> = revisions
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();
    if queue_ids.len() != ids.len()
        || disposition_revisions.len() != revisions.len()
        || queue_ids.contains("")
        || disposition_revisions.iter().any(|s| !is_revision(s))
        || ids.windows(2).any(|w| w[0].as_str() >= w[1].as_str())
        || revisions.windows(2).any(|w| w[0].as_str() >= w[1].as_str())
    {
        return fail("lineage identities not sorted, unique, or valid");
    }
    Ok(LocalCustody {
        projection: json!({
            "custody_lineage_id":lineage_id,"owner_revision":owner_revision,
            "owner_generation_id":generation,"owner_state_root_provenance_only":root,
            "owner_epoch":epoch,"state":state,"current_selection_established":false,
        }),
        queue_ids,
        disposition_revisions,
        inventory_revision: string(&lineage, "canonicalInventoryRevision")?.to_string(),
    })
}

fn classify(conn: &Connection, capture: &Loaded) -> T<Classified> {
    let mut inventory = scan_rows(conn)?;
    let queues = load_queues(conn)?;
    let dispositions = load_dispositions(conn)?;
    inventory.extend(registry_rows(capture, &dispositions)?);
    let mut q3 = None;
    let mut q4 = None;
    let mut disposition_keys = BTreeSet::new();
    let mut inventory_revisions = BTreeSet::new();
    for disposition in dispositions {
        inventory_revisions.insert(disposition.inventory_revision.clone());
        if !disposition_keys.insert(disposition.queue_id.clone()) {
            return fail("duplicate legacy disposition queue identity");
        }
        let queue = queues
            .get(&disposition.queue_id)
            .ok_or("disposition references absent queue")?;
        validate_disposition(&disposition, queue)?;
        let mut delivery_statement = conn.prepare(
            "SELECT claim_revision,CAST(claim_json AS BLOB),status,delivery_revision,\
             CAST(delivery_json AS BLOB) FROM legacy_context_input_successor_deliveries WHERE queue_id=?1"
        ).map_err(|e| e.to_string())?;
        let mut delivery_rows = delivery_statement
            .query([&disposition.queue_id])
            .map_err(|e| e.to_string())?;
        let delivery = if let Some(row) = delivery_rows.next().map_err(|e| e.to_string())? {
            let status: String = row.get(2).map_err(|e| e.to_string())?;
            if !matches!(status.as_str(), "pending" | "completed") {
                return fail("unsupported successor delivery status");
            }
            Some(status)
        } else {
            None
        };
        if delivery_rows.next().map_err(|e| e.to_string())?.is_some() {
            return fail("duplicate successor delivery rows");
        }
        if queue.status == "queued"
            && (queue.released_at.is_some() || queue.delivery_bytes.is_some())
        {
            return fail("queued legacy input carries contradictory release evidence");
        }
        if queue.status == "released"
            && (queue.released_at.is_none() || queue.delivery_bytes.is_none())
        {
            return fail("released legacy input lacks exact release evidence");
        }
        if let Some(bytes) = &queue.delivery_bytes {
            let _ = parsed_json(bytes, "legacy queue delivery receipt")?;
        }
        let release_unresolved = queue.status != "queued"
            || queue.released_at.is_some()
            || queue.delivery_bytes.is_some();
        let (class, reason, physical) = if release_unresolved {
            (
                "unclassified",
                Some(
                    "legacy queue release/delivery evidence requires separate exact completed-effect validation; Q3 no replay",
                ),
                "unresolved",
            )
        } else {
            match disposition.kind.as_str() {
                "terminally_retired" if delivery.is_none() => {
                    ("terminally_retired_no_replay", None, "not_applicable")
                }
                "durable_restart" if delivery.is_none() => {
                    ("pending_accepted_undelivered", None, "not_proved")
                }
                "durable_restart" => (
                    "unclassified",
                    Some("successor delivery row requires separate completed-effect validation"),
                    "unresolved",
                ),
                "withdrawn" if delivery.is_none() => {
                    ("historical_observation", None, "not_applicable")
                }
                _ => return fail("unsupported or conflicting legacy disposition/delivery state"),
            }
        };
        classify_row(
            &mut inventory,
            "context_input_queue",
            &queue.queue_id,
            class,
            reason,
        )?;
        classify_row(
            &mut inventory,
            "legacy_context_input_dispositions",
            &disposition.queue_id,
            class,
            reason,
        )?;
        if delivery.is_some() {
            classify_row(
                &mut inventory,
                "legacy_context_input_successor_deliveries",
                &disposition.queue_id,
                class,
                reason,
            )?;
        }
        let evidence = QEvidence {
            reference: QReference {
                queue_id: disposition.queue_id.clone(),
                input_revision: disposition.input_revision,
                disposition_revision: disposition.record_revision,
                authority_revision: disposition.authority,
                successor_acceptance_revision: disposition.acceptance,
                successor_logical_role_instance_id: disposition.successor,
                successor_delivery_identity: disposition.delivery_identity,
                physical_delivery: physical.to_string(),
            },
            input_bytes: queue.input_bytes.clone(),
            record_bytes: disposition.record_bytes,
        };
        match disposition.kind.as_str() {
            "terminally_retired" => {
                if q3.replace(evidence).is_some() {
                    return fail("more than one retired Q3 disposition");
                }
            }
            "durable_restart" => {
                if q4.replace(evidence).is_some() {
                    return fail("more than one held Q4 disposition");
                }
            }
            _ => {}
        }
    }
    let q3 = q3.ok_or("Q3 terminally retired disposition absent")?;
    let q4 = q4.ok_or("Q4 durable restart disposition absent")?;
    let owner = local_owner(conn, &capture.source_root)?;
    let observed_revisions: BTreeSet<String> = inventory
        .iter()
        .filter(|row| row.table == "legacy_context_input_dispositions")
        .filter_map(|row| row.revision.clone())
        .collect();
    if owner.queue_ids != disposition_keys
        || owner.disposition_revisions != observed_revisions
        || inventory_revisions.len() != 1
        || !inventory_revisions.contains(&owner.inventory_revision)
    {
        return fail("custody lineage does not exhaust queue/disposition identities");
    }
    Ok(Classified {
        inventory,
        q3,
        q4,
        local_owner: owner.projection,
    })
}

fn write_bytes(path: &Path, bytes: &[u8]) -> T<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| format!("create {}: {e}", path.display()))?;
    file.write_all(bytes)
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    synthetic_fault("file_sync")?;
    file.sync_all()
        .map_err(|e| format!("sync {}: {e}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> T<(usize, String)> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    write_bytes(path, &bytes)?;
    Ok((bytes.len(), sha(&bytes)))
}

fn sync_directory(path: &Path) -> T<()> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| format!("sync directory {}: {e}", path.display()))
}

fn stage_root(output_root: &Path) -> T<PathBuf> {
    if !output_root.is_absolute() {
        return fail("output root must be absolute");
    }
    match fs::symlink_metadata(output_root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Ok(_) => return fail("output root must be absent"),
        Err(error) => return fail(format!("output root metadata failed: {error}")),
    }
    let parent = output_root.parent().ok_or("output root has no parent")?;
    checked_dir(parent)?;
    let name = output_root
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("output basename invalid")?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    let stage = parent.join(format!(
        ".{name}.s9-a-{}-{}",
        std::process::id(),
        now.as_nanos()
    ));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&stage)
        .map_err(|e| format!("private staging create {}: {e}", stage.display()))?;
    Ok(stage)
}

// Linux no-replace directory rename: the target must not have appeared since
// the initial absence check. An ordinary rename could replace an empty target.
unsafe extern "C" {
    fn renameat2(oldfd: i32, oldpath: *const i8, newfd: i32, newpath: *const i8, flags: u32)
    -> i32;
}

fn rename_no_replace(stage: &Path, target: &Path) -> T<()> {
    const AT_FDCWD: i32 = -100;
    const RENAME_NOREPLACE: u32 = 1;
    let old =
        CString::new(stage.as_os_str().as_bytes()).map_err(|_| "staging path contains NUL")?;
    let new =
        CString::new(target.as_os_str().as_bytes()).map_err(|_| "output path contains NUL")?;
    let result = unsafe {
        renameat2(
            AT_FDCWD,
            old.as_ptr(),
            AT_FDCWD,
            new.as_ptr(),
            RENAME_NOREPLACE,
        )
    };
    if result != 0 {
        return fail(format!(
            "no-overwrite publication failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

fn synthetic_fault(label: &str) -> T<()> {
    if cfg!(debug_assertions)
        && std::env::var_os("S9_A_SYNTHETIC_TEST").as_deref() == Some(std::ffi::OsStr::new("1"))
        && std::env::var("S9_A_TEST_FAULT")
            .ok()
            .is_some_and(|faults| faults.split(',').any(|fault| fault == label))
    {
        return fail(format!("synthetic injected {label} failure"));
    }
    Ok(())
}

fn synthetic_barrier(label: &str) -> T<()> {
    if cfg!(debug_assertions)
        && std::env::var_os("S9_A_SYNTHETIC_TEST").as_deref() == Some(std::ffi::OsStr::new("1"))
        && std::env::var("S9_A_TEST_BARRIER").ok().as_deref() == Some(label)
    {
        let marker = PathBuf::from(
            std::env::var_os("S9_A_TEST_BARRIER_MARKER")
                .ok_or("synthetic barrier marker missing")?,
        );
        fs::write(&marker, label).map_err(|e| e.to_string())?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while marker.exists() {
            if std::time::Instant::now() >= deadline {
                return fail(format!("synthetic {label} barrier deadline"));
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    Ok(())
}

fn exact_child_names(dir: &Path, expected: &[&str]) -> T<()> {
    checked_dir(dir)?;
    let observed: BTreeSet<_> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .map(|entry| {
            entry
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .map_err(|e| e.to_string())
        })
        .collect::<Result<_, _>>()?;
    let expected: BTreeSet<_> = expected.iter().map(|name| (*name).to_string()).collect();
    if observed != expected {
        return fail(format!("output file-set mismatch in {}", dir.display()));
    }
    Ok(())
}

// Content verification deliberately does not require a future successful
// return value. It runs before that value can exist, after rename and sync.
fn verify_content(attempt_root: &Path, output: &Path, expected: &TranslationReceipt) -> T<()> {
    let capture = validate_capture(attempt_root)?;
    exact_child_names(
        output,
        &[
            "capsule.json",
            "raw",
            "inventory-private.json",
            "report-redacted.json",
            "manifest.json",
            "receipt.json",
        ],
    )?;
    exact_child_names(
        &output.join("raw"),
        &[
            "q3-input.json",
            "q3-disposition.json",
            "q4-input.json",
            "q4-disposition.json",
        ],
    )?;
    for dir in [output.to_path_buf(), output.join("raw")] {
        if fs::metadata(&dir)
            .map_err(|e| e.to_string())?
            .permissions()
            .mode()
            & 0o777
            != 0o700
        {
            return fail("output directory mode mismatch");
        }
    }
    let manifest_bytes = checked_file(&output.join("manifest.json"), 4_000_000)?;
    for name in ["manifest.json", "receipt.json"] {
        if fs::symlink_metadata(output.join(name))
            .map_err(|e| e.to_string())?
            .permissions()
            .mode()
            & 0o777
            != 0o600
        {
            return fail(format!("output {name} mode mismatch"));
        }
    }
    if sha(&manifest_bytes) != expected.manifest_sha256 {
        return fail("output manifest digest mismatch");
    }
    let manifest = parsed_json(&manifest_bytes, "output manifest")?;
    if manifest["schema_version"] != 1 {
        return fail("output manifest version unsupported");
    }
    let files = manifest["files"]
        .as_array()
        .ok_or("output manifest files invalid")?;
    let expected_names = [
        "capsule.json",
        "inventory-private.json",
        "raw/q3-disposition.json",
        "raw/q3-input.json",
        "raw/q4-disposition.json",
        "raw/q4-input.json",
        "report-redacted.json",
    ];
    if files.len() != expected_names.len() {
        return fail("output manifest file count mismatch");
    }
    for (entry, name) in files.iter().zip(expected_names) {
        exact_keys(
            entry,
            &["path", "length", "sha256"],
            "output manifest entry",
        )?;
        if string(entry, "path")? != name {
            return fail("output manifest ordering/path mismatch");
        }
        let path = output.join(name);
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if metadata.permissions().mode() & 0o777 != 0o600 {
            return fail("output file mode mismatch");
        }
        let bytes = checked_file(&path, 128_000_000)?;
        if entry["length"].as_u64() != Some(bytes.len() as u64)
            || string(entry, "sha256")? != sha(&bytes)
        {
            return fail(format!("output manifest byte mismatch: {name}"));
        }
    }
    let (capsule, capsule_hash) = json_file(&output.join("capsule.json"), 16_000_000)?;
    let (receipt, receipt_hash) = json_file(&output.join("receipt.json"), 4_000_000)?;
    exact_keys(
        &receipt,
        &[
            "schema_version",
            "status",
            "started_at",
            "ended_at",
            "attempt_root",
            "output_root",
            "source_attempt_sha256",
            "source_child_receipt_sha256",
            "source_manifest_sha256",
            "manifest_sha256",
            "capsule_sha256",
            "code_identity",
            "toolchain",
            "argv",
            "transfer_eligibility",
            "primary_error",
            "cleanup_error",
            "authority_conferred",
        ],
        "translation receipt",
    )?;
    let identity = field(&receipt, "code_identity")?;
    exact_keys(
        identity,
        &["crate", "version", "executable_sha256"],
        "code identity",
    )?;
    let executable_hash = field(identity, "executable_sha256")?;
    let executable_hash_valid = executable_hash.is_null()
        || executable_hash.as_str().is_some_and(|value| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        });
    let toolchain = field(&receipt, "toolchain")?;
    let toolchain_valid = toolchain.is_null()
        || toolchain
            .as_str()
            .is_some_and(|value| !value.trim().is_empty());
    let argv_valid = receipt["argv"]
        .as_array()
        .is_some_and(|args| !args.is_empty() && args.iter().all(Value::is_string));
    if capsule_hash != expected.capsule_sha256
        || receipt_hash != expected.receipt_sha256
        || receipt["schema_version"] != 1
        || receipt["status"] != "qualified_translation_only"
        || receipt["attempt_root"] != json!(attempt_root)
        || receipt["output_root"] != json!(output)
        || receipt["primary_error"] != Value::Null
        || receipt["cleanup_error"] != Value::Null
        || !producer_timestamp(string(&receipt, "started_at")?)
        || !producer_timestamp(string(&receipt, "ended_at")?)
        || identity["crate"] != "lifecycle-admin"
        || identity["version"]
            .as_str()
            .is_none_or(|value| value.trim().is_empty())
        || !executable_hash_valid
        || !toolchain_valid
        || !argv_valid
        || receipt["manifest_sha256"] != expected.manifest_sha256
        || receipt["capsule_sha256"] != expected.capsule_sha256
        || receipt["source_attempt_sha256"] != capture.outer_hash
        || receipt["source_child_receipt_sha256"] != capture.child_hash
        || receipt["source_manifest_sha256"] != capture.manifest_hash
        || receipt["source_attempt_sha256"] != expected.source_attempt_sha256
        || capsule["source_attempt_sha256"] != capture.outer_hash
        || capsule["source_child_receipt_sha256"] != capture.child_hash
        || capsule["source_manifest_sha256"] != capture.manifest_hash
        || capsule["selection_claim"] != "current_selection_unestablished"
        || capsule["schema_version"] != "s9-v28-obligations-v1"
        || capsule["translation_status"] != expected.translation_status
        || capsule["copied_image_sha256"] != json!(capture.image_hashes)
        || capsule["no_rust_custody"] != true
        || capsule["no_admission"] != true
        || capsule["no_delivery"] != true
        || capsule["no_execution"] != true
        || capsule["transfer_eligibility"] != expected.transfer_eligibility
        || receipt["transfer_eligibility"] != expected.transfer_eligibility
        || receipt["authority_conferred"] != false
    {
        return fail("output receipt/capsule/capture readback mismatch");
    }
    postflight_image(&capture)
}

/// A later consumer must supply the already-returned successful result and
/// recheck its exact bytes. A directory alone is never publication evidence.
pub fn verify_completed_translation(
    attempt_root: &Path,
    output: &Path,
    result: &TranslationReceipt,
) -> Result<(), String> {
    if result.output_root != output
        || result.publication_status != "durable_after_parent_sync_and_readback"
    {
        return fail("successful invocation result missing or unbound");
    }
    let (outer, outer_hash) =
        capture_json_file(&attempt_root.join("attempt-receipt.json"), 4_000_000)?;
    if outer_hash != result.source_attempt_sha256 || outer["status"] != "qualified_candidate_root" {
        return fail("successful result source capture mismatch");
    }
    verify_content(attempt_root, output, result)
}

fn redacted(identity: &str) -> String {
    format!("{}…", identity.chars().take(12).collect::<String>())
}

fn current_executable_sha256() -> Option<String> {
    std::env::current_exe()
        .ok()
        .and_then(|path| fs::read(path).ok())
        .map(|bytes| sha(&bytes))
}

fn perform_translation(
    attempt_root: &Path,
    output_root: &Path,
    stage: &Path,
    started: &str,
) -> T<TranslationReceipt> {
    let capture = validate_capture(attempt_root)?;
    let conn = open_copy(&capture.db_path)?;
    let classified = classify(&conn, &capture)?;
    drop(conn);
    synthetic_barrier("before_final_image_readback")?;
    postflight_image(&capture)?;
    let unclassified = classified
        .inventory
        .iter()
        .filter(|row| row.classification == "unclassified")
        .count();
    let transfer_eligibility = if unclassified == 0 {
        "requires_separate_B_authority"
    } else {
        "blocked_unclassified"
    };
    let classification_digests: Vec<_> = classified
        .inventory
        .iter()
        .map(|row| {
            json!({
                "table": row.table,
                "identity_sha256": sha(row.identity.as_bytes()),
                "source_projection_sha256": row.source_projection_sha256,
                "classification": row.classification,
            })
        })
        .collect();
    let capsule = json!({
        "schema_version": "s9-v28-obligations-v1",
        "source_attempt_sha256": capture.outer_hash,
        "source_child_receipt_sha256": capture.child_hash,
        "source_manifest_sha256": capture.manifest_hash,
        "copied_image_sha256": capture.image_hashes,
        "local_owner_source_observation": classified.local_owner,
        "q3": classified.q3.reference,
        "q4": classified.q4.reference,
        "row_classification_digests": classification_digests,
        "translation_status": "qualified",
        "transfer_eligibility": transfer_eligibility,
        "selection_claim": capture.selection_claim,
        "coherence": capture.coherence,
        "no_rust_custody": true,
        "no_admission": true,
        "no_delivery": true,
        "no_execution": true,
    });
    let mut entries: BTreeMap<String, (usize, String)> = BTreeMap::new();
    entries.insert(
        "capsule.json".to_string(),
        write_json(&stage.join("capsule.json"), &capsule)?,
    );
    let raw_dir = stage.join("raw");
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&raw_dir)
        .map_err(|e| e.to_string())?;
    for (name, bytes) in [
        ("q3-input.json", &classified.q3.input_bytes),
        ("q3-disposition.json", &classified.q3.record_bytes),
        ("q4-input.json", &classified.q4.input_bytes),
        ("q4-disposition.json", &classified.q4.record_bytes),
    ] {
        write_bytes(&raw_dir.join(name), bytes)?;
        entries.insert(format!("raw/{name}"), (bytes.len(), sha(bytes)));
    }
    let inventory_value = serde_json::to_value(&classified.inventory).map_err(|e| e.to_string())?;
    entries.insert(
        "inventory-private.json".to_string(),
        write_json(&stage.join("inventory-private.json"), &inventory_value)?,
    );
    let mut classes = BTreeMap::<String, usize>::new();
    for row in &classified.inventory {
        *classes.entry(row.classification.clone()).or_default() += 1;
    }
    let report = json!({
        "schema_version": 1,
        "translation_status": "qualified",
        "transfer_eligibility": transfer_eligibility,
        "selection_claim": "current_selection_unestablished",
        "q3": {"queue_id_redacted": redacted(&classified.q3.reference.queue_id), "status": "terminally_retired_no_replay"},
        "q4": {"queue_id_redacted": redacted(&classified.q4.reference.queue_id), "status": "pending_accepted_undelivered_or_unresolved"},
        "row_count": classified.inventory.len(),
        "unclassified_count": unclassified,
        "classification_counts": classes,
        "registry_revision": capture.registry["revision"],
        "registry_binding_count": capture.registry["bindings"].as_object().map_or(0, serde_json::Map::len),
        "registry_delivery_count": capture.registry["deliveries"].as_object().map_or(0, serde_json::Map::len),
        "payloads_redacted": true,
        "authority_conferred": false,
    });
    entries.insert(
        "report-redacted.json".to_string(),
        write_json(&stage.join("report-redacted.json"), &report)?,
    );
    let manifest_entries: Vec<_> = entries
        .iter()
        .map(|(path, (length, digest))| json!({"path":path,"length":length,"sha256":digest}))
        .collect();
    let manifest = json!({"schema_version":1,"files":manifest_entries});
    let (_, manifest_sha256) = write_json(&stage.join("manifest.json"), &manifest)?;
    let capsule_sha256 = entries
        .get("capsule.json")
        .ok_or("capsule entry lost")?
        .1
        .clone();
    let receipt = json!({
        "schema_version": 1,
        "status": "qualified_translation_only",
        "started_at": started,
        "ended_at": timestamp(),
        "attempt_root": attempt_root,
        "output_root": output_root,
        "source_attempt_sha256": capture.outer_hash,
        "source_child_receipt_sha256": capture.child_hash,
        "source_manifest_sha256": capture.manifest_hash,
        "manifest_sha256": manifest_sha256,
        "capsule_sha256": capsule_sha256,
        "code_identity": {"crate":"lifecycle-admin","version":env!("CARGO_PKG_VERSION"),"executable_sha256":current_executable_sha256()},
        "toolchain": option_env!("RUSTC_VERSION"),
        "argv": std::env::args().collect::<Vec<_>>(),
        "transfer_eligibility": transfer_eligibility,
        "primary_error": null,
        "cleanup_error": null,
        "authority_conferred": false,
    });
    let (_, receipt_sha256) = write_json(&stage.join("receipt.json"), &receipt)?;
    synthetic_fault("raw_sync")?;
    sync_directory(&raw_dir)?;
    synthetic_fault("stage_sync")?;
    sync_directory(stage)?;
    postflight_image(&capture)?;
    Ok(TranslationReceipt {
        output_root: output_root.to_path_buf(),
        capsule_sha256,
        manifest_sha256,
        receipt_sha256,
        translation_status: "qualified".to_string(),
        transfer_eligibility: transfer_eligibility.to_string(),
        unclassified_rows: unclassified,
        publication_status: "not_established_in_directory".to_string(),
        source_attempt_sha256: capture.outer_hash,
    })
}

/// Translate an already-qualified private capture bundle. No source-root path
/// from its provenance is ever opened or statted by this function.
pub fn translate_v28_capture(
    attempt_root: &Path,
    absent_output_root: &Path,
) -> Result<TranslationReceipt, TranslationError> {
    let started = timestamp();
    let stage = stage_root(absent_output_root).map_err(|primary_error| TranslationError {
        primary_error,
        phase: "stage_create".to_string(),
        actual_artifact: "not created by this invocation".to_string(),
        receipt_error: None,
        cleanup_error: None,
    })?;
    let mut phase = "stage_parent_sync";
    let mut renamed = false;
    let operation = (|| -> T<TranslationReceipt> {
        synthetic_fault("stage_parent_sync")?;
        sync_directory(absent_output_root.parent().ok_or("output parent lost")?)?;
        phase = "stage_content";
        let mut pending = perform_translation(attempt_root, absent_output_root, &stage, &started)?;
        phase = "no_replace_rename";
        synthetic_barrier("pre_rename")?;
        synthetic_fault("rename")?;
        rename_no_replace(&stage, absent_output_root)?;
        renamed = true;
        phase = "parent_sync";
        synthetic_barrier("post_rename")?;
        synthetic_fault("parent_sync")?;
        sync_directory(absent_output_root.parent().ok_or("output parent lost")?)?;
        phase = "postflight_readback";
        synthetic_fault("readback")?;
        verify_content(attempt_root, absent_output_root, &pending)?;
        pending.publication_status = "durable_after_parent_sync_and_readback".to_string();
        Ok(pending)
    })();
    match operation {
        Ok(receipt) => Ok(receipt),
        Err(primary_error) => {
            let actual = if renamed { absent_output_root } else { &stage };
            let diagnostic = if renamed {
                let basename = absent_output_root
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output");
                absent_output_root
                    .parent()
                    .unwrap_or(absent_output_root)
                    .join(format!(
                        ".{basename}.s9-a-{}-{}.failure-receipt.json",
                        std::process::id(),
                        timestamp()
                    ))
            } else {
                stage.join("failure-receipt.json")
            };
            let failure = json!({
                "schema_version": 1,
                "status": if renamed {"publication_uncertain"} else {"failed_before_publication"},
                "started_at": started,
                "ended_at": timestamp(),
                "attempt_root": attempt_root,
                "requested_output_root": absent_output_root,
                "phase": phase,
                "actual_artifact": actual,
                "retained_staging_root": if renamed {None::<&Path>} else {Some(stage.as_path())},
                "primary_error": primary_error,
                "cleanup_error": null,
                "authority_conferred": false,
            });
            let receipt_error = synthetic_fault("failure_receipt")
                .and_then(|_| write_json(&diagnostic, &failure).map(|_| ()))
                .and_then(|_| synthetic_fault("failure_receipt_sync"))
                .and_then(|_| {
                    sync_directory(diagnostic.parent().ok_or("failure receipt parent lost")?)
                })
                .err();
            Err(TranslationError {
                primary_error,
                phase: phase.to_string(),
                actual_artifact: actual.display().to_string(),
                receipt_error,
                cleanup_error: None,
            })
        }
    }
}
