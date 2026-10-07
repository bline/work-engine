use crate::{CompilerError, ErrorCode, canonical_json, sha256_hex};
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use yaml_rust::{
    Yaml,
    parser::{Event, EventReceiver, Parser, Tag},
    scanner::TScalarStyle,
};

const STATUS: &str = "experimental_non_authoritative";
const PRODUCER: &str = "work-engine.skill-compiler.rust-v1";
const RELATIONS: &[&str] = &[
    "bound_by",
    "may_invoke",
    "may_observe",
    "may_mutate",
    "owns",
    "consumes",
    "emits",
    "mediated_transitions",
    "forbidden_from",
];

#[derive(Debug, Clone)]
pub struct CompiledSkill {
    pub ir: Value,
    pub output: Vec<u8>,
}

fn error(code: ErrorCode, path: &str, message: impl Into<String>) -> CompilerError {
    CompilerError::new(code, Some(path.to_owned()), message)
}

fn object<'a>(
    value: &'a Value,
    code: ErrorCode,
    path: &str,
    allowed: &[&str],
) -> Result<&'a Map<String, Value>, CompilerError> {
    let map = value
        .as_object()
        .ok_or_else(|| error(code, path, "must be an object"))?;
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(error(
                code,
                &format!("{path}/{key}"),
                format!("unknown field {key}"),
            ));
        }
    }
    Ok(map)
}

fn field<'a>(map: &'a Map<String, Value>, key: &str) -> &'a Value {
    map.get(key).unwrap_or(&Value::Null)
}

fn text(value: &Value, code: ErrorCode, path: &str) -> Result<String, CompilerError> {
    match value.as_str() {
        Some(value) if !value.is_empty() => Ok(value.to_owned()),
        _ => Err(error(code, path, "must be nonempty text")),
    }
}

fn digest(value: &Value, code: ErrorCode, path: &str) -> Result<String, CompilerError> {
    let value = text(value, code, path)?;
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(value)
    } else {
        Err(error(code, path, "must be a lowercase SHA-256 digest"))
    }
}

fn integer(value: &Value, code: ErrorCode, path: &str) -> Result<Value, CompilerError> {
    let Some(number) = value.as_number().and_then(|number| number.as_f64()) else {
        return Err(error(code, path, "must be a nonnegative integer"));
    };
    if number.is_finite() && number >= 0.0 && number.fract() == 0.0 {
        if number < 18_446_744_073_709_551_616.0 {
            Ok(json!(number as u64))
        } else {
            Ok(json!(number))
        }
    } else {
        Err(error(code, path, "must be a nonnegative integer"))
    }
}

fn array<'a>(value: &'a Value, code: ErrorCode, path: &str) -> Result<&'a [Value], CompilerError> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| error(code, path, "must be an array"))
}

fn unique_texts(
    value: &Value,
    code: ErrorCode,
    path: &str,
    sorted: bool,
) -> Result<Vec<String>, CompilerError> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for (index, item) in array(value, code, path)?.iter().enumerate() {
        let item = text(item, code, &format!("{path}/{index}"))?;
        if !seen.insert(item.clone()) {
            return Err(error(code, path, "contains duplicates"));
        }
        result.push(item);
    }
    if sorted {
        result.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    }
    Ok(result)
}

pub(crate) fn parse_yaml(bytes: &[u8], label: &str) -> Result<Value, CompilerError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| error(ErrorCode::InvalidYaml, label, "source must be UTF-8"))?;
    let parse_source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let mut collected = YamlEvents::default();
    Parser::new(parse_source.chars())
        .load(&mut collected, true)
        .map_err(|issue| {
            error(
                ErrorCode::InvalidYaml,
                label,
                format!("invalid YAML: {issue}"),
            )
        })?;
    let events = collected.0;
    if events
        .iter()
        .filter(|event| matches!(event, Event::DocumentStart))
        .count()
        != 1
    {
        return Err(error(
            ErrorCode::InvalidYaml,
            label,
            "one YAML document is required",
        ));
    }
    let mut index = events
        .iter()
        .position(|event| matches!(event, Event::DocumentStart))
        .unwrap()
        + 1;
    let value = parse_yaml_node(&events, &mut index, label)?;
    if !matches!(events.get(index), Some(Event::DocumentEnd)) {
        return Err(error(
            ErrorCode::InvalidYaml,
            label,
            "invalid YAML document ending",
        ));
    }
    Ok(value)
}

