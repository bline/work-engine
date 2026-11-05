//! Startup-selected read capabilities. This adapter has no publication port.
use claim_evidence::{
    OpenFiles, RepositoryReadStartup, RepositoryReader,
    codec::{canonical_json, parse_json},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectionPage {
    #[serde(default)]
    offset: usize,
    #[serde(default = "projection_chunk_limit")]
    max_bytes: usize,
    #[serde(default)]
    sha256: Option<String>,
}
fn projection_chunk_limit() -> usize {
    32768
}

fn encode_projection(
    canonical: String,
    page: Option<ProjectionPage>,
) -> Result<Value, &'static str> {
    let sha = repository_evidence_cbm::sha256(canonical.as_bytes());
    if page.is_none() && canonical.len() <= projection_chunk_limit() {
        return Ok(
            json!({"encoding":"claim-evidence-canonical-json-v1","canonical_projection":canonical,"projection_sha256":sha}),
        );
    }
    let page = page.unwrap_or(ProjectionPage {
        offset: 0,
        max_bytes: projection_chunk_limit(),
        sha256: None,
    });
    if !(4..=32768).contains(&page.max_bytes)
        || page.offset >= canonical.len()
        || !canonical.is_char_boundary(page.offset)
        || (page.offset != 0 && page.sha256.is_none())
    {
        return Err(invalid());
    }
    if let Some(expected) = page.sha256 {
        if expected.len() != 64
            || !expected
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid());
        }
        if expected != sha {
            return Err("claim_projection_changed");
        }
    }
    let mut end = page
        .offset
        .saturating_add(page.max_bytes)
        .min(canonical.len());
    while !canonical.is_char_boundary(end) {
        end -= 1;
    }
    Ok(
        json!({"encoding":"claim-evidence-canonical-json-v1","projection_sha256":sha,"offset":page.offset,"total_bytes":canonical.len(),"canonical_projection_page":&canonical[page.offset..end],"has_more":end<canonical.len(),"next_offset":if end<canonical.len(){Some(end)}else{None}}),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadProfile {
    schema_version: u8,
    root: PathBuf,
    config: PathBuf,
    source: PathBuf,
    expected_config_sha256: String,
    expected_source_sha256: String,
    repository_id: String,
    decision_scope: String,
    #[serde(default)]
    evidence: Option<EvidenceProfile>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceProfile {
    artifacts: PathBuf,
    repository: PathBuf,
    project: String,
}

pub struct Reads {
    profile: ReadProfile,
}
fn invalid() -> &'static str {
    "invalid_claim_request"
}
fn owner() -> &'static str {
    "claim_owner_unavailable"
}

