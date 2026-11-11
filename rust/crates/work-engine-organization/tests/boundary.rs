//! 1A-10 boundary tests and the public-surface half of 1A-13(e). They read the crate's own
//! manifest and sources at test time (test code only; the crate itself performs no I/O) and check:
//! the dependency list, the absence of I/O, clock, environment and process APIs, and that no
//! public item is named or typed as an admission, acknowledgment, grant, gate output, dispatch
//! authority or capability, or can attach a principal to a component binding requirement.

use std::fs;
use std::path::PathBuf;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Non-test source files: everything in `src/` except in-crate test modules.
fn sources() -> Vec<(String, String)> {
    let mut files = Vec::new();
    for entry in fs::read_dir(crate_dir().join("src")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.ends_with(".rs") && !name.ends_with("_tests.rs") {
            files.push((name, fs::read_to_string(&path).unwrap()));
        }
    }
    files.sort();
    files
}

#[test]
fn normal_dependencies_are_exactly_the_four_pure_crates_and_there_are_no_others() {
    let manifest = fs::read_to_string(crate_dir().join("Cargo.toml")).unwrap();
    let mut section = String::new();
    let mut dependencies = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.to_owned();
        } else if !line.is_empty() && !line.starts_with('#') {
            if section.contains("dependencies") {
                dependencies.push((
                    section.clone(),
                    line.split(['=', ' ', '.']).next().unwrap().to_owned(),
                ));
            }
            assert!(
                !section.starts_with("features") && section != "build-dependencies",
                "no features or build scripts: {line}"
            );
        }
    }
    let expected: Vec<(String, String)> = ["serde", "serde_json", "sha2", "thiserror"]
        .iter()
        .map(|name| ("dependencies".to_owned(), (*name).to_owned()))
        .collect();
    assert_eq!(
        dependencies, expected,
        "no dev-dependencies, no other dependencies"
    );
    for forbidden in [
        "rustix",
        "rusqlite",
        "protected-fs",
        "protected_fs",
        "slice-campaign",
    ] {
        assert!(!manifest.contains(forbidden), "{forbidden}");
    }
    assert!(!crate_dir().join("build.rs").exists());
}

#[test]
fn the_crate_has_no_io_clock_environment_process_or_unsafe_code() {
    let forbidden = [
        "std::fs",
        "std::net",
        "std::process",
        "std::env",
        "std::io",
        "std::time",
        "std::thread",
        "SystemTime",
        "Instant",
        "rusqlite",
        "rustix",
        "protected_fs",
        "tokio",
        "unsafe",
        "File::",
        "Command::",
        "env::var",
        "TcpStream",
        "UdpSocket",
    ];
    let sources = sources();
    assert!(sources.len() >= 8, "found {} source files", sources.len());
    for (name, text) in &sources {
        for token in forbidden {
            assert!(!text.contains(token), "{name} contains {token:?}");
        }
        // `env!` is a compile-time macro (the crate version in the receipt); runtime access to
        // the environment is excluded by the `std::env` and `env::var` checks above.
        for (index, line) in text.lines().enumerate() {
            if line.contains("env!(") {
                assert!(
                    line.contains("CARGO_PKG_VERSION"),
                    "{name}:{}: {line}",
                    index + 1
                );
            }
        }
    }
}

struct PublicItems {
    /// (file, kind, name) for struct, enum, trait, type, mod, const, static and free `fn` items
    /// and names listed in `pub use`.
    types: Vec<(String, String, String)>,
    /// (file, signature line) for every `pub fn`, free functions and methods alike.
    functions: Vec<(String, String)>,
}

fn public_items() -> PublicItems {
    let mut types = Vec::new();
    let mut functions = Vec::new();
    for (file, text) in sources() {
        let mut reexport = String::new();
        for line in text.lines() {
            let trimmed = line.trim_start();
            let indented = line.len() != trimmed.len();
            if !reexport.is_empty() || trimmed.starts_with("pub use ") {
                reexport.push_str(trimmed);
                reexport.push(' ');
                if trimmed.ends_with(';') {
                    for name in reexport
                        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .filter(|word| !word.is_empty() && *word != "pub" && *word != "use")
                    {
                        types.push((file.clone(), "use".to_owned(), name.to_owned()));
                    }
                    reexport.clear();
                }
                continue;
            }
            for kind in ["struct", "enum", "trait", "type", "mod", "const", "static"] {
                if let Some(rest) = trimmed.strip_prefix(&format!("pub {kind} ")) {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    types.push((file.clone(), kind.to_owned(), name));
                }
            }
            if let Some(rest) = trimmed.strip_prefix("pub fn ") {
                functions.push((file.clone(), trimmed.to_owned()));
                if !indented {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    types.push((file.clone(), "fn".to_owned(), name));
                }
            }
        }
    }
    PublicItems { types, functions }
}

