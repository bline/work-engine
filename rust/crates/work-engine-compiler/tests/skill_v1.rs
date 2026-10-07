use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use work_engine_compiler::{ErrorCode, compile_skill_unverified, sha256_hex};

fn fixture(id: &str, name: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/v1/skills")
            .join(id)
            .join(name),
    )
    .unwrap()
}

fn corpus() -> Value {
    serde_json::from_slice(
        &fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1/cases.json"))
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn five_pinned_skills_match_legacy_oracle_bytes_and_ir() {
    for case in corpus()["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let structure = fixture(id, "structure.yaml");
        let interface = fixture(id, "interface.yaml");
        let expected_output = fixture(id, "expected.md");
        let expected_ir: Value = serde_json::from_slice(&fixture(id, "expected-ir.json")).unwrap();
        let compiled = compile_skill_unverified(&structure, &interface)
            .unwrap_or_else(|error| panic!("{id}: {error:?}"));
        assert_eq!(compiled.output, expected_output, "{id} Markdown");
        assert_eq!(
            sha256_hex(&compiled.output),
            case["output_sha256"],
            "{id} output hash"
        );
        assert_eq!(
            compiled.ir["runtime_requirements"]["verified_sources"], false,
            "{id}"
        );
        let mut neutral = compiled.ir;
        assert_eq!(neutral["compiler"], "work-engine.skill-compiler.rust-v1");
        neutral.as_object_mut().unwrap().remove("compiler");
        assert_eq!(neutral, expected_ir, "{id} producer-neutral IR");
        let second = compile_skill_unverified(&structure, &interface).unwrap();
        assert_eq!(second.output, expected_output, "{id} repeat");
    }
}

#[test]
fn malformed_schema_and_authority_refuse_partial_result() {
    let structure = String::from_utf8(fixture("p-rep", "structure.yaml")).unwrap();
    let interface = String::from_utf8(fixture("p-rep", "interface.yaml")).unwrap();
    let cases = [
        (
            format!("schema_version: 1\n{structure}"),
            interface.clone(),
            ErrorCode::InvalidYaml,
        ),
        (
            structure.replace("schema_version: 1", "schema_version: 2"),
            interface.clone(),
            ErrorCode::InvalidStructure,
        ),
        (
            structure.replace("canonical_authority", "unknown_authority"),
            interface.clone(),
            ErrorCode::InvalidStructure,
        ),
        (
            structure.clone(),
            interface.replace("authority.legacy-skill", "evidence.role-projection"),
            ErrorCode::InvalidInterface,
        ),
        (
            structure.clone(),
            interface.replace("continuity: ephemeral", "continuity: unlimited"),
            ErrorCode::InvalidInterface,
        ),
        (
            structure.clone(),
            interface.replacen(
                "    - capability.repository_evidence",
                "    - capability.missing",
                1,
            ),
            ErrorCode::InvalidInterface,
        ),
    ];
    for (source, interface, code) in cases {
        let error = compile_skill_unverified(source.as_bytes(), interface.as_bytes()).unwrap_err();
        assert_eq!(error.code, code, "{error:?}");
        assert!(error.path.is_some());
    }
}

#[test]
fn yaml_scalar_style_and_numeric_schema_match_pinned_oracle() {
    let structure = String::from_utf8(fixture("p-rep", "structure.yaml")).unwrap();
    let interface = fixture("p-rep", "interface.yaml");
    let baseline = compile_skill_unverified(structure.as_bytes(), &interface).unwrap();
    for version in ["1.0", "1e0"] {
        let changed = structure.replacen(
            "schema_version: 1",
            &format!("schema_version: {version}"),
            1,
        );
        let compiled = compile_skill_unverified(changed.as_bytes(), &interface).unwrap();
        assert_eq!(compiled.output, baseline.output, "{version}");
        assert_eq!(compiled.ir["schema_version"], 1, "{version}");
    }
    let plain_null = structure.replace("source_bindings:", "role_profile: Null\nsource_bindings:");
    let compiled = compile_skill_unverified(plain_null.as_bytes(), &interface).unwrap();
    assert!(compiled.ir.get("role_profile").is_none());
    let quoted_null = structure.replace(
        "source_bindings:",
        "role_profile: \"Null\"\nsource_bindings:",
    );
    assert_eq!(
        compile_skill_unverified(quoted_null.as_bytes(), &interface)
            .unwrap_err()
            .code,
        ErrorCode::InvalidStructure
    );
    for token in ["True", "FALSE", "Null"] {
        let changed =
            structure.replacen("schema_version: 1", &format!("schema_version: {token}"), 1);
        assert_eq!(
            compile_skill_unverified(changed.as_bytes(), &interface)
                .unwrap_err()
                .code,
            ErrorCode::InvalidStructure,
            "{token}"
        );
    }
    for token in ["nUlL", "TrUe", "FaLsE"] {
        let changed = structure.replace(
            "source_bindings:",
            &format!("role_profile: {token}\nsource_bindings:"),
        );
        assert_eq!(
            compile_skill_unverified(changed.as_bytes(), &interface)
                .unwrap_err()
                .code,
            ErrorCode::InvalidStructure,
            "{token}"
        );
    }
    for token in ["TrUe", "FaLsE"] {
        let changed = structure.replace("final_newline: true", &format!("final_newline: {token}"));
        assert_eq!(
            compile_skill_unverified(changed.as_bytes(), &interface)
                .unwrap_err()
                .code,
            ErrorCode::InvalidStructure,
            "{token}"
        );
    }
    for (token, expected_sha) in [
        (
            "true",
            "cf72505d1850e757f6cc6ef3331d0f66ee4c5bf51b52a527c27fe8b78c17a949",
        ),
        (
            "True",
            "cf72505d1850e757f6cc6ef3331d0f66ee4c5bf51b52a527c27fe8b78c17a949",
        ),
        (
            "TRUE",
            "cf72505d1850e757f6cc6ef3331d0f66ee4c5bf51b52a527c27fe8b78c17a949",
        ),
        (
            "false",
            "292b7d7677bd60b3b8586946c36bfef8bed0e1233434664a99082b0b1b861941",
        ),
        (
            "False",
            "292b7d7677bd60b3b8586946c36bfef8bed0e1233434664a99082b0b1b861941",
        ),
        (
            "FALSE",
            "292b7d7677bd60b3b8586946c36bfef8bed0e1233434664a99082b0b1b861941",
        ),
    ] {
        let changed = structure.replace("final_newline: true", &format!("final_newline: {token}"));
        let compiled = compile_skill_unverified(changed.as_bytes(), &interface).unwrap();
        assert_eq!(compiled.ir["output_sha256"], expected_sha, "{token}");
    }
}

#[test]
fn observation_limit_scalar_keys_match_node_property_conversion() {
    let structure = String::from_utf8(fixture("p-sup", "structure.yaml")).unwrap();
    let interface = fixture("p-sup", "interface.yaml");
    let admitted = [
        ("nUlL: bounded", "nUlL"),
        ("\"null\": bounded", "null"),
        ("TrUe: bounded", "TrUe"),
        (".inf: bounded", "Infinity"),
        ("-.inf: bounded", "-Infinity"),
        (".nan: bounded", "NaN"),
        ("1e0: bounded", "1"),
    ];
    for (entry, expected_key) in admitted {
        let changed = structure.replacen(
            "    observation_limits:\n",
            &format!("    observation_limits:\n      {entry}\n"),
            1,
        );
        let compiled = compile_skill_unverified(changed.as_bytes(), &interface).unwrap();
        assert_eq!(
            compiled.ir["role_profile"]["relations"]["observation_limits"][expected_key], "bounded",
            "{entry}"
        );
    }
    for entry in ["null: bounded", "~: bounded"] {
        let changed = structure.replacen(
            "    observation_limits:\n",
            &format!("    observation_limits:\n      {entry}\n"),
            1,
        );
        assert_eq!(
            compile_skill_unverified(changed.as_bytes(), &interface)
                .unwrap_err()
                .code,
            ErrorCode::InvalidStructure,
            "{entry}"
        );
    }
    let invalid_value = structure.replacen(
        "artifact.plan: concise plan and certificate consequences, not raw exploration",
        "artifact.plan: .inf",
        1,
    );
    assert_eq!(
        compile_skill_unverified(invalid_value.as_bytes(), &interface)
            .unwrap_err()
            .code,
        ErrorCode::InvalidStructure
    );
}

#[test]
fn collection_mapping_keys_are_rejected_by_accepted_c1_profile() {
    let structure = String::from_utf8(fixture("p-sup", "structure.yaml")).unwrap();
    let interface = fixture("p-sup", "interface.yaml");
    // The pinned JavaScript oracle admits these as observation-limit IDs
    // "[ a, b ]" and "{ a: b }". C1 deliberately excludes collection keys.
    for entry in ["[a,b]: bounded", "{a: b}: bounded"] {
        let changed = structure.replacen(
            "    observation_limits:\n",
            &format!("    observation_limits:\n      {entry}\n"),
            1,
        );
        assert_eq!(
            compile_skill_unverified(changed.as_bytes(), &interface)
                .unwrap_err()
                .code,
            ErrorCode::InvalidYaml,
            "{entry}"
        );
    }
}

#[test]
fn yaml_tags_anchors_documents_and_scalar_keys_are_characterized() {
    let structure = String::from_utf8(fixture("p-rep", "structure.yaml")).unwrap();
    let interface = fixture("p-rep", "interface.yaml");
    let baseline = compile_skill_unverified(structure.as_bytes(), &interface).unwrap();
    for (case, changed) in [
        structure.replace("skill_id: repo-search", "skill_id: &unused repo-search"),
        structure.replacen("schema_version: 1", "schema_version: !!int 1", 1),
        structure.replace("skill_id: repo-search", "skill_id: !!str repo-search"),
        structure.replace("skill_id: repo-search", "skill_id: !custom repo-search"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            compile_skill_unverified(changed.as_bytes(), &interface)
                .unwrap_or_else(|error| panic!("tag/anchor case {case}: {error:?}"))
                .output,
            baseline.output
        );
    }
    for (changed, code) in [
        (
            format!("{structure}\n---\nextra: true\n"),
            ErrorCode::InvalidYaml,
        ),
        (
            structure
                .replace("skill_id: repo-search", "skill_id: &used repo-search")
                .replace("name: repo-search", "name: *used"),
            ErrorCode::InvalidYaml,
        ),
        (
            structure.replacen("schema_version: 1", "1: ignored\nschema_version: 1", 1),
            ErrorCode::InvalidStructure,
        ),
    ] {
        assert_eq!(
            compile_skill_unverified(changed.as_bytes(), &interface)
                .unwrap_err()
                .code,
            code
        );
    }
}

#[test]
fn yaml_bom_changes_raw_identity_without_changing_compilation() {
    let structure = fixture("p-sup", "structure.yaml");
    let interface = fixture("p-sup", "interface.yaml");
    let baseline = compile_skill_unverified(&structure, &interface).unwrap();
    for (structure_source, interface_source, expected_structure_hash, expected_interface_hash) in [
        (
            [b"\xef\xbb\xbf".as_slice(), structure.as_slice()].concat(),
            interface.clone(),
            "0e1f2062b13e2581e4a0e4072ec2d17365a662d0b50f25690f5a509f97595f18",
            "62e2261c20e1b605ca281afbc2c5b5811e2b2dfb929de7c4e8ccfe97515c1fcc",
        ),
        (
            structure.clone(),
            [b"\xef\xbb\xbf".as_slice(), interface.as_slice()].concat(),
            "7cf2bd8c33a05b8c49a1567118352aee1d8c05d8b83e584bd1d6f84d137e850b",
            "9869c5eee1b45bdb9e66854e4f2ca7e7616470f9b1cef1c56aecb75f4f6abbe0",
        ),
    ] {
        let mut compiled = compile_skill_unverified(&structure_source, &interface_source).unwrap();
        assert_eq!(compiled.output, baseline.output);
        assert_eq!(
            compiled.ir["input_sha256"]["structure"],
            expected_structure_hash
        );
        assert_eq!(
            compiled.ir["input_sha256"]["interface"],
            expected_interface_hash
        );
        assert_eq!(compiled.ir["output_sha256"], baseline.ir["output_sha256"]);
        compiled.ir["input_sha256"] = baseline.ir["input_sha256"].clone();
        assert_eq!(compiled.ir, baseline.ir);
    }
}

#[test]
fn yaml_merge_inputs_refuse_but_literal_shift_keys_remain_scalar() {
    let structure = String::from_utf8(fixture("p-sup", "structure.yaml")).unwrap();
    let interface = fixture("p-sup", "interface.yaml");
    let baseline = compile_skill_unverified(structure.as_bytes(), &interface).unwrap();
    let mut literal_irs = Vec::new();
    for (entry, expected_input_hash) in [
        (
            "'<<': bounded",
            "0c78a5b8404b2d884bd741d9eb420793de04bce4b4d97d2e66f8d6bf93e0ea9d",
        ),
        (
            "<<: bounded",
            "6de49c97b8dd6505eff17011fd0bb10f1e413ebec6d57e5294bef3fd1fbf58ec",
        ),
    ] {
        let changed = structure.replacen(
            "    observation_limits:\n",
            &format!("    observation_limits:\n      {entry}\n"),
            1,
        );
        let compiled = compile_skill_unverified(changed.as_bytes(), &interface).unwrap();
        assert_eq!(compiled.output, baseline.output, "{entry}");
        assert_eq!(
            compiled.ir["input_sha256"]["structure"],
            expected_input_hash
        );
        assert_eq!(
            compiled.ir["role_profile"]["relations"]["observation_limits"]["<<"],
            "bounded"
        );
        literal_irs.push(compiled.ir);
    }
    literal_irs[1]["input_sha256"] = literal_irs[0]["input_sha256"].clone();
    assert_eq!(literal_irs[0], literal_irs[1]);

    for (entry, expected_code) in [
        ("<<: *defaults", ErrorCode::InvalidYaml),
        ("<<: {a: b}", ErrorCode::InvalidStructure),
    ] {
        let changed = structure.replacen(
            "    observation_limits:\n",
            &format!("    observation_limits:\n      {entry}\n"),
            1,
        );
        assert_eq!(
            compile_skill_unverified(changed.as_bytes(), &interface)
                .unwrap_err()
                .code,
            expected_code,
            "{entry}"
        );
    }
}

#[test]
fn runtime_lists_sort_by_javascript_utf16_units() {
    let structure = fixture("p-rep", "structure.yaml");
    let interface = String::from_utf8(fixture("p-rep", "interface.yaml")).unwrap();
    let changed = interface
        .replace(
            "required_capabilities:\n",
            "required_capabilities:\n    - capability.😀\n    - capability.\n",
        )
        .replace(
            "capability_ceiling:\n",
            "capability_ceiling:\n    - capability.😀\n    - capability.\n",
        );
    let compiled = compile_skill_unverified(&structure, changed.as_bytes()).unwrap();
    assert_eq!(
        compiled.ir["runtime_requirements"]["required_capabilities"],
        serde_json::json!([
            "capability.direct_source_observation",
            "capability.repository_evidence",
            "capability.😀",
            "capability."
        ])
    );
    assert_eq!(
        compiled.ir["runtime_requirements"]["sha256"],
        "ca5b7a465617631ae6e97a53bd267f45707728a812859d014a9813b6e902ee15"
    );
}

#[test]
fn raw_input_changes_do_not_rewrite_semantic_output() {
    let structure = fixture("p-rep", "structure.yaml");
    let interface = fixture("p-rep", "interface.yaml");
    let mut commented = b"# compatible comment\n".to_vec();
    commented.extend_from_slice(&structure);
    let baseline = compile_skill_unverified(&structure, &interface).unwrap();
    let changed = compile_skill_unverified(&commented, &interface).unwrap();
    assert_eq!(baseline.output, changed.output);
    assert_eq!(
        baseline.ir["runtime_requirements"],
        changed.ir["runtime_requirements"]
    );
    assert_ne!(
        baseline.ir["input_sha256"]["structure"],
        changed.ir["input_sha256"]["structure"]
    );
}

#[test]
fn aliases_and_unsupported_request_values_do_not_enter_pure_compiler() {
    let structure = fixture("p-rep", "structure.yaml");
    let interface = fixture("p-rep", "interface.yaml");
    let aliased = String::from_utf8(structure.clone())
        .unwrap()
        .replace("skill_id: repo-search", "skill_id: *unauthorized");
    assert_eq!(
        compile_skill_unverified(aliased.as_bytes(), &interface)
            .unwrap_err()
            .code,
        ErrorCode::InvalidYaml
    );
    let invalid_utf8 = [0xff, 0xfe, 0xfd];
    assert_eq!(
        compile_skill_unverified(&invalid_utf8, &interface)
            .unwrap_err()
            .code,
        ErrorCode::InvalidYaml
    );
    let malformed = String::from_utf8(structure).unwrap().replace(
        "source_span: {start_byte:",
        "source_span: {start_byte: -1 #",
    );
    assert!(compile_skill_unverified(malformed.as_bytes(), &interface).is_err());
}