impl Reads {
    fn startup(&self) -> RepositoryReadStartup {
        RepositoryReadStartup {
            files: OpenFiles {
                root: self.profile.root.clone(),
                config: self.profile.config.clone(),
                source: self.profile.source.clone(),
                expected_config_sha256: self.profile.expected_config_sha256.clone(),
                expected_source_sha256: self.profile.expected_source_sha256.clone(),
            },
            repository_id: self.profile.repository_id.clone(),
            decision_scope: self.profile.decision_scope.clone(),
        }
    }
    pub fn load(profile: ReadProfile) -> Result<Self, &'static str> {
        if profile.schema_version != 1
            || profile.repository_id.is_empty()
            || profile.decision_scope.is_empty()
        {
            return Err("invalid_claim_profile");
        }
        for path in [&profile.root, &profile.config, &profile.source] {
            if !path.is_absolute() || path.canonicalize().ok().as_ref() != Some(path) {
                return Err("invalid_claim_profile");
            }
        }
        if let Some(e) = &profile.evidence
            && (!e.artifacts.is_absolute()
                || e.artifacts.canonicalize().ok().as_ref() != Some(&e.artifacts)
                || !e.artifacts.is_dir()
                || !e.repository.is_absolute()
                || e.repository.canonicalize().ok().as_ref() != Some(&e.repository)
                || e.project.is_empty())
        {
            return Err("invalid_claim_profile");
        }
        let reads = Self { profile };
        RepositoryReader::open(reads.startup()).map_err(|_| "invalid_claim_profile")?;
        Ok(reads)
    }
    pub fn operation(name: &str) -> bool {
        matches!(
            name,
            "discover_repository_facts"
                | "resolve_repository_fact"
                | "read_repository_evidence"
                | "check_repository_evidence_currency"
        )
    }
    pub fn call(&self, name: &str, args: Value) -> Result<Value, &'static str> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Discover {
            query: String,
            #[serde(default = "default_limit")]
            limit: usize,
            #[serde(default)]
            cursor: Option<Value>,
            #[serde(default)]
            projection_page: Option<ProjectionPage>,
        }
        fn default_limit() -> usize {
            20
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Resolve {
            claim_id: String,
            revision_id: String,
            revision_sha256: String,
            #[serde(default)]
            projection_page: Option<ProjectionPage>,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Evidence {
            sha256: String,
            #[serde(default)]
            offset: usize,
            #[serde(default = "chunk_limit")]
            max_bytes: usize,
        }
        fn chunk_limit() -> usize {
            16384
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Currency {
            sha256: String,
        }
        let (projection, page) = match name {
            "discover_repository_facts" => {
                let request: Discover = serde_json::from_value(args).map_err(|_| invalid())?;
                if request.query.is_empty()
                    || request.query.len() > 512
                    || !(1..=100).contains(&request.limit)
                {
                    return Err(invalid());
                }
                let cursor = request
                    .cursor
                    .map(|v| parse_json(&v.to_string()))
                    .transpose()
                    .map_err(|_| invalid())?;
                let projection = RepositoryReader::open(self.startup())
                    .map_err(|_| owner())?
                    .discover(&request.query, request.limit, cursor.as_ref())
                    .map_err(|_| owner())?;
                (projection, request.projection_page)
            }
            "resolve_repository_fact" => {
                let request: Resolve = serde_json::from_value(args).map_err(|_| invalid())?;
                let projection = RepositoryReader::open(self.startup())
                    .map_err(|_| owner())?
                    .resolve(
                        &request.claim_id,
                        &request.revision_id,
                        &request.revision_sha256,
                    )
                    .map_err(|_| owner())?;
                (projection, request.projection_page)
            }
            "read_repository_evidence" => {
                let request: Evidence = serde_json::from_value(args).map_err(|_| invalid())?;
                if !(4..=32768).contains(&request.max_bytes) {
                    return Err(invalid());
                }
                let e = self.profile.evidence.as_ref().ok_or_else(invalid)?;
                let artifact = repository_evidence_cbm::read_artifact(
                    &e.artifacts,
                    &request.sha256,
                    &self.profile.repository_id,
                )
                .map_err(|_| owner())?;
                let text = std::str::from_utf8(artifact.bytes()).map_err(|_| owner())?;
                if request.offset >= text.len() || !text.is_char_boundary(request.offset) {
                    return Err(invalid());
                }
                let mut end = request
                    .offset
                    .saturating_add(request.max_bytes)
                    .min(text.len());
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
                return Ok(
                    json!({"schema_version":1,"artifact_sha256":artifact.sha256(),"reference":artifact.reference(),"offset":request.offset,"total_bytes":text.len(),"utf8":&text[request.offset..end],"has_more":end<text.len(),"next_offset":if end<text.len(){Some(end)}else{None},"documentation":artifact.capture().documentation_status().map_err(|_|owner())?,"applicability":"not_assessed"}),
                );
            }
            "check_repository_evidence_currency" => {
                let request: Currency = serde_json::from_value(args).map_err(|_| invalid())?;
                let e = self.profile.evidence.as_ref().ok_or_else(invalid)?;
                let artifact = repository_evidence_cbm::read_artifact(
                    &e.artifacts,
                    &request.sha256,
                    &self.profile.repository_id,
                )
                .map_err(|_| owner())?;
                return repository_evidence_cbm::source_currency(&artifact, &e.repository)
                    .map_err(|_| owner());
            }
            _ => return Err(invalid()),
        };
        // Keep the owner's exact JSON encoding, including historical UTF-16
        // strings that serde_json::Value cannot represent as Rust strings.
        let canonical = canonical_json(&projection).map_err(|_| owner())?;
        encode_projection(canonical, page)
    }
    pub fn tools(&self) -> Vec<Value> {
        let mut tools = vec![
            json!({"name":"discover_repository_facts","description":"Discover bounded repository fact candidates in the startup-selected read scope. canonical_projection preserves the native claim JSON and provenance; applicability is not_assessed. Read-only.","inputSchema":{"type":"object","additionalProperties":false,"required":["query"],"properties":{"query":{"type":"string","minLength":1,"maxLength":512},"limit":{"type":"integer","minimum":1,"maximum":100},"cursor":{"type":"object"}}}}),
            json!({"name":"resolve_repository_fact","description":"Resolve one exact immutable repository fact revision and digest. canonical_projection contains the complete native claim JSON, authority and limitations. This selects no latest revision and grants no authority. Read-only.","inputSchema":{"type":"object","additionalProperties":false,"required":["claim_id","revision_id","revision_sha256"],"properties":{"claim_id":{"type":"string","minLength":1},"revision_id":{"type":"string","minLength":1},"revision_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}}}),
        ];
        let page = json!({"type":"object","additionalProperties":false,"properties":{"offset":{"type":"integer","minimum":0},"max_bytes":{"type":"integer","minimum":4,"maximum":32768},"sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}});
        for tool in &mut tools {
            tool["inputSchema"]["properties"]["projection_page"] = page.clone();
            let description = tool["description"].as_str().unwrap().to_owned();
            tool["description"] = json!(format!(
                "{description} Projections above 32768 bytes return canonical_projection_page, projection_sha256 and next_offset. Reissue the same request with projection_page.offset and projection_page.sha256 to reconstruct exact bytes; claim_projection_changed requires restarting at offset zero."
            ));
        }
        if self.profile.evidence.is_some() {
            tools.push(json!({"name":"read_repository_evidence","description":"Read an integrity-checked immutable CBM capture in UTF-8 byte pages. Capture verification does not establish proposition truth or applicability. Read-only.","inputSchema":{"type":"object","additionalProperties":false,"required":["sha256"],"properties":{"sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"},"offset":{"type":"integer","minimum":0},"max_bytes":{"type":"integer","minimum":4,"maximum":32768}}}}));
            tools.push(json!({"name":"check_repository_evidence_currency","description":"Compare only the captured source anchors with the startup-selected checkout. Reports byte equality, change or unavailability; semantic applicability remains not_assessed. Read-only.","inputSchema":{"type":"object","additionalProperties":false,"required":["sha256"],"properties":{"sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}}}));
        }
        // These descriptors cover only local, nonmutating repository reads.
        // Hints describe behavior; startup custody still owns access and scope.
        for tool in &mut tools {
            tool["annotations"] = json!({
                "readOnlyHint": true,
                "destructiveHint": false,
                "idempotentHint": true,
                "openWorldHint": false
            });
        }
        tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_canonical_projections_reconstruct_with_bounded_digest_bound_pages() {
        // Includes UTF-8, escapes and a legacy UTF-16 surrogate representation.
        let canonical = canonical_json(
            &parse_json(&format!(
                "{{\"proposition\":{},\"legacy\":\"\\ud800\"}}",
                json!("🌲\n\\\"".repeat(160_000)),
            ))
            .unwrap(),
        )
        .unwrap();
        assert!(canonical.len() > 1024 * 1024);
        let mut page = encode_projection(canonical.clone(), None).unwrap();
        let sha = page["projection_sha256"].as_str().unwrap().to_owned();
        let mut bytes = String::new();
        let mut previous = 0;
        loop {
            assert!(page.to_string().len() < 1024 * 1024);
            assert!(page.get("canonical_projection").is_none());
            assert_eq!(page["projection_sha256"], sha);
            assert_eq!(page["offset"], previous);
            bytes.push_str(page["canonical_projection_page"].as_str().unwrap());
            if page["has_more"] == false {
                break;
            }
            let next = page["next_offset"].as_u64().unwrap() as usize;
            assert!(next > previous);
            previous = next;
            page = encode_projection(
                canonical.clone(),
                Some(ProjectionPage {
                    offset: next,
                    max_bytes: 32767,
                    sha256: Some(sha.clone()),
                }),
            )
            .unwrap();
        }
        assert_eq!(bytes, canonical);
        assert_eq!(repository_evidence_cbm::sha256(bytes.as_bytes()), sha);
        assert_eq!(
            encode_projection(
                format!("{canonical} "),
                Some(ProjectionPage {
                    offset: 4,
                    max_bytes: 1024,
                    sha256: Some(sha.clone())
                })
            ),
            Err("claim_projection_changed")
        );
        assert!(
            encode_projection(
                canonical.clone(),
                Some(ProjectionPage {
                    offset: 4,
                    max_bytes: 1024,
                    sha256: None
                })
            )
            .is_err()
        );
        assert!(
            encode_projection(
                canonical.clone(),
                Some(ProjectionPage {
                    offset: canonical.len(),
                    max_bytes: 1024,
                    sha256: Some(sha)
                })
            )
            .is_err()
        );
    }
}