#[derive(Default)]
struct YamlEvents(Vec<Event>);

impl EventReceiver for YamlEvents {
    fn on_event(&mut self, event: Event) {
        self.0.push(event);
    }
}

fn parse_yaml_node(
    events: &[Event],
    index: &mut usize,
    path: &str,
) -> Result<Value, CompilerError> {
    let event = events
        .get(*index)
        .ok_or_else(|| error(ErrorCode::InvalidYaml, path, "incomplete YAML node"))?;
    *index += 1;
    match event {
        Event::Scalar(value, style, _, tag) => parse_yaml_scalar(value, *style, tag.as_ref(), path),
        Event::Alias(_) => Err(error(
            ErrorCode::InvalidYaml,
            path,
            "YAML aliases are unsupported",
        )),
        Event::SequenceStart(..) => {
            let mut result = Vec::new();
            while !matches!(events.get(*index), Some(Event::SequenceEnd)) {
                if *index >= events.len() {
                    return Err(error(
                        ErrorCode::InvalidYaml,
                        path,
                        "incomplete YAML sequence",
                    ));
                }
                result.push(parse_yaml_node(
                    events,
                    index,
                    &format!("{path}/{}", result.len()),
                )?);
            }
            *index += 1;
            Ok(Value::Array(result))
        }
        Event::MappingStart(..) => {
            let mut result = Map::new();
            while !matches!(events.get(*index), Some(Event::MappingEnd)) {
                if *index >= events.len() {
                    return Err(error(
                        ErrorCode::InvalidYaml,
                        path,
                        "incomplete YAML mapping",
                    ));
                }
                let key = parse_yaml_key(events, index, path)?;
                if result.contains_key(&key) {
                    return Err(error(
                        ErrorCode::InvalidYaml,
                        path,
                        format!("duplicate YAML key {key}"),
                    ));
                }
                let value = parse_yaml_node(events, index, &format!("{path}/{key}"))?;
                result.insert(key, value);
            }
            *index += 1;
            Ok(Value::Object(result))
        }
        _ => Err(error(ErrorCode::InvalidYaml, path, "unexpected YAML event")),
    }
}

fn parse_yaml_key(
    events: &[Event],
    index: &mut usize,
    path: &str,
) -> Result<String, CompilerError> {
    let event = events
        .get(*index)
        .ok_or_else(|| error(ErrorCode::InvalidYaml, path, "incomplete YAML key"))?;
    let Event::Scalar(value, style, _, tag) = event else {
        return Err(error(
            ErrorCode::InvalidYaml,
            path,
            "collection-valued YAML key needs an accepted syntax-preserving codec",
        ));
    };
    *index += 1;
    let is_numeric_tag = matches!(tag, Some(Tag { handle, suffix }) if standard_yaml_tag(handle) && suffix == "float");
    if (*style == TScalarStyle::Plain || is_numeric_tag)
        && !matches!(tag, Some(Tag { handle, .. }) if !standard_yaml_tag(handle))
        && !matches!(tag, Some(Tag { handle, suffix }) if standard_yaml_tag(handle) && suffix != "float")
        && let Some(key) = yaml_nonfinite(value)
    {
        return Ok(key.to_owned());
    }
    let value = parse_yaml_scalar(value, *style, tag.as_ref(), path)?;
    match value {
        Value::Null => Ok(String::new()),
        Value::String(value) => Ok(value),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(canonical_json(&Value::Number(value))),
        _ => Err(error(
            ErrorCode::InvalidYaml,
            path,
            "unsupported scalar YAML key",
        )),
    }
}

