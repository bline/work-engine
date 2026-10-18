//! Pure extension attachment compilation and verification. Host admission is a separate input;
//! an authored bundle cannot supply or widen it.
use crate::{compile_skill_unverified, sha256_hex};
use serde_json::{Map, Value, json};
use std::collections::HashSet;

#[derive(Debug)]
pub struct ExtensionError {
    pub code: &'static str,
    pub message: &'static str,
}
impl ExtensionError {
    fn invalid(message: &'static str) -> Self {
        Self {
            code: "invalid_extension",
            message,
        }
    }
    fn mismatch(message: &'static str) -> Self {
        Self {
            code: "source_mismatch",
            message,
        }
    }
    fn limit(message: &'static str) -> Self {
        Self {
            code: "resource_limit",
            message,
        }
    }
    fn integrity(message: &'static str) -> Self {
        Self {
            code: "attachment_mismatch",
            message,
        }
    }
}

pub const EXTENSION_ATTACHMENT_PROFILE: &str = "work-engine.run-extension.rust-v1";
// The explicit byte arrays and their serialized envelope remain bounded.
const MAX_COMPILED_OUTPUT_BYTES: usize = 512 * 1024;

/// C4 canonical JSON retains serde_json's exact integer values. The older
/// compiler codec intentionally projects all numbers through binary64 for its
/// JavaScript contract, so it must not identify C4 attachments.
pub fn extension_canonical_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => serde_json::to_string(v).expect("string serialization cannot fail"),
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(extension_canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Object(values) => {
            let mut keys: Vec<_> = values.keys().collect();
            keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            let parts = keys
                .into_iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("key serialization cannot fail"),
                        extension_canonical_json(&values[key])
                    )
                })
                .collect::<Vec<_>>();
            format!("{{{}}}", parts.join(","))
        }
    }
}

pub struct ExtensionSkillSource {
    pub structure: Vec<u8>,
    pub interface: Vec<u8>,
}

pub struct CompiledExtension {
    pub descriptor: Value,
    pub canonical_json: String,
    pub sha256: String,
}
impl CompiledExtension {
    pub fn into_attachment(self) -> Value {
        json!({"schema_version":2,"profile":EXTENSION_ATTACHMENT_PROFILE,
            "descriptor":self.descriptor,"sha256":self.sha256})
    }
}

fn object<'a>(
    value: &'a Value,
    message: &'static str,
) -> Result<&'a Map<String, Value>, ExtensionError> {
    value
        .as_object()
        .ok_or_else(|| ExtensionError::invalid(message))
}
fn string<'a>(value: Option<&'a Value>, message: &'static str) -> Result<&'a str, ExtensionError> {
    value
        .and_then(Value::as_str)
        .ok_or_else(|| ExtensionError::invalid(message))
}
fn array<'a>(
    value: Option<&'a Value>,
    message: &'static str,
) -> Result<&'a [Value], ExtensionError> {
    value
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| ExtensionError::invalid(message))
}
fn list_or_empty<'a>(
    map: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a [Value], ExtensionError> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(&[]),
        value => array(value, "extension list is invalid"),
    }
}
fn texts(values: &[Value]) -> Result<Vec<String>, ExtensionError> {
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| ExtensionError::invalid("extension admission list requires text"))
        })
        .collect()
}
fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}
fn bytes_array(bytes: &[u8]) -> Value {
    Value::Array(bytes.iter().map(|byte| json!(byte)).collect())
}

struct Admission<'a> {
    bundle: &'a Map<String, Value>,
    namespace: String,
    capabilities: Vec<String>,
    effects: Vec<String>,
    adapters: Vec<String>,
    providers: Vec<String>,
}

