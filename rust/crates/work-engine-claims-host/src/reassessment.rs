//! Read-only reassessment worklist for exact repository claim revisions.
//!
//! Composition only: `claim_evidence::RepositoryReader` resolves exact revisions,
//! `repository_evidence_cbm` verifies immutable captures and compares their
//! anchored bytes with the selected checkout. Nothing here writes canonical state,
//! publishes, retires, invalidates, acquires the producer lease or runs CBM. A
//! nomination is a review request derived from byte equality and indexed
//! documentation rows; it is not semantic truth, applicability, or proof that an
//! implementation or reference is absent. Completeness covers the explicitly
//! selected items only.
use claim_evidence::{
    RepositoryReader,
    codec::{JsValue, canonical_json},
    sha256_bytes,
};
use repository_evidence_cbm::{
    Artifact, Binding, Error as CbmError, parse_json as parse_cbm_json, read_artifact,
    source_currency,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub const KIND: &str = "repository_reassessment_worklist_v1";
pub const MAX_SELECTIONS: usize = 16;
const INPUT_MAX: u64 = 1024 * 1024;
const MAX_ROWS: usize = 25;
const MAX_PATHS: usize = 25;
const MAX_ANCHORS: usize = 100;
const MAX_LIMITATIONS: usize = 8;
const CBM_OWNER: &str = "repository-evidence-cbm";
const UNRESOLVED_STATUSES: &[&str] = &["missing", "unresolved", "unavailable", "ambiguous"];

/// One exact revision address, optionally paired with a new capture to compare.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub claim_id: String,
    pub revision_id: String,
    pub revision_sha256: String,
    #[serde(default)]
    pub new_capture_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u8,
    pub selections: Vec<Selection>,
}

fn is_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl Request {
    /// Closed, bounded and duplicate-key-free. A malformed request rejects the
    /// whole batch; no selection is silently dropped or normalized.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() as u64 > INPUT_MAX {
            return Err("request exceeds capacity".into());
        }
        // Unique-key parser: a duplicated field is rejected, never last-one-wins.
        let request: Request =
            serde_json::from_value(parse_cbm_json(bytes).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if request.schema_version != 1 {
            return Err("unsupported reassessment request schema".into());
        }
        if request.selections.is_empty() || request.selections.len() > MAX_SELECTIONS {
            return Err(format!("selections must number 1..={MAX_SELECTIONS}"));
        }
        let mut seen = BTreeSet::new();
        for s in &request.selections {
            if s.claim_id.is_empty()
                || s.revision_id.is_empty()
                || s.claim_id.len() > 256
                || s.revision_id.len() > 256
                || !is_sha(&s.revision_sha256)
                || s.new_capture_sha256.as_deref().is_some_and(|v| !is_sha(v))
            {
                return Err(
                    "selection fields must be exact identifiers and lowercase SHA-256".into(),
                );
            }
            if !seen.insert((s.claim_id.clone(), s.revision_id.clone())) {
                return Err("duplicate selection".into());
            }
        }
        Ok(request)
    }
}

/// Operator-selected observation inputs, verified before any item is read. The
/// binding names the checkout whose current bytes are compared; its project,
/// executable and digest fields are not used because nothing is executed.
pub struct Source {
    repository_id: String,
    decision_scope: String,
    checkout: PathBuf,
    binding_sha256: String,
    artifacts: PathBuf,
}