fn standard_yaml_tag(handle: &str) -> bool {
    matches!(handle, "!!" | "tag:yaml.org,2002:")
}

fn parse_yaml_scalar(
    value: &str,
    style: TScalarStyle,
    tag: Option<&Tag>,
    path: &str,
) -> Result<Value, CompilerError> {
    if matches!(tag, Some(Tag { handle, .. }) if !standard_yaml_tag(handle)) {
        return Ok(Value::String(value.to_owned()));
    }
    let explicit = match tag {
        Some(Tag { handle, suffix }) if standard_yaml_tag(handle) => Some(suffix.as_str()),
        _ => None,
    };
    if explicit == Some("str") {
        return Ok(Value::String(value.to_owned()));
    }
    if explicit == Some("null") {
        return if yaml_null(value) {
            Ok(Value::Null)
        } else {
            Ok(Value::String(value.to_owned()))
        };
    }
    if explicit == Some("bool") {
        return if yaml_true(value) {
            Ok(Value::Bool(true))
        } else if yaml_false(value) {
            Ok(Value::Bool(false))
        } else {
            Ok(Value::String(value.to_owned()))
        };
    }
    if explicit == Some("int") {
        return match Yaml::from_str(value) {
            node @ Yaml::Integer(_) => yaml_number(node, path),
            _ => Ok(Value::String(value.to_owned())),
        };
    }
    if explicit == Some("float") {
        if yaml_nonfinite(value).is_some() {
            let code = if path.starts_with("/interface") {
                ErrorCode::InvalidInterface
            } else {
                ErrorCode::InvalidStructure
            };
            return Err(error(
                code,
                path,
                "non-finite YAML scalar is invalid at this value position",
            ));
        }
        return if value.chars().any(|ch| matches!(ch, '.' | 'e' | 'E')) {
            yaml_number(Yaml::Real(value.to_owned()), path)
        } else {
            Ok(Value::String(value.to_owned()))
        };
    }
    if explicit.is_some() {
        return Ok(Value::String(value.to_owned()));
    }
    if style != TScalarStyle::Plain {
        return Ok(Value::String(value.to_owned()));
    }
    if yaml_null(value) {
        return Ok(Value::Null);
    }
    if yaml_true(value) {
        return Ok(Value::Bool(true));
    }
    if yaml_false(value) {
        return Ok(Value::Bool(false));
    }
    if yaml_nonfinite(value).is_some() {
        let code = if path.starts_with("/interface") {
            ErrorCode::InvalidInterface
        } else {
            ErrorCode::InvalidStructure
        };
        return Err(error(
            code,
            path,
            "non-finite YAML scalar is invalid at this value position",
        ));
    }
    yaml_number(Yaml::from_str(value), path)
}

fn yaml_null(value: &str) -> bool {
    matches!(value, "" | "~" | "null" | "Null" | "NULL")
}
fn yaml_true(value: &str) -> bool {
    matches!(value, "true" | "True" | "TRUE")
}
fn yaml_false(value: &str) -> bool {
    matches!(value, "false" | "False" | "FALSE")
}

fn yaml_nonfinite(value: &str) -> Option<&'static str> {
    match value {
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => Some("Infinity"),
        "-.inf" | "-.Inf" | "-.INF" => Some("-Infinity"),
        ".nan" | ".NaN" | ".NAN" => Some("NaN"),
        _ => None,
    }
}

fn yaml_number(node: Yaml, path: &str) -> Result<Value, CompilerError> {
    match node {
        Yaml::Integer(value) => {
            if value.unsigned_abs() > 9_007_199_254_740_991 {
                Ok(json!(value as f64))
            } else {
                Ok(json!(value))
            }
        }
        Yaml::Real(value) => {
            let parsed = value
                .parse::<f64>()
                .map_err(|_| error(ErrorCode::InvalidYaml, path, "invalid YAML number"))?;
            if !parsed.is_finite() {
                return Err(error(
                    ErrorCode::InvalidYaml,
                    path,
                    "non-finite YAML number",
                ));
            }
            Ok(json!(parsed))
        }
        Yaml::String(value) => Ok(Value::String(value)),
        Yaml::Null => Ok(Value::Null),
        Yaml::Boolean(value) => Ok(Value::Bool(value)),
        _ => Err(error(
            ErrorCode::InvalidYaml,
            path,
            "unsupported YAML scalar",
        )),
    }
}

