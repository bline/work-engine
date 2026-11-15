//! CLI for the campaign-local evidence qualifier. Subcommands: manifest, make-plan, derive-checks,
//! preflight, postflight, verify-sdk. Output reports are written create-new; the exit code is 0
//! only when every required check identity passed. A panic, a CLI failure or an absent report is
//! never qualification.
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use we_evidence_gate::*;

fn flags(args: &[String]) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let mut it = args.iter();
    while let Some(k) = it.next() {
        if let (Some(name), Some(v)) = (k.strip_prefix("--"), it.next()) {
            m.insert(name.to_owned(), v.clone());
        }
    }
    m
}

fn need<'a>(f: &'a BTreeMap<String, String>, k: &str) -> &'a str {
    f.get(k).map(String::as_str).unwrap_or_else(|| {
        eprintln!("missing --{k}");
        std::process::exit(64)
    })
}

fn write_new(path: &str, value: &Value) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap_or_else(|e| {
            eprintln!("{path}: {e} (outputs are create-new)");
            std::process::exit(73)
        });
    file.write_all(&serde_json::to_vec_pretty(value).unwrap())
        .unwrap();
    file.write_all(b"\n").unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().cloned() else {
        eprintln!("usage: we-evidence-gate manifest|make-plan|derive-checks|preflight|postflight|verify-sdk --flag value ...");
        std::process::exit(64)
    };
    let f = flags(&args[1..]);
    let read = |k: &str| -> Value {
        serde_json::from_slice(&std::fs::read(need(&f, k)).unwrap_or_else(|e| {
            eprintln!("{k}: {e}");
            std::process::exit(66)
        }))
        .unwrap_or_else(|e| {
            eprintln!("{k}: {e}");
            std::process::exit(65)
        })
    };
    let run = || RunVars {
        invocation_id: need(&f, "invocation-id").to_owned(),
        run_dir: need(&f, "run-dir").to_owned(),
        records_dir: need(&f, "records-dir").to_owned(),
        map_sha256: need(&f, "expect-map-sha256").to_owned(),
        manifest_sha256: f.get("expect-manifest-sha256").cloned().unwrap_or_default(),
    };
    match cmd.as_str() {
        "manifest" => {
            let map = read("map");
            let sha = file_sha(Path::new(need(&f, "map"))).unwrap();
            match build_manifest(&map, &sha) {
                Ok(m) => write_new(need(&f, "out"), &m),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1)
                }
            }
        }
        "make-plan" => {
            let map = read("map");
            match make_plan(&map, &run()) {
                Ok(p) => write_new(need(&f, "out"), &p),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1)
                }
            }
        }
        "verify-sdk" => match verify_sdk(
            Path::new(need(&f, "root")),
            Path::new(need(&f, "manifest")),
            need(&f, "expect-manifest-sha256"),
        ) {
            Ok(v) => {
                let ok = v["exact_manifest_entry_set"] == Value::Bool(true);
                write_new(need(&f, "out"), &v);
                std::process::exit(if ok { 0 } else { 1 })
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1)
            }
        },
        "derive-checks" => {
            let map = read("map");
            match we_evidence_gate::spec::derive_checks(&map) {
                Ok(c) => write_new(need(&f, "out"), &c),
                Err(e) => {
                    for x in e {
                        eprintln!("{x}");
                    }
                    std::process::exit(1)
                }
            }
        }
        "preflight" | "postflight" => {
            let (map_p, plan_p, man_p) = (
                PathBuf::from(need(&f, "map")),
                PathBuf::from(need(&f, "plan")),
                PathBuf::from(need(&f, "manifest")),
            );
            let inputs = Inputs {
                map_path: &map_p,
                plan_path: &plan_p,
                manifest_path: &man_p,
                expect_map_sha256: need(&f, "expect-map-sha256"),
                expect_plan_sha256: need(&f, "expect-plan-sha256"),
                expect_manifest_sha256: need(&f, "expect-manifest-sha256"),
                run: run(),
            };
            let eval = if cmd == "preflight" {
                preflight(&inputs)
            } else {
                postflight(&inputs, Path::new(need(&f, "records-dir")))
            };
            let report = eval.to_json(&cmd);
            write_new(need(&f, "out"), &report);
            for e in &eval.spec_errors {
                eprintln!("spec: {e}");
            }
            for id in eval.incomplete() {
                eprintln!("incomplete: {id}");
            }
            let ok = eval.is_qualified();
            println!(
                "{cmd} qualified={ok} required={} passed={}",
                report["required_checks"], report["passed_checks"]
            );
            std::process::exit(if ok { 0 } else { 1 })
        }
        other => {
            eprintln!("unknown subcommand {other}");
            std::process::exit(64)
        }
    }
}