impl Source {
    pub fn prepare(
        repository_id: &str,
        decision_scope: &str,
        binding: &Path,
        artifacts: &Path,
    ) -> Result<Self, String> {
        let mut bytes = Vec::new();
        fs::File::open(binding)
            .map_err(|e| format!("binding: {e}"))?
            .take(INPUT_MAX + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("binding: {e}"))?;
        if bytes.len() as u64 > INPUT_MAX {
            return Err("binding exceeds capacity".into());
        }
        let parsed: Binding =
            serde_json::from_value(parse_cbm_json(&bytes).map_err(|e| e.to_string())?)
                .map_err(|e| format!("binding: {e}"))?;
        if parsed.repository_id != repository_id {
            return Err("binding repository_id does not match trusted startup".into());
        }
        let checkout = parsed
            .repository
            .canonicalize()
            .map_err(|e| format!("checkout: {e}"))?;
        if !parsed.repository.is_absolute() || !checkout.is_dir() {
            return Err("binding checkout must be an absolute directory".into());
        }
        if !artifacts.is_absolute()
            || artifacts
                .canonicalize()
                .map_err(|e| format!("artifact root: {e}"))?
                != artifacts
            || !artifacts.is_dir()
        {
            return Err("artifact root must be an existing canonical absolute directory".into());
        }
        Ok(Self {
            repository_id: repository_id.into(),
            decision_scope: decision_scope.into(),
            checkout,
            binding_sha256: sha256_bytes(&bytes),
            artifacts: artifacts.into(),
        })
    }
}

struct Finding {
    code: &'static str,
    capture: Option<String>,
    detail: Value,
    /// Field, index and role of the reference location that produced the finding.
    location: Value,
}
impl Finding {
    fn value(&self) -> Value {
        json!({"code": self.code, "capture_sha256": self.capture, "location": self.location, "detail": self.detail})
    }
}
#[derive(Default)]
struct Findings {
    reasons: Vec<Finding>,
    uncertainties: Vec<Finding>,
    blocked: Vec<Finding>,
    /// Observations about the immutable subject baseline. They never nominate and
    /// never change the disposition; they stay visible with their location.
    historical: Vec<Finding>,
    notes: Vec<String>,
}
impl Findings {
    fn mark(&self) -> [usize; 4] {
        [
            self.reasons.len(),
            self.uncertainties.len(),
            self.blocked.len(),
            self.historical.len(),
        ]
    }
    /// Attribute every finding added since `mark` to one reference location.
    fn stamp(&mut self, mark: [usize; 4], location: &Value) {
        let lists = [
            &mut self.reasons,
            &mut self.uncertainties,
            &mut self.blocked,
            &mut self.historical,
        ];
        for (list, from) in lists.into_iter().zip(mark) {
            for item in list.iter_mut().skip(from) {
                item.location = location.clone();
            }
        }
    }
}
fn finding(code: &'static str, capture: Option<&str>, detail: Value) -> Finding {
    Finding {
        code,
        capture: capture.map(Into::into),
        detail,
        location: Value::Null,
    }
}
fn values(findings: &[Finding]) -> Value {
    Value::Array(findings.iter().map(Finding::value).collect())
}
fn capped(paths: Vec<String>) -> Value {
    json!({"count": paths.len(), "paths": paths.into_iter().take(MAX_PATHS).collect::<Vec<_>>()})
}
fn to_value(value: &JsValue) -> Result<Value, String> {
    serde_json::from_str(&canonical_json(value).map_err(|e| e.message)?).map_err(|e| e.to_string())
}

/// Why a capture is being assessed. `Selected` (ordinary evidence) and
/// `Sensitivity` are current support of the selected revision; `Historical` is the
/// claim subject's immutable evidence baseline, which successors inherit unchanged;
/// `New` is the optional requested replacement capture.
#[derive(Clone, Copy, PartialEq)]
enum Role {
    Selected,
    Sensitivity,
    Historical,
    New,
}
impl Role {
    fn name(self) -> &'static str {
        match self {
            Role::Selected => "selected_evidence",
            Role::Sensitivity => "current_sensitivity",
            Role::Historical => "historical_baseline",
            Role::New => "new_capture",
        }
    }
}

fn read_failure(error: &CbmError) -> &'static str {
    match error {
        CbmError::Io(e) if e.kind() == std::io::ErrorKind::NotFound => "artifact_missing",
        CbmError::Invalid("capture scope mismatch") => "artifact_foreign_repository",
        CbmError::Invalid("capture digest mismatch") => "artifact_digest_mismatch",
        _ => "artifact_unreadable_or_invalid",
    }
}