type ValidatedStructure = (Value, Vec<Value>, Option<Value>, Vec<u8>);

fn validate_structure(raw: &Value) -> Result<ValidatedStructure, CompilerError> {
    use ErrorCode::InvalidStructure as C;
    let root = object(
        raw,
        C,
        "/structure",
        &[
            "schema_version",
            "status",
            "skill_id",
            "source",
            "frontmatter",
            "document",
            "source_bindings",
            "role_profile",
            "sections",
        ],
    )?;
    if field(root, "schema_version").as_f64() != Some(1.0) {
        return Err(error(C, "/structure/schema_version", "must be 1"));
    }
    if field(root, "status") != STATUS {
        return Err(error(
            C,
            "/structure/status",
            "must preserve experimental ownership",
        ));
    }
    let skill_id = text(field(root, "skill_id"), C, "/structure/skill_id")?;
    let source = object(
        field(root, "source"),
        C,
        "/structure/source",
        &["path", "repository_revision", "sha256", "producer"],
    )?;
    let source = json!({
        "path": text(field(source,"path"),C,"/structure/source/path")?,
        "repository_revision": text(field(source,"repository_revision"),C,"/structure/source/repository_revision")?,
        "sha256": digest(field(source,"sha256"),C,"/structure/source/sha256")?,
        "producer": text(field(source,"producer"),C,"/structure/source/producer")?,
    });
    let frontmatter = object(
        field(root, "frontmatter"),
        C,
        "/structure/frontmatter",
        &["name", "description"],
    )?;
    let name = text(field(frontmatter, "name"), C, "/structure/frontmatter/name")?;
    if name != skill_id {
        return Err(error(
            C,
            "/structure/frontmatter/name",
            "must equal skill_id",
        ));
    }
    let description = text(
        field(frontmatter, "description"),
        C,
        "/structure/frontmatter/description",
    )?;
    if description.contains('\n') {
        return Err(error(
            C,
            "/structure/frontmatter/description",
            "must be one line",
        ));
    }
    let document = object(
        field(root, "document"),
        C,
        "/structure/document",
        &["line_ending", "final_newline"],
    )?;
    if field(document, "line_ending") != "lf" {
        return Err(error(
            C,
            "/structure/document/line_ending",
            "only LF is supported",
        ));
    }
    let final_newline = field(document, "final_newline")
        .as_bool()
        .ok_or_else(|| error(C, "/structure/document/final_newline", "must be boolean"))?;
    let bindings = array(
        field(root, "source_bindings"),
        C,
        "/structure/source_bindings",
    )?;
    if bindings.is_empty() {
        return Err(error(C, "/structure/source_bindings", "must be nonempty"));
    }
    let mut source_bindings = Vec::new();
    let mut ids = HashSet::new();
    for (index, binding) in bindings.iter().enumerate() {
        let path = format!("/structure/source_bindings/{index}");
        let binding = object(binding, C, &path, &["id", "kind", "path", "sha256", "role"])?;
        let id = text(field(binding, "id"), C, &format!("{path}/id"))?;
        if !ids.insert(id.clone()) {
            return Err(error(C, &path, "duplicate binding id"));
        }
        let kind = text(field(binding, "kind"), C, &format!("{path}/kind"))?;
        if !["canonical_authority", "generated_evidence"].contains(&kind.as_str()) {
            return Err(error(
                C,
                &format!("{path}/kind"),
                "unsupported source binding kind",
            ));
        }
        source_bindings.push(json!({"id":id,"kind":kind,"path":text(field(binding,"path"),C,&format!("{path}/path"))?,"sha256":digest(field(binding,"sha256"),C,&format!("{path}/sha256"))?,"role":text(field(binding,"role"),C,&format!("{path}/role"))?}));
    }
    let sections = array(field(root, "sections"), C, "/structure/sections")?;
    if sections.len() < 2 {
        return Err(error(
            C,
            "/structure/sections",
            "must meaningfully decompose document",
        ));
    }
    let mut normalized_sections = Vec::new();
    let mut body = String::new();
    ids.clear();
    for (index, section) in sections.iter().enumerate() {
        let path = format!("/structure/sections/{index}");
        let section = object(
            section,
            C,
            &path,
            &["id", "kind", "heading", "content", "source_span"],
        )?;
        let id = text(field(section, "id"), C, &format!("{path}/id"))?;
        if !ids.insert(id.clone()) {
            return Err(error(C, &path, "duplicate section id"));
        }
        let kind = text(field(section, "kind"), C, &format!("{path}/kind"))?;
        if ![
            "identity",
            "operation",
            "authority",
            "lifecycle",
            "planning",
            "implementation",
            "validation",
            "escalation",
            "boundary",
            "receipt",
        ]
        .contains(&kind.as_str())
        {
            return Err(error(
                C,
                &format!("{path}/kind"),
                "unsupported section kind",
            ));
        }
        let heading = object(
            field(section, "heading"),
            C,
            &format!("{path}/heading"),
            &["level", "text"],
        )?;
        let level = integer(field(heading, "level"), C, &format!("{path}/heading/level"))?;
        let level_number = level.as_f64().unwrap_or(0.0);
        if !(1.0..=6.0).contains(&level_number) {
            return Err(error(
                C,
                &format!("{path}/heading/level"),
                "must be 1 through 6",
            ));
        }
        let heading_text = text(field(heading, "text"), C, &format!("{path}/heading/text"))?;
        let content = field(section, "content")
            .as_str()
            .ok_or_else(|| error(C, &format!("{path}/content"), "must be text"))?;
        let span = object(
            field(section, "source_span"),
            C,
            &format!("{path}/source_span"),
            &["start_byte", "end_byte", "sha256"],
        )?;
        let start = integer(
            field(span, "start_byte"),
            C,
            &format!("{path}/source_span/start_byte"),
        )?;
        let end = integer(
            field(span, "end_byte"),
            C,
            &format!("{path}/source_span/end_byte"),
        )?;
        if end.as_f64().unwrap_or(0.0) <= start.as_f64().unwrap_or(0.0) {
            return Err(error(
                C,
                &format!("{path}/source_span"),
                "empty source span",
            ));
        }
        let span_digest = digest(
            field(span, "sha256"),
            C,
            &format!("{path}/source_span/sha256"),
        )?;
        let rendered = format!(
            "{} {heading_text}\n{content}",
            "#".repeat(level_number as usize)
        );
        if sha256_hex(rendered.as_bytes()) != span_digest {
            return Err(error(
                C,
                &format!("{path}/source_span/sha256"),
                "source span digest mismatch",
            ));
        }
        body.push_str(&rendered);
        normalized_sections.push(json!({"id":id,"kind":kind,"heading":{"level":level,"text":heading_text},"content":content,"source_span":{"start_byte":start,"end_byte":end,"sha256":span_digest}}));
    }
    let role = if field(root, "role_profile").is_null() {
        None
    } else {
        Some(validate_role(field(root, "role_profile"))?)
    };
    if final_newline && !body.ends_with('\n') {
        body.push('\n');
    }
    if !final_newline && body.ends_with('\n') {
        body.pop();
    }
    let output =
        format!("---\nname: {name}\ndescription: {description}\n---\n\n{body}").into_bytes();
    let structure = json!({"schema_version":1,"status":STATUS,"skill_id":skill_id,"source":source,"frontmatter":{"name":name,"description":description},"document":{"line_ending":"lf","final_newline":final_newline},"source_bindings":source_bindings,"sections":normalized_sections});
    Ok((structure, normalized_sections, role, output))
}

