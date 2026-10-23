//! `check --kind estimate-set` (contract 2.2): structure, then the packet and projection
//! pointers, then each basis entry's resolution in the projection.
//!
//! Stored pointers `{path, sha256}` are read from the stored path and hashed; there is no way
//! to supply a path that could disagree with them (contract S6). The packet and the projection
//! are read as `serde_json::Value` and only the fields named in contract 2.2 are used; this
//! crate does not depend on `claim-evidence`.
//!
//! What a pass establishes is limited. The checker establishes that each basis claim revision
//! is *selected in the cited projection*. It does not establish the claim's freshness, nor that
//! the claim supports the estimate: those evidence semantics belong to claim-evidence. Whether
//! current claim-evidence tooling emits projections over `proposal-research-v1` claims is
//! `not established`; the fixtures in this crate are synthetic.

use crate::contract::{ParsedContract, parse_contract, validate_contract};
use crate::diag::{CheckReport, ErrorItem, Failure, codes};
use crate::digest::{is_sha256_hex, sha256_hex};
use crate::model::{BasisRef, Estimate, EstimateSet, FileRef};
use crate::scan::scan;
use crate::validate::{
    Parsed, is_normalised_absolute, is_proposal_id, parse_estimate_set, validate_estimate_set,
};
use serde_json::Value;

/// The `kind` printed in the report.
pub const KIND: &str = "estimate-set";
/// The projection class (`context_kind`) this checker accepts (claim-evidence `PROJECTION_CONTEXT_KIND`).
pub const PROJECTION_CONTEXT_KIND: &str = "rust_reviewer_exact_claim_projection_v1";
/// The claim profile a basis claim must have.
pub const BASIS_PROFILE: &str = "proposal-research-v1";

/// Report note: what a pass establishes (contract 2.2).
pub const NOTE_SELECTION_ONLY: &str = "basis resolution establishes only that each claim revision is selected in the cited projection; it does not establish the claim's freshness or that it supports the estimate";
/// Report note: the producer of conforming projections is not established.
pub const NOTE_PRODUCER: &str = "whether current claim-evidence tooling emits projections over proposal-research-v1 claims is not established; the fixtures are synthetic";

fn notes() -> Vec<String> {
    vec![NOTE_SELECTION_ONLY.to_string(), NOTE_PRODUCER.to_string()]
}

fn err(code: &str, at: impl Into<String>, message: impl Into<String>) -> ErrorItem {
    ErrorItem::new(code, at, message)
}

/// Check the bytes of an estimate-set file. Not JSON is a [`Failure`] (exit 2); every violated
/// rule of a parseable document is listed in the report (exit 1 when any).
pub fn check_estimate_set_bytes(bytes: &[u8]) -> Result<CheckReport, Failure> {
    let set = match parse_estimate_set(bytes)? {
        Parsed::Invalid(errors) => return Ok(CheckReport::new(KIND, errors, notes())),
        Parsed::Set(set) => set,
    };
    let mut errors = validate_estimate_set(&set);
    check_pointers(&set, &mut errors);
    Ok(CheckReport::new(KIND, errors, notes()))
}

/// The `kind` printed in a contract report.
pub const KIND_CONTRACT: &str = "contract";

/// Check the bytes of a comparison-contract file (contract 3.4). The contract holds no file
/// pointer, so nothing is resolved; every violated rule of a parseable document is listed.
pub fn check_contract_bytes(bytes: &[u8]) -> Result<CheckReport, Failure> {
    let errors = match parse_contract(bytes)? {
        ParsedContract::Invalid(errors) => errors,
        ParsedContract::Contract(c) => validate_contract(&c),
    };
    Ok(CheckReport::new(KIND_CONTRACT, errors, Vec::new()))
}

/// A pointer can be resolved only when its stored shape is valid, so a malformed path is never
/// handed to the filesystem (which would make the result depend on the working directory).
fn shape_ok(f: &FileRef) -> bool {
    is_normalised_absolute(&f.path) && is_sha256_hex(&f.sha256)
}

