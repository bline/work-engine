//! E2-AC1 (binding refusals and the path rule) and E2-AC7 (byte stability, limitations) through
//! the `compare` command line over real fixture files.

mod support;
use serde_json::json;
use std::path::PathBuf;
use support::*;

const MIN_REFUSAL_ROWS: usize = 6;

fn compare(w: &World, sets: &[&PathBuf], extra: &[&str]) -> Run {
    let mut args: Vec<String> = vec!["compare".into(), "--contract".into(), s(&w.contract)];
    for p in sets {
        args.push("--estimates".into());
        args.push(s(p));
    }
    args.extend(["--derived-at".into(), "2026-10-23T00:00:00Z".into()]);
    args.extend(extra.iter().map(|a| a.to_string()));
    cli(&args)
}

#[test]
fn compare_prints_a_stable_finding_with_every_limitation() {
    let w = world("compare-ok");
    let run = compare(&w, &[&w.sets[0], &w.sets[1]], &[]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stderr, "");
    assert!(run.stdout.ends_with("}\n") && !run.stdout.ends_with("\n\n"));
    let f = run.out_json();
    assert_eq!(f["schema"], json!("proposal-evaluation-comparison-v1"));
    assert_eq!(f["pairs"][0]["outcome"], json!("a_dominates_b"));
    assert_eq!(
        f["pairs"][0]["dropped_measures"][0]["measure_id"],
        json!("regression_risk")
    );
    assert_eq!(f["frontier"]["dominated"], json!(["alpha.two"]));
    assert_eq!(
        f["authority"],
        json!({"priority_authorized": false, "acceptance_authorized": false,
        "implementation_authorized": false, "readiness_judgment_issued": false})
    );
    assert_eq!(f["tool"]["name"], json!("work-engine-evaluation"));
    // Limitations: entries 1, 2, one dropped_measures entry for the one pair that drops, and 5.
    let lims: Vec<&str> = f["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect();
    assert_eq!(lims.len(), 4, "{lims:?}");
    assert!(lims[0].starts_with("conditional_on_contract:"));
    assert!(lims[1].starts_with("stale_is_cutoff_age:"));
    assert!(
        lims[2].starts_with("dropped_measures a=alpha.one b=alpha.two measures=regression_risk:")
    );
    assert!(lims[3].starts_with("estimate_sets_selected_at_derivation:"));
    // Byte stability: the same inputs give the same bytes, and the file form equals stdout.
    let again = compare(&w, &[&w.sets[0], &w.sets[1]], &[]);
    assert_eq!(run.stdout, again.stdout);
    let out = w.dir.join("finding.json");
    let filed = compare(&w, &[&w.sets[0], &w.sets[1]], &["--out", &s(&out)]);
    assert_eq!((filed.code, filed.stdout.as_str()), (0, ""));
    assert_eq!(std::fs::read_to_string(&out).unwrap(), run.stdout);
    // --out refuses to overwrite.
    let refused = compare(&w, &[&w.sets[0], &w.sets[1]], &["--out", &s(&out)]);
    assert_eq!(refused.code, 2);
    assert_eq!(std::fs::read_to_string(&out).unwrap(), run.stdout);
    // Estimate order on the command line does not change the bytes.
    let swapped = compare(&w, &[&w.sets[1], &w.sets[0]], &[]);
    assert_eq!(swapped.stdout, run.stdout);
}

#[test]
fn interval_robust_adds_the_degenerate_interval_limitation() {
    let w = world("compare-interval");
    rewrite_contract(&w, |c| c["uncertainty"] = json!("interval_robust"));
    let run = compare(&w, &[&w.sets[0], &w.sets[1]], &[]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let lims = run.out_json()["limitations"].clone();
    assert!(lims.as_array().unwrap().iter().any(|l| {
        l.as_str()
            .unwrap()
            .starts_with("point_as_degenerate_interval:")
    }));
}

#[test]
fn each_binding_refusal_exits_3_and_writes_nothing() {
    let (a, b) = (0, 1);
    let mut rows = 0;
    let mut case = |label: &str, code: &str, w: &World, sets: Vec<&PathBuf>| {
        let out = w.dir.join(format!("out-{rows}.json"));
        let run = compare(w, &sets, &["--out", &s(&out)]);
        assert_eq!(run.code, 3, "{label}: {}", run.stderr);
        assert_eq!(run.stdout, "", "{label}");
        assert!(
            run.pairs().iter().any(|(c, _)| c == code),
            "{label}: wanted {code} in {:?}",
            run.pairs()
        );
        assert!(!out.exists(), "{label}: an output file was written");
        rows += 1;
    };
    let w = world("refuse-missing");
    case("missing", "estimate_set_missing", &w, vec![&w.sets[a]]);
    let w = world("refuse-duplicate");
    case(
        "duplicate",
        "estimate_set_duplicate",
        &w,
        vec![&w.sets[a], &w.sets[b], &w.sets[a]],
    );
    let w = world("refuse-unexpected");
    let (third, _) = proposal(&w.dir, "alpha.three", 9);
    case(
        "unexpected",
        "estimate_set_unexpected",
        &w,
        vec![&w.sets[a], &w.sets[b], &third],
    );
    let w = world("refuse-revision");
    rewrite_contract(&w, |c| c["proposals"][0]["packet_sha256"] = json!(HEX_A));
    case(
        "revision",
        "basis_revision_mismatch",
        &w,
        vec![&w.sets[a], &w.sets[b]],
    );
    let w = world("refuse-cutoff");
    rewrite_contract(&w, |c| c["evidence_cutoff"] = json!("2026-10-22T09:59:59Z"));
    case(
        "after cutoff",
        "estimate_after_cutoff",
        &w,
        vec![&w.sets[a], &w.sets[b]],
    );
    let w = world("refuse-input");
    std::fs::write(w.dir.join("alpha.one/packet.json"), "{}\n").unwrap();
    case(
        "estimate set fails its check",
        "input_invalid",
        &w,
        vec![&w.sets[a], &w.sets[b]],
    );
    let w = world("refuse-contract");
    rewrite_contract(&w, |c| c["version"] = json!(0));
    case(
        "contract fails its check",
        "input_invalid",
        &w,
        vec![&w.sets[a], &w.sets[b]],
    );
    assert!(rows >= MIN_REFUSAL_ROWS, "{rows} rows");
}

#[test]
fn a_relative_path_exits_2_with_path_not_absolute_and_writes_nothing() {
    let w = world("relative");
    let out = w.dir.join("out.json");
    let rel_set = "alpha.one/set.json".to_string();
    let run = cli(&[
        "compare",
        "--contract",
        &s(&w.contract),
        "--estimates",
        &rel_set,
        "--estimates",
        &s(&w.sets[1]),
        "--derived-at",
        "2026-10-23T00:00:00Z",
        "--out",
        &s(&out),
    ]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.pairs()
            .contains(&("path_not_absolute".to_string(), "--estimates".to_string()))
    );
    let run = cli(&[
        "compare",
        "--contract",
        "contract.json",
        "--estimates",
        &s(&w.sets[0]),
        "--estimates",
        &s(&w.sets[1]),
        "--derived-at",
        "2026-10-23T00:00:00Z",
        "--out",
        &s(&out),
    ]);
    assert_eq!(run.code, 2);
    assert!(
        run.pairs()
            .contains(&("path_not_absolute".to_string(), "--contract".to_string()))
    );
    assert!(!out.exists());
    let dotted = format!("{}/./contract.json", s(&w.dir));
    let run = cli(&[
        "compare",
        "--contract",
        &dotted,
        "--estimates",
        &s(&w.sets[0]),
        "--derived-at",
        "2026-10-23T00:00:00Z",
    ]);
    assert_eq!(run.code, 2);
}