fn validate_role(raw: &Value) -> Result<Value, CompilerError> {
    use ErrorCode::InvalidStructure as C;
    let role = object(
        raw,
        C,
        "/structure/role_profile",
        &[
            "role_id",
            "label",
            "objective",
            "context_lifetime",
            "projection",
            "relations",
        ],
    )?;
    let projection = object(
        field(role, "projection"),
        C,
        "/structure/role_profile/projection",
        &["path", "sha256"],
    )?;
    let relations = object(
        field(role, "relations"),
        C,
        "/structure/role_profile/relations",
        &[
            "bound_by",
            "may_invoke",
            "may_observe",
            "may_mutate",
            "owns",
            "consumes",
            "emits",
            "mediated_transitions",
            "forbidden_from",
            "must_require",
            "observation_limits",
        ],
    )?;
    let mut normalized = Map::new();
    for relation in RELATIONS {
        let path = format!("/structure/role_profile/relations/{relation}");
        let mut entries = Vec::new();
        let mut seen = HashSet::new();
        for (index, entry) in array(field(relations, relation), C, &path)?
            .iter()
            .enumerate()
        {
            let item_path = format!("{path}/{index}");
            let normalized_entry = if *relation == "may_mutate"
                || *relation == "mediated_transitions"
            {
                let fields = if *relation == "may_mutate" {
                    ["target", "boundary"]
                } else {
                    ["transition", "mediated_by"]
                };
                let map = object(entry, C, &item_path, &fields)?;
                json!({fields[0]:text(field(map,fields[0]),C,&format!("{item_path}/{}",fields[0]))?,fields[1]:text(field(map,fields[1]),C,&format!("{item_path}/{}",fields[1]))?})
            } else {
                json!(text(entry, C, &item_path)?)
            };
            let key = canonical_json(&normalized_entry);
            if !seen.insert(key) {
                return Err(error(C, &path, "contains duplicates"));
            }
            entries.push(normalized_entry);
        }
        normalized.insert((*relation).to_owned(), Value::Array(entries));
    }
    for optional in ["must_require", "observation_limits"] {
        if field(relations, optional).is_null() {
            continue;
        }
        let path = format!("/structure/role_profile/relations/{optional}");
        let map = field(relations, optional)
            .as_object()
            .ok_or_else(|| error(C, &path, "must be an object"))?;
        let mut output = Map::new();
        for (key, value) in map {
            if key.is_empty() {
                return Err(error(C, &path, "empty relation key"));
            }
            let item = if optional == "must_require" {
                let list = unique_texts(value, C, &format!("{path}/{key}"), false)?;
                if list.is_empty() {
                    return Err(error(
                        C,
                        &format!("{path}/{key}"),
                        "must contain at least one invariant",
                    ));
                }
                json!(list)
            } else {
                json!(text(value, C, &format!("{path}/{key}"))?)
            };
            output.insert(key.clone(), item);
        }
        normalized.insert(optional.to_owned(), Value::Object(output));
    }
    Ok(
        json!({"role_id":text(field(role,"role_id"),C,"/structure/role_profile/role_id")?,"label":text(field(role,"label"),C,"/structure/role_profile/label")?,"objective":text(field(role,"objective"),C,"/structure/role_profile/objective")?,"context_lifetime":text(field(role,"context_lifetime"),C,"/structure/role_profile/context_lifetime")?,"projection":{"path":text(field(projection,"path"),C,"/structure/role_profile/projection/path")?,"sha256":digest(field(projection,"sha256"),C,"/structure/role_profile/projection/sha256")?},"relations":normalized}),
    )
}

