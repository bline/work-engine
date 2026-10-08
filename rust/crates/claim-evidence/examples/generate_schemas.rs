use claim_evidence::codec::canonical_json;
use claim_evidence::schema::{DTO_SCHEMA_KINDS, dto_schema};
fn main() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas");
    for kind in DTO_SCHEMA_KINDS {
        let path = directory.join(format!("{kind}.schema.json"));
        std::fs::write(
            path,
            canonical_json(&dto_schema(kind).expect("known schema")).expect("canonical schema"),
        )
        .expect("write schema");
    }
}
