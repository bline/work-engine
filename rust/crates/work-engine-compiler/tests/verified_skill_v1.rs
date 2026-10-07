use std::collections::HashMap;
use std::path::PathBuf;
use work_engine_compiler::{ErrorCode, prepare_skill_verified, sha256_hex};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}
fn fixture() -> PathBuf {
    repo().join("app-server/tests/fixtures/compiler-c2/root")
}
fn case(id: &str) -> (String, Vec<u8>) {
    let dir = repo()
        .join("rust/crates/work-engine-compiler/tests/fixtures/v1/skills")
        .join(id);
    (
        std::fs::read_to_string(dir.join("structure.yaml")).unwrap(),
        std::fs::read(dir.join("interface.yaml")).unwrap(),
    )
}
fn files(paths: &[String]) -> HashMap<String, Vec<u8>> {
    paths
        .iter()
        .map(|p| (p.clone(), std::fs::read(fixture().join(p)).unwrap()))
        .collect()
}

#[test]
fn role_free_source_checks_cover_complete_spans_and_every_binding() {
    let (structure, interface) = case("p-rep");
    let prepared = prepare_skill_verified(structure.as_bytes(), &interface).unwrap();
    let captured = files(&prepared.paths());
    let compared = prepared
        .check_sources(&captured)
        .unwrap()
        .compare_projection(None, None, None, None)
        .unwrap()
        .into_compiled();
    assert_eq!(
        compared.ir["runtime_requirements"]["verified_sources"], false,
        "pure comparison does not attest capture provenance"
    );
    let expected =
        repo().join("rust/crates/work-engine-compiler/tests/fixtures/v1/skills/p-rep/expected.md");
    assert_eq!(compared.output, std::fs::read(expected).unwrap());
    for path in captured.keys() {
        let mut damaged = captured.clone();
        damaged.get_mut(path).unwrap().push(b'!');
        let prepared = prepare_skill_verified(structure.as_bytes(), &interface).unwrap();
        assert_eq!(
            prepared.check_sources(&damaged).unwrap_err().code,
            ErrorCode::SourceMismatch,
            "{path}"
        );
    }
}
#[test]
fn matching_authored_digest_cannot_hide_frontmatter_or_span_gaps() {
    let (structure, interface) = case("p-rep");
    let old = structure
        .lines()
        .find(|line| line.trim_start().starts_with("sha256:"))
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap();
    let canonical = "skills/repo-search/SKILL.md";
    let mut changed = files(&[canonical.to_owned()]);
    changed.get_mut(canonical).unwrap()[0] = b'X';
    let new = sha256_hex(&changed[canonical]);
    let altered = structure.replace(old, &new);
    let prepared = prepare_skill_verified(altered.as_bytes(), &interface).unwrap();
    let mut all = files(&prepared.paths());
    all.insert(canonical.to_owned(), changed.remove(canonical).unwrap());
    assert_eq!(
        prepared.check_sources(&all).unwrap_err().code,
        ErrorCode::SourceMismatch
    );
    let original = prepare_skill_verified(structure.as_bytes(), &interface).unwrap();
    let good = files(&original.paths());
    let first = structure.find("start_byte: ").unwrap();
    let rest = &structure[first + "start_byte: ".len()..];
    let number = rest.split(',').next().unwrap().trim();
    let shifted = structure.replacen(
        &format!("start_byte: {number}"),
        &format!("start_byte: {}", number.parse::<u64>().unwrap() + 1),
        1,
    );
    let prepared = prepare_skill_verified(shifted.as_bytes(), &interface).unwrap();
    assert_eq!(
        prepared.check_sources(&good).unwrap_err().code,
        ErrorCode::SourceMismatch
    );
    for start in [618, 620] {
        let changed = structure.replacen("start_byte: 619", &format!("start_byte: {start}"), 1);
        let prepared = prepare_skill_verified(changed.as_bytes(), &interface).unwrap();
        assert_eq!(
            prepared.check_sources(&good).unwrap_err().code,
            ErrorCode::SourceMismatch
        );
    }
    let eof = structure.replacen("end_byte: 4344", "end_byte: 4343", 1);
    let prepared = prepare_skill_verified(eof.as_bytes(), &interface).unwrap();
    assert_eq!(
        prepared.check_sources(&good).unwrap_err().code,
        ErrorCode::SourceMismatch
    );
    let mut damaged = good.clone();
    damaged.get_mut(canonical).unwrap()[500] = b'X';
    let new = sha256_hex(&damaged[canonical]);
    let altered = structure.replace(old, &new);
    let prepared = prepare_skill_verified(altered.as_bytes(), &interface).unwrap();
    assert_eq!(
        prepared.check_sources(&damaged).unwrap_err().code,
        ErrorCode::SourceMismatch
    );
}