fn validate_interface(
    raw: &Value,
    structure: &Value,
    sections: &[Value],
    role: Option<&Value>,
) -> Result<Value, CompilerError> {
    use ErrorCode::InvalidInterface as C;
    let root = object(
        raw,
        C,
        "/interface",
        &[
            "schema_version",
            "status",
            "skill_id",
            "boundaries",
            "runtime_requirements",
        ],
    )?;
    if field(root, "schema_version").as_f64() != Some(1.0) {
        return Err(error(C, "/interface/schema_version", "must be 1"));
    }
    if field(root, "status") != STATUS {
        return Err(error(
            C,
            "/interface/status",
            "must preserve experimental ownership",
        ));
    }
    if *field(root, "skill_id") != structure["skill_id"] {
        return Err(error(
            C,
            "/interface/skill_id",
            "must match structure skill_id",
        ));
    }
    let entries = array(field(root, "boundaries"), C, "/interface/boundaries")?;
    if entries.is_empty() {
        return Err(error(C, "/interface/boundaries", "must be nonempty"));
    }
    let authority_ids: HashSet<_> = structure["source_bindings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["kind"] == "canonical_authority")
        .filter_map(|item| item["id"].as_str())
        .collect();
    let mut ids = HashSet::new();
    let mut boundaries = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let path = format!("/interface/boundaries/{index}");
        let entry = object(
            entry,
            C,
            &path,
            &[
                "id",
                "kind",
                "semantic_section",
                "authority_source",
                "enforcement",
            ],
        )?;
        let id = text(field(entry, "id"), C, &format!("{path}/id"))?;
        if !ids.insert(id.clone()) {
            return Err(error(C, &path, "duplicate boundary id"));
        }
        let kind = text(field(entry, "kind"), C, &format!("{path}/kind"))?;
        let allowed: &[&str] = match kind.as_str() {
            "authority" => &["authority", "boundary"],
            "mutation" => &["implementation", "boundary"],
            "mediation" => &["implementation", "receipt"],
            "prohibition" => &["planning", "boundary"],
            "interface" => &["receipt"],
            _ => {
                return Err(error(
                    C,
                    &format!("{path}/kind"),
                    "unsupported boundary kind",
                ));
            }
        };
        let enforcement = text(
            field(entry, "enforcement"),
            C,
            &format!("{path}/enforcement"),
        )?;
        if !["scoped_write", "mediated", "denied", "receipt_contract"]
            .contains(&enforcement.as_str())
        {
            return Err(error(
                C,
                &format!("{path}/enforcement"),
                "unsupported enforcement primitive",
            ));
        }
        let semantic_section = text(
            field(entry, "semantic_section"),
            C,
            &format!("{path}/semantic_section"),
        )?;
        let section = sections
            .iter()
            .find(|section| section["id"] == semantic_section)
            .ok_or_else(|| {
                error(
                    C,
                    &format!("{path}/semantic_section"),
                    "unresolved semantic section",
                )
            })?;
        if !allowed.iter().any(|kind| section["kind"] == *kind) {
            return Err(error(
                C,
                &format!("{path}/semantic_section"),
                "boundary kind incompatible with section kind",
            ));
        }
        let authority_source = text(
            field(entry, "authority_source"),
            C,
            &format!("{path}/authority_source"),
        )?;
        if !authority_ids.contains(authority_source.as_str()) {
            return Err(error(
                C,
                &format!("{path}/authority_source"),
                "unresolved canonical authority source",
            ));
        }
        boundaries.push(json!({"id":id,"kind":kind,"semantic_section":semantic_section,"authority_source":authority_source,"enforcement":enforcement}));
    }
    let requirements = object(
        field(root, "runtime_requirements"),
        C,
        "/interface/runtime_requirements",
        &[
            "required_capabilities",
            "capability_ceiling",
            "effect_ceiling",
            "prohibited_effects",
            "continuity",
        ],
    )?;
    let mut lists = Map::new();
    for key in [
        "required_capabilities",
        "capability_ceiling",
        "effect_ceiling",
        "prohibited_effects",
    ] {
        lists.insert(
            key.to_owned(),
            json!(unique_texts(
                field(requirements, key),
                C,
                &format!("/interface/runtime_requirements/{key}"),
                true
            )?),
        );
    }
    let required = lists["required_capabilities"].as_array().unwrap();
    let ceiling = lists["capability_ceiling"].as_array().unwrap();
    let effects = lists["effect_ceiling"].as_array().unwrap();
    let prohibited = lists["prohibited_effects"].as_array().unwrap();
    if let Some(role) = role {
        let invoke = role["relations"]["may_invoke"].as_array().unwrap();
        for item in required.iter().chain(ceiling) {
            if !invoke.contains(item) {
                return Err(error(
                    C,
                    "/interface/runtime_requirements/capability_ceiling",
                    "capability not declared by role",
                ));
            }
        }
        let targets: HashSet<_> = role["relations"]["may_mutate"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|item| item["target"].as_str())
            .collect();
        for item in effects {
            if !targets.contains(item.as_str().unwrap()) {
                return Err(error(
                    C,
                    "/interface/runtime_requirements/effect_ceiling",
                    "effect not declared by role",
                ));
            }
        }
    }
    for item in required {
        if !ceiling.contains(item) {
            return Err(error(
                C,
                "/interface/runtime_requirements/required_capabilities",
                "required capability exceeds ceiling",
            ));
        }
    }
    for item in prohibited {
        if effects.contains(item) {
            return Err(error(
                C,
                "/interface/runtime_requirements/prohibited_effects",
                "effect both permitted and prohibited",
            ));
        }
    }
    let continuity = text(
        field(requirements, "continuity"),
        C,
        "/interface/runtime_requirements/continuity",
    )?;
    if !["ephemeral", "retained"].contains(&continuity.as_str()) {
        return Err(error(
            C,
            "/interface/runtime_requirements/continuity",
            "unsupported continuity",
        ));
    }
    lists.insert("continuity".to_owned(), json!(continuity));
    Ok(
        json!({"schema_version":1,"status":STATUS,"skill_id":structure["skill_id"],"boundaries":boundaries,"runtime_requirements":lists}),
    )
}

