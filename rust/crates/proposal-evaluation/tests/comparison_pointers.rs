//! E2 review fixes: stored estimate-set pointers resolved by the plain check (P2-2), and a
//! located `at` for a pointer `--rederive` could not read (P3-1).

mod support;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use support::*;

fn derive_to(w: &World, name: &str) -> PathBuf {
    let out = w.dir.join(name);
    let run = cli(&[
        "compare".to_string(),
        "--contract".to_string(),
        s(&w.contract),
        "--estimates".to_string(),
        s(&w.sets[0]),
        "--estimates".to_string(),
        s(&w.sets[1]),
        "--derived-at".to_string(),
        "2026-10-23T00:00:00Z".to_string(),
        "--out".to_string(),
        s(&out),
    ]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    out
}

fn read(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn check(path: &Path, rederive: bool) -> Run {
    let mut args = vec!["check".to_string(), "--kind".into(), "comparison".into()];
    if rederive {
        args.push("--rederive".into());
    }
    args.push(s(path));
    cli(&args)
}

/// Write the edited finding and check it without `--rederive`.
fn plain_check_of(w: &World, base: &Path, edit: impl Fn(&mut Value)) -> Run {
    let mut doc = read(base);
    edit(&mut doc);
    let path = w.dir.join("mutant.json");
    write_json(&path, &doc);
    check(&path, false)
}

#[test]
fn a_plain_check_reads_and_hashes_every_stored_estimate_set_pointer() {
    let w = world("ptr-plain");
    let finding = derive_to(&w, "finding.json");
    assert_eq!(check(&finding, false).code, 0);

    // sha-only edit
    let run = plain_check_of(&w, &finding, |d| {
        d["estimate_sets"][1]["sha256"] = json!(HEX_A)
    });
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert_eq!(
        run.pairs(),
        [("digest_mismatch".to_string(), "estimate_sets.1".to_string())]
    );

    // a nonexistent path
    let run = plain_check_of(&w, &finding, |d| {
        d["estimate_sets"][0]["path"] = json!("/nonexistent/set.json")
    });
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert_eq!(
        run.pairs(),
        [(
            "pointer_unresolved".to_string(),
            "estimate_sets.0".to_string()
        )]
    );

    // a path that names a directory, not a regular file
    let dir = s(&w.dir);
    let run = plain_check_of(&w, &finding, |d| d["estimate_sets"][0]["path"] = json!(dir));
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(has(&run.pairs(), "pointer_unresolved", "estimate_sets.0"));

    // the two paths swapped, each keeping the other's sha256
    let run = plain_check_of(&w, &finding, |d| {
        let (a, b) = (
            d["estimate_sets"][0]["path"].clone(),
            d["estimate_sets"][1]["path"].clone(),
        );
        d["estimate_sets"][0]["path"] = b;
        d["estimate_sets"][1]["path"] = a;
    });
    assert_eq!(run.code, 1, "{}", run.stdout);
    let pairs = run.pairs();
    assert!(
        has(&pairs, "digest_mismatch", "estimate_sets.0"),
        "{pairs:?}"
    );
    assert!(
        has(&pairs, "digest_mismatch", "estimate_sets.1"),
        "{pairs:?}"
    );
}

#[test]
fn a_plain_check_of_a_finding_whose_contract_and_sets_are_gone_names_each_pointer() {
    let w = world("ptr-plain-gone");
    let finding = derive_to(&w, "finding.json");
    std::fs::remove_file(&w.contract).unwrap();
    std::fs::remove_file(&w.sets[0]).unwrap();
    let run = check(&finding, false);
    assert_eq!(run.code, 1, "{}", run.stdout);
    let pairs = run.pairs();
    assert!(has(&pairs, "pointer_unresolved", "contract"), "{pairs:?}");
    assert!(
        has(&pairs, "pointer_unresolved", "estimate_sets.0"),
        "{pairs:?}"
    );
    assert!(
        !pairs.iter().any(|(_, at)| at == "estimate_sets.1"),
        "{pairs:?}"
    );
}

#[test]
fn rederive_names_the_pointer_it_could_not_read() {
    let w = world("ptr-rederive-at");
    let finding = derive_to(&w, "finding.json");
    std::fs::remove_file(&w.sets[1]).unwrap();
    let run = check(&finding, true);
    assert_eq!(run.code, 3, "{}", run.stderr);
    assert_eq!(
        run.pairs(),
        [(
            "pointer_unresolved".to_string(),
            "estimate_sets.1".to_string()
        )]
    );
    std::fs::remove_file(&w.contract).unwrap();
    let run = check(&finding, true);
    assert_eq!(run.code, 3, "{}", run.stderr);
    assert_eq!(
        run.pairs(),
        [("pointer_unresolved".to_string(), "contract".to_string())]
    );
}
