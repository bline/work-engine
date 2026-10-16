use super::{Result, hex, sha, string};
use serde_json::{Value, json};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;

fn git(repository: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() || out.stdout.len() > 2 * 1024 * 1024 || out.stderr.len() > 8192 {
        return Err("bounded Git source read failed".into());
    }
    Ok(out.stdout)
}
fn git_text(repository: &Path, args: &[&str]) -> Result<String> {
    String::from_utf8(git(repository, args)?)
        .map(|s| s.trim().to_string())
        .map_err(|e| e.to_string())
}
fn object(repository: &Path, oid: &str, kind: &str, format: &str) -> Result<Vec<u8>> {
    if !matches!(oid.len(), 40 | 64) || !oid.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Git OID invalid".into());
    }
    if git_text(repository, &["cat-file", "-t", oid])? != kind {
        return Err("Git object kind differs".into());
    }
    let size = git_text(repository, &["cat-file", "-s", oid])?
        .parse::<usize>()
        .map_err(|e| e.to_string())?;
    if size > 1024 * 1024 {
        return Err("Git object too large".into());
    }
    let bytes = git(repository, &["cat-file", kind, oid])?;
    if bytes.len() != size {
        return Err("Git object size differs".into());
    }
    let mut framed = format!("{kind} {size}\0").into_bytes();
    framed.extend_from_slice(&bytes);
    let actual = match format {
        "sha1" => hex(&Sha1::digest(&framed)),
        "sha256" => hex(&Sha256::digest(&framed)),
        _ => return Err("Git object format unsupported".into()),
    };
    if actual != oid {
        return Err("raw Git object identity differs".into());
    }
    Ok(bytes)
}
fn entry(tree: &[u8], name: &str) -> Result<String> {
    let mut rest = tree;
    while !rest.is_empty() {
        let zero = rest
            .iter()
            .position(|b| *b == 0)
            .ok_or("tree entry invalid")?;
        let header = std::str::from_utf8(&rest[..zero]).map_err(|e| e.to_string())?;
        let (_, entry_name) = header.split_once(' ').ok_or("tree mode missing")?;
        let oid_len = 20;
        if rest.len() < zero + 1 + oid_len {
            return Err("tree OID truncated".into());
        }
        let oid = &rest[zero + 1..zero + 1 + oid_len];
        if entry_name == name {
            return Ok(hex(oid));
        }
        rest = &rest[zero + 1 + oid_len..];
    }
    Err("tree path absent".into())
}
// Git's SHA-256 tree entries carry 32-byte raw OIDs.
fn tree_entry(tree: &[u8], name: &str, format: &str) -> Result<String> {
    if format == "sha1" {
        return entry(tree, name);
    }
    let mut rest = tree;
    while !rest.is_empty() {
        let zero = rest
            .iter()
            .position(|b| *b == 0)
            .ok_or("tree entry invalid")?;
        let header = std::str::from_utf8(&rest[..zero]).map_err(|e| e.to_string())?;
        let (_, entry_name) = header.split_once(' ').ok_or("tree mode missing")?;
        if rest.len() < zero + 33 {
            return Err("tree OID truncated".into());
        }
        if entry_name == name {
            return Ok(hex(&rest[zero + 1..zero + 33]));
        }
        rest = &rest[zero + 33..];
    }
    Err("tree path absent".into())
}

pub fn inspect(grant: &Value) -> Result<Value> {
    let descriptor = &grant["metrics"];
    let repository = Path::new(string(grant, "repository")?);
    if !repository.is_absolute() || string(descriptor, "path")? != "docs/ai-workflow-metrics.jsonl"
    {
        return Err("metrics source path invalid".into());
    }
    let format = git_text(repository, &["rev-parse", "--show-object-format=storage"])?;
    let commit_oid = string(descriptor, "commit_oid")?;
    let commit = object(repository, commit_oid, "commit", &format)?;
    let commit_text = std::str::from_utf8(&commit).map_err(|e| e.to_string())?;
    let root_oid = commit_text
        .lines()
        .next()
        .and_then(|s| s.strip_prefix("tree "))
        .ok_or("commit tree missing")?;
    let root = object(repository, root_oid, "tree", &format)?;
    let docs_oid = tree_entry(&root, "docs", &format)?;
    let docs = object(repository, &docs_oid, "tree", &format)?;
    let blob_oid = tree_entry(&docs, "ai-workflow-metrics.jsonl", &format)?;
    if blob_oid != string(descriptor, "blob_oid")? {
        return Err("metrics blob identity differs".into());
    }
    let blob = object(repository, &blob_oid, "blob", &format)?;
    let row_bytes = blob
        .split(|b| *b == b'\n')
        .nth(20)
        .ok_or("metrics row absent")?;
    let row_bytes = row_bytes.strip_suffix(b"\r").unwrap_or(row_bytes);
    if sha(row_bytes) != string(descriptor, "row_sha256")? {
        return Err("metrics row digest differs".into());
    }
    let row: Value = serde_json::from_slice(row_bytes).map_err(|e| e.to_string())?;
    if string(&row, "run_id")? != string(grant, "run_id")? || row["slice_number"] != 6 {
        return Err("metrics case identity differs".into());
    }
    let gate = row["gate_checks"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or("metrics gate absent")?;
    if string(gate, "receipt_digest")? != string(descriptor, "gate_digest")? {
        return Err("metrics gate digest differs".into());
    }
    let stage_total = gate["stage_total"]
        .as_u64()
        .ok_or("metric stage count missing")?;
    let worker = &row["worker_metrics"];
    let independent_review = &worker["provider_role_metrics"]["independent_review"];
    let cost = worker["provider_cost_usd"]
        .as_number()
        .ok_or("metrics cost missing")?
        .to_string();
    super::decimal_scaled(&cost)?;
    Ok(
        json!({"source_owner":"slice-supervisor","source_class":"frozen_metrics_row",
        "commit_oid":commit_oid,"blob_oid":blob_oid,"row_number":21,
        "row_sha256":string(descriptor,"row_sha256")?,"gate_digest":string(gate,"receipt_digest")?,
        "stage_total":stage_total,"provider_cost_usd":cost,
        "provider_successful_calls":worker["provider_successful_calls"],
        "provider_failed_calls":worker["provider_failed_calls"],
        "provider_cache_creation_tokens":worker["provider_cache_creation_tokens"],
        "provider_cache_read_tokens":worker["provider_cache_read_tokens"],
        "provider_output_tokens":worker["provider_output_tokens"],
        "provider_thinking_tokens":worker["provider_thinking_tokens"],
        "independent_review_attempts":independent_review["attempts"],
        "independent_review_successful":independent_review["successful"],
        "independent_review_failed":independent_review["failed"],
        "source_limit":"Frozen reported metrics are not provider billing or a definition of gate stage semantics"}),
    )
}
