use repository_evidence_cbm::{
    parse_json, publish_artifact, read_artifact, sha256, source_currency, verify,
};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};

fn observation(tool: &str, data: Value, cursor: Option<&str>) -> Value {
    let mut args = json!({"project":"project-a","format":"json"});
    let result = if tool == "detect_changes" {
        args["scope"] = json!("doc_references");
        if let Some(cursor) = cursor {
            args["reference_cursor"] = json!(cursor);
        }
        data
    } else {
        args["snapshot"] = json!(true);
        json!({"store_generation":"generation-a","snapshot_consistent":true,"data":data})
    };
    json!({"request":{"tool":tool,"arguments":args},"response":json!({"content":[{"type":"text","text":result.to_string()}],"structuredContent":result,"isError":false}).to_string()})
}
fn document_page(more: bool) -> Value {
    json!({"scope":"doc_references","semantics":"referenced_targets_for_review","store_generation":"generation-a","coverage":"indexed_references_only","total":2,"total_relation":"eq","returned":1,"has_more":more,"truncated":more,"next_cursor":if more {json!("cursor-1")} else {Value::Null},"references":[{"document":"docs/notes.md","target":"module.post","status":"resolved"}]})
}
fn capture() -> Value {
    json!({"schema_version":1,"repository_id":"repo-a","project":"project-a","executable_sha256":"a".repeat(64),"initialization":json!({"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"codebase-memory-mcp","version":"fixture"}}}).to_string(),"project_binding":{"repository_path":"/fixture/repository","before":observation("query_graph",json!({"rows":[]}),None)["response"],"index_status":json!({"content":[{"type":"text","text":json!({"project":"project-a","root_path":"/fixture/repository"}).to_string()}],"structuredContent":{"project":"project-a","root_path":"/fixture/repository"},"isError":false}).to_string(),"after":observation("query_graph",json!({"rows":[]}),None)["response"]},"store_generation":"generation-a","anchors":[{"path":"docs/notes.md","content":"Posted entries are summed.\n","sha256":sha256(b"Posted entries are summed.\n")}],"observations":[observation("get_code_snippet",json!({"content_verified":true,"source":"def post(): pass"}),None)],"limitations":["Bounded attributed observations; applicability not assessed."]})
}
fn checked(
    value: &Value,
) -> Result<repository_evidence_cbm::Artifact, repository_evidence_cbm::Error> {
    let bytes = serde_json::to_vec(value).unwrap();
    verify(&bytes, &sha256(&bytes), "repo-a")
}
fn inventory(root: &Path) -> Vec<(String, u32, Vec<u8>)> {
    let mut entries: Vec<_> = fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().into_owned(),
                entry.metadata().unwrap().permissions().mode(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect();
    entries.sort();
    entries
}

#[test]
fn exact_capture_survives_publication_and_source_changes_without_changing_reference() {
    let artifacts = tempfile::tempdir().unwrap();
    let repository = tempfile::tempdir().unwrap();
    fs::create_dir(repository.path().join("docs")).unwrap();
    fs::write(
        repository.path().join("docs/notes.md"),
        "Posted entries are summed.\n",
    )
    .unwrap();
    let artifact = checked(&capture()).unwrap();
    let reference = artifact.reference();
    publish_artifact(artifacts.path(), &artifact).unwrap();
    publish_artifact(artifacts.path(), &artifact).unwrap();
    let before = inventory(artifacts.path());
    let reopened = read_artifact(artifacts.path(), artifact.sha256(), "repo-a").unwrap();
    assert_eq!(
        source_currency(&reopened, repository.path()).unwrap()["anchors"][0]["state"],
        "bytes_match"
    );
    fs::write(
        repository.path().join("docs/notes.md"),
        "Posted entries are discarded.\n",
    )
    .unwrap();
    let changed = source_currency(&reopened, repository.path()).unwrap();
    assert_eq!(changed["anchors"][0]["state"], "bytes_changed");
    assert_eq!(changed["applicability"], "not_assessed");
    fs::remove_file(repository.path().join("docs/notes.md")).unwrap();
    assert_eq!(
        source_currency(&reopened, repository.path()).unwrap()["anchors"][0]["state"],
        "unavailable"
    );
    assert_eq!(reference, reopened.reference());
    assert_eq!(before, inventory(artifacts.path()));
}

#[test]
fn historical_projects_share_repository_custody_after_relocation() {
    let artifacts = tempfile::tempdir().unwrap();
    let relocated = tempfile::tempdir().unwrap();
    fs::create_dir(relocated.path().join("docs")).unwrap();
    fs::write(
        relocated.path().join("docs/notes.md"),
        "Posted entries are summed.\n",
    )
    .unwrap();
    let old = checked(&capture()).unwrap();
    // Each capture retains its own internally coherent project/root binding.
    let new_value: Value = serde_json::from_str(
        &capture()
            .to_string()
            .replace("project-a", "project-b")
            .replace("/fixture/repository", "/fixture/relocated"),
    )
    .unwrap();
    let new = checked(&new_value).unwrap();
    publish_artifact(artifacts.path(), &old).unwrap();
    publish_artifact(artifacts.path(), &new).unwrap();
    let before = inventory(artifacts.path());
    for original in [&old, &new] {
        let historical = read_artifact(artifacts.path(), original.sha256(), "repo-a").unwrap();
        assert_eq!(historical.bytes(), original.bytes());
        assert_eq!(
            source_currency(&historical, relocated.path()).unwrap()["anchors"][0]["state"],
            "bytes_match"
        );
        assert!(read_artifact(artifacts.path(), original.sha256(), "another-repository").is_err());
    }
    let mut mixed = new_value;
    mixed["observations"][0]["request"]["arguments"]["project"] = json!("project-a");
    assert!(checked(&mixed).is_err());
    assert_eq!(before, inventory(artifacts.path()));
}

#[test]
fn mixed_publications_project_changes_and_unbound_reads_are_refused() {
    let mut value = capture();
    value["observations"]
        .as_array_mut()
        .unwrap()
        .push(observation("query_graph", json!({"rows":[]}), None));
    let mut response: Value =
        serde_json::from_str(value["observations"][1]["response"].as_str().unwrap()).unwrap();
    response["structuredContent"]["store_generation"] = json!("generation-b");
    response["content"][0]["text"] = json!(response["structuredContent"].to_string());
    value["observations"][1]["response"] = json!(response.to_string());
    assert!(checked(&value).is_err());
    let mut value = capture();
    value["observations"][0]["request"]["arguments"]["project"] = json!("other-project");
    assert!(checked(&value).is_err());
    let mut value = capture();
    value["observations"][0]["request"]["arguments"]["snapshot"] = json!(false);
    assert!(checked(&value).is_err());
}

#[test]
fn incomplete_doc_pages_remain_partial_and_missing_or_forged_pages_fail() {
    let mut value = capture();
    value["observations"] = json!([observation("detect_changes", document_page(true), None)]);
    let partial = checked(&value).unwrap();
    assert_eq!(
        partial.capture().documentation_status().unwrap()["pagination"],
        "partial"
    );
    value["observations"]
        .as_array_mut()
        .unwrap()
        .push(observation(
            "detect_changes",
            document_page(false),
            Some("cursor-1"),
        ));
    let full = checked(&value).unwrap();
    assert_eq!(
        full.capture().documentation_status().unwrap()["pagination"],
        "complete"
    );
    value["observations"][1]["request"]["arguments"]["reference_cursor"] = json!("skipped-page");
    assert!(checked(&value).is_err());
    let mut value = capture();
    let mut page = document_page(false);
    page["returned"] = json!(0);
    page["references"] = json!([]);
    value["observations"] = json!([observation("detect_changes", page, None)]);
    assert!(checked(&value).is_err());
}

#[test]
fn unresolved_and_unavailable_doc_rows_are_preserved() {
    let mut page = document_page(false);
    page["returned"] = json!(2);
    page["references"] = json!([{"document":"docs/notes.md","status":"unresolved","reason":"ambiguous"},{"document":"docs/deleted.md","status":"unavailable"}]);
    let mut value = capture();
    value["observations"] = json!([observation("detect_changes", page, None)]);
    let artifact = checked(&value).unwrap();
    assert!(
        artifact.capture().observations()[0]
            .response
            .contains("unavailable")
    );
    assert!(
        artifact.capture().observations()[0]
            .response
            .contains("ambiguous")
    );
    assert_eq!(
        artifact.capture().documentation_status().unwrap()["coverage"],
        "indexed_references_only"
    );
}

#[test]
fn integrity_ambiguity_and_path_tampering_are_rejected() {
    let artifact = checked(&capture()).unwrap();
    let mut bytes = artifact.bytes().to_vec();
    bytes.push(b' ');
    assert!(verify(&bytes, artifact.sha256(), "repo-a").is_err());
    let mut value = capture();
    value["anchors"][0]["content"] = json!("changed");
    assert!(checked(&value).is_err());
    let mut value = capture();
    value["anchors"][0]["path"] = json!("../outside");
    assert!(checked(&value).is_err());
    assert!(parse_json(br#"{"outer":{"project":"one","project":"two"}}"#).is_err());
    let mut value = capture();
    let mut response: Value =
        serde_json::from_str(value["observations"][0]["response"].as_str().unwrap()).unwrap();
    response["content"][0]["text"] = json!("{}");
    value["observations"][0]["response"] = json!(response.to_string());
    assert!(checked(&value).is_err());
}

#[test]
fn a_project_bound_to_another_checkout_is_rejected_even_with_a_valid_artifact_digest() {
    let mut value = capture();
    let mut status: Value =
        serde_json::from_str(value["project_binding"]["index_status"].as_str().unwrap()).unwrap();
    status["structuredContent"]["root_path"] = json!("/another/repository");
    status["content"][0]["text"] = json!(status["structuredContent"].to_string());
    value["project_binding"]["index_status"] = json!(status.to_string());
    assert!(checked(&value).is_err());
}

#[test]
fn evidence_reads_and_invalid_requests_do_not_create_or_repair_files() {
    let root = tempfile::tempdir().unwrap();
    let artifact = checked(&capture()).unwrap();
    let path = publish_artifact(root.path(), &artifact).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let before = inventory(root.path());
    assert!(read_artifact(root.path(), "../anything", "repo-a").is_err());
    assert!(read_artifact(root.path(), artifact.sha256(), "repo-b").is_err());
    assert!(read_artifact(root.path(), &"0".repeat(64), "repo-a").is_err());
    assert_eq!(before, inventory(root.path()));
    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::write(outside.path(), artifact.bytes()).unwrap();
    fs::remove_file(&path).unwrap();
    symlink(outside.path(), &path).unwrap();
    assert!(read_artifact(root.path(), artifact.sha256(), "repo-a").is_err());
    assert_eq!(fs::read(outside.path()).unwrap(), artifact.bytes());
}

#[test]
fn simultaneous_identical_publication_is_idempotent_and_exposes_only_full_bytes() {
    let root = tempfile::tempdir().unwrap();
    let artifact = checked(&capture()).unwrap();
    std::thread::scope(|scope| {
        for _ in 0..2 {
            scope.spawn(|| publish_artifact(root.path(), &artifact).unwrap());
        }
    });
    let entries = inventory(root.path());
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].2, artifact.bytes());
}