#[test]
fn no_public_item_is_named_as_an_admission_acknowledgment_grant_gate_dispatch_or_capability() {
    let items = public_items();
    assert!(
        items.types.len() > 60,
        "found {} public items",
        items.types.len()
    );
    let forbidden = [
        "admission",
        "admitted",
        "acknowledg",
        "grant",
        "gate",
        "dispatch",
        "authoriz",
        "capabilit",
        "permit",
        "executor",
        "verified",
        "acceptance",
    ];
    for (file, kind, name) in &items.types {
        let lower = name.to_lowercase();
        for token in forbidden {
            assert!(
                !lower.contains(token),
                "{file}: pub {kind} {name} contains {token:?}"
            );
        }
    }
    // Methods may not claim establishment or attach anything either.
    let method_contains = [
        "admission",
        "admitted",
        "acknowledg",
        "grant",
        "gate",
        "dispatch",
        "authoriz",
        "capabilit",
        "permit",
    ];
    let method_prefix = [
        "attach",
        "bind",
        "mark",
        "satisf",
        "establish",
        "set_",
        "with_",
    ];
    for (file, signature) in &items.functions {
        let name: String = signature
            .strip_prefix("pub fn ")
            .unwrap()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        for token in method_contains {
            assert!(!name.contains(token), "{file}: {signature}");
        }
        for token in method_prefix {
            assert!(!name.starts_with(token), "{file}: {signature}");
        }
    }
    // The crate root declares no public module: the surface is exactly the re-exports.
    assert!(items.types.iter().all(|(_, kind, _)| kind != "mod"));
    let lib = sources()
        .into_iter()
        .find(|(name, _)| name == "lib.rs")
        .unwrap()
        .1;
    assert!(!lib.contains("pub mod "));
}

fn struct_body(source: &str, name: &str) -> (String, String) {
    let start = source
        .find(&format!("pub struct {name} {{"))
        .unwrap_or_else(|| panic!("struct {name}"));
    let end = source[start..].find("\n}\n").unwrap() + start;
    // The attributes directly above the struct.
    let attributes = source[..start]
        .rsplit("\n\n")
        .next()
        .unwrap_or("")
        .lines()
        .filter(|line| line.trim_start().starts_with("#["))
        .collect::<Vec<_>>()
        .join("\n");
    (attributes, source[start..end].to_owned())
}

fn field_names(body: &str) -> Vec<String> {
    body.lines()
        .skip(1)
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("///") || line.starts_with('#') || line.is_empty() {
                return None;
            }
            let line = line.strip_prefix("pub ").unwrap_or(line);
            line.split(':').next().map(|name| name.trim().to_owned())
        })
        .collect()
}

#[test]
fn nothing_in_the_public_api_can_mark_separation_established_or_attach_a_principal_to_a_binding() {
    let derive = sources()
        .into_iter()
        .find(|(name, _)| name == "derive.rs")
        .unwrap()
        .1;

    // A component binding requirement has exactly `component` and `permissions`: no principal and
    // no status. It is `non_exhaustive`, so no other crate can construct one.
    let (attributes, body) = struct_body(&derive, "ComponentBindingRequirement");
    assert_eq!(field_names(&body), ["component", "permissions"]);
    assert!(attributes.contains("non_exhaustive"), "{attributes}");
    // A separation requirement has no status field and is likewise not constructible outside.
    let (attributes, body) = struct_body(&derive, "SeparationRequirement");
    assert_eq!(field_names(&body), ["condition", "acceptor", "vantages"]);
    assert!(attributes.contains("non_exhaustive"));

    // The candidate has no public field and no public constructor: only `derive` builds one.
    let (_, body) = struct_body(&derive, "OrganizationCandidate");
    assert!(
        !body.lines().skip(1).any(|line| line.contains("pub ")),
        "{body}"
    );

    // No public function takes, returns by value or builds a binding or separation requirement:
    // the only mentions are the read-only accessors on the candidate and the view.
    for (file, signature) in public_items().functions {
        for name in ["ComponentBindingRequirement", "SeparationRequirement"] {
            if signature.contains(name) {
                assert!(
                    signature.contains("&self) -> &[") && signature.contains(&format!("&[{name}]")),
                    "{file}: {signature}"
                );
            }
        }
    }
    // No `pub fn` is a mutator, and none returns a mutable reference: public values are
    // read-only through the API.
    for (file, signature) in public_items().functions {
        assert!(!signature.contains("&mut"), "{file}: {signature}");
    }
}