fn admit<'a>(
    bundle: &'a Value,
    host: &Value,
    schema_version: u64,
) -> Result<Admission<'a>, ExtensionError> {
    let b = object(bundle, "extension bundle identity is invalid")?;
    let h = object(host, "host admission is invalid")?;
    let revision = string(
        h.get("repository_revision"),
        "host repository revision is required",
    )?;
    let source_revision = string(
        b.get("source_revision"),
        "extension bundle identity is invalid",
    )?;
    if b.get("schema_version") != Some(&json!(schema_version))
        || b.get("bundle_id").and_then(Value::as_str).is_none()
        || !(40..=64).contains(&source_revision.len())
        || !source_revision
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(ExtensionError::invalid(
            "extension bundle identity is invalid",
        ));
    }
    if source_revision != revision {
        return Err(ExtensionError::mismatch(
            "extension bundle source revision is not the admitted repository revision",
        ));
    }
    let run = object(
        b.get("run").ok_or_else(|| {
            ExtensionError::invalid(
                "extension run must use sealed networking and no projected credentials",
            )
        })?,
        "extension run must use sealed networking and no projected credentials",
    )?;
    if run.get("network") != Some(&json!("sealed"))
        || run.get("credentials") != Some(&json!("none"))
    {
        return Err(ExtensionError::invalid(
            "extension run must use sealed networking and no projected credentials",
        ));
    }
    for field in [
        "repository_subject",
        "checkout",
        "artifact_root",
        "scratch_root",
        "retention",
        "namespace",
    ] {
        if string(run.get(field), "extension run field is required")?.is_empty() {
            return Err(ExtensionError::invalid("extension run field is required"));
        }
    }
    let namespace = run["namespace"].as_str().expect("checked");
    for field in ["artifact_root", "scratch_root"] {
        if !run[field]
            .as_str()
            .expect("checked")
            .starts_with(&format!("{namespace}/"))
        {
            return Err(ExtensionError::invalid(
                "extension roots must be namespaced",
            ));
        }
    }
    if !run["repository_subject"]
        .as_str()
        .expect("checked")
        .contains(revision)
        || !run["checkout"]
            .as_str()
            .expect("checked")
            .contains(revision)
    {
        return Err(ExtensionError::mismatch(
            "extension run repository binding differs from admitted revision",
        ));
    }
    let capabilities = sorted_unique(texts(list_or_empty(b, "capabilities")?)?);
    let effects = sorted_unique(texts(list_or_empty(b, "effects")?)?);
    let adapters = sorted_unique(texts(list_or_empty(b, "adapters")?)?);
    let providers = sorted_unique(texts(list_or_empty(b, "providers")?)?);
    let admitted_caps: HashSet<_> = texts(array(
        h.get("allowed_capabilities"),
        "host capability admission is invalid",
    )?)?
    .into_iter()
    .collect();
    let admitted_adapters: HashSet<_> = texts(array(
        h.get("allowed_adapters"),
        "host adapter admission is invalid",
    )?)?
    .into_iter()
    .collect();
    let admitted_providers: HashSet<_> = texts(array(
        h.get("allowed_providers"),
        "host provider admission is invalid",
    )?)?
    .into_iter()
    .collect();
    if capabilities.iter().any(|v| !admitted_caps.contains(v)) {
        return Err(ExtensionError::invalid(
            "extension capability exceeds host admission",
        ));
    }
    if effects.iter().any(|v| {
        [
            "filesystem.write",
            "network.install",
            "publication",
            "claims",
            "review",
            "human",
            "production",
        ]
        .contains(&v.as_str())
    }) {
        return Err(ExtensionError::invalid(
            "extension requests prohibited authority",
        ));
    }
    if !list_or_empty(b, "authority_requests")?.is_empty() {
        return Err(ExtensionError::invalid(
            "extension cannot acquire production authority",
        ));
    }
    if adapters.iter().any(|v| !admitted_adapters.contains(v)) {
        return Err(ExtensionError::invalid(
            "extension adapter is not host mediated",
        ));
    }
    if providers.iter().any(|v| !admitted_providers.contains(v)) {
        return Err(ExtensionError::invalid(
            "extension provider requirement is unavailable",
        ));
    }
    Ok(Admission {
        bundle: b,
        namespace: namespace.to_owned(),
        capabilities,
        effects,
        adapters,
        providers,
    })
}

pub fn preflight_extension_bundle(bundle: &Value, host: &Value) -> Result<(), ExtensionError> {
    admit(bundle, host, 1).map(|_| ())
}