/// The observation response is the exact CBM stdout the adapter already verified
/// when it admitted the artifact; this only reads rows back out of it.
fn page_payload(response: &str) -> Option<Value> {
    let raw = parse_cbm_json(response.as_bytes()).ok()?;
    let envelope = raw.get("result").unwrap_or(&raw);
    let content = envelope.get("content")?.as_array()?;
    if content.len() != 1 {
        return None;
    }
    let page = parse_cbm_json(content[0].get("text")?.as_str()?.as_bytes()).ok()?;
    page.get("references")?.as_array()?;
    Some(page)
}

fn assess_capture(
    role: Role,
    sha: &str,
    artifact: &Artifact,
    checkout: &Path,
    f: &mut Findings,
) -> Value {
    let capture = artifact.capture();
    let anchor_paths: BTreeSet<&str> = capture.anchors().iter().map(|a| a.path.as_str()).collect();
    let mut view = Map::new();
    view.insert("role".into(), json!(role.name()));
    if role == Role::Historical {
        view.insert("role_semantics".into(), json!("immutable subject baseline inherited unchanged by successors; byte changes here are historical observations, not current-support failures"));
    }
    view.insert("capture_sha256".into(), json!(sha));
    view.insert("store_generation".into(), json!(capture.store_generation()));
    view.insert("anchor_count".into(), json!(anchor_paths.len()));
    view.insert(
        "limitation_count".into(),
        json!(capture.limitations().len()),
    );
    view.insert(
        "limitations".into(),
        json!(
            capture
                .limitations()
                .iter()
                .take(MAX_LIMITATIONS)
                .collect::<Vec<_>>()
        ),
    );

    // Source currency: selected-anchor byte equality only.
    match source_currency(artifact, checkout) {
        Err(e) => f.blocked.push(finding(
            "source_currency_unavailable",
            Some(sha),
            json!({"error": e.to_string()}),
        )),
        Ok(currency) => {
            let anchors = currency["anchors"].as_array().cloned().unwrap_or_default();
            if anchors.len() != anchor_paths.len() || currency["applicability"] != "not_assessed" {
                f.blocked.push(finding(
                    "source_currency_shape_unexpected",
                    Some(sha),
                    json!({"anchors": anchors.len()}),
                ));
            }
            let (mut changed, mut unavailable) = (Vec::new(), Vec::new());
            for anchor in &anchors {
                let path = anchor["path"].as_str().unwrap_or_default().to_owned();
                match anchor["state"].as_str() {
                    Some("bytes_match") => {}
                    Some("bytes_changed") => {
                        if role != Role::New {
                            f.reasons.push(finding(
                                "anchor_bytes_changed",
                                Some(sha),
                                json!({"path": path, "captured_sha256": anchor["captured_sha256"]}),
                            ));
                        }
                        changed.push(path);
                    }
                    Some("unavailable") => {
                        if role != Role::New {
                            f.reasons.push(finding(
                                "anchor_bytes_unavailable",
                                Some(sha),
                                json!({"path": path, "captured_sha256": anchor["captured_sha256"]}),
                            ));
                        }
                        unavailable.push(path);
                    }
                    other => f.uncertainties.push(finding(
                        "anchor_state_unknown",
                        Some(sha),
                        json!({"path": path, "state": other}),
                    )),
                }
            }
            let all_match = !anchors.is_empty()
                && changed.is_empty()
                && unavailable.is_empty()
                && anchors.iter().all(|a| a["state"] == "bytes_match");
            if role == Role::New && !all_match {
                f.uncertainties.push(finding(
                    "new_capture_not_current",
                    Some(sha),
                    json!({"bytes_changed": capped(changed), "unavailable": capped(unavailable)}),
                ));
            }
            // Bounded: anchors needing attention come first; counts cover all of them.
            let matching = anchors
                .iter()
                .filter(|a| a["state"] == "bytes_match")
                .count();
            let (mut listed, matched): (Vec<Value>, Vec<Value>) = anchors
                .iter()
                .cloned()
                .partition(|a| a["state"] != "bytes_match");
            listed.extend(matched);
            let omitted = listed.len().saturating_sub(MAX_ANCHORS);
            listed.truncate(MAX_ANCHORS);
            view.insert(
                "source_currency".into(),
                json!({
                    "anchors": listed,
                    "anchors_total": anchors.len(),
                    "anchors_bytes_match": matching,
                    "omitted_anchors": omitted,
                    "all_anchors_bytes_match": all_match,
                    "applicability": "not_assessed",
                    "semantics": "selected anchor byte equality against the selected checkout only; not proposition truth",
                }),
            );
        }
    }

    // Documentation observations: coverage plus rows associated to anchored documents.
    let mut doc = Map::new();
    match capture.documentation_status() {
        Err(e) => f.blocked.push(finding(
            "documentation_status_unavailable",
            Some(sha),
            json!({"error": e.to_string()}),
        )),
        Ok(status) => {
            let pagination = status["pagination"]
                .as_str()
                .unwrap_or("unknown")
                .to_owned();
            doc.extend(status.as_object().cloned().unwrap_or_default());
            match (pagination.as_str(), role) {
                ("complete", _) => {}
                (
                    "partial" | "not_requested",
                    Role::Selected | Role::Sensitivity | Role::Historical,
                ) => f.uncertainties.push(finding(
                    if pagination == "partial" {
                        "documentation_coverage_partial"
                    } else {
                        "documentation_not_requested"
                    },
                    Some(sha),
                    json!({"pagination": pagination}),
                )),
                ("partial" | "not_requested", Role::New) => f.reasons.push(finding(
                    "new_capture_documentation_incomplete",
                    Some(sha),
                    json!({"pagination": pagination}),
                )),
                _ => f.uncertainties.push(finding(
                    "documentation_pagination_unknown",
                    Some(sha),
                    json!({"pagination": pagination}),
                )),
            }
        }
    }
    let (mut unresolved, mut resolved, mut other) = (Vec::new(), Vec::new(), 0u64);
    let (mut associated_unresolved, mut associated_resolved, mut unknown_status) =
        (0u64, 0u64, 0u64);
    let (mut unassociated, mut unparsed, mut other_unresolved) = (0u64, 0u64, 0u64);
    let mut statuses = BTreeSet::new();
    for (index, observation) in capture.observations().iter().enumerate() {
        if observation.request.tool != "detect_changes" {
            continue;
        }
        let Some(page) = page_payload(&observation.response) else {
            unparsed += 1;
            continue;
        };
        let cursor = observation
            .request
            .arguments
            .get("reference_cursor")
            .cloned()
            .unwrap_or(Value::Null);
        for row in page["references"].as_array().into_iter().flatten() {
            let status = row.get("status").and_then(Value::as_str);
            let is_unresolved = status.is_some_and(|s| UNRESOLVED_STATUSES.contains(&s));
            let class = match status {
                Some("resolved") => "resolved",
                _ if is_unresolved => "unresolved",
                _ => "unknown_status",
            };
            let Some(document) = row.get("document").and_then(Value::as_str) else {
                unassociated += 1;
                continue;
            };
            if !anchor_paths.contains(document) {
                other += 1;
                other_unresolved += u64::from(is_unresolved);
                continue;
            }
            let reported = json!({"capture_sha256": sha, "observation_index": index, "page_cursor": cursor, "classification": class, "row": row});
            match class {
                "resolved" => {
                    associated_resolved += 1;
                    resolved.push(reported);
                }
                "unresolved" => {
                    associated_unresolved += 1;
                    unresolved.push(reported);
                }
                _ => {
                    unknown_status += 1;
                    statuses.insert(status.unwrap_or("<absent or non-text>").to_owned());
                    unresolved.push(reported);
                }
            }
        }
    }
    if associated_unresolved > 0 {
        f.reasons.push(finding(
            if role != Role::New {
                "documentation_references_unresolved"
            } else {
                "new_capture_unresolved_documentation_references"
            },
            Some(sha),
            json!({"associated_unresolved_rows": associated_unresolved}),
        ));
    }
    if unknown_status > 0 {
        f.uncertainties.push(finding("documentation_row_status_unknown", Some(sha), json!({"rows": unknown_status, "statuses": statuses.iter().take(MAX_PATHS).collect::<Vec<_>>()})));
    }
    if unassociated > 0 {
        f.uncertainties.push(finding(
            "documentation_row_association_unresolved",
            Some(sha),
            json!({"rows_without_document_path": unassociated}),
        ));
    }
    if unparsed > 0 {
        f.uncertainties.push(finding(
            "documentation_observation_unparsed",
            Some(sha),
            json!({"observations": unparsed}),
        ));
    }
    let total_associated = unresolved.len() + resolved.len();
    let reported: Vec<Value> = unresolved
        .into_iter()
        .chain(resolved)
        .take(MAX_ROWS)
        .collect();
    doc.insert(
        "associated_resolved_rows".into(),
        json!(associated_resolved),
    );
    doc.insert(
        "associated_unresolved_rows".into(),
        json!(associated_unresolved),
    );
    doc.insert(
        "associated_unknown_status_rows".into(),
        json!(unknown_status),
    );
    doc.insert("other_document_rows".into(), json!(other));
    doc.insert(
        "other_document_unresolved_rows".into(),
        json!(other_unresolved),
    );
    doc.insert("rows_without_document_path".into(), json!(unassociated));
    doc.insert("unparsed_observations".into(), json!(unparsed));
    doc.insert(
        "omitted_associated_rows".into(),
        json!(total_associated - reported.len()),
    );
    doc.insert("associated_rows".into(), Value::Array(reported));
    doc.insert("association".into(), json!("rows are associated only to this capture's anchored paths; other documents are counted, not attributed"));
    view.insert("documentation".into(), Value::Object(doc));
    Value::Object(view)
}

