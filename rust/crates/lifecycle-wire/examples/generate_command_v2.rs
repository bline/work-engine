use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use lifecycle_wire::schema_bundle_v2;
use serde_json::{Value, json};
use work_engine_types::CodecContract;

const SOURCES: &[&str] = &[
    "src/command_v2.rs",
    "src/schema.rs",
    "examples/generate_command_v2.rs",
];
const FIXTURES: &[&str] = &["command-v2.json", "compatibility-v2.json"];

fn hashes(
    root: &Path,
    paths: impl IntoIterator<Item = &'static str>,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let mut result = BTreeMap::new();
    for path in paths {
        result.insert(
            path.into(),
            CodecContract::BinaryArtifactV1
                .digest_binary(&fs::read(root.join(path))?)?
                .hex(),
        );
    }
    Ok(result)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(env::args().nth(1).ok_or("expected output directory")?);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::create_dir_all(&output)?;
    for (name, schema) in schema_bundle_v2() {
        let mut bytes = serde_json::to_vec_pretty(&schema)?;
        bytes.push(b'\n');
        fs::write(output.join(name), bytes)?;
    }
    let schema_hashes = hashes(
        &output,
        ["command.schema.json", "command-result.schema.json"],
    )?;
    let bundle_hash = CodecContract::BinaryArtifactV1
        .digest_binary(&serde_json::to_vec(&schema_hashes)?)?
        .hex();
    let manifest: Value = json!({
        "schema_version":2,"protocol_version":2,
        "schema_dialect":"https://json-schema.org/draft/2020-12/schema",
        "generator":{"crate":"schemars","version":"1.2.2","settings":"draft2020_12"},
        "codec":{"domain":"work-engine.lifecycle","command":"jcs-rfc8785-v1","schema":2},
        "source_sha256":hashes(&root,SOURCES.iter().copied())?,
        "schema_sha256":schema_hashes,"schema_bundle_sha256":bundle_hash,
        "fixture_sha256":hashes(&root.join("tests/fixtures"),FIXTURES.iter().copied())?,
        "v1_compatibility_manifest_sha256":"f4c73a45602284989fc352f5529f421a5e2c73cdaece2c4678035825032727be",
    });
    let mut bytes = serde_json::to_vec_pretty(&manifest)?;
    bytes.push(b'\n');
    fs::write(output.join("manifest.json"), bytes)?;
    Ok(())
}
