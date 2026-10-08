use crate::codec::{JsValue, digest_unchecked as digest, exact_fields, field, parse_json};
use crate::contract::{ClaimResult, ensure};
use crate::findings::{array, text_field, validate_reference};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const PROFILE: &str = "native-review-claims-v1";
pub const FINDING_PROFILE: &str = "revision-bound-review-finding-v1";
pub const PRODUCTION_PATH_PROFILE: &str = "production-path-v1";

/// Host-owned source and configuration files. Their expected byte hashes are
/// selected by the trusted host startup, never by a finding request.
#[derive(Clone, Debug)]
pub struct BootstrapFiles {
    pub root: PathBuf,
    pub config: PathBuf,
    pub source: PathBuf,
    pub expected_config_sha256: String,
    pub expected_source_sha256: String,
}

#[derive(Clone, Debug)]
pub struct OpenFiles {
    pub root: PathBuf,
    pub config: PathBuf,
    pub source: PathBuf,
    pub expected_config_sha256: String,
    pub expected_source_sha256: String,
}
impl From<BootstrapFiles> for OpenFiles {
    fn from(value: BootstrapFiles) -> Self {
        Self {
            root: value.root,
            config: value.config,
            source: value.source,
            expected_config_sha256: value.expected_config_sha256,
            expected_source_sha256: value.expected_source_sha256,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Bootstrap {
    pub root_id: String,
    pub grants: Vec<JsValue>,
    pub config_sha256: String,
    pub source_sha256: String,
    pub trusted_custody: Option<JsValue>,
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn load(files: &OpenFiles) -> ClaimResult<Bootstrap> {
    let config_bytes = fs::read(&files.config)
        .map_err(|e| crate::contract::ClaimError::new(format!("config read: {e}")))?;
    let source_bytes = fs::read(&files.source)
        .map_err(|e| crate::contract::ClaimError::new(format!("source read: {e}")))?;
    let config_sha256 = sha256_bytes(&config_bytes);
    let source_sha256 = sha256_bytes(&source_bytes);
    ensure(
        config_sha256 == files.expected_config_sha256,
        "bootstrap config digest mismatch",
    )?;
    ensure(
        source_sha256 == files.expected_source_sha256,
        "bootstrap source digest mismatch",
    )?;
    let config = parse_json(
        std::str::from_utf8(&config_bytes)
            .map_err(|_| crate::contract::ClaimError::new("config is not UTF-8"))?,
    )?;
    let source = parse_json(
        std::str::from_utf8(&source_bytes)
            .map_err(|_| crate::contract::ClaimError::new("source is not UTF-8"))?,
    )?;
    let version = field(&config, "schema_version")?;
    ensure(
        version == field(&source, "schema_version")?,
        "bootstrap schema mismatch",
    )?;
    let v2 = version == &JsValue::Number(2.0);
    ensure(
        v2 || version == &JsValue::Number(1.0),
        "bootstrap schema mismatch",
    )?;
    let mut config_fields = vec!["schema_version", "profile", "root_id", "grants"];
    if v2 {
        config_fields.push("trusted_custody");
    }
    exact_fields(&config, &config_fields, "bootstrap config")?;
    let mut source_fields = vec![
        "schema_version",
        "owner",
        "reference",
        "revision",
        "freshness",
        "profile",
        "actors",
        "decision_scopes",
        "grant_ids",
    ];
    if v2 {
        source_fields.push("custody_config_sha256");
    }
    exact_fields(&source, &source_fields, "bootstrap source")?;
    let trusted_custody = if v2 {
        let custody = field(&config, "trusted_custody")?.clone();
        exact_fields(
            &custody,
            &[
                "owner",
                "verifier",
                "profile",
                "artifact_schemes",
                "reference_schemes",
            ],
            "trusted custody",
        )?;
        for key in ["owner", "verifier", "profile"] {
            text_field(&custody, key)?;
        }
        strings(&custody, "artifact_schemes")?;
        strings(&custody, "reference_schemes")?;
        ensure(
            text_field(&source, "custody_config_sha256")? == digest(&custody),
            "custody source mismatch",
        )?;
        Some(custody)
    } else {
        None
    };
    ensure(
        text_field(&config, "profile")? == PROFILE && text_field(&source, "profile")? == PROFILE,
        "bootstrap profile mismatch",
    )?;
    let root_id = text_field(&config, "root_id")?;
    let actors = strings(&source, "actors")?;
    let scopes = strings(&source, "decision_scopes")?;
    let ids = strings(&source, "grant_ids")?;
    let grants = array(field(&config, "grants")?, "grants")?.to_vec();
    ensure(
        !grants.is_empty() && grants.len() <= 64,
        "invalid grant count",
    )?;
    let mut seen = std::collections::HashSet::new();
    for grant in &grants {
        validate_grant(grant)?;
        ensure(
            v2 || text_field(grant, "profile")? == FINDING_PROFILE,
            "production grant requires bootstrap v2",
        )?;
        let grant_id = text_field(grant, "grant_id")?;
        ensure(
            seen.insert(grant_id.clone()) && ids.contains(&grant_id),
            "unregistered or duplicate grant",
        )?;
        ensure(
            actors.contains(&text_field(grant, "actor")?)
                && scopes.contains(&text_field(grant, "decision_scope")?),
            "grant actor or scope is not sourced",
        )?;
        let reference = field(grant, "authority_reference")?;
        for key in ["owner", "reference", "revision", "freshness"] {
            ensure(
                field(reference, key)? == field(&source, key)?,
                "grant authority source mismatch",
            )?;
        }
        ensure(
            text_field(reference, "integrity_sha256")? == source_sha256,
            "grant authority source integrity mismatch",
        )?;
    }
    ensure(
        seen.len() == ids.len(),
        "source grant set differs from config",
    )?;
    Ok(Bootstrap {
        root_id,
        grants,
        config_sha256,
        source_sha256,
        trusted_custody,
    })
}

fn strings(value: &JsValue, key: &str) -> ClaimResult<Vec<String>> {
    let mut set = std::collections::HashSet::new();
    let mut output = Vec::new();
    for item in array(field(value, key)?, key)? {
        let text = crate::findings::text(item, key)?;
        ensure(set.insert(text.clone()), "duplicate source entry")?;
        output.push(text);
    }
    ensure(!output.is_empty(), "empty source entry list")?;
    Ok(output)
}

pub(crate) fn validate_grant(grant: &JsValue) -> ClaimResult<()> {
    exact_fields(
        grant,
        &[
            "schema_version",
            "grant_id",
            "actor",
            "profile",
            "permissions",
            "decision_scope",
            "authority_reference",
        ],
        "grant",
    )?;
    ensure(
        field(grant, "schema_version")? == &JsValue::Number(1.0),
        "grant schema mismatch",
    )?;
    for key in ["grant_id", "actor", "decision_scope"] {
        text_field(grant, key)?;
    }
    let profile = text_field(grant, "profile")?;
    ensure(
        profile == FINDING_PROFILE || profile == PRODUCTION_PATH_PROFILE,
        "grant profile mismatch",
    )?;
    let permissions = strings(grant, "permissions")?;
    ensure(
        permissions.iter().all(|p| {
            if profile == FINDING_PROFILE {
                p == "create_claim" || p == "publish_revision" || p == "record_reliance"
            } else {
                p == "record_observation" || p == "establish_claim" || p == "read_admission"
            }
        }),
        "unsupported grant permission",
    )?;
    let reference = field(grant, "authority_reference")?;
    validate_reference(reference)?;
    ensure(
        text_field(reference, "status")? == "verified",
        "grant reference is not verified",
    )?;
    Ok(())
}

pub(crate) fn grant_digest(grant: &JsValue) -> String {
    digest(grant)
}

pub(crate) fn checked_absolute_root(path: &Path) -> ClaimResult<PathBuf> {
    ensure(path.is_absolute(), "root must be absolute")?;
    let canonical = path
        .canonicalize()
        .map_err(|e| crate::contract::ClaimError::new(format!("root path: {e}")))?;
    ensure(canonical.to_str().is_some(), "root path is not UTF-8")?;
    ensure(path == canonical, "root path is redirected")?;
    ensure(canonical.is_dir(), "root is not a directory")?;
    Ok(canonical)
}