fn anchor_map(artifact: &Artifact) -> BTreeMap<&str, &str> {
    artifact
        .capture()
        .anchors()
        .iter()
        .map(|a| (a.path.as_str(), a.sha256.as_str()))
        .collect()
}

fn compare(old: &Artifact, new: &Artifact, f: &mut Findings) {
    if old.sha256() == new.sha256() {
        f.notes.push(format!(
            "new capture {} is identical to selected evidence",
            new.sha256()
        ));
        return;
    }
    let (before, after) = (anchor_map(old), anchor_map(new));
    let changed = before
        .iter()
        .filter(|(p, s)| after.get(*p).is_some_and(|n| n != *s))
        .map(|(p, _)| (*p).to_owned())
        .collect();
    let added = after
        .keys()
        .filter(|p| !before.contains_key(*p))
        .map(|p| (*p).to_owned())
        .collect();
    let removed = before
        .keys()
        .filter(|p| !after.contains_key(*p))
        .map(|p| (*p).to_owned())
        .collect();
    f.reasons.push(finding(
        "new_capture_differs",
        Some(new.sha256()),
        json!({"selected_evidence_sha256": old.sha256(), "anchors_changed": capped(changed), "anchors_added": capped(added), "anchors_removed": capped(removed),
               "semantics": "different immutable capture bytes; not a proposition change"}),
    ));
}

