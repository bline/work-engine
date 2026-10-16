//! Raw Git identity must survive both Git object formats and replacement refs.
#[path = "../src/source_read/mod.rs"]
#[allow(dead_code)]
mod source_read;

use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_AUTHOR_NAME", "S5 Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "S5 Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{:?}: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn row(cost: &str) -> Value {
    serde_json::from_str(&format!(
        r#"{{"run_id":"skills-migration-app-server-20260828","slice_number":6,
      "gate_checks":[{{"receipt_digest":"{}","stage_total":8}}],
      "worker_metrics":{{"provider_cost_usd":{},"provider_successful_calls":9,
      "provider_failed_calls":1,"provider_cache_creation_tokens":5,
      "provider_cache_read_tokens":6,"provider_output_tokens":7,
      "provider_thinking_tokens":101197,"provider_role_metrics":{{"independent_review":
      {{"attempts":8,"successful":7,"failed":1}}}}}}}}"#,
        "a".repeat(64),
        cost
    ))
    .unwrap()
}

#[test]
fn sha256_git_tree_and_replacement_read_original_frozen_row() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let repo =
        std::env::temp_dir().join(format!("s5-metrics-sha256-{}-{unique}", std::process::id()));
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "--object-format=sha256", "-q"]);
    fs::create_dir(repo.join("docs")).unwrap();
    let path = repo.join("docs/ai-workflow-metrics.jsonl");
    let write_row = |cost: &str| {
        let mut lines = vec!["{}".to_string(); 20];
        lines.push(serde_json::to_string(&row(cost)).unwrap());
        fs::write(&path, format!("{}\n", lines.join("\n"))).unwrap();
        lines[20].clone()
    };
    let original_row = write_row("15.7545495");
    git(&repo, &["add", "docs/ai-workflow-metrics.jsonl"]);
    git(&repo, &["commit", "-qm", "original frozen row"]);
    let original = git(&repo, &["rev-parse", "HEAD"]);
    let blob = git(&repo, &["rev-parse", "HEAD:docs/ai-workflow-metrics.jsonl"]);
    assert_eq!(original.len(), 64);
    assert_eq!(blob.len(), 64);
    let grant = json!({"repository":repo,"run_id":"skills-migration-app-server-20260828",
        "metrics":{"path":"docs/ai-workflow-metrics.jsonl","commit_oid":original,
        "blob_oid":blob,"row_sha256":source_read::sha(original_row.as_bytes()),
        "gate_digest":"a".repeat(64)}});
    let first = source_read::frozen_metrics::inspect(&grant).unwrap();
    assert_eq!(first["provider_cost_usd"], "15.7545495");
    assert_eq!(first["row_number"], 21);
    write_row("999.0");
    git(&repo, &["add", "docs/ai-workflow-metrics.jsonl"]);
    git(&repo, &["commit", "-qm", "replacement row"]);
    let replacement = git(&repo, &["rev-parse", "HEAD"]);
    git(&repo, &["replace", original.as_str(), replacement.as_str()]);
    let after = source_read::frozen_metrics::inspect(&grant).unwrap();
    assert_eq!(after["provider_cost_usd"], "15.7545495");
    assert_eq!(after["row_sha256"], grant["metrics"]["row_sha256"]);
    fs::remove_dir_all(repo).unwrap();
}