fn dependencies_and_registry(
    bundle: &Map<String, Value>,
    namespace: &str,
    capabilities: &[String],
    adapters: &[String],
) -> Result<(Vec<Value>, Vec<Value>), ExtensionError> {
    let dependencies = list_or_empty(bundle, "dependencies")?.to_vec();
    for dependency in &dependencies {
        if dependency.get("mode") != Some(&json!("sealed_cache")) {
            return Err(ExtensionError::invalid(
                "extension dependency requires network",
            ));
        }
    }
    let registry = list_or_empty(bundle, "registry")?.to_vec();
    let mut names = HashSet::new();
    for entry in &registry {
        let capability = string(
            entry.get("capability"),
            "extension registry entry is invalid",
        )?;
        let adapter = string(entry.get("adapter"), "extension registry entry is invalid")?;
        if !capabilities.iter().any(|v| v == capability) || !adapters.iter().any(|v| v == adapter) {
            return Err(ExtensionError::invalid(
                "extension registry entry is not admitted",
            ));
        }
        let name = string(entry.get("name"), "extension registry entry is invalid")?;
        if !names.insert(format!("{namespace}:{name}")) {
            return Err(ExtensionError::invalid("extension registry conflict"));
        }
    }
    Ok((dependencies, registry))
}

pub fn compile_extension_bundle(
    bundle: &Value,
    host: &Value,
    sources: &[ExtensionSkillSource],
) -> Result<CompiledExtension, ExtensionError> {
    let admission = admit(bundle, host, 1)?;
    let b = admission.bundle;
    let namespace = admission.namespace;
    let capabilities = admission.capabilities;
    let effects = admission.effects;
    let adapters = admission.adapters;
    let providers = admission.providers;
    let skills = list_or_empty(b, "skills")?;
    if skills.len() != sources.len() {
        return Err(ExtensionError::invalid(
            "extension skill sources differ from bundle",
        ));
    }
    let mut compiled_skills = Vec::new();
    let mut output_bytes = 0usize;
    for (skill, source) in skills.iter().zip(sources) {
        let skill = object(skill, "extension skill entry is invalid")?;
        std::str::from_utf8(&source.structure)
            .map_err(|_| ExtensionError::invalid("extension skill source must be UTF-8"))?;
        std::str::from_utf8(&source.interface)
            .map_err(|_| ExtensionError::invalid("extension skill source must be UTF-8"))?;
        if sha256_hex(&source.structure)
            != string(
                skill.get("structure_sha256"),
                "extension skill digest is required",
            )?
            || sha256_hex(&source.interface)
                != string(
                    skill.get("interface_sha256"),
                    "extension skill digest is required",
                )?
        {
            return Err(ExtensionError::mismatch(
                "extension skill compiler input digest mismatch",
            ));
        }
        let compiled = compile_skill_unverified(&source.structure, &source.interface)
            .map_err(|_| ExtensionError::invalid("extension skill compilation failed"))?;
        output_bytes = output_bytes.saturating_add(compiled.output.len());
        if output_bytes > MAX_COMPILED_OUTPUT_BYTES {
            return Err(ExtensionError::limit(
                "extension compiled output exceeds limit",
            ));
        }
        let ir = &compiled.ir;
        compiled_skills.push(json!({"skill_id":ir["skill_id"],"input_sha256":ir["input_sha256"],"output_sha256":ir["output_sha256"],"runtime_requirements":ir["runtime_requirements"],"output_bytes":bytes_array(&compiled.output)}));
    }
    if compiled_skills.is_empty() {
        return Err(ExtensionError::invalid(
            "extension bundle requires compiled skills",
        ));
    }
    let (dependencies, registry) =
        dependencies_and_registry(b, &namespace, &capabilities, &adapters)?;
    let descriptor = json!({
        "schema_version":2,"bundle_id":b["bundle_id"],"source_revision":b["source_revision"],
        "run":b["run"],"precedence":b.get("precedence").filter(|v| !v.is_null()).cloned().unwrap_or(json!("below_core")),
        "capabilities":capabilities,"effects":effects,"adapters":adapters,"providers":providers,
        "dependencies":dependencies,"registry":registry,"compiled_skills":compiled_skills
    });
    let canonical_json = extension_canonical_json(&descriptor);
    let sha256 = sha256_hex(canonical_json.as_bytes());
    Ok(CompiledExtension {
        descriptor,
        canonical_json,
        sha256,
    })
}