const BASELINE_FIELD: &str = "subject.evidence_baseline";
const EVIDENCE_FIELD: &str = "revision.evidence_references";
const SENSITIVITY_FIELD: &str = "revision.sensitivity_references";

/// Field, optional list index and role of one material reference location.
struct Location {
    field: &'static str,
    index: Option<usize>,
    role: Role,
}
impl Location {
    fn baseline() -> Self {
        Location {
            field: BASELINE_FIELD,
            index: None,
            role: Role::Historical,
        }
    }
    fn value(&self) -> Value {
        json!({"field": self.field, "index": self.index, "role": self.role.name()})
    }
}

#[derive(Default)]
struct Refs {
    baseline: Value,
    sensitivity: Vec<Value>,
    accounting: Value,
}

/// Historical-baseline observations never nominate: they are kept apart from the
/// disposition inputs, while a baseline that cannot be assessed stays blocked.
fn assess_located(
    role: Role,
    sha: &str,
    artifact: &Artifact,
    checkout: &Path,
    f: &mut Findings,
) -> Value {
    if role != Role::Historical {
        return assess_capture(role, sha, artifact, checkout, f);
    }
    let mut scratch = Findings::default();
    let view = assess_capture(role, sha, artifact, checkout, &mut scratch);
    f.blocked.extend(scratch.blocked);
    f.historical
        .extend(scratch.reasons.into_iter().chain(scratch.uncertainties));
    view
}

