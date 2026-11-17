use pi_accounting_spec_check::{self as spec, Dependencies};
use std::{env, fs, path::PathBuf, process};
fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "check".into());
    let root = PathBuf::from(args.next().unwrap_or_else(|| "../..".into()));
    let s = spec::read_spec(&root)?;
    spec::validate(&s)?;
    let interface = serde_json::from_slice(
        &fs::read(root.join("spec/interface.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    match command.as_str() {
        "check" => {
            let deps: Dependencies = serde_json::from_slice(
                &fs::read(root.join("spec/dependencies.json")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            let pending = spec::check_acknowledgements(&s, &deps)?;
            if args.next().as_deref() == Some("--generated") {
                spec::compare_generated(&root, &s, &interface)?;
            }
            println!(
                "{}",
                serde_json::json!({"status":"structurally_valid","nodes":spec::node_digests(&s)?.len(),"unestablished_consumers":pending,"limits":"declarations only; no financial evaluation or freeze"})
            );
        }
        "render" => {
            let dest = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| root.clone());
            if dest.components().any(|c| c.as_os_str() == "oracle") {
                return Err("oracle is never a rendering destination".into());
            }
            for (p, b) in spec::generated(&s, &interface)? {
                let target = dest.join(p);
                fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
                if target
                    .symlink_metadata()
                    .is_ok_and(|m| m.file_type().is_symlink())
                    || target
                        .parent()
                        .unwrap()
                        .canonicalize()
                        .map_err(|e| e.to_string())?
                        .components()
                        .any(|c| c.as_os_str() == "oracle")
                {
                    return Err("oracle/symlink rendering destination forbidden".into());
                }
                fs::write(target, b).map_err(|e| e.to_string())?;
            }
            println!("rendered approved destinations only");
        }
        "canonical" => {
            let path = args.next().ok_or("missing canonical path")?;
            let value = spec::parse_json(&fs::read(&path).map_err(|e| e.to_string())?)?;
            let schema: serde_json::Value = serde_json::from_slice(
                &fs::read(root.join("spec/ck.schema.json")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            spec::validate_schema(&schema, &value)?;
            let candidate: spec::Spec = serde_json::from_value(value).map_err(|e| e.to_string())?;
            spec::validate(&candidate)?;
            println!("canonical schema and structural agreement");
        }
        "corpus" => {
            let partition = args.next();
            let (cases, scopes) = spec::validate_corpus_documents(&root, partition.as_deref())?;
            println!(
                "finite supplied corpus references: {cases} cases/{scopes} scopes; raw reducer/live authority unestablished"
            );
        }
        "evidence" | "wire" => {
            let path = args.next().ok_or("missing document path")?;
            let value = spec::parse_json(&fs::read(&path).map_err(|e| e.to_string())?)?;
            if command == "wire" {
                spec::validate_wire_document(&s, &value)?;
            } else if value.get("documents").is_some() {
                let payloads = spec::resolve_accounting_payloads(&root, std::path::Path::new(&path), &value)?;
                spec::validate_accounting_bundle_with_payloads(&value, &payloads)?;
            } else {
                spec::validate_accounting_document(&value)?;
            }
            println!(
                "structurally_valid; external evidence/admission and financial truth unestablished"
            );
        }
        "impact" => {
            let deps: Dependencies = serde_json::from_slice(
                &fs::read(root.join("spec/dependencies.json")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            let changed: Vec<String> = args.collect();
            if !changed.is_empty() {
                println!(
                    "{}",
                    serde_json::json!({"changed":changed,"affected_registered_consumers":spec::affected_consumers(&s,&deps,&changed)?,"generated_views_require_comparison":true})
                );
                return Ok(());
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&spec::consumer_digests(&s, &deps)?)
                    .map_err(|e| e.to_string())?
            );
        }
        _ => return Err(
            "usage: pi-accounting-spec-check check|render|impact ROOT [--generated|OUTPUT_ROOT]"
                .into(),
        ),
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        process::exit(1)
    }
}