/// The most bytes read from a stored path. A larger file is `pointer_unresolved`: upstream
/// packets and projections are small records, and an unbounded read would let a stored path
/// exhaust memory.
pub const MAX_UPSTREAM_BYTES: u64 = 32 * 1024 * 1024;

/// Read `path` only if it is a regular file (symlinks are followed, as `read` does), at most
/// [`MAX_UPSTREAM_BYTES`]. A FIFO, device or directory is refused before any read: opening a
/// FIFO with no writer would block, and a device such as `/dev/zero` has no end.
pub(crate) fn read_regular_file(path: &str) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let refuse = |what: &str| format!("{what} (a stored path must name a regular file)");
    let regular = |meta: &std::fs::Metadata| -> Result<(), String> {
        if meta.is_file() {
            Ok(())
        } else {
            Err(refuse("not a regular file"))
        }
    };
    regular(&std::fs::metadata(path).map_err(|e| e.to_string())?)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    // If the path is swapped for a FIFO between the check above and the open, do not block.
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0o4000); // O_NONBLOCK
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    regular(&meta)?;
    let too_big = || refuse(&format!("larger than {MAX_UPSTREAM_BYTES} bytes"));
    if meta.len() > MAX_UPSTREAM_BYTES {
        return Err(too_big());
    }
    let mut bytes = Vec::new();
    file.take(MAX_UPSTREAM_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_UPSTREAM_BYTES {
        return Err(too_big());
    }
    Ok(bytes)
}

/// Read the file at the stored path and hash it. `None` after reporting when the file cannot
/// be read, the digest differs, or the bytes are not a JSON object (a class mismatch: nothing
/// in them can be trusted as the expected record).
fn resolve(f: &FileRef, at: &str, errors: &mut Vec<ErrorItem>) -> Option<Value> {
    let bytes = match read_regular_file(&f.path) {
        Ok(bytes) => bytes,
        Err(e) => {
            errors.push(err(
                codes::POINTER_UNRESOLVED,
                at,
                format!("cannot read {}: {e}", f.path),
            ));
            return None;
        }
    };
    let actual = sha256_hex(&bytes);
    if actual != f.sha256 {
        errors.push(err(
            codes::DIGEST_MISMATCH,
            at,
            format!(
                "sha256 of {} is {actual}, the set stores {}",
                f.path, f.sha256
            ),
        ));
        return None;
    }
    // A document that repeats an object key has no single meaning: refuse it before any field
    // is read. (A document that does not parse falls through to the class mismatch below.)
    if let Ok(findings) = scan(&bytes)
        && !findings.is_empty()
    {
        for finding in findings {
            let place = if finding.at.is_empty() {
                "the document".to_string()
            } else {
                format!("`{}`", finding.at)
            };
            errors.push(err(
                codes::SCHEMA,
                at,
                format!("{} has {}: {}", f.path, place, finding.message),
            ));
        }
        return None;
    }
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(v @ Value::Object(_)) => Some(v),
        _ => {
            errors.push(err(
                codes::UPSTREAM_CLASS_MISMATCH,
                at,
                format!("{} is not a JSON object", f.path),
            ));
            None
        }
    }
}

fn schema_version_is_one(v: &Value) -> bool {
    v.get("schema_version").and_then(Value::as_i64) == Some(1)
}

fn check_pointers(set: &EstimateSet, errors: &mut Vec<ErrorItem>) {
    if shape_ok(&set.proposal.packet)
        && let Some(packet) = resolve(&set.proposal.packet, "proposal.packet", errors)
    {
        check_packet(set, &packet, errors);
    }
    let Some(reference) = &set.claim_projection else {
        return;
    };
    if !shape_ok(reference) {
        return;
    }
    let Some(projection) = resolve(reference, "claim_projection", errors) else {
        return;
    };
    let Some(selected) = projection_selection(&projection, errors) else {
        return;
    };
    for (i, estimate) in set.estimates.iter().enumerate() {
        for (at, basis) in basis_sites(estimate, i) {
            resolve_basis(selected, basis, &at, errors);
        }
    }
}