fn classify_reference(reference: &Value) -> Result<String, (&'static str, Value)> {
    let text = |key: &str| reference.get(key).and_then(Value::as_str);
    if !reference.is_object() {
        return Err(("malformed_cbm_reference", json!({"reference": reference})));
    }
    if text("owner") != Some(CBM_OWNER) {
        return Err((
            "unsupported_evidence_owner",
            json!({"owner": reference.get("owner"), "reference": reference.get("reference")}),
        ));
    }
    let sha = text("integrity_sha256").filter(|s| is_sha(s));
    match sha {
        Some(sha)
            if text("revision") == Some(sha)
                && text("reference") == Some(format!("cbm-capture@{sha}").as_str()) =>
        {
            if text("status") == Some("verified") {
                Ok(sha.to_owned())
            } else {
                Err((
                    "evidence_reference_not_verified",
                    json!({"status": reference.get("status"), "reference": reference.get("reference")}),
                ))
            }
        }
        _ => Err(("malformed_cbm_reference", json!({"reference": reference}))),
    }
}

fn assess_item(reader: &RepositoryReader, source: &Source, selection: &Selection) -> Value {
    let mut f = Findings::default();
    let echo = json!({
        "claim_id": selection.claim_id, "revision_id": selection.revision_id,
        "revision_sha256": selection.revision_sha256, "new_capture_sha256": selection.new_capture_sha256,
    });
    let finish = |f: Findings,
                  claim: Value,
                  projection: Value,
                  evidence: Vec<Value>,
                  refs: Refs,
                  new_capture: Value| {
        let disposition = if !f.blocked.is_empty() {
            "blocked"
        } else if !f.reasons.is_empty() {
            "review_nominated"
        } else if !f.uncertainties.is_empty() {
            "uncertain"
        } else {
            "no_nomination_observed"
        };
        json!({
            "selection": echo, "status": if f.blocked.is_empty() { "assessed" } else { "blocked" },
            "disposition": disposition, "claim": claim, "projection": projection,
            "evidence": evidence, "sensitivity": refs.sensitivity,
            "historical_baseline": refs.baseline, "reference_accounting": refs.accounting,
            "new_capture": new_capture,
            "reasons": values(&f.reasons), "uncertainties": values(&f.uncertainties),
            "blocked": values(&f.blocked), "historical_observations": values(&f.historical),
            "notes": f.notes,
        })
    };
    let blocked_item = |mut f: Findings, code: &'static str, detail: Value| {
        f.blocked.push(finding(code, None, detail));
        finish(
            f,
            Value::Null,
            Value::Null,
            vec![],
            Refs::default(),
            Value::Null,
        )
    };

    let projection = match reader
        .resolve(
            &selection.claim_id,
            &selection.revision_id,
            &selection.revision_sha256,
        )
        .map_err(|e| e.message)
        .and_then(|js| to_value(&js))
    {
        Ok(v) => v,
        Err(error) => return blocked_item(f, "claim_resolution_failed", json!({"error": error})),
    };
    let items = projection["items"].as_array().cloned().unwrap_or_default();
    let item = match items.as_slice() {
        [item]
            if projection["kind"] == "repository_fact_projection_v1"
                && projection["repository_id"] == source.repository_id.as_str()
                && projection["decision_scope"] == source.decision_scope.as_str()
                && item["revision_sha256"] == selection.revision_sha256.as_str()
                && item["revision"]["id"] == selection.revision_id.as_str()
                && item["revision"]["claim_id"] == selection.claim_id.as_str()
                && item["revision"]["decision_scope"] == source.decision_scope.as_str()
                && item["claim"]["id"] == selection.claim_id.as_str()
                && item["current_head"].is_boolean()
                && item["head_revision_ids"].is_array() =>
        {
            item.clone()
        }
        _ => {
            return blocked_item(
                f,
                "projection_shape_unexpected",
                json!({"items": items.len(), "kind": projection["kind"]}),
            );
        }
    };
    let claim = json!({
        "claim_id": selection.claim_id, "revision_id": selection.revision_id,
        "revision_sha256": selection.revision_sha256, "decision_scope": item["revision"]["decision_scope"],
        "current_head": item["current_head"], "head_revision_ids": item["head_revision_ids"],
        "predecessor_revision": item["revision"].get("predecessor_revision").cloned().unwrap_or(Value::Null),
        "subject": item["claim"]["subject"], "applicability": "not_assessed",
        "unresolved_references": item["unresolved_references"],
    });
    let projection_identity = json!({
        "repository_id": projection["repository_id"], "decision_scope": projection["decision_scope"],
        "kind": projection["kind"], "schema_version": projection["schema_version"],
        "projection": projection["projection"],
    });
    if item["current_head"] == false {
        f.notes.push("selected revision is not a current head; newer heads are named in claim.head_revision_ids and are not followed".into());
    }

    let new_capture = match selection.new_capture_sha256.as_deref() {
        None => None,
        Some(sha) => match read_artifact(&source.artifacts, sha, &source.repository_id) {
            Ok(artifact) => Some(artifact),
            Err(e) => {
                f.blocked.push(finding(
                    read_failure(&e),
                    Some(sha),
                    json!({"role": "new_capture", "error": e.to_string()}),
                ));
                None
            }
        },
    };

    // Every material reference location of the selected revision, each with its own
    // role: the claim subject's immutable baseline (historical), the revision's
    // ordinary evidence and its sensitivity references (current support).
    let mut locations: Vec<(Location, Value)> = Vec::new();
    match item["claim"]["subject"].get("evidence_baseline") {
        Some(reference) => locations.push((Location::baseline(), reference.clone())),
        None => f.blocked.push(finding(
            "baseline_reference_unreadable",
            None,
            json!({"field": BASELINE_FIELD}),
        )),
    }
    let mut declared = (1u64, 0u64, 0u64);
    for (field, role, slot) in [
        (EVIDENCE_FIELD, Role::Selected, 1usize),
        (SENSITIVITY_FIELD, Role::Sensitivity, 2),
    ] {
        let key = field.rsplit('.').next().unwrap_or(field);
        match item["revision"][key].as_array() {
            None => f.blocked.push(finding(
                if role == Role::Selected {
                    "evidence_references_unreadable"
                } else {
                    "sensitivity_references_unreadable"
                },
                None,
                json!({"field": field}),
            )),
            Some(references) => {
                if slot == 1 {
                    declared.1 = references.len() as u64;
                    if references.is_empty() {
                        f.blocked.push(finding(
                            "no_evidence_references",
                            None,
                            json!({"semantics": "nothing to reassess; this is not a clean result"}),
                        ));
                    }
                } else {
                    declared.2 = references.len() as u64;
                }
                for (index, reference) in references.iter().enumerate() {
                    locations.push((
                        Location {
                            field,
                            index: Some(index),
                            role,
                        },
                        reference.clone(),
                    ));
                }
            }
        }
    }

    // One physical read per distinct capture; every location keeps its own
    // classification, assessment and findings.
    let mut reads: BTreeMap<String, Result<Artifact, (&'static str, String)>> = BTreeMap::new();
    let (mut evidence, mut sensitivity) = (Vec::new(), Vec::new());
    let (mut baseline_view, mut accounting) = (Value::Null, Vec::new());
    for (location, reference) in &locations {
        let mark = f.mark();
        let place = location.value();
        let mut entry = json!({"index": location.index, "location": place, "reference": reference});
        let mut capture_sha = Value::Null;
        match classify_reference(reference) {
            Err((code, detail)) => f.blocked.push(finding(
                code,
                None,
                json!({"index": location.index, "detail": detail}),
            )),
            Ok(sha) => {
                capture_sha = json!(sha);
                let read = reads.entry(sha.clone()).or_insert_with(|| {
                    read_artifact(&source.artifacts, &sha, &source.repository_id)
                        .map_err(|e| (read_failure(&e), e.to_string()))
                });
                match read {
                    Err((code, error)) => f.blocked.push(finding(
                        code,
                        Some(&sha),
                        json!({"index": location.index, "role": location.role.name(), "error": error}),
                    )),
                    Ok(old) => {
                        entry["capture"] = assess_located(location.role, &sha, old, &source.checkout, &mut f);
                        if location.role == Role::Selected
                            && let Some(new) = &new_capture
                        {
                            compare(old, new, &mut f);
                        }
                    }
                }
            }
        }
        f.stamp(mark, &place);
        let blocked_here: Vec<&str> = f.blocked[mark[2]..].iter().map(|b| b.code).collect();
        accounting.push(json!({
            "field": location.field, "index": location.index, "role": location.role.name(),
            "capture_sha256": capture_sha,
            "outcome": if blocked_here.is_empty() { "assessed" } else { "blocked" },
            "blocked_codes": blocked_here,
        }));
        match location.role {
            Role::Historical => baseline_view = entry,
            Role::Sensitivity => sensitivity.push(entry),
            _ => evidence.push(entry),
        }
    }
    let assessed = accounting
        .iter()
        .filter(|a| a["outcome"] == "assessed")
        .count();
    let refs = Refs {
        baseline: baseline_view,
        sensitivity,
        accounting: json!({
            "declared": {"baseline": declared.0, "evidence_references": declared.1, "sensitivity_references": declared.2},
            "locations": accounting.len(), "assessed": assessed, "blocked": accounting.len() - assessed,
            "entries": accounting,
            "semantics": "every declared reference location is classified; an unreadable field or location is a blocked entry, never an omission",
        }),
    };
    let new_view = match &new_capture {
        None => Value::Null,
        Some(new) => assess_capture(Role::New, new.sha256(), new, &source.checkout, &mut f),
    };
    finish(f, claim, projection_identity, evidence, refs, new_view)
}