pub fn compile_skill_unverified(
    structure_source: &[u8],
    interface_source: &[u8],
) -> Result<CompiledSkill, CompilerError> {
    let structure_raw = parse_yaml(structure_source, "/structure")?;
    let interface_raw = parse_yaml(interface_source, "/interface")?;
    let (structure, sections, role, output) = validate_structure(&structure_raw)?;
    let interface = validate_interface(&interface_raw, &structure, &sections, role.as_ref())?;
    let output_sha256 = sha256_hex(&output);
    let bindings = structure["source_bindings"]
        .as_array()
        .expect("validated bindings");
    let canonical = bindings
        .iter()
        .find(|item| item["kind"] == "canonical_authority");
    let mut contract = json!({"path":structure["source"]["path"],"sha256":structure["source"]["sha256"],"must_be_activated":true,"kind":if role.is_some(){"role_contract"}else{"skill"}});
    if let Some(binding) = canonical {
        contract["source_binding_id"] = binding["id"].clone();
    }
    let mut requirements = json!({"schema_version":1,"skill_id":structure["skill_id"],"contract":contract,"compiled_skill_sha256":output_sha256,"verified_sources":false});
    if let Some(role) = &role {
        requirements["role_id"] = role["role_id"].clone();
    }
    for (key, value) in interface["runtime_requirements"]
        .as_object()
        .expect("validated requirements")
    {
        requirements[key] = value.clone();
    }
    let digest = sha256_hex(canonical_json(&requirements));
    requirements["sha256"] = json!(digest);
    let mut input =
        json!({"structure":sha256_hex(structure_source),"interface":sha256_hex(interface_source)});
    if let Some(role) = &role {
        input["role_projection"] = role["projection"]["sha256"].clone();
    }
    let provenance:Vec<_>=sections.iter().map(|section| json!({"id":section["id"],"kind":section["kind"],"source_span":section["source_span"]})).collect();
    let mut ir = json!({"schema_version":1,"compiler":PRODUCER,"skill_id":structure["skill_id"],"status":STATUS,"input_sha256":input,"source":structure["source"],"section_provenance":provenance,"source_bindings":structure["source_bindings"],"interface":interface,"runtime_requirements":requirements,"output_sha256":output_sha256});
    if let Some(role) = role {
        ir["role_profile"] = role;
    }
    Ok(CompiledSkill { ir, output })
}