/// Packet class (contract S7, packet schema PK:7-10) and `proposal_id` equality.
fn check_packet(set: &EstimateSet, packet: &Value, errors: &mut Vec<ErrorItem>) {
    let packet_id = packet
        .get("proposal_id")
        .and_then(Value::as_str)
        .filter(|id| is_proposal_id(id));
    let Some(packet_id) = packet_id.filter(|_| schema_version_is_one(packet)) else {
        errors.push(err(
            codes::UPSTREAM_CLASS_MISMATCH,
            "proposal.packet",
            "not a proposal packet: needs schema_version 1 and a proposal_id string of the packet pattern",
        ));
        return;
    };
    if packet_id != set.proposal.proposal_id {
        errors.push(err(
            codes::PROPOSAL_ID_MISMATCH,
            "proposal.proposal_id",
            format!(
                "the set names {:?}, the packet names {packet_id:?}",
                set.proposal.proposal_id
            ),
        ));
    }
}

/// Projection class (contract S7): `schema_version` 1, the `context_kind`, and a
/// `selection_metadata` array. Returns that array when the class matches.
fn projection_selection<'a>(
    projection: &'a Value,
    errors: &mut Vec<ErrorItem>,
) -> Option<&'a [Value]> {
    let kind = projection.get("context_kind").and_then(Value::as_str);
    let selection = projection
        .get("selection_metadata")
        .and_then(Value::as_array);
    match (schema_version_is_one(projection), kind, selection) {
        (true, Some(PROJECTION_CONTEXT_KIND), Some(selection)) => Some(selection.as_slice()),
        _ => {
            errors.push(err(
                codes::UPSTREAM_CLASS_MISMATCH,
                "claim_projection",
                format!(
                    "not a claim projection: needs schema_version 1, context_kind {PROJECTION_CONTEXT_KIND:?} and a selection_metadata array"
                ),
            ));
            None
        }
    }
}

/// Every basis entry of an estimate with its dotted position, competing readings included.
fn basis_sites(estimate: &Estimate, index: usize) -> Vec<(String, &BasisRef)> {
    let mut sites: Vec<(String, &BasisRef)> = estimate
        .basis()
        .iter()
        .enumerate()
        .map(|(j, b)| (format!("estimates.{index}.basis.{j}"), b))
        .collect();
    if let Estimate::Contested(c) = estimate {
        for (k, competing) in c.competing.iter().enumerate() {
            for (j, b) in competing.basis.iter().enumerate() {
                sites.push((format!("estimates.{index}.competing.{k}.basis.{j}"), b));
            }
        }
    }
    sites
}

/// An element of `selection_metadata[]` with the basis entry's revision id, revision digest
/// and claim id (contract 2.2 step 4), then its claim profile.
fn resolve_basis(selected: &[Value], basis: &BasisRef, at: &str, errors: &mut Vec<ErrorItem>) {
    let text = |v: &Value, path: &[&str]| -> Option<String> {
        let mut cur = v;
        for key in path {
            cur = cur.get(key)?;
        }
        cur.as_str().map(str::to_string)
    };
    let found = selected.iter().find(|element| {
        text(element, &["revision_id"]).as_deref() == Some(basis.claim_revision_id.as_str())
            && text(element, &["revision_sha256"]).as_deref()
                == Some(basis.claim_revision_sha256.as_str())
            && text(element, &["claim", "id"]).as_deref() == Some(basis.claim_id.as_str())
    });
    match found {
        None => errors.push(err(
            codes::BASIS_UNRESOLVED,
            at,
            format!(
                "no selection_metadata element has revision_id {:?}, revision_sha256 {:?} and claim.id {:?}",
                basis.claim_revision_id, basis.claim_revision_sha256, basis.claim_id
            ),
        )),
        Some(element) => {
            let profile = text(element, &["claim", "profile"]);
            if profile.as_deref() != Some(BASIS_PROFILE) {
                errors.push(err(
                    codes::BASIS_WRONG_PROFILE,
                    at,
                    format!(
                        "the basis claim has profile {:?}, expected {BASIS_PROFILE:?}",
                        profile.unwrap_or_default()
                    ),
                ));
            }
        }
    }
}