/// Resolve every explicit selection; the returned object is the whole artifact.
pub fn reassess(reader: &RepositoryReader, source: &Source, request: &Request) -> Value {
    let items: Vec<Value> = request
        .selections
        .iter()
        .map(|s| assess_item(reader, source, s))
        .collect();
    let count = |disposition: &str| {
        items
            .iter()
            .filter(|i| i["disposition"] == disposition)
            .count()
    };
    let blocked = count("blocked");
    let locations = |key: &str| -> u64 {
        items
            .iter()
            .filter_map(|i| i["reference_accounting"][key].as_u64())
            .sum()
    };
    let historical_observations: usize = items
        .iter()
        .filter_map(|i| i["historical_observations"].as_array())
        .map(Vec::len)
        .sum();
    json!({
        "schema_version": 1,
        "kind": KIND,
        "observation_only": true,
        "repository_id": source.repository_id,
        "decision_scope": source.decision_scope,
        "applicability": "not_assessed",
        "source_checkout": {"path": source.checkout, "binding_sha256": source.binding_sha256,
            "semantics": "current working-tree bytes of the binding checkout; not a commit identity"},
        "completeness": {
            "scope": "explicit_selections_only", "selected": items.len(), "assessed": items.len() - blocked,
            "blocked": blocked, "review_nominated": count("review_nominated"), "complete": blocked == 0,
            "reference_locations": {"classified": locations("locations"), "assessed": locations("assessed"), "blocked": locations("blocked")},
            "historical_observations": historical_observations,
        },
        "items": items,
        "limitations": [
            "Observation only: no canonical mutation, grant, publication, retirement, invalidation or workflow effect.",
            "A nomination reflects changed/unavailable anchored bytes, new-capture differences or indexed documentation rows; it is not semantic truth.",
            "bytes_match and resolved documentation links do not establish that a claim is still true or applicable; absent or unresolved references are navigation evidence, not proof of absent implementation.",
            "Every declared reference location (subject baseline, ordinary evidence, sensitivity references) is classified with field, index and role; an unassessable location blocks completeness. The subject baseline is immutable history inherited by successors: changed or unavailable baseline anchors are reported as historical observations and do not nominate, while an invalid or unresolved baseline blocks.",
            "Completeness covers only the selected items; it says nothing about other claims in the repository.",
            "Documentation rows cover indexed references only; partial or not-requested coverage is reported, never treated as clear."
        ],
    })
}
