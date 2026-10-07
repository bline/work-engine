use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use lifecycle_wire::schema_bundle_v1;
use serde_json::{Value, json};
use work_engine_types::CodecContract;

const SOURCES: &[&str] = &[
    "src/lib.rs",
    "src/json.rs",
    "src/observation.rs",
    "src/schema.rs",
    "examples/generate_schema_bundle.rs",
    "../lifecycle-client/src/lib.rs",
    "../lifecycle-client/README.md",
];
const FIXTURES: &[&str] = &[
    "command-v1.json",
    "failed-unsettled-v1.json",
    "historical-complete-current-blocked-v1.json",
    "observation-unavailable-v1.json",
    "unsupported-observation-v2.json",
    "compatibility-v1.json",
];

fn hashes(
    root: &Path,
    paths: impl IntoIterator<Item = &'static str>,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let mut result = BTreeMap::new();
    for path in paths {
        let bytes = fs::read(root.join(path))?;
        result.insert(
            path.into(),
            CodecContract::BinaryArtifactV1.digest_binary(&bytes)?.hex(),
        );
    }
    Ok(result)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(env::args().nth(1).ok_or("expected output directory")?);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::create_dir_all(&output)?;
    for (name, schema) in schema_bundle_v1() {
        let bytes = serde_json::to_vec_pretty(&schema)?;
        fs::write(output.join(name), [bytes.as_slice(), b"\n"].concat())?;
    }
    let schema_hashes = hashes(
        &output,
        [
            "command.schema.json",
            "error.schema.json",
            "snapshot.schema.json",
            "wait-request.schema.json",
            "wait-result.schema.json",
        ],
    )?;
    // The manifest itself is excluded so this digest has no self-reference.
    let bundle_hash = CodecContract::BinaryArtifactV1
        .digest_binary(&serde_json::to_vec(&schema_hashes)?)?
        .hex();
    let source_hashes = hashes(&root, SOURCES.iter().copied())?;
    let fixture_hashes = hashes(&root.join("tests/fixtures"), FIXTURES.iter().copied())?;
    let manifest: Value = json!({
        "schema_version": 1,
        "protocol_version": 1,
        "schema_dialect": "https://json-schema.org/draft/2020-12/schema",
        "generator": {"crate": "schemars", "version": "1.2.2", "settings": "draft2020_12"},
        "codec": {"domain": "work-engine.lifecycle", "command": "jcs-rfc8785-v1", "snapshot": "jcs-rfc8785-v1"},
        "source_sha256": source_hashes,
        "schema_sha256": schema_hashes,
        "schema_bundle_sha256": bundle_hash,
        "schema_bundle_hash_basis": "sha256 of compact lexicographically keyed schema_sha256 JSON",
        "fixture_sha256": fixture_hashes,
        "client_api": "ReadClient<T: ObservationTransport>::snapshot(subject,budget)/reconnect(subject,cursor,budget)/wait(subject,target,budget); target-aware budgeted passive snapshot/wait_hint",
        "unsupported_observation_variant": {"code": "unsupported_observation_variant", "behavior": "reject without lifecycle progress"},
        "u0_scope": "contract fixtures, not a live store projection",
    });
    fs::write(
        output.join("manifest.json"),
        [serde_json::to_vec_pretty(&manifest)?.as_slice(), b"\n"].concat(),
    )?;
    Ok(())
}