/// Recheck a serialized attachment against independent host admission. A matching
/// digest establishes byte integrity of this value, not compiler provenance.
pub fn verify_extension_attachment<'a>(
    attachment: &'a Value,
    host: &Value,
) -> Result<&'a Value, ExtensionError> {
    let wrapper = object(attachment, "extension attachment must be an object")?;
    let wrapper_fields = ["schema_version", "profile", "descriptor", "sha256"];
    if wrapper.len() != wrapper_fields.len()
        || wrapper_fields.iter().any(|key| !wrapper.contains_key(*key))
        || wrapper["schema_version"] != json!(2)
        || wrapper["profile"] != json!(EXTENSION_ATTACHMENT_PROFILE)
    {
        return Err(ExtensionError::invalid(
            "unsupported extension attachment profile",
        ));
    }
    let descriptor = &wrapper["descriptor"];
    let b = object(descriptor, "extension descriptor must be an object")?;
    let descriptor_fields = [
        "schema_version",
        "bundle_id",
        "source_revision",
        "run",
        "precedence",
        "capabilities",
        "effects",
        "adapters",
        "providers",
        "dependencies",
        "registry",
        "compiled_skills",
    ];
    if b.len() != descriptor_fields.len()
        || descriptor_fields.iter().any(|key| !b.contains_key(*key))
    {
        return Err(ExtensionError::invalid(
            "extension descriptor fields differ from profile",
        ));
    }
    let recorded = string(
        wrapper.get("sha256"),
        "extension attachment digest is required",
    )?;
    if recorded.len() != 64
        || !recorded
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        || sha256_hex(extension_canonical_json(descriptor)) != recorded
    {
        return Err(ExtensionError::integrity(
            "extension attachment digest mismatch",
        ));
    }
    let admission = admit(descriptor, host, 2)?;
    for (key, expected) in [
        ("capabilities", &admission.capabilities),
        ("effects", &admission.effects),
        ("adapters", &admission.adapters),
        ("providers", &admission.providers),
    ] {
        if b[key] != json!(expected) {
            return Err(ExtensionError::invalid(
                "extension admission lists are not canonical",
            ));
        }
    }
    dependencies_and_registry(
        b,
        &admission.namespace,
        &admission.capabilities,
        &admission.adapters,
    )?;
    let skills = array(
        b.get("compiled_skills"),
        "extension compiled skills are invalid",
    )?;
    if skills.is_empty() {
        return Err(ExtensionError::invalid(
            "extension bundle requires compiled skills",
        ));
    }
    let mut total = 0usize;
    for skill in skills {
        let skill = object(skill, "extension compiled skill is invalid")?;
        let fields = [
            "skill_id",
            "input_sha256",
            "output_sha256",
            "runtime_requirements",
            "output_bytes",
        ];
        if skill.len() != fields.len()
            || fields.iter().any(|key| !skill.contains_key(*key))
            || string(
                skill.get("skill_id"),
                "extension compiled skill ID is invalid",
            )?
            .is_empty()
            || !skill["input_sha256"].is_object()
            || skill["runtime_requirements"]["verified_sources"] != json!(false)
        {
            return Err(ExtensionError::invalid(
                "extension compiled skill is invalid",
            ));
        }
        let output = array(
            skill.get("output_bytes"),
            "extension output bytes are invalid",
        )?;
        total = total.saturating_add(output.len());
        if total > MAX_COMPILED_OUTPUT_BYTES {
            return Err(ExtensionError::limit(
                "extension compiled output exceeds limit",
            ));
        }
        let bytes: Vec<u8> = output
            .iter()
            .map(|value| {
                value
                    .as_u64()
                    .and_then(|v| u8::try_from(v).ok())
                    .ok_or_else(|| ExtensionError::invalid("extension output bytes are invalid"))
            })
            .collect::<Result<_, _>>()?;
        if sha256_hex(&bytes)
            != string(
                skill.get("output_sha256"),
                "extension output digest is required",
            )?
        {
            return Err(ExtensionError::integrity(
                "extension compiled output digest mismatch",
            ));
        }
    }
    Ok(descriptor)
}
