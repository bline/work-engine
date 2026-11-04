use serde_json::{Value, json};
use session_task_admission::{ReviewError, validate_review};

fn fixture() -> Value {
    let snapshot: Value = serde_json::from_str(include_str!("projection-fixture.json")).unwrap();
    serde_json::from_str(snapshot["tasks"][0]["request_json"].as_str().unwrap()).unwrap()
}
fn validate(row: &Value) -> Result<session_task_admission::Ref, ReviewError> {
    validate_review(&serde_json::to_string(row).unwrap())
}
#[test]
fn valid_review_binds_exact_raw_request() {
    let row = fixture();
    let raw = serde_json::to_string(&row).unwrap();
    let result = validate_review(&raw).unwrap();
    assert_eq!(result.id, "fixture-task");
    assert_ne!(result.sha256, validate_review(&(raw + " ")).unwrap().sha256);
}
#[test]
fn rejects_missing_or_modified_artifact() {
    let mut row = fixture();
    row["data"]["intent_artifacts"][0]["content_json"] = json!("{}");
    assert_eq!(validate(&row), Err(ReviewError::InvalidReference));
    let mut row = fixture();
    row["data"]["intent_artifacts"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    assert_eq!(validate(&row), Err(ReviewError::MissingArtifact));
}
#[test]
fn rejects_unknown_terms_and_effect_expansion() {
    let mut row = fixture();
    row["data"]["intent"]["new_effect"] = json!(true);
    assert_eq!(validate(&row), Err(ReviewError::InvalidJson));
    let mut row = fixture();
    row["data"]["intent"]["effects"]["network"] = json!(true);
    assert_eq!(validate(&row), Err(ReviewError::Unsupported));
}
#[test]
fn rejects_duplicate_artifact_and_away_mandate() {
    let mut row = fixture();
    let a = row["data"]["intent_artifacts"][0].clone();
    row["data"]["intent_artifacts"]
        .as_array_mut()
        .unwrap()
        .push(a);
    assert_eq!(validate(&row), Err(ReviewError::ConflictingArtifact));
    let mut row = fixture();
    row["data"]["intent"]["authority"]["mandate"] =
        row["data"]["intent"]["authority"]["mode"].clone();
    assert_eq!(validate(&row), Err(ReviewError::Unsupported));
}

#[test]
fn rejects_missing_mandate_field_and_conflicting_revision() {
    let mut row = fixture();
    row["data"]["intent"]["authority"]
        .as_object_mut()
        .unwrap()
        .remove("mandate");
    assert_eq!(validate(&row), Err(ReviewError::InvalidJson));
    let mut row = fixture();
    let mut other = row["data"]["intent_artifacts"][0].clone();
    other["content_json"] = json!("{}");
    other["ref"]["sha256"] =
        json!("44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a");
    row["data"]["intent_artifacts"]
        .as_array_mut()
        .unwrap()
        .push(other);
    assert_eq!(validate(&row), Err(ReviewError::ConflictingArtifact));
}

#[test]
fn rejects_lexical_path_escape() {
    let mut row = fixture();
    row["data"]["intent"]["effects"]["write_paths"] = json!(["../outside"]);
    assert_eq!(validate(&row), Err(ReviewError::Unsupported));
    row["data"]["intent"]["effects"]["write_paths"] = json!(["/absolute"]);
    assert_eq!(validate(&row), Err(ReviewError::Unsupported));
}

#[test]
fn rejects_signed_away_or_incomplete_mode_view() {
    use sha2::{Digest, Sha256};
    for incomplete in [false, true] {
        let mut row = fixture();
        let artifacts = row["data"]["intent_artifacts"].as_array_mut().unwrap();
        let mode = artifacts
            .iter_mut()
            .find(|a| a["ref"]["id"] == "mode")
            .unwrap();
        let mut content: Value =
            serde_json::from_str(mode["content_json"].as_str().unwrap()).unwrap();
        if incomplete {
            content
                .as_object_mut()
                .unwrap()
                .remove("pending_transition");
        } else {
            content["mode"] = json!("away");
        }
        let raw = content.to_string();
        mode["ref"]["sha256"] = json!(format!("{:x}", Sha256::digest(raw.as_bytes())));
        mode["content_json"] = json!(raw);
        let r = mode["ref"].clone();
        row["data"]["intent"]["authority"]["mode"] = r;
        assert_eq!(validate(&row), Err(ReviewError::Unsupported));
    }
}

#[test]
fn invalid_money_or_clock_terms_hold() {
    let mut row = fixture();
    row["data"]["intent"]["budget"]["maximum"] = json!("not-money");
    assert_eq!(validate(&row), Err(ReviewError::Unsupported));
    let mut row = fixture();
    row["data"]["intent"]["budget"]["closure_reserve"] = json!("2.00");
    assert_eq!(validate(&row), Err(ReviewError::Unsupported));
    let mut row = fixture();
    row["data"]["intent"]["clock"]["close_by"] = json!("2026-11-02T00:00:00Z");
    assert_eq!(validate(&row), Err(ReviewError::Unsupported));
}
